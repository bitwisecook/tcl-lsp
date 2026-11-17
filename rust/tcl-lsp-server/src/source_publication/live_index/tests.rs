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

use super::super::super::DocumentState;
use super::super::super::tests::test_backend;
use std::str::FromStr as _;
use std::time::Duration;
use tower_lsp_server::ls_types::Uri;

/// `until_document_superseded` keeps the caller's seed future across rounds.
/// A seed parked in the open-document map's fair queue keeps its place there,
/// so the next round's currency check, queued behind it, must not stop the
/// seed being polled: when the holder releases the map it is handed to the
/// seed, and only polling the seed takes and releases it. Waiting for the
/// check alone left the map assigned to a future nobody polled, with every
/// later request parked behind it (#1657, the didOpen seed path).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_seed_queued_on_the_map_is_polled_while_the_currency_check_waits_1657() {
    let short = Duration::from_millis(200);
    let backend = test_backend();
    let uri = Uri::from_str("file:///seed.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new("set x 1\n".to_owned(), "tcl8.6".to_owned()),
    );
    let (start_tx, start_rx) = tokio::sync::oneshot::channel::<()>();
    let documents = &backend.documents;
    let seed = std::pin::pin!(async move {
        let _ = start_rx.await;
        drop(documents.lock("test_seed").await);
    });
    let superseded = backend.until_document_superseded(&uri, seed, |_| true);
    tokio::pin!(superseded);
    assert!(
        crate::rt::timeout(short, &mut superseded).await.is_err(),
        "the seed waits for its start signal",
    );

    let elsewhere = backend.documents.lock("test_holder").await;
    start_tx.send(()).unwrap();
    assert!(
        crate::rt::timeout(short, &mut superseded).await.is_err(),
        "the seed queues on the map behind the current holder",
    );
    backend.live_publication_advanced.notify_waiters();
    assert!(
        crate::rt::timeout(short, &mut superseded).await.is_err(),
        "a publication elsewhere sends the loop back to its currency check",
    );
    drop(elsewhere);
    assert_eq!(
        crate::rt::timeout(Duration::from_secs(5), &mut superseded)
            .await
            .expect("the released map reaches the seed and the check in turn"),
        Some(()),
    );
    assert!(
        crate::rt::timeout(short, backend.documents.lock("test_after"))
            .await
            .is_ok(),
        "the map is free once the seed has finished",
    );
}
