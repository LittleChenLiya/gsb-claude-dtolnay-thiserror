use thiserror::Error;

#[derive(Error, Debug)]
pub enum ErrorEnum {
    #[error("first variant")]
    First(#[source] std::io::Error),
    #[error("second variant")]
    Second {
        #[source]
        a: std::io::Error,
        #[source]
        b: anyhow::Error,
    },
}

fn main() {}
