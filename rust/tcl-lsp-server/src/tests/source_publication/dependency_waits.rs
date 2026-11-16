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

//! Fair waits behind a contended publication dependency.

use super::*;

/// A disk transaction must join each failed dependency fairly only after
/// releasing both outer gates. This drives one publication through a held
/// database and then a held seed store, while an unrelated live edit proves
/// the second wait does not monopolise foreground publication.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disk_publication_releases_gates_while_waiting_fairly_1854() {
    let backend = Arc::new(test_backend());
    let disk_uri = Uri::from_str("file:///disk-fair-queue-1854.tcl").unwrap();
    let disk_text = "proc disk_fair_queue {} {}\n";
    let analysis = Analyser::new().analyse(disk_text, "tcl8.6").clone();
    let live_uri = Uri::from_str("file:///live-during-disk-wait-1854.tcl").unwrap();
    let live_text = "set live current\n";
    backend
        .db_set_source(&live_uri, "set live old\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    backend.documents.lock("test").await.insert(
        live_uri.clone(),
        DocumentState::new(live_text.to_owned(), "tcl8.6".to_owned()).with_publication_pending(),
    );

    let database = backend.db.lock().await;
    let seeds = backend.rehomed_source_seeds.lock().await;
    let publishing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let disk_uri = disk_uri.clone();
        async move {
            backend
                .publish_disk_results(
                    &[(
                        disk_uri,
                        disk_text.to_owned(),
                        "tcl8.6".to_owned(),
                        analysis,
                    )],
                    &[],
                    DiskRemovalPolicy::ClosedOnly,
                    "test_disk_fair_queue",
                )
                .await
                .unwrap()
        }
    });

    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if backend.db.contention().contains("1 queued waiter") {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the disk publisher must join the contended database queue");
    drop(database);

    assert!(
        crate::rt::timeout(
            std::time::Duration::from_millis(500),
            backend.commit_live_source(
                "test_live_during_disk_wait",
                &live_uri,
                live_text,
                "tcl8.6",
                0,
                false,
            ),
        )
        .await
        .expect("the live publisher must not wait for the disk-only seed store")
        .unwrap(),
        "the unrelated current live source must publish",
    );
    assert!(
        !publishing.is_finished(),
        "the seed guard must still keep the disk transaction pending",
    );

    drop(seeds);
    assert_eq!(
        crate::rt::timeout(std::time::Duration::from_secs(5), publishing)
            .await
            .expect("disk publication must finish after its queued turn")
            .expect("the disk publisher must not panic"),
        vec![disk_uri],
    );
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .workspace_command_exists("::disk_fair_queue"),
        "the queued transaction must publish its complete index update",
    );
}

/// A live publisher that joins a
/// contended dependency must release the global publication gate first.
/// Otherwise sustained readers can repeatedly take the acquired fair turn
/// back and one URI prevents every later live edit from publishing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn live_publication_releases_gate_while_waiting_fairly_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///live-fair-gate-1854.tcl").unwrap();
    let text = "proc current {} {}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(text.to_owned(), "tcl8.6".to_owned(), 1)
            .with_publication_pending(),
    );

    let database = backend.db.lock().await;
    let publishing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend
                .commit_live_source("test_live_fair_gate", &uri, text, "tcl8.6", 0, false)
                .await
                .unwrap()
        }
    });

    let publication = crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.live_publication_gate.lock(),
    )
    .await
    .expect("the contended live publisher must release its global gate");
    drop(database);
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), async {
            while !publishing.is_finished() {
                crate::rt::yield_now().await;
            }
        })
        .await
        .is_err(),
        "the publisher must reacquire the publication gate before committing",
    );
    drop(publication);
    assert!(
        crate::rt::timeout(std::time::Duration::from_secs(5), publishing)
            .await
            .expect("the live publisher must finish after both locks are released")
            .expect("the live publisher must not panic"),
        "the still-current live source must publish",
    );
}

/// The periodic fair-queue join must not
/// retain `live_publication_gate` after a newer edit invalidates the
/// publisher. The authoritative edit must be able to take the gate while
/// the stale publisher remains queued on `db`, then both retire in order
/// once that dependency becomes available.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stale_fair_queue_wait_yields_to_newer_edit_1854() {
    use tower_lsp_server::ls_types::{
        TextDocumentContentChangeEvent, VersionedTextDocumentIdentifier,
    };

    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///stale-fair-queue-1854.tcl").unwrap();
    let initial = "set value old\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(initial.to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("tcl".to_owned())
            .with_publication_pending(),
    );

    let database = backend.db.lock().await;
    let stale_publisher = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            // Match production: the operation invalidates older publishers
            // before entering its own publication loop.
            backend.invalidate_live_publication(std::iter::once(&uri));
            let generation = backend
                .live_publication_generation
                .load(std::sync::atomic::Ordering::Acquire);
            let uri_generation = backend.live_publication_uri_generation(&uri);
            let publication = backend.live_publication_gate.lock().await;
            let mut retry = LivePublicationRetry::new(generation, uri_generation);
            retry.retries = 1023;
            assert!(
                backend
                    .pause_open_publication(
                        publication,
                        "did_open",
                        &uri,
                        LivePublicationWait::Database,
                        &mut retry,
                    )
                    .await
                    .is_none(),
                "the stale publisher must observe its URI invalidation",
            );
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if backend.db.contention().contains("1 queued waiter") {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the stale publisher must join the held database's fair queue");

    let changed_text = "set value current\n";
    let current_publisher = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend
                .did_change(DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier { uri, version: 2 },
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: None,
                        range_length: None,
                        text: changed_text.to_owned(),
                    }],
                })
                .await;
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let changed = backend
                .documents
                .lock("test")
                .await
                .get(&uri)
                .is_some_and(|doc| doc.version == Some(2) && doc.text.as_ref() == changed_text);
            if changed {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the newer edit must publish its authoritative buffer");
    drop(database);
    crate::rt::timeout(std::time::Duration::from_secs(5), current_publisher)
        .await
        .expect("the current publisher must finish once db is released")
        .expect("the current publisher must not panic");
    crate::rt::timeout(std::time::Duration::from_secs(5), stale_publisher)
        .await
        .expect("the invalidated publisher must retire after the current edit")
        .expect("the stale publisher must not panic");
    let docs = backend.documents.lock("test").await;
    let current = docs.get(&uri).expect("the edited document remains open");
    assert_eq!(current.publication, DocumentPublication::Indexed);
}

/// The invalidation generation is
/// global, so an edit to another URI may wake a current publisher's fair
/// queue wait. It must adopt the new generation and join even when the
/// document map itself is the contended dependency it cannot inspect.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unrelated_invalidation_refreshes_fair_queue_generation_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///fair-queue-a-1854.tcl").unwrap();
    let text = "proc current {} {}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(text.to_owned(), "tcl8.6".to_owned(), 1)
            .with_publication_pending(),
    );
    let generation = backend
        .live_publication_generation
        .load(std::sync::atomic::Ordering::Acquire);
    let mut retry =
        LivePublicationRetry::new(generation, backend.live_publication_uri_generation(&uri));

    // This represents an edit to document B: A's identity remains current,
    // but its captured global generation is now old.
    let unrelated = Uri::from_str("file:///fair-queue-b-1854.tcl").unwrap();
    backend.invalidate_live_publication(std::iter::once(&unrelated));
    {
        let docs = backend.documents.lock("test").await;
        assert!(
            docs.get(&uri)
                .is_some_and(|doc| doc.matches_open_publication(text, "tcl8.6", 1)),
            "the unrelated invalidation must not obsolete document A",
        );
    }
    retry.retries = 1023;

    let documents = backend.documents.lock("test-held-map").await;
    let joining = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            let publication = backend.live_publication_gate.lock().await;
            assert!(
                backend
                    .pause_open_publication(
                        publication,
                        "did_open",
                        &uri,
                        LivePublicationWait::Documents,
                        &mut retry,
                    )
                    .await
                    .is_some(),
                "an unrelated invalidation must preserve the publisher",
            );
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let (_, waiters) = backend.documents.watchdog_snapshot();
            if waiters.iter().any(|waiter| waiter.site == "did_open") {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the current publisher must rejoin the document map's fair queue");
    drop(documents);
    crate::rt::timeout(std::time::Duration::from_secs(5), joining)
        .await
        .expect("the refreshed fair-queue join must finish")
        .expect("the refreshed fair-queue join must not panic");
}
