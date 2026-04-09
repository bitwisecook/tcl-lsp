//! Position-aware lexer for Tcl, iRules, and related dialects.
//!
//! This crate is intentionally empty in the initial workspace bootstrap
//! (chunk **L0** of the Python-to-Rust migration). Subsequent chunks will
//! populate it with `Token`, `TokenType`, `SourcePosition`, a `LexerConfig`,
//! and a streaming `Lexer` iterator.
//!
//! The crate has no `pyo3` dependency and no Python-compat concerns — those
//! belong in the `tcl-lsp-rust` binding crate. See
//! `docs/kcs/kcs-rust-migration.md` in the main repository for the full
//! migration strategy.

#![deny(missing_docs)]

/// Crate version string, useful for migration diagnostics.
///
/// ```
/// assert!(!tcl_lexer::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
