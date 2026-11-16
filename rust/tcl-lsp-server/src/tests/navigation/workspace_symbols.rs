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

//! `workspace/symbol`: indexed hits, pending buffers and closed sources.

use super::*;

/// While a cold open is pending, Salsa still
/// contains the scanned disk source. The workspace-symbol fallback must
/// analyse the authoritative live bytes instead of accepting that stale
/// cache hit.
#[tokio::test]
async fn pending_open_workspace_symbols_use_live_text_1854() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///pending-symbols-1854.tcl").unwrap();
    let disk_text = "proc obsolete_disk_symbol {} {}\n";
    let live_text = "proc current_live_symbol {} {}\n";
    backend
        .db_set_source(&uri, disk_text, "tcl8.6".to_owned())
        .await
        .unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(live_text.to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("tcl".to_owned())
            .with_publication_pending(),
    );
    backend
        .workspace_index
        .write()
        .await
        .remove_document(uri.as_str());

    let current = backend
        .open_but_unindexed_symbols("current_live", 10)
        .await
        .unwrap()
        .fallback_hits;
    assert_eq!(current.len(), 1, "the live-only proc must be searchable");
    assert_eq!(current[0].name, "current_live_symbol");
    assert!(
        backend
            .open_but_unindexed_symbols("obsolete_disk", 10)
            .await
            .unwrap()
            .fallback_hits
            .is_empty(),
        "the pre-open disk proc must not survive through a Salsa cache hit",
    );
}

/// A brand-new or untitled pending buffer has
/// no scanned disk input and may not have created its first Salsa handle.
/// Its authoritative live declarations must still reach workspace/symbol.
#[tokio::test]
async fn pending_new_workspace_symbols_need_no_salsa_handle_1854() {
    let backend = test_backend();
    let uri = Uri::from_str("untitled:pending-symbols-1854").unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(
            "proc brand_new_live_symbol {} {}\n".to_owned(),
            "tcl8.6".to_owned(),
            1,
        )
        .with_language_id("tcl".to_owned())
        .with_publication_pending(),
    );
    assert!(
        !backend.db_files.lock().await.contains_key(&uri),
        "precondition: no Salsa handle exists",
    );

    let found = crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.symbol(WorkspaceSymbolParams {
            query: "brand_new".to_owned(),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }),
    )
    .await
    .expect("workspace/symbol must not wait for Indexed publication")
    .expect("workspace/symbol request must succeed");
    let Some(WorkspaceSymbolResponse::Flat(found)) = found else {
        panic!("expected the pending live symbol, got {found:?}");
    };
    assert_eq!(found.len(), 1, "the pending live proc must be searchable");
    assert_eq!(found[0].name, "brand_new_live_symbol");
    assert_eq!(found[0].location.uri, uri);
    assert_eq!(found[0].location.range.start.character, 5);
}

/// A pending no-handle buffer still belongs to
/// the project and must consume its published class-factory oracle. Looking
/// up the oracle through the missing consumer handle would silently omit
/// the class manufactured by this cross-file metaclass.
#[tokio::test]
async fn pending_new_workspace_symbols_use_project_factory_oracle_1854() {
    use salsa::Setter as _;

    let backend = test_backend();
    let factory_uri = Uri::from_str("file:///published-factory-1854.tcl").unwrap();
    let consumer_uri = Uri::from_str("untitled:pending-factory-consumer-1854").unwrap();
    let consumer = concat!(
        "::tk::Megawidget create IconList FocusableWidget {\n",
        "    method GetSpecs {} { return iconlist }\n",
        "}\n",
    );
    let oracle = {
        let mut analyser = Analyser::new();
        Arc::new(
            analyser
                .analyse(
                    concat!(
                        "oo::class create ::tk::MegawidgetClass {}\n",
                        "oo::class create ::tk::Megawidget {\n",
                        "    superclass oo::class\n",
                        "    self method create {name superclasses body} {\n",
                        "        next $name [list superclass ::tk::MegawidgetClass",
                        " {*}$superclasses]\\;$body\n",
                        "    }\n",
                        "}\n",
                    ),
                    "tcl9.0",
                )
                .class_factories(),
        )
    };
    backend
        .db_set_source(
            &factory_uri,
            "# published oracle carrier\n",
            "tcl9.0".to_owned(),
        )
        .await
        .unwrap();
    {
        let mut db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        let file = *files
            .get(&factory_uri)
            .expect("the existing project file is a Salsa input");
        file.set_workspace_class_factories(&mut *db)
            .to(Some(oracle));
    }
    backend.documents.lock("test").await.insert(
        consumer_uri.clone(),
        DocumentState::with_version(consumer.to_owned(), "tcl9.0".to_owned(), 1)
            .with_language_id("tcl".to_owned())
            .with_publication_pending(),
    );
    assert!(
        !backend.db_files.lock().await.contains_key(&consumer_uri),
        "precondition: the pending consumer has no Salsa handle",
    );

    let found = crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.symbol(WorkspaceSymbolParams {
            query: "IconList".to_owned(),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }),
    )
    .await
    .expect("factory symbol mapping must not wait for Indexed publication")
    .expect("workspace/symbol request must succeed");
    let Some(WorkspaceSymbolResponse::Flat(found)) = found else {
        panic!("expected the manufactured pending class, got {found:?}");
    };
    assert_eq!(found.len(), 1, "the manufactured class must be searchable");
    assert_eq!(found[0].name, "IconList");
    assert_eq!(found[0].kind, SymbolKind::CLASS);
    assert_eq!(found[0].location.uri, consumer_uri);
}

/// Fallback classification and ordinary index
/// hits must describe one snapshot. If the pending URI publishes while its
/// fresh analysis awaits configuration, a second index read would append
/// the new hit and map both revisions through the old captured source.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_symbols_do_not_mix_fallback_and_later_index_hits_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("untitled:workspace-symbol-snapshot-1854").unwrap();
    let old_text = "proc race_symbol {} {}\n";
    let new_text = "set pad 1\nproc race_symbol {} {}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(old_text.to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("tcl".to_owned())
            .with_publication_pending(),
    );

    // Stall fresh analysis just after the atomic documents/index snapshot.
    let config = backend.db_config.lock().await;
    let acquisitions = backend.documents.contention().acquisitions;
    let request = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        async move {
            backend
                .symbol(WorkspaceSymbolParams {
                    query: "race_symbol".to_owned(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if backend.documents.contention().acquisitions > acquisitions {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("workspace/symbol must capture its document/index snapshot");

    // Publish a newer revision and index entry while the old fresh analysis
    // is still blocked. The in-flight request must retain only its old
    // fallback hit, not append this later index hit for the same URI.
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(new_text.to_owned(), "tcl8.6".to_owned(), 2)
            .with_language_id("tcl".to_owned()),
    );
    let new_analysis = {
        let mut analyser = Analyser::new();
        analyser.analyse(new_text, "tcl8.6").clone()
    };
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &new_analysis);
    drop(config);

    let found = crate::rt::timeout(std::time::Duration::from_secs(5), request)
        .await
        .expect("workspace/symbol must finish")
        .expect("workspace/symbol task must not panic")
        .expect("workspace/symbol request must succeed");
    let Some(WorkspaceSymbolResponse::Flat(found)) = found else {
        panic!("expected the captured fallback symbol, got {found:?}");
    };
    assert_eq!(
        found.len(),
        1,
        "later index publication must not duplicate the hit"
    );
    assert_eq!(found[0].name, "race_symbol");
    assert_eq!(found[0].location.range.start.line, 0);
    assert_eq!(found[0].location.range.start.character, 5);
}

/// An indexed hit and its open source must be
/// captured together. An unrelated pending document can make fallback
/// analysis await while the indexed document publishes a newer revision;
/// the old byte span must still be mapped through the old source.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_symbols_map_indexed_hits_through_captured_sources_1854() {
    let backend = Arc::new(test_backend());
    let pending_uri = Uri::from_str("untitled:pending-snapshot-1854").unwrap();
    let indexed_uri = Uri::from_str("file:///indexed-snapshot-1854.tcl").unwrap();
    let old_indexed_text = "set pad 1\nproc indexed_snapshot_symbol {} {}\n";
    let new_indexed_text = "proc indexed_snapshot_symbol {} {}\n";
    backend.documents.lock("test").await.insert(
        pending_uri,
        DocumentState::with_version(
            "proc pending_snapshot_symbol {} {}\n".to_owned(),
            "tcl8.6".to_owned(),
            1,
        )
        .with_language_id("tcl".to_owned())
        .with_publication_pending(),
    );
    backend.documents.lock("test").await.insert(
        indexed_uri.clone(),
        DocumentState::with_version(old_indexed_text.to_owned(), "tcl8.6".to_owned(), 1)
            .with_language_id("tcl".to_owned()),
    );
    let old_analysis = {
        let mut analyser = Analyser::new();
        analyser.analyse(old_indexed_text, "tcl8.6").clone()
    };
    backend
        .workspace_index
        .write()
        .await
        .add_document(indexed_uri.as_str(), &old_analysis);

    // Hold configuration so the pending document's fresh analysis pauses
    // after the documents/index/source snapshot has been taken.
    let config = backend.db_config.lock().await;
    let acquisitions = backend.documents.contention().acquisitions;
    let request = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        async move {
            backend
                .symbol(WorkspaceSymbolParams {
                    query: "snapshot_symbol".to_owned(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if backend.documents.contention().acquisitions > acquisitions {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("workspace/symbol must capture its document/index snapshot");

    // Publish a shorter revision for the already-indexed document while
    // the unrelated fallback analysis is blocked. The request still owns
    // the old index hit, whose name starts on line one in its old source.
    backend.documents.lock("test").await.insert(
        indexed_uri.clone(),
        DocumentState::with_version(new_indexed_text.to_owned(), "tcl8.6".to_owned(), 2)
            .with_language_id("tcl".to_owned()),
    );
    let new_analysis = {
        let mut analyser = Analyser::new();
        analyser.analyse(new_indexed_text, "tcl8.6").clone()
    };
    backend
        .workspace_index
        .write()
        .await
        .add_document(indexed_uri.as_str(), &new_analysis);
    drop(config);

    let found = crate::rt::timeout(std::time::Duration::from_secs(5), request)
        .await
        .expect("workspace/symbol must finish")
        .expect("workspace/symbol task must not panic")
        .expect("workspace/symbol request must succeed");
    let Some(WorkspaceSymbolResponse::Flat(found)) = found else {
        panic!("expected captured workspace symbols, got {found:?}");
    };
    let indexed = found
        .iter()
        .find(|symbol| symbol.location.uri == indexed_uri)
        .expect("the indexed document must contribute its captured hit");
    assert_eq!(indexed.name, "indexed_snapshot_symbol");
    assert_eq!(indexed.location.range.start.line, 1);
    assert_eq!(indexed.location.range.start.character, 5);
}

#[derive(Debug)]
struct ClosedSymbolRaceStore {
    contents: std::sync::Mutex<Vec<u8>>,
    reads: std::sync::atomic::AtomicUsize,
    first_read_entered: std::sync::atomic::AtomicBool,
    first_read_release: (std::sync::Mutex<bool>, std::sync::Condvar),
}

impl ClosedSymbolRaceStore {
    pub(in super::super) fn new(contents: &str) -> Self {
        Self {
            contents: std::sync::Mutex::new(contents.as_bytes().to_vec()),
            reads: std::sync::atomic::AtomicUsize::new(0),
            first_read_entered: std::sync::atomic::AtomicBool::new(false),
            first_read_release: (std::sync::Mutex::new(false), std::sync::Condvar::new()),
        }
    }

    pub(in super::super) fn replace(&self, contents: &str) {
        *self
            .contents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = contents.as_bytes().to_vec();
    }

    fn release_first_read(&self) {
        let (released, wake) = &self.first_read_release;
        *released
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        wake.notify_all();
    }
}

impl vfs::SourceStore for ClosedSymbolRaceStore {
    fn document_uri_path(
        &self,
        original_uri: &str,
    ) -> Result<vfs::SourceUriPath, vfs::SourceUriPathUnavailable> {
        vfs::SourceStore::document_uri_path(&vfs::NativeStore, original_uri)
    }
    fn source_path_origin(
        &self,
        path: &Path,
    ) -> Result<vfs::SourcePathOrigin, vfs::SourceUriPathUnavailable> {
        vfs::SourceStore::source_path_origin(&vfs::NativeStore, path)
    }
    fn validate_source_path_origin(
        &self,
        origin: &vfs::SourcePathOrigin,
    ) -> Result<(), vfs::SourceUriPathUnavailable> {
        vfs::SourceStore::validate_source_path_origin(&vfs::NativeStore, origin)
    }
    fn validate_document_uri_path(
        &self,
        selected: &vfs::SourceUriPath,
    ) -> Result<(), vfs::SourceUriPathUnavailable> {
        vfs::SourceStore::validate_document_uri_path(&vfs::NativeStore, selected)
    }

    fn with_current_document_paths(
        &self,
        selected: &[&vfs::SourceUriPath],
        publish: &mut dyn FnMut(),
    ) -> Result<(), vfs::SourceUriPathUnavailable> {
        vfs::SourceStore::with_current_document_paths(&vfs::NativeStore, selected, publish)
    }

    fn with_current_source_inputs(
        &self,
        mappings: &[&vfs::SourceUriPath],
        documents: &[(&vfs::SourceUriPath, &[u8])],
        paths: &[(&vfs::SourcePathOrigin, &[u8])],
        publish: &mut dyn FnMut(),
    ) -> Result<(), vfs::SourceUriPathUnavailable> {
        if !paths.is_empty() {
            return Err(vfs::SourceUriPathUnavailable::ProviderUnavailable);
        }
        for selected in mappings
            .iter()
            .copied()
            .chain(documents.iter().map(|(selected, _)| *selected))
        {
            self.validate_document_uri_path(selected)?;
        }
        let contents = self
            .contents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if documents
            .iter()
            .any(|(_, expected)| contents.as_slice() != *expected)
        {
            return Err(vfs::SourceUriPathUnavailable::SourceImageUnavailable);
        }
        publish();
        Ok(())
    }

    fn read(&self, _path: &Path) -> std::io::Result<Vec<u8>> {
        use std::sync::atomic::Ordering;

        if self.reads.fetch_add(1, Ordering::SeqCst) == 0 {
            self.first_read_entered.store(true, Ordering::SeqCst);
            let (released, wake) = &self.first_read_release;
            let mut released = released
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            while !*released {
                released = wake
                    .wait(released)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        }
        Ok(self
            .contents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone())
    }

    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        String::from_utf8(self.read(path)?)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    }

    fn read_dir(&self, _path: &Path) -> std::io::Result<Vec<vfs::DirEntry>> {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "test store has no directories",
        ))
    }

    fn metadata(&self, _path: &Path) -> std::io::Result<vfs::Metadata> {
        let len = self
            .contents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len();
        Ok(vfs::Metadata {
            is_dir: false,
            len: u64::try_from(len).unwrap_or(u64::MAX),
        })
    }

    fn write(&self, _path: &Path, contents: &[u8]) -> std::io::Result<()> {
        *self
            .contents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = contents.to_vec();
        Ok(())
    }

    fn create_dir_all(&self, _path: &Path) -> std::io::Result<()> {
        Ok(())
    }
}

/// Fallback matches take response priority, so
/// a full fallback result must not read closed indexed sources whose hits
/// cannot be returned.
#[tokio::test]
async fn workspace_symbols_skip_closed_reads_after_fallback_limit_1854() {
    use std::sync::atomic::Ordering;

    let closed_text = "proc capacity_symbol_closed {} {}\n";
    let store = Arc::new(ClosedSymbolRaceStore::new(closed_text));
    // Avoid wedging the regression if the unwanted read returns: the
    // assertion below, rather than a timeout, diagnoses that behaviour.
    store.release_first_read();
    let mut backend = test_backend();
    backend.state_mut_for_test().store = Arc::clone(&store) as Arc<dyn vfs::SourceStore>;
    let pending_uri = Uri::from_str("untitled:capacity-limit-1854").unwrap();
    let closed_uri = Uri::from_str("file:///capacity-limit-1854.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        pending_uri,
        DocumentState::with_version(
            "proc capacity_symbol_pending {} {}\n".to_owned(),
            "tcl8.6".to_owned(),
            1,
        )
        .with_language_id("tcl".to_owned())
        .with_publication_pending(),
    );
    let closed_analysis = {
        let mut analyser = Analyser::new();
        analyser.analyse(closed_text, "tcl8.6").clone()
    };
    backend
        .workspace_index
        .write()
        .await
        .add_document(closed_uri.as_str(), &closed_analysis);

    let candidates = backend
        .open_but_unindexed_symbols("capacity_symbol", 1)
        .await
        .unwrap();
    assert_eq!(candidates.fallback_hits.len(), 1);
    assert_eq!(
        candidates.indexed_hits,
        [] as [tcl_lsp_core::workspace_symbols::IndexedWorkspaceSymbol; 0]
    );
    assert_eq!(
        store.reads.load(Ordering::SeqCst),
        0,
        "an indexed hit beyond the remaining result capacity must not load its source",
    );
}

/// A closed source read must be revalidated
/// against the per-document index revision. Otherwise a watcher can
/// replace both the file and its index entry after the old hit was
/// captured, and the old byte span is mapped through the new bytes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_symbols_revalidate_closed_source_snapshot_1854() {
    use std::sync::atomic::Ordering;

    let old_text = "set pad 1\nproc closed_snapshot_symbol {} {}\n";
    let new_text = "proc closed_snapshot_symbol {} {}\n";
    let store = Arc::new(ClosedSymbolRaceStore::new(old_text));
    let mut backend = test_backend();
    backend.state_mut_for_test().store = Arc::clone(&store) as Arc<dyn vfs::SourceStore>;
    let backend = Arc::new(backend);
    let pending_uri = Uri::from_str("untitled:closed-source-race-1854").unwrap();
    let closed_uri = Uri::from_str("file:///closed-source-race-1854.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        pending_uri,
        DocumentState::with_version(
            "proc pending_snapshot_symbol {} {}\n".to_owned(),
            "tcl8.6".to_owned(),
            1,
        )
        .with_language_id("tcl".to_owned())
        .with_publication_pending(),
    );
    let old_analysis = {
        let mut analyser = Analyser::new();
        analyser.analyse(old_text, "tcl8.6").clone()
    };
    backend
        .workspace_index
        .write()
        .await
        .add_document(closed_uri.as_str(), &old_analysis);

    let request = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        async move {
            backend
                .symbol(WorkspaceSymbolParams {
                    query: "snapshot_symbol".to_owned(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        while !store.first_read_entered.load(Ordering::SeqCst) {
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the closed source read must begin");

    // The first read has not copied its bytes yet. Replace both source and
    // index through the slot-reuse ABA sequence: remove this
    // URI, give its slot to another URI, then re-add it in a fresh slot.
    // Let the read return the new bytes to the request that still owns the
    // old hit. Revision revalidation must retry that snapshot.
    store.replace(new_text);
    let new_analysis = {
        let mut analyser = Analyser::new();
        analyser.analyse(new_text, "tcl8.6").clone()
    };
    let slot_consumer = {
        let mut analyser = Analyser::new();
        analyser
            .analyse("proc unrelated_slot_consumer {} {}\n", "tcl8.6")
            .clone()
    };
    {
        let mut index = backend.workspace_index.write().await;
        index.remove_document(closed_uri.as_str());
        index.add_document("file:///slot-consumer-1854.tcl", &slot_consumer);
        index.add_document(closed_uri.as_str(), &new_analysis);
    }
    store.release_first_read();

    let found = crate::rt::timeout(std::time::Duration::from_secs(5), request)
        .await
        .expect("workspace/symbol must finish")
        .expect("workspace/symbol task must not panic")
        .expect("workspace/symbol request must succeed");
    let Some(WorkspaceSymbolResponse::Flat(found)) = found else {
        panic!("expected captured workspace symbols, got {found:?}");
    };
    let closed = found
        .iter()
        .find(|symbol| symbol.location.uri == closed_uri)
        .expect("the stable closed-file retry must contribute its hit");
    assert_eq!(closed.name, "closed_snapshot_symbol");
    assert_eq!(closed.location.range.start.line, 0);
    assert_eq!(closed.location.range.start.character, 5);
    assert!(
        store.reads.load(Ordering::SeqCst) >= 2,
        "a changed index revision must force a second source read",
    );
}

/// `workspace/symbol` is answered from the workspace index, so a file the
/// folder scan indexed but the editor never opened is searchable.  Walking
/// the open-document map instead would make every symbol in an unopened
/// file invisible to Ctrl+T.
#[tokio::test]
async fn workspace_symbol_finds_a_symbol_in_an_unopened_indexed_file() {
    let backend = test_backend();
    let root = unique_scratch_dir("ws-symbol-unopened");
    let on_disk = root.join("scanned.tcl");
    let src = "proc scanned_only_helper {} {}\n";
    std::fs::write(&on_disk, src).unwrap();
    let uri = Uri::from_file_path(&on_disk).unwrap();
    // Indexed like the folder scan does it — never inserted into
    // `documents`, so an open-document walk could not see it.
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(src, "tcl8.6").clone()
    };
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &analysis);
    assert!(
        backend.documents.lock("test").await.is_empty(),
        "the fixture must not be an open document",
    );

    let found = backend
        .symbol(WorkspaceSymbolParams {
            query: "scanned_only".to_owned(),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        })
        .await
        .expect("ok");
    let Some(WorkspaceSymbolResponse::Flat(syms)) = found else {
        panic!("expected a flat symbol list, got {found:?}");
    };
    assert_eq!(syms.len(), 1, "{syms:?}");
    assert_eq!(syms[0].name, "scanned_only_helper");
    assert_eq!(syms[0].kind, SymbolKind::FUNCTION);
    assert_eq!(syms[0].location.uri, uri);
    // The span was resolved against the file's own text, read from disk.
    assert_eq!(syms[0].location.range.start.line, 0);
    assert_eq!(syms[0].location.range.start.character, 5);
    std::fs::remove_dir_all(&root).ok();
}

/// The answer is capped, so an empty or one-character query
/// against a large workspace cannot turn a per-keystroke request into an
/// unbounded response.
#[tokio::test]
async fn workspace_symbol_caps_its_answer() {
    let backend = test_backend();
    let over_the_cap = core_workspace_symbols::MAX_WORKSPACE_SYMBOL_RESULTS + 25;
    // The cap is a workspace-wide response contract, not a large-file
    // analyser test. Reuse one small, real analysis across enough indexed
    // documents to exceed the cap; a single 1,025-proc fixture made this
    // otherwise constant-time assertion pay the analyser's large-document
    // scaling cost on every CI run.
    let symbols_per_document = 41;
    let document_count = over_the_cap.div_ceil(symbols_per_document);
    let src = (0..symbols_per_document).fold(String::new(), |mut src, n| {
        use std::fmt::Write;
        let _ = writeln!(src, "proc capped_{n} {{}} {{}}");
        src
    });
    let analysis = {
        let mut analyser = Analyser::new();
        analyser.analyse(&src, "tcl8.6").clone()
    };
    let uris: Vec<Uri> = (0..document_count)
        .map(|n| Uri::from_str(&format!("file:///w/many-{n}.tcl")).unwrap())
        .collect();
    {
        let mut documents = backend.documents.lock("test").await;
        for uri in &uris {
            documents.insert(
                uri.clone(),
                DocumentState::new(src.clone(), "tcl8.6".to_owned()),
            );
        }
    }
    {
        let mut index = backend.workspace_index.write().await;
        for uri in &uris {
            index.add_document(uri.as_str(), &analysis);
        }
    }

    let found = backend
        .symbol(WorkspaceSymbolParams {
            query: "capped_".to_owned(),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        })
        .await
        .expect("ok");
    let Some(WorkspaceSymbolResponse::Flat(syms)) = found else {
        panic!("expected a flat symbol list, got {found:?}");
    };
    assert_eq!(
        syms.len(),
        core_workspace_symbols::MAX_WORKSPACE_SYMBOL_RESULTS,
    );
}
