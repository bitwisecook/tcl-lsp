//! Pure Rust LSP feature providers for tcl-lsp.
//!
//! This crate owns the algorithmic LSP feature surface — folding,
//! document symbols, diagnostic projection, and (future) hover,
//! completion, references, rename, and semantic tokens. It contains
//! no `pyo3` dependency and no Python-compat shims; the binding
//! crate wraps these providers for Python callers, and the
//! eventual `tcl-lsp-server` binary links against this crate
//! directly over the LSP protocol.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod folding;

/// Crate version string.
///
/// ```
/// assert!(!tcl_lsp_core::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
