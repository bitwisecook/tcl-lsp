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

//! Native port of `tests/lsp_e2e/test_code_actions_e2e.py`.
//!
//! Code actions, end-to-end against the packaged server. Full-parity port of
//! the Tcl-dialect cases in `tests/test_code_actions.py`. The realistic flow
//! drives diagnostics back into the `codeAction` context — exactly what an
//! editor does — so quick-fix offers are exercised against real (or, where a
//! specific range matters, fabricated) diagnostics.
//!
//! The F5 iRules-only code actions (collect-bootstrap, the IRULE3001/3002
//! wrap-with-encode taint fixes, profile headers) run against the dedicated
//! iRules server in `irules.rs`. T102 (tainted data in option position) is
//! dialect-general — not an iRules-specific check — so its `--`-insertion
//! fix is exercised here against the plain `tcl` dialect.

use crate::common::{Lsp, unique_uri};

use serde_json::{Value, json};

// -- local helpers -------------------------------------------------------

/// An LSP `Range` from `(line, char)` tuples.
fn range(start: (u32, u32), end: (u32, u32)) -> Value {
    json!({
        "start": { "line": start.0, "character": start.1 },
        "end": { "line": end.0, "character": end.1 },
    })
}

/// Request code actions with an explicit `only` filter (the harness's
/// `code_actions` has no `only` parameter, so build the request directly).
fn code_actions_only(lsp: &mut Lsp, uri: &str, rng: Value, diags: Value, only: &[&str]) -> Value {
    let mut params = json!({ "textDocument": { "uri": uri }, "context": { "only": only } });
    params["range"] = rng;
    params["context"]["diagnostics"] = diags;
    lsp.request("textDocument/codeAction", params)
}

/// Every `newText` in a code-action list's workspace edits (`_new_texts`).
fn new_texts(actions: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let empty = Vec::new();
    for action in actions.as_array().unwrap_or(&empty) {
        let edit = action.get("edit").cloned().unwrap_or(Value::Null);
        if let Some(changes) = edit.get("changes").and_then(Value::as_object) {
            for edits in changes.values() {
                for e in edits.as_array().unwrap_or(&empty) {
                    if let Some(t) = e.get("newText").and_then(Value::as_str) {
                        out.push(t.to_owned());
                    }
                }
            }
        }
        if let Some(doc_changes) = edit.get("documentChanges").and_then(Value::as_array) {
            for change in doc_changes {
                for e in change
                    .get("edits")
                    .and_then(Value::as_array)
                    .unwrap_or(&empty)
                {
                    if let Some(t) = e.get("newText").and_then(Value::as_str) {
                        out.push(t.to_owned());
                    }
                }
            }
        }
    }
    out
}

/// The `title` of every action (`_titles`).
fn titles(actions: &Value) -> Vec<String> {
    actions
        .as_array()
        .map(|a| {
            a.iter()
                .map(|x| {
                    x.get("title")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_owned()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The `title` of a single action.
fn action_title(action: &Value) -> &str {
    action.get("title").and_then(Value::as_str).unwrap_or("")
}

/// Actions whose `kind` starts with `kind_prefix` (`_kinds`).
fn kinds(actions: &Value, kind_prefix: &str) -> Vec<Value> {
    actions
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|x| {
                    x.get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .starts_with(kind_prefix)
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// A synthetic diagnostic (`_diag`).
fn diag(code: &str, message: &str, start: (u32, u32), end: (u32, u32)) -> Value {
    json!({
        "range": {
            "start": { "line": start.0, "character": start.1 },
            "end": { "line": end.0, "character": end.1 },
        },
        "code": code,
        "message": message,
        "source": "tcl-lsp",
    })
}

/// Diagnostics from `diags` carrying `code`.
fn with_code(diags: &[Value], code: &str) -> Vec<Value> {
    diags
        .iter()
        .filter(|d| d.get("code").and_then(Value::as_str) == Some(code))
        .cloned()
        .collect()
}

// -- TestQuickFixes ------------------------------------------------------

#[test]
fn test_w100_offers_brace_wrap() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if $a {puts x}\n");
    let w100 = with_code(&diags, "W100");
    assert!(
        !w100.is_empty(),
        "expected a W100 diagnostic to drive the quick fix"
    );
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 14)), json!(w100));
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("brace"))
    );
    assert!(new_texts(&actions).iter().any(|nt| nt.contains("{$a}")));
}

#[test]
fn test_w003_offers_lsearch_fix_for_in() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.4\nexpr {2 in {1 2 3}}\n");
    let w003 = with_code(&diags, "W003");
    assert_eq!(w003.len(), 1, "{diags:?}");
    let actions = lsp.code_actions(&uri, range((1, 0), (1, 19)), json!(w003));
    assert!(
        new_texts(&actions)
            .iter()
            .any(|nt| nt == "([lsearch -exact {1 2 3} 2] >= 0)"),
        "{actions:?}"
    );
}

#[test]
fn test_w003_offers_string_compare_fix_for_lt() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.4\nif {$x lt $y} { puts hi }\n");
    let w003 = with_code(&diags, "W003");
    assert_eq!(w003.len(), 1, "{diags:?}");
    let actions = lsp.code_actions(&uri, range((1, 0), (1, 26)), json!(w003));
    assert!(
        new_texts(&actions)
            .iter()
            .any(|nt| nt == "([string compare $x $y] < 0)"),
        "{actions:?}"
    );
}

#[test]
fn test_w003_no_fix_when_operator_nested_in_larger_expression() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl8.4\nif {$a in $b && $c} { puts hi }\n",
    );
    let w003 = with_code(&diags, "W003");
    assert_eq!(w003.len(), 1, "{diags:?}");
    let actions = lsp.code_actions(&uri, range((1, 0), (1, 32)), json!(w003));
    assert!(
        new_texts(&actions).iter().all(|nt| !nt.contains("lsearch")),
        "a nested occurrence must not offer the lsearch rewrite: {actions:?}"
    );
}

#[test]
fn test_w302_adds_result_capture_actions() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "catch {error oops}\n");
    let w302 = with_code(&diags, "W302");
    assert!(!w302.is_empty());
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 0), (0, 4)),
        json!(w302),
        &["quickfix"],
    );
    let snippets = new_texts(&actions);
    assert!(snippets.iter().any(|s| s == " result"));
    assert!(snippets.iter().any(|s| s == " result opts"));
}

#[test]
fn test_w302_no_fix_when_result_present() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "catch {error oops} result\n");
    // Fabricate the W302 range an editor would send; the server must still
    // decline because the source already captures a result variable.
    let d = json!({
        "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 4 } },
        "code": "W302",
        "message": "catch without a result variable silently swallows errors.",
        "source": "tcl-lsp",
    });
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 0), (0, 4)),
        json!([d]),
        &["quickfix"],
    );
    assert!(new_texts(&actions).is_empty());
}

#[test]
fn test_e004_extra_words_offers_merge_into_body_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if {1} {a} {b} {c}\n");
    let e004 = with_code(&diags, "E004");
    assert!(!e004.is_empty(), "expected an E004 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 15), (0, 18)),
        json!(e004),
        &["quickfix"],
    );
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("merge"))
    );
    assert!(new_texts(&actions).iter().any(|nt| nt == "{{b} {c}}"));
}

#[test]
fn test_e004_dangling_elseif_offers_remove_clause_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if {1} {a} elseif\n");
    let e004 = with_code(&diags, "E004");
    assert!(!e004.is_empty(), "expected an E004 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 12), (0, 18)),
        json!(e004),
        &["quickfix"],
    );
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("remove"))
    );
    assert!(new_texts(&actions).iter().any(String::is_empty));
}

#[test]
fn test_e004_missing_first_clause_offers_no_fix() {
    // `if {1}` has no well-formed prefix to fall back to — no fix.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if {1}\n");
    let e004 = with_code(&diags, "E004");
    assert!(!e004.is_empty(), "expected an E004 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 3), (0, 6)),
        json!(e004),
        &["quickfix"],
    );
    assert!(new_texts(&actions).is_empty(), "got {actions:?}");
}

#[test]
fn test_w004_offers_remove_option_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# tcl-dialect: tcl8.6\nlsearch -stride 2 {a b c d} b\n";
    let diags = lsp.open_ready(&uri, src);
    let w004 = with_code(&diags, "W004");
    assert!(!w004.is_empty(), "expected a W004 diagnostic: {diags:?}");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (1, 30)),
        json!(w004),
        &["quickfix"],
    );
    assert!(
        titles(&actions).iter().any(|t| t.contains("-stride")),
        "expected a 'Remove ...-stride...' quick fix: {:?}",
        titles(&actions)
    );
    assert!(
        new_texts(&actions).iter().any(String::is_empty),
        "the remove-option fix deletes text (empty newText): {:?}",
        new_texts(&actions)
    );
}

/// T102 (option injection) is dialect-general, not iRules-specific, so its
/// `--`-insertion fix must reach the code-action response for a plain `tcl`
/// document too — not only under `f5-irules` (see `irules::t102_insert_double_dash`).
#[test]
fn test_t102_insert_double_dash_default_dialect() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set pattern [gets stdin]\nregexp $pattern $haystack\n",
    );
    let t102 = with_code(&diags, "T102");
    assert!(
        !t102.is_empty(),
        "expected a T102 diagnostic to drive the quick fix, got {diags:?}"
    );
    let t102_range = t102[0]["range"].clone();
    let actions = code_actions_only(&mut lsp, &uri, t102_range, json!(t102), &["quickfix"]);
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("--"))
        .cloned()
        .collect();
    assert!(
        !fixes.is_empty(),
        "expected at least one '--' quick fix, got {actions:?}"
    );
    assert!(new_texts(&json!(fixes)).iter().any(|s| s == "-- "));
}

/// The `set matched [regexp $pattern $s]` capture idiom must fire T102
/// exactly like the bare `regexp $pattern $s` call it wraps — a false
/// negative in the taint pass's statement-shape handling, not a code-action
/// concern, but only observable end-to-end through published diagnostics.
#[test]
fn test_t102_fires_for_assign_value_wrapped_call() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set pattern [gets stdin]\nset result [regexp $pattern $haystack]\n",
    );
    let t102 = with_code(&diags, "T102");
    assert!(
        !t102.is_empty(),
        "expected T102 for tainted $pattern inside `set result [regexp ...]`, got {diags:?}"
    );
}

// -- TestRefactorActions -------------------------------------------------

#[test]
fn test_extract_proc_available_without_diagnostics() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set value 42\nputs $value\nputs done\n");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (2, 0)),
        json!([]),
        &["refactor.extract"],
    );
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("extract"))
    );
}

// -- TestRefactorActionsExtended -----------------------------------------

#[test]
fn test_extract_proc_snippets() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set value 42\nputs $value\nputs done\n");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (2, 0)),
        json!([]),
        &["refactor.extract"],
    );
    let extract = kinds(&actions, "refactor.extract");
    assert!(!extract.is_empty());
    let snippets = new_texts(&json!(extract));
    assert!(
        snippets
            .iter()
            .any(|s| s.contains("proc extracted_proc {value}"))
    );
    assert!(snippets.iter().any(|s| s.contains("extracted_proc $value")));
}

#[test]
fn test_extract_proc_attaches_rename_command() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set value 42\nputs $value\nputs done\n");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (2, 0)),
        json!([]),
        &["refactor.extract"],
    );
    let extract = kinds(&actions, "refactor.extract");
    let cmd = extract[0].get("command").cloned().unwrap_or(Value::Null);
    assert!(!cmd.is_null());
    assert_eq!(
        cmd.get("command").and_then(Value::as_str),
        Some("tclLsp.renameSymbolAtPosition")
    );
    let args = cmd
        .get("arguments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let line = args[0].as_i64().unwrap();
    let start = args[1].as_i64().unwrap();
    let end = args[2].as_i64().unwrap();
    assert_eq!(line, 0);
    assert_eq!(start, i64::try_from("proc ".len()).unwrap());
    assert_eq!(end, start + i64::try_from("extracted_proc".len()).unwrap());
}

#[test]
fn test_inline_proc_available() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {name} { puts $name }\ngreet Bob\n");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 1), (1, 1)),
        json!([]),
        &["refactor.inline"],
    );
    let inline = kinds(&actions, "refactor.inline");
    assert!(!inline.is_empty());
    assert!(
        new_texts(&json!(inline))
            .iter()
            .any(|s| s.contains("puts Bob"))
    );
}

#[test]
fn test_inline_proc_skips_returning_proc() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc wrap {x} { return $x }\nwrap value\n");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 1), (1, 1)),
        json!([]),
        &["refactor.inline"],
    );
    assert!(kinds(&actions, "refactor.inline").is_empty());
}

// -- TestExprRefactorActions ---------------------------------------------

/// The `_rewrite` helper: request `refactor.rewrite`-only actions.
fn rewrite(lsp: &mut Lsp, uri: &str, start: (u32, u32), end: (u32, u32)) -> Vec<Value> {
    let actions = code_actions_only(
        lsp,
        uri,
        range(start, end),
        json!([]),
        &["refactor.rewrite"],
    );
    kinds(&actions, "refactor.rewrite")
}

#[test]
fn test_demorgan_forward() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "if {!($a && $b)} { puts yes }\n");
    let dm: Vec<Value> = rewrite(&mut lsp, &uri, (0, 4), (0, 15))
        .into_iter()
        .filter(|a| action_title(a).contains("De Morgan"))
        .collect();
    assert_eq!(dm.len(), 1);
    assert!(
        new_texts(&json!(dm))
            .iter()
            .any(|s| s.contains("!$a || !$b"))
    );
}

#[test]
fn test_demorgan_reverse() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "if {!$a || !$b} { puts yes }\n");
    let dm: Vec<Value> = rewrite(&mut lsp, &uri, (0, 4), (0, 14))
        .into_iter()
        .filter(|a| action_title(a).contains("De Morgan"))
        .collect();
    assert_eq!(dm.len(), 1);
    assert!(
        new_texts(&json!(dm))
            .iter()
            .any(|s| s.contains("!($a && $b)"))
    );
}

#[test]
fn test_invert_expression() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "if {$a == $b} { puts yes }\n");
    let inv: Vec<Value> = rewrite(&mut lsp, &uri, (0, 4), (0, 12))
        .into_iter()
        .filter(|a| action_title(a).contains("Invert"))
        .collect();
    assert_eq!(inv.len(), 1);
    assert!(
        new_texts(&json!(inv))
            .iter()
            .any(|s| s.contains("$a != $b"))
    );
}

#[test]
fn test_no_actions_for_non_expression_selection() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts hello\n");
    let acts = rewrite(&mut lsp, &uri, (0, 0), (0, 4));
    assert!(
        !acts
            .iter()
            .any(|a| action_title(a).contains("De Morgan") || action_title(a).contains("Invert"))
    );
}

#[test]
fn test_no_actions_for_empty_selection() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "if {$a == $b} { puts yes }\n");
    let acts = rewrite(&mut lsp, &uri, (0, 5), (0, 5));
    assert!(
        !acts
            .iter()
            .any(|a| action_title(a).contains("De Morgan") || action_title(a).contains("Invert"))
    );
}

#[test]
fn test_invert_subexpression_in_compound() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "if {$a == $b && $c < $d} { puts yes }\n");
    let inv: Vec<Value> = rewrite(&mut lsp, &uri, (0, 16), (0, 23))
        .into_iter()
        .filter(|a| action_title(a).contains("Invert"))
        .collect();
    assert_eq!(inv.len(), 1);
}

// -- TestW115CommentContinuation -----------------------------------------

#[test]
fn test_simple_continuation_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "# hello \\\nworld");
    let d = diag("W115", "Backslash-newline in comment", (0, 0), (1, 4));
    let actions = lsp.code_actions(&uri, range((0, 0), (1, 4)), json!([d]));
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("per-line"))
        .cloned()
        .collect();
    assert_eq!(fixes.len(), 1);
    let snippet = &new_texts(&json!(fixes))[0];
    assert!(snippet.contains("# hello"));
    assert!(snippet.contains("# world"));
    assert!(!snippet.contains('\\'));
}

#[test]
fn test_chained_continuation_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "# line1 \\\nline2 \\\nline3");
    let d = diag("W115", "Backslash-newline in comment", (0, 0), (2, 4));
    let actions = lsp.code_actions(&uri, range((0, 0), (2, 4)), json!([d]));
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("per-line"))
        .cloned()
        .collect();
    assert_eq!(fixes.len(), 1);
    let snippet = &new_texts(&json!(fixes))[0];
    assert!(snippet.contains("# line1"));
    assert!(snippet.contains("# line2"));
    assert!(snippet.contains("# line3"));
}

#[test]
fn test_already_commented_continuation_not_doubled() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "# line1 \\\n# line2");
    let d = diag("W115", "Backslash-newline in comment", (0, 0), (1, 6));
    let actions = lsp.code_actions(&uri, range((0, 0), (1, 6)), json!([d]));
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("per-line"))
        .cloned()
        .collect();
    let texts = new_texts(&json!(fixes));
    let lines: Vec<&str> = texts[0].split('\n').collect();
    assert_eq!(lines[0], "# line1");
    assert_eq!(lines[1], "# line2");
}

// -- TestIPConversionActions ---------------------------------------------

#[test]
fn test_ipv4_offers_ipv6_mapped() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set addr 10.0.0.1\n");
    let actions = lsp.code_actions(&uri, range((0, 10), (0, 10)), json!([]));
    assert!(titles(&actions).iter().any(|t| t.contains("IPv6-mapped")));
    assert!(new_texts(&actions).iter().any(|s| s.contains("::ffff:")));
}

#[test]
fn test_ipv6_mapped_offers_ipv4() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set addr ::ffff:192.168.1.1\n");
    let actions = lsp.code_actions(&uri, range((0, 12), (0, 12)), json!([]));
    assert!(titles(&actions).iter().any(|t| t.contains("IPv4")));
    assert!(
        new_texts(&actions)
            .iter()
            .any(|s| s.contains("192.168.1.1"))
    );
}

#[test]
fn test_non_mapped_ipv6_no_ipv4_action() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set addr ::1\n");
    let actions = lsp.code_actions(&uri, range((0, 10), (0, 10)), json!([]));
    assert!(
        !titles(&json!(kinds(&actions, "refactor")))
            .iter()
            .any(|t| t.contains("IPv4"))
    );
}

#[test]
fn test_ipv4_with_prefix_preserves_suffix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set net 10.0.0.0/8\n");
    let actions = lsp.code_actions(&uri, range((0, 10), (0, 10)), json!([]));
    assert!(
        new_texts(&json!(kinds(&actions, "refactor")))
            .iter()
            .any(|s| s.contains("/8"))
    );
}

#[test]
fn test_non_ip_word_no_action() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set name hello\n");
    let actions = lsp.code_actions(&uri, range((0, 10), (0, 10)), json!([]));
    assert!(kinds(&actions, "refactor").is_empty());
}

#[test]
fn test_cursor_past_end_of_line_does_not_crash() {
    // The request must succeed (no server-side IndexError); an empty result
    // comes back as JSON null over the wire rather than [].
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\n");
    let result = lsp.code_actions(&uri, range((0, 999), (0, 999)), json!([]));
    assert!(result.is_null() || result.is_array());
}

#[test]
fn test_cursor_on_empty_line_does_not_crash() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\n\nset y 2\n");
    let result = lsp.code_actions(&uri, range((1, 0), (1, 0)), json!([]));
    assert!(result.is_null() || result.is_array());
}

#[test]
fn test_cursor_past_last_line_does_not_crash() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\n");
    let result = lsp.code_actions(&uri, range((99, 0), (99, 0)), json!([]));
    assert!(result.is_null() || result.is_array());
}

// -- TestGenerateDocstringAction -----------------------------------------

#[test]
fn test_offered_for_undocumented_proc() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {name} { puts $name }\n");
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 0)), json!([]));
    let sa = kinds(&actions, "source");
    let doc: Vec<Value> = sa
        .into_iter()
        .filter(|a| action_title(a).to_lowercase().contains("docstring"))
        .collect();
    assert_eq!(doc.len(), 1);
    assert!(action_title(&doc[0]).contains("'greet'"));
}

#[test]
fn test_not_offered_for_documented_proc() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "# Already documented\nproc greet {name} { puts $name }\n",
    );
    let actions = lsp.code_actions(&uri, range((1, 0), (1, 0)), json!([]));
    let sa = kinds(&actions, "source");
    assert!(
        sa.iter()
            .all(|a| !action_title(a).to_lowercase().contains("docstring"))
    );
}

#[test]
fn test_generated_edit_contains_param_tags() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc add {a b} { expr {$a + $b} }\n");
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 0)), json!([]));
    let doc: Vec<Value> = kinds(&actions, "source")
        .into_iter()
        .filter(|a| action_title(a).to_lowercase().contains("docstring"))
        .collect();
    let snippets = new_texts(&json!(doc));
    assert!(snippets.iter().any(|s| s.contains("@param a")));
    assert!(snippets.iter().any(|s| s.contains("@param b")));
}

// -- TestEvalListQuickFix ------------------------------------------------

#[test]
fn test_eval_string_to_list_action() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "eval \"process $x\"\n");
    let w101 = with_code(&diags, "W101");
    assert!(!w101.is_empty());
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 17)), json!(w101));
    assert!(
        new_texts(&actions)
            .iter()
            .any(|s| s.contains("[list process $x]"))
    );
}

// -- TestProfilesNotOfferedForTcl ----------------------------------------

#[test]
fn test_no_profiles_action_for_tcl_dialect() {
    // On the plain-Tcl server, a proc gets a docstring source action but no
    // F5 "# Profiles:" header action.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc hello {} { puts hi }\n");
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 0)), json!([]));
    let sa = kinds(&actions, "source");
    assert!(sa.iter().all(|a| !action_title(a).contains("Profiles")));
}
// -- TestE100E102QuickFixes -----------------------------------------------

#[test]
fn test_e100_offers_insert_bracket_before_known_command() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "puts string]\n");
    let e100 = with_code(&diags, "E100");
    assert!(!e100.is_empty(), "expected an E100 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 0), (0, 12)),
        json!(e100),
        &["quickfix"],
    );
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("insert") && t.contains('[')),
        "{:?}",
        titles(&actions)
    );
    assert!(new_texts(&actions).iter().any(|s| s == "["));
}

#[test]
fn test_e100_offers_insert_bracket_before_user_proc() {
    // The bracket-insertion heuristic must recognise a call to an
    // already-declared user proc, not just a registry builtin.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc myHelper {a b} {return $a}\nset y myHelper arg1 arg2]\n";
    let diags = lsp.open_ready(&uri, src);
    let e100 = with_code(&diags, "E100");
    assert!(!e100.is_empty(), "expected an E100 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (1, 26)),
        json!(e100),
        &["quickfix"],
    );
    assert!(new_texts(&actions).iter().any(|s| s == "["));
}

#[test]
fn test_e100_no_fix_offered_without_known_command_or_overflow() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x blah]\n");
    let e100 = with_code(&diags, "E100");
    assert!(!e100.is_empty(), "expected an E100 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 0), (0, 11)),
        json!(e100),
        &["quickfix"],
    );
    // No recognisable command / arity overflow, so the E100-specific
    // bracket-insertion fix must not be offered (other, unrelated
    // quickfixes for the range — e.g. a W123 "did you mean" suggestion
    // on the unresolved `blah` command — may still appear).
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("insert") && t.contains('[')),
        "{:?}",
        titles(&actions)
    );
}

#[test]
fn test_e102_offers_remove_extra_brace_for_isolated_line() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x 1\n}\n");
    let e102 = with_code(&diags, "E102");
    assert!(!e102.is_empty(), "expected an E102 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (1, 1)),
        json!(e102),
        &["quickfix"],
    );
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("remove")),
        "{:?}",
        titles(&actions)
    );
    assert!(new_texts(&actions).iter().any(String::is_empty));
}

#[test]
fn test_e102_no_fix_offered_for_embedded_brace() {
    // `foo}bar` is flagged but not auto-fixed — deleting one character
    // out of a bareword isn't a safe guess the way an isolated stray-brace
    // line is.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x foo}bar\n");
    let e102 = with_code(&diags, "E102");
    assert!(!e102.is_empty(), "expected an E102 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 0), (0, 13)),
        json!(e102),
        &["quickfix"],
    );
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("remove") && t.contains('}')),
        "{:?}",
        titles(&actions)
    );
}

// -- TestShimmerNoqaSuppressQuickFix --------------------------------------

#[test]
fn test_s100_offers_noqa_suppress_action() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x hello\nincr x\n");
    let s100 = with_code(&diags, "S100");
    assert!(!s100.is_empty(), "expected an S100 to drive the quick fix");
    let actions = lsp.code_actions(&uri, range((1, 0), (1, 6)), json!(s100));
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t == "Suppress S100 with a noqa comment"),
        "expected an S100 noqa-suppress action, got {actions:?}"
    );
    assert!(new_texts(&actions).iter().any(|s| s == "# noqa: S100\n"));
}
