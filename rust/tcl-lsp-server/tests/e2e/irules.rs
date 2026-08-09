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

//! Native port of `tests/lsp_e2e/test_irules_e2e.py`.
//!
//! F5 iRules dialect features, end-to-end against a dedicated server.
//!
//! These run against an iRules-dedicated server (`Lsp::irules`) rather than the
//! shared Tcl server: opening an iRules document auto-switches the server's
//! process-global command pack into the `f5-irules` dialect, so dialect-sensitive
//! cases are isolated on their own server to keep the main Tcl server
//! uncontaminated.

use crate::common::helpers::*;
use crate::common::{Lsp, scaled_timeout, unique_uri};

use serde_json::{Value, json};
use std::time::Duration;

// -- local helpers -------------------------------------------------------

/// Open `source` as an iRule and return its URI.
fn open_irule(lsp: &mut Lsp, source: &str) -> String {
    let uri = unique_uri("irule");
    lsp.open_ready_lang(&uri, source, "tcl-irule");
    uri
}

/// Completion labels at a position.
fn labels(lsp: &mut Lsp, uri: &str, line: u32, ch: u32) -> Vec<String> {
    completion_labels(&lsp.completion(uri, line, ch))
}

/// Hover text at a position.
fn hover(lsp: &mut Lsp, uri: &str, line: u32, ch: u32) -> String {
    hover_text(&lsp.hover(uri, line, ch))
}

/// Build a synthetic diagnostic (`_diag` in the pytest suite).
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

/// Every `newText` in a code-action list's workspace edits (`_ca_new_texts`).
fn ca_new_texts(actions: &Value) -> Vec<String> {
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

/// The `title` of a single action.
fn action_title(action: &Value) -> &str {
    action.get("title").and_then(Value::as_str).unwrap_or("")
}

/// Actions whose `kind` is exactly `source` (`_source_actions`).
fn source_actions(actions: &Value) -> Vec<Value> {
    actions
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|x| x.get("kind").and_then(Value::as_str).unwrap_or("") == "source")
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// The set of `code` strings on a diagnostics array (`_irules_codes`).
fn irules_codes(diags: &[Value]) -> std::collections::BTreeSet<String> {
    diags
        .iter()
        .map(|d| match d.get("code") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => "None".to_owned(),
        })
        .collect()
}

/// A diagnostic's `code`, stringified.
fn code_str(d: &Value) -> String {
    match d.get("code") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => "None".to_owned(),
    }
}

/// Decode semantic tokens for an iRule and map their type index to the legend
/// name (`_irules_typed`).
fn irules_typed(lsp: &mut Lsp, uri: &str) -> Vec<(SemToken, String)> {
    let legend: Vec<String> =
        lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|v| v.as_str().unwrap_or("").to_owned())
                    .collect()
            })
            .unwrap_or_default();
    let toks = decode_semantic_tokens(&lsp.semantic_tokens(uri));
    toks.into_iter()
        .map(|t| {
            let name = legend
                .get(usize::try_from(t.ttype).unwrap())
                .cloned()
                .unwrap_or_default();
            (t, name)
        })
        .collect()
}

// -- TestIrulesHover -----------------------------------------------------

#[test]
fn irules_subcommand_hover() {
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    lsp.open_ready_lang(&uri, "HTTP::header insert X-Test 1\n", "tcl-irule");
    let text = hover(&mut lsp, &uri, 0, 15).to_lowercase();
    assert!(text.contains("insert"), "{text:?}");
    assert!(text.contains("header"), "{text:?}");
}

#[test]
fn curated_irules_hover_does_not_mark_refinement_status() {
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    lsp.open_ready_lang(
        &uri,
        "when HTTP_REQUEST { log local0. \"ok\" }\n",
        "tcl-irule",
    );
    assert!(!hover(&mut lsp, &uri, 0, 2).to_lowercase().contains("note:"));
}

#[test]
fn namespace_only_irules_hover_shows_profile_requirement() {
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    lsp.open_ready_lang(&uri, "ACCESS::log 1 \"trace\"\n", "tcl-irule");
    let text = hover(&mut lsp, &uri, 0, 5);
    assert!(text.contains("Requires"), "{text:?}");
    assert!(text.contains("ACCESS"), "{text:?}");
}

// -- TestIrulesCompletion ------------------------------------------------

#[test]
fn when_event_name_completion() {
    let mut lsp = Lsp::irules();
    let uri = open_irule(&mut lsp, "when ");
    let ls = labels(&mut lsp, &uri, 0, 5);
    assert!(ls.contains(&"HTTP_REQUEST".to_owned()));
    assert!(ls.contains(&"CLIENT_ACCEPTED".to_owned()));
}

#[test]
fn when_priority_and_timing_keywords_after_event() {
    let src = "when HTTP_REQUEST ";
    let mut lsp = Lsp::irules();
    let uri = open_irule(&mut lsp, src);
    let ls = labels(&mut lsp, &uri, 0, u32::try_from(src.len()).unwrap());
    assert!(ls.contains(&"priority".to_owned()));
    assert!(ls.contains(&"timing".to_owned()));
}

#[test]
fn when_priority_and_timing_partial_keyword() {
    let src = "when HTTP_REQUEST pr";
    let mut lsp = Lsp::irules();
    let uri = open_irule(&mut lsp, src);
    let ls = labels(&mut lsp, &uri, 0, u32::try_from(src.len()).unwrap());
    assert!(ls.contains(&"priority".to_owned()));
    assert!(!ls.contains(&"timing".to_owned()));
}

#[test]
fn when_timing_value_keywords_after_timing() {
    let src = "when HTTP_REQUEST timing ";
    let mut lsp = Lsp::irules();
    let uri = open_irule(&mut lsp, src);
    let ls = labels(&mut lsp, &uri, 0, u32::try_from(src.len()).unwrap());
    assert!(ls.contains(&"enable".to_owned()));
    assert!(ls.contains(&"disable".to_owned()));
}

#[test]
fn when_timing_values_not_suggested_after_priority() {
    let src = "when HTTP_REQUEST priority ";
    let mut lsp = Lsp::irules();
    let uri = open_irule(&mut lsp, src);
    let ls = labels(&mut lsp, &uri, 0, u32::try_from(src.len()).unwrap());
    assert!(!ls.contains(&"enable".to_owned()));
    assert!(!ls.contains(&"disable".to_owned()));
}

#[test]
fn http_header_subcommand_keywords() {
    let mut lsp = Lsp::irules();
    let uri = open_irule(&mut lsp, "HTTP::header ");
    let ls: std::collections::BTreeSet<String> =
        labels(&mut lsp, &uri, 0, 13).into_iter().collect();
    for expected in ["insert", "replace", "value"] {
        assert!(ls.contains(expected), "missing {expected:?} in {ls:?}");
    }
}

#[test]
fn http_header_partial_keyword() {
    let mut lsp = Lsp::irules();
    let uri = open_irule(&mut lsp, "HTTP::header re");
    let ls = labels(&mut lsp, &uri, 0, 15);
    assert!(ls.contains(&"remove".to_owned()));
    assert!(ls.contains(&"replace".to_owned()));
    assert!(!ls.contains(&"insert".to_owned()));
}

#[test]
fn http_respond_options_after_status_code() {
    let mut lsp = Lsp::irules();
    let uri = open_irule(&mut lsp, "HTTP::respond 302 ");
    let ls: std::collections::BTreeSet<String> =
        labels(&mut lsp, &uri, 0, 18).into_iter().collect();
    for expected in ["content", "noserver", "version"] {
        assert!(ls.contains(expected), "missing {expected:?} in {ls:?}");
    }
}

#[test]
fn irules_event_valid_command_ranked_before_invalid() {
    let mut lsp = Lsp::irules();
    let uri = open_irule(&mut lsp, "when HTTP_REQUEST {\n    \n}\n");
    let mut by = std::collections::BTreeMap::new();
    for item in completion_items(&lsp.completion(&uri, 1, 4)) {
        let label = item
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        by.insert(label, item);
    }
    let http = by["HTTP::header"]["sortText"].as_str().unwrap();
    let tcp = by["TCP::collect"]["sortText"].as_str().unwrap();
    assert!(
        http < tcp,
        "HTTP::header sortText {http:?} not < TCP::collect {tcp:?}"
    );
}

#[test]
fn when_priority_and_timing_after_priority_value() {
    let src = "when HTTP_REQUEST priority 500 ";
    let mut lsp = Lsp::irules();
    let uri = open_irule(&mut lsp, src);
    let ls = labels(&mut lsp, &uri, 0, u32::try_from(src.len()).unwrap());
    assert!(ls.contains(&"priority".to_owned()));
    assert!(ls.contains(&"timing".to_owned()));
}

#[test]
fn argument_value_has_documentation() {
    let mut lsp = Lsp::irules();
    let uri = open_irule(&mut lsp, "when ");
    let items = completion_items(&lsp.completion(&uri, 0, 5));
    assert!(!items.is_empty());
    assert!(
        items
            .iter()
            .filter(|i| i.get("label").and_then(Value::as_str) == Some("HTTP_REQUEST"))
            .any(|i| i.get("documentation").is_some_and(|d| !d.is_null())),
        "no HTTP_REQUEST item with documentation"
    );
}

// -- TestIrulesCollectCodeActions ----------------------------------------

#[test]
fn irule1005_adds_only_registered_collect_bootstrap() {
    let mut lsp = Lsp::irules();
    let uri = open_irule(
        &mut lsp,
        "when HTTP_REQUEST_DATA {\n    set payload [HTTP::payload]\n}\n",
    );
    let d = diag(
        "IRULE1005",
        "'HTTP_REQUEST_DATA' will never fire without a client HTTP::collect call in another event.",
        (0, 5),
        (0, 22),
    );
    let actions = lsp.code_actions(&uri, range((0, 5), (0, 22)), json!([d]));
    let snippets = ca_new_texts(&actions);
    assert!(
        snippets
            .iter()
            .any(|s| s.contains("when HTTP_REQUEST") && s.contains("HTTP::collect")),
        "{snippets:?}"
    );
    assert!(
        snippets.iter().all(|s| !s.contains("UDP::collect")),
        "only registry-declared collect commands are offered: {snippets:?}",
    );
}

#[test]
fn irule1006_prefers_server_ssl_handshake_bootstrap() {
    let mut lsp = Lsp::irules();
    let uri = open_irule(&mut lsp, "when SERVERSSL_DATA {\n    SSL::payload\n}\n");
    let d = diag(
        "IRULE1006",
        "'SSL::payload' without a SSL::collect call. The payload buffer will be empty.",
        (1, 4),
        (1, 16),
    );
    let actions = lsp.code_actions(&uri, range((1, 4), (1, 16)), json!([d]));
    let snippets = ca_new_texts(&actions);
    assert!(
        snippets
            .iter()
            .any(|s| s.contains("when SERVERSSL_HANDSHAKE") && s.contains("SSL::collect")),
        "{snippets:?}"
    );
}

// -- TestIrulesTaintQuickFixes -------------------------------------------

/// The `_fix` helper: open `source`, synthesise `diag`, request quickfix-only
/// code actions.
fn taint_fix(
    lsp: &mut Lsp,
    source: &str,
    code: &str,
    message: &str,
    start: (u32, u32),
    end: (u32, u32),
) -> Value {
    let uri = open_irule(lsp, source);
    let d = diag(code, message, start, end);
    code_actions_only(lsp, &uri, range(start, end), json!([d]), &["quickfix"])
}

#[test]
fn irule3001_wrap_html_encode() {
    let mut lsp = Lsp::irules();
    let actions = taint_fix(
        &mut lsp,
        "HTTP::respond 200 content $raw\n",
        "IRULE3001",
        "Tainted variable $raw in HTTP response body (HTTP::respond); risk of XSS",
        (0, 0),
        (0, 29),
    );
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("html_encode"))
        .cloned()
        .collect();
    assert_eq!(fixes.len(), 1);
    assert!(
        ca_new_texts(&json!(fixes))
            .iter()
            .any(|s| s.contains("[html_encode $raw]"))
    );
}

#[test]
fn irule3002_wrap_uri_encode() {
    let mut lsp = Lsp::irules();
    let actions = taint_fix(
        &mut lsp,
        "HTTP::header insert X-Fwd $raw\n",
        "IRULE3002",
        "Tainted variable $raw in HTTP header/cookie value (HTTP::header insert); risk of header injection",
        (0, 0),
        (0, 29),
    );
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("URI::encode"))
        .cloned()
        .collect();
    assert_eq!(fixes.len(), 1);
    assert!(
        ca_new_texts(&json!(fixes))
            .iter()
            .any(|s| s.contains("[URI::encode $raw]"))
    );
}

#[test]
fn t103_wrap_regex_quote() {
    let mut lsp = Lsp::irules();
    let actions = taint_fix(
        &mut lsp,
        "regexp $pat $data\n",
        "T103",
        "Tainted variable $pat in regexp pattern position (regexp); risk of regex injection",
        (0, 0),
        (0, 16),
    );
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("regex::quote"))
        .cloned()
        .collect();
    assert_eq!(fixes.len(), 1);
    assert!(
        ca_new_texts(&json!(fixes))
            .iter()
            .any(|s| s.contains("[regex::quote $pat]"))
    );
}

#[test]
fn t102_insert_double_dash() {
    let mut lsp = Lsp::irules();
    let actions = taint_fix(
        &mut lsp,
        "regexp $pat $data\n",
        "T102",
        "Tainted variable $pat in option position of 'regexp' without '--' terminator; risk of option injection",
        (0, 0),
        (0, 16),
    );
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("--"))
        .cloned()
        .collect();
    assert_eq!(fixes.len(), 1);
    assert!(
        ca_new_texts(&json!(fixes))
            .iter()
            .any(|s| s.contains("-- "))
    );
}

#[test]
fn t100_subst_add_nocommands() {
    let mut lsp = Lsp::irules();
    let actions = taint_fix(
        &mut lsp,
        "subst $tainted\n",
        "T100",
        "Tainted variable $tainted flows into subst; possible code injection",
        (0, 0),
        (0, 14),
    );
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("-nocommands"))
        .cloned()
        .collect();
    assert_eq!(fixes.len(), 1);
    assert!(
        ca_new_texts(&json!(fixes))
            .iter()
            .any(|s| s.contains("-nocommands"))
    );
}

#[test]
fn braced_variable_wrapped() {
    let mut lsp = Lsp::irules();
    let actions = taint_fix(
        &mut lsp,
        "HTTP::respond 200 content ${raw}\n",
        "IRULE3001",
        "Tainted variable $raw in HTTP response body (HTTP::respond); risk of XSS",
        (0, 0),
        (0, 30),
    );
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("html_encode"))
        .cloned()
        .collect();
    assert!(
        ca_new_texts(&json!(fixes))
            .iter()
            .any(|s| s.contains("[html_encode ${raw}]"))
    );
}

#[test]
fn t101_wrap_strip_crlf() {
    let mut lsp = Lsp::irules();
    let actions = taint_fix(
        &mut lsp,
        "puts $raw\n",
        "T101",
        "Tainted variable $raw flows into puts; output may contain injected content",
        (0, 0),
        (0, 8),
    );
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("strip CR/LF"))
        .cloned()
        .collect();
    assert_eq!(fixes.len(), 1, "{actions:?}");
    assert!(
        ca_new_texts(&json!(fixes))
            .iter()
            .any(|s| s.contains("[string map") && s.contains("$raw]")),
        "{fixes:?}"
    );
}

#[test]
fn irule3003_wrap_strip_crlf() {
    let mut lsp = Lsp::irules();
    let actions = taint_fix(
        &mut lsp,
        "log local0.info $raw\n",
        "IRULE3003",
        "Tainted variable $raw in log output (log); risk of log injection or log forging",
        (0, 0),
        (0, 16),
    );
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("strip CR/LF"))
        .cloned()
        .collect();
    assert_eq!(fixes.len(), 1, "{actions:?}");
    assert!(
        ca_new_texts(&json!(fixes))
            .iter()
            .any(|s| s.contains("[string map") && s.contains("$raw]")),
        "{fixes:?}"
    );
}

#[test]
fn no_fix_for_unknown_code() {
    let mut lsp = Lsp::irules();
    let actions = taint_fix(
        &mut lsp,
        "eval $raw\n",
        "T100",
        "Tainted variable $raw flows into eval; possible code injection",
        (0, 0),
        (0, 8),
    );
    let matched: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| {
            [
                "HTML::encode",
                "URI::encode",
                "regex::quote",
                "--",
                "strip CR/LF",
            ]
            .iter()
            .any(|k| action_title(a).contains(k))
        })
        .cloned()
        .collect();
    assert!(matched.is_empty(), "{matched:?}");
}

#[test]
fn t106_remove_redundant_encode() {
    let mut lsp = Lsp::irules();
    let actions = taint_fix(
        &mut lsp,
        "set double [HTML::encode $safe]\n",
        "T106",
        "Variable $safe is already HTML-escaped; passing through HTML::encode double-encodes the value",
        (0, 0),
        (0, 30),
    );
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("Remove redundant"))
        .cloned()
        .collect();
    assert_eq!(fixes.len(), 1);
    assert!(ca_new_texts(&json!(fixes)).iter().any(|s| s == "$safe"));
}

#[test]
fn irule3004_no_autofix() {
    let mut lsp = Lsp::irules();
    let actions = taint_fix(
        &mut lsp,
        "HTTP::redirect $url\n",
        "IRULE3004",
        "Tainted variable $url in redirect URL (HTTP::redirect); risk of open redirect",
        (0, 0),
        (0, 18),
    );
    let matched: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| {
            ["html_encode", "URI::encode", "regex::quote"]
                .iter()
                .any(|k| action_title(a).contains(k))
        })
        .cloned()
        .collect();
    assert!(matched.is_empty(), "{matched:?}");
}

// -- TestIrulesTaintProcInsertion ----------------------------------------

#[test]
fn t103_inserts_regex_quote_proc() {
    let mut lsp = Lsp::irules();
    let actions = taint_fix(
        &mut lsp,
        "regexp $pat $data\n",
        "T103",
        "Tainted variable $pat in regexp pattern position (regexp); risk of regex injection",
        (0, 0),
        (0, 16),
    );
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("regex::quote"))
        .cloned()
        .collect();
    let snippets = ca_new_texts(&json!(fixes));
    assert!(snippets.iter().any(|s| s.contains("[regex::quote $pat]")));
    assert!(snippets.iter().any(|s| s.contains("proc regex::quote")));
    assert!(snippets.iter().any(|s| s.contains("regsub")));
}

#[test]
fn irule3001_inserts_html_encode_proc() {
    let mut lsp = Lsp::irules();
    let actions = taint_fix(
        &mut lsp,
        "HTTP::respond 200 content $raw\n",
        "IRULE3001",
        "Tainted variable $raw in HTTP response body (HTTP::respond); risk of XSS",
        (0, 0),
        (0, 29),
    );
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("html_encode"))
        .cloned()
        .collect();
    let snippets = ca_new_texts(&json!(fixes));
    assert!(snippets.iter().any(|s| s.contains("[html_encode $raw]")));
    assert!(snippets.iter().any(|s| s.contains("proc html_encode")));
    assert!(snippets.iter().any(|s| s.contains("string map")));
}

#[test]
fn irule3002_no_proc_insert() {
    let mut lsp = Lsp::irules();
    let actions = taint_fix(
        &mut lsp,
        "HTTP::header insert X-Fwd $raw\n",
        "IRULE3002",
        "Tainted variable $raw in HTTP header/cookie value (HTTP::header insert); risk of header injection",
        (0, 0),
        (0, 29),
    );
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("URI::encode"))
        .cloned()
        .collect();
    assert!(
        !ca_new_texts(&json!(fixes))
            .iter()
            .any(|s| s.contains("proc "))
    );
}

// -- TestIrulesProfilesHeader --------------------------------------------

/// The `_src` helper: open `source`, return its `source`-kind code actions
/// at position (0, 0).
fn profile_source_actions(lsp: &mut Lsp, source: &str) -> Vec<Value> {
    let uri = open_irule(lsp, source);
    source_actions(&lsp.code_actions(&uri, range((0, 0), (0, 0)), json!([])))
}

#[test]
fn http_event_generates_http_profile() {
    let mut lsp = Lsp::irules();
    let sa = profile_source_actions(&mut lsp, "when HTTP_REQUEST {\n    HTTP::respond 200\n}\n");
    assert_eq!(sa.len(), 1);
    assert_eq!(ca_new_texts(&json!(sa))[0], "# Profiles: HTTP\n");
}

#[test]
fn existing_matching_directive_no_action() {
    let mut lsp = Lsp::irules();
    let sa = profile_source_actions(
        &mut lsp,
        "# Profiles: HTTP\nwhen HTTP_REQUEST {\n    HTTP::respond 200\n}\n",
    );
    assert_eq!(sa.len(), 0);
}

#[test]
fn dns_event_generates_dns_profile() {
    let mut lsp = Lsp::irules();
    let sa = profile_source_actions(
        &mut lsp,
        "when DNS_REQUEST {\n    set q [DNS::question name]\n}\n",
    );
    assert_eq!(sa.len(), 1);
    assert_eq!(ca_new_texts(&json!(sa))[0], "# Profiles: DNS\n");
}

#[test]
fn multiple_events_combines_profiles() {
    let mut lsp = Lsp::irules();
    let sa = profile_source_actions(
        &mut lsp,
        "when HTTP_REQUEST {\n    HTTP::uri\n}\nwhen CLIENTSSL_HANDSHAKE {\n    log local0. \"done\"\n}\n",
    );
    assert_eq!(sa.len(), 1);
    assert_eq!(ca_new_texts(&json!(sa))[0], "# Profiles: CLIENTSSL, HTTP\n");
}

#[test]
fn existing_outdated_directive_offers_update() {
    let mut lsp = Lsp::irules();
    let sa = profile_source_actions(
        &mut lsp,
        "# Profiles: HTTP\nwhen HTTP_REQUEST {\n    SSL::extensions -type renegotiation\n}\n",
    );
    assert_eq!(sa.len(), 1);
    assert!(
        action_title(&sa[0]).contains("Update"),
        "{:?}",
        action_title(&sa[0])
    );
    assert!(ca_new_texts(&json!(sa))[0].contains("CLIENTSSL"));
}

#[test]
fn rule_init_only_no_action() {
    let mut lsp = Lsp::irules();
    let sa = profile_source_actions(&mut lsp, "when RULE_INIT {\n    set ::counter 0\n}\n");
    assert_eq!(sa.len(), 0);
}

#[test]
fn irule1006_bootstrap_action_is_deduplicated() {
    let mut lsp = Lsp::irules();
    let uri = open_irule(
        &mut lsp,
        "when CLIENT_DATA {\n    set a [TCP::payload]\n    set b [TCP::payload]\n}\n",
    );
    let first = diag(
        "IRULE1006",
        "'TCP::payload' without a TCP::collect call.",
        (1, 11),
        (1, 23),
    );
    let second = diag(
        "IRULE1006",
        "'TCP::payload' without a TCP::collect call.",
        (2, 11),
        (2, 23),
    );
    let actions = lsp.code_actions(&uri, range((1, 11), (1, 23)), json!([first, second]));
    let collect: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("TCP::collect"))
        .cloned()
        .collect();
    assert_eq!(collect.len(), 1);
}

// -- TestIrulesTaintProcAlreadyDefined -----------------------------------

#[test]
fn t103_no_proc_insert_when_already_defined() {
    let mut lsp = Lsp::irules();
    let source = "proc regex::quote {str} { regsub -all {[][{}()*+?.\\\\^$|]} $str {\\\\&} }\n\
                  regexp $pat $data\n";
    let actions = taint_fix(
        &mut lsp,
        source,
        "T103",
        "Tainted variable $pat in regexp pattern position (regexp); risk of regex injection",
        (1, 0),
        (1, 16),
    );
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("regex::quote"))
        .cloned()
        .collect();
    let snippets = ca_new_texts(&json!(fixes));
    assert!(snippets.iter().any(|s| s.contains("[regex::quote $pat]")));
    assert!(!snippets.iter().any(|s| s.contains("proc regex::quote")));
}

#[test]
fn irule3001_no_proc_insert_when_already_defined() {
    let mut lsp = Lsp::irules();
    let source = "proc html_encode {str} { string map {& &amp; < &lt;} $str }\n\
                  HTTP::respond 200 content $raw\n";
    let actions = taint_fix(
        &mut lsp,
        source,
        "IRULE3001",
        "Tainted variable $raw in HTTP response body (HTTP::respond); risk of XSS",
        (1, 0),
        (1, 29),
    );
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("html_encode"))
        .cloned()
        .collect();
    let snippets = ca_new_texts(&json!(fixes));
    assert!(snippets.iter().any(|s| s.contains("[html_encode $raw]")));
    assert!(!snippets.iter().any(|s| s.contains("proc html_encode")));
}

// -- TestIrulesProfilesHeaderExtended ------------------------------------

#[test]
fn http_event_plus_ssl_command() {
    let mut lsp = Lsp::irules();
    let sa = profile_source_actions(
        &mut lsp,
        "when HTTP_REQUEST {\n    SSL::extensions -type renegotiation\n}\n",
    );
    assert_eq!(sa.len(), 1);
    let text = &ca_new_texts(&json!(sa))[0];
    assert!(text.contains("CLIENTSSL"), "{text:?}");
    assert!(text.contains("HTTP"), "{text:?}");
}

// Analyser-carried iRules quick fixes, end-to-end: open the iRule, feed the
// published diagnostic back into `codeAction`, and pin the fix by title plus
// the *applied* edit (never the message text). The Tcl-dialect analyser
// fixes (W120/W123/W001/W213/W216/W217) run in `code_actions.rs`; these are
// the `f5-irules`-only codes.

/// Diagnostics from `diags` carrying `code` (mirrors `code_actions.rs`).
fn with_code(diags: &[Value], code: &str) -> Vec<Value> {
    diags
        .iter()
        .filter(|d| d.get("code").and_then(Value::as_str) == Some(code))
        .cloned()
        .collect()
}

/// Open `source` as an iRule, then request quickfix-only actions over the
/// first `code` diagnostic's own range with that diagnostic as context —
/// panics (showing the full diagnostic set) when the diagnostic is absent.
fn irule_quickfixes_for_code(lsp: &mut Lsp, source: &str, code: &str) -> Value {
    let uri = unique_uri("irule");
    let diags = lsp.open_ready_lang(&uri, source, "tcl-irule");
    let matching = with_code(&diags, code);
    assert!(
        !matching.is_empty(),
        "expected a {code} diagnostic to drive the quick fix, got {diags:?}"
    );
    let rng = matching[0]["range"].clone();
    code_actions_only(lsp, &uri, rng, json!(matching), &["quickfix"])
}

/// IRULE2001: the deprecated 3-arg `matchclass` carries a whole-command
/// rewrite to `class match`, preserving item / operator / class verbatim
/// (including the `[HTTP::uri]` substitution and its closing bracket).
#[test]
fn irule2001_matchclass_offers_class_match_rewrite() {
    let mut lsp = Lsp::irules();
    let src = "when HTTP_REQUEST {\n    matchclass [HTTP::uri] equals my_dg\n}\n";
    let actions = irule_quickfixes_for_code(&mut lsp, src, "IRULE2001");
    assert_fix_applies(
        &actions,
        src,
        "Replace with 'class match'",
        "when HTTP_REQUEST {\n    class match [HTTP::uri] equals my_dg\n}\n",
    );
}

/// IRULE2001: the 2-arg shorthand expands with the default operator, so the
/// rewrite inserts `equals` between the preserved item and class.
#[test]
fn irule2001_two_arg_matchclass_rewrite_inserts_equals() {
    let mut lsp = Lsp::irules();
    let src = "when HTTP_REQUEST {\n    matchclass [HTTP::uri] my_dg\n}\n";
    let actions = irule_quickfixes_for_code(&mut lsp, src, "IRULE2001");
    assert_fix_applies(
        &actions,
        src,
        "Replace with 'class match'",
        "when HTTP_REQUEST {\n    class match [HTTP::uri] equals my_dg\n}\n",
    );
}

/// IRULE2001 guard: an ambiguous-arity `matchclass` still warns but offers
/// no rewrite — forcing one would corrupt the command.
#[test]
fn irule2001_ambiguous_arity_offers_no_rewrite() {
    let mut lsp = Lsp::irules();
    let src = "when HTTP_REQUEST {\n    matchclass [HTTP::uri]\n}\n";
    let actions = irule_quickfixes_for_code(&mut lsp, src, "IRULE2001");
    assert!(
        !actions
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .any(|a| action_title(a) == "Replace with 'class match'"),
        "no rewrite for ambiguous arity: {actions:?}"
    );
}

/// IRULE5005: a user proc invoked directly inside an event body carries a
/// fix that rewrites the head to the required `call PROC` form, leaving the
/// arguments in place.
#[test]
fn irule5005_direct_proc_call_offers_call_prefix_fix() {
    let mut lsp = Lsp::irules();
    let src = "proc helper {args} { return $args }\nwhen HTTP_REQUEST {\n    helper x y\n}\n";
    let actions = irule_quickfixes_for_code(&mut lsp, src, "IRULE5005");
    assert_fix_applies(
        &actions,
        src,
        "Use 'call helper'",
        "proc helper {args} { return $args }\nwhen HTTP_REQUEST {\n    call helper x y\n}\n",
    );
}

/// IRULE5005 FP guard: the `call`-prefixed invocation is the correct form —
/// no diagnostic and no fix.
#[test]
fn irule5005_call_prefixed_invocation_offers_no_fix() {
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready_lang(
        &uri,
        "proc helper {args} { return $args }\nwhen HTTP_REQUEST {\n    call helper x y\n}\n",
        "tcl-irule",
    );
    assert!(
        with_code(&diags, "IRULE5005").is_empty(),
        "`call helper` is the correct form: {diags:?}"
    );
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((2, 4), (2, 19)),
        json!([]),
        &["quickfix"],
    );
    assert!(
        !actions
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .any(|a| action_title(a).starts_with("Use 'call")),
        "{actions:?}"
    );
}

/// IRULE6001: a `::`-prefixed global write pins the virtual server to one
/// TMM; the fix replaces the variable word with its `static::` namespace
/// counterpart.
#[test]
fn irule6001_global_write_offers_static_namespace_replacement() {
    let mut lsp = Lsp::irules();
    let src = "when HTTP_REQUEST {\n    set ::counter 0\n}\n";
    let actions = irule_quickfixes_for_code(&mut lsp, src, "IRULE6001");
    assert_fix_applies(
        &actions,
        src,
        "Replace '::counter' with 'static::counter'",
        "when HTTP_REQUEST {\n    set static::counter 0\n}\n",
    );
}

/// IRULE6001, `RULE_INIT` implicit-global form: a bare `set` in `RULE_INIT`
/// is global (`RULE_INIT` runs at global namespace scope), and the fix
/// likewise rewrites the variable word into the `static::` namespace.
#[test]
fn irule6001_rule_init_implicit_global_offers_static_replacement() {
    let mut lsp = Lsp::irules();
    let src = "when RULE_INIT {\n    set greeting hi\n}\n";
    let actions = irule_quickfixes_for_code(&mut lsp, src, "IRULE6001");
    assert_fix_applies(
        &actions,
        src,
        "Replace 'greeting' with 'static::greeting'",
        "when RULE_INIT {\n    set static::greeting hi\n}\n",
    );
}

/// IRULE6001, `global NAME` form: the diagnostic fires but carries no
/// auto-fix — rewriting the import plus every subsequent use is not a
/// single-edit change, so the analyser attaches none by design.
#[test]
fn irule6001_global_import_form_offers_no_fix() {
    let mut lsp = Lsp::irules();
    let src = "when HTTP_REQUEST {\n    global counter\n    puts $counter\n}\n";
    let actions = irule_quickfixes_for_code(&mut lsp, src, "IRULE6001");
    assert!(
        !actions
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .any(|a| action_title(a).starts_with("Replace '")),
        "the global-import form carries no fix: {actions:?}"
    );
}

#[test]
fn existing_matching_directive_comma_format() {
    let mut lsp = Lsp::irules();
    let sa = profile_source_actions(
        &mut lsp,
        "# Profiles: CLIENTSSL, HTTP, SERVERSSL\nwhen HTTP_REQUEST {\n    SSL::extensions -type renegotiation\n}\n",
    );
    assert_eq!(sa.len(), 0);
}

#[test]
fn fasthttp_normalised_to_http() {
    let mut lsp = Lsp::irules();
    let sa = profile_source_actions(
        &mut lsp,
        "when HTTP_REQUEST {\n    set uri [HTTP::uri]\n}\n",
    );
    assert_eq!(sa.len(), 1);
    let text = &ca_new_texts(&json!(sa))[0];
    assert!(!text.contains("FASTHTTP"), "{text:?}");
    assert!(text.contains("HTTP"), "{text:?}");
}

#[test]
fn clientssl_event_generates_clientssl_profile() {
    let mut lsp = Lsp::irules();
    let sa = profile_source_actions(
        &mut lsp,
        "when CLIENTSSL_HANDSHAKE {\n    log local0. \"TLS done\"\n}\n",
    );
    assert_eq!(sa.len(), 1);
    assert_eq!(ca_new_texts(&json!(sa))[0], "# Profiles: CLIENTSSL\n");
}

#[test]
fn clientssl_clienthello_omits_persist_helper_profile() {
    let mut lsp = Lsp::irules();
    let sa = profile_source_actions(
        &mut lsp,
        "when CLIENTSSL_CLIENTHELLO {\n    log local0. \"TLS hello\"\n}\n",
    );
    assert_eq!(sa.len(), 1);
    let text = &ca_new_texts(&json!(sa))[0];
    assert_eq!(text, "# Profiles: CLIENTSSL\n");
    assert!(!text.contains("PERSIST"), "{text:?}");
}

// -- TestIrulesSemanticTokens --------------------------------------------

#[test]
fn comment_with_namespace_qualifiers_stays_one_comment() {
    let source = "# TCP::collect / TCP::payload / TCP::release\n";
    let mut lsp = Lsp::irules();
    let uri = open_irule(&mut lsp, source);
    let tokens = irules_typed(&mut lsp, &uri);
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].1, "comment");
    assert_eq!(
        tokens[0].0.length,
        i64::try_from(source.trim_end_matches('\n').len()).unwrap()
    );
}

#[test]
fn comment_header_block_all_comments() {
    let source = "# Flow:\n\
                  #   1. CLIENT_ACCEPTED / SERVER_CONNECTED -> TCP::collect\n\
                  #   2. CLIENT_DATA    / SERVER_DATA      -> TCP::payload ... TCP::release\n";
    let mut lsp = Lsp::irules();
    let uri = open_irule(&mut lsp, source);
    let tokens = irules_typed(&mut lsp, &uri);
    assert!(tokens.iter().all(|(_, ty)| ty == "comment"), "{tokens:?}");
}

// -- TestIrulesTaintDiagnostics ------------------------------------------
// Taint diagnostics must actually *fire* on the wire (positive + negative).

#[test]
fn taint_source_in_http_sink_fires_irule3102() {
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready_lang(
        &uri,
        "when HTTP_REQUEST {\n    HTTP::respond 200 content [HTTP::uri]\n}\n",
        "tcl-irule",
    );
    assert!(
        irules_codes(&diags).contains("IRULE3102"),
        "{:?}",
        irules_codes(&diags)
    );
}

#[test]
fn constant_in_http_sink_is_silent() {
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready_lang(
        &uri,
        "when HTTP_REQUEST {\n    HTTP::respond 200 content \"static body\"\n}\n",
        "tcl-irule",
    );
    assert!(
        !irules_codes(&diags).contains("IRULE3102"),
        "{:?}",
        irules_codes(&diags)
    );
}

// -- TestIrulesByteArrayCorruption ---------------------------------------
// S110 byte-array corruption on the wire (F5 KB K22406348).

/// A deep-tier iRules code that fires for every `*::payload` body here.
const DEEP_MARKER: &str = "IRULE1006";

/// Open `source` and poll the version-1 publish until the deep marker lands,
/// returning the final (basic + deep) diagnostics (`_deep_diags`).
fn deep_diags(lsp: &mut Lsp, source: &str) -> Vec<Value> {
    let uri = unique_uri("irule");
    lsp.open_ready_lang(&uri, source, "tcl-irule");
    let deadline = std::time::Instant::now() + scaled_timeout(Duration::from_secs(25));
    let mut diags = Vec::new();
    while std::time::Instant::now() < deadline {
        diags = lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(25));
        if diags.iter().any(|d| code_str(d) == DEEP_MARKER) {
            return diags;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!(
        "deep diagnostics ({DEEP_MARKER}) never published; saw {:?}",
        irules_codes(&diags)
    );
}

#[test]
fn payload_string_roundtrip_fires_s110() {
    let mut lsp = Lsp::irules();
    let diags = deep_diags(
        &mut lsp,
        "when HTTP_REQUEST_DATA {\n\
        \x20   set original_data [HTTP::payload]\n\
        \x20   set new_data \"$original_data MODIFIED\"\n\
        \x20   HTTP::payload replace 0 100 $new_data\n\
        }\n",
    );
    let s110: Vec<&Value> = diags.iter().filter(|d| code_str(d) == "S110").collect();
    assert!(!s110.is_empty(), "{:?}", irules_codes(&diags));
    assert_eq!(s110[0].get("severity").and_then(Value::as_i64), Some(2)); // Warning
}

#[test]
fn clean_payload_writeback_silent() {
    let mut lsp = Lsp::irules();
    let diags = deep_diags(
        &mut lsp,
        "when HTTP_REQUEST_DATA {\n\
        \x20   set original_data [HTTP::payload]\n\
        \x20   HTTP::payload replace 0 100 $original_data\n\
        }\n",
    );
    assert!(
        !irules_codes(&diags).contains("S110"),
        "{:?}",
        irules_codes(&diags)
    );
}

#[test]
fn payload_string_range_transparent_silent() {
    // `string range` keeps the byte-array representation (verified vs tclsh
    // 8.6/9.0), so slicing a payload and writing it back is byte-exact — the
    // canonical idiom must NOT fire S110.
    let mut lsp = Lsp::irules();
    let diags = deep_diags(
        &mut lsp,
        "when HTTP_REQUEST_DATA {\n\
        \x20   set original_data [HTTP::payload]\n\
        \x20   set slice [string range $original_data 0 99]\n\
        \x20   HTTP::payload replace 0 100 $slice\n\
        }\n",
    );
    assert!(
        !irules_codes(&diags).contains("S110"),
        "string range on a payload must not fire S110: {:?}",
        irules_codes(&diags)
    );
}

#[test]
fn binary_scan_fix_silent() {
    let mut lsp = Lsp::irules();
    let diags = deep_diags(
        &mut lsp,
        "when HTTP_REQUEST_DATA {\n\
        \x20   set original_data [HTTP::payload]\n\
        \x20   set new_data \"$original_data MODIFIED\"\n\
        \x20   binary scan $new_data c* -\n\
        \x20   HTTP::payload replace 0 100 $new_data\n\
        }\n",
    );
    assert!(
        !irules_codes(&diags).contains("S110"),
        "{:?}",
        irules_codes(&diags)
    );
}

#[test]
fn mqtt_payload_roundtrip_fires_s110() {
    // MQTT `replace <data>` puts the data operand at index 1, not 3 — the
    // registry-driven layout must still fire S110 here (PR #658 review gap).
    let mut lsp = Lsp::irules();
    let diags = deep_diags(
        &mut lsp,
        "when MQTT_MESSAGE {\n\
        \x20   set p [MQTT::payload]\n\
        \x20   set bad \"$p x\"\n\
        \x20   MQTT::payload replace $bad\n\
        }\n",
    );
    let s110: Vec<&Value> = diags.iter().filter(|d| code_str(d) == "S110").collect();
    assert!(!s110.is_empty(), "{:?}", irules_codes(&diags));
    assert_eq!(s110[0].get("severity").and_then(Value::as_i64), Some(2)); // Warning
}

#[test]
fn diameter_payload_roundtrip_fires_s110() {
    // DIAMETER `replace PAYLOAD` — data operand at index 1.
    let mut lsp = Lsp::irules();
    let diags = deep_diags(
        &mut lsp,
        "when DIAMETER_INGRESS {\n\
        \x20   set p [DIAMETER::payload]\n\
        \x20   set bad \"$p x\"\n\
        \x20   DIAMETER::payload replace $bad\n\
        }\n",
    );
    let s110: Vec<&Value> = diags.iter().filter(|d| code_str(d) == "S110").collect();
    assert!(!s110.is_empty(), "{:?}", irules_codes(&diags));
    assert_eq!(s110[0].get("severity").and_then(Value::as_i64), Some(2)); // Warning
}

// -- TestIrulesWhenBodyAnalysed ------------------------------------------
// Dialect-gated `when` body recursion (PR #640), iRules side.

#[test]
fn when_body_is_analysed_under_irules() {
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready_lang(
        &uri,
        "when HTTP_REQUEST {\n    set y $undefvar\n}\n",
        "tcl-irule",
    );
    // The W210 on the never-set `undefvar` read proves the braced body was
    // recursed into as a script (under plain Tcl it would be opaque data).
    assert!(
        irules_codes(&diags).contains("W210"),
        "{:?}",
        irules_codes(&diags)
    );
}

// -- TestIrulesWordOperatorFold ------------------------------------------
// Issue #1048: the dialect reaches the lowering, so a word-operator condition
// on a known-constant subject folds and draws I230 on the wire.

/// Open `source` as an iRule and poll the version-1 publish until `marker`
/// lands, returning the final diagnostics. Mirrors [`deep_diags`], but for a
/// caller-chosen marker.
fn diags_until(lsp: &mut Lsp, source: &str, marker: &str) -> Vec<Value> {
    let uri = unique_uri("irule");
    lsp.open_ready_lang(&uri, source, "tcl-irule");
    let deadline = std::time::Instant::now() + scaled_timeout(Duration::from_secs(25));
    let mut diags = Vec::new();
    while std::time::Instant::now() < deadline {
        diags = lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(25));
        if diags.iter().any(|d| code_str(d) == marker) {
            return diags;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("{marker} never published; saw {:?}", irules_codes(&diags));
}

/// `$x contains "cd"` with `$x` a known literal is a constant condition, so the
/// alternate branch is unreachable and I230 fires.
///
/// Before issue #1048 the lowering parsed every condition with no dialect, so
/// the word operator reached the IR as an opaque expression the fold could not
/// evaluate — I230 could not fire here even with the iRules dialect selected.
/// The plain-Tcl control (the same text must draw no I230, only W003) lives in
/// `tcl-compiler`'s `dialect_threading` suite: this server is iRules-dedicated,
/// so opening a plain Tcl document on it would switch its dialect.
#[test]
fn i230_fires_on_irules_word_operator_condition() {
    let mut lsp = Lsp::irules();
    let diags = diags_until(
        &mut lsp,
        "when HTTP_REQUEST {\n\
        \x20   set x \"abcdef\"\n\
        \x20   if {$x contains \"cd\"} { HTTP::respond 200 }\n\
        }\n",
        "I230",
    );
    let i230: Vec<&Value> = diags.iter().filter(|d| code_str(d) == "I230").collect();
    assert!(!i230.is_empty(), "{:?}", irules_codes(&diags));
    // The message names the condition it folded, not some other branch.
    let message = i230[0]
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        message.contains("contains"),
        "I230 must be reported for the word-operator condition; got {message:?}"
    );
}

/// FP guard: neither operand of `$x contains $y` is knowable here, so the
/// condition is not constant and I230 must stay silent — the fold is proven,
/// not assumed from the operator. `IRULE3102` (the unnormalised URI getter)
/// is the deep-diagnostic settle marker; a bare `when` uses BIG-IP's valid
/// default priority and does not manufacture IRULE1004.
#[test]
fn i230_silent_on_non_constant_irules_word_operator_condition() {
    let mut lsp = Lsp::irules();
    let diags = diags_until(
        &mut lsp,
        "when HTTP_REQUEST {\n\
        \x20   set y [HTTP::uri]\n\
        \x20   set x [HTTP::host]\n\
        \x20   if {$x contains $y} { HTTP::respond 200 }\n\
        }\n",
        "IRULE3102",
    );
    assert!(
        !irules_codes(&diags).contains("I230"),
        "{:?}",
        irules_codes(&diags)
    );
}
