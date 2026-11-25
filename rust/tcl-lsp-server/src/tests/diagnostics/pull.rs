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

//! Pull diagnostics and the reports shared by push and pull.

use super::*;

/// Push publication and the full pull report preserve the same conditional
/// held-target/capture/baked/suffix source signature subject and head span.
#[tokio::test]
async fn original_callback_push_and_pull_share_source_signature_advice() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    let backend = test_backend();
    let caller = Uri::from_str("file:///callback-main.tcl").unwrap();
    let provider = Uri::from_str("file:///callback-provider.tcl").unwrap();
    let source = "interp alias {} cb {} ::external FIXED\nlsort -command {cb BAKED} {2 1}";
    register_callback_source(
        &backend,
        &provider,
        "proc external {a b c} {}\nproc cb {a b c d} {}",
    )
    .await
    .unwrap();
    register_callback_source(&backend, &caller, source)
        .await
        .unwrap();
    set_callback_project_advice(&backend, true).await;
    backend
        .publish_analyser_diagnostics(
            caller.clone(),
            source.to_owned(),
            "tcl8.6".to_owned(),
            0,
            None,
        )
        .await;
    let pushed = {
        let cache = backend.pull_diag_cache.lock().await;
        callback_source_wire_diagnostics(&cache.get(&caller).unwrap().diagnostics)
    };
    let pulled = callback_pull_report(&backend, &caller, source)
        .await
        .unwrap();
    assert_eq!(
        pushed.len(),
        1,
        "genuine held target must issue source advice"
    );
    assert_eq!(pushed, pulled);
    assert_eq!(diag_codes(&pulled), ["E003"]);
    let subject = &pulled[0].data.as_ref().unwrap()["subject"];
    assert_eq!(subject["bakedArgumentCount"], 1);
    assert_eq!(subject["sourceTarget"]["capturedMinimum"], 1);
    assert_eq!(subject["counts"]["counts"], serde_json::json!([4]));
    assert_eq!(
        subject["declarations"][0]["simpleNameBytes"],
        serde_json::json!(b"external")
    );
    assert_eq!(
        pulled[0].range,
        Range::new(Position::new(1, 16), Position::new(1, 18))
    );
}

/// Opt-in source advice reads current project headers and the exact
/// current registration image, without donating a callback execution claim.
#[tokio::test]
async fn original_callback_pull_uses_current_source_and_external_headers() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    let backend = test_backend();
    let caller = Uri::from_str("file:///callback-current.tcl").unwrap();
    let provider = Uri::from_str("file:///callback-current-provider.tcl").unwrap();
    let source = "lsort -command ::external {2 1}";
    register_callback_source(&backend, &provider, "proc external {a} {}")
        .await
        .unwrap();
    register_callback_source(&backend, &caller, source)
        .await
        .unwrap();
    set_callback_project_advice(&backend, false).await;
    assert_eq!(
        callback_pull_report(&backend, &caller, source)
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Diagnostic; 0]
    );
    set_callback_project_advice(&backend, true).await;
    assert_eq!(
        diag_codes(
            &callback_pull_report(&backend, &caller, source)
                .await
                .unwrap()
        ),
        ["E003"]
    );
    let old_analysis = backend
        .analysis_for(&caller, Arc::from(source), "tcl8.6".to_owned())
        .await
        .unwrap();
    register_callback_source(&backend, &provider, "proc external {a b c d} {}")
        .await
        .unwrap();
    assert_eq!(
        diag_codes(
            &callback_pull_report(&backend, &caller, source)
                .await
                .unwrap()
        ),
        ["E002"]
    );
    let current_source = "lsort -command {::external EXTRA MORE} {2 1}";
    register_callback_source(&backend, &caller, current_source)
        .await
        .unwrap();
    assert_eq!(
        callback_pull_report(&backend, &caller, current_source)
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Diagnostic; 0]
    );
    let stale = backend
        .project_callback_diagnostics_if(true, current_source, &old_analysis, &HashSet::new())
        .await
        .unwrap();
    assert!(
        stale
            .iter()
            .all(|diagnostic| diagnostic.callback_source_arity().is_none())
    );
}

/// A known source deletion cannot be revived by an otherwise matching
/// project header on either push publication or the full pull surface.
#[tokio::test]
async fn original_callback_push_and_pull_keep_known_source_barriers() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    let backend = test_backend();
    let caller = Uri::from_str("file:///callback-deleted.tcl").unwrap();
    let provider = Uri::from_str("file:///callback-deleted-provider.tcl").unwrap();
    register_callback_source(&backend, &provider, "proc cb {a b c} {}")
        .await
        .unwrap();
    set_callback_project_advice(&backend, true).await;
    let source = "proc cb {a b c} {}\nrename cb {}\nlsort -command cb {2 1}";
    register_callback_source(&backend, &caller, source)
        .await
        .unwrap();
    backend
        .publish_analyser_diagnostics(
            caller.clone(),
            source.to_owned(),
            "tcl8.6".to_owned(),
            0,
            None,
        )
        .await;
    let pushed = {
        let cache = backend.pull_diag_cache.lock().await;
        callback_source_wire_diagnostics(&cache.get(&caller).unwrap().diagnostics)
    };
    assert_eq!(pushed, [] as [tower_lsp_server::ls_types::Diagnostic; 0]);
    assert_eq!(
        callback_pull_report(&backend, &caller, source)
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Diagnostic; 0]
    );
    let analysis = backend
        .analysis_for(&caller, Arc::from(source), "tcl8.6".to_owned())
        .await
        .unwrap();
    assert!(
        analysis.command_invocations.iter().any(|invocation| {
            invocation
                .original_callback_signature_lookup
                .as_ref()
                .is_some_and(|selection| {
                    !selection.original().permits_external_signature_lookup()
                        && selection.original().target().is_none()
                })
        }),
        "the negative result retains a genuine terminal source barrier"
    );
}

/// Fix #1: the pull path (`full_diagnostics_for`) must feed the IRULE4002
/// compiler check the *URI-scoped* generic-variable patterns, so a folder's
/// `diagnostics.genericVariablePatterns` override applies on the pull path
/// exactly as it does on the push path.  POSITIVE: a folder that disables the
/// check (empty pattern list) suppresses IRULE4002.  NEGATIVE: a doc outside
/// that folder keeps the default-pattern IRULE4002.
#[tokio::test]
async fn pull_path_honours_folder_generic_variable_patterns() {
    let backend = test_backend();
    let folder = Uri::from_str("file:///proj").unwrap();
    let inside = Uri::from_str("file:///proj/rule.tcl").unwrap();
    let outside = Uri::from_str("file:///other/rule.tcl").unwrap();
    *backend.workspace_folders.lock().await = vec![folder.clone()];
    // `static::debug` matches the built-in generic set ⇒ IRULE4002 by default.
    let src = "when RULE_INIT { set static::debug 1 }\n";

    // NEGATIVE (no folder override anywhere): IRULE4002 fires with defaults.
    let baseline = backend
        .full_diagnostics_for(
            &outside,
            Arc::from(src),
            "f5-irules".to_owned(),
            "tcl-irule",
        )
        .await
        .unwrap();
    assert!(
        diag_codes(&baseline).iter().any(|c| c == "IRULE4002"),
        "IRULE4002 should fire with the default generic patterns, got: {:?}",
        diag_codes(&baseline),
    );

    // The folder explicitly empties the generic-pattern set (disables the
    // check).  On the push path this is `resolved_generic_variable_patterns`
    // returning `Some(vec![])`; the pull path must consult the same resolver.
    let fc = FolderConfig {
        generic_variable_patterns: FolderGenericPatterns::Replace(Vec::new()),
        ..FolderConfig::default()
    };
    backend
        .apply_folder_configs(vec![(folder.clone(), fc)])
        .await;

    // POSITIVE: a doc *inside* the folder no longer reports IRULE4002.
    let suppressed = backend
        .full_diagnostics_for(&inside, Arc::from(src), "f5-irules".to_owned(), "tcl-irule")
        .await
        .unwrap();
    assert!(
        !diag_codes(&suppressed).iter().any(|c| c == "IRULE4002"),
        "folder's empty genericVariablePatterns must disable IRULE4002 on the \
             pull path, got: {:?}",
        diag_codes(&suppressed),
    );

    // NEGATIVE: a doc *outside* the folder still gets the default IRULE4002,
    // proving the override is folder-scoped, not process-global.
    let still_fires = backend
        .full_diagnostics_for(
            &outside,
            Arc::from(src),
            "f5-irules".to_owned(),
            "tcl-irule",
        )
        .await
        .unwrap();
    assert!(
        diag_codes(&still_fires).iter().any(|c| c == "IRULE4002"),
        "a doc outside the folder must keep the default IRULE4002, got: {:?}",
        diag_codes(&still_fires),
    );
}

/// The pull path must apply the same W120 package refinement the
/// push path applies, so a workspace whose package database proves a required
/// package transitively provides the flagged package suppresses the false
/// W120.  POSITIVE: with a resolver proving `http` is transitively available,
/// the W120 is refined away.  NEGATIVE: with the default empty resolver, the
/// (genuine) W120 still publishes.
#[tokio::test]
async fn pull_path_applies_w120_package_refinement() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///w120.tcl").unwrap();
    // `package require mywrap` (so `package_requires` is non-empty) plus a
    // call to `http::register`, which the registry marks as requiring the
    // `http` package ⇒ single-file W120 for `http`.
    let src = "package require mywrap\nhttp::register foo 80 bar\n";

    for source in [src, "package require $name\nhttp::register foo 80 bar\n"] {
        let unrefined = backend
            .full_diagnostics_for(&uri, Arc::from(source), "tcl8.6".to_owned(), "tcl")
            .await
            .unwrap();
        assert!(
            diag_codes(&unrefined).iter().any(|code| code == "W120"),
            "an unknown package leaves unrelated W120: {:?}",
            diag_codes(&unrefined),
        );
    }

    // POSITIVE: a wrapper whose implementation does `package require http`
    // makes `http` transitively available ⇒ the W120 is refined away.
    let wrap_ws = TmpWs::new("w120-wrap");
    wrap_ws.write(
        "mywrap/mywrap.tcl",
        "package provide mywrap 1.0\npackage require http\nproc mywrap::go {} {}\n",
    );
    wrap_ws.write(
        "mywrap/pkgIndex.tcl",
        "package ifneeded mywrap 1.0 [list source [file join $dir mywrap.tcl]]\n",
    );
    // Also provide an `http` package so it is resolvable in the database.
    wrap_ws.write(
        "http/http.tcl",
        "package provide http 2.9\nproc http::register {a b c} {}\n",
    );
    wrap_ws.write(
        "http/pkgIndex.tcl",
        "package ifneeded http 2.9 [list source [file join $dir http.tcl]]\n",
    );
    {
        let mut resolver = PackageResolver::new();
        resolver.scan_tree(&wrap_ws.0, 100);
        *backend.package_resolver.write().await = resolver;
    }
    let refined = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        !diag_codes(&refined).iter().any(|c| c == "W120"),
        "the pull path must refine away the W120 once the workspace proves \
             http is transitively available, got: {:?}",
        diag_codes(&refined),
    );
}

// Document-lifecycle + diagnostic-core internals: the
// `did_open` / `did_change` / `did_close` handlers and the synchronous
// diagnostic driver `publish_analyser_diagnostics` → `run_diagnostics_core`,
// asserting on observable state
// (`documents`, `pull_diag_cache`) since the test client's socket is
// detached so published notifications are no-ops.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publish_analyser_diagnostics_caches_tcl_diagnostics() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///life.tcl").unwrap();
    // `y` is set but never read → W211.
    let src = "proc foo {} { set y 1 }\n";
    register(&backend, &uri, src).await;
    backend
        .db_set_source(&uri, src, "tcl8.6".to_owned())
        .await
        .unwrap();
    backend
        .publish_analyser_diagnostics(uri.clone(), src.to_owned(), "tcl8.6".to_owned(), 0, Some(1))
        .await;
    let cache = backend.pull_diag_cache.lock().await;
    let entry = cache
        .get(&uri)
        .expect("run_diagnostics_core should populate the pull cache");
    assert!(
        entry.diagnostics.iter().any(|d| matches!(
            &d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W211"
        )),
        "expected W211 in cached diagnostics, got {:?}",
        entry.diagnostics,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pull_diagnostics_include_compiler_and_optimiser_codes() {
    // The pull handler (`textDocument/diagnostic`) must return
    // the same full set as the push path — analyser + compiler/optimiser +
    // source-style — so editors on the pull path don't lose O-codes.
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({ "optimiser": { "profile": "full" } }))
        .await
        .unwrap();
    let uri = Uri::from_str("file:///pull.tcl").unwrap();
    let src = "if {1} { set x 1 } else { set y 2 }\n";
    let full = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        has_code(&full, "O100"),
        "pull diagnostics must include the optimiser O100, got {:?}",
        full.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
    );
    let analysis = Analyser::new().analyse(src, "tcl8.6");
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::O100),
        "the analyser alone should not emit O100 (sanity)",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pull_diagnostics_share_the_push_paths_byte_evidence_contract() {
    let backend = test_backend();
    let proven_uri = Uri::from_str("file:///pull-bad-utf8.tcl").unwrap();
    let (proven_text, report) = tcl_lsp_core::source_decode::decode_source(b"puts \xff\n");
    let mut proven = DocumentState::with_version(proven_text.clone(), "tcl9.0".into(), 1);
    proven.decode_report = Some(report);
    backend
        .documents
        .lock("test")
        .await
        .insert(proven_uri.clone(), proven);

    let proven_diags = backend
        .full_diagnostics_for(
            &proven_uri,
            Arc::from(proven_text),
            "tcl9.0".to_owned(),
            "tcl",
        )
        .await
        .unwrap();
    assert!(
        proven_diags.iter().any(|d| {
            matches!(
                &d.code,
                Some(tower_lsp_server::ls_types::NumberOrString::String(code))
                    if code == "W107"
            )
        }),
        "a synchronous pull before the push cache is primed must retain W107"
    );

    // FP/TN: the same Unicode character in an unsaved buffer is valid Tcl
    // text. With no matching bytes, pull and push both abstain.
    let literal_uri = Uri::from_str("file:///pull-literal-fffd.tcl").unwrap();
    let literal = "puts \u{fffd}\n";
    backend.documents.lock("test").await.insert(
        literal_uri.clone(),
        DocumentState::with_version(literal.to_owned(), "tcl9.0".into(), 1),
    );
    let literal_diags = backend
        .full_diagnostics_for(&literal_uri, Arc::from(literal), "tcl9.0".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        literal_diags.iter().all(|d| {
            !matches!(
                &d.code,
                Some(tower_lsp_server::ls_types::NumberOrString::String(code))
                    if code == "W107"
            )
        }),
        "a literal U+FFFD must not be inferred to be malformed bytes"
    );
}

/// The pull handler returns a `Full` report carrying a `result_id` and,
/// when the client echoes that same id back as `previousResultId` and the
/// document has not changed, an `Unchanged` report naming the same id.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pull_diagnostic_returns_unchanged_when_result_id_matches() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///pull-unchanged.tcl").unwrap();
    register(&backend, &uri, "set x 1\n").await;

    let first = backend
        .diagnostic(doc_diag_params(&uri, None))
        .await
        .unwrap();
    let result_id = match first {
        DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => full
            .full_document_diagnostic_report
            .result_id
            .expect("first pull must carry a result_id"),
        other => panic!("expected a Full report, got {other:?}"),
    };

    let second = backend
        .diagnostic(doc_diag_params(&uri, Some(&result_id)))
        .await
        .unwrap();
    match second {
        DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Unchanged(u)) => {
            assert_eq!(
                u.unchanged_document_diagnostic_report.result_id, result_id,
                "Unchanged report must name the cached result_id",
            );
        }
        other => panic!("expected an Unchanged report, got {other:?}"),
    }
}

/// `workspace/diagnostic` reports each cached (open) document, and honours a
/// matching `previousResultId` with an `Unchanged` per-document report.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_diagnostic_reports_cached_documents() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///ws-diag.tcl").unwrap();
    register(&backend, &uri, "set x 1\n").await;
    // Prime the pull cache through the document handler.
    let first = backend
        .diagnostic(doc_diag_params(&uri, None))
        .await
        .unwrap();
    let result_id = match first {
        DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => {
            full.full_document_diagnostic_report.result_id.unwrap()
        }
        other => panic!("expected Full, got {other:?}"),
    };

    let params = WorkspaceDiagnosticParams {
        identifier: None,
        previous_result_ids: vec![tower_lsp_server::ls_types::PreviousResultId {
            uri: uri.clone(),
            value: result_id.clone(),
        }],
        work_done_progress_params: tower_lsp_server::ls_types::WorkDoneProgressParams::default(),
        partial_result_params: tower_lsp_server::ls_types::PartialResultParams::default(),
    };
    let report = backend.workspace_diagnostic(params).await.unwrap();
    let WorkspaceDiagnosticReportResult::Report(report) = report else {
        panic!("expected a full workspace report");
    };
    assert_eq!(
        report.items.len(),
        1,
        "the one open document must be reported"
    );
    match &report.items[0] {
        WorkspaceDocumentDiagnosticReport::Unchanged(u) => {
            assert_eq!(u.uri, uri);
            assert_eq!(u.unchanged_document_diagnostic_report.result_id, result_id);
        }
        WorkspaceDocumentDiagnosticReport::Full(full) => {
            panic!(
                "matching previousResultId must yield Unchanged, got Full for {:?}",
                full.uri,
            )
        }
    }
}

/// A pull arriving after an edit (revision bumped) but before the debounced
/// worker refreshes the cache must recompute, not serve the stale cached
/// report — and must not answer `Unchanged` even when the client echoes the
/// stale `result_id`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pull_diagnostic_recomputes_after_edit_bumps_revision() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///pull-stale.tcl").unwrap();
    register(&backend, &uri, "set x 1\n").await;
    let first = backend
        .diagnostic(doc_diag_params(&uri, None))
        .await
        .unwrap();
    let stale_id = match first {
        DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => {
            full.full_document_diagnostic_report.result_id.unwrap()
        }
        other => panic!("expected Full, got {other:?}"),
    };

    // Simulate `did_change`: bump the revision (and text) without running
    // the worker, so the cache entry is now for an older revision.
    {
        let mut docs = backend.documents.lock("test").await;
        let doc = docs.get_mut(&uri).unwrap();
        doc.text = Arc::from("set x 2\n");
        doc.bump_revision(1);
        // This test deliberately bypasses `did_change` and its deferred
        // Salsa publication; it is testing pull-cache revision currency,
        // so mark the synthetic state as already published.
        doc.publication = DocumentPublication::Indexed;
    }

    backend
        .db_set_source(&uri, "set x 2\n", "tcl8.6".to_owned())
        .await
        .expect("the synthetic edit publishes its actual current SourceFile");
    let second = backend
        .diagnostic(doc_diag_params(&uri, Some(&stale_id)))
        .await
        .unwrap();
    match second {
        DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => {
            assert_ne!(
                full.full_document_diagnostic_report.result_id,
                Some(stale_id),
                "a stale result_id must not be reused after an edit",
            );
        }
        other => panic!("an edited buffer must recompute, not serve Unchanged: {other:?}"),
    }
}
