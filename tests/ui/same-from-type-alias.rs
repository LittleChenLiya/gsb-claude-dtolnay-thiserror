use thiserror::Error;

pub type Io = std::io::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to open")]
    OpenFile(#[from] std::io::Error),
    #[error("failed to close")]
    CloseFile(#[from] Io),
}

fn main() {}
