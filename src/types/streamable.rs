use super::super::header::Value;

/// CBOR Bytestream (indefinite and definite) with reference to the bytes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bytes<'a> {
    Imm(BytesData<'a>),
    Chunks(Vec<BytesData<'a>>),
}

/// CBOR Bytestream (indefinite and definite) with owned bytes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BytesOwned {
    Imm(BytesDataOwned),
    Chunks(Vec<BytesDataOwned>),
}

/// CBOR Bytestream reference to a chunk of byte
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytesData<'a>(pub(crate) Value, pub(crate) &'a [u8]);

/// CBOR Bytestream owned chunk of byte
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytesDataOwned(pub(crate) Value, pub(crate) Vec<u8>);

/// CBOR Text (UTF-8) (indefinite and definite) with reference to text chunk
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Text<'a> {
    Imm(TextData<'a>),
    Chunks(Vec<TextData<'a>>),
}

/// CBOR Text (UTF-8) (indefinite and definite) with owned text chunk
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextOwned {
    Imm(TextDataOwned),
    Chunks(Vec<TextDataOwned>),
}

/// CBOR Text chunk with reference to the chunk of utf8 sequence
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextData<'a>(pub(crate) Value, pub(crate) &'a str);

/// CBOR Text chunk with owned chunk of utf8 sequence
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDataOwned(pub(crate) Value, pub(crate) String);

impl<'a> Bytes<'a> {
    pub fn len(&self) -> usize {
        match self {
            Bytes::Imm(bd) => bd.1.len(),
            Bytes::Chunks(chunks) => chunks.iter().fold(0, |acc, c| acc + c.1.len()),
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        match self {
            Bytes::Imm(bd) => bd.1.to_vec(),
            Bytes::Chunks(chunks) => chunks.iter().fold(Vec::new(), |mut v, c| {
                v.extend_from_slice(c.1);
                v
            }),
        }
    }

    pub fn from_slice(slice: &'a [u8]) -> Self {
        Bytes::Imm(BytesData::from_slice(slice))
    }

    pub fn owned(&self) -> BytesOwned {
        match self {
            Bytes::Imm(bd) => BytesOwned::Imm(bd.owned()),
            Bytes::Chunks(chunks) => {
                BytesOwned::Chunks(chunks.iter().map(|bd| bd.owned()).collect())
            }
        }
    }
}

impl<'a> Text<'a> {
    pub fn to_string(&self) -> String {
        match self {
            Text::Imm(bd) => bd.1.to_string(),
            Text::Chunks(chunks) => chunks.iter().fold(String::new(), |mut v, c| {
                v.push_str(c.1);
                v
            }),
        }
    }

    pub fn to_str_chunks(&self) -> Vec<&'a str> {
        match self {
            Text::Imm(td) => vec![td.as_ref()],
            Text::Chunks(td) => td.iter().map(|td| *td.as_ref()).collect(),
        }
    }

    pub fn from_str(str: &'a str) -> Self {
        Text::Imm(TextData::from_str(str))
    }

    pub fn owned(&self) -> TextOwned {
        match self {
            Text::Imm(bd) => TextOwned::Imm(bd.owned()),
            Text::Chunks(chunks) => TextOwned::Chunks(chunks.iter().map(|bd| bd.owned()).collect()),
        }
    }
}

impl<'a> TextData<'a> {
    pub fn value(&self) -> Value {
        self.0
    }

    pub fn as_str(&self) -> &'a str {
        self.1
    }

    pub fn from_str(str: &'a str) -> Self {
        TextData(Value::canonical(str.len() as u64), str)
    }

    pub fn owned(&self) -> TextDataOwned {
        TextDataOwned(self.0, self.1.to_string())
    }
}

impl TextDataOwned {
    pub fn value(&self) -> Value {
        self.0
    }

    pub fn as_str(&self) -> &str {
        &self.1
    }

    pub fn borrow<'a>(&'a self) -> TextData<'a> {
        TextData(self.0, self.1.as_str())
    }

    pub fn from_string(string: String) -> Self {
        TextDataOwned(Value::canonical(string.len() as u64), string)
    }
}

impl<'a> AsRef<&'a str> for TextData<'a> {
    fn as_ref(&self) -> &&'a str {
        &self.1
    }
}

impl<'a> BytesData<'a> {
    pub fn value(&self) -> Value {
        self.0
    }
    pub fn as_slice(&self) -> &'a [u8] {
        self.1
    }
    pub fn from_slice(slice: &'a [u8]) -> Self {
        BytesData(Value::canonical(slice.len() as u64), slice)
    }
    pub fn owned(&self) -> BytesDataOwned {
        BytesDataOwned(self.0, self.1.to_vec())
    }
}

impl BytesDataOwned {
    pub fn value(&self) -> Value {
        self.0
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.1
    }
    pub fn borrow<'a>(&'a self) -> BytesData<'a> {
        BytesData(self.0, self.1.as_ref())
    }

    pub fn from_vec(bytes: Vec<u8>) -> Self {
        BytesDataOwned(Value::canonical(bytes.len() as u64), bytes)
    }
}

impl BytesOwned {
    pub fn borrow<'a>(&'a self) -> Bytes<'a> {
        match self {
            BytesOwned::Imm(bd) => Bytes::Imm(bd.borrow()),
            BytesOwned::Chunks(vec) => Bytes::Chunks(vec.iter().map(|bd| bd.borrow()).collect()),
        }
    }

    pub fn from_vec(bytes: Vec<u8>) -> Self {
        BytesOwned::Imm(BytesDataOwned::from_vec(bytes))
    }
}

impl TextOwned {
    pub fn borrow<'a>(&'a self) -> Text<'a> {
        match self {
            TextOwned::Imm(bd) => Text::Imm(bd.borrow()),
            TextOwned::Chunks(vec) => Text::Chunks(vec.iter().map(|bd| bd.borrow()).collect()),
        }
    }

    pub fn from_string(string: String) -> Self {
        TextOwned::Imm(TextDataOwned::from_string(string))
    }
}
