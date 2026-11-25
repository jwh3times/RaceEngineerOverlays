//! Reader for the iRacing SDK shared-memory telemetry interface.
//!
//! The sim publishes a header, a table of variable descriptions, a YAML
//! session-info string, and up to four rotating rows of telemetry. [`Client`]
//! copies the newest consistent row on each poll. Callers resolve typed
//! [`Var`] handles once per connection and read values from the copied row.
//!
//! Platform-specific mapping lives behind [`SharedMemory`], so parsing and
//! polling are tested against in-memory images on any OS.

mod client;
pub mod layout;
mod memory;
mod vars;
#[cfg(windows)]
mod windows;

pub use client::{Client, Frame, Update};
pub use layout::{Header, VarType};
pub use memory::{SharedBuffer, SharedMemory};
pub use vars::{Value, Var, VarHeader, VarTable, VarValue};
#[cfg(windows)]
pub use windows::{DataValidEvent, IRACING_EVENT_NAME, IRACING_MAPPING_NAME, MappedFile};

pub mod testing;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("shared memory too small to hold the {0}")]
    Truncated(&'static str),
    #[error("unsupported SDK header version {0}")]
    UnsupportedVersion(i32),
    #[error("invalid SDK header: {0}")]
    InvalidHeader(&'static str),
    #[error("the sim kept rewriting the {0} while it was being read")]
    Busy(&'static str),
}
