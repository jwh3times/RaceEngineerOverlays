//! Variable table and typed, pre-resolved handles into a telemetry row.

use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;

use crate::layout::{RawVarHeader, VarType};

/// A validated variable description.
#[derive(Clone, Debug, PartialEq)]
pub struct VarHeader {
    pub name: String,
    pub desc: String,
    pub unit: String,
    pub ty: VarType,
    /// Byte offset of the first element within a row.
    pub offset: usize,
    /// Number of elements (1 for scalars).
    pub count: usize,
    pub count_as_time: bool,
}

/// All variables published by the sim for the current connection.
#[derive(Clone, Debug, Default)]
pub struct VarTable {
    headers: Vec<VarHeader>,
    by_name: HashMap<String, usize>,
    /// Variables dropped because their type was unknown or they overran the row.
    pub rejected: Vec<String>,
}

impl VarTable {
    /// Builds a table, dropping any variable that would read outside a row of `row_len` bytes.
    pub fn from_raw(raw: Vec<RawVarHeader>, row_len: usize) -> Self {
        let mut table = Self::default();
        for var in raw {
            let fits = VarType::from_raw(var.ty).and_then(|ty| {
                let offset = usize::try_from(var.offset).ok()?;
                let count = usize::try_from(var.count).ok().filter(|&c| c > 0)?;
                let end = offset.checked_add(ty.size().checked_mul(count)?)?;
                (end <= row_len).then_some((ty, offset, count))
            });
            match fits {
                Some((ty, offset, count)) => {
                    table.by_name.insert(var.name.clone(), table.headers.len());
                    table.headers.push(VarHeader {
                        name: var.name,
                        desc: var.desc,
                        unit: var.unit,
                        ty,
                        offset,
                        count,
                        count_as_time: var.count_as_time,
                    });
                }
                None => table.rejected.push(var.name),
            }
        }
        table
    }

    pub fn len(&self) -> usize {
        self.headers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &VarHeader> {
        self.headers.iter()
    }

    pub fn get(&self, name: &str) -> Option<&VarHeader> {
        self.by_name.get(name).map(|&i| &self.headers[i])
    }

    /// Resolves a typed handle. Returns `None` if the variable is missing or its
    /// type cannot be read as `T`.
    pub fn handle<T: VarValue>(&self, name: &str) -> Option<Var<T>> {
        let header = self.get(name)?;
        T::ACCEPTS.contains(&header.ty).then_some(Var {
            offset: header.offset,
            count: header.count,
            ty: header.ty,
            _marker: PhantomData,
        })
    }

    /// Reads element `index` of `header` from `row` as a dynamically typed value.
    pub fn value(header: &VarHeader, row: &[u8], index: usize) -> Option<Value> {
        if index >= header.count {
            return None;
        }
        let at = header.offset + index * header.ty.size();
        let bytes = row.get(at..at + header.ty.size())?;
        Some(match header.ty {
            VarType::Char => Value::Char(bytes[0]),
            VarType::Bool => Value::Bool(bytes[0] != 0),
            VarType::Int => Value::Int(i32::decode(bytes, header.ty)),
            VarType::BitField => Value::BitField(u32::decode(bytes, header.ty)),
            VarType::Float => Value::Float(f32::decode(bytes, header.ty)),
            VarType::Double => Value::Double(f64::decode(bytes, header.ty)),
        })
    }
}

/// A dynamically typed variable value, for tools and diagnostics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Char(u8),
    Bool(bool),
    Int(i32),
    BitField(u32),
    Float(f32),
    Double(f64),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Char(v) => write!(f, "{v}"),
            Self::Bool(v) => write!(f, "{v}"),
            Self::Int(v) => write!(f, "{v}"),
            Self::BitField(v) => write!(f, "0x{v:08x}"),
            Self::Float(v) => write!(f, "{v:.4}"),
            Self::Double(v) => write!(f, "{v:.4}"),
        }
    }
}

/// Rust types a variable can be read as.
pub trait VarValue: Copy {
    /// SDK types that can be read as `Self`.
    const ACCEPTS: &'static [VarType];
    /// Decodes one element. `bytes.len()` equals `ty.size()`.
    fn decode(bytes: &[u8], ty: VarType) -> Self;
}

fn le4(bytes: &[u8]) -> [u8; 4] {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[..4]);
    raw
}

impl VarValue for f32 {
    const ACCEPTS: &'static [VarType] = &[VarType::Float];
    fn decode(bytes: &[u8], _: VarType) -> Self {
        f32::from_le_bytes(le4(bytes))
    }
}

impl VarValue for f64 {
    const ACCEPTS: &'static [VarType] = &[VarType::Double, VarType::Float];
    fn decode(bytes: &[u8], ty: VarType) -> Self {
        if ty == VarType::Float {
            return f64::from(f32::from_le_bytes(le4(bytes)));
        }
        let mut raw = [0u8; 8];
        raw.copy_from_slice(&bytes[..8]);
        f64::from_le_bytes(raw)
    }
}

impl VarValue for i32 {
    const ACCEPTS: &'static [VarType] = &[VarType::Int, VarType::BitField];
    fn decode(bytes: &[u8], _: VarType) -> Self {
        i32::from_le_bytes(le4(bytes))
    }
}

impl VarValue for u32 {
    const ACCEPTS: &'static [VarType] = &[VarType::BitField, VarType::Int];
    fn decode(bytes: &[u8], _: VarType) -> Self {
        u32::from_le_bytes(le4(bytes))
    }
}

impl VarValue for bool {
    const ACCEPTS: &'static [VarType] = &[VarType::Bool];
    fn decode(bytes: &[u8], _: VarType) -> Self {
        bytes[0] != 0
    }
}

impl VarValue for u8 {
    const ACCEPTS: &'static [VarType] = &[VarType::Char];
    fn decode(bytes: &[u8], _: VarType) -> Self {
        bytes[0]
    }
}

/// A typed handle to one variable, resolved once per connection.
///
/// Handles are only valid for rows produced under the same [`VarTable`]; re-resolve
/// them when the client reports that the variable layout changed.
#[derive(Clone, Copy, Debug)]
pub struct Var<T> {
    offset: usize,
    count: usize,
    ty: VarType,
    _marker: PhantomData<T>,
}

impl<T: VarValue> Var<T> {
    pub fn count(&self) -> usize {
        self.count
    }

    /// First element. `None` only if `row` is shorter than the layout says.
    pub fn get(&self, row: &[u8]) -> Option<T> {
        self.at(row, 0)
    }

    pub fn at(&self, row: &[u8], index: usize) -> Option<T> {
        if index >= self.count {
            return None;
        }
        let size = self.ty.size();
        let start = self.offset + index * size;
        row.get(start..start + size).map(|b| T::decode(b, self.ty))
    }

    /// Iterates over every element that fits in `row`.
    pub fn iter<'a>(&self, row: &'a [u8]) -> impl Iterator<Item = T> + 'a
    where
        T: 'a,
    {
        let var = *self;
        (0..self.count).map_while(move |i| var.at(row, i))
    }
}
