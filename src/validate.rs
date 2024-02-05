//! A CBOR validator for raw data
pub use super::context::CborDataMissing;
use super::context::*;
use super::header::{Header, HeaderValueStream};
use super::prim::*;
use super::state::{State, StateError};
use crate::lowlevel::lead::*;

/// Enumeration of possible Validator error
#[derive(Debug, Clone)]
pub enum ValidateError {
    /// The element header was invalid
    LeadError(LeadError),
    /// Trying to read more data than was available in the buffer.
    /// This can be used to fill more data in the buffer, as it return the number
    /// of byte missing
    DataMissing(CborDataMissing),
    /// State machine error in the CBOR stream
    StateError(StateError),
}

impl From<LeadError> for ValidateError {
    fn from(e: LeadError) -> Self {
        ValidateError::LeadError(e)
    }
}

impl From<StateError> for ValidateError {
    fn from(e: StateError) -> Self {
        ValidateError::StateError(e)
    }
}

impl From<CborDataMissing> for ValidateError {
    fn from(e: CborDataMissing) -> Self {
        ValidateError::DataMissing(e)
    }
}

/// Validator structure (Data stream + CBOR State machine)
pub struct Validator<'a> {
    reader: CborDataReader<'a>,
    state: State,
}

impl<'a> Validator<'a> {
    pub fn remaining_bytes(&self) -> usize {
        self.reader.remaining_bytes()
    }

    // internal call to expect some bytes
    fn expect(&mut self, context: CborDataContext, n: usize) -> Result<&'a [u8], ValidateError> {
        self.reader.consume(context, n).map_err(|e| e.into())
    }

    fn peek_at(
        &self,
        context: CborDataContext,
        offset: usize,
        n: usize,
    ) -> Result<&'a [u8], ValidateError> {
        self.reader.peek(context, offset, n).map_err(|e| e.into())
    }

    pub fn new(data: &'a [u8]) -> Self {
        assert!(data.len() > 0);
        let reader = CborDataReader::new(data);
        Self {
            reader,
            state: State::new(),
        }
    }

    /// read the byte header
    fn lead(&self) -> Result<Lead, ValidateError> {
        let hdr = self.peek_at(CborDataContext::Header, 0, 1)?;
        let header = Lead::from_byte(hdr[0])?;
        Ok(header)
    }

    /// get the indirect content (1, 2, 4 or 8 bytes)
    fn resolve_indirect(&self, indirect_len: IndirectLen) -> Result<IndirectValue, ValidateError> {
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
    fn header_parts(&mut self) -> Result<(Lead, usize, Option<IndirectValue>), ValidateError> {
        let lead = self.lead()?;
        let maybe_indirect = lead.expected_extra();
        let (advance, indirect) = match maybe_indirect {
            None => (1, None),
            Some(v) => (1 + v.len_bytes(), Some(self.resolve_indirect(v)?)),
        };
        Ok((lead, advance, indirect))
    }

    fn consume_data_streamable(&mut self, v: HeaderValueStream) -> Result<(), ValidateError> {
        match v {
            // beginning of a indefinite Text or indefinite Bytes
            None => {}
            // A regular text or bytes, we need to consume the data of specified size,
            Some(b) => {
                let sz = b.to_size();
                let _data = self.expect(CborDataContext::Content, sz)?;
            }
        }
        Ok(())
    }

    fn process_header(&mut self, header: Header) -> Result<(), ValidateError> {
        match header {
            Header::Positive(_) => self.state.simple()?,
            Header::Negative(_) => self.state.simple()?,
            Header::Bytes(c) => {
                self.consume_data_streamable(c)?;
                self.state.bytes(c)?
            }
            Header::Text(c) => {
                self.consume_data_streamable(c)?;
                self.state.text(c)?;
            }
            Header::Array(c) => self.state.array(c)?,
            Header::Map(c) => self.state.map(c)?,
            Header::Tag(_) => self.state.tag()?,
            Header::Constant(_) => self.state.simple()?,
            Header::Byte(_) => self.state.simple()?,
            Header::Float(_) => self.state.simple()?,
            Header::Break => self.state.brk()?,
        };
        Ok(())
    }

    /// Advance to the next boundary of a finished CBOR structure
    ///
    /// If the data points to a simple object (integer, ..), then it will just pop this object
    ///
    /// If the data points to a composite object (array, map, indefinite bytestring), then it will advance and validate
    /// until the end of this composite object
    ///
    /// On success, it returns the validated CBOR slice and the displacement in bytes
    /// On error, it returns a `ValidateError` containing
    pub fn next(&mut self) -> Result<(&'a CborSlice, usize), ValidateError> {
        let start = self.reader.index;

        loop {
            let (ld, advance, ival) = self.header_parts()?;
            let header = Header::from_parts(ld, ival);
            self.reader.advance(advance);
            self.process_header(header)?;

            if self.state.acceptable() {
                let valid = self.reader.slice_from(start);
                return Ok((valid, self.reader.index - start));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! validate_all {
        ($s: expr) => {
            match Validator::new($s).next() {
                Ok((_, n)) if $s.len() == n => (),
                Ok((_, n)) => panic!(
                    "validated length {} is different than input {}",
                    n,
                    $s.len()
                ),
                Err(e) => panic!("expecting validated but failed with {:?}", e),
            }
        };
    }

    macro_rules! validate_error {
        ($s: expr) => {
            match Validator::new($s).next() {
                Ok((_, _)) => panic!("expecting error but got success"),
                Err(e) => e,
            }
        };
    }

    #[test]
    fn data_missing() {
        // expecting a Bytes of 3 bytes
        let e = validate_error!(&[0x43]);
        assert!(matches!(
            e,
            ValidateError::DataMissing(CborDataMissing {
                expecting_bytes: 3,
                got_bytes: 0,
                context: _
            })
        ));

        // expecting a Bytes of 3 bytes
        let e = validate_error!(&[0x43, 0x1]);
        assert!(matches!(
            e,
            ValidateError::DataMissing(CborDataMissing {
                expecting_bytes: 3,
                got_bytes: 1,
                context: _
            })
        ));

        // expected 2 bytes after integer, to encode 258: 0x19, 0x01, 0x02
        let e = validate_error!(&[0x19, 0x01]);
        assert!(
            matches!(
                e,
                ValidateError::DataMissing(CborDataMissing {
                    expecting_bytes: 2,
                    got_bytes: 1,
                    context: CborDataContext::IndirectLen,
                },),
            ),
            "{:?}",
            e
        )
    }

    #[test]
    fn array_array() {
        validate_all!(&[0x83, 0x01, 0x82, 0x02, 0x03, 0x82, 0x04, 0x05])
    }

    #[test]
    fn array_indef_array() {
        validate_all!(&[0x9f, 0x01, 0x82, 0x02, 0x03, 0x9f, 0x04, 0x05, 0xff, 0xff])
    }

    #[test]
    fn array_indef_array2() {
        validate_all!(&[0x9f, 0x01, 0x82, 0x02, 0x03, 0x82, 0x04, 0x05, 0xff])
    }

    #[test]
    fn array_indef_array3() {
        validate_all!(&[0x83, 0x01, 0x82, 0x02, 0x03, 0x9f, 0x04, 0x05, 0xff])
    }

    #[test]
    fn array_indef_array4() {
        validate_all!(&[0x83, 0x01, 0x9f, 0x02, 0x03, 0xff, 0x82, 0x04, 0x05])
    }

    #[test]
    fn tag1() {
        validate_all!(&[0xC2, 0x49, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,]);
    }
}
