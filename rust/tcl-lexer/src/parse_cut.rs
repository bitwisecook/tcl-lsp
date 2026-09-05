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

//! The one owner of **where a script stops parsing** — the *cut* (issue
//! #1787).
//!
//! C Tcl parses a script one command at a time and evaluates each before
//! parsing the next, so a malformed command does not erase what preceded
//! it: `puts pre; puts "x${abc"` prints `pre` and *then* raises `missing
//! close-brace for variable name`.  Two Rust consumers need to know where
//! that boundary falls, and both used to work it out privately:
//!
//! * `runtime/rust` walks the words of the command it is about to evaluate
//!   (`parse::first_parse_error`) and defers the failure to the word that
//!   carries it;
//! * `tcl-compiler` filtered the [`Lexer`](crate::Lexer)'s **warning
//!   stream** against a private list of eight message strings and took the
//!   one with the lowest offset, so a VM front-end could turn a malformed
//!   script into a catchable runtime error.
//!
//! The second answer was wrong in two measurable ways, because a warning
//! stream is flat and C's parse is not.  For
//! `list [sfx one] [list "oops]` the lexer warns `missing close-bracket`
//! at the end of the script while C — which parses the bracket's own
//! script during the outer command's parse — reports `missing "`.  For
//! `puts $a([set q "x)` the lexer warns `missing )` at the `(` while C
//! again reports `missing "` from inside the bracket.  And a warning
//! stream cannot see [`WordSpan::welded_after_close`] at all, so
//! `set y {a}b` was accepted as three words where C rejects it.
//!
//! # What the cut is
//!
//! [`first_parse_cut`] walks [`group_commands`] in source order, then each
//! command's words in source order, then each word's components in source
//! order, descending into `[…]` bodies and `$arr(index)` components
//! exactly as C's `ParseTokens` does.  The first construct C rejects wins,
//! and the answer carries the **top-level command index** it was found
//! under — which is what a bytecode front-end needs in order to compile
//! the clean prefix and raise only after it has run.
//!
//! Nothing here is a new scanner.  Each class of failure is delegated to
//! the primitive that already owns its spelling:
//! [`quoted_word_close`](crate::word_parts::quoted_word_close) for
//! `missing "` and the close-quote position,
//! [`word_closer_offset_at`](crate::word_closer_offset_at) for an
//! unterminated brace, [`decompose_spanned`] for everything inside a word,
//! and [`group_commands`] for `{*}` and the welded close-brace.  This
//! module only decides the **order** they are asked in.
//!
//! # Not an evaluator's parse
//!
//! A cut says *where* a script stops being parseable, not what to do about
//! it.  `runtime/rust` keeps its own per-command walk over the borrowed
//! tree it has already built — re-deriving the cut from source there would
//! re-lex every command it evaluates — and `runtime/rust`'s
//! `parse_cut_owner_agrees` test is what keeps the two applications of this
//! policy honest.

use crate::script::{CommandSpan, WordKind, group_commands};
use crate::word_parts::{
    EXTRA_AFTER_CLOSE_BRACE, MISSING_CLOSE_BRACE, SpannedPart, SubstFlags, WordPart,
    decompose_spanned, quoted_word_close,
};
use crate::{Lexer, LexerConfig, SourceMap, Token, word_closer_offset_at, word_span_at};

/// C's message for content that follows a word's closing `"`.
///
/// Spelled here rather than in [`word_parts`](crate::word_parts) because
/// only a *word in a command* can have anything after its closer — the
/// content scan that module owns never sees past one.  The lexer raises the
/// same text under `strict_quoting`; this is the shared constant the lexer,
/// the cut owner and `runtime/rust` all name.
pub const EXTRA_AFTER_CLOSE_QUOTE: &str = "extra characters after close-quote";

/// Where a script stops parsing, in C's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseCut {
    /// Index into [`group_commands`]'s result for the **top-level** script
    /// of the command the failure was found under.
    ///
    /// Commands before this one parse cleanly and, in C, run before the
    /// error is raised.  A failure inside a `[…]` body reports the
    /// top-level command that contains the bracket, not the inner command
    /// index — the inner script is not separately evaluable.
    pub command: usize,
    /// Byte offset in the scanned source of the construct C rejected.
    ///
    /// Exact for a failure in a word or in a `[…]` body.  For one found
    /// inside a `$arr(index)` the offset is the reference's `$`:
    /// [`decompose_spanned`] does not carry extents for index components,
    /// and the `$` is the nearest position that is certainly correct.
    ///
    /// An *unterminated* construct — a brace or quote that never closes —
    /// cuts where the parse ran out of input, so the offset is one past the
    /// last byte scanned and can equal the length of the scanned source.
    /// Slice with it only after bounds-checking.
    pub offset: u32,
    /// C's exact message for the construct.
    pub message: &'static str,
}

/// The first parse cut in `src`, or `None` when the whole script parses.
///
/// Lexes `src` under `config` and delegates to [`first_parse_cut_in`]; a
/// caller that already holds the token stream should use that instead of
/// paying for a second lex.
///
/// ```
/// use tcl_lexer::{LexerConfig, first_parse_cut};
/// let cut = first_parse_cut("puts pre\nputs \"x${abc\"", LexerConfig::default()).unwrap();
/// assert_eq!(cut.command, 1);
/// assert_eq!(cut.message, "missing close-brace for variable name");
/// // Nothing to cut.
/// assert!(first_parse_cut("puts pre\nputs done", LexerConfig::default()).is_none());
/// ```
#[must_use]
pub fn first_parse_cut(src: &str, config: LexerConfig) -> Option<ParseCut> {
    let (tokens, commands) = lex_and_group(src, config)?;
    first_parse_cut_in(&commands, &tokens, src, config)
}

/// [`first_parse_cut`] over a token stream and grouping the caller already
/// has.
///
/// `commands` must come from [`group_commands`] over `tokens`, and
/// `tokens` from a [`Lexer`] run over `src` under `config`.
#[must_use]
pub fn first_parse_cut_in(
    commands: &[CommandSpan],
    tokens: &[Token],
    src: &str,
    config: LexerConfig,
) -> Option<ParseCut> {
    commands.iter().enumerate().find_map(|(index, command)| {
        command_cut(command, tokens, src, config).map(|(offset, message)| ParseCut {
            command: index,
            offset,
            message,
        })
    })
}

/// Lex and group `src`, or `None` when the lexer could not produce a stream
/// at all.
///
/// A hard [`LexError`](crate::LexError) is not a cut: it is the lexer
/// refusing the input outright (a malformed encoding), which no consumer of
/// this module models as a Tcl parse error.
fn lex_and_group(src: &str, config: LexerConfig) -> Option<(Vec<Token>, Vec<CommandSpan>)> {
    let lexer = Lexer::with_source_map(SourceMap::new(src), config);
    let tokens = lexer.tokenise_all().ok()?;
    let commands = group_commands(&tokens, src, config);
    Some((tokens, commands))
}

/// One level of the walk's explicit stack.
///
/// The descent is iterative, and deliberately **unbounded**: C's parser has
/// no nesting limit of its own.  Measured on 8.6.16 and 9.0.4,
/// `puts [list [list … set y {a}b …]]` nested 5000 deep still reports
/// `extra characters after close-brace`, while the *well-formed* script at
/// the same depth gets that far and fails only later, at evaluation, with
/// `too many nested evaluations`.  A parse-depth cap here would answer
/// `None` — "the whole script parses" — for a script C rejects, which is the
/// one answer this owner must never give.  Depth costs heap frames rather
/// than native stack, so the walk is O(input) like the parse it mirrors and
/// needs no limit of its own; the lexer's source-size limit bounds it.
enum Frame<'s> {
    /// A grouped script's words, flattened in document order.
    Words {
        jobs: Vec<WordJob<'s>>,
        next: usize,
        report_at: Option<u32>,
    },
    /// A decomposed word's components.
    Parts {
        parts: Vec<SpannedPart<'s>>,
        base: u32,
        next: usize,
        report_at: Option<u32>,
        /// The word's own delimiter failure, reported if — and only if —
        /// nothing *inside* the word fails first.  This is how an
        /// unterminated quoted word yields C's answer: `puts "[foo"` is
        /// `missing close-bracket`, not `missing "`.
        fallback: Option<(u32, &'static str)>,
    },
    /// A `$arr(index)`'s components, which carry no extents of their own, so
    /// everything found inside one reports at the reference's `$`.
    Index {
        parts: Vec<WordPart<'s>>,
        at: u32,
        next: usize,
    },
}

/// One word of a script, resolved to what the walk must do with it.
///
/// Resolving every word of a body up front keeps a [`Frame`] free of borrows
/// into the token stream it was grouped from, which is what lets a nested
/// body be pushed without the enclosing frame still holding it.
enum WordJob<'s> {
    /// The word fails here, whatever its content holds.
    Cut(u32, &'static str),
    /// Walk this content; if nothing in it fails, report `fallback`.
    Content {
        content: &'s [u8],
        base: u32,
        fallback: Option<(u32, &'static str)>,
    },
    /// A braced word: C does not parse its content as anything.
    Literal,
}

/// The first cut among one command's words, in source order.
fn command_cut(
    command: &CommandSpan,
    tokens: &[Token],
    src: &str,
    config: LexerConfig,
) -> Option<(u32, &'static str)> {
    let jobs = word_jobs(&command.words, tokens, src, 0);
    let mut stack = vec![Frame::Words {
        jobs,
        next: 0,
        report_at: None,
    }];
    drain(&mut stack, config)
}

/// Resolve every word of a grouped script into its [`WordJob`], with offsets
/// already rebased onto the enclosing source.
fn word_jobs<'s>(
    words: &[crate::script::WordSpan],
    tokens: &[Token],
    src: &'s str,
    base: u32,
) -> Vec<WordJob<'s>> {
    words
        .iter()
        .map(|word| word_job(word, tokens, src, base))
        .collect()
}

/// Check one word's own delimiters.
fn word_job<'s>(
    word: &crate::script::WordSpan,
    tokens: &[Token],
    src: &'s str,
    base: u32,
) -> WordJob<'s> {
    let at = |offset: u32| offset.saturating_add(base);
    // A brace group that closed but has content welded to it — `{a}b`,
    // `{a}{b}`, `{a}{*}$b` — is C's first complaint about the word, and the
    // grouping owner is the only thing that can see it.
    if word.welded_after_close {
        return WordJob::Cut(at(weld_offset(word, tokens, src)), EXTRA_AFTER_CLOSE_BRACE);
    }
    let written = written_span(word, tokens, src);
    let (start, end) = (written.start() as usize, written.end() as usize);
    let content =
        |from: usize, to: usize, fallback: Option<(u32, &'static str)>| match src.get(from..to) {
            Some(text) => WordJob::Content {
                content: text.as_bytes(),
                base: at(offset_of(from)),
                fallback,
            },
            None => WordJob::Literal,
        };
    match word.kind {
        // A braced word is C's `TCL_TOKEN_SIMPLE_WORD`: its content is not
        // parsed as anything, so the only thing that can fail is the brace
        // itself failing to close.
        //
        // Two traps in that one test.  [`WordKind::Braced`] means *one `Str`
        // token* — the lexer's literal-word class — not *brace-delimited*: a
        // word that is a lone `$` is also one `Str` token, so the opening
        // byte has to be checked before a closer is demanded.  And the
        // closer test takes the **lexer's** span, not the widened one:
        // widening already consumed the `}`, so asking
        // [`word_closer_offset_at`](crate::word_closer_offset_at) about the
        // widened span asks whether the byte *after* the `}` is a `}`.
        WordKind::Braced => {
            if src.as_bytes().get(start) == Some(&b'{')
                && word_closer_offset_at(src, word.span).is_none()
            {
                WordJob::Cut(at(written.end()), MISSING_CLOSE_BRACE)
            } else {
                WordJob::Literal
            }
        }
        WordKind::Quoted => match quoted_word_close(src, start) {
            // The quote never closed — but that is not necessarily why C
            // stopped.  `quoted_word_close` steps over complete `[…]`
            // substitutions to find the closer, so an *incomplete* one makes
            // it give up here while C, parsing the word's tokens left to
            // right, has already failed inside the bracket:
            // `puts "[foo"` is `missing close-bracket` on 8.6.16 and 9.0.4.
            // Walk the content first and keep `missing "` as the fallback.
            Err(message) => content(start + 1, end, Some((at(written.end()), message))),
            // Anything written between the closing `"` and the end of the
            // word is C's `extra characters after close-quote`.
            Ok(close) if end > close + 1 => {
                WordJob::Cut(at(offset_of(close + 1)), EXTRA_AFTER_CLOSE_QUOTE)
            }
            Ok(close) => content(start + 1, close, None),
        },
        WordKind::Bare => content(start, end, None),
    }
}

/// Run the stack down to empty, or to the first cut.
fn drain(stack: &mut Vec<Frame<'_>>, config: LexerConfig) -> Option<(u32, &'static str)> {
    while let Some(frame) = stack.last_mut() {
        match frame {
            Frame::Words {
                jobs,
                next,
                report_at,
            } => {
                let report_at = *report_at;
                let Some(job) = jobs.get(*next) else {
                    stack.pop();
                    continue;
                };
                *next += 1;
                match job {
                    WordJob::Literal => {}
                    WordJob::Cut(offset, message) => {
                        return Some((report_at.unwrap_or(*offset), message));
                    }
                    WordJob::Content {
                        content,
                        base,
                        fallback,
                    } => {
                        let pushed = Frame::Parts {
                            parts: decompose_spanned(content, SubstFlags::default(), config),
                            base: *base,
                            next: 0,
                            report_at,
                            fallback: fallback
                                .map(|(offset, message)| (report_at.unwrap_or(offset), message)),
                        };
                        stack.push(pushed);
                    }
                }
            }
            Frame::Parts {
                parts,
                base,
                next,
                report_at,
                fallback,
            } => {
                let (base, report_at) = (*base, *report_at);
                let Some(part) = parts.get(*next) else {
                    let fallback = *fallback;
                    stack.pop();
                    if let Some(found) = fallback {
                        return Some(found);
                    }
                    continue;
                };
                *next += 1;
                let at = report_at.unwrap_or(base.saturating_add(offset_of(part.start)));
                match &part.part {
                    WordPart::Text(_) => {}
                    WordPart::ParseError(message) => return Some((at, message)),
                    WordPart::Variable(var) => {
                        if let Some(index) = var.index.clone() {
                            stack.push(Frame::Index {
                                parts: index,
                                at,
                                next: 0,
                            });
                        }
                    }
                    // The `[` is one byte, so the body begins one past the part.
                    WordPart::Command(body) => {
                        let pushed = body_frame(body, at.saturating_add(1), report_at, config);
                        stack.extend(pushed);
                    }
                }
            }
            Frame::Index { parts, at, next } => {
                let at = *at;
                let Some(part) = parts.get(*next) else {
                    stack.pop();
                    continue;
                };
                *next += 1;
                match part {
                    WordPart::Text(_) => {}
                    WordPart::ParseError(message) => return Some((at, message)),
                    WordPart::Variable(var) => {
                        if let Some(index) = var.index.clone() {
                            stack.push(Frame::Index {
                                parts: index,
                                at,
                                next: 0,
                            });
                        }
                    }
                    WordPart::Command(body) => {
                        let pushed = body_frame(body, at, Some(at), config);
                        stack.extend(pushed);
                    }
                }
            }
        }
    }
    None
}

/// A complete `[…]` body as a script frame to walk.
///
/// A body that failed to *close* never reaches here — [`decompose_spanned`]
/// reports that as a [`WordPart::ParseError`] on the enclosing word — so this
/// is the descent C performs while parsing an outer command whose bracket is
/// well-formed but whose inner script is not: `list [set y {a}b]` is
/// `extra characters after close-brace`, found one level down.
fn body_frame(
    body: &[u8],
    base: u32,
    report_at: Option<u32>,
    config: LexerConfig,
) -> Option<Frame<'_>> {
    let body = std::str::from_utf8(body).ok()?;
    let (tokens, commands) = lex_and_group(body, config)?;
    let jobs = commands
        .iter()
        .flat_map(|command| word_jobs(&command.words, &tokens, body, base))
        .collect();
    Some(Frame::Words {
        jobs,
        next: 0,
        report_at,
    })
}

/// The whole written word, closing delimiter included.
///
/// Widening is the **last token's** job, not the word's: the lexer's
/// inner-end convention leaves a braced or bracketed final fragment's `}` /
/// `]` one byte past the token span, and
/// [`word_span_at`](crate::word_span_at) can only widen a span that *opens*
/// with a delimiter.  Asking it about the whole word instead leaves
/// `lappend x pre-[cmd arg]` a byte short and the trailing `]` invisible,
/// which reads as an unterminated bracket.  This is `build.rs`'s
/// `widen_word_end` policy, which the boundary owner documents on
/// [`CommandSpan::span`](crate::CommandSpan::span).
fn written_span(word: &crate::script::WordSpan, tokens: &[Token], src: &str) -> crate::Span {
    let end = word
        .tokens
        .end
        .checked_sub(1)
        .and_then(|last| tokens.get(last))
        .map_or(word.span.end(), |last| word_span_at(src, last.span).end());
    crate::Span::new(word.span.start(), end.max(word.span.end()))
}

/// Byte offset of the content welded to a word's closing brace.
///
/// [`WordSpan::welded_after_close`] marks the word, not the position; the
/// offset is where the *next* fragment of the word starts, which is the
/// byte after the `}` that C stopped at.  Falls back to the word's own end
/// if the word somehow holds a single token (it cannot: welding needs two).
fn weld_offset(word: &crate::script::WordSpan, tokens: &[Token], src: &str) -> u32 {
    let after_first = word
        .tokens
        .clone()
        .find(|&i| {
            tokens
                .get(i)
                .is_some_and(|t| t.span.start() > word.span.start())
        })
        .and_then(|i| tokens.get(i))
        .map(|t| t.span.start());
    after_first.unwrap_or_else(|| word_span_at(src, word.span).end())
}

/// A source offset as the `u32` every span in this crate uses.
///
/// Offsets past `u32::MAX` cannot occur: the lexer already refuses a source
/// that large, so saturating is unreachable rather than lossy.
fn offset_of(at: usize) -> u32 {
    u32::try_from(at).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::{EXTRA_AFTER_CLOSE_QUOTE, ParseCut, first_parse_cut};
    use crate::LexerConfig;

    /// Measured on tclsh 8.4.20, 8.5.19, 8.6.16, 9.0.4 and 9.1b0 — byte
    /// identical on all five — by running each script in a fresh child
    /// interpreter whose `puts` appends to a list, then reporting the error
    /// and what had already run:
    ///
    /// ```text
    /// prefix-var      error missing close-brace for variable name   ran=pre
    /// prefix-quote    error missing "                               ran=pre
    /// prefix-bracket  error missing close-bracket                   ran=pre
    /// prefix-brace    error extra characters after close-brace      ran=pre
    /// prefix-paren    error missing )                               ran=pre
    /// prefix-quotex   error extra characters after close-quote      ran=pre
    /// bracket-inner-q error missing "                               ran=pre
    /// bracket-inner-b error extra characters after close-brace      ran=pre
    /// two-before      error missing close-brace for variable name   ran=one two
    /// welded-expand   error extra characters after close-brace      ran=pre
    /// arr-index-err   error missing "                               ran=pre
    /// semicolon-run   error missing )                               ran=a b
    /// ```
    ///
    /// The `ran=` column is what the command index has to reproduce: it is
    /// exactly the count of commands before the cut.  `bracket-inner-q` and
    /// `arr-index-err` are the two rows the warning-stream scan this owner
    /// replaced got *wrong* — it answered `missing close-bracket` and
    /// `missing )`, because a flat stream cannot see that C parses a
    /// bracket's own script during the enclosing command's parse.
    const CUT_SHEET: &[(&str, &str, usize, &str)] = &[
        (
            "prefix-var",
            "puts pre; puts \"x${abc\"",
            1,
            crate::MISSING_CLOSE_BRACE_FOR_VAR,
        ),
        (
            "prefix-quote",
            "puts pre; puts \"unterminated",
            1,
            crate::word_parts::MISSING_QUOTE,
        ),
        (
            "prefix-bracket",
            "puts pre; puts [foo",
            1,
            crate::word_parts::MISSING_CLOSE_BRACKET,
        ),
        (
            "prefix-brace",
            "puts pre; set y {a}b",
            1,
            crate::word_parts::EXTRA_AFTER_CLOSE_BRACE,
        ),
        (
            "prefix-paren",
            "puts pre; puts $a(",
            1,
            crate::word_parts::MISSING_PAREN,
        ),
        (
            "prefix-quotex",
            "puts pre; puts \"a\"b",
            1,
            EXTRA_AFTER_CLOSE_QUOTE,
        ),
        (
            "bracket-inner-q",
            "puts pre; list [sfx one] [list \"oops]",
            1,
            crate::word_parts::MISSING_QUOTE,
        ),
        (
            "bracket-inner-b",
            "puts pre; list [sfx one] [set y {a}b]",
            1,
            crate::word_parts::EXTRA_AFTER_CLOSE_BRACE,
        ),
        (
            "two-before",
            "puts one; puts two; puts \"x${abc\"",
            2,
            crate::MISSING_CLOSE_BRACE_FOR_VAR,
        ),
        (
            "welded-expand",
            "puts pre; sfx {a}{*}$b",
            1,
            crate::word_parts::EXTRA_AFTER_CLOSE_BRACE,
        ),
        (
            "arr-index-err",
            "puts pre; puts $a([set q \"x)",
            1,
            crate::word_parts::MISSING_QUOTE,
        ),
        (
            "semicolon-run",
            "sfx a; sfx b; puts $c(",
            2,
            crate::word_parts::MISSING_PAREN,
        ),
    ];

    #[test]
    fn cut_sheet_matches_c() {
        for (label, script, command, message) in CUT_SHEET {
            let cut = first_parse_cut(script, LexerConfig::default())
                .unwrap_or_else(|| panic!("{label}: expected a cut in {script:?}"));
            assert_eq!(
                (cut.command, cut.message),
                (*command, *message),
                "{label}: {script:?}"
            );
        }
    }

    /// The offset points at the construct C stopped on, not at the start of
    /// the command or the end of the script.
    #[test]
    fn cut_offset_lands_on_the_rejected_construct() {
        for (script, want) in [
            // The byte after the `}` that closed.
            ("puts pre; set y {a}b", 19),
            // The byte after the `"` that closed.
            ("puts pre; puts \"a\"b", 18),
            // The `{` of the `${` that never closed.
            ("puts pre; puts \"x${abc\"", 17),
            // The `[` that never closed.
            ("puts pre; puts [foo", 15),
            // One level down: the byte after the inner `}`.
            ("puts pre; list [sfx one] [set y {a}b]", 35),
            // Inside an array index, which carries no extents — the `$`.
            ("puts pre; puts $a([set q \"x)", 15),
        ] {
            let cut = first_parse_cut(script, LexerConfig::default()).expect("a cut");
            assert_eq!(cut.offset as usize, want, "{script:?}");
        }
    }

    /// An unterminated construct cuts where the parse ran out of input, so
    /// the offset is allowed to be one past the last byte — but never more.
    #[test]
    fn cut_offset_never_exceeds_the_source() {
        for script in [
            "puts pre; puts \"unterminated",
            "puts pre; set y {unclosed",
            "puts pre; puts [foo",
        ] {
            let cut = first_parse_cut(script, LexerConfig::default()).expect("a cut");
            assert!(
                cut.offset as usize <= script.len(),
                "{script:?}: offset {} past len {}",
                cut.offset,
                script.len()
            );
        }
    }

    /// A braced word is not a script.  C does not parse one while parsing
    /// the command that contains it, so a `catch`/`eval`/`proc` body that is
    /// itself malformed is *not* a cut of the enclosing script — it is a cut
    /// of the body, found when that body is compiled in its own right.
    /// Measured: `puts pre; catch {set y "a"b} e; puts after` runs all three
    /// commands on every shell, catching the parse error inside.
    #[test]
    fn a_braced_body_is_not_descended_into() {
        for script in [
            "puts pre; catch {set y \"a\"b} e; puts after",
            "puts pre; catch {puts $a(} e; puts after",
            "puts pre; eval {set y \"a\"b}",
            "proc p {} {set y \"a\"b}",
        ] {
            assert_eq!(
                first_parse_cut(script, LexerConfig::default()),
                None,
                "{script:?}"
            );
        }
    }

    /// Words that *look* like a delimiter problem and are not.  Every one of
    /// these is accepted by all five shells; the first three are the shapes
    /// that broke while this owner was being written.
    #[test]
    fn valid_scripts_have_no_cut() {
        for script in [
            // A `{` mid-word is ordinary data — C only opens a braced word
            // at word start.
            "set y hi\nputs $={y}",
            "puts a{b}c",
            // A lone `$` is a literal dollar, and lexes as one `Str` token —
            // the same token shape a braced word has.
            "puts [list $ $x]",
            "set z $",
            // A bracketed *final* fragment's `]` sits past the token span.
            "lappend ev pre-[lindex $args end]",
            // Nested, quoted, and escaped delimiters that do close.
            "puts \"a[foo \\\"b\\\"]c\"",
            "puts {$x eq {}}",
            "set x [list \"a b\" {c d}]",
            "puts pre\nputs done",
        ] {
            assert_eq!(
                first_parse_cut(script, LexerConfig::default()),
                None,
                "{script:?}"
            );
        }
    }

    /// The cut is dialect-aware: `{*}` is an ordinary word under 8.4, where
    /// `{*}{a b}` is a welded close-brace rather than an expansion (#1462).
    #[test]
    fn cut_follows_the_configured_dialect() {
        let script = "puts pre; foo {*}{a b}";
        assert_eq!(
            first_parse_cut(script, LexerConfig::for_dialect("tcl8.4")),
            Some(ParseCut {
                command: 1,
                offset: 17,
                message: crate::word_parts::EXTRA_AFTER_CLOSE_BRACE,
            }),
        );
        assert_eq!(
            first_parse_cut(script, LexerConfig::for_dialect("tcl8.6")),
            None,
        );
    }

    /// An unterminated quoted word is not automatically `missing "`.
    ///
    /// `quoted_word_close` steps over *complete* `[…]` substitutions to find
    /// the closer, so an incomplete one makes it give up — but C, parsing the
    /// word's tokens left to right, has already failed inside the bracket.
    /// Measured one script per `tclsh` run on 8.6.16 and 9.0.4, identical:
    ///
    /// ```text
    /// puts "[foo"        -> missing close-bracket
    /// puts "a[foo b"     -> missing close-bracket
    /// list "[foo"        -> missing close-bracket
    /// puts "unterminated -> missing "
    /// ```
    #[test]
    fn an_unterminated_quote_yields_the_error_inside_it_first() {
        for (script, want) in [
            ("puts \"[foo\"", crate::word_parts::MISSING_CLOSE_BRACKET),
            ("puts \"a[foo b\"", crate::word_parts::MISSING_CLOSE_BRACKET),
            ("list \"[foo\"", crate::word_parts::MISSING_CLOSE_BRACKET),
            ("puts \"unterminated", crate::word_parts::MISSING_QUOTE),
            // The quote's own error still wins when the word holds nothing
            // that failed earlier.
            ("puts \"a${b}c", crate::word_parts::MISSING_QUOTE),
        ] {
            assert_eq!(
                first_parse_cut(script, LexerConfig::default()).map(|c| c.message),
                Some(want),
                "{script:?}"
            );
        }
    }

    /// C's parser has no nesting limit, so neither can this one.
    ///
    /// Measured on 8.6.16 and 9.0.4: `puts [list [list … set y {a}b …]]`
    /// nested 1000 and 5000 deep both report `extra characters after
    /// close-brace`, while the *well-formed* script at the same depth parses
    /// and fails only at evaluation (`too many nested evaluations`). A
    /// depth-capped walk answered `None` for the malformed one — "the whole
    /// script parses" — which is the one answer this owner must never give.
    #[test]
    fn nesting_past_any_cap_still_finds_the_cut() {
        for depth in [64usize, 129, 400] {
            let malformed = format!(
                "puts {}set y {{a}}b{}",
                "[list ".repeat(depth),
                "]".repeat(depth)
            );
            assert_eq!(
                first_parse_cut(&malformed, LexerConfig::default()).map(|c| c.message),
                Some(crate::word_parts::EXTRA_AFTER_CLOSE_BRACE),
                "depth {depth}"
            );
            let well_formed = format!("puts {}ok{}", "[list ".repeat(depth), "]".repeat(depth));
            assert_eq!(
                first_parse_cut(&well_formed, LexerConfig::default()),
                None,
                "depth {depth}"
            );
        }
    }

    /// Deep `[…]` nesting terminates instead of overflowing the stack.
    #[test]
    fn deep_bracket_nesting_is_bounded() {
        let deep = format!("puts {}x{}", "[foo ".repeat(400), "]".repeat(400));
        assert_eq!(first_parse_cut(&deep, LexerConfig::default()), None);
        let shallow_error = format!(
            "puts \"a\"b; puts {}x{}",
            "[foo ".repeat(400),
            "]".repeat(400)
        );
        assert_eq!(
            first_parse_cut(&shallow_error, LexerConfig::default()).map(|c| c.message),
            Some(EXTRA_AFTER_CLOSE_QUOTE),
        );
    }
}
