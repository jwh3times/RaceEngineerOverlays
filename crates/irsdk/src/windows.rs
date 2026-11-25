//! Windows named shared-memory mapping and data-valid event.

use std::io;
use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Memory::{
    FILE_MAP_READ, MEMORY_BASIC_INFORMATION, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile,
    OpenFileMappingW, UnmapViewOfFile, VirtualQuery,
};
use windows::Win32::System::Threading::{
    OpenEventW, SYNCHRONIZATION_SYNCHRONIZE, WaitForSingleObject,
};
use windows::core::HSTRING;

use crate::memory::SharedMemory;

/// Object names used by the live sim.
pub const IRACING_MAPPING_NAME: &str = r"Local\IRSDKMemMapFileName";
pub const IRACING_EVENT_NAME: &str = r"Local\IRSDKDataValidEvent";

/// A read-only view of a named file mapping created by another process.
pub struct MappedFile {
    handle: HANDLE,
    view: MEMORY_MAPPED_VIEW_ADDRESS,
    len: usize,
}

// SAFETY: the handle and view are process-wide resources with no thread affinity,
// and `read_into` only performs copies out of the view.
unsafe impl Send for MappedFile {}
// SAFETY: as above; concurrent copies out of a read-only view are sound.
unsafe impl Sync for MappedFile {}

impl MappedFile {
    /// Opens an existing mapping. Fails with `NotFound` when the sim isn't running.
    pub fn open(name: &str) -> io::Result<Self> {
        let name = HSTRING::from(name);
        // SAFETY: `name` is a valid NUL-terminated wide string for the duration of the call.
        let handle =
            unsafe { OpenFileMappingW(FILE_MAP_READ.0, false, &name) }.map_err(io::Error::from)?;
        // SAFETY: `handle` is a valid mapping handle we own; mapping the whole object read-only.
        let view = unsafe { MapViewOfFile(handle, FILE_MAP_READ, 0, 0, 0) };
        if view.Value.is_null() {
            let err = io::Error::last_os_error();
            // SAFETY: closing the handle we just opened.
            unsafe { CloseHandle(handle) }.ok();
            return Err(err);
        }
        let mut info = MEMORY_BASIC_INFORMATION::default();
        // SAFETY: `view.Value` is the base of a live view and `info` is a valid out-parameter.
        let written = unsafe {
            VirtualQuery(
                Some(view.Value),
                &mut info,
                size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        let len = if written == 0 { 0 } else { info.RegionSize };
        Ok(Self { handle, view, len })
    }
}

impl SharedMemory for MappedFile {
    fn len(&self) -> usize {
        self.len
    }

    fn read_into(&self, offset: usize, dst: &mut [u8]) -> bool {
        match offset.checked_add(dst.len()) {
            Some(end) if end <= self.len => {
                // SAFETY: the range lies within the committed view, which stays mapped
                // for our lifetime. The writer is another process, so we copy raw bytes
                // and validate consistency (tick counts) after the copy.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        self.view.Value.cast::<u8>().add(offset),
                        dst.as_mut_ptr(),
                        dst.len(),
                    );
                }
                true
            }
            _ => false,
        }
    }
}

impl Drop for MappedFile {
    fn drop(&mut self) {
        // SAFETY: the view and handle were created in `open` and are released exactly once.
        unsafe {
            UnmapViewOfFile(self.view).ok();
            CloseHandle(self.handle).ok();
        }
    }
}

/// The event the sim signals each time it publishes a new row.
pub struct DataValidEvent(HANDLE);

// SAFETY: event handles have no thread affinity.
unsafe impl Send for DataValidEvent {}
// SAFETY: waiting on the same event from several threads is allowed.
unsafe impl Sync for DataValidEvent {}

impl DataValidEvent {
    pub fn open(name: &str) -> io::Result<Self> {
        let name = HSTRING::from(name);
        // SAFETY: `name` is a valid NUL-terminated wide string for the duration of the call.
        let handle = unsafe { OpenEventW(SYNCHRONIZATION_SYNCHRONIZE, false, &name) }
            .map_err(io::Error::from)?;
        Ok(Self(handle))
    }

    /// Waits until the sim signals new data or `timeout` passes. Returns `true` if signalled.
    pub fn wait(&self, timeout: Duration) -> bool {
        let ms = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
        // SAFETY: `self.0` is a valid event handle for our lifetime.
        unsafe { WaitForSingleObject(self.0, ms) == WAIT_OBJECT_0 }
    }
}

impl Drop for DataValidEvent {
    fn drop(&mut self) {
        // SAFETY: the handle was opened in `open` and is closed exactly once.
        unsafe { CloseHandle(self.0) }.ok();
    }
}
