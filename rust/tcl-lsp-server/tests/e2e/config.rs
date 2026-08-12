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

//! Effective-config + per-feature toggles, end-to-end against the packaged
//! server. Drives the `workspace/configuration` path: `getEffectiveConfig` must
//! expose a resolved `features` map (plus dialect, line length, optimiser
//! switch), a disabled provider must return empty/None, the optimiser master
//! switch must round-trip, and formatting must honour the request's indent width.
//!
//! Each test owns its server, so a config change leaves no shared state to
//! undo. The *behavioural* round-trip — disable a feature, then re-enable it and
//! assert the provider recovers — is a real contract (config re-pull works both
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

/// Wait until `predicate` holds over the live server, bounded by a load-scaled
/// deadline that **panics** naming `what`.
///
/// The per-folder `workspace/configuration` pull is a round trip the server
/// makes on its own schedule, so a test that reads a folder-resolved value
/// straight after `initialize` is reading it before the answer exists. There is
/// no notification announcing the pull landed — `getEffectiveConfig` is the
/// observation — so this is a bounded settle, not a fixed sleep, and it is the
/// one place the shape is written.
fn settle(lsp: &mut Lsp, what: &str, predicate: impl Fn(&mut Lsp) -> bool) {
    use std::time::{Duration, Instant};
    let budget = scaled_timeout(Duration::from_secs(20));
    let deadline = Instant::now() + budget;
    loop {
        if predicate(lsp) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{what} never settled within {budget:?} (load-scaled)"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
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

    // Re-enable and settle; the provider must recover, proving the config
    // re-pull works in both directions.
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

    // Round-trip: re-enable and settle — the optimiser switch flips back on.
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
// again once re-enabled — never stuck off.

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
    settle(&mut lsp, "per-folder dialects", |lsp| {
        lsp.effective_config(&uri_a).get("dialect") == Some(&json!("tcl8.4"))
            && lsp.effective_config(&uri_b).get("dialect") == Some(&json!("f5-irules"))
    });

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

/// Issue #1295: `getEffectiveConfig`'s `features` map must resolve through the
/// same folder chain the provider gates use.
///
/// It used to report the process-global toggles while every gate consulted the
/// deepest containing folder first.  A client that polls this command before
/// asserting a toggle took effect — which is exactly what the VS Code suite's
/// `waitForFeatureToggle` barrier does — was therefore waiting on a different
/// fact from the one the provider would act on, and in a multi-root workspace
/// a folder-scoped toggle was invisible here entirely.
///
/// Both halves are asserted together, because the report is only worth anything
/// if it agrees with the behaviour: the folder that disables `selectionRange`
/// must both *say* so and *suppress* the provider, and the folder that does not
/// must keep both.
#[test]
fn a_folder_scoped_feature_toggle_is_reported_and_applied_for_that_folder() {
    let base = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-folder-feature-{}-{}",
        std::process::id(),
        line!()
    ));
    let root_off = base.join("proj-off");
    let root_on = base.join("proj-on");
    std::fs::create_dir_all(&root_off).expect("mk proj-off");
    std::fs::create_dir_all(&root_on).expect("mk proj-on");
    let src = "proc greet {} {\n    set x 1\n    return $x\n}\n";
    let file_off = root_off.join("foo.tcl");
    let file_on = root_on.join("foo.tcl");
    std::fs::write(&file_off, src).expect("write proj-off fixture");
    std::fs::write(&file_on, src).expect("write proj-on fixture");

    let mut lsp = Lsp::multi_root(
        // The unscoped pull leaves the feature at its default (on), so the only
        // thing that can turn it off for `proj-off` is that folder's override.
        json!({}),
        &[
            (
                root_off.as_path(),
                json!({ "features": { "selectionRange": false } }),
            ),
            (root_on.as_path(), json!({})),
        ],
    );

    let doc_off = format!("file://{}", file_off.to_string_lossy());
    let doc_on = format!("file://{}", file_on.to_string_lossy());
    lsp.open_ready(&doc_off, src);
    lsp.open_ready(&doc_on, src);

    let position = json!([{ "line": 2, "character": 11 }]);

    // The report. `proj-off` must say the feature is off *for a document in it*
    // — the global map says nothing about this folder and would report `true`.
    settle(&mut lsp, "per-folder selectionRange toggles", |lsp| {
        feature_disabled(&lsp.effective_config(&doc_off), "selectionRange")
            && feature_enabled(&lsp.effective_config(&doc_on), "selectionRange")
    });

    // The behaviour, which is the fact the report is a proxy for.
    let suppressed = lsp.selection_range(&doc_off, position.clone());
    assert!(
        is_empty(&suppressed),
        "proj-off disables selectionRange, so its provider must return empty: {suppressed}"
    );
    let served = lsp.selection_range(&doc_on, position);
    assert!(
        !is_empty(&served),
        "proj-on leaves selectionRange enabled, so its provider must answer: {served}"
    );

    std::fs::remove_dir_all(&base).ok();
}

/// Issue #1217: a session dialect override survives a `workspace/configuration`
/// pull, and is cleared only by the command that set it.
///
/// The chat commands pin a buffer to `f5-irules` and put it back afterwards.
/// Doing that with a `didChangeConfiguration` push meant the very next pull —
/// which the server makes on all sorts of unrelated events — re-applied the
/// user's configured `tclLsp.dialect` and silently reverted the pin.
#[test]
fn a_session_dialect_override_outlives_a_config_pull() {
    let uri = unique_uri("dialect-override");
    let mut lsp = Lsp::tcl();
    lsp.open_ready(&uri, "set x 1\n");
    lsp.apply_configuration_settle(json!({ "dialect": "tcl8.6" }), "", |cfg| {
        cfg["dialect"] == json!("tcl8.6")
    });

    let set = lsp.execute_command("tcl-lsp.setSessionDialectOverride", json!(["f5-irules"]));
    assert_eq!(set["success"], json!(true), "{set}");
    let cfg = lsp.effective_config("");
    assert_eq!(cfg["dialect"], json!("f5-irules"), "{cfg}");
    assert_eq!(cfg["session_dialect_override"], json!("f5-irules"), "{cfg}");

    // A full config pull carrying the user's configured dialect must not touch
    // the override.  Settled on a *second* key that the same pull carries, so
    // the barrier proves the pull landed — settling on the override itself
    // would pass before the pull even started.
    let cfg = lsp.apply_configuration_settle(
        json!({ "dialect": "tcl9.0", "formatting": { "lineLength": 97 } }),
        "",
        |cfg| cfg["line_length"] == json!(97),
    );
    assert_eq!(
        cfg["dialect"],
        json!("f5-irules"),
        "a config pull must not revert the override: {cfg}"
    );
    assert_eq!(cfg["session_dialect_override"], json!("f5-irules"), "{cfg}");

    // Clearing hands the session back to the configured value.
    let cleared = lsp.execute_command("tcl-lsp.setSessionDialectOverride", json!([]));
    assert_eq!(cleared["success"], json!(true), "{cleared}");
    let cfg = lsp.effective_config("");
    assert_eq!(cfg["dialect"], json!("tcl9.0"), "{cfg}");
    assert_eq!(cfg["session_dialect_override"], Value::Null, "{cfg}");
}

/// Issue #1213: a burst of `didChangeConfiguration` notifications must produce
/// **one** `workspace/configuration` pull, not one per notification.
///
/// A settings-editor burst of 16 used to produce 32 pull batches (one unscoped
/// request plus one scoped batch per folder, each time) and re-analyse every
/// open buffer 16 times.  The last generation must still run to completion, so
/// the settings the final notification carried are the ones in effect.
#[test]
fn a_configuration_burst_pulls_once_and_settles_on_the_last_generation() {
    use std::time::{Duration, Instant};

    let uri = unique_uri("config-burst");
    let mut lsp = Lsp::tcl();
    lsp.open_ready(&uri, "set x 1\n");
    // Let start-up's own pull (and the workspace scan behind it) finish before
    // the burst, so only the burst's pulls are counted.
    lsp.effective_config(&uri);
    std::thread::sleep(Duration::from_millis(300));

    let pulls_before = lsp
        .server_requests()
        .iter()
        .filter(|r| r["method"] == "workspace/configuration")
        .count();

    // Sixteen notifications with no gap, exactly as the settings editor emits
    // them — one per changed key. The last one wins.
    for i in 0..16u32 {
        let line_length = 90 + i;
        lsp.set_config(json!({ "formatting": { "lineLength": line_length } }));
        lsp.notify(
            "workspace/didChangeConfiguration",
            json!({ "settings": {} }),
        );
    }

    // Settle on the last generation's value.
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let cfg = lsp.effective_config(&uri);
        if cfg["line_length"] == json!(105) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the final generation must run to completion; last config: {cfg}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    // Give a straggling generation a chance to show up before counting.
    std::thread::sleep(Duration::from_millis(500));

    let pulls = lsp
        .server_requests()
        .iter()
        .filter(|r| r["method"] == "workspace/configuration")
        .count()
        - pulls_before;
    assert!(
        pulls <= 2,
        "16 notifications must coalesce into a single pull \
         (one batch, plus at most one straggler generation); got {pulls}"
    );
}

/// Issue #1215: the `workspace/didChangeWatchedFiles` registration must match
/// every casing of every extension the server itself treats as Tcl.
///
/// VS Code matches watcher globs against the platform file system —
/// case-sensitively on Linux — and the registration carries no `ignoreCase`
/// option, so the single-cased `**/*.{tcl,…}` it used to send never fired for
/// an `UPPER.TCL`.  The glob now folds case per character.  Also asserts the
/// project-config watcher rides along, so the layered-settings live-reload does
/// not depend on each client registering its own watcher.
#[test]
fn watched_files_registration_folds_case_and_covers_project_config() {
    use std::time::Duration;

    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-watch-glob-{}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&root).expect("mk workspace root");
    let path = root.join("seed.tcl");
    std::fs::write(&path, "set x 1\n").expect("write fixture");
    let mut lsp = Lsp::at_workspace_root(&root);
    lsp.open_ready(&format!("file://{}", path.to_string_lossy()), "set x 1\n");

    let registration = lsp
        .try_await_server_request("client/registerCapability", Duration::from_secs(10), 0)
        .expect("the server registers workspace/didChangeWatchedFiles");
    let watchers: Vec<Value> = registration["params"]["registrations"]
        .as_array()
        .expect("registrations array")
        .iter()
        .filter(|r| r["method"] == "workspace/didChangeWatchedFiles")
        .flat_map(|r| {
            r["registerOptions"]["watchers"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .collect();
    let globs: Vec<&str> = watchers
        .iter()
        .filter_map(|w| w["globPattern"].as_str())
        .collect();
    assert!(
        globs.iter().any(|g| g.contains("[tT][cC][lL]")),
        "the source watcher glob must fold case per character: {globs:?}"
    );
    assert!(
        !globs.iter().any(|g| g.contains("{tcl,")),
        "the single-cased glob must be gone: {globs:?}"
    );
    assert!(
        globs.contains(&"**/.tcl-lsp.ini"),
        "the project config must be watched by the server itself: {globs:?}"
    );

    std::fs::remove_dir_all(&root).ok();
}

/// Issue #1215, the behavioural half: an external create / change / delete
/// cycle on an upper-cased `.TCL` must reach the cross-document index, exactly
/// as a lower-cased one does.
#[test]
fn watched_upper_case_extension_round_trips_through_the_index() {
    use std::time::{Duration, Instant};

    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-upper-ext-{}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&root).expect("mk workspace root");
    // An open buffer so the session has a document; the watched file itself is
    // never opened — the whole point is that only the watch event indexes it.
    let anchor = root.join("anchor.tcl");
    std::fs::write(&anchor, "set anchor 1\n").expect("write anchor");
    let mut lsp = Lsp::at_workspace_root(&root);
    lsp.open_ready(
        &format!("file://{}", anchor.to_string_lossy()),
        "set anchor 1\n",
    );

    let upper = root.join("UPPER.TCL");
    let upper_uri = format!("file://{}", upper.to_string_lossy());
    let marker_uris = |lsp: &mut Lsp| -> Vec<String> {
        lsp.workspace_symbols("upper_marker")
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter(|s| s.get("name").and_then(Value::as_str) == Some("upper_marker"))
            .filter_map(|s| {
                s.get("location")
                    .and_then(|l| l.get("uri"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .collect()
    };
    let settle = |lsp: &mut Lsp, want: &[String]| {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let got = marker_uris(lsp);
            if got == want {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "expected {want:?} for the UPPER.TCL marker, got {got:?}"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    };

    // CREATED.
    std::fs::write(&upper, "proc upper_marker {} { return 1 }\n").expect("write UPPER.TCL");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": upper_uri, "type": 1 }] }),
    );
    settle(&mut lsp, std::slice::from_ref(&upper_uri));

    // CHANGED — the definition moves out, so the index must drop it.
    std::fs::write(&upper, "proc other_marker {} { return 1 }\n").expect("rewrite UPPER.TCL");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": upper_uri, "type": 2 }] }),
    );
    settle(&mut lsp, &[]);

    // CREATED again, then DELETED.
    std::fs::write(&upper, "proc upper_marker {} { return 1 }\n").expect("rewrite UPPER.TCL");
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": upper_uri, "type": 2 }] }),
    );
    settle(&mut lsp, std::slice::from_ref(&upper_uri));
    std::fs::remove_file(&upper).ok();
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": upper_uri, "type": 3 }] }),
    );
    settle(&mut lsp, &[]);

    std::fs::remove_dir_all(&root).ok();
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
