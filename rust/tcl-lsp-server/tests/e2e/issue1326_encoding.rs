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
//! What reaches the server here is *text*, because that is what an editor
//! sends: VS Code decodes the file itself and hands the result over
//! `didOpen`. So these tests exercise the text-only precision tier described
//! in `tcl_lsp_core::source_decode` — the same codes the byte tier raises,
//! with messages that cannot name a byte offset. The byte tier's precision is
//! covered by `tcl-lsp-core`'s unit tests and by the `tcl diag` fixtures.

use crate::common::{Lsp, unique_uri};

use serde_json::Value;

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

// ---------------------------------------------------------------------------
// W107 — the file is not valid UTF-8.
// ---------------------------------------------------------------------------

#[test]
fn w107_fires_on_text_carrying_replacement_characters() {
    // What the editor hands us after *its* lossy decode of a broken file.
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let text = BODY.replace("\"hit\"", "\"hi\u{fffd}t\"");
    let diags = lsp.open_ready(&uri, &text);
    let w107 = with_code(&diags, "W107");
    assert_eq!(w107.len(), 1, "expected exactly one W107 in {diags:?}");
    assert!(
        w107[0]
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|m| m.contains("U+FFFD")),
        "message should name the replacement character: {w107:?}"
    );
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

// ---------------------------------------------------------------------------
// W109 — the file is not UTF-8 text at all, and the analysis abstains.
// ---------------------------------------------------------------------------

#[test]
fn w109_fires_on_nul_interleaved_text_and_suppresses_the_nonsense() {
    // A UTF-16LE file whose NULs survived the editor's decode. Before this
    // fix the same input produced dozens of `E102`/`W108`/`W123` findings
    // about characters that are half of a UTF-16 code unit.
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let text: String = BODY.chars().flat_map(|c| [c, '\u{0}']).collect();
    let diags = lsp.open_ready(&uri, &text);
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

// ---------------------------------------------------------------------------
// W305 — bidi controls (Trojan Source).
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// #1325 regression guard — the complementary path must stay fixed.
// ---------------------------------------------------------------------------

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
