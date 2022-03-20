use super::lead::Lead;

mod float;
mod scalar;
mod streamable;
mod structure;

pub use float::Float;
pub use scalar::*;
pub use streamable::*;
pub use structure::*;

/// One of CBOR possible type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    /// CBOR Positive number between `0` to `2^64-1`
    Positive,
    /// CBOR Negative number representing between `-1 - 0` to `-1 - 2^64-1`
    Negative,
    /// CBOR Byte stream (sequence of 0 to many bytes)
    Bytes,
    /// CBOR Text (sequence of 0 to many utf8 encoded character)
    Text,
    /// CBOR Array (sequence of 0 to many CBOR elements)
    Array,
    /// CBOR Map (sequence of 0 to many tuple of CBOR key/value elements)
    Map,
    /// CBOR Tag (unsigned integer followed by a CBOR element)
    Tag,
    /// CBOR False constant
    False,
    /// CBOR True constant
    True,
    /// CBOR Null constant
    Null,
    /// CBOR Undefined constant
    Undefined,
    /// CBOR float (fp16/half precision, fp32/normal precision, fp64/double precision)
    Float,
    /// CBOR Byte (isomorphic to a u8)
    Byte,
    /// CBOR Break (not an element, just marking the end of a indefinite array, map, bytes, text)
    Break,
}

impl Type {
    pub fn from_lead(ld: Lead) -> Type {
        match ld {
            Lead::Positive(_) => Type::Positive,
            Lead::Negative(_) => Type::Negative,
            Lead::Bytes(_) => Type::Bytes,
            Lead::Text(_) => Type::Text,
            Lead::Array(_) => Type::Array,
            Lead::Map(_) => Type::Map,
            Lead::Tag(_) => Type::Tag,
            Lead::ByteImm(_) => Type::Byte,
            Lead::False => Type::False,
            Lead::True => Type::True,
            Lead::Null => Type::Null,
            Lead::Undefined => Type::Undefined,
            Lead::ByteI1 => Type::Byte,
            Lead::FP16 => Type::Float,
            Lead::FP32 => Type::Float,
            Lead::FP64 => Type::Float,
            Lead::Break => Type::Break,
        }
    }
}

/// One CBOR Data element with references to the data
#[derive(Debug, Clone)]
pub enum Data<'a> {
    Positive(Positive),
    Negative(Negative),
    Float(Float),
    Byte(Byte),
    Bytes(Bytes<'a>),
    Text(Text<'a>),
    Array(Array<'a>),
    Map(Map<'a>),
    Tag(Tag<'a>),
    True,
    False,
    Null,
    Undefined,
}

/// One CBOR Data element with owned data
#[derive(Debug, Clone)]
pub enum DataOwned {
    Positive(Positive),
    Negative(Negative),
    Float(Float),
    Byte(Byte),
    Bytes(BytesOwned),
    Text(TextOwned),
    Array(ArrayOwned),
    Map(MapOwned),
    Tag(TagOwned),
    True,
    False,
    Null,
    Undefined,
}

impl DataOwned {
    pub fn borrow<'a>(&'a self) -> Data<'a> {
        match self {
            DataOwned::Positive(v) => Data::Positive(*v),
            DataOwned::Negative(v) => Data::Negative(*v),
            DataOwned::Float(v) => Data::Float(*v),
            DataOwned::Byte(v) => Data::Byte(*v),
            DataOwned::Bytes(v) => Data::Bytes(v.borrow()),
            DataOwned::Text(v) => Data::Text(v.borrow()),
            DataOwned::Array(v) => Data::Array(v.borrow()),
            DataOwned::Map(v) => Data::Map(v.borrow()),
            DataOwned::Tag(v) => Data::Tag(v.borrow()),
            DataOwned::True => Data::True,
            DataOwned::False => Data::False,
            DataOwned::Null => Data::Null,
            DataOwned::Undefined => Data::Undefined,
        }
    }
}
