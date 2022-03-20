use super::context::CborDataOf;
use super::reader::{Reader, ReaderError};
use super::types::{Data, DataOwned};
use std::fmt;

/// Possible errors when decoding an element
#[derive(Debug, Clone)]
pub enum DecodeError {
    /// Underlying reader has an error
    ReaderError(ReaderError),
    /// Reader has some trailing data, when trying to decode an element
    ReaderNotTerminated { remaining_bytes: usize },
    /// Underlying conversion is out of range, it gives the u64 values that was attempted to
    /// be converted, and the range that was expected by the conversion
    OutOfRange { min: u64, max: u64, got: u64 },
    /// A custom error for the decoder
    Custom(String),
}

impl std::error::Error for DecodeError {}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl From<ReaderError> for DecodeError {
    fn from(r: ReaderError) -> DecodeError {
        DecodeError::ReaderError(r)
    }
}

/// Generic Decode trait to read an element T from the CBOR reader
pub trait Decode: Sized {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError>;
}

macro_rules! assert_range {
    ($got:ident <= $max:literal) => {
        if $got > $max {
            return Err(DecodeError::OutOfRange {
                got: $got,
                min: 0,
                max: $max,
            });
        }
    };
    ($min:literal <= $got:ident <= $max:literal) => {
        if !($got >= $min && $got <= $max) {
            return Err(DecodeError::OutOfRange {
                got: $got,
                min: $min,
                max: $max,
            });
        }
    };
}

impl<'b> Decode for Data<'b> {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        reader.decode()
    }
}

impl Decode for DataOwned {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        reader.decode().to_owned()
    }
}

impl Decode for u8 {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let pos = reader.positive()?;
        let val = pos.to_u64();
        assert_range!(val <= 255);
        Ok(val as u8)
    }
}

impl Decode for u16 {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let pos = reader.positive()?;
        let val = pos.to_u64();
        assert_range!(val <= 65535);
        Ok(val as u16)
    }
}

impl Decode for u32 {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let pos = reader.positive()?;
        let val = pos.to_u64();
        assert_range!(val <= 0xffff_ffff);
        Ok(val as u32)
    }
}

impl Decode for u64 {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let pos = reader.positive()?;
        Ok(pos.to_u64())
    }
}

impl Decode for String {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let t = reader.text()?;
        Ok(t.to_string())
    }
}

impl Decode for Vec<u8> {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let t = reader.bytes()?;
        Ok(t.to_vec())
    }
}

impl<T: Decode + 'static> Decode for CborDataOf<T> {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        reader.decodable_slice().map(|slice| slice.to_owned())
    }
}
