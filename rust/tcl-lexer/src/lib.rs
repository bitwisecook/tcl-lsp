//! Position-aware lexer for Tcl, iRules, and related dialects.
//!
//! Currently exported:
//!
//! - [`backslash_subst`] — Tcl backslash escape processing.
//! - [`Token`], [`TokenType`], [`SourcePosition`] — token data types.
//!   `Token` carries only a [`Span`]; text and positions are resolved
//!   through a [`SourceMap`].
//! - [`Span`], [`LineIndex`], [`SourceMap`] — the span-threaded
//!   source-mapping layer. Every positional entity holds a bare
//!   [`Span`] and asks a [`SourceMap`] for text or positions on
//!   demand.
//! - [`word_closer_offset`], [`word_end_position`] — source-aware
//!   authoritative closer accessors for delimited word tokens, derived
//!   from the lexer's content geometry (correct for empty `{}` / `[]` /
//!   `""` and backslash-bearing quoted words).
//! - [`Lexer`], [`LexerConfig`], [`LexError`] — the lexer itself.
//!   Handles EOF, SEP, EOL, COMMENT, and plain ESC tokens; every other
//!   construct is surfaced as a `SyntaxError` (in strict-quoting mode)
//!   or a warning.

#![deny(missing_docs)]

mod expr_lexer;
#[cfg(feature = "html")]
mod highlight;
mod lexer;
mod line_index;
mod ranges;
mod source_map;
mod span;
mod structural_index;
mod substitution;
mod tokens;

pub use expr_lexer::{
    ExprToken, ExprTokenType, math_functions as expr_math_functions, tokenise_expr,
    tokenise_expr_checked,
};
#[cfg(feature = "html")]
pub use highlight::highlight_tcl;
pub use lexer::{LexError, LexWarning, Lexer, LexerConfig};
pub use line_index::LineIndex;
pub use ranges::{word_closer_offset, word_end_position};
pub use source_map::SourceMap;
pub use span::Span;
pub use structural_index::{
    BraceIndex, BracketIndex, ExprParenIndex, ParenBalance, command_boundaries, reparse_window,
    script_is_complete,
};
pub use substitution::backslash_subst;
pub use tokens::{ByteCol, SourcePosition, Token, TokenType, Utf16Col, Utf16Position};

/// Crate version string.
///
/// ```
/// assert!(!tcl_lexer::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
