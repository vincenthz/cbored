# 0.3.4 (2023-08-19)

- Fix bug with parsing Bytes chunks raising wrong error type and advancing the parser spuriously
- Add some encoding accessor (value) and raw data for Bytes and Text chunks

# 0.3.3 (2022-08-26)

- add writer and decode/encode implementation for the Scalar type

# 0.3.2 (2022-08-25)

- add `encode_vec` and `decode_vec`

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

