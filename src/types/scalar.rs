use super::super::header::{HeaderValue, HeaderValue8};

/// CBOR Positive value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Positive(pub(crate) HeaderValue);

/// CBOR Negative value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Negative(pub(crate) HeaderValue);

/// Union type of CBOR Positive or Negative value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scalar {
    Positive(Positive),
    Negative(Negative),
}

/// CBOR Byte value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Byte(pub(crate) HeaderValue8);

/// CBOR constants (False/True/Null/Undefined)
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum Constant {
    False,
    True,
    Null,
    Undefined,
}

impl Positive {
    /// Get the underlying Value for this Positive value
    pub fn raw_value(&self) -> HeaderValue {
        self.0
    }

    /// Extract the positive CBOR value into an unsigned value
    pub fn to_u64(&self) -> u64 {
        self.0.to_u64()
    }

    /// Create a canonical Positive element from a u64,
    /// taking the smallest possible CBOR representation
    pub fn canonical(v: u64) -> Self {
        Self(HeaderValue::canonical(v))
    }

    /// Check if the encoded Positive CBOR element have
    /// the smallest representation possible (canonical)
    pub fn is_canonical(&self) -> bool {
        self.0.is_canonical()
    }
}

impl Negative {
    /// Get the underlying Value for this Negative value
    pub fn raw_value(self) -> HeaderValue {
        self.0
    }

    /// Extract the negative CBOR value into an unsigned value, which represent
    /// the integer : -1 - value
    pub fn negative_u64(self) -> u64 {
        self.0.to_u64()
    }

    /// Try to convert a negative CBOR number into a i64 representing the value
    ///
    /// Note this operation might fail as the CBOR representation can represent
    /// any negative number between -1 and -(2^64-1), whereas i64 represent
    /// a number between -(2^63) to 2^63-1
    pub fn to_i64(self) -> Option<i64> {
        // use `checked_sub_unsigned` when out of nightly
        // [https://github.com/rust-lang/rust/issues/87840]
        i64::try_from(self.0.to_u64())
            .ok()
            .and_then(|v| (-1i64).checked_sub(v))
    }

    /// Create a canonical Negative element from a u64 that represent the CBOR integer -1 - value,
    /// taking the smallest possible CBOR representation
    ///
    /// To represent the CBOR negative value -1, `Negative::canonical(0)`
    pub fn canonical(v: u64) -> Self {
        Self(HeaderValue::canonical(v))
    }

    /// Check if the encoded Negative CBOR element have
    /// the smallest representation possible (canonical)
    pub fn is_canonical(self) -> bool {
        self.0.is_canonical()
    }
}

impl Scalar {
    /// Return whether the scalar is a positive encoding
    pub fn is_positive(self) -> bool {
        match self {
            Scalar::Positive(_) => true,
            Scalar::Negative(_) => false,
        }
    }

    /// Return whether the scalar is a negative encoding
    pub fn is_negative(self) -> bool {
        !self.is_positive()
    }

    /// Get the underlying Value for this Negative value
    pub fn raw_value(self) -> HeaderValue {
        match self {
            Scalar::Positive(v) => v.raw_value(),
            Scalar::Negative(v) => v.raw_value(),
        }
    }

    /// Try to convert a negative CBOR number into a i64 representing the value
    ///
    /// Note this operation might fail as the CBOR representation can represent
    /// any negative number between -1 and -(2^64-1), and positive number any number
    /// between 0 and 2^64-1 whereas i64 represent
    /// a number between -(2^63) to 2^63-1
    pub fn to_i64(self) -> Option<i64> {
        match self {
            Scalar::Positive(v) => i64::try_from(v.to_u64()).ok(),
            Scalar::Negative(n) => n.to_i64(),
        }
    }

    /// Create a canonical Positive element from a u64 that represent the CBOR integer 0 to 2^64
    /// taking the smallest possible CBOR representation
    pub fn canonical_positive(v: u64) -> Self {
        Self::Positive(Positive::canonical(v))
    }

    /// Create a canonical Negative element from a u64 that represent the CBOR integer -1 to -2^64,
    /// taking the smallest possible CBOR representation
    pub fn canonical_negative(v: u64) -> Self {
        Self::Negative(Negative::canonical(v))
    }

    /// Check if the encoded Negative CBOR element have
    /// the smallest representation possible (canonical)
    pub fn is_canonical(&self) -> bool {
        match self {
            Scalar::Positive(v) => v.is_canonical(),
            Scalar::Negative(v) => v.is_canonical(),
        }
    }
}

impl Byte {
    /// Get the underlying Value8 for this Negative value
    pub fn raw_value(self) -> HeaderValue8 {
        self.0
    }

    pub fn to_u8(self) -> u8 {
        self.0.to_u8()
    }

    pub fn canonical(v: u8) -> Self {
        Byte(HeaderValue8::canonical(v))
    }

    pub fn is_canonical(self) -> bool {
        self.0.is_canonical()
    }
}
