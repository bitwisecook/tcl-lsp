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

//! Reindexing closed files from disk.

use super::*;

/// End to end: `reindex_index_from_disk` (run on close) must store
/// the source-directed dialect on the salsa `SourceFile`, since the cached
/// base analysis reads its dialect from there — not the folder/default alone.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reindex_stores_source_directed_dialect() {
    let root = unique_scratch_dir("reindex-dialect");
    let on_disk = root.join("app.tcl");
    // Pinned to f5-irules by directive; the folder/default would give tcl8.6.
    std::fs::write(
        &on_disk,
        "# tcl-dialect: f5-irules\nwhen HTTP_REQUEST { }\n",
    )
    .unwrap();
    let backend = test_backend();
    let uri = Uri::from_file_path(&on_disk).unwrap();

    backend.reindex_index_from_disk(&uri).await.unwrap();

    let file = backend
        .db_files
        .lock()
        .await
        .get(&uri)
        .copied()
        .expect("reindex must index the on-disk file");
    let db = backend.db.lock().await;
    assert_eq!(
        file.dialect(&*db).as_str(),
        "f5-irules",
        "reindex must store the directive-derived dialect the base analysis reads",
    );
    drop(db);
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn reindex_from_disk_refreshes_index_rather_than_dropping() {
    let root = unique_scratch_dir("reindex");
    let on_disk = root.join("lib.tcl");
    std::fs::write(&on_disk, "proc helper {} {}\n").unwrap();

    let backend = test_backend();
    let uri = Uri::from_file_path(&on_disk).unwrap();
    // Seed the index with a now-closed buffer version (proc `fresh`),
    // standing in for what did_open would have indexed.
    {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &analysis);
    }

    // What did_close does: refresh from disk instead of removing.
    backend.reindex_index_from_disk(&uri).await.unwrap();

    let index = backend.workspace_index.read().await;
    assert!(
        !index.proc_definitions("helper", "other").is_empty(),
        "on-disk proc must remain indexed after the buffer closes",
    );
    assert!(
        index.proc_definitions("fresh", "other").is_empty(),
        "stale buffer-only proc must be gone after reindex from disk",
    );
    drop(index);

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn reindex_from_disk_does_not_clobber_open_document() {
    let root = unique_scratch_dir("reindex-open");
    let on_disk = root.join("buf.tcl");
    std::fs::write(&on_disk, "proc stale {} {}\n").unwrap();

    let backend = test_backend();
    let uri = Uri::from_file_path(&on_disk).unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new("proc fresh {} {}\n".to_owned(), "tcl8.6".to_owned()),
    );
    {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &analysis);
    }

    backend.reindex_index_from_disk(&uri).await.unwrap();

    let index = backend.workspace_index.read().await;
    assert!(
        !index.proc_definitions("fresh", "other").is_empty(),
        "open-buffer definitions must remain indexed",
    );
    assert!(
        index.proc_definitions("stale", "other").is_empty(),
        "disk reindex must not overwrite a live buffer",
    );
    drop(index);

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn reindex_from_disk_drops_entry_when_file_is_gone() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///does/not/exist/gone.tcl").unwrap();
    {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse("proc ghost {} {}\n", "tcl8.6").clone();
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &analysis);
    }

    backend.reindex_index_from_disk(&uri).await.unwrap();

    let index = backend.workspace_index.read().await;
    assert!(
        index.proc_definitions("ghost", "other").is_empty(),
        "an entry whose file no longer exists must be dropped",
    );
}
