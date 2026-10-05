#![allow(clippy::items_after_statements)]

use anyhow::anyhow;
use std::error::Error as _;
use std::io;
use thiserror::Error;

#[test]
fn test_transparent_struct() {
    #[derive(Error, Debug)]
    #[error(transparent)]
    struct Error(ErrorKind);

    #[derive(Error, Debug)]
    enum ErrorKind {
        #[error("E0")]
        E0,
        #[error("E1")]
        E1(#[from] io::Error),
    }

    let error = Error(ErrorKind::E0);
    assert_eq!("E0", error.to_string());
    assert!(error.source().is_none());

    let io = io::Error::other("oh no!");
    let error = Error(ErrorKind::from(io));
    assert_eq!("E1", error.to_string());
    error.source().unwrap().downcast_ref::<io::Error>().unwrap();
}

#[test]
fn test_transparent_enum() {
    #[derive(Error, Debug)]
    enum Error {
        #[error("this failed")]
        This,
        #[error(transparent)]
        Other(anyhow::Error),
    }

    let error = Error::This;
    assert_eq!("this failed", error.to_string());

    let error = Error::Other(anyhow!("inner").context("outer"));
    assert_eq!("outer", error.to_string());
    assert_eq!("inner", error.source().unwrap().to_string());
}

#[test]
fn test_transparent_enum_with_default_message() {
    #[derive(Error, Debug)]
    #[error("this failed: {0}_{1}")]
    enum Error {
        This(i32, i32),
        #[error(transparent)]
        Other(anyhow::Error),
    }

    let error = Error::This(-1, -1);
    assert_eq!("this failed: -1_-1", error.to_string());

    let error = Error::Other(anyhow!("inner").context("outer"));
    assert_eq!("outer", error.to_string());
    assert_eq!("inner", error.source().unwrap().to_string());
}

#[test]
fn test_transparent_enum_generic() {
    #[derive(Error, Debug)]
    enum Error<E> {
        #[error("this failed")]
        This,
        #[error(transparent)]
        Other(E),
    }

    #[derive(Error, Debug)]
    #[error("inner error")]
    struct Inner;

    let error = Error::<Inner>::This;
    assert_eq!("this failed", error.to_string());

    let error = Error::Other(Inner);
    assert_eq!("inner error", error.to_string());
    assert!(error.source().is_none());

    #[derive(Error, Debug)]
    #[error("wrapped")]
    struct WithSource(#[source] io::Error);

    let io = io::Error::other("oh no!");
    let error = Error::Other(WithSource(io));
    assert_eq!("wrapped", error.to_string());
    assert_eq!("oh no!", error.source().unwrap().to_string());
}

#[test]
fn test_anyhow() {
    #[derive(Error, Debug)]
    #[error(transparent)]
    struct Any(#[from] anyhow::Error);

    let error = Any::from(anyhow!("inner").context("outer"));
    assert_eq!("outer", error.to_string());
    assert_eq!("inner", error.source().unwrap().to_string());
}

#[test]
fn test_non_static() {
    #[derive(Error, Debug)]
    #[error(transparent)]
    struct Error<'a> {
        inner: ErrorKind<'a>,
    }

    #[derive(Error, Debug)]
    enum ErrorKind<'a> {
        #[error("unexpected token: {:?}", token)]
        Unexpected { token: &'a str },
    }

    let error = Error {
        inner: ErrorKind::Unexpected { token: "error" },
    };
    assert_eq!("unexpected token: \"error\"", error.to_string());
    assert!(error.source().is_none());
}

#[test]
fn test_transparent_generic_struct_source_chain() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Diagnosing an error through its source chain must neither execute
    // Display nor construct new errors.
    static DISPLAY_CALLS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug)]
    struct Inner {
        source: io::Error,
    }

    impl std::fmt::Display for Inner {
        fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            DISPLAY_CALLS.fetch_add(1, Ordering::SeqCst);
            formatter.write_str("inner")
        }
    }

    impl std::error::Error for Inner {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.source)
        }
    }

    // A transparent generic wrapper compiles with the inferred `T: Error`
    // bound on the generated Error impl and keeps the source chain of the
    // wrapped error.
    #[derive(Error, Debug)]
    #[error(transparent)]
    struct Error<T>(T);

    let error = Error(Inner {
        source: io::Error::other("oh no!"),
    });

    let mut chain = 0;
    let mut current = error.source();
    while let Some(source) = current {
        chain += 1;
        current = source.source();
    }
    assert_eq!(chain, 1);
    error.source().unwrap().downcast_ref::<io::Error>().unwrap();
    assert_eq!(DISPLAY_CALLS.load(Ordering::SeqCst), 0);

    // Display is still forwarded to the inner error when actually requested.
    assert_eq!(error.to_string(), "inner");
    assert_eq!(DISPLAY_CALLS.load(Ordering::SeqCst), 1);
}
