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
