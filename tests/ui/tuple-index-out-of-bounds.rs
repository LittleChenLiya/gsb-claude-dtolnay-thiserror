use thiserror::Error;

#[derive(Error, Debug)]
#[error("out of bounds: {1}")]
pub struct TupleStruct(u32);

#[derive(Error, Debug)]
pub enum EnumError {
    #[error("out of bounds: {2}")]
    TupleVariant(u32, u32),
}

fn main() {}
