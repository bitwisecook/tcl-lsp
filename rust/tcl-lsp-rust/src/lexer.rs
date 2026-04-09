// The `?` operator in a `#[pyfunction]` that returns `PyResult<_>`
// trips `clippy::useless_conversion` inside the macro expansion
// because `PyErr: From<PyErr>` is identity. This is unavoidable with
// PyO3's standard error-propagation pattern; silence the lint for
// the whole module.
#![allow(clippy::useless_conversion)]

//! `PyO3` wrapper exposing `tcl_lexer::Lexer` to Python.
//!
//! The pure-Rust lexer produces `Token` values carrying only a
//! [`Span`]; the binding crate is responsible for resolving each
//! span to owned `text` / `start` / `end` fields so Python callers
//! see the same dataclass shape they always have.
//!
//! L3 exposes a single function, `lexer_tokenise(source)`, used by
//! the differential test harness in
//! `tests/test_rust_lexer_differential.py` to compare Rust and
//! Python token streams on known-simple inputs. A richer `PyO3`
//! interface (iterator object, sub-lexing, borrowing the same
//! [`SourceMap`] across multiple lex invocations) arrives when the
//! first real consumer shows up.
//!
//! [`Span`]: tcl_lexer::Span
//! [`SourceMap`]: tcl_lexer::SourceMap

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use tcl_lexer::{LexError, Lexer, SourceMap, Token};

use crate::tokens::{PySourcePosition, PyToken, PyTokenType};

/// Tokenise `source` via the Rust lexer and return the result as a
/// list of `tcl_lsp_rust.Token` instances.
///
/// Raises `ValueError` if the Rust lexer trips
/// `LexError::UnsupportedCharacter` — this is the signal the
/// differential harness uses to filter inputs the L3 skeleton does
/// not yet understand. The message includes the offending character
/// and its position so the harness can log skipped cases.
#[pyfunction]
#[pyo3(text_signature = "(source, /)")]
pub fn lexer_tokenise(source: &str) -> PyResult<Vec<PyToken>> {
    let lexer = Lexer::new(source);
    let source_map = lexer.source_map().clone();
    let tokens: Vec<Token> = lexer.tokenise_all().map_err(|err| to_py_err(&err))?;
    Ok(tokens
        .into_iter()
        .map(|tok| lift(tok, &source_map))
        .collect())
}

/// Lift a pure-Rust `Token` into a Python-visible `PyToken`,
/// resolving its span against `source_map` to produce the
/// `text` / `start` / `end` fields the Python API exposes.
fn lift(tok: Token, source_map: &SourceMap<'_>) -> PyToken {
    let (start_pos, end_pos) = source_map.range_positions(tok.span);
    PyToken::new_from_core(
        PyTokenType::from(tok.kind),
        source_map.text(tok.span).to_owned(),
        PySourcePosition::from_core(start_pos),
        PySourcePosition::from_core(end_pos),
        tok.in_quote,
    )
}

fn to_py_err(err: &LexError) -> PyErr {
    PyValueError::new_err(err.to_string())
}
