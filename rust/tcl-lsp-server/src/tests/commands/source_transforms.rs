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

//! Minify, optimise and unminify.

use super::*;

#[tokio::test]
async fn minify_document_command_returns_source_and_handles_missing_doc() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///m.tcl").unwrap();
    register(&backend, &uri, "set x 1\nputs $x\n").await;
    let out = backend
        .minify_document_command(&[serde_json::json!(uri.as_str())])
        .await
        .expect("ok")
        .expect("some");
    assert!(
        out.get("source")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "minify result should carry a source string: {out:?}",
    );
    // No argument → None; an unregistered URI → None.
    assert!(
        backend
            .minify_document_command(&[])
            .await
            .expect("ok")
            .is_none()
    );
    assert!(
        backend
            .minify_document_command(&[serde_json::json!("file:///nope.tcl")])
            .await
            .expect("ok")
            .is_none(),
    );
}

#[tokio::test]
async fn optimise_document_command_returns_source_for_registered_doc() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///o.tcl").unwrap();
    register(&backend, &uri, "set x [expr {1 + 2}]\nputs $x\n").await;
    let out = backend
        .optimise_document_command(&[serde_json::json!(uri.as_str())])
        .await
        .expect("ok")
        .expect("some");
    assert!(
        out.get("source")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "optimise result should carry a source string: {out:?}",
    );
    assert!(
        backend
            .optimise_document_command(&[])
            .await
            .expect("ok")
            .is_none()
    );
}

/// Issue #2119: `tcl-lsp.optimiseDocument` is the batch form of the
/// optimiser code actions, so it must apply the same policy the published
/// O-code diagnostics do. Before the fix it read only its `profile`
/// argument and called the unfiltered optimiser, so each of these three
/// gates was ignored and the `optimiser_enabled` field doc — "when
/// `false`, the `tcl-lsp.optimiseDocument` command yields no rewrites" —
/// was simply untrue.
///
/// The policy is the folder's layers, so each backend is configured
/// through `apply_global_config` — the editor layer the report reads.
#[tokio::test]
async fn optimise_document_command_honours_the_optimiser_policy_issue_2119() {
    let src = "set x [expr {1 + 2}]\nputs $x\n";
    let uri = Uri::from_str("file:///o2119.tcl").unwrap();

    // The default editor profile is `Readability`, under which constant
    // folding and DCE are opt-in — so honouring the profile is itself
    // observable: before the fix this command folded regardless.
    let default_profile = test_backend();
    register(&default_profile, &uri, src).await;
    let out = default_profile
        .optimise_document_command(&[serde_json::json!(uri.as_str())])
        .await
        .expect("ok")
        .expect("some");
    assert_eq!(
        out.get("optimisations")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len),
        0,
        "the default Readability profile makes constant folding opt-in, so \
             the batch command must not fold either: {out:?}",
    );

    // Baseline for the remaining gates: a profile that does enable them.
    let backend = test_backend();
    register(&backend, &uri, src).await;
    backend
        .apply_global_config(&serde_json::json!({ "optimiser": { "profile": "aggressive" } }))
        .await
        .unwrap();
    let baseline = backend
        .optimise_document_command(&[serde_json::json!(uri.as_str())])
        .await
        .expect("ok")
        .expect("some");
    let count = |v: &serde_json::Value| {
        v.get("optimisations")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len)
    };
    assert!(
        count(&baseline) > 0,
        "expected a baseline rewrite: {baseline:?}"
    );
    let baseline_codes: Vec<String> = baseline["optimisations"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|o| o.get("code").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
        .collect();

    // 1. The master switch. Source must come back untouched.
    let off = test_backend();
    register(&off, &uri, src).await;
    off.apply_global_config(&serde_json::json!({
        "optimiser": { "profile": "aggressive", "enabled": false }
    }))
    .await
    .unwrap();
    let out = off
        .optimise_document_command(&[serde_json::json!(uri.as_str())])
        .await
        .expect("ok")
        .expect("some");
    assert_eq!(
        count(&out),
        0,
        "master switch off must yield no rewrites: {out:?}"
    );
    assert_eq!(
        out.get("source").and_then(serde_json::Value::as_str),
        Some(src),
        "master switch off must leave the source untouched: {out:?}",
    );

    // 2. The per-code disabled set.
    let per_code = test_backend();
    register(&per_code, &uri, src).await;
    let mut optimiser = serde_json::Map::new();
    optimiser.insert("profile".to_owned(), serde_json::json!("aggressive"));
    for code in &baseline_codes {
        optimiser.insert(code.clone(), serde_json::Value::Bool(false));
    }
    per_code
        .apply_global_config(&serde_json::json!({ "optimiser": optimiser }))
        .await
        .unwrap();
    let out = per_code
        .optimise_document_command(&[serde_json::json!(uri.as_str())])
        .await
        .expect("ok")
        .expect("some");
    assert_eq!(
        count(&out),
        0,
        "every baseline code was disabled per-code, so none may be applied: {out:?}",
    );
}

/// A group's edits apply all-or-nothing, and `optimiseDocument` *applies*
/// what it admits — so a line-keyed `# noqa` over one member of a pair
/// that straddles two lines must not leave the other member eligible.
///
/// O127's pair is an inline of the assignment at the use site plus a
/// delete of the original, on different lines. Admitting only the delete
/// removes the assignment without inlining it, which is the #2149
/// corruption in the apply path rather than the publish path. Codex
/// caught this on #2150 after the publish-path half was already fixed.
#[tokio::test]
async fn optimise_document_command_never_applies_half_a_group_issue_2149() {
    // `# noqa` sits above the *use* site, so it covers the inline member
    // and not the delete one line up.
    let src = "proc p {y} {\n    set x [llength $y]\n    # noqa\n    puts $x\n}\n";
    let backend = test_backend();
    let uri = Uri::from_str("file:///o2149a.tcl").unwrap();
    register(&backend, &uri, src).await;
    backend
        .apply_global_config(&serde_json::json!({ "optimiser": { "profile": "aggressive" } }))
        .await
        .unwrap();
    let out = backend
        .optimise_document_command(&[serde_json::json!(uri.as_str())])
        .await
        .expect("ok")
        .expect("some");
    let source = out
        .get("source")
        .and_then(serde_json::Value::as_str)
        .expect("a source");

    // Whatever else happens, the assignment must not vanish while the use
    // site still reads the variable.
    let assignment_gone = !source.contains("set x [llength $y]");
    let use_site_remains = source.contains("puts $x");
    assert!(
        !(assignment_gone && use_site_remains),
        "half the O127 group was applied — the store is gone but the read \
             remains, which is a broken program: {source:?}",
    );
    let applied: Vec<String> = out["optimisations"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|o| o.get("code").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
        .collect();
    assert!(
        !applied.iter().any(|c| c == "O127"),
        "the group lost a member to the directive, so none of it applies: \
             {applied:?} / {source:?}",
    );
}

/// Issue #2119, the part the unit tests first got wrong: the command's
/// own `profile` argument names an `OptimisationProfile` — the same
/// vocabulary `tcl opt --profile` parses — and it selects the *category*
/// set, overriding the configured editor profile.
///
/// The first cut of the #2119 fix took categories from
/// `resolved_analysis_settings` unconditionally, so
/// `optimiseDocument uri "full"` was answered with the default
/// `readability` profile, where constant folding is opt-in, and folded
/// nothing. Twelve e2e cases caught it; this pins it at unit level,
/// where the two meanings of "profile" are easy to conflate again.
/// `tclLsp.features.diagnostics` turns off the published squiggles, not
/// the rewrite a user asks for: `optimiseDocument` is `tcl opt`'s peer
/// and reads no whole-document gate.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn optimise_document_command_reads_no_diagnostics_feature_toggle() {
    let src = "puts [llength [list a b c]]\n";
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({ "features": { "diagnostics": false } }))
        .await
        .unwrap();
    let uri = Uri::from_str("file:///o-features-off.tcl").unwrap();
    register(&backend, &uri, src).await;
    let out = backend
        .optimise_document_command(&[serde_json::json!(uri.as_str()), serde_json::json!("full")])
        .await
        .expect("ok")
        .expect("some");
    assert!(
        out.get("source")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|s| s.contains("puts 3")),
        "the diagnostics toggle must not silence the command: {out:?}",
    );
}

#[tokio::test]
async fn optimise_document_command_profile_argument_selects_the_categories_issue_2119() {
    let src = "puts [llength [list a b c]]\n";
    let backend = test_backend();
    let uri = Uri::from_str("file:///o2119f.tcl").unwrap();
    register(&backend, &uri, src).await;
    // Deliberately left at the default `readability`, under which
    // constant folding is opt-in — the argument has to win.
    let out = backend
        .optimise_document_command(&[serde_json::json!(uri.as_str()), serde_json::json!("full")])
        .await
        .expect("ok")
        .expect("some");
    assert!(
        out.get("source")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|s| s.contains("puts 3")),
        "an explicit `full` profile must fold, whatever the editor's \
             configured profile is: {out:?}",
    );

    // An unrecognised name is not a licence to widen: it falls back to
    // the configured profile, which folds nothing by default.
    let fallback = test_backend();
    let uri2 = Uri::from_str("file:///o2119g.tcl").unwrap();
    register(&fallback, &uri2, src).await;
    let out = fallback
        .optimise_document_command(&[
            serde_json::json!(uri2.as_str()),
            serde_json::json!("not-a-profile"),
        ])
        .await
        .expect("ok")
        .expect("some");
    assert!(
        out.get("source")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|s| !s.contains("puts 3")),
        "an unrecognised profile falls back to the configured one: {out:?}",
    );
}

/// Issue #2119, the suppression-directive half: a `# noqa` must silence
/// the batch command exactly as it silences the published diagnostic.
/// Split from the switch/profile/per-code gates so neither test runs past
/// the pedantic line limit.
///
/// A differential rather than a bare count: the same source with and
/// without a `# noqa` directly above the rewritten statement. `set x …`
/// still folds in both, because the directive names one line and not the
/// file, so the assertion is about the rewrite the directive covers.
#[tokio::test]
async fn optimise_document_command_honours_noqa_issue_2119() {
    let codes_on = |v: &serde_json::Value| -> Vec<String> {
        v["optimisations"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|o| o.get("code").and_then(serde_json::Value::as_str))
            .map(str::to_owned)
            .collect()
    };
    let run = async |src: &str, name: &str| {
        let backend = test_backend();
        let uri = Uri::from_str(name).unwrap();
        register(&backend, &uri, src).await;
        backend
            .apply_global_config(&serde_json::json!({ "optimiser": { "profile": "aggressive" } }))
            .await
            .unwrap();
        backend
            .optimise_document_command(&[serde_json::json!(uri.as_str())])
            .await
            .expect("ok")
            .expect("some")
    };
    let plain = Box::pin(run("set x [expr {1 + 2}]\nputs $x\n", "file:///o2119p.tcl")).await;
    let noqa = Box::pin(run(
        "set x [expr {1 + 2}]\n# noqa\nputs $x\n",
        "file:///o2119n.tcl",
    ))
    .await;
    assert!(
        codes_on(&plain).contains(&"O100".to_owned()),
        "the read on `puts $x` should fold without a directive: {plain:?}",
    );
    assert!(
        !codes_on(&noqa).contains(&"O100".to_owned()),
        "a `# noqa` directly above `puts $x` must silence the batch command \
             exactly as it silences the published diagnostic: {noqa:?}",
    );
}

/// `optimiseDocument` over `src` under a session whose only layer is the
/// project file `project`, with `args` after the URI.
async fn optimise_under_project(
    src: &str,
    name: &str,
    project: serde_json::Value,
    args: &[serde_json::Value],
) -> serde_json::Value {
    let backend = test_backend();
    let uri = Uri::from_str(name).unwrap();
    register(&backend, &uri, src).await;
    *backend.policy_layers.lock().await = PolicyLayers {
        project,
        ..PolicyLayers::default()
    };
    let mut call = vec![serde_json::json!(uri.as_str())];
    call.extend_from_slice(args);
    backend
        .optimise_document_command(&call)
        .await
        .expect("ok")
        .expect("some")
}

/// The owner's ruling: the command's named profile is the profile in
/// force, and a project `[optimiser] profile` is only the default for a
/// call that names none (`docs/design/compiler/diagnostic-policy.md`
/// § Configuration).
#[tokio::test]
async fn an_invocation_profile_overrules_the_project_file() {
    let src = "set x [expr {1 + 2}]\n";
    let readability = serde_json::json!({ "optimiser": { "profile": "readability" } });
    let named = optimise_under_project(
        src,
        "file:///o2150n.tcl",
        readability.clone(),
        &[serde_json::json!("full")],
    )
    .await;
    assert_eq!(
        named.get("source").and_then(serde_json::Value::as_str),
        Some("set x 3\n"),
        "the argument's `full` folds over the project's `readability`: {named:?}",
    );
    let unnamed = optimise_under_project(src, "file:///o2150u.tcl", readability, &[]).await;
    assert_eq!(
        unnamed.get("source").and_then(serde_json::Value::as_str),
        Some(src),
        "with no argument the project's `readability` runs, and O101 is \
             outside it: {unnamed:?}",
    );
}

/// A named profile replaces the category set only: the project's
/// per-code override and its master switch still decide.
#[tokio::test]
async fn a_named_profile_leaves_the_switch_and_the_codes_to_the_layers() {
    let src = "set x [expr {1 + 2}]\n";
    for (project, name) in [
        (
            serde_json::json!({ "optimiser": { "O101": false } }),
            "file:///o2150o.tcl",
        ),
        (
            serde_json::json!({ "optimiser": { "enabled": false } }),
            "file:///o2150e.tcl",
        ),
    ] {
        let out =
            optimise_under_project(src, name, project.clone(), &[serde_json::json!("full")]).await;
        assert_eq!(
            out.get("source").and_then(serde_json::Value::as_str),
            Some(src),
            "{project} still keeps the fold off under `full`: {out:?}",
        );
    }
}

/// The pass count is the profile in force's: `aggressive` runs to a
/// fixpoint, so its second pass removes the store the first pass's
/// folds left unused (O126), where `full` stops after one pass — and a
/// named `full` decides it over the project's `aggressive`.
#[tokio::test]
async fn the_pass_count_follows_the_profile_in_force() {
    let src =
        "proc p {} {\n    set a [expr {1 + 2}]\n    set b [expr {$a * 2}]\n    return $b\n}\n";
    let aggressive = serde_json::json!({ "optimiser": { "profile": "aggressive" } });
    let codes = |out: &serde_json::Value| -> Vec<String> {
        out["optimisations"]
            .as_array()
            .expect("optimisations")
            .iter()
            .filter_map(|o| o.get("code").and_then(serde_json::Value::as_str))
            .map(str::to_owned)
            .collect()
    };
    let fixpoint = optimise_under_project(src, "file:///o2150a.tcl", aggressive.clone(), &[]).await;
    assert!(
        codes(&fixpoint).iter().any(|code| code == "O126"),
        "the project's `aggressive` runs a second pass: {fixpoint:?}",
    );
    let single = optimise_under_project(
        src,
        "file:///o2150f.tcl",
        aggressive,
        &[serde_json::json!("full")],
    )
    .await;
    assert!(
        !codes(&single).iter().any(|code| code == "O126"),
        "a named `full` is one pass: {single:?}",
    );
    assert!(
        codes(&single).iter().any(|code| code == "O100"),
        "and that pass still folds: {single:?}",
    );
}

#[test]
fn unminify_error_command_echoes_original_and_flags_change() {
    // No argument → None.
    assert!(Backend::unminify_error_command(&[]).is_none());
    let out = Backend::unminify_error_command(&[serde_json::json!("oops at a")]).expect("some");
    assert_eq!(
        out.get("originalError").and_then(serde_json::Value::as_str),
        Some("oops at a"),
    );
    assert!(
        out.get("translatedError")
            .and_then(serde_json::Value::as_str)
            .is_some()
    );
    assert!(
        out.get("changed")
            .and_then(serde_json::Value::as_bool)
            .is_some()
    );
}

#[tokio::test]
async fn minify_document_command_returns_minified_source() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///m.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new("# c\nputs   hi\n".to_owned(), "tcl8.6".to_owned()),
    );
    // Plain minify (compact=false, aggressive=false).
    let args = vec![
        serde_json::Value::String(uri.to_string()),
        serde_json::Value::Bool(false),
        serde_json::Value::Bool(false),
        serde_json::Value::Bool(false),
    ];
    let result = backend
        .minify_document_command(&args)
        .await
        .expect("ok")
        .expect("some");
    assert_eq!(result["source"], serde_json::json!("puts hi"));
    assert_eq!(result["minifiedLength"], serde_json::json!(7));
    assert!(result.get("symbolMap").is_none());
}

#[tokio::test]
async fn minify_document_command_compact_includes_symbol_map() {
    // Implementation contract: naming.minifier.original-syntax-correspondence
    // docs/design/analysis/name-resolution-proofs/minifier-original-syntax-correspondence.md
    let backend = test_backend();
    let uri = Uri::from_str("file:///m.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new(
            "proc greet {name} {\n    return $name\n}\n".to_owned(),
            "tcl8.6".to_owned(),
        ),
    );
    let args = vec![
        serde_json::Value::String(uri.to_string()),
        serde_json::Value::Bool(true), // compact
        serde_json::Value::Bool(false),
        serde_json::Value::Bool(false), // local formal contract only
    ];
    let result = backend
        .minify_document_command(&args)
        .await
        .expect("ok")
        .expect("some");
    assert_eq!(
        result["source"],
        serde_json::json!("proc greet {a} {return $a}")
    );
    assert!(
        result["symbolMap"]
            .as_str()
            .is_some_and(|s| s.contains("a <- name")),
        "{result:?}"
    );
    assert_eq!(result["refusals"], serde_json::json!([]));
    let mut isolated = args;
    isolated[3] = serde_json::Value::Bool(true);
    let result = backend
        .minify_document_command(&isolated)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        result["source"],
        serde_json::json!("proc greet {a} {return $a}")
    );
    assert!(
        result["refusals"]
            .as_array()
            .unwrap()
            .iter()
            .any(|refusal| { refusal["code"] == "missing-alpha-rename-contract" })
    );
}

#[test]
fn unminify_error_command_translates_names() {
    let args = vec![
        serde_json::Value::String("can't read \"a\": no such variable".to_owned()),
        serde_json::Value::String("# Procs\n  a <- greet".to_owned()),
    ];
    let result = Backend::unminify_error_command(&args).expect("some");
    assert_eq!(
        result["translatedError"],
        serde_json::json!("can't read \"greet\": no such variable")
    );
    assert_eq!(result["changed"], serde_json::json!(true));
}
