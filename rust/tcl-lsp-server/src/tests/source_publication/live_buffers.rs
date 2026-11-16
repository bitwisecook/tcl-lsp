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

//! `didOpen`, `didChange` and `didClose` publication under contention.

use super::*;

/// A `didOpen` waiting for Salsa must release both global choke points:
/// the open-document map *and* the edit-order barrier every request
/// crosses.  Holding either while suspended on a Salsa store wedges the
/// whole server.
///
/// Construct the `db` edge directly. The live buffer must become visible
/// and the turn must settle while the deferred Salsa/index publication is
/// still blocked; releasing `db` then lets that currency-checked commit
/// finish. The parent implementation fails both liveness probes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn did_open_waiting_for_db_leaves_requests_serviceable_1849() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///w/release-wedge-db-1849.tcl").unwrap();
    let other = Uri::from_str("file:///w/unrelated-1849.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        other.clone(),
        DocumentState::new("set unrelated 1\n".to_owned(), "tcl8.6".to_owned()),
    );
    let db_held = backend.db.lock().await;
    let mut opening = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            Box::pin(backend.did_open(DidOpenTextDocumentParams {
                text_document: tower_lsp_server::ls_types::TextDocumentItem {
                    uri,
                    language_id: "tcl".to_owned(),
                    version: 1,
                    text: "set release_ready 1\n".to_owned(),
                },
            }))
            .await;
        }
    });

    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if backend
                .documents
                .lock("issue-1849-wait-visible")
                .await
                .contains_key(&uri)
            {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("didOpen must publish the authoritative live buffer before waiting for Salsa");

    let docs = crate::rt::timeout(
        std::time::Duration::from_millis(250),
        backend.documents.lock("issue-1849-unrelated-reader"),
    )
    .await
    .expect("didOpen waiting for Salsa must leave the open-document map available (#1849)");
    assert!(
        docs.contains_key(&uri),
        "the ordered live-buffer stage must already be visible",
    );
    drop(docs);
    crate::rt::timeout(
        std::time::Duration::from_millis(500),
        backend.edits_settled(),
    )
    .await
    .expect("didOpen waiting for Salsa must not retain the request barrier (#1849)");
    let unrelated = crate::rt::timeout(std::time::Duration::from_millis(500), async {
        backend.read_document(&other).await.unwrap()
    })
    .await
    .expect("a request for another document must remain serviceable (#1849)")
    .expect("the unrelated live document must remain readable");
    assert_eq!(unrelated.text.as_ref(), "set unrelated 1\n");
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), &mut opening)
            .await
            .is_err(),
        "the liveness probes must run while the deferred db publication is still blocked",
    );

    drop(db_held);
    crate::rt::timeout(std::time::Duration::from_secs(30), opening)
        .await
        .expect("didOpen must finish once Salsa is available")
        .expect("didOpen must not panic");
    assert!(
        backend
            .documents
            .lock("issue-1849-assertion")
            .await
            .contains_key(&uri),
        "the final transaction must publish the open document",
    );
}

/// The incident's more likely edge: `files_with_covered_load_targets` held
/// `db_files` while queued on the fair workspace-index reader. Pin that
/// mutex directly and prove the same map + request-barrier liveness as the
/// `db` variant above. The helper now snapshots the URI set before awaiting
/// the index, removing that background lock inversion as well.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn did_open_waiting_for_db_files_leaves_requests_serviceable_1849() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///w/release-wedge-db-files-1849.tcl").unwrap();
    let files_held = backend.db_files.lock().await;
    let mut opening = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            Box::pin(backend.did_open(DidOpenTextDocumentParams {
                text_document: tower_lsp_server::ls_types::TextDocumentItem {
                    uri,
                    language_id: "tcl".to_owned(),
                    version: 1,
                    text: "set release_ready 1\n".to_owned(),
                },
            }))
            .await;
        }
    });

    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if backend
                .documents
                .lock("issue-1849-files-wait-visible")
                .await
                .contains_key(&uri)
            {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("didOpen must publish the live buffer before waiting for db_files");

    let docs = crate::rt::timeout(
        std::time::Duration::from_millis(250),
        backend.documents.lock("issue-1849-files-unrelated-reader"),
    )
    .await
    .expect("didOpen waiting for db_files must leave the document map available (#1849)");
    assert!(docs.contains_key(&uri));
    drop(docs);
    crate::rt::timeout(
        std::time::Duration::from_millis(500),
        backend.edits_settled(),
    )
    .await
    .expect("didOpen waiting for db_files must not retain the request barrier (#1849)");
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), &mut opening)
            .await
            .is_err(),
        "the liveness probes must run while deferred db_files publication is still blocked",
    );

    drop(files_held);
    crate::rt::timeout(std::time::Duration::from_secs(30), opening)
        .await
        .expect("didOpen must finish once db_files is available")
        .expect("didOpen must not panic");
}

/// Once the live buffer is visible, no request
/// may still observe facts indexed from the disk copy it superseded. Hold
/// a real Salsa snapshot so deferred publication cannot finish, then prove
/// `didOpen` queues stale-slot retirement ahead of a later index reader.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn did_open_queues_stale_index_retirement_before_later_readers_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///w/stale-open-index-1854.tcl").unwrap();
    let disk_text = "proc old_disk_name {} {}\n";
    let live_text = "proc live_buffer_name {} {}\n";
    let disk_analysis = Analyser::new().analyse(disk_text, "tcl8.6").clone();
    backend
        .db_set_source(&uri, disk_text, "tcl8.6".into())
        .await
        .unwrap();
    let snapshot = backend
        .db
        .snapshot_from(&*backend.db.lock().await, "held-index")
        .expect("no publisher is draining snapshots");
    let mut index_held = backend.workspace_index.write().await;
    index_held.add_document(uri.as_str(), &disk_analysis);
    let mut opening = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            Box::pin(backend.did_open(DidOpenTextDocumentParams {
                text_document: tower_lsp_server::ls_types::TextDocumentItem {
                    uri,
                    language_id: "tcl".to_owned(),
                    version: 1,
                    text: live_text.to_owned(),
                },
            }))
            .await;
        }
    });

    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let pending = backend
                .documents
                .lock("issue-1854-stale-index-visible")
                .await
                .get(&uri)
                .is_some_and(|doc| !doc.publication.indexed());
            if pending {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the live buffer must become visible while Salsa publication is held");
    crate::rt::timeout(
        std::time::Duration::from_millis(500),
        backend.edits_settled(),
    )
    .await
    .expect("didOpen must release its turn without waiting for Salsa");

    let mut reading_index = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            let index = backend.workspace_index.read().await;
            (
                index.contains_document(uri.as_str()),
                index.proc_definitions("old_disk_name", "other").len(),
            )
        }
    });
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), &mut reading_index,)
            .await
            .is_err(),
        "the post-barrier reader must queue while the old index writer is held"
    );
    drop(index_held);
    let indexed = crate::rt::timeout(std::time::Duration::from_secs(5), reading_index)
        .await
        .expect("the post-barrier index reader must resume")
        .expect("the post-barrier index reader must not panic");
    assert_eq!(
        indexed,
        (false, 0),
        "a post-barrier reader must see neither the disk slot nor its definition"
    );
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), &mut opening)
            .await
            .is_err(),
        "the stale-index assertion must run while Salsa publication is still blocked"
    );

    drop(snapshot);
    crate::rt::timeout(std::time::Duration::from_secs(30), opening)
        .await
        .expect("didOpen must finish when its own database snapshot retires")
        .expect("didOpen must not panic");
    let index = backend.workspace_index.read().await;
    assert_eq!(
        (
            index.proc_definitions("live_buffer_name", "other").len(),
            index.proc_definitions("old_disk_name", "other").len(),
        ),
        (1, 0),
        "the completed open must seed only the live cross-document view"
    );
}

/// Snapshot safety is scoped to one Salsa database. A query in one client
/// session must never stall live publication in another session merely
/// because both backends happen to live in the same server process.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unrelated_backend_snapshot_does_not_block_live_publication_1854() {
    let holding_backend = Arc::new(test_backend());
    let publishing_backend = Arc::new(test_backend());
    let snapshot = {
        let db = holding_backend.db.lock().await;
        holding_backend
            .db
            .snapshot_from(&db, "issue-1854-unrelated-backend-snapshot")
            .expect("no publisher is draining the holding backend")
    };
    let uri = Uri::from_str("file:///w/isolated-census-1854.tcl").unwrap();

    Box::pin(crate::rt::timeout(
        std::time::Duration::from_secs(5),
        publishing_backend.did_open(DidOpenTextDocumentParams {
            text_document: tower_lsp_server::ls_types::TextDocumentItem {
                uri: uri.clone(),
                language_id: "tcl".to_owned(),
                version: 1,
                text: "proc independently_published {} {}\n".to_owned(),
            },
        }),
    ))
    .await
    .expect("an unrelated backend's snapshot must not delay didOpen");

    let db = publishing_backend.db.lock().await;
    let files = publishing_backend.db_files.lock().await;
    assert_eq!(
        files.get(&uri).map(|file| file.text(&*db).as_str()),
        Some("proc independently_published {} {}\n"),
        "the publishing backend must install its live source"
    );
    drop(files);
    drop(db);
    drop(snapshot);
}

/// The no-wait-under-turn rule applies to edits too. The
/// authoritative splice must become visible and release `edits_settled`
/// while its deferred Salsa publication is pinned on `db`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn did_change_waiting_for_db_leaves_requests_serviceable_1849() {
    use tower_lsp_server::ls_types::{
        TextDocumentContentChangeEvent, VersionedTextDocumentIdentifier,
    };

    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///w/release-wedge-change-1849.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version("set release_ready 1\n".to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("tcl".to_owned()),
    );
    backend
        .db_set_source(&uri, "set release_ready 1\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    let diag_inputs = backend
        .diag_inputs(&uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
        .await;

    let db_held = backend.db.lock().await;
    let mut editing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend
                .did_change(DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier { uri, version: 2 },
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: None,
                        range_length: None,
                        text: "set release_ready 2\n".to_owned(),
                    }],
                })
                .await;
        }
    });

    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if backend
                .documents
                .lock("issue-1849-change-wait-visible")
                .await
                .get(&uri)
                .is_some_and(|doc| doc.text.as_ref() == "set release_ready 2\n")
            {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("didChange must publish its authoritative splice before waiting for Salsa");
    crate::rt::timeout(
        std::time::Duration::from_millis(500),
        backend.edits_settled(),
    )
    .await
    .expect("didChange waiting for Salsa must not retain the request barrier (#1849)");
    assert!(
        diag_inputs.capture_job(&uri).await.is_none(),
        "diagnostics must not pair the pending live revision with the preceding Salsa source",
    );
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), &mut editing)
            .await
            .is_err(),
        "the liveness probe must run while the Salsa publication remains blocked",
    );

    let mut reading_current = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move { backend.read_document(&uri).await.unwrap() }
    });
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), &mut reading_current,)
            .await
            .is_err(),
        "a same-document request must not observe text ahead of Salsa publication",
    );

    drop(db_held);
    crate::rt::timeout(std::time::Duration::from_secs(30), editing)
        .await
        .expect("didChange must finish once Salsa is available")
        .expect("didChange must not panic");
    let current = crate::rt::timeout(std::time::Duration::from_secs(30), reading_current)
        .await
        .expect("the same-document request must resume after publication")
        .expect("the same-document request task must not panic")
        .expect("the edited document must remain open");
    assert_eq!(current.text.as_ref(), "set release_ready 2\n");
    let published_job = diag_inputs
        .capture_job(&uri)
        .await
        .expect("diagnostics capture resumes after matching source publication");
    assert_eq!(published_job.text.as_ref(), "set release_ready 2\n");
    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    assert_eq!(
        files.get(&uri).map(|file| file.text(&*db).as_str()),
        Some("set release_ready 2\n"),
        "the deferred publication must install the edited revision",
    );
}

/// Releasing the edit turn is insufficient if
/// a deferred Salsa setter enters `cancel_others` while it owns db/files.
/// Hold a real tracked snapshot, start an edit, and prove the publication
/// waits with the entire bundle released. A path that blocks inside the
/// setter deterministically times out acquiring both stores here.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn did_change_waiting_for_snapshot_releases_salsa_stores_1854() {
    use tower_lsp_server::ls_types::{
        TextDocumentContentChangeEvent, VersionedTextDocumentIdentifier,
    };

    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///w/release-wedge-snapshot-1854.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version("set release_ready 1\n".to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("tcl".to_owned()),
    );
    backend
        .db_set_source(&uri, "set release_ready 1\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    // Production snapshots are handed directly to a worker. Retaining one
    // here is deliberate: it makes Salsa's setter precondition false until
    // this test says otherwise.
    let snapshot = {
        let db = backend.db.lock().await;
        backend
            .db
            .snapshot_from(&db, "issue-1854-held-test-snapshot")
            .expect("the test creates its snapshot before publication")
    };
    let mut editing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend
                .did_change(DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier { uri, version: 2 },
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: None,
                        range_length: None,
                        text: "set release_ready 2\n".to_owned(),
                    }],
                })
                .await;
        }
    });

    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let pending = backend
                .documents
                .lock("issue-1854-snapshot-wait-visible")
                .await
                .get(&uri)
                .is_some_and(|doc| {
                    doc.text.as_ref() == "set release_ready 2\n"
                        && doc.publication != DocumentPublication::Indexed
                });
            if pending && backend.live_publication_gate.try_lock().is_err() {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("didChange must reach deferred publication with its live revision pending");

    let db = crate::rt::timeout(std::time::Duration::from_millis(250), backend.db.lock())
        .await
        .expect("a live setter waiting for snapshots must release db (#1854)");
    drop(db);
    let files = crate::rt::timeout(
        std::time::Duration::from_millis(250),
        backend.db_files.lock(),
    )
    .await
    .expect("a live setter waiting for snapshots must release db_files (#1854)");
    drop(files);
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), &mut editing)
            .await
            .is_err(),
        "the edit must remain pending until its snapshot retires",
    );

    drop(snapshot);
    crate::rt::timeout(std::time::Duration::from_secs(30), editing)
        .await
        .expect("didChange must finish once every Salsa snapshot retires")
        .expect("didChange must not panic");
    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    assert_eq!(
        files.get(&uri).map(|file| file.text(&*db).as_str()),
        Some("set release_ready 2\n"),
        "the deferred setter must publish after the census reaches zero",
    );
}

/// A close blocked on the workspace index must likewise
/// remove the live buffer and release the request barrier first.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn did_close_waiting_for_index_leaves_requests_serviceable_1849() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///w/release-wedge-close-1849.tcl").unwrap();
    register(&backend, &uri, "set release_ready 1\n").await;

    let index_held = backend.workspace_index.write().await;
    let mut closing = crate::rt::spawn({
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
            if !backend
                .documents
                .lock("issue-1849-close-wait-removed")
                .await
                .contains_key(&uri)
            {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("didClose must remove the live buffer before waiting for the index");
    crate::rt::timeout(
        std::time::Duration::from_millis(500),
        backend.edits_settled(),
    )
    .await
    .expect("didClose waiting for the index must not retain the request barrier (#1849)");
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), &mut closing)
            .await
            .is_err(),
        "the liveness probe must run while index removal remains blocked",
    );

    drop(index_held);
    crate::rt::timeout(std::time::Duration::from_secs(30), closing)
        .await
        .expect("didClose must finish once the index is available")
        .expect("didClose must not panic");
}

/// A close tail superseded by a reopen cannot clear the old lifecycle's
/// token caches itself. The reopen clears them before advertising Salsa
/// readiness, so its first coarse/enriched comparison starts clean.
#[tokio::test]
async fn reopen_clears_prior_semantic_token_lifecycle_1854() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///reopen-token-cache-1854.tcl").unwrap();
    let text = "proc current {} {}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(text.to_owned(), "tcl8.6".to_owned(), 2)
            .with_language_id("tcl".to_owned())
            .with_publication_pending(),
    );
    backend
        .last_semantic_tokens
        .lock()
        .await
        .insert(uri.clone(), ("old".to_owned(), vec![1, 2, 3]));
    backend
        .semantic_tokens_refresh_asked
        .lock()
        .await
        .insert(uri.clone(), 42);
    let analysis = Arc::new(Analyser::new().analyse(text, "tcl8.6").clone());
    let analyser_inputs_epoch = backend.diag_inputs_epoch();
    let path_admission = backend.document_path_admission(&uri).await.unwrap();

    assert!(
        backend
            .commit_open_document_with_seed(&uri, text, "tcl8.6", 2, async move {
                Ok(FreshAnalysisSeed {
                    path_admission,
                    analysis,
                    analyser_inputs_epoch,
                    class_factory_generation: 0,
                })
            })
            .await
            .unwrap()
    );
    assert!(!backend.last_semantic_tokens.lock().await.contains_key(&uri));
    assert!(
        !backend
            .semantic_tokens_refresh_asked
            .lock()
            .await
            .contains_key(&uri)
    );
}
