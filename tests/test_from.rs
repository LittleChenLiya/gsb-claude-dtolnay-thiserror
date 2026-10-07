#![allow(clippy::extra_unused_type_parameters)]

use std::error::Error as StdError;
use std::fmt;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("...")]
pub struct ErrorStruct {
    #[from]
    source: io::Error,
}

#[derive(Error, Debug)]
#[error("...")]
pub struct ErrorStructOptional {
    #[from]
    source: Option<io::Error>,
}

#[derive(Error, Debug)]
#[error("...")]
pub struct ErrorTuple(#[from] io::Error);

#[derive(Error, Debug)]
#[error("...")]
pub struct ErrorTupleOptional(#[from] Option<io::Error>);

#[derive(Error, Debug)]
#[error("...")]
pub enum ErrorEnum {
    Test {
        #[from]
        source: io::Error,
    },
}

#[derive(Error, Debug)]
#[error("...")]
pub enum ErrorEnumOptional {
    Test {
        #[from]
        source: Option<io::Error>,
    },
}

#[derive(Error, Debug)]
#[error("...")]
pub enum Many {
    Any(#[from] anyhow::Error),
    Io(#[from] io::Error),
}

#[derive(Error, Debug)]
pub enum DistinctWithOptional {
    #[error("io error")]
    Io(#[from] io::Error),
    #[error("optional fmt error")]
    Fmt(#[from] Option<fmt::Error>),
}

#[derive(Debug)]
pub struct Alpha;

impl fmt::Display for Alpha {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "alpha")
    }
}

impl StdError for Alpha {}

#[derive(Debug)]
pub struct Beta;

impl fmt::Display for Beta {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "beta")
    }
}

impl StdError for Beta {}

#[derive(Error, Debug)]
pub enum NamedDistinct {
    #[error("alpha")]
    Alpha {
        #[from]
        source: Alpha,
    },
    #[error("beta")]
    Beta {
        #[from]
        source: Beta,
    },
}

fn assert_impl<T: From<io::Error>>() {}

#[test]
fn test_from() {
    assert_impl::<ErrorStruct>();
    assert_impl::<ErrorStructOptional>();
    assert_impl::<ErrorTuple>();
    assert_impl::<ErrorTupleOptional>();
    assert_impl::<ErrorEnum>();
    assert_impl::<ErrorEnumOptional>();
    assert_impl::<Many>();
}

#[test]
fn test_enum_distinct_from_types() {
    // Distinct #[from] source types in two variants must both generate From.
    let alpha = NamedDistinct::from(Alpha);
    assert!(alpha.source().unwrap().is::<Alpha>());
    let beta = NamedDistinct::from(Beta);
    assert!(beta.source().unwrap().is::<Beta>());

    // A plain T and an Option<U> of a distinct U are not a conflict, and the
    // optional variant still converts from U with an Option-wrapped source.
    let io_err = DistinctWithOptional::from(io::Error::other("boom"));
    assert!(io_err.source().unwrap().is::<io::Error>());
    let fmt_err = DistinctWithOptional::from(fmt::Error);
    assert!(fmt_err.source().unwrap().is::<fmt::Error>());

    // The value is held inside Option in the optional From variant.
    match &fmt_err {
        DistinctWithOptional::Fmt(source) => assert!(source.is_some()),
        _ => panic!("wrong variant"),
    }
}
