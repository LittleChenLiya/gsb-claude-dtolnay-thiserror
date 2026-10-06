#![allow(clippy::needless_raw_string_hashes)]

use std::cell::Cell;
use thiserror::Error;

// Three-variant example: each variant has its own format parameter domain.
// Named fields, tuple indices, and expression arguments are all parsed
// according to the #[error(...)] attribute syntax.
#[derive(Error, Debug)]
enum DataError {
    #[error("named fields: {lo}..={hi}")]
    Range { lo: u8, hi: u8 },
    #[error("tuple indices: {1} then {0} then {1}")]
    Pair(u8, u8),
    #[error("expression arg: {} bytes", .0.len())]
    Payload(String),
}

#[test]
fn test_three_variant_domains() {
    assert_eq!(
        "named fields: 1..=9",
        DataError::Range { lo: 1, hi: 9 }.to_string()
    );
    // Repeated use of the same tuple index reuses a single binding; the
    // generated Display does not perform additional field accesses.
    assert_eq!(
        "tuple indices: 4 then 3 then 4",
        DataError::Pair(3, 4).to_string()
    );
    assert_eq!(
        "expression arg: 5 bytes",
        DataError::Payload("hello".to_owned()).to_string()
    );
}

#[test]
fn test_escaped_braces_are_not_parameters() {
    // `{{` and `}}` are literal braces, not parameter references, so the
    // apparent "field" inside them must not be diagnosed as missing.
    #[derive(Error, Debug)]
    #[error("literal {{missing}} braces around {field}")]
    struct Error {
        field: u32,
    }

    assert_eq!(
        "literal {missing} braces around 7",
        Error { field: 7 }.to_string()
    );
}

#[test]
fn test_escaped_braces_tuple() {
    #[derive(Error, Debug)]
    #[error("{{{0}}}")]
    struct Error(u32);

    assert_eq!("{5}", Error(5).to_string());
}

#[test]
fn test_expression_evaluated_once_per_display() {
    // Expression arguments are parsed, not evaluated, by the macro. At
    // runtime each actual Display call evaluates the expression once.
    #[derive(Error, Debug)]
    #[error("side effect: {}", .0.replace(.0.get() + 1))]
    struct Error(Cell<u32>);

    let error = Error(Cell::new(0));
    assert_eq!("side effect: 0", error.to_string());
    assert_eq!(
        1,
        error.0.get(),
        "expression must be evaluated exactly once"
    );
    assert_eq!("side effect: 1", error.to_string());
    assert_eq!(2, error.0.get(), "one evaluation per actual Display call");
}

#[test]
fn test_named_arg_evaluated_once() {
    #[derive(Error, Debug)]
    #[error("{seen} {seen}", seen = .0.replace(.0.get() + 1))]
    struct Error(Cell<u32>);

    let error = Error(Cell::new(10));
    assert_eq!("10 10", error.to_string());
    assert_eq!(11, error.0.get());
}

#[test]
fn test_named_struct_positional_args_still_work() {
    // In containers with named fields, `{0}` refers to the additional
    // unnamed format arguments, not to a field.
    #[derive(Error, Debug)]
    #[error("{0} + {1} = {sum}", 1, 2, sum = 3)]
    struct Error {
        sum: u32,
    }

    assert_eq!("1 + 2 = 3", Error { sum: 0 }.to_string());
}
