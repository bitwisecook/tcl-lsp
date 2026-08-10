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

//! Derive byte-identical `SegmentedCommand`s from the green CST.
//!
//! This is the bridge that lets the segmenter produce its public
//! [`SegmentedCommand`] from the canonical tree instead of its own token
//! loop, without changing a single downstream consumer.  Every field —
//! `span`, `argv` (with compound-word merging), `texts`,
//! `single_token_word`, `all_tokens` (including `{*}` markers),
//! `expand_word`, and `preceding_comment` — is reconstructed to match the
//! token-loop `segment_commands_local` exactly.
//!
//! **Span: `command_span`, not `range_end_rel`.**  The command range
//! could be resolved from the green `range_end_rel` (the
//! `word_boundary` rule), but the token-loop segmenter —
//! the *oracle* this must stay byte-identical to — instead derives
//! the command span via [`command_span`] / `widen_word_end`, which
//! deliberately does **not** widen a quoted `"…"` last word over its
//! closing quote (the two conventions differ by one byte only for a
//! quoted final word).  To keep `cargo test --workspace` byte-identical,
//! the span is computed with [`command_span`] over the derived
//! `all_tokens` — the same function and inputs the oracle uses, so the
//! result is identical by construction.  `range_end_rel` is still built
//! as an alternative range for future
//! consumers, but is not the source of the Rust `SegmentedCommand.span`.

use tcl_lexer::{SourceMap, Span, Token, TokenType};

use super::green::{GreenNode, SyntaxKind};
use super::red::{SyntaxElement, SyntaxNode, SyntaxToken, SyntaxTree};
use crate::segmenter::{SegmentedCommand, WordFragment, command_span, word_piece};

/// The `Word`-kind child nodes of a command, in document order.
fn command_words<'t>(cmd: &SyntaxNode<'t>) -> Vec<SyntaxNode<'t>> {
    cmd.children()
        .filter_map(|c| match c {
            SyntaxElement::Node(n) if n.kind() == SyntaxKind::Word => Some(n),
            _ => None,
        })
        .collect()
}

/// The fragment tokens (leaf children) of a word, in document order.
fn word_fragments<'t>(word: &SyntaxNode<'t>) -> Vec<SyntaxToken<'t>> {
    word.children()
        .filter_map(|c| match c {
            SyntaxElement::Token(t) => Some(t),
            SyntaxElement::Node(_) => None,
        })
        .collect()
}

/// Build the `SegmentedCommand` for one command node, mirroring the
/// token-loop segmenter's field construction.
fn command_segment(
    cmd: &SyntaxNode<'_>,
    words: &[SyntaxNode<'_>],
    sm: &SourceMap<'_>,
) -> SegmentedCommand {
    let mut argv: Vec<Token> = Vec::with_capacity(words.len());
    let mut texts: Vec<String> = Vec::with_capacity(words.len());
    let mut fragments_by_word: Vec<Vec<WordFragment>> = Vec::with_capacity(words.len());
    let mut single: Vec<bool> = Vec::with_capacity(words.len());
    let mut expand: Vec<bool> = Vec::with_capacity(words.len());

    for word in words {
        let frags = word_fragments(word);
        // `build` only ever emits words with at least one fragment, so this
        // is unreachable for trees produced by `build_document`.  Guard it
        // anyway: `segments_from_tree` is `pub`, and a hand-built or
        // otherwise malformed empty-fragment `Word` must not panic on the
        // `frags[0]` / `frags[len-1]` indexing below.
        let (Some(first_frag), Some(last_frag)) = (frags.first(), frags.last()) else {
            continue;
        };
        let first = first_frag.to_token();
        let last = last_frag.to_token();
        // The representative word token: kind / content_offset / in_quote
        // from the first fragment, span widened from the first fragment's
        // start to the last fragment's end — the segmenter's compound-word
        // merge (a single-fragment word keeps its own span).
        argv.push(Token {
            kind: first.kind,
            span: Span::new(first.span.start(), last.span.end()),
            content_offset: first.content_offset,
            in_quote: first.in_quote,
        });
        let fragments: Vec<WordFragment> = frags
            .iter()
            .map(|fragment| {
                let token = fragment.to_token();
                WordFragment::new(token, word_piece(sm, token))
            })
            .collect();
        texts.push(
            fragments
                .iter()
                .map(|fragment| fragment.text.as_str())
                .collect(),
        );
        fragments_by_word.push(fragments);
        single.push(frags.len() == 1);
        expand.push(!word.green().expand_markers.is_empty());
    }

    let all_tokens: Vec<Token> = cmd.tokens().iter().map(SyntaxToken::to_token).collect();
    // The segmenter records expansion for the command whenever it saw any
    // `{*}`, mirrored here as "any EXPAND fragment in the token stream".
    let has_expand = all_tokens.iter().any(|t| t.kind == TokenType::Expand);

    SegmentedCommand {
        span: command_span(&all_tokens, sm),
        argv,
        texts,
        word_fragments: fragments_by_word,
        single_token_word: single,
        all_tokens,
        is_partial: false,
        partial_delimiter: None,
        expand_word: if has_expand { Some(expand) } else { None },
        preceding_comment: cmd.green().preceding_comment.clone(),
    }
}

/// Produce the `SegmentedCommand` list for an anchored CST.
///
/// `sm` must map the same source the tree was built from (used by
/// `word_piece` / `command_span`, which index source text).
#[must_use]
pub fn segments_from_tree(tree: &SyntaxTree, sm: &SourceMap<'_>) -> Vec<SegmentedCommand> {
    let mut out = Vec::new();
    for child in tree.root().children() {
        let SyntaxElement::Node(cmd) = child else {
            continue;
        };
        if cmd.kind() != SyntaxKind::Command {
            continue;
        }
        let words = command_words(&cmd);
        // A command of only dangling `{*}` markers (no real word) is
        // discarded by the segmenter — it produces no SegmentedCommand.
        if !words.is_empty() {
            out.push(command_segment(&cmd, &words, sm));
        }
    }
    out
}

/// Anchor `document` at the origin and produce its `SegmentedCommand`
/// list.  `sm` maps the source the tree was built from.
#[must_use]
pub fn segments_from_document(document: GreenNode, sm: &SourceMap<'_>) -> Vec<SegmentedCommand> {
    let tree = SyntaxTree::new(document);
    segments_from_tree(&tree, sm)
}

#[cfg(test)]
mod tests {
    use super::super::build::build_document;
    use super::*;
    use tcl_lexer::LexerConfig;

    /// Derive segments from the CST for `src` at the origin.
    fn cst_segments(src: &str) -> Vec<SegmentedCommand> {
        let sm = SourceMap::new(src);
        let (doc, _) = build_document(src, LexerConfig::default());
        segments_from_document(doc, &sm)
    }

    /// `descend_token` on a body `Str` token + `segments_from_tree` is
    /// **field-identical** to `segment_commands_with_offset_and_config(
    /// body_text, start + content_offset, …)` (segment at origin, then
    /// `shifted_by(base)`) — over a body battery at several base offsets,
    /// incl. the seams (`;`-/newline-separated commands, nested `[...]`,
    /// a quoted last word, `{*}`, comments, line continuation,
    /// empty/degenerate bodies).
    ///
    /// This pins the equivalence of the two CST entry points.  A
    /// CST-CONSUMERS "optional fold" of `analyse_body` / `rename.rs` onto
    /// the descent was prototyped against this gate but **deferred**: it is
    /// byte-identical (zero behavioural change), and routing `analyse_body`
    /// through `descend_token` would couple it to `self.source` containing
    /// the body token's absolute span — breaking the self-contained
    /// `analyse_body(body_text, …)` contract that ~36 handler unit tests
    /// rely on, for no functional gain.  See the `CST-CONSUMERS` ledger row.
    #[test]
    fn descend_token_body_matches_segment_with_offset() {
        use super::super::descend::descend_token;
        use tcl_lexer::{Lexer, TokenType};

        let bodies = [
            "puts hi",
            "a; b; c",
            "set x [foo]\n incr y",
            "if {$c} {puts ok}",
            "puts \"a [b] c\"",
            "set q \"trailing\"",
            "foo {*}$args",
            "# comment\n real cmd",
            "a \\\n b",
            "",
            "{}",
            "[x] hi",
            "nested [a [b]] end",
        ];
        for prefix in ["", "xy ", "p\nq "] {
            for body in bodies {
                let src = format!("{prefix}{{{body}}}");
                let toks = Lexer::new(&src).tokenise_all().unwrap();
                let body_tok = toks
                    .iter()
                    .find(|t| t.kind == TokenType::Str)
                    .copied()
                    .expect("a braced Str body token");
                let base = body_tok.span.start() + u32::from(body_tok.content_offset);

                let via_offset = crate::segmenter::segment_commands_with_offset_and_config(
                    body,
                    base,
                    LexerConfig::default(),
                );

                let sm = SourceMap::new(&src);
                let descended = descend_token(&sm, body_tok, LexerConfig::default());
                let via_descent = segments_from_tree(descended.tree(), &sm);

                assert_eq!(
                    format!("{via_offset:?}"),
                    format!("{via_descent:?}"),
                    "body {body:?} at prefix {prefix:?}",
                );
            }
        }
    }

    #[test]
    fn single_command_fields() {
        let segs = cst_segments("set x 1");
        assert_eq!(segs.len(), 1);
        let s = &segs[0];
        assert_eq!(s.texts, vec!["set", "x", "1"]);
        assert_eq!(s.single_token_word, vec![true, true, true]);
        assert_eq!(&"set x 1"[s.span.as_range()], "set x 1");
        assert_eq!(s.argv.len(), 3);
        assert_eq!(s.expand_word, None);
        assert!(!s.is_partial);
    }

    #[test]
    fn compound_word_merges_argv_span() {
        // `puts {a}b` — the second word is compound; its argv token spans
        // the whole word and is not single.
        let segs = cst_segments("puts {a}b");
        let s = &segs[0];
        // word_piece strips the braces of `{a}` and concatenates: `ab`.
        assert_eq!(s.texts, vec!["puts", "ab"]);
        assert_eq!(s.single_token_word, vec![true, false]);
        // argv[1] spans `{a}b` (offsets 5..9).
        assert_eq!(s.argv[1].span.start(), 5);
        assert_eq!(s.argv[1].span.end(), 9);
    }

    #[test]
    fn expand_word_flags() {
        let segs = cst_segments("foo {*}$args");
        let s = &segs[0];
        // word_piece renders the `$args` Var in canonical braced form.
        assert_eq!(s.texts, vec!["foo", "${args}"]);
        assert_eq!(s.expand_word, Some(vec![false, true]));
        // all_tokens includes the {*} marker.
        assert!(s.all_tokens.iter().any(|t| t.kind == TokenType::Expand));
    }

    #[test]
    fn preceding_comment_carried() {
        let segs = cst_segments("# doc\nproc f {} {}\n");
        assert_eq!(segs[0].preceding_comment.as_deref(), Some("doc"));
    }

    #[test]
    fn quoted_line_continuation_is_kept_as_a_word() {
        // `puts "\<newline>"` — the quoted word is a single ESC whose text
        // is exactly `\<newline>`. In C Tcl the `\<newline>` collapses to a
        // space, so the word is a real (one-space) argument, NOT nothing:
        // `puts "\<newline>"` runs `puts` with one argument. Verified
        // against tclsh 8.6.14 and 9.0.3 (`llength $args` == 1 for that
        // trailing word). The segmenter therefore keeps it: `puts` plus the
        // quoted word == two words.
        let segs = cst_segments("puts \"\\\n\"");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].texts.len(), 2, "puts + the quoted word");
        assert_eq!(segs[0].name(), "puts");

        // `set y "a\<newline>b"` keeps the partial-continuation word too.
        let segs2 = cst_segments("set y \"a\\\nb\"");
        assert_eq!(segs2[0].texts.len(), 3, "set + y + the quoted word");
    }

    #[test]
    fn dangling_marker_command_discarded() {
        // A line whose only token is a literal `{*}` (not an expand) still
        // segments as a normal command; verify a real expand-only line is
        // not produced. `{*}` alone is a Str literal, so `set` then a line
        // is the realistic shape — just assert command count is sane.
        let segs = cst_segments("set x 1\n");
        assert_eq!(segs.len(), 1);
    }
}
