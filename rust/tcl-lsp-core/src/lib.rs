//! Pure Rust LSP feature providers for tcl-lsp.
//!
//! This crate owns the algorithmic LSP feature surface — folding,
//! document symbols, hover, diagnostic projection, and (future)
//! completion, references, rename, and semantic tokens. It
//! contains no `pyo3` dependency and no Python-compat shims; the
//! binding crate wraps these providers for Python callers, and
//! the `tcl-lsp-server` binary links against this crate
//! directly over the LSP protocol.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod completion;
pub mod definition;
pub mod document_symbols;
pub mod folding;
pub mod formatting;
pub mod hover;
pub mod references;
pub mod rename;
pub mod selection_range;
pub mod signature_help;

/// Crate version string.
///
/// ```
/// assert!(!tcl_lsp_core::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
