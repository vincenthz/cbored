use super::super::header::{Value, Value8};

/// CBOR Positive value
#[derive(Debug, Clone, Copy)]
pub struct Positive(pub(crate) Value);

/// CBOR Negative value
#[derive(Debug, Clone, Copy)]
pub struct Negative(pub(crate) Value);

/// CBOR Byte value
#[derive(Debug, Clone, Copy)]
pub struct Byte(pub(crate) Value8);

/// CBOR constants (False/True/Null/Undefined)
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum Constant {
    False,
    True,
    Null,
    Undefined,
}

impl Positive {
    pub fn to_u64(&self) -> u64 {
        self.0.to_u64()
    }

    pub fn canonical(v: u64) -> Self {
        match v {
            _ if v < 24 => Self(Value::Imm(v as u8)),
            _ if v < 0x100 => Self(Value::U8(v as u8)),
            _ if v < 0x10000 => Self(Value::U16(v as u16)),
            _ if v < 0x100000000 => Self(Value::U32(v as u32)),
            _ => Self(Value::U64(v)),
        }
    }

    pub fn is_canonical(&self) -> bool {
        match self.0 {
            // don't check if imm is < 24, as it shouldn't be allowed
            Value::Imm(_) => true,
            Value::U8(v) => v >= 24,
            Value::U16(v) => v >= 0x100,
            Value::U32(v) => v >= 0x10000,
            Value::U64(v) => v >= 0x100000000,
        }
    }
}
