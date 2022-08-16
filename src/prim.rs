use super::decode::{Decode, DecodeError, DecodeErrorKind};
use super::encode::Encode;
use super::reader::Reader;
use super::writer::Writer;
use std::{borrow::Borrow, marker::PhantomData};

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
        let result = <T>::decode(&mut r)
            .map(|_| unsafe { &*(&self.0 as *const [u8] as *const CborSliceOf<T>) })?;
        if !r.is_finished() {
            return Err(DecodeErrorKind::ReaderNotTerminated {
                remaining_bytes: r.remaining_bytes(),
            }
            .context::<Self>());
        }
        Ok(result)
    }
}

impl<'a> CborSlice {
    pub fn reader(&'a self) -> super::reader::Reader<'a> {
        super::reader::Reader::new(&self.0)
    }

    pub fn decode<T: Decode>(&'a self) -> Result<T, DecodeError> {
        let mut reader = self.reader();
        let t = <T>::decode(&mut reader)?;
        reader
            .expect_finished()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<T>())?;
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
#[derive(Debug)]
pub struct CborDataOf<T>(PhantomData<T>, pub(crate) Vec<u8>);

impl<T> Clone for CborDataOf<T> {
    fn clone(&self) -> Self {
        CborDataOf(self.0.clone(), self.1.clone())
    }
}

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

    // don't want this exposed to public, only use this when we know we parsed a T already
    pub(crate) fn type_unchecked<T>(self) -> CborDataOf<T> {
        CborDataOf(PhantomData, self.0)
    }
}

impl CborData {
    pub fn read<'a>(&'a self) -> super::reader::Reader<'a> {
        super::reader::Reader::new(&self.0)
    }

    pub fn decode<T: Decode>(&self) -> Result<T, DecodeError> {
        let mut reader = self.read();
        let t = <T>::decode(&mut reader)?;
        reader
            .expect_finished()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<T>())?;
        Ok(t)
    }
}

impl AsRef<[u8]> for CborData {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<T> AsRef<[u8]> for CborDataOf<T> {
    fn as_ref(&self) -> &[u8] {
        &self.1
    }
}

impl<T> CborSliceOf<T> {
    pub fn untype<'a>(&'a self) -> &'a CborSlice {
        unsafe { &*(&self.1 as *const [u8] as *const CborSlice) }
    }
}

impl<T> CborDataOf<T> {
    pub fn untype(self) -> CborData {
        CborData(self.1)
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

impl<T: Encode> From<&T> for CborDataOf<T> {
    fn from(t: &T) -> Self {
        let mut writer = Writer::new();
        writer.encode(t);
        let data = writer.finalize_data();
        data.type_unchecked()
    }
}

impl<T> ToOwned for CborSliceOf<T> {
    type Owned = CborDataOf<T>;

    fn to_owned(&self) -> Self::Owned {
        CborDataOf(self.0, self.1.to_vec())
    }
}
