use super::prim::CborSlice;

#[derive(Debug, Clone, Copy)]
pub enum CborDataContext {
    Header,
    IndirectLen,
    Content,
}

/// Trying to read data from a buffer, but missing bytes
#[derive(Debug, Clone)]
pub struct CborDataMissing {
    /// The number of bytes that try to be read
    pub expecting_bytes: usize,
    /// The number of bytes available
    pub got_bytes: usize,
    /// The CBOR context which trigger this error
    pub context: CborDataContext,
}

#[derive(Clone)]
pub struct CborDataReader<'a> {
    data: &'a [u8],
    pub index: usize,
}

impl<'a> CborDataReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, index: 0 }
    }

    /// Return the number of bytes to read in the Data Reader
    pub fn remaining_bytes(&self) -> usize {
        self.data.len() - self.index
    }

    pub fn peek(
        &self,
        context: CborDataContext,
        offset: usize,
        n: usize,
    ) -> Result<&'a [u8], CborDataMissing> {
        if n == 0 {
            return Ok(&[]);
        }
        let rem = self.remaining_bytes();
        // if offset is more than remaining bytes, that might be
        // an internal error as previous peeking should have verify
        // the previous byte(s) are in the buffer already;
        // only rem == offset could genuinely happen
        if rem <= offset {
            return Err(CborDataMissing {
                expecting_bytes: n,
                got_bytes: 0,
                context,
            });
        }
        let offseted_rem = rem - offset;
        let start = self.index + offset;
        let end = start + n;
        if n > offseted_rem {
            Err(CborDataMissing {
                expecting_bytes: n,
                got_bytes: rem - offset,
                context,
            })
        } else {
            Ok(&self.data[start..end])
        }
    }

    pub fn advance(&mut self, n: usize) {
        self.index += n;
    }

    pub fn consume(
        &mut self,
        context: CborDataContext,
        n: usize,
    ) -> Result<&'a [u8], CborDataMissing> {
        let dat = self.peek(context, 0, n)?;
        self.index += n;
        Ok(dat)
    }

    // no check to see if that the slice is valid CBOR
    pub fn slice_from(&self, start: usize) -> &'a CborSlice {
        let slice = &self.data[start..self.index];
        unsafe { &*(slice as *const [u8] as *const CborSlice) }
    }
}
