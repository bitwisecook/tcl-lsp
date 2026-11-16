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
use salsa::Setter as _;

async fn actual_open_source(backend: &Backend, uri: &Uri) -> super::super::DocumentState {
    let admission = Arc::new(DocumentPathAdmission::capture(uri, backend.store.as_ref()).unwrap());
    let document = super::super::DocumentState::with_version(
        "puts \"\\x4142\"".to_owned(),
        "tcl8.4".to_owned(),
        7,
    )
    .with_path_admission(admission)
    .normalised_for_analysis();
    backend
        .documents
        .lock("coarse source fixture")
        .await
        .insert(uri.clone(), document.clone());
    document
}

#[tokio::test]
async fn coarse_receipt_uses_actual_producer_generations_without_renewal() {
    for advance_factory in [false, true] {
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///coarse-producer-generations.tcl".parse().unwrap();
        let snapshot = actual_open_source(&backend, &uri).await;
        let inputs_epoch = backend.diag_inputs_epoch();
        let factory_generation = *backend.class_factory_generation.read().await;
        advance_coarse_generation(&backend, advance_factory).await;
        let analysis = backend
            .capture_fresh_source_analysis(
                &uri,
                snapshot.clone(),
                FreshSourceAnalysisPurpose::SemanticStructure,
            )
            .await
            .unwrap()
            .unwrap();
        let torn = CoarseSemanticTokenSource {
            analysis,
            inputs_epoch,
            factory_generation,
        };
        assert!(!coarse_source_is_current(&backend, &torn).await);
        let source = CoarseSemanticTokenSource::from_fresh_analysis(torn.analysis).unwrap();
        assert!(coarse_source_is_current(&backend, &source).await);
        assert_ne!(
            super::super::configured_coarse_semantic_tokens(&snapshot.text, source.analysis())
                .unwrap(),
            [] as [u32; 0]
        );
        advance_coarse_generation(&backend, !advance_factory).await;
        assert!(!coarse_source_is_current(&backend, &source).await);
        let old = CoarseSemanticTokenSource::from_fresh_analysis(source.analysis).unwrap();
        assert!(!coarse_source_is_current(&backend, &old).await);
        let fresh = backend
            .capture_coarse_semantic_source(&uri, &snapshot)
            .await
            .unwrap()
            .unwrap();
        assert!(coarse_source_is_current(&backend, &fresh).await);
        let complete = backend
            .capture_fresh_source_analysis(
                &uri,
                snapshot.clone(),
                FreshSourceAnalysisPurpose::ReadonlyComplete,
            )
            .await
            .unwrap()
            .unwrap();
        assert!(CoarseSemanticTokenSource::from_fresh_analysis(complete).is_none());
    }
}

async fn advance_coarse_generation(backend: &Backend, factory: bool) {
    if factory {
        *backend.class_factory_generation.write().await += 1;
    } else {
        backend.invalidate_diag_inputs();
    }
}

#[tokio::test]
async fn coarse_tokens_retain_actual_configured_input_and_final_source_owner() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///coarse-configured-input.tcl".parse().unwrap();
    let snapshot = actual_open_source(&backend, &uri).await;
    let source = backend
        .capture_coarse_semantic_source(&uri, &snapshot)
        .await
        .unwrap()
        .expect("actual configured structure-only source");
    let analysis = source.analysis();
    let input = analysis.resolved_input.as_ref().unwrap();
    assert_eq!(
        input.analyser_profile(),
        crate::profile_for_dialect("tcl8.4")
    );
    assert_eq!(analysis.body_lexer_config, Some(input.lexer_config()));
    assert!(tcl_compiler::source_graph::current_analysis(&snapshot.text, analysis).is_some());
    let data = super::super::configured_coarse_semantic_tokens(&snapshot.text, analysis).unwrap();
    assert_ne!(data.len(), 0);
    let original = data.clone();
    let issued = backend
        .with_current_coarse_semantic_source(&source, || {
            let mut cache = backend.last_semantic_tokens.try_lock().ok()?;
            cache.insert(
                uri.clone(),
                ("actual-coarse-control".to_owned(), data.clone()),
            );
            Some(())
        })
        .await
        .unwrap();
    assert!(issued.is_some());
    assert_eq!(backend.last_semantic_tokens.lock().await[&uri].1, original);
    backend
        .documents
        .lock("change actual version")
        .await
        .get_mut(&uri)
        .unwrap()
        .version = Some(8);
    assert!(
        backend
            .with_current_coarse_semantic_source(&source, || -> Option<()> {
                panic!("stale source must not reach cache/wire projection")
            })
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(backend.last_semantic_tokens.lock().await[&uri].1, original);
}

#[tokio::test]
async fn coarse_recompute_requires_unchanged_document_after_invalidation() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///coarse-recompute.tcl".parse().unwrap();
    let snapshot = actual_open_source(&backend, &uri).await;
    let source = backend
        .capture_coarse_semantic_source(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    assert!(
        backend
            .coarse_source_recompute_snapshot(&source)
            .await
            .unwrap()
            .is_none()
    );
    backend.invalidate_diag_inputs();
    assert!(!coarse_source_is_current(&backend, &source).await);
    assert!(
        backend
            .coarse_source_recompute_snapshot(&source)
            .await
            .unwrap()
            .is_some()
    );
    let fresh = backend
        .capture_coarse_semantic_source(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    assert!(coarse_source_is_current(&backend, &fresh).await);
    backend
        .documents
        .lock("changed coarse recompute document")
        .await
        .get_mut(&uri)
        .unwrap()
        .version = Some(99);
    assert!(
        backend
            .coarse_source_recompute_snapshot(&source)
            .await
            .unwrap()
            .is_none()
    );
    assert!(!coarse_source_is_current(&backend, &fresh).await);
}

#[tokio::test]
async fn coarse_recompute_retains_resolved_same_profile_generation() {
    let backend = crate::tests::test_backend();
    backend
        .apply_global_config(&serde_json::json!({"dialect":"tcl8.4"}))
        .await
        .unwrap();
    let uri: Uri = "file:///coarse-resolved-generation.tcl".parse().unwrap();
    let snapshot = actual_open_source(&backend, &uri).await;
    let source = backend
        .capture_coarse_semantic_source(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    assert!(coarse_source_is_current(&backend, &source).await);
    backend.reresolve_open_document_dialects().await.unwrap();
    assert!(!coarse_source_is_current(&backend, &source).await);
    let current = backend
        .coarse_source_recompute_snapshot(&source)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.raw(), snapshot.raw());
    assert_eq!(current.dialect, snapshot.dialect);
    assert!(current.dialect_resolution_generation > snapshot.dialect_resolution_generation);
    assert!(!current.dialect_resolution_pending());
    let fresh = backend
        .capture_coarse_semantic_source(&uri, &current)
        .await
        .unwrap()
        .unwrap();
    assert!(coarse_source_is_current(&backend, &fresh).await);
    backend
        .apply_global_config(&serde_json::json!({"dialect":"tcl9.0"}))
        .await
        .unwrap();
    backend.reresolve_open_document_dialects().await.unwrap();
    assert!(
        backend
            .coarse_source_recompute_snapshot(&fresh)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn coarse_recompute_refuses_changed_config_provider_and_source_generation() {
    let mut backend = crate::tests::test_backend();
    let store = Arc::new(crate::vfs::MemoryStore::new());
    let uri: Uri = "file:///coarse-recompute-provider.tcl".parse().unwrap();
    store.upsert_uri_aliases(
        &[uri.as_str()],
        "/coarse-recompute-provider.tcl",
        "puts \"\\x4142\"".as_bytes().to_vec(),
    );
    backend.state_mut_for_test().store = store.clone();
    let snapshot = actual_open_source(&backend, &uri).await;
    let source = backend
        .capture_coarse_semantic_source(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    assert!(coarse_source_is_current(&backend, &source).await);
    backend.invalidate_diag_inputs();
    assert!(
        backend
            .coarse_source_recompute_snapshot(&source)
            .await
            .unwrap()
            .is_some()
    );
    let config = backend.resolved_db_config(&uri).await;
    config
        .set_extra_commands(&mut *backend.db.lock().await)
        .to(vec!["changed_config".to_owned()]);
    assert!(
        backend
            .coarse_source_recompute_snapshot(&source)
            .await
            .unwrap()
            .is_none()
    );
    let refreshed = backend
        .capture_coarse_semantic_source(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    assert!(coarse_source_is_current(&backend, &refreshed).await);
    backend
        .documents
        .lock("source ABA generation")
        .await
        .get_mut(&uri)
        .unwrap()
        .revision += 1;
    assert!(
        backend
            .coarse_source_recompute_snapshot(&refreshed)
            .await
            .unwrap()
            .is_none()
    );
    // Restoring equal bytes cannot undo the source generation change.
    assert_eq!(
        backend.documents.lock("same source bytes").await[&uri].raw(),
        snapshot.raw()
    );
    let current = backend.documents.lock("current provider snapshot").await[&uri].clone();
    let provider_source = backend
        .capture_coarse_semantic_source(&uri, &current)
        .await
        .unwrap()
        .unwrap();
    assert!(coarse_source_is_current(&backend, &provider_source).await);
    backend.invalidate_diag_inputs();
    assert!(
        backend
            .coarse_source_recompute_snapshot(&provider_source)
            .await
            .unwrap()
            .is_some()
    );
    assert!(store.remove_uri(uri.as_str()));
    store.upsert_uri_aliases(
        &[uri.as_str()],
        "/coarse-recompute-provider.tcl",
        "puts \"\\x4142\"".as_bytes().to_vec(),
    );
    assert!(!matches!(
        backend
            .coarse_source_recompute_snapshot(&provider_source)
            .await,
        Ok(Some(_))
    ));
}

#[tokio::test]
async fn coarse_source_keeps_its_inputs_across_an_unrelated_database_write() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///coarse-independent-input.tcl".parse().unwrap();
    let snapshot = actual_open_source(&backend, &uri).await;
    let unrelated = {
        let db = backend.db.lock().await;
        tcl_lsp_db::SourceFile::new(
            &*db,
            "set unrelated 1".to_owned(),
            "tcl8.4".to_owned(),
            Some("/unrelated.tcl".to_owned()),
        )
    };
    let coarse = backend
        .capture_coarse_semantic_source(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    let complete = backend
        .capture_fresh_source_analysis(
            &uri,
            snapshot.clone(),
            FreshSourceAnalysisPurpose::ReadonlyComplete,
        )
        .await
        .unwrap()
        .unwrap();
    let inputs_epoch = backend.diag_inputs_epoch();
    let factory_generation = *backend.class_factory_generation.read().await;
    let admissions = [complete.admission()];
    assert!(
        backend
            .with_current_readonly_source_state(
                inputs_epoch,
                factory_generation,
                &admissions,
                None,
                |state| complete
                    .matches_current_state(state, inputs_epoch, factory_generation)
                    .then_some(()),
                |_, ()| Some(()),
            )
            .await
            .unwrap()
            .is_some()
    );
    let before = tcl_lsp_db::database_revision(&*backend.db.lock().await);
    unrelated
        .set_text(&mut *backend.db.lock().await)
        .to("set unrelated 2".to_owned());
    assert_ne!(
        tcl_lsp_db::database_revision(&*backend.db.lock().await),
        before
    );
    assert!(
        backend
            .with_current_coarse_semantic_source(&coarse, || Some(()))
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        backend
            .with_current_readonly_source_state(
                inputs_epoch,
                factory_generation,
                &admissions,
                None,
                |state| complete
                    .matches_current_state(state, inputs_epoch, factory_generation)
                    .then_some(()),
                |_, ()| Some(()),
            )
            .await
            .unwrap()
            .is_none()
    );
}

async fn set_open_publication(
    backend: &Backend,
    uri: &Uri,
    publication: super::super::DocumentPublication,
) {
    backend
        .documents
        .lock("advance coarse fixture publication")
        .await
        .get_mut(uri)
        .unwrap()
        .publication = publication;
}

async fn coarse_source_is_current(backend: &Backend, source: &CoarseSemanticTokenSource) -> bool {
    backend
        .with_current_coarse_semantic_source(source, || Some(()))
        .await
        .unwrap()
        .is_some()
}

async fn complete_source_is_current(
    backend: &Backend,
    source: &WorkspaceSymbolSourceAnalysis,
    inputs_epoch: u64,
    factory_generation: u64,
) -> bool {
    let admissions = [source.admission()];
    backend
        .with_current_readonly_source_state(
            inputs_epoch,
            factory_generation,
            &admissions,
            None,
            |state| {
                source
                    .matches_current_state(state, inputs_epoch, factory_generation)
                    .then_some(())
            },
            |_, ()| Some(()),
        )
        .await
        .unwrap()
        .is_some()
}

#[tokio::test]
async fn coarse_source_keeps_its_own_revision_when_the_index_finishes() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///coarse-index-progress.tcl".parse().unwrap();
    actual_open_source(&backend, &uri).await;
    let snapshot = {
        let mut docs = backend.documents.lock("initial Salsa source").await;
        let live = docs.get_mut(&uri).unwrap();
        live.publication = super::super::DocumentPublication::Salsa;
        live.clone()
    };
    let coarse = backend
        .capture_coarse_semantic_source(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    let complete = backend
        .capture_fresh_source_analysis(
            &uri,
            snapshot.clone(),
            FreshSourceAnalysisPurpose::ReadonlyComplete,
        )
        .await
        .unwrap()
        .unwrap();
    let inputs_epoch = backend.diag_inputs_epoch();
    let factory_generation = *backend.class_factory_generation.read().await;
    assert!(coarse_source_is_current(&backend, &coarse).await);
    assert!(
        complete_source_is_current(&backend, &complete, inputs_epoch, factory_generation).await
    );
    set_open_publication(&backend, &uri, super::super::DocumentPublication::Indexed).await;
    assert!(coarse_source_is_current(&backend, &coarse).await);
    assert!(
        !complete_source_is_current(&backend, &complete, inputs_epoch, factory_generation).await
    );
    assert_eq!(
        snapshot.publication,
        super::super::DocumentPublication::Salsa
    );
    set_open_publication(&backend, &uri, super::super::DocumentPublication::Pending).await;
    assert!(!coarse_source_is_current(&backend, &coarse).await);
    set_open_publication(&backend, &uri, super::super::DocumentPublication::Indexed).await;
    // Capture the actual indexed publication for the downgrade control.
    let indexed_snapshot = backend
        .documents
        .lock("indexed source snapshot")
        .await
        .get(&uri)
        .unwrap()
        .clone();
    let indexed = backend
        .capture_coarse_semantic_source(&uri, &indexed_snapshot)
        .await
        .unwrap()
        .unwrap();
    assert!(coarse_source_is_current(&backend, &indexed).await);
    set_open_publication(&backend, &uri, super::super::DocumentPublication::Salsa).await;
    assert!(!coarse_source_is_current(&backend, &indexed).await);
    set_open_publication(&backend, &uri, super::super::DocumentPublication::Indexed).await;
    assert!(coarse_source_is_current(&backend, &indexed).await);
    let replacement_snapshot = {
        let mut docs = backend
            .documents
            .lock("equal-byte new source revision")
            .await;
        let live = docs.get_mut(&uri).unwrap();
        live.revision += 1;
        live.clone()
    };
    assert_eq!(replacement_snapshot.text, indexed_snapshot.text);
    assert!(!coarse_source_is_current(&backend, &indexed).await);
    let replacement = backend
        .capture_coarse_semantic_source(&uri, &replacement_snapshot)
        .await
        .unwrap()
        .unwrap();
    assert!(coarse_source_is_current(&backend, &replacement).await);
    let generation = backend.live_publication_uri_generation(&uri);
    backend.invalidate_live_publication(std::iter::once(&uri));
    assert_ne!(backend.live_publication_uri_generation(&uri), generation);
    assert!(!coarse_source_is_current(&backend, &replacement).await);
}

#[tokio::test]
async fn coarse_tokens_refuse_changed_actual_resource_config_without_epoch_donation() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///coarse-config-change.tcl".parse().unwrap();
    let snapshot = actual_open_source(&backend, &uri).await;
    let source = backend
        .capture_coarse_semantic_source(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    assert!(
        backend
            .with_current_coarse_semantic_source(&source, || Some(()))
            .await
            .unwrap()
            .is_some()
    );
    let epoch = backend.diag_inputs_epoch();
    let config = backend.resolved_db_config(&uri).await;
    config
        .set_extra_commands(&mut *backend.db.lock().await)
        .to(vec!["changed_actual_resource_command".to_owned()]);
    assert_eq!(backend.diag_inputs_epoch(), epoch);
    assert!(
        backend
            .with_current_coarse_semantic_source(&source, || -> Option<()> {
                panic!("changed resource configuration must not reach publication")
            })
            .await
            .unwrap()
            .is_none()
    );
    let current = backend
        .capture_coarse_semantic_source(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    assert!(
        backend
            .with_current_coarse_semantic_source(&current, || Some(()))
            .await
            .unwrap()
            .is_some()
    );
}
#[tokio::test]
async fn coarse_source_keeps_factory_and_original_virtual_provider_receipts() {
    let mut backend = crate::tests::test_backend();
    let store = Arc::new(crate::vfs::MemoryStore::new());
    let uri: Uri = "file:///coarse-provider.tcl".parse().unwrap();
    store.upsert_uri(
        uri.as_str(),
        "/coarse-provider.tcl",
        b"puts \"\\x4142\"".to_vec(),
    );
    backend.state_mut_for_test().store = store.clone();
    let snapshot = actual_open_source(&backend, &uri).await;
    let original = backend
        .capture_coarse_semantic_source(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    assert!(
        backend
            .with_current_coarse_semantic_source(&original, || Some(()))
            .await
            .unwrap()
            .is_some()
    );
    *backend.class_factory_generation.write().await += 1;
    assert!(
        backend
            .with_current_coarse_semantic_source(&original, || -> Option<()> {
                panic!("old factory receipt must not publish")
            })
            .await
            .unwrap()
            .is_none()
    );
    let current = backend
        .capture_coarse_semantic_source(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    assert!(
        backend
            .with_current_coarse_semantic_source(&current, || Some(()))
            .await
            .unwrap()
            .is_some()
    );
    assert!(store.remove_uri(uri.as_str()));
    store.upsert_uri(
        uri.as_str(),
        "/coarse-provider.tcl",
        snapshot.raw().as_bytes().to_vec(),
    );
    assert!(
        backend
            .with_current_coarse_semantic_source(&current, || -> Option<()> {
                panic!("equal-path provider ABA must not publish")
            })
            .await
            .is_err()
    );
    assert!(
        backend
            .capture_coarse_semantic_source(&uri, &snapshot)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn closed_path_admission_is_not_an_open_source_publication_receipt() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///coarse-closed-source.tcl".parse().unwrap();
    let snapshot = actual_open_source(&backend, &uri).await;
    assert!(
        backend
            .capture_coarse_semantic_source(&uri, &snapshot)
            .await
            .unwrap()
            .is_some()
    );
    backend
        .documents
        .lock("actual close before coarse capture")
        .await
        .remove(&uri);
    assert!(
        backend
            .capture_coarse_semantic_source(&uri, &snapshot)
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn unresolved_document_dialect_cannot_issue_coarse_input() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///coarse-dirty-dialect.tcl".parse().unwrap();
    let snapshot = actual_open_source(&backend, &uri).await;
    let original = backend
        .capture_coarse_semantic_source(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    let dirty = {
        let mut docs = backend
            .documents
            .lock("actual pending dialect resolution")
            .await;
        let document = docs.get_mut(&uri).unwrap();
        document.mark_dialect_resolution_dirty();
        document.clone()
    };
    assert_ne!(
        dirty.dialect_resolution_generation,
        dirty.resolved_dialect_generation
    );
    assert!(
        backend
            .capture_coarse_semantic_source(&uri, &dirty)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        backend
            .with_current_coarse_semantic_source(&original, || -> Option<()> {
                panic!("previous dialect generation cannot publish")
            })
            .await
            .unwrap()
            .is_none()
    );
}
