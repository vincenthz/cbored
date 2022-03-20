use super::decode::{Decode, DecodeError};
use super::reader::Reader;
use std::{borrow::Borrow, marker::PhantomData};

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
        if rem < offset {
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
                got_bytes: rem,
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

/// A Validated CBOR slice of data
#[derive(Debug)]
pub struct CborSlice(pub(crate) [u8]);

impl<'a> AsRef<[u8]> for &'a CborSlice {
    fn as_ref(&self) -> &'a [u8] {
        &self.0
    }
}

impl CborSlice {
    pub fn validate_as<'a, T: Decode>(&'a self) -> Result<&'a CborSliceOf<T>, DecodeError> {
        let mut r = Reader::new(&self.0);
        <T>::decode(&mut r).map(|_| unsafe { &*(&self.0 as *const [u8] as *const CborSliceOf<T>) })
    }
}

impl<'a> CborSlice {
    pub fn reader(&'a self) -> super::reader::Reader<'a> {
        super::reader::Reader::new(&self.0)
    }

    pub fn decode<T: Decode>(&'a self) -> Result<T, DecodeError> {
        let mut reader = self.reader();
        let t = <T>::decode(&mut reader)?;
        reader.expect_finished()?;
        Ok(t)
    }
}

pub struct CborSliceOf<T>(PhantomData<T>, pub(crate) [u8]);

/// A Validated CBOR slice of data
#[derive(Clone, Debug)]
pub struct CborData(pub(crate) Vec<u8>);

/// A Validated CBOR slice of data containing the type T
///
/// Call `.unserialize()` to get the inner T type
#[derive(Clone, Debug)]
pub struct CborDataOf<T>(PhantomData<T>, pub(crate) Vec<u8>);

impl CborData {
    pub fn validate_as<'a, T: Decode>(&'a self) -> Result<CborDataOf<T>, DecodeError> {
        // complex lifetime issue here, so end up with a nasty clone of the data; TODO investigate
        let mut r = Reader::new(&self.0);
        match <T>::decode(&mut r) {
            Ok(_) => (),
            Err(e) => return Err(e),
        };
        Ok(CborDataOf(PhantomData, self.0.clone()))
    }
}

impl CborData {
    pub fn read<'a>(&'a self) -> super::reader::Reader<'a> {
        super::reader::Reader::new(&self.0)
    }

    pub fn decode<T: Decode>(&self) -> Result<T, DecodeError> {
        let mut reader = self.read();
        let t = <T>::decode(&mut reader)?;
        reader.expect_finished()?;
        Ok(t)
    }
}

impl AsRef<[u8]> for CborData {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<'a, T: Decode> CborDataOf<T> {
    pub fn unserialize(&'a self) -> T {
        let mut r = Reader::new(&self.1);
        match <T>::decode(&mut r) {
            Err(_) => panic!("unserialize of cbor-data-of<T> returned an error"),
            Ok(t) => t,
        }
    }
}

impl Borrow<CborSlice> for CborData {
    fn borrow(&self) -> &CborSlice {
        let slice = self.0.borrow();
        unsafe { &*(slice as *const [u8] as *const CborSlice) }
    }
}

impl ToOwned for CborSlice {
    type Owned = CborData;

    fn to_owned(&self) -> Self::Owned {
        CborData(self.0.to_vec())
    }
}

impl<T> Borrow<CborSliceOf<T>> for CborDataOf<T> {
    fn borrow(&self) -> &CborSliceOf<T> {
        let slice = self.1.borrow();
        unsafe { &*(slice as *const [u8] as *const CborSliceOf<T>) }
    }
}

impl<T> ToOwned for CborSliceOf<T> {
    type Owned = CborDataOf<T>;

    fn to_owned(&self) -> Self::Owned {
        CborDataOf(self.0, self.1.to_vec())
    }
}
