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
//! - [`word_closer_offset`], [`word_end_position`],
//!   [`word_append_offset`], [`word_span`] — source-aware
//!   authoritative closer accessors for delimited word tokens, derived
//!   from the lexer's content geometry (correct for empty `{}` / `[]` /
//!   `""` and backslash-bearing quoted words).  [`word_span`] is the one
//!   a caller anchoring a diagnostic or slicing a word's raw text wants:
//!   it returns the whole written word, closing delimiter included, where
//!   [`Token::span`] alone stops at the end of the word's *content*.
//! - [`close_quote_offset`] — the same closer question for a `"…"` word
//!   answered from a bare byte offset, for callers holding source and an
//!   offset rather than a token (it skips `\`-escapes and whole `[…]`
//!   command substitutions).
//! - [`command_substitution_end`] — byte offset just past a complete `[…]`
//!   command substitution, including Tcl's nested-script/comment rules.
//! - [`word_closer_offset_at`], [`word_span_at`] — the same closer
//!   question for **any** delimited word (`${name}` included) answered
//!   from the word's own [`Span`], for callers that kept a span but not
//!   the [`Token`] it came from.
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
    tokenise_expr_checked, tokenise_expr_checked_for_profile, tokenise_expr_for_profile,
};
#[cfg(feature = "html")]
pub use highlight::{
    HlRange, highlight_ranges, highlight_ranges_with_config, highlight_tcl,
    highlight_tcl_with_config,
};
pub use lexer::{LeadingBom, LexError, LexWarning, Lexer, LexerConfig, UTF8_BOM};
pub use line_index::{LineIndex, normalise_lone_cr};
pub use ranges::{
    close_quote_offset, command_substitution_end, word_append_offset, word_closer_offset,
    word_closer_offset_at, word_end_position, word_span, word_span_at,
};
pub use source_map::SourceMap;
pub use span::Span;
pub use structural_index::{
    BraceIndex, BracketIndex, ExprParenIndex, ParenBalance, command_boundaries, reparse_window,
    script_is_complete,
};
pub use substitution::{
    EscapeSegment, backslash_escape_end, backslash_escape_end_in, backslash_subst,
    backslash_subst_in, split_backslash_escapes, split_backslash_escapes_in,
};
// Re-exported from the foundational dialect crate so existing
// `tcl_lexer::BracedVarStyle` imports keep working — the enum moved down to
// `tcl-dialect` (dialect-profile-model.md §3) where the `DialectProfile`
// grammar axis shares it.
pub use tcl_dialect::{BracedVarStyle, EscapeSyntax};
pub use tokens::{ByteCol, SourcePosition, Token, TokenType, Utf16Col, Utf16Position};

/// Return the physical line numbers whose first non-horizontal-whitespace
/// character is a Tcl comment token.
///
/// The lexer is the authority for whether `#` starts a comment: it is only a
/// comment at command position, and never while a braced, quoted, or bracketed
/// word is open. Inline command-position comments after a semicolon are not
/// returned because they do not occupy a comment line by themselves.
#[must_use]
pub fn comment_line_starts(source: &str, config: LexerConfig) -> Vec<u32> {
    let line_index = LineIndex::new(source);
    let Ok(tokens) = Lexer::with_config(source, config).tokenise_all() else {
        return Vec::new();
    };
    tokens
        .into_iter()
        .filter(|token| token.kind == TokenType::Comment)
        .filter_map(|token| {
            let start = usize::try_from(token.span.start()).ok()?;
            let line_start = source[..start].rfind('\n').map_or(0, |offset| offset + 1);
            source[line_start..start]
                .bytes()
                .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | 0x0b | 0x0c))
                .then(|| line_index.line_at(token.span.start()))
        })
        .collect()
}

/// Crate version string.
///
/// ```
/// assert!(!tcl_lexer::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
