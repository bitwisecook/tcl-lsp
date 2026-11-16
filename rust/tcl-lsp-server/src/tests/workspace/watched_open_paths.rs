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

//! Watched deletes and creates of paths that are open in the editor.

use super::*;

async fn mark_open_buffer_deleted_while_publication_waits_1854(
    backend: &Arc<Backend>,
    uri: &Uri,
) -> crate::rt::JoinHandle<()> {
    let deleting = crate::rt::spawn({
        let backend = Arc::clone(backend);
        let uri = uri.clone();
        async move {
            backend
                .did_change_watched_files(DidChangeWatchedFilesParams {
                    changes: vec![tower_lsp_server::ls_types::FileEvent {
                        uri,
                        typ: FileChangeType::DELETED,
                    }],
                })
                .await;
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let orphaned = backend
                .documents
                .lock("test")
                .await
                .get(uri)
                .is_some_and(|doc| doc.backing_file_deleted);
            if orphaned {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the watcher must mark the open buffer as an orphan");
    deleting
}

async fn wait_for_salsa_source_text_1854(backend: &Backend, uri: &Uri, expected: &str) {
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let current = {
                let db = backend.db.lock().await;
                let files = backend.db_files.lock().await;
                files
                    .get(uri)
                    .is_some_and(|file| file.text(&*db).as_str() == expected)
            };
            if current {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the expected text must reach Salsa");
}

async fn assert_orphaned_publication_stays_retired_1854(
    backend: &Backend,
    uri: &Uri,
    text: &str,
    label: &str,
    publishing: crate::rt::JoinHandle<bool>,
    deleting: crate::rt::JoinHandle<()>,
) {
    assert!(
        !crate::rt::timeout(std::time::Duration::from_secs(5), publishing)
            .await
            .expect("the obsolete source publisher must finish")
            .expect("the obsolete source publisher must not panic"),
        "the {label} publisher must reject the orphan",
    );
    crate::rt::timeout(std::time::Duration::from_secs(5), deleting)
        .await
        .expect("the watched deletion must finish")
        .expect("the watched deletion must not panic");

    let current = crate::rt::timeout(std::time::Duration::from_millis(500), async {
        backend.read_document(uri).await.unwrap()
    })
    .await
    .expect("an orphaned open buffer must not wait for a rejected index seed")
    .expect("the orphaned open buffer remains locally readable");
    assert_eq!(current.text.as_ref(), text);
    assert_eq!(current.publication, DocumentPublication::Indexed);
    assert!(current.backing_file_deleted);
    assert!(!backend.db_files.lock().await.contains_key(uri));
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .contains_document(uri.as_str()),
        "the {label} publisher must not resurrect the deleted index slot",
    );
}

#[derive(Clone, Copy)]
enum OrphanedPublicationKind {
    Open,
    Change,
    Dialect,
}

/// Watched deletion must not wake a
/// pending live buffer at Salsa readiness while the database still names
/// the preceding revision. It becomes locally readable only after the
/// deletion transaction retires that stale handle.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn watched_delete_keeps_pending_buffer_below_salsa_readiness_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///deleted-pending-source-1854.tcl").unwrap();
    let stale = "proc stale {} {}\n";
    let current = "proc current {} {}\n";
    backend
        .db_set_source(&uri, stale, "tcl8.6".to_owned())
        .await
        .unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(current.to_owned(), "tcl8.6".to_owned(), 1)
            .with_publication_pending(),
    );

    let database = backend.db.lock().await;
    let deleting = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend
                .did_change_watched_files(DidChangeWatchedFilesParams {
                    changes: vec![tower_lsp_server::ls_types::FileEvent {
                        uri,
                        typ: FileChangeType::DELETED,
                    }],
                })
                .await;
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let orphaned = backend
                .documents
                .lock("test")
                .await
                .get(&uri)
                .is_some_and(|doc| doc.backing_file_deleted);
            if orphaned {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the watched delete must mark the buffer orphaned");
    assert_eq!(
        backend
            .documents
            .lock("test")
            .await
            .get(&uri)
            .expect("the open buffer remains present")
            .publication,
        DocumentPublication::Pending,
        "the stale Salsa handle must not be labelled ready",
    );
    assert!(
        crate::rt::timeout(
            std::time::Duration::from_millis(100),
            backend.read_local_document(&uri),
        )
        .await
        .is_err(),
        "local providers must wait while the stale handle remains",
    );

    drop(database);
    crate::rt::timeout(std::time::Duration::from_secs(5), deleting)
        .await
        .expect("the deletion transaction must finish after db is released")
        .expect("the deletion task must not panic");
    assert!(!backend.db_files.lock().await.contains_key(&uri));
    let readable = crate::rt::timeout(std::time::Duration::from_secs(5), async {
        backend.read_local_document(&uri).await.unwrap()
    })
    .await
    .expect("the orphan must become readable after stale Salsa retirement")
    .expect("the orphaned live buffer remains present");
    assert_eq!(readable.text.as_ref(), current);
    assert_eq!(readable.publication, DocumentPublication::Indexed);
}

/// A watched deletion can overtake
/// any deferred live-source publisher before its Salsa setter. didOpen,
/// didChange, and dialect publication must all reject the orphan rather
/// than recreate its source and move settled readiness back to Salsa.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn watched_delete_cancels_queued_live_source_publications_1854() {
    for (label, kind) in [
        ("open", OrphanedPublicationKind::Open),
        ("change", OrphanedPublicationKind::Change),
        ("dialect", OrphanedPublicationKind::Dialect),
    ] {
        let backend = Arc::new(test_backend());
        let uri = Uri::from_str(&format!("file:///deleted-{label}-source-1854.tcl")).unwrap();
        let text = "proc ghost {} {}\n";
        backend
            .db_set_source(&uri, text, "tcl8.6".to_owned())
            .await
            .unwrap();
        let analysis = {
            let mut analyser = Analyser::new();
            Arc::new(analyser.analyse(text, "tcl8.6").clone())
        };
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &analysis);
        backend.documents.lock("test").await.insert(
            uri.clone(),
            DocumentState::with_version(text.to_owned(), "tcl9.0".to_owned(), 1)
                .with_language_id("tcl".to_owned())
                .with_publication_pending(),
        );

        let db_held = backend.db.lock().await;
        let publishing =
            crate::rt::spawn({
                let backend = Arc::clone(&backend);
                let uri = uri.clone();
                let analyser_inputs_epoch = backend.diag_inputs_epoch();
                let seed = FreshAnalysisSeed {
                    path_admission: Arc::new(
                        DocumentPathAdmission::capture(&uri, backend.store.as_ref()).unwrap(),
                    ),
                    analysis,
                    analyser_inputs_epoch,
                    class_factory_generation: 0,
                };
                async move {
                    match kind {
                        OrphanedPublicationKind::Open => backend
                            .commit_open_document_with_seed(&uri, text, "tcl9.0", 1, async move {
                                Ok(seed)
                            })
                            .await
                            .unwrap(),
                        OrphanedPublicationKind::Change => backend
                            .commit_live_source("test_change", &uri, text, "tcl9.0", 0, true)
                            .await
                            .unwrap(),
                        OrphanedPublicationKind::Dialect => backend
                            .commit_live_dialect("test_dialect", &uri, text, "tcl9.0", 0)
                            .await
                            .unwrap(),
                    }
                }
            });
        crate::rt::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if backend.live_publication_gate.try_lock().is_err() {
                    break;
                }
                crate::rt::yield_now().await;
            }
        })
        .await
        .expect("the source publisher must wait behind the held database");

        let deleting = mark_open_buffer_deleted_while_publication_waits_1854(&backend, &uri).await;
        drop(db_held);
        assert_orphaned_publication_stays_retired_1854(
            &backend, &uri, text, label, publishing, deleting,
        )
        .await;
    }
}

/// A watched deletion can overtake
/// the off-Salsa cold-open seed. Its orphan mark must make the queued seed
/// fail currency instead of resurrecting the dead path in the index.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn watched_delete_cancels_a_queued_open_index_seed_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///deleted-open-seed-1854.tcl").unwrap();
    let text = "proc ghost {} {}\n";
    backend
        .db_set_source(&uri, text, "tcl8.6".to_owned())
        .await
        .unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(text.to_owned(), "tcl8.6".to_owned(), 1),
    );
    backend
        .documents
        .lock("test")
        .await
        .get_mut(&uri)
        .expect("the open document exists")
        .publication = DocumentPublication::Salsa;
    let seed = backend
        .fresh_analysis_seed(&uri, Arc::from(text), "tcl8.6".to_owned())
        .await
        .unwrap();

    let index_reader = backend.workspace_index.read().await;
    let publishing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend
                .publish_open_index_if_current(&uri, text, "tcl8.6", 1, seed)
                .await
                .unwrap()
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if backend.workspace_index.try_read().is_err() {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the cold-open seed must queue behind the held index reader");

    let deleting = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend
                .did_change_watched_files(DidChangeWatchedFilesParams {
                    changes: vec![tower_lsp_server::ls_types::FileEvent {
                        uri,
                        typ: FileChangeType::DELETED,
                    }],
                })
                .await;
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let orphaned = backend
                .documents
                .lock("test")
                .await
                .get(&uri)
                .is_some_and(|doc| {
                    doc.backing_file_deleted && doc.publication == DocumentPublication::Indexed
                });
            if orphaned {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the watcher must settle the deleted open buffer as an orphan");
    drop(index_reader);

    assert!(
        !crate::rt::timeout(std::time::Duration::from_secs(5), publishing)
            .await
            .expect("the obsolete seed must finish after the index reader drains")
            .expect("the obsolete seed must not panic"),
        "the orphan mark must reject the cold-open seed",
    );
    crate::rt::timeout(std::time::Duration::from_secs(5), deleting)
        .await
        .expect("the watched deletion must finish")
        .expect("the watched deletion must not panic");
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .contains_document(uri.as_str()),
        "the deleted path must remain absent from the workspace index",
    );
}

/// A watched deletion deliberately
/// settles an open orphan with no cross-document view. If that path is
/// created again, the authoritative editor buffer must republish both its
/// Salsa source and index even when diagnostics cannot repair them later.
#[tokio::test]
async fn watched_create_republishes_a_revived_open_buffer_1854() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///revived-open-buffer-1854.tcl").unwrap();
    let text = "proc revived {} {}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(text.to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("tcl".to_owned()),
    );
    backend
        .db_set_source(&uri, text, "tcl8.6".to_owned())
        .await
        .unwrap();
    let analysis = Arc::new(Analyser::new().analyse(text, "tcl8.6").clone());
    backend
        .workspace_index
        .write()
        .await
        .replace_document(uri.as_str(), &analysis);

    backend
        .did_change_watched_files(DidChangeWatchedFilesParams {
            changes: vec![tower_lsp_server::ls_types::FileEvent {
                uri: uri.clone(),
                typ: FileChangeType::DELETED,
            }],
        })
        .await;
    let orphan_revision = {
        let docs = backend.documents.lock("test").await;
        let doc = docs.get(&uri).expect("the open orphan remains tracked");
        assert!(doc.backing_file_deleted);
        assert_eq!(doc.publication, DocumentPublication::Indexed);
        doc.revision
    };
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .contains_document(uri.as_str()),
        "the watched deletion retires the orphan's index view",
    );

    backend
        .did_change_watched_files(DidChangeWatchedFilesParams {
            changes: vec![tower_lsp_server::ls_types::FileEvent {
                uri: uri.clone(),
                typ: FileChangeType::CREATED,
            }],
        })
        .await;
    let docs = backend.documents.lock("test").await;
    let revived = docs
        .get(&uri)
        .expect("the revived open buffer remains tracked");
    assert!(!revived.backing_file_deleted);
    assert_eq!(revived.publication, DocumentPublication::Indexed);
    assert!(
        revived.revision > orphan_revision,
        "revival must invalidate every pre-deletion publisher",
    );
    drop(docs);
    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    assert_eq!(
        files.get(&uri).map(|file| file.text(&*db).as_str()),
        Some(text),
        "the revived editor buffer must be restored to Salsa",
    );
    drop(files);
    drop(db);
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .workspace_command_exists("::revived"),
        "the revived editor buffer must regain its workspace-index facts",
    );
}

/// A disk notification for an
/// already-live open buffer is a no-op because the editor buffer remains
/// authoritative. It must not retire a contended publisher without
/// starting a replacement, especially when diagnostics cannot repair it.
#[tokio::test]
async fn watched_change_preserves_live_open_publication_currency_1854() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///live-watched-change-1854.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(
            "proc still_pending {} {}\n".to_owned(),
            "tcl8.6".to_owned(),
            1,
        )
        .with_language_id("tcl".to_owned())
        .with_publication_pending(),
    );
    backend.feature_toggles.lock().await.apply(
        serde_json::json!({ "diagnostics": false })
            .as_object()
            .unwrap(),
    );
    let global_before = backend
        .live_publication_generation
        .load(std::sync::atomic::Ordering::Acquire);
    let uri_before = backend.live_publication_uri_generation(&uri);

    backend
        .did_change_watched_files(DidChangeWatchedFilesParams {
            changes: vec![tower_lsp_server::ls_types::FileEvent {
                uri: uri.clone(),
                typ: FileChangeType::CHANGED,
            }],
        })
        .await;

    assert_eq!(
        backend
            .live_publication_generation
            .load(std::sync::atomic::Ordering::Acquire),
        global_before,
        "a no-op open-buffer disk event must not wake every publisher",
    );
    assert_eq!(
        backend.live_publication_uri_generation(&uri),
        uri_before,
        "a no-op open-buffer disk event must not obsolete its publisher",
    );
    let docs = backend.documents.lock("test").await;
    let doc = docs.get(&uri).expect("the open buffer remains tracked");
    assert!(!doc.backing_file_deleted);
    assert_eq!(doc.publication, DocumentPublication::Pending);
}

/// Orphan filtering must cover indexed
/// hits during the interval after the watcher marks the buffer deleted but
/// before its transactional index removal can acquire the rehoming gate.
#[tokio::test]
async fn workspace_symbols_exclude_orphan_during_disk_removal_wait_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///orphan-index-window-1854.tcl").unwrap();
    let text = "proc obsolete_disk_symbol {} {}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(text.to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("tcl".to_owned()),
    );
    backend
        .db_set_source(&uri, text, "tcl8.6".to_owned())
        .await
        .unwrap();
    let analysis = Analyser::new().analyse(text, "tcl8.6").clone();
    backend
        .workspace_index
        .write()
        .await
        .replace_document(uri.as_str(), &analysis);
    backend.feature_toggles.lock().await.apply(
        serde_json::json!({ "diagnostics": false })
            .as_object()
            .unwrap(),
    );

    let rehoming = backend.rehoming_gate.lock().await;
    let deleting = mark_open_buffer_deleted_while_publication_waits_1854(&backend, &uri).await;
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .contains_document(uri.as_str()),
        "the regression must query inside the pre-removal index window",
    );
    let symbols = backend
        .open_but_unindexed_symbols("obsolete_disk_symbol", 10)
        .await
        .unwrap();
    assert!(
        symbols.fallback_hits.is_empty() && symbols.indexed_hits.is_empty(),
        "neither symbol channel may expose the orphan while removal waits",
    );

    drop(rehoming);
    crate::rt::timeout(std::time::Duration::from_secs(5), deleting)
        .await
        .expect("the watched deletion must finish after the gate is released")
        .expect("the watched deletion task must not panic");
}

/// Deleting an open path retires its
/// cross-document identity, not its editor buffer. A subsequent didChange
/// must publish the new bytes to Salsa and settle local-provider readiness
/// while leaving the orphan absent from the workspace index.
#[tokio::test]
async fn post_deletion_edit_remains_locally_publishable_1854() {
    use tower_lsp_server::ls_types::{
        TextDocumentContentChangeEvent, VersionedTextDocumentIdentifier,
    };

    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///post-deletion-edit-1854.tcl").unwrap();
    let initial = "proc before_delete {} {}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(initial.to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("tcl".to_owned()),
    );
    backend
        .db_set_source(&uri, initial, "tcl8.6".to_owned())
        .await
        .unwrap();
    let analysis = Arc::new(Analyser::new().analyse(initial, "tcl8.6").clone());
    backend
        .workspace_index
        .write()
        .await
        .replace_document(uri.as_str(), &analysis);
    backend.feature_toggles.lock().await.apply(
        serde_json::json!({ "diagnostics": false })
            .as_object()
            .unwrap(),
    );

    let rehoming = backend.rehoming_gate.lock().await;
    let deleting = mark_open_buffer_deleted_while_publication_waits_1854(&backend, &uri).await;

    let edited = "# tcl-dialect: tcl9.0\nproc after_delete {} {}\n";
    backend
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: edited.to_owned(),
            }],
        })
        .await;

    drop(rehoming);
    crate::rt::timeout(std::time::Duration::from_secs(5), deleting)
        .await
        .expect("the deletion must finish after its disk commit is released")
        .expect("the deletion task must not panic");

    let current = crate::rt::timeout(std::time::Duration::from_millis(500), async {
        backend.read_document(&uri).await.unwrap()
    })
    .await
    .expect("local providers must not wedge after editing an orphan")
    .expect("the orphaned buffer remains open");
    assert_eq!(current.text.as_ref(), edited);
    assert_eq!(current.dialect, "tcl9.0");
    assert!(current.backing_file_deleted);
    assert_eq!(current.publication, DocumentPublication::Indexed);
    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    let orphan_file = files.get(&uri).copied();
    assert_eq!(
        orphan_file.map(|file| file.text(&*db).as_str()),
        Some(edited),
        "the post-deletion revision must be the Salsa source",
    );
    assert_eq!(
        orphan_file.map(|file| file.dialect(&*db).as_str()),
        Some("tcl9.0"),
        "the post-deletion dialect revision must be current in Salsa",
    );
    let project = *backend.db_project.lock().await;
    assert!(
        orphan_file.is_some_and(|file| {
            project.is_some_and(|project| !project.files(&*db).contains(&file))
        }),
        "the local orphan handle must remain outside Salsa project membership",
    );
    drop(files);
    drop(db);
    assert!(
        !backend.db_project_members.lock().await.contains(&uri),
        "the dead path must not be admitted to the cross-document project",
    );
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .contains_document(uri.as_str()),
        "the edited orphan must not regain a cross-document identity",
    );
    let symbols = backend
        .open_but_unindexed_symbols("after_delete", 10)
        .await
        .unwrap();
    assert!(
        symbols.fallback_hits.is_empty() && symbols.indexed_hits.is_empty(),
        "workspace/symbol fallback must not resurrect an edited orphan",
    );
}

/// An orphan remains locally live
/// when an in-source or configuration change re-resolves its dialect. The
/// new dialect must reach Salsa readiness without restoring any project or
/// workspace-index identity for the missing path.
#[tokio::test]
async fn orphan_dialect_change_republishes_only_local_salsa_state_1854() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///orphan-dialect-change-1854.tcl").unwrap();
    let text = "proc orphan_local {} {}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(text.to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("tcl".to_owned()),
    );
    backend
        .db_set_source(&uri, text, "tcl8.6".to_owned())
        .await
        .unwrap();
    backend.feature_toggles.lock().await.apply(
        serde_json::json!({ "diagnostics": false })
            .as_object()
            .unwrap(),
    );
    backend
        .did_change_watched_files(DidChangeWatchedFilesParams {
            changes: vec![tower_lsp_server::ls_types::FileEvent {
                uri: uri.clone(),
                typ: FileChangeType::DELETED,
            }],
        })
        .await;

    *backend.default_dialect.lock().await = "tcl9.0".to_owned();
    *backend.default_dialect_explicit.lock().await = true;
    backend
        .documents
        .lock("test")
        .await
        .get_mut(&uri)
        .expect("the orphan remains open")
        .mark_dialect_resolution_dirty();
    backend
        .resolve_dirty_open_document_dialect(&uri, "test_orphan_dialect")
        .await
        .unwrap();

    let current = crate::rt::timeout(std::time::Duration::from_millis(500), async {
        backend.read_document(&uri).await.unwrap()
    })
    .await
    .expect("local readiness must settle after an orphan dialect change")
    .expect("the orphan remains readable");
    assert_eq!(current.dialect, "tcl9.0");
    assert!(current.backing_file_deleted);
    assert_eq!(current.publication, DocumentPublication::Indexed);

    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    let file = *files
        .get(&uri)
        .expect("the dialect publisher restores a local Salsa handle");
    assert_eq!(file.dialect(&*db), "tcl9.0");
    assert!(
        backend
            .db_project
            .lock()
            .await
            .is_some_and(|project| !project.files(&*db).contains(&file)),
        "an orphan dialect change must remain outside project membership",
    );
    drop(files);
    drop(db);
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .contains_document(uri.as_str()),
        "an orphan dialect change must not restore workspace-index facts",
    );
}

/// A watched delete can mark an old
/// open revision, stall at the disk-publication gate, and then be overtaken
/// by didClose plus a new didOpen. Its final removal must currency-check the
/// open identity rather than deleting the reopened Salsa source.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stale_watched_delete_preserves_reopened_source_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///reopened-during-delete-1854.tcl").unwrap();
    let initial = "proc stale_open {} {}\n";
    let reopened = "proc authoritative_reopen {} {}\n";
    register(&backend, &uri, initial).await;
    backend
        .db_set_source(&uri, initial, "tcl8.6".to_owned())
        .await
        .unwrap();

    let rehoming = backend.rehoming_gate.lock().await;
    let deleting = mark_open_buffer_deleted_while_publication_waits_1854(&backend, &uri).await;

    let closing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend
                .did_close(DidCloseTextDocumentParams {
                    text_document: TextDocumentIdentifier { uri },
                })
                .await;
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if !backend.documents.lock("test").await.contains_key(&uri) {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("didClose must retire the old open identity");

    let opening = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            Box::pin(backend.did_open(DidOpenTextDocumentParams {
                text_document: tower_lsp_server::ls_types::TextDocumentItem {
                    uri,
                    language_id: "tcl".to_owned(),
                    version: 2,
                    text: reopened.to_owned(),
                },
            }))
            .await;
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let reopened_is_visible = backend
                .documents
                .lock("test")
                .await
                .get(&uri)
                .is_some_and(|doc| !doc.backing_file_deleted && doc.text.as_ref() == reopened);
            if reopened_is_visible {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the new didOpen identity must become authoritative");

    wait_for_salsa_source_text_1854(&backend, &uri, reopened).await;

    drop(rehoming);
    for (label, task) in [
        ("deletion", deleting),
        ("close", closing),
        ("reopen", opening),
    ] {
        crate::rt::timeout(std::time::Duration::from_secs(30), task)
            .await
            .unwrap_or_else(|_| panic!("the {label} task must finish"))
            .unwrap_or_else(|err| panic!("the {label} task must not panic: {err}"));
    }

    let current = backend
        .read_document(&uri)
        .await
        .unwrap()
        .expect("the reopened document remains readable");
    assert_eq!(current.text.as_ref(), reopened);
    assert!(!current.backing_file_deleted);
    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    assert_eq!(
        files.get(&uri).map(|file| file.text(&*db).as_str()),
        Some(reopened),
        "the stale watched deletion must not remove the reopened Salsa source",
    );
    drop(files);
    drop(db);
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .workspace_command_exists("::authoritative_reopen"),
        "the reopened document must retain its current index view",
    );
}

/// Watched handlers are concurrent,
/// so an older delete must finish its Salsa/index removal before a later
/// create republishes the same still-open buffer.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn watched_delete_transaction_precedes_later_revive_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///ordered-watched-revive-1854.tcl").unwrap();
    let text = "proc ordered_revive {} {}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(text.to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("tcl".to_owned()),
    );
    backend
        .db_set_source(&uri, text, "tcl8.6".to_owned())
        .await
        .unwrap();
    let analysis = Arc::new(Analyser::new().analyse(text, "tcl8.6").clone());
    backend
        .workspace_index
        .write()
        .await
        .replace_document(uri.as_str(), &analysis);

    let rehoming = backend.rehoming_gate.lock().await;
    let deletion = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend
                .did_change_watched_files(DidChangeWatchedFilesParams {
                    changes: vec![tower_lsp_server::ls_types::FileEvent {
                        uri,
                        typ: FileChangeType::DELETED,
                    }],
                })
                .await;
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let orphaned = backend
                .documents
                .lock("test")
                .await
                .get(&uri)
                .is_some_and(|doc| doc.backing_file_deleted);
            if orphaned {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the deletion must mark the open buffer before its removal blocks");
    assert!(
        backend.watched_file_change_gate.try_lock().is_err(),
        "the deletion must retain the watched transaction gate",
    );

    let revival = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend
                .did_change_watched_files(DidChangeWatchedFilesParams {
                    changes: vec![tower_lsp_server::ls_types::FileEvent {
                        uri,
                        typ: FileChangeType::CREATED,
                    }],
                })
                .await;
        }
    });
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(25), async {
            while !revival.is_finished() {
                crate::rt::yield_now().await;
            }
        })
        .await
        .is_err(),
        "the later create must wait for the older deletion transaction",
    );

    drop(rehoming);
    crate::rt::timeout(std::time::Duration::from_secs(5), deletion)
        .await
        .expect("the deletion must finish after rehoming is released")
        .expect("the deletion task must not panic");
    crate::rt::timeout(std::time::Duration::from_secs(5), revival)
        .await
        .expect("the later create must finish after the deletion")
        .expect("the revival task must not panic");

    let docs = backend.documents.lock("test").await;
    let revived = docs.get(&uri).expect("the open buffer remains tracked");
    assert!(!revived.backing_file_deleted);
    assert_eq!(revived.publication, DocumentPublication::Indexed);
    drop(docs);
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .workspace_command_exists("::ordered_revive"),
        "the older deletion must not remove the later revived index view",
    );
}
