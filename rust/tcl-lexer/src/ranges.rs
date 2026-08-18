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

//! Authoritative span/range accessors for delimited word tokens.
//!
//! The source-aware closer accessors `word_closer_offset` /
//! `word_end_position` derive a delimited
//! word's closing `}` / `]` / `"` from the lexer's content geometry —
//! **not** by re-deriving `tok.end.offset + 1`, which overshoots an
//! empty `{}` / `[]` / `""` (a trailing empty `{}` swallows
//! the enclosing body's `}`).
//!
//! These are the lossless, source-aware primitives the concrete syntax
//! tree and any caller that needs to *slice source* (a
//! refactor edit extracting a word's raw text) build on.  Command/word
//! *ranges* owned by the segmenter use the inner-end convention and
//! widen only when they need the closer — see
//! [`crate::SourceMap::range_positions`] and the segmenter's
//! `command_span`.
//!
//! The [`Token`] carries only a [`Span`], with text and positions
//! resolved on demand through a [`SourceMap`].  These accessors
//! therefore take `&SourceMap` to recover a word's source geometry.
//!
//! [`close_quote_offset`] and [`word_closer_offset_at`] /
//! [`word_span_at`] are the token-free members of the family.
//! [`close_quote_offset`] answers the same "where does this word close?"
//! question for a `"…"` word from a bare byte offset;
//! [`word_closer_offset_at`] answers it for **any** delimited word from
//! the word's own lexer [`Span`], for callers that kept a span but not
//! the [`Token`] it came from (an optimiser pass rewriting from a CFG
//! terminator's condition span, a rename edit built from an analysis
//! reference span, the explorer widening an IR range).
//!
//! [`Token`]: crate::Token
//! [`Span`]: crate::Span
//! [`SourceMap`]: crate::SourceMap

use tcl_dialect::BracedVarStyle;

use crate::source_map::SourceMap;
use crate::substitution::{backslash_escape_end, is_line_continuation};
use crate::tokens::{ByteCol, SourcePosition, Token};

/// The closing delimiter for an opening `"` / `{` / `[`, or `None`.
const fn closer_for(opener: u8) -> Option<u8> {
    match opener {
        b'"' => Some(b'"'),
        b'{' => Some(b'}'),
        b'[' => Some(b']'),
        _ => None,
    }
}

/// Byte offset of the `}` that closes a `${…}` variable reference whose **name
/// starts at `name_start`** (the byte just past the `${`), or `None` when the
/// form is unterminated.
///
/// This is the one release-aware `${…}` close rule. `Tcl_ParseVarName` changed
/// between the 8.x family and 9.x, and the difference is observable:
///
/// - [`BracedVarStyle::Tcl9Nesting`] (9.0.4 `tclParse.c:1315`) tracks nested
///   `{…}` pairs and consumes `\X` as an inert two-character unit, so
///   `${a{b}c}` names the variable `a{b}c` and `${a\}b}` names `a\}b`.
/// - [`BracedVarStyle::FirstClose`] (8.6.16 `tclParse.c:1398`) ends the name at
///   the **first** literal `}` with no nesting and no backslash processing, so
///   `${a{b}c}` names `a{b` and `c}` is ordinary word text.
///
/// Every consumer that has to find that `}` — the lexer's own `parse_var` and
/// array-index scans, both `subst` engines — resolves it here rather than
/// re-implementing a scan, because an engine that hard-codes one release's rule
/// answers `subst {${a{b}c}}` wrongly on the other (issue #1457).
///
/// `src` need not be valid UTF-8; the scan reacts only to ASCII bytes, and the
/// 9.x backslash pair consumes a whole UTF-8 character so it matches the
/// lexer's `char`-based walk byte for byte.
#[must_use]
pub fn braced_var_name_end(src: &[u8], name_start: usize, style: BracedVarStyle) -> Option<usize> {
    let n = src.len();
    let mut i = name_start;
    if !style.nests() {
        // 8.x: scan to the first literal `}`; `{` and `\` are name characters.
        while i < n && src[i] != b'}' {
            i += 1;
        }
        return (i < n).then_some(i);
    }
    let mut depth: u32 = 0;
    while i < n {
        match src[i] {
            b'}' if depth == 0 => return Some(i),
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth -= 1;
                i += 1;
            }
            b'\\' => {
                // The backslash and the following *character* are one inert
                // unit, so an escaped `}` does not close the reference.
                i += 1;
                if i < n {
                    i += utf8_char_len(src[i]);
                }
            }
            b => i += utf8_char_len(b),
        }
    }
    None
}

/// Byte length of the UTF-8 character whose lead byte is `b` (1 for ASCII and
/// for a stray continuation byte, so the scan always advances).
const fn utf8_char_len(b: u8) -> usize {
    match b {
        0x00..=0xbf => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

/// Position of the closing delimiter one byte past *end*.
///
/// *`last_inner`* is the byte
/// at `end.offset` (the last byte inside the word).  When it is a
/// newline the closer sits at column 0 of the next line, so the
/// line/column advance accordingly — keeping them consistent with
/// `offset` for line/column-based consumers.
///
/// Only `\n` breaks a line, matching [`crate::LineIndex`]'s `\n`-only model
/// (the #537 invariant).  A bare `\r` last-inner byte is *not* a line break,
/// so treating it as one would place the closer at a line/column
/// `position_at` never reports — an inconsistent, nonexistent position.
fn closer_position(end: SourcePosition, last_inner: Option<u8>) -> SourcePosition {
    if last_inner == Some(b'\n') {
        SourcePosition::new(end.line + 1, ByteCol::new(0), end.offset + 1)
    } else {
        SourcePosition::new(
            end.line,
            ByteCol::new(end.character.get() + 1),
            end.offset + 1,
        )
    }
}

/// Byte offset of *tok*'s closing `}` / `]` / `"` in the source, or `None`.
///
/// The authoritative way to locate a delimited word's closing delimiter
/// for a caller that needs to *slice source* (e.g. extracting the raw
/// argument text of a refactor edit).  It must be called rather than
/// re-deriving the position as `tok.span.end()`: the lexer follows the
/// inner-end convention (a braced/bracketed/quoted word's span ends one
/// byte *before* the closer) **except** for an empty `{}` / `[]` / `""`,
/// whose span already covers the closer.  [`SourceMap::token_text`] is
/// the discriminator — an empty word has none — so the closer is at the
/// inclusive end when the word is empty and one byte past it otherwise.
/// Deriving emptiness from the token text keeps this correct for quoted
/// words whose inner text contains backslash escapes (`"a\"b"`) and for
/// a non-empty word whose last inner byte is itself a closer (`{a\}}`).
///
/// Returns `None` when *tok* does not begin with an opening delimiter, or
/// when the word is unterminated (the computed position is not the
/// matching closer).  *tok* and *sm* must share a coordinate frame.
#[must_use]
pub fn word_closer_offset(sm: &SourceMap<'_>, tok: Token) -> Option<u32> {
    let bytes = sm.source().as_bytes();
    let start = tok.span.start();
    let opener = *bytes.get(start as usize)?;
    let closer = closer_for(opener)?;
    // `end.offset` is `tok.end.offset` (inclusive end): the
    // last inner byte for a non-empty word, or the closer itself for an
    // empty `{}` / `[]` / `""`.
    let (_first, end) = sm.range_positions(tok.span);
    let closer_off = if sm.token_text(tok).is_empty() {
        end.offset
    } else {
        end.offset + 1
    };
    if bytes.get(closer_off as usize) == Some(&closer) {
        Some(closer_off)
    } else {
        None
    }
}

/// Byte offset immediately **after** *tok*'s last source byte — the point
/// at which a further word can be appended to the command *tok* belongs to.
///
/// This is the anchor a quick-fix uses when it splices an extra trailing
/// argument onto an invocation (`catch BODY` → `catch BODY result`).  It
/// must be derived from the word's own geometry rather than from the
/// command-head token: a body-bearing command's head span covers only the
/// word `catch`, so anchoring there writes the new word *before* the body
/// and silently re-assigns every argument position (issue #1190).
///
/// For a delimited word this is one byte past its closing `}` / `]` / `"`
/// (via [`word_closer_offset`], so an empty `{}` / `[]` / `""` is not
/// overshot).  For a bare word — and for an unterminated delimited word,
/// where no closer exists to step over — the token span's exclusive end is
/// already the append point.
#[must_use]
pub fn word_append_offset(sm: &SourceMap<'_>, tok: Token) -> u32 {
    match word_closer_offset(sm, tok) {
        Some(closer) => closer.saturating_add(1),
        None => tok.span.end(),
    }
}

/// The **whole written word** *tok* denotes, closing delimiter included.
///
/// # Why this exists — the inner-end trap, in plain English
///
/// A [`Token`]'s own `span` does *not* cover the word's closing delimiter.
/// The lexer records where the word's **content** ends, not where the word
/// ends, so for `[foo]`, `{foo}` and `"foo"` the `]` / `}` / `"` sits one
/// byte *past* `tok.span.end()`.  Slicing the source with the raw span
/// therefore yields a word with its tail lopped off, and any diagnostic
/// anchored on the raw span underlines one character short — or, when the
/// word is written across a line continuation, underlines a region that
/// stops mid-word on the wrong line (issue #1330).
///
/// Worked example — the source `[Dog new] bark`, with byte offsets:
///
/// ```text
///   offset:  0 1 2 3 4 5 6 7 8 9 …
///   byte:    [ D o g _ n e w ] _ b a r k
///                            ^
///                            the `]` lives at offset 8
///
///   tok.span            == 0..8    →  source[0..8] == "[Dog new"    <- wrong
///   word_span(sm, tok)  == 0..9    →  source[0..9] == "[Dog new]"   <- right
/// ```
///
/// The naive fix — "just add one" — is wrong twice over, which is exactly
/// why this helper exists instead of a `+ 1` at each call site:
///
/// * An **empty** group (`{}` / `[]` / `""`) is the exception: the lexer
///   already extends its span over the closer, so `+ 1` overshoots into
///   whatever follows.  A trailing empty `{}` would swallow the enclosing
///   body's own `}`.
/// * A word whose last *inner* byte happens to be a closer (`{a\}}`) fools
///   a naive "does the text already end with the closer?" check into
///   skipping the widening it really does need.
///
/// [`word_closer_offset`] settles both cases from the lexer's own content
/// geometry; this is the [`Span`]-returning wrapper over it.  For a bare
/// word, or a word left unterminated at end of input, the token span is
/// already the whole word and is returned unchanged.
///
/// # Known gap: `${name}`
///
/// [`word_closer_offset`] keys the closer off the word's **opening byte**,
/// and a braced *variable* word opens with `$`, not with a delimiter — so a
/// `${name}` word's closing `}` is not recovered here and its span comes
/// back one byte short.  The distinguishing fact is on the token
/// (`content_offset == 2` means `${` was stripped), so this is fixable, but
/// [`word_closer_offset`] is also the anchor several quick-fixes splice
/// against and widening it is a change in its own right — tracked
/// separately rather than smuggled in.  Callers that must handle `${name}`
/// today do it themselves (see `value_shapes`'s `orphaned_closer` step).
///
/// *tok* and *sm* must share a coordinate frame.
///
/// ```
/// use tcl_lexer::{Lexer, SourceMap, TokenType, word_span};
/// let src = "[Dog new] bark";
/// let sm = SourceMap::new(src);
/// let tok = Lexer::new(src)
///     .tokenise_all()
///     .unwrap()
///     .into_iter()
///     .find(|t| t.kind == TokenType::Cmd)
///     .unwrap();
/// assert_eq!(&src[tok.span.as_range()], "[Dog new");
/// assert_eq!(&src[word_span(&sm, tok).as_range()], "[Dog new]");
/// ```
///
/// [`Span`]: crate::Span
#[must_use]
pub fn word_span(sm: &SourceMap<'_>, tok: Token) -> crate::Span {
    crate::Span::new(tok.span.start(), word_append_offset(sm, tok))
}

/// Byte offset of the closing `}` / `]` / `"` of the word whose **lexer
/// span** is *span*, or `None`.
///
/// The token-free sibling of [`word_closer_offset`], for a caller that kept
/// a word's [`Span`] but not the [`Token`] it came from — an optimiser pass
/// holding a CFG terminator's condition span, a rename edit built from an
/// analysis reference span, the compiler explorer widening an IR range.
/// Those callers used to hand-roll the arithmetic and the copies drifted:
/// `branch_folding`'s decided the question with a textual
/// `!text.ends_with('}')` guess, so a condition ending in a *nested* empty
/// pair — `while {$x eq {}}` — looked already-widened and the outer `}` was
/// dropped from the rewrite target (issue #1423).
///
/// *span* must be the lexer's own span for a whole delimited word: the
/// opening delimiter is its first byte and, by the inner-end convention,
/// the closer sits one byte past the span **except** for an empty
/// `{}` / `[]` / `""` / `${}`, whose span already covers it.  The empty
/// form is exactly a two-byte body after the opener, which is what
/// [`SourceMap::token_text`] reports as empty — so this agrees with
/// [`word_closer_offset`] byte for byte on every word token, without
/// needing the token.
///
/// Unlike [`word_closer_offset`] this also resolves the braced *variable*
/// form `${name}`: the span already encodes whichever
/// [`BracedVarStyle`](tcl_dialect::BracedVarStyle) the lexer applied, so
/// recovering the closer from it stays release-blind.
///
/// Returns `None` when the span is empty or out of bounds, when it does not
/// open with a delimiter, or when the word is unterminated (the computed
/// position is not the matching closer).
#[must_use]
pub fn word_closer_offset_at(source: &str, span: crate::Span) -> Option<u32> {
    let bytes = source.as_bytes();
    let (start, end) = (span.start() as usize, span.end() as usize);
    if start >= end || end > bytes.len() {
        return None;
    }
    // A `${name}` word opens with `$`; every other delimited word opens
    // with the delimiter itself.
    let opener_len = usize::from(bytes[start] == b'$' && bytes.get(start + 1) == Some(&b'{'));
    let closer = closer_for(bytes[start + opener_len])?;
    let empty = end == start + opener_len + 2 && bytes[end - 1] == closer;
    let closer_off = if empty { end - 1 } else { end };
    if bytes.get(closer_off) == Some(&closer) {
        u32::try_from(closer_off).ok()
    } else {
        None
    }
}

/// The **whole written word** the lexer span *span* denotes, closing
/// delimiter included — the token-free sibling of [`word_span`].
///
/// Widens *span* through [`word_closer_offset_at`], so an empty
/// `{}` / `[]` / `""` / `${}` is never overshot and a word whose last inner
/// byte is itself a closer (`{a\}}`, `{$x eq {}}`) is never left short.  A
/// bare word, an unterminated word, and a span that is already the whole
/// word come back unchanged.
///
/// ```
/// use tcl_lexer::{Span, word_span_at};
/// // `{$x eq {}}` lexes with the span `{$x eq {}` — it already ends in a
/// // `}` (the inner empty pair's), yet still needs its own closer.
/// let src = "{$x eq {}}";
/// assert_eq!(&src[word_span_at(src, Span::new(0, 9)).as_range()], src);
/// // The empty pair's own span is already whole.
/// assert_eq!(word_span_at("{}", Span::new(0, 2)), Span::new(0, 2));
/// ```
#[must_use]
pub fn word_span_at(source: &str, span: crate::Span) -> crate::Span {
    match word_closer_offset_at(source, span) {
        Some(closer) => crate::Span::new(span.start(), closer.saturating_add(1)),
        None => span,
    }
}

/// Inclusive end [`SourcePosition`] covering *tok*'s closing delimiter.
///
/// The position-returning sibling of [`word_closer_offset`], for callers
/// that build a range/LSP position rather than slice by offset.  For a
/// delimited word it returns the position *of* the closing `}` / `]` /
/// `"` (with line/column advanced when the closer falls on the next
/// line); for an empty `{}` / `[]` / `""` the inclusive end already *is*
/// the closer, so the token's inclusive end is returned unchanged.  For
/// any non-delimited or unterminated token the inclusive end is returned
/// unchanged.  Unlike the segmenter's `command_span` widening this also
/// covers quoted `"..."` words.
#[must_use]
pub fn word_end_position(sm: &SourceMap<'_>, tok: Token) -> SourcePosition {
    let (_first, end) = sm.range_positions(tok.span);
    let Some(closer_off) = word_closer_offset(sm, tok) else {
        return end;
    };
    if closer_off == end.offset {
        // Empty `{}` / `[]` / `""`: the inclusive end already is the closer.
        return end;
    }
    let last_inner = sm.source().as_bytes().get(end.offset as usize).copied();
    closer_position(end, last_inner)
}

/// Byte offset of the `"` that closes the quoted word opening at
/// *`open_quote`*, or `None` when there is none.
///
/// The token-free sibling of [`word_closer_offset`], for a caller that
/// holds source text and a byte offset rather than a lexed [`Token`] — a
/// rewrite pass that has only an argv span, or a minifier folding a
/// `"…"` argument in place.  Such callers used to hand-roll the scan, and
/// the copies drifted: one skipped `\`-escapes but not command
/// substitutions, so `"a[foo "b"]c"` closed at the quote *opening* the
/// inner `"b"` and any edit made against that offset truncated the word
/// mid-substitution (issue #1424).
///
/// The scan follows the same rules the lexer's own quoted-word and
/// command-substitution parsers use:
///
/// * a `\`-escape is skipped as one unit via [`backslash_escape_end`], so
///   `\"` is content and `\[` opens nothing.  The release-blind (Tcl 9.0) form
///   is deliberate: the escape grammar is release-variant in *width*, but every
///   form's payload is hex or octal digits, none of which is a delimiter, so no
///   release's widths move a closing `"` or `]` (issue #1479);
/// * a `[…]` command substitution is skipped whole — quotes inside it
///   belong to the substituted command, not to this word — with brackets
///   nesting and a `]` inside a braced or quoted word of the inner
///   command left inert;
/// * a `#` at *command position* inside that substitution opens a Tcl
///   comment, so a `]` written in the comment is inert and does not end
///   the substitution early.
///
/// Returns `None` when *`open_quote`* does not point at a `"`, when the
/// word runs to end of input unterminated, or when an inner command
/// substitution is itself unterminated (no offset can then be trusted).
/// The offset is the closer itself, so an exclusive end is one past it.
///
/// ```
/// use tcl_lexer::close_quote_offset;
/// let src = r#"set x "a[foo "b"]c""#;
/// assert_eq!(close_quote_offset(src, 6), Some(src.len() - 1));
/// assert_eq!(close_quote_offset(r#""""#, 0), Some(1));
/// assert_eq!(close_quote_offset(r#""abc"#, 0), None);
/// ```
#[must_use]
pub fn close_quote_offset(source: &str, open_quote: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(open_quote) != Some(&b'"') {
        return None;
    }
    let mut pos = open_quote + 1;
    while pos < bytes.len() {
        match bytes[pos] {
            b'"' => return Some(pos),
            b'\\' => pos = backslash_escape_end(source, pos),
            b'[' => pos = command_substitution_end(source, pos)?,
            _ => pos += 1,
        }
    }
    None
}

/// Byte offset one past the `]` closing the command substitution opening
/// at *`open_bracket`*, or `None` when it is unterminated.
///
/// Brackets nest, a `\`-escape is inert, and a `]` inside a braced or
/// quoted word of the inner command does not close the substitution.
///
/// # Comments are part of the grammar here
///
/// A substituted script is a *script*, so C Tcl's parser runs its
/// comment rule inside the brackets: at command position an unescaped
/// `#` starts a comment that runs to the next unescaped newline, and
/// every `]`, `[`, `"` and `{` written in that comment is inert.  A
/// scanner without comment state stops at the first commented-out `]`
/// and hands its caller a truncated span — for
/// `"a[\n# ] comment\nset y "b"\n]c"` it would report the `"` opening
/// the inner `"b"` as the outer word's closer, and O129 or the minifier
/// would then rewrite over the middle of otherwise valid source.
///
/// Command position is the start of the substitution, and whatever
/// follows an unescaped newline or `;` at bracket top level, leading
/// whitespace skipped.  A backslash-newline counts as that leading
/// whitespace — it substitutes to a space — so a `#` after one still
/// opens a comment and the `]` in `"a[\n\\\n# ] c\n]b"` stays inert.
/// A `#` inside a `"…"` or `{…}` word is ordinary
/// text, never a comment — hence the invariant that `at_command_start`
/// is only ever raised while `braces == 0 && !in_quotes`, which is what
/// lets the `#` arm below test that flag alone.
#[must_use]
pub fn command_substitution_end(source: &str, open_bracket: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(open_bracket) != Some(&b'[') {
        return None;
    }
    let mut pos = open_bracket + 1;
    let mut depth: u32 = 1;
    let mut braces: u32 = 0;
    let mut in_quotes = false;
    let mut in_comment = false;
    let mut at_command_start = true;
    while pos < bytes.len() {
        let byte = bytes[pos];
        if byte == b'\\' {
            // One unit everywhere, comments included: a backslash-newline
            // continues a comment onto the following line rather than
            // ending it.
            //
            // A continuation substitutes to a space, and C Tcl runs
            // `ParseAllWhiteSpace` — which absorbs it — *before* testing for
            // the `#` that opens a comment, so it keeps command position just
            // as a literal space does. Any other escape is content and ends
            // command position.
            let continuation = is_line_continuation(source, pos);
            pos = backslash_escape_end(source, pos);
            at_command_start &= continuation;
            continue;
        }
        if in_comment {
            if byte == b'\n' {
                in_comment = false;
                at_command_start = true;
            }
            pos += 1;
            continue;
        }
        match byte {
            b'#' if at_command_start => in_comment = true,
            b'"' if braces == 0 => {
                in_quotes = !in_quotes;
                at_command_start = false;
            }
            b'{' if !in_quotes => {
                braces += 1;
                at_command_start = false;
            }
            b'}' if !in_quotes => {
                braces = braces.saturating_sub(1);
                at_command_start = false;
            }
            b'[' if braces == 0 && !in_quotes => {
                depth += 1;
                // The nested script opens at command position too.
                at_command_start = true;
            }
            b']' if braces == 0 && !in_quotes => {
                depth -= 1;
                if depth == 0 {
                    return Some(pos + 1);
                }
                at_command_start = false;
            }
            b'\n' | b';' if braces == 0 && !in_quotes => at_command_start = true,
            // Whitespace before the first word of a command keeps the
            // scanner at command position, so `[foo; # ]` still comments.
            b' ' | b'\t' | b'\r' | b'\x0b' | b'\x0c' => {}
            _ => at_command_start = false,
        }
        pos += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Lexer, SourceMap, Span, TokenType};

    /// The `${…}` close rule (issue #1457). `Tcl_ParseVarName` delimits the
    /// brace form differently in the 8.x family (`tclParse.c(8.6.16):1398` —
    /// first literal `}`) and in 9.x (`tclParse.c(9.0.4):1315` — brace depth
    /// plus inert `\X` pairs). Both `subst` engines and the lexer's own
    /// `parse_var` resolve the form here.
    #[test]
    fn braced_var_name_end_follows_the_release_rule() {
        use BracedVarStyle::{FirstClose, Tcl9Nesting};
        let end = |src: &str, style| braced_var_name_end(src.as_bytes(), 2, style);

        // A plain name closes at the same place under both rules.
        assert_eq!(end("${abc}", Tcl9Nesting), Some(5));
        assert_eq!(end("${abc}", FirstClose), Some(5));
        // `${}` — the variable literally named "" — closes immediately.
        assert_eq!(end("${}", Tcl9Nesting), Some(2));
        assert_eq!(end("${}", FirstClose), Some(2));

        // Nested braces: 9.x balances them, 8.x stops at the first `}`.
        assert_eq!(end("${a{b}c}", Tcl9Nesting), Some(7)); // name `a{b}c`
        assert_eq!(end("${a{b}c}", FirstClose), Some(5)); // name `a{b`
        // Two levels deep.
        assert_eq!(end("${a{b{c}d}e}", Tcl9Nesting), Some(11));
        assert_eq!(end("${a{b{c}d}e}", FirstClose), Some(7));

        // An escaped `}` is inert in 9.x only.
        assert_eq!(end(r"${a\}b}", Tcl9Nesting), Some(6)); // name `a\}b`
        assert_eq!(end(r"${a\}b}", FirstClose), Some(4)); // name `a\`

        // Unterminated forms have no closer under either rule.
        assert_eq!(end("${abc", Tcl9Nesting), None);
        assert_eq!(end("${abc", FirstClose), None);
        assert_eq!(end("${a{b", Tcl9Nesting), None);
        // …but 8.x still finds a `}` that 9.x consumes as nesting.
        assert_eq!(end("${a{b}", Tcl9Nesting), None);
        assert_eq!(end("${a{b}", FirstClose), Some(5));
        // A trailing lone backslash swallows nothing further in 9.x.
        assert_eq!(end(r"${a\", Tcl9Nesting), None);

        // Multi-byte text is walked a whole character at a time, so a
        // continuation byte can never be mistaken for a delimiter.
        assert_eq!(end("${aéb}", Tcl9Nesting), Some(6));
        assert_eq!(end("${aéb}", FirstClose), Some(6));
        // A backslash before a multi-byte character consumes the whole
        // character, matching the lexer's `char`-based walk.
        assert_eq!(end("${a\\é}", Tcl9Nesting), Some(6));
    }

    /// Lex `src` and return the first non-trivia token (the word under test).
    fn first_word(src: &str) -> Token {
        let toks = Lexer::new(src).tokenise_all().unwrap();
        *toks
            .iter()
            .find(|t| {
                !matches!(
                    t.kind,
                    TokenType::Sep | TokenType::Eol | TokenType::Eof | TokenType::Comment
                )
            })
            .expect("a word token")
    }

    /// Lex `src` and return the `nth` (0-based) non-trivia token.
    fn nth_word(src: &str, n: usize) -> Token {
        let toks = Lexer::new(src).tokenise_all().unwrap();
        *toks
            .iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    TokenType::Sep | TokenType::Eol | TokenType::Eof | TokenType::Comment
                )
            })
            .nth(n)
            .expect("an nth word token")
    }

    #[test]
    fn closer_offset_non_empty_braced_word() {
        // `{abc}` — span covers `{abc`, the closer `}` sits at span.end().
        let src = "{abc}";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        assert_eq!(tok.kind, TokenType::Str);
        let off = word_closer_offset(&sm, tok).unwrap();
        assert_eq!(off, 4);
        assert_eq!(src.as_bytes()[off as usize], b'}');
    }

    #[test]
    fn closer_offset_non_empty_command_sub() {
        // `[x]` — CMD token, span covers `[x`, closer `]` at span.end().
        let src = "[x]";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        assert_eq!(tok.kind, TokenType::Cmd);
        let off = word_closer_offset(&sm, tok).unwrap();
        assert_eq!(off, 2);
        assert_eq!(src.as_bytes()[off as usize], b']');
    }

    #[test]
    fn closer_offset_non_empty_quoted_word() {
        // `"ab"` lexes as ESC; the closer is the closing `"`. Unlike the
        // segmenter's Str/Cmd-only widening, this accessor covers quoted
        // words too.
        let src = "\"ab\"";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        assert_eq!(tok.kind, TokenType::Esc);
        let off = word_closer_offset(&sm, tok).unwrap();
        assert_eq!(off, 3);
        assert_eq!(src.as_bytes()[off as usize], b'"');
    }

    #[test]
    fn closer_offset_backslash_bearing_quoted_word() {
        // `"a\"b"` — the inner `\"` is an escaped quote, NOT the closer.
        // Emptiness keys on the token text (non-empty here), so the
        // accessor lands on the final `"`, not the escaped one.
        let src = "\"a\\\"b\"";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        let off = word_closer_offset(&sm, tok).unwrap();
        assert_eq!(off, 5);
        assert_eq!(src.as_bytes()[off as usize], b'"');
    }

    #[test]
    fn closer_offset_escaped_brace_in_braced_word() {
        // `{a\}}` — the `\}` is an escaped brace; the real closer is the
        // final `}` at span.end(). A byte-only "last inner byte is `}`?"
        // heuristic would wrongly treat this as an empty form.
        let src = "{a\\}}";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        let off = word_closer_offset(&sm, tok).unwrap();
        assert_eq!(off, 4);
        assert_eq!(src.as_bytes()[off as usize], b'}');
    }

    #[test]
    fn closer_offset_empty_braced_word_lands_on_own_closer() {
        // `{}` — span already covers both braces; the closer is the
        // inclusive end, NOT one byte past it.
        let src = "{}";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        let off = word_closer_offset(&sm, tok).unwrap();
        assert_eq!(off, 1);
        assert_eq!(src.as_bytes()[off as usize], b'}');
    }

    #[test]
    fn closer_offset_empty_command_sub() {
        let src = "[]";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        let off = word_closer_offset(&sm, tok).unwrap();
        assert_eq!(off, 1);
        assert_eq!(src.as_bytes()[off as usize], b']');
    }

    #[test]
    fn closer_offset_empty_quoted_word() {
        let src = "\"\"";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        let off = word_closer_offset(&sm, tok).unwrap();
        assert_eq!(off, 1);
        assert_eq!(src.as_bytes()[off as usize], b'"');
    }

    #[test]
    fn closer_offset_empty_brace_does_not_grab_enclosing_closer() {
        // The #527 case: an empty `{}` immediately followed by an
        // enclosing `}`. The empty brace's own closer is at offset 3;
        // the naive `span.end()` (offset 4) would point at the
        // *enclosing* `}` — an overshoot. This pins the faithful answer.
        let src = "a {}}";
        let sm = SourceMap::new(src);
        let empty_brace = nth_word(src, 1);
        assert_eq!(empty_brace.kind, TokenType::Str);
        assert_eq!(empty_brace.span.start(), 2);
        assert_eq!(empty_brace.span.end(), 4); // naive closer guess
        let off = word_closer_offset(&sm, empty_brace).unwrap();
        assert_eq!(off, 3, "must land on the empty brace's own closer");
        assert_ne!(off, 4, "must not overshoot into the enclosing `}}`");
    }

    #[test]
    fn closer_offset_none_for_bare_word() {
        let src = "foo bar";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        assert_eq!(tok.kind, TokenType::Esc);
        assert!(word_closer_offset(&sm, tok).is_none());
    }

    #[test]
    fn closer_offset_none_for_unterminated_braced_word() {
        // `{abc` runs to EOF with no closer — the computed position is
        // past the buffer, so the accessor reports `None`.
        let src = "{abc";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        assert!(word_closer_offset(&sm, tok).is_none());
    }

    #[test]
    fn end_position_non_empty_braced_word_advances_one() {
        // `{abc}` — inclusive end is the `c` (offset 3); the closer `}`
        // is one column further on the same line.
        let src = "{abc}";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        let pos = word_end_position(&sm, tok);
        assert_eq!(pos, SourcePosition::new(0, ByteCol::new(4), 4));
    }

    #[test]
    fn end_position_empty_brace_unchanged() {
        // `{}` — the inclusive end already is the closer, returned as-is.
        let src = "{}";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        let pos = word_end_position(&sm, tok);
        // Inclusive end of the span: offset 1, on the `}`.
        assert_eq!(pos, sm.range_positions(tok.span).1);
        assert_eq!(pos.offset, 1);
    }

    #[test]
    fn end_position_multiline_closer_advances_line() {
        // A braced body whose last inner byte is a newline: the closing
        // `}` sits at column 0 of the next line.
        let src = "{a\n}";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        assert_eq!(tok.kind, TokenType::Str);
        let pos = word_end_position(&sm, tok);
        assert_eq!(pos, SourcePosition::new(1, ByteCol::new(0), 3));
        assert_eq!(src.as_bytes()[pos.offset as usize], b'}');
    }

    #[test]
    fn end_position_bare_cr_is_not_a_line_break() {
        // `{a\r}` — the last inner byte is a bare `\r`. LineIndex counts only
        // `\n`, so the closer must stay on line 0 (issue 161); treating `\r`
        // as a break would report a position `position_at` never emits.
        let src = "{a\r}";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        assert_eq!(tok.kind, TokenType::Str);
        let pos = word_end_position(&sm, tok);
        // The closer `}` is at offset 3, same line, column 3.
        assert_eq!(pos, SourcePosition::new(0, ByteCol::new(3), 3));
        assert_eq!(src.as_bytes()[pos.offset as usize], b'}');
        // Consistent with the line model: position_at agrees.
        assert_eq!(pos, sm.position_at(3));
    }

    #[test]
    fn end_position_crlf_closer_advances_line() {
        // `{a\r\n}` — the last inner byte is `\n` (a real break), so the
        // closer moves to column 0 of the next line, as with a bare `\n`.
        let src = "{a\r\n}";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        assert_eq!(tok.kind, TokenType::Str);
        let pos = word_end_position(&sm, tok);
        assert_eq!(pos, SourcePosition::new(1, ByteCol::new(0), 4));
        assert_eq!(src.as_bytes()[pos.offset as usize], b'}');
    }

    #[test]
    fn end_position_bare_word_unchanged() {
        let src = "foo bar";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        let pos = word_end_position(&sm, tok);
        assert_eq!(pos, sm.range_positions(tok.span).1);
    }

    #[test]
    fn end_position_unterminated_unchanged() {
        let src = "{abc";
        let sm = SourceMap::new(src);
        let tok = first_word(src);
        let pos = word_end_position(&sm, tok);
        assert_eq!(pos, sm.range_positions(tok.span).1);
    }

    #[test]
    fn append_offset_lands_past_a_braced_body() {
        // The issue #1190 shape: `catch {error oops}` — a new trailing word
        // belongs immediately after the body's `}`, at offset 18.
        let src = "catch {error oops}";
        let sm = SourceMap::new(src);
        let body = nth_word(src, 1);
        assert_eq!(body.kind, TokenType::Str);
        let off = word_append_offset(&sm, body);
        assert_eq!(off, u32::try_from(src.len()).unwrap());
        assert_eq!(&src[..off as usize], "catch {error oops}");
    }

    #[test]
    fn append_offset_lands_past_a_multiline_braced_body() {
        let src = "catch {\n    error oops\n}";
        let sm = SourceMap::new(src);
        let body = nth_word(src, 1);
        let off = word_append_offset(&sm, body);
        assert_eq!(off, u32::try_from(src.len()).unwrap());
    }

    #[test]
    fn append_offset_does_not_overshoot_an_empty_body() {
        // `catch {}` — the empty brace's span already covers its closer, so
        // a naive `span.end() + 1` would land one byte past the buffer.
        let src = "catch {}";
        let sm = SourceMap::new(src);
        let body = nth_word(src, 1);
        let off = word_append_offset(&sm, body);
        assert_eq!(off, 8);
        assert_eq!(off, u32::try_from(src.len()).unwrap());
    }

    #[test]
    fn append_offset_after_a_bare_word() {
        let src = "catch $body";
        let sm = SourceMap::new(src);
        let body = nth_word(src, 1);
        let off = word_append_offset(&sm, body);
        assert_eq!(off, u32::try_from(src.len()).unwrap());
    }

    #[test]
    fn append_offset_after_a_quoted_word() {
        let src = "catch \"error oops\"";
        let sm = SourceMap::new(src);
        let body = nth_word(src, 1);
        let off = word_append_offset(&sm, body);
        assert_eq!(off, u32::try_from(src.len()).unwrap());
    }

    #[test]
    fn append_offset_after_a_command_substitution() {
        let src = "catch [get body]";
        let sm = SourceMap::new(src);
        let body = nth_word(src, 1);
        assert_eq!(body.kind, TokenType::Cmd);
        let off = word_append_offset(&sm, body);
        assert_eq!(off, u32::try_from(src.len()).unwrap());
    }

    #[test]
    fn span_closer_agrees_with_the_token_accessor() {
        // The token-free sibling must land on the same byte as
        // `word_closer_offset` for every word shape the family covers —
        // empty pairs, escaped closers, quoted words with escapes, and a
        // word whose last inner byte is a nested pair's closer.
        for src in [
            "puts {abc}",
            "puts {}",
            "puts {a\\}}",
            "puts {$x eq {}}",
            "puts [x]",
            "puts []",
            "puts \"ab\"",
            "puts \"\"",
            "puts \"a\\\"b\"",
            "puts plain",
            "puts {abc",
        ] {
            let sm = SourceMap::new(src);
            let tok = nth_word(src, 1);
            assert_eq!(
                word_closer_offset_at(src, tok.span),
                word_closer_offset(&sm, tok),
                "closer offset for {src:?}",
            );
            assert_eq!(
                word_span_at(src, tok.span),
                word_span(&sm, tok),
                "word span for {src:?}",
            );
        }
    }

    #[test]
    fn span_word_covers_a_nested_empty_pair() {
        // Issue #1423: the span `{$x eq {}` already *ends* in a `}` — the
        // inner empty pair's closer — so a textual "does it end with `}`?"
        // test skips the widening the outer word really needs and the
        // rewrite target loses the outer brace.
        let src = "while {$x eq {}} {}";
        let sm = SourceMap::new(src);
        let cond = nth_word(src, 1);
        assert_eq!(&src[cond.span.as_range()], "{$x eq {}");
        let widened = word_span_at(src, cond.span);
        assert_eq!(&src[widened.as_range()], "{$x eq {}}");
        assert_eq!(widened, word_span(&sm, cond));
    }

    #[test]
    fn span_word_does_not_overshoot_a_nested_empty_pair() {
        // The `{}` argument's own span is already whole: widening it would
        // swallow the enclosing body's `}`.
        let src = "a {}}";
        assert_eq!(word_span_at(src, Span::new(2, 4)), Span::new(2, 4));
        assert_eq!(word_closer_offset_at(src, Span::new(2, 4)), Some(3));
    }

    #[test]
    fn span_word_does_not_overshoot_an_empty_command_substitution() {
        // `[x []]` — the inner empty `[]`'s span already covers its own
        // `]`, so widening it swallows the enclosing substitution's closer.
        // The optimiser's own deleted `full_word_span` byte-counter widened
        // here on the bare "next byte is the expected closer" rule.
        let src = "[x []]";
        assert_eq!(&src[Span::new(3, 5).as_range()], "[]");
        assert_eq!(word_span_at(src, Span::new(3, 5)), Span::new(3, 5));
    }

    #[test]
    fn span_word_covers_a_braced_variable() {
        // The `${name}` shape `word_closer_offset` cannot key off the
        // opening byte for: the span already encodes the lexer's
        // dialect-resolved name, so the closer is recoverable from it.
        assert_eq!(word_span_at("${x}", Span::new(0, 3)), Span::new(0, 4));
        // `${a{b}}` under Tcl 9 brace-nesting names `a{b}` — the span stops
        // before the *outer* `}`, exactly as the ordinary form does.
        assert_eq!(word_span_at("${a{b}}", Span::new(0, 6)), Span::new(0, 7));
    }

    #[test]
    fn span_word_leaves_the_degenerate_empty_variable_alone() {
        // `${}` immediately followed by an unrelated literal `}`: the
        // lexer already extended the span over the empty name's own
        // closer, so widening again would swallow the literal.
        assert_eq!(word_span_at("${}}", Span::new(0, 3)), Span::new(0, 3));
        assert_eq!(word_closer_offset_at("${}}", Span::new(0, 3)), Some(2));
    }

    #[test]
    fn span_word_leaves_undelimited_and_unterminated_words_alone() {
        assert_eq!(word_span_at("$x", Span::new(0, 2)), Span::new(0, 2));
        assert_eq!(word_span_at("plain", Span::new(0, 5)), Span::new(0, 5));
        assert_eq!(word_span_at("[x", Span::new(0, 2)), Span::new(0, 2));
        assert_eq!(word_span_at("${x", Span::new(0, 3)), Span::new(0, 3));
        assert_eq!(word_closer_offset_at("abc", Span::new(0, 0)), None);
        assert_eq!(word_closer_offset_at("abc", Span::new(0, 9)), None);
    }

    #[test]
    fn close_quote_plain_word() {
        let src = "\"abc\"";
        assert_eq!(close_quote_offset(src, 0), Some(4));
    }

    #[test]
    fn close_quote_empty_word() {
        // `""` — the closer is the very next byte.
        assert_eq!(close_quote_offset("\"\"", 0), Some(1));
    }

    #[test]
    fn close_quote_skips_escaped_quote() {
        // `"a\"b"` — the `\"` is content, not the closer.
        let src = "\"a\\\"b\"";
        assert_eq!(close_quote_offset(src, 0), Some(5));
        assert_eq!(src.as_bytes()[5], b'"');
    }

    #[test]
    fn close_quote_skips_multi_byte_escape() {
        // `\x41` is one escape unit; a naive "+2" would resume on `4`.
        let src = "\"\\x41\"";
        assert_eq!(close_quote_offset(src, 0), Some(5));
    }

    #[test]
    fn close_quote_skips_quote_inside_command_substitution() {
        // The issue #1424 shape: the `"b"` belongs to the substituted
        // command, so the word closes at the final `"`.
        let src = "\"a[foo \"b\"]c\"";
        let close = close_quote_offset(src, 0).unwrap();
        assert_eq!(close, src.len() - 1);
        assert_eq!(&src[..=close], src);
    }

    #[test]
    fn close_quote_skips_nested_command_substitutions() {
        // `[a [b "c"] d]` — brackets nest and the inner quote is inert.
        let src = "\"x[a [b \"c\"] d]y\"";
        let close = close_quote_offset(src, 0).unwrap();
        assert_eq!(close, src.len() - 1);
    }

    #[test]
    fn close_quote_ignores_escaped_open_bracket() {
        // `\[` is a literal `[`, so it opens no substitution and the
        // following `"` closes the word.
        let src = "\"a\\[b\" tail";
        assert_eq!(close_quote_offset(src, 0), Some(5));
    }

    #[test]
    fn close_quote_ignores_bracket_inside_braced_word_of_a_substitution() {
        // `[puts {]}]` — the `]` inside the braced word is inert, so the
        // substitution ends at the final `]`.
        let src = "\"a[puts {]}]b\"";
        let close = close_quote_offset(src, 0).unwrap();
        assert_eq!(close, src.len() - 1);
    }

    #[test]
    fn close_quote_spans_a_comment_hiding_the_substitution_closer() {
        // The reported shape.  C Tcl (verified against tclsh 8.6 and 9.0)
        // yields `abc` here, so the `]` inside `# ] comment` is inert and
        // the word runs to the final `"`.  Closing at the quote that opens
        // the inner `"b"` would make O129 and the minifier rewrite over the
        // middle of valid source.
        let src = "\"a[\n# ] comment\nset y \"b\"\n]c\"";
        let close = close_quote_offset(src, 0).unwrap();
        assert_eq!(close, src.len() - 1);
        assert_eq!(&src[..=close], src);
    }

    #[test]
    fn close_quote_comment_bracket_does_not_close_the_substitution() {
        // Minimal form of the same bug: the only `]` before `format` is
        // commented out.
        let src = "\"a[\n# ]\nformat b]c\"";
        assert_eq!(close_quote_offset(src, 0), Some(src.len() - 1));
    }

    #[test]
    fn close_quote_comment_may_open_the_substitution() {
        // `[` puts the scanner at command position, so a `#` immediately
        // after it comments.
        let src = "\"a[# ]\nformat b]c\"";
        assert_eq!(close_quote_offset(src, 0), Some(src.len() - 1));
    }

    #[test]
    fn close_quote_hash_mid_command_is_not_a_comment() {
        // `#` away from command position is an ordinary word character, so
        // the `]` after it closes normally and the word ends at offset 13 —
        // not at the quote opening `"tail"`.
        let src = "\"a[foo #bar]c\" \"tail\"";
        assert_eq!(close_quote_offset(src, 0), Some(13));
        assert_eq!(&src[..=13], "\"a[foo #bar]c\"");
    }

    #[test]
    fn close_quote_comment_continues_over_a_backslash_newline() {
        // A comment line ending in `\` swallows the next line too, so the
        // `]` opening that continuation line stays inert.
        let src = "\"a[\n# one \\\n] still comment\nformat b]c\"";
        assert_eq!(close_quote_offset(src, 0), Some(src.len() - 1));
    }

    #[test]
    fn close_quote_backslash_newline_keeps_command_position() {
        // The reviewer's reproducer on PR #1481: the line between the `[` and
        // the `#` is a lone backslash. The continuation substitutes to a
        // space, so the `#` is still at command position and the `]` it
        // comments out is inert. Clearing command position there closed the
        // substitution on that `]`, and O129 then anchored on the `"` opening
        // the inner `"b"` and rewrote over valid source.
        let src = "\"x[string length abc]a[\n\\\n# ] comment\nset y \"b\"\n]c\"";
        assert_eq!(close_quote_offset(src, 0), Some(src.len() - 1));
    }

    #[test]
    fn close_quote_backslash_crlf_keeps_command_position() {
        // The same rule for the CRLF spelling of the continuation, whose
        // trailing spaces and tabs belong to the escape too. The later
        // `"b"` is what makes the assertion bite: closing on the commented
        // `]` would report the quote opening it as this word's closer.
        let src = "\"a[\\\r\n  # ]\nset y \"b\"\n]c\"";
        assert_eq!(close_quote_offset(src, 0), Some(src.len() - 1));
    }

    #[test]
    fn close_quote_ordinary_escape_ends_command_position() {
        // Only a continuation is whitespace. `\q` is content, so the `#`
        // after it is an ordinary word character and the `]` beside it closes
        // the substitution as usual — the word must not run on to the quote
        // opening `"tail"`.
        let src = "\"a[\\q#bar]c\" \"tail\"";
        assert_eq!(close_quote_offset(src, 0), Some(11));
        assert_eq!(&src[..=11], "\"a[\\q#bar]c\"");
    }

    #[test]
    fn close_quote_hash_inside_a_braced_word_is_not_a_comment() {
        // `{x\n# y}` is a braced word: treating the `#` as a comment would
        // swallow the `}` that closes it, leaving the substitution
        // unterminated.
        let src = "\"a[string length {x\n# y}]c\"";
        assert_eq!(close_quote_offset(src, 0), Some(src.len() - 1));
    }

    #[test]
    fn close_quote_hash_inside_a_quoted_word_is_not_a_comment() {
        // A newline inside a `"…"` word of the substituted command is not a
        // command boundary, so the following `#` is literal text and the
        // `]` beside it is inert.
        let src = "\"a[string cat \"x\n#]\" ]c\"";
        assert_eq!(close_quote_offset(src, 0), Some(src.len() - 1));
    }

    #[test]
    fn close_quote_semicolon_opens_command_position() {
        // `;` ends a command just as a newline does, so `# ]` after it is a
        // comment; the word must reach the final `"`, not the one opening
        // the inner `"x"`.
        let src = "\"a[foo; # ]\nbar \"x\"]c\"";
        assert_eq!(close_quote_offset(src, 0), Some(src.len() - 1));
    }

    #[test]
    fn close_quote_substitution_commented_to_end_of_input_is_none() {
        // The only `]` is commented out and none follows: unterminated, so
        // no offset can be trusted.
        assert_eq!(close_quote_offset("\"a[\n# ]\nfoo", 0), None);
    }

    #[test]
    fn close_quote_unterminated_word_is_none() {
        assert_eq!(close_quote_offset("\"abc", 0), None);
    }

    #[test]
    fn close_quote_unterminated_command_substitution_is_none() {
        // No trustworthy closer exists once the `[` never closes.
        assert_eq!(close_quote_offset("\"a[foo \"", 0), None);
    }

    #[test]
    fn command_substitution_end_requires_a_complete_open_bracket() {
        let src = "[set x 1; # ] comment\nHTTP::host]";
        assert_eq!(command_substitution_end(src, 0), Some(src.len()));
        assert_eq!(command_substitution_end(src, 1), None);
        assert_eq!(command_substitution_end("[unterminated", 0), None);
    }

    #[test]
    fn close_quote_requires_an_opening_quote() {
        assert_eq!(close_quote_offset("plain", 0), None);
        assert_eq!(close_quote_offset("\"abc\"", 99), None);
    }

    #[test]
    fn close_quote_from_a_mid_source_offset() {
        // The optimiser/minifier call shape: an offset into a larger script.
        let src = "puts \"$a $b\" ; puts done";
        assert_eq!(close_quote_offset(src, 5), Some(11));
    }

    #[test]
    fn close_quote_agrees_with_the_token_accessor() {
        // Where a quoted word lexes as a single token, the token-bearing
        // sibling must land on the same byte — including the escaped-quote
        // shape that trips a naive "first `\"` wins" scan.
        for src in ["puts \"ab\"", "puts \"a\\\"b\"", "puts \"\""] {
            let sm = SourceMap::new(src);
            let tok = nth_word(src, 1);
            let by_offset = close_quote_offset(src, tok.span.start() as usize).unwrap();
            assert_eq!(
                word_closer_offset(&sm, tok),
                Some(u32::try_from(by_offset).unwrap()),
                "for {src:?}",
            );
        }
    }

    #[test]
    fn append_offset_of_an_unterminated_word_stays_in_bounds() {
        let src = "catch {abc";
        let sm = SourceMap::new(src);
        let body = nth_word(src, 1);
        let off = word_append_offset(&sm, body);
        assert!(
            off as usize <= src.len(),
            "append point must stay in bounds"
        );
    }
}
