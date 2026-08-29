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

//! Source-decoding tests (issue #1326), organised as a true/false ×
//! positive/negative matrix: each new code has to fire on every malformation
//! class it claims to cover (TP), stay silent on the legitimate inputs closest
//! to those classes (TN / FP guard), and keep firing through the paths that
//! could quietly drop it (FN guard).

use super::*;

/// Codes present in `diags`, in order.
fn codes(diags: &[StyleDiagnostic]) -> Vec<&str> {
    diags.iter().map(|d| d.code).collect()
}

/// The full byte → diagnostics pipeline, the way a file-reading caller sees it.
fn from_bytes(bytes: &[u8]) -> (String, Vec<StyleDiagnostic>) {
    let (text, report) = decode_source(bytes);
    let diags = encoding_integrity_diagnostics(&text, Some(&report));
    (text, diags)
}

// W107 — ill-formed UTF-8.  True positives, one per malformation class.

#[test]
fn w107_fires_on_every_malformation_class() {
    // (bytes, expected fault, expected byte offset)
    let cases: &[(&[u8], Utf8Fault, usize)] = &[
        // 3-byte lead with one continuation byte, then a valid ASCII byte.
        (b"puts \xe2\x82x\n", Utf8Fault::Truncated, 5),
        // 2-byte lead at end of buffer — `error_len()` is `None`.
        (b"puts \xc3", Utf8Fault::Truncated, 5),
        // C0 AF: overlong `/`.
        (b"puts \xc0\xaf\n", Utf8Fault::Overlong, 5),
        // E0 80 80: overlong NUL in three bytes.
        (b"puts \xe0\x80\x80\n", Utf8Fault::Overlong, 5),
        // F0 80 80 80: overlong in four bytes.
        (b"puts \xf0\x80\x80\x80\n", Utf8Fault::Overlong, 5),
        // ED A0 80: U+D800, a lone high surrogate (CESU-8).
        (b"puts \xed\xa0\x80\n", Utf8Fault::Surrogate, 5),
        // ED BF BF: U+DFFF, a lone low surrogate.
        (b"puts \xed\xbf\xbf\n", Utf8Fault::Surrogate, 5),
        // F5..FF: above the U+10FFFF ceiling.
        (b"puts \xf5\x80\x80\x80\n", Utf8Fault::OutOfRange, 5),
        (b"puts \xff\n", Utf8Fault::OutOfRange, 5),
        // F4 90..: also above the ceiling, but with a legal lead byte.
        (b"puts \xf4\x90\x80\x80\n", Utf8Fault::OutOfRange, 5),
        // A continuation byte with nothing in front of it.
        (b"puts \x80\n", Utf8Fault::StrayContinuation, 5),
        // Latin-1 `é` mislabelled as UTF-8.
        (b"puts \xe9\n", Utf8Fault::Truncated, 5),
    ];
    for (bytes, fault, offset) in cases {
        let (_, report) = decode_source(bytes);
        let err = report
            .first_error
            .unwrap_or_else(|| panic!("expected an error for {bytes:x?}"));
        assert_eq!(err.fault, *fault, "fault class for {bytes:x?}");
        assert_eq!(err.byte_offset, *offset, "byte offset for {bytes:x?}");
        assert!(!report.is_faithful());

        let (_, diags) = from_bytes(bytes);
        assert_eq!(codes(&diags), ["W107"], "codes for {bytes:x?}");
        assert!(
            diags[0].message.contains(fault.label()),
            "message for {bytes:x?} should name the fault class: {}",
            diags[0].message
        );
        assert!(diags[0].message.contains(&format!("byte offset {offset}")));
    }
}

#[test]
fn w107_counts_every_ill_formed_sequence_not_just_the_first() {
    // The count is the number of substitutions made — one per U+FFFD the lossy
    // decode emits. `\xc0\xaf` contributes two (neither byte can start or
    // continue a sequence), plus the stray `\x80` and the out-of-range `\xf5`.
    let (text, report) = decode_source(b"a\xc0\xafb\x80c\xf5\n");
    assert_eq!(report.error_count, text.matches('\u{fffd}').count());
    assert_eq!(report.error_count, 4);
    // The first is still the one reported, and it is the earliest.
    assert_eq!(report.first_error.map(|e| e.byte_offset), Some(1));
}

#[test]
fn w107_is_one_diagnostic_per_file_however_broken() {
    // Twenty ill-formed sequences must still be one finding: a mis-decoded
    // file has one problem, and repeating it per byte buries everything else.
    let mut bytes = Vec::new();
    for _ in 0..20 {
        bytes.extend_from_slice(b"x\xff");
    }
    let (_, diags) = from_bytes(&bytes);
    assert_eq!(codes(&diags), ["W107"]);
    assert!(diags[0].message.contains("20 ill-formed sequences"));
}

// W107 — false-positive guards.

#[test]
fn w107_silent_on_valid_utf8_including_non_ascii() {
    for src in [
        "puts hello\n",
        "",
        // Legitimate non-ASCII content of every width.
        "set greeting \"héllo\"\n",
        "set greeting \"مرحبا بالعالم\"\n",
        "set greeting \"日本語のテキスト\"\n",
        "set emoji \"\u{1f388}\"\n",
        // A UTF-8 BOM at the head is ordinary data (documented non-finding).
        "\u{feff}puts hello\n",
        // CRLF, mixed endings, and a lone CR are text questions, not encoding
        // ones — W118's job, not W107's.
        "puts a\r\nputs b\r\n",
        "puts a\r\nputs b\nputs c\r",
    ] {
        let (text, diags) = from_bytes(src.as_bytes());
        assert_eq!(text, src);
        assert!(
            diags.is_empty(),
            "unexpected {:?} for {src:?}",
            codes(&diags)
        );
    }
}

#[test]
fn w107_requires_byte_evidence_and_never_accuses_a_literal_replacement_char() {
    // A U+FFFD the author actually typed. With the bytes in hand the decoder
    // can prove the file was valid UTF-8, so this must stay silent...
    let src = "set marker \"\u{fffd}\"\n";
    let (_, diags) = from_bytes(src.as_bytes());
    assert!(diags.is_empty(), "byte path: {:?}", codes(&diags));

    // ...and the text-only LSP path must make the same safe choice. It cannot
    // distinguish this literal character from a decoder substitution, so it
    // abstains rather than emit a false positive.
    let diags = encoding_integrity_diagnostics(src, None);
    assert!(diags.is_empty(), "text-only path: {:?}", codes(&diags));
}

#[test]
fn w107_does_not_infer_a_decode_failure_from_lsp_text_alone() {
    // The text is exactly what a lossy editor decode might send, but it could
    // also be authored content. The byte path reports the proven malformed
    // file; the text-only path correctly has no verdict.
    let bytes = b"when HTTP_REQUEST { log local0. \"\xc0\xaf\" }\n";
    let (text, byte_diags) = from_bytes(bytes);
    let text_diags = encoding_integrity_diagnostics(&text, None);
    assert_eq!(codes(&byte_diags), ["W107"]);
    assert!(text_diags.is_empty(), "text-only: {:?}", codes(&text_diags));
    assert!(byte_diags[0].message.contains("overlong encoding"));
}

#[test]
fn w107_anchors_on_the_first_replacement_character() {
    let (text, diags) = from_bytes(b"puts a\nputs \xff\n");
    assert_eq!(text, "puts a\nputs \u{fffd}\n");
    assert_eq!(diags[0].range.start_line, 1);
    assert_eq!(diags[0].range.start_character, 5);
    assert_eq!(diags[0].range.end_character, 6);
}

#[test]
fn w107_anchors_on_the_decoder_insertion_after_an_authored_replacement_character() {
    // The first U+FFFD is valid source text. The second one is the decoder's
    // replacement for the bad byte and is the only honest W107 anchor.
    let mut bytes = "set authored \u{fffd}\nputs 𝄞".as_bytes().to_vec();
    bytes.push(0xff);
    bytes.push(b'\n');
    let (text, diags) = from_bytes(&bytes);
    assert_eq!(text, "set authored \u{fffd}\nputs 𝄞\u{fffd}\n");
    assert_eq!(diags[0].range.start_line, 1);
    // `puts ` is five UTF-16 units and U+1D11E is two.
    assert_eq!(diags[0].range.start_character, 7);
    assert_eq!(diags[0].range.end_character, 8);
    let report = decode_source(&bytes).1;
    let err = report.first_error.expect("bad byte report");
    assert_eq!(
        text.get(err.replacement_text_offset..)
            .and_then(|tail| tail.chars().next()),
        Some('\u{fffd}')
    );
}

#[test]
fn positions_are_utf16_code_units_not_bytes_or_chars() {
    // `𝄞` is one char, two UTF-16 code units, four UTF-8 bytes. An LSP
    // character offset must count the middle one.
    let src = "set x \"𝄞\"\n";
    let (text, diags) = from_bytes(src.as_bytes());
    assert!(diags.is_empty(), "{:?}", codes(&diags));
    let clef = text.find('𝄞').expect("clef");
    // `set x "` is 7 UTF-16 units; the clef itself is 2 more.
    assert_eq!(super::position_of(&text, clef), (0, 7));
    assert_eq!(super::position_of(&text, clef + 4), (0, 9));
    // A byte offset landing *inside* the clef clamps to the boundary before it
    // rather than panicking — the posture issue #1325 established.
    assert_eq!(super::position_of(&text, clef + 2), (0, 7));
}

// W109 — this is not UTF-8 text.

/// The body the encoding fixtures wrap, as a `&str`.
const BODY: &str = "when HTTP_REQUEST {\n    log local0. \"hit\"\n}\n";

fn utf16le(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn utf16be(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_be_bytes).collect()
}

#[test]
fn w109_fires_on_utf16_with_and_without_a_bom() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("utf16le+bom", utf16le(&format!("\u{feff}{BODY}"))),
        ("utf16be+bom", utf16be(&format!("\u{feff}{BODY}"))),
        ("utf16le, no bom", utf16le(BODY)),
        ("utf16be, no bom", utf16be(BODY)),
        (
            "utf32le+bom",
            format!("\u{feff}{BODY}")
                .chars()
                .flat_map(|c| (c as u32).to_le_bytes())
                .collect(),
        ),
    ];
    for (label, bytes) in cases {
        let (_, diags) = from_bytes(&bytes);
        assert_eq!(codes(&diags), ["W109"], "{label}");
        // W109 suppresses W107: a file that is not UTF-8 at all would
        // otherwise say the same thing twice.
        assert!(diags[0].message.contains("does not look like UTF-8"));
        let (_, report) = decode_source(&bytes);
        assert!(should_abstain(Some(&report)), "{label}");
    }
}

#[test]
fn w109_does_not_guess_from_nul_interleaved_lsp_text() {
    // A sequence of NULs is valid Tcl data. Without the bytes, the LSP server
    // cannot prove it was UTF-16 and must not suppress real diagnostics.
    let (text, _) = decode_source(&utf16le(BODY));
    let diags = encoding_integrity_diagnostics(&text, None);
    assert!(diags.is_empty(), "text-only: {:?}", codes(&diags));
    assert!(!should_abstain(None));
}

#[test]
fn w109_silent_on_valid_utf8_containing_occasional_nuls() {
    // The false-positive guard the issue calls for: NULs in otherwise-valid
    // UTF-8 are real (if odd) text, not a mis-encoded file.
    let mut src = String::from(BODY);
    src.push_str("# marker \u{0}\u{0}\u{0}\n");
    let (_, diags) = from_bytes(src.as_bytes());
    assert!(diags.is_empty(), "{:?}", codes(&diags));
    assert!(!should_abstain(None));
}

#[test]
fn w109_silent_on_ordinary_source_and_on_an_empty_file() {
    for src in ["", BODY, "puts \"héllo\"\n"] {
        let (_text, report) = decode_source(src.as_bytes());
        assert!(!should_abstain(Some(&report)), "{src:?}");
    }
}

#[test]
fn w109_needs_both_a_nul_floor_and_a_ratio() {
    // Below the absolute floor, ratio alone is not enough — a two-byte file
    // that is 50% NUL is not evidence of anything.
    let (_, report) = decode_source(b"a\x00");
    assert_eq!(report.non_text, None);
    assert!(!should_abstain(Some(&report)));
    // Above the floor but below the ratio, likewise.
    let mut bytes = vec![b'x'; 200];
    bytes.extend(std::iter::repeat_n(0u8, 10));
    let (_, report) = decode_source(&bytes);
    assert_eq!(report.non_text, None);
    assert!(!should_abstain(Some(&report)));
}

// #1325 regression guard — the complementary path.

#[test]
fn positions_never_panic_on_offsets_inside_a_multi_byte_character() {
    // #1325 was a panic on valid UTF-8 whose span landed mid-character. This
    // module's own offset arithmetic must not reintroduce it: every offset
    // into a multi-byte string, plus every out-of-range offset, must clamp.
    let text = "aé𝄞漢\u{fffd}z";
    for offset in 0..=text.len() + 8 {
        let _ = super::position_of(text, offset);
        let _ = super::range_at(text, offset, 1);
    }
}

#[test]
fn decode_of_a_large_buffer_stays_linear_and_reports_once() {
    // A megabyte of valid UTF-8 with one bad byte at the very end: the scan
    // must find it, and must not degrade into per-byte re-validation.
    let mut bytes = "puts hello\n".repeat(90_000).into_bytes();
    let offset = bytes.len();
    bytes.push(0xFF);
    let (_, report) = decode_source(&bytes);
    assert_eq!(report.error_count, 1);
    assert_eq!(report.first_error.map(|e| e.byte_offset), Some(offset));
}
