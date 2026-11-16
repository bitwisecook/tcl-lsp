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

use super::*;

#[test]
fn virtual_issuer_keeps_authority_counted_data_and_refuses_invalid_uri() {
    let store = MemoryStore::new();
    let selected = store
        .document_uri_path("file://Srv/share/%C3%A9%252F.tcl")
        .unwrap();
    assert_eq!(selected.kind(), SourceUriPathKind::Virtual);
    assert_eq!(selected.path(), Path::new("//Srv/share/é%2F.tcl"));
    store.validate_document_uri_path(&selected).unwrap();
    for raw in [
        "file:///a%FF",
        "file:///a%00",
        "file:///a%2Fb",
        "file://user@host/a",
        "file:///a?query",
    ] {
        store.upsert_uri(raw, "/registered/does-not-forgive.tcl", Vec::new());
        assert!(store.document_uri_path(raw).is_err(), "{raw}");
    }
}

#[test]
fn registration_is_stronger_and_old_receipt_cannot_revive_or_cross_provider() {
    let store = MemoryStore::new();
    let uri = "file:///browser/a.tcl";
    let derived = store.document_uri_path(uri).unwrap();
    store.upsert_uri(uri, derived.path(), b"one".to_vec());
    assert_eq!(
        store.validate_document_uri_path(&derived),
        Err(SourceUriPathUnavailable::WithdrawnMapping)
    );
    let first = store.document_uri_path(uri).unwrap();
    store.upsert_uri(uri, first.path(), b"two".to_vec());
    store.validate_document_uri_path(&first).unwrap();
    store.upsert_uri(uri, "/different/a.tcl", b"three".to_vec());
    store.upsert_uri(uri, first.path(), b"four".to_vec());
    assert_eq!(
        store.validate_document_uri_path(&first),
        Err(SourceUriPathUnavailable::WithdrawnMapping)
    );
    let foreign = MemoryStore::new();
    foreign.upsert_uri(uri, first.path(), b"four".to_vec());
    assert_eq!(
        foreign.validate_document_uri_path(&first),
        Err(SourceUriPathUnavailable::ForeignProvider)
    );
    let current = store.document_uri_path(uri).unwrap();
    drop(store);
    assert_eq!(
        current.check_current(),
        Err(SourceUriPathUnavailable::ProviderUnavailable)
    );
}

#[test]
fn original_and_service_aliases_share_actual_row_and_withdraw_together() {
    let store = MemoryStore::new();
    let raw = "file:///browser/%61.tcl";
    let normalized = "file:///browser/a.tcl";
    store.upsert_uri_aliases(&[raw, normalized], "/actual/mounted.tcl", b"body".to_vec());
    let first = store.document_uri_path(raw).unwrap();
    let second = store.document_uri_path(normalized).unwrap();
    assert_eq!(first.path(), second.path());
    let mut calls = 0;
    store
        .with_current_document_paths(&[&first, &second], &mut || calls += 1)
        .unwrap();
    assert_eq!(calls, 1);
    store.upsert_uri_aliases(&[raw, normalized], first.path(), b"edited".to_vec());
    store.validate_document_uri_path(&first).unwrap();
    assert!(store.remove_uri(normalized));
    assert_eq!(
        store.validate_document_uri_path(&first),
        Err(SourceUriPathUnavailable::WithdrawnMapping)
    );
    assert!(store.path_for_uri(raw).is_none());
    assert_eq!(
        store.with_current_document_paths(&[&first], &mut || calls += 1),
        Err(SourceUriPathUnavailable::WithdrawnMapping)
    );
    assert_eq!(calls, 1);
}

#[test]
fn publication_holds_the_actual_mapping_until_continuation_returns() {
    let store = Arc::new(MemoryStore::new());
    let uri = "file:///browser/leased.tcl";
    store.upsert_uri(uri, "/actual/first.tcl", Vec::new());
    let selected = store.document_uri_path(uri).unwrap();
    let (attempt_tx, attempt_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let other = Arc::clone(&store);
    let mut worker = None;
    store
        .with_current_document_paths(&[&selected], &mut || {
            let other = Arc::clone(&other);
            let attempt_tx = attempt_tx.clone();
            let done_tx = done_tx.clone();
            worker = Some(std::thread::spawn(move || {
                attempt_tx.send(()).unwrap();
                other.upsert_uri(uri, "/actual/second.tcl", Vec::new());
                done_tx.send(()).unwrap();
            }));
            attempt_rx.recv().unwrap();
            assert!(matches!(
                done_rx.try_recv(),
                Err(std::sync::mpsc::TryRecvError::Empty)
            ));
        })
        .unwrap();
    worker.take().unwrap().join().unwrap();
    done_rx.recv().unwrap();
    assert_eq!(
        store.validate_document_uri_path(&selected),
        Err(SourceUriPathUnavailable::WithdrawnMapping)
    );
}

#[cfg(unix)]
#[test]
fn different_counted_path_spelling_is_a_new_registration() {
    let store = MemoryStore::new();
    let uri = "file:///browser/counted.tcl";
    store.upsert_uri(uri, "/actual//counted.tcl", b"one".to_vec());
    let original = store.document_uri_path(uri).unwrap();
    assert_eq!(
        original.path().as_os_str(),
        std::ffi::OsStr::new("/actual//counted.tcl")
    );
    store.upsert_uri(uri, "/actual/counted.tcl", b"two".to_vec());
    assert_eq!(
        store.validate_document_uri_path(&original),
        Err(SourceUriPathUnavailable::WithdrawnMapping)
    );
    let current = store.document_uri_path(uri).unwrap();
    assert_eq!(
        current.path().as_os_str(),
        std::ffi::OsStr::new("/actual/counted.tcl")
    );
}

#[cfg(target_os = "wasi")]
#[test]
fn actual_wasi_store_issues_guest_path_without_native_host_claim() {
    let selected = NativeStore
        .document_uri_path("file:///workspace/a.tcl")
        .unwrap();
    assert_eq!(selected.kind(), SourceUriPathKind::WasiGuest);
    assert_eq!(selected.path(), Path::new("/workspace/a.tcl"));
    NativeStore.validate_document_uri_path(&selected).unwrap();
}
