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

//! The source-decoding contract — one decoder, and the diagnostics that say
//! what it had to substitute (issue #1326).
//!
//! # The problem
//!
//! Every path that turns source *bytes* into a Rust `String` has to decide
//! what to do about bytes that are not valid UTF-8.  Failing hard would make
//! the toolchain useless on the messy files it exists to help with, so the
//! answer is a lossy decode — each ill-formed sequence becomes `U+FFFD`.  That
//! is the right default and this module keeps it.
//!
//! What was wrong is that it happened **silently**: the analysed text was not
//! the file on disk, every offset past the first substitution was derived from
//! characters that were never there, and a truncated sequence, an overlong
//! encoding, a CESU-8 lone surrogate, and a file that genuinely contained
//! `U+FFFD` were all indistinguishable afterwards.  C Tcl 9.0 does not
//! tolerate any of them — `source`ing such a file fails outright with
//! `invalid or incomplete multibyte or wide character` — so this is not a
//! stylistic nicety; the file the user is editing will not run.
//!
//! Recovery stays best-effort.  It just is not mute any more.
//!
//! # The two entry points, and why there are two
//!
//! | Caller | Sees | Entry point |
//! |---|---|---|
//! | CLI, workspace scan, anything reading a file | raw bytes | [`decode_source`] |
//! | LSP `didOpen`/`didChange` | text the editor already decoded | — |
//!
//! An editor hands the server text over the wire, so it normally cannot prove
//! how that text was decoded.  The server may retain a [`DecodeReport`] when
//! the on-disk bytes lossily decode to the exact text it opened.  After an
//! unsaved edit, or for an untitled buffer, no report is available and these
//! checks abstain.  In particular, `U+FFFD` is valid Tcl source and is *not*
//! evidence of a decoding error by itself.
//!
//! # Codes
//!
//! * **W107** — the source is not valid UTF-8.  One diagnostic per file,
//!   anchored at the first affected position, naming the byte offset and
//!   malformation class when they are known.
//! * **W109** — the source does not look like UTF-8 *text* at all (UTF-16 /
//!   UTF-32 / binary).  The caller is expected to **abstain** from the rest of
//!   the analysis rather than publish findings derived from mis-decoded bytes:
//!   a three-line UTF-16LE iRule used to produce 87 nonsense diagnostics.
//!
//! W305 (bidirectional formatting controls) is a Unicode-text finding rather
//! than a byte-decoding finding. Its canonical producer lives in
//! [`tcl_compiler::analyser::bidi_control_diagnostics`], so analyser-only and
//! MCP consumers receive it too.
//!
//! # What is *not* detected, and why
//!
//! * **Legacy 8-bit encodings that decode cleanly.**  A latin-1 file whose
//!   high bytes happen to form valid UTF-8 sequences is indistinguishable from
//!   a UTF-8 file; there is nothing to abstain on.  (A latin-1 file whose high
//!   bytes *don't* form valid sequences — the common case — is W107.)
//! * **Which encoding a non-UTF-8 file actually is.**  W109 names the family
//!   it matched (a BOM, or interleaved NULs) and stops there.  Guessing
//!   between, say, UTF-16LE and a corrupt UTF-8 file is not provable from the
//!   bytes, and a wrong guess in a security-review tool is worse than silence.
//! * **Malformed bytes that are no longer available.** An LSP client sends
//!   Unicode text, not the bytes it decoded. Once an unsaved buffer differs
//!   from disk, there is no sound way to distinguish a literal `U+FFFD` from a
//!   replacement a client inserted while decoding. The check deliberately
//!   abstains rather than emit a false positive.
//! * **Line endings and NUL bytes inside otherwise-valid UTF-8.**  Those are
//!   real, decodable text: CRLF/mixed endings are W118's job, and a lone NUL
//!   is left to W108.  Only a NUL *density in the raw bytes* consistent with
//!   UTF-16 trips W109.

use crate::definition::{LspRange, utf16_len};
use crate::source_style::{StyleDiagnostic, StyleSeverity};

/// How a byte sequence failed to be UTF-8.  Purely descriptive — every class
/// is reported the same way; the class is what tells a user whether they are
/// looking at a truncated file, a mislabelled legacy encoding, or something
/// deliberately crafted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Utf8Fault {
    /// A multi-byte sequence ran off the end of the file, or was cut short by
    /// a byte that cannot continue it. Usually a truncated download or a
    /// buffer split mid-character.
    Truncated,
    /// A codepoint encoded in more bytes than the minimum (`\xc0\xaf` for
    /// `/`). Historically a filter-evasion technique: the security check sees
    /// one thing, the decoder another.
    Overlong,
    /// A UTF-16 surrogate half (`U+D800`-`U+DFFF`) encoded directly — CESU-8 /
    /// WTF-8. Produced by naive UTF-16 → UTF-8 conversion; not valid UTF-8.
    Surrogate,
    /// A lead byte that cannot start any valid sequence (`\xf5`-`\xff`, above
    /// the `U+10FFFF` ceiling).
    OutOfRange,
    /// A continuation byte (`\x80`-`\xbf`) with no lead byte before it —
    /// the signature of a file spliced mid-character, or of legacy 8-bit text.
    StrayContinuation,
}

impl Utf8Fault {
    /// A short human-readable label for the message.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Truncated => "truncated multi-byte sequence",
            Self::Overlong => "overlong encoding",
            Self::Surrogate => "lone surrogate (CESU-8/WTF-8)",
            Self::OutOfRange => "out-of-range lead byte",
            Self::StrayContinuation => "stray continuation byte",
        }
    }
}

/// The first ill-formed sequence found in a byte buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Utf8Error {
    /// Offset of the first ill-formed byte, in the **original** buffer.
    pub byte_offset: usize,
    /// Offset of the `U+FFFD` inserted for this fault in the decoded text.
    ///
    /// This is recorded at the byte-to-text boundary rather than recovered by
    /// searching the decoded string: a valid source file may already contain
    /// a literal `U+FFFD` before the decoder has to insert one.
    pub replacement_text_offset: usize,
    /// How it was ill-formed.
    pub fault: Utf8Fault,
}

/// A non-UTF-8 encoding family [`decode_source`] recognised well enough to
/// refuse the file. Never a claim about the *exact* encoding — see the module
/// docs on what is deliberately not guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonTextEncoding {
    /// A `FF FE` / `FE FF` byte-order mark.
    Utf16Bom,
    /// A `FF FE 00 00` / `00 00 FE FF` byte-order mark.
    Utf32Bom,
    /// No BOM, but NUL bytes at a density no UTF-8 text has — the signature of
    /// BOM-less UTF-16 holding mostly-ASCII content.
    NulInterleaved,
}

impl NonTextEncoding {
    /// A short human-readable label for the message.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Utf16Bom => "a UTF-16 byte-order mark",
            Self::Utf32Bom => "a UTF-32 byte-order mark",
            Self::NulInterleaved => "NUL bytes interleaved through the text (BOM-less UTF-16?)",
        }
    }
}

/// What [`decode_source`] had to substitute, if anything. An all-`None`
/// report is the proof that the decoded text *is* the bytes on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DecodeReport {
    /// The first ill-formed sequence, if the buffer was not valid UTF-8.
    pub first_error: Option<Utf8Error>,
    /// How many ill-formed sequences there were in total (each became one
    /// `U+FFFD`). Zero exactly when `first_error` is `None`.
    pub error_count: usize,
    /// Set when the buffer does not look like UTF-8 text at all.
    pub non_text: Option<NonTextEncoding>,
}

impl DecodeReport {
    /// Whether the decoded text faithfully represents the bytes it came from.
    #[must_use]
    pub const fn is_faithful(&self) -> bool {
        self.first_error.is_none() && self.non_text.is_none()
    }

    /// Whether consumers must abstain from analysing the decoded text.
    ///
    /// This is a property of the byte evidence, not of whether the user chose
    /// to display W109. Disabling that diagnostic may hide the explanation,
    /// but it must never turn mis-decoded UTF-16/UTF-32 bytes into analysable
    /// Tcl source.
    #[must_use]
    pub const fn requires_abstention(&self) -> bool {
        self.non_text.is_some()
    }
}

/// Minimum NUL count for the interleaved-NUL encoding signature to apply at
/// all. Mostly-ASCII UTF-16 is about 50% NUL, so the 25% integer threshold
/// below remains conservative; the floor stops a two-byte file qualifying.
const NUL_COUNT_FLOOR: usize = 8;

/// Decode source bytes to text, reporting everything that had to be
/// substituted.
///
/// This is **the** byte → `String` boundary for Tcl source: the CLI's file
/// reader and the LSP's workspace scan both go through it, so neither can
/// re-introduce a silent lossy read.
///
/// The decode itself never fails. Valid UTF-8 is returned unchanged (and
/// without a copy of the validation work being repeated); anything else is
/// decoded lossily *after* being described.
///
/// ```
/// use tcl_lsp_core::source_decode::{Utf8Fault, decode_source};
///
/// // Valid UTF-8 round-trips with an empty report.
/// let (text, report) = decode_source("puts «héllo»\n".as_bytes());
/// assert_eq!(text, "puts «héllo»\n");
/// assert!(report.is_faithful());
///
/// // An overlong encoding of `/` is named, not silently replaced.
/// let (text, report) = decode_source(b"puts \xc0\xaf\n");
/// assert_eq!(text, "puts \u{fffd}\u{fffd}\n");
/// let err = report.first_error.expect("ill-formed");
/// assert_eq!(err.byte_offset, 5);
/// assert_eq!(err.fault, Utf8Fault::Overlong);
/// ```
#[must_use]
pub fn decode_source(bytes: &[u8]) -> (String, DecodeReport) {
    let non_text = detect_non_text(bytes);
    let Ok(text) = core::str::from_utf8(bytes) else {
        let (first_error, error_count) = scan_utf8_errors(bytes);
        return (
            String::from_utf8_lossy(bytes).into_owned(),
            DecodeReport {
                first_error,
                error_count,
                non_text,
            },
        );
    };
    (
        text.to_owned(),
        DecodeReport {
            first_error: None,
            error_count: 0,
            non_text,
        },
    )
}

/// Read a source file and decode it through [`decode_source`].
///
/// The reason to prefer this over `std::fs::read_to_string` for *source* is
/// that `read_to_string` fails outright on ill-formed UTF-8, and every call
/// site in a language server ends that failure with `.ok()?` — so a file with
/// one bad byte disappears from the workspace index entirely, taking its
/// procedures, its `source` edges and its symbols with it, and the user is
/// told nothing. Reading through the lossy decoder keeps the file in the
/// index and keeps the reason for it in [`DecodeReport`].
///
/// # Errors
///
/// Propagates the underlying I/O error. Decoding itself never fails.
pub fn read_source_file(path: &std::path::Path) -> std::io::Result<(String, DecodeReport)> {
    Ok(decode_source(&std::fs::read(path)?))
}

/// Recognise a buffer that is not UTF-8 text at all, from its BOM or its NUL
/// density. Deliberately conservative: a file with a handful of NULs in
/// otherwise-valid UTF-8 is real (if odd) text and must not be refused.
fn detect_non_text(bytes: &[u8]) -> Option<NonTextEncoding> {
    // UTF-32 BOMs are checked first: `FF FE 00 00` also starts with the
    // UTF-16LE BOM, so testing UTF-16 first would mislabel it.
    if bytes.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) || bytes.starts_with(&[0x00, 0x00, 0xFE, 0xFF])
    {
        return Some(NonTextEncoding::Utf32Bom);
    }
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        return Some(NonTextEncoding::Utf16Bom);
    }
    // A fold rather than `filter(..).count()`: the latter is the shape
    // `clippy::naive_bytecount` flags, and its suggested fix is a new
    // dependency this crate does not need for one count on a cold path.
    let nuls = bytes.iter().fold(0usize, |n, &b| n + usize::from(b == 0));
    (nuls >= NUL_COUNT_FLOOR && nuls >= bytes.len().div_ceil(4))
        .then_some(NonTextEncoding::NulInterleaved)
}

/// Walk `bytes`, classifying the first ill-formed sequence and counting them
/// all. Only reached when `from_utf8` has already said the buffer is invalid.
fn scan_utf8_errors(bytes: &[u8]) -> (Option<Utf8Error>, usize) {
    let mut first = None;
    let mut count = 0usize;
    let mut rest = bytes;
    let mut base = 0usize;
    loop {
        match core::str::from_utf8(rest) {
            Ok(_) => return (first, count),
            Err(e) => {
                let at = base + e.valid_up_to();
                count += 1;
                if first.is_none() {
                    first = Some(Utf8Error {
                        byte_offset: at,
                        // `at` follows a completely valid UTF-8 prefix. The
                        // lossy decoder copies that prefix byte-for-byte, so
                        // its first inserted replacement starts at the same
                        // byte offset in the decoded `String`. Keep the text
                        // offset as explicit evidence so range construction
                        // never has to guess from the string's contents.
                        replacement_text_offset: at,
                        fault: classify_fault(&bytes[at..]),
                    });
                }
                // `error_len() == None` means "ran off the end": nothing left
                // to scan, so the count is final.
                let Some(skip) = e.error_len() else {
                    return (first, count);
                };
                let consumed = e.valid_up_to() + skip;
                base += consumed;
                rest = &rest[consumed..];
            }
        }
    }
}

/// Classify the ill-formed sequence starting at `at[0]`.
///
/// The lead byte — and, where the lead byte alone is not decisive, the byte
/// after it — picks the class, following the Unicode 15 well-formed
/// byte-sequence table (§3.9, Table 3-7). Anything left over is a well-formed
/// lead byte that was not followed by the continuation bytes it needs, which
/// is [`Utf8Fault::Truncated`] whether the file ran out or the next byte was
/// simply not a continuation.
fn classify_fault(at: &[u8]) -> Utf8Fault {
    let Some(&lead) = at.first() else {
        return Utf8Fault::Truncated;
    };
    let next = at.get(1).copied();
    match lead {
        0x80..=0xBF => Utf8Fault::StrayContinuation,
        // C0/C1 can only ever encode a codepoint that fits in one byte.
        0xC0 | 0xC1 => Utf8Fault::Overlong,
        // E0 A0..BF is the shortest 3-byte form; E0 80..9F is overlong.
        0xE0 if matches!(next, Some(0x80..=0x9F)) => Utf8Fault::Overlong,
        // ED A0..BF encodes U+D800..U+DFFF — the surrogate range.
        0xED if matches!(next, Some(0xA0..=0xBF)) => Utf8Fault::Surrogate,
        // F0 90..BF is the shortest 4-byte form; F0 80..8F is overlong.
        0xF0 if matches!(next, Some(0x80..=0x8F)) => Utf8Fault::Overlong,
        // F4 90..BF would exceed U+10FFFF, as does any lead byte above F4.
        0xF4 if matches!(next, Some(0x90..=0xBF)) => Utf8Fault::OutOfRange,
        0xF5..=0xFF => Utf8Fault::OutOfRange,
        _ => Utf8Fault::Truncated,
    }
}

/// Position of the byte at `offset` in `text`, as an LSP line/character pair
/// (character = UTF-16 code units, per the LSP default).
///
/// Offsets that land mid-character, or past the end, clamp to the nearest
/// character boundary rather than panicking — the same defensive posture
/// issue #1325 established for token spans.
fn position_of(text: &str, offset: usize) -> (u32, u32) {
    let offset = offset.min(text.len());
    let before = &text[..text.floor_char_boundary(offset)];
    let line = before.matches('\n').count();
    let col_start = before.rfind('\n').map_or(0, |i| i + 1);
    (
        u32::try_from(line).unwrap_or(u32::MAX),
        utf16_len(&before[col_start..]),
    )
}

/// A one-character range at `offset`, or an empty range at end-of-file.
fn range_at(text: &str, offset: usize, ch_len: usize) -> LspRange {
    let (start_line, start_character) = position_of(text, offset);
    let (end_line, end_character) = position_of(text, offset + ch_len);
    LspRange {
        start_line,
        start_character,
        end_line,
        end_character,
    }
}

/// W107 / W109 — the encoding-integrity pass over already-decoded `text`.
///
/// `report` is the byte-level decoder's verdict when the caller read bytes
/// ([`decode_source`]). `None` means the caller has only Unicode text (such as
/// an unsaved LSP buffer), which is insufficient evidence for an encoding
/// finding and therefore produces no W107/W109 diagnostic.
///
/// At most one W107 and at most one W109 per file: a mis-decoded file has one
/// problem, not one per replaced byte, and repeating it per occurrence buries
/// everything else.
///
/// ```
/// use tcl_lsp_core::source_decode::{decode_source, encoding_integrity_diagnostics};
///
/// let (text, report) = decode_source(b"puts \xed\xa0\x80\n");
/// let diags = encoding_integrity_diagnostics(&text, Some(&report));
/// assert_eq!(diags.len(), 1);
/// assert_eq!(diags[0].code, "W107");
/// assert!(diags[0].message.contains("byte offset 5"));
/// assert!(diags[0].message.contains("lone surrogate"));
/// ```
#[must_use]
pub fn encoding_integrity_diagnostics(
    text: &str,
    report: Option<&DecodeReport>,
) -> Vec<StyleDiagnostic> {
    let mut out = Vec::new();

    // ---- W109: this is not UTF-8 text at all -------------------------------
    // This is a byte-level verdict. A sequence of NUL characters in an LSP
    // string can be deliberate Tcl data; only the original bytes establish an
    // encoding signature.
    let non_text = report.and_then(|r| r.non_text);
    if let Some(kind) = non_text {
        out.push(StyleDiagnostic {
            range: LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 0,
            },
            message: format!(
                "Source does not look like UTF-8 text — found {}. Re-save the file as UTF-8; \
                 analysis of the rest of this file is skipped rather than reporting findings \
                 derived from mis-decoded bytes.",
                kind.label()
            ),
            severity: StyleSeverity::Warning,
            code: "W109",
            fix: None,
        });
        // A file that is not UTF-8 at all will also be full of ill-formed
        // sequences; reporting both says the same thing twice.
        return out;
    }

    // ---- W107: valid text, but not the bytes on disk -----------------------
    if let Some(&DecodeReport {
        first_error: Some(err),
        error_count,
        ..
    }) = report
    {
        let plural = if error_count == 1 { "" } else { "s" };
        out.push(StyleDiagnostic {
            range: replacement_range(text, err.replacement_text_offset),
            message: format!(
                "Source is not valid UTF-8: {} at byte offset {} ({} ill-formed sequence{} in \
                 total, each replaced with U+FFFD). The analysed text is not the file on \
                 disk, so positions and content in this file may differ from what you see. \
                 Tcl 9 refuses to read such a file at all.",
                err.fault.label(),
                err.byte_offset,
                error_count,
                plural,
            ),
            severity: StyleSeverity::Warning,
            code: "W107",
            fix: None,
        });
    }
    out
}

/// Range of the decoder-inserted `U+FFFD` at `offset`.
///
/// A mismatched report/text pair violates the caller contract. Stay
/// best-effort in that case by returning a file-level range, but never search
/// for a different, possibly authored, replacement character.
fn replacement_range(text: &str, offset: usize) -> LspRange {
    if text
        .get(offset..)
        .is_some_and(|tail| tail.starts_with('\u{fffd}'))
    {
        return range_at(text, offset, '\u{fffd}'.len_utf8());
    }
    LspRange {
        start_line: 0,
        start_character: 0,
        end_line: 0,
        end_character: 0,
    }
}

/// Whether the caller should **abstain** from analysing beyond the encoding
/// diagnostics — true exactly when the byte report identifies non-UTF-8 text.
///
/// Abstention here is a positive answer, not a silent degradation: the file
/// gets one accurate diagnostic saying it is not UTF-8, instead of dozens of
/// findings about characters that are decoding artefacts.
#[must_use]
pub fn should_abstain(report: Option<&DecodeReport>) -> bool {
    report.is_some_and(DecodeReport::requires_abstention)
}

#[cfg(test)]
mod tests;
