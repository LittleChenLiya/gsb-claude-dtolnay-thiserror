use std::backtrace::Backtrace;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Other(#[backtrace] Backtrace),
}

fn main() {}
