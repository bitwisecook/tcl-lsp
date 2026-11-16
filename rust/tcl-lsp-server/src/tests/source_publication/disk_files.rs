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

//! Disk publication: what it holds, what it yields to and what it serialises.

use super::*;

/// A closed-file disk refresh is background work.  If its
/// workspace-index publication is delayed, it must not retain the global
/// open-document map and stop every diagnostics worker / document-sync
/// handler behind it.
///
/// The interleaving is constructed rather than timed by load: hold the
/// exact downstream writer the reindex needs, wait until it has entered the
/// disk-publication transaction, then prove an unrelated document-map
/// reader can still enter. The disk-only gate remains held across fair
/// dependency waits, unlike the deliberately released rehoming gate, so it
/// is the stable observation point. The pre-fix implementation
/// deterministically times out here because it takes `documents` before
/// waiting for this writer.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disk_reindex_waiting_for_the_index_never_holds_documents_1800() {
    let root = unique_scratch_dir("reindex-lock-scope-1800");
    let on_disk = root.join("closed.tcl");
    std::fs::write(&on_disk, "proc disk_copy {} {}\n").unwrap();

    let backend = Arc::new(test_backend());
    let uri = Uri::from_file_path(&on_disk).unwrap();
    let index_held = backend.workspace_index.write().await;
    let reindexing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move { backend.reindex_index_from_disk(&uri).await.unwrap() }
    });

    // Reaching the transaction includes blocking-pool admission plus the
    // full disk read and analysis. Under the complete nextest suite that
    // setup can be delayed by unrelated CPU work; it is not the lock-scope
    // assertion, whose deliberately tight deadline remains below.
    crate::rt::timeout(std::time::Duration::from_secs(30), async {
        loop {
            if backend.disk_publication_gate.try_lock().is_err() {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the disk refresh must reach its publication transaction");
    // Give the publisher ample time to observe the held writer and retry;
    // this is synchronisation, not the assertion's deadline.
    crate::rt::sleep(std::time::Duration::from_millis(50)).await;

    let docs = crate::rt::timeout(
        std::time::Duration::from_millis(250),
        backend.documents.lock("issue-1800-unrelated-reader"),
    )
    .await
    .expect(
        "a disk refresh waiting on the workspace index must leave the \
             open-document map available (#1800)",
    );
    drop(docs);

    drop(index_held);
    crate::rt::timeout(std::time::Duration::from_secs(30), reindexing)
        .await
        .expect("the refresh must finish once the index writer is released")
        .expect("the refresh task must not panic");
    std::fs::remove_dir_all(&root).ok();
}

/// The other blocking edge inside that critical section is
/// salsa's synchronous `cancel_others`: a setter waits for every database
/// snapshot to retire. Hold one deliberately and prove background reindex
/// waits asynchronously without owning `documents`, then finishes after
/// the snapshot drops.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disk_reindex_waiting_for_a_salsa_snapshot_never_holds_documents_1800() {
    let root = unique_scratch_dir("reindex-snapshot-1800");
    let on_disk = root.join("closed.tcl");
    std::fs::write(&on_disk, "proc disk_copy {} {}\n").unwrap();

    let backend = Arc::new(test_backend());
    let uri = Uri::from_file_path(&on_disk).unwrap();
    // Production code must never retain a snapshot across an await. This
    // test does so intentionally to make the setter's blocking precondition
    // stable for the duration of the experiment.
    let snapshot = {
        let db = backend.db.lock().await;
        backend
            .db
            .snapshot_from(&db, "issue-1800-held-test-snapshot")
            .expect("the test creates its snapshot before publication")
    };
    let reindexing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move { backend.reindex_index_from_disk(&uri).await.unwrap() }
    });

    // Match the disk-analysis admission allowance in the index-contention
    // experiment above. The document-map assertion still has its own
    // deliberately tight deadline below.
    crate::rt::timeout(std::time::Duration::from_secs(30), async {
        loop {
            if backend.disk_publication_gate.try_lock().is_err() {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the disk refresh must reach its publication transaction");
    crate::rt::sleep(std::time::Duration::from_millis(50)).await;

    let docs = crate::rt::timeout(
        std::time::Duration::from_millis(250),
        backend
            .documents
            .lock("issue-1800-snapshot-unrelated-reader"),
    )
    .await
    .expect(
        "a disk refresh waiting for a salsa snapshot must leave the \
             open-document map available (#1800)",
    );
    drop(docs);

    drop(snapshot);
    crate::rt::timeout(std::time::Duration::from_secs(30), reindexing)
        .await
        .expect("the refresh must finish once the snapshot is released")
        .expect("the refresh task must not panic");
    std::fs::remove_dir_all(&root).ok();
}

/// A background scan that was read before `didOpen` must yield publication
/// to the new live buffer and must not later overwrite it. This constructs
/// that ordering at the index writer: reindex reaches the transaction
/// first, `didOpen` arrives while it is waiting, and foreground sync is
/// nevertheless the waiter that commits once the writer is released.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disk_reindex_yields_to_a_racing_live_buffer_1800() {
    let root = unique_scratch_dir("reindex-live-race-1800");
    let on_disk = root.join("racing.tcl");
    std::fs::write(&on_disk, "proc stale {} {}\n").unwrap();

    let backend = Arc::new(test_backend());
    let uri = Uri::from_file_path(&on_disk).unwrap();
    let mut index_held = backend.workspace_index.write().await;
    let stale_analysis = Analyser::new()
        .analyse("proc stale {} {}\n", "tcl8.6")
        .clone();
    index_held.add_document(uri.as_str(), &stale_analysis);

    let reindexing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move { backend.reindex_index_from_disk(&uri).await.unwrap() }
    });
    // This waits through blocking-pool admission and disk analysis before
    // constructing the ordering below; it is not a performance assertion.
    crate::rt::timeout(std::time::Duration::from_secs(30), async {
        loop {
            if backend.disk_publication_gate.try_lock().is_err() {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the disk refresh must reach its publication transaction");

    let opening = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            Box::pin(backend.did_open(DidOpenTextDocumentParams {
                text_document: tower_lsp_server::ls_types::TextDocumentItem {
                    uri,
                    language_id: "tcl".to_owned(),
                    version: 1,
                    text: "proc fresh {} {}\n".to_owned(),
                },
            }))
            .await;
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let live_is_visible = backend
                .documents
                .lock("issue-1800-race-wait-visible")
                .await
                .get(&uri)
                .is_some_and(|doc| doc.text.as_ref() == "proc fresh {} {}\n");
            if live_is_visible && backend.edit_order.holder.lock().unwrap().is_none() {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect(
        "foreground didOpen must publish its live buffer and release the turn while retrying \
             the held index",
    );

    drop(index_held);
    crate::rt::timeout(std::time::Duration::from_secs(30), opening)
        .await
        .expect("didOpen must finish once the index writer is released")
        .expect("didOpen must not panic");
    crate::rt::timeout(std::time::Duration::from_secs(30), reindexing)
        .await
        .expect("the superseded disk refresh must finish")
        .expect("the disk refresh must not panic");

    let live_dialect = {
        let docs = backend.documents.lock("issue-1800-race-assertion").await;
        let live = docs.get(&uri).expect("didOpen must retain its document");
        assert_eq!(live.text.as_ref(), "proc fresh {} {}\n");
        live.dialect.clone()
    };
    let live_source_matches = {
        let db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        files.get(&uri).is_some_and(|file| {
            file.text(&*db) == "proc fresh {} {}\n" && file.dialect(&*db).as_str() == live_dialect
        })
    };
    assert!(
        live_source_matches,
        "the live buffer must remain the salsa source of truth",
    );
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .proc_definitions("stale", "other")
            .is_empty(),
        "the pre-open disk analysis must not be published after didOpen",
    );

    std::fs::remove_dir_all(&root).ok();
}

/// Disk publication shares the live
/// source gate, so it must also establish writer intent while snapshots
/// drain. Otherwise an overlapping request stream can keep its census
/// non-empty forever and strand every later didOpen/didChange behind it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disk_publication_intent_stops_new_snapshots_until_drain_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///disk-snapshot-drain-1854.tcl").unwrap();
    let text = "proc disk_current {} {}\n";
    let analysis = Analyser::new().analyse(text, "tcl8.6").clone();
    let existing = backend
        .db
        .snapshot("issue-1854-pre-disk-drain-reader")
        .await;
    let publishing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            let replacements = vec![(uri, text.to_owned(), "tcl8.6".to_owned(), analysis)];
            backend
                .publish_disk_results(
                    &replacements,
                    &[],
                    DiskRemovalPolicy::ClosedOnly,
                    "test_disk_snapshot_drain",
                )
                .await
                .unwrap()
        }
    });
    wait_for_publication_intents_1854(&backend, 1, "disk publication must claim intent").await;

    let mut later_reader = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        async move {
            backend
                .db
                .snapshot("issue-1854-post-disk-drain-reader")
                .await
        }
    });
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), &mut later_reader)
            .await
            .is_err(),
        "a later reader must queue instead of replenishing the disk census",
    );

    drop(existing);
    assert_eq!(
        crate::rt::timeout(std::time::Duration::from_secs(5), publishing)
            .await
            .expect("disk publication must finish when the old reader retires")
            .expect("the disk publication task must not panic"),
        vec![uri.clone()],
    );
    let later = crate::rt::timeout(std::time::Duration::from_secs(5), later_reader)
        .await
        .expect("queued readers must resume after disk publication")
        .expect("the reader task must not panic");
    drop(later);

    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    assert_eq!(
        files.get(&uri).map(|file| file.text(&*db).as_str()),
        Some(text),
        "disk publication must run between the old and later snapshots",
    );
}

/// Releasing the outer gates must not let a newer disk snapshot overtake a
/// blocked older one and then be overwritten when the older work resumes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disk_publication_serialises_captured_replacements_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///disk-order-1854.tcl").unwrap();
    let old_text = "proc older_disk_read {} {}\n";
    let new_text = "proc newer_disk_read {} {}\n";
    let seeds = backend.rehomed_source_seeds.lock().await;
    let older = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        let analysis = Analyser::new().analyse(old_text, "tcl8.6").clone();
        async move {
            backend
                .publish_disk_results(
                    &[(uri, old_text.to_owned(), "tcl8.6".to_owned(), analysis)],
                    &[],
                    DiskRemovalPolicy::ClosedOnly,
                    "test_older_disk_read",
                )
                .await
                .unwrap()
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        while backend.disk_publication_gate.try_lock().is_ok() {
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the older disk snapshot must own the serialisation gate");

    let newer = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        let analysis = Analyser::new().analyse(new_text, "tcl8.6").clone();
        async move {
            backend
                .publish_disk_results(
                    &[(uri, new_text.to_owned(), "tcl8.6".to_owned(), analysis)],
                    &[],
                    DiskRemovalPolicy::ClosedOnly,
                    "test_newer_disk_read",
                )
                .await
                .unwrap()
        }
    });
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), async {
            while !newer.is_finished() {
                crate::rt::yield_now().await;
            }
        })
        .await
        .is_err(),
        "the newer disk snapshot must queue behind the older one",
    );

    drop(seeds);
    assert_eq!(
        crate::rt::timeout(std::time::Duration::from_secs(5), older)
            .await
            .expect("the older disk publication must finish")
            .expect("the older disk publisher must not panic"),
        vec![uri.clone()],
    );
    assert_eq!(
        crate::rt::timeout(std::time::Duration::from_secs(5), newer)
            .await
            .expect("the newer disk publication must finish second")
            .expect("the newer disk publisher must not panic"),
        vec![uri.clone()],
    );
    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    assert_eq!(
        files.get(&uri).map(|file| file.text(&*db).as_str()),
        Some(new_text),
        "the later captured disk replacement must remain authoritative",
    );
}

/// Disk publication must establish
/// writer intent before it can acquire `db`. Otherwise a sustained fair
/// mutex queue can prevent the optimistic publisher from ever reaching
/// the census check that creates the drain guard.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disk_publication_establishes_intent_before_db_bundle_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///disk-pre-intent-1854.tcl").unwrap();
    let text = "proc disk_pre_intent {} {}\n";
    let analysis = Analyser::new().analyse(text, "tcl8.6").clone();
    let database = backend.db.lock().await;
    let publishing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend
                .publish_disk_results(
                    &[(uri, text.to_owned(), "tcl8.6".to_owned(), analysis)],
                    &[],
                    DiskRemovalPolicy::ClosedOnly,
                    "test_disk_pre_intent",
                )
                .await
                .unwrap()
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        while backend
            .db
            .snapshots
            .publication_intent
            .load(std::sync::atomic::Ordering::Acquire)
            == 0
        {
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("disk publication must claim intent before acquiring db");

    let mut later_reader = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        async move { backend.db.snapshot("issue-1854-pre-intent-reader").await }
    });
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), &mut later_reader)
            .await
            .is_err(),
        "new readers must not join the fair db queue ahead of the publisher",
    );

    drop(database);
    assert_eq!(
        crate::rt::timeout(std::time::Duration::from_secs(5), publishing)
            .await
            .expect("the disk publisher must finish after db is released")
            .expect("the disk publication task must not panic"),
        vec![uri],
    );
    drop(
        crate::rt::timeout(std::time::Duration::from_secs(5), later_reader)
            .await
            .expect("the queued reader must resume after publication")
            .expect("the reader task must not panic"),
    );
}
