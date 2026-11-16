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

//! Sub-tokens inside `binary`, `clock` and `format` specifications and backslash escapes.

use super::argument_override::subspec_content;
use super::emission::{Entry, TokenPositionContext, closing_delimiter, flush_run, push_subtoken};
use super::legend::TokenKind;
use tcl_lexer::{LineIndex, Token, TokenType};

/// Sub-tokenise a `binary format`/`scan` field string into its
/// specifiers: digit runs → `BinaryCount`, specifier letters →
/// `BinarySpec`, a `u` suffix after any specifier (Tcl 8.5+)
/// or a trailing `*` → `BinaryFlag`.  Whitespace and unrecognised
/// characters are skipped.  Returns `false` when nothing was emitted.
pub(super) fn push_binary_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    dialect: &'static tcl_dialect::DialectProfile,
    entries: &mut Vec<Entry>,
) -> bool {
    if !matches!(tok.kind, TokenType::Str | TokenType::Esc) {
        return false;
    }
    let cstart = tok.span.start() as usize + tok.content_offset as usize;
    let cend = (tok.span.end() as usize).min(source.len());
    let Some(inner) = source.get(cstart..cend) else {
        return false;
    };
    let profile = dialect;
    let allow_mod = tcl_cmd_core::binary::signedness_available(profile);
    let mut emitted = false;
    for spec in tcl_cmd_core::binary::specifiers(inner.as_bytes(), allow_mod) {
        push_subtoken(
            source,
            line_index,
            cstart + spec.letter_start,
            &inner[spec.letter_start..=spec.letter_start],
            TokenKind::BinarySpec,
            entries,
        );
        emitted = true;
        let count_start = spec
            .letter_start
            .saturating_add(1)
            .saturating_add(usize::from(spec.modifier.is_some()));
        if count_start > spec.letter_start + 1 {
            push_subtoken(
                source,
                line_index,
                cstart + spec.letter_start + 1,
                &inner[spec.letter_start + 1..count_start],
                TokenKind::BinaryFlag,
                entries,
            );
            emitted = true;
        }
        if count_start < spec.end {
            push_subtoken(
                source,
                line_index,
                cstart + count_start,
                &inner[count_start..spec.end],
                if spec.star {
                    TokenKind::BinaryFlag
                } else {
                    TokenKind::BinaryCount
                },
                entries,
            );
            emitted = true;
        }
    }
    emitted
}

/// Sub-tokenise a `clock format`/`scan` field string into its `%`
/// specifiers (`ClockPercent` + optional `ClockModifier` + `ClockSpec`),
/// literal runs classified as `string`.  Returns `false` when there are
/// no specifiers.
pub(super) fn push_clock_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, inner)) = subspec_content(source, tok) else {
        return false;
    };
    let pos = TokenPositionContext { source, line_index };
    let mut emitted = false;
    let mut run = 0usize;
    for spec in tcl_cmd_core::clock::specifiers(inner) {
        flush_run(
            pos,
            cstart,
            inner,
            run,
            spec.start,
            TokenKind::String,
            entries,
        );
        push_subtoken(
            source,
            line_index,
            cstart + spec.start,
            "%",
            TokenKind::ClockPercent,
            entries,
        );
        if spec.modifier.is_some() {
            push_subtoken(
                source,
                line_index,
                cstart + spec.start + 1,
                &inner[spec.start + 1..spec.start + 2],
                TokenKind::ClockModifier,
                entries,
            );
        }
        let letter_start = spec.end - 1;
        push_subtoken(
            source,
            line_index,
            cstart + letter_start,
            &inner[letter_start..spec.end],
            TokenKind::ClockSpec,
            entries,
        );
        emitted = true;
        run = spec.end;
    }
    if !emitted {
        return false;
    }
    flush_run(
        pos,
        cstart,
        inner,
        run,
        inner.len(),
        TokenKind::String,
        entries,
    );
    true
}

/// Sub-tokenise a `format`/`scan` conversion string into its `%`
/// specifier components (`FormatPercent` / `FormatFlag` / `FormatWidth`
/// / `FormatSpec`), with literal runs classified as `string`.  Returns
/// `false` (emitting nothing) when there are no `%` specifiers.
pub(super) fn push_sprintf_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    dialect: &'static tcl_dialect::DialectProfile,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, inner)) = subspec_content(source, tok) else {
        return false;
    };
    let bytes = inner.as_bytes();
    let pos_ctx = TokenPositionContext { source, line_index };
    let profile = dialect;
    let mut emitted = false;
    let mut run = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(cuts) = parse_sprintf_cuts(bytes, i, profile)
        {
            flush_run(pos_ctx, cstart, inner, run, i, TokenKind::String, entries);
            let mut pos = i;
            for (end, kind) in cuts {
                emit_part(pos_ctx, cstart, inner, &mut pos, end, kind, entries);
            }
            emitted = true;
            i = pos;
            run = i;
            continue;
        }
        i += 1;
    }
    if !emitted {
        return false;
    }
    flush_run(
        pos_ctx,
        cstart,
        inner,
        run,
        inner.len(),
        TokenKind::String,
        entries,
    );
    true
}

/// Parse one `%`-specifier at `b[start]` into its component
/// `(end, kind)` cuts (monotonic ends, consumed in order by
/// [`emit_part`]), or `None` when it isn't a valid conversion (no type
/// letter — the `%` is then a literal).
fn parse_sprintf_cuts(
    b: &[u8],
    start: usize,
    profile: &tcl_dialect::DialectProfile,
) -> Option<Vec<(usize, TokenKind)>> {
    let mut owner_end = start + 1;
    let owner_spec = tcl_syntax::format::parse_spec(b, &mut owner_end)?;
    // `%%` is a literal percent in Tcl format strings, not a value-bearing
    // conversion. The shared parser represents it as verb `%` for renderers.
    if owner_spec.verb == b'%' {
        return None;
    }
    // `%%` is a literal percent in Tcl format strings, not a value-bearing
    // conversion.  The shared parser represents it as verb `%` so renderers
    // can handle it uniformly; semantic tokens must keep it as string text.
    if !tcl_cmd_core::format::is_verb(owner_spec.verb)
        || !tcl_cmd_core::format::is_available(&owner_spec, profile)
    {
        return None;
    }
    let n = b.len();
    let mut cuts: Vec<(usize, TokenKind)> = Vec::new();
    let mut j = start + 1;
    cuts.push((j, TokenKind::FormatPercent)); // `%`

    // Positional `<digits>$` (or `<digits>\$`).
    let pos_start = j;
    while j < n && b[j].is_ascii_digit() {
        j += 1;
    }
    if j > pos_start {
        let mut k = j;
        if b.get(k) == Some(&b'\\') {
            k += 1;
        }
        if b.get(k) == Some(&b'$') {
            cuts.push((j, TokenKind::FormatWidth)); // position digits
            cuts.push((k + 1, TokenKind::FormatPercent)); // `\`?`$`
            j = k + 1;
        } else {
            j = pos_start; // not positional — the digits are the width
        }
    }

    // Flags `[-+ 0#]*`.
    let flags_start = j;
    while j < n && matches!(b[j], b'-' | b'+' | b' ' | b'0' | b'#') {
        j += 1;
    }
    if j > flags_start {
        cuts.push((j, TokenKind::FormatFlag));
    }

    // Width `*` | digits.
    let width_start = j;
    if b.get(j) == Some(&b'*') {
        j += 1;
    } else {
        while j < n && b[j].is_ascii_digit() {
            j += 1;
        }
    }
    if j > width_start {
        let kind = digit_or_flag(b[width_start]);
        cuts.push((j, kind));
    }

    // Precision `.` then `*` | digits.  The separator is matched as a
    // literal `.` — the actual sprintf precision separator — not any
    // character, so a malformed `%5,3d` stays a plain string rather than
    // being mis-split into width-5 / precision-3 (highlighting only).
    if b.get(j) == Some(&b'.') {
        let value_start = j + 1;
        let mut k = value_start;
        if b.get(k) == Some(&b'*') {
            k += 1;
        } else {
            while k < n && b[k].is_ascii_digit() {
                k += 1;
            }
        }
        cuts.push((value_start, TokenKind::FormatFlag)); // the `.`
        if k > value_start {
            cuts.push((k, digit_or_flag(b[value_start])));
        }
        j = k;
    }

    // Length modifier `[hlLzq]`.
    if j < n && matches!(b[j], b'h' | b'l' | b'L' | b'z' | b'q') {
        j += 1;
        cuts.push((j, TokenKind::FormatFlag));
    }

    // Conversion type — required.
    if owner_end > start && b.get(owner_end - 1) == Some(&owner_spec.verb) {
        cuts.push((owner_end, TokenKind::FormatSpec));
        Some(cuts)
    } else {
        None
    }
}

/// `FormatWidth` for a digit, `FormatFlag` for `*` (variable width/prec).
fn digit_or_flag(first: u8) -> TokenKind {
    if first.is_ascii_digit() {
        TokenKind::FormatWidth
    } else {
        TokenKind::FormatFlag
    }
}

/// Emit `inner[*pos..end]` (absolute offset `cstart + *pos`) as `kind`
/// and advance `*pos`, when non-empty.  The sub-token cursor helper for
/// [`push_sprintf_subtokens`].
fn emit_part(
    pos_ctx: TokenPositionContext<'_>,
    cstart: usize,
    inner: &str,
    pos: &mut usize,
    end: usize,
    kind: TokenKind,
    entries: &mut Vec<Entry>,
) {
    if end > *pos {
        push_subtoken(
            pos_ctx.source,
            pos_ctx.line_index,
            cstart + *pos,
            &inner[*pos..end],
            kind,
            entries,
        );
        *pos = end;
    }
}

/// Sub-tokenise a string / bareword token's backslash escapes (`\n`, `\t`,
/// `\\`, `\x41`, `é`, `\101`, …): literal runs become `String`, each
/// escape becomes `Escape`.
/// Returns `false` (emitting nothing) when the token carries no backslash, so
/// the caller falls back to a single `String` token.  Multi-line tokens are
/// left to the caller.
pub(super) fn push_escape_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    config: tcl_lexer::LexerConfig,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, text)) = subspec_content(source, tok) else {
        return false;
    };
    if !text.contains('\\') || text.contains('\n') {
        return false;
    }
    // The split is `tcl-lexer`'s, beside the evaluator that defines the escape
    // widths — every highlighter that colours a Tcl string needs the same rule,
    // and three private copies of it had already drifted apart.
    let segments = tcl_lexer::split_backslash_escapes_in(text, config.escapes);
    if !segments.iter().any(|s| s.is_escape) {
        return false;
    }
    for seg in segments {
        let kind = if seg.is_escape {
            TokenKind::Escape
        } else {
            TokenKind::String
        };
        push_subtoken(
            source,
            line_index,
            cstart + seg.start,
            &text[seg.start..seg.end],
            kind,
            entries,
        );
    }
    // `subspec_content` yields the word's *content*, so this path — unlike
    // `push_token` — must emit both delimiters itself: `"with \"esc\" inside"`
    // would otherwise leave its opening and closing `"` unstyled.  Entries are sorted by
    // position before encoding, so appending them out of order here is fine.
    let start = tok.span.start() as usize;
    push_subtoken(
        source,
        line_index,
        start,
        &source[start..cstart],
        TokenKind::String,
        entries,
    );
    let content_end = cstart + text.len();
    if closing_delimiter(source, tok.span.start())
        .is_some_and(|c| source.as_bytes().get(content_end) == Some(&c))
    {
        push_subtoken(
            source,
            line_index,
            content_end,
            &source[content_end..=content_end],
            TokenKind::String,
            entries,
        );
    }
    true
}
