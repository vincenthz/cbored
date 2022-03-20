#[derive(Debug, Clone, Copy)]
pub enum Major {
    Positive,
    Negative,
    Bytes,
    Text,
    Array,
    Map,
    Tag,
    Other,
}

impl Major {
    pub fn from_byte(v: u8) -> Major {
        // only 3 bits
        match v >> 5 {
            0 => Major::Positive,
            1 => Major::Negative,
            2 => Major::Bytes,
            3 => Major::Text,
            4 => Major::Array,
            5 => Major::Map,
            6 => Major::Tag,
            7 => Major::Other,
            _ => unreachable!(),
        }
    }

    pub fn to_value(self) -> u8 {
        match self {
            Major::Positive => 0,
            Major::Negative => 1,
            Major::Bytes => 2,
            Major::Text => 3,
            Major::Array => 4,
            Major::Map => 5,
            Major::Tag => 6,
            Major::Other => 7,
        }
    }

    pub fn to_high_bits(self) -> u8 {
        self.to_value() << 5
    }
}

#[derive(Clone, Copy, Debug)]
pub enum IndirectLen {
    I1,
    I2,
    I4,
    I8,
}
impl IndirectLen {
    pub fn len_bytes(self) -> usize {
        match self {
            IndirectLen::I1 => 1,
            IndirectLen::I2 => 2,
            IndirectLen::I4 => 4,
            IndirectLen::I8 => 8,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum IndirectValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
}

// only fits 5 bits (not enforced)
#[allow(non_camel_case_types)]
pub type u5 = u8;

const CONTENT_MASK: u8 = 0b001_1111;

#[derive(Clone, Copy, Debug)]
pub enum Content {
    Imm(u5),
    Indirect(IndirectLen),
}

impl Content {
    fn from_byte(byte: u8) -> Result<Self, LeadError> {
        let b = byte & CONTENT_MASK;
        match b {
            0..=0x17 => Ok(Self::Imm(b)),
            0x18 => Ok(Self::Indirect(IndirectLen::I1)),
            0x19 => Ok(Self::Indirect(IndirectLen::I2)),
            0x1a => Ok(Self::Indirect(IndirectLen::I4)),
            0x1b => Ok(Self::Indirect(IndirectLen::I8)),
            0x1c..=0x1e => Err(LeadError::Reserved(byte)),
            0x1f => Err(LeadError::IndefiniteNotSupported(byte)),
            _ => unreachable!(),
        }
    }

    fn expected_extra(self) -> Option<IndirectLen> {
        match self {
            Self::Imm(_) => None,
            Self::Indirect(i) => Some(i),
        }
    }

    pub(crate) fn to_byte(self) -> u8 {
        match self {
            Content::Imm(v) => {
                assert!(v <= 0x17);
                v
            }
            Content::Indirect(IndirectLen::I1) => 0x18,
            Content::Indirect(IndirectLen::I2) => 0x19,
            Content::Indirect(IndirectLen::I4) => 0x1a,
            Content::Indirect(IndirectLen::I8) => 0x1b,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ContentStream {
    Imm(u5),
    Indirect(IndirectLen),
    Stream,
}

impl ContentStream {
    fn from_byte(byte: u8) -> Result<Self, LeadError> {
        let b = byte & CONTENT_MASK;
        match b {
            0..=0x17 => Ok(Self::Imm(b)),
            0x18 => Ok(Self::Indirect(IndirectLen::I1)),
            0x19 => Ok(Self::Indirect(IndirectLen::I2)),
            0x1a => Ok(Self::Indirect(IndirectLen::I4)),
            0x1b => Ok(Self::Indirect(IndirectLen::I8)),
            0x1c..=0x1e => Err(LeadError::Reserved(byte)),
            0x1f => Ok(Self::Stream),
            _ => unreachable!(),
        }
    }

    fn expected_extra(self) -> Option<IndirectLen> {
        match self {
            Self::Imm(_) => None,
            Self::Indirect(i) => Some(i),
            Self::Stream => None,
        }
    }

    pub(crate) fn to_byte(self) -> u8 {
        match self {
            ContentStream::Imm(v) => {
                assert!(v <= 0x17);
                v
            }
            ContentStream::Indirect(IndirectLen::I1) => 0x18,
            ContentStream::Indirect(IndirectLen::I2) => 0x19,
            ContentStream::Indirect(IndirectLen::I4) => 0x1a,
            ContentStream::Indirect(IndirectLen::I8) => 0x1b,
            ContentStream::Stream => 0x1f,
        }
    }
}

impl From<Option<Content>> for ContentStream {
    fn from(v: Option<Content>) -> ContentStream {
        match v {
            None => ContentStream::Stream,
            Some(Content::Imm(v)) => ContentStream::Imm(v),
            Some(Content::Indirect(ilen)) => ContentStream::Indirect(ilen),
        }
    }
}

pub enum Lead {
    /// Positive Integer
    Positive(Content),
    /// Negative Integer
    Negative(Content),
    /// Sequence of Bytes
    Bytes(ContentStream),
    /// Sequence of Chars using UTF8 encoding
    Text(ContentStream),
    /// Array of elements
    Array(ContentStream),
    /// Map of elements
    Map(ContentStream),
    /// Tag
    Tag(Content),
    /// a value from 0 to 0x13 (included)
    ByteImm(u5),
    /// False constant
    False,
    /// True constant
    True,
    Null,
    Undefined,
    ByteI1,
    FP16,
    FP32,
    FP64,
    Break,
}

#[derive(Debug, Clone)]
pub enum LeadError {
    IndefiniteNotSupported(u8),
    Reserved(u8),
}

impl Lead {
    fn special(byte: u8) -> Result<Self, LeadError> {
        let b = byte & CONTENT_MASK;
        match b {
            0x0..=0x13 => Ok(Lead::ByteImm(b & CONTENT_MASK)),
            0x14 => Ok(Lead::False),
            0x15 => Ok(Lead::True),
            0x16 => Ok(Lead::Null),
            0x17 => Ok(Lead::Undefined),
            0x18 => Ok(Lead::ByteI1),
            0x19 => Ok(Lead::FP16),
            0x1a => Ok(Lead::FP32),
            0x1b => Ok(Lead::FP64),
            0x1c..=0x1e => Err(LeadError::Reserved(b)),
            0x1f => Ok(Lead::Break),
            _ => unreachable!(),
        }
    }

    pub fn from_byte(b: u8) -> Result<Lead, LeadError> {
        match Major::from_byte(b) {
            Major::Positive => Content::from_byte(b).map(Lead::Positive),
            Major::Negative => Content::from_byte(b).map(Lead::Negative),
            Major::Bytes => ContentStream::from_byte(b).map(Lead::Bytes),
            Major::Text => ContentStream::from_byte(b).map(Lead::Text),
            Major::Array => ContentStream::from_byte(b).map(Lead::Array),
            Major::Map => ContentStream::from_byte(b).map(Lead::Map),
            Major::Tag => Content::from_byte(b).map(Lead::Tag),
            Major::Other => Self::special(b),
        }
    }

    pub fn expected_extra(&self) -> Option<IndirectLen> {
        match self {
            Lead::Positive(c) => c.expected_extra(),
            Lead::Negative(c) => c.expected_extra(),
            Lead::Bytes(c) => c.expected_extra(),
            Lead::Text(c) => c.expected_extra(),
            Lead::Array(c) => c.expected_extra(),
            Lead::Map(c) => c.expected_extra(),
            Lead::Tag(c) => c.expected_extra(),
            Lead::ByteImm(_) => None,
            Lead::False => None,
            Lead::True => None,
            Lead::Null => None,
            Lead::Undefined => None,
            Lead::ByteI1 => Some(IndirectLen::I1),
            Lead::FP16 => Some(IndirectLen::I2),
            Lead::FP32 => Some(IndirectLen::I4),
            Lead::FP64 => Some(IndirectLen::I8),
            Lead::Break => None,
        }
    }
}
