//! `PyO3` bindings exposing the tcl-lsp Rust crates to Python.
//!
//! This crate is the sole place where Python-compatibility concerns live:
//! `#[pyclass]` wrappers, `PyErr` translations, and any back-compat shims
//! that mimic the current Python API surface. The underlying Rust crates
//! (starting with `tcl-lexer`) remain free of `pyo3` and are shaped for
//! idiomatic Rust use.
//!
//! In the initial workspace bootstrap (chunk **L0**) this module exposes a
//! single `hello_rust()` function that is exercised by
//! `tests/test_rust_bindings_smoke.py` to verify that the whole build,
//! install, and import pipeline is wired up correctly end-to-end. Real
//! lexer bindings arrive in later chunks (L1 onward).

use pyo3::prelude::*;

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

/// The Python-visible module. The name here must match the `name` field in
/// `rust/tcl-lsp-rust/pyproject.toml` so `maturin` emits a wheel that
/// imports as `tcl_lsp_rust`.
#[pymodule]
fn tcl_lsp_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(hello_rust, m)?)?;
    m.add_function(wrap_pyfunction!(lexer_version, m)?)?;
    Ok(())
}
