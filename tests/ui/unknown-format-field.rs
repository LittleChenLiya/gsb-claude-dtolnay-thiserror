use thiserror::Error;

#[derive(Error, Debug)]
#[error("missing field: {misspelled}")]
pub struct StructError {
    pub field: u32,
}

#[derive(Error, Debug)]
#[error("tuple index out of range: {1}")]
pub struct TupleError(u32);

#[derive(Error, Debug)]
pub enum EnumError {
    #[error("missing field: {misspelled}")]
    StructVariant { field: u32 },
    #[error("tuple index out of range: {2}")]
    TupleVariant(u32, u32),
    #[error("missing field: {misspelled}")]
    UnitVariant,
}

fn main() {}
