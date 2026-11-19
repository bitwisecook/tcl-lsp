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

//! Sub-tokens inside regular expressions and `regsub` substitution specs.

use super::argument_override::{ArgOverride, subspec_content};
use super::emission::{Entry, TokenPositionContext, flush_run, push_subtoken};
use super::legend::TokenKind;
use rustc_hash::FxHashMap;
use tcl_lexer::{LineIndex, Token, TokenType};

/// Colour the captures proved to fall inside this fragment of the source word.
pub(super) fn push_regsub_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    captures: &[(std::ops::Range<usize>, TokenKind)],
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, inner)) = subspec_content(source, tok) else {
        return false;
    };
    let pos = TokenPositionContext { source, line_index };
    let mut run = 0;
    for (span, kind) in captures {
        if span.start < cstart || span.end > cstart + inner.len() {
            continue;
        }
        let start = span.start - cstart;
        flush_run(pos, cstart, inner, run, start, TokenKind::String, entries);
        push_subtoken(
            source,
            line_index,
            span.start,
            &source[span.clone()],
            *kind,
            entries,
        );
        run = span.end - cstart;
    }
    if run == 0 {
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

/// Tag each literal (`Str`/`Esc`) fragment of the word spanning `word_span`
/// with `ov`, leaving `Var`/`Cmd` substitution fragments untouched (they fall
/// through to the default classifier).  A single-fragment literal word (a
/// braced `{a+b}` pattern, a plain `"abc"`) is tagged as one piece — the
/// common case — while a word interleaving literals and substitutions gets
/// each literal run tagged independently.
pub(super) fn mark_literal_fragments(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    word_span: tcl_lexer::Span,
    ov: &ArgOverride,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    for t in &seg.all_tokens {
        if t.span.start() >= word_span.start()
            && t.span.end() <= word_span.end()
            && matches!(t.kind, TokenType::Str | TokenType::Esc)
        {
            overrides
                .entry(t.span.start())
                .or_insert_with(|| ov.clone());
        }
    }
}

/// Retag the literal fragments of any word the compiler flagged as a regex
/// source (`set pat {…}` that later feeds `regexp`/`regsub`) so they highlight
/// as regex.  Keyed on the def-site word start; the compiler-supplied span is
/// authoritative for the fragment scan (the segmenter's token span can clamp a
/// closing delimiter).
pub(super) fn mark_regex_source_words(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    regex_sources: &FxHashMap<u32, tcl_lexer::Span>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if regex_sources.is_empty() {
        return;
    }
    for word in &seg.argv {
        if let Some(&full_span) = regex_sources.get(&word.span.start()) {
            mark_literal_fragments(seg, full_span, &ArgOverride::RegexPattern, overrides);
        }
    }
}

/// Sub-tokenise a regex pattern token into ARE components (groups,
/// character classes, quantifiers, anchors, escapes, backreferences,
/// alternation), with the literal runs between them classified as
/// `regexp`.  Returns `false` (emitting nothing) when the token isn't a
/// braced/quoted literal or contains no metacharacters — the caller then
/// falls back to a single `regexp` token.
pub(super) fn push_regex_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, inner)) = subspec_content(source, tok) else {
        return false;
    };
    let bytes = inner.as_bytes();
    let pos_ctx = TokenPositionContext { source, line_index };
    let mut matched_any = false;
    let mut pos = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if let Some(end) = scan_are_token(bytes, i) {
            flush_run(pos_ctx, cstart, inner, pos, i, TokenKind::Regexp, entries);
            let kind = classify_regex_component(&inner[i..end]);
            push_subtoken(
                source,
                line_index,
                cstart + i,
                &inner[i..end],
                kind,
                entries,
            );
            matched_any = true;
            i = end;
            pos = i;
        } else {
            i += 1;
        }
    }
    if !matched_any {
        return false;
    }
    if pos < inner.len() {
        push_subtoken(
            source,
            line_index,
            cstart + pos,
            &inner[pos..],
            TokenKind::Regexp,
            entries,
        );
    }
    true
}

/// Recognise one ARE metacharacter construct starting at `b[i]`,
/// returning its exclusive end, or `None` when `b[i]` is a literal
/// character.
fn scan_are_token(b: &[u8], i: usize) -> Option<usize> {
    let len = b.len();
    match b[i] {
        b'(' => {
            if b.get(i + 1) != Some(&b'?') {
                return Some(i + 1); // group open
            }
            // non-capturing / lookaround open: `(?:` `(?=` `(?!` `(?>`
            if let Some(b':' | b'=' | b'!' | b'>') = b.get(i + 2) {
                return Some(i + 3);
            }
            // embedded flags `(?imsx-imsx)`
            let mut j = i + 2;
            while j < len
                && matches!(
                    b[j],
                    b'i' | b'm' | b'n' | b's' | b'x' | b'w' | b'p' | b'q' | b'-'
                )
            {
                j += 1;
            }
            // Closed flag group → the whole `(?…)`; else just `(`.
            if b.get(j) == Some(&b')') {
                Some(j + 1)
            } else {
                Some(i + 1)
            }
        }
        b')' | b'|' | b'^' | b'$' | b'.' => Some(i + 1),
        b'*' | b'+' | b'?' => Some(if b.get(i + 1) == Some(&b'?') {
            i + 2
        } else {
            i + 1
        }),
        b'[' => scan_are_class(b, i),
        b'{' => scan_are_brace_quant(b, i),
        b'\\' if i + 1 < len => scan_are_escape(b, i),
        _ => None,
    }
}

/// Scan a bracket expression `[…]` starting at `b[i] == '['`.
///
/// `[` optional `^` optional leading `]`, then members up to the closing `]`.
/// A member is a `\`-escape (ARE recognises backslash escapes inside brackets,
/// e.g. `[\d]`), or a POSIX / collating / equivalence **sub-bracket**
/// (`[:alpha:]`, `[.ch.]`, `[=a=]`) whose internal `]` does **not** close the
/// outer bracket — so `[[:alpha:]]` scans as one char class, matching the ARE
/// engine (and C Tcl), not `[[:alpha:]` + a dangling `]`.
pub(super) fn scan_are_class(b: &[u8], i: usize) -> Option<usize> {
    let len = b.len();
    let mut j = i + 1;
    if b.get(j) == Some(&b'^') {
        j += 1;
    }
    // A `]` immediately after `[` / `[^` is a literal member, not the close.
    if b.get(j) == Some(&b']') {
        j += 1;
    }
    while j < len && b[j] != b']' {
        if b[j] == b'[' && matches!(b.get(j + 1), Some(b':' | b'.' | b'=')) {
            // Sub-bracket `[X … X]` (X ∈ `:.=`): skip to the matching `X]`.
            let delim = b[j + 1];
            let mut k = j + 2;
            while k + 1 < len && !(b[k] == delim && b[k + 1] == b']') {
                k += 1;
            }
            if k + 1 < len {
                j = k + 2; // past the closing `X]`
            } else {
                return None; // unterminated sub-bracket → not a token
            }
        } else if b[j] == b'\\' && j + 1 < len {
            j += 2;
        } else {
            j += 1;
        }
    }
    (j < len).then_some(j + 1) // unterminated class → not a token
}

/// Scan a brace quantifier `{n}` / `{n,}` / `{n,m}` at `b[i] == '{'`.
fn scan_are_brace_quant(b: &[u8], i: usize) -> Option<usize> {
    let len = b.len();
    let mut j = i + 1;
    let digits = j;
    while j < len && b[j].is_ascii_digit() {
        j += 1;
    }
    if j == digits {
        return None;
    }
    if b.get(j) == Some(&b',') {
        j += 1;
        while j < len && b[j].is_ascii_digit() {
            j += 1;
        }
    }
    (b.get(j) == Some(&b'}')).then_some(j + 1)
}

/// Scan a backslash escape at `b[i] == '\\'` (caller guarantees `i + 1`
/// is in bounds): a two-char class/anchor/backref/escaped-metachar, or a
/// `\xHH` / `\uHHHH` / `\UHHHHHHHH` hex escape.
fn scan_are_escape(b: &[u8], i: usize) -> Option<usize> {
    let len = b.len();
    let esc = b[i + 1];
    match esc {
        // class shortcuts / anchors / backref / escaped metachar /
        // escape sequence — all two characters.
        b'A'
        | b'b'
        | b'B'
        | b'd'
        | b'D'
        | b'm'
        | b'M'
        | b's'
        | b'S'
        | b'w'
        | b'W'
        | b'y'
        | b'Y'
        | b'z'
        | b'Z'
        | b'0'..=b'9'
        | b'a'
        | b'e'
        | b'f'
        | b'n'
        | b'r'
        | b't'
        | b'v'
        | b'.'
        | b'*'
        | b'+'
        | b'?'
        | b'('
        | b')'
        | b'{'
        | b'}'
        | b'['
        | b']'
        | b'|'
        | b'^'
        | b'$'
        | b'\\' => Some(i + 2),
        // `\xHH` (1-2 hex), `\uHHHH` (1-4), `\UHHHHHHHH` (1-8).
        b'x' | b'u' | b'U' => {
            let max = match esc {
                b'x' => 2,
                b'u' => 4,
                _ => 8,
            };
            let mut j = i + 2;
            while j < len && j < i + 2 + max && b[j].is_ascii_hexdigit() {
                j += 1;
            }
            // Requires at least one hex digit, else not a token.
            (j > i + 2).then_some(j)
        }
        _ => None, // `\` before an unrecognised char → literal
    }
}

/// Classify a single ARE metacharacter run.
pub(super) fn classify_regex_component(matched: &str) -> TokenKind {
    let bytes = matched.as_bytes();
    if matched.starts_with('[') {
        return TokenKind::RegexpCharClass;
    }
    if matched.starts_with('\\') && bytes.len() >= 2 {
        let ch = bytes[1];
        return if ch.is_ascii_digit() {
            TokenKind::RegexpBackref
        } else if matches!(
            ch,
            b'a' | b'e' | b'f' | b'n' | b'r' | b't' | b'v' | b'x' | b'u' | b'U'
        ) {
            TokenKind::RegexpEscape
        } else if matches!(ch, b'd' | b'D' | b's' | b'S' | b'w' | b'W') {
            TokenKind::RegexpCharClass
        } else if matches!(
            ch,
            b'b' | b'B' | b'm' | b'M' | b'y' | b'Y' | b'A' | b'z' | b'Z'
        ) {
            TokenKind::RegexpAnchor
        } else {
            TokenKind::RegexpEscape
        };
    }
    // registry-axis-ok: irreducible — ARE (`regexp`/`regsub` pattern-string)
    // metacharacters, not Tcl command syntax; `|`/`^`/`$` coincide with
    // `::tcl::mathop` operator spellings the same way `eq`/`ne`/`-` do
    // elsewhere in the tree (see the irreducible waivers on those); until
    // never
    match matched {
        "^" | "$" => TokenKind::RegexpAnchor,
        "|" => TokenKind::RegexpAlternation,
        "." => TokenKind::RegexpCharClass,
        // A group's *closer* is as much a group delimiter as its opener —
        // falling through to the quantifier catch-all below would paint every
        // `)` in the quantifier colour.
        ")" => TokenKind::RegexpGroup,
        _ if matched.starts_with('(') => TokenKind::RegexpGroup,
        _ => TokenKind::RegexpQuantifier,
    }
}
