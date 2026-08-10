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

use tcl_lexer::{LexerConfig, SourceMap, Span, Token, TokenType};

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
    /// recovery segmenter from the suspicious EOF-reaching token; drives
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

/// Return the source-level text fragment for a single token.
///
/// Variables are prefixed with `$` and command substitutions are
/// wrapped in `[...]` so that the result mirrors what the user wrote.
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
                && text.contains('(')
                && text.ends_with(')')
                && (text.contains('$') || text.contains('['))
            {
                return format!("${text}");
            }
            if text.contains('}') {
                format!("${text}")
            } else {
                format!("${{{text}}}")
            }
        }
        TokenType::Cmd => format!("[{text}]"),
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
/// `fp-sweep` and blanked the LSP's diagnostics for the file (issue #1325).
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
/// slice guards ([`crate::analyser::Analyser::source_slice`]) are what keep
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

/// The part of a body word's `text` whose spans a walk may rebase by `base`,
/// given that the word occupies `base..region_end` in `source`.
///
/// A body text is rebasable when it maps 1:1 onto its source region, which two
/// different constructions both achieve: an ordinary braced body, whose value
/// *is* the source between the braces, and
/// [`crate::analyser::utils::concat_script_window`]'s padded window, which
/// blanks the delimiters but keeps every real word at its written offset.
/// Both fill the region exactly, so a length match is the cheap, sufficient
/// test for either — and it is what keeps the window (deliberately *not*
/// byte-identical to the source) out of [`contiguous_prefix`]'s comparison.
///
/// A word value that does not fill its region is the compound `{body}x` shape
/// (issue #1325); [`contiguous_prefix`] clamps it to the braced part.
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
/// form of a [`tcl_registry::CommandSpec::case_list`] command (`switch`'s
/// braced-list form, Expect's `expect { ... }`).
///
/// Uses the segmenter to split the body: every word across every
/// (pseudo-)command in the body is one element, so the result alternates
/// pattern, body, pattern, body, … in source order, whatever whitespace or
/// brace nesting separates them.
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
/// truthful even for a compound `{…}x` clause-list word (issue #1325).
#[must_use]
pub fn flatten_clause_list_elements(
    source: &str,
    body_text: &str,
    body_tok: Token,
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
    let cmds = segment_commands_with_offset(body_text, base_offset);
    let mut elements = Vec::new();
    for cmd in cmds {
        if cmd.is_partial {
            continue;
        }
        for (text, tok) in cmd.texts.iter().zip(cmd.argv.iter()) {
            elements.push((text.clone(), *tok));
        }
    }
    elements
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
) -> Vec<SegmentedCommand> {
    if old_text == new_text {
        return old_commands.to_vec();
    }
    if old_commands.is_empty() {
        return segment_commands(new_text);
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
    out.extend(segment_commands_with_offset(&new_text[lo as usize..], lo));
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

/// Segment `source` with ghost-token error recovery.
///
/// Does a first plain parse, runs the E201 unterminated-`[` heuristics
/// over it to derive zero-width ghost `]` insertions (the
/// comment / known-command / brace cases), and — when any are found —
/// re-lexes with those ghosts so a swallowed following command
/// (`set x [foo bar` then `puts done`) splits into a clean
/// `[foo bar]` + `puts done` stream.  Returns the (possibly re-lexed)
/// commands **and** the E201 diagnostics for each applied ghost; an
/// empty diagnostic list means no recovery was applied and the commands
/// are exactly [`segment_commands_local`] (the caller then
/// keeps its own scan-to-next stream).
///
/// `known` is the "known command" name universe the E201 heuristics
/// consult (see `analyser::utils::recovery_known_commands`) — the active
/// registry's names plus every proc/class/alias the document itself
/// defines, so a break just before a call to a user-defined proc recovers
/// as readily as one before a builtin.
///
/// The E204-E206 lexer-warning
/// codes are emitted separately by the analyser.
#[must_use]
pub fn segment_with_recovery(
    source: &str,
    config: LexerConfig,
    known: &crate::analyser::utils::RecoveryKnownCommands,
) -> (
    Vec<SegmentedCommand>,
    Vec<crate::analyser::types::Diagnostic>,
) {
    // Cap the re-lex iterations to bound work on pathological input.
    const MAX_GHOST_RECOVERY_PASSES: usize = 32;

    let commands = segment_commands_local(source, config);
    if commands.is_empty() {
        return (commands, Vec::new());
    }
    // Accumulated zero-width ghost `]` insertions (keyed by original-source
    // offset — ghosts don't shift later offsets, so the map stays valid
    // across re-lexes).  `diag_by_bracket` keeps one E201 per *bracket*
    // (keyed on its `[` offset), preferring a fix-bearing diagnostic over a
    // fix-less fallback so a bracket recovered by a ghost reports its
    // insertion fix, not the bare fallback the re-lexed stream would yield.
    let mut ghosts: std::collections::BTreeMap<u32, u8> = std::collections::BTreeMap::new();
    let mut diag_by_bracket: std::collections::BTreeMap<u32, crate::analyser::types::Diagnostic> =
        std::collections::BTreeMap::new();
    let sm = SourceMap::new(source);
    let mut current = commands;
    // Iterate: a single re-lex can expose a *further* unterminated `[` that
    // the previous parse had swallowed (`set a [foo` / `set c [bar` /
    // `proc …`).  Re-derive ghosts from the re-lexed stream and re-lex
    // again until a pass adds no new ghost.
    for _ in 0..MAX_GHOST_RECOVERY_PASSES {
        let mut new_ghost = false;
        for cmd in &current {
            for diag in
                crate::analyser::syntax_checks::unterminated_bracket_diagnostics(cmd, source, known)
            {
                let bracket_off = diag.span.start();
                // A heuristic case carries a `]`-insertion fix whose offset
                // is the ghost offset; a bare fallback has no fix and stays
                // unterminated after the re-lex.
                if let Some(fix) = diag.fixes.first()
                    && let std::collections::btree_map::Entry::Vacant(e) =
                        ghosts.entry(fix.span.start())
                {
                    e.insert(b']');
                    new_ghost = true;
                }
                // Keep a fix-bearing diagnostic once recorded; otherwise take
                // whatever (fix-bearing or fallback) we see first.
                let keep_existing = diag_by_bracket
                    .get(&bracket_off)
                    .is_some_and(|d| !d.fixes.is_empty());
                if !keep_existing {
                    diag_by_bracket.insert(bracket_off, diag);
                }
            }
        }
        if !new_ghost {
            break;
        }
        let (document, _warnings) = crate::parsing::syntax::build::build_document_with_ghosts(
            source,
            config,
            ghosts.clone(),
        );
        current = crate::parsing::syntax::segment::segments_from_document(document, &sm);
    }
    if ghosts.is_empty() {
        // No heuristic insertion → no re-lex.  The caller keeps its
        // scan-to-next stream and its own (fallback) E201 detector.
        return (current, Vec::new());
    }
    let diagnostics: Vec<crate::analyser::types::Diagnostic> =
        diag_by_bracket.into_values().collect();
    (current, diagnostics)
}

fn segment_commands_local(source: &str, config: LexerConfig) -> Vec<SegmentedCommand> {
    // The segmenter now derives its `SegmentedCommand`s from the canonical
    // red-green CST (`parsing::syntax`) rather than its own token loop —
    // the 150-line `SegmenterState` accumulator and `flush_eol_or_eof` are
    // gone.  `build_document` reshapes the
    // dialect-configured lexer stream into a green tree (no second parser),
    // and `segments_from_document` derives the public `SegmentedCommand`
    // shape from it.  Verified byte-identical, field for field, against a
    // **frozen copy of the former token loop** (preserved as the
    // independent oracle in `tests/differential_segment.rs`) over the
    // edge-case table + the full Tcl 8.4/8.5/8.6/9.0 corpus.  The
    // derivation runs in local-offset space; relocation stays the caller's
    // job via `SegmentedCommand::shifted_by`.
    let sm = SourceMap::new(source);
    let (document, _warnings) = crate::parsing::syntax::build::build_document(source, config);
    crate::parsing::syntax::segment::segments_from_document(document, &sm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_core_types::DiagCode;

    /// `(start, end, texts, is_partial, token-spans)` — the fields that
    /// define a command's identity for incremental-vs-full comparison.
    type CmdProjection = (u32, u32, Vec<String>, bool, Vec<(u32, u32)>);

    fn project(c: &SegmentedCommand) -> CmdProjection {
        (
            c.span.start(),
            c.span.end(),
            c.texts.clone(),
            c.is_partial,
            c.all_tokens
                .iter()
                .map(|t| (t.span.start(), t.span.end()))
                .collect(),
        )
    }

    fn projected(cmds: &[SegmentedCommand]) -> Vec<CmdProjection> {
        cmds.iter().map(project).collect()
    }

    struct Lcg(u64);
    impl Lcg {
        fn next_u32(&mut self) -> u32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            (self.0 >> 33) as u32
        }
    }

    #[test]
    fn segment_commands_incremental_pins() {
        // Edit inside one command.
        let old = "set x 1\nputs hi\nset y 2\n";
        let new = "set x 1\nputs bye\nset y 2\n";
        assert_eq!(
            projected(&segment_commands_incremental(
                old,
                &segment_commands(old),
                new
            )),
            projected(&segment_commands(new)),
        );
        // Insert a whole new command.
        let new2 = "set x 1\nputs hi\nincr n\nset y 2\n";
        assert_eq!(
            projected(&segment_commands_incremental(
                old,
                &segment_commands(old),
                new2
            )),
            projected(&segment_commands(new2)),
        );
        // Delete a command.
        let new3 = "set x 1\nset y 2\n";
        assert_eq!(
            projected(&segment_commands_incremental(
                old,
                &segment_commands(old),
                new3
            )),
            projected(&segment_commands(new3)),
        );
        // Append at the end.
        let new4 = "set x 1\nputs hi\nset y 2\nputs done\n";
        assert_eq!(
            projected(&segment_commands_incremental(
                old,
                &segment_commands(old),
                new4
            )),
            projected(&segment_commands(new4)),
        );
    }

    #[test]
    fn segment_commands_incremental_matches_full_under_fuzz() {
        // Differential acceptance gate: a random edit applied to a base
        // document, then incremental re-segmentation must byte-for-byte
        // match a full re-segmentation of the edited text.
        let bases = [
            "set x 1\nputs hi\nset y 2\nproc p {} { return 1 }\n",
            "if {$a} {\n  puts a\n} else {\n  puts b\n}\nputs done\n",
            "namespace eval n {\n  variable v 1\n  proc q {} {}\n}\nn::q\n",
            "set s {a\nb\nc}\nputs $s\nset t \"x;y\"\n",
            "# comment\nset x [expr {1 + 2}]\nputs $x\n",
        ];
        let inserts = [
            "", "z", "puts Z\n", "}", "{", "\n", "[a]", "; ", "\"q\"", "\\",
        ];
        let mut rng = Lcg(0xC0FF_EE12_3456_789A);
        let mut checked = 0usize;
        for base in bases {
            let old_cmds = segment_commands(base);
            let blen = base.len();
            for _ in 0..600 {
                // Random byte range [s, e) on char boundaries.
                let mut s = (rng.next_u32() as usize) % (blen + 1);
                while !base.is_char_boundary(s) {
                    s -= 1;
                }
                let mut e = s + (rng.next_u32() as usize) % (blen + 1 - s);
                while !base.is_char_boundary(e) {
                    e += 1;
                }
                let ins = inserts[(rng.next_u32() as usize) % inserts.len()];
                let new = format!("{}{}{}", &base[..s], ins, &base[e..]);
                let inc = segment_commands_incremental(base, &old_cmds, &new);
                let full = segment_commands(&new);
                assert_eq!(
                    projected(&inc),
                    projected(&full),
                    "incremental != full for base {base:?} edit [{s},{e}) ins {ins:?} -> {new:?}",
                );
                checked += 1;
            }
        }
        assert!(checked > 2000, "fuzz corpus too small: {checked}");
    }

    /// Cross-check the cheap incremental-reparse boundary scanner
    /// (`tcl_lexer::command_boundaries`) against the production
    /// segmenter: no segmented command may straddle a top-level
    /// boundary, so the cheap byte-scan agrees with the full tokeniser
    /// on where commands split.
    #[test]
    fn command_boundaries_agree_with_segmenter() {
        let cases = [
            "set x 1\nputs hi\n",
            "if {1} {\n  puts a\n  puts b\n}\nputs done\n",
            "set y [a; b]\nputs \"a;b\"\n",
            "proc p {} {\n  return [expr {1 + 2}]\n}\np\n",
            "namespace eval n {\n  variable v 1\n}\n",
            "a; b; c\nd\n",
            "set s {a\nb\nc}\nputs $s\n",
        ];
        for src in cases {
            let bounds = tcl_lexer::command_boundaries(src);
            for cmd in segment_commands(src) {
                if cmd.argv.is_empty() {
                    continue;
                }
                let (s, e) = (cmd.span.start(), cmd.span.end());
                // The command must lie within a single boundary interval
                // [prev, next): no boundary may fall strictly inside it.
                let straddles = bounds.iter().any(|&b| b > s && b < e);
                assert!(
                    !straddles,
                    "command {s}..{e} straddles a boundary in {bounds:?} for {src:?}",
                );
            }
        }
    }

    #[test]
    fn empty_source() {
        assert!(segment_commands("").is_empty());
    }

    #[test]
    fn recovery_splits_swallowed_following_command() {
        // An unterminated `[` whose next line is a
        // known command re-lexes into a clean two-command stream and
        // yields an E201 diagnostic.
        let reg = tcl_registry::CommandRegistry::build_default();
        let known: crate::analyser::utils::RecoveryKnownCommands =
            reg.command_names().map(str::to_owned).collect();
        let cfg = LexerConfig::default();
        let (rec, diags) = segment_with_recovery("set x [foo bar\nputs done\n", cfg, &known);
        assert_eq!(rec.len(), 2);
        assert_eq!(rec[0].texts, vec!["set", "x", "[foo bar]"]);
        assert_eq!(rec[1].texts, vec!["puts", "done"]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagCode::E201);
    }

    #[test]
    fn recovery_is_a_noop_on_clean_input() {
        let reg = tcl_registry::CommandRegistry::build_default();
        let known: crate::analyser::utils::RecoveryKnownCommands =
            reg.command_names().map(str::to_owned).collect();
        let cfg = LexerConfig::default();
        let src = "set ok [foo]\nputs hi\n";
        let (rec, diags) = segment_with_recovery(src, cfg, &known);
        let plain = segment_commands(src);
        // No recoverable imbalance → byte-identical words, no diagnostics.
        let words =
            |cs: &[SegmentedCommand]| cs.iter().map(|c| c.texts.clone()).collect::<Vec<_>>();
        assert_eq!(words(&rec), words(&plain));
        assert!(diags.is_empty());
    }

    #[test]
    fn config_variant_threads_dialect_expand_syntax() {
        // The `_and_config` variants let
        // the analyser segment under a document's dialect.  `{*}$x` is the
        // expansion operator on 8.5+ (the default) but a literal brace
        // word on 8.4 / iRules, so the dialect flag must reach the lexer.
        // 8.5+ default → word 1 is an expanded `$x`.
        let on = segment_commands_with_offset_and_config("cmd {*}$x", 0, LexerConfig::default());
        assert_eq!(on.len(), 1);
        assert_eq!(on[0].expand_word, Some(vec![false, true]));
        // 8.4 → `{*}` is literal, so the word is an ordinary composite and
        // no expansion is recorded.
        let off = segment_commands_with_offset_and_config(
            "cmd {*}$x",
            0,
            LexerConfig::for_dialect("tcl8.4"),
        );
        assert_eq!(off.len(), 1);
        assert!(off[0].expand_word.is_none());
        // The recovery variant threads the same config to both the initial
        // pass and any recovered tail.
        let kc: std::collections::HashSet<&str> = ["cmd"].into_iter().collect();
        let rec = segment_commands_with_recovery_and_config(
            "cmd {*}$x",
            &kc,
            LexerConfig::for_dialect("tcl8.4"),
        );
        assert!(rec[0].expand_word.is_none());
    }

    #[test]
    fn config_variant_threads_irules_brace_separator() {
        // The other dialect flag carried by
        // `LexerConfig::for_dialect` — iRules injects a zero-width SEP at a
        // `}{` brace boundary, so `cmd {a}{b}` is three words under
        // `f5-irules` but a two-word command (`{a}{b}` is one composite
        // word) under the vanilla default.
        let irules = segment_commands_with_offset_and_config(
            "cmd {a}{b}",
            0,
            LexerConfig::for_dialect("f5-irules"),
        );
        assert_eq!(irules.len(), 1);
        assert_eq!(
            irules[0].argv.len(),
            3,
            "iRules `}}{{` splits into two words: {:?}",
            irules[0].texts,
        );
        let vanilla =
            segment_commands_with_offset_and_config("cmd {a}{b}", 0, LexerConfig::default());
        assert_eq!(
            vanilla[0].argv.len(),
            2,
            "vanilla keeps `{{a}}{{b}}` as one composite word: {:?}",
            vanilla[0].texts,
        );
    }

    #[test]
    fn single_command() {
        let cmds = segment_commands("puts hello");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name(), "puts");
        assert_eq!(cmds[0].args(), &["hello"]);
    }

    #[test]
    fn two_commands() {
        let cmds = segment_commands("set x 1\nputs $x");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].name(), "set");
        assert_eq!(cmds[0].texts.len(), 3); // set, x, 1
        assert_eq!(cmds[1].name(), "puts");
    }

    #[test]
    fn semicolon_separator() {
        let cmds = segment_commands("set x 1; set y 2");
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn variable_word() {
        let cmds = segment_commands("puts $name");
        assert_eq!(cmds.len(), 1);
        // Variable should be wrapped as ${name}.
        assert_eq!(cmds[0].texts[1], "${name}");
    }

    #[test]
    fn array_with_literal_index_braces_canonically() {
        // Literal-index bare `$arr(idx)` normalises to `${arr(idx)}`
        // so the bytecode codegen can emit array loads.
        let cmds = segment_commands("puts $arr(idx)");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].texts[1], "${arr(idx)}");
    }

    #[test]
    fn array_with_dollar_substituted_index_round_trips_bare() {
        // Bare `$arr($idx)` with a `$`-substituted index must NOT be
        // braced — wrapping in `${...}` would disable array-element
        // interpretation and turn the recursive substitution into a
        // literal scalar lookup (cmdAH-1.4 / 1.5 idiom).
        let cmds = segment_commands("puts $numargErrors($cmd)");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].texts[1], "$numargErrors($cmd)");
    }

    #[test]
    fn array_with_cmd_substituted_index_round_trips_bare() {
        // Same as above but with a `[…]` command substitution in the
        // index — must also stay bare.
        let cmds = segment_commands("puts $arr([f x])");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].texts[1], "$arr([f x])");
    }

    #[test]
    fn braced_var_form_preserved_when_explicit() {
        // Explicit `${arr(idx)}` keeps the braced form since
        // `content_offset == 2` marks the token as braced.
        let cmds = segment_commands("puts ${arr(idx)}");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].texts[1], "${arr(idx)}");
    }

    #[test]
    fn command_substitution() {
        let cmds = segment_commands("puts [expr 1+2]");
        assert_eq!(cmds.len(), 1);
        // Command substitution wrapped in brackets.
        assert!(cmds[0].texts[1].starts_with('['));
        assert!(cmds[0].texts[1].ends_with(']'));
    }

    #[test]
    fn braced_string() {
        let cmds = segment_commands("if {$x > 0} {puts yes}");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name(), "if");
        assert_eq!(cmds[0].texts.len(), 3);
    }

    #[test]
    fn single_token_tracking() {
        let cmds = segment_commands("set x {hello world}");
        assert_eq!(cmds.len(), 1);
        // "set" is single, "x" is single, "{hello world}" is single.
        assert!(cmds[0].single_token_word.iter().all(|&s| s));
    }

    #[test]
    fn multi_token_word() {
        let cmds = segment_commands("puts $a$b");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].texts.len(), 2); // "puts", "${a}${b}"
        assert!(!cmds[0].single_token_word[1]); // multi-token word
    }

    #[test]
    fn multi_token_word_argv_spans_full_word() {
        // For a multi-token word the representative argv token's
        // span must cover the whole reconstructed word, not just
        // the first sub-token.
        let src = "source $script_dir/init.tcl";
        let cmds = segment_commands(src);
        assert_eq!(cmds.len(), 1);
        let arg_span = cmds[0].argv[1].span;
        assert_eq!(
            &src[arg_span.as_range()],
            "$script_dir/init.tcl",
            "argv[1].span must cover the whole multi-token word",
        );
        assert!(!cmds[0].single_token_word[1]);
    }

    #[test]
    fn comment_ignored() {
        let cmds = segment_commands("# this is a comment\nputs hello");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name(), "puts");
    }

    #[test]
    fn arg_tokens_and_arg_single() {
        let cmds = segment_commands("set x 1");
        assert_eq!(cmds[0].arg_tokens().len(), 2); // x, 1
        assert_eq!(cmds[0].arg_single_token().len(), 2);
    }

    #[test]
    fn blank_lines_between_commands() {
        let cmds = segment_commands("set x 1\n\nset y 2");
        assert_eq!(cmds.len(), 2);
    }

    // -- whole-command range covers the last
    // word's closing delimiter

    #[test]
    fn command_span_includes_trailing_brace() {
        // `if {$x} {body}` — the final word is a braced body whose
        // STR token stops on the `}`. The whole-command span must
        // reach past the closer so consumers don't drop it.
        let src = "if {$x} {body}";
        let cmds = segment_commands(src);
        assert_eq!(cmds.len(), 1);
        assert_eq!(&src[cmds[0].span.as_range()], "if {$x} {body}");
    }

    #[test]
    fn command_span_includes_trailing_bracket() {
        // Final word is a command substitution `[...]`; the CMD
        // token stops on `]`, so the span must include it.
        let src = "set x [expr 1+2]";
        let cmds = segment_commands(src);
        assert_eq!(cmds.len(), 1);
        assert_eq!(&src[cmds[0].span.as_range()], "set x [expr 1+2]");
    }

    #[test]
    fn command_span_includes_multiline_closing_brace() {
        // Multi-line braced body — the closer is on its own line.
        let src = "proc f {} {\n  set x 1\n}";
        let cmds = segment_commands(src);
        assert_eq!(cmds.len(), 1);
        assert!(
            src[cmds[0].span.as_range()].ends_with('}'),
            "span text: {:?}",
            &src[cmds[0].span.as_range()],
        );
    }

    #[test]
    fn command_span_unaffected_by_degenerate_empty_brace() {
        // Degenerate `{}` already covers its closer in the token
        // span (the lexer extends it by one), so widening must not
        // over-reach past the `}`.
        let src = "proc f {}";
        let cmds = segment_commands(src);
        assert_eq!(cmds.len(), 1);
        assert_eq!(&src[cmds[0].span.as_range()], "proc f {}");
    }

    #[test]
    fn widen_word_end_does_not_overshoot_empty_brace_before_enclosing_closer() {
        // The empty-`{}` case made reachable directly: an empty
        // `{}` as the final word of a command, with an enclosing `}`
        // immediately after it in the *same* buffer. `command_span` must
        // stop at the empty brace's own closer, never the enclosing one.
        //
        // The old byte-check `source[span.end()] == '}'` over-reached here
        // (the byte one past the empty `{}` span *is* the enclosing `}`);
        // the faithful text-empty predicate does not. The public
        // segmenter lexes `a {}}` as `a`, `{}`, `}` (the stray `}` is its
        // own word), so the overshoot is constructed at the token level.
        let src = "a {}}";
        let sm = SourceMap::new(src);
        let toks = tcl_lexer::Lexer::new(src).tokenise_all().unwrap();
        let words: Vec<Token> = toks
            .iter()
            .copied()
            .filter(|t| !matches!(t.kind, TokenType::Sep | TokenType::Eol | TokenType::Eof))
            .collect();
        // words == [`a`, `{}`, `}`]; drop the trailing stray `}` so the
        // empty `{}` is the command's final word.
        assert_eq!(words[1].kind, TokenType::Str);
        assert_eq!(words[1].span.end(), 4);
        let span = command_span(&words[..2], &sm);
        assert_eq!(&src[span.as_range()], "a {}");
        assert_eq!(span.end(), 4, "must not swallow the enclosing `}}`");
        // The lone widen helper agrees.
        assert_eq!(widen_word_end(words[1], &sm), 4);
    }

    #[test]
    fn command_span_unaffected_for_plain_last_word() {
        // A bare-word final argument has no closer to widen.
        let src = "set x 1";
        let cmds = segment_commands(src);
        assert_eq!(cmds.len(), 1);
        assert_eq!(&src[cmds[0].span.as_range()], "set x 1");
    }
}

#[cfg(test)]
mod recovery_tests {
    //! Tests for [`segment_commands_with_recovery`].
    //!
    //! When an unclosed `{` / `[` / `"` causes the lexer to
    //! consume the rest of the file as one giant token, the
    //! segmenter scans the inner text for a line whose first word
    //! is a known command, marks the broken command `is_partial`,
    //! and re-segments from the recovery point so later commands
    //! still surface to consumers (workspace index, signature
    //! scan).

    use super::*;
    use std::collections::HashSet;

    fn known<const N: usize>(names: [&'static str; N]) -> HashSet<&'static str> {
        HashSet::from(names)
    }

    #[test]
    fn unclosed_brace_recovers_at_known_command() {
        // Recovery from an unclosed brace scans forward for the next
        // line whose first word is in
        // the known-commands set. Without recovery the
        // segmenter would yield a single command — `proc early` —
        // whose body STR token swallows everything to EOF.
        let src = "proc early {} {\n    # missing close brace\n\nproc late {} {}\n";
        let cmds = segment_commands_with_recovery(src, &known(["proc"]));
        assert_eq!(cmds.len(), 2, "expected partial + recovered, got {cmds:?}");
        assert!(cmds[0].is_partial, "first command should be marked partial");
        assert_eq!(cmds[1].name(), "proc");
        // Verify the recovered command's argv span points at the
        // *outer* source buffer's offset of `proc late`, not the
        // local-to-slice offset.
        let argv0 = cmds[1].argv[0];
        assert_eq!(&src[argv0.span.as_range()], "proc");
    }

    #[test]
    fn recovery_records_partial_delimiter() {
        // Unclosed brace → BRACE; the recovered partial carries the
        // precise delimiter for the E200 message + stolen-brace gate.
        let src = "proc early {} {\n    # missing close brace\n\nproc late {} {}\n";
        let cmds = segment_commands_with_recovery(src, &known(["proc"]));
        assert_eq!(cmds[0].partial_delimiter, Some(UnclosedDelimiter::Brace));
        // Unclosed bracket → BRACKET (no line-span threshold for `[`).
        let src = "set x [foo\nputs done\n";
        let cmds = segment_commands_with_recovery(src, &known(["set", "puts"]));
        assert!(cmds[0].is_partial);
        assert_eq!(cmds[0].partial_delimiter, Some(UnclosedDelimiter::Bracket));
        // A complete command records no partial delimiter.
        let cmds = segment_commands_with_recovery("set x 1\n", &known(["set"]));
        assert_eq!(cmds[0].partial_delimiter, None);
    }

    #[test]
    fn unclosed_brace_without_known_command_keeps_swallowed_input() {
        // No recovery point reachable — the broken command stays
        // as the only command and is *not* marked partial (recovery
        // never fires).
        let src = "proc early {} {\n    line one\n    line two\n";
        let cmds = segment_commands_with_recovery(src, &known(["proc"]));
        assert_eq!(cmds.len(), 1);
        assert!(!cmds[0].is_partial);
    }

    #[test]
    fn unclosed_brace_below_line_threshold_skipped() {
        // Multi-line braced literals shorter than the line
        // threshold (3 lines) are NOT treated as suspicious — they
        // are likely legitimate string content.
        let src = "set x {\n  line two\n";
        let cmds = segment_commands_with_recovery(src, &known(["proc", "set"]));
        // Recovery does not fire; we still get the one (over-large)
        // command without a partial split.
        assert_eq!(cmds.len(), 1);
        assert!(!cmds[0].is_partial);
    }

    #[test]
    fn unclosed_bracket_recovers_without_line_threshold() {
        // Unclosed `[` is always suspicious (no line span check) —
        // a valid `[…]` always closes. Note: braced empty bodies
        // `{}` round-trip through `word_piece` as empty strings
        // because `SourceMap::token_text` strips the opening `{`,
        // leaving the brace's content (empty) behind.
        let src = "set x [foo\nproc late {} {}\n";
        let cmds = segment_commands_with_recovery(src, &known(["proc"]));
        assert!(
            cmds.len() >= 2,
            "expected partial + recovered, got {cmds:?}"
        );
        let recovered = cmds.last().unwrap();
        assert_eq!(recovered.name(), "proc");
        assert_eq!(recovered.args(), ["late", "", ""]);
    }

    #[test]
    fn empty_known_commands_set_never_recovers() {
        let src = "proc early {} {\n    # missing close brace\n\nproc late {} {}\n";
        let cmds = segment_commands_with_recovery(src, &HashSet::new());
        assert_eq!(cmds.len(), 1);
        assert!(!cmds[0].is_partial);
    }

    #[test]
    fn closed_input_is_unaffected_by_recovery() {
        // Recovery must not perturb well-formed inputs — same
        // commands as `segment_commands(src)` would produce.
        let src = "proc foo {} {}\nproc bar {} {}";
        let kc = known(["proc"]);
        let recovered = segment_commands_with_recovery(src, &kc);
        let raw = segment_commands(src);
        assert_eq!(recovered.len(), raw.len());
        for (r, p) in recovered.iter().zip(raw.iter()) {
            assert_eq!(r.name(), p.name());
            assert_eq!(r.args(), p.args());
            assert_eq!(r.is_partial, p.is_partial);
        }
    }

    #[test]
    fn closed_command_sub_followed_by_trailing_newline_not_suspicious() {
        // Regression for the EOF off-by-one: ``Span::end()`` is
        // exclusive, so a closed ``[cmd]`` whose ``]`` sits at
        // ``source_len - 2`` (followed by a single ``\n``) ends
        // at ``source_len - 1``. The previous ``end + 1 < source_len``
        // check incorrectly accepted such tokens as "reaches EOF",
        // which spuriously triggered recovery on ``Cmd`` tokens
        // (no line-span check) when their inner text contained a
        // known command name like ``proc``.
        //
        // After the fix: only ``end == source_len`` qualifies as
        // EOF, so this input segments cleanly without recovery.
        let src = "set x [proc foo {} {}]\n";
        let kc = known(["proc"]);
        let cmds = segment_commands_with_recovery(src, &kc);
        // Should be exactly one command (`set x [...]`); no
        // partial split, no spurious extra recovered commands.
        assert_eq!(cmds.len(), 1, "expected one command, got {cmds:?}");
        assert_eq!(cmds[0].name(), "set");
        assert!(!cmds[0].is_partial);
    }

    #[test]
    fn closed_multiline_brace_followed_by_trailing_newline_not_suspicious() {
        // Same regression but for the ``Str`` (brace) path. A
        // closed multi-line braced string spanning more than the
        // ``RECOVERY_LINE_THRESHOLD`` lines, with a trailing
        // newline, must not be treated as suspicious.
        let src = "set x {\n  line1\n  line2\n  line3\n}\n";
        let kc = known(["set"]);
        let cmds = segment_commands_with_recovery(src, &kc);
        assert_eq!(cmds.len(), 1, "expected one command, got {cmds:?}");
        assert_eq!(cmds[0].name(), "set");
        assert!(!cmds[0].is_partial);
    }

    #[test]
    fn recovery_picks_first_matching_line_not_later_ones() {
        // Two `set` commands appear in the broken body. The
        // segmenter must recover at the first match — the line
        // starting with `set y 1` — not skip ahead to the later
        // `set z 2`. The recovered slice is then re-segmented on
        // its own, picking up `foo bar` and the second `set` too.
        let src = "proc early {} {\n    set y 1\nfoo bar\nset z 2\n";
        let cmds = segment_commands_with_recovery(src, &known(["set"]));
        assert!(cmds.len() >= 2);
        assert!(cmds[0].is_partial);
        // First recovered command points at `set y 1`, not `set z 2`.
        assert_eq!(cmds[1].name(), "set");
        assert_eq!(cmds[1].args(), ["y", "1"]);
        // The later `set z 2` command is also reachable in the
        // recovered slice.
        let last_set = cmds
            .iter()
            .rev()
            .find(|c| c.name() == "set")
            .expect("at least one set");
        assert_eq!(last_set.args(), ["z", "2"]);
    }

    #[test]
    fn recovery_skips_indented_lines_and_finds_unindented_match() {
        // Recovery scans for the first line whose first word is
        // known, regardless of indentation — the line lookup
        // strips leading whitespace before the word match.
        let src = "proc early {} {\n    not_known foo\n    proc indented {} {}\n";
        let cmds = segment_commands_with_recovery(src, &known(["proc"]));
        assert_eq!(cmds.len(), 2);
        assert!(cmds[0].is_partial);
        assert_eq!(cmds[1].name(), "proc");
        // `{}` empty bodies become empty strings in `texts` (the
        // segmenter strips the opening delimiter before emitting
        // the word piece).
        assert_eq!(cmds[1].args(), ["indented", "", ""]);
    }

    #[test]
    fn recovery_from_offset_into_source_uses_absolute_offsets() {
        // The recovered slice must use absolute (outer-source)
        // offsets so downstream consumers (signature_scan, the
        // workspace index) can slice the recovered token text from
        // the original buffer without further bookkeeping.
        let src = "proc early {} {\n    # broken\n\nproc late {} {}\n";
        let cmds = segment_commands_with_recovery(src, &known(["proc"]));
        let late = cmds.iter().find(|c| !c.is_partial).expect("recovered cmd");
        let argv0 = late.argv[0];
        assert_eq!(&src[argv0.span.as_range()], "proc");
        // Confirm the recovered command's argv tokens span the
        // outer-source bytes (the `late` name token text matches
        // the source byte range).
        let name_tok = late.argv[1];
        assert_eq!(&src[name_tok.span.as_range()], "late");
    }
}

#[cfg(test)]
mod span_absolute_tests {
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    #[test]
    fn proc_body_statement_spans_are_absolute() {
        let src = "proc ::f {} { set x 1; return $x }";
        let r = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &r, false);
        let proc = cu.ir_module.procedures.get("::f").expect("proc");
        let first = proc.body.statements.first().expect("body stmt");
        let span = first.span();
        let text = &src[span.as_range()];
        assert!(
            text.starts_with("set"),
            "expected absolute span pointing at `set`, got {text:?}",
        );
    }
}
