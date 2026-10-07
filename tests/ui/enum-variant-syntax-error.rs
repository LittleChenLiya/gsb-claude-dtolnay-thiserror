use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(=)]
    First,

    #[error(transparent)]
    Second(std::io::Error, std::io::Error),
}

fn main() {}
