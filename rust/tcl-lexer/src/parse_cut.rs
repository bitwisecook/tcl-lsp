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

//! The one owner of **where a script stops parsing** — the *cut*.
//!
//! C Tcl parses a script one command at a time and evaluates each before
//! parsing the next, so a malformed command does not erase what preceded
//! it: `puts pre; puts "x${abc"` prints `pre` and *then* raises `missing
//! close-brace for variable name`.  Two Rust consumers need to know where
//! that boundary falls:
//!
//! * `runtime/rust` walks the words of the command it is about to evaluate
//!   (`parse::first_parse_error`) and defers the failure to the word that
//!   carries it;
//! * `tcl-compiler` needs the cut to turn a malformed script into a
//!   catchable runtime error for a VM front-end, and gets it from this
//!   module rather than filtering the [`Lexer`]'s **warning stream**
//!   against a list of message strings and taking the one with the lowest
//!   offset — a warning stream is flat and C's parse is not, so
//!   that approach is wrong in two measurable ways.  For
//!   `list [sfx one] [list "oops]` it would report `missing close-bracket`
//!   at the end of the script, where C — which parses the bracket's own
//!   script during the outer command's parse — reports `missing "`.  For
//!   `puts $a([set q "x)` it would report `missing )` at the `(`, where C
//!   again reports `missing "` from inside the bracket.  And a warning
//!   stream cannot see
//!   [`WordSpan::welded_after_close`](crate::WordSpan::welded_after_close)
//!   at all, so `set y {a}b` would be accepted as three words where C
//!   rejects it.
//!
//! # What the cut is
//!
//! [`first_parse_cut`] walks [`crate::group_commands`] in source order, then each
//! command's words in source order, then each word's components in source
//! order, descending into `[…]` bodies and `$arr(index)` components
//! exactly as C's `ParseTokens` does.  The first construct C rejects wins,
//! and the answer carries the **top-level command index** it was found
//! under — which is what a bytecode front-end needs in order to compile
//! the clean prefix and raise only after it has run.
//!
//! Nothing here is a new scanner.  Each class of failure is delegated to
//! the primitive that already owns its spelling:
//! [`quoted_word_close`](crate::word_parts::quoted_word_close) for `missing "`
//! and the close-quote position,
//! [`word_closer_offset_at`] for an unterminated brace,
//! [`ExecutablePartArena`] for everything inside a word, and
//! [`crate::group_commands`] for `{*}` and the welded close-brace.  This
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

use std::rc::Rc;

use crate::script::{CommandSpan, WordKind, group_commands_bytes};
use crate::word_parts::{
    ExecutablePart, ExecutablePartArena, PartListId, SubstFlags, quoted_word_close_with_issue_in,
};
use crate::{
    Lexer, LexerConfig, SourceChannel, SourceImage, SourceMap, Span, Token, word_closer_offset_at,
    word_span_at,
};

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
    /// Index into [`crate::group_commands`]'s result for the **top-level** script
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
    /// The reporting contract anchors an index failure at its enclosing `$`;
    /// the arena separately retains the exact inner source extent for `term`.
    ///
    /// An *unterminated* construct — a brace or quote that never closes —
    /// cuts where the parse ran out of input, so the offset is one past the
    /// last byte scanned and can equal the length of the scanned source.
    /// Slice with it only after bounds-checking.
    pub offset: u32,
    /// Byte offset of C's `parsePtr->term` for this cut — the position the
    /// `while executing` frame's quoted command text runs *through*,
    /// inclusively.
    ///
    /// `TclCompileScript` logs a parse failure with
    /// `Tcl_LogCommandInfo(interp, script, commandStart,
    /// term + 1 - commandStart)`, so this is what a runtime needs to name the
    /// offending command (#2172).
    ///
    /// It is **not** [`Self::offset`]. For an unterminated construct C's term
    /// is the character that *opened* it, where `offset` is where the parse
    /// ran out of input; and where an inner failure is reported at an outer
    /// construct's position, `offset` takes that outer position while this
    /// stays with the construct that actually failed. The two coincide for a
    /// failure C reports in place, such as `extra characters after
    /// close-brace`.
    pub term: u32,
    /// Structured issue retained from its original creation site.
    pub kind: crate::ParseIssueKind,
    /// C's exact message for the construct, retained as presentation only.
    pub message: &'static str,
}

/// Whether an authentic syntax cut can be resolved by appending input.
/// This is a parse projection only; neither case licenses evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseCutCompleteness {
    /// A closer is absent at the reached original input boundary.
    Incomplete,
    /// A terminal syntax error remains complete for the C script query.
    Terminal,
}

impl ParseCut {
    /// Project the retained issue kind into the C parser's incomplete flag.
    /// Diagnostic presentation does not participate in this query.
    #[must_use]
    pub const fn completeness(self) -> ParseCutCompleteness {
        if self.kind.is_incomplete() {
            ParseCutCompleteness::Incomplete
        } else {
            ParseCutCompleteness::Terminal
        }
    }
}

/// A lexical stream or source geometry the cut owner cannot inspect.
/// This is distinct from an authentic syntax cut and from a clean script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseCutUnavailable {
    /// The selected lexer declined to publish a complete original stream.
    LexicalStream(crate::LexError),
    /// Original component geometry could not be retained.
    SourceGeometry(crate::word_parts::ExecutablePartsUnavailable),
}

impl std::fmt::Display for ParseCutUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LexicalStream(error) => {
                write!(formatter, "parse-cut lexical stream unavailable: {error}")
            }
            Self::SourceGeometry(error) => write!(
                formatter,
                "parse-cut source geometry unavailable: {error:?}"
            ),
        }
    }
}

impl std::error::Error for ParseCutUnavailable {}

/// Advisory parse-cut projection for Unicode source.
/// `None` also covers unavailable lexical ownership; executable callers use
/// [`first_parse_cut_image_checked`] to distinguish it from a clean script.
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
    first_parse_cut_image_checked(&SourceImage::document(src), config)
        .ok()
        .flatten()
}

/// Advisory parse-cut projection for original native byte source.
/// Opaque bytes are ordinary data. Unavailability is omitted by this wrapper;
/// executable callers use [`first_parse_cut_image_checked`].
#[must_use]
pub fn first_parse_cut_bytes(src: &[u8], config: LexerConfig) -> Option<ParseCut> {
    first_parse_cut_image_checked(&SourceImage::native(src), config)
        .ok()
        .flatten()
}

/// [`first_parse_cut`] over a token stream and grouping the caller already
/// has.
///
/// `commands` must come from [`crate::group_commands`] over `tokens`, and
/// `tokens` from a [`Lexer`] run over `src` under `config`.
#[must_use]
pub fn first_parse_cut_in(
    commands: &[CommandSpan],
    tokens: &[Token],
    src: &str,
    config: LexerConfig,
) -> Option<ParseCut> {
    first_parse_cut_channel_in(
        commands,
        tokens,
        src.as_bytes(),
        config,
        SourceChannel::Document,
    )
    .ok()
    .flatten()
}

/// Inspect an existing native byte token stream in its original coordinates.
#[must_use]
pub fn first_parse_cut_bytes_in(
    commands: &[CommandSpan],
    tokens: &[Token],
    src: &[u8],
    config: LexerConfig,
) -> Option<ParseCut> {
    first_parse_cut_channel_in(commands, tokens, src, config, SourceChannel::NativeValue)
        .ok()
        .flatten()
}

/// Advisory parse-cut projection retaining the image's original channel.
/// Executable callers use [`first_parse_cut_image_checked`] for unavailability.
#[must_use]
pub fn first_parse_cut_image(source: &SourceImage, config: LexerConfig) -> Option<ParseCut> {
    first_parse_cut_image_checked(source, config).ok().flatten()
}

/// Syntax depth selected by a genuine parser entry, independent of execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseCutDescent {
    /// Validate substituted command scripts through the existing full-depth walk.
    RecursiveCommands,
    /// Validate this script's original words and component delimiters, delaying
    /// command-substitution body syntax until that child parser is entered.
    EnclosingScript,
}

/// Inspect an original image without hiding unavailable lexical ownership.
///
/// # Errors
/// Returns the selected lexer or original component geometry refusal.
pub fn first_parse_cut_image_checked(
    source: &SourceImage,
    config: LexerConfig,
) -> Result<Option<ParseCut>, ParseCutUnavailable> {
    first_parse_cut_image_with_descent_checked(source, config, ParseCutDescent::RecursiveCommands)
}

/// Inspect an original script under its producer-selected syntax descent.
/// The same lexical/group/component owners retain source and unavailable cuts.
/// Delaying command bodies changes no delimiter scanner, variable-index
/// traversal or parse-error term and supplies no execution/entry authority.
///
/// # Errors
/// Returns unavailable selected lexical or original component ownership.
pub fn first_parse_cut_image_with_descent_checked(
    source: &SourceImage,
    config: LexerConfig,
    descent: ParseCutDescent,
) -> Result<Option<ParseCut>, ParseCutUnavailable> {
    let (tokens, commands) = lex_and_group(source.bytes(), config, source.channel())?;
    first_parse_cut_channel_in_with_descent(
        &commands,
        &tokens,
        source.bytes(),
        config,
        source.channel(),
        descent,
    )
}

/// Inspect retained tokens and grouping under the same original image/config.
/// `commands` and `tokens` must originate from that image's selected lexer.
///
/// # Errors
/// Returns unavailable original component geometry or a nested lexical stream.
pub fn first_parse_cut_image_in_checked(
    commands: &[CommandSpan],
    tokens: &[Token],
    source: &SourceImage,
    config: LexerConfig,
) -> Result<Option<ParseCut>, ParseCutUnavailable> {
    first_parse_cut_channel_in(commands, tokens, source.bytes(), config, source.channel())
}

/// Clean recursive verdicts for complete command-substitution bodies of one
/// whole original image.
///
/// A body enters the memo only after the full recursive walk of that body,
/// under its exact nested parser configuration and input channel, finished
/// without a cut and without unavailable ownership. Every later walk that
/// reaches the identical whole-image body extent under the identical
/// configuration would repeat that same complete walk and conclude the same,
/// so it skips the descent instead of re-lexing the body. A body with a cut or
/// an unavailable stream is never recorded and is always walked again, so the
/// first cut, its term and its ordering are produced by the unchanged walk.
///
/// The memo binds to the first whole image it serves; a walk over any other
/// image bypasses it entirely.
#[derive(Default)]
pub struct NestedBodyCutMemo {
    image: std::sync::OnceLock<SourceImage>,
    clean: std::sync::Mutex<std::collections::HashSet<NestedBodyKey>>,
}

// Derived cache warmth is neither semantic identity nor diagnostic output.
impl std::fmt::Debug for NestedBodyCutMemo {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("NestedBodyCutMemo")
    }
}

/// Everything a body walk reads besides its own bytes: the whole-image
/// extent (which fixes the bytes of the bound image), the walk's own
/// configuration and the input channel.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct NestedBodyKey {
    span: Span,
    config: LexerConfig,
    channel: SourceChannel,
}

impl NestedBodyCutMemo {
    fn bound_to(&self, image: &SourceImage) -> bool {
        self.image.get_or_init(|| image.clone()) == image
    }

    fn is_clean(&self, key: NestedBodyKey) -> bool {
        self.clean
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains(&key)
    }

    fn record_clean(&self, key: NestedBodyKey) {
        self.clean
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key);
    }
}

/// A walk over the region of a bound whole image that starts at `origin`.
#[derive(Clone, Copy)]
struct MemoAnchor<'m> {
    memo: &'m NestedBodyCutMemo,
    origin: u32,
}

impl MemoAnchor<'_> {
    /// The whole-image key of a command body found at `body` in an arena whose
    /// own coordinates start `base` bytes into the walked region.
    fn key(
        self,
        base: u32,
        body: Span,
        config: LexerConfig,
        channel: SourceChannel,
    ) -> Option<NestedBodyKey> {
        let start = self.origin.checked_add(base)?.checked_add(body.start())?;
        let end = self.origin.checked_add(base)?.checked_add(body.end())?;
        Some(NestedBodyKey {
            span: Span::new(start, end),
            config,
            channel,
        })
    }
}

/// [`first_parse_cut_image_in_checked`] over `local`, the bytes of `whole`
/// starting at `origin`, sharing clean nested-body verdicts through `memo`.
/// The answer is identical to the unmemoised walk over `local`.
///
/// # Errors
/// Returns unavailable original component geometry or a nested lexical stream.
pub(crate) fn first_parse_cut_region_in_checked_with_memo(
    commands: &[CommandSpan],
    tokens: &[Token],
    local: &SourceImage,
    config: LexerConfig,
    whole: &SourceImage,
    origin: u32,
    memo: &NestedBodyCutMemo,
) -> Result<Option<ParseCut>, ParseCutUnavailable> {
    let anchored = local.channel() == whole.channel()
        && usize::try_from(origin).is_ok_and(|start| {
            whole
                .bytes()
                .get(start..)
                .and_then(|rest| rest.get(..local.bytes().len()))
                == Some(local.bytes())
        })
        && memo.bound_to(whole);
    first_parse_cut_channel_in_anchored(
        commands,
        tokens,
        local.bytes(),
        config,
        local.channel(),
        ParseCutDescent::RecursiveCommands,
        anchored.then_some(MemoAnchor { memo, origin }),
    )
}

fn first_parse_cut_channel_in(
    commands: &[CommandSpan],
    tokens: &[Token],
    src: &[u8],
    config: LexerConfig,
    channel: SourceChannel,
) -> Result<Option<ParseCut>, ParseCutUnavailable> {
    first_parse_cut_channel_in_with_descent(
        commands,
        tokens,
        src,
        config,
        channel,
        ParseCutDescent::RecursiveCommands,
    )
}

fn first_parse_cut_channel_in_with_descent(
    commands: &[CommandSpan],
    tokens: &[Token],
    src: &[u8],
    config: LexerConfig,
    channel: SourceChannel,
    descent: ParseCutDescent,
) -> Result<Option<ParseCut>, ParseCutUnavailable> {
    first_parse_cut_channel_in_anchored(commands, tokens, src, config, channel, descent, None)
}

fn first_parse_cut_channel_in_anchored(
    commands: &[CommandSpan],
    tokens: &[Token],
    src: &[u8],
    config: LexerConfig,
    channel: SourceChannel,
    descent: ParseCutDescent,
    anchor: Option<MemoAnchor<'_>>,
) -> Result<Option<ParseCut>, ParseCutUnavailable> {
    // One image of the whole script serves every command: building it hashes
    // and line-indexes all of `src`, so a per-command image is quadratic.
    let mut image = None;
    for (index, command) in commands.iter().enumerate() {
        let image = image.get_or_insert_with(|| SourceImage::from_bytes(src, channel));
        if let Some((offset, term, kind)) =
            command_cut(command, tokens, image, config, channel, descent, anchor)?
        {
            return Ok(Some(ParseCut {
                command: index,
                offset,
                term,
                kind,
                message: kind.message(),
            }));
        }
    }
    Ok(None)
}

/// Lex and group `src`, preserving an unavailable lexical stream.
///
/// A hard [`LexError`](crate::LexError) is a strict parser-mode rejection.
/// Original source bytes need no Unicode decoding to produce this stream.
fn lex_and_group(
    src: &[u8],
    config: LexerConfig,
    channel: SourceChannel,
) -> Result<(Vec<Token>, Vec<CommandSpan>), ParseCutUnavailable> {
    let lexer = Lexer::with_source_map(SourceMap::from_bytes_with_channel(src, channel), config);
    let tokens = lexer
        .tokenise_all()
        .map_err(ParseCutUnavailable::LexicalStream)?;
    let commands = group_commands_bytes(&tokens, src, config);
    Ok((tokens, commands))
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
enum Frame {
    /// A grouped script's original words, in document order.
    Words {
        image: SourceImage,
        jobs: Vec<WordJob>,
        base: u32,
        next: usize,
        report_at: Option<u32>,
        /// The whole-image key of a command body, recorded clean when every
        /// job of the body has been walked without a cut.
        clean_key: Option<NestedBodyKey>,
    },
    /// One ordered component list in a retained flat executable arena.
    Parts {
        arena: Rc<ExecutablePartArena>,
        list: PartListId,
        base: u32,
        next: usize,
        report_at: Option<u32>,
        /// The word's own delimiter failure, used only if no inner failure wins.
        fallback: Option<(u32, u32, crate::ParseIssueKind)>,
    },
}

/// One word of a script, resolved to what the walk must do with it.
///
/// Resolving every word of a body up front keeps a [`Frame`] free of borrows
/// into the token stream it was grouped from, which is what lets a nested
/// body be pushed without the enclosing frame still holding it.
enum WordJob {
    Unavailable,
    /// The word fails here, whatever its content holds.
    Cut(u32, u32, crate::ParseIssueKind),
    /// Walk this content; if nothing in it fails, report `fallback`.
    Content {
        content: crate::Span,
        fallback: Option<(u32, u32, crate::ParseIssueKind)>,
    },
    /// A braced word: C does not parse its content as anything.
    Literal,
}

/// The first cut among one command's words, in source order.
fn command_cut(
    command: &CommandSpan,
    tokens: &[Token],
    image: &SourceImage,
    config: LexerConfig,
    channel: SourceChannel,
    descent: ParseCutDescent,
    anchor: Option<MemoAnchor<'_>>,
) -> Result<Option<(u32, u32, crate::ParseIssueKind)>, ParseCutUnavailable> {
    let src = image.bytes();
    let jobs = word_jobs(&command.words, tokens, src, 0, config);
    let mut stack = vec![Frame::Words {
        image: image.clone(),
        jobs,
        base: 0,
        next: 0,
        report_at: None,
        clean_key: None,
    }];
    drain(&mut stack, config, channel, descent, anchor)
}

/// Resolve every word of a grouped script into its [`WordJob`], with offsets
/// already rebased onto the enclosing source.
fn word_jobs(
    words: &[crate::script::WordSpan],
    tokens: &[Token],
    src: &[u8],
    base: u32,
    config: LexerConfig,
) -> Vec<WordJob> {
    words
        .iter()
        .map(|word| word_job(word, tokens, src, base, config))
        .collect()
}

/// Check one word's own delimiters.
fn word_job(
    word: &crate::script::WordSpan,
    tokens: &[Token],
    src: &[u8],
    base: u32,
    config: LexerConfig,
) -> WordJob {
    let at = |offset: u32| offset.saturating_add(base);
    // A brace group that closed but has content welded to it — `{a}b`,
    // `{a}{b}`, `{a}{*}$b` — is C's first complaint about the word, and the
    // grouping owner is the only thing that can see it.
    if word.welded_after_close {
        // C reports this one in place, so the term is the offending byte.
        let weld = at(weld_offset(word, tokens, src));
        return WordJob::Cut(weld, weld, crate::ParseIssueKind::ExtraAfterBrace);
    }
    let Some(written) = written_span(word, tokens, src, config) else {
        return WordJob::Unavailable;
    };
    let (start, end) = (written.start() as usize, written.end() as usize);
    let content =
        |from: usize, to: usize, fallback: Option<(u32, u32, crate::ParseIssueKind)>| match src
            .get(from..to)
        {
            Some(_) => WordJob::Content {
                content: crate::Span::new(offset_of(from), offset_of(to)),
                fallback,
            },
            None => WordJob::Unavailable,
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
            if src.get(start) == Some(&b'{') && word_closer_offset_at(src, word.span).is_none() {
                // Unterminated: C's term is the `{` that opened it.
                WordJob::Cut(
                    at(written.end()),
                    at(written.start()),
                    crate::ParseIssueKind::MissingBrace,
                )
            } else {
                WordJob::Literal
            }
        }
        WordKind::Quoted => match quoted_word_close_with_issue_in(src, start, config) {
            // The quote never closed — but that is not necessarily why C
            // stopped.  `quoted_word_close` steps over complete `[…]`
            // substitutions to find the closer, so an *incomplete* one makes
            // it give up here while C, parsing the word's tokens left to
            // right, has already failed inside the bracket:
            // `puts "[foo"` is `missing close-bracket` on 8.6.16 and 9.0.4.
            // Walk the content first and keep `missing "` as the fallback.
            // Unterminated: C's term is the `"` that opened it.
            Err(kind) => content(
                start + 1,
                end,
                Some((at(written.end()), at(written.start()), kind)),
            ),
            // Anything written between the closing `"` and the end of the
            // word is C's `extra characters after close-quote`.
            Ok(close) if end > close + 1 && config.quote_termination.is_strict() => {
                // Reported in place, so the term is the offending byte.
                {
                    let extra = at(offset_of(close + 1));
                    WordJob::Cut(extra, extra, crate::ParseIssueKind::ExtraAfterQuote)
                }
            }
            Ok(close) if end > close + 1 => content(start + 1, end, None),
            Ok(close) => content(start + 1, close, None),
        },
        WordKind::Bare => content(start, end, None),
    }
}

/// Run the stack down to empty, or to the first cut.
fn drain(
    stack: &mut Vec<Frame>,
    config: LexerConfig,
    channel: SourceChannel,
    descent: ParseCutDescent,
    anchor: Option<MemoAnchor<'_>>,
) -> Result<Option<(u32, u32, crate::ParseIssueKind)>, ParseCutUnavailable> {
    while let Some(frame) = stack.last_mut() {
        match frame {
            Frame::Words {
                image,
                jobs,
                base,
                next,
                report_at,
                clean_key,
            } => {
                let report_at = *report_at;
                let Some(job) = jobs.get(*next) else {
                    // Every nested frame this body pushed has already been
                    // popped without a cut, so the whole body walk is clean.
                    if let (Some(anchor), Some(key)) = (anchor, *clean_key) {
                        anchor.memo.record_clean(key);
                    }
                    stack.pop();
                    continue;
                };
                *next += 1;
                match job {
                    WordJob::Unavailable => return Err(source_geometry()),
                    WordJob::Literal => {}
                    WordJob::Cut(offset, term, kind) => {
                        return Ok(Some((report_at.unwrap_or(*offset), *term, *kind)));
                    }
                    WordJob::Content { content, fallback } => {
                        let pushed = content_part_frame(
                            image.clone(),
                            *content,
                            *base,
                            report_at,
                            *fallback,
                            config,
                        )?;
                        stack.push(pushed);
                    }
                }
            }
            Frame::Parts {
                arena,
                list,
                base,
                next,
                report_at,
                fallback,
            } => {
                let (base, report_at) = (*base, *report_at);
                let Some(component) = arena.list(*list).get(*next) else {
                    let fallback = *fallback;
                    stack.pop();
                    if let Some(found) = fallback {
                        return Ok(Some(found));
                    }
                    continue;
                };
                *next += 1;
                let here = base.saturating_add(component.span.start());
                let at = report_at.unwrap_or(here);
                match component.part {
                    ExecutablePart::Text(_)
                    | ExecutablePart::Expression { .. }
                    | ExecutablePart::Variable { index: None, .. } => {}
                    ExecutablePart::ParseError(_) => {
                        return component_cut(component, base, at).map(Some);
                    }
                    ExecutablePart::Variable {
                        index: Some(index), ..
                    } => {
                        let pushed = Frame::Parts {
                            arena: Rc::clone(arena),
                            list: index,
                            base,
                            next: 0,
                            report_at: Some(at),
                            fallback: None,
                        };
                        stack.push(pushed);
                    }
                    ExecutablePart::Command { body } => {
                        if descent == ParseCutDescent::EnclosingScript {
                            continue;
                        }
                        let clean_key =
                            anchor.and_then(|anchor| anchor.key(base, body, config, channel));
                        // An identical body already walked clean cannot cut here.
                        if let (Some(anchor), Some(key)) = (anchor, clean_key)
                            && anchor.memo.is_clean(key)
                        {
                            continue;
                        }
                        // The inner script's original extent is independent of
                        // the enclosing variable's diagnostic reporting anchor.
                        let pushed = command_part_frame(
                            arena, body, base, report_at, config, channel, clean_key,
                        )?;
                        stack.push(pushed);
                    }
                }
            }
        }
    }
    Ok(None)
}

/// The cut a component's retained parse error reports at `at`.
fn component_cut(
    component: &crate::SpannedExecutablePart,
    base: u32,
    at: u32,
) -> Result<(u32, u32, crate::ParseIssueKind), ParseCutUnavailable> {
    let term = component
        .parse_error_term()
        .and_then(|term| base.checked_add(term))
        .ok_or_else(source_geometry)?;
    let kind = component.parse_issue_kind().ok_or_else(source_geometry)?;
    Ok((at, term, kind))
}

const fn source_geometry() -> ParseCutUnavailable {
    ParseCutUnavailable::SourceGeometry(
        crate::word_parts::ExecutablePartsUnavailable::SourceGeometry,
    )
}

fn content_part_frame(
    image: SourceImage,
    content: Span,
    base: u32,
    report_at: Option<u32>,
    fallback: Option<(u32, u32, crate::ParseIssueKind)>,
    config: LexerConfig,
) -> Result<Frame, ParseCutUnavailable> {
    let arena = ExecutablePartArena::decompose(image, content, SubstFlags::default(), config)
        .map_err(ParseCutUnavailable::SourceGeometry)?;
    let list = arena.root();
    Ok(Frame::Parts {
        arena: Rc::new(arena),
        list,
        base,
        next: 0,
        report_at,
        fallback: fallback.map(|(offset, term, kind)| (report_at.unwrap_or(offset), term, kind)),
    })
}

fn command_part_frame(
    arena: &ExecutablePartArena,
    body: Span,
    base: u32,
    report_at: Option<u32>,
    config: LexerConfig,
    channel: SourceChannel,
    clean_key: Option<NestedBodyKey>,
) -> Result<Frame, ParseCutUnavailable> {
    let unavailable = || {
        ParseCutUnavailable::SourceGeometry(
            crate::word_parts::ExecutablePartsUnavailable::SourceBounds,
        )
    };
    let child_base = base.checked_add(body.start()).ok_or_else(unavailable)?;
    let body = arena.bytes(body).ok_or_else(unavailable)?;
    body_frame(body, child_base, report_at, config, channel, clean_key)
}

/// A complete `[…]` body as a script frame to walk.
///
/// A body that failed to *close* never reaches here — [`ExecutablePartArena`]
/// reports that as an [`ExecutablePart::ParseError`] on the enclosing word — so this
/// is the descent C performs while parsing an outer command whose bracket is
/// well-formed but whose inner script is not: `list [set y {a}b]` is
/// `extra characters after close-brace`, found one level down.
fn body_frame(
    body: &[u8],
    base: u32,
    report_at: Option<u32>,
    config: LexerConfig,
    channel: SourceChannel,
    clean_key: Option<NestedBodyKey>,
) -> Result<Frame, ParseCutUnavailable> {
    let config = crate::SourceSegmentationPurpose::NestedScript.config(config);
    let (tokens, commands) = lex_and_group(body, config, channel)?;
    let jobs = commands
        .iter()
        .flat_map(|command| word_jobs(&command.words, &tokens, body, base, config))
        .collect();
    Ok(Frame::Words {
        image: SourceImage::from_bytes(body, channel),
        jobs,
        base,
        next: 0,
        report_at,
        clean_key,
    })
}

/// The whole written word, closing delimiter included.
///
/// Both the grouped word and its final component retain closing delimiters.
/// A quoted word can end in an escape fragment with no opening quote of its
/// own; a bare concatenating word can end in a bracketed component with its
/// own closer. The shared group geometry covers both cases.
fn written_span(
    word: &crate::script::WordSpan,
    tokens: &[Token],
    src: &[u8],
    config: LexerConfig,
) -> Option<crate::Span> {
    // An empty or foreign token range cannot borrow the preceding word's token.
    let last = tokens.get(word.tokens.clone())?.last()?;
    let end = match crate::native_word::complete_group_span(src, word, *last, config) {
        Ok(span) => span.end(),
        // An incomplete variable still needs its original bytes inspected for
        // the native syntax message, rather than becoming a geometry refusal.
        Err(crate::word_parts::NativeWordError::Parse(_)) => last.span.end(),
        Err(_) => return None,
    };
    Some(crate::Span::new(
        word.span.start(),
        end.max(word.span.end()),
    ))
}

/// Byte offset of the content welded to a word's closing brace.
///
/// [`WordSpan::welded_after_close`] marks the word, not the position; the
/// offset is where the *next* fragment of the word starts, which is the
/// byte after the `}` that C stopped at.  Falls back to the word's own end
/// if the word somehow holds a single token (it cannot: welding needs two).
fn weld_offset(word: &crate::script::WordSpan, tokens: &[Token], src: &[u8]) -> u32 {
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
    use crate::{
        ExecutablePartArena, SourceImage, Span, SubstFlags, first_parse_cut_image_checked,
    };

    #[test]
    fn original_bracket_child_cut_keeps_bom_content_at_local_zero() {
        // A true bracket child keeps nested syntax even at its local byte zero.
        let config = LexerConfig {
            leading_bom: crate::LeadingBom::Skip,
            ..LexerConfig::for_dialect("tcl8.6")
        };
        for source in ["list [\u{feff}{x}tail]", "list [\u{feff}list {x}]"] {
            for image in [
                SourceImage::document(source),
                SourceImage::native(source.as_bytes()),
            ] {
                assert_eq!(first_parse_cut_image_checked(&image, config), Ok(None));
            }
        }
        let malformed = "list [\u{feff}list {x}tail]";
        for image in [
            SourceImage::document(malformed),
            SourceImage::native(malformed.as_bytes()),
        ] {
            let cut = first_parse_cut_image_checked(&image, config)
                .unwrap()
                .unwrap();
            assert_eq!(cut.message, crate::word_parts::EXTRA_AFTER_CLOSE_BRACE);
            assert_eq!(cut.term as usize, malformed.find("tail").unwrap());
        }
    }

    #[test]
    fn deep_index_cut_preserves_outer_anchor_and_exact_inner_term() {
        let mut source = b"puts pre; list ".to_vec();
        let anchor = source.len();
        for _ in 0..2_000 {
            source.extend_from_slice(b"$a(");
        }
        source.extend_from_slice(b"[set y {a}b]");
        let term = source.windows(4).position(|part| part == b"{a}b").unwrap() + 3;
        source.extend(std::iter::repeat_n(b')', 2_000));
        let cut = super::first_parse_cut_image_checked(
            &crate::SourceImage::native(source),
            LexerConfig::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(cut.command, 1);
        assert_eq!(cut.offset as usize, anchor);
        assert_eq!(cut.term as usize, term);
        assert_eq!(cut.message, crate::word_parts::EXTRA_AFTER_CLOSE_BRACE);
    }

    #[test]
    fn checked_cut_separates_clean_syntax_geometry_and_lexical_refusal() {
        let config = LexerConfig::default();
        let image = crate::SourceImage::native(b"list ${x}".as_slice());
        assert_eq!(
            super::first_parse_cut_image_checked(&image, config),
            Ok(None)
        );
        let tokens = crate::Lexer::with_source_image(&image, config)
            .tokenise_all()
            .unwrap();
        let mut groups = crate::group_commands_bytes(&tokens, image.bytes(), config);
        groups[0].words[1].tokens.end = tokens.len() + 1;
        assert!(matches!(
            super::first_parse_cut_image_in_checked(&groups, &tokens, &image, config),
            Err(super::ParseCutUnavailable::SourceGeometry(_)),
        ));
        assert!(matches!(
            super::first_parse_cut_image_checked(
                &crate::SourceImage::native(b"list \"x\"y".as_slice()),
                LexerConfig {
                    strict_quoting: true,
                    ..config
                },
            ),
            Err(super::ParseCutUnavailable::LexicalStream(_)),
        ));
    }

    #[test]
    fn complete_jim_expression_components_do_not_cut_the_script() {
        use tcl_dialect::model::{Family, Release, grammar};
        let config = LexerConfig::from_grammar(grammar(Family::Jim, Release::JIM_0_84));
        for source in [
            "set (k) ELEMENT; set i k; list $(k) $($i)",
            "list prefix$(k)",
            "list $()",
            "list $(($a+1)*2)",
            r"list $(a\)b)",
        ] {
            assert_eq!(first_parse_cut(source, config), None, "{source}");
        }
        let cut = first_parse_cut("set i k; list $($i", config).unwrap();
        assert_eq!(cut.command, 1);
        assert_eq!(
            cut.message,
            "missing close-paren for expression substitution"
        );
    }

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
            "bracket-inner-var",
            "puts pre; list [sfx one] [set y \"a${bcd\"]",
            1,
            crate::MISSING_CLOSE_BRACE_FOR_VAR,
        ),
        (
            "bracket-bare-var",
            "puts pre; list [set y a${bcd]",
            1,
            crate::MISSING_CLOSE_BRACE_FOR_VAR,
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
    /// `{*}{a b}` is a welded close-brace rather than an expansion.
    #[test]
    fn cut_follows_the_configured_dialect() {
        let script = "puts pre; foo {*}{a b}";
        assert_eq!(
            first_parse_cut(script, LexerConfig::for_dialect("tcl8.4")),
            Some(ParseCut {
                command: 1,
                offset: 17,
                // Reported in place, so the term is the same byte.
                term: 17,
                kind: crate::ParseIssueKind::ExtraAfterBrace,
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

    #[test]
    fn original_nested_parse_error_terms_keep_the_failed_delimiter() {
        // The original C8.4 capture rows 6 and 9 retain terms 13 and 11.
        for source in [
            SourceImage::document("set x [bad x \"abc"),
            SourceImage::native(b"set x [bad x \"abc".as_slice()),
        ] {
            let cut = first_parse_cut_image_checked(&source, LexerConfig::for_dialect("tcl8.4"))
                .unwrap()
                .unwrap();
            assert_eq!(
                (cut.command, cut.offset, cut.term, cut.message),
                (0, 6, 13, crate::word_parts::MISSING_QUOTE)
            );
            let arena = ExecutablePartArena::decompose(
                source.clone(),
                Span::new(6, u32::try_from(source.len()).unwrap()),
                SubstFlags::default(),
                LexerConfig::for_dialect("tcl8.4"),
            )
            .unwrap();
            let [component] = arena.list(arena.root()) else {
                panic!("one rejected original component");
            };
            assert_eq!(component.span.start(), 6);
            assert_eq!(component.parse_error_term(), Some(13));
        }
        let source = SourceImage::native(b"set x [bad {".as_slice());
        let cut = first_parse_cut_image_checked(&source, LexerConfig::for_dialect("tcl8.4"))
            .unwrap()
            .unwrap();
        assert_eq!(
            (cut.command, cut.offset, cut.term, cut.message),
            (0, 6, 11, crate::word_parts::MISSING_CLOSE_BRACE)
        );
    }

    #[test]
    fn original_error_term_rebasing_preserves_native_bytes_and_inner_reference() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let config = LexerConfig::for_dialect(environment);
            for (source, offset, term, message) in [
                (
                    b"set x [bad \xff \"\0".as_slice(),
                    6,
                    13,
                    crate::word_parts::MISSING_QUOTE,
                ),
                (
                    b"set x [bad ${name".as_slice(),
                    6,
                    12,
                    crate::MISSING_CLOSE_BRACE_FOR_VAR,
                ),
                (
                    b"set x $a([bad \"abc".as_slice(),
                    6,
                    14,
                    crate::word_parts::MISSING_QUOTE,
                ),
                (
                    b"set x [[".as_slice(),
                    6,
                    7,
                    crate::word_parts::MISSING_CLOSE_BRACKET,
                ),
            ] {
                let cut = first_parse_cut_image_checked(&SourceImage::native(source), config)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    (cut.offset, cut.term, cut.message),
                    (offset, term, message),
                    "{environment} {source:?}"
                );
            }
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

#[cfg(test)]
mod original_parse_descent_tests {
    use super::*;

    #[test]
    fn original_enclosing_script_cut_delays_child_syntax_without_hiding_root_delimiters() {
        // The physical Direct observations are separate from this syntax-only
        // projection; neither channel nor descent issues an execution receipt.
        // Syntax depth only; genuine Registry entry independently selects its use.
        let config = LexerConfig::for_dialect("jim");
        for source in [
            "set before 1; list [list {a}b]",
            "set before 1; list \"[list {a}b]\"",
            "set before 1; list $arr([list {a}b])",
        ] {
            let image = SourceImage::document(source);
            assert_eq!(
                first_parse_cut_image_with_descent_checked(
                    &image,
                    config,
                    ParseCutDescent::EnclosingScript
                ),
                Ok(None),
                "{source}"
            );
            assert!(
                first_parse_cut_image_checked(&image, config)
                    .unwrap()
                    .is_some(),
                "{source}"
            );
        }
        for source in [
            "set before 1; {a}b",
            "set before 1; list [list {a]",
            "set before 1; list \"unterminated",
        ] {
            let image = SourceImage::document(source);
            let recursive = first_parse_cut_image_checked(&image, config).unwrap();
            let enclosing = first_parse_cut_image_with_descent_checked(
                &image,
                config,
                ParseCutDescent::EnclosingScript,
            )
            .unwrap();
            assert!(enclosing.is_some(), "{source}");
            assert_eq!(enclosing, recursive, "{source}");
        }
        let child = SourceImage::document("list {a}b");
        assert!(
            first_parse_cut_image_with_descent_checked(
                &child,
                config.nested(),
                ParseCutDescent::EnclosingScript
            )
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn original_parse_descent_keeps_recursive_wrapper_and_unavailable_geometry_distinct() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let config = LexerConfig::for_dialect(dialect);
            for source in ["list 1", "list [list {a}b]", "list ${a{b}c}"] {
                let image = SourceImage::document(source);
                assert_eq!(
                    first_parse_cut_image_checked(&image, config),
                    first_parse_cut_image_with_descent_checked(
                        &image,
                        config,
                        ParseCutDescent::RecursiveCommands
                    )
                );
            }
        }
        let image = SourceImage::document("list 1");
        let tokens = Lexer::with_source_image(&image, LexerConfig::default())
            .tokenise_all()
            .unwrap();
        let mut groups = group_commands_bytes(&tokens, image.bytes(), LexerConfig::default());
        groups[0].words[0].tokens = tokens.len()..tokens.len();
        for descent in [
            ParseCutDescent::RecursiveCommands,
            ParseCutDescent::EnclosingScript,
        ] {
            assert!(
                first_parse_cut_channel_in_with_descent(
                    &groups,
                    &tokens,
                    image.bytes(),
                    LexerConfig::default(),
                    image.channel(),
                    descent
                )
                .is_err()
            );
        }
    }
}

#[cfg(test)]
mod nested_body_cut_memo_tests {
    use super::*;

    fn span_of(source: &str, part: &str) -> Span {
        let start = source.find(part).unwrap();
        Span::new(
            u32::try_from(start).unwrap(),
            u32::try_from(start + part.len()).unwrap(),
        )
    }

    fn whole_span(image: &SourceImage) -> Span {
        Span::new(0, u32::try_from(image.len()).unwrap())
    }

    /// The unmemoised and memoised cuts of `region`, lexed as an original
    /// region capture lexes it.
    fn region_cuts(
        whole: &SourceImage,
        region: Span,
        config: LexerConfig,
        memo: &NestedBodyCutMemo,
    ) -> [Result<Option<ParseCut>, ParseCutUnavailable>; 2] {
        let local = SourceImage::from_bytes(&whole.bytes()[region.as_range()], whole.channel());
        let config = crate::SourceSegmentationPurpose::NestedScript
            .config(config)
            .normalized();
        let tokens = Lexer::with_source_image(&local, config)
            .tokenise_all()
            .unwrap();
        let groups = group_commands_bytes(&tokens, local.bytes(), config);
        [
            first_parse_cut_image_in_checked(&groups, &tokens, &local, config),
            first_parse_cut_region_in_checked_with_memo(
                &groups,
                &tokens,
                &local,
                config,
                whole,
                region.start(),
                memo,
            ),
        ]
    }

    fn clean_bodies(memo: &NestedBodyCutMemo) -> usize {
        memo.clean.lock().unwrap().len()
    }

    #[test]
    fn nested_body_memo_shares_a_clean_body_between_overlapping_regions() {
        let source = "set a [list [list x] [list y]]; set b 1";
        let whole = SourceImage::document(source);
        let memo = NestedBodyCutMemo::default();
        let debug = format!("{memo:?}");
        for region in [
            span_of(source, "set a [list [list x] [list y]]"),
            span_of(source, "list [list x] [list y]"),
            whole_span(&whole),
        ] {
            let [plain, memoised] = region_cuts(&whole, region, LexerConfig::default(), &memo);
            assert_eq!(plain, Ok(None));
            assert_eq!(memoised, plain);
            // `list x`, `list y` and the body enclosing them.
            assert_eq!(clean_bodies(&memo), 3);
        }
        assert_eq!(format!("{memo:?}"), debug);
    }

    #[test]
    fn nested_body_memo_rewalks_a_cut_beside_a_clean_body() {
        let source = "list [list [list a] [set y {a}b]]";
        let whole = SourceImage::document(source);
        let memo = NestedBodyCutMemo::default();
        for region in [
            whole_span(&whole),
            span_of(source, "list [list a] [set y {a}b]"),
            whole_span(&whole),
        ] {
            let [plain, memoised] = region_cuts(&whole, region, LexerConfig::default(), &memo);
            assert_eq!(
                plain.clone().unwrap().unwrap().message,
                crate::word_parts::EXTRA_AFTER_CLOSE_BRACE
            );
            assert_eq!(memoised, plain);
            assert_eq!(clean_bodies(&memo), 1);
        }
    }

    #[test]
    fn nested_body_memo_keeps_the_unterminated_quote_fallback_after_a_clean_body() {
        let source = "puts \"[list [list a] b]";
        let whole = SourceImage::document(source);
        let memo = NestedBodyCutMemo::default();
        let [plain, memoised] = region_cuts(
            &whole,
            span_of(source, "list [list a] b"),
            LexerConfig::default(),
            &memo,
        );
        assert_eq!(plain, Ok(None));
        assert_eq!(memoised, plain);
        assert_eq!(clean_bodies(&memo), 1);
        for _ in 0..2 {
            let [plain, memoised] =
                region_cuts(&whole, whole_span(&whole), LexerConfig::default(), &memo);
            assert_eq!(
                plain.clone().unwrap().unwrap().message,
                crate::word_parts::MISSING_QUOTE
            );
            assert_eq!(memoised, plain);
            assert_eq!(clean_bodies(&memo), 2);
        }
    }

    #[test]
    fn nested_body_memo_keeps_the_variable_index_reporting_anchor() {
        let source = "puts $a([list [list x] {y}z])";
        let whole = SourceImage::document(source);
        let memo = NestedBodyCutMemo::default();
        let [plain, memoised] = region_cuts(
            &whole,
            span_of(source, "list [list x] {y}z"),
            LexerConfig::default(),
            &memo,
        );
        assert!(plain.clone().unwrap().is_some());
        assert_eq!(memoised, plain);
        assert_eq!(clean_bodies(&memo), 1);
        let [plain, memoised] =
            region_cuts(&whole, whole_span(&whole), LexerConfig::default(), &memo);
        let cut = plain.clone().unwrap().unwrap();
        assert_eq!(cut.offset as usize, source.find('$').unwrap());
        assert_eq!(cut.term as usize, source.find("{y}z").unwrap() + 3);
        assert_eq!(cut.message, crate::word_parts::EXTRA_AFTER_CLOSE_BRACE);
        assert_eq!(memoised, plain);
        assert_eq!(clean_bodies(&memo), 1);
    }

    #[test]
    fn nested_body_memo_never_records_unavailable_streams() {
        let strict = LexerConfig {
            strict_quoting: true,
            ..LexerConfig::default()
        };
        let source = "list [list a] [list \"x\"y]";
        let whole = SourceImage::document(source);
        let memo = NestedBodyCutMemo::default();
        for _ in 0..2 {
            let [plain, memoised] = region_cuts(&whole, whole_span(&whole), strict, &memo);
            assert!(matches!(plain, Err(ParseCutUnavailable::LexicalStream(_))));
            assert_eq!(memoised, plain);
            assert_eq!(clean_bodies(&memo), 1);
        }

        let source = "list [list a] b";
        let whole = SourceImage::document(source);
        let config = LexerConfig::default();
        let tokens = Lexer::with_source_image(&whole, config)
            .tokenise_all()
            .unwrap();
        let mut groups = group_commands_bytes(&tokens, whole.bytes(), config);
        groups[0].words[2].tokens.end = tokens.len() + 1;
        let memo = NestedBodyCutMemo::default();
        for _ in 0..2 {
            let plain = first_parse_cut_image_in_checked(&groups, &tokens, &whole, config);
            let memoised = first_parse_cut_region_in_checked_with_memo(
                &groups, &tokens, &whole, config, &whole, 0, &memo,
            );
            assert!(matches!(plain, Err(ParseCutUnavailable::SourceGeometry(_))));
            assert_eq!(memoised, plain);
            assert_eq!(clean_bodies(&memo), 1);
        }
    }

    #[test]
    fn nested_body_memo_keys_each_lexer_configuration_separately() {
        let source = "list [foo {*}{a b}]";
        let whole = SourceImage::document(source);
        let memo = NestedBodyCutMemo::default();
        for (dialect, cuts) in [("tcl8.6", false), ("tcl8.4", true), ("tcl8.6", false)] {
            let [plain, memoised] = region_cuts(
                &whole,
                whole_span(&whole),
                LexerConfig::for_dialect(dialect),
                &memo,
            );
            assert_eq!(plain.clone().unwrap().is_some(), cuts, "{dialect}");
            assert_eq!(memoised, plain, "{dialect}");
            assert_eq!(clean_bodies(&memo), 1, "{dialect}");
        }
    }

    #[test]
    fn nested_body_memo_bypasses_a_foreign_image() {
        let memo = NestedBodyCutMemo::default();
        let bound = SourceImage::document("list [list {x} y]");
        // The same body extent cuts in the first foreign image; the other two
        // would add a clean body had the memo served them.
        for (image, cuts) in [
            (bound.clone(), false),
            (SourceImage::document("list [list {x}y ]"), true),
            (SourceImage::document("list [list q]"), false),
            (SourceImage::native(bound.bytes()), false),
        ] {
            let [plain, memoised] =
                region_cuts(&image, whole_span(&image), LexerConfig::default(), &memo);
            assert_eq!(plain.clone().unwrap().is_some(), cuts);
            assert_eq!(memoised, plain);
            assert_eq!(clean_bodies(&memo), 1);
        }
    }
}
