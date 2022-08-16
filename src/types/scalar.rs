use super::super::header::{Value, Value8};

/// CBOR Positive value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Positive(pub(crate) Value);

/// CBOR Negative value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Negative(pub(crate) Value);

/// CBOR Byte value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Byte(pub(crate) Value8);

/// CBOR constants (False/True/Null/Undefined)
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum Constant {
    False,
    True,
    Null,
    Undefined,
}

impl Positive {
    /// Extract the positive CBOR value into an unsigned value
    pub fn to_u64(&self) -> u64 {
        self.0.to_u64()
    }

    /// Create a canonical Positive element from a u64,
    /// taking the smallest possible CBOR representation
    pub fn canonical(v: u64) -> Self {
        Self(Value::canonical(v))
    }

    /// Check if the encoded Positive CBOR element have
    /// the smallest representation possible (canonical)
    pub fn is_canonical(&self) -> bool {
        self.0.is_canonical()
    }
}

impl Negative {
    /// Extract the negative CBOR value into an unsigned value, which represent
    /// the integer : -1 - value
    pub fn negative_u64(&self) -> u64 {
        self.0.to_u64()
    }

    /// Try to convert a negative CBOR number into a i64 representing the value
    ///
    /// Note this operation might fail as the CBOR representation can represent
    /// any negative number between -1 and -(2^64-1), whereas i64 represent
    /// a number between -(2^63) to 2^63-1
    pub fn to_i64(&self) -> Option<i64> {
        // use `checked_sub_unsigned` when out of nightly
        // [https://github.com/rust-lang/rust/issues/87840]
        i64::try_from(self.0.to_u64())
            .ok()
            .and_then(|v| (-1i64).checked_sub(v))
    }

    /// Create a canonical Positive element from a u64,
    /// taking the smallest possible CBOR representation
    pub fn canonical(v: u64) -> Self {
        Self(Value::canonical(v))
    }

    /// Check if the encoded Positive CBOR element have
    /// the smallest representation possible (canonical)
    pub fn is_canonical(&self) -> bool {
        self.0.is_canonical()
    }
}
