//! iRacing adapter for the shared telemetry model.
//!
//! [`IRacingSource`] connects to the sim's shared memory (reconnecting when the
//! sim restarts), maps each tick onto [`reo_model::Telemetry`], and re-parses the
//! session YAML into [`reo_model::SessionInfo`] whenever iRacing revises it.

pub mod session;
mod telemetry;

use std::sync::Arc;
use std::time::{Duration, Instant};

use reo_irsdk::{Client, Frame, SharedMemory, Update, VarTable};
use reo_model::{Poll, SessionInfo, SimId, Telemetry, TelemetrySource};

pub use telemetry::{Handles, flags as map_flags};

/// How long to wait between attempts to open the mapping while the sim is closed.
const RECONNECT_INTERVAL: Duration = Duration::from_secs(1);

/// Blocks until the sim signals new data or the timeout passes.
pub type Waiter = Box<dyn Fn(Duration) -> bool + Send>;

/// An open shared-memory connection.
pub struct Connection<M> {
    pub client: Client<M>,
    /// Waits on the sim's data-valid event. Without one, polls never block.
    pub waiter: Option<Waiter>,
}

type Opener<M> = Box<dyn FnMut() -> Option<Connection<M>> + Send>;

/// A [`TelemetrySource`] backed by iRacing's shared memory (or anything laid out like it).
pub struct IRacingSource<M> {
    opener: Opener<M>,
    conn: Option<Connection<M>>,
    retry_at: Option<Instant>,
    frame: Frame,
    handles: Handles,
    session: Option<Arc<SessionInfo>>,
    session_yaml: Option<String>,
    last_error: Option<String>,
}

impl<M: SharedMemory> IRacingSource<M> {
    /// Builds a source that calls `opener` to (re)connect.
    pub fn with_opener(opener: impl FnMut() -> Option<Connection<M>> + Send + 'static) -> Self {
        Self {
            opener: Box::new(opener),
            conn: None,
            retry_at: None,
            frame: Frame::default(),
            handles: Handles::default(),
            session: None,
            session_yaml: None,
            last_error: None,
        }
    }

    /// Variable table of the current connection.
    pub fn vars(&self) -> Option<&Arc<VarTable>> {
        self.conn.as_ref().map(|c| c.client.vars())
    }

    /// The raw row behind the last [`Poll::Updated`].
    pub fn raw_frame(&self) -> &Frame {
        &self.frame
    }

    /// Unparsed session YAML as last read from the sim.
    pub fn session_yaml(&self) -> Option<&str> {
        self.session_yaml.as_deref()
    }

    /// Most recent connection, read or parse problem, kept for diagnostics.
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    fn connect(&mut self, timeout: Duration) -> bool {
        if self.conn.is_some() {
            return true;
        }
        let now = Instant::now();
        if self.retry_at.is_some_and(|t| now < t) {
            std::thread::sleep(timeout.min(self.retry_at.unwrap_or(now) - now));
            return false;
        }
        self.conn = (self.opener)();
        if self.conn.is_none() {
            self.retry_at = Some(now + RECONNECT_INTERVAL);
            std::thread::sleep(timeout.min(RECONNECT_INTERVAL));
        }
        self.conn.is_some()
    }

    fn read_once(&mut self, out: &mut Telemetry) -> Result<Poll, reo_irsdk::Error> {
        let Some(conn) = self.conn.as_mut() else {
            return Ok(Poll::Disconnected);
        };
        let (vars_changed, session_changed) = match conn.client.poll(&mut self.frame)? {
            Update::Disconnected => return Ok(Poll::Disconnected),
            Update::Unchanged => return Ok(Poll::Unchanged),
            Update::NewFrame {
                vars_changed,
                session_changed,
            } => (vars_changed, session_changed),
        };
        if vars_changed {
            self.handles = Handles::resolve(conn.client.vars());
        }
        let mut session_updated = false;
        if session_changed {
            // A failed read leaves the revision unseen, so the next tick retries it.
            match conn.client.session_info() {
                Ok((yaml, revision)) => {
                    match session::parse(&yaml, revision) {
                        Ok(info) => {
                            self.session = Some(Arc::new(info));
                            session_updated = true;
                        }
                        Err(e) => self.last_error = Some(e.to_string()),
                    }
                    self.session_yaml = Some(yaml);
                }
                Err(e) => self.last_error = Some(e.to_string()),
            }
        }
        self.handles.fill(&self.frame.row, self.frame.tick, out);
        Ok(Poll::Updated {
            session_changed: session_updated,
        })
    }
}

impl<M: SharedMemory> TelemetrySource for IRacingSource<M>
where
    M: Send,
{
    fn sim(&self) -> SimId {
        SimId::IRacing
    }

    fn poll(&mut self, timeout: Duration, out: &mut Telemetry) -> Poll {
        if !self.connect(timeout) {
            return Poll::Disconnected;
        }
        let mut waited = false;
        loop {
            let result = match self.read_once(out) {
                Ok(r) => r,
                Err(e) => {
                    // A malformed or truncated mapping: drop it and reconnect later.
                    self.last_error = Some(e.to_string());
                    self.conn = None;
                    self.retry_at = Some(Instant::now() + RECONNECT_INTERVAL);
                    return Poll::Disconnected;
                }
            };
            let updated = matches!(result, Poll::Updated { .. });
            let waiter = self.conn.as_ref().and_then(|c| c.waiter.as_ref());
            match waiter {
                Some(wait) if !updated && !waited => {
                    wait(timeout);
                    waited = true;
                }
                _ => return result,
            }
        }
    }

    fn session(&self) -> Option<Arc<SessionInfo>> {
        self.session.clone()
    }
}

#[cfg(windows)]
mod live {
    use super::{Connection, IRacingSource};
    use reo_irsdk::{Client, DataValidEvent, IRACING_EVENT_NAME, IRACING_MAPPING_NAME, MappedFile};

    impl IRacingSource<MappedFile> {
        /// Connects to the running sim.
        pub fn live() -> Self {
            Self::named(IRACING_MAPPING_NAME, IRACING_EVENT_NAME)
        }

        /// Connects to a mapping and event with non-default names, e.g. a replay tool's.
        pub fn named(mapping: &str, event: &str) -> Self {
            let (mapping, event) = (mapping.to_owned(), event.to_owned());
            Self::with_opener(move || {
                let mem = MappedFile::open(&mapping).ok()?;
                let waiter = DataValidEvent::open(&event)
                    .ok()
                    .map(|ev| Box::new(move |t| ev.wait(t)) as super::Waiter);
                Some(Connection {
                    client: Client::new(mem),
                    waiter,
                })
            })
        }
    }
}
