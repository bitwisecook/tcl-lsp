//! Error hierarchy for the query DSL.
//!
//! Faithful port of `dialects/f5/query/errors.py`. Every error message is
//! shaped so the CLI verb can prefix it with `error:` and present it
//! directly to the user. The optional *offset* on lex / parse errors lets
//! the CLI underline the offending span in the source.
//!
//! Only the front-end variants (`Lex` / `Parse`) are modelled so far; the
//! `Eval` / `Edit` / `Builtin` / `Renderer` arms from the Python hierarchy
//! land alongside the evaluator and builtin library.

use std::fmt;

/// Base error type for everything raised by the query DSL.
///
/// Mirrors the Python `QueryError` subclass hierarchy. Each `Display`
/// rendering reproduces the corresponding Python `__str__`, so the CLI's
/// `error: {exc}` formatting stays byte-for-byte identical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    /// The lexer hit a character it does not understand.
    Lex { message: String, offset: usize },
    /// The parser saw a token it did not expect.
    Parse { message: String, offset: usize },
}

impl QueryError {
    /// The bare message, without the trailing `at offset N` suffix.
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            QueryError::Lex { message, .. } | QueryError::Parse { message, .. } => message,
        }
    }

    /// The byte offset (Python: code-point offset) the error points at.
    #[must_use]
    pub fn offset(&self) -> usize {
        match self {
            QueryError::Lex { offset, .. } | QueryError::Parse { offset, .. } => *offset,
        }
    }
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Both `LexError` and `ParseError` render as
        // ``f"{message} at offset {offset}"`` in Python.
        match self {
            QueryError::Lex { message, offset } | QueryError::Parse { message, offset } => {
                write!(f, "{message} at offset {offset}")
            }
        }
    }
}

impl std::error::Error for QueryError {}
