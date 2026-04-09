//! `PyO3` bindings exposing the tcl-lsp Rust crates to Python.
//!
//! This crate is the sole place where Python-compatibility concerns live:
//! `#[pyclass]` wrappers, `PyErr` translations, and any back-compat shims
//! that mimic the current Python API surface. The underlying Rust crates
//! (starting with `tcl-lexer`) remain free of `pyo3` and are shaped for
//! idiomatic Rust use.
//!
//! Exposed so far:
//!
//! - `hello_rust()` / `lexer_version()` — L0 smoke-test bridge.
//! - `backslash_subst(text)` — L1 port of
//!   `core/parsing/substitution.py::backslash_subst`.
//! - `TokenType`, `SourcePosition`, `Token` — L2 port of the
//!   `core/parsing/tokens.py` data types.
//! - `lexer_tokenise(source)` — L3 port of the Tcl lexer skeleton
//!   (EOF / SEP / EOL / COMMENT / plain ESC). Inputs containing
//!   deferred constructs (`$ [ ] {} " \`) raise `ValueError` so the
//!   differential harness can filter them.

use std::borrow::Cow;

use pyo3::prelude::*;

mod lexer;
mod tokens;

/// Return the Rust-side greeting used by the smoke test.
///
/// The exact return value is asserted by
/// `tests/test_rust_bindings_smoke.py`; if you change it, update the test
/// in the same commit.
#[pyfunction]
fn hello_rust() -> &'static str {
    "hello from rust"
}

/// Return the version of the underlying `tcl-lexer` crate, so Python-side
/// code can report which native build it is running against.
#[pyfunction]
fn lexer_version() -> &'static str {
    tcl_lexer::VERSION
}

/// Process Tcl backslash escapes in `text`.
///
/// Thin wrapper around [`tcl_lexer::backslash_subst`]. The underlying Rust
/// function returns `Cow<'_, str>` so backslash-free inputs cost zero
/// allocations on the Rust side; `PyO3` still materialises exactly one
/// Python `str` on the way back either way. See the core crate docs for
/// the full list of supported escapes.
#[pyfunction]
#[pyo3(text_signature = "(text, /)")]
fn backslash_subst(text: &str) -> Cow<'_, str> {
    tcl_lexer::backslash_subst(text)
}

/// The Python-visible module. The name here must match the `name` field in
/// `rust/tcl-lsp-rust/pyproject.toml` so `maturin` emits a wheel that
/// imports as `tcl_lsp_rust`.
#[pymodule]
fn tcl_lsp_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(hello_rust, m)?)?;
    m.add_function(wrap_pyfunction!(lexer_version, m)?)?;
    m.add_function(wrap_pyfunction!(backslash_subst, m)?)?;
    m.add_function(wrap_pyfunction!(lexer::lexer_tokenise, m)?)?;
    tokens::register_with(m)?;
    Ok(())
}
