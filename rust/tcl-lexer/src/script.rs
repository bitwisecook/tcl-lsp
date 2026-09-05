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

//! The one owner of Tcl **command and word boundaries** (issue #1786).
//!
//! Two independent groupers used to answer "where does this command end
//! and where does each word begin" over the *same* [`Lexer`](crate::Lexer)
//! stream — `tcl-compiler`'s CST builder
//! (`parsing/syntax/build.rs`) and `runtime/rust`'s
//! `parse_script_with_config`.  They disagreed (measurably) on `{*}`
//! immediately after a braced word, and neither matched C.  This module is
//! the shared substrate they both fold onto.
//!
//! # What this is, and is not
//!
//! It is a **token-stream grouper**, not a byte scanner: it takes the
//! tokens the dialect-configured lexer already produced and answers only
//! the boundary question.  It does not re-scan source, does not decode
//! escapes, does not build values, and does not own trivia, CST shape, or
//! error recovery — those stay with the consumers.
//!
//! It is **borrow-free**: [`WordSpan`] and [`CommandSpan`] hold nothing but
//! [`Span`]s and indices into the caller's token slice, so `runtime/rust`'s
//! zero-copy contract (`memory-management.md` MM-B.6) still holds and
//! `tcl-compiler` can build its owned `SegmentedCommand` on top without
//! this module ever allocating a `String`.  The one place a `String` is
//! unavoidable — the joined preceding comment — is a separate opt-in
//! accessor, [`CommandSpan::comment_text`].
//!
//! # The grouping rules
//!
//! Extracted from `tcl-compiler`'s `parsing/syntax/build.rs` (the nominal
//! owner per the owner-resolution contract, and the behaviour 299 call
//! sites and every LSP diagnostic already depend on):
//!
//! * `Sep` and `Eol` are trivia.  A content token that follows either
//!   starts a new word; any other content token extends the word in
//!   progress.  `Eol` closes the command.
//! * `Comment` is trivia that accumulates into the **following** command:
//!   the leading `#` is stripped, the line is trimmed, consecutive comment
//!   lines join with `\n`, and a blank line — an `Eol` whose text holds
//!   more than one `\n` — resets the accumulator.  See
//!   [`CommandSpan::comment_text`].
//! * `Expand` (`{*}`) finishes the word in progress, marks the **next**
//!   word for expansion, and sets the previous-token state to `Sep`
//!   **without** advancing any word boundary — build.rs calls this the
//!   "stale-boundary quirk".  It is preserved here: the state reset is what
//!   makes `{a}{*}$b` two words (`{a}` and `$b`) rather than one, which is
//!   the segmenter's answer and the one this module adopts.  (`build.rs`'s
//!   `word_boundary` itself has no counterpart here, because
//!   [`CommandSpan::span`] follows the segmenter's shipped `command_span`
//!   convention rather than its unused `range_end_rel` alternative.)
//! * A command consisting only of dangling `{*}` markers — no real word —
//!   is discarded, exactly as the segmenter discards it.
//!
//! # `welded_after_close` and `welded_after_close_quote`
//!
//! New here, and advisory only: [`WordSpan::welded_after_close`] records
//! that a content token followed a braced (`Str`) token with **no
//! intervening separator** — the `{a}b`, `{a}{b}`, `{a}$b`, `{a}{*}$b`
//! shapes that C rejects outright with
//! [`EXTRA_AFTER_CLOSE_BRACE`](crate::EXTRA_AFTER_CLOSE_BRACE).  Nothing in
//! this module acts on it: it is carried so each consumer can pick its own
//! severity.  The analyser stays lenient and diagnoses, since it must keep
//! tokenising broken source; the eval-facing `runtime/rust` raises C's error
//! from it (`parse.rs` `build_word`).
//!
//! [`WordSpan::welded_after_close_quote`] is its sibling for the
//! close-quote weld (`"a"b`, `""b`, `"a$x"b`), C's
//! [`EXTRA_AFTER_CLOSE_QUOTE`](crate::EXTRA_AFTER_CLOSE_QUOTE) (issue
//! #1828).  The two are kept apart because C raises different messages and
//! stops at whichever closer comes first: `{a}"b"c` carries only the brace
//! flag.

use std::ops::Range;

use crate::{LexerConfig, Span, Token, TokenType};

/// How a word was written: bare, `"…"`-quoted, or `{…}`-braced.
///
/// The rule is `runtime/rust`'s (`parse.rs` `build_word`), lifted verbatim
/// because it is the only one of the two consumers that had an explicit
/// word-kind concept at all:
///
/// * **`Braced`** — the word is exactly one `Str` token.  A `Str` welded to
///   more fragments (`{a}b`) is *not* braced: its value is a concatenation,
///   not a literal.
/// * **`Quoted`** — the word's first source byte is `"`.  [`Token::in_quote`]
///   is unreliable for this (the lexer clears it on the *last* token of a
///   quoted word and never sets it on a single-token quoted word), so the
///   opening byte is used instead; for a quoted word the first token's span
///   always starts at the `"`.
/// * **`Bare`** — everything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WordKind {
    /// An unquoted, unbraced word (`$x`, `foo`, `a[b]c`, …).
    Bare,
    /// A `"…"` word.
    Quoted,
    /// A word that is exactly one `{…}` group.
    Braced,
}

/// One word of a command, as spans and token indices only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordSpan {
    /// How the word was written.
    pub kind: WordKind,
    /// Whether a `{*}` marker immediately precedes this word.
    ///
    /// A word may carry more than one marker (`foo {*}{*}$args`); the
    /// markers' token indices are in [`CommandSpan::expand_markers`].
    pub expand: bool,
    /// Half-open index range into the token slice passed to
    /// [`group_commands`] covering this word's fragments.  The range is
    /// always contiguous and never contains trivia: a `Sep`, `Eol`,
    /// `Comment`, or `Expand` token ends the word.
    pub tokens: Range<usize>,
    /// First fragment's span start to last fragment's span end, in the
    /// lexer's *inner-end* convention — a braced or bracketed final
    /// fragment's closing `}` / `]` sits one byte past `span.end()`.  Widen
    /// with [`word_span`](crate::word_span) when the whole written word is
    /// wanted.
    pub span: Span,
    /// A content token followed a braced (`Str`) fragment of this word with
    /// no separator between them — `{a}b`, `{a}{b}`, `{a}$b`, `{a}{*}$b`.
    ///
    /// C rejects every one of those with
    /// [`EXTRA_AFTER_CLOSE_BRACE`](crate::EXTRA_AFTER_CLOSE_BRACE); both Rust
    /// groupers accepted them, differently.  The flag itself is advisory —
    /// this module never acts on it — but `runtime/rust`'s eval-facing parser
    /// does, raising C's error for the word that carries it.
    pub welded_after_close: bool,
    /// A content token followed a **quoted** fragment of this word after its
    /// closing `"`, with no separator between them — `"a"b`, `""b`,
    /// `"a"$b`, `"a"[b]`, `"a"{b}`, `"a$x"b`, `"a[x]"c`.
    ///
    /// C rejects every one of those with
    /// [`EXTRA_AFTER_CLOSE_QUOTE`](crate::EXTRA_AFTER_CLOSE_QUOTE), the
    /// sibling of the close-brace flag above (issue #1828).  Like it, this
    /// is advisory — the module never acts on it, the analyser stays
    /// lenient — and `runtime/rust`'s eval-facing parser raises C's error
    /// from it.  It is dialect-blind, as the whole grouper is: under
    /// `JimTcl`'s concatenating quote rule the flag is still set and the
    /// consumer decides, exactly as
    /// [`first_parse_cut`](crate::first_parse_cut) already reports the shape
    /// regardless of dialect.  Never set on a word that opened with `{` — C
    /// stops at the close-brace first, so `{a}"b"c` carries
    /// [`welded_after_close`](Self::welded_after_close) only.
    pub welded_after_close_quote: bool,
}

impl WordSpan {
    /// Whether the word is a single lexer token (the segmenter's
    /// `single_token_word`).
    #[must_use]
    pub const fn is_single_token(&self) -> bool {
        self.tokens.end - self.tokens.start == 1
    }
}

/// One command: its words, its `{*}` markers, its preceding comment, and
/// the terminator that closed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpan {
    /// The command's words, in source order.  Never empty — a command of
    /// only dangling `{*}` markers is discarded rather than reported.
    pub words: Vec<WordSpan>,
    /// Token indices of every `{*}` marker in the command, in source order.
    ///
    /// A marker whose index is not immediately before some word's
    /// `tokens.start` is *dangling* — it never found a word to expand.
    pub expand_markers: Vec<usize>,
    /// Token indices of the `Comment` tokens accumulated for this command,
    /// in source order.  Render them with [`Self::comment_text`].
    pub comment: Vec<usize>,
    /// Token index of the `Eol` that closed the command, or `None` when the
    /// command was closed by end of stream.
    pub terminator: Option<usize>,
    /// First content token's span start to last content token's span end.
    ///
    /// This is the *token* span. It matches the compiler segmenter's
    /// `SegmentedCommand::span` except for the segmenter's `command_span` /
    /// `widen_word_end` policy of widening a braced or bracketed **final**
    /// token over its closer; apply [`word_span`](crate::word_span) to the
    /// last token to reproduce that.
    pub span: Span,
}

impl CommandSpan {
    /// The command's preceding comment, rendered the way `build.rs` and the
    /// segmenter render it: each comment line's leading `#`s stripped, the
    /// remainder trimmed, consecutive lines joined with `\n`.  `None` when
    /// no comment precedes.
    ///
    /// `tokens` must be the slice this command was grouped from.
    #[must_use]
    pub fn comment_text(&self, tokens: &[Token], src: &str) -> Option<String> {
        if self.comment.is_empty() {
            return None;
        }
        let mut out = String::new();
        for (n, &i) in self.comment.iter().enumerate() {
            if n > 0 {
                out.push('\n');
            }
            let raw = src.get(tokens[i].span.as_range()).unwrap_or_default();
            out.push_str(raw.trim_start_matches('#').trim());
        }
        Some(out)
    }
}

/// Group a lexer token stream into commands and words.
///
/// `tokens` must come from a [`Lexer`](crate::Lexer) run over `src` in the
/// same offset space (the local, `base_offset == 0` space both consumers
/// group in).  `config` is the dialect the stream was lexed under.
///
/// See the [module docs](self) for the rules.
#[must_use]
pub fn group_commands(tokens: &[Token], src: &str, config: LexerConfig) -> Vec<CommandSpan> {
    // Grouping is fully determined by the token stream: every dialect axis
    // that moves a command or word boundary — `expand_syntax` (whether `{*}`
    // is an `Expand` at all), the F5 `brace_line_continuation` ghost `Sep`,
    // `word_separators` — has already been applied by the lexer that
    // produced `tokens`, and the F5 `else`/`elseif` lookahead (N5) is a
    // *command-level* post-pass the compiler owns, not a lexical rule. The
    // config is still threaded so a future dialect-specific grouping rule
    // lands here rather than at the ~300 call sites, and so this entry point
    // matches every sibling in the crate.
    let _ = config;
    Grouper::new(tokens, src).run()
}

/// A word still being accumulated.
struct PendingWord {
    start: usize,
    end: usize,
    expand: bool,
    welded: bool,
    welded_quote: bool,
}

/// In-flight grouping state, mirroring `build.rs`'s `Builder` minus
/// everything that is CST- rather than boundary-shaped.
struct Grouper<'a> {
    tokens: &'a [Token],
    src: &'a str,
    out: Vec<CommandSpan>,

    words: Vec<WordSpan>,
    cur: Option<PendingWord>,
    /// `{*}` markers awaiting their word (`build.rs`'s `markers`).
    markers: Vec<usize>,
    /// Every `{*}` seen in the command so far.
    expand_markers: Vec<usize>,
    comment: Vec<usize>,
    prev_type: TokenType,
    /// Index of the token `prev_type` came from, so the weld test can read its
    /// source bytes rather than trusting its kind alone.
    prev_tok: Option<usize>,

    first_tok: Option<usize>,
    last_tok: usize,
}

impl<'a> Grouper<'a> {
    fn new(tokens: &'a [Token], src: &'a str) -> Self {
        Self {
            tokens,
            src,
            out: Vec::new(),
            words: Vec::new(),
            cur: None,
            markers: Vec::new(),
            expand_markers: Vec::new(),
            comment: Vec::new(),
            // The lexer's own initial state: the first byte of the source is
            // at a word boundary and at command position.
            prev_type: TokenType::Eol,
            prev_tok: None,
            first_tok: None,
            last_tok: 0,
        }
    }

    fn run(mut self) -> Vec<CommandSpan> {
        for i in 0..self.tokens.len() {
            let tok = self.tokens[i];
            match tok.kind {
                // The lexer does not emit `Eof`, but a hand-built stream may;
                // `build.rs` stops on it and closes through `finalise`.
                TokenType::Eof => break,
                // A comment does not advance `prev_type` — it can only occur
                // at command position, so there is no word in progress to
                // break — and accumulates for the *next* command.
                TokenType::Comment => self.comment.push(i),
                TokenType::Sep => {
                    self.prev_type = TokenType::Sep;
                    self.prev_tok = Some(i);
                }
                TokenType::Eol => {
                    self.close(Some(i));
                    self.prev_type = TokenType::Eol;
                    self.prev_tok = Some(i);
                }
                TokenType::Expand => {
                    self.note_weld();
                    self.finish_word();
                    self.markers.push(i);
                    self.expand_markers.push(i);
                    self.note_content(i);
                    // The stale-boundary quirk: `{*}` reads as a separator for
                    // word-start purposes without advancing a word boundary.
                    self.prev_type = TokenType::Sep;
                    self.prev_tok = Some(i);
                }
                // Esc / Str / Cmd / Var / ExprSugar — word content.
                _ => {
                    self.note_content(i);
                    if matches!(self.prev_type, TokenType::Sep | TokenType::Eol) {
                        self.finish_word();
                        self.start_word(i);
                    } else if self.prev_closed_a_brace() {
                        if let Some(word) = self.cur.as_mut() {
                            word.end = i;
                            word.welded = true;
                        }
                    } else if self.prev_closed_a_quote() {
                        // After the brace test, never before it: C stops at
                        // whichever closer comes first, so `{a}"b"c` is a
                        // close-*brace* error and carries only that flag.
                        if let Some(word) = self.cur.as_mut() {
                            word.end = i;
                            word.welded_quote = true;
                        }
                    } else if let Some(word) = self.cur.as_mut() {
                        word.end = i;
                    } else {
                        self.start_word(i);
                    }
                    self.prev_type = tok.kind;
                    self.prev_tok = Some(i);
                }
            }
        }
        self.close(None);
        self.out
    }

    fn start_word(&mut self, i: usize) {
        self.cur = Some(PendingWord {
            start: i,
            end: i,
            expand: !self.markers.is_empty(),
            welded: false,
            welded_quote: false,
        });
        self.markers.clear();
    }

    /// Flag the word in progress when the token about to be consumed sits
    /// directly against a braced fragment's close-brace.
    /// Did the previous token close a **braced** word?
    ///
    /// `TokenType::Str` alone does not mean a brace: the lexer also gives a
    /// nameless `$` that kind, so `$$x` is `Str,Var` with no brace anywhere
    /// and Tcl reports no error for it (`$` is literal, `$x` substitutes).
    /// Keying the weld on the kind alone flagged those, contradicting
    /// [`WordSpan::welded_after_close`]'s contract. Read the opening byte
    /// instead.
    fn prev_closed_a_brace(&self) -> bool {
        self.prev_type == TokenType::Str
            && self
                .prev_tok
                .and_then(|i| self.tokens.get(i))
                .and_then(|t| self.src.as_bytes().get(t.span.start() as usize))
                == Some(&b'{')
    }

    /// Did the previous token close the **quoted** word in progress?
    ///
    /// Only an `Esc` can: inside a quote the lexer dispatches `$` and `[` to
    /// their own parsers and everything else — the closing `"` included — to
    /// `parse_quoted`, so the byte that closes a quote is always consumed by
    /// an `Esc` (the content run that stopped at it, or a zero-content
    /// marker when a `$x` / `[…]` ran straight into it). The word must have
    /// *opened* with `"`: a `"` inside a bare word is literal (`a"b"c`), and
    /// a word that opened with `{` is C's close-*brace* case whatever
    /// follows (`{a}"b"c`).
    ///
    /// Where the closer sits follows the #527 convention, not
    /// `span.end() + 1`: one past the span for a non-empty run (`"a`), but
    /// *inside* the span for the empty-content clamp — `""`, and the bare
    /// closing `"` after a substitution — whose span was extended to cover
    /// the stop byte. `token_text_in` — the body of `SourceMap::token_text`
    /// — is the one owner of "is this token's content empty", so the same
    /// call that distinguishes the two also rejects the opening `"$` / `"[`
    /// clamp: its last byte is the introducer, not a `"`, and `puts "$"`
    /// must stay the text `$`.
    fn prev_closed_a_quote(&self) -> bool {
        if self.prev_type != TokenType::Esc {
            return false;
        }
        let bytes = self.src.as_bytes();
        let opened_with_quote = self
            .cur
            .as_ref()
            .and_then(|word| self.tokens.get(word.start))
            .and_then(|first| bytes.get(first.span.start() as usize))
            == Some(&b'"');
        if !opened_with_quote {
            return false;
        }
        let Some(prev) = self.prev_tok.and_then(|i| self.tokens.get(i)) else {
            return false;
        };
        let end = prev.span.end() as usize;
        let closer = if crate::source_map::token_text_in(self.src, *prev).is_empty() {
            end.checked_sub(1)
        } else {
            Some(end)
        };
        closer.and_then(|at| bytes.get(at)) == Some(&b'"')
    }

    fn note_weld(&mut self) {
        if self.prev_closed_a_brace()
            && let Some(word) = self.cur.as_mut()
        {
            word.welded = true;
        }
    }

    fn note_content(&mut self, i: usize) {
        if self.first_tok.is_none() {
            self.first_tok = Some(i);
        }
        self.last_tok = i;
    }

    fn finish_word(&mut self) {
        let Some(word) = self.cur.take() else { return };
        let first = self.tokens[word.start];
        let last = self.tokens[word.end];
        let kind = if word.start == word.end && first.kind == TokenType::Str {
            WordKind::Braced
        } else if self.src.as_bytes().get(first.span.start() as usize) == Some(&b'"') {
            WordKind::Quoted
        } else {
            WordKind::Bare
        };
        self.words.push(WordSpan {
            kind,
            expand: word.expand,
            tokens: word.start..word.end + 1,
            span: Span::new(first.span.start(), last.span.end()),
            welded_after_close: word.welded,
            welded_after_close_quote: word.welded_quote,
        });
    }

    /// Close the command at `terminator` (an `Eol` token index, or `None`
    /// for end of stream).
    fn close(&mut self, terminator: Option<usize>) {
        let has_content = self.cur.is_some() || !self.words.is_empty();
        if has_content {
            self.finish_word();
            let start = self.tokens[self.first_tok.expect("a word implies a content token")]
                .span
                .start();
            let span = Span::new(start, self.tokens[self.last_tok].span.end());
            self.out.push(CommandSpan {
                words: std::mem::take(&mut self.words),
                expand_markers: std::mem::take(&mut self.expand_markers),
                comment: std::mem::take(&mut self.comment),
                terminator,
                span,
            });
        } else {
            // No real word: a dangling-`{*}` command is discarded, and a
            // blank line resets the comment accumulator.  Both branches keep
            // any accumulated comment for the next real command otherwise.
            if let Some(i) = terminator
                && self
                    .src
                    .get(self.tokens[i].span.as_range())
                    .is_some_and(|raw| raw.bytes().filter(|&b| b == b'\n').count() > 1)
            {
                self.comment.clear();
            }
        }
        self.cur = None;
        self.words.clear();
        self.markers.clear();
        self.expand_markers.clear();
        self.first_tok = None;
        self.last_tok = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Lexer;

    fn group(src: &str) -> (Vec<Token>, Vec<CommandSpan>) {
        group_with(src, LexerConfig::default())
    }

    fn group_with(src: &str, config: LexerConfig) -> (Vec<Token>, Vec<CommandSpan>) {
        let tokens = Lexer::with_config(src, config)
            .tokenise_all()
            .expect("lenient lexing never fails without strict_quoting");
        let cmds = group_commands(&tokens, src, config);
        (tokens, cmds)
    }

    /// The written text of each word (whole word, closer included).
    fn word_texts(src: &str, cmd: &CommandSpan, tokens: &[Token]) -> Vec<String> {
        cmd.words
            .iter()
            .map(|w| {
                let last = tokens[w.tokens.end - 1];
                let end = crate::word_span(&crate::SourceMap::new(src), last)
                    .end()
                    .max(w.span.end());
                src[w.span.start() as usize..end as usize].to_string()
            })
            .collect()
    }

    #[test]
    fn simple_command_words() {
        let (toks, cmds) = group("set x 1");
        assert_eq!(cmds.len(), 1);
        assert_eq!(word_texts("set x 1", &cmds[0], &toks), ["set", "x", "1"]);
        assert!(cmds[0].words.iter().all(WordSpan::is_single_token));
        assert!(cmds[0].words.iter().all(|w| w.kind == WordKind::Bare));
        assert_eq!(cmds[0].span, Span::new(0, 7));
        // Closed by the lexer's trailing ghost EOL.
        assert!(cmds[0].terminator.is_some());
    }

    #[test]
    fn kinds_bare_quoted_braced() {
        let src = "puts \"a $b\" {c d} e";
        let (_toks, cmds) = group(src);
        let kinds: Vec<WordKind> = cmds[0].words.iter().map(|w| w.kind).collect();
        assert_eq!(
            kinds,
            [
                WordKind::Bare,
                WordKind::Quoted,
                WordKind::Braced,
                WordKind::Bare
            ]
        );
    }

    #[test]
    fn a_welded_brace_is_one_bare_word_not_braced() {
        let src = "puts {a}b";
        let (toks, cmds) = group(src);
        assert_eq!(cmds[0].words.len(), 2);
        assert_eq!(word_texts(src, &cmds[0], &toks), ["puts", "{a}b"]);
        assert_eq!(cmds[0].words[1].kind, WordKind::Bare);
        assert!(!cmds[0].words[1].is_single_token());
        assert!(cmds[0].words[1].welded_after_close);
    }

    #[test]
    fn semicolon_and_newline_both_terminate() {
        let src = "a; b\nc";
        let (_toks, cmds) = group(src);
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].span, Span::new(0, 1));
        assert_eq!(cmds[1].span, Span::new(3, 4));
        assert_eq!(cmds[2].span, Span::new(5, 6));
    }

    // ---------------------------------------------------------------
    // `{*}` — the rule this module adopts (the segmenter's), pinned
    // against the measured behaviour of *both* former groupers.
    // ---------------------------------------------------------------

    #[test]
    fn expand_marks_the_following_word() {
        let src = "foo {*}$b";
        let (_toks, cmds) = group(src);
        assert_eq!(cmds[0].words.len(), 2);
        assert_eq!(
            cmds[0].words.iter().map(|w| w.expand).collect::<Vec<_>>(),
            [false, true]
        );
        assert_eq!(cmds[0].expand_markers.len(), 1);
        assert!(!cmds[0].words[1].welded_after_close);
    }

    #[test]
    fn expand_after_a_braced_word_finishes_that_word() {
        // Measured: the compiler segmenter says two words (`{a}` and `$b`);
        // `runtime/rust` said one welded Bare word. This module adopts the
        // segmenter's answer, and flags the weld.
        let src = "{a}{*}$b";
        let (toks, cmds) = group(src);
        assert_eq!(word_texts(src, &cmds[0], &toks), ["{a}", "$b"]);
        assert_eq!(
            cmds[0].words.iter().map(|w| w.expand).collect::<Vec<_>>(),
            [false, true]
        );
        assert_eq!(cmds[0].words[0].kind, WordKind::Braced);
        assert!(cmds[0].words[0].welded_after_close);
        assert!(!cmds[0].words[1].welded_after_close);
    }

    #[test]
    fn expand_after_a_braced_word_then_a_bare_word() {
        let src = "{a}{*}b";
        let (toks, cmds) = group(src);
        assert_eq!(word_texts(src, &cmds[0], &toks), ["{a}", "b"]);
        assert!(cmds[0].words[0].welded_after_close);
    }

    #[test]
    fn expand_after_a_braced_argument() {
        let src = "set x {a}{*}$y";
        let (toks, cmds) = group(src);
        assert_eq!(word_texts(src, &cmds[0], &toks), ["set", "x", "{a}", "$y"]);
    }

    #[test]
    fn expand_after_a_quoted_word_is_literal_text() {
        // A quoted word does not leave the lexer's `last_kind == Str`, so
        // `{*}` there is not an `Expand` at all: it is ordinary content
        // welded onto the quoted word.
        let src = "foo \"q\"{*}$z";
        let (toks, cmds) = group(src);
        assert_eq!(word_texts(src, &cmds[0], &toks), ["foo", "\"q\"{*}$z"]);
        assert!(cmds[0].expand_markers.is_empty());
        assert!(cmds[0].words.iter().all(|w| !w.expand));
        // The close-quote weld is the sibling flag, not this one.
        assert!(!cmds[0].words[1].welded_after_close);
        assert!(cmds[0].words[1].welded_after_close_quote);
    }

    #[test]
    fn double_expand_marks_one_word_and_lists_both_markers() {
        let src = "foo {*}{*}$args";
        let (_toks, cmds) = group(src);
        assert_eq!(cmds[0].words.len(), 2);
        assert_eq!(cmds[0].expand_markers.len(), 2);
        assert!(cmds[0].words[1].expand);
    }

    #[test]
    fn trailing_brace_star_is_a_literal_not_an_expand() {
        // The lexer only makes `{*}` an `Expand` when a non-separator
        // follows, so a trailing `{*}` is an ordinary braced word.
        let src = "foo {*}";
        let (_toks, cmds) = group(src);
        assert_eq!(cmds[0].words.len(), 2);
        assert!(cmds[0].expand_markers.is_empty());
        assert_eq!(cmds[0].words[1].kind, WordKind::Braced);
    }

    #[test]
    fn a_command_of_only_dangling_markers_is_discarded() {
        // The lexer never *emits* a dangling `Expand` — it only makes `{*}` an
        // `Expand` when a non-separator follows, and `;` is a separator byte,
        // so `{*};` lexes as a braced `Str`. `build.rs` and the segmenter both
        // guard the case anyway, so the guard is pinned over a hand-built
        // stream: `{*}` then EOL then a real command.
        let src = "{*}\nfoo\n";
        let tokens = vec![
            Token::new(TokenType::Expand, Span::empty(0)),
            Token::new(TokenType::Eol, Span::new(3, 4)),
            Token::new(TokenType::Esc, Span::new(4, 7)),
            Token::new(TokenType::Eol, Span::new(7, 8)),
        ];
        let cmds = group_commands(&tokens, src, LexerConfig::default());
        assert_eq!(cmds.len(), 1, "the marker-only command is discarded");
        assert_eq!(cmds[0].span, Span::new(4, 7));
        assert!(cmds[0].expand_markers.is_empty());
    }

    #[test]
    fn a_dangling_marker_command_does_not_steal_the_pending_comment() {
        // `build.rs` closes a marker-only command with *no* comment, leaving
        // the accumulator for the next real command.
        let src = "# doc\n{*}\nfoo\n";
        let tokens = vec![
            Token::new(TokenType::Comment, Span::new(0, 5)),
            Token::new(TokenType::Eol, Span::new(5, 6)),
            Token::new(TokenType::Expand, Span::empty(6)),
            Token::new(TokenType::Eol, Span::new(9, 10)),
            Token::new(TokenType::Esc, Span::new(10, 13)),
            Token::new(TokenType::Eol, Span::new(13, 14)),
        ];
        let cmds = group_commands(&tokens, src, LexerConfig::default());
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].comment_text(&tokens, src).as_deref(), Some("doc"));
    }

    #[test]
    fn expand_is_off_under_tcl84() {
        let src = "foo {*}$b";
        let (_toks, cmds) = group_with(src, LexerConfig::for_dialect("tcl8.4"));
        assert_eq!(cmds[0].words.len(), 2);
        assert!(cmds[0].expand_markers.is_empty());
        assert!(!cmds[0].words[1].expand);
        // `{*}$b` is one word: a braced `{*}` welded to `$b`.
        assert!(cmds[0].words[1].welded_after_close);
    }

    // ---------------------------------------------------------------
    // welded_after_close
    // ---------------------------------------------------------------

    #[test]
    fn weld_shapes() {
        for (src, welded) in [
            ("puts {a}b", true),
            ("puts {a}{b}", true),
            ("puts {a}$b", true),
            ("puts {a}[b]", true),
            ("puts {a}{*}$b", true),
            ("puts {a}", false),
            ("puts {a} b", false),
            ("puts a{b}", false),   // `{` mid-word is literal, not a Str token
            ("puts \"a\"b", false), // close-quote weld: the sibling flag
            // A nameless `$` is also a `Str` token, with no brace anywhere:
            // `$$x` is `Str,Var` and Tcl reports no error for it (the first
            // `$` is literal, `$x` substitutes). Keying the weld on the token
            // kind alone flagged these.
            ("puts $$x", false),
            ("puts $$$x", false),
            ("puts ${a}$b", false),
        ] {
            let (_toks, cmds) = group(src);
            let last = cmds[0].words[1].welded_after_close;
            assert_eq!(last, welded, "{src:?}");
        }
    }

    #[test]
    fn weld_is_per_word_not_per_command() {
        let src = "puts {a}b {c} d";
        let (_toks, cmds) = group(src);
        let flags: Vec<bool> = cmds[0].words.iter().map(|w| w.welded_after_close).collect();
        assert_eq!(flags, [false, true, false, false]);
    }

    // ---------------------------------------------------------------
    // welded_after_close_quote
    // ---------------------------------------------------------------

    /// Measured on tclsh 8.4.20, 8.5.19, 8.6.16, 9.0.4 and 9.1b0 (#1828):
    /// every `true` row is `extra characters after close-quote`; every
    /// `false` row is accepted.
    #[test]
    fn quote_weld_shapes() {
        for (src, welded) in [
            ("puts \"a\"b", true),
            ("puts \"\"b", true), // empty `""`: the #527 clamp, closer inside the span
            ("puts \"a\"$b", true),
            ("puts \"a\"[b]", true),
            ("puts \"a\"{b}", true),
            ("puts \"a\"\"b\"", true),
            ("puts \"a\\\"b\"c", true), // escaped quote inside the run
            ("puts \"a$x\"b", true),    // bare closing marker after a `$x`
            ("puts \"$x\"b", true),
            ("puts \"a[x]\"c", true), // bare closing marker after a `[…]`
            ("puts \"a$\"b", true),
            ("puts \"$\"b", true),
            ("puts \"a\"$", true),
            ("puts \"a\"\\ b", true),
            ("puts \"a\"}", true),
            ("puts \"a\"]", true),
            ("puts \"q\"{*}$z", true),
            ("puts \"a\"", false),
            ("puts \"a\" b", false),
            ("puts \"\"", false),
            ("puts \"$\"", false), // `$` with no name: text, not a weld
            ("puts \"a$ b\"", false),
            ("puts \"$x\"", false),
            ("puts \"a[x]\"", false),
            ("puts a\"b\"c", false), // `"` inside a bare word is literal
            ("puts $a\"b\"c", false),
            ("puts \"a\"\\\nb", false), // backslash-newline is a separator
        ] {
            let (_toks, cmds) = group(src);
            let word = &cmds[0].words[1];
            assert_eq!(word.welded_after_close_quote, welded, "{src:?}");
            assert!(
                !word.welded_after_close,
                "{src:?}: brace flag stays brace-only"
            );
        }
    }

    /// C stops at the close-brace, so a quote welded onto a braced word is
    /// still the *brace* error and only that flag carries it.
    #[test]
    fn a_brace_weld_followed_by_a_quote_carries_only_the_brace_flag() {
        for src in ["puts {a}\"b\"", "puts {a}\"b\"c", "puts {a}\"b"] {
            let (_toks, cmds) = group(src);
            let word = &cmds[0].words[1];
            assert!(word.welded_after_close, "{src:?}");
            assert!(!word.welded_after_close_quote, "{src:?}");
        }
    }

    #[test]
    fn quote_weld_is_per_word_not_per_command() {
        let src = "puts \"a\"b \"c\" d";
        let (_toks, cmds) = group(src);
        let flags: Vec<bool> = cmds[0]
            .words
            .iter()
            .map(|w| w.welded_after_close_quote)
            .collect();
        assert_eq!(flags, [false, true, false, false]);
    }

    #[test]
    fn f5_ghost_separator_after_a_close_quote_is_not_a_weld() {
        // F5 splits `"a"b` into two words with no diagnostic
        // (bigip-irule-parser-measurements.md, "Word-formation evidence");
        // the ghost `Sep` the lexer emits starts a new word here, so nothing
        // is welded.
        let src = "set y \"a\"b";
        let (_toks, cmds) = group_with(src, LexerConfig::for_dialect("f5-irules"));
        assert_eq!(cmds[0].words.len(), 4);
        assert!(cmds[0].words.iter().all(|w| !w.welded_after_close_quote));
    }

    // ---------------------------------------------------------------
    // comments
    // ---------------------------------------------------------------

    #[test]
    fn comment_attaches_to_the_following_command() {
        let src = "# doc\nproc f {} {}\n";
        let (toks, cmds) = group(src);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].comment_text(&toks, src).as_deref(), Some("doc"));
    }

    #[test]
    fn consecutive_comment_lines_join_with_newline() {
        let src = "# c1\n#  c2\nproc f {} {}\n";
        let (toks, cmds) = group(src);
        assert_eq!(cmds[0].comment_text(&toks, src).as_deref(), Some("c1\nc2"));
    }

    #[test]
    fn a_blank_line_resets_the_comment_accumulator() {
        let src = "# orphan\n\nputs hi\n";
        let (toks, cmds) = group(src);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].comment_text(&toks, src), None);
    }

    #[test]
    fn a_command_between_comments_takes_only_what_precedes_it() {
        let src = "# one\na\n# two\nb\n";
        let (toks, cmds) = group(src);
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].comment_text(&toks, src).as_deref(), Some("one"));
        assert_eq!(cmds[1].comment_text(&toks, src).as_deref(), Some("two"));
    }

    #[test]
    fn a_dangling_trailing_comment_carries_forward() {
        // `puts hi ;# dangling` — the `;` closes `puts hi` before the comment
        // is seen, so the comment belongs to whatever comes next.
        let src = "puts hi ;# dangling\nnext\n";
        let (toks, cmds) = group(src);
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].comment_text(&toks, src), None);
        assert_eq!(
            cmds[1].comment_text(&toks, src).as_deref(),
            Some("dangling")
        );
    }

    // ---------------------------------------------------------------
    // shape / degenerate input
    // ---------------------------------------------------------------

    #[test]
    fn empty_and_whitespace_only_sources_produce_nothing() {
        for src in ["", "   ", "\n\n\n", "# only a comment\n"] {
            let (_toks, cmds) = group(src);
            assert!(cmds.is_empty(), "{src:?}");
        }
    }

    #[test]
    fn word_token_ranges_are_contiguous_and_cover_every_content_token() {
        let src = "set a $b[c]d {e} \"f$g\" ;# t\n{*}$h\n";
        let (toks, cmds) = group(src);
        for cmd in &cmds {
            let mut expected = cmd.words[0].tokens.start;
            for word in &cmd.words {
                assert!(word.tokens.start >= expected);
                assert!(word.tokens.end > word.tokens.start);
                expected = word.tokens.end;
                for i in word.tokens.clone() {
                    assert!(
                        !matches!(
                            toks[i].kind,
                            TokenType::Sep
                                | TokenType::Eol
                                | TokenType::Comment
                                | TokenType::Expand
                        ),
                        "trivia inside a word at token {i}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_command_without_a_terminator_still_closes() {
        // A hand-built stream ending in content (no ghost EOL) exercises the
        // end-of-stream close.
        let src = "puts hi";
        let tokens: Vec<Token> = Lexer::new(src)
            .tokenise_all()
            .unwrap()
            .into_iter()
            .filter(|t| t.kind != TokenType::Eol)
            .collect();
        let cmds = group_commands(&tokens, src, LexerConfig::default());
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].terminator, None);
        assert_eq!(cmds[0].words.len(), 2);
    }

    #[test]
    fn an_eof_token_stops_the_scan() {
        let src = "a b";
        let mut tokens = Lexer::new(src).tokenise_all().unwrap();
        tokens.insert(0, Token::new(TokenType::Eof, Span::empty(0)));
        let cmds = group_commands(&tokens, src, LexerConfig::default());
        assert!(cmds.is_empty());
    }
}
