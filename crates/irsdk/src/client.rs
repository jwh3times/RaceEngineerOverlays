//! Polls a shared-memory region for new telemetry rows and session-info revisions.

use std::sync::Arc;

use crate::Error;
use crate::layout::{
    HEADER_SIZE, Header, RawVarHeader, SUPPORTED_VERSION, VAR_HEADER_SIZE, tick_count_offset,
};
use crate::memory::SharedMemory;
use crate::vars::VarTable;

/// Upper bounds that reject a corrupt or foreign header before allocating.
const MAX_ROW_LEN: usize = 1 << 20;
const MAX_VARS: usize = 8192;
const MAX_SESSION_INFO_LEN: usize = 16 << 20;
/// Attempts to copy a row that the sim is not rewriting at the same time.
const READ_ATTEMPTS: usize = 4;
/// How much of the session-info region is copied before looking for its end.
const SESSION_CHUNK: usize = 16 * 1024;

/// One telemetry row, reused between polls to avoid per-tick allocation.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub tick: i32,
    pub row: Vec<u8>,
}

/// Result of one poll.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Update {
    /// The sim is not running or not in a session.
    Disconnected,
    /// Connected, but no tick newer than the last one returned.
    Unchanged,
    /// `frame` now holds a newer tick.
    NewFrame {
        /// The variable layout changed; re-resolve every [`crate::Var`] handle.
        vars_changed: bool,
        /// The session-info YAML has a new revision; call [`Client::session_info`].
        session_changed: bool,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct LayoutKey {
    ver: i32,
    num_vars: i32,
    var_header_offset: i32,
    buf_len: i32,
}

impl LayoutKey {
    fn of(h: &Header) -> Self {
        Self {
            ver: h.ver,
            num_vars: h.num_vars,
            var_header_offset: h.var_header_offset,
            buf_len: h.buf_len,
        }
    }
}

/// Reads telemetry out of an iRacing SDK shared-memory region.
pub struct Client<M> {
    mem: M,
    header: Header,
    layout: Option<LayoutKey>,
    vars: Arc<VarTable>,
    last_tick: Option<i32>,
    session_update: Option<i32>,
}

impl<M: SharedMemory> Client<M> {
    pub fn new(mem: M) -> Self {
        Self {
            mem,
            header: Header::default(),
            layout: None,
            vars: Arc::new(VarTable::default()),
            last_tick: None,
            session_update: None,
        }
    }

    /// Header as of the last poll.
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Variable table for the current connection. Empty until the first frame.
    pub fn vars(&self) -> &Arc<VarTable> {
        &self.vars
    }

    pub fn memory(&self) -> &M {
        &self.mem
    }

    /// Checks for a newer tick and copies it into `frame`.
    pub fn poll(&mut self, frame: &mut Frame) -> Result<Update, Error> {
        let header = self.read_header()?;
        self.header = header;

        if !header.is_connected() {
            self.forget_connection();
            return Ok(Update::Disconnected);
        }
        validate(&header)?;

        let mut vars_changed = false;
        let key = LayoutKey::of(&header);
        if self.layout != Some(key) {
            self.vars = Arc::new(self.read_var_table(&header)?);
            self.layout = Some(key);
            self.last_tick = None;
            self.session_update = None;
            vars_changed = true;
        }

        let Some(tick) = self.read_latest_row(frame)? else {
            return Ok(Update::Unchanged);
        };
        if self.last_tick == Some(tick) {
            return Ok(Update::Unchanged);
        }
        self.last_tick = Some(tick);
        frame.tick = tick;

        let session_changed = self.session_update != Some(self.header.session_info_update);
        Ok(Update::NewFrame {
            vars_changed,
            session_changed,
        })
    }

    /// Reads and decodes the current session-info YAML, and marks this revision
    /// as seen. Returns the text and its revision number. Allocates; a poll loop
    /// uses [`Self::read_session_info`] with a reused buffer instead.
    pub fn session_info(&mut self) -> Result<(String, i32), Error> {
        let mut raw = Vec::new();
        let revision = self.read_session_info(&mut raw)?;
        Ok((decode_session_info(&raw), revision))
    }

    /// Copies the current session-info text into `out`, still encoded as
    /// Windows-1252 and without its NUL terminator, and marks this revision as
    /// seen. Returns the revision number.
    ///
    /// `out` is cleared first and its capacity reused, so a caller that keeps
    /// the buffer allocates only when a revision outgrows it. The sim reserves a
    /// region much larger than the text; the copy stops at the terminator.
    pub fn read_session_info(&mut self, out: &mut Vec<u8>) -> Result<i32, Error> {
        for _ in 0..READ_ATTEMPTS {
            let header = self.read_header()?;
            let len = usize::try_from(header.session_info_len)
                .ok()
                .filter(|&l| l <= MAX_SESSION_INFO_LEN)
                .ok_or(Error::InvalidHeader("session info length"))?;
            let offset = usize::try_from(header.session_info_offset)
                .map_err(|_| Error::InvalidHeader("session info offset"))?;
            self.copy_until_nul(offset, len, out)?;
            // The sim may have started a new revision while we copied.
            if self.read_header()?.session_info_update != header.session_info_update {
                continue;
            }
            self.session_update = Some(header.session_info_update);
            return Ok(header.session_info_update);
        }
        Err(Error::Busy("session info"))
    }

    /// Copies `len` bytes at `offset` into `out` a chunk at a time, stopping at
    /// the first NUL, which is not kept.
    fn copy_until_nul(&self, offset: usize, len: usize, out: &mut Vec<u8>) -> Result<(), Error> {
        out.clear();
        // The whole region must exist even though only the text is copied.
        if offset
            .checked_add(len)
            .is_none_or(|end| end > self.mem.len())
        {
            return Err(Error::Truncated("session info"));
        }
        while out.len() < len {
            let start = out.len();
            let end = (start + SESSION_CHUNK).min(len);
            out.resize(end, 0);
            if !self.mem.read_into(offset + start, &mut out[start..]) {
                return Err(Error::Truncated("session info"));
            }
            // `CStr` finds the NUL a word at a time; a byte loop costs ~20 µs here.
            if let Ok(text) = std::ffi::CStr::from_bytes_until_nul(&out[start..]) {
                out.truncate(start + text.count_bytes());
                return Ok(());
            }
        }
        Ok(())
    }

    fn forget_connection(&mut self) {
        self.layout = None;
        self.last_tick = None;
        self.session_update = None;
    }

    fn read_header(&self) -> Result<Header, Error> {
        let mut raw = [0u8; HEADER_SIZE];
        if !self.mem.read_into(0, &mut raw) {
            return Err(Error::Truncated("header"));
        }
        Ok(Header::parse(&raw))
    }

    fn read_var_table(&self, header: &Header) -> Result<VarTable, Error> {
        let count = header.num_vars as usize;
        let mut raw = vec![0u8; count * VAR_HEADER_SIZE];
        if !self
            .mem
            .read_into(header.var_header_offset as usize, &mut raw)
        {
            return Err(Error::Truncated("variable headers"));
        }
        let headers = raw
            .as_chunks::<VAR_HEADER_SIZE>()
            .0
            .iter()
            .map(|chunk| RawVarHeader::parse(chunk))
            .collect();
        Ok(VarTable::from_raw(headers, header.buf_len as usize))
    }

    /// Copies the newest row into `frame.row`. Returns its tick, or `None` if the
    /// sim kept rewriting the buffer while we copied.
    fn read_latest_row(&mut self, frame: &mut Frame) -> Result<Option<i32>, Error> {
        let row_len = self.header.buf_len as usize;
        frame.row.resize(row_len, 0);
        for attempt in 0..READ_ATTEMPTS {
            if attempt > 0 {
                self.header = self.read_header()?;
            }
            let (index, buf) = self.header.latest_buf();
            if self.last_tick == Some(buf.tick_count) {
                return Ok(Some(buf.tick_count));
            }
            let offset = usize::try_from(buf.buf_offset)
                .map_err(|_| Error::InvalidHeader("buffer offset"))?;
            if !self.mem.read_into(offset, &mut frame.row) {
                return Err(Error::Truncated("telemetry row"));
            }
            let after = self
                .mem
                .read_i32(tick_count_offset(index))
                .ok_or(Error::Truncated("header"))?;
            if after == buf.tick_count {
                return Ok(Some(buf.tick_count));
            }
        }
        Ok(None)
    }
}

/// Decodes session-info text copied by [`Client::read_session_info`] from
/// Windows-1252, which is what iRacing writes.
pub fn decode_session_info(raw: &[u8]) -> String {
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(raw);
    text.into_owned()
}

fn validate(h: &Header) -> Result<(), Error> {
    if h.ver != SUPPORTED_VERSION {
        return Err(Error::UnsupportedVersion(h.ver));
    }
    if !(1..=crate::layout::MAX_BUFS as i32).contains(&h.num_buf) {
        return Err(Error::InvalidHeader("buffer count"));
    }
    if h.buf_len <= 0 || h.buf_len as usize > MAX_ROW_LEN {
        return Err(Error::InvalidHeader("row length"));
    }
    if h.num_vars < 0 || h.num_vars as usize > MAX_VARS || h.var_header_offset < 0 {
        return Err(Error::InvalidHeader("variable table"));
    }
    Ok(())
}
