use cbored_derive::CborRepr;

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
pub struct Point2 {
    x: u32,
    y: Option<u32>,
}

#[derive(CborRepr)]
#[cborrepr(enumtype = "tagvariant", skipkey = 1)]
// serialized as
// * One : ARRAY(2) [ UINT(0), UINT ]
// * Two : ARRAY(3) [ UINT(2), UINT, TEXT ]
// * Three : ARRAY(2) [ UINT(3), ARRAY(2) [ UINT, UINT ] ]
// * Four : ARRAY(1) [ UINT(4) ]
pub enum Variant {
    One(u32),
    Two(u64, String),
    Three(Point),
    Four,
}

fn main() {
    println!("Hello, world!");
}
