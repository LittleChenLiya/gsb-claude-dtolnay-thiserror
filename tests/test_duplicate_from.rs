// Local tests for the duplicate-source-type handling of #[from]:
// variants converting from genuinely different source types must keep
// generating working From impls, and Option<T> sources stay compatible with
// other distinct From types in the same enum.
use std::error::Error as _;
use std::fmt;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error")]
    Io(#[from] io::Error),
    #[error("fmt error")]
    Fmt(#[from] fmt::Error),
}

#[derive(Error, Debug)]
pub enum OptionalAndPlain {
    #[error("optional io error")]
    OptIo(#[from] Option<io::Error>),
    #[error("fmt error")]
    Fmt(#[from] fmt::Error),
}

fn assert_from<F, T: From<F>>() {}

#[test]
fn test_distinct_from_types() {
    assert_from::<io::Error, Error>();
    assert_from::<fmt::Error, Error>();

    let error: Error = io::Error::other("boom").into();
    assert!(matches!(error, Error::Io(_)));
    assert!(error.source().is_some());

    let error: Error = fmt::Error.into();
    assert!(matches!(error, Error::Fmt(_)));
    // The #[from] field is itself reported as the source, even though its own
    // Error::source() returns None.
    assert!(error.source().is_some());
    assert_eq!(
        error.source().and_then(|s| s.source()).map(|s| s.to_string()),
        None,
    );
}

#[test]
fn test_optional_from_is_compatible_with_other_types() {
    // #[from] Option<io::Error> generates From<io::Error>, which is distinct
    // from From<fmt::Error>, so both must coexist.
    assert_from::<io::Error, OptionalAndPlain>();
    assert_from::<fmt::Error, OptionalAndPlain>();

    let error: OptionalAndPlain = io::Error::other("boom").into();
    match &error {
        OptionalAndPlain::OptIo(Some(io_error)) => {
            assert_eq!(io_error.to_string(), "boom");
        }
        _ => panic!("unexpected variant: {error:?}"),
    }
    assert!(error.source().is_some());

    let error: OptionalAndPlain = fmt::Error.into();
    assert!(matches!(error, OptionalAndPlain::Fmt(_)));
}
