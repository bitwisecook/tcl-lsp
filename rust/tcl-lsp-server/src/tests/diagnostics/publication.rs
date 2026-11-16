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

//! Publication currency while the index is contended.

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_diagnostics_worker_cannot_overwrite_current_analysis() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///stale.tcl").unwrap();
    let current_src = "proc current {} {}\n";
    let stale_src = "proc stale {} {}\n";

    let mut analyser = Analyser::new();
    let current_analysis = analyser.analyse(current_src, "tcl8.6").clone();
    // The current document is at revision 2; its analysis lives in the
    // query database (set via the SourceFile input) and the workspace index.
    backend
        .db_set_source(&uri, current_src, "tcl8.6".to_owned())
        .await
        .unwrap();
    {
        let mut doc = DocumentState::with_version(current_src.to_owned(), "tcl8.6".to_owned(), 2);
        doc.revision = 2;
        backend
            .documents
            .lock("test")
            .await
            .insert(uri.clone(), doc);
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &current_analysis);
    }

    // A stale worker (revision 1) must not overwrite state for revision 2.
    backend
        .publish_analyser_diagnostics(
            uri.clone(),
            stale_src.to_owned(),
            "tcl8.6".to_owned(),
            1,
            Some(1),
        )
        .await;

    // The query database still reflects the current document (its input was
    // never changed to the stale text).
    let analysis = backend
        .cached_analysis(&uri)
        .await
        .expect("current analysis available from the query database");
    assert!(analysis.all_procs.contains_key("::current"));
    assert!(!analysis.all_procs.contains_key("::stale"));

    let index = backend.workspace_index.read().await;
    assert!(
        !index.proc_definitions("current", "other").is_empty(),
        "current document should remain indexed",
    );
    assert!(
        index.proc_definitions("stale", "other").is_empty(),
        "stale worker must not re-index obsolete text",
    );
}

/// Run [`publish_diagnostics_result`] for `uri` at `revision` on its own
/// task, with the analysis of `text` as the set to index. The test client's
/// socket is detached, so delivery settles without a client.
fn spawn_diagnostics_publish(
    backend: &Arc<Backend>,
    uri: &Uri,
    text: &'static str,
    revision: u64,
) -> crate::rt::JoinHandle<bool> {
    let backend = Arc::clone(backend);
    let uri = uri.clone();
    crate::rt::spawn(async move {
        let analysis = Arc::new(Analyser::new().analyse(text, "tcl8.6").clone());
        let delivery = DeliveryCtx {
            sidecar_revision: backend.fresh_analysis_cache.lock().unwrap().revision(),
            inputs_epoch: Some(backend.diag_inputs_epoch()),
            configuration_epoch: backend.diag_inputs_epoch(),
            source_inputs_epoch: Arc::clone(&backend.diag_inputs_epoch),
            source_inputs_gate: Arc::clone(&backend.analyser_inputs_gate),
            diagnostic_pass: None,
            client: &backend.client,
            diagnostic_publisher: &backend.diagnostic_publisher,
            documents: &backend.documents,
            store: &backend.store,
            diag_slots: &backend.diag_slots,
            pull_diag_cache: &backend.pull_diag_cache,
            closed_diag_gen: &backend.closed_diag_gen,
            source: "",
            pack_seq: 0,
            pack_set: None,
            registry: None,
            spec_packs: None,
            uri: &uri,
            currency: DiagCurrency::Open(revision),
            version: Some(1),
            client_supports_pull: false,
        };
        publish_diagnostics_result(
            &delivery,
            &backend.workspace_index,
            &backend.rehomed_source_seeds,
            &backend.rehoming_gate,
            &analysis,
            (Err(DiagnosticUnavailable::Context), Ok(Vec::new()), None),
            PublishTiming {
                started: crate::rt::Instant::now(),
                uri_str: uri.as_str(),
                line_count: 1,
            },
        )
        .await
    })
}

/// Wait until `count` writers are parked in the workspace index's queue.
async fn await_queued_index_writers(backend: &Backend, count: usize) {
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let queued = backend
                .workspace_index
                .tracking()
                .waiters
                .values()
                .filter(|(access, _)| *access == RwLockAccess::Write)
                .count();
            if queued >= count {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the publishers must queue for the index writer");
}

/// A diagnostics publish that waits for the workspace index must not do so
/// holding `documents`, or the `didClose` holding the edit turn parks
/// behind the map and every request behind the turn. With a held index
/// reader and publishes for several open documents in flight — the first
/// parked on the index writer, the rest behind it on the rehoming gate —
/// an unrelated close must still hand the barrier on, and `edits_settled`
/// (the wait every handler makes first) must answer inside the budget.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn diagnostics_publish_waiting_for_the_index_never_holds_documents() {
    let backend = Arc::new(test_backend());
    let texts: [&'static str; 6] = [
        "proc pack_a {} {}\n",
        "proc pack_b {} {}\n",
        "proc pack_c {} {}\n",
        "proc pack_d {} {}\n",
        "proc pack_e {} {}\n",
        "proc pack_f {} {}\n",
    ];
    let mut open = Vec::new();
    for (i, text) in texts.iter().enumerate() {
        let uri = Uri::from_str(&format!("file:///pack/doc{i}.tcl")).unwrap();
        register(&backend, &uri, text).await;
        let revision = backend
            .documents
            .lock("test")
            .await
            .get(&uri)
            .expect("registered")
            .revision;
        open.push((uri, *text, revision));
    }
    let closing_uri = Uri::from_str("file:///pack/closing.tcl").unwrap();
    register(&backend, &closing_uri, "set closing 1\n").await;

    let index_reader = backend.workspace_index.read().await;
    let publishes: Vec<_> = open
        .iter()
        .map(|(uri, text, revision)| spawn_diagnostics_publish(&backend, uri, text, *revision))
        .collect();
    await_queued_index_writers(&backend, 1).await;

    let docs = crate::rt::timeout(
        std::time::Duration::from_millis(500),
        backend.documents.lock("test_publish_liveness"),
    )
    .await
    .expect("a publish queued for the index must not retain the document map");
    drop(docs);

    let closing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = closing_uri.clone();
        async move {
            backend
                .did_close(DidCloseTextDocumentParams {
                    text_document: TextDocumentIdentifier { uri },
                })
                .await;
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(2), backend.edits_settled())
        .await
        .expect(
            "the edit barrier must advance past a didClose while diagnostics publishes \
                 wait for the workspace index — every request handler waits here first",
        );
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if !backend
                .documents
                .lock("test_closed")
                .await
                .contains_key(&closing_uri)
            {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the close must remove its buffer while the index stays unavailable");

    drop(index_reader);
    crate::rt::timeout(std::time::Duration::from_secs(10), closing)
        .await
        .expect("didClose must finish once the index is available")
        .expect("didClose must not panic");
    for publish in publishes {
        assert!(
            crate::rt::timeout(std::time::Duration::from_secs(10), publish)
                .await
                .expect("each publish must reach its index write once the reader drains")
                .expect("a publish must not panic"),
            "a current publish settles its revision",
        );
    }
    let index = backend.workspace_index.read().await;
    for (uri, _, _) in &open {
        assert!(
            index.document_revision(uri.as_str()).is_some(),
            "{uri:?} must be indexed by its publish",
        );
    }
}

/// The re-check half of the pattern: a publish that had to release the map
/// to wait for the index must re-establish currency under a fresh map
/// guard, so an edit that landed during the wait wins and the stale
/// analysis never reaches the index.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn diagnostics_publish_rechecks_currency_after_waiting_for_the_index() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///pack/edited-during-wait.tcl").unwrap();
    register(&backend, &uri, "set initial 1\n").await;
    let revision = backend
        .documents
        .lock("test")
        .await
        .get(&uri)
        .expect("registered")
        .revision;

    let index_reader = backend.workspace_index.read().await;
    let publish =
        spawn_diagnostics_publish(&backend, &uri, "proc stale_from_publish {} {}\n", revision);
    await_queued_index_writers(&backend, 1).await;

    // The edit that supersedes the parked publish, landing while it waits.
    // Bounded, so a publish that regresses to waiting under the map fails
    // here instead of hanging the test.
    crate::rt::timeout(
        std::time::Duration::from_millis(500),
        backend.documents.lock("test_edit"),
    )
    .await
    .expect("a publish queued for the index must not retain the document map")
    .get_mut(&uri)
    .expect("still open")
    .revision = revision + 1;

    drop(index_reader);
    assert!(
        crate::rt::timeout(std::time::Duration::from_secs(10), publish)
            .await
            .expect("the superseded publish must return once the index frees")
            .expect("a publish must not panic"),
        "a superseded publish is settled, not retried",
    );
    let index = backend.workspace_index.read().await;
    assert!(
        !index.workspace_command_exists("::stale_from_publish"),
        "an analysis superseded during the index wait must never be indexed",
    );
}

/// A diagnostics run publishes a standalone analysis before the next
/// source-site reconciliation. It must not do that while a navigation request is
/// consuming the freshly re-homed index snapshot: otherwise the query can
/// map the declaration through `::x` and then gather calls from the
/// standalone `::helper` view, yielding an empty reference set.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn diagnostics_publish_waits_for_rehomed_navigation_snapshot() {
    let backend = Arc::new(test_backend());
    let a = Uri::from_str("file:///proj/a.tcl").unwrap();
    let b = Uri::from_str("file:///proj/b.tcl").unwrap();
    let b_src = "proc helper {} {}\n";
    register(&backend, &b, b_src).await;
    register(
        &backend,
        &a,
        "namespace eval ::x { source b.tcl }\n::x::helper\n",
    )
    .await;

    let query_guard = backend.rehomed_index_guard().await.unwrap();
    let query_started = Arc::new(tokio::sync::Notify::new());
    let publish_finished = Arc::new(tokio::sync::Notify::new());
    let publisher_backend = Arc::clone(&backend);
    let publisher_b = b.clone();
    let publisher_started = Arc::clone(&query_started);
    let publisher_finished = Arc::clone(&publish_finished);
    let standalone = Arc::new(Analyser::new().analyse(b_src, "tcl8.6").clone());
    let publisher = crate::rt::spawn(async move {
        let revision = publisher_backend
            .documents
            .lock("test")
            .await
            .get(&publisher_b)
            .expect("b.tcl stays open")
            .revision;
        let uri_string = publisher_b.to_string();
        let delivery = unassociated_delivery(
            &publisher_backend,
            &publisher_b,
            DiagCurrency::Open(revision),
            None,
        );
        publisher_started.notify_one();
        let settled = publish_diagnostics_result(
            &delivery,
            &publisher_backend.workspace_index,
            &publisher_backend.rehomed_source_seeds,
            &publisher_backend.rehoming_gate,
            &standalone,
            (Err(DiagnosticUnavailable::Context), Ok(Vec::new()), None),
            PublishTiming {
                started: crate::rt::Instant::now(),
                uri_str: &uri_string,
                line_count: 1,
            },
        )
        .await;
        publisher_finished.notify_one();
        settled
    });

    query_started.notified().await;
    assert!(
        crate::rt::timeout(
            std::time::Duration::from_millis(100),
            publish_finished.notified(),
        )
        .await
        .is_err(),
        "standalone diagnostics publication must wait for the active re-homed snapshot",
    );
    {
        let index = backend.workspace_index.read().await;
        assert!(
            index
                .linked_invocations_of("::x::helper", b.as_str())
                .iter()
                .any(|site| site.uri == a.as_str()),
            "the guarded snapshot must retain the source-site call view",
        );
    }
    drop(query_guard);

    assert_rehomed_diagnostics_publisher_resumes(publisher).await;

    let analysis = Analyser::new().analyse(b_src, "tcl8.6").clone();
    let refs = backend
        .cross_document_references(&b, b_src, &analysis, Position::new(0, 6), false)
        .await
        .unwrap();
    assert!(
        refs.iter().any(|location| location.uri == a),
        "the next query must reconcile the standalone publish before reading: {refs:?}",
    );
}

async fn assert_rehomed_diagnostics_publisher_resumes(publisher: crate::rt::JoinHandle<bool>) {
    assert!(
        crate::rt::timeout(std::time::Duration::from_secs(5), publisher)
            .await
            .expect("publisher must resume after the query releases the gate")
            .expect("publisher task must not panic"),
        "the current diagnostics run must publish",
    );
}
