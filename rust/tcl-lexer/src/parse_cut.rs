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
/// same text under `strict_quoting`; this is the shared constant both now
/// name.
pub const EXTRA_AFTER_CLOSE_QUOTE: &str = "extra characters after close-quote";

/// How deep a `[…]` chain is followed before the walk gives up and reports
/// no cut for the remainder.
///
/// Mirrors `runtime/rust`'s `MAX_PARSE_ERROR_SCAN_DEPTH` and for the same
/// reason: each level is one `eval_command_subst` when the script actually
/// runs, so a script nested deeper than an interpreter's own eval-depth
/// limit could never have evaluated anyway.  Capping keeps this walk's
/// native recursion bounded by a constant instead of by the input.
const MAX_CUT_SCAN_DEPTH: u32 = 128;

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
        command_cut(command, tokens, src, config, 0).map(|(offset, message)| ParseCut {
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

/// The first cut among one command's words, in source order.
fn command_cut(
    command: &CommandSpan,
    tokens: &[Token],
    src: &str,
    config: LexerConfig,
    depth: u32,
) -> Option<(u32, &'static str)> {
    command
        .words
        .iter()
        .find_map(|word| word_cut(word, tokens, src, config, depth))
}

/// The first cut in one word: its own delimiters first, then its
/// components in source order.
fn word_cut(
    word: &crate::script::WordSpan,
    tokens: &[Token],
    src: &str,
    config: LexerConfig,
    depth: u32,
) -> Option<(u32, &'static str)> {
    // A brace group that closed but has content welded to it — `{a}b`,
    // `{a}{b}`, `{a}{*}$b` — is C's first complaint about the word, and
    // the grouping owner is the only thing that can see it.
    if word.welded_after_close {
        return Some((weld_offset(word, tokens, src), EXTRA_AFTER_CLOSE_BRACE));
    }
    let written = written_span(word, tokens, src);
    let (start, end) = (written.start() as usize, written.end() as usize);
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
        WordKind::Braced => (src.as_bytes().get(start) == Some(&b'{')
            && word_closer_offset_at(src, word.span).is_none())
        .then(|| (written.end(), MISSING_CLOSE_BRACE)),
        WordKind::Quoted => match quoted_word_close(src, start) {
            Err(message) => Some((written.end(), message)),
            // Anything written between the closing `"` and the end of the
            // word is C's `extra characters after close-quote`.
            Ok(close) if end > close + 1 => Some((offset_of(close + 1), EXTRA_AFTER_CLOSE_QUOTE)),
            Ok(close) => content_cut(src, start + 1, close, config, depth),
        },
        WordKind::Bare => content_cut(src, start, end, config, depth),
    }
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

/// The first cut among the substitution components of `src[start..end]`.
fn content_cut(
    src: &str,
    start: usize,
    end: usize,
    config: LexerConfig,
    depth: u32,
) -> Option<(u32, &'static str)> {
    let content = src.get(start..end)?;
    let parts = decompose_spanned(content.as_bytes(), SubstFlags::default(), config);
    parts_cut(&parts, start, config, depth)
}

/// The first cut among `parts`, whose extents are relative to `base`.
fn parts_cut(
    parts: &[SpannedPart<'_>],
    base: usize,
    config: LexerConfig,
    depth: u32,
) -> Option<(u32, &'static str)> {
    parts.iter().find_map(|spanned| {
        let at = offset_of(base + spanned.start);
        match &spanned.part {
            WordPart::Text(_) => None,
            WordPart::ParseError(message) => Some((at, *message)),
            // The `[` is one byte, so the body begins one past the part.
            WordPart::Command(body) => body_cut(body, config, depth + 1)
                .map(|(inner, message)| (at.saturating_add(inner).saturating_add(1), message)),
            // Index components carry no extents of their own, so anything
            // found inside one is reported at the reference's `$`.
            WordPart::Variable(var) => var
                .index
                .as_deref()
                .and_then(|index| index_cut(index, at, config, depth + 1)),
        }
    })
}

/// The first cut inside a complete `[…]` body, relative to the body.
///
/// A body that failed to *close* never reaches here — [`decompose_spanned`]
/// reports that as a [`WordPart::ParseError`] on the enclosing word — so
/// this is the descent C performs while parsing an outer command whose
/// bracket is well-formed but whose inner script is not: `list [set y {a}b]`
/// is `extra characters after close-brace`, found one level down.
fn body_cut(body: &[u8], config: LexerConfig, depth: u32) -> Option<(u32, &'static str)> {
    if depth >= MAX_CUT_SCAN_DEPTH {
        return None;
    }
    let body = std::str::from_utf8(body).ok()?;
    let (tokens, commands) = lex_and_group(body, config)?;
    commands
        .iter()
        .find_map(|command| command_cut(command, &tokens, body, config, depth))
}

/// The first cut among a `$arr(index)` reference's own components, reported
/// at `at` (the reference's `$`).
///
/// [`decompose`](crate::word_parts::decompose) nests index components
/// without extents, so every position inside one collapses to the
/// reference's own — see [`ParseCut::offset`].
fn index_cut(
    parts: &[WordPart<'_>],
    at: u32,
    config: LexerConfig,
    depth: u32,
) -> Option<(u32, &'static str)> {
    if depth >= MAX_CUT_SCAN_DEPTH {
        return None;
    }
    parts.iter().find_map(|part| match part {
        WordPart::Text(_) => None,
        WordPart::ParseError(message) => Some((at, *message)),
        WordPart::Variable(var) => var
            .index
            .as_deref()
            .and_then(|index| index_cut(index, at, config, depth + 1)),
        WordPart::Command(body) => body_cut(body, config, depth + 1).map(|(_, m)| (at, m)),
    })
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

    /// Deep `[…]` nesting terminates instead of overflowing the stack, and
    /// still finds a cut that sits above the cap.
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
