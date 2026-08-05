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

//! Native port of `tests/lsp_e2e/test_config_e2e.py`.
//!
//! Effective-config + per-feature toggles, end-to-end against the packaged
//! server. Drives the `workspace/configuration` path: `getEffectiveConfig` must
//! expose a resolved `features` map (plus dialect, line length, optimiser
//! switch), a disabled provider must return empty/None, the optimiser master
//! switch must round-trip, and formatting must honour the request's indent width.
//!
//! Each Rust test owns its server, so pytest's `config_session` *cleanup* (undo
//! the change so the shared session server isn't polluted) is moot. But its
//! *behavioural* round-trip — disable a feature, then re-enable it and assert the
//! provider recovers — is a real contract (config re-pull works both
//! directions), so it is exercised explicitly here via a second
//! `apply_configuration_settle` back to the enabled state.

use crate::common::{Lsp, scaled_timeout, unique_uri};

use serde_json::{Value, json};

/// Feature toggles whose handler must return empty/None when disabled, keyed by
/// the camelCase `tclLsp.features.*` name.
const TOGGLEABLE_FEATURES: &[&str] = &[
    "hover",
    "completion",
    "documentSymbols",
    "definition",
    "references",
    "signatureHelp",
    "folding",
    "selectionRange",
    "documentLinks",
];

/// LSP "no result": null, an empty list, or an empty completion/`items` list.
fn is_empty(result: &Value) -> bool {
    match result {
        Value::Null => true,
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => {
            if let Some(items) = o.get("items").and_then(Value::as_array) {
                items.is_empty()
            } else {
                o.is_empty()
            }
        }
        _ => false,
    }
}

/// Whether `cfg.features.<feature>` is exactly `false`.
fn feature_disabled(cfg: &Value, feature: &str) -> bool {
    cfg.get("features").and_then(|f| f.get(feature)) == Some(&Value::Bool(false))
}

/// Whether `cfg.features.<feature>` is exactly `true`.
fn feature_enabled(cfg: &Value, feature: &str) -> bool {
    cfg.get("features").and_then(|f| f.get(feature)) == Some(&Value::Bool(true))
}

// -- TestEffectiveConfigShape --------------------------------------------

#[test]
fn reports_resolved_feature_map() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\n");
    let cfg = lsp.effective_config(&uri);
    assert!(cfg.is_object());
    let features = cfg.get("features").cloned().unwrap_or(Value::Null);
    assert!(features.is_object(), "{cfg}");
    let missing: Vec<&str> = TOGGLEABLE_FEATURES
        .iter()
        .copied()
        .filter(|k| features.get(*k).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "effective config missing feature keys {missing:?}: {features}"
    );
    for (_k, v) in features.as_object().unwrap() {
        assert!(v.is_boolean(), "{features}");
    }
}

#[test]
fn reports_dialect_and_scalars() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\n");
    let cfg = lsp.effective_config(&uri);
    let dialect = cfg.get("dialect").and_then(Value::as_str).unwrap_or("");
    assert!(!dialect.is_empty(), "{cfg}");
    assert!(cfg.get("line_length").is_some_and(Value::is_i64), "{cfg}");
    assert!(
        cfg.get("optimiser_enabled").is_some_and(Value::is_boolean),
        "{cfg}"
    );
}

// -- Per-provider disable contract — one case per toggleable feature. -----
//
// Each test opens a document that yields a non-empty result for its feature,
// asserts the enabled baseline is non-empty, disables just that feature (settling
// on the effective-config reflecting it), then asserts the provider is empty.

/// Open the document that yields a non-empty result for `feature`.
fn open_probe(lsp: &mut Lsp, uri: &str, feature: &str) {
    match feature {
        "hover" => lsp.open_ready(uri, "puts hello\n"),
        "completion" => lsp.open_ready(uri, "pu"),
        "documentSymbols" => lsp.open_ready(uri, "proc greet {} { return }\n"),
        "definition" | "references" => lsp.open_ready(
            uri,
            "proc greet {name} { puts \"Hello $name\" }\ngreet World\n",
        ),
        "signatureHelp" => {
            lsp.open_ready(uri, "proc greet {name greeting} { return }\ngreet World\n")
        }
        "folding" | "selectionRange" => {
            lsp.open_ready(uri, "proc greet {} {\n    set x 1\n    return $x\n}\n")
        }
        "documentLinks" => lsp.open_ready(uri, "source other.tcl\nputs done\n"),
        other => panic!("no probe for feature {other:?}"),
    };
}

/// Re-run the request for `feature` against `uri` (handlers read the toggle at
/// request time, so this is safe to call before and after disabling).
fn query_probe(lsp: &mut Lsp, uri: &str, feature: &str) -> Value {
    match feature {
        "hover" => lsp.hover(uri, 0, 2),
        "completion" => lsp.completion(uri, 0, 2),
        "documentSymbols" => lsp.document_symbols(uri),
        "definition" => lsp.definition(uri, 1, 2),
        "references" => lsp.references(uri, 0, 6, true),
        "signatureHelp" => lsp.signature_help(uri, 1, 12),
        "folding" => lsp.folding_range(uri),
        "selectionRange" => lsp.selection_range(uri, json!([{ "line": 2, "character": 11 }])),
        "documentLinks" => lsp.document_links(uri),
        other => panic!("no probe for feature {other:?}"),
    }
}

/// The body of the parametrized `test_disabling_feature_suppresses_its_provider`.
fn disabling_feature_suppresses_its_provider(feature: &str) {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    open_probe(&mut lsp, &uri, feature);

    // Baseline: the provider produces a non-empty result with defaults on.
    let baseline = query_probe(&mut lsp, &uri, feature);
    assert!(
        !is_empty(&baseline),
        "{feature}: expected a non-empty baseline, got {baseline}"
    );

    // Disable just this feature; settle on the effective-config reflecting it.
    let feat = feature.to_owned();
    lsp.apply_configuration_settle(json!({ "features": { feature: false } }), &uri, move |c| {
        feature_disabled(c, &feat)
    });

    let disabled = query_probe(&mut lsp, &uri, feature);
    assert!(
        is_empty(&disabled),
        "{feature}: provider must return empty/None when disabled, got {disabled}"
    );

    // Re-enable and settle; the provider must recover (the config re-pull works
    // both directions). Mirrors pytest's post-`config_session` "provider did not
    // recover after re-enable" assertion.
    let feat = feature.to_owned();
    lsp.apply_configuration_settle(json!({ "features": { feature: true } }), &uri, move |c| {
        feature_enabled(c, &feat)
    });
    let recovered = query_probe(&mut lsp, &uri, feature);
    assert!(
        !is_empty(&recovered),
        "{feature}: provider did not recover after re-enable, got {recovered}"
    );
}

macro_rules! disable_feature_test {
    ($name:ident, $feature:literal) => {
        #[test]
        fn $name() {
            disabling_feature_suppresses_its_provider($feature);
        }
    };
}

disable_feature_test!(disabling_hover_suppresses_provider, "hover");
disable_feature_test!(disabling_completion_suppresses_provider, "completion");
disable_feature_test!(
    disabling_document_symbols_suppresses_provider,
    "documentSymbols"
);
disable_feature_test!(disabling_definition_suppresses_provider, "definition");
disable_feature_test!(disabling_references_suppresses_provider, "references");
disable_feature_test!(
    disabling_signature_help_suppresses_provider,
    "signatureHelp"
);
disable_feature_test!(disabling_folding_suppresses_provider, "folding");
disable_feature_test!(
    disabling_selection_range_suppresses_provider,
    "selectionRange"
);
disable_feature_test!(
    disabling_document_links_suppresses_provider,
    "documentLinks"
);

// -- TestOptimiserToggle -------------------------------------------------

#[test]
fn optimiser_disable_round_trips() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // `llength` of a literal list folds to a constant — an O1xx optimiser offer
    // when the optimiser is on.
    lsp.open_ready(&uri, "puts [llength [list a b c]]\n");

    let on = lsp.effective_config(&uri);
    assert_eq!(
        on.get("optimiser_enabled"),
        Some(&Value::Bool(true)),
        "{on}"
    );
    let offers_on = lsp.execute_command("tcl-lsp.optimiseDocument", json!([uri, "full"]));
    let offers_source = offers_on
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(offers_source.contains("puts 3"), "{offers_on}");

    let off =
        lsp.apply_configuration_settle(json!({ "optimiser": { "enabled": false } }), &uri, |c| {
            c.get("optimiser_enabled") == Some(&Value::Bool(false))
        });
    assert_eq!(
        off.get("optimiser_enabled"),
        Some(&Value::Bool(false)),
        "{off}"
    );

    // Round-trip: re-enable and settle — the optimiser switch flips back on
    // (pytest's post-`config_session` "# Restored." assertion).
    let on_again =
        lsp.apply_configuration_settle(json!({ "optimiser": { "enabled": true } }), &uri, |c| {
            c.get("optimiser_enabled") == Some(&Value::Bool(true))
        });
    assert_eq!(
        on_again.get("optimiser_enabled"),
        Some(&Value::Bool(true)),
        "{on_again}"
    );
}

// -- TestFormattingIndentRoundTrip ---------------------------------------

const FMT_SRC: &str = "proc f {} {\nputs hi\n}\n";

/// The leading whitespace of the `puts` body line.
fn body_indent(formatted: &str) -> String {
    for line in formatted.lines() {
        if line.trim_start().starts_with("puts") {
            let trimmed = line.trim_start();
            return line[..line.len() - trimmed.len()].to_owned();
        }
    }
    panic!("no `puts` body line in {formatted:?}");
}

/// The `newText` of the first formatting edit.
fn first_edit_text(edits: &Value) -> String {
    edits
        .as_array()
        .and_then(|a| a.first())
        .and_then(|e| e.get("newText"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

#[test]
fn two_space_indent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, FMT_SRC);
    let edits = lsp.formatting(&uri, 2, true);
    assert!(
        edits.as_array().is_some_and(|a| !a.is_empty()),
        "expected formatting edits"
    );
    assert_eq!(body_indent(&first_edit_text(&edits)), "  ");
}

#[test]
fn four_space_indent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, FMT_SRC);
    let edits = lsp.formatting(&uri, 4, true);
    assert!(
        edits.as_array().is_some_and(|a| !a.is_empty()),
        "expected formatting edits"
    );
    assert_eq!(body_indent(&first_edit_text(&edits)), "    ");
}

// -- TestToggleNoStickyState ---------------------------------------------
//
// A feature flipped off and back on leaves no sticky state: across repeated
// disable→re-enable cycles the provider works, goes empty while off, and works
// again once re-enabled — never stuck off. (pytest ran this on a shared server
// via `config_session`; here one per-test server drives the cycle directly.)

#[test]
fn repeated_cycles_keep_provider_working() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts hello\n");

    for _ in 0..2 {
        // Enabled: hover works.
        assert!(!is_empty(&lsp.hover(&uri, 0, 2)));

        // Disable + settle: hover goes empty.
        lsp.apply_configuration_settle(json!({ "features": { "hover": false } }), &uri, |c| {
            feature_disabled(c, "hover")
        });
        assert!(is_empty(&lsp.hover(&uri, 0, 2)));

        // Re-enable + settle: hover works again — never stuck off.
        lsp.apply_configuration_settle(json!({ "features": { "hover": true } }), &uri, |c| {
            feature_enabled(c, "hover")
        });
        assert!(!is_empty(&lsp.hover(&uri, 0, 2)));
    }
}

// -- Per-code severity override (issue #941) ------------------------------
//
// `tclLsp.diagnosticSeverity.<CODE>` re-levels how prominently a diagnostic is
// published, without changing the analysis. LSP severity ints: 1=Error,
// 2=Warning, 3=Information, 4=Hint.

/// The published severity of the first `code` diagnostic in `diags`, if any.
fn severity_of(diags: &[Value], code: &str) -> Option<i64> {
    diags
        .iter()
        .find(|d| d.get("code").and_then(Value::as_str) == Some(code))
        .and_then(|d| d.get("severity"))
        .and_then(Value::as_i64)
}

#[test]
fn diagnostic_severity_override_relevels_a_diagnostic() {
    use std::time::{Duration, Instant};

    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // `tmp` is set but never read → W211, emitted at Hint (4) by default.
    lsp.open_document(&uri, "proc f {} { set tmp 5 }\n");
    let base = lsp.pull_diagnostics(&uri);
    assert_eq!(
        severity_of(&base, "W211"),
        Some(4),
        "W211 must default to Hint (4); got {base:?}"
    );

    // Raise W211 to a warning (2) via didChangeConfiguration; the analysis is
    // unchanged, only the published severity moves. Poll the deterministic pull
    // path until the re-pulled config takes effect.
    lsp.apply_configuration(json!({ "diagnosticSeverity": { "W211": "warning" } }));
    let deadline = Instant::now() + scaled_timeout(Duration::from_secs(10));
    let mut diags = lsp.pull_diagnostics(&uri);
    while severity_of(&diags, "W211") != Some(2) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
        diags = lsp.pull_diagnostics(&uri);
    }
    assert_eq!(
        severity_of(&diags, "W211"),
        Some(2),
        "override must re-level W211 to Warning (2); got {diags:?}"
    );

    // Reset to default (empty override) and confirm it returns to Hint.
    lsp.apply_configuration(json!({ "diagnosticSeverity": {} }));
    let deadline = Instant::now() + scaled_timeout(Duration::from_secs(10));
    let mut diags = lsp.pull_diagnostics(&uri);
    while severity_of(&diags, "W211") != Some(4) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
        diags = lsp.pull_diagnostics(&uri);
    }
    assert_eq!(
        severity_of(&diags, "W211"),
        Some(4),
        "clearing the override must restore Hint (4); got {diags:?}"
    );
}

#[test]
fn diagnostic_severity_override_is_per_code() {
    use std::time::{Duration, Instant};

    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // Two hint-severity lifecycle codes: W211 (`x` unused) and W220 (`y` dead
    // store, overwritten before read).
    let src = "proc f {} { set x 1\n set y 1\n set y 2\n return $y }\n";
    lsp.open_document(&uri, src);

    // Override only W211 → error (1); W220 must keep its emitted Hint (4).
    lsp.apply_configuration(json!({ "diagnosticSeverity": { "W211": "error" } }));
    let deadline = Instant::now() + scaled_timeout(Duration::from_secs(10));
    let mut diags = lsp.pull_diagnostics(&uri);
    while severity_of(&diags, "W211") != Some(1) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
        diags = lsp.pull_diagnostics(&uri);
    }
    assert_eq!(
        severity_of(&diags, "W211"),
        Some(1),
        "W211 must be overridden to Error (1); got {diags:?}"
    );
    assert_eq!(
        severity_of(&diags, "W220"),
        Some(4),
        "an un-overridden code (W220) must keep its emitted Hint (4); got {diags:?}"
    );
}

/// Regression (#407): a folder-scoped `tclLsp.dialect` must reach the server.
///
/// A multi-root editor answers the *unscoped* `workspace/configuration` pull
/// with the workspace-merged settings — folder-level values are invisible
/// there — and only the *scoped* pull carries a folder's own `tclLsp.dialect`.
/// The scoped reply used to be parsed into a `FolderConfig` that had no dialect
/// field at all, so the value was dropped on the floor and every document in
/// every root resolved to the session default. Mirrors the multi-root VS Code
/// suite (`multiFolderConfig.test.ts`) without an editor.
#[test]
fn folder_scoped_dialect_reaches_documents_in_that_folder() {
    use std::time::{Duration, Instant};

    let base = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-folder-dialect-{}-{}",
        std::process::id(),
        line!()
    ));
    let root_a = base.join("proj-a");
    let root_b = base.join("proj-b");
    std::fs::create_dir_all(&root_a).expect("mk proj-a");
    std::fs::create_dir_all(&root_b).expect("mk proj-b");
    // Deliberately signature-free source: nothing in it makes
    // `detect_dialect` (directive / shebang / `package require Tcl` / content
    // signatures) pick a dialect, so the folder override is what decides — the
    // exact case issue #407 is about.  `puts` is a core Tcl command and is
    // *not* in the iRules surface, so it is the cross-folder discriminator.
    let src = "proc greet {who} {\n    puts \"hi $who\"\n}\ngreet world\n";
    let file_a = root_a.join("foo.tcl");
    let file_b = root_b.join("foo.tcl");
    std::fs::write(&file_a, src).expect("write proj-a fixture");
    std::fs::write(&file_b, src).expect("write proj-b fixture");

    let mut lsp = Lsp::multi_root(
        // What the unscoped pull returns — the package.json default, naming
        // neither folder's dialect.
        json!({ "dialect": "tcl8.6" }),
        &[
            (root_a.as_path(), json!({ "dialect": "tcl8.4" })),
            (root_b.as_path(), json!({ "dialect": "f5-irules" })),
        ],
    );

    // The folder URIs themselves resolve through the folder chain: this is what
    // the VS Code suite polls to know the per-folder pull has settled.
    let uri_a = format!("file://{}", root_a.to_string_lossy());
    let uri_b = format!("file://{}", root_b.to_string_lossy());
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let a = lsp.effective_config(&uri_a);
        let b = lsp.effective_config(&uri_b);
        if a.get("dialect") == Some(&json!("tcl8.4"))
            && b.get("dialect") == Some(&json!("f5-irules"))
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "per-folder dialects never settled: proj-a={a}, proj-b={b}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }

    // …and so do documents inside them, which is what actually changes analysis.
    let doc_a = format!("file://{}", file_a.to_string_lossy());
    let doc_b = format!("file://{}", file_b.to_string_lossy());
    let diags_a = lsp.open_ready(&doc_a, src);
    let diags_b = lsp.open_ready(&doc_b, src);
    assert_eq!(
        lsp.effective_config(&doc_a).get("dialect"),
        Some(&json!("tcl8.4")),
        "proj-a/foo.tcl must resolve its folder's dialect"
    );
    assert_eq!(
        lsp.effective_config(&doc_b).get("dialect"),
        Some(&json!("f5-irules")),
        "proj-b/foo.tcl must resolve its folder's dialect"
    );
    let unknown_puts = |diags: &[Value]| {
        diags.iter().any(|d| {
            d.get("code").and_then(Value::as_str) == Some("W123")
                && d.get("message")
                    .and_then(Value::as_str)
                    .is_some_and(|m| m.contains("puts"))
        })
    };
    assert!(
        !unknown_puts(&diags_a),
        "tcl8.4 (proj-a) must know `puts`: {diags_a:?}"
    );
    assert!(
        unknown_puts(&diags_b),
        "f5-irules (proj-b) has no `puts` and must flag it: {diags_b:?}"
    );

    // The four fields the multi-root VS Code suite's `EffectiveConfig` shape
    // declares, and which nothing used to emit.
    let cfg = lsp.effective_config(&doc_a);
    assert_eq!(cfg.get("folder_uri"), Some(&json!(uri_a)));
    assert_eq!(cfg.get("dialect_explicitly_set"), Some(&json!(true)));
    assert!(
        cfg.get("extra_commands").is_some_and(Value::is_array),
        "extra_commands must always be an array: {cfg}"
    );
    let known: Vec<&str> = cfg
        .get("known_folder_uris")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    assert!(
        known.contains(&uri_a.as_str()) && known.contains(&uri_b.as_str()),
        "known_folder_uris must name both roots: {cfg}"
    );

    std::fs::remove_dir_all(&base).ok();
}

/// Regression: a **deleted-while-open** file must stop contributing to the
/// cross-document index, without losing the buffer.
///
/// An out-of-band rename (`mv` in a terminal, a branch switch) makes the editor
/// send `didChangeWatchedFiles` DELETED for the old path and CREATED for the
/// new one — but no `didClose`, because the buffer is still on screen. The
/// DELETED event used to be skipped outright for any URI with an open document,
/// so the dead path stayed in the workspace index forever: every proc appeared
/// twice in the symbol picker, and go-to-definition could land on a file that
/// no longer exists.
#[test]
fn watched_delete_retires_an_open_documents_index_entry() {
    use std::time::{Duration, Instant};

    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-deleted-while-open-{}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&root).expect("mk workspace root");
    let old_path = root.join("live.tcl");
    let new_path = root.join("live_moved.tcl");
    let src = "proc ghost_marker {} { return 1 }\n";
    std::fs::write(&old_path, src).expect("write fixture");

    let mut lsp = Lsp::at_workspace_root(&root);
    let old_uri = format!("file://{}", old_path.to_string_lossy());
    let new_uri = format!("file://{}", new_path.to_string_lossy());
    lsp.open_ready(&old_uri, src);

    let ghost_uris = |lsp: &mut Lsp| -> Vec<String> {
        let result = lsp.workspace_symbols("ghost_marker");
        result
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter(|s| s.get("name").and_then(Value::as_str) == Some("ghost_marker"))
            .filter_map(|s| {
                s.get("location")
                    .and_then(|l| l.get("uri"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .collect()
    };
    assert_eq!(
        ghost_uris(&mut lsp),
        vec![old_uri.clone()],
        "precondition: the open document is indexed under its own URI"
    );

    // Rename on disk; the buffer stays open at the old URI.
    std::fs::rename(&old_path, &new_path).expect("rename fixture");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [
            { "uri": old_uri, "type": 3 },
            { "uri": new_uri, "type": 1 },
        ]}),
    );

    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let uris = ghost_uris(&mut lsp);
        if uris == vec![new_uri.clone()] {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the deleted path must be retired and only the new one indexed; got {uris:?}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }

    // The buffer itself keeps working — only the workspace index dropped it.
    let symbols = lsp.document_symbols(&old_uri);
    let names = crate::common::helpers::symbol_names(&symbols);
    assert!(
        names.contains("ghost_marker"),
        "the still-open buffer must keep answering its own requests: {symbols:?}"
    );

    std::fs::remove_dir_all(&root).ok();
}
