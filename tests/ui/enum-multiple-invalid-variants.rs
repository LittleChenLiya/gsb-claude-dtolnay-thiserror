use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    First(std::io::Error, std::io::Error),

    #[error("second")]
    Second {
        #[from]
        a: std::io::Error,
        #[from]
        b: std::io::Error,
    },
}

fn main() {}
