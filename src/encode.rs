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

impl Encode for Byte {
    fn encode(&self, writer: &mut Writer) {
        writer.byte(*self)
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
