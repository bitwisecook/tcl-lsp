// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
pub use highlight::{
    HlRange, highlight_ranges, highlight_ranges_with_config, highlight_tcl,
    highlight_tcl_with_config,
};
pub use lexer::{LexError, LexWarning, Lexer, LexerConfig};
pub use line_index::{LineIndex, normalise_lone_cr};
pub use ranges::{word_closer_offset, word_end_position};
pub use source_map::SourceMap;
pub use span::Span;
pub use structural_index::{
    BraceIndex, BracketIndex, ExprParenIndex, ParenBalance, command_boundaries, reparse_window,
    script_is_complete,
};
pub use substitution::{
    EscapeSegment, backslash_escape_end, backslash_subst, split_backslash_escapes,
};
// Re-exported from the foundational dialect crate so existing
// `tcl_lexer::BracedVarStyle` imports keep working — the enum moved down to
// `tcl-dialect` (dialect-profile-model.md §3) where the `DialectProfile`
// grammar axis shares it.
pub use tcl_dialect::BracedVarStyle;
pub use tokens::{ByteCol, SourcePosition, Token, TokenType, Utf16Col, Utf16Position};

/// Crate version string.
///
/// ```
/// assert!(!tcl_lexer::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
