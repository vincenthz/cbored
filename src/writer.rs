use super::encode::Encode;
use super::header::*;
use super::prim::CborData;
use super::types::*;
use crate::lowlevel::lead::*;

/// CBOR Data structure to write CBOR elements to a growing byte vector
pub struct Writer {
    data: Vec<u8>,
}

impl Writer {
    /// Create a new CBOR Writer
    pub fn new() -> Self {
        Writer { data: Vec::new() }
    }

    pub fn finalize_data(self) -> CborData {
        CborData(self.data)
    }

    /// Finalize the CBOR writer and get the data as bytes
    pub fn finalize(self) -> Vec<u8> {
        self.data
    }

    /// Write a T encodable type in the writer
    pub fn encode<T: Encode + ?Sized>(&mut self, t: &T) {
        t.encode(self)
    }

    fn append_byte(&mut self, b: u8) {
        self.data.push(b)
    }

    pub(crate) fn append_slice(&mut self, b: &[u8]) {
        self.data.extend_from_slice(b)
    }

    fn write_break(&mut self) {
        self.append_byte(0xff);
    }

    fn write_value(&mut self, m: Major, v: HeaderValue) {
        let lead = m.to_high_bits() | v.to_lead_content().to_byte();
        self.append_byte(lead);
        match v {
            HeaderValue::Imm(_) => (),
            HeaderValue::U8(v) => self.append_byte(v),
            HeaderValue::U16(v) => self.append_slice(&v.to_be_bytes()),
            HeaderValue::U32(v) => self.append_slice(&v.to_be_bytes()),
            HeaderValue::U64(v) => self.append_slice(&v.to_be_bytes()),
        };
    }

    fn write_value_stream(&mut self, m: Major, v: HeaderValueStream) {
        let lead = m.to_high_bits() | ContentStream::from(v.map(|c| c.to_lead_content())).to_byte();
        self.append_byte(lead);
        match v {
            None | Some(HeaderValue::Imm(_)) => (),
            Some(HeaderValue::U8(v)) => self.append_byte(v),
            Some(HeaderValue::U16(v)) => self.append_slice(&v.to_be_bytes()),
            Some(HeaderValue::U32(v)) => self.append_slice(&v.to_be_bytes()),
            Some(HeaderValue::U64(v)) => self.append_slice(&v.to_be_bytes()),
        };
    }

    fn write_structure_length(&mut self, m: Major, v: StructureLength) {
        match v {
            StructureLength::Indefinite => self.write_value_stream(m, None),
            StructureLength::Definite(v) => self.write_value_stream(m, Some(v)),
        }
    }

    /// Append a Positive value in the writer
    pub fn positive(&mut self, d: Positive) {
        self.write_value(Major::Positive, d.0)
    }

    /// Append a Negative value in the writer
    pub fn negative(&mut self, d: Negative) {
        self.write_value(Major::Negative, d.0)
    }

    pub fn scalar(&mut self, d: Scalar) {
        match d {
            Scalar::Positive(p) => self.positive(p),
            Scalar::Negative(n) => self.negative(n),
        }
    }

    /// Append a Byte value in the writer
    pub fn byte(&mut self, d: Byte) {
        match d.0 {
            HeaderValue8::Imm(v) => {
                assert!(v < 0x14);
                self.append_byte(0xe0 + v);
            }
            HeaderValue8::U8(v) => {
                self.append_byte(0xf8);
                self.append_byte(v);
            }
        }
    }

    /// Append a Bytes value in the writer, depending of the Bytes CBOR encoding, it will be either
    /// represented as indefinite sequence of sequence bytes (terminated by CBOR break),
    /// or an immediate bytes sequence.
    pub fn bytes<'a>(&mut self, d: &Bytes<'a>) {
        match d {
            Bytes::Imm(bd) => {
                self.write_value(Major::Bytes, bd.0);
                self.append_slice(bd.1);
            }
            Bytes::Chunks(vec) => {
                self.write_value_stream(Major::Bytes, None);
                for elem in vec {
                    self.write_value(Major::Bytes, elem.0);
                    self.append_slice(elem.1);
                }
                self.write_break()
            }
        }
    }

    /// Append a Text value in the writer, depending of the Text CBOR encoding, it will be either
    /// represented as indefinite sequence of sequence utf8 bytes (terminated by CBOR break),
    /// or an immediate utf8 bytes sequence.
    pub fn text<'a>(&mut self, d: &Text<'a>) {
        match d {
            Text::Imm(bd) => {
                self.write_value(Major::Text, bd.0);
                self.append_slice(bd.1.as_bytes());
            }
            Text::Chunks(vec) => {
                self.write_value_stream(Major::Text, None);
                for elem in vec {
                    self.write_value(Major::Text, elem.0);
                    self.append_slice(elem.1.as_bytes());
                }
                self.write_break()
            }
        }
    }

    /// Append an Array in the writer using a closure
    pub fn array_build<F>(&mut self, len: StructureLength, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.write_structure_length(Major::Array, len);
        f(self);
        if len.is_indefinite() {
            self.write_break()
        }
    }

    /// Append an Array in the writer
    pub fn array<'a>(&mut self, d: &Array<'a>) {
        self.write_structure_length(Major::Array, d.len_encoding);
        for v in d.elements.iter() {
            self.append_slice(&v.0);
        }
        if d.len_encoding.is_indefinite() {
            self.write_break()
        }
    }

    /// Append a Map in the writer using a closure
    pub fn map_build<F>(&mut self, len: StructureLength, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.write_structure_length(Major::Map, len);
        f(self);
        if len.is_indefinite() {
            self.write_break()
        }
    }

    /// Append a Map in the writer
    pub fn map<'a>(&mut self, d: &Map<'a>) {
        self.write_structure_length(Major::Map, d.len_encoding);
        for (k, v) in d.elements.iter() {
            self.append_slice(&k.0);
            self.append_slice(&v.0);
        }
        if d.len_encoding.is_indefinite() {
            self.write_break()
        }
    }

    /// Append a Tagged value (TAG + CBOR value) in the writer
    pub fn tag<'a>(&mut self, d: &Tag<'a>) {
        self.write_value(Major::Tag, d.tag_val.0);
        self.append_slice(&d.data.0);
    }

    /// Append a Tagged value (TAG + CBOR value) in the writer
    pub fn tag_build<F>(&mut self, tag_val: TagValue, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.write_value(Major::Tag, tag_val.0);
        f(self)
    }

    /// Append a constant value (false, true, null, undefined) in the writer
    pub fn constant(&mut self, d: Constant) {
        match d {
            Constant::False => self.append_byte(0xf4),
            Constant::True => self.append_byte(0xf5),
            Constant::Null => self.append_byte(0xf6),
            Constant::Undefined => self.append_byte(0xf7),
        }
    }

    /// Append a boolean (as True/False) in the writer
    pub fn bool(&mut self, d: bool) {
        match d {
            true => self.constant(Constant::True),
            false => self.constant(Constant::False),
        }
    }

    /// Append a float (one of half, normal, double precision) in the writer
    pub fn float(&mut self, d: Float) {
        match d {
            Float::FP16(v) => {
                self.append_byte(0xf9);
                self.append_slice(&v.to_be_bytes());
            }
            Float::FP32(v) => {
                self.append_byte(0xfa);
                self.append_slice(&v.to_be_bytes());
            }
            Float::FP64(v) => {
                self.append_byte(0xfb);
                self.append_slice(&v.to_be_bytes());
            }
        }
    }

    /// Append some CBOR data in the writer
    pub fn data<'a>(&mut self, d: &Data<'a>) {
        match d {
            Data::Positive(v) => self.positive(*v),
            Data::Negative(v) => self.negative(*v),
            Data::Float(v) => self.float(*v),
            Data::Byte(v) => self.byte(*v),
            Data::Bytes(v) => self.bytes(v),
            Data::Text(v) => self.text(v),
            Data::Array(v) => self.array(v),
            Data::Map(v) => self.map(v),
            Data::Tag(v) => self.tag(v),
            Data::True => self.constant(Constant::True),
            Data::False => self.constant(Constant::False),
            Data::Null => self.constant(Constant::Null),
            Data::Undefined => self.constant(Constant::Undefined),
        }
    }
}
