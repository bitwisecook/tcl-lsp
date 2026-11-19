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
    collapse_continuations_mapped(raw, trim_preceding, channel, false).bytes
}

struct CollapsedSource<'a> {
    bytes: Cow<'a, [u8]>,
    boundaries: Vec<usize>,
    continuations: Vec<std::ops::Range<usize>>,
}

fn collapse_continuations_mapped(
    raw: &[u8],
    trim_preceding: bool,
    channel: tcl_lexer::SourceChannel,
    mapped: bool,
) -> CollapsedSource<'_> {
    if !raw
        .iter()
        .enumerate()
        .any(|(i, _)| tcl_lexer::source_backslash_continuation_end(raw, i, channel).is_some())
        && !mapped
    {
        return CollapsedSource {
            bytes: source_literal_bytes(raw, channel),
            boundaries: Vec::new(),
            continuations: Vec::new(),
        };
    }
    let mut out = Vec::with_capacity(raw.len());
    let mut boundaries = Vec::new();
    let mut continuations = Vec::new();
    if mapped {
        boundaries.push(0);
    }
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
                        if mapped {
                            boundaries.truncate(out.len() + 1);
                        }
                    }
                    let start = out.len();
                    out.push(b' ');
                    if mapped {
                        boundaries.push(i);
                        continuations.push(start..out.len());
                    }
                    continue;
                }
                // A non-continuation backslash escapes the next byte for
                // scanning purposes, so `\\<newline>` stays a literal `\\` +
                // newline rather than the second backslash starting a
                // continuation.
                None if let Some(&b) = raw.get(i + 1) => {
                    out.push(b'\\');
                    out.push(b);
                    if mapped {
                        boundaries.extend([i + 1, i + 2]);
                    }
                    i += 2;
                    continue;
                }
                None => {
                    out.push(b'\\');
                    if mapped {
                        boundaries.push(i + 1);
                    }
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
        if mapped {
            boundaries.push(i);
        }
    }
    CollapsedSource {
        bytes: Cow::Owned(out),
        boundaries,
        continuations,
    }
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
        let (byte, next) = source_literal_byte(raw, at, channel);
        value.push(byte);
        at = next;
    }
    Cow::Owned(value)
}

fn source_literal_byte(raw: &[u8], at: usize, channel: tcl_lexer::SourceChannel) -> (u8, usize) {
    if channel == tcl_lexer::SourceChannel::Document && raw[at] == b'\r' {
        (b'\n', at + 1 + usize::from(raw.get(at + 1) == Some(&b'\n')))
    } else {
        (raw[at], at + 1)
    }
}

/// Map [`source_literal_bytes`] boundaries to their original source extent.
/// Document offsets must delimit complete UTF-8 characters; CRLF and lone CR
/// use the same newline translation as the value view. `NativeValue` offsets
/// remain counted byte boundaries. This supplies no native units or edit
/// authority and does not interpret escapes.
#[must_use]
pub fn source_literal_extent(
    raw: &[u8],
    channel: tcl_lexer::SourceChannel,
    literal: std::ops::Range<usize>,
) -> Option<std::ops::Range<usize>> {
    if literal.start > literal.end {
        return None;
    }
    if channel == tcl_lexer::SourceChannel::NativeValue {
        return (literal.end <= raw.len()).then_some(literal);
    }
    let text = std::str::from_utf8(raw).ok()?;
    let mut at = 0;
    let mut value_at = 0;
    let mut start = None;
    loop {
        if value_at == literal.start {
            start = Some(at);
        }
        if value_at == literal.end {
            let start = start?;
            return (text.is_char_boundary(start) && text.is_char_boundary(at))
                .then_some(start..at);
        }
        if at == raw.len() {
            return None;
        }
        let (_, next) = source_literal_byte(raw, at, channel);
        at = next;
        value_at += 1;
    }
}

/// Native literal bytes produced by an independently selected source channel.
/// A Unicode Document models UTF-8 character-channel ingress and automatic
/// newline translation. A `NativeValue` is already a counted native string and
/// remains byte-exact. This does not materialise a resident object, select a
/// name purpose, or change any original source coordinate.
///
/// # Errors
/// A Document must contain valid Unicode; arbitrary native bytes use
/// [`tcl_lexer::SourceChannel::NativeValue`].
pub fn native_source_literal_bytes(
    raw: &[u8],
    channel: tcl_lexer::SourceChannel,
    protocol: crate::native_string::NativeStringProtocol,
) -> Result<Cow<'_, [u8]>, NativeSourceUnavailable> {
    if channel == tcl_lexer::SourceChannel::NativeValue {
        return Ok(Cow::Borrowed(raw));
    }
    Ok(native_document_ingress(raw, protocol)?.bytes)
}

/// Map native literal-value boundaries to their original source extent.
/// Only the channel ingress map participates: no escapes, word evaluation,
/// name purpose or edit authority is selected here. A boundary inside a
/// produced native unit cannot acquire an original source coordinate.
#[must_use]
pub fn native_source_literal_extent(
    raw: &[u8],
    channel: tcl_lexer::SourceChannel,
    protocol: crate::native_string::NativeStringProtocol,
    native: std::ops::Range<usize>,
) -> Option<std::ops::Range<usize>> {
    if native.start > native.end {
        return None;
    }
    if channel == tcl_lexer::SourceChannel::NativeValue {
        return (native.end <= raw.len()).then_some(native);
    }
    let ingress = native_document_ingress(raw, protocol).ok()?;
    if native.end > ingress.bytes.len() {
        return None;
    }
    let start = ingress.original_offset(native.start)?;
    let end = ingress.original_offset(native.end)?;
    let text = std::str::from_utf8(raw).ok()?;
    (text.is_char_boundary(start) && text.is_char_boundary(end)).then_some(start..end)
}

/// Map native escaped-text boundaries to the original source extent. Literal
/// runs use the channel ingress map; each scanner-selected escape maps only
/// at the boundaries of its complete result. A byte inside a produced unit or
/// an ambiguous zero-length result has no source coordinate. This selects no
/// word, variable, compiler, edit or execution purpose.
#[must_use]
pub fn native_source_string_extent(
    raw: &[u8],
    channel: tcl_lexer::SourceChannel,
    escapes: EscapeSyntax,
    protocol: crate::native_string::NativeStringProtocol,
    native: std::ops::Range<usize>,
) -> Option<std::ops::Range<usize>> {
    if native.start > native.end || escapes != protocol.escape_syntax() {
        return None;
    }
    if !raw.contains(&b'\\') {
        return native_source_literal_extent(raw, channel, protocol, native);
    }
    let mut position = 0;
    let mut produced = 0_usize;
    let mut start = None;
    let mut end = None;
    while position < raw.len() {
        let (next, length, is_escape_fragment) = if raw[position] == b'\\' {
            let fragment =
                native_source_escape_channel_in(raw, position, channel, escapes, protocol).ok()?;
            (fragment.end, fragment.bytes.len(), true)
        } else {
            let next = raw[position..]
                .iter()
                .position(|&byte| byte == b'\\')
                .map_or(raw.len(), |offset| position + offset);
            let length = native_source_literal_bytes(&raw[position..next], channel, protocol)
                .ok()?
                .len();
            (next, length, false)
        };
        if next <= position {
            return None;
        }
        let produced_end = produced.checked_add(length)?;
        for (boundary, selected) in [(native.start, &mut start), (native.end, &mut end)] {
            if !(produced..=produced_end).contains(&boundary) {
                continue;
            }
            let local = boundary - produced;
            let original = if is_escape_fragment {
                if length == 0 {
                    return None;
                }
                if local == 0 {
                    position
                } else if local == length {
                    next
                } else {
                    return None;
                }
            } else {
                position.checked_add(
                    native_source_literal_extent(
                        &raw[position..next],
                        channel,
                        protocol,
                        local..local,
                    )?
                    .start,
                )?
            };
            if selected.is_some_and(|previous| previous != original) {
                return None;
            }
            *selected = Some(original);
        }
        position = next;
        produced = produced_end;
    }
    Some(start?..end?)
}

/// Braced source value with native channel production and independently
/// retained continuation grammar. Literal backslashes remain literal.
///
/// # Errors
/// Returns the same Document Unicode refusal as [`native_source_literal_bytes`].
pub fn native_source_braced_word_bytes(
    raw: &[u8],
    channel: tcl_lexer::SourceChannel,
    rule: tcl_dialect::BraceBackslashNewline,
    protocol: crate::native_string::NativeStringProtocol,
) -> Result<Cow<'_, [u8]>, NativeSourceUnavailable> {
    match source_braced_word_bytes(raw, channel, rule) {
        Cow::Borrowed(bytes) => native_source_literal_bytes(bytes, channel, protocol),
        Cow::Owned(bytes) => Ok(Cow::Owned(
            native_source_literal_bytes(&bytes, channel, protocol)?.into_owned(),
        )),
    }
}

/// Readonly correspondence for an original braced word and its selected value.
/// A folded continuation has no ordinary original occurrence extent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeBracedWordSourceMap {
    bytes: Vec<u8>,
    boundaries: Vec<Option<usize>>,
    originals: Vec<(usize, usize)>,
    continuations: Vec<std::ops::Range<usize>>,
}

impl NativeBracedWordSourceMap {
    /// Exact value bytes under the independently selected source recipe.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Map complete native units without crossing a folded continuation.
    #[must_use]
    pub fn original_extent(
        &self,
        native: std::ops::Range<usize>,
    ) -> Option<std::ops::Range<usize>> {
        if native.start > native.end
            || self.continuations.iter().any(|continuation| {
                native.start < continuation.end && continuation.start < native.end
            })
        {
            return None;
        }
        Some(
            self.boundaries.get(native.start).copied().flatten()?
                ..self.boundaries.get(native.end).copied().flatten()?,
        )
    }

    /// Reverse only exact original boundaries whose forward map agrees.
    #[must_use]
    pub fn native_extent(
        &self,
        original: std::ops::Range<usize>,
    ) -> Option<std::ops::Range<usize>> {
        if original.start > original.end {
            return None;
        }
        let boundary = |at| {
            self.originals
                .binary_search_by_key(&at, |&(original, _)| original)
                .ok()
                .map(|index| self.originals[index].1)
        };
        let native = boundary(original.start)?..boundary(original.end)?;
        (self.original_extent(native.clone())? == original).then_some(native)
    }
}

/// Retain the existing continuation and native-unit owners' boundary map.
/// This grants source correspondence, never edit or execution authority.
///
/// # Errors
/// Returns the same source-channel refusal as [`native_source_braced_word_bytes`].
pub fn native_source_braced_word_map(
    raw: &[u8],
    channel: tcl_lexer::SourceChannel,
    rule: tcl_dialect::BraceBackslashNewline,
    protocol: crate::native_string::NativeStringProtocol,
) -> Result<NativeBracedWordSourceMap, NativeSourceUnavailable> {
    let collapsed = if rule.folds() {
        collapse_continuations_mapped(raw, false, channel, true)
    } else {
        CollapsedSource {
            bytes: Cow::Borrowed(raw),
            boundaries: (0..=raw.len()).collect(),
            continuations: Vec::new(),
        }
    };
    let ingress = if channel == tcl_lexer::SourceChannel::Document {
        native_document_ingress(&collapsed.bytes, protocol)?
    } else {
        NativeDocumentIngress {
            bytes: Cow::Borrowed(&collapsed.bytes),
            boundaries: None,
        }
    };
    let text = if channel == tcl_lexer::SourceChannel::Document {
        Some(
            std::str::from_utf8(raw)
                .map_err(|_| NativeSourceUnavailable::InvalidDocumentUnicode)?,
        )
    } else {
        None
    };
    let boundaries: Vec<_> = (0..=ingress.bytes.len())
        .map(|native| {
            let collapsed_offset = ingress.original_offset(native)?;
            let original = *collapsed.boundaries.get(collapsed_offset)?;
            text.is_none_or(|text| text.is_char_boundary(original))
                .then_some(original)
        })
        .collect();
    let originals = boundaries
        .iter()
        .enumerate()
        .filter_map(|(native, original)| original.map(|original| (original, native)))
        .collect();
    let continuations = collapsed
        .continuations
        .iter()
        .map(|extent| {
            Some(ingress.native_offset(extent.start)?..ingress.native_offset(extent.end)?)
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(NativeSourceUnavailable::GrammarProtocolMismatch)?;
    Ok(NativeBracedWordSourceMap {
        bytes: ingress.bytes.into_owned(),
        boundaries,
        originals,
        continuations,
    })
}

/// Checked original extent of a braced native value range.
#[must_use]
pub fn native_source_braced_word_extent(
    raw: &[u8],
    channel: tcl_lexer::SourceChannel,
    rule: tcl_dialect::BraceBackslashNewline,
    protocol: crate::native_string::NativeStringProtocol,
    native: std::ops::Range<usize>,
) -> Option<std::ops::Range<usize>> {
    native_source_braced_word_map(raw, channel, rule, protocol)
        .ok()?
        .original_extent(native)
}

/// Source-to-native value map. Only boundaries owned by original input are
/// mapped; a native byte inside a produced unit cannot fabricate a source span.
struct NativeDocumentIngress<'a> {
    bytes: Cow<'a, [u8]>,
    boundaries: Option<Vec<Option<usize>>>,
}

impl NativeDocumentIngress<'_> {
    fn native_offset(&self, original: usize) -> Option<usize> {
        self.boundaries.as_ref().map_or(Some(original), |map| {
            map.iter().position(|boundary| *boundary == Some(original))
        })
    }

    fn original_offset(&self, native: usize) -> Option<usize> {
        self.boundaries
            .as_ref()
            .map_or(Some(native), |map| map.get(native).copied().flatten())
    }
}

fn native_document_ingress(
    raw: &[u8],
    protocol: crate::native_string::NativeStringProtocol,
) -> Result<NativeDocumentIngress<'_>, NativeSourceUnavailable> {
    use crate::native_string::NativeStringProtocol;
    use tcl_dialect::TclVersion;
    let text =
        std::str::from_utf8(raw).map_err(|_| NativeSourceUnavailable::InvalidDocumentUnicode)?;
    let c_version = protocol.tcl_version();
    let changes_units = c_version.is_some()
        && (raw.contains(&0)
            || (c_version.is_some_and(|version| version < TclVersion::V9_0)
                && text.chars().any(|character| u32::from(character) > 0xffff)));
    if !changes_units && !raw.contains(&b'\r') {
        return Ok(NativeDocumentIngress {
            bytes: Cow::Borrowed(raw),
            boundaries: None,
        });
    }
    let mut bytes = Vec::with_capacity(raw.len());
    let mut boundaries = vec![Some(0)];
    let mut characters = text.char_indices().peekable();
    while let Some((start, character)) = characters.next() {
        let mut end = start + character.len_utf8();
        let character = if character == '\r' {
            if characters.peek().is_some_and(|(_, next)| *next == '\n') {
                let (offset, next) = characters.next().expect("peeked LF");
                end = offset + next.len_utf8();
            }
            '\n'
        } else {
            character
        };
        match protocol {
            NativeStringProtocol::Jim084 => {
                let mut encoded = [0; 4];
                bytes.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
            }
            NativeStringProtocol::C(version) => {
                let utf = crate::native_tcl_utf::NativeTclUtf::for_version(version);
                if u32::from(character) > 0xffff && version < TclVersion::V8_6 {
                    // C84/85 UTF-8 channels admit the four external bytes as
                    // Latin-1 units. The counted source adapter does not.
                    for (ordinal, byte) in raw[start..end].iter().enumerate() {
                        utf.encode_unit(u32::from(*byte), &mut bytes)
                            .expect("one admitted Latin-1 unit");
                        boundaries.resize(bytes.len() + 1, None);
                        boundaries[bytes.len()] = Some(start + ordinal + 1);
                    }
                } else if u32::from(character) > 0xffff && version == TclVersion::V8_6 {
                    let mut units = [0; 2];
                    for unit in character.encode_utf16(&mut units) {
                        utf.encode_unit(u32::from(*unit), &mut bytes)
                            .expect("admitted UTF-16 channel unit");
                    }
                } else {
                    utf.encode_unit(u32::from(character), &mut bytes)
                        .expect("admitted character-channel unit");
                }
            }
        }
        boundaries.resize(bytes.len() + 1, None);
        boundaries[bytes.len()] = Some(end);
    }
    Ok(NativeDocumentIngress {
        bytes: Cow::Owned(bytes),
        boundaries: Some(boundaries),
    })
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

pub use tcl_value_syntax::backslash::{
    NativeSourceEscape, NativeSourceUnavailable, native_source_escape_in,
    native_source_string_bytes_in,
};

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
            native_source_literal_bytes(original, arena.image().channel(), protocol)
                .map_err(NativeArenaTextUnavailable::Source)
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
    if escapes != protocol.escape_syntax() {
        return Err(NativeSourceUnavailable::GrammarProtocolMismatch);
    }
    if channel == tcl_lexer::SourceChannel::Document {
        let ingress = native_document_ingress(raw, protocol)?;
        let native_pos = ingress
            .native_offset(pos)
            .ok_or(NativeSourceUnavailable::InvalidEscapePosition)?;
        let decoded = native_source_escape_in(&ingress.bytes, native_pos, escapes, protocol)?;
        let end = ingress
            .original_offset(decoded.end)
            .ok_or(NativeSourceUnavailable::InvalidEscapePosition)?;
        return Ok(NativeSourceEscape {
            end,
            bytes: decoded.bytes,
        });
    }
    native_source_escape_in(raw, pos, escapes, protocol)
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
    if channel == tcl_lexer::SourceChannel::Document {
        return match native_document_ingress(raw, protocol)?.bytes {
            Cow::Borrowed(bytes) => native_source_string_bytes_in(bytes, escapes, protocol),
            Cow::Owned(bytes) => Ok(Cow::Owned(
                native_source_string_bytes_in(&bytes, escapes, protocol)?.into_owned(),
            )),
        };
    }
    native_source_string_bytes_in(raw, escapes, protocol)
}

/// Quote literal source characters for the content of a double-quoted word.
/// The selected escape grammar must reproduce every character. This lexical
/// spelling operation grants no native unit, name, lookup or execution proof.
#[must_use]
pub fn literal_quoted_source_fragment(text: &str, escapes: EscapeSyntax) -> Option<String> {
    let fragment = quote_literal_source_content(text, false);
    (decode_in(&fragment, escapes).as_ref() == text).then_some(fragment)
}

/// Literal quoted-word content that also remains inert inside a braced
/// script container. The existing character codec is shared; both the Unicode
/// value and complete braced source container are checked under actual grammar.
/// This grants source spelling only, not expression evaluation or edit rights.
#[must_use]
pub fn literal_quoted_source_fragment_in_braced_script(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let fragment = quote_literal_source_content(text, true);
    (decode_in(&fragment, config.escapes).as_ref() == text).then_some(())?;
    let word = format!("\"{fragment}\"");
    crate::word_rules::lexical_unicode_source_word_matches(&word, text, config).then_some(())?;
    let container = format!("{{{word}}}");
    crate::word_rules::lexical_unicode_source_word_matches(&container, &word, config)
        .then_some(())?;
    Some(fragment)
}

fn quote_literal_source_content(text: &str, protect_braces: bool) -> String {
    let mut result = String::new();
    for character in text.chars() {
        match character {
            '{' | '}' if protect_braces => {
                result.push('\\');
                result.push(character);
            }
            '\\' | '"' | '$' | '[' | ']' => {
                result.push('\\');
                result.push(character);
            }
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(character),
        }
    }
    result
}

/// Place an original substitution template inside a double-quoted word.
/// Literal quotes are escaped; variable, command and expression components
/// keep their original source spelling. The shared parser must reproduce the
/// same components under the retained complete lexical configuration.
///
/// Malformed or incompletely retained substitutions remain unavailable. This
/// checks lexical components only, without evaluating any substitution.
#[must_use]
pub fn substituting_quoted_source_fragment(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    use tcl_lexer::{SubstFlags, WordPart};
    let parts = tcl_lexer::word_parts::decompose_spanned_checked(
        source.as_bytes(),
        SubstFlags::default(),
        config,
    )
    .ok()?;
    let mut fragment = String::new();
    for part in &parts {
        let written = source.get(part.start..part.end)?;
        match &part.part {
            WordPart::ParseError(_) => return None,
            WordPart::Text(_) => {
                let mut at = 0;
                while at < written.len() {
                    if written.as_bytes()[at] == b'\\' {
                        let end = escape_end_in(written, at, config.escapes);
                        if end <= at || end > written.len() || at + 1 == written.len() {
                            return None;
                        }
                        fragment.push_str(written.get(at..end)?);
                        at = end;
                    } else {
                        let character = written.get(at..)?.chars().next()?;
                        if character == '"' {
                            fragment.push('\\');
                        }
                        fragment.push(character);
                        at += character.len_utf8();
                    }
                }
            }
            WordPart::Variable(_) | WordPart::Command(_) | WordPart::Expression(_) => {
                fragment.push_str(written);
            }
        }
    }
    let projected = tcl_lexer::word_parts::decompose_spanned_checked(
        fragment.as_bytes(),
        SubstFlags::default(),
        config,
    )
    .ok()?;
    parts
        .iter()
        .map(|part| &part.part)
        .eq(projected.iter().map(|part| &part.part))
        .then_some(fragment)
}

/// Render native units as literal source text, without numeric escapes or
/// word quoting. Lexical variable roots use this purpose because their names
/// do not evaluate backslash escapes. The selected input channel must reproduce
/// every original unit; invalid/unpaired units remain unavailable.
#[must_use]
pub fn native_literal_source_text(
    value: &[u8],
    channel: tcl_lexer::SourceChannel,
    protocol: crate::native_string::NativeStringProtocol,
) -> Option<String> {
    let exact = |text: &str| {
        native_source_literal_bytes(text.as_bytes(), channel, protocol)
            .is_ok_and(|bytes| bytes.as_ref() == value)
    };
    if let Ok(text) = std::str::from_utf8(value)
        && exact(text)
    {
        return Some(text.to_owned());
    }
    let version = protocol.tcl_version()?;
    let native = crate::native_tcl_utf::NativeTclUtf::for_version(version);
    let units = native.decode_units(value);
    if native.encode_units(&units)?.as_slice() != value {
        return None;
    }
    let text = if channel == tcl_lexer::SourceChannel::Document
        && version < tcl_dialect::TclVersion::V8_6
    {
        legacy_document_literal_text(&units)?
    } else if version < tcl_dialect::TclVersion::V9_0 {
        let utf16 = units
            .into_iter()
            .map(u16::try_from)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        char::decode_utf16(utf16)
            .collect::<Result<String, _>>()
            .ok()?
    } else {
        units
            .into_iter()
            .map(char::from_u32)
            .collect::<Option<String>>()?
    };
    exact(&text).then_some(text)
}

fn legacy_document_literal_text(units: &[u32]) -> Option<String> {
    let mut text = String::new();
    let mut remaining = units;
    while let Some((&unit, tail)) = remaining.split_first() {
        // C 8.4/8.5 Document ingress represents a supplementary character as
        // four Latin-1 byte units. This is a proposed spelling only; the caller
        // verifies the complete selected channel roundtrip before returning it.
        let supplementary = remaining.get(..4).and_then(|four| {
            let bytes = [
                u8::try_from(four[0]).ok()?,
                u8::try_from(four[1]).ok()?,
                u8::try_from(four[2]).ok()?,
                u8::try_from(four[3]).ok()?,
            ];
            let decoded = std::str::from_utf8(&bytes).ok()?;
            let mut characters = decoded.chars();
            let character = characters.next()?;
            (u32::from(character) > 0xffff && characters.next().is_none()).then_some(character)
        });
        if let Some(character) = supplementary {
            text.push(character);
            remaining = &remaining[4..];
        } else {
            text.push(char::from_u32(unit)?);
            remaining = tail;
        }
    }
    Some(text)
}

/// Render one literal source word whose selected native value is exactly
/// `value`. The source channel, complete escape grammar and independently
/// selected native string recipe are retained separately. A value that cannot
/// be written in the requested channel is unavailable, rather than repaired.
///
/// This is a source spelling operation only. It supplies no name-key, object,
/// lookup, compiler, cache or runtime-entry capability.
#[must_use]
pub fn native_literal_source_word(
    value: &[u8],
    channel: tcl_lexer::SourceChannel,
    config: tcl_lexer::LexerConfig,
    protocol: crate::native_string::NativeStringProtocol,
) -> Option<String> {
    fn quote(text: &str) -> String {
        format!("\"{}\"", quote_literal_source_content(text, false))
    }
    let exact = |word: &str| {
        native_source_string_bytes_channel_in(
            &word.as_bytes()[1..word.len() - 1],
            channel,
            config.escapes,
            protocol,
        )
        .is_ok_and(|produced| produced.as_ref() == value)
    };
    if let Ok(text) = std::str::from_utf8(value) {
        let word = quote(text);
        if exact(&word) {
            return Some(word);
        }
    }
    let version = protocol.tcl_version()?;
    let native = crate::native_tcl_utf::NativeTclUtf::for_version(version);
    let units = native.decode_units(value);
    // Invalid byte fallbacks and raw zero must never acquire the spelling of
    // a different modified-UTF value through a Unicode display projection.
    if native.encode_units(&units)?.as_slice() != value {
        return None;
    }
    let mut word = String::from("\"");
    for unit in units {
        use std::fmt::Write;
        if unit <= 0xffff {
            write!(word, "\\u{unit:04x}").ok()?;
        } else {
            write!(word, "\\U{unit:08x}").ok()?;
        }
    }
    word.push('"');
    exact(&word).then_some(word)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod native_source_tests;

#[cfg(test)]
mod executable_text_tests;

#[cfg(test)]
mod original_literal_extent_tests;

#[cfg(test)]
mod braced_script_quoted_content_tests;
