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

//! Command segmentation for Tcl token streams.
//!
//! Splits a flat token stream into per-command structures at EOL
//! boundaries. Both the analyser and lowerer consume these structures
//! instead of running their own token-iteration loops.

use tcl_lexer::{Lexer, LexerConfig, SourceMap, Span, Token, TokenType};

/// One lexical fragment that contributes to a segmented Tcl word.
///
/// [`SegmentedCommand::argv`] deliberately retains only one representative
/// token per word for compatibility with the analyser and existing lowering
/// hooks.  That summary loses the ordered shape of a compound word such as
/// `prefix-$name-[clock seconds]`.  This companion record keeps the original
/// fragment sequence and its already-reconstructed source spelling so the
/// semantic IR can be derived without re-lexing or guessing from a flattened
/// argv string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WordFragment {
    /// Original lexer token, including its source span and quoting metadata.
    pub token: Token,
    /// Compatibility spelling reconstructed with [`word_piece`].
    ///
    /// Variable and command-substitution fragments retain their Tcl wrappers
    /// here, matching [`SegmentedCommand::texts`]. Their exact written form is
    /// available through [`Self::token`]'s source span.
    pub text: String,
}

impl WordFragment {
    /// Create a fragment from a lexer token and its source-surface spelling.
    #[must_use]
    pub fn new(token: Token, text: String) -> Self {
        Self { token, text }
    }
}

/// Which delimiter a partial command left unclosed. The token-type
/// mapping is `Str` → `Brace`, `Cmd` → `Bracket`, `Esc` → `Quote`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnclosedDelimiter {
    /// Unclosed `{`.
    Brace,
    /// Unclosed `[`.
    Bracket,
    /// Unclosed `"`.
    Quote,
}

impl UnclosedDelimiter {
    /// Map the suspicious EOF-reaching token's kind to the delimiter it
    /// left open, or `None` for a token kind that is never a delimiter.
    #[must_use]
    pub fn from_token_kind(kind: TokenType) -> Option<Self> {
        match kind {
            TokenType::Str => Some(Self::Brace),
            TokenType::Cmd => Some(Self::Bracket),
            TokenType::Esc => Some(Self::Quote),
            _ => None,
        }
    }

    /// The E200 "missing …" message for this unclosed delimiter.
    #[must_use]
    pub fn missing_message(self) -> &'static str {
        match self {
            Self::Brace => "missing close-brace",
            Self::Bracket => "missing close-bracket",
            Self::Quote => "missing \"",
        }
    }
}

/// A single Tcl command parsed from the token stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentedCommand {
    /// Byte span covering the whole command.
    pub span: Span,
    /// Per-word representative tokens (one per argv entry).
    pub argv: Vec<Token>,
    /// Per-word reconstructed text.
    pub texts: Vec<String>,
    /// Ordered lexical fragments for every word.
    ///
    /// This is the lossless companion to the compatibility-oriented
    /// [`Self::argv`] / [`Self::texts`] parallel arrays.  New semantic IR
    /// consumers should use it when they need to preserve word substitution
    /// order; existing consumers may continue to use the representative-token
    /// view unchanged.
    pub word_fragments: Vec<Vec<WordFragment>>,
    /// Whether each word is a single token.
    pub single_token_word: Vec<bool>,
    /// All tokens in the command (including separators).
    pub all_tokens: Vec<Token>,
    /// Whether the command is incomplete (unclosed delimiter).
    pub is_partial: bool,
    /// Which delimiter was left unclosed, when `is_partial`. Set by the
    /// recovery segmenter or checked script's structured parse cut; drives
    /// the precise E200 message and gates stolen-close-brace detection to
    /// brace partials only.
    pub partial_delimiter: Option<UnclosedDelimiter>,
    /// `{*}` expansion markers per word, if any word uses expansion.
    pub expand_word: Option<Vec<bool>>,
    /// Concatenated text of comment line(s) immediately preceding
    /// the command (without the leading ``#`` and with leading
    /// whitespace trimmed); ``None``
    /// when no comment precedes.  Used to populate `ProcDef.doc` /
    /// `ClassDef.doc`.
    pub preceding_comment: Option<String>,
}

impl SegmentedCommand {
    /// Command name (first word).
    #[must_use]
    pub fn name(&self) -> &str {
        self.texts.first().map_or("", String::as_str)
    }

    /// Full written span to compile or replay as this command.
    ///
    /// [`Self::span`] follows the analyser's `cmd.range`
    /// convention, which deliberately leaves the final `"` of a quoted last
    /// word outside the range. Executable consumers need the whole written
    /// command instead. Build the last word's complete fragment span, then
    /// widen it through the lexer's canonical delimiter-range owner so quotes,
    /// braces, brackets, empty words, and nested closers stay consistent.
    #[must_use]
    pub fn execution_span(&self, source: &str) -> Span {
        let Some(fragments) = self.word_fragments.last() else {
            return self.span;
        };
        let (Some(first), Some(last)) = (fragments.first(), fragments.last()) else {
            return self.span;
        };
        let last_word = Span::new(first.token.span.start(), last.token.span.end());
        let last_word = tcl_lexer::word_span_at(source, last_word);
        Span::new(self.span.start(), self.span.end().max(last_word.end()))
    }

    /// Arguments (words after the command name).
    #[must_use]
    pub fn args(&self) -> &[String] {
        if self.texts.len() > 1 {
            &self.texts[1..]
        } else {
            &[]
        }
    }

    /// Per-arg representative tokens.
    #[must_use]
    pub fn arg_tokens(&self) -> &[Token] {
        if self.argv.len() > 1 {
            &self.argv[1..]
        } else {
            &[]
        }
    }

    /// Per-arg single-token flags.
    #[must_use]
    pub fn arg_single_token(&self) -> &[bool] {
        if self.single_token_word.len() > 1 {
            &self.single_token_word[1..]
        } else {
            &[]
        }
    }

    /// Whether every per-word segmentation view has the same length.
    ///
    /// Command recovery can splice words after segmentation, so this makes the
    /// parallel-array invariant explicit at the one shared representation.
    #[must_use]
    pub fn word_views_aligned(&self) -> bool {
        let count = self.argv.len();
        self.texts.len() == count
            && self.word_fragments.len() == count
            && self.single_token_word.len() == count
            && self
                .expand_word
                .as_ref()
                .is_none_or(|expand| expand.len() == count)
    }

    /// Return a copy of `self` with every span shifted by
    /// `base_offset`. Used by
    /// [`segment_commands_with_offset`] to relocate a body
    /// script's spans into the outer source buffer's offset
    /// space.
    #[must_use]
    pub fn shifted_by(mut self, base_offset: u32) -> Self {
        self.span = shift_span(self.span, base_offset);
        for tok in &mut self.argv {
            tok.span = shift_span(tok.span, base_offset);
        }
        for tok in &mut self.all_tokens {
            tok.span = shift_span(tok.span, base_offset);
        }
        for word in &mut self.word_fragments {
            for fragment in word {
                fragment.token.span = shift_span(fragment.token.span, base_offset);
            }
        }
        self
    }
}

fn shift_span(span: Span, by: u32) -> Span {
    Span::new(span.start() + by, span.end() + by)
}

/// The whole written source of `tok`'s word, closing delimiter included, when
/// the span lies inside `sm`'s source.
///
/// Widened through [`tcl_lexer::word_span_at`], the shared owner of the
/// inner-end convention, so an empty `${}` is not overshot and a name whose
/// last inner byte is itself a `}` is not left short.
fn word_span_source<'s>(sm: &SourceMap<'s>, tok: Token) -> Option<&'s str> {
    let source = sm.source();
    let span = tcl_lexer::word_span_at(source, tok.span);
    source.get(span.start() as usize..span.end() as usize)
}

/// Return the source-level text fragment for a single token.
///
/// Variables are prefixed with `$` and command substitutions are
/// wrapped in `[...]` so that the result mirrors what the user wrote.
///
/// A braced `${…}` variable round-trips **verbatim from source**, because the
/// lexer already applied the target release's `Tcl_ParseVarName` rule when it
/// spanned the token — see the `is_braced` arm below.
///
/// Bare `$arr(idx)` whose index contains a `$` or `[` substitution
/// round-trips verbatim — wrapping in braces would disable array-
/// element interpretation and turn the recursive substitution into a
/// literal scalar lookup (cmdAH-1.4 / 1.5 `$numargErrors($cmd)`).
/// Bare-vs-braced is decided by `content_offset`: the Rust lexer
/// emits `content_offset = 1` for bare `$name` / `$arr(idx)` (skips
/// the `$`) and `content_offset = 2` for braced `${name}` (skips
/// `${`).
#[must_use]
pub fn word_piece(sm: &SourceMap<'_>, tok: Token) -> String {
    let text = sm.token_text(tok);
    match tok.kind {
        TokenType::Var => {
            let is_braced = tok.content_offset >= 2;
            if !is_braced
                && tcl_syntax::naming::split_element_ref(text).is_some()
                && (text.contains('$') || text.contains('['))
            {
                return format!("${text}");
            }
            // A braced `${…}` word is re-spelt **verbatim from source**.
            //
            // Falling back to a bare `$name` spelling whenever the name
            // contains a `}` — for want of an unambiguous `${…}` formed from
            // the name alone — is wrong: under 9.x the lexer spans `${a{b}c}`
            // as one `Var` whose name is `a{b}c`, so that bail produces
            // `$a{b}c`, which no decoder recognises as a reference and which
            // is then pushed as a literal, substituting nothing.
            //
            // Echoing the source needs no release rule of its own, which is
            // the point: the **lexer** already applied the target release's
            // `Tcl_ParseVarName` rule when it decided where this token ends,
            // so its span encodes the answer. `word_span_at` recovers the
            // closer from that span and is documented release-blind for
            // exactly this form. Under 8.x the same source lexes as `${a{b}`
            // plus an `Esc` `c}`, and concatenating the pieces reproduces
            // `${a{b}c}` — the identical spelling, which the decoders then
            // read under *their* release's rule.
            if is_braced && let Some(src) = word_span_source(sm, tok) {
                return src.to_owned();
            }
            if text.contains('}') {
                format!("${text}")
            } else {
                format!("${{{text}}}")
            }
        }
        TokenType::Cmd => format!("[{text}]"),
        // `JimTcl`'s `$(…)`: the token text is the expression body, so the
        // logical word keeps the sugar around it rather than losing it.
        TokenType::ExprSugar => format!("$({text})"),
        _ => text.to_owned(),
    }
}

/// Compute the whole-command span, widening the final word to cover its
/// closing delimiter.
///
/// The lexer follows an "inner-end"
/// span convention: a braced (`{…}`) or bracketed (`[…]`) word token's
/// `span.end()` is the *exclusive* offset of the closing `}` / `]`, so the
/// closer itself sits one byte past the end. A command whose final word is
/// braced (`if {$x} {body}`) would therefore drop the trailing `}` from its
/// whole-command range. Widen the end by one byte when the last token is a
/// `Str` / `Cmd` whose closer actually sits at `span.end()`.
///
/// The closer character is derived from the token *type*; whether the
/// closer is *already covered* by the
/// span is derived from the token *text*, not a source byte — see
/// [`widen_word_end`].
pub(crate) fn command_span(tokens: &[Token], sm: &SourceMap<'_>) -> Span {
    if tokens.is_empty() {
        return Span::new(0, 0);
    }
    let start = tokens.first().unwrap().span.start();
    let end = widen_word_end(*tokens.last().unwrap(), sm);
    Span::new(start, end)
}

/// Exclusive end offset of `tok` including its closing delimiter, for the
/// braced / bracketed word forms. See [`command_span`].
///
/// The inner-end convention itself — and every trap in widening past it (the
/// empty `{}` / `[]` whose span *already* covers its closer, the
/// backslash-escaped inner closer of `{a\}}`) — lives once in
/// [`tcl_lexer::word_span`], the one authoritative "whole written word"
/// helper; this is only the `cmd.range`-specific *policy* layered on top.
///
/// That policy is the braced/bracketed **type gate**: quoted `"…"` words are
/// deliberately not widened here, because `cmd.range` consumers (W105
/// unbraced-body detection, segmenter tiling) rely on the inner-end for
/// them. Anything that wants the whole word regardless of kind must call
/// [`tcl_lexer::word_span`] directly rather than reach for this.
fn widen_word_end(tok: Token, sm: &SourceMap<'_>) -> u32 {
    if tok.kind.group_closer().is_none() {
        return tok.span.end();
    }
    tcl_lexer::word_span(sm, tok).end()
}

/// Segment a token stream into per-command structures at EOL boundaries.
///
/// The core segmentation loop. Use
/// [`segment_commands_with_recovery`] when a known-commands set is
/// available and the caller wants error recovery
/// (scanning past unclosed `{` / `[` / `"` for the next known
/// command name).
#[must_use]
pub fn segment_commands(source: &str) -> Vec<SegmentedCommand> {
    segment_commands_with_offset(source, 0)
}

/// Segment with a base byte offset (for body scripts inside braces).
///
/// The lexer tokenises `source` starting at local offset `0`.
/// Segmentation runs in local-offset space so the `SourceMap`
/// can slice text via [`Token::span`]; immediately before
/// returning, every `SegmentedCommand` has its spans relocated
/// by `base_offset` so downstream IR / optimiser / def-use
/// consumers see absolute offsets into the outer source buffer.
#[must_use]
pub fn segment_commands_with_offset(source: &str, base_offset: u32) -> Vec<SegmentedCommand> {
    // dialect-drift-ok: the dialect-blind entry point; a caller holding the
    // document's config uses `segment_commands_with_offset_and_config`.
    segment_commands_with_offset_and_config(source, base_offset, LexerConfig::default())
}

/// Like [`segment_commands_with_offset`] but with an explicit dialect
/// [`LexerConfig`].
///
/// The config supplies the lexer's dialect flags — `expand_syntax`
/// (`{*}` expansion, off for Tcl 8.4 / iRules) and
/// `irules_brace_separator` (`}{` ghost SEP, iRules-only).  Build it
/// with [`tcl_lexer::LexerConfig::for_dialect`].  The config's *offset*
/// fields (`base_offset` / `base_line` / `base_col`) are ignored:
/// segmentation always runs in local-offset space and relocation is
/// done here via `base_offset`, exactly as the default-config path.
#[must_use]
pub fn segment_commands_with_offset_and_config(
    source: &str,
    base_offset: u32,
    config: LexerConfig,
) -> Vec<SegmentedCommand> {
    let commands = segment_commands_local(source, config);
    if base_offset == 0 {
        return commands;
    }
    commands
        .into_iter()
        .map(|c| c.shifted_by(base_offset))
        .collect()
}

/// Segment an unchanged Unicode structural view of an original source image.
/// The same lexer/grouper/CST owners consume the retained input channel. Opaque
/// bytes or an unavailable token stream return `None`, rather than no commands.
/// Spans in the returned projection are relocated by `base_offset`.
#[must_use]
pub fn segment_commands_image_with_offset_and_config(
    image: &tcl_lexer::SourceImage,
    base_offset: u32,
    config: LexerConfig,
) -> Option<Vec<SegmentedCommand>> {
    let source = image.try_text().ok()?;
    let sm = SourceMap::from_image(image);
    let (document, _) = crate::cst::build::build_document_image(image, config)?;
    let mut segments = crate::cst::segment::segments_from_document(document, &sm);
    if config.brace_line_continuation.continues() {
        segments = merge_f5_if_else_lookahead(source, segments);
    }
    if base_offset != 0 {
        segments = segments
            .into_iter()
            .map(|segment| segment.shifted_by(base_offset))
            .collect();
    }
    Some(segments)
}

/// An authentic malformed command after a complete command prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentedParseCut {
    /// Original incomplete or terminal command, separate from executable prefix.
    pub command: SegmentedCommand,
    /// Structured syntax issue and offsets in the selected source coordinates.
    pub cut: tcl_lexer::ParseCut,
}

/// CST commands joined to the shared selected-grammar parse-cut owner.
/// This syntax projection supplies no command, frame or execution authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentedScript {
    /// All complete commands before the first malformed command.
    pub commands: Vec<SegmentedCommand>,
    /// Authentic syntax failure, distinct from unavailable source ownership.
    pub fatal_tail: Option<SegmentedParseCut>,
}

impl SegmentedScript {
    /// Keep an incomplete command for editor assistance. Terminal errors and
    /// commands after the first failure cannot become declaration or call facts.
    #[must_use]
    pub fn into_editor_commands(mut self) -> Vec<SegmentedCommand> {
        if let Some(tail) = self.fatal_tail
            && tail.cut.completeness() == tcl_lexer::ParseCutCompleteness::Incomplete
        {
            self.commands.push(tail.command);
        }
        self.commands
    }
}

/// Segment an original image at its first selected-grammar syntax failure.
///
/// The cut ordinal joins the unmerged CST, then F5 lookahead applies only to
/// the complete prefix. Raw and heuristic recovery segmentation are separate
/// projections of the same CST. Nested callers supply their actual body config.
///
/// # Errors
/// Returns unavailable original source geometry or lexical/component ownership.
pub fn segment_script_image_with_offset_and_config(
    image: &tcl_lexer::SourceImage,
    base_offset: u32,
    config: LexerConfig,
) -> Result<SegmentedScript, tcl_lexer::ParseCutUnavailable> {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    let unavailable = || {
        tcl_lexer::ParseCutUnavailable::SourceGeometry(
            tcl_lexer::ExecutablePartsUnavailable::SourceGeometry,
        )
    };
    let source = image.try_text().map_err(|_| unavailable())?;
    let source_len = u32::try_from(image.bytes().len()).map_err(|_| unavailable())?;
    base_offset
        .checked_add(source_len)
        .ok_or_else(unavailable)?;
    let cut = tcl_lexer::first_parse_cut_image_checked(image, config)?;
    let sm = SourceMap::from_image(image);
    let (document, _) =
        crate::cst::build::build_document_image(image, config).ok_or_else(unavailable)?;
    let mut commands = crate::cst::segment::segments_from_document(document, &sm);
    let fatal_tail = if let Some(mut cut) = cut {
        if cut.command >= commands.len() {
            return Err(unavailable());
        }
        let mut command = commands.remove(cut.command);
        commands.truncate(cut.command);
        command.is_partial = cut.kind.is_incomplete();
        command.partial_delimiter = match cut.kind {
            tcl_lexer::ParseIssueKind::MissingBrace => Some(UnclosedDelimiter::Brace),
            tcl_lexer::ParseIssueKind::MissingBracket => Some(UnclosedDelimiter::Bracket),
            tcl_lexer::ParseIssueKind::MissingQuote => Some(UnclosedDelimiter::Quote),
            _ => None,
        };
        cut.offset = cut
            .offset
            .checked_add(base_offset)
            .ok_or_else(unavailable)?;
        cut.term = cut.term.checked_add(base_offset).ok_or_else(unavailable)?;
        Some(SegmentedParseCut {
            command: command.shifted_by(base_offset),
            cut,
        })
    } else {
        None
    };
    if config.brace_line_continuation.continues() {
        commands = merge_f5_if_else_lookahead(source, commands);
    }
    if base_offset != 0 {
        commands = commands
            .into_iter()
            .map(|command| command.shifted_by(base_offset))
            .collect();
    }
    Ok(SegmentedScript {
        commands,
        fatal_tail,
    })
}

/// Whether `source` contains exactly one Tcl command under `config`.
///
/// This is the conservative shape gate for consumers that re-lex a command's
/// words for an optional optimisation. It delegates command boundaries to the
/// canonical CST-backed segmenter, so semicolons, newlines, comments, and
/// dialect-specific continuation rules cannot be mistaken for word separators
/// by a consumer-local token walk.
#[must_use]
pub fn has_exactly_one_command_with_config(source: &str, config: LexerConfig) -> bool {
    segment_commands_with_offset_and_config(source, 0, config).len() == 1
}

/// The longest prefix of `text` that is byte-identical to `source` starting at
/// `base` — the region over which rebasing `text`'s spans by `base` yields
/// truthful absolute offsets into `source`.
///
/// Every body walk segments a *word value* this module produced and adds a
/// single base offset to the resulting spans.  That is exact only while the
/// value is a verbatim slice of the document, which holds for an ordinary
/// braced body: `{…}`'s value is the source between the braces, backslash
/// sequences and all.
///
/// It stops holding for a **compound** word — a braced group welded to more
/// word characters, `{body}x`.  Real Tcl rejects that outright ("extra
/// characters after close-brace"); the analyser accepts it so the braced part
/// still gets diagnosed, and the word's value is then the brace content
/// concatenated with the trailing fragment, closing `}` dropped.  Rebasing
/// *that* slides every token past the dropped brace one byte left.  On ASCII
/// the result is an off-by-one span; on a multi-byte character it is an offset
/// inside a UTF-8 sequence, which panics the first consumer that slices the
/// source with it — how 40 zero-width spaces in a real iRule aborted
/// `fp-sweep` and blanked the LSP's diagnostics for the file.
///
/// Truncating to the verbatim prefix keeps the walk on the contiguous braced
/// region and drops the welded tail, which is not a script in the first place.
/// The common case returns `text` unchanged.
///
/// This corrects only what `source` can actually *prove* wrong.  When `source`
/// does not reach `base + text.len()` — an isolated body fragment analysed at
/// offset 0 on the per-item path, or a handler called directly with fabricated
/// spans in a unit test — there is nothing to compare against, so `text` is
/// returned as given rather than truncated on no evidence.  The consumer-side
/// consumer slice guards are what keep
/// that case safe.
#[must_use]
pub fn contiguous_prefix<'t>(source: &str, base: usize, text: &'t str) -> &'t str {
    let Some(rest) = source.get(base..) else {
        return text;
    };
    if rest.len() < text.len() {
        return text;
    }
    if rest.as_bytes()[..text.len()] == *text.as_bytes() {
        return text;
    }
    let mut i = 0;
    while i < text.len() && rest.as_bytes()[i] == text.as_bytes()[i] {
        i += 1;
    }
    while i > 0 && !text.is_char_boundary(i) {
        i -= 1;
    }
    &text[..i]
}

/// The original source text of a braced body whose cooked `text` is the same
/// region read through a translating Document channel. Under C Tcl's
/// `-translation auto`, `source` turns each CRLF and lone CR into LF before the
/// parser runs, so the word value is shorter than its region while every span
/// stays an original offset. Segmenting the region itself keeps those spans
/// exact: CR is a word separator before the pair's LF and a lone CR ends a
/// command exactly as the LF it reads as. A braced word's backslash-newline
/// continuations fold the same way (tclsh 8.4 to 9.0 read `{a\<LF>   b}` as
/// `a b`), and the region's continuation separates words as its space does.
/// Any other difference keeps `text`.
#[must_use]
pub fn braced_body_source_region<'t>(
    source: &'t str,
    base: usize,
    close: usize,
    text: &'t str,
) -> &'t str {
    // A braced token's span may end at its closing brace or just past it, so
    // the content may stop at `close` or one byte later; only a region that
    // reads as `text` is taken.
    [close, close + 1]
        .into_iter()
        .find_map(|close| {
            let region = source.get(base..close)?;
            if region.len() == text.len() {
                return (region == text).then_some(region);
            }
            let folded = if region.contains('\r') {
                std::borrow::Cow::Owned(region.replace("\r\n", "\n").replace('\r', "\n"))
            } else {
                std::borrow::Cow::Borrowed(region)
            };
            // The region is kept when it reads as `text` verbatim or under C's
            // continuation fold; a grammar that keeps continuations (Jim) matches
            // verbatim, and any other difference keeps `text`.
            let continuations_folded =
                // dialect-drift-ok: compared against `text`, never read as the document's grammar
                tcl_syntax::backslash::collapse_brace_continuations_str(&folded) == text;
            (folded == text || continuations_folded).then_some(region)
        })
        .unwrap_or(text)
}

/// The part of a body word's `text` whose spans a walk may rebase by `base`,
/// given that the word occupies `base..region_end` in `source`.
///
/// A body text is rebasable when it maps 1:1 onto its source region, which two
/// different constructions both achieve: an ordinary braced body, whose value
/// *is* the source between the braces, and
/// the consumer's concatenated-script window's padded window, which
/// blanks the delimiters but keeps every real word at its written offset.
/// Both fill the region exactly, so a length match is the cheap, sufficient
/// test for either — and it is what keeps the window (deliberately *not*
/// byte-identical to the source) out of [`contiguous_prefix`]'s comparison.
///
/// A word value that does not fill its region is the compound `{body}x` shape;
/// [`contiguous_prefix`] clamps it to the braced part.
#[must_use]
pub fn body_text_in_region<'t>(
    source: &str,
    base: usize,
    region_end: usize,
    text: &'t str,
) -> &'t str {
    if text.len() == region_end.saturating_sub(base) {
        return text;
    }
    contiguous_prefix(source, base, text)
}

/// Split a `{pattern body ?pattern body ...?}` clause-list body into its
/// flat, alternating sequence of pattern/body elements — the single-braced
/// form of a registry-described case-list command (`switch`'s
/// braced-list form, Expect's `expect { ... }`).
///
/// Uses the Tcl list parser to split the body.  A clause list is a list, not a
/// script: `#` and `;` are valid literal patterns, and consequently must not
/// be treated as a comment or command separator while locating its elements.
/// The returned tokens are constructed directly from the list parser's source
/// offsets, so their ordinal positions agree with
/// [`tcl_syntax::case_list::split_case_list`].
///
/// Returns `(text, token)` pairs; each token's span is rebased into the
/// outer source's offset space via `body_tok`'s `content_offset`, so a
/// caller holding `body_tok` from the outer document gets absolute spans
/// with no further arithmetic. Dynamic bodies (`body_tok.kind !=
/// TokenType::Str` — a `$var` / `[cmd]` clause list computed at runtime)
/// yield an empty list: the shape can't be statically split, so the caller
/// must fall back to whatever it does for a non-literal clause list.
///
/// `source` is the document `body_tok`'s span indexes into; the split runs
/// over [`contiguous_prefix`] of `body_text` so the rebased element spans are
/// truthful even for a compound `{…}x` clause-list word.
///
/// The representative token kind each element is given comes from re-lexing
/// that element's text, so it must be read under the grammar the document was
/// lexed with — an iRules `}{`, a Jim `$(…)` or an 8.4 `{*}` classify
/// differently otherwise, which is why `config` is required rather than
/// defaulted. Only the extracted element parser uses `config.nested()`;
/// list geometry and the enclosing original word retain their parent purpose.
#[must_use]
pub fn flatten_clause_list_elements_with_config(
    source: &str,
    body_text: &str,
    body_tok: Token,
    config: LexerConfig,
) -> Vec<(String, Token)> {
    if body_tok.kind != TokenType::Str {
        return Vec::new();
    }
    let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
    let body_text = body_text_in_region(
        source,
        base_offset as usize,
        body_tok.span.end() as usize,
        body_text,
    );
    let mut elements = Vec::new();
    let mut scan = 0;
    while let Ok(Some(element)) = tcl_syntax::list::find_element(body_text, scan) {
        let raw = &body_text[element.value.clone()];
        let text = if element.literal {
            raw.to_owned()
        } else {
            tcl_lexer::backslash_subst(raw).into_owned()
        };
        let start = element.value.start - usize::from(element.braced);
        let (Ok(start), Ok(value_start), Ok(end)) = (
            u32::try_from(base_offset as usize + start),
            u32::try_from(base_offset as usize + element.value.start),
            u32::try_from(base_offset as usize + element.value.end),
        ) else {
            return Vec::new();
        };
        let token = if element.braced {
            Token::with_content_offset(TokenType::Str, Span::new(start, end), 1)
        } else {
            // Pairing and spans come solely from the list grammar; lexing the
            // one already-located element merely preserves the representative
            // token kind (`$var`, `[command]`, or bare text) that consumers
            // use for semantic classification.
            Lexer::with_config(raw, config.nested())
                .tokenise_all()
                .ok()
                .and_then(|tokens| {
                    tokens.into_iter().find(|token| {
                        token.kind != TokenType::Eof
                            && token.span.start() == 0
                            && u32::try_from(raw.len()) == Ok(token.span.end())
                    })
                })
                .map_or_else(
                    || Token::new(TokenType::Esc, Span::new(start, end)),
                    |mut token| {
                        token.span = shift_span(token.span, value_start);
                        token
                    },
                )
        };
        elements.push((text, token));
        if element.next <= scan {
            break;
        }
        scan = element.next;
    }
    elements
}

/// Flatten a registry-described case-list into validated pattern/body pairs.
/// Clause flags and their value words are consumed by the syntax-layer grammar,
/// so Expect's per-clause options cannot shift the body token. Returned tokens
/// retain absolute source spans from the enclosing list token.
#[must_use]
pub fn flatten_case_list_clauses(
    source: &str,
    text: &str,
    tok: Token,
    shape: &tcl_syntax::case_list::CaseListShape,
    config: LexerConfig,
) -> Vec<((String, Token), (String, Token))> {
    let elements = flatten_clause_list_elements_with_config(source, text, tok, config);
    tcl_syntax::case_list::split_case_list(text, shape)
        .into_iter()
        .filter_map(|clause| {
            let (Some(pattern_index), Some(body_index)) = (clause.pattern_index, clause.body_index)
            else {
                return None;
            };
            let pattern = elements.get(pattern_index).cloned()?;
            let mut body = elements.get(body_index).cloned()?;
            // A non-braced, literal case action is still a script the
            // case-list command evaluates.  It has no substitution barrier,
            // so mark the complete list element as script-capable; this is
            // what lets `puts;unknown` reach both commands.  Dynamic or
            // backslash-built actions retain their conservative opaque kind.
            let raw_body = source
                .get(body.1.span.start() as usize..body.1.span.end() as usize)
                .unwrap_or_default();
            if body.1.kind == TokenType::Esc
                && !tcl_syntax::naming::is_dynamic_word(&body.0)
                && !raw_body.contains('\\')
            {
                body.1 = Token::with_content_offset(TokenType::Str, body.1.span, 0);
            }
            Some((pattern, body))
        })
        .collect()
}

/// Incrementally re-segment `new_text` from the segmentation of
/// `old_text` (`old_commands`), reusing the unchanged **prefix** of
/// top-level commands and re-lexing only from the first affected command
/// onward.
///
/// The edit's start is recovered as the byte length of the common prefix
/// of the two texts. `lo` is then the start of the *last old command that
/// begins at or before that point* — a clean command boundary in the
/// shared prefix region (where `old_text` and `new_text` are
/// byte-identical), so it is equally a command boundary in `new_text`.
/// Old commands that begin before `lo` are byte-identical and reused
/// verbatim; `new_text[lo..]` is re-segmented (with its spans offset by
/// `lo`) and replaces everything from `lo` on.
///
/// Because `lo` is a command boundary determined solely by the shared
/// bytes `[0, lo)`, segmenting `new_text[lo..]` in isolation is identical
/// to the `[lo..]` tail of [`segment_commands`] on the whole `new_text`,
/// so the result is byte-for-byte identical to a full re-segmentation
/// (pinned by a differential fuzz harness). The win is avoiding the
/// re-lex of the unchanged prefix; suffix reuse is intentionally *not*
/// attempted — a matching byte-suffix is not a safe split point because
/// command-boundary-ness depends on the differing bytes that precede it.
#[must_use]
pub fn segment_commands_incremental(
    old_text: &str,
    old_commands: &[SegmentedCommand],
    new_text: &str,
    config: tcl_lexer::LexerConfig,
) -> Vec<SegmentedCommand> {
    if old_text == new_text {
        return old_commands.to_vec();
    }
    if old_commands.is_empty() {
        return segment_commands_with_offset_and_config(new_text, 0, config);
    }
    let prefix = common_prefix_len(old_text, new_text);
    // `lo` = start of the last old command beginning at or before the
    // edit; everything strictly before it is byte-identical and reused.
    let lo = old_commands
        .iter()
        .map(|c| c.span.start())
        .filter(|&s| (s as usize) <= prefix)
        .max()
        .unwrap_or(0);
    let mut out: Vec<SegmentedCommand> = old_commands
        .iter()
        .filter(|c| c.span.start() < lo)
        .cloned()
        .collect();
    // `lo` is the retained whole-document command boundary. A suffix at
    // zero remains the actual file input; a nonzero suffix is an extracted
    // mid-file region, so its first BOM is content rather than a file marker.
    let suffix_config = if lo == 0 { config } else { config.nested() };
    out.extend(segment_commands_with_offset_and_config(
        &new_text[lo as usize..],
        lo,
        suffix_config,
    ));
    out
}

/// Longest common prefix of `a` and `b` in bytes, backed off to a `char`
/// boundary.
fn common_prefix_len(a: &str, b: &str) -> usize {
    let (ab, bb) = (a.as_bytes(), b.as_bytes());
    let max = ab.len().min(bb.len());
    let mut i = 0;
    while i < max && ab[i] == bb[i] {
        i += 1;
    }
    while i > 0 && !a.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Segment with error recovery.
///
/// After the raw
/// segmentation, the last command is inspected for a token that
/// looks like an unclosed delimiter (a `Str` / `Cmd` / `Esc` token
/// reaching EOF; for `Str` and `Esc` also requiring a line span of
/// at least three lines to avoid mistaking legitimate multi-line
/// strings for run-aways). If found and the suspicious token's
/// inner text contains a line whose first word is in
/// `known_commands`, the broken command is marked
/// `is_partial = true` and the source is re-segmented from the
/// recovery point; the recovered commands are appended.
///
/// Recovery is intended for top-level segmentation only — body
/// scripts (inside `{…}` blocks) should call [`segment_commands`]
/// or [`segment_commands_with_offset`] without recovery to avoid
/// false positives on legitimate multi-line braced strings.
#[must_use]
pub fn segment_commands_with_recovery<S>(
    source: &str,
    known_commands: &std::collections::HashSet<&str, S>,
) -> Vec<SegmentedCommand>
where
    S: std::hash::BuildHasher,
{
    // dialect-drift-ok: the dialect-blind entry point; a caller holding the
    // document's config uses `segment_commands_with_recovery_and_config`.
    segment_commands_with_recovery_and_config(source, known_commands, LexerConfig::default())
}

/// Like [`segment_commands_with_recovery`] but with an explicit dialect
/// [`LexerConfig`] (see [`segment_commands_with_offset_and_config`]).
/// The recovery re-segmentation threads the same config, so a recovered
/// tail lexes under the document's dialect too.
#[must_use]
pub fn segment_commands_with_recovery_and_config<S>(
    source: &str,
    known_commands: &std::collections::HashSet<&str, S>,
    config: LexerConfig,
) -> Vec<SegmentedCommand>
where
    S: std::hash::BuildHasher,
{
    let mut commands = segment_commands_local(source, config);
    let Some(last_cmd) = commands.last_mut() else {
        return commands;
    };
    let Some(suspicious_tok) = find_suspicious_token(source, &last_cmd.all_tokens) else {
        return commands;
    };
    // Strip the opening delimiter (`{` / `[` / `"`) via `content_offset`
    // so the inner text + offset arithmetic excludes it.
    let content_start =
        suspicious_tok.span.start() as usize + suspicious_tok.content_offset as usize;
    let content_end = suspicious_tok.span.end() as usize;
    let token_text = &source[content_start..content_end];
    let Some(recovery_offset) = find_recovery_offset(token_text, content_start, known_commands)
    else {
        return commands;
    };
    last_cmd.is_partial = true;
    last_cmd.partial_delimiter = UnclosedDelimiter::from_token_kind(suspicious_tok.kind);
    // Re-segment the slice starting at the recovery point. The
    // recovered slice's spans are relative to the slice; shift them
    // back into the outer source buffer's offset space.
    let remaining = &source[recovery_offset..];
    if !remaining.trim().is_empty() {
        let base = u32::try_from(recovery_offset).expect("recovery_offset fits in u32");
        // A recovery slice is mid-file, never a file head — a U+FEFF there is
        // ordinary data, so the file-entry BOM skip must not carry over.
        let config = LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Content,
            ..config
        };
        let recovered = segment_commands_with_offset_and_config(remaining, base, config);
        commands.extend(recovered);
    }
    commands
}

/// Minimum line span for a `Str` / `Esc` token to be treated as a
/// run-away unclosed delimiter rather than a legitimate multi-line
/// literal. Tuned to avoid false positives.
const RECOVERY_LINE_THRESHOLD: usize = 3;

/// Return the first token in `tokens` that looks like an unclosed
/// delimiter (`{` / `[` / `"`).
///
/// A `Cmd` token reaching EOF is always suspicious — a valid
/// `[…]` always closes — so no line-span threshold applies. `Str`
/// (`{…}`) and `Esc` (best-effort marker for unclosed `"…"` runs)
/// must also span at least [`RECOVERY_LINE_THRESHOLD`] lines.
fn find_suspicious_token(source: &str, tokens: &[Token]) -> Option<Token> {
    let source_len = source.len();
    for &tok in tokens {
        let is_brace_or_quote = matches!(tok.kind, TokenType::Str | TokenType::Esc);
        let is_bracket = matches!(tok.kind, TokenType::Cmd);
        if !is_brace_or_quote && !is_bracket {
            continue;
        }
        // Token must reach EOF — properly closed delimiters end
        // before EOF. `Span::end()` is exclusive, so a token
        // reaches EOF only when ``end == source_len``. The
        // previous ``end + 1 < source_len`` check incorrectly
        // accepted tokens ending at ``source_len - 1`` (e.g. a
        // closed multi-line ``{...}`` followed by a trailing
        // newline), which spuriously triggered recovery on
        // valid input — most visibly for ``Cmd`` tokens, which
        // have no line-span check to filter the false positive.
        let end = tok.span.end() as usize;
        if end < source_len {
            continue;
        }
        if is_bracket {
            return Some(tok);
        }
        // Brace / quote: also require line span ≥ threshold.
        let token_text = &source[tok.span.as_range()];
        let line_span = token_text.bytes().filter(|&b| b == b'\n').count();
        if line_span >= RECOVERY_LINE_THRESHOLD {
            return Some(tok);
        }
    }
    None
}

/// Find a byte offset in the original source where parsing can
/// resume after an unclosed delimiter.
///
/// Scans the inner text of a suspiciously large token line by
/// line, skipping line 0 (which is part of the broken command).
/// Returns the source offset of the first non-blank line whose
/// leading word is a known command name, or `None` if no recovery
/// point is found.
///
/// `token_text` is the content of the suspicious token with its
/// opening delimiter stripped (`{` / `[` / `"`); `content_start`
/// is the source offset of `token_text`'s first byte.
fn find_recovery_offset<S>(
    token_text: &str,
    content_start: usize,
    known_commands: &std::collections::HashSet<&str, S>,
) -> Option<usize>
where
    S: std::hash::BuildHasher,
{
    let mut inner_offset: usize = 0;
    for (i, line) in token_text.split('\n').enumerate() {
        if i == 0 {
            // Skip the first line — part of the broken command.
            inner_offset += line.len() + 1;
            continue;
        }
        let stripped = line.trim_start_matches([' ', '\t']);
        if !stripped.is_empty() {
            // First word: up to whitespace, ';', '{', or '['.
            let word_end = stripped
                .find([' ', '\t', '\n', '\r', ';', '{', '['])
                .unwrap_or(stripped.len());
            let first_word = &stripped[..word_end];
            if known_commands.contains(first_word) {
                let leading_ws = line.len() - stripped.len();
                return Some(content_start + inner_offset + leading_ws);
            }
        }
        inner_offset += line.len() + 1;
    }
    None
}

fn segment_commands_local(source: &str, config: LexerConfig) -> Vec<SegmentedCommand> {
    // `SegmentedCommand`s are derived from the canonical red-green CST
    // (`parsing::syntax`), not from a token loop of this module's own:
    // `build_document` reshapes the dialect-configured lexer stream into a
    // green tree (no second parser), and `segments_from_document` derives the
    // public `SegmentedCommand` shape from it.  `tests/differential_segment.rs`
    // holds an independent token-loop implementation and pins the two
    // byte-identical, field for field, over the edge-case table and the full
    // Tcl 8.4/8.5/8.6/9.0 corpus.  The derivation runs in local-offset space;
    // relocation stays the caller's job via `SegmentedCommand::shifted_by`.
    let sm = SourceMap::new(source);
    let (document, _warnings) = crate::cst::build::build_document(source, config);
    let segments = crate::cst::segment::segments_from_document(document, &sm);
    if config.brace_line_continuation.continues() {
        return merge_f5_if_else_lookahead(source, segments);
    }
    segments
}

/// Whether the source bytes in `gap` (exclusive command end → next command
/// start) are exactly one newline plus horizontal whitespace — the **single
/// newline** the F5 `if` lookahead reads across (N5). A blank line (a second
/// `\n`), a comment (`#`), or a `;` separator answers `false`, keeping those
/// cases diagnosed exactly as stock.
fn single_newline_gap(source: &str, gap_start: u32, gap_end: u32) -> bool {
    let Some(gap) = source.get(gap_start as usize..gap_end as usize) else {
        return false;
    };
    let mut newlines = 0usize;
    for byte in gap.bytes() {
        match byte {
            b'\n' => newlines += 1,
            b' ' | b'\t' | b'\r' => {}
            _ => return false,
        }
    }
    newlines == 1
}

/// **N5** (`docs/design/f5/bigip-irule-parser-measurements.md` §2): under the
/// F5 trunk grammar, `else` / `elseif` are a *separate* lookahead performed
/// by `if` itself — picked up across a single newline (a line starting with
/// `else` is not a `{` line, so the lexical N-rules do not cover it), but
/// **not** across a blank line, where they fall back to being an unknown
/// command (`undefined procedure: else`). Measured on TMM in a cli script
/// reproducing the parser; a trunk fact, so it rides the same
/// `brace_line_continuation` axis the N-rules ride (only the F5 trunk
/// grammar carries `Continues`).
///
/// Mirror it here by folding an `else`/`elseif`-headed command back into
/// the immediately preceding `if` command when only a single newline (plus
/// horizontal whitespace) separates them: the analyser then sees one `if`
/// command (no `undefined procedure: else` / orphaned-keyword diagnostic)
/// and lowering builds the correct CFG. The blank-line case fails
/// [`single_newline_gap`] and stays a standalone command, diagnosed exactly
/// as stock.
fn merge_f5_if_else_lookahead(
    source: &str,
    commands: Vec<SegmentedCommand>,
) -> Vec<SegmentedCommand> {
    let mut out: Vec<SegmentedCommand> = Vec::with_capacity(commands.len());
    for cmd in commands {
        let mergeable = matches!(cmd.name(), "else" | "elseif")
            && cmd
                .argv
                .first()
                .is_some_and(|tok| tok.kind == TokenType::Esc)
            && cmd.single_token_word.first().copied().unwrap_or(false)
            && out.last().is_some_and(|prev| {
                prev.name() == "if"
                    && !prev.is_partial
                    && single_newline_gap(source, prev.span.end(), cmd.span.start())
            });
        if !mergeable {
            out.push(cmd);
            continue;
        }
        let prev = out.last_mut().expect("mergeable requires a predecessor");
        // The lookahead consumes the newline as an ordinary word
        // separator, so the merged command carries a `Sep` token across
        // the gap and the `else`/`elseif` words become further `if`
        // arguments.
        prev.all_tokens.push(Token::new(
            TokenType::Sep,
            Span::new(prev.span.end(), cmd.span.start()),
        ));
        prev.span = Span::new(prev.span.start(), cmd.span.end());
        // Reconcile the optional per-word expansion markers before the
        // parallel word views are extended (F5 grammars have no `{*}`
        // expansion, so this is a formality that keeps the views aligned).
        if prev.expand_word.is_some() || cmd.expand_word.is_some() {
            let mut expand = prev
                .expand_word
                .take()
                .unwrap_or_else(|| vec![false; prev.argv.len()]);
            expand.extend(
                cmd.expand_word
                    .clone()
                    .unwrap_or_else(|| vec![false; cmd.argv.len()]),
            );
            prev.expand_word = Some(expand);
        }
        prev.argv.extend(cmd.argv);
        prev.texts.extend(cmd.texts);
        prev.word_fragments.extend(cmd.word_fragments);
        prev.single_token_word.extend(cmd.single_token_word);
        prev.all_tokens.extend(cmd.all_tokens);
        prev.is_partial |= cmd.is_partial;
        if prev.partial_delimiter.is_none() {
            prev.partial_delimiter = cmd.partial_delimiter;
        }
    }
    out
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod recovery_tests;

#[cfg(test)]
mod original_clause_element_purpose_tests;

#[cfg(test)]
mod checked_script_tests;
