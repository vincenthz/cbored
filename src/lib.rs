//! CBOR exact reader and writer
//!
//! ```
//! use cbored::Reader;
//! let mut reader = Reader::new(&[0x80]);
//! let array = reader.array().expect("valid array");
//! assert_eq!(array.len(), 0);
//! ```
//!
//! ```
//! use cbored::{Writer, StructureLength, Positive};
//! let mut writer = Writer::new();
//! writer.array_build(StructureLength::Indefinite, |writer| {
//!     writer.positive(Positive::canonical(10));
//! })

mod context;
mod prim;

mod reader;
mod writer;

mod decode;
mod encode;

pub mod tagged;

mod lowlevel;

pub(crate) mod header;
pub mod state;
mod types;
pub mod validate;

pub use reader::{Reader, ReaderError};
pub use writer::Writer;

pub use decode::{decode_vec, Decode, DecodeError, DecodeErrorKind};
pub use encode::{encode_vec, Encode};

pub use prim::{CborDataOf, CborSliceOf, CborSlice};
pub use types::*;

#[cfg(feature = "derive")]
pub use cbored_derive::CborRepr;

/// Try to decode bytes into T from its CBOR bytes representation
pub fn decode_from_bytes<T: Decode>(slice: &[u8]) -> Result<T, DecodeError> {
    let mut reader = Reader::new(slice);
    let t = reader.decode()?;
    reader
        .expect_finished()
        .map_err(DecodeErrorKind::ReaderError)
        .map_err(|e| e.context::<T>())?;
    Ok(t)
}

/// Encode an encodable type T into its CBOR bytes representation
pub fn encode_to_bytes<T: Encode>(t: &T) -> Vec<u8> {
    let mut writer = Writer::new();
    t.encode(&mut writer);
    writer.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test1() {
        let mut writer = Writer::new();
        writer.constant(Constant::True);
        let cbor = writer.finalize();

        let mut reader = Reader::new(&cbor);
        let con = reader.constant().unwrap();
        assert_eq!(reader.is_finished(), true);
        assert_eq!(con, Constant::True);
    }

    #[test]
    fn test2() {
        let mut writer = Writer::new();
        writer.constant(Constant::True);
        writer.positive(Positive::canonical(124));
        let cbor = writer.finalize();

        let mut reader = Reader::new(&cbor);
        let con = reader.constant().unwrap();
        let pos = reader.positive().unwrap();
        assert_eq!(reader.is_finished(), true);
        assert_eq!(con, Constant::True);
        assert_eq!(pos.to_u64(), 124);
    }

    #[test]
    fn test_err_reading_type() {
        let mut writer = Writer::new();
        writer.constant(Constant::True);
        writer.positive(Positive::canonical(124));
        let cbor = writer.finalize();

        let mut reader = Reader::new(&cbor);
        let con = reader.constant().unwrap();

        // try to read a different type than the expected positive should
        // error, and also not consume anything, so that the next correct reading
        // will work
        assert_eq!(reader.negative().is_err(), true);

        let pos = reader.positive().unwrap();
        assert_eq!(reader.is_finished(), true);
        assert_eq!(con, Constant::True);
        assert_eq!(pos.to_u64(), 124);
    }

    pub struct Inner(u64, bool);

    impl Decode for Inner {
        fn decode<'a>(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
            let a = reader
                .array()
                .map_err(DecodeErrorKind::ReaderError)
                .map_err(|e| e.context::<Self>())?;
            if a.len() != 2 {
                return Err(
                    DecodeErrorKind::Custom(format!("expectig length of 2")).context::<Self>()
                );
            }

            let i = a[0].decode().expect("inner integer");
            let b = a[1].decode().expect("inner bool");
            Ok(Inner(i, b))
        }
    }

    #[test]
    fn test_decode() {
        const DATA: &[u8] = &[0x83, 0x1, 0x7, 0x82, 0x1A, 0x41, 0x70, 0xCB, 0x17, 0xF4];

        let mut r = Reader::new(DATA);
        let a = r.array().expect("array");
        assert_eq!(a.len(), 3);
        let e0: u64 = a[0].decode().expect("u64 (1)");
        assert_eq!(e0, 1);

        let e1: u64 = a[1].decode().expect("u64 (2)");
        assert_eq!(e1, 7);

        let e2: Inner = a[2].decode().expect("e2");
        assert_eq!(e2.0, 1097911063);
        assert_eq!(e2.1, false);
        assert!(r.is_finished());
    }

    #[test]
    fn test_map_array() {
        const DATA: &[u8] = &[
            0x83, 0xa4, 0x00, 0x82, 0x82, 0x58, 0x20, 0x3b, 0x40, 0x26, 0x51, 0x11, 0xd8, 0xbb,
            0x3c, 0x3c, 0x60, 0x8d, 0x95, 0xb3, 0xa0, 0xbf, 0x83, 0x46, 0x1a, 0xce, 0x32, 0xd7,
            0x93, 0x36, 0x57, 0x9a, 0x19, 0x39, 0xb3, 0xaa, 0xd1, 0xc0, 0xb7, 0x18, 0x2a, 0x82,
            0x58, 0x20, 0x82, 0x83, 0x9f, 0x82, 0x00, 0xd8, 0x18, 0x58, 0x24, 0x82, 0x58, 0x20,
            0x3b, 0x40, 0x26, 0x51, 0x11, 0xd8, 0xbb, 0x3c, 0x3c, 0x60, 0x8d, 0x95, 0xb3, 0xa0,
            0xbf, 0x83, 0x46, 0x1a, 0xce, 0x32, 0x07, 0x01, 0x82, 0x82, 0x58, 0x1d, 0x61, 0x1c,
            0x61, 0x6f, 0x1a, 0xcb, 0x46, 0x06, 0x68, 0xa9, 0xb2, 0xf1, 0x23, 0xc8, 0x03, 0x72,
            0xc2, 0xad, 0xad, 0x35, 0x83, 0xb9, 0xc6, 0xcd, 0x2b, 0x1d, 0xee, 0xed, 0x1c, 0x19,
            0x01, 0x21, 0x82, 0x58, 0x1d, 0x61, 0xbc, 0xd1, 0x8f, 0xcf, 0xfa, 0x79, 0x7c, 0x16,
            0xc0, 0x07, 0x01, 0x4e, 0x2b, 0x85, 0x53, 0xb8, 0xb9, 0xb1, 0xe9, 0x4c, 0x50, 0x76,
            0x88, 0x72, 0x62, 0x43, 0xd6, 0x11, 0x1a, 0x34, 0x20, 0x98, 0x9c, 0x02, 0x1a, 0x00,
            0x16, 0x90, 0x3a, 0x03, 0x19, 0x03, 0xe7, 0xa1, 0x00, 0x82, 0x82, 0x58, 0x20, 0xf9,
            0xaa, 0x3f, 0xcc, 0xb7, 0xfe, 0x53, 0x9e, 0x47, 0x11, 0x88, 0xcc, 0xc9, 0xee, 0x65,
            0x51, 0x4c, 0x59, 0x61, 0xc0, 0x70, 0xb0, 0x6c, 0xa1, 0x85, 0x96, 0x24, 0x84, 0xa4,
            0x81, 0x3b, 0xee, 0x58, 0x40, 0x93, 0x8c, 0xd3, 0xfe, 0xb7, 0x31, 0xfe, 0x67, 0x49,
            0xc9, 0xb2, 0xb1, 0x27, 0xd3, 0x95, 0x88, 0x21, 0xfc, 0x76, 0x49, 0xb3, 0x37, 0xf4,
            0xaa, 0xae, 0x10, 0x5b, 0xf5, 0x1a, 0xd7, 0x59, 0x06, 0xae, 0xc0, 0x11, 0x2f, 0xb4,
            0x0e, 0xcc, 0x2c, 0x2e, 0x7c, 0x00, 0x59, 0x2a, 0x1e, 0xc5, 0xac, 0x59, 0xdd, 0xf3,
            0x1a, 0xd6, 0x04, 0x88, 0x63, 0x64, 0xf7, 0x8d, 0xbf, 0x6d, 0xa8, 0xfe, 0x0c, 0x82,
            0x58, 0x20, 0x68, 0x72, 0xb0, 0xa8, 0x74, 0xac, 0xfe, 0x1c, 0xac, 0xe1, 0x2b, 0x20,
            0xea, 0x34, 0x85, 0x59, 0xa7, 0xec, 0xc9, 0x12, 0xf2, 0xfc, 0x7f, 0x67, 0x4f, 0x43,
            0x48, 0x1d, 0xf9, 0x73, 0xd9, 0x2c, 0x58, 0x40, 0x2a, 0x44, 0x2f, 0xac, 0xbd, 0xe5,
            0xb6, 0x7d, 0x7d, 0x15, 0x58, 0x1e, 0x1b, 0x59, 0xee, 0x44, 0xb3, 0x75, 0x0b, 0xd0,
            0x18, 0xf0, 0x1a, 0x2f, 0xec, 0xb4, 0xbc, 0x82, 0xb5, 0x58, 0xc9, 0x0f, 0x94, 0x6a,
            0xfc, 0xb9, 0xf0, 0x3f, 0x18, 0xe7, 0xff, 0xb4, 0x0b, 0xd5, 0x9c, 0x47, 0x93, 0x87,
            0x1b, 0x92, 0x4a, 0xb7, 0x07, 0xcf, 0xfd, 0xfd, 0xa1, 0x0d, 0xd5, 0x59, 0x5e, 0x29,
            0xe7, 0x09, 0xf6,
        ];

        let mut r = Reader::new(DATA);
        let a = r.array().expect("array");
        assert_eq!(a.len(), 3);

        {
            let mut r = a[0].reader();
            let map = r.map().expect("a[0] map");
            assert_eq!(map.len(), 4);

            {
                let (k, v) = map[0];
                {
                    let mut r = k.reader();
                    let p = r.positive().expect("positive key[0]");
                    assert_eq!(p.to_u64(), 0);
                    assert!(r.is_finished());
                }
                {
                    let mut r = v.reader();
                    let array = r.array().expect("array value[0]");
                    assert_eq!(array.len(), 2);
                    assert!(r.is_finished());
                }
            }

            assert!(r.is_finished());
        }

        {
            let mut r = a[1].reader();
            let map = r.map().expect("a[1] map");
            assert_eq!(map.len(), 1);
            assert!(r.is_finished());
        }

        {
            let mut r = a[2].reader();
            r.null().expect("byte");
            assert!(r.is_finished());
        }

        assert!(r.is_finished());
    }
}
