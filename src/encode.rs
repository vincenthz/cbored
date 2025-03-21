use super::header::HeaderValue;
use super::prim::{CborData, CborDataOf, CborSlice, CborSliceOf};
use super::types::*;
use super::writer::Writer;

/// Generic Encode trait to write an element T into the CBOR writer
pub trait Encode {
    fn encode(&self, writer: &mut Writer);
}

// *** CBOR types ***

impl Encode for Positive {
    fn encode(&self, writer: &mut Writer) {
        writer.positive(*self)
    }
}

impl Encode for Negative {
    fn encode(&self, writer: &mut Writer) {
        writer.negative(*self)
    }
}

impl Encode for Scalar {
    fn encode(&self, writer: &mut Writer) {
        writer.scalar(*self)
    }
}

impl Encode for Byte {
    fn encode(&self, writer: &mut Writer) {
        writer.byte(*self)
    }
}

impl Encode for BytesOwned {
    fn encode(&self, writer: &mut Writer) {
        writer.encode(&self.borrow())
    }
}

impl Encode for Constant {
    fn encode(&self, writer: &mut Writer) {
        writer.constant(*self)
    }
}

impl Encode for Float {
    fn encode(&self, writer: &mut Writer) {
        writer.float(*self)
    }
}

impl<'a> Encode for Bytes<'a> {
    fn encode(&self, writer: &mut Writer) {
        writer.bytes(self)
    }
}

impl<'a> Encode for Text<'a> {
    fn encode(&self, writer: &mut Writer) {
        writer.text(self)
    }
}

impl<'a> Encode for Array<'a> {
    fn encode(&self, writer: &mut Writer) {
        writer.array(self)
    }
}

impl<'a> Encode for Map<'a> {
    fn encode(&self, writer: &mut Writer) {
        writer.map(self)
    }
}

impl<'a> Encode for Tag<'a> {
    fn encode(&self, writer: &mut Writer) {
        writer.tag(self)
    }
}

impl<'a> Encode for Data<'a> {
    fn encode(&self, writer: &mut Writer) {
        writer.data(self)
    }
}

impl Encode for DataOwned {
    fn encode(&self, writer: &mut Writer) {
        writer.data(&self.borrow())
    }
}

// *** native rust types ***

impl Encode for bool {
    fn encode(&self, writer: &mut Writer) {
        writer.bool(*self)
    }
}

impl Encode for u8 {
    fn encode(&self, writer: &mut Writer) {
        writer.positive(Positive::canonical(*self as u64))
    }
}

impl Encode for u16 {
    fn encode(&self, writer: &mut Writer) {
        writer.positive(Positive::canonical(*self as u64))
    }
}

impl Encode for u32 {
    fn encode(&self, writer: &mut Writer) {
        writer.positive(Positive::canonical(*self as u64))
    }
}

impl Encode for u64 {
    fn encode(&self, writer: &mut Writer) {
        writer.positive(Positive::canonical(*self))
    }
}

impl Encode for String {
    fn encode(&self, writer: &mut Writer) {
        writer.text(&Text::from_str(self))
    }
}

impl Encode for str {
    fn encode(&self, writer: &mut Writer) {
        writer.text(&Text::from_str(self))
    }
}

impl Encode for [u8] {
    fn encode(&self, writer: &mut Writer) {
        writer.bytes(&Bytes::from_slice(self))
    }
}

impl<const N: usize> Encode for [u8; N] {
    fn encode(&self, writer: &mut Writer) {
        writer.bytes(&Bytes::from_slice(self))
    }
}

impl Encode for Vec<u8> {
    fn encode(&self, writer: &mut Writer) {
        writer.encode(self.as_slice())
    }
}

// don't need the bound to encode, but just enforce it for soudness of `CborDataOf`
impl<T: Encode> Encode for CborDataOf<T> {
    fn encode(&self, writer: &mut Writer) {
        writer.append_slice(&self.1)
    }
}

// don't need the bound to encode, but just enforce it for soudness of `CborDataOf`
impl<T: Encode> Encode for CborSliceOf<T> {
    fn encode(&self, writer: &mut Writer) {
        writer.append_slice(&self.1)
    }
}

impl Encode for CborData {
    fn encode(&self, writer: &mut Writer) {
        writer.append_slice(&self.0)
    }
}

impl Encode for CborSlice {
    fn encode(&self, writer: &mut Writer) {
        writer.append_slice(&self.0)
    }
}

/// Encode a slice of Ts, as a CBOR array of definite length and each
/// element written with the encode method for each T value sequentially
pub fn encode_vec<T: Encode>(elements: &[T], writer: &mut Writer) {
    writer.array_build(
        StructureLength::Definite(HeaderValue::canonical(elements.len() as u64)),
        |inner_writer| {
            for e in elements {
                e.encode(inner_writer)
            }
        },
    )
}
