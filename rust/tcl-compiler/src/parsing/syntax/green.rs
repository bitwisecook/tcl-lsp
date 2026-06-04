//! Position-independent *green* layer of the Tcl concrete syntax tree.
//!
//! Ports `compiler/parsing/syntax/green.py` (#533 / `SYNC-JUN06`).
//!
//! This is the canonical, lossless syntax representation the whole
//! pipeline is meant to ride on — the segmenter, AOT lowering, the
//! formatter, the minifier, and the per-command tooling.  The design is
//! the classic red-green split (Roslyn / rust-analyzer):
//!
//! - **Green** (this module) is *position-independent*: a node knows
//!   only its *width* and its children, never an absolute offset.  Two
//!   structurally identical regions (e.g. two empty `{}` words) are
//!   structurally equal, and a green subtree is reusable verbatim across
//!   edits.
//! - **Red** ([`super::super::syntax`], strip 3) overlays a green tree
//!   with an anchoring and resolves absolute positions lazily,
//!   reproducing exactly the `Token` offsets/lines/columns the lexer
//!   emits.
//!
//! Trivia (whitespace, end-of-line, comments) is **attached** to the
//! adjacent token as leading/trailing trivia rather than living as
//! sibling tokens, so a [`SyntaxKind::Command`] / [`SyntaxKind::Word`]
//! node is pure syntax while the bytes between words are still
//! preserved.  Concatenating every element's [`GreenElement::full_text`]
//! in document order reproduces the source byte-for-byte — the
//! losslessness the formatter and minifier require.
//!
//! **Byte offsets vs Python codepoints.**  The Rust lexer works in byte
//! offsets throughout (see [`tcl_lexer::Span`]), so widths here are byte
//! lengths (`str::len`), where Python's `green.py` uses codepoint
//! `len()`.  For ASCII these coincide; for multibyte text the byte
//! widths stay consistent with the byte offsets the red layer resolves
//! against, matching the rest of the Rust rewrite.
//!
//! See `docs/design/compiler/syntax-tree.md`.

use tcl_lexer::TokenType;

/// A piece of inter-token text that carries no syntactic value of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriviaKind {
    /// Intra-line separators (lexer `SEP`, backslash-newline).
    Whitespace,
    /// Newline(s) / `;` command terminator (lexer `EOL`).
    Eol,
    /// `#` … end-of-line comment (lexer `COMMENT`).
    Comment,
}

/// Structural role of an interior [`GreenNode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxKind {
    /// The whole region: a sequence of commands.
    Document,
    /// One Tcl command (one logical line).
    Command,
    /// One Tcl word: one or more abutting fragments.
    Word,
}

/// One whitespace / end-of-line / comment run, stored by its literal text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GreenTrivia {
    /// Which kind of trivia this run is.
    pub kind: TriviaKind,
    /// The literal source text of the run.
    pub text: String,
}

impl GreenTrivia {
    /// Construct a trivia run from its kind and literal text.
    #[must_use]
    pub fn new(kind: TriviaKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }

    /// Byte width of the run.
    #[must_use]
    pub fn width(&self) -> u32 {
        u32::try_from(self.text.len()).unwrap_or(u32::MAX)
    }
}

/// Sum the byte widths of a trivia slice.
fn trivia_width(trivia: &[GreenTrivia]) -> u32 {
    trivia.iter().map(GreenTrivia::width).sum()
}

/// A leaf: one lexer word-fragment, with attached leading/trailing trivia.
///
/// `token_type` is the lexer [`TokenType`] of the fragment
/// (`Esc` / `Str` / `Cmd` / `Var` / `Expand`); the trivia kinds
/// `Sep` / `Eol` / `Comment` never appear here — they become
/// [`GreenTrivia`].
///
/// Two text fields are kept because both are needed and neither is
/// derivable from the other without re-encoding the lexer's delimiter
/// conventions:
///
/// - [`GreenToken::text`] is the lexer's *inner* text (`abc` for
///   `{abc}`), what `Token.text` carries downstream.
/// - [`GreenToken::raw`] is the full source slice the fragment occupies
///   (`{abc}`), what makes the tree lossless and defines
///   [`GreenToken::width`].
///
/// [`GreenToken::end_rel`] is `Token.end.offset - Token.start.offset` —
/// the position-independent shape that lets the red layer reconstruct
/// the lexer's `end` (which, by the #527 convention, sits on the last
/// *inner* character for a non-empty `{…}` but *on the closer* for an
/// empty `{}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreenToken {
    /// Lexer kind of the fragment.
    pub token_type: TokenType,
    /// Inner text (`Token.text`): `abc` for `{abc}`.
    pub text: String,
    /// Full source slice the fragment occupies: `{abc}`.
    pub raw: String,
    /// `Token.end.offset - Token.start.offset`; the red layer adds the
    /// raw start to reconstruct the lexer's inclusive `end` offset.
    pub end_rel: u32,
    /// Leading delimiter bytes the lexer strips from `raw` to get the
    /// human-readable `text` (1 for `{` / `[` / `$` / `"`, 2 for `${`,
    /// 0 for a bare word).  Position-independent, like [`Self::end_rel`].
    ///
    /// **Rust-specific** (no field in Python's `green.py`): the Rust
    /// [`tcl_lexer::Token`] is span + `content_offset`, where Python's
    /// `Token` is `text` + `start` + `end`.  Green stores whatever the
    /// platform's `Token` needs to be reconstructed losslessly — `text`
    /// (as Python) **and** `content_offset` — so the red layer's
    /// [`super::super::syntax::red::SyntaxToken::to_token`] can rebuild
    /// the exact lexer `Token` (span via `end_rel`, `content_offset`
    /// verbatim).
    pub content_offset: u8,
    /// Whether the fragment was lexed inside a quoted-string context.
    pub in_quote: bool,
    /// Trivia immediately preceding the fragment, in document order.
    pub leading: Vec<GreenTrivia>,
    /// Trivia immediately following the fragment, in document order.
    pub trailing: Vec<GreenTrivia>,
    /// Cached `leading + raw + trailing` byte width.  Derived from the
    /// other fields, so it never affects equality (equal inputs ⇒ equal
    /// width).
    full_width: u32,
}

impl GreenToken {
    /// Construct a green token, caching its full width.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        token_type: TokenType,
        text: impl Into<String>,
        raw: impl Into<String>,
        end_rel: u32,
        content_offset: u8,
        in_quote: bool,
        leading: Vec<GreenTrivia>,
        trailing: Vec<GreenTrivia>,
    ) -> Self {
        let raw = raw.into();
        let full_width = trivia_width(&leading)
            + u32::try_from(raw.len()).unwrap_or(u32::MAX)
            + trivia_width(&trailing);
        Self {
            token_type,
            text: text.into(),
            raw,
            end_rel,
            content_offset,
            in_quote,
            leading,
            trailing,
            full_width,
        }
    }

    /// Narrow width: just the fragment's own bytes (`raw`).
    #[must_use]
    pub fn width(&self) -> u32 {
        u32::try_from(self.raw.len()).unwrap_or(u32::MAX)
    }

    /// Total byte width of the leading trivia.
    #[must_use]
    pub fn leading_width(&self) -> u32 {
        trivia_width(&self.leading)
    }

    /// Total byte width of the trailing trivia.
    #[must_use]
    pub fn trailing_width(&self) -> u32 {
        trivia_width(&self.trailing)
    }

    /// Full byte width including attached trivia.  O(1) (cached).
    #[must_use]
    pub fn full_width(&self) -> u32 {
        self.full_width
    }

    /// Append the fragment's full text (leading trivia + raw + trailing
    /// trivia) to `out`.
    pub fn push_full_text(&self, out: &mut String) {
        for t in &self.leading {
            out.push_str(&t.text);
        }
        out.push_str(&self.raw);
        for t in &self.trailing {
            out.push_str(&t.text);
        }
    }

    /// Full text of the fragment including its attached trivia.
    #[must_use]
    pub fn full_text(&self) -> String {
        let mut out = String::with_capacity(self.full_width as usize);
        self.push_full_text(&mut out);
        out
    }
}

/// A green tree element: an interior [`GreenNode`] or a leaf [`GreenToken`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GreenElement {
    /// An interior node.
    Node(GreenNode),
    /// A leaf token fragment.
    Token(GreenToken),
}

impl GreenElement {
    /// Full byte width including attached trivia.  O(1) (cached).
    #[must_use]
    pub fn full_width(&self) -> u32 {
        match self {
            Self::Node(n) => n.full_width(),
            Self::Token(t) => t.full_width(),
        }
    }

    /// Append the element's full text to `out`.
    pub fn push_full_text(&self, out: &mut String) {
        match self {
            Self::Node(n) => n.push_full_text(out),
            Self::Token(t) => t.push_full_text(out),
        }
    }

    /// Full text of the element including attached trivia.
    #[must_use]
    pub fn full_text(&self) -> String {
        let mut out = String::with_capacity(self.full_width() as usize);
        self.push_full_text(&mut out);
        out
    }
}

/// An interior node: a `Document`, `Command`, or `Word`.
///
/// `expand_markers` are the `{*}` `Expand` leaves when this is an
/// expanded word; they precede the word's fragments in document order
/// but are *not* `children` (the segmenter reports the expanded word's
/// representative token as the first real fragment, not a `{*}` marker).
/// Normally there is one, but `{*}{*}x` lexes as two adjacent markers,
/// so it is a list.
///
/// `trailing` carries document-level dangling trivia — a trailing
/// comment with no following command (`puts hi ;# bye`) attaches to
/// nothing yet must still round-trip; it is appended here on the
/// `Document` node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreenNode {
    /// Structural role of this node.
    pub kind: SyntaxKind,
    /// Child elements in document order.
    pub children: Vec<GreenElement>,
    /// `Word` only: the `{*}` `Expand` markers preceding the fragments.
    pub expand_markers: Vec<GreenToken>,
    /// `Document` only: dangling trivia with no following command.
    pub trailing: Vec<GreenTrivia>,
    /// `Command` only: byte offset of the command's faithful range end,
    /// relative to its first token's raw start (so it stays
    /// position-independent).  Captures the segmenter's word-boundary
    /// rule — including the stale-boundary quirk a trailing `{*}`
    /// introduces — which is not derivable from node geometry.
    pub range_end_rel: Option<u32>,
    /// `Command` only: the comment(s) attached forward to this command,
    /// captured by the segmenter's accumulation rule (which crosses
    /// dangling-`{*}` commands), so it cannot be read off this node's
    /// own leading trivia.
    pub preceding_comment: Option<String>,
    /// Cached full byte width.  Derived from the other fields, so it
    /// never affects equality.
    full_width: u32,
}

impl GreenNode {
    /// Construct a green node, caching its full width.
    ///
    /// Prefer the focused constructors [`GreenNode::document`],
    /// [`GreenNode::command`], and [`GreenNode::word`] at call sites that
    /// build a specific kind.
    #[must_use]
    pub fn new(
        kind: SyntaxKind,
        children: Vec<GreenElement>,
        expand_markers: Vec<GreenToken>,
        trailing: Vec<GreenTrivia>,
        range_end_rel: Option<u32>,
        preceding_comment: Option<String>,
    ) -> Self {
        let width = children.iter().map(GreenElement::full_width).sum::<u32>()
            + expand_markers
                .iter()
                .map(GreenToken::full_width)
                .sum::<u32>()
            + trivia_width(&trailing);
        Self {
            kind,
            children,
            expand_markers,
            trailing,
            range_end_rel,
            preceding_comment,
            full_width: width,
        }
    }

    /// Construct a `Document` node from its commands and dangling trivia.
    #[must_use]
    pub fn document(children: Vec<GreenElement>, trailing: Vec<GreenTrivia>) -> Self {
        Self::new(
            SyntaxKind::Document,
            children,
            Vec::new(),
            trailing,
            None,
            None,
        )
    }

    /// Construct a `Command` node from its words, range end, and comment.
    #[must_use]
    pub fn command(
        children: Vec<GreenElement>,
        range_end_rel: Option<u32>,
        preceding_comment: Option<String>,
    ) -> Self {
        Self::new(
            SyntaxKind::Command,
            children,
            Vec::new(),
            Vec::new(),
            range_end_rel,
            preceding_comment,
        )
    }

    /// Construct a `Word` node from its fragments and `{*}` markers.
    #[must_use]
    pub fn word(children: Vec<GreenElement>, expand_markers: Vec<GreenToken>) -> Self {
        Self::new(
            SyntaxKind::Word,
            children,
            expand_markers,
            Vec::new(),
            None,
            None,
        )
    }

    /// True when this word carries one or more `{*}` expansion markers.
    #[must_use]
    pub fn is_expand(&self) -> bool {
        !self.expand_markers.is_empty()
    }

    /// Full byte width including attached trivia.  O(1) (cached).
    #[must_use]
    pub fn full_width(&self) -> u32 {
        self.full_width
    }

    /// Append the node's full text (expand markers, then children, then
    /// dangling trivia) to `out`.
    pub fn push_full_text(&self, out: &mut String) {
        for m in &self.expand_markers {
            m.push_full_text(out);
        }
        for c in &self.children {
            c.push_full_text(out);
        }
        for t in &self.trailing {
            out.push_str(&t.text);
        }
    }

    /// Full text of the node including all attached trivia.  Concatenated
    /// in document order over a whole `Document`, this reproduces the
    /// source byte-for-byte (losslessness).
    #[must_use]
    pub fn full_text(&self) -> String {
        let mut out = String::with_capacity(self.full_width as usize);
        self.push_full_text(&mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws(text: &str) -> GreenTrivia {
        GreenTrivia::new(TriviaKind::Whitespace, text)
    }

    /// A bare-word token with no trivia (`raw == text`, `end_rel` = last
    /// inner byte).
    fn bare(text: &str) -> GreenToken {
        let end_rel = u32::try_from(text.len().saturating_sub(1)).unwrap();
        GreenToken::new(
            TokenType::Esc,
            text,
            text,
            end_rel,
            0, // bare word: no leading delimiter
            false,
            Vec::new(),
            Vec::new(),
        )
    }

    #[test]
    fn trivia_width_is_byte_length() {
        assert_eq!(ws("   ").width(), 3);
        assert_eq!(GreenTrivia::new(TriviaKind::Eol, "\n").width(), 1);
        assert_eq!(GreenTrivia::new(TriviaKind::Comment, "# hi").width(), 4);
    }

    #[test]
    fn token_widths_account_for_trivia() {
        // `{abc}` braced fragment: raw `{abc}` (5), inner `abc`, with a
        // leading space and a trailing newline.
        let tok = GreenToken::new(
            TokenType::Str,
            "abc",
            "{abc}",
            3, // end_rel: inner end (the `c`) relative to start
            1, // content_offset: the `{`
            false,
            vec![ws(" ")],
            vec![GreenTrivia::new(TriviaKind::Eol, "\n")],
        );
        assert_eq!(tok.width(), 5);
        assert_eq!(tok.leading_width(), 1);
        assert_eq!(tok.trailing_width(), 1);
        assert_eq!(tok.full_width(), 7);
        assert_eq!(tok.full_text(), " {abc}\n");
    }

    #[test]
    fn empty_braced_token_keeps_both_text_fields() {
        // `{}` — raw `{}` (width 2), inner text empty, end_rel 1 (the
        // closer, per the #527 convention).
        let tok = GreenToken::new(
            TokenType::Str,
            "",
            "{}",
            1,
            1,
            false,
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(tok.text, "");
        assert_eq!(tok.raw, "{}");
        assert_eq!(tok.width(), 2);
        assert_eq!(tok.end_rel, 1);
        assert_eq!(tok.full_text(), "{}");
    }

    #[test]
    fn word_node_sums_child_and_marker_widths() {
        // Compound word `{a}b`: two abutting fragments, no trivia.
        let frag_a = GreenToken::new(
            TokenType::Str,
            "a",
            "{a}",
            1,
            1,
            false,
            Vec::new(),
            Vec::new(),
        );
        let frag_b = bare("b");
        let word = GreenNode::word(
            vec![GreenElement::Token(frag_a), GreenElement::Token(frag_b)],
            Vec::new(),
        );
        assert_eq!(word.kind, SyntaxKind::Word);
        assert!(!word.is_expand());
        assert_eq!(word.full_width(), 4); // `{a}` (3) + `b` (1)
        assert_eq!(word.full_text(), "{a}b");
    }

    #[test]
    fn expand_word_carries_markers_outside_children() {
        // `{*}$x` — one EXPAND marker, one VAR fragment.
        let marker = GreenToken::new(
            TokenType::Expand,
            "",
            "{*}",
            0, // EXPAND lexes to a zero-width ghost token (end_rel 0)
            0,
            false,
            Vec::new(),
            Vec::new(),
        );
        let frag = GreenToken::new(
            TokenType::Var,
            "x",
            "$x",
            1,
            1,
            false,
            Vec::new(),
            Vec::new(),
        );
        let word = GreenNode::word(vec![GreenElement::Token(frag)], vec![marker]);
        assert!(word.is_expand());
        // Markers are NOT children but DO contribute to width/text.
        assert_eq!(word.children.len(), 1);
        assert_eq!(word.expand_markers.len(), 1);
        assert_eq!(word.full_width(), 5); // `{*}` (3) + `$x` (2)
        assert_eq!(word.full_text(), "{*}$x");
    }

    #[test]
    fn command_node_records_range_and_comment() {
        let word = GreenNode::word(vec![GreenElement::Token(bare("puts"))], Vec::new());
        let cmd = GreenNode::command(
            vec![GreenElement::Node(word)],
            Some(4),
            Some("doc".to_string()),
        );
        assert_eq!(cmd.kind, SyntaxKind::Command);
        assert_eq!(cmd.range_end_rel, Some(4));
        assert_eq!(cmd.preceding_comment.as_deref(), Some("doc"));
        assert_eq!(cmd.full_text(), "puts");
    }

    #[test]
    fn document_full_text_round_trips_source() {
        // `puts hi\n` — one command, the inter-word space is trailing
        // trivia on `puts`, the newline trailing trivia on `hi`.
        let puts = GreenToken::new(
            TokenType::Esc,
            "puts",
            "puts",
            3,
            0,
            false,
            Vec::new(),
            vec![ws(" ")],
        );
        let hi = GreenToken::new(
            TokenType::Esc,
            "hi",
            "hi",
            1,
            0,
            false,
            Vec::new(),
            vec![GreenTrivia::new(TriviaKind::Eol, "\n")],
        );
        let cmd = GreenNode::command(
            vec![
                GreenElement::Node(GreenNode::word(vec![GreenElement::Token(puts)], Vec::new())),
                GreenElement::Node(GreenNode::word(vec![GreenElement::Token(hi)], Vec::new())),
            ],
            None,
            None,
        );
        let doc = GreenNode::document(vec![GreenElement::Node(cmd)], Vec::new());
        assert_eq!(doc.full_text(), "puts hi\n");
        assert_eq!(doc.full_width(), 8);
    }

    #[test]
    fn document_trailing_trivia_round_trips() {
        // `puts hi ;# bye` — the dangling `;# bye` comment after the
        // command attaches to the document's `trailing`.
        let puts = GreenToken::new(
            TokenType::Esc,
            "puts",
            "puts",
            3,
            0,
            false,
            Vec::new(),
            vec![ws(" ")],
        );
        let hi = GreenToken::new(
            TokenType::Esc,
            "hi",
            "hi",
            1,
            0,
            false,
            Vec::new(),
            Vec::new(),
        );
        let cmd = GreenNode::command(
            vec![
                GreenElement::Node(GreenNode::word(vec![GreenElement::Token(puts)], Vec::new())),
                GreenElement::Node(GreenNode::word(vec![GreenElement::Token(hi)], Vec::new())),
            ],
            None,
            None,
        );
        let doc = GreenNode::document(
            vec![GreenElement::Node(cmd)],
            vec![
                GreenTrivia::new(TriviaKind::Eol, " ;"),
                GreenTrivia::new(TriviaKind::Comment, "# bye"),
            ],
        );
        assert_eq!(doc.full_text(), "puts hi ;# bye");
    }

    #[test]
    fn structurally_identical_empty_braces_compare_equal() {
        // The shareability property: two independently built empty `{}`
        // words are structurally equal (the cached width does not
        // perturb equality).
        let a = GreenNode::word(
            vec![GreenElement::Token(GreenToken::new(
                TokenType::Str,
                "",
                "{}",
                1,
                1,
                false,
                Vec::new(),
                Vec::new(),
            ))],
            Vec::new(),
        );
        let b = GreenNode::word(
            vec![GreenElement::Token(GreenToken::new(
                TokenType::Str,
                "",
                "{}",
                1,
                1,
                false,
                Vec::new(),
                Vec::new(),
            ))],
            Vec::new(),
        );
        assert_eq!(a, b);
    }

    #[test]
    fn cached_width_does_not_perturb_token_equality() {
        // Two tokens with identical fields are equal; the cached
        // full_width is a pure function of those fields.
        let a = bare("abc");
        let b = bare("abc");
        assert_eq!(a, b);
        assert_eq!(a.full_width(), b.full_width());
    }
}
