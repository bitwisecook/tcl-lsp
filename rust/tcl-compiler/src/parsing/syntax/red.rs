//! The *red* layer: lazy absolute positions over a green tree.
//!
//! Ports `compiler/parsing/syntax/red.py` (#533 / `SYNC-JUN06`).
//!
//! A [`GreenNode`] knows only widths.  A [`SyntaxTree`] anchors one at
//! an absolute `(offset, line, column)` and resolves positions on
//! demand: a node's absolute start is its parent's start plus the full
//! width of the siblings before it, and line/column come from a line
//! index built from the green tree's *own* reconstructed text — so the
//! red layer needs no reference to the original source and reproduces
//! exactly the positions the lexer emits.
//!
//! The anchoring mirrors the lexer's `base_offset` / `base_line` /
//! `base_col` contract: `base_col` shifts only the first line of the
//! region (everything after the first newline starts at column 0),
//! exactly as the lexer's `SourceMap` does.
//!
//! **Lazy views.**  [`SyntaxNode`] / [`SyntaxToken`] are cheap borrowed
//! views (`&SyntaxTree` + `&Green…` + a resolved region offset), created
//! on walk.  Where Python yields generators, this port returns iterators
//! ([`SyntaxNode::children`]) or eagerly-collected `Vec`s
//! ([`SyntaxNode::tokens`]); the output order is identical.  Python's
//! unused `parent` back-pointer is omitted (no red method reads it).

use tcl_lexer::{SourcePosition, Span, Token, TokenType};

use super::green::{GreenElement, GreenNode, GreenToken};

/// Byte offsets at which each line begins (line 0 at offset 0).
///
/// Identical to the lexer's own index, so a caller that has already
/// built one (the segmenter) can pass it to both the lexer and the red
/// layer to avoid the O(n) rebuild.  Byte offsets, matching the Rust
/// lexer's byte-offset convention.
#[must_use]
pub fn build_line_starts(text: &str) -> Vec<u32> {
    let mut starts = vec![0u32];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            starts.push(u32::try_from(i + 1).unwrap_or(u32::MAX));
        }
    }
    starts
}

/// A green [`GreenNode`] anchored at an absolute position.
///
/// [`SyntaxTree::root`] is the [`SyntaxNode`] for the green `Document`;
/// walking it yields [`SyntaxNode`] / [`SyntaxToken`] views that compute
/// their absolute positions lazily through this anchoring.
#[derive(Debug, Clone)]
pub struct SyntaxTree {
    green: GreenNode,
    base_offset: u32,
    base_line: u32,
    base_col: u32,
    text: String,
    line_starts: Vec<u32>,
}

impl SyntaxTree {
    /// Anchor `green` at the document origin `(0, 0, 0)`, deriving the
    /// region text and line index from the tree's own reconstruction.
    #[must_use]
    pub fn new(green: GreenNode) -> Self {
        Self::anchored(green, 0, 0, 0)
    }

    /// Anchor `green` at an absolute `(base_offset, base_line,
    /// base_col)`, deriving the region text and line index from the
    /// tree's own reconstruction (losslessness guarantees they match the
    /// original source).
    #[must_use]
    pub fn anchored(green: GreenNode, base_offset: u32, base_line: u32, base_col: u32) -> Self {
        let text = green.full_text();
        let line_starts = build_line_starts(&text);
        Self {
            green,
            base_offset,
            base_line,
            base_col,
            text,
            line_starts,
        }
    }

    /// Anchor `green` with a caller-supplied region `text` and
    /// `line_starts`, skipping the O(n) rebuilds.  The caller is
    /// responsible for ensuring `text == green.full_text()` and that
    /// `line_starts` is its line index (the segmenter, which already
    /// holds both, uses this path).
    #[must_use]
    pub fn with_parts(
        green: GreenNode,
        base_offset: u32,
        base_line: u32,
        base_col: u32,
        text: String,
        line_starts: Vec<u32>,
    ) -> Self {
        Self {
            green,
            base_offset,
            base_line,
            base_col,
            text,
            line_starts,
        }
    }

    /// The region's reconstructed source text (lossless).
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Borrow the anchored green root.
    #[must_use]
    pub fn green(&self) -> &GreenNode {
        &self.green
    }

    /// The red view of the green `Document` root.
    #[must_use]
    pub fn root(&self) -> SyntaxNode<'_> {
        SyntaxNode {
            tree: self,
            green: &self.green,
            full_start: 0,
        }
    }

    /// Resolve a region-relative byte offset to an absolute position.
    ///
    /// Mirrors the lexer's `_pos_at`: the line index is region-relative,
    /// then the anchoring shifts line by `base_line` and the *first*
    /// line's column by `base_col`.
    #[must_use]
    pub fn position_at(&self, region_offset: u32) -> SourcePosition {
        // `partition_point(<= off)` is Python's `bisect_right`; `- 1`
        // gives the line whose start is the greatest `<= region_offset`.
        // `line_starts` always begins with 0, so the result is >= 1.
        let line_idx = self.line_starts.partition_point(|&s| s <= region_offset) - 1;
        let col = region_offset - self.line_starts[line_idx];
        let line_idx_u32 = u32::try_from(line_idx).unwrap_or(u32::MAX);
        SourcePosition {
            line: self.base_line + line_idx_u32,
            character: if line_idx == 0 {
                self.base_col + col
            } else {
                col
            },
            offset: self.base_offset + region_offset,
        }
    }
}

/// A red view of a [`GreenToken`] at a resolved region offset.
#[derive(Debug, Clone, Copy)]
pub struct SyntaxToken<'t> {
    tree: &'t SyntaxTree,
    green: &'t GreenToken,
    full_start: u32,
}

impl<'t> SyntaxToken<'t> {
    /// The underlying green token.
    #[must_use]
    pub fn green(&self) -> &'t GreenToken {
        self.green
    }

    /// Lexer kind of the fragment.
    #[must_use]
    pub fn token_type(&self) -> TokenType {
        self.green.token_type
    }

    /// Inner text (`Token.text`).
    #[must_use]
    pub fn text(&self) -> &'t str {
        &self.green.text
    }

    /// Full source slice the fragment occupies.
    #[must_use]
    pub fn raw(&self) -> &'t str {
        &self.green.raw
    }

    /// Whether the fragment was lexed inside a quoted-string context.
    #[must_use]
    pub fn in_quote(&self) -> bool {
        self.green.in_quote
    }

    /// Region offset of the fragment's first byte (past its leading
    /// trivia).
    #[must_use]
    pub fn raw_start(&self) -> u32 {
        self.full_start + self.green.leading_width()
    }

    /// Absolute `Token.start`.
    #[must_use]
    pub fn start_position(&self) -> SourcePosition {
        self.tree.position_at(self.raw_start())
    }

    /// Absolute `Token.end` — the lexer's inner-end convention (#527).
    #[must_use]
    pub fn end_position(&self) -> SourcePosition {
        self.tree.position_at(self.raw_start() + self.green.end_rel)
    }

    /// Reconstruct the lexer [`Token`] this fragment came from.
    ///
    /// The Rust `Token` is span + `content_offset`, so the span is
    /// rebuilt from the absolute raw start and `end_rel`: a token's
    /// exclusive end is its inner-end offset plus one
    /// (`raw_start + end_rel + 1`), matching the lexer's span.  The
    /// `Expand` (`{*}`) marker is the lone exception — the lexer emits it
    /// as a zero-width ghost token (empty span at its start), even though
    /// its `raw` is the three bytes `{*}` — so its span is empty.
    #[must_use]
    pub fn to_token(&self) -> Token {
        let abs_raw_start = self.tree.base_offset + self.raw_start();
        let span = if self.green.token_type == TokenType::Expand {
            Span::empty(abs_raw_start)
        } else {
            Span::new(abs_raw_start, abs_raw_start + self.green.end_rel + 1)
        };
        Token {
            kind: self.green.token_type,
            span,
            content_offset: self.green.content_offset,
            in_quote: self.green.in_quote,
        }
    }
}

/// A red view of an interior [`GreenNode`] at a resolved region offset.
#[derive(Debug, Clone, Copy)]
pub struct SyntaxNode<'t> {
    tree: &'t SyntaxTree,
    green: &'t GreenNode,
    full_start: u32,
}

/// A red tree element: an interior [`SyntaxNode`] or a leaf [`SyntaxToken`].
#[derive(Debug, Clone, Copy)]
pub enum SyntaxElement<'t> {
    /// An interior node view.
    Node(SyntaxNode<'t>),
    /// A leaf token view.
    Token(SyntaxToken<'t>),
}

impl<'t> SyntaxNode<'t> {
    /// The underlying green node.
    #[must_use]
    pub fn green(&self) -> &'t GreenNode {
        self.green
    }

    /// Structural role of this node.
    #[must_use]
    pub fn kind(&self) -> super::green::SyntaxKind {
        self.green.kind
    }

    /// Region offset of this node's first byte *including* leading trivia.
    #[must_use]
    pub fn full_start(&self) -> u32 {
        self.full_start
    }

    /// The `{*}` markers preceding this word's fragments, each anchored.
    #[must_use]
    pub fn expand_markers(&self) -> Vec<SyntaxToken<'t>> {
        let mut out = Vec::with_capacity(self.green.expand_markers.len());
        let mut offset = self.full_start;
        for marker in &self.green.expand_markers {
            out.push(SyntaxToken {
                tree: self.tree,
                green: marker,
                full_start: offset,
            });
            offset += marker.full_width();
        }
        out
    }

    /// Child red views, each anchored at its resolved region offset.
    ///
    /// Lazy: a view is built only as the iterator is advanced.
    pub fn children(&self) -> impl Iterator<Item = SyntaxElement<'t>> + '_ {
        let tree = self.tree;
        let mut offset = self.full_start;
        for marker in &self.green.expand_markers {
            offset += marker.full_width();
        }
        self.green.children.iter().map(move |child| {
            let cur = offset;
            offset += child.full_width();
            match child {
                GreenElement::Token(t) => SyntaxElement::Token(SyntaxToken {
                    tree,
                    green: t,
                    full_start: cur,
                }),
                GreenElement::Node(n) => SyntaxElement::Node(SyntaxNode {
                    tree,
                    green: n,
                    full_start: cur,
                }),
            }
        })
    }

    /// Every [`SyntaxToken`] under this node, depth-first (expand markers
    /// first, then children in order).
    #[must_use]
    pub fn tokens(&self) -> Vec<SyntaxToken<'t>> {
        let mut out = Vec::new();
        self.collect_tokens(&mut out);
        out
    }

    fn collect_tokens(&self, out: &mut Vec<SyntaxToken<'t>>) {
        out.extend(self.expand_markers());
        for child in self.children() {
            match child {
                SyntaxElement::Token(t) => out.push(t),
                SyntaxElement::Node(n) => n.collect_tokens(out),
            }
        }
    }

    /// Absolute start, *excluding* this node's leading trivia.
    #[must_use]
    pub fn start_position(&self) -> SourcePosition {
        let lead = match self.green.expand_markers.first() {
            Some(marker) => marker.leading_width(),
            None => first_leading_width(self.green),
        };
        self.tree.position_at(self.full_start + lead)
    }
}

/// Leading-trivia width of the first fragment under `green` (recursively).
///
/// Mirrors `red.py::_first_leading_width`: descends the first child until
/// it reaches a token, returning that token's leading-trivia width (0 for
/// a child-less node).
fn first_leading_width(green: &GreenNode) -> u32 {
    match green.children.first() {
        Some(GreenElement::Token(t)) => t.leading_width(),
        Some(GreenElement::Node(n)) => first_leading_width(n),
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::super::green::{GreenTrivia, SyntaxKind, TriviaKind};
    use super::*;
    use tcl_lexer::{Lexer, SourceMap};

    fn ws(text: &str) -> GreenTrivia {
        GreenTrivia::new(TriviaKind::Whitespace, text)
    }

    /// Build a single-fragment word from an already-resolved green token.
    fn word(tok: GreenToken) -> GreenElement {
        GreenElement::Node(GreenNode::word(vec![GreenElement::Token(tok)], Vec::new()))
    }

    #[test]
    fn line_starts_byte_offsets() {
        assert_eq!(build_line_starts("abc"), vec![0]);
        assert_eq!(build_line_starts("ab\ncd\n"), vec![0, 3, 6]);
        assert_eq!(build_line_starts("\n\n"), vec![0, 1, 2]);
    }

    #[test]
    fn position_at_resolves_lines_and_base_anchor() {
        // Region text "ab\ncd", anchored at base (offset 10, line 2, col 4).
        let doc = GreenNode::document(
            vec![word(GreenToken::new(
                TokenType::Esc,
                "ab\ncd",
                "ab\ncd",
                4,
                0,
                false,
                Vec::new(),
                Vec::new(),
            ))],
            Vec::new(),
        );
        let tree = SyntaxTree::anchored(doc, 10, 2, 4);
        // Offset 0 — first line, base_col applies.
        assert_eq!(tree.position_at(0), SourcePosition::new(2, 4, 10));
        // Offset 1 — still first line.
        assert_eq!(tree.position_at(1), SourcePosition::new(2, 5, 11));
        // Offset 3 — start of second region line; base_col does NOT apply.
        assert_eq!(tree.position_at(3), SourcePosition::new(3, 0, 13));
        // Offset 4 — second char of line 2.
        assert_eq!(tree.position_at(4), SourcePosition::new(3, 1, 14));
    }

    #[test]
    fn token_raw_start_skips_leading_trivia() {
        // `  abc` — two leading spaces then the bare word.
        let tok = GreenToken::new(
            TokenType::Esc,
            "abc",
            "abc",
            2,
            0,
            false,
            vec![ws("  ")],
            Vec::new(),
        );
        let tree = SyntaxTree::new(GreenNode::document(vec![word(tok)], Vec::new()));
        let root = tree.root();
        let toks = root.tokens();
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].raw_start(), 2);
        assert_eq!(toks[0].start_position(), SourcePosition::new(0, 2, 2));
        assert_eq!(toks[0].end_position(), SourcePosition::new(0, 4, 4));
    }

    /// Cross-check the red layer against the real lexer: build a green
    /// token whose fields mirror the lexer's output for a single-token
    /// source, anchor it at the origin, and assert `to_token` reproduces
    /// the lexer `Token` exactly (span, `content_offset`, `in_quote`) and
    /// the positions match `SourceMap::range_positions`.
    fn assert_red_matches_lexer(
        src: &str,
        token_type: TokenType,
        end_rel: u32,
        content_offset: u8,
    ) {
        let sm = SourceMap::new(src);
        let lex = Lexer::new(src).tokenise_all().unwrap();
        let lex_tok = lex
            .iter()
            .find(|t| {
                !matches!(
                    t.kind,
                    TokenType::Sep | TokenType::Eol | TokenType::Eof | TokenType::Comment
                )
            })
            .copied()
            .expect("a word token");
        let (lex_start, lex_end) = sm.range_positions(lex_tok.span);

        let green = GreenToken::new(
            token_type,
            sm.token_text(lex_tok),
            src,
            end_rel,
            content_offset,
            lex_tok.in_quote,
            Vec::new(),
            Vec::new(),
        );
        let tree = SyntaxTree::new(GreenNode::document(vec![word(green)], Vec::new()));
        let red = tree.root().tokens()[0];

        assert_eq!(red.token_type(), lex_tok.kind, "kind for {src:?}");
        assert_eq!(red.start_position(), lex_start, "start for {src:?}");
        assert_eq!(red.end_position(), lex_end, "end for {src:?}");
        assert_eq!(red.to_token(), lex_tok, "to_token for {src:?}");
        assert_eq!(
            sm.token_text(red.to_token()),
            sm.token_text(lex_tok),
            "token_text for {src:?}"
        );
    }

    #[test]
    fn to_token_reproduces_lexer_braced_word() {
        assert_red_matches_lexer("{abc}", TokenType::Str, 3, 1);
    }

    #[test]
    fn to_token_reproduces_lexer_empty_brace() {
        // The #527 convention: empty `{}` end sits ON the closer (end_rel 1).
        assert_red_matches_lexer("{}", TokenType::Str, 1, 1);
    }

    #[test]
    fn to_token_reproduces_lexer_var_and_braced_var() {
        assert_red_matches_lexer("$x", TokenType::Var, 1, 1);
        assert_red_matches_lexer("${x}", TokenType::Var, 2, 2);
    }

    #[test]
    fn to_token_reproduces_lexer_command_sub() {
        assert_red_matches_lexer("[cmd]", TokenType::Cmd, 3, 1);
    }

    #[test]
    fn to_token_reproduces_lexer_bare_and_quoted_word() {
        assert_red_matches_lexer("abc", TokenType::Esc, 2, 0);
        assert_red_matches_lexer("\"ab\"", TokenType::Esc, 2, 1);
    }

    #[test]
    fn to_token_reproduces_lexer_multiline_braced_body() {
        // `{a\nbc}` — the inner end (the `c`) is on line 1.
        let src = "{a\nbc}";
        let sm = SourceMap::new(src);
        let lex_tok = Lexer::new(src).tokenise_all().unwrap()[0];
        let (lex_start, lex_end) = sm.range_positions(lex_tok.span);
        let green = GreenToken::new(
            TokenType::Str,
            "a\nbc",
            src,
            4,
            1,
            false,
            Vec::new(),
            Vec::new(),
        );
        let tree = SyntaxTree::new(GreenNode::document(vec![word(green)], Vec::new()));
        let red = tree.root().tokens()[0];
        assert_eq!(red.start_position(), lex_start);
        assert_eq!(red.end_position(), lex_end);
        assert_eq!(red.end_position(), SourcePosition::new(1, 1, 4));
        assert_eq!(red.to_token(), lex_tok);
    }

    #[test]
    fn lexer_line_index_agrees_with_red_overlay_on_lone_cr() {
        // #537 (SYNC-JUN08): a lone CR is *not* a line break — post-fix
        // main counts only `\n` (and the LF of a CRLF) in every line
        // index. The red overlay's `build_line_starts` already does, so
        // the lexer's `SourceMap` / `LineIndex` must agree, or the CST
        // and the lexer report different lines for a token after a bare
        // CR (the position-equivalence inconsistency CST fuzzing exposed:
        // a CR-counting lexer index put `b` in `"a\rb"` on line 1 while
        // the red overlay put it on line 0).
        //
        // `build_line_starts` is an independent `\n`-only index, so this
        // is a genuine cross-check, not a self-comparison.
        for src in ["a\rb", "set a\rset b", "x\r\ny\rz", "\r\r", "a\rb\nc"] {
            let sm = SourceMap::new(src);
            let red_starts = build_line_starts(src);
            for off in 0..=u32::try_from(src.len()).expect("len fits u32") {
                let lex = sm.position_at(off);
                let line = red_starts.partition_point(|&s| s <= off) - 1;
                let col = off - red_starts[line];
                assert_eq!(
                    (lex.line, lex.character),
                    (u32::try_from(line).expect("line fits u32"), col),
                    "offset {off} in {src:?}",
                );
            }
        }
        // Pin the absolute post-fix-Python position too: in `"a\rb"`,
        // `b` (offset 2) is line 0, column 2 — not line 1.
        assert_eq!(
            SourceMap::new("a\rb").position_at(2),
            SourcePosition::new(0, 2, 2),
        );
    }

    #[test]
    fn expand_marker_reconstructs_to_empty_span() {
        // `{*}$x` — the EXPAND marker is a zero-width ghost token at the
        // word's start; the VAR `$x` carries the real content.
        let src = "{*}$x";
        let lex = Lexer::new(src).tokenise_all().unwrap();
        let lex_expand = lex.iter().find(|t| t.kind == TokenType::Expand).unwrap();
        let lex_var = lex.iter().find(|t| t.kind == TokenType::Var).unwrap();

        let marker = GreenToken::new(
            TokenType::Expand,
            "",
            "{*}",
            0,
            0,
            false,
            Vec::new(),
            Vec::new(),
        );
        let var = GreenToken::new(
            TokenType::Var,
            "x",
            "$x",
            1,
            1,
            false,
            Vec::new(),
            Vec::new(),
        );
        let word_node = GreenNode::word(vec![GreenElement::Token(var)], vec![marker]);
        let cmd = GreenNode::command(vec![GreenElement::Node(word_node)], None, None);
        let tree = SyntaxTree::new(GreenNode::document(
            vec![GreenElement::Node(cmd)],
            Vec::new(),
        ));

        let toks = tree.root().tokens();
        // Document → Command → Word; tokens() yields the marker then the var.
        assert_eq!(toks.len(), 2);
        assert_eq!(toks[0].token_type(), TokenType::Expand);
        assert!(toks[0].to_token().span.is_empty());
        assert_eq!(toks[0].to_token(), *lex_expand);
        assert_eq!(toks[1].token_type(), TokenType::Var);
        assert_eq!(toks[1].to_token(), *lex_var);
    }

    #[test]
    fn children_and_node_start_position_skip_leading_trivia() {
        // ` puts hi` — leading space on `puts`. A command node's
        // start_position excludes its own leading trivia.
        let puts = GreenToken::new(
            TokenType::Esc,
            "puts",
            "puts",
            3,
            0,
            false,
            vec![ws(" ")],
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
        let tree = SyntaxTree::new(GreenNode::document(
            vec![GreenElement::Node(cmd)],
            Vec::new(),
        ));
        let root = tree.root();
        let cmd_view = match root.children().next().unwrap() {
            SyntaxElement::Node(n) => n,
            SyntaxElement::Token(_) => panic!("expected a command node"),
        };
        assert_eq!(cmd_view.kind(), SyntaxKind::Command);
        // The command's leading space is excluded: start at offset 1.
        assert_eq!(cmd_view.start_position(), SourcePosition::new(0, 1, 1));
        // Two words under the command.
        let words: Vec<_> = cmd_view.children().collect();
        assert_eq!(words.len(), 2);
    }

    #[test]
    fn full_text_round_trips_via_tree() {
        // The tree's text is the green reconstruction; losslessness holds.
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
        let tree = SyntaxTree::new(GreenNode::document(
            vec![GreenElement::Node(cmd)],
            Vec::new(),
        ));
        assert_eq!(tree.text(), "puts hi\n");
    }
}
