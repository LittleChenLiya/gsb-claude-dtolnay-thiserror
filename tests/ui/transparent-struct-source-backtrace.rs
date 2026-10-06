use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct Error(#[source] #[backtrace] anyhow::Error);

fn main() {}
