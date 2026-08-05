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

//! Build a green CST from the lexer token stream.
//!
//! This re-shapes the existing lexer output into the canonical tree
//! rather than introducing a second parser: it tokenises the region
//! through the dialect-configured [`Lexer`] and groups the stream into
//! commands and words, folding `Sep` / `Eol` / `Comment` tokens into
//! attached [`GreenTrivia`].
//!
//! The grouping mirrors the segmenter's `segment_commands_local`
//! exactly — same word-merging, same `{*}` handling, same continuation
//! handling, and the same `word_boundary` range rule (stale-boundary
//! quirk included) — so that segments derived from the tree
//! are byte-identical to the token-loop segmenter.
//!
//! Raw fragment text is recovered by *start-to-start tiling*: the lexer
//! advances its cursor monotonically, so
//! `source[tok[i].start .. tok[i+1].start]` is exactly the bytes
//! fragment *i* occupies — delimiters included — which sidesteps the
//! inner-end / empty-delimiter convention entirely.
//!
//! **Local-offset space.**  The green tree is position-independent, so it
//! lexes in *local* space (base 0, exactly as `segment_commands_local`)
//! and leaves the anchoring to the red [`super::red::SyntaxTree`].

use tcl_lexer::{LexWarning, Lexer, LexerConfig, SourceMap, Token, TokenType};

use super::green::{GreenElement, GreenNode, GreenToken, GreenTrivia, TokenTrivia, TriviaKind};

/// `Sep` / `Eol` — the two trivia types the `word_boundary` rule treats
/// as a word break.
const fn is_sep_or_eol(t: TokenType) -> bool {
    matches!(t, TokenType::Sep | TokenType::Eol)
}

/// Tokenise `source` with the given dialect [`LexerConfig`] and build its
/// green `Document` node, returning `(document, warnings)`.
///
/// *warnings* is the lexer's non-fatal warning list, passed through for
/// diagnostic emission.  On a hard [`LexError`](tcl_lexer::LexError)
/// (only reachable under `strict_quoting`, which the segmenter does not
/// set) an empty document and no warnings are returned, matching the
/// segmenter's graceful degradation.
///
/// The config's offset fields (`base_offset` / `base_line` / `base_col`)
/// are ignored — building always runs in local-offset space; anchor the
/// result with [`super::red::SyntaxTree::anchored`].
#[must_use]
pub fn build_document(source: &str, config: LexerConfig) -> (GreenNode, Vec<LexWarning>) {
    build_document_with_ghosts(source, config, std::collections::BTreeMap::new())
}

/// As [`build_document`], but injects zero-width ghost closing
/// delimiters (offset → byte) into the lexer for error recovery.
/// An empty map is identical to [`build_document`].
#[must_use]
pub fn build_document_with_ghosts(
    source: &str,
    config: LexerConfig,
    ghosts: std::collections::BTreeMap<u32, u8>,
) -> (GreenNode, Vec<LexWarning>) {
    let config = LexerConfig {
        base_offset: 0,
        base_line: 0,
        base_col: 0,
        ..config
    };
    let sm = SourceMap::new(source);
    let lexer = Lexer::with_source_map(SourceMap::new(source), config).with_ghosts(ghosts);
    let Ok((tokens, warnings)) = lexer.tokenise_all_with_warnings() else {
        return (GreenNode::document(Vec::new(), Vec::new()), Vec::new());
    };
    let document = Builder::new(source, &sm, &tokens).run();
    (document, warnings)
}

/// In-flight state for the start-to-start tiling.
struct Builder<'a> {
    source: &'a str,
    sm: &'a SourceMap<'a>,
    tokens: &'a [Token],

    commands: Vec<GreenElement>,  // finished COMMAND nodes
    cur_words: Vec<GreenElement>, // finished WORD nodes of the current command
    frag: Vec<GreenToken>,        // fragments of the word currently being built
    pending: Vec<GreenTrivia>,    // leading trivia awaiting the next fragment
    markers: Vec<GreenToken>,     // {*} markers awaiting their word
    last_comment: Option<String>, // comment(s) accumulating for the next command
    prev_type: TokenType,

    // Range tracking, mirroring the word_boundary rule.  All offsets are
    // region-relative (local, base 0) so the stored end stays anchor-free.
    first_region: Option<u32>, // region offset of the command's first token
    last_end_region: u32,      // region offset of the last content token's inner end
    word_boundary: Option<u32>, // region offset after the last word fragment
}

impl<'a> Builder<'a> {
    fn new(source: &'a str, sm: &'a SourceMap<'a>, tokens: &'a [Token]) -> Self {
        Self {
            source,
            sm,
            tokens,
            commands: Vec::new(),
            cur_words: Vec::new(),
            frag: Vec::new(),
            pending: Vec::new(),
            markers: Vec::new(),
            last_comment: None,
            prev_type: TokenType::Eol,
            first_region: None,
            last_end_region: 0,
            word_boundary: None,
        }
    }

    /// Raw source slice fragment `i` occupies, via start-to-start tiling.
    fn raw_of(&self, i: usize) -> &'a str {
        let lo = self.tokens[i].span.start() as usize;
        let hi = if i + 1 < self.tokens.len() {
            self.tokens[i + 1].span.start() as usize
        } else {
            self.source.len()
        };
        &self.source[lo..hi]
    }

    /// Region offset of a token's inner end.
    fn end_region(&self, tok: Token) -> u32 {
        self.sm.range_positions(tok.span).1.offset
    }

    fn finish_word(&mut self) {
        if self.frag.is_empty() {
            return;
        }
        let children = std::mem::take(&mut self.frag)
            .into_iter()
            .map(GreenElement::Token)
            .collect();
        let markers = std::mem::take(&mut self.markers);
        self.cur_words
            .push(GreenElement::Node(GreenNode::word(children, markers)));
    }

    fn reset_command(&mut self) {
        self.cur_words = Vec::new();
        self.first_region = None;
        self.last_end_region = 0;
        self.word_boundary = None;
    }

    fn range_end_rel(&self, eol_region: Option<u32>) -> Option<u32> {
        let first_region = self.first_region?;
        let boundary = if eol_region.is_some() && !is_sep_or_eol(self.prev_type) {
            eol_region // the EOL directly follows the last token
        } else {
            self.word_boundary
        };
        let end_region = match boundary {
            Some(b) if b > first_region => b - 1,
            _ => self.last_end_region, // fallback: last token's inner end
        };
        Some(end_region - first_region)
    }

    fn close_command(
        &mut self,
        terminator: GreenTrivia,
        eol_region: Option<u32>,
        pc: Option<String>,
    ) {
        let end_rel = self.range_end_rel(eol_region);
        // Trailing whitespace after the last token + the terminator attach
        // to the last token (document order) as trailing trivia, keeping the
        // tree lossless; the command's range comes from end_rel, not this.
        let mut trail = std::mem::take(&mut self.pending);
        trail.push(terminator);

        let mut extra: Vec<GreenElement> = Vec::new();
        if !self.frag.is_empty() {
            let last = self.frag.pop().unwrap();
            self.frag.push(last.with_trailing(trail));
            self.finish_word();
        } else if !self.markers.is_empty() {
            let last = self.markers.pop().unwrap();
            self.markers.push(last.with_trailing(trail));
            extra = std::mem::take(&mut self.markers)
                .into_iter()
                .map(GreenElement::Token)
                .collect();
        } else if let Some(last) = self.cur_words.pop() {
            // The last word already finished; attach the trail to its last
            // fragment (the last child of the WORD node).
            let GreenElement::Node(word) = last else {
                unreachable!("cur_words holds WORD nodes");
            };
            self.cur_words
                .push(GreenElement::Node(word.with_last_child_trailing(trail)));
        }

        let mut children = std::mem::take(&mut self.cur_words);
        children.extend(extra);
        self.commands.push(GreenElement::Node(GreenNode::command(
            children, end_rel, pc,
        )));
        self.reset_command();
    }

    fn run(mut self) -> GreenNode {
        // Source that precedes the first token is still source. The lexer skips
        // a leading byte-order mark when the file entry asks it to
        // (`LexerConfig::leading_bom`, issue #1218), and the start-to-start
        // tiling below only covers `tokens[0].start ..`, so without this the
        // tree would be three bytes short at the front and *every* offset it
        // derives would slide back by that much. Attaching the prefix as
        // leading trivia keeps the tree lossless, which is the property the
        // offsets rest on.
        if let Some(first) = self.tokens.first()
            && first.span.start() > 0
            && let Some(lead) = self.source.get(..first.span.start() as usize)
            && !lead.is_empty()
        {
            self.pending
                .push(GreenTrivia::new(TriviaKind::Whitespace, lead));
        }
        for i in 0..self.tokens.len() {
            let tok = self.tokens[i];
            if tok.kind == TokenType::Eof {
                break;
            }
            let raw = self.raw_of(i);
            let region = tok.span.start();
            // Trivia and continuation tokens fold into attached
            // GreenTrivia and never become fragments.
            if self.handle_trivia(tok, raw, region) {
                continue;
            }
            self.handle_fragment(tok, raw, region);
        }
        self.finalise();
        GreenNode::document(self.commands, self.pending)
    }

    /// Fold a `Comment` / `Sep` / `Eol` / backslash-newline token into
    /// attached trivia (and close commands on `Eol`).  Returns `true`
    /// when the token was trivia and the caller should advance.
    fn handle_trivia(&mut self, tok: Token, raw: &'a str, region: u32) -> bool {
        match tok.kind {
            TokenType::Comment => {
                let line = raw.trim_start_matches('#').trim();
                self.last_comment = Some(match self.last_comment.take() {
                    Some(prev) => format!("{prev}\n{line}"),
                    None => line.to_string(),
                });
                self.pending
                    .push(GreenTrivia::new(TriviaKind::Comment, raw));
                true
            }
            // A backslash-newline that is a *separator* (between or after
            // words, after a `{*}` marker, or mid-word) is lexed as `Sep` and
            // folds into whitespace trivia like any other separator, advancing
            // the word boundary when it follows a real word.
            //
            // The lexer emits `\<newline>` as an `Esc` token *only* as
            // quoted-word content (`"\<newline>"`, terminated or not), where it
            // is a real fragment of that word — so an `Esc` is NOT folded here
            // and falls through to the fragment path below. Folding it would
            // drop a token the lexer reports and lose the (possibly only)
            // fragment of the quoted word.
            TokenType::Sep => {
                if !is_sep_or_eol(self.prev_type) {
                    self.word_boundary = Some(region);
                }
                self.pending
                    .push(GreenTrivia::new(TriviaKind::Whitespace, raw));
                self.prev_type = TokenType::Sep;
                true
            }
            TokenType::Eol => {
                self.handle_eol(raw, region);
                self.prev_type = TokenType::Eol;
                true
            }
            _ => false,
        }
    }

    /// Close (or skip) a command on an `Eol` terminator.
    fn handle_eol(&mut self, raw: &'a str, region: u32) {
        let eol_triv = GreenTrivia::new(TriviaKind::Eol, raw);
        if !self.frag.is_empty() || !self.cur_words.is_empty() {
            // A real command closes here: it takes the accumulated comment.
            let pc = self.last_comment.take();
            self.close_command(eol_triv, Some(region), pc);
        } else if !self.markers.is_empty() {
            // A dangling-{*} command closes but keeps no comment; a blank
            // line still resets accumulation.
            if raw.matches('\n').count() > 1 {
                self.last_comment = None;
            }
            self.close_command(eol_triv, Some(region), None);
        } else {
            if raw.matches('\n').count() > 1 {
                self.last_comment = None;
            }
            self.pending.push(eol_triv);
        }
    }

    /// Turn a content token (`Esc` / `Str` / `Cmd` / `Var` / `Expand`)
    /// into a fragment, merging it into the current word or starting a new
    /// one.
    fn handle_fragment(&mut self, tok: Token, raw: &'a str, region: u32) {
        let end_region = self.end_region(tok);
        let leaf = GreenToken::new(
            tok.kind,
            self.sm.token_text(tok),
            raw,
            end_region - region,
            tok.content_offset,
            tok.in_quote,
            TokenTrivia::leading(std::mem::take(&mut self.pending)),
        );
        if self.first_region.is_none() {
            self.first_region = Some(region);
        }
        self.last_end_region = end_region;

        if tok.kind == TokenType::Expand {
            // {*} ends any word in progress and marks the *next* word for
            // expansion; like the segmenter it advances prev_type to SEP
            // WITHOUT touching word_boundary (the stale-boundary quirk).
            self.finish_word();
            self.markers.push(leaf);
            self.prev_type = TokenType::Sep;
            return;
        }

        if is_sep_or_eol(self.prev_type) {
            self.finish_word();
            self.frag = vec![leaf];
        } else {
            self.frag.push(leaf);
        }
        self.prev_type = tok.kind;
    }

    /// Close a command left open at end-of-stream (only reachable when the
    /// lexer emits no trailing `Eol` — `puts hi` without a final newline).
    fn finalise(&mut self) {
        if self.frag.is_empty() && self.cur_words.is_empty() && self.markers.is_empty() {
            return;
        }
        let end_rel = self.range_end_rel(None);
        let pc = if !self.frag.is_empty() || !self.cur_words.is_empty() {
            self.last_comment.take()
        } else {
            None
        };
        self.finish_word(); // consumes frag and its leading markers
        let mut children = std::mem::take(&mut self.cur_words);
        children.extend(
            std::mem::take(&mut self.markers)
                .into_iter()
                .map(GreenElement::Token),
        );
        if !children.is_empty() {
            self.commands.push(GreenElement::Node(GreenNode::command(
                children, end_rel, pc,
            )));
            self.reset_command();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::green::SyntaxKind;
    use super::super::red::SyntaxTree;
    use super::*;

    fn build(source: &str) -> GreenNode {
        build_document(source, LexerConfig::default()).0
    }

    /// Losslessness: the document's full text reproduces the source.
    fn assert_lossless(source: &str) {
        let doc = build(source);
        assert_eq!(doc.full_text(), source, "round-trip for {source:?}");
    }

    #[test]
    fn lossless_over_a_range_of_sources() {
        for src in [
            "",
            "puts hi",
            "puts hi\n",
            "puts hi\n\n",
            "set x {a b c}",
            "if {$x} {\n  puts yes\n}",
            "# a comment\nputs hi",
            "puts hi ;# trailing",
            "proc f {} {}",
            "set x {}",
            "foo {*}$args",
            "foo {*}{*}$args",
            "list a \\\n b",
            "a; b; c",
            "set x \"quoted string\"",
            "puts [expr {1 + 2}]",
            "  indented puts hi  \n",
        ] {
            assert_lossless(src);
        }
    }

    #[test]
    fn document_groups_commands_and_words() {
        let doc = build("set x 1\nputs hi\n");
        assert_eq!(doc.kind, SyntaxKind::Document);
        assert_eq!(doc.children.len(), 2);
        for cmd in &doc.children {
            let GreenElement::Node(c) = cmd else {
                panic!("command node");
            };
            assert_eq!(c.kind, SyntaxKind::Command);
        }
        // First command `set x 1` has three words.
        let GreenElement::Node(first) = &doc.children[0] else {
            unreachable!()
        };
        assert_eq!(first.children.len(), 3);
    }

    #[test]
    fn compound_word_merges_fragments() {
        // `{a}b` is one word with two abutting fragments.
        let doc = build("puts {a}b");
        let GreenElement::Node(cmd) = &doc.children[0] else {
            unreachable!()
        };
        // words: `puts`, `{a}b`.
        assert_eq!(cmd.children.len(), 2);
        let GreenElement::Node(word) = &cmd.children[1] else {
            unreachable!()
        };
        assert_eq!(word.kind, SyntaxKind::Word);
        assert_eq!(word.children.len(), 2);
        // The inter-word separator is leading trivia on the first
        // fragment, so the word's full_text carries it; its fragment raws
        // are the merge `{a}` + `b`.
        assert_eq!(word.full_text(), " {a}b");
        let raws: String = word
            .children
            .iter()
            .map(|c| match c {
                GreenElement::Token(t) => t.raw.clone(),
                GreenElement::Node(_) => unreachable!("word holds tokens"),
            })
            .collect();
        assert_eq!(raws, "{a}b");
    }

    #[test]
    fn expand_marker_attaches_to_word() {
        let doc = build("foo {*}$args");
        let GreenElement::Node(cmd) = &doc.children[0] else {
            unreachable!()
        };
        // words: `foo`, then the expanded `$args`.
        assert_eq!(cmd.children.len(), 2);
        let GreenElement::Node(expanded) = &cmd.children[1] else {
            unreachable!()
        };
        assert!(expanded.is_expand());
        assert_eq!(expanded.expand_markers.len(), 1);
        // The inter-word separator is the marker's leading trivia.
        assert_eq!(expanded.full_text(), " {*}$args");
        assert_eq!(expanded.expand_markers[0].raw, "{*}");
        let raws: String = expanded
            .children
            .iter()
            .map(|c| match c {
                GreenElement::Token(t) => t.raw.clone(),
                GreenElement::Node(_) => unreachable!("word holds tokens"),
            })
            .collect();
        assert_eq!(raws, "$args");
    }

    #[test]
    fn preceding_comment_attaches_forward() {
        let doc = build("# doc line\nproc f {} {}\n");
        let GreenElement::Node(cmd) = &doc.children[0] else {
            unreachable!()
        };
        assert_eq!(cmd.preceding_comment.as_deref(), Some("doc line"));
    }

    #[test]
    fn comment_accumulates_across_lines() {
        let doc = build("# first\n# second\nputs hi\n");
        let GreenElement::Node(cmd) = &doc.children[0] else {
            unreachable!()
        };
        assert_eq!(cmd.preceding_comment.as_deref(), Some("first\nsecond"));
    }

    #[test]
    fn blank_line_resets_comment_accumulation() {
        // A blank line between the comment and the command detaches it.
        let src = "# orphan\n\nputs hi\n";
        let doc = build(src);
        // The blank line after the comment reset the forward-attachment, so
        // the command's preceding_comment is None (the comment survives as
        // pending leading trivia for losslessness).
        let cmd = doc
            .children
            .iter()
            .find_map(|c| match c {
                GreenElement::Node(n) if n.kind == SyntaxKind::Command => Some(n),
                _ => None,
            })
            .expect("a command");
        assert_eq!(cmd.preceding_comment, None);
        assert_eq!(doc.full_text(), src);
    }

    #[test]
    fn dangling_trailing_comment_lands_on_document() {
        // `puts hi ;# bye` — `;` closes the command, `# bye` dangles. The
        // dangling comment must round-trip on the document's trailing.
        let src = "puts hi ;# bye";
        let doc = build(src);
        assert_eq!(doc.full_text(), src);
        // One real command; the trailing `# bye` is on the document.
        assert!(!doc.trailing.is_empty());
    }

    #[test]
    fn range_end_rel_covers_braced_body_via_red() {
        // The command range (resolved through the red layer) ends on the
        // closing `}` of a final braced body — matching the segmenter.
        let src = "if {$x} {body}";
        let doc = build(src);
        let GreenElement::Node(cmd) = &doc.children[0] else {
            unreachable!()
        };
        let end_rel = cmd.range_end_rel.expect("a range");
        let tree = SyntaxTree::new(doc.clone());
        // Resolve the command's first-token start + end_rel against the red
        // tree and confirm it lands on the final `}`.
        let cmd_view = match tree.root().children().next().unwrap() {
            super::super::red::SyntaxElement::Node(n) => n,
            super::super::red::SyntaxElement::Token(_) => unreachable!(),
        };
        let first_tok = cmd_view.tokens()[0];
        let end_offset = first_tok.raw_start() + end_rel;
        assert_eq!(src.as_bytes()[end_offset as usize], b'}');
        assert_eq!(end_offset as usize, src.len() - 1);
    }

    #[test]
    fn range_end_rel_does_not_overshoot_trailing_empty_brace() {
        // `set x {}` — the final empty `{}`; the command range ends on its
        // own `}` (offset 7), the non-overshoot the word_boundary rule
        // makes structural.
        let src = "set x {}";
        let doc = build(src);
        let GreenElement::Node(cmd) = &doc.children[0] else {
            unreachable!()
        };
        let end_rel = cmd.range_end_rel.expect("a range");
        let tree = SyntaxTree::new(doc.clone());
        let cmd_view = match tree.root().children().next().unwrap() {
            super::super::red::SyntaxElement::Node(n) => n,
            super::super::red::SyntaxElement::Token(_) => unreachable!(),
        };
        let first_tok = cmd_view.tokens()[0];
        let end_offset = first_tok.raw_start() + end_rel;
        assert_eq!(end_offset as usize, src.len() - 1); // the `}` at offset 7
        assert_eq!(src.as_bytes()[end_offset as usize], b'}');
    }

    #[test]
    fn trailing_command_without_final_eol() {
        // No trailing newline: the open command is closed at end-of-stream.
        let doc = build("puts hi");
        assert_eq!(doc.children.len(), 1);
        assert_eq!(doc.full_text(), "puts hi");
        let GreenElement::Node(cmd) = &doc.children[0] else {
            unreachable!()
        };
        assert_eq!(cmd.children.len(), 2);
    }

    #[test]
    fn semicolon_separates_commands() {
        let doc = build("a; b; c");
        // Three commands separated by `;` (an EOL terminator).
        let cmds: Vec<_> = doc
            .children
            .iter()
            .filter(|c| matches!(c, GreenElement::Node(n) if n.kind == SyntaxKind::Command))
            .collect();
        assert_eq!(cmds.len(), 3);
        assert_eq!(doc.full_text(), "a; b; c");
    }
}
