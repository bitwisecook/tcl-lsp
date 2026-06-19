//! LSP feature providers — Python bindings.
//!
//! Houses the per-feature `PyO3` wrappers for the pure providers
//! that live in [`tcl_lsp_core`]. Each binding (`<feature>_binding.rs`)
//! is the only Python-facing surface for its feature; the algorithm
//! itself stays in `tcl-lsp-core` so the eventual native LSP server
//! and the Python LSP can share one canonical implementation.
//!
//! `register_with` aggregates the per-feature `register_with` entry
//! points so [`crate::register_with`] only has to call one function
//! to wire the whole feature surface.

use pyo3::prelude::*;

pub(crate) mod document_symbols_binding;
pub(crate) mod folding_binding;

/// Register every feature provider's Python-facing functions on the
/// extension module.
pub fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    folding_binding::register_with(m)?;
    document_symbols_binding::register_with(m)?;
    Ok(())
}
