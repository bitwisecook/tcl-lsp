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

//! Retirement of a renamed URI's per-URI state.

use super::*;

/// One `workspace/didRenameFiles` notification for `pairs`.
fn rename_params(pairs: &[(&Uri, &Uri)]) -> RenameFilesParams {
    RenameFilesParams {
        files: pairs
            .iter()
            .map(|(old, new)| tower_lsp_server::ls_types::FileRename {
                old_uri: old.to_string(),
                new_uri: new.to_string(),
            })
            .collect(),
    }
}

/// `retire_renamed_uri` is the *sole* cleanup for a renamed-away path, so
/// it must drop the `rehomed_source_seeds` record along with every other
/// per-URI map — which every other retirement path (`did_close`, the
/// watched-file DELETED branch, the folder drop, the batch reindex, the
/// workspace scan) does.
///
/// The rename arrives on its own here, with **no** `didChangeWatchedFiles`
/// alongside it: a racing watch event masks the leak, so a test that sends
/// both proves nothing about this path.
#[tokio::test]
async fn did_rename_forgets_the_source_rehoming_seed_1298() {
    let backend = test_backend();
    let a = Uri::from_str("file:///rehome-rename/a.tcl").unwrap();
    let b = Uri::from_str("file:///rehome-rename/b.tcl").unwrap();
    let c = Uri::from_str("file:///rehome-rename/c.tcl").unwrap();
    let sourced = "proc helper {} {}\n";
    register(&backend, &b, sourced).await;
    register(&backend, &a, "namespace eval ::ns { source b.tcl }\n").await;
    backend.refresh_source_rehoming().await.unwrap();
    assert_eq!(
        backend
            .rehomed_source_seeds
            .lock()
            .await
            .get(b.as_str())
            .cloned(),
        Some(vec!["::ns".to_owned()]),
        "precondition: the sourced file is indexed under the ::ns source site",
    );

    // The client renames b.tcl → c.tcl: the file leaves its old path, the
    // buffer follows it to the new URI, and the workspace edit
    // `will_rename_files` handed back rewrites the `source` literal.
    backend.documents.lock("test").await.remove(&b);
    register(&backend, &c, sourced).await;
    backend
        .workspace_index
        .write()
        .await
        .remove_document(a.as_str());
    register(&backend, &a, "namespace eval ::ns { source c.tcl }\n").await;
    backend.did_rename_files(rename_params(&[(&b, &c)])).await;

    assert!(
        !backend
            .rehomed_source_seeds
            .lock()
            .await
            .contains_key(b.as_str()),
        "the renamed-away path's seed record must go with it",
    );

    backend.refresh_source_rehoming().await.unwrap();
    let desired: std::collections::BTreeMap<String, Vec<String>> = backend
        .workspace_index
        .read()
        .await
        .source_seed_map(resolve_source_edge)
        .into_iter()
        .map(|(uri, seeds)| (uri, seeds.into_iter().collect()))
        .collect();
    let recorded: std::collections::BTreeMap<String, Vec<String>> = backend
        .rehomed_source_seeds
        .lock()
        .await
        .clone()
        .into_iter()
        .collect();
    assert_eq!(
        recorded, desired,
        "reconciliation must have converged: anything recorded that the \
             index no longer asks for is re-queued as work on every round of \
             every later call, and dies at the `read_document` guard before it \
             can heal itself",
    );
    assert_eq!(
        recorded.keys().map(String::as_str).collect::<Vec<_>>(),
        vec![c.as_str()],
        "only the new path carries a seed record",
    );
}

#[tokio::test]
async fn renaming_a_sourced_pair_restores_the_rehoming_early_return_1298() {
    let backend = test_backend();
    let a = Uri::from_str("file:///rehome-moved/a.tcl").unwrap();
    let b = Uri::from_str("file:///rehome-moved/b.tcl").unwrap();
    let a2 = Uri::from_str("file:///moved/a.tcl").unwrap();
    let b2 = Uri::from_str("file:///moved/b.tcl").unwrap();
    register(&backend, &b, "proc helper {} {}\n").await;
    register(&backend, &a, "namespace eval ::ns { source b.tcl }\n").await;
    backend.refresh_source_rehoming().await.unwrap();
    assert!(
        !backend.rehomed_source_seeds.lock().await.is_empty(),
        "precondition: a seed record exists to be leaked",
    );

    // A directory move: the client renames both files in one notification
    // and neither old path survives.
    backend.documents.lock("test").await.remove(&a);
    backend.documents.lock("test").await.remove(&b);
    backend
        .did_rename_files(rename_params(&[(&a, &a2), (&b, &b2)]))
        .await;

    // Both halves of the early-return guard, asserted as the guard reads.
    assert!(
        !backend.workspace_index.read().await.has_source_edges(),
        "the `source` edge left with the renamed files",
    );
    assert!(
        backend.rehomed_source_seeds.lock().await.is_empty(),
        "a renamed-away seed record blocks the early return for the whole \
             session — the real cost of #1298, far beyond the bytes retained",
    );
    backend.refresh_source_rehoming().await.unwrap();
    assert!(backend.rehomed_source_seeds.lock().await.is_empty());
}

/// FP/TN guard: retiring a path that was never source-rehomed
/// must disturb nobody — an unrelated document's seed record is still
/// relevant and must survive the rename.
#[tokio::test]
async fn did_rename_of_an_unrelated_file_keeps_other_seeds_1298() {
    let backend = test_backend();
    let a = Uri::from_str("file:///rehome-fp/a.tcl").unwrap();
    let b = Uri::from_str("file:///rehome-fp/b.tcl").unwrap();
    let unrelated = Uri::from_str("file:///rehome-fp/tool.tcl").unwrap();
    let renamed = Uri::from_str("file:///rehome-fp/tool2.tcl").unwrap();
    register(&backend, &b, "proc helper {} {}\n").await;
    register(&backend, &a, "namespace eval ::ns { source b.tcl }\n").await;
    register(&backend, &unrelated, "proc standalone {} {}\n").await;
    backend.refresh_source_rehoming().await.unwrap();
    let before = backend.rehomed_source_seeds.lock().await.clone();
    assert_eq!(
        before.get(b.as_str()).cloned(),
        Some(vec!["::ns".to_owned()]),
        "precondition: only the sourced file has a record",
    );

    backend.documents.lock("test").await.remove(&unrelated);
    backend
        .did_rename_files(rename_params(&[(&unrelated, &renamed)]))
        .await;

    assert_eq!(
        *backend.rehomed_source_seeds.lock().await,
        before,
        "renaming a file that was never source-rehomed must leave every \
             other document's seed record exactly as it was",
    );
}

/// Second half: `retire_renamed_uri` must also clear
/// `autoloaded_library_uris`, whose entries are otherwise drained only by a
/// full package-database rebuild — a renamed library file would keep
/// claiming to be merged into the index long after its index entry is gone.
#[tokio::test]
async fn did_rename_forgets_the_autoloaded_library_record_1298() {
    let backend = test_backend();
    let lib_uri = seed_autoload_library(&backend, "autoload-rename").await;
    let app = Uri::from_str("file:///autoload-rename-app.tcl").unwrap();
    let app_src = "Rbc_ActiveLegend .g\n";
    register(&backend, &app, app_src).await;
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(app_src, "tcl8.6").clone()
    };
    let refs = backend
        .cross_document_references(&app, app_src, &analysis, Position::new(0, 3), true)
        .await
        .unwrap();
    assert!(
        refs.iter().any(|l| l.uri == lib_uri),
        "precondition: the autoload tier merged the library: {refs:?}",
    );
    assert!(
        backend
            .autoloaded_library_uris
            .lock()
            .await
            .contains(lib_uri.as_str()),
        "precondition: the merge is recorded against the library URI",
    );

    let moved = Uri::from_str("file:///autoload-rename-moved.tcl").unwrap();
    backend
        .did_rename_files(rename_params(&[(&lib_uri, &moved)]))
        .await;

    assert!(
        !backend
            .autoloaded_library_uris
            .lock()
            .await
            .contains(lib_uri.as_str()),
        "the renamed-away library must not stay recorded as merged",
    );
}

/// Both retirement paths clear the *same*
/// group of per-URI caches, because both go through
/// [`Backend::forget_uri_states`].  Asserting the whole group on the rename
/// path is what stops the two drifting apart again.
#[tokio::test]
async fn did_rename_forgets_every_per_uri_cache_1298() {
    let backend = test_backend();
    let old = Uri::from_str("file:///forget-all/old.tcl").unwrap();
    let new = Uri::from_str("file:///forget-all/new.tcl").unwrap();
    let bystander = Uri::from_str("file:///forget-all/other.tcl").unwrap();
    seed_per_uri_caches(&backend, &old).await;
    seed_per_uri_caches(&backend, &bystander).await;

    backend
        .did_rename_files(rename_params(&[(&old, &new)]))
        .await;

    assert_eq!(
        per_uri_cache_hits(&backend, &old).await,
        [false; 5],
        "every per-URI cache keyed on the dead path must be forgotten",
    );
    assert_eq!(
        per_uri_cache_hits(&backend, &bystander).await,
        [true; 5],
        "no other document's state may be touched",
    );
}
