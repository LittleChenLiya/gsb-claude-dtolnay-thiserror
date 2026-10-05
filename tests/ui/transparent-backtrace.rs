use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct StructError(#[backtrace] anyhow::Error);

#[derive(Error, Debug)]
pub enum EnumError {
    #[error(transparent)]
    Other(#[backtrace] anyhow::Error),
}

fn main() {}
