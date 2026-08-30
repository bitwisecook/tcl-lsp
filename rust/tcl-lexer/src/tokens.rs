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

//! Token, position, and kind types produced by the Tcl lexer.
//!
//! Idiomatic Rust token types:
//!
//! - [`TokenType`] is a `Copy` enum with `PascalCase` variants. The
//!   `PyO3` binding crate exposes the variants under their original
//!   `SCREAMING_CASE` names.
//! - [`SourcePosition`] is a 12-byte `Copy` struct of `u32` fields.
//!   Line, character, and offset all fit comfortably in 32 bits for
//!   any source we care about. Exposed via the binding
//!   crate and returned by [`SourceMap`] lookups.
//! - [`Token`] carries a [`Span`] — not an inline text slice and not
//!   inline start/end positions. Text and positions are resolved on
//!   demand via a [`SourceMap`], the same way every future IR and
//!   CFG node will work. `Token` has no lifetime, is 16 bytes, and
//!   trivially `Copy`.
//!
//! Field names follow Rust conventions (`kind`, not `type`); the
//! binding crate renames `kind` to `type` and
//! resolves `span` to `start` / `end` / `text` at the FFI boundary.
//!
//! [`Span`]: crate::Span
//! [`SourceMap`]: crate::SourceMap

/// Kinds of tokens produced by the Tcl lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenType {
    /// Plain string fragment, possibly containing escape sequences.
    Esc,
    /// Braced string `{…}` (the braces are stripped from the token text).
    Str,
    /// Command substitution `[…]` (the brackets are stripped).
    Cmd,
    /// Variable substitution `$name`, `${name}`, or `$arr(idx)`.
    Var,
    /// Run of intra-command whitespace separators (space, tab, etc.).
    Sep,
    /// End-of-line: newline or `;`.
    Eol,
    /// End-of-input sentinel.
    Eof,
    /// Comment from `#` to end of line.
    Comment,
    /// `{*}` argument-expansion prefix (Tcl 8.5+).
    Expand,
    /// `JimTcl` expression substitution `$(…)` — the parenthesised text is an
    /// `expr` body, not a script and not a dict index (the parens are
    /// stripped from the token text).
    ///
    /// Jim's parser reaches this through the *command*-substitution scanner
    /// and then retags the token (`JimParseVar` → `JIM_TT_EXPRSUGAR`,
    /// `jim.c:1728-1732`), so it nests and terminates exactly like
    /// [`Self::Cmd`]. It is a distinct kind because the body's *grammar*
    /// differs: `$($a * 2)` holds the expression `$a * 2`, which a consumer
    /// must analyse with the expression parser rather than the script one.
    ///
    /// Emitted only under [`VarSyntax::Jim`](tcl_dialect::VarSyntax::Jim); no
    /// build of the Tcl core produces it.
    ExprSugar,
}

impl TokenType {
    /// The closing delimiter stripped from this token kind's text (`Str` →
    /// `}`, `Cmd` → `]`), or `None` for a kind that isn't a bracketed group.
    /// A group token's raw span may or may not include its closer — see
    /// the "inner-end convention" this closer feeds into at each call site
    /// (`segmenter::widen_word_end`, `value_shapes::widened_token_text`).
    #[must_use]
    pub const fn group_closer(self) -> Option<char> {
        match self {
            Self::Str => Some('}'),
            Self::Cmd => Some(']'),
            Self::ExprSugar => Some(')'),
            _ => None,
        }
    }
}

impl TokenType {
    /// Symbolic name of the variant — `"ESC"`, `"STR"`, etc.
    ///
    /// Used by the `PyO3` wrapper to mimic `enum.Enum.name` and by
    /// debug-print code in CLI tools. The mapping is fixed by the
    /// public API surface and must not change without a coordinated
    /// shim update.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Esc => "ESC",
            Self::Str => "STR",
            Self::Cmd => "CMD",
            Self::Var => "VAR",
            Self::Sep => "SEP",
            Self::Eol => "EOL",
            Self::Eof => "EOF",
            Self::Comment => "COMMENT",
            Self::Expand => "EXPAND",
            Self::ExprSugar => "EXPRSUGAR",
        }
    }
}

/// A zero-based column counted in **bytes** from the start of its line.
///
/// This is the raw column [`LineIndex::position_at`](crate::LineIndex::position_at)
/// produces; it agrees with an LSP UTF-16 column ([`Utf16Col`]) only for ASCII
/// text. Kept a distinct type so a byte column can never be handed to an LSP
/// consumer that expects UTF-16 code units — the conversion has to go through
/// [`LineIndex::position_at_utf16`](crate::LineIndex::position_at_utf16). Build
/// one with [`ByteCol::new`], read it back with [`ByteCol::get`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ByteCol(u32);

impl ByteCol {
    /// Wrap a raw byte-column count.
    #[must_use]
    pub const fn new(bytes: u32) -> Self {
        Self(bytes)
    }

    /// The raw byte-column count.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for ByteCol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A position in source text whose `character` column is counted in **bytes**
/// (see [`ByteCol`]).
///
/// Stores a 0-based line and byte column plus the absolute byte offset into the
/// source string. Produced by
/// [`LineIndex::position_at`](crate::LineIndex::position_at); the UTF-16-column
/// counterpart is [`Utf16Position`]. Keeping the two columns as separate types
/// means the byte column can never be mistaken for an LSP UTF-16 column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourcePosition {
    /// 0-based line number.
    pub line: u32,
    /// 0-based column, in bytes from the line start.
    pub character: ByteCol,
    /// Byte offset into the source string.
    pub offset: u32,
}

impl SourcePosition {
    /// Construct a new `SourcePosition`. `const`-friendly so callers can
    /// build sentinel positions in static contexts.
    #[must_use]
    pub const fn new(line: u32, character: ByteCol, offset: u32) -> Self {
        Self {
            line,
            character,
            offset,
        }
    }
}

/// A zero-based column counted in **UTF-16 code units** from the start of its
/// line — the unit the LSP `Position.character` field is defined in.
///
/// Kept distinct from a byte column ([`SourcePosition::character`]) so the two
/// cannot be interchanged without an explicit conversion: only ASCII text makes
/// them numerically equal, and silently swapping the units is the classic
/// off-by-column LSP bug. Build one with [`Utf16Col::new`], read it back with
/// [`Utf16Col::get`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Utf16Col(u32);

impl Utf16Col {
    /// Wrap a raw UTF-16 code-unit count.
    #[must_use]
    pub const fn new(units: u32) -> Self {
        Self(units)
    }

    /// The raw UTF-16 code-unit count.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for Utf16Col {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An LSP-style source position whose `character` column is counted in UTF-16
/// code units (see [`Utf16Col`]).
///
/// Produced by [`LineIndex::position_at_utf16`](crate::LineIndex::position_at_utf16);
/// the byte-column counterpart is [`SourcePosition`]. Keeping the two as
/// separate types means a byte position can never be handed to an LSP consumer
/// that expects UTF-16 columns without the mismatch surfacing at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Utf16Position {
    /// 0-based line number.
    pub line: u32,
    /// 0-based column, in UTF-16 code units.
    pub character: Utf16Col,
    /// Byte offset into the source string.
    pub offset: u32,
}

impl Utf16Position {
    /// Construct a new `Utf16Position`.
    #[must_use]
    pub const fn new(line: u32, character: Utf16Col, offset: u32) -> Self {
        Self {
            line,
            character,
            offset,
        }
    }
}

/// A Tcl token: kind, source span, and quoting context.
///
/// Stores **only** the kind, a byte-level [`Span`] into the source
/// buffer, and the `in_quote` flag. Text and `(line, character)`
/// positions live exclusively on [`SourceMap`], so callers that need
/// them ask the `SourceMap` explicitly:
///
/// ```
/// use tcl_lexer::{Lexer, SourceMap};
/// let source = "puts hello";
/// let source_map = SourceMap::new(source);
/// let tokens = Lexer::new(source).tokenise_all().unwrap();
/// let first = tokens[0];
/// assert_eq!(source_map.text(first.span), "puts");
/// let (start, end) = source_map.range_positions(first.span);
/// assert_eq!(start.line, 0);
/// assert_eq!(end.character.get(), 3);
/// ```
///
/// The `Token` itself is 16 bytes, `Copy`, and has no lifetime —
/// trivially storable in arrays, channels, IR nodes, and CFG
/// structures.
///
/// [`Span`]: crate::Span
/// [`SourceMap`]: crate::SourceMap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    /// Token kind.
    pub kind: TokenType,
    /// Byte range in the source. Resolve to text or positions via a
    /// [`SourceMap`](crate::SourceMap).
    pub span: crate::span::Span,
    /// Number of leading bytes in `span` that are delimiters rather
    /// than content. Used by [`SourceMap::token_text`] to strip the
    /// opening `$` / `${` / `[` / `{` / `"` (etc.) from wrapper
    /// tokens so the "human-readable" text is the inner content
    /// without needing a separate content range.
    ///
    /// Always `0` for bare-word `Esc` / `Sep` / `Eol` / `Comment`
    /// tokens where the span is the content. For `Var`, `Cmd`,
    /// `Str`, and quoted-opening `Esc` tokens, the lexer sets this
    /// to the number of opening delimiter bytes (1 for most
    /// wrappers, 2 for `${...}`).
    ///
    /// [`SourceMap::token_text`]: crate::SourceMap::token_text
    pub content_offset: u8,
    /// True when the token was emitted inside a quoted-string context.
    /// Carried for downstream consumers that need to distinguish bare
    /// words from quoted runs.
    pub in_quote: bool,
}

impl Token {
    /// Construct a token with `in_quote = false` and
    /// `content_offset = 0`. Used for bare-word and
    /// non-wrapper kinds.
    #[must_use]
    pub const fn new(kind: TokenType, span: crate::span::Span) -> Self {
        Self {
            kind,
            span,
            content_offset: 0,
            in_quote: false,
        }
    }

    /// Construct a token with an explicit content offset (bytes of
    /// leading delimiter to strip). Used by parsers for wrapper
    /// kinds — `Var` (strip `$` or `${`), `Cmd` (strip `[`),
    /// `Str` (strip `{`), and quoted-opening `Esc` (strip `"`).
    #[must_use]
    pub const fn with_content_offset(
        kind: TokenType,
        span: crate::span::Span,
        content_offset: u8,
    ) -> Self {
        Self {
            kind,
            span,
            content_offset,
            in_quote: false,
        }
    }

    /// Construct a token with `in_quote = true` and
    /// `content_offset = 0`.
    #[must_use]
    pub const fn new_quoted(kind: TokenType, span: crate::span::Span) -> Self {
        Self {
            kind,
            span,
            content_offset: 0,
            in_quote: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // The tests below exercise the token types and the
    // variant-to-name contract. These types are otherwise exercised
    // implicitly through the lexer suite; the
    // assertions here pin down the contract directly so a regression in
    // the Rust types is caught at `cargo test` time, before the
    // differential lexer harness runs.

    #[test]
    fn token_type_name_exact_mapping() {
        // Rust variant → binding-visible name. This is the contract the
        // PyO3 binding crate relies on when it exposes the enum with
        // `#[pyo3(name = "…")]` on each variant; if these ever
        // disagree, callers see a mismatched symbol.
        assert_eq!(TokenType::Esc.name(), "ESC");
        assert_eq!(TokenType::Str.name(), "STR");
        assert_eq!(TokenType::Cmd.name(), "CMD");
        assert_eq!(TokenType::Var.name(), "VAR");
        assert_eq!(TokenType::Sep.name(), "SEP");
        assert_eq!(TokenType::Eol.name(), "EOL");
        assert_eq!(TokenType::Eof.name(), "EOF");
        assert_eq!(TokenType::Comment.name(), "COMMENT");
        assert_eq!(TokenType::Expand.name(), "EXPAND");
    }

    #[test]
    fn token_type_variants_have_distinct_names() {
        let names = [
            TokenType::Esc.name(),
            TokenType::Str.name(),
            TokenType::Cmd.name(),
            TokenType::Var.name(),
            TokenType::Sep.name(),
            TokenType::Eol.name(),
            TokenType::Eof.name(),
            TokenType::Comment.name(),
            TokenType::Expand.name(),
        ];
        let unique: HashSet<&'static str> = names.into_iter().collect();
        assert_eq!(unique.len(), names.len());
        assert!(unique.contains("ESC"));
        assert!(unique.contains("STR"));
        assert!(unique.contains("CMD"));
        assert!(unique.contains("VAR"));
        assert!(unique.contains("SEP"));
        assert!(unique.contains("EOL"));
        assert!(unique.contains("EOF"));
        assert!(unique.contains("COMMENT"));
        assert!(unique.contains("EXPAND"));
    }

    #[test]
    fn token_type_equality() {
        // Same variant compares equal; different variants do not.
        assert_eq!(TokenType::Esc, TokenType::Esc);
        assert_ne!(TokenType::Esc, TokenType::Str);
        // Rust pattern-matching on the token type.
        let kind = TokenType::Var;
        let label = match kind {
            TokenType::Var => "var",
            _ => "other",
        };
        assert_eq!(label, "var");
    }

    #[test]
    fn token_type_is_copy_and_hashable() {
        // Compile-time `Copy` check via shadowing.
        let a = TokenType::Cmd;
        let b = a;
        let _ = a;
        let _ = b;
        // Used as a `HashSet` key.
        let mut set = HashSet::new();
        set.insert(TokenType::Esc);
        set.insert(TokenType::Esc);
        set.insert(TokenType::Str);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn source_position_construction_and_field_access() {
        // A position at line 5, character 10, byte offset 100.
        let pos = SourcePosition::new(5, ByteCol::new(10), 100);
        assert_eq!(pos.line, 5);
        assert_eq!(pos.character.get(), 10);
        assert_eq!(pos.offset, 100);
    }

    #[test]
    fn source_position_default_is_origin() {
        let pos = SourcePosition::default();
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character.get(), 0);
        assert_eq!(pos.offset, 0);
    }

    #[test]
    fn source_position_equality_and_hash() {
        let a = SourcePosition::new(1, ByteCol::new(2), 3);
        let b = SourcePosition::new(1, ByteCol::new(2), 3);
        let c = SourcePosition::new(1, ByteCol::new(2), 4);
        assert_eq!(a, b);
        assert_ne!(a, c);

        let mut set = HashSet::new();
        set.insert(a);
        set.insert(b);
        set.insert(c);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn source_position_is_copy() {
        // Copy check via shadowing.
        let a = SourcePosition::new(1, ByteCol::new(2), 3);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn source_position_accepts_u32_max_values() {
        // A paranoid check that the struct's `u32` fields do not
        // narrow on any supported target. We don't expect real Tcl
        // files to approach these values, but the type must not
        // panic or overflow if a fabricated test fixture does.
        let pos = SourcePosition::new(u32::MAX, ByteCol::new(u32::MAX), u32::MAX);
        assert_eq!(pos.line, u32::MAX);
        assert_eq!(pos.character.get(), u32::MAX);
        assert_eq!(pos.offset, u32::MAX);
    }

    #[test]
    fn token_construction_carries_span() {
        // Matches the shape every real call site uses: build a span
        // pointing at some byte range in the source, then resolve it
        // via a SourceMap for text or positions.
        use crate::{SourceMap, Span};
        let source = "hello world";
        let map = SourceMap::new(source);
        let span = Span::new(0, 5);
        let tok = Token::new(TokenType::Esc, span);
        assert_eq!(tok.kind, TokenType::Esc);
        assert_eq!(tok.span, span);
        assert!(!tok.in_quote);
        assert_eq!(map.text(tok.span), "hello");
    }

    #[test]
    fn token_default_in_quote_is_false() {
        use crate::Span;
        let tok = Token::new(TokenType::Str, Span::empty(0));
        assert!(!tok.in_quote);
    }

    #[test]
    fn token_quoted_constructor_sets_in_quote() {
        use crate::Span;
        let tok = Token::new_quoted(TokenType::Esc, Span::empty(0));
        assert!(tok.in_quote);
    }

    #[test]
    fn token_equality_compares_all_fields() {
        use crate::Span;
        let span = Span::new(0, 1);
        let baseline = Token::new(TokenType::Esc, span);
        let same = Token::new(TokenType::Esc, span);
        let different_span = Token::new(TokenType::Esc, Span::new(1, 2));
        let different_kind = Token::new(TokenType::Str, span);
        let different_quote = Token::new_quoted(TokenType::Esc, span);
        assert_eq!(baseline, same);
        assert_ne!(baseline, different_span);
        assert_ne!(baseline, different_kind);
        assert_ne!(baseline, different_quote);
    }

    #[test]
    fn token_hash_distinguishes_in_quote() {
        use crate::Span;
        let span = Span::new(0, 1);
        let bare = Token::new(TokenType::Esc, span);
        let quoted = Token::new_quoted(TokenType::Esc, span);
        let mut set = HashSet::new();
        set.insert(bare);
        set.insert(quoted);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn ghost_eof_token_uses_empty_span() {
        // EOF tokens have no source coverage — they are "ghost"
        // tokens, anchored at the EOF offset via an empty span. The
        // SourceMap resolves them to a zero-width range at the
        // correct position. "Ghost" is our term of art for tokens
        // (and, later, characters emitted during error recovery)
        // that exist in the stream without backing source bytes —
        // chosen over "synthetic" / "virtual" to avoid collisions
        // with Rust vocabulary (`virtual` is a reserved keyword).
        use crate::{SourceMap, Span};
        let source = "foo";
        let map = SourceMap::new(source);
        let eof_offset = u32::try_from(source.len()).unwrap();
        let eof = Token::new(TokenType::Eof, Span::empty(eof_offset));
        assert_eq!(eof.kind, TokenType::Eof);
        assert!(eof.span.is_empty());
        assert_eq!(map.text(eof.span), "");
        let (start, end) = map.range_positions(eof.span);
        assert_eq!(start, SourcePosition::new(0, ByteCol::new(3), 3));
        assert_eq!(end, SourcePosition::new(0, ByteCol::new(3), 3));
    }

    #[test]
    fn token_has_no_lifetime() {
        // Compile-time invariant: tokens are pure data, no lifetime,
        // so they can be stored in `Vec`s that outlive the source
        // buffer without lifetime bookkeeping. Resolving tokens back
        // to text always requires a fresh `SourceMap` reference.
        use crate::Span;
        fn make_token() -> Token {
            Token::new(TokenType::Esc, Span::new(0, 5))
        }
        let tok = make_token();
        assert_eq!(tok.span.len(), 5);
    }
}
