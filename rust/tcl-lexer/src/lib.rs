//! Position-aware lexer for Tcl, iRules, and related dialects.
//!
//! This crate is populated chunk-by-chunk as the Python-to-Rust migration
//! progresses. Currently exported:
//!
//! - [`backslash_subst`] — Tcl backslash escape processing (chunk **L1**).
//!
//! Upcoming chunks will add `Token`, `TokenType`, `SourcePosition`, a
//! `LexerConfig`, and a streaming `Lexer` iterator.
//!
//! The crate has no `pyo3` dependency and no Python-compat concerns — those
//! belong in the `tcl-lsp-rust` binding crate. See `docs/rust-rewrite.md`
//! in the main repository for the full migration strategy.

#![deny(missing_docs)]

mod substitution;

pub use substitution::backslash_subst;

/// Crate version string, useful for migration diagnostics.
///
/// ```
/// assert!(!tcl_lexer::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
