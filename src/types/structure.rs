use super::super::decode::{Decode, DecodeError, DecodeErrorKind};
use super::super::encode::Encode;
use super::super::header::Value;
use super::super::prim::{CborData, CborSlice};
use super::super::reader::{Reader, ReaderError};
use super::super::writer::Writer;
use std::borrow::{Borrow, ToOwned};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureLength {
    Indefinite,
    Definite(Value),
}

impl StructureLength {
    pub fn is_indefinite(self) -> bool {
        match self {
            StructureLength::Indefinite => true,
            StructureLength::Definite(_) => false,
        }
    }
}

impl From<Option<Value>> for StructureLength {
    fn from(v: Option<Value>) -> StructureLength {
        match v {
            None => StructureLength::Indefinite,
            Some(val) => StructureLength::Definite(val),
        }
    }
}

impl From<u64> for StructureLength {
    fn from(v: u64) -> StructureLength {
        StructureLength::Definite(Value::canonical(v))
    }
}

/// CBOR Array with references to elements
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Array<'a> {
    pub(crate) len_encoding: StructureLength,
    pub(crate) elements: Vec<&'a CborSlice>,
}

/// CBOR Array with owned elements
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayOwned {
    pub(crate) len_encoding: StructureLength,
    pub(crate) elements: Vec<CborData>,
}

/// CBOR Array builder, when constructing
pub struct ArrayBuilder {
    elements: Vec<CborData>,
}

/// CBOR Map with references to keys and values
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map<'a> {
    pub(crate) len_encoding: StructureLength,
    pub(crate) elements: Vec<(&'a CborSlice, &'a CborSlice)>,
}

/// CBOR Map with owned keys and values
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapOwned {
    pub(crate) len_encoding: StructureLength,
    pub(crate) elements: Vec<(CborData, CborData)>,
}

/// CBOR Tag Value in a Tag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagValue(pub(crate) Value);

/// CBOR Tag with reference to the tagged element
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag<'a> {
    pub(crate) tag_val: TagValue,
    pub(crate) data: &'a CborSlice,
}

/// CBOR Tag with owned tagged element
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagOwned {
    pub(crate) tag_val: TagValue,
    pub(crate) data: CborData,
}

impl<'a> Array<'a> {
    /// Get the structure length of the Array
    pub fn struct_len(&self) -> StructureLength {
        self.len_encoding
    }

    /// Return the number of CBOR element in this Array
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Get an iterator to the reader of each element of the array
    pub fn iter(&'a self) -> impl Iterator<Item = Reader<'a>> {
        self.elements.iter().map(|v| v.reader())
    }

    /// Try to turn an array of CBOR element into a homogenous Vec of T,
    /// where each reader is processed with the function f in parameter
    pub fn to_vec<F, T: Decode>(&self, f: F) -> Result<Vec<T>, DecodeErrorKind>
    where
        F: for<'b> Fn(&mut Reader<'b>) -> Result<T, DecodeErrorKind>,
    {
        let mut output = Vec::with_capacity(self.len());
        for element in self.elements.iter() {
            let mut reader = Reader::new(element.as_ref());
            let value = f(&mut reader)?;
            output.push(value)
        }
        Ok(output)
    }

    /// Turn a borrow Array into an Owned array
    ///
    /// Effectively all elements (represented by their slice `CborSlice`) is turn into owned
    /// data (`CborData`)
    pub fn owned(&self) -> ArrayOwned {
        ArrayOwned {
            len_encoding: self.len_encoding.clone(),
            elements: self
                .elements
                .iter()
                .map(|slice| (*slice).to_owned())
                .collect::<Vec<CborData>>(),
        }
    }
}

impl ArrayOwned {
    /// Get the structure length of the Array
    pub fn struct_len(&self) -> StructureLength {
        self.len_encoding
    }

    /// Return the number of CBOR element in this Array
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Get an iterator to the reader of each element of the array
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = Reader<'a>> {
        self.elements
            .iter()
            .map(|v| v.borrow())
            .map(|v: &'a CborSlice| v.reader())
    }

    /// Try to turn an array of CBOR element into a homogenous Vec of T,
    /// where each reader is processed with the function f in parameter
    pub fn to_vec<'a, F, T: Decode>(&'a self, f: F) -> Result<Vec<T>, DecodeErrorKind>
    where
        F: Fn(&mut Reader<'a>) -> Result<T, DecodeErrorKind>,
    {
        let mut output = Vec::with_capacity(self.len());
        for element in self.elements.iter() {
            let mut reader = Reader::new(element.as_ref());
            let value = f(&mut reader)?;
            output.push(value)
        }
        Ok(output)
    }

    /// Turn a Owned array into a Borrowed Array
    ///
    /// Effectively all elements (represented by their data `CborData`) is turn into borrowed slice
    /// (`CborSlice`)
    pub fn borrow<'a>(&'a self) -> Array<'a> {
        Array {
            len_encoding: self.len_encoding.clone(),
            elements: self
                .elements
                .iter()
                .map(|v| v.borrow())
                .collect::<Vec<&'a CborSlice>>(),
        }
    }
}

impl ArrayBuilder {
    /// Create a new array builder
    pub fn new() -> Self {
        Self { elements: vec![] }
    }

    /// Append a new data into the builder
    pub fn append(&mut self, data: CborData) {
        self.elements.push(data)
    }

    /// Add a Encoded T in the array.
    pub fn append_encodable<T: Encode>(&mut self, t: &T) {
        let mut writer = Writer::new();
        writer.encode(t);
        self.append(writer.finalize_data())
    }

    /// Terminate the array into 1 finite array
    pub fn finite(self) -> ArrayOwned {
        ArrayOwned {
            len_encoding: StructureLength::from(self.elements.len() as u64),
            elements: self.elements,
        }
    }

    /// Terminate the array into indefinite array
    pub fn indefinite(self) -> ArrayOwned {
        ArrayOwned {
            len_encoding: StructureLength::Indefinite,
            elements: self.elements,
        }
    }
}

impl<'a> std::ops::Index<usize> for Array<'a> {
    type Output = &'a CborSlice;

    fn index(&self, index: usize) -> &Self::Output {
        &self.elements[index]
    }
}

impl std::ops::Index<usize> for ArrayOwned {
    type Output = CborData;

    fn index(&self, index: usize) -> &Self::Output {
        &self.elements[index]
    }
}

impl<'a> Map<'a> {
    /// Get the structure length of the Map
    pub fn struct_len(&self) -> StructureLength {
        self.len_encoding
    }

    /// Return the number of CBOR key-value pairs in this Map
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Get an iterator to the reader of each pair of element of the Map
    pub fn iter(&'a self) -> impl Iterator<Item = (Reader<'a>, Reader<'a>)> {
        self.elements.iter().map(|(k, v)| (k.reader(), v.reader()))
    }

    /// Get an iterator to the reader of each keys of the Map
    pub fn keys(&'a self) -> impl Iterator<Item = Reader<'a>> {
        self.elements.iter().map(|(k, _v)| (k.reader()))
    }

    /// Get an iterator to the reader of each values of the Map
    pub fn values(&'a self) -> impl Iterator<Item = Reader<'a>> {
        self.elements.iter().map(|(_k, v)| (v.reader()))
    }

    /// Try to turn a map of pair of CBOR elements into a homogenous Vec of (K,V),
    /// where each key reader is processed with the `f` decoding function
    /// and each value reader is processed with the `g` decoding function
    pub fn to_vec<F, G, K: Decode, V: Decode>(
        &self,
        f: F,
        g: G,
    ) -> Result<Vec<(K, V)>, DecodeErrorKind>
    where
        F: for<'b> Fn(&mut Reader<'b>) -> Result<K, DecodeErrorKind>,
        G: for<'b> Fn(&mut Reader<'b>) -> Result<V, DecodeErrorKind>,
    {
        let mut output = Vec::with_capacity(self.len());
        for (k, v) in self.elements.iter() {
            let mut reader_k = Reader::new(k.as_ref());
            let key = f(&mut reader_k)?;

            let mut reader_v = Reader::new(v.as_ref());
            let value = g(&mut reader_v)?;
            output.push((key, value))
        }
        Ok(output)
    }

    /// Turn a Map into an Owned Map
    pub fn owned(&self) -> MapOwned {
        MapOwned {
            len_encoding: self.len_encoding.clone(),
            elements: self
                .elements
                .iter()
                .map(|(slice1, slice2)| ((*slice1).to_owned(), (*slice2).to_owned()))
                .collect::<Vec<(CborData, CborData)>>(),
        }
    }
}

impl MapOwned {
    /// Get the structure length of the Map
    pub fn struct_len(&self) -> StructureLength {
        self.len_encoding
    }

    /// Return the number of CBOR key-value pairs in this Map
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (Reader<'a>, Reader<'a>)> {
        self.elements.iter().map(|(k, v)| (k.read(), v.read()))
    }

    pub fn keys<'a>(&'a self) -> impl Iterator<Item = Reader<'a>> {
        self.elements.iter().map(|(k, _v)| k.read())
    }

    pub fn values<'a>(&'a self) -> impl Iterator<Item = Reader<'a>> {
        self.elements.iter().map(|(_k, v)| v.read())
    }

    pub fn to_vec<F, G, K: Decode, V: Decode>(
        &self,
        f: F,
        g: G,
    ) -> Result<Vec<(K, V)>, DecodeErrorKind>
    where
        F: for<'b> Fn(&mut Reader<'b>) -> Result<K, DecodeErrorKind>,
        G: for<'b> Fn(&mut Reader<'b>) -> Result<V, DecodeErrorKind>,
    {
        let mut output = Vec::with_capacity(self.len());
        for (k, v) in self.elements.iter() {
            let mut reader_k = Reader::new(k.as_ref());
            let key = f(&mut reader_k)?;

            let mut reader_v = Reader::new(v.as_ref());
            let value = g(&mut reader_v)?;
            output.push((key, value))
        }
        Ok(output)
    }

    pub fn borrow<'a>(&'a self) -> Map<'a> {
        Map {
            len_encoding: self.len_encoding.clone(),
            elements: self
                .elements
                .iter()
                .map(|(k, v)| (k.borrow(), v.borrow()))
                .collect::<Vec<(&'a CborSlice, &'a CborSlice)>>(),
        }
    }
}

impl<'a> std::ops::Index<usize> for Map<'a> {
    type Output = (&'a CborSlice, &'a CborSlice);

    fn index(&self, index: usize) -> &Self::Output {
        &self.elements[index]
    }
}

impl TagValue {
    /// Get the underlying Value for this Tag value
    pub fn raw_value(self) -> Value {
        self.0
    }

    pub fn to_u64(&self) -> u64 {
        self.0.to_u64()
    }

    pub fn from_u64(v: u64) -> Self {
        Self(Value::canonical(v))
    }
}

impl<'a> Tag<'a> {
    pub fn tag_repr(&self) -> TagValue {
        self.tag_val
    }

    pub fn value(&self) -> u64 {
        self.tag_val.to_u64()
    }

    pub fn data(&self) -> &'a CborSlice {
        &self.data
    }

    pub fn reader(&self) -> Reader<'a> {
        self.data.reader()
    }

    pub fn read_data<F, T>(&self, f: F) -> Result<T, ReaderError>
    where
        F: FnOnce(&mut Reader<'a>) -> Result<T, ReaderError>,
    {
        let mut reader: Reader<'a> = self.data.reader();
        let t = f(&mut reader)?;
        reader.expect_finished()?;
        Ok(t)
    }

    pub fn decode_data<T: Decode>(&self) -> Result<T, DecodeError> {
        let mut reader: Reader<'a> = self.data.reader();
        let t = <T>::decode(&mut reader)?;
        reader
            .expect_finished()
            .map_err(DecodeErrorKind::ReaderError)
            .map_err(|e| e.context::<Self>())?;
        Ok(t)
    }

    pub fn owned(&self) -> TagOwned {
        TagOwned {
            tag_val: self.tag_val,
            data: self.data.to_owned(),
        }
    }
}

impl TagOwned {
    pub fn tag_repr(&self) -> TagValue {
        self.tag_val
    }

    pub fn value(&self) -> u64 {
        self.tag_val.to_u64()
    }

    pub fn data(&self) -> &CborData {
        &self.data
    }

    pub fn read_data<'a>(&'a self) -> Reader<'a> {
        self.data.read()
    }

    pub fn borrow<'a>(&'a self) -> Tag<'a> {
        Tag {
            tag_val: self.tag_val,
            data: self.data.borrow(),
        }
    }
}
