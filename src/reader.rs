use super::context::*;
use super::decode::*;
use super::header::*;
use super::prim::*;
use super::state::*;
use super::types::*;
use crate::lowlevel::lead::*;

/// Possible error when reading CBOR from a data stream
#[derive(Debug, Clone)]
pub enum ReaderError {
    /// The element header was invalid
    LeadError(LeadError),
    /// Trying to read more data than was available in the buffer.
    /// This can be used to fill more data in the buffer, as it return the number
    /// of byte missing
    DataMissing(CborDataMissing),
    /// When trying to get element, the overall CBOR state machine represent
    /// an invalid transition, for example a break in a non indefinite structure
    StateError(StateError),
    /// Wrong expected type, the user is asking for a specific expected type, but
    /// got some other type.
    WrongExpectedType { expected: Type, got: Type },
    /// Wrong expected types, the user is asking for a specific set of expected types, but
    /// got some other type. This is similar to `WrongExpectedType` but works with a list of
    /// multiple types
    WrongExpectedTypes {
        expected: &'static [Type],
        got: Type,
    },
    /// Wrong expected tags, the user is asking for a specific expected tag, but
    /// got some other type.
    WrongExpectedTag { expected: u64, got: u64 },
    /// Wrong expected tags, the user is asking for a specific set of expected tags, but
    /// got some other tag. This is similar to `WrongExpectedTag` but works with a list of
    /// multiple tags
    WrongExpectedTags { expected: &'static [u64], got: u64 },
    /// Length expected is not met
    WrongExpectedLength { expected: usize, got: usize },
    /// Unexpected break type
    UnexpectedBreakType,
    /// Text is not a valid UTF8 string
    TextUTF8Error(std::str::Utf8Error),
    /// Indefinite text into another indefinite text
    TextChunksInTextChunks,
    /// Indefinite bytes into another indefinite bytes
    BytesChunksInBytesChunks,
    /// Unexpected type received in an indefinite Text where only definite Text chunk are allowed
    WrongExpectedTypeInText { got: Type },
    /// Unexpected type received in an indefinite Bytes where only definite Bytes chunk are allowed
    WrongExpectedTypeInBytes { got: Type },
    /// Expected termination, but still some trailing data available
    NotTerminated {
        at: usize,
        remaining_bytes: usize,
        next_byte: u8,
    },
}

impl From<LeadError> for ReaderError {
    fn from(e: LeadError) -> Self {
        ReaderError::LeadError(e)
    }
}

impl From<StateError> for ReaderError {
    fn from(e: StateError) -> Self {
        ReaderError::StateError(e)
    }
}

impl From<CborDataMissing> for ReaderError {
    fn from(e: CborDataMissing) -> Self {
        ReaderError::DataMissing(e)
    }
}

/// CBOR Data structure to read CBOR elements from a slice of byte
pub struct Reader<'a> {
    reader: CborDataReader<'a>,
}

macro_rules! matches_type {
    ($hdr:ident, $ty:path, $hdrty:path) => {
        match $hdr {
            $hdrty(content) => Ok(content),
            _ => Err(ReaderError::WrongExpectedType {
                expected: $ty,
                got: $hdr.to_type(),
            }),
        }
    };
}

fn state_process_header(state: &mut State, header: Header) -> Result<(), ReaderError> {
    match header {
        Header::Positive(_) => state.simple()?,
        Header::Negative(_) => state.simple()?,
        Header::Bytes(c) => state.bytes(c)?,
        Header::Text(c) => state.text(c)?,
        Header::Array(c) => state.array(c)?,
        Header::Map(c) => state.map(c)?,
        Header::Tag(_) => state.tag()?,
        Header::Constant(_) => state.simple()?,
        Header::Byte(_) => state.simple()?,
        Header::Float(_) => state.simple()?,
        Header::Break => state.brk()?,
    };
    Ok(())
}

impl<'a> Reader<'a> {
    /// Return the number of bytes remaining to be processed by the reader
    pub fn remaining_bytes(&self) -> usize {
        self.reader.remaining_bytes()
    }

    /// Return the number of bytes consumed since the start of the Reader
    pub fn consumed_bytes(&self) -> usize {
        self.reader.index
    }

    /// Return if all the bytes have been consumed by the reader
    pub fn is_finished(&self) -> bool {
        self.remaining_bytes() == 0
    }

    /// Assume the reader is finished (no more bytes to process), or
    /// otherwise return a `ReaderError::NotTerminated`
    pub fn expect_finished(&self) -> Result<(), ReaderError> {
        if !self.is_finished() {
            return Err(ReaderError::NotTerminated {
                at: self.consumed_bytes(),
                remaining_bytes: self.remaining_bytes(),
                next_byte: self.reader.peek_byte(),
            });
        }
        Ok(())
    }

    // internal call to expect some bytes
    fn expect(&mut self, context: CborDataContext, n: usize) -> Result<&'a [u8], ReaderError> {
        self.reader.consume(context, n).map_err(|e| e.into())
    }

    fn peek_at(
        &self,
        context: CborDataContext,
        offset: usize,
        n: usize,
    ) -> Result<&'a [u8], ReaderError> {
        self.reader.peek(context, offset, n).map_err(|e| e.into())
    }

    pub fn new(data: &'a [u8]) -> Self {
        assert!(data.len() > 0);
        let reader = CborDataReader::new(data);
        Self { reader }
    }

    /// read the byte header
    fn lead(&self) -> Result<Lead, ReaderError> {
        let hdr = self.peek_at(CborDataContext::Header, 0, 1)?;
        let header = Lead::from_byte(hdr[0])?;
        Ok(header)
    }

    /// get the indirect content (1, 2, 4 or 8 bytes)
    ///
    /// always get data from self.index+1
    fn resolve_indirect(&self, indirect_len: IndirectLen) -> Result<IndirectValue, ReaderError> {
        match indirect_len {
            IndirectLen::I1 => {
                let len_slice = self.peek_at(CborDataContext::IndirectLen, 1, 1)?;
                Ok(IndirectValue::U8(len_slice[0]))
            }
            IndirectLen::I2 => {
                let mut data = [0u8; 2];
                data.copy_from_slice(self.peek_at(CborDataContext::IndirectLen, 1, 2)?);
                Ok(IndirectValue::U16(u16::from_be_bytes(data)))
            }
            IndirectLen::I4 => {
                let mut data = [0u8; 4];
                data.copy_from_slice(self.peek_at(CborDataContext::IndirectLen, 1, 4)?);
                Ok(IndirectValue::U32(u32::from_be_bytes(data)))
            }
            IndirectLen::I8 => {
                let mut data = [0u8; 8];
                data.copy_from_slice(self.peek_at(CborDataContext::IndirectLen, 1, 8)?);
                Ok(IndirectValue::U64(u64::from_be_bytes(data)))
            }
        }
    }

    /// peek the data type
    fn header_parts(&self) -> Result<(Lead, usize, Option<IndirectValue>), ReaderError> {
        let lead = self.lead()?;
        let maybe_indirect = lead.expected_extra();
        let (advance, indirect) = match maybe_indirect {
            None => (1, None),
            Some(v) => (1 + v.len_bytes(), Some(self.resolve_indirect(v)?)),
        };
        Ok((lead, advance, indirect))
    }

    fn header(&self) -> Result<(Header, usize), ReaderError> {
        let (ld, advance, ival) = self.header_parts()?;
        let header = Header::from_parts(ld, ival);
        Ok((header, advance))
    }

    fn advance_data(&mut self, header: &Header) -> Result<(), ReaderError> {
        match header {
            Header::Bytes(c) | Header::Text(c) => match c {
                None => {}
                Some(b) => {
                    let sz = b.to_size();
                    let _data = self.expect(CborDataContext::Content, sz)?;
                }
            },
            _ => (),
        }
        Ok(())
    }

    /// Peek at the next type in the buffer
    ///
    /// Note that it can still return error if there's no data available (end of buffer),
    /// or that the CBOR lead byte is not well-formed
    pub fn peek_type(&self) -> Result<Type, ReaderError> {
        let lead = self.lead()?;
        Ok(Type::from_lead(lead))
    }

    pub fn positive(&mut self) -> Result<Positive, ReaderError> {
        let (hdr, advance) = self.header()?;
        let content = matches_type!(hdr, Type::Positive, Header::Positive)?;
        self.reader.advance(advance);
        Ok(content)
    }

    pub fn negative(&mut self) -> Result<Negative, ReaderError> {
        let (hdr, advance) = self.header()?;
        let content = matches_type!(hdr, Type::Negative, Header::Negative)?;
        self.reader.advance(advance);
        Ok(content)
    }

    pub fn scalar(&mut self) -> Result<Scalar, ReaderError> {
        let (hdr, advance) = self.header()?;
        let content = match hdr {
            Header::Positive(pos) => Ok(Scalar::Positive(pos)),
            Header::Negative(neg) => Ok(Scalar::Negative(neg)),
            _ => Err(ReaderError::WrongExpectedTypes {
                expected: &[Type::Positive, Type::Negative],
                got: hdr.to_type(),
            }),
        }?;
        self.reader.advance(advance);
        Ok(content)
    }

    pub fn byte(&mut self) -> Result<Byte, ReaderError> {
        let (hdr, advance) = self.header()?;
        let content = matches_type!(hdr, Type::Byte, Header::Byte)?;
        self.reader.advance(advance);
        Ok(content)
    }

    pub fn float(&mut self) -> Result<Float, ReaderError> {
        let (hdr, advance) = self.header()?;
        let content = matches_type!(hdr, Type::Float, Header::Float)?;
        self.reader.advance(advance);
        Ok(content)
    }

    pub fn constant(&mut self) -> Result<Constant, ReaderError> {
        let (hdr, advance) = self.header()?;
        let content = match hdr {
            Header::Constant(constant) => Ok(constant),
            _ => Err(ReaderError::WrongExpectedTypes {
                expected: &[Type::False, Type::True, Type::Null, Type::Undefined],
                got: hdr.to_type(),
            }),
        }?;
        self.reader.advance(advance);
        Ok(content)
    }

    pub fn null(&mut self) -> Result<(), ReaderError> {
        let (hdr, advance) = self.header()?;
        let _content = matches_type!(hdr, Type::Null, Header::Constant)?;
        self.reader.advance(advance);
        Ok(())
    }

    pub fn undefined(&mut self) -> Result<(), ReaderError> {
        let (hdr, advance) = self.header()?;
        let _content = matches_type!(hdr, Type::Undefined, Header::Constant)?;
        self.reader.advance(advance);
        Ok(())
    }

    pub fn bool(&mut self) -> Result<bool, ReaderError> {
        let (hdr, advance) = self.header()?;
        let content = match hdr {
            Header::Constant(Constant::True) => Ok(true),
            Header::Constant(Constant::False) => Ok(false),
            _ => Err(ReaderError::WrongExpectedTypes {
                expected: &[Type::False, Type::True],
                got: hdr.to_type(),
            }),
        }?;
        self.reader.advance(advance);
        Ok(content)
    }

    pub fn bytes(&mut self) -> Result<Bytes<'a>, ReaderError> {
        let (hdr, advance) = self.header()?;
        let content = matches_type!(hdr, Type::Bytes, Header::Bytes)?;
        self.reader.advance(advance);
        match content {
            // indefinite bytes
            None => {
                let mut out = Vec::new();
                loop {
                    let (hdr, advance) = self.header()?;
                    self.reader.advance(advance);
                    match hdr {
                        Header::Break => {
                            break;
                        }
                        Header::Bytes(t) => match t {
                            None => return Err(ReaderError::BytesChunksInBytesChunks),
                            Some(b) => {
                                let sz = b.to_size();
                                let data = self.expect(CborDataContext::Content, sz)?;
                                out.push(BytesData(b, data));
                            }
                        },
                        _ => {
                            return Err(ReaderError::WrongExpectedTypeInBytes {
                                got: hdr.to_type(),
                            });
                        }
                    }
                }
                Ok(Bytes::Chunks(out))
            }
            // immediate bytes
            Some(b) => {
                let sz = b.to_size();
                let data = self.expect(CborDataContext::Content, sz)?;
                Ok(Bytes::Imm(BytesData(b, data)))
            }
        }
    }

    fn text_data(&mut self, b: HeaderValue) -> Result<TextData<'a>, ReaderError> {
        let sz = b.to_size();
        let data = self.expect(CborDataContext::Content, sz)?;
        let data_str = std::str::from_utf8(data).map_err(|err| ReaderError::TextUTF8Error(err))?;
        Ok(TextData(b, data_str))
    }

    pub fn text(&mut self) -> Result<Text<'a>, ReaderError> {
        let (hdr, advance) = self.header()?;
        let content = matches_type!(hdr, Type::Text, Header::Text)?;

        self.reader.advance(advance);
        match content {
            // indefinite UTF8 string
            None => {
                let mut out = Vec::new();
                loop {
                    let (hdr, advance) = self.header()?;
                    self.reader.advance(advance);
                    match hdr {
                        Header::Break => {
                            break;
                        }
                        Header::Text(t) => match t {
                            None => return Err(ReaderError::TextChunksInTextChunks),
                            Some(b) => {
                                let textdata = self.text_data(b)?;
                                out.push(textdata)
                            }
                        },
                        _ => {
                            return Err(ReaderError::WrongExpectedTypeInText {
                                got: hdr.to_type(),
                            });
                        }
                    }
                }
                Ok(Text::Chunks(out))
            }
            // immediate UTF8 string
            Some(b) => {
                let textdata = self.text_data(b)?;
                Ok(Text::Imm(textdata))
            }
        }
    }

    /// return the slice of data of one next element (whatever it is)
    fn cbor_slice_neutral(&mut self) -> Result<&'a CborSlice, ReaderError> {
        let start = self.reader.index;
        let mut state = State::new();
        loop {
            let (header, advance) = self.header()?;
            self.reader.advance(advance);

            self.advance_data(&header)?;
            state_process_header(&mut state, header)?;
            if state.acceptable() {
                break;
            }
        }
        let data = self.reader.slice_from(start);
        Ok(data)
    }

    pub fn array(&mut self) -> Result<Array<'a>, ReaderError> {
        let (hdr, advance) = self.header()?;
        let content = matches_type!(hdr, Type::Array, Header::Array)?;

        self.reader.advance(advance);

        let mut elements = Vec::new();
        match content {
            // indefinite Array
            None => {
                // loop for cbor slices until we find a cbor break
                while self.peek_type()? != Type::Break {
                    let data = self.cbor_slice_neutral()?;
                    elements.push(data);
                }
                // skip the break now that we found it
                self.reader.advance(1);

                Ok(Array {
                    len_encoding: content.into(),
                    elements,
                })
            }
            // definite Array
            Some(len) => {
                let sz = len.to_size();
                for _ in 0..sz {
                    let data = self.cbor_slice_neutral()?;
                    elements.push(data);
                }

                Ok(Array {
                    len_encoding: content.into(),
                    elements,
                })
            }
        }
    }

    pub fn map(&mut self) -> Result<Map<'a>, ReaderError> {
        let (hdr, advance) = self.header()?;
        let content = matches_type!(hdr, Type::Map, Header::Map)?;

        self.reader.advance(advance);

        let mut elements = Vec::new();
        match content {
            // indefinite Map
            None => {
                // loop for cbor key/value slices until we find a cbor break
                while self.peek_type()? != Type::Break {
                    let key = self.cbor_slice_neutral()?;
                    let value = self.cbor_slice_neutral()?;
                    elements.push((key, value));
                }

                // skip the break now that we found it
                self.reader.advance(1);

                Ok(Map {
                    len_encoding: content.into(),
                    elements,
                })
            }
            // definite Map
            Some(len) => {
                let sz = len.to_size();
                for _ in 0..sz {
                    let key = self.cbor_slice_neutral()?;
                    let value = self.cbor_slice_neutral()?;
                    elements.push((key, value));
                }

                Ok(Map {
                    len_encoding: content.into(),
                    elements,
                })
            }
        }
    }

    pub fn tag(&mut self) -> Result<Tag<'a>, ReaderError> {
        let (hdr, advance) = self.header()?;
        let tag_val = TagValue(matches_type!(hdr, Type::Tag, Header::Tag)?);

        self.reader.advance(advance);
        let data = self.cbor_slice_neutral()?;

        Ok(Tag { tag_val, data })
    }

    pub fn data(&mut self) -> Result<Data<'a>, ReaderError> {
        let ty = self.peek_type()?;
        match ty {
            Type::Positive => self.positive().map(Data::Positive),
            Type::Negative => self.negative().map(Data::Negative),
            Type::Bytes => self.bytes().map(Data::Bytes),
            Type::Text => self.text().map(Data::Text),
            Type::Array => self.array().map(Data::Array),
            Type::Map => self.map().map(Data::Map),
            Type::Tag => self.tag().map(Data::Tag),
            Type::False => self.constant().map(|_| Data::False),
            Type::True => self.constant().map(|_| Data::True),
            Type::Null => self.constant().map(|_| Data::Null),
            Type::Undefined => self.constant().map(|_| Data::Undefined),
            Type::Float => self.float().map(Data::Float),
            Type::Byte => self.byte().map(Data::Byte),
            Type::Break => Err(ReaderError::UnexpectedBreakType),
        }
    }

    pub fn decode<T: Decode>(&mut self) -> Result<T, DecodeError> {
        <T>::decode(self)
    }

    pub fn decode_one<T: Decode>(&mut self) -> Result<T, DecodeError> {
        let t = <T>::decode(self)?;
        let remaining_bytes = self.remaining_bytes();
        if remaining_bytes == 0 {
            Ok(t)
        } else {
            Err(DecodeErrorKind::ReaderNotTerminated { remaining_bytes }.context::<T>())
        }
    }

    pub fn exact_decodable_slice<T: Decode>(&mut self) -> Result<&'a CborSliceOf<T>, DecodeError> {
        let slice = self
            .cbor_slice_neutral()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<&'a CborSliceOf<T>>())?;
        slice.validate_as()
    }

    pub fn exact_decodable_data<T: Decode>(&mut self) -> Result<CborDataOf<T>, DecodeError> {
        let slice = self
            .cbor_slice_neutral()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<CborDataOf<T>>())?;
        slice.validate_as().map(|slice| slice.to_owned())
    }
}
