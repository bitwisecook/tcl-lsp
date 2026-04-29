//! LSP feature providers.
//!
//! Houses the per-feature ports of `lsp/features/*.py` together with
//! their `PyO3` bindings.  Each feature lives in its own submodule:
//! the pure provider (`<feature>.rs`) is `pyo3`-free so it can later
//! lift cleanly into `tcl-lsp-core` per `ARCH0` in
//! [`docs/rust-rewrite.md`], while the binding (`<feature>_binding.rs`)
//! holds the Python-facing wrapper.
//!
//! `register_with` aggregates the per-feature `register_with` entry
//! points so `lib.rs` only knows about `features::register_with`.

use pyo3::prelude::*;

pub mod document_symbols;
mod document_symbols_binding;
pub mod folding;
mod folding_binding;

/// Register every feature provider's Python-facing functions on the
/// extension module.
pub fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    folding_binding::register_with(m)?;
    document_symbols_binding::register_with(m)?;
    Ok(())
}
