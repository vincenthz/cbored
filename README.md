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
#[cborrepr(structure = "flat")]
// serialized as : UINT, UINT
pub struct FlatPoint {
    x: u32,
    y: u32,
}

#[derive(CborRepr)]
#[cborrepr(enumtype = "tagvariant")]
// serialized as
// * One : ARRAY(2) [ UINT(0), ARRAY(2) [UINT, UINT] ]
// * Two : ARRAY(3) [ UINT(1), ARRAY(2) [UINT, UINT], ARRAY(2) [UINT, UINT] ]
pub enum Variant {
    One(Point),
    Two(Point, Point),
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
```


Structure:

* `array`: the structure is serialized one after another inside an array of the length reflecting the number of elements
* `flat`: each field is serialized one after another, using the Decode/Encode instance of each type. not recommended in general case, as it doesn't play nice with array / map structure.

Enums :

* `tagvariant`: array with a leading integer representing the variant, following by any fields in the 
* `enumint`: just an integer for variant with no inner element. integer is sequentially incremented between variant, and starts at 0
