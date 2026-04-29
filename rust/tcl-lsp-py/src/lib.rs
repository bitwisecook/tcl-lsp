//! Public `PyO3` bindings exposing the tcl-lsp Rust crates to Python.
//!
//! This crate is the sole place where Python-compatibility concerns live:
//! `#[pyclass]` wrappers, `PyErr` translations, and any back-compat shims
//! that mimic the current Python API surface. The underlying Rust crates
//! (starting with `tcl-lexer`) remain free of `pyo3` and are shaped for
//! idiomatic Rust use.
//!
//! ARCH7 finished the binding-only audit: every binding module in
//! this crate is now `#[pyfunction]` / `#[pyclass]` definitions plus
//! Python-type conversion glue. Algorithm bodies, command tables,
//! and wire-form mappings all live in pure crates (`tcl-compiler`,
//! `tcl-lsp-core`, `tcl-registry`).
//!
//! ARCH9 split this crate out from the transitional `tcl-lsp-rust`
//! cdylib. The same Rust code now backs two Python modules:
//!
//! - `tcl_lsp_py` — the canonical, public API. New Python code
//!   should `import tcl_lsp_py`.
//! - `tcl_lsp_rust` — a one-release alias that re-exports the same
//!   functions and classes via [`register_with`]. Existing Python
//!   code keeps working unchanged.
//!
//! Both wheels share the same Rust binding source; only the
//! `#[pymodule]` entry-point name differs.
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

#![deny(missing_docs)]

use std::borrow::Cow;

use pyo3::prelude::*;

// Modules stay `pub(crate)` because the only external consumer is
// the `tcl-lsp-rust` alias crate, which calls
// [`register_with`] — none of the internal binding types need to
// be reachable cross-crate. This keeps `#![deny(missing_docs)]`
// from forcing per-`#[pyfunction]` doc comments that only the
// PyO3 macro reads.
pub(crate) mod analyser;
pub(crate) mod compilation_unit;
pub(crate) mod compiler_checks;
pub(crate) mod expr_lexer;
pub(crate) mod expr_parser;
pub(crate) mod folding_binding;
pub(crate) mod gvn;
pub(crate) mod interprocedural;
pub(crate) mod lexer;
pub(crate) mod optimiser;
pub(crate) mod registry;
pub(crate) mod signature_scan;
pub(crate) mod tokens;

/// Return the Rust-side greeting used by the smoke test.
///
/// The exact return value is asserted by
/// `tests/test_rust_bindings_smoke.py`; if you change it, update the test
/// in the same commit.
#[pyfunction]
#[must_use]
pub fn hello_rust() -> &'static str {
    "hello from rust"
}

/// Return the version of the underlying `tcl-lexer` crate, so Python-side
/// code can report which native build it is running against.
#[pyfunction]
#[must_use]
pub fn lexer_version() -> &'static str {
    tcl_lexer::VERSION
}

/// Return the version of the underlying `tcl-compiler` crate.
#[pyfunction]
#[must_use]
pub fn compiler_version() -> &'static str {
    tcl_compiler::VERSION
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
#[must_use]
pub fn backslash_subst(text: &str) -> Cow<'_, str> {
    tcl_lexer::backslash_subst(text)
}

/// Register every public binding on the Python module `m`.
///
/// Both `#[pymodule] tcl_lsp_py` (this crate's cdylib) and
/// `#[pymodule] tcl_lsp_rust` (the legacy alias crate) call this
/// function so the two wheels expose identical surfaces.
pub fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(hello_rust, m)?)?;
    m.add_function(wrap_pyfunction!(lexer_version, m)?)?;
    m.add_function(wrap_pyfunction!(compiler_version, m)?)?;
    m.add_function(wrap_pyfunction!(backslash_subst, m)?)?;
    m.add_function(wrap_pyfunction!(lexer::lexer_tokenise, m)?)?;
    m.add_function(wrap_pyfunction!(lexer::lexer_tokenise_with_config, m)?)?;
    tokens::register_with(m)?;
    expr_lexer::register_with(m)?;
    expr_parser::register_with(m)?;
    registry::register_with(m)?;
    optimiser::register_with(m)?;
    compiler_checks::register_with(m)?;
    interprocedural::register_with(m)?;
    gvn::register_with(m)?;
    compilation_unit::register_with(m)?;
    signature_scan::register_with(m)?;
    analyser::register_with(m)?;
    folding_binding::register_with(m)?;
    Ok(())
}

/// The canonical Python-visible module. The name here must match
/// the `module-name` field in `rust/tcl-lsp-py/pyproject.toml` so
/// `maturin` emits a wheel that imports as `tcl_lsp_py`.
#[pymodule]
fn tcl_lsp_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    register_with(m)
}
