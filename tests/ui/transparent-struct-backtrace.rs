use std::backtrace::Backtrace;
use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct Error(#[backtrace] Backtrace);

fn main() {}
