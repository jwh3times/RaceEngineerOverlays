//! Binary layout of the iRacing SDK shared-memory header and variable headers.
//!
//! All values are little-endian. Offsets follow `irsdk_defines.h` (SDK version 2).

/// Size in bytes of `irsdk_header`.
pub const HEADER_SIZE: usize = 112;
/// Size in bytes of one `irsdk_varHeader`.
pub const VAR_HEADER_SIZE: usize = 144;
/// Maximum number of rotating data buffers in the header.
pub const MAX_BUFS: usize = 4;
/// `irsdk_stConnected` bit in `Header::status`.
pub const STATUS_CONNECTED: i32 = 1;
/// Header version this reader understands.
pub const SUPPORTED_VERSION: i32 = 2;

const VAR_BUF_BASE: usize = 48;
const VAR_BUF_STRIDE: usize = 16;

/// Byte offset of `varBuf[index].tickCount` from the start of the mapping.
pub const fn tick_count_offset(index: usize) -> usize {
    VAR_BUF_BASE + VAR_BUF_STRIDE * index
}

/// One entry of the rotating data-buffer table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VarBuf {
    pub tick_count: i32,
    pub buf_offset: i32,
}

/// Parsed `irsdk_header`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Header {
    pub ver: i32,
    pub status: i32,
    pub tick_rate: i32,
    pub session_info_update: i32,
    pub session_info_len: i32,
    pub session_info_offset: i32,
    pub num_vars: i32,
    pub var_header_offset: i32,
    pub num_buf: i32,
    pub buf_len: i32,
    pub var_bufs: [VarBuf; MAX_BUFS],
}

impl Header {
    pub fn parse(bytes: &[u8; HEADER_SIZE]) -> Self {
        let mut var_bufs = [VarBuf::default(); MAX_BUFS];
        for (i, buf) in var_bufs.iter_mut().enumerate() {
            let base = tick_count_offset(i);
            buf.tick_count = i32_at(bytes, base);
            buf.buf_offset = i32_at(bytes, base + 4);
        }
        Self {
            ver: i32_at(bytes, 0),
            status: i32_at(bytes, 4),
            tick_rate: i32_at(bytes, 8),
            session_info_update: i32_at(bytes, 12),
            session_info_len: i32_at(bytes, 16),
            session_info_offset: i32_at(bytes, 20),
            num_vars: i32_at(bytes, 24),
            var_header_offset: i32_at(bytes, 28),
            num_buf: i32_at(bytes, 32),
            buf_len: i32_at(bytes, 36),
            var_bufs,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.status & STATUS_CONNECTED != 0
    }

    /// Number of buffers in use, clamped to the valid range.
    pub fn buffer_count(&self) -> usize {
        self.num_buf.clamp(1, MAX_BUFS as i32) as usize
    }

    /// Index and entry of the buffer holding the newest tick.
    pub fn latest_buf(&self) -> (usize, VarBuf) {
        let mut best = 0;
        for i in 1..self.buffer_count() {
            if self.var_bufs[i].tick_count > self.var_bufs[best].tick_count {
                best = i;
            }
        }
        (best, self.var_bufs[best])
    }
}

/// Variable data types (`irsdk_VarType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VarType {
    Char,
    Bool,
    Int,
    BitField,
    Float,
    Double,
}

impl VarType {
    pub fn from_raw(raw: i32) -> Option<Self> {
        Some(match raw {
            0 => Self::Char,
            1 => Self::Bool,
            2 => Self::Int,
            3 => Self::BitField,
            4 => Self::Float,
            5 => Self::Double,
            _ => return None,
        })
    }

    pub const fn size(self) -> usize {
        match self {
            Self::Char | Self::Bool => 1,
            Self::Int | Self::BitField | Self::Float => 4,
            Self::Double => 8,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Char => "char",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::BitField => "bitfield",
            Self::Float => "float",
            Self::Double => "double",
        }
    }
}

/// Raw `irsdk_varHeader` before validation against the buffer length.
#[derive(Clone, Debug, PartialEq)]
pub struct RawVarHeader {
    pub ty: i32,
    pub offset: i32,
    pub count: i32,
    pub count_as_time: bool,
    pub name: String,
    pub desc: String,
    pub unit: String,
}

impl RawVarHeader {
    pub fn parse(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() >= VAR_HEADER_SIZE);
        Self {
            ty: i32_at(bytes, 0),
            offset: i32_at(bytes, 4),
            count: i32_at(bytes, 8),
            count_as_time: bytes[12] != 0,
            name: c_string(&bytes[16..48]),
            desc: c_string(&bytes[48..112]),
            unit: c_string(&bytes[112..144]),
        }
    }
}

pub(crate) fn i32_at(bytes: &[u8], offset: usize) -> i32 {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[offset..offset + 4]);
    i32::from_le_bytes(raw)
}

/// Decodes a NUL-terminated Windows-1252 string, which is what iRacing writes.
fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes[..end]);
    text.into_owned()
}
