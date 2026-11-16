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

//! The open document's index seed: publication, rollback and retries.

use super::*;

/// An edit must not evict the edited document from the workspace index.
/// `publish_diagnostics_result` re-indexes it (remove + add) behind its own
/// `is_current` check, so a per-keystroke removal buys nothing but a window
/// in which the file contributes no procs, classes or call sites to any
/// cross-file query — on top of fourteen workspace-wide `retain` passes
/// per edit.
///
/// Pinned the same way as the turn-handover test above: `diag_slots` is
/// held, so the handler is parked in its tail and no publish can have run by the
/// time the assertion reads the index.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn did_change_does_not_evict_the_document_from_the_workspace_index() {
    use tower_lsp_server::ls_types::{
        TextDocumentContentChangeEvent, VersionedTextDocumentIdentifier,
    };
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///w/indexed.tcl").unwrap();
    register(&backend, &uri, "proc published {} {}\n").await;

    let slots_held = backend.diag_slots.lock().await;
    let editing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend
                .did_change(DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier { uri, version: 2 },
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: None,
                        range_length: None,
                        text: "proc renamed {} {}\n".to_owned(),
                    }],
                })
                .await;
        }
    });

    assert!(
        one_edit_turn_released(&backend).await,
        "did_change must release its turn before scheduling diagnostics",
    );
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .procs()
            .any(|p| p.uri == uri.as_str() && p.name == "published"),
        "the last published analysis must still be indexed mid-edit",
    );

    drop(slots_held);
    crate::rt::timeout(std::time::Duration::from_secs(30), editing)
        .await
        .expect("the edit must finish once its tail can take the diag_slots lock")
        .expect("did_change panicked");
}

/// Publishing the completed open seed may wait
/// behind a slow workspace-index reader, but must do so without retaining
/// the global document map. Otherwise the next edit holds its ordered turn
/// while waiting for that map and restores the whole-server barrier wedge.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn open_index_publish_releases_documents_while_an_index_reader_drains_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///open-index-reader-1854.tcl").unwrap();
    let text = "proc current {} {}\n";
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
    let analysis = {
        let mut analyser = Analyser::new();
        Arc::new(analyser.analyse(text, "tcl8.6").clone())
    };
    let seed = FreshAnalysisSeed {
        path_admission: Arc::new(
            DocumentPathAdmission::capture(&uri, backend.store.as_ref()).unwrap(),
        ),
        analysis,
        analyser_inputs_epoch: backend.diag_inputs_epoch(),
        class_factory_generation: 0,
    };
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
    .expect("the open publisher must queue behind the held index reader");
    let documents = crate::rt::timeout(
        std::time::Duration::from_millis(500),
        backend.documents.lock("test_index_publish_liveness"),
    )
    .await
    .expect("an index wait must not retain the document map");
    drop(documents);

    // Queue a reader behind the publisher's fair writer turn, then keep it
    // held. Dropping the writer before the currency-checked commit would
    // admit this reader and leave the seed pending indefinitely.
    let (late_reader_queued, queued) = tokio::sync::oneshot::channel();
    let (release_late_reader, release) = tokio::sync::oneshot::channel();
    let late_reader = crate::rt::spawn({
        let workspace_index = Arc::clone(&backend.workspace_index);
        async move {
            let _ = late_reader_queued.send(());
            let _guard = workspace_index.read().await;
            let _ = release.await;
        }
    });
    queued
        .await
        .expect("the late reader must start queueing behind the writer");
    crate::rt::yield_now().await;

    drop(index_reader);
    assert!(
        crate::rt::timeout(std::time::Duration::from_secs(2), publishing)
            .await
            .expect("the queued writer must commit before the later reader")
            .expect("the open index publisher must not panic"),
        "the still-current open seed must publish",
    );
    let _ = release_late_reader.send(());
    crate::rt::timeout(std::time::Duration::from_secs(5), late_reader)
        .await
        .expect("the later reader must finish after release")
        .expect("the later reader must not panic");
}

/// A live index publisher waiting for an existing index reader must leave
/// the document map available to that reader. Retaining exclusive document
/// admission while awaiting the index writer lets an earlier reader that
/// next needs the document map form a permanent cycle, leaving `didOpen`
/// at Salsa readiness for ever.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn open_index_publish_does_not_wedge_an_index_reader_needing_documents_1849() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///open-index-reader-documents-1849.tcl").unwrap();
    let text = "proc current {} {}\n";
    let documents = backend.documents.lock("test_alternating_initial").await;
    drop(documents);
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
    let seed = FreshAnalysisSeed {
        path_admission: Arc::new(
            DocumentPathAdmission::capture(&uri, backend.store.as_ref()).unwrap(),
        ),
        analysis: Arc::new(Analyser::new().analyse(text, "tcl8.6").clone()),
        analyser_inputs_epoch: backend.diag_inputs_epoch(),
        class_factory_generation: 0,
    };
    let documents = backend.documents.lock("test_alternating_initial").await;
    let index = backend.workspace_index.read().await;
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
        while !backend
            .documents
            .waiters()
            .iter()
            .any(|waiter| waiter.site == "did_open_index_currency")
        {
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the publisher must first join the document-map queue");
    drop(documents);
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        while backend.workspace_index.try_read().is_ok() {
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the publisher must queue for the index writer");

    let reader_documents = crate::rt::timeout(
        std::time::Duration::from_millis(500),
        backend.documents.lock("index_reader_followup_1849"),
    )
    .await
    .expect(
        "an index reader must be able to finish document work while live publication waits \
             for its index guard (#1849)",
    );
    drop(reader_documents);

    drop(index);
    assert!(
        crate::rt::timeout(std::time::Duration::from_secs(5), publishing)
            .await
            .expect("the publisher must finish once the index reader drains")
            .expect("the publisher task must not panic"),
        "the still-current open seed must publish",
    );
}

/// If a newer document revision lands after an
/// index replacement but before the final currency check, the obsolete
/// publisher must remove its own records instead of leaving stale spans
/// visible under the newer buffer.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn superseded_open_index_replacement_is_rolled_back_1907() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///superseded-open-index-1907.tcl").unwrap();
    let text = "proc obsolete {} {}\n";
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
    let seed = FreshAnalysisSeed {
        path_admission: Arc::new(
            DocumentPathAdmission::capture(&uri, backend.store.as_ref()).unwrap(),
        ),
        analysis: Arc::new(Analyser::new().analyse(text, "tcl8.6").clone()),
        analyser_inputs_epoch: backend.diag_inputs_epoch(),
        class_factory_generation: 0,
    };

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
        while backend.workspace_index.try_read().is_ok() {
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the old publisher must queue behind the index reader");

    let mut documents = backend.documents.lock("test_newer_revision").await;
    let mut newer =
        DocumentState::with_version("proc current {} {}\n".to_owned(), "tcl8.6".to_owned(), 2);
    newer.publication = DocumentPublication::Salsa;
    documents.insert(uri.clone(), newer);
    drop(index_reader);
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if backend
                .workspace_index
                .try_read()
                .is_ok_and(|index| index.workspace_command_exists("::obsolete"))
            {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the obsolete replacement must reach the index before its final check");
    drop(documents);

    assert!(
        !crate::rt::timeout(std::time::Duration::from_secs(5), publishing)
            .await
            .expect("the obsolete publisher must finish")
            .expect("the obsolete publisher must not panic"),
        "the newer document revision must supersede the old publisher",
    );
    let index = backend.workspace_index.read().await;
    assert!(
        !index.contains_document(uri.as_str()),
        "the obsolete publisher must roll back its own index revision",
    );
    assert!(!index.workspace_command_exists("::obsolete"));
}

/// A cold seed analysed under an old
/// class-factory oracle must be recomputed if the project publishes a new
/// oracle before its standalone index commit.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn open_index_seed_retries_after_factory_oracle_change_1854() {
    use salsa::Setter as _;

    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///factory-oracle-race-1854.tcl").unwrap();
    let consumer = concat!(
        "::tk::Megawidget create IconList FocusableWidget {\n",
        "    method GetSpecs {} { return iconlist }\n",
        "}\n",
    );
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(consumer.to_owned(), "tcl9.0".to_owned(), 1)
            .with_language_id("tcl9.0".to_owned())
            .with_publication_pending(),
    );
    let stale_analysis = {
        let mut analyser = Analyser::new();
        Arc::new(analyser.analyse(consumer, "tcl9.0").clone())
    };
    assert!(
        !stale_analysis.all_classes.contains_key("::IconList"),
        "the stale seed must not know the cross-file factory",
    );
    let oracle = {
        let mut analyser = Analyser::new();
        Arc::new(
            analyser
                .analyse(
                    concat!(
                        "oo::class create ::tk::MegawidgetClass {}\n",
                        "oo::class create ::tk::Megawidget {\n",
                        "    superclass oo::class\n",
                        "    self method create {name superclasses body} {\n",
                        "        next $name [list superclass ::tk::MegawidgetClass",
                        " {*}$superclasses]\\;$body\n",
                        "    }\n",
                        "}\n",
                    ),
                    "tcl9.0",
                )
                .class_factories(),
        )
    };

    let seed_started = Arc::new(tokio::sync::Notify::new());
    let seed_release = Arc::new(tokio::sync::Notify::new());
    let started = seed_started.notified();
    tokio::pin!(started);
    started.as_mut().enable();
    let analyser_inputs_epoch = backend.diag_inputs_epoch();
    let publishing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        let seed_started = Arc::clone(&seed_started);
        let seed_release = Arc::clone(&seed_release);
        async move {
            let path_admission =
                Arc::new(DocumentPathAdmission::capture(&uri, backend.store.as_ref()).unwrap());
            backend
                .commit_open_document_with_seed(&uri, consumer, "tcl9.0", 1, async move {
                    seed_started.notify_one();
                    seed_release.notified().await;
                    Ok(FreshAnalysisSeed {
                        path_admission,
                        analysis: stale_analysis,
                        analyser_inputs_epoch,
                        class_factory_generation: 0,
                    })
                })
                .await
                .unwrap()
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), &mut started)
        .await
        .expect("the source tier must publish before the stale seed is released");

    {
        let mut generation = backend.class_factory_generation.write().await;
        let mut db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        let file = *files.get(&uri).expect("the live Salsa source exists");
        file.set_workspace_class_factories(&mut *db)
            .to(Some(Arc::clone(&oracle)));
        *generation = generation.saturating_add(1);
    }
    seed_release.notify_one();
    assert!(
        crate::rt::timeout(std::time::Duration::from_secs(5), publishing)
            .await
            .expect("the seed publisher must finish")
            .expect("the seed publisher must not panic"),
        "the current open seed must publish after recomputation",
    );
    let index = backend.workspace_index.read().await;
    assert!(
        index.workspace_command_exists("::IconList"),
        "the revalidated seed must carry the newly published factory oracle",
    );
}

/// The class-factory generation is
/// only one analyser input. A config or SpecTcl-pack change while a cold
/// open is being analysed must also reject its seed, even when diagnostics
/// are disabled or excluded and therefore cannot repair the index later.
#[tokio::test]
async fn open_index_seed_retries_after_analyser_input_change_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///analyser-input-race-1854.tcl").unwrap();
    let text = "proc current {} {}\n";
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
    let stale_analysis = Arc::new(
        Analyser::new()
            .analyse("proc stale {} {}\n", "tcl8.6")
            .clone(),
    );
    let seed = FreshAnalysisSeed {
        path_admission: Arc::new(
            DocumentPathAdmission::capture(&uri, backend.store.as_ref()).unwrap(),
        ),
        analysis: stale_analysis,
        analyser_inputs_epoch: backend.diag_inputs_epoch(),
        class_factory_generation: 0,
    };

    let update = backend.analyser_inputs_gate.write().await;
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
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(25), async {
            while !publishing.is_finished() {
                crate::rt::yield_now().await;
            }
        })
        .await
        .is_err(),
        "index publication must wait for the analyser-input writer",
    );
    backend.invalidate_diag_inputs();
    drop(update);
    let published = crate::rt::timeout(std::time::Duration::from_secs(5), publishing)
        .await
        .expect("the seed publisher must finish after the input transaction")
        .expect("the seed publisher must not panic");
    assert!(
        published,
        "the current open must publish its recomputed seed",
    );
    let index = backend.workspace_index.read().await;
    assert!(
        index.workspace_command_exists("::current"),
        "the analyser-input epoch change must force current facts",
    );
    assert!(
        !index.workspace_command_exists("::stale"),
        "facts captured before the analyser-input change must be rejected",
    );
}

/// A cold `didOpen` index seed must not own either
/// the live-publication gate or a Salsa snapshot. Otherwise a newer edit
/// cannot publish until obsolete whole-file analysis finishes. Hold the
/// exact seed future pending, deliver a real `didChange`, and prove the
/// newer revision supersedes it immediately.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn did_change_supersedes_a_pending_did_open_index_seed_1854() {
    use tower_lsp_server::ls_types::{
        TextDocumentContentChangeEvent, VersionedTextDocumentIdentifier,
    };

    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///open-seed-superseded-1854.tcl").unwrap();
    let opened_text = "proc obsolete {} {}\n";
    let edited_text = "proc current {} {}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(opened_text.to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("tcl8.6".to_owned())
            .with_publication_pending(),
    );

    let seed_started = Arc::new(tokio::sync::Notify::new());
    let seed_release = Arc::new(tokio::sync::Notify::new());
    let started = seed_started.notified();
    tokio::pin!(started);
    started.as_mut().enable();
    let obsolete_analysis = {
        let mut analyser = Analyser::new();
        Arc::new(analyser.analyse(opened_text, "tcl8.6").clone())
    };
    let analyser_inputs_epoch = backend.diag_inputs_epoch();
    let opening = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        let seed_started = Arc::clone(&seed_started);
        let seed_release = Arc::clone(&seed_release);
        async move {
            let path_admission =
                Arc::new(DocumentPathAdmission::capture(&uri, backend.store.as_ref()).unwrap());
            backend
                .commit_open_document_with_seed(&uri, opened_text, "tcl8.6", 1, async move {
                    seed_started.notify_one();
                    seed_release.notified().await;
                    Ok(FreshAnalysisSeed {
                        path_admission,
                        analysis: obsolete_analysis,
                        analyser_inputs_epoch,
                        class_factory_generation: 0,
                    })
                })
                .await
                .unwrap()
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), &mut started)
        .await
        .expect("the open source tier must publish before its index seed starts");

    crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: edited_text.to_owned(),
            }],
        }),
    )
    .await
    .expect("the newer edit must not wait for the obsolete open index seed");

    let current = backend
        .documents
        .lock("test")
        .await
        .get(&uri)
        .cloned()
        .expect("the edited document remains open");
    assert_eq!(current.text.as_ref(), edited_text);
    assert_eq!(current.version, Some(2));
    assert_eq!(current.publication, DocumentPublication::Indexed);
    assert_superseding_edit_database_and_index(&backend, &uri, edited_text).await;
    seed_release.notify_waiters();
    assert!(
        !crate::rt::timeout(std::time::Duration::from_secs(5), opening)
            .await
            .expect("the obsolete seed task must finish")
            .expect("the obsolete seed task must not panic"),
        "the old seed must lose its final revision check",
    );
}

async fn assert_superseding_edit_database_and_index(
    backend: &Backend,
    uri: &Uri,
    edited_text: &str,
) {
    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    assert_eq!(
        files.get(uri).map(|file| file.text(&*db).as_str()),
        Some(edited_text),
        "Salsa must contain the superseding edit while the old seed is pending",
    );
    drop(files);
    drop(db);
    {
        let index = backend.workspace_index.read().await;
        assert!(
            index.workspace_command_exists("::current"),
            "indexed readiness must include the superseding revision's facts",
        );
        assert!(
            !index.workspace_command_exists("::obsolete"),
            "the rejected open seed must not leave obsolete facts",
        );
    }
}
