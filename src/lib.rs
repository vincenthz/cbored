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

mod reader;
mod writer;

mod decode;
mod encode;

pub mod tagged;

pub(crate) mod header;
mod lead;
pub mod state;
mod types;
pub mod validate;

pub use reader::{Reader, ReaderError};
pub use writer::Writer;

pub use decode::{Decode, DecodeError};
pub use encode::Encode;

pub use types::*;

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
            let a = reader.array()?;
            if a.len() != 2 {
                return Err(DecodeError::Custom(format!("expectig length of 2")));
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
}
