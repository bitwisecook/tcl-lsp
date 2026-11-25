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

//! Watched-file batches and their effect on closed files and the index.

use super::*;

#[test]
fn is_config_file_recognises_project_and_user_config() {
    assert!(is_config_file(
        &Uri::from_str("file:///ws/.tcl-lsp.ini").unwrap()
    ));
    assert!(is_config_file(
        &Uri::from_str("file:///home/me/.config/tcl-lsp/config.ini").unwrap()
    ));
    assert!(!is_config_file(
        &Uri::from_str("file:///ws/foo.tcl").unwrap()
    ));
    assert!(!is_config_file(
        &Uri::from_str("file:///ws/settings.ini").unwrap()
    ));
}

/// An external on-disk change to a closed, badged file refreshes its badge.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watched_change_refreshes_closed_file_badge() {
    let root = unique_scratch_dir("watched-refresh");
    let on_disk = root.join("warn.tcl");
    // Starts clean; a prior open/close primed an (empty) badge.
    std::fs::write(&on_disk, "set x 1\n").unwrap();
    let backend = test_backend();
    let uri = crate::canonical_file_uri(&on_disk).unwrap();
    backend
        .db_set_source(&uri, "set x 1\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    backend.pull_diag_cache.lock().await.insert(
        uri.clone(),
        PullDiagEntry {
            currency: None,
            result_id: next_pull_diag_result_id(),
            revision: u64::MAX,
            diagnostics: Vec::new(),
        },
    );
    // An external edit introduces a W211.
    std::fs::write(&on_disk, "proc foo {} { set y 1 }\n").unwrap();

    backend
        .did_change_watched_files(DidChangeWatchedFilesParams {
            changes: vec![tower_lsp_server::ls_types::FileEvent {
                uri: uri.clone(),
                typ: FileChangeType::CHANGED,
            }],
        })
        .await;

    let cache = backend.pull_diag_cache.lock().await;
    let entry = cache.get(&uri).expect("badge stays cached");
    assert!(
        entry.diagnostics.iter().any(|d| matches!(
            &d.code,
            Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W211"
        )),
        "an external change must refresh a closed file's badge: {:?}",
        entry.diagnostics,
    );
    drop(cache);
    std::fs::remove_dir_all(&root).ok();
}

/// Deleting a closed, badged file on disk clears its badge.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watched_delete_clears_closed_file_badge() {
    let root = unique_scratch_dir("watched-delete");
    let on_disk = root.join("warn.tcl");
    std::fs::write(&on_disk, "proc foo {} { set y 1 }\n").unwrap();
    let backend = test_backend();
    let uri = crate::canonical_file_uri(&on_disk).unwrap();
    backend
        .db_set_source(&uri, "proc foo {} { set y 1 }\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    backend.pull_diag_cache.lock().await.insert(
        uri.clone(),
        PullDiagEntry {
            currency: None,
            result_id: next_pull_diag_result_id(),
            revision: u64::MAX,
            diagnostics: vec![tower_lsp_server::ls_types::Diagnostic::default()],
        },
    );
    std::fs::remove_file(&on_disk).unwrap();

    backend
        .did_change_watched_files(DidChangeWatchedFilesParams {
            changes: vec![tower_lsp_server::ls_types::FileEvent {
                uri: uri.clone(),
                typ: FileChangeType::DELETED,
            }],
        })
        .await;

    assert!(
        !backend.pull_diag_cache.lock().await.contains_key(&uri),
        "deleting a closed file must clear its badge",
    );
    std::fs::remove_dir_all(&root).ok();
}

/// A single `did_change_watched_files` event batch carrying a CREATED, a
/// CHANGED and a DELETED file together must land on the same end state a
/// one-file-at-a-time serial path would produce — the
/// `workspace_index` and salsa db reflecting exactly the final on-disk
/// population, in one batched pass rather than three sequential full
/// analyses.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watched_files_batch_applies_created_changed_and_deleted_together() {
    let root = unique_scratch_dir("watched-batch");
    let created_path = root.join("created.tcl");
    let changed_path = root.join("changed.tcl");
    let deleted_path = root.join("deleted.tcl");
    std::fs::write(&created_path, "proc created_proc {} {}\n").unwrap();
    std::fs::write(&changed_path, "proc changed_proc {} {}\n").unwrap();
    std::fs::write(&deleted_path, "proc deleted_proc {} {}\n").unwrap();

    let backend = test_backend();
    let created_uri = crate::canonical_file_uri(&created_path).unwrap();
    let changed_uri = crate::canonical_file_uri(&changed_path).unwrap();
    let deleted_uri = crate::canonical_file_uri(&deleted_path).unwrap();

    // Seed the pre-event state: `changed.tcl` and `deleted.tcl` are
    // already indexed (as a prior scan would have left them) under their
    // *old* content — `changed.tcl`'s on-disk copy above is already the
    // new content, standing in for the edit the CHANGED event reports;
    // `deleted.tcl`'s on-disk file is removed below before the event
    // fires, standing in for the delete. `created.tcl` starts entirely
    // unindexed.
    for (uri, old_src) in [
        (&changed_uri, "proc old_changed_proc {} {}\n"),
        (&deleted_uri, "proc deleted_proc {} {}\n"),
    ] {
        backend
            .db_set_source(uri, old_src, "tcl8.6".to_owned())
            .await
            .unwrap();
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(old_src, "tcl8.6").clone();
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &analysis);
    }
    std::fs::remove_file(&deleted_path).unwrap();

    backend
        .did_change_watched_files(DidChangeWatchedFilesParams {
            changes: vec![
                tower_lsp_server::ls_types::FileEvent {
                    uri: created_uri.clone(),
                    typ: FileChangeType::CREATED,
                },
                tower_lsp_server::ls_types::FileEvent {
                    uri: changed_uri.clone(),
                    typ: FileChangeType::CHANGED,
                },
                tower_lsp_server::ls_types::FileEvent {
                    uri: deleted_uri.clone(),
                    typ: FileChangeType::DELETED,
                },
            ],
        })
        .await;

    let index = backend.workspace_index.read().await;
    assert!(
        !index
            .proc_definitions("created_proc", "created_proc")
            .is_empty(),
        "the CREATED file must be indexed",
    );
    assert!(
        !index
            .proc_definitions("changed_proc", "changed_proc")
            .is_empty(),
        "the CHANGED file's new content must be indexed",
    );
    assert!(
        index
            .proc_definitions("old_changed_proc", "old_changed_proc")
            .is_empty(),
        "the CHANGED file's stale content must not survive the batch",
    );
    assert!(
        index
            .proc_definitions("deleted_proc", "deleted_proc")
            .is_empty(),
        "the DELETED file must be dropped from the index",
    );
    drop(index);

    let files = backend.db_files.lock().await;
    assert!(
        files.contains_key(&created_uri),
        "the CREATED file must gain a salsa SourceFile input",
    );
    assert!(
        files.contains_key(&changed_uri),
        "the CHANGED file keeps its salsa SourceFile input",
    );
    assert!(
        !files.contains_key(&deleted_uri),
        "the DELETED file's salsa SourceFile input must be retired",
    );
    drop(files);

    std::fs::remove_dir_all(&root).ok();
}

/// A watched `DELETED` → `CREATED` pair — what a `git checkout` or
/// branch switch fires for every changed file — must not allocate a second
/// salsa input per cycle.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watched_delete_then_create_reuses_source_input() {
    let root = unique_scratch_dir("watched-recreate");
    let on_disk = root.join("switched.tcl");
    std::fs::write(&on_disk, "proc foo {} {}\n").unwrap();
    let backend = test_backend();
    let uri = crate::canonical_file_uri(&on_disk).unwrap();
    backend
        .db_set_source(&uri, "proc foo {} {}\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    let before = *backend
        .db_files
        .lock()
        .await
        .get(&uri)
        .expect("the scanned file is tracked");

    // The branch switch: the file disappears, then reappears with new content.
    std::fs::remove_file(&on_disk).unwrap();
    backend
        .did_change_watched_files(DidChangeWatchedFilesParams {
            changes: vec![tower_lsp_server::ls_types::FileEvent {
                uri: uri.clone(),
                typ: FileChangeType::DELETED,
            }],
        })
        .await;
    std::fs::write(&on_disk, "proc bar {} {}\n").unwrap();
    backend
        .did_change_watched_files(DidChangeWatchedFilesParams {
            changes: vec![tower_lsp_server::ls_types::FileEvent {
                uri: uri.clone(),
                typ: FileChangeType::CREATED,
            }],
        })
        .await;

    let after = *backend
        .db_files
        .lock()
        .await
        .get(&uri)
        .expect("the re-created file is tracked again");
    assert!(
        before == after,
        "a delete/re-create cycle must reuse the retired input, not leak it",
    );
    assert!(
        backend.db_tombstones.lock().await.is_empty(),
        "the revived input must leave the tombstone map",
    );
    std::fs::remove_dir_all(&root).ok();
}

/// `did_change_watched_files` drops a deleted file from the cross-document
/// index and re-indexes a (closed) changed file from disk.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watched_files_deletion_drops_index_entry() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///watched-gone.tcl").unwrap();
    // Index a file that is NOT open (no `documents` entry).
    {
        let mut a = Analyser::new();
        let analysis = a.analyse("proc gone {} {}\n", "tcl8.6").clone();
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &analysis);
    }
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .document_uris()
            .iter()
            .any(|u| u == uri.as_str()),
        "precondition: file is indexed",
    );

    backend
        .did_change_watched_files(DidChangeWatchedFilesParams {
            changes: vec![tower_lsp_server::ls_types::FileEvent {
                uri: uri.clone(),
                typ: FileChangeType::DELETED,
            }],
        })
        .await;

    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .document_uris()
            .iter()
            .any(|u| u == uri.as_str()),
        "a DELETED watched-file event must drop the index entry",
    );
}

/// A watched-file create/change/delete shifts the cross-file resolution domain
/// without an open document's own edit, so open documents with cross-file
/// diagnostics enabled must be rescheduled — otherwise a push-diagnostic client
/// keeps stale W123/arity after a defining file disappears (git checkout /
/// delete).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watched_file_delete_reschedules_open_xc_documents() {
    let backend = test_backend();
    backend.feature_toggles.lock().await.apply(
        serde_json::json!({ "crossFileResolution": true })
            .as_object()
            .unwrap(),
    );
    // An open caller with crossFileResolution enabled.
    let caller = Uri::from_str("file:///caller.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        caller.clone(),
        DocumentState::new("helper x\n".to_owned(), "tcl8.6".to_owned()),
    );

    // A watched, non-open file is deleted — its procs leave the `Project`.
    let deleted = Uri::from_str("file:///lib.tcl").unwrap();
    backend
        .did_change_watched_files(DidChangeWatchedFilesParams {
            changes: vec![tower_lsp_server::ls_types::FileEvent {
                uri: deleted,
                typ: FileChangeType::DELETED,
            }],
        })
        .await;

    // The open caller was rescheduled — a diagnostics slot now exists for it.
    assert!(
        backend.diag_slots.lock().await.contains_key(&caller),
        "an open crossFileResolution document must be rescheduled after a watched-file delete",
    );
}

/// The always-on W120/W123 workspace refinement
/// (`refine_workspace_w120`/`refine_workspace_w123`) is not gated by
/// `crossFileResolution`, so a watched-file domain change must reschedule
/// *every* open document — not just `crossFileResolution`-enabled ones.
/// Rescheduling only the narrower cross-file subset leaves a plain
/// document's stale W120 (e.g. from a `source` ancestor that only just
/// appeared on disk) unrefreshed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watched_file_delete_reschedules_non_xc_document_too() {
    let backend = test_backend();
    // No `crossFileResolution` toggle — a plain document.
    let caller = Uri::from_str("file:///plain-caller.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        caller.clone(),
        DocumentState::new("helper x\n".to_owned(), "tcl8.6".to_owned()),
    );

    let deleted = Uri::from_str("file:///lib.tcl").unwrap();
    backend
        .did_change_watched_files(DidChangeWatchedFilesParams {
            changes: vec![tower_lsp_server::ls_types::FileEvent {
                uri: deleted,
                typ: FileChangeType::DELETED,
            }],
        })
        .await;

    assert!(
        backend.diag_slots.lock().await.contains_key(&caller),
        "a plain (non-crossFileResolution) open document must also be rescheduled \
             after a watched-file delete, since the W120/W123 workspace \
             refinement is unconditional",
    );
}

/// The registration the server actually sends: the case-folded source glob
/// plus the project config file, all three change kinds.
#[test]
fn watcher_registration_covers_sources_and_project_config() {
    let opts = DidChangeWatchedFilesRegistrationOptions {
        watchers: vec![
            FileSystemWatcher {
                glob_pattern: GlobPattern::String(tcl_source_watch_glob()),
                kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
            },
            FileSystemWatcher {
                glob_pattern: GlobPattern::String(PROJECT_CONFIG_GLOB.to_owned()),
                kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
            },
        ],
    };
    let json = serde_json::to_value(&opts).expect("registration serialises");
    let watchers = json["watchers"].as_array().expect("watchers array");
    assert_eq!(watchers.len(), 2, "{json}");
    assert!(
        watchers[0]["globPattern"]
            .as_str()
            .is_some_and(|g| g.contains("[tT][cC][lL]")),
        "{json}",
    );
    assert_eq!(watchers[1]["globPattern"], PROJECT_CONFIG_GLOB);
    // `.tcl-lsp.ini` is what the config live-reload branch recognises.
    assert!(is_config_file(
        &"file:///w/.tcl-lsp.ini".parse::<Uri>().unwrap()
    ));
}
