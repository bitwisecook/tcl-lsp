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

//! Snapshot-drain intent shared by live and disk publication.

use super::*;

/// A stream of overlapping request snapshots
/// must not keep a live source publication short of Salsa forever. Once a
/// publisher announces drain intent, later snapshot requests queue; the
/// finite pre-existing set retires and the writer runs before readers are
/// admitted again.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn live_publication_intent_stops_new_snapshots_until_drain_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///snapshot-drain-1854.tcl").unwrap();
    let old_text = "set value old\n";
    let live_text = "set value current\n";
    backend
        .db_set_source(&uri, old_text, "tcl8.6".to_owned())
        .await
        .unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(live_text.to_owned(), "tcl8.6".to_owned(), 2)
            .with_language_id("tcl".to_owned())
            .with_publication_pending(),
    );

    let existing = backend.db.snapshot("issue-1854-pre-drain-reader").await;
    let publishing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend
                .commit_live_source("test_snapshot_drain", &uri, live_text, "tcl8.6", 0, false)
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
    .expect("the live publisher must establish drain intent");

    let database = crate::rt::timeout(std::time::Duration::from_secs(5), backend.db.lock())
        .await
        .expect("the test must briefly win transient database contention");
    crate::rt::sleep(std::time::Duration::from_millis(25)).await;
    assert!(
        backend
            .db
            .snapshots
            .publication_intent
            .load(std::sync::atomic::Ordering::Acquire)
            != 0,
        "transient database contention must not reopen the snapshot stream",
    );
    drop(database);

    let mut later_reader = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        async move { backend.db.snapshot("issue-1854-post-drain-reader").await }
    });
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), &mut later_reader)
            .await
            .is_err(),
        "a later reader must queue instead of extending the snapshot stream",
    );

    drop(existing);
    assert!(
        crate::rt::timeout(std::time::Duration::from_secs(5), publishing)
            .await
            .expect("publication must finish as soon as the old reader retires")
            .expect("the publication task must not panic"),
        "the current live source must publish",
    );
    let later = crate::rt::timeout(std::time::Duration::from_secs(5), later_reader)
        .await
        .expect("queued readers must resume after publication")
        .expect("the reader task must not panic");
    drop(later);

    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    assert_eq!(
        files.get(&uri).map(|file| file.text(&*db).as_str()),
        Some(live_text),
        "the writer must run between the old and later snapshots",
    );
}

/// A live publisher entering while disk work retains drain ownership must
/// share the reader barrier rather than panic or release it prematurely.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn live_publication_shares_disk_snapshot_drain_intent_1854() {
    let backend = Arc::new(test_backend());
    let disk_uri = Uri::from_str("file:///disk-shared-drain-1854.tcl").unwrap();
    let disk_text = "proc disk_shared_drain {} {}\n";
    let disk_analysis = Analyser::new().analyse(disk_text, "tcl8.6").clone();
    let live_uri = Uri::from_str("file:///live-shared-drain-1854.tcl").unwrap();
    let live_text = "set live current\n";
    backend
        .db_set_source(&live_uri, "set live old\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    backend.documents.lock("test").await.insert(
        live_uri.clone(),
        DocumentState::new(live_text.to_owned(), "tcl8.6".to_owned()).with_publication_pending(),
    );

    let existing = backend.db.snapshot("issue-1854-shared-drain-reader").await;
    let mut disk_publishing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let disk_uri = disk_uri.clone();
        async move {
            backend
                .publish_disk_results(
                    &[(
                        disk_uri,
                        disk_text.to_owned(),
                        "tcl8.6".to_owned(),
                        disk_analysis,
                    )],
                    &[],
                    DiskRemovalPolicy::ClosedOnly,
                    "test_disk_shared_drain",
                )
                .await
                .unwrap()
        }
    });
    wait_for_publication_intents_1854(&backend, 1, "disk publication must claim intent").await;

    let mut live_publishing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let live_uri = live_uri.clone();
        async move {
            backend
                .commit_live_source(
                    "test_live_shared_drain",
                    &live_uri,
                    live_text,
                    "tcl8.6",
                    0,
                    false,
                )
                .await
                .unwrap()
        }
    });
    wait_for_publication_intents_1854(
        &backend,
        2,
        "both publishers must own the shared drain intent",
    )
    .await;
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), &mut live_publishing)
            .await
            .is_err(),
        "the live publisher must wait for the pre-existing snapshot",
    );
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(100), &mut disk_publishing)
            .await
            .is_err(),
        "the disk publisher must retain its own drain ownership",
    );

    drop(existing);
    assert!(
        crate::rt::timeout(std::time::Duration::from_secs(5), live_publishing)
            .await
            .expect("the live publication must finish after the reader retires")
            .expect("the live publisher must not panic"),
        "the current live source must publish",
    );
    assert_eq!(
        crate::rt::timeout(std::time::Duration::from_secs(5), disk_publishing)
            .await
            .expect("the disk publication must finish after the reader retires")
            .expect("the disk publisher must not panic"),
        vec![disk_uri.clone()],
    );

    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    assert_eq!(
        files.get(&live_uri).map(|file| file.text(&*db).as_str()),
        Some(live_text),
    );
    assert_eq!(
        files.get(&disk_uri).map(|file| file.text(&*db).as_str()),
        Some(disk_text),
    );
}
