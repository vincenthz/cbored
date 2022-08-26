use super::prim::CborDataOf;
use super::reader::{Reader, ReaderError};
use super::types::{DataOwned, Scalar};
use std::borrow::Cow;
use std::fmt;

/// Possible errors when decoding an element
#[derive(Debug, Clone)]
pub enum DecodeErrorKind {
    /// Underlying reader has an error
    ReaderError(ReaderError),
    /// Reader has some trailing data, when trying to decode an element
    ReaderNotTerminated { remaining_bytes: usize },
    /// Underlying conversion is out of range, it gives the u64 values that was attempted to
    /// be converted, and the range that was expected by the conversion
    OutOfRange { min: u64, max: u64, got: u64 },
    /// Unexpected length whilst decoding type
    UnexpectedLength { expected: usize, got: usize },
    /// A custom error for the decoder
    Custom(String),
}

impl DecodeErrorKind {
    pub fn context<T: ?Sized>(self) -> DecodeError {
        DecodeError::new::<T>(self)
    }
    pub fn context_str(self, s: &'static str) -> DecodeError {
        DecodeError::new_str(s, self)
    }
}

impl std::error::Error for DecodeErrorKind {}

impl fmt::Display for DecodeErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl From<ReaderError> for DecodeErrorKind {
    fn from(r: ReaderError) -> DecodeErrorKind {
        DecodeErrorKind::ReaderError(r)
    }
}

/// Possible decode error when decoding an element
#[derive(Debug, Clone)]
pub struct DecodeError {
    context: Vec<Cow<'static, str>>,
    error: DecodeErrorKind,
}

impl DecodeError {
    pub fn new<T: ?Sized>(e: DecodeErrorKind) -> Self {
        DecodeError {
            context: vec![Cow::Borrowed(std::any::type_name::<T>())],
            error: e,
        }
    }

    pub fn new_str(ctx: &'static str, e: DecodeErrorKind) -> Self {
        DecodeError {
            context: vec![Cow::Borrowed(ctx)],
            error: e,
        }
    }

    pub fn new_string(ctx: String, e: DecodeErrorKind) -> Self {
        DecodeError {
            context: vec![Cow::Owned(ctx)],
            error: e,
        }
    }

    pub fn push<T: ?Sized>(mut self) -> Self {
        self.context.push(Cow::Borrowed(std::any::type_name::<T>()));
        self
    }

    pub fn push_str(mut self, s: &'static str) -> Self {
        self.context.push(Cow::Borrowed(s));
        self
    }

    pub fn push_string(mut self, s: String) -> Self {
        self.context.push(Cow::Owned(s));
        self
    }

    /// Return the error
    pub fn error(&self) -> &DecodeErrorKind {
        &self.error
    }

    /// Return the context from the innermost context, to the outer ones
    pub fn context(&self) -> &[Cow<'static, str>] {
        &self.context
    }

    pub fn context_as_path(&self) -> String {
        let mut s = String::new();
        for ctx in self.context.iter().rev() {
            if !s.is_empty() {
                s.push_str("->");
            }
            s.push_str(ctx);
        }
        s
    }
}

impl std::error::Error for DecodeError {}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}: {}", self.context_as_path(), self.error)
    }
}

/// Generic Decode trait to read an element T from the CBOR reader
pub trait Decode: Sized {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError>;
}

/// Decode zero to many Ts in an array
///
/// this is identical to Array::to_vec, but has better error reporting
/// and just assume that inner element use the decode implementation for
/// the T type.
pub fn decode_vec<'a, T: Decode>(reader: &mut Reader<'a>) -> Result<Vec<T>, DecodeError> {
    let a = reader
        .array()
        .map_err(DecodeErrorKind::ReaderError)
        .map_err(|e| e.context_str("Vec"))?;
    let mut out = Vec::with_capacity(a.len());
    for (i, mut inner_reader) in a.iter().enumerate() {
        let v = <T>::decode(&mut inner_reader)
            .map_err(|e| e.push_string(format!("{}", i)).push_str("Vec"))?;
        out.push(v)
    }
    Ok(out)
}

macro_rules! assert_range {
    ($got:ident <= $max:literal) => {
        if $got > $max {
            return Err(DecodeError::new::<Self>(DecodeErrorKind::OutOfRange {
                got: $got,
                min: 0,
                max: $max,
            }));
        }
    };
    ($min:literal <= $got:ident <= $max:literal) => {
        if !($got >= $min && $got <= $max) {
            return Err(DecodeError::leaf_static::<Self>(DecodeError::Leaf(
                DecodeErrorKind::OutOfRange {
                    got: $got,
                    min: $min,
                    max: $max,
                },
            )));
        }
    };
}

impl Decode for DataOwned {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let data = reader
            .data()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<Self>())?;
        Ok(data.owned())
    }
}

impl Decode for bool {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        reader
            .bool()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<Self>())
    }
}

impl Decode for u8 {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let pos = reader
            .positive()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<Self>())?;
        let val = pos.to_u64();
        assert_range!(val <= 255);
        Ok(val as u8)
    }
}

impl Decode for u16 {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let pos = reader
            .positive()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<Self>())?;
        let val = pos.to_u64();
        assert_range!(val <= 65535);
        Ok(val as u16)
    }
}

impl Decode for u32 {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let pos = reader
            .positive()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<Self>())?;
        let val = pos.to_u64();
        assert_range!(val <= 0xffff_ffff);
        Ok(val as u32)
    }
}

impl Decode for u64 {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let pos = reader
            .positive()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<Self>())?;
        Ok(pos.to_u64())
    }
}

impl Decode for String {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let t = reader
            .text()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<Self>())?;
        Ok(t.to_string())
    }
}

impl<const N: usize> Decode for [u8; N] {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let bytes = reader
            .bytes()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<Self>())?;
        if bytes.len() == N {
            let mut output = [0u8; N];
            // optimise to not do a to_vec() here
            output.copy_from_slice(&bytes.to_vec());
            Ok(output)
        } else {
            Err(DecodeErrorKind::UnexpectedLength {
                expected: N,
                got: bytes.len(),
            }
            .context::<Self>())
        }
    }
}

impl Decode for Vec<u8> {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let t = reader
            .bytes()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<Self>())?;
        Ok(t.to_vec())
    }
}

impl Decode for Scalar {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        reader
            .scalar()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<Self>())
    }
}

impl<T: Decode> Decode for CborDataOf<T> {
    fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        reader.exact_decodable_data()
    }
}
