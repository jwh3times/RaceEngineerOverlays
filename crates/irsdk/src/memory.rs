//! Access to the shared-memory region. iRacing writes to it from another process
//! while we read, so every read is a copy into memory we own; nothing hands out
//! references into the mapping.

/// A readable view of an iRacing-style shared-memory region.
pub trait SharedMemory {
    /// Size of the readable region in bytes.
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Copies `dst.len()` bytes starting at `offset` into `dst`.
    /// Returns `false` without copying if the range is out of bounds.
    fn read_into(&self, offset: usize, dst: &mut [u8]) -> bool;

    fn read_i32(&self, offset: usize) -> Option<i32> {
        let mut raw = [0u8; 4];
        self.read_into(offset, &mut raw)
            .then(|| i32::from_le_bytes(raw))
    }
}

/// An in-process byte buffer, used by tests, tape playback and tools.
impl SharedMemory for Vec<u8> {
    fn len(&self) -> usize {
        self.as_slice().len()
    }

    fn read_into(&self, offset: usize, dst: &mut [u8]) -> bool {
        match offset.checked_add(dst.len()) {
            Some(end) if end <= self.as_slice().len() => {
                dst.copy_from_slice(&self[offset..end]);
                true
            }
            _ => false,
        }
    }
}

/// A cloneable, thread-safe in-process image, for tape playback and fake sims
/// where one side writes and a [`crate::Client`] reads.
#[derive(Clone, Debug, Default)]
pub struct SharedBuffer(std::sync::Arc<std::sync::RwLock<Vec<u8>>>);

impl SharedBuffer {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(std::sync::Arc::new(std::sync::RwLock::new(bytes)))
    }

    /// Copies `data` to `offset`. Returns `false` without writing if out of bounds.
    pub fn write(&self, offset: usize, data: &[u8]) -> bool {
        let mut bytes = self.0.write().unwrap_or_else(|e| e.into_inner());
        match offset.checked_add(data.len()) {
            Some(end) if end <= bytes.len() => {
                bytes[offset..end].copy_from_slice(data);
                true
            }
            _ => false,
        }
    }

    pub fn write_i32(&self, offset: usize, value: i32) -> bool {
        self.write(offset, &value.to_le_bytes())
    }
}

impl SharedMemory for SharedBuffer {
    fn len(&self) -> usize {
        self.0.read().unwrap_or_else(|e| e.into_inner()).len()
    }

    fn read_into(&self, offset: usize, dst: &mut [u8]) -> bool {
        self.0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .read_into(offset, dst)
    }
}

impl<M: SharedMemory + ?Sized> SharedMemory for Box<M> {
    fn len(&self) -> usize {
        (**self).len()
    }

    fn read_into(&self, offset: usize, dst: &mut [u8]) -> bool {
        (**self).read_into(offset, dst)
    }
}
