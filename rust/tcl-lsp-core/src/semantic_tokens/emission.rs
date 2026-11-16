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

//! Token entries: pushing, comment tokens, delimiters and the wire encoding.

use super::SemanticTokens;
use super::legend::TokenKind;
use super::script_walk::ScriptCtx;
use crate::definition::utf16_len;
use rustc_hash::FxHashMap;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{LineIndex, Span, Token, TokenType};

/// One collected token: `(line, col, length, kind, modifiers)` with
/// absolute line/column and a token-modifier bitmask (see
/// [`legend_token_modifiers`]).
pub(super) type Entry = (u32, u32, u32, TokenKind, u32);

#[derive(Clone, Copy)]
pub(super) struct TokenPositionContext<'a> {
    pub(super) source: &'a str,
    pub(super) line_index: &'a LineIndex,
}

/// Emit the literal run `inner[run..end]` (absolute start `cstart + run`)
/// as `kind`, when non-empty.  The inter-construct filler for the
/// sub-language scanners.
pub(super) fn flush_run(
    pos: TokenPositionContext<'_>,
    cstart: usize,
    inner: &str,
    run: usize,
    end: usize,
    kind: TokenKind,
    entries: &mut Vec<Entry>,
) {
    if end > run {
        push_subtoken(
            pos.source,
            pos.line_index,
            cstart + run,
            &inner[run..end],
            kind,
            entries,
        );
    }
}

/// Push one regex sub-token at absolute byte offset `abs_off` covering
/// `text`.  Skips empty runs; a multi-line run is split into one entry per
/// covered line (see [`push_span_entries`]).
pub(super) fn push_subtoken(
    source: &str,
    line_index: &LineIndex,
    abs_off: usize,
    text: &str,
    kind: TokenKind,
    entries: &mut Vec<Entry>,
) {
    push_span_entries(source, line_index, abs_off, text, kind, 0, entries);
}

/// Emit token [`Entry`] values for `text` at absolute byte offset `abs_off`.
///
/// The LSP semantic-tokens encoding cannot represent a single token spanning
/// a newline (each token carries only a length, not an end position), so a
/// multi-line token is split into one entry per covered line, each covering
/// that line's slice of the token.  This keeps multi-line literals — braced
/// (`{…}`) or quoted (`"…"`) strings that span lines — highlighted
/// rather than dropped.  Empty per-line slices (blank lines, the trailing
/// slice after a final newline) are skipped, and the newline / `\r` bytes
/// themselves are never covered.
pub(super) fn push_span_entries(
    source: &str,
    line_index: &LineIndex,
    abs_off: usize,
    text: &str,
    kind: TokenKind,
    modifiers: u32,
    entries: &mut Vec<Entry>,
) {
    if text.is_empty() {
        return;
    }
    if !text.contains('\n') {
        let pos = line_index.position_at_utf16(u32::try_from(abs_off).unwrap_or(0), source);
        entries.push((
            pos.line,
            pos.character.get(),
            utf16_len(text),
            kind,
            modifiers,
        ));
        return;
    }
    let mut off = 0usize;
    for line in text.split_inclusive('\n') {
        let seg = line.strip_suffix('\n').unwrap_or(line);
        let seg = seg.strip_suffix('\r').unwrap_or(seg);
        if !seg.is_empty() {
            let pos =
                line_index.position_at_utf16(u32::try_from(abs_off + off).unwrap_or(0), source);
            entries.push((
                pos.line,
                pos.character.get(),
                utf16_len(seg),
                kind,
                modifiers,
            ));
        }
        off += line.len();
    }
}

/// Push a BIG-IP `object` token for `span`, unless an existing entry on
/// the same line already overlaps its column range (keeps the stream
/// overlap-free).
pub(super) fn push_object_token(
    source: &str,
    line_index: &LineIndex,
    span: tcl_lexer::Span,
    entries: &mut Vec<Entry>,
) {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    if start.line != end.line {
        return;
    }
    let len = end.character.get().saturating_sub(start.character.get());
    if len == 0 {
        return;
    }
    // An object reference is more specific than the generic bareword
    // `string` classification the (now recursive) body walk produces — drop
    // an overlapping `string` entry and emit the object token instead.  A
    // more specific overlapping kind (keyword / function / variable / …)
    // wins and suppresses the object token.
    let mut other_overlap = false;
    entries.retain(|(l, c, ln, kind, _)| {
        let overlaps = *l == start.line
            && *c < start.character.get() + len
            && start.character.get() < *c + *ln;
        if overlaps {
            if *kind == TokenKind::String {
                return false;
            }
            other_overlap = true;
        }
        true
    });
    if !other_overlap {
        entries.push((start.line, start.character.get(), len, TokenKind::Object, 0));
    }
}

/// Emit genuine original comment tokens without overlapping another colour.
pub(super) fn push_comment_tokens(
    source: &str,
    line_index: &LineIndex,
    analysis: Option<&AnalysisResult>,
    entries: &mut Vec<Entry>,
) {
    let Some(comments) = analysis.and_then(|analysis| {
        tcl_compiler::analyser::utils::script_command_comment_facts_from_analysis(source, analysis)
    }) else {
        return;
    };
    push_comment_facts(source, line_index, comments, entries);
}

/// The lexer's comments in exactly the script regions the walk entered and
/// coloured as script. A source shell has no other source of comments.
pub(super) fn push_entered_comment_tokens(
    ctx: ScriptCtx<'_>,
    entered: &[(Span, u32)],
    entries: &mut Vec<Entry>,
) {
    let image = tcl_lexer::SourceImage::document(ctx.full_source);
    let Some(mut comments) = tcl_compiler::analyser::utils::original_script_command_comment_facts(
        &image, entered, ctx.config,
    ) else {
        return;
    };
    comments.sort_by_key(|comment| comment.span.start());
    comments.dedup_by_key(|comment| comment.span.start());
    push_comment_facts(ctx.full_source, ctx.line_index, comments, entries);
}

fn push_comment_facts(
    source: &str,
    line_index: &LineIndex,
    comments: Vec<tcl_compiler::analyser::utils::ScriptCommentFact>,
    entries: &mut Vec<Entry>,
) {
    let mut covered: FxHashMap<u32, Vec<(u32, u32)>> = FxHashMap::default();
    for (line, column, length, _, _) in entries.iter() {
        covered.entry(*line).or_default().push((*column, *length));
    }
    for comment in comments {
        let pos = line_index.position_at_utf16(comment.span.start(), source);
        // This is only a rendering conflict check: the retained lexer token
        // already established command-position comment phase.
        let character = pos.character.get();
        let already_covered = covered.get(&pos.line).is_some_and(|spans| {
            spans
                .iter()
                .any(|(column, length)| *column <= character && character < *column + *length)
        });
        if !already_covered {
            push_span_entries(
                source,
                line_index,
                comment.span.start() as usize,
                &comment.text,
                TokenKind::Comment,
                0,
                entries,
            );
        }
    }
}

/// The closing delimiter a word opened at `start` expects, if it is delimited.
///
/// The lexer's span convention (documented on the `switch` case-list rebuild
/// above) is that a delimited word's `span.end()` sits **at** its closing `}` /
/// `"`, not past it — so an emitter that takes `start..end` verbatim covers
/// `opener + content` and silently drops the terminator.  Every delimited-word
/// emit path therefore has to ask for the closer back.
pub(super) fn closing_delimiter(source: &str, start: u32) -> Option<u8> {
    let bytes = source.as_bytes();
    match bytes.get(start as usize)? {
        b'"' => Some(b'"'),
        b'{' => Some(b'}'),
        // `${name}` — a braced *variable*, whose opener is two bytes.
        b'$' if bytes.get(start as usize + 1) == Some(&b'{') => Some(b'}'),
        _ => None,
    }
}

/// Extend `end` over the word's closing delimiter when the lexer left it
/// uncovered (see [`closing_delimiter`]).
///
/// Deliberately keyed on the *byte at `end`* rather than on the token kind: a
/// span that already covers its terminator (an empty `""`, whose `end` lands
/// past the closing quote) has some other byte there and is left alone, so this
/// is idempotent and cannot double-count.
pub(super) fn end_over_terminator(source: &str, start: u32, end: u32) -> u32 {
    match closing_delimiter(source, start) {
        Some(closer) if source.as_bytes().get(end as usize) == Some(&closer) => end + 1,
        _ => end,
    }
}

/// Push a single token into the entries list, computing
/// (line, column, length-in-chars, kind).
pub(super) fn push_token(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    kind: TokenKind,
    modifiers: u32,
    entries: &mut Vec<Entry>,
) {
    let span = tok.span;
    let start = span.start();
    let mut end = span.end();
    // The lexer's empty-content clamp (tcl-lexer `parse_quoted`) extends a
    // quoted `Esc` fragment's span by one byte over the `$` / `[` that
    // introduces the *next* substitution token, so `token_text` stays empty
    // while `span.end` lands on the terminator.  That introducer byte
    // belongs to the following `Var` / `Cmd` token; emitting it here would
    // produce overlapping semantic tokens (e.g. `"$x"` → the opening
    // fragment `"$` overlapping the `$x` variable).  A clamped-empty ESC is
    // recognised by `span_len == content_offset + 1` with a `$` / `[` last
    // byte; trim it back to just its leading delimiter (the opening `"`, or
    // nothing when there is no delimiter, e.g. between adjacent `$a$b`).
    if tok.kind == TokenType::Esc
        && end - start == u32::from(tok.content_offset) + 1
        && let Some(&last) = source.as_bytes().get((end - 1) as usize)
        && (last == b'$' || last == b'[')
    {
        end = start + u32::from(tok.content_offset);
    } else {
        // Cover the word's closing `}` / `"`, which the lexer's span convention
        // leaves just past `span.end()`.  Not applied to the clamped
        // fragment above: that one was trimmed *back* precisely because its span
        // ran into the next token, and re-extending it would overlap.
        end = end_over_terminator(source, start, end);
    }
    if end <= start {
        return;
    }
    let text = source.get(start as usize..end as usize).unwrap_or("");
    // The LSP encoding wants per-line entries, so a multi-line token (a braced
    // or quoted string literal spanning lines) is split into one entry per
    // line rather than dropped — see [`push_span_entries`].
    push_span_entries(
        source,
        line_index,
        start as usize,
        text,
        kind,
        modifiers,
        entries,
    );
}

/// Encode entries into the LSP packed integer stream:
/// `[deltaLine, deltaCol, length, type, modifiers]` per token.
pub(super) fn encode_entries(entries: &[Entry]) -> SemanticTokens {
    let mut data: Vec<u32> = Vec::with_capacity(entries.len() * 5);
    let mut prev_line: u32 = 0;
    let mut prev_col: u32 = 0;
    for (line, col, len, kind, modifiers) in entries {
        let delta_line = line.saturating_sub(prev_line);
        let delta_col = if delta_line == 0 {
            col.saturating_sub(prev_col)
        } else {
            *col
        };
        data.push(delta_line);
        data.push(delta_col);
        data.push(*len);
        data.push(*kind as u32);
        data.push(*modifiers);
        prev_line = *line;
        prev_col = *col;
    }
    SemanticTokens { data }
}

/// Number of packed integers per semantic token
/// (`[deltaLine, deltaCol, length, type, modifiers]`).
pub(super) const TOKEN_STRIDE: usize = 5;
