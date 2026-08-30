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

//! Issue #1326 — the encoding-integrity codes over real JSON-RPC.
//!
//! LSP synchronisation carries text, not source bytes. The server therefore
//! retains an encoding report only when the on-disk bytes lossily decode to
//! exactly the `didOpen` text. These tests exercise that on-wire contract,
//! including the false-positive guard for unsaved Unicode text.

use crate::common::{Lsp, unique_uri};

use serde_json::{Value, json};

/// A diagnostic's `code`, as a string.
fn code_str(d: &Value) -> Option<&str> {
    d.get("code").and_then(Value::as_str)
}

/// Every code in the payload, sorted and deduplicated.
fn codes(diags: &[Value]) -> Vec<String> {
    let mut v: Vec<String> = diags
        .iter()
        .filter_map(|d| code_str(d).map(str::to_owned))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Diagnostics carrying `code`.
fn with_code(diags: &[Value], code: &str) -> Vec<Value> {
    diags
        .iter()
        .filter(|d| code_str(d) == Some(code))
        .cloned()
        .collect()
}

/// The shared iRule body every case wraps, so counts are comparable.
const BODY: &str = "when HTTP_REQUEST priority 500 {\n    if { [HTTP::uri] eq \"/a\" } {\n        log local0. \"hit\"\n    }\n}\n";

/// Put `bytes` at a unique on-disk URI and return the URI plus the exact text
/// a lossy editor decode would send in `didOpen`.
fn malformed_file(bytes: &[u8], suffix: &str) -> (String, std::path::PathBuf, String) {
    let path =
        std::env::temp_dir().join(format!("tcl-lsp-issue1326-{}-{suffix}", std::process::id()));
    std::fs::write(&path, bytes).expect("write encoding fixture");
    let uri = format!("file://{}", path.display());
    (uri, path, String::from_utf8_lossy(bytes).into_owned())
}

// W107 — the file is not valid UTF-8.

#[test]
fn w107_fires_when_matching_on_disk_bytes_prove_a_decode_failure() {
    let bytes = b"when HTTP_REQUEST { log local0. \"hi\xfft\" }\n";
    let (uri, path, text) = malformed_file(bytes, "bad-utf8.irule");
    let mut lsp = Lsp::irules();
    let diags = lsp.open_ready(&uri, &text);
    std::fs::remove_file(path).expect("remove encoding fixture");
    let w107 = with_code(&diags, "W107");
    assert_eq!(w107.len(), 1, "expected exactly one W107 in {diags:?}");
    assert!(
        w107[0]
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|m| m.contains("byte offset")),
        "matching bytes should retain the exact byte evidence: {w107:?}"
    );
}

#[test]
fn w107_range_selects_the_decoder_insertion_not_an_earlier_literal_replacement() {
    let mut bytes = "set authored \u{fffd}\nputs ".as_bytes().to_vec();
    bytes.push(0xff);
    bytes.push(b'\n');
    let (uri, path, text) = malformed_file(&bytes, "range.tcl");
    let mut lsp = Lsp::tcl();
    let diags = lsp.open_ready(&uri, &text);
    std::fs::remove_file(path).expect("remove encoding fixture");
    let w107 = with_code(&diags, "W107");
    assert_eq!(w107.len(), 1, "{diags:?}");
    assert_eq!(w107[0].pointer("/range/start/line"), Some(&json!(1)));
    assert_eq!(w107[0].pointer("/range/start/character"), Some(&json!(5)));
    assert_eq!(w107[0].pointer("/range/end/character"), Some(&json!(6)));
}

#[test]
fn w107_is_silent_on_clean_source_and_on_legitimate_non_ascii() {
    for text in [
        BODY.to_owned(),
        BODY.replace("\"hit\"", "\"héllo\""),
        BODY.replace("\"hit\"", "\"مرحبا\""),
    ] {
        let mut lsp = Lsp::irules();
        let uri = unique_uri("irule");
        let diags = lsp.open_ready(&uri, &text);
        assert!(
            with_code(&diags, "W107").is_empty(),
            "false positive on {text:?}: {diags:?}"
        );
    }
}

#[test]
fn w107_is_silent_for_an_unsaved_literal_replacement_character() {
    // TN/FP guard: an LSP string alone cannot distinguish this authored
    // character from a decoder substitution, so it must never claim a fault.
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready(&uri, &BODY.replace("\"hit\"", "\"hi\u{fffd}t\""));
    assert!(with_code(&diags, "W107").is_empty(), "{diags:?}");
}

// W109 — the file is not UTF-8 text at all, and the analysis abstains.

#[test]
fn w109_fires_on_matching_utf16_bytes_and_suppresses_the_nonsense() {
    // Before this fix the same file produced dozens of E102/W108/W123
    // findings about characters that were half of a UTF-16 code unit.
    let bytes: Vec<u8> = BODY.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let (uri, path, text) = malformed_file(&bytes, "utf16le.irule");
    let mut lsp = Lsp::irules();
    let diags = lsp.open_ready(&uri, &text);
    std::fs::remove_file(path).expect("remove encoding fixture");
    assert_eq!(
        codes(&diags),
        vec!["W109".to_owned()],
        "abstention should leave exactly one accurate finding, got {diags:?}"
    );
    assert_eq!(
        diags[0]
            .pointer("/range/start/line")
            .and_then(Value::as_i64),
        Some(0),
        "W109 is a file-level finding"
    );
}

#[test]
fn w109_is_silent_on_valid_source_containing_a_few_nuls() {
    // The false-positive guard: NULs in otherwise-valid UTF-8 are real text.
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let text = BODY.replace("\"hit\"", "\"hi\u{0}\u{0}t\"");
    let diags = lsp.open_ready(&uri, &text);
    assert!(
        with_code(&diags, "W109").is_empty(),
        "false positive: {diags:?}"
    );
    // ...and the ordinary iRules findings still come through, i.e. the
    // abstention did not fire.
    assert!(diags.len() > 1, "expected normal analysis: {diags:?}");
}

#[test]
fn w109_does_not_guess_from_an_unsaved_nul_interleaved_buffer() {
    // FP guard: Tcl permits NUL characters in a string. The raw byte signature
    // is required before the server abstains from normal analysis.
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let text: String = BODY.chars().flat_map(|c| [c, '\u{0}']).collect();
    let diags = lsp.open_ready(&uri, &text);
    assert!(with_code(&diags, "W109").is_empty(), "{diags:?}");
}

#[test]
fn w109_disabled_still_abstains_for_bigip_and_apl_documents() {
    let cases = [
        (
            "utf16le.conf",
            "tcl-bigip",
            "ltm rule /Common/r {\n  when HTTP_REQUEST { pool /Common/missing }\n}\n",
        ),
        (
            "utf16le.apl",
            "tcl-apl",
            "section basic {\n  string unused\n}\n",
        ),
    ];
    for (suffix, language_id, source) in cases {
        let bytes: Vec<u8> = source.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let (uri, path, text) = malformed_file(&bytes, suffix);
        let mut lsp = Lsp::tcl();
        lsp.apply_configuration_settle(
            json!({
                "features": { "linkedEditingRange": true },
                "diagnostics": { "W109": false }
            }),
            &uri,
            |config| {
                config["disabled_diagnostics"]
                    .as_array()
                    .is_some_and(|codes| codes.iter().any(|code| code == "W109"))
            },
        );
        lsp.open_document_lang(&uri, &text, language_id, 1);
        let pushed =
            lsp.await_diagnostics_version(&uri, Some(1), std::time::Duration::from_secs(30));
        let pulled = lsp.pull_diagnostics(&uri);
        std::fs::remove_file(path).expect("remove encoding fixture");
        assert!(
            pushed.is_empty() && pulled.is_empty(),
            "disabling W109 hides its message but must not publish derived findings: \
             pushed={pushed:?}, pulled={pulled:?}"
        );
    }
}

// W305 — bidi controls (Trojan Source).

#[test]
fn w305_fires_on_a_bidi_override_at_error_severity() {
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let text = BODY.replace("\"hit\"", "\"hi\u{202e}drowssap\u{202c}t\"");
    let diags = lsp.open_ready(&uri, &text);
    let w305 = with_code(&diags, "W305");
    assert_eq!(w305.len(), 2, "one per control character: {diags:?}");
    for d in &w305 {
        assert_eq!(
            d.get("severity").and_then(Value::as_i64),
            Some(1),
            "W305 is an error — a file that lies to its reviewer is not a style preference"
        );
        assert!(
            d.get("message")
                .and_then(Value::as_str)
                .is_some_and(|m| m.contains("Trojan Source")),
            "{d:?}"
        );
    }
    // The generic non-ASCII lint must not double-report the same characters.
    let w108_ranges: Vec<Value> = with_code(&diags, "W108")
        .iter()
        .filter_map(|d| d.get("range").cloned())
        .collect();
    for d in &w305 {
        assert!(
            !w108_ranges.contains(d.get("range").expect("range")),
            "W108 and W305 both claimed the same character"
        );
    }
}

#[test]
fn w305_is_silent_on_ordinary_right_to_left_content() {
    // Arabic or Hebrew *content* is not a Trojan Source attack. Nor are the
    // directional marks legitimate bidirectional text uses.
    for marker in ["مرحبا بالعالم", "שלום עולם", "\u{200e}1\u{200f}2\u{61c}3"] {
        let mut lsp = Lsp::irules();
        let uri = unique_uri("irule");
        let diags = lsp.open_ready(&uri, &BODY.replace("\"hit\"", &format!("\"{marker}\"")));
        assert!(
            with_code(&diags, "W305").is_empty(),
            "false positive on {marker:?}: {diags:?}"
        );
    }
}

#[test]
fn w305_finds_a_control_in_a_comment_too() {
    // The classic attack hides in a comment, which a per-argument scan would
    // never reach.
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready(&uri, &format!("# \u{202e} reversed comment\n{BODY}"));
    assert_eq!(with_code(&diags, "W305").len(), 1, "{diags:?}");
}

#[test]
fn f5_model_documents_publish_w107_and_w305_on_push_and_pull() {
    let cases = [
        (
            "integrity.conf",
            "tcl-bigip",
            "ltm rule /Common/r {\n  when HTTP_REQUEST {\n    # \u{202e}hidden\n    pool /Common/missing\n  }\n}\n",
        ),
        (
            "integrity.apl",
            "tcl-apl",
            "# \u{202e}hidden\nsection basic {\n  string addr\n}\n",
        ),
    ];
    for (suffix, language_id, source) in cases {
        let mut bytes = source.as_bytes().to_vec();
        bytes.insert(bytes.len().saturating_sub(1), 0xff);
        let (uri, path, text) = malformed_file(&bytes, suffix);
        let mut lsp = Lsp::tcl();
        lsp.open_document_lang(&uri, &text, language_id, 1);
        let pushed =
            lsp.await_diagnostics_version(&uri, Some(1), std::time::Duration::from_secs(30));
        let pulled = lsp.pull_diagnostics(&uri);
        std::fs::remove_file(path).expect("remove encoding fixture");
        for diagnostics in [&pushed, &pulled] {
            assert_eq!(with_code(diagnostics, "W107").len(), 1, "{diagnostics:?}");
            assert_eq!(with_code(diagnostics, "W305").len(), 1, "{diagnostics:?}");
        }
    }
}

// #1325 regression guard — the complementary path must stay fixed.

#[test]
fn valid_multi_byte_source_still_analyses_without_a_panic() {
    // #1325 was a panic on valid UTF-8 whose token span landed mid-character.
    // None of the new offset arithmetic may bring it back.
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready(&uri, &BODY.replace("\"hit\"", "\"日本語 𝄞 émoji 🎈\""));
    assert!(
        with_code(&diags, "W107").is_empty() && with_code(&diags, "W109").is_empty(),
        "valid multi-byte source is not an encoding fault: {diags:?}"
    );
}
