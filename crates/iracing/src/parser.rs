//! Parses session YAML on a worker thread, away from the telemetry poll.
//!
//! [`SessionParser`] is the poll thread's side: it hands each revision to a
//! [`Worker`], picks finished parses up without blocking, and keeps the newest
//! parsed [`SessionInfo`]. Results are never applied out of order: one older
//! than the session already shown is dropped.

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use reo_model::SessionInfo;

use crate::session::{self, ParseError};

/// Counters for the session parse, for diagnostics such as `reo check`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionStats {
    /// Session revisions read from the sim and handed to the parser.
    pub revisions: u64,
    /// Parses that replaced the current [`SessionInfo`].
    pub applied: u64,
    /// How long the most recent finished parse took on the worker.
    pub last_parse: Option<Duration>,
}

/// The poll thread's side of the session parse.
#[derive(Default)]
pub(crate) struct SessionParser {
    /// Started on the first revision, and again on the next one after it stops.
    worker: Option<Worker>,
    /// Hand-off number of the newest revision; also the count of revisions.
    handed_off: u64,
    /// Highest hand-off number that came back or was given up on.
    settled: u64,
    /// Hand-off number of the parse behind `session`.
    current: u64,
    session: Option<Arc<SessionInfo>>,
    /// A parse was applied since the last `take_fresh`.
    fresh: bool,
    applied: u64,
    last_parse: Option<Duration>,
    error: Option<String>,
}

impl SessionParser {
    pub fn session(&self) -> Option<Arc<SessionInfo>> {
        self.session.clone()
    }

    /// Whether a parse was applied since the last call.
    pub fn take_fresh(&mut self) -> bool {
        std::mem::take(&mut self.fresh)
    }

    /// The newest parse or worker problem, once.
    pub fn take_error(&mut self) -> Option<String> {
        self.error.take()
    }

    pub fn stats(&self) -> SessionStats {
        SessionStats {
            revisions: self.handed_off,
            applied: self.applied,
            last_parse: self.last_parse,
        }
    }

    /// Hands a revision to the worker, starting one if needed. Never parses
    /// here: if no worker can take it, the revision is reported lost and the
    /// next one tries again.
    pub fn hand_off(&mut self, yaml: String, revision: i32) {
        self.handed_off += 1;
        let req = Request {
            seq: self.handed_off,
            yaml,
            revision,
        };
        if self.worker.is_none() {
            match Worker::spawn() {
                Ok(worker) => self.worker = Some(worker),
                Err(e) => return self.give_up(format!("session parser did not start: {e}")),
            }
        }
        if self.worker.as_ref().is_some_and(|w| w.send(req).is_err()) {
            self.stopped();
        }
    }

    /// Applies every finished parse. Never blocks and never allocates.
    pub fn collect(&mut self) {
        loop {
            let next = match &self.worker {
                Some(w) => w.try_recv(),
                None => return,
            };
            match next {
                Ok(parsed) => self.finish(parsed),
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => return self.stopped(),
            }
        }
    }

    /// Waits up to `timeout` until nothing handed off is still pending.
    /// Returns `false` on timeout.
    pub fn wait(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            self.collect();
            if self.settled >= self.handed_off {
                return true;
            }
            let next = match &self.worker {
                Some(w) => w.recv_timeout(deadline.saturating_duration_since(Instant::now())),
                None => return true,
            };
            match next {
                Ok(parsed) => self.finish(parsed),
                Err(RecvTimeoutError::Timeout) => return false,
                Err(RecvTimeoutError::Disconnected) => self.stopped(),
            }
        }
    }

    /// Applies a parse newer than the session shown. Applying a newer one even
    /// while a later revision is still pending keeps a steady stream of
    /// revisions from starving the session.
    fn finish(&mut self, parsed: Parsed) {
        self.settled = self.settled.max(parsed.seq);
        self.last_parse = Some(parsed.took);
        if parsed.seq <= self.current {
            return;
        }
        match parsed.result {
            Ok(info) => {
                self.current = parsed.seq;
                self.session = Some(Arc::new(info));
                self.fresh = true;
                self.applied += 1;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// The worker died, which only a panic in the parse can cause. Telemetry
    /// carries on with the previous session; the next revision starts a new worker.
    fn stopped(&mut self) {
        self.worker = None;
        self.give_up("session parser stopped unexpectedly".into());
    }

    fn give_up(&mut self, error: String) {
        self.settled = self.handed_off;
        self.error = Some(error);
    }
}

/// A session revision copied out of shared memory, numbered in hand-off order.
struct Request {
    seq: u64,
    yaml: String,
    revision: i32,
}

/// A finished parse, carrying the hand-off number of the request it answers.
struct Parsed {
    seq: u64,
    result: Result<SessionInfo, ParseError>,
    took: Duration,
}

/// The parse thread. Dropping it closes both channels before joining, so the
/// thread finishes at most the parse it is running and skips anything queued.
struct Worker {
    requests: Option<Sender<Request>>,
    results: Option<Receiver<Parsed>>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn spawn() -> std::io::Result<Self> {
        let (requests, inbox) = mpsc::channel();
        let (outbox, results) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("reo-session-parse".into())
            .spawn(move || run(&inbox, &outbox))?;
        Ok(Self {
            requests: Some(requests),
            results: Some(results),
            thread: Some(thread),
        })
    }

    fn send(&self, req: Request) -> Result<(), ()> {
        match &self.requests {
            Some(tx) => tx.send(req).map_err(drop),
            None => Err(()),
        }
    }

    fn try_recv(&self) -> Result<Parsed, TryRecvError> {
        self.results
            .as_ref()
            .map_or(Err(TryRecvError::Disconnected), Receiver::try_recv)
    }

    fn recv_timeout(&self, timeout: Duration) -> Result<Parsed, RecvTimeoutError> {
        self.results
            .as_ref()
            .map_or(Err(RecvTimeoutError::Disconnected), |rx| {
                rx.recv_timeout(timeout)
            })
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.requests = None;
        self.results = None;
        if let Some(thread) = self.thread.take() {
            // A worker that panicked already showed as a closed result channel.
            let _ = thread.join();
        }
    }
}

fn run(inbox: &Receiver<Request>, outbox: &Sender<Parsed>) {
    while let Ok(mut req) = inbox.recv() {
        // Newest wins: revisions that queued up during the last parse are stale.
        while let Ok(newer) = inbox.try_recv() {
            req = newer;
        }
        let begin = Instant::now();
        let result = session::parse(&req.yaml, req.revision);
        let parsed = Parsed {
            seq: req.seq,
            result,
            took: begin.elapsed(),
        };
        if outbox.send(parsed).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = "WeekendInfo:\n TrackName: spa\n";

    fn request(seq: u64) -> Request {
        Request {
            seq,
            yaml: YAML.into(),
            revision: i32::try_from(seq).unwrap(),
        }
    }

    fn parsed(seq: u64) -> Parsed {
        let req = request(seq);
        Parsed {
            seq,
            result: session::parse(&req.yaml, req.revision),
            took: Duration::ZERO,
        }
    }

    fn revision(parser: &SessionParser) -> Option<i64> {
        parser.session.as_ref().map(|s| s.revision)
    }

    #[test]
    fn the_worker_parses_only_the_newest_queued_revision() {
        let (requests, inbox) = mpsc::channel();
        let (outbox, results) = mpsc::channel();
        for seq in 1..=3 {
            requests.send(request(seq)).unwrap();
        }
        drop(requests);
        run(&inbox, &outbox);
        drop(outbox);
        let seqs: Vec<u64> = results.iter().map(|p| p.seq).collect();
        assert_eq!(seqs, [3]);
    }

    #[test]
    fn an_older_parse_never_replaces_a_newer_one() {
        let mut parser = SessionParser {
            handed_off: 2,
            ..SessionParser::default()
        };
        parser.finish(parsed(2));
        parser.finish(parsed(1));
        assert_eq!(revision(&parser), Some(2));
        assert_eq!(parser.stats().applied, 1);
    }

    #[test]
    fn a_newer_parse_applies_while_a_later_revision_is_pending() {
        let mut parser = SessionParser {
            handed_off: 3,
            ..SessionParser::default()
        };
        parser.finish(parsed(2));
        assert_eq!(revision(&parser), Some(2));
        assert!(parser.take_fresh());
        assert!(
            parser.settled < parser.handed_off,
            "revision 3 is still pending"
        );
    }
}
