//! CBOR State
//!
//! a new empty state is created with `State::new()`
//!
//! Then the following API are used:
//!
//! * array: new array has been added
//! * map: new map has been added
//! * brk: process a CBOR break, which terminate an indefinite structure (bytes, texts, arrays, maps)
//! * text: add a text item
//! * bytes: add a bytes item
//! * tag: a new tag event
//! * simple: add a simple item (everything else from tag, array, map, bytes, text)
//!
//! * acceptable: check if the state is done

pub use super::header::{Value, ValueStream};
use std::fmt;

#[derive(Debug, Clone, Copy)]
enum StreamType {
    Array,
    Map(bool),
    Bytes,
    Text,
}

impl StreamType {
    pub fn composite_scalar(self) -> bool {
        match self {
            StreamType::Array | StreamType::Map(_) => false,
            StreamType::Bytes | StreamType::Text => true,
        }
    }
}

#[derive(Debug, Clone)]
enum StructTy {
    Array(usize),
    Map { exp_val: bool, elements: usize },
    Stream(StreamType),
    Tag(bool),
}

impl StructTy {
    /// check if an array/map is now empty, or a tag filled
    pub fn reduceable(&self) -> bool {
        match self {
            StructTy::Array(elements)
            | StructTy::Map {
                exp_val: _,
                elements,
            } => *elements == 0,
            StructTy::Tag(t) => *t,
            StructTy::Stream(_) => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum StateError {
    /// Break in non indefinite structure
    BreakInNonStreamable,
    /// Break without any structure
    BreakEmptyStack,
    /// indefinite value in indefinite text or indefinite bytes.
    ChunksInChunks,
    /// structure start in indefinite text or indefinite bytes.
    StructureInChunks,
    /// Structure end but structure still have some items to process
    StructureNotFinished,
    /// Structure end not in structure
    StructureEndNotInStructure,
    /// Trying to add a simple type like int,constants,... in a indefinite bytes/text
    InvalidTypeInChunk,
    /// Trying to close an unfinished Tag
    TagNotFinished,
    /// Tag in Chunk
    TagInChunk,
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateError::BreakInNonStreamable => write!(f, "break in non streamable"),
            StateError::BreakEmptyStack => write!(f, "break in empty stack"),
            StateError::ChunksInChunks => write!(f, "chunks in chunks"),
            StateError::StructureInChunks => write!(f, "structure in chunks"),
            StateError::StructureNotFinished => write!(f, "structure not finished"),
            StateError::StructureEndNotInStructure => write!(f, "structure end not in structure"),
            StateError::InvalidTypeInChunk => write!(f, "invalid type in chunk"),
            StateError::TagNotFinished => write!(f, "tag not finished"),
            StateError::TagInChunk => write!(f, "tag in chunk"),
        }
    }
}

pub struct State {
    ctx: Vec<StructTy>,
}

#[derive(Clone, Copy)]
pub enum ContextAction {
    None,
    Pop,
}

impl State {
    /// Create a new empty validation state
    pub fn new() -> Self {
        Self { ctx: vec![] }
    }

    /// return if the state is in a stable accepted position
    pub fn acceptable(&self) -> bool {
        self.ctx.is_empty()
    }

    fn push_stream(&mut self, ty: StreamType) -> Result<(), StateError> {
        // if there was a context already, then we check
        match self.ctx.last() {
            Some(StructTy::Stream(s)) if s.composite_scalar() => {
                return Err(StateError::StructureInChunks);
            }
            _ => (),
        };

        self.ctx.push(StructTy::Stream(ty));
        Ok(())
    }

    fn advance(&mut self) -> Result<(), StateError> {
        match self.ctx.last_mut() {
            None => Ok(()),
            Some(ctx) => match ctx {
                StructTy::Array(z) => {
                    assert_ne!(*z, 0, "array is empty");
                    *z = *z - 1;
                    Ok(())
                }
                StructTy::Map { exp_val, elements } => {
                    if *exp_val {
                        *exp_val = false;
                    } else {
                        assert_ne!(*elements, 0, "elements is empty");
                        *elements = *elements - 1;
                    }
                    Ok(())
                }
                StructTy::Tag(finished) => {
                    assert_ne!(*finished, true, "tag is finished already");
                    *finished = true;
                    Ok(())
                }
                StructTy::Stream(_sty) => Ok(()),
            },
        }
    }

    fn reduce(&mut self) -> Result<(), StateError> {
        match self.ctx.pop() {
            Some(StructTy::Array(elements)) => {
                if elements > 0 {
                    Err(StateError::StructureNotFinished)
                } else {
                    Ok(())
                }
            }
            Some(StructTy::Map {
                elements,
                exp_val: _,
            }) => {
                if elements > 0 {
                    Err(StateError::StructureNotFinished)
                } else {
                    Ok(())
                }
            }
            Some(StructTy::Tag(finished)) => {
                if finished {
                    Ok(())
                } else {
                    Err(StateError::TagNotFinished)
                }
            }
            Some(StructTy::Stream(_)) => Err(StateError::StructureNotFinished),
            None => Err(StateError::StructureEndNotInStructure),
        }?;
        Ok(())
    }

    /*
    fn reduce(&mut self) -> Result<(), StateError> {

    }
    */

    fn check_reduce(&mut self) -> Result<(), StateError> {
        loop {
            //self.advance()?;
            match self.ctx.last() {
                Some(structure) if structure.reduceable() => {
                    self.reduce()?;
                    self.advance()?;
                }
                _ => break,
            }
        }
        Ok(())
    }

    // text or bytes
    fn item_streamable(
        &mut self,
        v: ValueStream,
        new_stream: StreamType,
    ) -> Result<(), StateError> {
        match v {
            // beginning of a indefinite Text or indefinite Bytes
            None => {
                self.push_stream(new_stream)?;
                Ok(())
            }
            // A regular text or bytes, we can just call item_simple
            Some(_b) => {
                self.advance()?;
                self.check_reduce()
            }
        }
    }

    fn struct_start<F>(
        &mut self,
        v: ValueStream,
        stream: StreamType,
        f: F,
    ) -> Result<(), StateError>
    where
        F: FnOnce(usize) -> StructTy,
    {
        // in a bytes/texts stream we can't add a structure, only bytes/text
        match self.ctx.last() {
            Some(StructTy::Stream(s)) if s.composite_scalar() => {
                return Err(StateError::StructureInChunks);
            }
            _ => (),
        };

        // push a new struct (Array or Map) in the context
        match v {
            None => {
                self.push_stream(stream)?;
            }
            Some(number_of_items) => {
                let sz = number_of_items.to_size();
                self.ctx.push(f(sz));
                self.check_reduce()?;
            }
        };
        Ok(())
    }

    /// Add a simple value.
    ///
    /// All values except text, bytes, array, map, tag
    pub fn simple(&mut self) -> Result<(), StateError> {
        self.advance()?;
        self.check_reduce()
    }

    /// Add a Text value.
    ///
    /// Take the ValueStream in parameter to keep track whether the event
    /// is using indefinite or definite text.
    pub fn text(&mut self, v: ValueStream) -> Result<(), StateError> {
        self.item_streamable(v, StreamType::Text)
    }

    /// Add a Bytes value.
    ///
    /// Take the ValueStream in parameter to keep track whether the event
    /// is using indefinite or definite bytes.
    pub fn bytes(&mut self, v: ValueStream) -> Result<(), StateError> {
        self.item_streamable(v, StreamType::Bytes)
    }

    /// Add a new Array into the context
    pub fn array(&mut self, v: ValueStream) -> Result<(), StateError> {
        self.struct_start(v, StreamType::Array, |nz| StructTy::Array(nz))
    }

    /// Add a new Map into the context
    pub fn map(&mut self, v: ValueStream) -> Result<(), StateError> {
        self.struct_start(v, StreamType::Map(false), |nz| StructTy::Map {
            exp_val: false,
            elements: nz,
        })
    }

    /// Process a CBOR break, which terminate either an indefinite array, map, bytes, text
    pub fn brk(&mut self) -> Result<(), StateError> {
        match self.ctx.pop() {
            Some(StructTy::Stream(_sty)) => (),
            Some(_sty) => {
                return Err(StateError::BreakInNonStreamable);
            }
            None => {
                return Err(StateError::BreakEmptyStack);
            }
        };
        self.advance()?;
        self.check_reduce()?;
        Ok(())
    }

    /// Add a new tag into the context
    pub fn tag(&mut self) -> Result<(), StateError> {
        match self.ctx.last() {
            Some(StructTy::Stream(s)) if s.composite_scalar() => {
                return Err(StateError::TagInChunk);
            }
            _ => (),
        };

        self.ctx.push(StructTy::Tag(false));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_state_ok {
        ($r:expr) => {
            match $r {
                Err(e) => panic!("expecting ok, but got error: {:?}", e),
                Ok(()) => (),
            }
        };
    }

    #[test]
    fn array_empty() {
        let mut state = State::new();
        assert_state_ok!(state.array(Some(Value::U64(0))));
        assert!(state.acceptable());
    }

    #[test]
    fn array1() {
        let mut state = State::new();
        assert_state_ok!(state.array(Some(Value::U64(3))));
        assert_state_ok!(state.simple());
        assert_state_ok!(state.simple());
        assert_state_ok!(state.simple());
        assert!(state.acceptable());
    }

    #[test]
    fn map1() {
        let mut state = State::new();
        assert_state_ok!(state.map(Some(Value::U64(2))));
        assert_state_ok!(state.simple());
        assert_state_ok!(state.simple());
        assert_state_ok!(state.simple());
        assert_state_ok!(state.simple());
        assert!(state.acceptable());
    }

    #[test]
    fn map_rec1() {
        let mut state = State::new();
        assert_state_ok!(state.map(Some(Value::U64(2))));
        assert_state_ok!(state.simple());
        assert_state_ok!(state.simple());
        assert_state_ok!(state.simple());
        assert_state_ok!(state.map(Some(Value::U64(1))));
        assert_state_ok!(state.simple());
        assert_state_ok!(state.simple());
        assert!(state.acceptable());
    }

    #[test]
    fn array_rec1() {
        let mut state = State::new();
        assert_state_ok!(state.array(Some(Value::U64(3))));
        assert_state_ok!(state.simple());
        assert_state_ok!(state.array(None));
        assert_state_ok!(state.brk());
        assert_state_ok!(state.simple());
        assert!(state.acceptable());
    }

    #[test]
    fn array_rec2() {
        let mut state = State::new();
        assert_state_ok!(state.array(Some(Value::U64(3))));
        assert_state_ok!(state.simple());
        assert_state_ok!(state.simple());
        assert_state_ok!(state.array(None));
        assert_state_ok!(state.brk());
        assert!(state.acceptable());
    }

    #[test]
    fn chunk_bytes() {
        let mut state = State::new();
        assert_state_ok!(state.array(Some(Value::U64(1))));
        assert_state_ok!(state.bytes(None));
        assert_state_ok!(state.bytes(Some(Value::U64(1))));
        assert_state_ok!(state.bytes(Some(Value::U64(1))));
        assert_state_ok!(state.brk());
        assert!(state.acceptable());
    }

    #[test]
    fn chunk_text() {
        let mut state = State::new();
        assert_state_ok!(state.array(Some(Value::U64(1))));
        assert_state_ok!(state.text(None));
        assert_state_ok!(state.text(Some(Value::U64(1))));
        assert_state_ok!(state.text(Some(Value::U64(1))));
        assert_state_ok!(state.brk());
        assert!(state.acceptable());
    }

    #[test]
    fn tag() {
        let mut state = State::new();
        assert_state_ok!(state.array(Some(Value::U64(3))));
        assert_state_ok!(state.simple());
        assert_state_ok!(state.tag());
        assert_state_ok!(state.simple());
        assert_state_ok!(state.simple());
        assert!(state.acceptable());
    }
}
