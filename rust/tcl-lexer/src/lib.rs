//! Position-aware lexer for Tcl, iRules, and related dialects.
//!
//! This crate is populated chunk-by-chunk as the Python-to-Rust migration
//! progresses. Currently exported:
//!
//! - [`backslash_subst`] — Tcl backslash escape processing (chunk **L1**).
//! - [`Token`], [`TokenType`], [`SourcePosition`] — token data types
//!   (chunk **L2**). `Token` carries only a [`Span`]; text and
//!   positions are resolved through a [`SourceMap`].
//! - [`Span`], [`LineIndex`], [`SourceMap`] — the span-threaded
//!   source-mapping layer. Every positional entity in the Rust
//!   rewrite (tokens now, IR and CFG nodes later) holds a bare
//!   [`Span`] and asks a [`SourceMap`] for text or positions on
//!   demand (chunk **L3**).
//! - [`Lexer`], [`LexerConfig`], [`LexError`] — the L3 lexer
//!   skeleton. Handles EOF, SEP, EOL, COMMENT, and plain ESC tokens;
//!   every other construct is surfaced as a `SyntaxError` (in
//!   strict-quoting mode) or a warning until later chunks implement
//!   it.
//!
//! The crate has no `pyo3` dependency and no Python-compat concerns —
//! those belong in the `tcl-lsp-rust` binding crate. See
//! `docs/rust-rewrite.md` in the main repository for the full
//! migration strategy.

#![deny(missing_docs)]

mod expr_lexer;
mod lexer;
mod line_index;
mod source_map;
mod span;
mod substitution;
mod tokens;

pub use expr_lexer::{
    math_functions as expr_math_functions, tokenise_expr, tokenise_expr_checked, ExprToken,
    ExprTokenType,
};
pub use lexer::{LexError, Lexer, LexerConfig};
pub use line_index::LineIndex;
pub use source_map::SourceMap;
pub use span::Span;
pub use substitution::backslash_subst;
pub use tokens::{SourcePosition, Token, TokenType};

/// Crate version string, useful for migration diagnostics.
///
/// ```
/// assert!(!tcl_lexer::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
