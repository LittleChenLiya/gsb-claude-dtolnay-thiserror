#![allow(clippy::uninlined_format_args)]

use core::sync::atomic::{AtomicUsize, Ordering};
use thiserror::Error;

// Side-effecting expression argument. The expression is parsed by the derive
// macro but not evaluated at compile time; it must run only when Display
// actually runs, and only once per Display call.
static EXPR_EVALS: AtomicUsize = AtomicUsize::new(0);

fn counted_expr() -> &'static str {
    EXPR_EVALS.fetch_add(1, Ordering::SeqCst);
    "expr"
}

// Three variant kinds, each with its own format-argument domain: named
// fields, tuple indices, and expression arguments. Escaped braces are
// literal text, not argument references.
#[derive(Error, Debug)]
pub enum Error {
    #[error(
        "struct variant: {name} code {code} (again {name}) {{literal}} {}",
        counted_expr()
    )]
    Struct { name: &'static str, code: u16 },
    #[error("tuple variant: {0} {1} {0}")]
    Tuple(u8, u8),
    #[error("unit variant")]
    Unit,
}

#[test]
fn test_three_variants() {
    EXPR_EVALS.store(0, Ordering::SeqCst);

    let err = Error::Struct {
        name: "thing",
        code: 7,
    };
    // Merely constructing the error must not evaluate the expression.
    assert_eq!(0, EXPR_EVALS.load(Ordering::SeqCst));
    assert_eq!(
        "struct variant: thing code 7 (again thing) {literal} expr",
        err.to_string()
    );

    assert_eq!("tuple variant: 1 2 1", Error::Tuple(1, 2).to_string());
    assert_eq!("unit variant", Error::Unit.to_string());
}

#[test]
fn test_expr_evaluated_once_per_display() {
    EXPR_EVALS.store(0, Ordering::SeqCst);

    let err = Error::Struct {
        name: "thing",
        code: 7,
    };
    let _ = err.to_string();
    assert_eq!(1, EXPR_EVALS.load(Ordering::SeqCst));
    let _ = err.to_string();
    assert_eq!(2, EXPR_EVALS.load(Ordering::SeqCst));
}

#[test]
fn test_escaped_braces_are_not_arguments() {
    // If `{{literal}}` were parsed as an argument reference, `literal` would
    // be a nonexistent field and the derive would fail to compile.
    #[derive(Error, Debug)]
    #[error("100% {{guaranteed}}")]
    struct Braces;

    assert_eq!("100% {guaranteed}", Braces.to_string());
}

#[test]
fn test_named_and_positional_expr_args() {
    // Explicit named arguments and expression arguments bind per variant,
    // independently of the variant's fields.
    #[derive(Error, Debug)]
    pub enum Expr {
        #[error("{what} failed after {elapsed}ms", what = "download", elapsed = 2 * 21)]
        Named,
        #[error("{} and {}", 1 + 1, "two".to_uppercase())]
        Positional,
    }

    assert_eq!("download failed after 42ms", Expr::Named.to_string());
    assert_eq!("2 and TWO", Expr::Positional.to_string());
}
