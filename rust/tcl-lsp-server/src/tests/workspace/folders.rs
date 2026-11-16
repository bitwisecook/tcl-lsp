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

//! Adding and removing workspace folders.

use super::*;

/// Folder-add variant of the test above: adding a
/// workspace folder scans it into `workspace_index` / `package_resolver`
/// (the always-on W120/W123 refinement's inputs), so every open document
/// must be rescheduled — not just `crossFileResolution`-enabled ones.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_folder_add_reschedules_non_xc_document_too() {
    let backend = test_backend();
    let caller = Uri::from_str("file:///plain-caller2.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        caller.clone(),
        DocumentState::new("helper x\n".to_owned(), "tcl8.6".to_owned()),
    );
    let root = unique_scratch_dir("folder-add-reschedule");

    backend
        .did_change_workspace_folders(DidChangeWorkspaceFoldersParams {
            event: tower_lsp_server::ls_types::WorkspaceFoldersChangeEvent {
                added: vec![tower_lsp_server::ls_types::WorkspaceFolder {
                    uri: Uri::from_file_path(&root).unwrap(),
                    name: "folder-add-reschedule".to_owned(),
                }],
                removed: Vec::new(),
            },
        })
        .await;

    assert!(
        backend.diag_slots.lock().await.contains_key(&caller),
        "a plain (non-crossFileResolution) open document must also be rescheduled \
             after a workspace-folder add",
    );
}

/// Removing a workspace folder drops index entries for its (closed) files
/// while leaving files under other folders intact.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_folder_removal_drops_index_under_folder() {
    let backend = test_backend();
    let gone = Uri::from_str("file:///proj-a/lib.tcl").unwrap();
    let kept = Uri::from_str("file:///proj-b/lib.tcl").unwrap();
    {
        let mut a = Analyser::new();
        let analysis = a.analyse("proc p {} {}\n", "tcl8.6").clone();
        let mut index = backend.workspace_index.write().await;
        index.add_document(gone.as_str(), &analysis);
        index.add_document(kept.as_str(), &analysis);
    }
    *backend.workspace_folders.lock().await = vec![
        Uri::from_str("file:///proj-a").unwrap(),
        Uri::from_str("file:///proj-b").unwrap(),
    ];

    backend
        .did_change_workspace_folders(DidChangeWorkspaceFoldersParams {
            event: tower_lsp_server::ls_types::WorkspaceFoldersChangeEvent {
                added: Vec::new(),
                removed: vec![tower_lsp_server::ls_types::WorkspaceFolder {
                    uri: Uri::from_str("file:///proj-a").unwrap(),
                    name: "proj-a".to_owned(),
                }],
            },
        })
        .await;

    let uris = backend.workspace_index.read().await.document_uris();
    assert!(
        !uris.iter().any(|u| u == gone.as_str()),
        "files under the removed folder must be dropped",
    );
    assert!(
        uris.iter().any(|u| u == kept.as_str()),
        "files under a retained folder must remain",
    );
    // The folder list itself no longer carries the removed root.
    assert!(
        !backend
            .workspace_folders
            .lock()
            .await
            .iter()
            .any(|f| f.as_str() == "file:///proj-a"),
    );
}

/// Removing a workspace folder must clear the index, the salsa sources,
/// the rehoming seeds and the diagnostics badge of the closed files it
/// takes with it — *and* `last_semantic_tokens`,
/// `semantic_tokens_refresh_asked` and `workspace_class_analyses`, which
/// would otherwise hold an entry per file for the process's life even
/// though the file is no longer in any workspace folder.  That asymmetry
/// is what [`Backend::forget_uri_states`] makes impossible.
///
/// The negatives matter as much as the positive: a file under a folder that
/// stayed keeps everything, and so does an **open** document under the
/// removed folder — its live buffer is the source of truth regardless of the
/// folder set, and that exclusion is deliberate.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_folder_removal_forgets_per_uri_caches_1300() {
    let backend = test_backend();
    let dropped = Uri::from_str("file:///drop-a/closed.tcl").unwrap();
    let still_open = Uri::from_str("file:///drop-a/open.tcl").unwrap();
    let kept = Uri::from_str("file:///drop-b/closed.tcl").unwrap();
    let src = "proc p {} {}\n";
    {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(src, "tcl8.6").clone();
        let mut index = backend.workspace_index.write().await;
        for uri in [&dropped, &still_open, &kept] {
            index.add_document(uri.as_str(), &analysis);
        }
    }
    backend.documents.lock("test").await.insert(
        still_open.clone(),
        DocumentState::new(src.to_owned(), "tcl8.6".to_owned()),
    );
    for uri in [&dropped, &still_open, &kept] {
        seed_per_uri_caches(&backend, uri).await;
    }
    *backend.workspace_folders.lock().await = vec![
        Uri::from_str("file:///drop-a").unwrap(),
        Uri::from_str("file:///drop-b").unwrap(),
    ];

    backend
        .did_change_workspace_folders(DidChangeWorkspaceFoldersParams {
            event: tower_lsp_server::ls_types::WorkspaceFoldersChangeEvent {
                added: Vec::new(),
                removed: vec![tower_lsp_server::ls_types::WorkspaceFolder {
                    uri: Uri::from_str("file:///drop-a").unwrap(),
                    name: "drop-a".to_owned(),
                }],
            },
        })
        .await;

    assert_eq!(
        per_uri_cache_hits(&backend, &dropped).await,
        [false; 5],
        "a closed file under the removed folder must lose every per-URI cache",
    );
    assert_eq!(
        per_uri_cache_hits(&backend, &kept).await,
        [true; 5],
        "a file under a folder that stayed keeps its entries",
    );
    assert_eq!(
        per_uri_cache_hits(&backend, &still_open).await,
        [true; 5],
        "an open document under the removed folder keeps its state — that \
             exclusion is deliberate",
    );
    let uris = backend.workspace_index.read().await.document_uris();
    assert!(
        uris.iter().any(|u| u == still_open.as_str()),
        "…and keeps its index entry too",
    );
}

/// Removing a workspace folder shifts the salsa `Project` without an open
/// document's own edit, so open documents with cross-file diagnostics enabled
/// must be rescheduled (matching the watched-file path) — otherwise a
/// push-diagnostic client keeps stale cross-file results.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn folder_removal_reschedules_open_xc_documents() {
    let backend = test_backend();
    backend.feature_toggles.lock().await.apply(
        serde_json::json!({ "crossFileResolution": true })
            .as_object()
            .unwrap(),
    );
    let caller = Uri::from_str("file:///proj/caller.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        caller.clone(),
        DocumentState::new("helper x\n".to_owned(), "tcl8.6".to_owned()),
    );
    *backend.workspace_folders.lock().await = vec![Uri::from_str("file:///proj").unwrap()];

    backend
        .did_change_workspace_folders(DidChangeWorkspaceFoldersParams {
            event: tower_lsp_server::ls_types::WorkspaceFoldersChangeEvent {
                added: Vec::new(),
                removed: vec![tower_lsp_server::ls_types::WorkspaceFolder {
                    uri: Uri::from_str("file:///proj").unwrap(),
                    name: "proj".to_owned(),
                }],
            },
        })
        .await;

    assert!(
        backend.diag_slots.lock().await.contains_key(&caller),
        "an open crossFileResolution document must be rescheduled after a folder removal",
    );
}
