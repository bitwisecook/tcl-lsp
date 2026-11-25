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

//! Edit tickets, turns and the barrier requests wait behind.

use super::*;

/// Register a document in the store and the workspace index.
/// A request that reads a document must not observe a buffer that is missing
/// an already-received edit.
///
/// `tower-lsp-server` drives requests and notifications through one
/// `buffer_unordered` pool, so a request handler can be polled while an
/// earlier `didChange` is still in flight. `read_document` must therefore
/// wait for the outstanding edit rather than racing ahead and answering from
/// the pre-edit text (which yielded semantic tokens whose lines/lengths
/// described text the client had already replaced).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_document_waits_for_an_in_flight_edit() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///ordering.tcl").unwrap();
    register(&backend, &uri, "set x 1\n").await;

    // Stand in for a `didChange` that has arrived and taken its turn, but has
    // not yet spliced the buffer.
    let ticket = backend.edit_order.take_ticket("didChange", &uri);
    let edit_in_flight = backend.edit_order.wait_turn(ticket).await;

    let mut reader = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move { backend.read_document(&uri).await.unwrap().map(|d| d.text) }
    });

    // While the edit is in flight the read must not resolve.
    let elapsed = std::time::Duration::from_millis(300);
    if let Ok(done) = crate::rt::timeout(elapsed, &mut reader).await {
        panic!("read_document returned {done:?} while an edit was still in flight");
    }

    // Land the edit, then release the ordering lock: the read must now
    // resolve against the *settled* buffer, never the pre-edit text.
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new("set x 2\n".to_owned(), "tcl8.6".to_owned()),
    );
    drop(edit_in_flight);
    let text = crate::rt::timeout(std::time::Duration::from_secs(5), reader)
        .await
        .expect("read_document must resolve once the edit lands")
        .expect("reader task panicked");
    assert_eq!(text.as_deref(), Some("set x 2\n"));
}

/// A ticket whose waiter is dropped *before* its turn is granted must not
/// stop the sequence for ever.
///
/// [`EditOrder::wait_turn`]'s own docs claim this already holds — "the guard
/// releases it on drop — including on ... a dropped (cancelled) handler
/// future, so a turn is never lost". That is true only once the guard
/// exists, and the guard is created at the *grant*, not at the draw. A
/// future dropped while parked in the wait leaves `now_serving` pointing at
/// a ticket nobody holds and nobody will ever release.
///
/// The consequence is total and permanent: every later document-sync
/// notification blocks in `wait_turn`, and every request handler blocks in
/// `edits_settled` — no CPU, no output, stdin still draining.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_dropped_waiter_does_not_wedge_the_edit_order() {
    let order = EditOrder::default();
    let uri = Uri::from_str("file:///w/ordering.tcl").unwrap();

    // Ticket 0 is held by an edit that is getting on with its work.
    let first = order.take_ticket("didChange", &uri);
    let first_turn = order.wait_turn(first).await;

    // Ticket 1's handler is dropped while parked — it never reaches the
    // grant, so it never owns an `EditTurn`.
    let abandoned = order.take_ticket("didChange", &uri);
    let abandoned_n = abandoned.ticket;
    {
        let mut waiting = Box::pin(order.wait_turn(abandoned));
        // Poll it so it really is registered and parked, then drop it.
        assert!(
            crate::rt::timeout(std::time::Duration::from_millis(50), &mut waiting)
                .await
                .is_err(),
            "ticket {abandoned_n} must not be granted while an earlier ticket holds the turn",
        );
    }

    // Ticket 2 is a later edit, waiting its turn behind the abandoned one.
    let third = order.take_ticket("didChange", &uri);
    let third_n = third.ticket;
    let next = order.wait_turn(third);

    drop(first_turn);

    assert!(
        crate::rt::timeout(std::time::Duration::from_secs(5), next)
            .await
            .is_ok(),
        "a waiter dropped before its turn was granted stopped the edit order for ever: \
             ticket {third_n} never ran behind abandoned ticket {abandoned_n}. Every later edit \
             and every request handler waiting on `edits_settled` would now be blocked \
             permanently (#1657)",
    );

    // And the sequence is genuinely caught up, not merely unblocked: a
    // request barrier taken now must see every drawn ticket accounted for.
    assert!(
        crate::rt::timeout(
            std::time::Duration::from_secs(5),
            order.settled_to(order.settle_target()),
        )
        .await
        .is_ok(),
        "`settled` must not still be waiting on the abandoned ticket",
    );
}

/// `did_close` must hand its `EditOrder` turn on as soon as the
/// ordered mutations are applied, rather than holding it across the disk
/// reindex and the closed-file republish.  The ticket is a *global*
/// barrier — every request handler awaits `edits_settled` and the next
/// `did_change` waits for the ticket itself — so a close that keeps it
/// across that tail freezes hover / completion / semantic tokens in every
/// open document for as long as the close takes.
///
/// The `rehoming_gate` stands in for the tail's cost: the ordered mutations
/// (the documents map, the index entry) never take it, and the reindex takes
/// it before any other tail lock. Holding that first lock pins the close
/// without letting the tail hold `documents` while this test inspects it.
/// Using the later `db` lock here made the test itself racy: the reindex
/// could win `documents`, block on `db`, and deadlock the inspection below.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn did_close_hands_its_edit_turn_on_before_the_reindex_tail() {
    let backend = Arc::new(test_backend());
    let root = unique_scratch_dir("close-off-ticket");
    let on_disk = root.join("closing.tcl");
    std::fs::write(&on_disk, "proc foo {} {}\n").unwrap();
    let uri = crate::canonical_file_uri(&on_disk).unwrap();
    register(&backend, &uri, "proc foo {} {}\n").await;

    let rehoming_held = backend.rehoming_gate.lock().await;
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

    assert!(
        one_edit_turn_released(&backend).await,
        "did_close must release its turn before the reindex, which cannot \
             run while the rehoming gate is held",
    );
    assert!(
        !backend.documents.lock("test").await.contains_key(&uri),
        "the mutations the ticket covers must already be applied",
    );
    assert!(
        !backend.closed_diag_gen.lock().await.contains_key(&uri),
        "the closed republish is the tail; it must still be pending here",
    );

    drop(rehoming_held);
    crate::rt::timeout(std::time::Duration::from_secs(30), closing)
        .await
        .expect("the close must finish once its tail can take the rehoming gate")
        .expect("did_close panicked");
    assert!(
        backend.closed_diag_gen.lock().await.contains_key(&uri),
        "the closed republish still lands, ordered after the mutations",
    );
    std::fs::remove_dir_all(&root).ok();
}

/// The edit side: `did_change` must hand its turn on once the splice
/// and the salsa source are committed — not hold it across the diagnostics
/// scheduling and the whole-source `detect_dialect` re-scan.
///
/// `diag_slots` stands in for that tail here: the ordered mutations never
/// take it and `schedule_diagnostics` takes it first thing, so a held
/// `diag_slots` lock pins the handler in its tail.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn did_change_hands_its_edit_turn_on_before_the_scheduling_tail() {
    use tower_lsp_server::ls_types::{
        TextDocumentContentChangeEvent, VersionedTextDocumentIdentifier,
    };
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///w/edited.tcl").unwrap();
    register(&backend, &uri, "set x 1\n").await;

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
                        text: "set x 2\n".to_owned(),
                    }],
                })
                .await;
        }
    });

    assert!(
        one_edit_turn_released(&backend).await,
        "did_change must release its turn before scheduling diagnostics",
    );
    assert_eq!(
        backend
            .documents
            .lock("test")
            .await
            .get(&uri)
            .map(|doc| doc.text.clone())
            .as_deref(),
        Some("set x 2\n"),
        "the splice the ticket covers must already be committed",
    );

    drop(slots_held);
    crate::rt::timeout(std::time::Duration::from_secs(30), editing)
        .await
        .expect("the edit must finish once its tail can take the diag_slots lock")
        .expect("did_change panicked");
}
