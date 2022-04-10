//! A non exhaustive implementation of the most common tagged CBOR extension

use super::*;

/// CBOR Standard Date/Time String (Tag 0)
#[derive(Clone, Debug)]
pub struct StandardDateTime(TagValue, TextOwned);

/// CBOR Positive Bignum (Tag 2)
#[derive(Clone, Debug)]
pub struct PositiveBignum(TagValue, BytesOwned);

/// CBOR Negative Bignum (Tag 3)
#[derive(Clone, Debug)]
pub struct NegativeBignum(TagValue, BytesOwned);

/// CBOR data in CBOR (Tag 24)
#[derive(Clone, Debug)]
pub struct EncodedCBOR(TagValue, BytesOwned);

/// CBOR Rational (Tag 30)
#[derive(Clone, Debug)]
pub struct RationalNumber {
    tag: TagValue,
    len_encoding: StructureLength,
    numerator: RationalNumerator,
    denominator: RationalDenominator,
}

#[derive(Clone, Debug)]
pub enum RationalNumerator {
    Positive(Positive),
    Negative(Negative),
    PositiveBignum(PositiveBignum),
    NegativeBignum(NegativeBignum),
}

#[derive(Clone, Debug)]
pub enum RationalDenominator {
    Positive(Positive),
    PositiveBignum(PositiveBignum),
}

macro_rules! matches_tag {
    ($reader:ident, $value:literal) => {{
        let tag = $reader.tag()?;
        if tag.value() != $value {
            return Err(ReaderError::WrongExpectedTag {
                expected: $value,
                got: tag.value(),
            });
        }
        tag
    }};
}

macro_rules! encode_decode {
    ($type:ident) => {
        impl Decode for $type {
            fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
                $type::read(reader).map_err(|e| e.into())
            }
        }
        impl Encode for $type {
            fn encode(&self, writer: &mut Writer) {
                self.write(writer)
            }
        }
    };
}

impl StandardDateTime {
    pub fn read<'a>(reader: &mut Reader<'a>) -> Result<Self, ReaderError> {
        let tag = matches_tag!(reader, 0);
        let text = tag.read_data(|reader| reader.text())?;
        Ok(StandardDateTime(tag.tag_repr(), text.owned()))
    }

    fn write(&self, writer: &mut Writer) {
        writer.tag_build(self.0, |writer| writer.text(&self.1.borrow()));
    }
}

encode_decode!(StandardDateTime);

impl PositiveBignum {
    pub fn read<'a>(reader: &mut Reader<'a>) -> Result<Self, ReaderError> {
        let tag = matches_tag!(reader, 2);
        let bytes = tag.read_data(|reader| reader.bytes())?;
        Ok(PositiveBignum(tag.tag_repr(), bytes.owned()))
    }

    fn write(&self, writer: &mut Writer) {
        writer.tag_build(self.0, |writer| writer.bytes(&self.1.borrow()));
    }

    /// Write the bignum as a big endian representation
    pub fn to_be_bytes(&self) -> Vec<u8> {
        self.1.borrow().to_vec()
    }
}

encode_decode!(PositiveBignum);

impl NegativeBignum {
    pub fn read<'a>(reader: &mut Reader<'a>) -> Result<Self, ReaderError> {
        let tag = matches_tag!(reader, 3);
        let bytes = tag.read_data(|reader| reader.bytes())?;
        Ok(NegativeBignum(tag.tag_repr(), bytes.owned()))
    }
    fn write(&self, writer: &mut Writer) {
        writer.tag_build(self.0, |writer| writer.bytes(&self.1.borrow()));
    }

    /// Write the bignum as a big endian representation for -1 - n
    pub fn to_be_bytes(&self) -> Vec<u8> {
        self.1.borrow().to_vec()
    }
}

encode_decode!(NegativeBignum);

impl EncodedCBOR {
    pub fn read<'a>(reader: &mut Reader<'a>) -> Result<Self, ReaderError> {
        let tag = matches_tag!(reader, 24);
        let bytes = tag.read_data(|reader| reader.bytes())?;
        Ok(EncodedCBOR(tag.tag_repr(), bytes.owned()))
    }

    fn write(&self, writer: &mut Writer) {
        writer.tag_build(self.0, |writer| writer.bytes(&self.1.borrow()));
    }

    /// Get the CBOR data as Bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.1.borrow().to_vec()
    }

    pub fn from_bytes(cbor_bytes: &[u8]) -> Self {
        EncodedCBOR(
            TagValue::from_u64(24),
            BytesOwned::from_vec(cbor_bytes.to_vec()),
        )
    }
}

encode_decode!(EncodedCBOR);

impl RationalNumber {
    pub fn read<'a>(reader: &mut Reader<'a>) -> Result<Self, ReaderError> {
        let tag = matches_tag!(reader, 30);
        let rational = tag.read_data(|reader| {
            let array = reader.array()?;
            if array.len() != 2 {
                return Err(ReaderError::WrongExpectedLength {
                    expected: 2,
                    got: array.len(),
                });
            }
            let numerator = {
                let mut inner_reader = array[0].reader();
                let res = match inner_reader.peek_type()? {
                    Type::Positive => inner_reader.positive().map(RationalNumerator::Positive),
                    Type::Negative => inner_reader.negative().map(RationalNumerator::Negative),
                    Type::Tag => PositiveBignum::read(&mut inner_reader)
                        .map(RationalNumerator::PositiveBignum)
                        .or_else(|_| {
                            NegativeBignum::read(&mut inner_reader)
                                .map(RationalNumerator::NegativeBignum)
                        }),
                    ty => Err(ReaderError::WrongExpectedTypes {
                        expected: &[Type::Positive, Type::Negative, Type::Tag],
                        got: ty,
                    }),
                }?;
                inner_reader.expect_finished()?;
                res
            };
            let denominator = {
                let mut inner_reader = array[1].reader();
                let res = match inner_reader.peek_type()? {
                    Type::Positive => inner_reader.positive().map(RationalDenominator::Positive),
                    Type::Tag => PositiveBignum::read(&mut inner_reader)
                        .map(RationalDenominator::PositiveBignum),
                    ty => Err(ReaderError::WrongExpectedTypes {
                        expected: &[Type::Positive, Type::Tag],
                        got: ty,
                    }),
                }?;
                inner_reader.expect_finished()?;
                res
            };
            Ok(RationalNumber {
                tag: tag.tag_repr(),
                len_encoding: array.len_encoding,
                numerator,
                denominator,
            })
        })?;
        Ok(rational)
    }

    fn write(&self, writer: &mut Writer) {
        // check that it's encoding a value of 2 if defined
        match self.len_encoding {
            StructureLength::Indefinite => {}
            StructureLength::Definite(v) if v.to_u64() == 2 => {}
            StructureLength::Definite(v) => {
                panic!("RationalNumber length encoding is not 2, {}", v.to_u64())
            }
        };
        writer.tag_build(self.tag, |writer| {
            writer.array_build(self.len_encoding, |writer| {
                match &self.numerator {
                    RationalNumerator::Positive(v) => writer.positive(*v),
                    RationalNumerator::Negative(v) => writer.negative(*v),
                    RationalNumerator::PositiveBignum(v) => v.write(writer),
                    RationalNumerator::NegativeBignum(v) => v.write(writer),
                };
                match &self.denominator {
                    RationalDenominator::Positive(v) => writer.positive(*v),
                    RationalDenominator::PositiveBignum(v) => v.write(writer),
                }
            })
        })
    }

    pub fn numerator(&self) -> &RationalNumerator {
        &self.numerator
    }

    pub fn denominator(&self) -> &RationalDenominator {
        &self.denominator
    }
}

encode_decode!(RationalNumber);
