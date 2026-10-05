use thiserror::Error;

#[derive(Error, Debug)]
pub enum ErrorEnum {
    Fine {
        #[source]
        source: std::io::Error,
    },
    Confusing {
        #[source]
        a: std::io::Error,
        #[source]
        b: anyhow::Error,
    },
}

fn main() {}
