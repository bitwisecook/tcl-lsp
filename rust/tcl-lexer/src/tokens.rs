//! Token, position, and kind types produced by the Tcl lexer.
//!
//! Ports `core/parsing/tokens.py` as idiomatic Rust:
//!
//! - [`TokenType`] is a `Copy` enum with `PascalCase` variants. The
//!   `PyO3` binding crate exposes the variants under their original
//!   `SCREAMING_CASE` Python names.
//! - [`SourcePosition`] is a 12-byte `Copy` struct of `u32` fields. Both
//!   line/character (LSP UTF-16 code units) and offset (byte offset)
//!   comfortably fit in 32 bits for any source we care about.
//! - [`Token`] borrows its text from the source buffer via `&'src str`.
//!   The `PyO3` wrapper clones the slice into an owned `String` on the
//!   way out — Python doesn't model lifetimes, and the source buffer
//!   cannot be assumed to outlive the Python token object.
//!
//! Field names follow Rust conventions (`kind`, not `type`); the binding
//! crate renames `kind` to `type` for Python callers in the obvious
//! place.

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
}

impl TokenType {
    /// Symbolic name of the variant — `"ESC"`, `"STR"`, etc.
    ///
    /// Used by the `PyO3` wrapper to mimic `enum.Enum.name` and by
    /// debug-print code in CLI tools. The mapping is fixed by the
    /// Python API surface and must not change without a coordinated
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
        }
    }
}

/// A position in source text.
///
/// Stores 0-based line and character (the latter measured in UTF-16 code
/// units to match the LSP specification) plus the absolute byte offset
/// into the source string. All three fields are `u32`: that matches the
/// LSP wire types and bounds the largest source file we care about at
/// 4 GiB, well above any realistic Tcl/iRules input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourcePosition {
    /// 0-based line number.
    pub line: u32,
    /// 0-based column in UTF-16 code units (per LSP spec).
    pub character: u32,
    /// Byte offset into the source string.
    pub offset: u32,
}

impl SourcePosition {
    /// Construct a new `SourcePosition`. `const`-friendly so callers can
    /// build sentinel positions in static contexts.
    #[must_use]
    pub const fn new(line: u32, character: u32, offset: u32) -> Self {
        Self {
            line,
            character,
            offset,
        }
    }
}

/// A token: kind, text, source range, and quoting context.
///
/// `text` borrows from the source buffer via `&'src str`. Synthetic
/// tokens (e.g. EOF, error-recovery markers) borrow from `&""` or any
/// other `'static` string. Borrowing rather than owning means the lexer
/// allocates nothing per token; the binding crate clones into owned
/// `String`s only when crossing the FFI boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token<'src> {
    /// Token kind.
    pub kind: TokenType,
    /// Borrowed text content (no surrounding `{}`, `[]`, `"`, or `$`).
    pub text: &'src str,
    /// Position of the first character of the token in the source.
    pub start: SourcePosition,
    /// Position of the last character of the token in the source
    /// (inclusive).
    pub end: SourcePosition,
    /// True when the token was emitted inside a quoted-string context.
    /// The Python lexer carries this for downstream consumers that need
    /// to distinguish bare words from quoted runs.
    pub in_quote: bool,
}

impl<'src> Token<'src> {
    /// Construct a token with `in_quote = false`. The lexer uses this
    /// for the overwhelmingly common bare-word case; tokens emitted
    /// inside quotes use [`Token::new_quoted`].
    #[must_use]
    pub const fn new(
        kind: TokenType,
        text: &'src str,
        start: SourcePosition,
        end: SourcePosition,
    ) -> Self {
        Self {
            kind,
            text,
            start,
            end,
            in_quote: false,
        }
    }

    /// Construct a token with `in_quote = true`.
    #[must_use]
    pub const fn new_quoted(
        kind: TokenType,
        text: &'src str,
        start: SourcePosition,
        end: SourcePosition,
    ) -> Self {
        Self {
            kind,
            text,
            start,
            end,
            in_quote: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // The tests below port the Python-side coverage of `tokens.py`. The
    // Python tests historically exercise these types implicitly through
    // the lexer suite (tests/test_lexer.py, tests/test_token_positions.py,
    // tests/test_incremental_update.py, tests/test_formatter.py); the
    // assertions here pin down the contract directly so a regression in
    // the Rust types is caught at `cargo test` time, before the
    // differential lexer harness runs.

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
    fn token_type_equality_matches_python_semantics() {
        // Same variant compares equal; different variants do not.
        assert_eq!(TokenType::Esc, TokenType::Esc);
        assert_ne!(TokenType::Esc, TokenType::Str);
        // Pattern-matching, the Rust analogue of Python `match tok.type`.
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
        // Mirrors `SourcePosition(line=5, character=10, offset=100)` from
        // tests/test_incremental_update.py::test_shift_position.
        let pos = SourcePosition::new(5, 10, 100);
        assert_eq!(pos.line, 5);
        assert_eq!(pos.character, 10);
        assert_eq!(pos.offset, 100);
    }

    #[test]
    fn source_position_default_is_origin() {
        let pos = SourcePosition::default();
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
        assert_eq!(pos.offset, 0);
    }

    #[test]
    fn source_position_equality_and_hash() {
        let a = SourcePosition::new(1, 2, 3);
        let b = SourcePosition::new(1, 2, 3);
        let c = SourcePosition::new(1, 2, 4);
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
        let a = SourcePosition::new(1, 2, 3);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn token_construction_borrows_text() {
        // Mirrors `Token(type=TokenType.ESC, text="hello", start=p, end=p)`
        // from tests/test_formatter.py.
        let source = String::from("hello world");
        let start = SourcePosition::new(0, 0, 0);
        let end = SourcePosition::new(0, 4, 4);
        let tok = Token::new(TokenType::Esc, &source[..5], start, end);
        assert_eq!(tok.kind, TokenType::Esc);
        assert_eq!(tok.text, "hello");
        assert_eq!(tok.start, start);
        assert_eq!(tok.end, end);
        assert!(!tok.in_quote);
    }

    #[test]
    fn token_default_in_quote_is_false() {
        let pos = SourcePosition::default();
        let tok = Token::new(TokenType::Str, "abc", pos, pos);
        assert!(!tok.in_quote);
    }

    #[test]
    fn token_quoted_constructor_sets_in_quote() {
        let pos = SourcePosition::default();
        let tok = Token::new_quoted(TokenType::Esc, "abc", pos, pos);
        assert!(tok.in_quote);
    }

    #[test]
    fn token_equality_compares_all_fields() {
        let pos = SourcePosition::default();
        let baseline = Token::new(TokenType::Esc, "x", pos, pos);
        let same = Token::new(TokenType::Esc, "x", pos, pos);
        let different_text = Token::new(TokenType::Esc, "y", pos, pos);
        let different_kind = Token::new(TokenType::Str, "x", pos, pos);
        let different_quote = Token::new_quoted(TokenType::Esc, "x", pos, pos);
        assert_eq!(baseline, same);
        assert_ne!(baseline, different_text);
        assert_ne!(baseline, different_kind);
        assert_ne!(baseline, different_quote);
    }

    #[test]
    fn token_hash_distinguishes_in_quote() {
        let pos = SourcePosition::default();
        let bare = Token::new(TokenType::Esc, "x", pos, pos);
        let quoted = Token::new_quoted(TokenType::Esc, "x", pos, pos);
        let mut set = HashSet::new();
        set.insert(bare);
        set.insert(quoted);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn token_text_lifetime_borrows_from_source() {
        // Compile-time check: a Token's text borrows the source slice
        // and is not allowed to outlive it. The function signature
        // makes the borrow explicit; if `Token` ever grew an owned
        // `String` field this would still compile but the test would
        // become much less informative.
        fn first_word(src: &str) -> Token<'_> {
            let end_idx =
                u32::try_from(src.find(' ').unwrap_or(src.len())).expect("test source fits in u32");
            Token::new(
                TokenType::Esc,
                &src[..end_idx as usize],
                SourcePosition::new(0, 0, 0),
                SourcePosition::new(0, end_idx, end_idx),
            )
        }
        let source = String::from("alpha beta");
        let tok = first_word(&source);
        assert_eq!(tok.text, "alpha");
        assert_eq!(tok.end.offset, 5);
    }
}
