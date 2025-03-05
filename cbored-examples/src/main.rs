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

fn main() {
    println!("Hello, world!");
}
