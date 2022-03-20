use super::lead::{u5, Content, ContentStream, IndirectLen, IndirectValue, Lead};
use super::types::*;

#[derive(Clone, Copy, Debug)]
pub enum Value {
    Imm(u5),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
}

#[derive(Clone, Copy, Debug)]
pub enum Value8 {
    Imm(u5),
    U8(u8),
}

impl Value {
    pub fn to_size(self) -> usize {
        match self {
            Value::Imm(u) => u as usize,
            Value::U8(u) => u as usize,
            Value::U16(u) => u as usize,
            Value::U32(u) => u as usize,
            Value::U64(u) => u as usize,
        }
    }

    pub fn to_u64(self) -> u64 {
        match self {
            Value::Imm(u) => u as u64,
            Value::U8(u) => u as u64,
            Value::U16(u) => u as u64,
            Value::U32(u) => u as u64,
            Value::U64(u) => u,
        }
    }

    pub fn to_lead_content(self) -> Content {
        match self {
            Value::Imm(v) => Content::Imm(v),
            Value::U8(_) => Content::Indirect(IndirectLen::I1),
            Value::U16(_) => Content::Indirect(IndirectLen::I2),
            Value::U32(_) => Content::Indirect(IndirectLen::I4),
            Value::U64(_) => Content::Indirect(IndirectLen::I8),
        }
    }

    pub fn canonical(v: u64) -> Self {
        match v {
            _ if v < 24 => Self::Imm(v as u8),
            _ if v < 0x100 => Self::U8(v as u8),
            _ if v < 0x10000 => Self::U16(v as u16),
            _ if v < 0x100000000 => Self::U32(v as u32),
            _ => Self::U64(v),
        }
    }

    pub fn is_canonical(&self) -> bool {
        match self {
            // don't check if imm is < 24, as it shouldn't be allowed
            Value::Imm(_) => true,
            Value::U8(v) => *v >= 24,
            Value::U16(v) => *v >= 0x100,
            Value::U32(v) => *v >= 0x10000,
            Value::U64(v) => *v >= 0x100000000,
        }
    }
}

impl From<IndirectValue> for Value {
    fn from(v: IndirectValue) -> Value {
        match v {
            IndirectValue::U8(v) => Value::U8(v),
            IndirectValue::U16(v) => Value::U16(v),
            IndirectValue::U32(v) => Value::U32(v),
            IndirectValue::U64(v) => Value::U64(v),
        }
    }
}

pub type ValueStream = Option<Value>;

// resolve the value
pub(crate) fn resolve_value(con: Content, ival: Option<IndirectValue>) -> Value {
    match (con, ival) {
        (Content::Imm(imm), None) => Value::Imm(imm),
        (Content::Indirect(_), Some(value)) => value.into(),
        (_, _) => panic!("internal error"),
    }
}

// resolve the value with streamable
pub(crate) fn resolve_value_stream(con: ContentStream, ival: Option<IndirectValue>) -> ValueStream {
    match (con, ival) {
        (ContentStream::Stream, _) => None,
        (ContentStream::Imm(v), _) => Some(Value::U8(v)),
        (ContentStream::Indirect(_), Some(val)) => Some(val.into()),
        _ => panic!("internal error"),
    }
}

#[derive(Clone, Debug)]
pub enum Header {
    Positive(Positive),
    Negative(Negative),
    Bytes(ValueStream),
    Text(ValueStream),
    Array(ValueStream),
    Map(ValueStream),
    Tag(Value),
    Constant(Constant),
    Float(Float),
    Byte(Byte),
    Break,
}

impl Header {
    /// get the type of next element
    pub fn to_type(&self) -> Type {
        match self {
            Header::Positive(_) => Type::Positive,
            Header::Negative(_) => Type::Negative,
            Header::Bytes(_) => Type::Bytes,
            Header::Text(_) => Type::Text,
            Header::Array(_) => Type::Array,
            Header::Map(_) => Type::Map,
            Header::Tag(_) => Type::Tag,
            Header::Constant(Constant::True) => Type::True,
            Header::Constant(Constant::False) => Type::False,
            Header::Constant(Constant::Null) => Type::Null,
            Header::Constant(Constant::Undefined) => Type::Undefined,
            Header::Float(Float::FP16(_)) => Type::Float,
            Header::Float(Float::FP32(_)) => Type::Float,
            Header::Float(Float::FP64(_)) => Type::Float,
            Header::Byte(_) => Type::Byte,
            Header::Break => Type::Break,
        }
    }

    pub fn from_parts(ld: Lead, ival: Option<IndirectValue>) -> Self {
        fn other_payload(val: Option<IndirectValue>) -> Header {
            match val {
                Some(IndirectValue::U8(v)) => Header::Byte(Byte(Value8::U8(v))),
                Some(IndirectValue::U16(v)) => Header::Float(Float::FP16(v)),
                Some(IndirectValue::U32(v)) => Header::Float(Float::FP32(v)),
                Some(IndirectValue::U64(v)) => Header::Float(Float::FP64(v)),
                None => panic!("internal error"),
            }
        }
        match ld {
            Lead::Positive(c) => Header::Positive(Positive(resolve_value(c, ival))),
            Lead::Negative(c) => Header::Negative(Negative(resolve_value(c, ival))),
            Lead::Bytes(c) => Header::Bytes(resolve_value_stream(c, ival)),
            Lead::Text(c) => Header::Text(resolve_value_stream(c, ival)),
            Lead::Array(c) => Header::Array(resolve_value_stream(c, ival)),
            Lead::Map(c) => Header::Map(resolve_value_stream(c, ival)),
            Lead::Tag(c) => Header::Tag(resolve_value(c, ival)),
            Lead::ByteImm(v) => Header::Byte(Byte(Value8::Imm(v))),
            Lead::False => Header::Constant(Constant::False),
            Lead::True => Header::Constant(Constant::True),
            Lead::Null => Header::Constant(Constant::Null),
            Lead::Undefined => Header::Constant(Constant::Undefined),
            Lead::ByteI1 => other_payload(ival),
            Lead::FP16 => other_payload(ival),
            Lead::FP32 => other_payload(ival),
            Lead::FP64 => other_payload(ival),
            Lead::Break => Header::Break,
        }
    }
}

impl Header {}
