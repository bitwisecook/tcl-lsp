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

use super::*;

async fn publish_source(
    backend: &Backend,
    store: &vfs::MemoryStore,
    path: &Path,
    source: &str,
    version: i32,
) -> Uri {
    let uri = Uri::from_file_path(path).unwrap();
    store.upsert_uri(uri.as_str(), path, source.as_bytes().to_vec());
    backend
        .db_set_source(&uri, source, "tcl8.6".into())
        .await
        .expect("real configured SourceFile/project publication");
    let admission = backend
        .source_path_admissions
        .lock()
        .unwrap()
        .get(&uri)
        .unwrap()
        .admission
        .clone();
    backend
        .documents
        .lock("proposal_endpoint_fixture")
        .await
        .insert(
            uri.clone(),
            DocumentState::with_version(source.into(), "tcl8.6".into(), version)
                .with_path_admission(admission)
                .with_language_id("tcl".into()),
        );
    let analysis = backend
        .analysis_for(&uri, Arc::from(source), "tcl8.6".into())
        .await
        .expect("fixture retains the actual selected full analysis input");
    backend
        .workspace_index
        .write()
        .await
        .replace_document(uri.as_str(), &analysis);
    uri
}

async fn fixture() -> (Backend, Arc<vfs::MemoryStore>, Uri, Uri, PathBuf) {
    let store = Arc::new(vfs::MemoryStore::new());
    let mut backend = crate::tests::test_backend();
    backend.state_mut_for_test().store = store.clone();
    let root = std::env::temp_dir().join("tcl-lsp-proposal-endpoint-controls");
    let selected = publish_source(&backend, &store, &root.join("selected.tcl"), "", 7).await;
    let other = publish_source(
        &backend,
        &store,
        &root.join("other.tcl"),
        "# unchanged second project input\n",
        2,
    )
    .await;
    // Use the real epoch issuers, after the actual selected registries and
    // indexed analysis have been prepared. These are not guessed labels.
    sync_evaluator_epoch(&backend.db).await;
    sync_overlay_epoch(&backend.db).await;
    (backend, store, selected, other, root)
}

async fn command(
    backend: &Backend,
    uri: &Uri,
    version: i32,
    source: &str,
    proposal: &str,
) -> serde_json::Value {
    backend
        .proposal_diagnostics_command(&[serde_json::json!({
            "uri": uri,
            "version": version,
            "source": source,
            "proposal": proposal,
        })])
        .await
        .expect("actual proposal endpoint response")
        .expect("serialized response")
}

fn same_optional_arc<T>(before: Option<&Arc<T>>, after: Option<&Arc<T>>) -> bool {
    match (before, after) {
        (None, None) => true,
        (Some(before), Some(after)) => Arc::ptr_eq(before, after),
        _ => false,
    }
}

fn assert_invalid_proposal(invalid: &serde_json::Value, selected: &Uri, clean: &serde_json::Value) {
    assert_eq!(invalid["status"], "complete", "{invalid}");
    assert_eq!(invalid["uri"], selected.as_str());
    assert_eq!(invalid["version"], 7);
    assert_eq!(invalid["source"], "");
    assert_eq!(invalid["proposal"], "set\n");
    assert!(
        invalid["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["code"] == "E002"),
        "{invalid}"
    );
    assert_ne!(invalid["resultId"], clean["resultId"]);
}

fn assert_clean_proposal(clean: &serde_json::Value) {
    assert_eq!(clean["status"], "complete", "{clean}");
    assert!(
        clean["diagnostics"].as_array().unwrap().is_empty(),
        "{clean}"
    );
}

fn proposal_file_values(
    db: &tcl_lsp_db::TclDatabase,
    file: tcl_lsp_db::SourceFile,
    config: tcl_lsp_db::AnalyserConfig,
) -> (
    String,
    String,
    Option<String>,
    u64,
    tcl_lsp_db::DiagnosticConfigurationSnapshot,
) {
    (
        file.text(db).clone(),
        file.dialect(db).clone(),
        file.path(db).clone(),
        file.sidecar_stubs_epoch(db),
        config.diagnostic_report_configuration(db),
    )
}

#[tokio::test]
async fn actual_proposal_command_changes_diagnostics_without_mutating_live_project() {
    let (backend, _, selected, other, _) = fixture().await;
    let index_generation = backend.workspace_index.read().await.generation();
    let members = backend.db_project_members.lock().await.clone();
    let project = backend.db_project.lock().await.unwrap();
    let configurations = project
        .token_configurations(&*backend.db.lock().await)
        .clone();
    let mut before = Vec::new();
    let revision = {
        let db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        for uri in [&selected, &other] {
            let file = files[uri];
            let config = configurations
                .as_ref()
                .unwrap()
                .iter()
                .find(|(candidate, _)| *candidate == file)
                .unwrap()
                .1;
            before.push((
                uri.clone(),
                file,
                proposal_file_values(&db, file, config),
                (
                    file.external_call_sites(&*db).clone(),
                    file.workspace_class_factories(&*db).clone(),
                    file.workspace_subclass_methods(&*db).clone(),
                ),
            ));
        }
        tcl_lsp_db::database_revision(&db)
    };
    let selected_admission = backend.source_path_admissions.lock().unwrap()[&selected]
        .admission
        .clone();
    let clean = command(&backend, &selected, 7, "", "").await;
    assert_clean_proposal(&clean);
    let invalid = command(&backend, &selected, 7, "", "set\n").await;
    assert_invalid_proposal(&invalid, &selected, &clean);

    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    assert_eq!(tcl_lsp_db::database_revision(&db), revision);
    assert!(project.token_configurations(&*db) == &configurations);
    for (uri, file, values, evidence) in &before {
        assert!(files.get(uri) == Some(file));
        let config = configurations
            .as_ref()
            .unwrap()
            .iter()
            .find(|(candidate, _)| candidate == file)
            .unwrap()
            .1;
        assert_eq!(proposal_file_values(&db, *file, config), *values);
        assert!(same_optional_arc(
            evidence.0.as_ref(),
            file.external_call_sites(&*db).as_ref()
        ));
        assert!(same_optional_arc(
            evidence.1.as_ref(),
            file.workspace_class_factories(&*db).as_ref()
        ));
        assert!(same_optional_arc(
            evidence.2.as_ref(),
            file.workspace_subclass_methods(&*db).as_ref()
        ));
    }
    drop(files);
    drop(db);
    assert!(*backend.db_project.lock().await == Some(project));
    assert_eq!(*backend.db_project_members.lock().await, members);
    assert_eq!(
        backend.workspace_index.read().await.generation(),
        index_generation
    );
    let documents = backend.documents.lock("proposal_endpoint_unchanged").await;
    let document = &documents[&selected];
    assert_eq!(document.raw(), "");
    assert_eq!(document.version, Some(7));
    assert!(Arc::ptr_eq(
        document.path_admission.as_ref().unwrap(),
        &selected_admission
    ));
    assert_eq!(
        documents[&other].raw(),
        "# unchanged second project input\n"
    );
    assert_eq!(documents[&other].version, Some(2));
    drop(documents);
    assert!(
        backend.pull_diag_cache.lock().await.is_empty(),
        "proposal is not a push/cache publication"
    );
}

#[tokio::test]
async fn actual_proposal_command_refuses_stale_original_and_withdrawn_source_admission() {
    let (backend, store, selected, _, root) = fixture().await;
    let first = command(&backend, &selected, 7, "", "").await;
    assert_eq!(first["status"], "complete", "{first}");
    let current_source = "# edited original source\n";
    publish_source(
        &backend,
        &store,
        &root.join("selected.tcl"),
        current_source,
        8,
    )
    .await;
    let stale = command(&backend, &selected, 7, "", "set\n").await;
    assert_eq!(stale["status"], "unavailable", "{stale}");
    assert_eq!(stale["reason"], "stale");
    let current = command(&backend, &selected, 8, current_source, "").await;
    assert_eq!(current["status"], "complete", "{current}");
    assert!(store.remove_uri(selected.as_str()));
    let withdrawn = command(&backend, &selected, 8, current_source, "").await;
    assert_eq!(withdrawn["status"], "unavailable", "{withdrawn}");
    assert_eq!(withdrawn["reason"], "context");
}

#[tokio::test]
async fn actual_proposal_command_refuses_selected_provider_and_unversioned_disk_provider() {
    for selected_is_provider in [true, false] {
        let (backend, store, selected, _, root) = fixture().await;
        let clean = command(&backend, &selected, 7, "", "").await;
        assert_eq!(clean["status"], "complete", "{clean}");
        let provider_path = if selected_is_provider {
            root.join("selected.tcl")
        } else {
            let path = root.join("disk-provider.tcl");
            // Readable SourceStore bytes are deliberately not a retained
            // indexed SourceFile or a byte-version lease.
            store.upsert(&path, b"proc provided {} {}\n".to_vec());
            assert!(backend.store.read_source(&path).is_ok());
            assert!(
                !backend
                    .db_files
                    .lock()
                    .await
                    .contains_key(&Uri::from_file_path(&path).unwrap())
            );
            path
        };
        let provider_name = provider_path.file_name().unwrap().to_str().unwrap();
        backend
            .package_resolver
            .write()
            .await
            .add_original_pkg_index(
                &format!("package ifneeded own 1.0 {{source {provider_name}}}"),
                &root,
                &root.join("pkgIndex.tcl"),
                &|_| true,
                &|_| Vec::new(),
            );
        assert!(
            backend
                .package_resolver
                .read()
                .await
                .provider_source_files()
                .contains(&provider_path)
        );
        let answer = command(
            &backend,
            &selected,
            7,
            "",
            "package require own\nprovided\n",
        )
        .await;
        assert_eq!(answer["status"], "unavailable", "{answer}");
        assert_eq!(answer["reason"], "context");
        let db = backend.db.lock().await;
        let file = backend.db_files.lock().await[&selected];
        assert_eq!(file.text(&*db), "");
    }
}

#[tokio::test]
async fn actual_proposal_command_never_turns_suppression_or_incomplete_source_into_clean() {
    let (backend, _, selected, _, _) = fixture().await;
    backend
        .feature_toggles
        .lock()
        .await
        .set
        .insert("diagnostics".into(), false);
    backend.invalidate_diag_inputs();
    let suppressed = command(&backend, &selected, 7, "", "").await;
    assert_eq!(suppressed["status"], "suppressed", "{suppressed}");
    assert_eq!(suppressed["reason"], "disabled");
    assert!(suppressed.get("diagnostics").is_none());
    backend
        .feature_toggles
        .lock()
        .await
        .set
        .insert("diagnostics".into(), true);
    backend.invalidate_diag_inputs();
    let incomplete = command(&backend, &selected, 7, "", "proc unfinished {").await;
    assert_eq!(incomplete["status"], "unavailable", "{incomplete}");
    assert_eq!(incomplete["reason"], "context");
    assert!(incomplete.get("diagnostics").is_none());
}
async fn poll_capture_once<F: std::future::Future>(
    mut future: std::pin::Pin<&mut F>,
) -> std::task::Poll<F::Output> {
    std::future::poll_fn(|context| std::task::Poll::Ready(future.as_mut().poll(context))).await
}

async fn capture_inputs(backend: &Backend, selected: &Uri) -> DiagInputs {
    let inputs = backend
        .diag_inputs(selected, tcl_lsp_core::profile_for_dialect("tcl8.6"))
        .await;
    assert!(inputs.inputs_epoch.is_some());
    assert_eq!(backend.db.snapshot_report().outstanding, 0);
    inputs
}

#[tokio::test]
async fn proposal_index_wait_owns_no_snapshot_and_recaptures_after_actual_factory_write() {
    let (backend, _, selected, _, _) = fixture().await;
    let inputs = capture_inputs(&backend, &selected).await;
    let handles = EvidenceHandles::from_inputs(&inputs);
    let file = backend.db_files.lock().await[&selected];
    let revision = tcl_lsp_db::database_revision(&*backend.db.lock().await);
    let index = backend.workspace_index.write().await;
    let mut capture = Box::pin(backend.capture_proposal_workspace(
        &inputs,
        inputs.inputs_epoch.unwrap(),
        &handles,
    ));
    assert!(poll_capture_once(capture.as_mut()).await.is_pending());
    assert_eq!(
        backend.db.snapshot_report().outstanding,
        0,
        "a proposal parked on the index must not pin Salsa writers"
    );
    let wanted = Arc::new(tcl_compiler::analyser::ClassFactoryIndex::default());
    let changed = crate::rt::timeout(
        std::time::Duration::from_secs(3),
        crate::apply_workspace_class_factories_if_current(
            &handles,
            revision,
            Some(Arc::clone(&wanted)),
            vec![(selected, file)],
        ),
    )
    .await
    .expect("the actual factory writer must finish while proposal index capture is parked");
    assert!(matches!(changed, crate::ClassFactoryRound::Moved(_)));
    let expected_revision = tcl_lsp_db::database_revision(&*backend.db.lock().await);
    let expected_generation = *backend.class_factory_generation.read().await;
    drop(index);
    let workspace = crate::rt::timeout(std::time::Duration::from_secs(3), capture)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(workspace.captured.revision, expected_revision);
    assert_eq!(workspace.factory_generation, expected_generation);
    assert!(Arc::ptr_eq(
        file.workspace_class_factories(&*workspace.captured.snapshot)
            .as_ref()
            .unwrap(),
        &wanted
    ));
    drop(workspace);
    assert_eq!(backend.db.snapshot_report().outstanding, 0);
}

#[tokio::test]
async fn proposal_factory_wait_owns_no_snapshot_and_captures_the_locked_generation() {
    use salsa::Setter as _;
    let (backend, _, selected, _, _) = fixture().await;
    let inputs = capture_inputs(&backend, &selected).await;
    let handles = EvidenceHandles::from_inputs(&inputs);
    let file = backend.db_files.lock().await[&selected];
    let mut factory = backend.class_factory_generation.write().await;
    let mut capture = Box::pin(backend.capture_proposal_workspace(
        &inputs,
        inputs.inputs_epoch.unwrap(),
        &handles,
    ));
    assert!(poll_capture_once(capture.as_mut()).await.is_pending());
    assert_eq!(
        backend.db.snapshot_report().outstanding,
        0,
        "a proposal waiting for the factory writer must not retain the snapshot that writer needs to drain"
    );
    let wanted = Arc::new(tcl_compiler::analyser::ClassFactoryIndex::default());
    let expected_revision = {
        let mut db = backend.db.lock().await;
        file.set_workspace_class_factories(&mut *db)
            .to(Some(Arc::clone(&wanted)));
        tcl_lsp_db::database_revision(&db)
    };
    *factory += 1;
    let expected_generation = *factory;
    drop(factory);
    let workspace = crate::rt::timeout(std::time::Duration::from_secs(3), capture)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(workspace.captured.revision, expected_revision);
    assert_eq!(workspace.factory_generation, expected_generation);
    assert!(Arc::ptr_eq(
        file.workspace_class_factories(&*workspace.captured.snapshot)
            .as_ref()
            .unwrap(),
        &wanted
    ));
    drop(workspace);
    assert_eq!(backend.db.snapshot_report().outstanding, 0);
}

#[tokio::test]
async fn proposal_package_wait_owns_no_snapshot_and_recaptures_after_factory_write() {
    let (backend, _, selected, _, _) = fixture().await;
    let inputs = capture_inputs(&backend, &selected).await;
    let handles = EvidenceHandles::from_inputs(&inputs);
    let file = backend.db_files.lock().await[&selected];
    let revision = tcl_lsp_db::database_revision(&*backend.db.lock().await);
    let packages = backend.package_resolver.write().await;
    let mut capture = Box::pin(backend.capture_proposal_workspace(
        &inputs,
        inputs.inputs_epoch.unwrap(),
        &handles,
    ));
    assert!(poll_capture_once(capture.as_mut()).await.is_pending());
    assert_eq!(
        backend.db.snapshot_report().outstanding,
        0,
        "a proposal waiting for package inputs must not retain a Salsa snapshot"
    );
    let wanted = Arc::new(tcl_compiler::analyser::ClassFactoryIndex::default());
    let changed = crate::rt::timeout(
        std::time::Duration::from_secs(3),
        crate::apply_workspace_class_factories_if_current(
            &handles,
            revision,
            Some(Arc::clone(&wanted)),
            vec![(selected, file)],
        ),
    )
    .await
    .expect("factory writer must finish while proposal package capture is parked");
    assert!(matches!(changed, crate::ClassFactoryRound::Moved(_)));
    let expected_revision = tcl_lsp_db::database_revision(&*backend.db.lock().await);
    let expected_generation = *backend.class_factory_generation.read().await;
    drop(packages);
    let workspace = crate::rt::timeout(std::time::Duration::from_secs(3), capture)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(workspace.captured.revision, expected_revision);
    assert_eq!(workspace.factory_generation, expected_generation);
    assert!(Arc::ptr_eq(
        file.workspace_class_factories(&*workspace.captured.snapshot)
            .as_ref()
            .unwrap(),
        &wanted
    ));
    drop(workspace);
    assert_eq!(backend.db.snapshot_report().outstanding, 0);
}
