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

//! Issue #1333 — `DiagnosticTag` on the wire, and issue #1326's encoding
//! codes on the wire.
//!
//! These assert on the **raw `publishDiagnostics` payload**, deliberately.
//! Every diagnostic here already fired before the fix; a test that only
//! checked the `code` would have passed against the broken server. The `tags`
//! array is the whole point, so the `tags` array is what is asserted.

use crate::common::{Lsp, unique_uri};

use serde_json::Value;

/// LSP `DiagnosticTag.Unnecessary`.
const UNNECESSARY: i64 = 1;
/// LSP `DiagnosticTag.Deprecated`.
const DEPRECATED: i64 = 2;

/// A diagnostic's `code`, as a string.
fn code_str(d: &Value) -> Option<&str> {
    d.get("code").and_then(Value::as_str)
}

/// The raw `tags` array of the diagnostic carrying `code`, as sent on the
/// wire. `None` distinguishes "the field is absent" (what the server sent
/// before this fix) from "the field is an empty array".
fn tags_of(diags: &[Value], code: &str) -> Option<Vec<i64>> {
    let d = diags.iter().find(|d| code_str(d) == Some(code))?;
    Some(
        d.get("tags")?
            .as_array()?
            .iter()
            .filter_map(Value::as_i64)
            .collect(),
    )
}

/// Diagnostics carrying `code`.
fn with_code(diags: &[Value], code: &str) -> Vec<Value> {
    diags
        .iter()
        .filter(|d| code_str(d) == Some(code))
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// Unnecessary — the reported symptom.
// ---------------------------------------------------------------------------

/// nico's exact repro from the issue.
const UNUSED_PARAM: &str =
    "proc test {a b c} {\n    set result [expr {$a + $b}]\n    return $result\n}\ntest 1 2 3\n";

#[test]
fn w214_unused_parameter_carries_tags_1_on_the_wire() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, UNUSED_PARAM);
    assert_eq!(
        tags_of(&diags, "W214"),
        Some(vec![UNNECESSARY]),
        "W214 must arrive tagged Unnecessary — got {diags:?}"
    );
}

#[test]
fn w214_keeps_hint_severity_once_tagged() {
    // The issue's explicit position: the tag is the fix, not the severity.
    // An unused parameter is very often deliberate (interface conformance,
    // callback signatures), so promoting it to a warning would misreport it.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, UNUSED_PARAM);
    let w214 = with_code(&diags, "W214");
    assert_eq!(w214.len(), 1, "got {diags:?}");
    assert_eq!(
        w214[0].get("severity").and_then(Value::as_i64),
        Some(4),
        "W214 must stay a Hint"
    );
    // ...and the span is still the one character, so the fade is the only
    // thing a user can reasonably notice.
    assert_eq!(
        w214[0]
            .pointer("/range/start/character")
            .and_then(Value::as_i64),
        Some(15)
    );
}

#[test]
fn w211_unused_variable_carries_tags_1_on_the_wire() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {} {\n    set unused 1\n    return 2\n}\np\n");
    assert_eq!(
        tags_of(&diags, "W211"),
        Some(vec![UNNECESSARY]),
        "{diags:?}"
    );
}

// ---------------------------------------------------------------------------
// Deprecated.
// ---------------------------------------------------------------------------

#[test]
fn deprecated_irules_command_carries_tags_2_on_the_wire() {
    // `HTTP::class` is deprecated in the iRules registry. Which commands are
    // deprecated is registry data, so this asserts the whole chain: spec →
    // IRULE2002 → code table → `tags: [2]`.
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready(
        &uri,
        "when HTTP_REQUEST priority 500 {\n    if { [HTTP::class match [HTTP::uri] equals dg] } {\n        log local0. \"hit\"\n    }\n}\n",
    );
    assert_eq!(
        tags_of(&diags, "IRULE2002"),
        Some(vec![DEPRECATED]),
        "{diags:?}"
    );
}

// ---------------------------------------------------------------------------
// Negative: tags land on exactly the intended codes and on no others.
// ---------------------------------------------------------------------------

#[test]
fn ordinary_diagnostics_carry_no_tags_field_at_all() {
    // A `tags: []` would be legal LSP but tells a client nothing; every
    // untagged diagnostic must keep the field absent, as it always was.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if $a {puts x}\ncatch {error e}\n");
    assert!(!diags.is_empty());
    for d in &diags {
        assert_eq!(
            d.get("tags"),
            None,
            "{:?} should not carry tags",
            code_str(d)
        );
    }
}

#[test]
fn read_before_set_is_never_faded() {
    // W210 describes a real defect. Fading it would hide it — the one way
    // "tag the unused-variable family" could go wrong.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {} {\n    puts $never\n}\np\n");
    let w210 = with_code(&diags, "W210");
    assert!(!w210.is_empty(), "expected W210 in {diags:?}");
    assert_eq!(w210[0].get("tags"), None);
}
