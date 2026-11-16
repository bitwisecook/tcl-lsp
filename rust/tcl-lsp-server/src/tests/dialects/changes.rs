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

//! Dialect changes racing edits, diagnostics and the index.

use super::*;

/// A session-default change that snapshots a
/// document just before an edit must retry against that edit's current
/// text/revision instead of silently abandoning the configuration change.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn default_dialect_change_retries_after_concurrent_edit_1854() {
    use tower_lsp_server::ls_types::{
        TextDocumentContentChangeEvent, VersionedTextDocumentIdentifier,
    };

    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///default-dialect-edit-race-1854.tcl").unwrap();
    let initial = "set x 1\n";
    backend
        .db_set_source(&uri, initial, "tcl8.6".to_owned())
        .await
        .unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(initial.to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("plaintext".to_owned()),
    );

    let mut default = backend.default_dialect.lock().await;
    *default = "tcl9.0".to_owned();
    let resolving = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        async move { backend.reresolve_open_document_dialects().await.unwrap() }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if backend
                .documents
                .lock("test")
                .await
                .get(&uri)
                .is_some_and(DocumentState::dialect_resolution_pending)
            {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("configuration re-resolution must mark the document dirty");

    backend
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "set x 2\n".to_owned(),
            }],
        })
        .await;
    drop(default);
    crate::rt::timeout(std::time::Duration::from_secs(5), resolving)
        .await
        .expect("dialect resolution must retry after the edit")
        .expect("dialect resolver panicked");

    let docs = backend.documents.lock("test").await;
    let current = docs.get(&uri).expect("the document remains open");
    assert_eq!(current.version, Some(2));
    assert_eq!(current.text.as_ref(), "set x 2\n");
    assert_eq!(current.dialect, "tcl9.0");
    assert!(!current.dialect_resolution_pending());
}

/// If a hint-changing edit loses deferred
/// publication to a later unrelated edit, the later revision must inherit
/// and resolve the dirty hint rather than leaving the combined buffer on
/// the old dialect.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn superseding_edit_carries_dialect_hint_dirtiness_1854() {
    use tower_lsp_server::ls_types::{
        TextDocumentContentChangeEvent, VersionedTextDocumentIdentifier,
    };

    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///hint-edit-race-1854.tcl").unwrap();
    let initial = "set x 1\n";
    backend
        .db_set_source(&uri, initial, "tcl8.6".to_owned())
        .await
        .unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(initial.to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("tcl".to_owned()),
    );
    let publication = backend.live_publication_gate.lock().await;
    let hinted = format!("# tcl-dialect: tcl9.0\n{}set x 1\n", "\n".repeat(24));
    let first = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        let hinted = hinted.clone();
        async move {
            backend
                .did_change(DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier { uri, version: 2 },
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: None,
                        range_length: None,
                        text: hinted,
                    }],
                })
                .await;
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let ready = backend
                .documents
                .lock("test")
                .await
                .get(&uri)
                .is_some_and(|doc| doc.version == Some(2) && doc.dialect_resolution_pending());
            if ready {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the hint edit must become visible and dirty before publication");

    let second = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend
                .did_change(DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier { uri, version: 3 },
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: Some(Range::new(Position::new(25, 7), Position::new(25, 7))),
                        range_length: Some(0),
                        text: "\nset y 2".to_owned(),
                    }],
                })
                .await;
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if backend
                .documents
                .lock("test")
                .await
                .get(&uri)
                .is_some_and(|doc| doc.version == Some(3))
            {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the unrelated edit must supersede the stalled hint edit");
    drop(publication);
    crate::rt::timeout(std::time::Duration::from_secs(5), first)
        .await
        .expect("the superseded hint handler must finish")
        .expect("the hint handler panicked");
    crate::rt::timeout(std::time::Duration::from_secs(5), second)
        .await
        .expect("the authoritative edit must finish")
        .expect("the authoritative edit panicked");

    let docs = backend.documents.lock("test").await;
    let current = docs.get(&uri).expect("the document remains open");
    assert_eq!(current.version, Some(3));
    assert_eq!(current.dialect, "tcl9.0");
    assert!(current.text.contains("set y 2"));
    assert!(!current.dialect_resolution_pending());
}

/// A diagnostics worker that captured the
/// current text under the old dialect must lose publication authority when
/// a configuration-only dialect change starts a new analysis generation.
#[tokio::test]
async fn dialect_change_invalidates_in_flight_diagnostics_1854() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///dialect-diag-generation-1854.tcl").unwrap();
    let text = "proc helper {} {}\n";
    backend
        .db_set_source(&uri, text, "tcl8.6".to_owned())
        .await
        .unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(text.to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("tcl".to_owned()),
    );
    let old_revision = backend
        .documents
        .lock("test")
        .await
        .get(&uri)
        .expect("the document is open")
        .revision;
    let old_delivery = DeliveryCtx {
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
        currency: DiagCurrency::Open(old_revision),
        version: Some(1),
        client_supports_pull: false,
    };

    *backend.default_dialect.lock().await = "tcl9.0".to_owned();
    backend.reresolve_open_document_dialects().await.unwrap();

    let docs = backend.documents.lock("test").await;
    let current = docs.get(&uri).expect("the document remains open");
    assert_eq!(current.dialect, "tcl9.0");
    assert_eq!(current.version, Some(1), "the editor version is unchanged");
    assert!(
        current.revision > old_revision,
        "a dialect-only change must advance analysis currency",
    );
    assert!(
        !old_delivery.is_current(&docs).await,
        "old-dialect diagnostics must not publish after the change",
    );
}

/// Changing dialect retires facts analysed
/// under the old grammar. The document may become Salsa-ready immediately,
/// but it is not Indexed until replacement facts for that same live
/// revision have been published.
#[tokio::test]
async fn dialect_change_rebuilds_index_before_indexed_readiness_1854() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///dialect-reindex-1854.tcl").unwrap();
    let text = "proc current {} {}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(text.to_owned(), "tcl9.0".to_owned(), 2)
            .with_publication_pending(),
    );

    assert!(
        backend
            .commit_live_dialect("test_dialect_reindex", &uri, text, "tcl9.0", 0)
            .await
            .unwrap(),
        "the current dialect revision must publish",
    );
    assert_eq!(
        backend
            .documents
            .lock("test")
            .await
            .get(&uri)
            .expect("the document remains live")
            .publication,
        DocumentPublication::Indexed,
    );
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .workspace_command_exists("::current"),
        "Indexed readiness must include replacement facts under the new dialect",
    );
}

/// A dialect rebuild first publishes a
/// standalone index view. Its old source-site seed marker must disappear
/// atomically with that replacement, or reconciliation mistakes the old
/// qualified view for the one still present and never reapplies it.
#[tokio::test]
async fn dialect_change_invalidates_rehoming_seed_before_reconciliation_1854() {
    let backend = test_backend();
    let caller = Uri::from_str("file:///dialect-rehome/a.tcl").unwrap();
    let sourced = Uri::from_str("file:///dialect-rehome/b.tcl").unwrap();
    let sourced_text = "proc helper {} {}\n";
    register(&backend, &sourced, sourced_text).await;
    register(&backend, &caller, "namespace eval ::ns { source b.tcl }\n").await;
    backend.refresh_source_rehoming().await.unwrap();
    assert_eq!(
        backend
            .rehomed_source_seeds
            .lock()
            .await
            .get(sourced.as_str())
            .cloned(),
        Some(vec!["::ns".to_owned()]),
        "precondition: the sourced file carries its applied namespace seed",
    );
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .workspace_command_exists("::ns::helper"),
        "precondition: the qualified source-site view is published",
    );

    {
        let mut docs = backend.documents.lock("test").await;
        let doc = docs.get_mut(&sourced).expect("the sourced buffer is open");
        doc.dialect = "tcl9.0".to_owned();
        doc.publication = DocumentPublication::Pending;
    }
    assert!(
        backend
            .commit_live_dialect("test_dialect_rehome", &sourced, sourced_text, "tcl9.0", 0,)
            .await
            .unwrap(),
        "the current dialect revision must publish",
    );
    assert!(
        !backend
            .rehomed_source_seeds
            .lock()
            .await
            .contains_key(sourced.as_str()),
        "standalone replacement and seed invalidation are one transaction",
    );

    backend.refresh_source_rehoming().await.unwrap();
    assert_eq!(
        backend
            .rehomed_source_seeds
            .lock()
            .await
            .get(sourced.as_str())
            .cloned(),
        Some(vec!["::ns".to_owned()]),
        "the next reconciliation must reapply the namespace seed",
    );
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .workspace_command_exists("::ns::helper"),
        "qualified definitions survive the dialect change",
    );
}
