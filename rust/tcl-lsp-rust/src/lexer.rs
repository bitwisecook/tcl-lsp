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

use tcl_lexer::{LexError, Lexer, LexerConfig, SourceMap, Token};

use crate::tokens::{PySourcePosition, PyToken, PyTokenType};

/// Tokenise `source` via the Rust lexer and return the result as a
/// list of `tcl_lsp_rust.Token` instances.
///
/// Raises `ValueError` if the Rust lexer trips
/// `LexError::UnsupportedCharacter` — this is the signal the
/// differential harness uses to filter inputs the L3 skeleton does
/// not yet understand. The message includes the offending character
/// and its position so the harness can log skipped cases.
/// Tokenise `source` via the Rust lexer with default config.
#[pyfunction]
#[pyo3(text_signature = "(source, /)")]
pub fn lexer_tokenise(source: &str) -> PyResult<Vec<PyToken>> {
    lexer_tokenise_with_config(source, true, false, 0, 0, 0)
}

/// Tokenise `source` via the Rust lexer with explicit config.
///
/// This is the full-config entry point used by the Python shim
/// when dialect flags or sub-lexing offsets are in play.
#[pyfunction]
#[pyo3(signature = (source, expand_syntax=true, irules_brace_separator=false, base_offset=0, base_line=0, base_col=0))]
pub fn lexer_tokenise_with_config(
    source: &str,
    expand_syntax: bool,
    irules_brace_separator: bool,
    base_offset: u32,
    base_line: u32,
    base_col: u32,
) -> PyResult<Vec<PyToken>> {
    let config = LexerConfig {
        expand_syntax,
        irules_brace_separator,
        strict_quoting: false,
        base_offset,
        base_line,
        base_col,
    };
    let source_map = SourceMap::new(source).with_base(base_offset, base_line, base_col);
    let lexer = Lexer::with_source_map(source_map, config);
    let sm = lexer.source_map().clone();
    let tokens: Vec<Token> = lexer.tokenise_all().map_err(|err| to_py_err(&err))?;
    Ok(tokens.into_iter().map(|tok| lift(tok, &sm)).collect())
}

/// Lift a pure-Rust `Token` into a Python-visible `PyToken`,
/// resolving its span against `source_map` to produce the
/// `text` / `start` / `end` fields the Python API exposes.
///
/// Note that the Rust `Token.span` covers the full extent of the
/// token including any leading `$` / `${` wrappers, so that
/// `range_positions(span)` produces the same start/end the Python
/// lexer does. The text field, however, uses `source_map.token_text`
/// which strips those wrappers — matching Python's quirky-but-
/// long-standing convention that `tok.text` is the "human-readable"
/// content (e.g. `"foo"` for `$foo`, `"name"` for `${name}`).
fn lift(tok: Token, source_map: &SourceMap<'_>) -> PyToken {
    let (start_pos, end_pos) = source_map.range_positions(tok.span);
    PyToken::new_from_core(
        PyTokenType::from(tok.kind),
        source_map.token_text(tok).to_owned(),
        PySourcePosition::from_core(start_pos),
        PySourcePosition::from_core(end_pos),
        tok.in_quote,
    )
}

fn to_py_err(err: &LexError) -> PyErr {
    PyValueError::new_err(err.to_string())
}
