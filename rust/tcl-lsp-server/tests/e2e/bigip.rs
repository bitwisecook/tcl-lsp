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

//! F5 BIG-IP `*.conf` handling, end-to-end against the packaged server. Two
//! 1.11.0 fixes are pinned here, keyed on the canonical BIG-IP basename:
//!
//!   #534 — the document outline must never emit an empty symbol `name`
//!          (VS Code rejects the entire outline when any name is falsy);
//!   #571 — the general Tcl analyser must never run on BIG-IP config text, so
//!          its encrypted-string markers (`$M$…$`) are not mis-read as Tcl
//!          variable references (W210) and no general Tcl diagnostics are
//!          published.
//!
//! These use `Lsp::bigip()` and `open_document` (not `open_ready`): the BIG-IP
//! path doesn't emit the Tcl `workspace_state.update` marker, so we wait on the
//! published (BIG-IP) diagnostics instead. BIG-IP routing keys on the canonical
//! basename (`bigip.conf`), so `bigip_uri` fixes the basename while making the
//! directory unique per call.

use crate::common::Lsp;
use crate::common::helpers::*;

use serde_json::Value;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static BIGIP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A fresh unique `file://` URI whose basename is the canonical BIG-IP name.
/// The server routes BIG-IP handling on the basename, so the basename is fixed
/// (`bigip.conf`) while the directory is made unique per call.
fn bigip_uri() -> String {
    let n = BIGIP_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("file:///bigip/{}_{n}/bigip.conf", std::process::id())
}

/// A representative BIG-IP config: nameless global singletons (which previously
/// produced empty outline names) plus a pool, plus an embedded iRule whose body
/// holds an encrypted-string marker the Tcl analyser would mis-read as `$M`.
const BIGIP_CONF: &str = "\
auth password-policy {
    lockout-duration 10
}
net self-allow {
    defaults {
        tcp:443
    }
}
sys diags ihealth {
    user admin
}
ltm pool /Common/p1 {
    members {
        1.2.3.4:80 { }
    }
}
ltm rule /Common/r {
    when HTTP_REQUEST {
        log local0. \"$M$mn$9uYx+bTjLD8YSGZA=\"
    }
}
";

/// The set of `code` strings on a diagnostics array.
fn codes(diags: &[Value]) -> BTreeSet<String> {
    diags
        .iter()
        .map(|d| match d.get("code") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => "None".to_owned(),
        })
        .collect()
}

/// The symbol `name`s from a (possibly hierarchical) document-symbol result.
fn symbol_name_list(result: &Value) -> Vec<Option<String>> {
    flatten_symbols(result)
        .iter()
        .map(|s| s.get("name").and_then(Value::as_str).map(str::to_owned))
        .collect()
}

// -- TestBigipDocumentOutline --------------------------------------------
// Issue #534 — every outline symbol carries a non-empty name.

/// A configuration with two rules in different partitions, so a test can tell
/// "found the right one" from "found the first one".
const TWO_RULES_CONF: &str = "\
ltm pool /Common/p1 {
    members {
        1.2.3.4:80 { }
    }
}
ltm rule /Common/first {
    when HTTP_REQUEST {
        pool /Common/p1
    }
}
ltm rule /Tenant-A/second {
    when HTTP_RESPONSE {
        log local0. \"done\"
    }
}
";

#[test]
fn list_rules_returns_every_embedded_rule() {
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, TWO_RULES_CONF);
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));

    let result = lsp.execute_command("tcl-lsp.listRules", serde_json::json!([uri]));
    let rules = result.as_array().expect("rule array");
    let paths: Vec<&str> = rules
        .iter()
        .filter_map(|r| r["fullPath"].as_str())
        .collect();
    assert_eq!(paths, ["/Common/first", "/Tenant-A/second"], "{result}");

    // The client opens a scratch buffer from `body` and writes it back with
    // the offsets, so both have to describe the same span.
    for rule in rules {
        let start =
            usize::try_from(rule["bodyStartOffset"].as_u64().expect("start")).expect("offset fits");
        let end =
            usize::try_from(rule["bodyEndOffset"].as_u64().expect("end")).expect("offset fits");
        assert_eq!(
            &TWO_RULES_CONF[start..end],
            rule["body"].as_str().expect("body"),
            "body and offsets disagree: {rule}"
        );
        assert_eq!(rule["uri"], serde_json::Value::from(uri.clone()));
    }
}

#[test]
fn list_rules_is_an_empty_list_when_the_config_has_none() {
    // Empty is not the same answer as null: the clients say "no rules here"
    // for one and warn about an unreadable document for the other.
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, "ltm pool /Common/p1 {\n    members { }\n}\n");
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));

    let result = lsp.execute_command("tcl-lsp.listRules", serde_json::json!([uri]));
    assert_eq!(result, serde_json::json!([]), "{result}");
}

#[test]
fn extract_rule_finds_the_rule_containing_the_offset() {
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, TWO_RULES_CONF);
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));

    // An offset inside the second rule's body, not the first.
    let offset = TWO_RULES_CONF.find("HTTP_RESPONSE").expect("fixture");
    let result = lsp.execute_command("tcl-lsp.extractRule", serde_json::json!([uri, offset]));
    assert_eq!(
        result["fullPath"],
        serde_json::Value::from("/Tenant-A/second"),
        "{result}"
    );
    assert_eq!(
        result["name"],
        serde_json::Value::from("second"),
        "{result}"
    );

    // An offset outside every rule is null, which the client renders as
    // "cursor is not inside an ltm rule or gtm rule block".
    let outside = TWO_RULES_CONF.find("ltm pool").expect("fixture");
    let miss = lsp.execute_command("tcl-lsp.extractRule", serde_json::json!([uri, outside]));
    assert!(miss.is_null(), "{miss}");
}

#[test]
fn write_rule_back_refuses_a_span_the_document_cannot_honour() {
    // Writing a body into the wrong range corrupts the configuration, so an
    // impossible span is refused rather than clamped. The client branches on
    // this boolean to warn instead of reporting a successful save.
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, TWO_RULES_CONF);
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));

    let past_end = TWO_RULES_CONF.len() + 100;
    let result = lsp.execute_command(
        "tcl-lsp.writeRuleBack",
        serde_json::json!([uri, past_end, past_end + 10, "body"]),
    );
    assert_eq!(result, serde_json::json!(false), "{result}");

    let inverted = lsp.execute_command(
        "tcl-lsp.writeRuleBack",
        serde_json::json!([uri, 50, 10, "body"]),
    );
    assert_eq!(inverted, serde_json::json!(false), "{inverted}");
}

#[test]
fn outline_symbols_all_have_non_empty_names() {
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, BIGIP_CONF);
    // The BIG-IP path doesn't emit the Tcl `workspace_state.update` marker, so
    // wait on the published (BIG-IP) diagnostics instead of open_ready.
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));
    let syms = lsp.document_symbols(&uri);
    let names = symbol_name_list(&syms);
    assert!(!names.is_empty(), "expected a non-empty BIG-IP outline");
    assert!(
        names
            .iter()
            .all(|n| n.as_deref().is_some_and(|s| !s.is_empty())),
        "empty symbol name(s) in outline: {names:?}"
    );
}

#[test]
fn nameless_singleton_falls_back_to_kind_label() {
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, BIGIP_CONF);
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));
    let syms = lsp.document_symbols(&uri);
    let names: BTreeSet<String> = symbol_name_list(&syms).into_iter().flatten().collect();
    // The nameless `auth password-policy` singleton surfaces by its module /
    // kind labels rather than an empty string.
    assert!(names.contains("auth"), "{names:?}");
}

// -- TestBigipFolding ----------------------------------------------------
// A `.conf` is not Tcl, so the Tcl brace walk found only comment blocks in it
// and left every stanza unfoldable; folding now runs off the stanza tree.

/// The `(startLine, endLine)` pairs of a folding-range result.
fn fold_spans(result: &Value) -> BTreeSet<(i64, i64)> {
    result
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|r| {
                    (
                        r.get("startLine").and_then(Value::as_i64).unwrap_or(-1),
                        r.get("endLine").and_then(Value::as_i64).unwrap_or(-1),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn conf_stanzas_and_their_nested_blocks_fold() {
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, BIGIP_CONF);
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));
    let spans = fold_spans(&lsp.folding_range(&uri));
    // `net self-allow` (3..7) and the `defaults` block it wraps (4..6).
    assert!(spans.contains(&(3, 6)), "self-allow stanza: {spans:?}");
    assert!(spans.contains(&(4, 5)), "defaults block: {spans:?}");
    // `ltm pool` (11..15) and its `members` block (12..14).
    assert!(spans.contains(&(11, 14)), "pool stanza: {spans:?}");
    assert!(spans.contains(&(12, 13)), "members block: {spans:?}");
}

#[test]
fn tcl_inside_an_embedded_rule_folds() {
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, BIGIP_CONF);
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));
    let spans = fold_spans(&lsp.folding_range(&uri));
    // `ltm rule /Common/r` (16..20) and the `when HTTP_REQUEST` block in it.
    assert!(spans.contains(&(16, 19)), "rule stanza: {spans:?}");
    assert!(spans.contains(&(17, 18)), "when block: {spans:?}");
}

// -- TestBigipDiagnosticSuppression --------------------------------------
// Issue #571 — no general Tcl diagnostics on BIG-IP config text.

#[test]
fn encrypted_marker_does_not_raise_tcl_diagnostics() {
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, BIGIP_CONF);
    let diags = lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));
    let cs = codes(&diags);
    // `$M$…$` would be a W210 (read-before-set) if analysed as Tcl; the whole
    // W/E/S general-Tcl family must be absent on a BIG-IP conf.
    assert!(!cs.contains("W210"), "{cs:?}");
    assert!(
        !cs.iter()
            .any(|c| c.starts_with('W') || c.starts_with('E') || c.starts_with('S')),
        "general Tcl diagnostics leaked on BIG-IP conf: {cs:?}"
    );
}

#[test]
fn bare_set_arity_not_flagged_on_bigip_conf() {
    // A line that, as Tcl, is a bare `set` (E002 arity) — but here it is BIG-IP
    // config text, so the Tcl arity checker must not fire.
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, "ltm pool /Common/p { }\nset\n");
    let diags = lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));
    assert!(!codes(&diags).contains("E002"), "{:?}", codes(&diags));
}
