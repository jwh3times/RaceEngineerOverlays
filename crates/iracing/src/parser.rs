//! Decodes and parses session YAML on a worker thread, away from the telemetry poll.
//!
//! [`SessionParser`] is the poll thread's side: it copies each revision into a
//! reused buffer, leaves it in a one-slot [`Mailbox`] for the [`Worker`], picks
//! finished parses up without blocking, and keeps the newest parsed
//! [`SessionInfo`] and the YAML behind it. Results are never applied out of
//! order: one older than the session already shown is dropped.
//!
//! In steady state the poll thread's side does not allocate: the copy buffer
//! goes round between the two threads, and the worker builds everything it
//! returns. A new buffer is allocated only for the first revision, for one
//! that arrives while the worker holds the only buffer and another revision is
//! already waiting, and when a revision outgrows the buffer it reuses.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use reo_model::SessionInfo;

use crate::session;

/// Counters for the session parse, for diagnostics such as `reo check`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionStats {
    /// Session revisions read from the sim and handed to the parser.
    pub revisions: u64,
    /// Parses that replaced the current [`SessionInfo`].
    pub applied: u64,
    /// How long the poll thread took to copy the most recent revision out of
    /// shared memory.
    pub last_copy: Option<Duration>,
    /// How long the poll thread took to hand the most recent revision to the
    /// worker, after the copy.
    pub last_hand_off: Option<Duration>,
    /// How long the most recent finished decode and parse took on the worker.
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
    /// Hand-off number of the newest result applied, parsed or not.
    current: u64,
    session: Option<Arc<SessionInfo>>,
    /// Raw YAML of the revision behind `current`.
    yaml: Option<String>,
    /// A copy buffer the worker has finished with, or that a newer revision displaced.
    spare: Option<Vec<u8>>,
    /// A parse was applied since the last `take_fresh`.
    fresh: bool,
    applied: u64,
    last_copy: Option<Duration>,
    last_hand_off: Option<Duration>,
    last_parse: Option<Duration>,
    error: Option<String>,
}

impl SessionParser {
    pub fn session(&self) -> Option<Arc<SessionInfo>> {
        self.session.clone()
    }

    /// Raw YAML of the newest revision the worker has finished with, whether
    /// or not it parsed.
    pub fn yaml(&self) -> Option<&str> {
        self.yaml.as_deref()
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
            last_copy: self.last_copy,
            last_hand_off: self.last_hand_off,
            last_parse: self.last_parse,
        }
    }

    /// Copies a revision with `read` into a reused buffer and hands it to the
    /// worker, starting one if needed. `read` fills the buffer with the raw
    /// text and returns its revision; if it fails, nothing is handed off.
    /// Never parses here: if no worker can take it, the revision is reported
    /// lost and the next one tries again.
    pub fn read_and_hand_off<E>(
        &mut self,
        read: impl FnOnce(&mut Vec<u8>) -> Result<i32, E>,
    ) -> Result<(), E> {
        let begin = Instant::now();
        let mut raw = self.take_buffer();
        let revision = match read(&mut raw) {
            Ok(revision) => revision,
            Err(e) => {
                self.spare = Some(raw);
                return Err(e);
            }
        };
        let copied = Instant::now();
        self.hand_off(raw, revision);
        self.last_copy = Some(copied - begin);
        self.last_hand_off = Some(copied.elapsed());
        Ok(())
    }

    fn take_buffer(&mut self) -> Vec<u8> {
        self.spare
            .take()
            .or_else(|| self.worker.as_ref().and_then(Worker::take_spare))
            .unwrap_or_default()
    }

    fn hand_off(&mut self, raw: Vec<u8>, revision: i32) {
        self.handed_off += 1;
        if self.worker.is_none() {
            match Worker::spawn() {
                Ok(worker) => self.worker = Some(worker),
                Err(e) => {
                    self.spare = Some(raw);
                    return self.give_up(format!("session parser did not start: {e}"));
                }
            }
        }
        let req = Request {
            seq: self.handed_off,
            raw,
            revision,
        };
        if let Some(worker) = &self.worker {
            // Newest wins: a revision the worker has not started is stale now.
            // Its hand-off number is settled when this one comes back.
            if let Some(displaced) = worker.post(req) {
                self.spare.get_or_insert(displaced.raw);
            }
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

    /// Applies a result newer than the one shown. Applying a newer one even
    /// while a later revision is still pending keeps a steady stream of
    /// revisions from starving the session.
    fn finish(&mut self, parsed: Parsed) {
        self.settled = self.settled.max(parsed.seq);
        self.last_parse = Some(parsed.took);
        if parsed.seq <= self.current {
            return;
        }
        self.current = parsed.seq;
        self.yaml = Some(parsed.yaml);
        match parsed.result {
            Ok(info) => {
                self.session = Some(info);
                self.fresh = true;
                self.applied += 1;
            }
            Err(e) => self.error = Some(e),
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

/// A session revision copied out of shared memory, still Windows-1252, numbered
/// in hand-off order.
struct Request {
    seq: u64,
    raw: Vec<u8>,
    revision: i32,
}

/// A finished decode and parse, carrying the hand-off number of the request it
/// answers. The worker builds every allocation in it, so applying it allocates
/// nothing on the poll thread.
struct Parsed {
    seq: u64,
    yaml: String,
    result: Result<Arc<SessionInfo>, String>,
    took: Duration,
}

/// The one revision waiting for the worker, and the buffer it gives back.
#[derive(Default)]
struct Mailbox {
    slot: Mutex<Slot>,
    posted: Condvar,
}

#[derive(Default)]
struct Slot {
    request: Option<Request>,
    spare: Option<Vec<u8>>,
    closed: bool,
}

impl Mailbox {
    fn lock(&self) -> MutexGuard<'_, Slot> {
        // Nothing panics while holding the lock, so the slot is always whole.
        self.slot.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Leaves `req` for the worker, returning the request it displaces.
    fn post(&self, req: Request) -> Option<Request> {
        let displaced = self.lock().request.replace(req);
        self.posted.notify_one();
        displaced
    }

    /// Blocks until a request is posted. `None` once the mailbox is closed,
    /// even with a request still waiting.
    fn wait_for_request(&self) -> Option<Request> {
        let mut slot = self.lock();
        loop {
            if slot.closed {
                return None;
            }
            if let Some(req) = slot.request.take() {
                return Some(req);
            }
            slot = self.posted.wait(slot).unwrap_or_else(|e| e.into_inner());
        }
    }

    /// Keeps one spare buffer; one already waiting is dropped, here on the worker.
    fn give_back(&self, buffer: Vec<u8>) {
        self.lock().spare = Some(buffer);
    }

    fn take_spare(&self) -> Option<Vec<u8>> {
        self.lock().spare.take()
    }

    fn close(&self) {
        self.lock().closed = true;
        self.posted.notify_one();
    }
}

/// The parse thread. Dropping it closes the mailbox and the result channel
/// before joining, so the thread finishes at most the parse it is running and
/// skips anything waiting.
struct Worker {
    mailbox: Arc<Mailbox>,
    results: Option<Receiver<Parsed>>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn spawn() -> std::io::Result<Self> {
        let mailbox = Arc::new(Mailbox::default());
        let (outbox, results) = mpsc::channel();
        let inbox = Arc::clone(&mailbox);
        let thread = std::thread::Builder::new()
            .name("reo-session-parse".into())
            .spawn(move || run(&inbox, &outbox))?;
        Ok(Self {
            mailbox,
            results: Some(results),
            thread: Some(thread),
        })
    }

    /// Leaves `req` for the thread, returning the request it displaces.
    fn post(&self, req: Request) -> Option<Request> {
        self.mailbox.post(req)
    }

    /// A buffer the thread has finished with.
    fn take_spare(&self) -> Option<Vec<u8>> {
        self.mailbox.take_spare()
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
        self.mailbox.close();
        self.results = None;
        if let Some(thread) = self.thread.take() {
            // A worker that panicked already showed as a closed result channel.
            let _ = thread.join();
        }
    }
}

fn run(inbox: &Mailbox, outbox: &Sender<Parsed>) {
    while let Some(req) = inbox.wait_for_request() {
        let parsed = decode_and_parse(&req);
        // Back before the result, so the poll thread that applies it can reuse it.
        inbox.give_back(req.raw);
        if outbox.send(parsed).is_err() {
            return;
        }
    }
}

fn decode_and_parse(req: &Request) -> Parsed {
    let begin = Instant::now();
    let yaml = reo_irsdk::decode_session_info(&req.raw);
    let result = session::parse(&yaml, req.revision)
        .map(Arc::new)
        .map_err(|e| e.to_string());
    Parsed {
        seq: req.seq,
        yaml,
        result,
        took: begin.elapsed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = "WeekendInfo:\n TrackName: spa\n";

    fn request(seq: u64) -> Request {
        Request {
            seq,
            raw: YAML.as_bytes().to_vec(),
            revision: i32::try_from(seq).unwrap(),
        }
    }

    fn parsed(seq: u64) -> Parsed {
        decode_and_parse(&request(seq))
    }

    fn revision(parser: &SessionParser) -> Option<i64> {
        parser.session.as_ref().map(|s| s.revision)
    }

    #[test]
    fn a_newer_revision_displaces_one_the_worker_has_not_started() {
        let mailbox = Mailbox::default();
        assert!(mailbox.post(request(1)).is_none());
        assert_eq!(mailbox.post(request(2)).map(|r| r.seq), Some(1));
        assert_eq!(mailbox.post(request(3)).map(|r| r.seq), Some(2));
        assert_eq!(mailbox.wait_for_request().map(|r| r.seq), Some(3));
    }

    #[test]
    fn a_closed_mailbox_skips_the_waiting_request() {
        let mailbox = Mailbox::default();
        mailbox.post(request(1));
        mailbox.close();
        assert!(mailbox.wait_for_request().is_none());
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
