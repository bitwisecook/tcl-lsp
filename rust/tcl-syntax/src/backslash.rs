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

//! Tcl backslash-escape decoding — the canonical decoder.
//!
//! The lexer owns lexical escape fragments and their selected extents. Native
//! source values use [`native_source_string_bytes_in`], with an independently
//! selected string recipe. Unicode presentation uses [`decode_in`]. These
//! outputs have separate native-unit encoders and share one lexical grammar.
//!
//! The grammar is **release-variant** ([`EscapeSyntax`]): TIP 388 (8.6) capped
//! `\x` at two hex digits, added `\U`, and guarded the octal third digit, so
//! `\x4142` is `B` under 8.5 and `A42` from 8.6. Each entry point comes in two
//! forms — a bare name pinned to Tcl 9.0, for consumers with no release in
//! scope, and an `_in` form taking the release. Decode and extent must always
//! use the *same* form: an escape's width and its value come from one scan.

use std::borrow::Cow;

pub use tcl_dialect::EscapeSyntax;
pub use tcl_lexer::backslash_escape_end as escape_end;
pub use tcl_lexer::backslash_escape_end_in as escape_end_in;
pub use tcl_lexer::backslash_subst as decode;
pub use tcl_lexer::backslash_subst_in as decode_in;

/// Decode escapes in original string bytes under Tcl 9.0's grammar.
/// Backslash-free input borrows its exact bytes. A release-aware runtime uses
/// [`decode_bytes_in`] with its retained escape grammar.
#[must_use]
pub fn decode_bytes(raw: &[u8]) -> Cow<'_, [u8]> {
    decode_bytes_in(raw, EscapeSyntax::default())
}

/// Byte-valued lexical presentation under the selected escape grammar.
/// This entry point does not authenticate C native string units; a native
/// source consumer uses [`native_source_string_bytes_in`] instead.
#[must_use]
pub fn decode_bytes_in(raw: &[u8], escapes: EscapeSyntax) -> Cow<'_, [u8]> {
    tcl_lexer::backslash_subst_bytes_in(raw, escapes)
}

/// Collapse Tcl brace-word line continuations: a backslash immediately followed
/// by LF — `\<LF>` — together with any spaces and tabs after it, becomes a
/// single space. Raw `\<CR>` and `\<CRLF>` are data; a source channel may
/// translate CRLF to LF before this parser seam. This is the **only**
/// backslash processing a `{braced}` word undergoes (every other backslash byte
/// stays literal), and it matches C's pre-pass rule for the backslash-newline
/// sequence, which the `Tcl` language summary notes applies *even inside
/// braces*. An escaped backslash (`\\`) before a newline is a literal `\\` and
/// does not start a continuation. Borrows unchanged when the input contains no
/// continuation (the common case).
#[must_use]
pub fn collapse_brace_continuations(raw: &[u8]) -> Cow<'_, [u8]> {
    collapse_continuations(raw, false, tcl_lexer::SourceChannel::NativeValue)
}

/// [`collapse_brace_continuations`] for word-*separator* contexts: the spaces
/// and tabs **preceding** the backslash collapse too, so a whole
/// `<ws>\<newline><ws>` run becomes a single space. Between the words of a
/// command (a command substitution `[…]`, a bare word) the run is one
/// inter-word separator the parser collapses; inside a `{braced}` or
/// `"quoted"` word the preceding whitespace is string data, which is why
/// [`collapse_brace_continuations`] keeps it. Borrows unchanged when the input
/// contains no continuation.
#[must_use]
pub fn collapse_separator_continuations(raw: &[u8]) -> Cow<'_, [u8]> {
    collapse_continuations(raw, true, tcl_lexer::SourceChannel::NativeValue)
}

/// The shared continuation collapse; `trim_preceding` selects the separator
/// rule (drop spaces/tabs already emitted before the backslash).
fn collapse_continuations(
    raw: &[u8],
    trim_preceding: bool,
    channel: tcl_lexer::SourceChannel,
) -> Cow<'_, [u8]> {
    if !raw
        .iter()
        .enumerate()
        .any(|(i, _)| tcl_lexer::source_backslash_continuation_end(raw, i, channel).is_some())
    {
        return source_literal_bytes(raw, channel);
    }
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'\\' {
            match tcl_lexer::source_backslash_continuation_end(raw, i, channel) {
                Some(end) => {
                    i = end;
                    if trim_preceding {
                        while matches!(out.last(), Some(b' ' | b'\t')) {
                            out.pop();
                        }
                    }
                    out.push(b' ');
                    continue;
                }
                // A non-continuation backslash escapes the next byte for
                // scanning purposes, so `\\<newline>` stays a literal `\\` +
                // newline rather than the second backslash starting a
                // continuation.
                None if let Some(&b) = raw.get(i + 1) => {
                    out.push(b'\\');
                    out.push(b);
                    i += 2;
                    continue;
                }
                None => {
                    out.push(b'\\');
                    i += 1;
                    continue;
                }
            }
        }
        if channel == tcl_lexer::SourceChannel::Document && raw[i] == b'\r' {
            out.push(b'\n');
            i += usize::from(raw.get(i + 1) == Some(&b'\n'));
        } else {
            out.push(raw[i]);
        }
        i += 1;
    }
    Cow::Owned(out)
}

/// Literal value bytes selected by the original source channel. This produces
/// a value view only; it never replaces the original source or its spans.
#[must_use]
pub fn source_literal_bytes(raw: &[u8], channel: tcl_lexer::SourceChannel) -> Cow<'_, [u8]> {
    if channel != tcl_lexer::SourceChannel::Document || !raw.contains(&b'\r') {
        return Cow::Borrowed(raw);
    }
    let mut value = Vec::with_capacity(raw.len());
    let mut at = 0;
    while at < raw.len() {
        if raw[at] == b'\r' {
            value.push(b'\n');
            at += usize::from(raw.get(at + 1) == Some(&b'\n'));
        } else {
            value.push(raw[at]);
        }
        at += 1;
    }
    Cow::Owned(value)
}

/// Braced literal value under an explicit source channel and brace rule.
/// Every span still belongs to the original image; only returned value bytes
/// reflect channel translation and the selected continuation-folding rule.
#[must_use]
pub fn source_braced_word_bytes(
    raw: &[u8],
    channel: tcl_lexer::SourceChannel,
    rule: tcl_dialect::BraceBackslashNewline,
) -> Cow<'_, [u8]> {
    if rule.folds() {
        collapse_continuations(raw, false, channel)
    } else {
        source_literal_bytes(raw, channel)
    }
}

/// [`collapse_brace_continuations`], gated on the dialect's
/// [`BraceBackslashNewline`](tcl_dialect::BraceBackslashNewline) rule.
///
/// Every build of the Tcl core folds — `TclCopyAndCollapse` rewrites the
/// backslash-newline run to one space, so `{a\<newline>b}` is `a b`.
/// `JimTcl` keeps the bytes (`JimParseSubBrace`, jim.c:1444-1485),
/// deliberately, so line numbers survive a braced body.
///
/// Measured: `string length {a\<newline>b}` is 3 under tclsh 8.6 and 9.0,
/// and 4 under every modelled Jim release.
///
/// Under `Literal` this borrows the input unchanged. Downstream *list*
/// parsing still applies its own element escapes, which is why not folding
/// here is enough to reproduce Jim end to end.
#[must_use]
pub fn collapse_brace_continuations_for(
    raw: &[u8],
    rule: tcl_dialect::BraceBackslashNewline,
) -> Cow<'_, [u8]> {
    if rule.folds() {
        collapse_brace_continuations(raw)
    } else {
        Cow::Borrowed(raw)
    }
}

/// [`collapse_brace_continuations_for`] for a `&str`.
#[must_use]
pub fn collapse_brace_continuations_str_for(
    text: &str,
    rule: tcl_dialect::BraceBackslashNewline,
) -> Cow<'_, str> {
    if rule.folds() {
        collapse_brace_continuations_str(text)
    } else {
        Cow::Borrowed(text)
    }
}

/// [`collapse_brace_continuations`] for a `&str`, returning a `Cow<str>`. The
/// collapse only ever rewrites ASCII bytes (`\`, newline, spaces, tabs), so a
/// valid-UTF-8 input always yields valid UTF-8. Borrows when there is no
/// continuation to collapse.
#[must_use]
pub fn collapse_brace_continuations_str(text: &str) -> Cow<'_, str> {
    collapsed_bytes_to_str(text, collapse_brace_continuations(text.as_bytes()))
}

/// [`collapse_separator_continuations`] for a `&str`, returning a `Cow<str>`
/// under the same UTF-8-preservation argument as
/// [`collapse_brace_continuations_str`].
#[must_use]
pub fn collapse_separator_continuations_str(text: &str) -> Cow<'_, str> {
    collapsed_bytes_to_str(text, collapse_separator_continuations(text.as_bytes()))
}

/// Rewrap a byte-level collapse of `text` as `Cow<str>`.
fn collapsed_bytes_to_str<'s>(text: &'s str, collapsed: Cow<'s, [u8]>) -> Cow<'s, str> {
    match collapsed {
        Cow::Borrowed(_) => Cow::Borrowed(text),
        Cow::Owned(bytes) => {
            Cow::Owned(String::from_utf8(bytes).expect("continuation collapse preserves UTF-8"))
        }
    }
}

/// Unsupported source recipe or malformed host request, never a guest error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSourceUnavailable {
    /// Source grammar and independently selected string recipe disagree.
    GrammarProtocolMismatch,
    /// The requested position is not a backslash or has no valid bounded extent.
    InvalidEscapePosition,
}

/// Unavailable original executable text, distinct from a guest syntax error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeArenaTextUnavailable {
    /// The supplied component is not a text component of the retained arena.
    NotText,
    /// Its original byte extent is outside the arena's source image.
    SourceGeometry,
    /// The independently selected native source decoder declined the request.
    Source(NativeSourceUnavailable),
}

/// Materialise a text component from its original executable source extent.
/// `component` must come from `arena`; source spans address that arena's image.
/// Original text retains counted native value bytes or returns the explicit
/// Document channel's translated value view. Decoded text runs the native
/// decoder over the original span, rather than treating advisory UTF-8 output
/// as native string units. The string recipe is independent of compiler-local
/// naming and runtime representation receipts.
///
/// # Errors
/// Returns a typed ownership/decoder refusal, never a Tcl completion.
pub fn native_arena_text<'a>(
    arena: &'a tcl_lexer::ExecutablePartArena,
    component: &tcl_lexer::SpannedExecutablePart,
    escapes: EscapeSyntax,
    protocol: crate::native_string::NativeStringProtocol,
) -> Result<Cow<'a, [u8]>, NativeArenaTextUnavailable> {
    let original = arena
        .bytes(component.span)
        .ok_or(NativeArenaTextUnavailable::SourceGeometry)?;
    match &component.part {
        tcl_lexer::ExecutablePart::Text(tcl_lexer::ExecutableText::Original) => {
            Ok(source_literal_bytes(original, arena.image().channel()))
        }
        tcl_lexer::ExecutablePart::Text(tcl_lexer::ExecutableText::Decoded(_)) => {
            native_source_string_bytes_channel_in(
                original,
                arena.image().channel(),
                escapes,
                protocol,
            )
            .map_err(NativeArenaTextUnavailable::Source)
        }
        _ => Err(NativeArenaTextUnavailable::NotText),
    }
}

/// Native bytes for one scanner-selected source escape token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSourceEscape {
    /// End of the original selected token, in the supplied source buffer.
    pub end: usize,
    /// Exact resulting native string units.
    pub bytes: Vec<u8>,
}

/// Decode one source escape through the shared lexer fragment scanner.
///
/// C source scanning selects a token extent before the evaluator decodes that
/// bounded token. The distinction matters for truncated C8.6 astral input:
/// scanning can select a high surrogate's one-byte extent, while decoding the
/// selected token yields that original lead byte's native character unit.
/// Literal bytes after the selected extent remain literal, independently of
/// what the `subst` command does with its remaining string representation.
///
/// # Errors
/// Returns an error for a grammar/recipe mismatch or an invalid requested escape extent.
pub fn native_source_escape_in(
    raw: &[u8],
    pos: usize,
    escapes: EscapeSyntax,
    protocol: crate::native_string::NativeStringProtocol,
) -> Result<NativeSourceEscape, NativeSourceUnavailable> {
    native_source_escape_channel_in(
        raw,
        pos,
        tcl_lexer::SourceChannel::NativeValue,
        escapes,
        protocol,
    )
}

/// Decode one original source escape under its explicit input channel.
///
/// # Errors
/// Returns mismatched decoder policy or unavailable original extent.
pub fn native_source_escape_channel_in(
    raw: &[u8],
    pos: usize,
    channel: tcl_lexer::SourceChannel,
    escapes: EscapeSyntax,
    protocol: crate::native_string::NativeStringProtocol,
) -> Result<NativeSourceEscape, NativeSourceUnavailable> {
    use tcl_lexer::{BackslashFragmentValue, source_backslash_fragment_in};
    if escapes != protocol.escape_syntax() {
        return Err(NativeSourceUnavailable::GrammarProtocolMismatch);
    }
    let scan = source_backslash_fragment_in(raw, pos, channel, escapes, |suffix| {
        native_original_escape_unit(protocol, suffix)
    })
    .ok_or(NativeSourceUnavailable::InvalidEscapePosition)?;
    let decoded = source_backslash_fragment_in(&raw[..scan.end], pos, channel, escapes, |suffix| {
        native_original_escape_unit(protocol, suffix)
    })
    .ok_or(NativeSourceUnavailable::InvalidEscapePosition)?;
    let mut bytes = Vec::new();
    match decoded.value {
        BackslashFragmentValue::Byte(byte) => bytes.push(byte),
        BackslashFragmentValue::Literal(range) => bytes.extend_from_slice(&raw[range]),
        BackslashFragmentValue::Codepoint(value) => match protocol {
            crate::native_string::NativeStringProtocol::Jim084 => {
                tcl_lexer::encode_jim084_unicode(&mut bytes, value);
            }
            crate::native_string::NativeStringProtocol::C(version) => {
                let value = if version < tcl_dialect::TclVersion::V9_0 && value > 0xffff {
                    0xfffd
                } else {
                    value
                };
                crate::native_tcl_utf::NativeTclUtf::for_version(version).encode_unit(value, &mut bytes)
                    .expect("shared numeric scanner and native input-unit decoder select a supported C unit");
            }
        },
    }
    Ok(NativeSourceEscape {
        end: scan.end,
        bytes,
    })
}

/// Encode escaped source content as native string bytes. This does not parse
/// word boundaries, perform substitutions, materialise byte arrays, decode
/// file channels or grant compilation permission. The caller supplies already
/// selected word content and retains its original source/object provenance.
/// Backslash-free content borrows the complete original byte string.
///
/// # Errors
/// Returns an error for a grammar/recipe mismatch or an invalid requested escape extent.
pub fn native_source_string_bytes_in(
    raw: &[u8],
    escapes: EscapeSyntax,
    protocol: crate::native_string::NativeStringProtocol,
) -> Result<Cow<'_, [u8]>, NativeSourceUnavailable> {
    native_source_string_bytes_channel_in(
        raw,
        tcl_lexer::SourceChannel::NativeValue,
        escapes,
        protocol,
    )
}

/// Native value bytes decoded from original source under an explicit channel.
/// Original source geometry and the returned channel-translated value remain
/// distinct; no normalized buffer is used as lexical provenance.
///
/// # Errors
/// Returns a native decoder policy or original escape-extent refusal.
pub fn native_source_string_bytes_channel_in(
    raw: &[u8],
    channel: tcl_lexer::SourceChannel,
    escapes: EscapeSyntax,
    protocol: crate::native_string::NativeStringProtocol,
) -> Result<Cow<'_, [u8]>, NativeSourceUnavailable> {
    if escapes != protocol.escape_syntax() {
        return Err(NativeSourceUnavailable::GrammarProtocolMismatch);
    }
    if !raw.contains(&b'\\') {
        return Ok(source_literal_bytes(raw, channel));
    }
    let mut output = Vec::with_capacity(raw.len());
    let mut pos = 0;
    while pos < raw.len() {
        if raw[pos] == b'\\' {
            let fragment = native_source_escape_channel_in(raw, pos, channel, escapes, protocol)?;
            output.extend_from_slice(&fragment.bytes);
            pos = fragment.end;
        } else {
            if channel == tcl_lexer::SourceChannel::Document && raw[pos] == b'\r' {
                output.push(b'\n');
                pos += usize::from(raw.get(pos + 1) == Some(&b'\n'));
            } else {
                output.push(raw[pos]);
            }
            pos += 1;
        }
    }
    Ok(Cow::Owned(output))
}

fn native_original_escape_unit(
    protocol: crate::native_string::NativeStringProtocol,
    suffix: &[u8],
) -> tcl_lexer::EscapedInputUnit {
    use tcl_lexer::{EscapedInputUnit, EscapedInputValue};
    let crate::native_string::NativeStringProtocol::C(version) = protocol else {
        return EscapedInputUnit {
            width: 1,
            value: EscapedInputValue::CopyOriginal,
        };
    };
    let units = crate::native_tcl_utf::NativeTclUtf::for_version(version);
    let first = units
        .decode_unit(suffix, None)
        .expect("unrecognised escape has an input byte");
    if version == tcl_dialect::TclVersion::V8_6
        && (0xd800..=0xdbff).contains(&first.value)
        && let Some(low) = units.decode_unit(&suffix[first.width..], Some(first.value))
        && (0xdc00..=0xdfff).contains(&low.value)
    {
        return EscapedInputUnit {
            width: first.width + low.width,
            value: EscapedInputValue::Codepoint(
                ((first.value & 0x3ff) << 10 | (low.value & 0x3ff)) + 0x1_0000,
            ),
        };
    }
    EscapedInputUnit {
        width: first.width,
        value: EscapedInputValue::Codepoint(first.value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brace_continuations_collapse_like_tcl9() {
        // `\<newline>` plus following spaces/tabs ⇒ one space.
        assert_eq!(&*collapse_brace_continuations(b"a\\\nb"), b"a b");
        assert_eq!(&*collapse_brace_continuations(b"a\\\n   b"), b"a b");
        assert_eq!(&*collapse_brace_continuations(b"a\\\n\t b"), b"a b");
        // TclParseBackslash has no CR arm: raw CR and CRLF stay byte-exact.
        assert_eq!(&*collapse_brace_continuations(b"a\\\rb"), b"a\\\rb");
        assert_eq!(
            &*collapse_brace_continuations(b"a\\\r\n   b"),
            b"a\\\r\n   b"
        );
        assert_eq!(&*collapse_brace_continuations(b"a\\\r\tb"), b"a\\\r\tb");
        // No continuation ⇒ borrowed, byte-identical (including a literal `\n`).
        assert!(matches!(
            collapse_brace_continuations(b"p\\nq"),
            Cow::Borrowed(_)
        ));
        // An escaped backslash before a newline is not a continuation.
        assert_eq!(&*collapse_brace_continuations(b"x\\\\\ny"), b"x\\\\\ny");
        assert_eq!(&*collapse_brace_continuations(b"x\\\\\r\ny"), b"x\\\\\r\ny");
        // Two continuations in a row ⇒ two spaces (each `\<eol>` → one space).
        assert_eq!(&*collapse_brace_continuations(b"m\\\n\\\nn"), b"m  n");
    }

    #[test]
    fn brace_continuations_str_matches_bytes_and_borrows() {
        assert_eq!(&*collapse_brace_continuations_str("a\\\n   b"), "a b");
        assert_eq!(&*collapse_brace_continuations_str("a\\\r\nb"), "a\\\r\nb");
        // No continuation ⇒ borrowed (no allocation), including a literal `\t`.
        assert!(matches!(
            collapse_brace_continuations_str("a\\tb"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn separator_continuations_trim_preceding_whitespace() {
        // Word-separator context: the whole `<ws>\<eol><ws>` run is ONE
        // separator, so the spaces/tabs before the backslash collapse too —
        // for LF. Raw CR/CRLF are ordinary escaped data.
        assert_eq!(&*collapse_separator_continuations(b"a \\\n  b"), b"a b");
        assert_eq!(
            &*collapse_separator_continuations(b"a\t \\\r\n\t b"),
            b"a\t \\\r\n\t b"
        );
        assert_eq!(
            &*collapse_separator_continuations(b"a \\\r  b"),
            b"a \\\r  b"
        );
        // Contrast: the brace/quote rule keeps the preceding space as data.
        assert_eq!(&*collapse_brace_continuations(b"a \\\n  b"), b"a  b");
        // FP guards: no continuation borrows unchanged (preceding whitespace
        // untouched), and an escaped backslash is not a continuation.
        assert!(matches!(
            collapse_separator_continuations(b"a  b"),
            Cow::Borrowed(_)
        ));
        assert_eq!(
            &*collapse_separator_continuations(b"x \\\\\ny"),
            b"x \\\\\ny"
        );
    }

    #[test]
    fn separator_continuations_str_matches_bytes() {
        assert_eq!(
            &*collapse_separator_continuations_str("a \\\r\n b"),
            "a \\\r\n b"
        );
        assert!(matches!(
            collapse_separator_continuations_str("a\\tb"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn bytes_decode_matches_tcl9() {
        assert_eq!(&*decode_bytes(b"a\\tb"), b"a\tb");
        // `\xff` is U+00FF ⇒ two UTF-8 bytes (the old bs.rs emitted one raw byte).
        assert_eq!(&*decode_bytes(b"\\xff"), "\u{FF}".as_bytes());
        assert_eq!(&*decode_bytes(b"\\u00e9"), "é".as_bytes());
        // no backslash ⇒ borrowed, byte-identical.
        assert!(matches!(decode_bytes(b"plain"), Cow::Borrowed(_)));
    }

    #[test]
    fn decode_control_escapes_match_tclsh() {
        // Codepoints verified against tclsh8.6/9.0 (`scan "\X" %c`):
        // \a=7 \b=8 \f=12 \n=10 \r=13 \t=9 \v=11 \\=92.
        assert_eq!(&*decode("\\a"), "\u{07}");
        assert_eq!(&*decode("\\b"), "\u{08}");
        assert_eq!(&*decode("\\f"), "\u{0c}");
        assert_eq!(&*decode("\\n"), "\n");
        assert_eq!(&*decode("\\r"), "\r");
        assert_eq!(&*decode("\\t"), "\t");
        assert_eq!(&*decode("\\v"), "\u{0b}");
        assert_eq!(&*decode("\\\\"), "\\");
    }

    #[test]
    fn decode_numeric_escapes_match_tclsh() {
        // tclsh: \x41=A (exactly 2 hex), \101=A (octal), A=A (1-4 hex).
        assert_eq!(&*decode("\\x41"), "A");
        assert_eq!(&*decode("\\101"), "A");
        assert_eq!(&*decode("\\u0041"), "A");
        // \x consumes exactly two hex digits, so `\x4142` → "A42" (tclsh: the
        // string-length is 3, not 1).
        assert_eq!(&*decode("\\x4142"), "A42");
        // \xff → U+00FF (Tcl 9 / UTF-8 internal rep), not a raw 0xFF byte.
        assert_eq!(&*decode("\\xff"), "\u{FF}");
        // \u with fewer than 4 hex digits still decodes (\u41 → A).
        assert_eq!(&*decode("\\u41"), "A");
    }

    #[test]
    fn decode_bytes_follows_the_release() {
        // The user-visible symptom of issue #1479: a script pinned to 8.4/8.5
        // reads `\x4142` as `B` (all trailing hex digits, low byte), 8.6+ as
        // `A42` (TIP 388's two-digit cap). `\U` exists only from 8.6, and 8.6's
        // stock UTF-16-internal build degrades an astral scalar to U+FFFD.
        assert_eq!(&*decode_bytes_in(b"\\x4142", EscapeSyntax::Tcl84), b"B");
        assert_eq!(&*decode_bytes_in(b"\\x4142", EscapeSyntax::Tcl86), b"A42");
        assert_eq!(&*decode_bytes_in(b"\\x4142", EscapeSyntax::Tcl90), b"A42");
        assert_eq!(
            &*decode_bytes_in(b"\\U0001F600", EscapeSyntax::Tcl84),
            b"U0001F600"
        );
        assert_eq!(
            &*decode_bytes_in(b"\\U0001F600", EscapeSyntax::Tcl86),
            "\u{FFFD}".as_bytes()
        );
        assert_eq!(
            &*decode_bytes_in(b"\\U0001F600", EscapeSyntax::Tcl90),
            "\u{1F600}".as_bytes()
        );
        // The release-blind entry point is the 9.0 one.
        assert_eq!(
            &*decode_bytes(b"\\x4142"),
            &*decode_bytes_in(b"\\x4142", EscapeSyntax::Tcl90)
        );
    }

    #[test]
    fn escape_extent_follows_the_release() {
        // Width and value must agree per release, or a per-escape scanner
        // slices mid-escape.
        assert_eq!(escape_end_in("\\x4142", 0, EscapeSyntax::Tcl84), 6);
        assert_eq!(escape_end_in("\\x4142", 0, EscapeSyntax::Tcl90), 4);
        assert_eq!(escape_end_in("\\U0001F600", 0, EscapeSyntax::Tcl84), 2);
        assert_eq!(escape_end_in("\\U0001F600", 0, EscapeSyntax::Tcl90), 10);
        assert_eq!(
            escape_end("\\x4142", 0),
            escape_end_in("\\x4142", 0, EscapeSyntax::Tcl90)
        );
    }

    #[test]
    fn decode_unknown_escape_drops_backslash() {
        // tclsh: an unknown escape keeps the character (`\q` → `q`).
        assert_eq!(&*decode("\\q"), "q");
        // Escaped quote/brace are literal.
        assert_eq!(&*decode("\\\""), "\"");
        assert_eq!(&*decode("\\{"), "{");
    }

    #[test]
    fn decode_no_backslash_borrows() {
        // Nothing to decode ⇒ a borrowed slice (no allocation).
        assert!(matches!(decode("plain text"), Cow::Borrowed(_)));
    }

    #[test]
    fn decode_bytes_passes_through_non_utf8() {
        // A non-UTF-8 byte slice cannot be a well-formed internal rep, but the
        // defensive branch borrows it unchanged rather than panicking.
        let raw = [0xff, 0xfe, b'a'];
        assert!(matches!(decode_bytes(&raw), Cow::Borrowed(_)));
        assert_eq!(&*decode_bytes(&raw), &raw);
    }
}

#[cfg(test)]
mod native_source_tests {
    use super::*;
    use crate::native_string::{NativeStringInput, NativeStringProtocol};
    use tcl_dialect::TclVersion;

    fn unhex(text: &str) -> Vec<u8> {
        assert_eq!(text.len() % 2, 0);
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let digit = |byte| {
                    char::from(byte)
                        .to_digit(16)
                        .expect("native vector has hex bytes")
                };
                u8::try_from(digit(pair[0]) * 16 + digit(pair[1]))
                    .expect("two hex digits fit a byte")
            })
            .collect()
    }

    #[test]
    fn source_values_match_actual_c84_through_c91_and_current_jim() {
        let protocols = [
            NativeStringProtocol::C(TclVersion::V8_4),
            NativeStringProtocol::C(TclVersion::V8_5),
            NativeStringProtocol::C(TclVersion::V8_6),
            NativeStringProtocol::C(TclVersion::V9_0),
            NativeStringProtocol::C(TclVersion::V9_1),
            NativeStringProtocol::Jim084,
        ];
        let vectors = include_str!("../test-data/native-source-bytes.tsv");
        let mut count = 0;
        for row in vectors.lines().filter(|row| !row.starts_with('#')) {
            let columns: Vec<_> = row.split('\t').collect();
            assert_eq!(columns.len(), 8);
            let original = unhex(columns[0]);
            for (index, protocol) in protocols.into_iter().enumerate() {
                let actual =
                    native_source_string_bytes_in(&original, protocol.escape_syntax(), protocol)
                        .unwrap();
                assert_eq!(
                    actual.as_ref(),
                    unhex(columns[index + 1]),
                    "{} under {protocol:?}",
                    columns[7]
                );
            }
            count += 1;
        }
        assert_eq!(count, 38);
    }

    #[test]
    fn materialisation_source_escape_and_literal_bytes_keep_separate_inputs() {
        let protocol = NativeStringProtocol::C(TclVersion::V9_0);
        assert_eq!(
            native_source_string_bytes_in(&[0], protocol.escape_syntax(), protocol)
                .unwrap()
                .as_ref(),
            &[0]
        );
        assert_eq!(
            native_source_string_bytes_in(br"\u0000", protocol.escape_syntax(), protocol)
                .unwrap()
                .as_ref(),
            &[0xc0, 0x80]
        );
        assert_eq!(
            protocol
                .materialize(NativeStringInput::PureByteArray(&[0]))
                .unwrap()
                .as_ref(),
            &[0xc0, 0x80]
        );
        assert_eq!(
            protocol
                .materialize(NativeStringInput::ResidentString(&[0]))
                .unwrap()
                .as_ref(),
            &[0]
        );
        assert_eq!(
            native_source_string_bytes_in(br"\u0000", EscapeSyntax::Jim, protocol),
            Err(NativeSourceUnavailable::GrammarProtocolMismatch)
        );
        assert_eq!(
            native_source_escape_in(b"ordinary", 0, protocol.escape_syntax(), protocol),
            Err(NativeSourceUnavailable::InvalidEscapePosition)
        );
    }

    #[test]
    fn c86_selected_token_decode_retains_original_literal_suffix() {
        let protocol = NativeStringProtocol::C(TclVersion::V8_6);
        let source = [b'\\', 0xf0, 0x9f, 0x98];
        let escape =
            native_source_escape_in(&source, 0, protocol.escape_syntax(), protocol).unwrap();
        assert_eq!(escape.end, 2);
        assert_eq!(escape.bytes, [0xc3, 0xb0]);
        assert_eq!(
            native_source_string_bytes_in(&source, protocol.escape_syntax(), protocol)
                .unwrap()
                .as_ref(),
            &[0xc3, 0xb0, 0x9f, 0x98]
        );
    }
}

#[cfg(test)]
mod executable_text_tests {
    use super::*;
    use tcl_lexer::{ExecutablePartArena, LexerConfig, SourceImage, Span, SubstFlags};

    #[test]
    fn source_channel_changes_value_bytes_without_changing_original_extents() {
        use tcl_lexer::SourceChannel::{Document, NativeValue};
        let protocol = crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1);
        let config =
            LexerConfig::from_grammar(tcl_dialect::DialectProfile::find("tcl9.1").unwrap().grammar);
        let source = b"A\\\r\n\t B";
        for (channel, expected) in [
            (Document, b"A B".as_slice()),
            (NativeValue, b"A\r\n\t B".as_slice()),
        ] {
            let image = SourceImage::from_bytes(source.as_slice(), channel);
            let arena = ExecutablePartArena::decompose(
                image.clone(),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                SubstFlags::default(),
                config,
            )
            .unwrap();
            let part = &arena.list(arena.root())[0];
            assert_eq!(
                native_arena_text(&arena, part, config.escapes, protocol)
                    .unwrap()
                    .as_ref(),
                expected
            );
            assert_eq!(arena.image(), &image);
            assert_eq!(arena.bytes(part.span), Some(source.as_slice()));
        }
        assert_eq!(
            source_braced_word_bytes(source, Document, config.brace_backslash_newline).as_ref(),
            b"A B"
        );
        assert_eq!(
            source_braced_word_bytes(source, NativeValue, config.brace_backslash_newline).as_ref(),
            source
        );
        assert_eq!(source_literal_bytes(b"A\r\nB", Document).as_ref(), b"A\nB");
        assert_eq!(
            source_literal_bytes(b"A\r\nB", NativeValue).as_ref(),
            b"A\r\nB"
        );
    }

    #[test]
    fn original_arena_text_keeps_raw_nul_separate_from_escaped_nul() {
        for (name, protocol, escaped) in [
            (
                "tcl8.4",
                crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
                b"\xc0\x80".as_slice(),
            ),
            (
                "tcl8.5",
                crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
                b"\xc0\x80".as_slice(),
            ),
            (
                "tcl8.6",
                crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
                b"\xc0\x80".as_slice(),
            ),
            (
                "tcl9.0",
                crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
                b"\xc0\x80".as_slice(),
            ),
            (
                "tcl9.1",
                crate::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
                b"\xc0\x80".as_slice(),
            ),
            (
                "jimtcl",
                crate::native_string::NativeStringProtocol::Jim084,
                b"\0".as_slice(),
            ),
        ] {
            let grammar = if protocol == crate::native_string::NativeStringProtocol::Jim084 {
                tcl_dialect::model::grammar(
                    tcl_dialect::model::Family::Jim,
                    tcl_dialect::model::Release::JIM_0_84,
                )
            } else {
                tcl_dialect::DialectProfile::find(name).unwrap().grammar
            };
            let config = LexerConfig::from_grammar(grammar);
            for (source, expected) in [
                (b"\\u0000".as_slice(), escaped),
                (b"\0".as_slice(), b"\0".as_slice()),
            ] {
                let arena = ExecutablePartArena::decompose(
                    SourceImage::native(source),
                    Span::new(0, u32::try_from(source.len()).unwrap()),
                    SubstFlags::default(),
                    config,
                )
                .unwrap();
                let part = &arena.list(arena.root())[0];
                assert_eq!(
                    native_arena_text(&arena, part, config.escapes, protocol)
                        .unwrap()
                        .as_ref(),
                    expected,
                    "{name}: {source:?}"
                );
            }
        }
    }
}
