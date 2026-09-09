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
//! invariants are pinned here, keyed on the canonical BIG-IP basename:
//!
//!   * the document outline must never emit an empty symbol `name`
//!     (VS Code rejects the entire outline when any name is falsy);
//!   * the general Tcl analyser must never run on BIG-IP config text, so
//!     its encrypted-string markers (`$M$…$`) are not mis-read as Tcl
//!     variable references (W210) and no general Tcl diagnostics are
//!     published.
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

/// A representative BIG-IP config: nameless global singletons (which would
/// otherwise produce empty outline names) plus a pool, plus an embedded iRule whose body
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

// Every outline symbol carries a non-empty name.

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

/// A virtual that references a pool, which references a monitor — three
/// objects at three depths from the virtual.
const LINKED_CONF: &str = "\
ltm monitor http /Common/m1 {
    defaults-from http
}
ltm pool /Common/p1 {
    members {
        1.2.3.4:80 { }
    }
    monitor /Common/m1
}
ltm virtual /Common/vs1 {
    destination 10.0.0.1:80
    pool /Common/p1
}
";

#[test]
fn extract_linked_objects_walks_from_the_object_at_the_cursor() {
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, LINKED_CONF);
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));

    let offset = LINKED_CONF.find("ltm virtual").expect("fixture");
    let result = lsp.execute_command(
        "tcl-lsp.extractLinkedObjects",
        serde_json::json!([uri, offset, 5, 400, serde_json::Value::Null]),
    );
    assert!(!result.is_null(), "no result for the virtual");

    assert_eq!(
        result["rootHeader"],
        serde_json::Value::from("ltm virtual /Common/vs1"),
        "{result}"
    );

    // The pool the virtual names, and the monitor that pool names, both
    // reached — the walk follows references rather than stopping at one hop.
    let identifiers: BTreeSet<String> = result["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter_map(|n| n["identifier"].as_str().map(str::to_owned))
        .collect();
    assert!(identifiers.contains("/Common/vs1"), "{identifiers:?}");
    assert!(identifiers.contains("/Common/p1"), "{identifiers:?}");

    // Depth is distance from the root, so the root is 0 and what it names is
    // further out.
    let depth_of = |ident: &str| -> u64 {
        result["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .find(|n| n["identifier"].as_str() == Some(ident))
            .and_then(|n| n["depth"].as_u64())
            .expect("depth")
    };
    assert_eq!(depth_of("/Common/vs1"), 0, "{result}");
    assert!(depth_of("/Common/p1") > 0, "{result}");

    // Every edge names two nodes that are actually in the result, or the
    // client draws an arrow into nothing.
    let ids: BTreeSet<String> = result["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter_map(|n| n["id"].as_str().map(str::to_owned))
        .collect();
    for edge in result["edges"].as_array().expect("edges") {
        let (source, target) = (
            edge["source"].as_str().expect("source"),
            edge["target"].as_str().expect("target"),
        );
        assert!(ids.contains(source), "dangling edge source {source}");
        assert!(ids.contains(target), "dangling edge target {target}");
    }
}

#[test]
fn extract_linked_objects_stops_at_the_requested_depth() {
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, LINKED_CONF);
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));

    let offset = LINKED_CONF.find("ltm virtual").expect("fixture");
    let shallow = lsp.execute_command(
        "tcl-lsp.extractLinkedObjects",
        serde_json::json!([uri, offset, 0, 400, serde_json::Value::Null]),
    );
    let nodes = shallow["nodes"].as_array().expect("nodes");
    assert_eq!(
        nodes.len(),
        1,
        "depth 0 should be the root alone: {shallow}"
    );
    assert_eq!(
        nodes[0]["identifier"],
        serde_json::Value::from("/Common/vs1")
    );
}

#[test]
fn extract_linked_objects_is_null_away_from_every_object() {
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, LINKED_CONF);
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));

    // Past the end of the document, so no stanza contains it.
    let result = lsp.execute_command(
        "tcl-lsp.extractLinkedObjects",
        serde_json::json!([uri, LINKED_CONF.len() + 50, 5, 400, serde_json::Value::Null]),
    );
    assert!(result.is_null(), "{result}");
}

/// One pool a virtual uses and one nothing references, so a cleanup run has
/// something to propose and something it must not.
const CLEANUP_CONF: &str = "\
ltm pool /Common/used {
    members {
        1.2.3.4:80 { }
    }
}
ltm pool /Common/orphan {
    members {
        5.6.7.8:80 { }
    }
}
ltm virtual /Common/vs1 {
    destination 10.0.0.1:80
    pool /Common/used
}
";

#[test]
fn bigip_cleanup_proposes_only_the_unreferenced_object() {
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, CLEANUP_CONF);
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));

    let result = lsp.execute_command(
        "tcl-lsp.bigipCleanup",
        serde_json::json!([[uri], serde_json::Value::Null, false]),
    );
    assert!(!result.is_null(), "no cleanup report");

    let proposed: BTreeSet<String> = result["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .filter_map(|c| c["fullPath"].as_str().map(str::to_owned))
        .collect();
    assert!(proposed.contains("/Common/orphan"), "{proposed:?}");
    // The pool the virtual uses must never be proposed for deletion, and nor
    // must the virtual itself, which is a root.
    assert!(!proposed.contains("/Common/used"), "{proposed:?}");
    assert!(!proposed.contains("/Common/vs1"), "{proposed:?}");

    // The script is what the client opens for review, so every candidate has
    // to appear in it.
    let script = result["tmshScript"].as_str().expect("tmshScript");
    assert!(script.contains("/Common/orphan"), "{script}");
    assert!(!script.contains("/Common/used"), "{script}");
}

#[test]
fn bigip_cleanup_spares_a_kept_path() {
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, CLEANUP_CONF);
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));

    let result = lsp.execute_command(
        "tcl-lsp.bigipCleanup",
        serde_json::json!([[uri], ["/Common/orphan"], false]),
    );
    let proposed: BTreeSet<String> = result["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .filter_map(|c| c["fullPath"].as_str().map(str::to_owned))
        .collect();
    assert!(
        !proposed.contains("/Common/orphan"),
        "a kept path was still proposed: {proposed:?}"
    );
}

/// A partition and two objects inside it, so a rename has both the stanza
/// header and the path prefixes to carry.
const PARTITION_CONF: &str = "\
auth partition Tenant {
    description \"a tenant\"
}
ltm pool /Tenant/p1 {
    members {
        1.2.3.4:80 { }
    }
}
ltm virtual /Tenant/vs1 {
    destination 10.0.0.1:80
    pool /Tenant/p1
}
";

#[test]
fn rename_partition_rewrites_the_stanza_and_every_path() {
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, PARTITION_CONF);
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));

    let result = lsp.execute_command(
        "tcl-lsp.renamePartition",
        serde_json::json!([uri, "Tenant", "Renamed"]),
    );
    assert_eq!(result["success"], serde_json::Value::Bool(true), "{result}");

    let text = result["edit"]["changes"][&uri][0]["newText"]
        .as_str()
        .expect("edit text");
    // The header and both object paths move together — a rename that missed
    // any of them would leave the file referring to a partition that is gone.
    assert!(text.contains("auth partition Renamed"), "{text}");
    assert!(text.contains("/Renamed/p1"), "{text}");
    assert!(text.contains("/Renamed/vs1"), "{text}");
    assert!(!text.contains("/Tenant/"), "old paths survived:\n{text}");
}

#[test]
fn rename_partition_reports_why_it_refused() {
    let mut lsp = Lsp::bigip();
    let uri = bigip_uri();
    lsp.open_document(&uri, PARTITION_CONF);
    lsp.await_diagnostics_version(&uri, Some(1), Duration::from_secs(30));

    // A partition this file does not have.
    let absent = lsp.execute_command(
        "tcl-lsp.renamePartition",
        serde_json::json!([uri, "Missing", "Renamed"]),
    );
    assert_eq!(
        absent["success"],
        serde_json::Value::Bool(false),
        "{absent}"
    );
    assert!(
        absent["error"].as_str().is_some_and(|e| !e.is_empty()),
        "a refusal must say why: {absent}"
    );

    // A path rather than a bare name: the engine's own rule, surfaced as the
    // message the client shows rather than re-checked here.
    let path_form = lsp.execute_command(
        "tcl-lsp.renamePartition",
        serde_json::json!([uri, "/Tenant", "Renamed"]),
    );
    assert_eq!(
        path_form["success"],
        serde_json::Value::Bool(false),
        "{path_form}"
    );
    assert!(
        path_form["error"].as_str().is_some_and(|e| !e.is_empty()),
        "{path_form}"
    );
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

// No general Tcl diagnostics on BIG-IP config text.

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
