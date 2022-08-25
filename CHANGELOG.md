# 0.3.1 (2022-08-18)

- dependency mismatch between cbored and cbored-derive

# 0.3.0 (2022-08-18)

- add new `enumtype` strategy for enums, to allow the serialization to be based on the cbor type of the object
- add new `array_lastopt` strategy for struct, to allow optional last field
- add new `mapint` strategy for struct, serializing record using their index from start of the structure as integral index for a map
- add new methods to handle negative values
- add new Scalar type to handle mixed positive/negative type

# 0.2.0 (2022-08-16)

- add record contextual ability for `DecodeError`
- add PartialEq and Eq for most CBOR types 
- add cbored-derive ability to encode and decode structure with CBOR maps
- document cbored-derive code a bit more to make the code more easy to follow

### Notes

- In this case, the equality trait is structural, not representation, so for
  example a CBOR int encoded in 1 byte of the value 10, and one encoded in 2
  bytes of the same value, will not be equal. At a later date, it could be
  valuable to add a representational Eq trait

### Breaking Changes
    
- `DecodeError` is now `DecodeErrorKind` and `DecodeError` is now
  a `DecodeErrorKind` along with a user driven context recorder
  in a form of strings.
