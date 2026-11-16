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

//! Scan rows checked against the inputs they were analysed under.

use super::*;

type CapturedWorkspaceScanInputs = (
    u64,
    u64,
    Arc<HashMap<String, String>>,
    Vec<(Uri, String)>,
    String,
);

async fn captured_workspace_scan_inputs(backend: &Backend) -> CapturedWorkspaceScanInputs {
    let analyser_inputs_guard = backend.analyser_inputs_gate.read().await;
    let epoch = backend.diag_inputs_epoch();
    let document_overrides = backend.document_dialect_override_snapshot().await;
    let folder_dialects = backend.folder_dialect_overrides().await;
    let default_dialect = backend.session_dialect().await;
    let class_factory_generation = *backend.class_factory_generation.read().await;
    drop(analyser_inputs_guard);
    (
        epoch,
        class_factory_generation,
        document_overrides,
        folder_dialects,
        default_dialect,
    )
}

async fn analyse_one_captured_workspace_file(
    backend: &Backend,
    path: PathBuf,
    inputs: CapturedWorkspaceScanInputs,
) -> (PackageResolver, usize, bool, bool) {
    let (epoch, factory_generation, document_overrides, folder_dialects, default_dialect) = inputs;
    backend
        .analyse_and_merge_scanned_files(
            vec![path],
            HashSet::new(),
            ScanAnalysisInputs {
                analyser_inputs_epoch: epoch,
                class_factory_generation: factory_generation,
                document_dialect_overrides: document_overrides,
                folder_dialects,
                default_dialect,
            },
            PackageResolver::new(),
        )
        .await
        .expect("the fixed source file is readable")
}

/// Positive control: a closed row analysed and published under its captured
/// document override remains in the workspace index with complete coverage.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_scan_publishes_under_unchanged_closed_document_override() {
    let root = TmpWs::new("scan-dialect-current");
    let path = root.0.join("source.tcl");
    root.write("source.tcl", "proc stable {} {}\n");
    let uri = Uri::from_file_path(&path).unwrap();
    let backend = test_backend();
    backend
        .set_document_dialect_override_command(&[
            serde_json::json!(uri.as_str()),
            serde_json::json!("f5-irules"),
        ])
        .await
        .expect("install the closed-file override");
    let inputs = captured_workspace_scan_inputs(&backend).await;
    assert_eq!(
        Backend::dialect_for_closed_sync(
            &uri,
            "proc stable {} {}\n",
            inputs.2.get(uri.as_str()).map(String::as_str),
            &inputs.3,
            &inputs.4,
        ),
        "f5-irules",
        "the closed-file override is the selected row dialect",
    );

    let (_, files_count, inputs_current, coverage_complete) =
        analyse_one_captured_workspace_file(&backend, path, inputs).await;

    assert!(inputs_current, "the captured inputs did not change");
    assert!(
        coverage_complete,
        "a current published row has full coverage"
    );
    assert_eq!(files_count, 1, "the current closed row should publish");
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .contains_document(uri.as_str()),
    );
}

/// Adversarial control for the original A→B race: the URI stays closed and
/// its source bytes stay fixed, but the override changes after the scan
/// snapshot. The old row must be omitted and cannot certify a complete
/// source inventory.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_scan_omits_closed_row_after_document_override_changes() {
    let root = TmpWs::new("scan-dialect-stale-document");
    let path = root.0.join("source.tcl");
    root.write("source.tcl", "proc stable {} {}\n");
    let uri = Uri::from_file_path(&path).unwrap();
    let backend = test_backend();
    backend
        .set_document_dialect_override_command(&[
            serde_json::json!(uri.as_str()),
            serde_json::json!("f5-irules"),
        ])
        .await
        .expect("install override A");
    let inputs = captured_workspace_scan_inputs(&backend).await;
    let stale_epoch = inputs.0;
    backend
        .set_document_dialect_override_command(&[
            serde_json::json!(uri.as_str()),
            serde_json::json!("tcl9.0"),
        ])
        .await
        .expect("replace override A with B while the URI is closed");

    let (_, files_count, inputs_current, coverage_complete) =
        analyse_one_captured_workspace_file(&backend, path, inputs).await;

    assert_eq!(files_count, 0, "the stale A row must be rejected");
    assert!(!coverage_complete, "stale input omissions are not complete");
    assert!(!inputs_current, "A's epoch must not be relabelled as B");
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .contains_document(uri.as_str()),
    );
    assert!(
        !backend
            .publish_workspace_scan_inventory_if_current(
                vfs::WorkspaceSourceInventoryProvider::Filesystem,
                Vec::new(),
                true,
                stale_epoch,
                *backend.class_factory_generation.read().await,
            )
            .await
    );
    assert_eq!(
        backend
            .workspace_index
            .read()
            .await
            .source_inventory_scan_complete(),
        Some(false),
        "an unavailable stale candidate must not certify complete absence",
    );
}

/// The explicit default setter and the session override setter are both
/// analyzer-input generations: a captured closed row becomes unavailable
/// when either selection changes before its worker starts.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_scan_rejects_rows_after_default_or_session_dialect_changes() {
    for change_session_override in [false, true] {
        let root = TmpWs::new(if change_session_override {
            "scan-dialect-stale-session"
        } else {
            "scan-dialect-stale-default"
        });
        let path = root.0.join("source.tcl");
        root.write("source.tcl", "proc stable {} {}\n");
        let backend = test_backend();
        let inputs = captured_workspace_scan_inputs(&backend).await;
        if change_session_override {
            backend
                .set_session_dialect_override_command(&[serde_json::json!("f5-irules")])
                .await
                .expect("change the session override");
        } else {
            backend
                .set_dialect_command(&[serde_json::json!("tcl9.0")])
                .await
                .expect("change the explicit default");
        }

        let (_, files_count, inputs_current, coverage_complete) =
            analyse_one_captured_workspace_file(&backend, path, inputs).await;
        assert!(!coverage_complete, "a stale row cannot certify absence");

        assert_eq!(files_count, 0, "the captured row is stale");
        assert!(
            !inputs_current,
            "the changed dialect must advance the epoch"
        );
    }
}

/// A changed class-factory generation also makes a closed analysis row
/// unavailable, even when its source and dialect inputs are unchanged.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_scan_rejects_rows_after_factory_generation_changes() {
    let root = TmpWs::new("scan-factory-stale");
    let path = root.0.join("source.tcl");
    root.write("source.tcl", "proc stable {} {}\n");
    let backend = test_backend();
    let inputs = captured_workspace_scan_inputs(&backend).await;
    let captured_epoch = inputs.0;
    let captured_factory_generation = inputs.1;
    {
        let mut generation = backend.class_factory_generation.write().await;
        *generation = generation.saturating_add(1);
    }

    let (_, files_count, inputs_current, coverage_complete) =
        analyse_one_captured_workspace_file(&backend, path, inputs).await;

    assert_eq!(files_count, 0, "the old factory view must be omitted");
    assert!(
        !coverage_complete,
        "a stale factory row cannot certify absence"
    );
    assert!(!inputs_current, "the captured factory generation is stale");
    assert!(
        !backend
            .publish_workspace_scan_inventory_if_current(
                vfs::WorkspaceSourceInventoryProvider::Filesystem,
                Vec::new(),
                true,
                captured_epoch,
                captured_factory_generation,
            )
            .await
    );
    assert_eq!(
        backend
            .workspace_index
            .read()
            .await
            .source_inventory_scan_complete(),
        Some(false),
        "the final receipt must join the captured factory generation",
    );
}

/// Open files and candidates confirmed absent by the store are intentional
/// skips and do not turn a completed census into unavailable coverage.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_scan_open_and_absent_candidates_remain_intentional_skips() {
    let root = TmpWs::new("scan-intentional-skips");
    let open_path = root.0.join("open.tcl");
    let absent_path = root.0.join("absent.tcl");
    root.write("open.tcl", "proc openProc {} {}\n");
    root.write("absent.tcl", "proc absentProc {} {}\n");
    std::fs::remove_file(&absent_path).unwrap();
    let open_uri = Uri::from_file_path(&open_path).unwrap();
    let backend = test_backend();
    let (epoch, factory_generation, document_overrides, folder_dialects, default_dialect) =
        captured_workspace_scan_inputs(&backend).await;

    let (_, files_count, inputs_current, coverage_complete) = backend
        .analyse_and_merge_scanned_files(
            vec![open_path, absent_path],
            HashSet::from([open_uri]),
            ScanAnalysisInputs {
                analyser_inputs_epoch: epoch,
                class_factory_generation: factory_generation,
                document_dialect_overrides: document_overrides,
                folder_dialects,
                default_dialect,
            },
            PackageResolver::new(),
        )
        .await
        .expect("intentional skips do not fail the workspace scan");

    assert!(inputs_current, "the configured inputs stayed current");
    assert_eq!(files_count, 0, "neither skipped candidate is published");
    assert!(
        coverage_complete,
        "open and confirmed-absent paths are accounted for"
    );
}

/// A transient writer can refuse a batch after analysis. Once it releases,
/// stable final generations must not renew the incomplete coverage receipt.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_scan_transient_busy_batch_stays_incomplete() {
    let root = TmpWs::new("scan-transient-busy-batch");
    let path = root.0.join("source.tcl");
    let text = "proc stable {} {}\n";
    root.write("source.tcl", text);
    let uri = Uri::from_file_path(&path).unwrap();
    let backend = test_backend();
    let (epoch, factory_generation, document_overrides, folder_dialects, default_dialect) =
        captured_workspace_scan_inputs(&backend).await;
    let admission = Arc::new(DocumentPathAdmission::capture(&uri, backend.store.as_ref()).unwrap());
    let driver = backend.fresh_analysis_driver(admission).await;
    let dialect = Backend::dialect_for_closed_sync(
        &uri,
        text,
        document_overrides.get(uri.as_str()).map(String::as_str),
        &folder_dialects,
        &default_dialect,
    );
    let seed = driver.analyse(text, &dialect, None).unwrap();
    let batch = backend
        .current_scanned_rows(vec![ScannedDocument {
            uri: uri.clone(),
            text: text.to_owned(),
            dialect,
            expected_analyser_inputs_epoch: Some(epoch),
            expected_class_factory_generation: Some(factory_generation),
            seed,
        }])
        .await
        .unwrap();
    assert_eq!(
        batch.rows.len(),
        1,
        "the control starts with an admissible row"
    );

    let writer = backend.analyser_inputs_gate.write().await;
    let (published, currentness) = backend
        .publish_scanned_disk_results(
            &batch,
            &[],
            DiskRemovalPolicy::ClosedOnly,
            "workspace_scan_transient_busy_batch",
        )
        .await
        .unwrap();
    drop(writer);
    assert!(
        published.is_empty(),
        "busy configured inputs refuse the row"
    );
    assert_eq!(currentness, DiskPublicationOutcome::Unavailable);

    let complete = backend
        .publish_workspace_scan_inventory_if_current(
            vfs::WorkspaceSourceInventoryProvider::Filesystem,
            Vec::new(),
            currentness != DiskPublicationOutcome::Unavailable,
            epoch,
            factory_generation,
        )
        .await;
    assert!(
        !complete,
        "a later stable read cannot renew refused coverage"
    );
    assert_eq!(
        backend
            .workspace_index
            .read()
            .await
            .source_inventory_scan_complete(),
        Some(false),
    );
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .contains_document(uri.as_str()),
        "the refused row must remain unpublished",
    );
}

/// A real overlay miss returns an analysis seed with a typed unavailable
/// context. The row may be published with that status, but the scan cannot
/// certify complete absence or replace its resolver from this candidate.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_scan_unavailable_analysis_keeps_coverage_incomplete() {
    use salsa::Setter as _;
    use vfs::SourceStore as _;

    let root = PathBuf::from("/workspace/scan-unavailable-analysis");
    let path = root.join("source.tcl");
    let uri = Uri::from_file_path(&path).unwrap();
    let store = Arc::new(vfs::MemoryStore::new());
    store.upsert_uri(
        uri.as_str(),
        path.clone(),
        b"proc unavailableProc {} {}\n".to_vec(),
    );
    store.set_workspace_source_inventory(
        root.clone(),
        "complete-source-census",
        vfs::WorkspaceSourceInventoryCompleteness::Complete,
    );
    let mut backend = test_backend();
    backend.state_mut_for_test().store = store.clone();
    let provider = store.workspace_source_inventory_provider();
    let observations = vec![store.workspace_source_inventory(&root)];
    assert!(observations[0].admits_complete_inventory());
    {
        let _inputs = backend.analyser_inputs_gate.write().await;
        let config = *backend.db_config.lock().await;
        let mut db = backend.db.lock().await;
        config.set_spec_pack_key(&mut *db).to(u64::MAX);
        backend.invalidate_diag_inputs();
    }
    let inputs = captured_workspace_scan_inputs(&backend).await;
    let captured_epoch = inputs.0;
    let captured_factory_generation = inputs.1;
    let admission = Arc::new(DocumentPathAdmission::capture(&uri, backend.store.as_ref()).unwrap());
    let driver = backend.fresh_analysis_driver(admission).await;
    let seed = driver
        .analyse("proc unavailableProc {} {}\n", "tcl8.6", None)
        .expect("the real overlay miss is returned as a typed analysis result");
    assert!(
        seed.analysis.analysis_context_unavailable.is_some(),
        "the configured analyzer must report the unavailable overlay ingress",
    );

    let (_, files_count, inputs_current, coverage_complete) =
        analyse_one_captured_workspace_file(&backend, path, inputs).await;

    assert!(
        inputs_current,
        "the analyzer and factory inputs stayed stable"
    );
    assert_eq!(
        files_count, 1,
        "the typed unavailable row remains represented"
    );
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .contains_document(uri.as_str()),
        "the unavailable seed still travels through the existing typed row path",
    );
    assert!(
        !coverage_complete,
        "an unavailable declaration walk cannot prove absence"
    );
    assert!(
        !backend
            .publish_workspace_scan_resolver_if_current(
                PackageResolver::new(),
                captured_epoch,
                captured_factory_generation,
                inputs_current && coverage_complete,
            )
            .await
    );
    assert!(
        !backend
            .publish_workspace_scan_inventory_if_current(
                provider,
                observations,
                coverage_complete,
                captured_epoch,
                captured_factory_generation,
            )
            .await
    );
}
