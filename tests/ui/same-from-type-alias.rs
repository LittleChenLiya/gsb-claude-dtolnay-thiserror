// An alias whose written type differs from the original path must not be caught
// by the derive's pre-check (equality is not guessed from path text); the
// compiler's coherence check (E0119) is the authority for real type equality.
use thiserror::Error;

type IoError = std::io::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to open")]
    OpenFile(#[from] std::io::Error),
    #[error("failed to close")]
    CloseFile(#[from] IoError),
}

fn main() {}
