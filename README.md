# cbored - Exact CBOR

cbored is a CBOR reader and writer, focused on exact 1-1 CBOR representation.

## Design decision

This CBOR crate is based on the following decisions:

* Keep close to the CBOR types
* Able to recover the exact bytes of a given encoded CBOR type, specially in the case of non-canonical CBOR use
* Hide the indefinite/definite possible options in given CBOR types to keep usage simple

The main use cases is to deal with CBOR data stream that are not requiring one
canonical given representation (standard CBOR canonical or another) and when the
data serialized need to be kept as is (e.g. when used in cryptographic settings)

## Caveat Emptor

* signed integer are not particularly well supported, due to not being used in my use cases, so there's quite a few missing conversion; contribution welcome

## Auto CBOR Encode and Decode derive

Automatic proc-macro derive can be enabled in the `Cargo.toml`:

```
cbored = { version = "0.1", features = ["derive"] }
```

Which allow to derive Decode and Encode implementation for enum and struct types:

```rust
#[derive(CborRepr)]
#[cborrepr(structure = "array")]
// serialized as : ARRAY(2) [UINT, UINT]
pub struct Point {
    x: u32,
    y: u32,
}

#[derive(CborRepr)]
#[cborrepr(structure = "array_lastopt")]
// serialized as : ARRAY(2) [UINT, UINT]
//            or : ARRAY(1) [UINT]
pub struct Point {
    x: u32,
    y: Option<u32>,
}

#[derive(CborRepr)]
#[cborrepr(structure = "flat")]
// serialized as : UINT, UINT
pub struct FlatPoint {
    x: u32,
    y: u32,
}

#[derive(CborRepr)]
#[cborrepr(structure = "mapint")]
// serialized as : MAP(2) { UINT(0) => UINT, UINT(1) => UINT }
//            or : MAP(3) { UINT(0) => UINT, UINT(1) => UINT, UINT(2) => UINT }
pub struct FlatPoint {
    #[cborrepr(mandatory)]
    x: u32,
    #[cborrepr(mandatory)]
    y: u32,
    z: Option<u32>
}

#[derive(CborRepr)]
#[cborrepr(enumtype = "tagvariant")]
// serialized as
// * One : ARRAY(2) [ UINT(0), UINT ]
// * Two : ARRAY(3) [ UINT(1), UINT, TEXT ]
// * Three : ARRAY(2) [ UINT(2), ARRAY(2) [ UINT, UINT ] ]
// * Four : ARRAY(1) [ UINT(3) ]
pub enum Variant {
    One(u32),
    Two(u64, String),
    Three(Point),
    Four,
}

#[derive(CborRepr)]
#[cborrepr(enumtype = "enumint")]
// serialized as
// * Code1 : UINT(0)
// * Code2 : UINT(1)
// * Code3 : UINT(2)
pub enum Code {
    Code1,
    Code2,
    Code3,
}

#[derive(CborRepr)]
#[cborrepr(enumtype = "enumtype")]
// serialized as
// * Empty : NULL
// * One   : UINT
pub enum OneOrEmpty {
    #[cborrepr(cbortype = "null")]
    Empty,
    #[cborrepr(cbortype = "positive")]
    One(u64),
}

```


Structure:

* `array`: the structure is serialized one after another inside an array of the length reflecting the number of elements
* `flat`: each field is serialized one after another, using the Decode/Encode instance of each type. not recommended in general case, as it doesn't play nice with array / map structure.
* `mapint`: the structure is serialized as a map, where the key index is the index of the field relative to the `map_starts_at` argument (if not present starts at 0)

Enums :

* `tagvariant`: array with a leading integer representing the variant, following by any fields in the 
* `enumint`: just an integer for variant with no inner element. integer is sequentially incremented between variant, and starts at 0
