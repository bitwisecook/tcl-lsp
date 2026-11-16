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

//! The diagnostics mailbox and a stalled client.

use super::*;

fn pending_diagnostic_summary(publisher: &DiagnosticPublisher, uri: &Uri) -> Option<bool> {
    let state = publisher
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state
        .pending
        .get(uri)
        .map(|pending| pending.diagnostics.is_empty())
}

/// A client notification that never drains may stop diagnostics, but it
/// must not stop edits or requests from taking the global document map.
/// The close committed behind it must also replace the pending stale set.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stalled_diagnostic_delivery_cannot_hold_documents_and_close_wins() {
    use std::sync::atomic::Ordering;

    let backend = Arc::new(test_backend());
    // Keep the persistent consumer deliberately parked without involving
    // transport timing. Enqueue still commits normally; receipts simply do
    // not resolve until a newer state replaces them.
    backend
        .diagnostic_publisher
        .started
        .store(true, Ordering::Release);
    let uri = Uri::from_str("file:///w/mailbox-stall.tcl").unwrap();
    register(&backend, &uri, "set x 1\n").await;
    let revision = backend
        .documents
        .lock("test revision")
        .await
        .get(&uri)
        .expect("registered document")
        .revision;

    let publishing_backend = Arc::clone(&backend);
    let publishing_uri = uri.clone();
    let publish = tokio::spawn(async move {
        unassociated_delivery(
            &publishing_backend,
            &publishing_uri,
            DiagCurrency::Open(revision),
            Some(1),
        )
        .deliver_if_current(vec![tower_lsp_server::ls_types::Diagnostic::default()])
        .await
    });

    crate::rt::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if pending_diagnostic_summary(&backend.diagnostic_publisher, &uri).is_some() {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the authoritative publish must reach the mailbox");

    let mut docs = crate::rt::timeout(
        std::time::Duration::from_millis(250),
        backend.documents.lock("test close"),
    )
    .await
    .expect("a pending client send must not retain the documents lock");
    docs.remove(&uri);
    drop(docs);

    let clearing_backend = Arc::clone(&backend);
    let clearing_uri = uri.clone();
    let clear = tokio::spawn(async move {
        clearing_backend
            .clear_closed_diagnostics(&clearing_uri)
            .await;
    });

    crate::rt::timeout(std::time::Duration::from_secs(1), async {
        loop {
            let close_is_pending =
                pending_diagnostic_summary(&backend.diagnostic_publisher, &uri) == Some(true);
            if close_is_pending {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the close must replace the pending stale diagnostics");
    assert!(
        !crate::rt::timeout(std::time::Duration::from_secs(1), publish)
            .await
            .expect("replacement must settle the stale receipt")
            .expect("the publish task must not panic"),
        "the superseded run must not claim delivery or emit its completion marker",
    );
    let state = backend
        .diagnostic_publisher
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let pending = state.pending.get(&uri).expect("close clear stays pending");
    assert!(
        pending.diagnostics.is_empty(),
        "the newest retained state for a closed URI must be the clear",
    );
    assert_eq!(
        state.order.iter().filter(|queued| *queued == &uri).count(),
        1
    );
    drop(state);
    clear.abort();
}

/// Latest-wins replacement is bounded per URI and moves the replacement to
/// its real cross-URI commit position.
#[tokio::test]
async fn diagnostic_mailbox_is_latest_wins_and_preserves_surviving_order() {
    use std::sync::atomic::Ordering;

    let backend = test_backend();
    let publisher = &backend.diagnostic_publisher;
    publisher.started.store(true, Ordering::Release);
    let a = Uri::from_str("file:///a.tcl").unwrap();
    let b = Uri::from_str("file:///b.tcl").unwrap();
    let first_a = publisher.enqueue(a.clone(), Vec::new(), Some(1), false, false);
    let _b = publisher.enqueue(b.clone(), Vec::new(), Some(1), false, false);
    let _latest_a = publisher.enqueue(a.clone(), Vec::new(), Some(2), false, false);

    assert_eq!(
        first_a.await.expect("publisher remains alive"),
        DiagnosticPublishOutcome::Superseded,
    );
    let state = publisher
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(state.pending.len(), 2, "one retained vector per URI");
    assert_eq!(state.order, VecDeque::from([b, a.clone()]));
    assert_eq!(state.pending.get(&a).expect("latest a").version, Some(2),);
}
