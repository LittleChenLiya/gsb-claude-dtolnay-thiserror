use thiserror::Error;

#[derive(Error, Debug)]
#[error("missing field: {nope}")]
pub struct StructError {
    id: u32,
}

#[derive(Error, Debug)]
pub enum EnumError {
    // Each variant has its own parameter domain: `code` is a field of the
    // other variant, not of this one.
    #[error("named variant: {code}")]
    Named { id: u32 },
}

#[derive(Error, Debug)]
pub enum TupleVariantError {
    #[error("tuple variant: {nope}")]
    Tuple(u32),
}

fn main() {}
