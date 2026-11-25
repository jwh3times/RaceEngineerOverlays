//! Builds synthetic shared-memory images in the SDK layout, for tests and tools
//! that need telemetry without a running sim.

use std::collections::HashMap;

use crate::layout::{
    HEADER_SIZE, MAX_BUFS, STATUS_CONNECTED, SUPPORTED_VERSION, VAR_HEADER_SIZE, VarType,
    tick_count_offset,
};

/// Values that can be written into a synthetic row.
pub trait Encode: Copy {
    fn encode(self, out: &mut [u8]);
}

macro_rules! encode_le {
    ($($t:ty),*) => {$(
        impl Encode for $t {
            fn encode(self, out: &mut [u8]) {
                let bytes = self.to_le_bytes();
                out[..bytes.len()].copy_from_slice(&bytes);
            }
        }
    )*};
}
encode_le!(f32, f64, i32, u32, u8);

impl Encode for bool {
    fn encode(self, out: &mut [u8]) {
        out[0] = u8::from(self);
    }
}

struct VarSpec {
    name: String,
    ty: VarType,
    count: usize,
    unit: String,
}

/// Declares the variables and buffers of a synthetic image.
pub struct ImageBuilder {
    vars: Vec<VarSpec>,
    num_buf: usize,
    session_capacity: usize,
    tick_rate: i32,
}

impl Default for ImageBuilder {
    fn default() -> Self {
        Self {
            vars: Vec::new(),
            num_buf: 3,
            session_capacity: 64 * 1024,
            tick_rate: 60,
        }
    }
}

impl ImageBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn var(mut self, name: &str, ty: VarType, count: usize) -> Self {
        self.vars.push(VarSpec {
            name: name.to_owned(),
            ty,
            count,
            unit: String::new(),
        });
        self
    }

    pub fn var_with_unit(mut self, name: &str, ty: VarType, count: usize, unit: &str) -> Self {
        self = self.var(name, ty, count);
        if let Some(last) = self.vars.last_mut() {
            last.unit = unit.to_owned();
        }
        self
    }

    pub fn buffers(mut self, n: usize) -> Self {
        self.num_buf = n.clamp(1, MAX_BUFS);
        self
    }

    pub fn session_capacity(mut self, bytes: usize) -> Self {
        self.session_capacity = bytes;
        self
    }

    pub fn build(self) -> Image {
        let var_header_offset = HEADER_SIZE;
        let session_offset = align16(var_header_offset + self.vars.len() * VAR_HEADER_SIZE);
        let first_buf = align16(session_offset + self.session_capacity);

        let mut offsets = HashMap::new();
        let mut row_len = 0usize;
        for v in &self.vars {
            let size = v.ty.size();
            row_len = row_len.div_ceil(size) * size;
            offsets.insert(v.name.clone(), (row_len, v.ty, v.count));
            row_len += size * v.count;
        }
        let row_len = align16(row_len.max(16));

        let mut bytes = vec![0u8; first_buf + row_len * self.num_buf];
        let put = |bytes: &mut Vec<u8>, at: usize, v: i32| {
            bytes[at..at + 4].copy_from_slice(&v.to_le_bytes())
        };
        put(&mut bytes, 0, SUPPORTED_VERSION);
        put(&mut bytes, 4, STATUS_CONNECTED);
        put(&mut bytes, 8, self.tick_rate);
        put(&mut bytes, 20, session_offset as i32);
        put(&mut bytes, 24, self.vars.len() as i32);
        put(&mut bytes, 28, var_header_offset as i32);
        put(&mut bytes, 32, self.num_buf as i32);
        put(&mut bytes, 36, row_len as i32);
        for i in 0..self.num_buf {
            put(&mut bytes, tick_count_offset(i), -1);
            put(
                &mut bytes,
                tick_count_offset(i) + 4,
                (first_buf + i * row_len) as i32,
            );
        }

        for (i, v) in self.vars.iter().enumerate() {
            let at = var_header_offset + i * VAR_HEADER_SIZE;
            let (offset, ty, count) = offsets[&v.name];
            put(&mut bytes, at, ty_raw(ty));
            put(&mut bytes, at + 4, offset as i32);
            put(&mut bytes, at + 8, count as i32);
            write_c_str(&mut bytes[at + 16..at + 48], &v.name);
            write_c_str(&mut bytes[at + 112..at + 144], &v.unit);
        }

        Image {
            bytes,
            offsets,
            session_offset,
            session_capacity: self.session_capacity,
            first_buf,
            row_len,
            num_buf: self.num_buf,
        }
    }
}

/// A synthetic shared-memory image. `bytes` implements [`crate::SharedMemory`].
pub struct Image {
    pub bytes: Vec<u8>,
    offsets: HashMap<String, (usize, VarType, usize)>,
    session_offset: usize,
    session_capacity: usize,
    first_buf: usize,
    row_len: usize,
    num_buf: usize,
}

impl Image {
    pub fn row_len(&self) -> usize {
        self.row_len
    }

    /// Writes element `index` of variable `name` into rotating buffer `buf`.
    ///
    /// # Panics
    /// If the variable is unknown, `index` is out of range or `value` is the wrong size.
    pub fn set<T: Encode>(&mut self, buf: usize, name: &str, index: usize, value: T) {
        let (offset, ty, count) = self.offsets[name];
        assert!(index < count, "{name}[{index}] out of range");
        assert_eq!(size_of::<T>(), ty.size(), "{name} is {}", ty.name());
        let at = self.first_buf + buf * self.row_len + offset + index * ty.size();
        value.encode(&mut self.bytes[at..at + ty.size()]);
    }

    pub fn set_tick(&mut self, buf: usize, tick: i32) {
        assert!(buf < self.num_buf);
        let at = tick_count_offset(buf);
        self.bytes[at..at + 4].copy_from_slice(&tick.to_le_bytes());
    }

    pub fn set_connected(&mut self, connected: bool) {
        let status = if connected { STATUS_CONNECTED } else { 0 };
        self.bytes[4..8].copy_from_slice(&status.to_le_bytes());
    }

    /// Replaces the session-info text (encoded as Windows-1252) and sets its revision.
    pub fn set_session_info(&mut self, yaml: &str, revision: i32) {
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(yaml);
        assert!(
            encoded.len() < self.session_capacity,
            "session info exceeds capacity"
        );
        let region =
            &mut self.bytes[self.session_offset..self.session_offset + self.session_capacity];
        region.fill(0);
        region[..encoded.len()].copy_from_slice(&encoded);
        self.bytes[12..16].copy_from_slice(&revision.to_le_bytes());
        self.bytes[16..20].copy_from_slice(&(self.session_capacity as i32).to_le_bytes());
    }
}

fn align16(n: usize) -> usize {
    n.div_ceil(16) * 16
}

fn ty_raw(ty: VarType) -> i32 {
    match ty {
        VarType::Char => 0,
        VarType::Bool => 1,
        VarType::Int => 2,
        VarType::BitField => 3,
        VarType::Float => 4,
        VarType::Double => 5,
    }
}

fn write_c_str(dst: &mut [u8], s: &str) {
    let n = s.len().min(dst.len() - 1);
    dst[..n].copy_from_slice(&s.as_bytes()[..n]);
}
