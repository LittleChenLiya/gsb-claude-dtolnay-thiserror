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
fn test_transparent_generic_source_chain() {
    // A transparent wrapper around a generic field must compile with the
    // inferred `E: Error` bound on the generated Error impl, and must keep
    // the inner error's whole source chain.
    #[derive(Error, Debug)]
    #[error(transparent)]
    struct Transparent<E>(E);

    #[derive(Error, Debug)]
    #[error("middle")]
    struct Middle(#[source] io::Error);

    let io = io::Error::other("oh no!");
    let error = Transparent(Middle(io));

    // Transparent: Display delegates to the inner error, and source()
    // delegates to the inner error's own source, so the inner error's
    // source chain is preserved through the generic wrapper.
    assert_eq!("middle", error.to_string());
    let source = error.source().unwrap();
    assert_eq!("oh no!", source.to_string());
    assert!(source.source().is_none());
}

#[test]
fn test_transparent_source_chain_does_not_execute_display() {
    // Diagnosing an error through its source chain must not execute Display
    // of the transparently wrapped error, nor construct new error values.
    use std::sync::atomic::{AtomicUsize, Ordering};

    static DISPLAY_CALLS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Error, Debug)]
    #[error(transparent)]
    struct Transparent<E>(E);

    struct Counted {
        source: io::Error,
    }

    impl std::fmt::Debug for Counted {
        fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.debug_struct("Counted").finish()
        }
    }

    impl std::fmt::Display for Counted {
        fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            DISPLAY_CALLS.fetch_add(1, Ordering::SeqCst);
            formatter.write_str("counted")
        }
    }

    impl std::error::Error for Counted {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.source)
        }
    }

    let error = Transparent(Counted {
        source: io::Error::other("oh no!"),
    });

    // The source chain of the wrapped error is kept: source() of the
    // transparent wrapper is the wrapped error's own source.
    let source = error.source().unwrap();
    assert!(source.downcast_ref::<io::Error>().is_some());
    assert!(source.source().is_none());

    // Walking the source chain never executed Display of the wrapped error.
    assert_eq!(0, DISPLAY_CALLS.load(Ordering::SeqCst));
}
