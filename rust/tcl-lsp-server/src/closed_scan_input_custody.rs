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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[derive(Debug)]
struct ReadPauseStore {
    target: PathBuf,
    reads: AtomicUsize,
    pauses: AtomicUsize,
    native_saved_sources: bool,
    started: tokio::sync::Notify,
    resume: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
}
impl vfs::SourceStore for ReadPauseStore {
    fn document_uri_path(
        &self,
        uri: &str,
    ) -> Result<vfs::SourceUriPath, vfs::SourceUriPathUnavailable> {
        vfs::SourceStore::document_uri_path(&vfs::NativeStore, uri)
    }
    fn validate_document_uri_path(
        &self,
        selected: &vfs::SourceUriPath,
    ) -> Result<(), vfs::SourceUriPathUnavailable> {
        vfs::SourceStore::validate_document_uri_path(&vfs::NativeStore, selected)
    }
    fn with_current_document_paths(
        &self,
        paths: &[&vfs::SourceUriPath],
        publish: &mut dyn FnMut(),
    ) -> Result<(), vfs::SourceUriPathUnavailable> {
        vfs::SourceStore::with_current_document_paths(&vfs::NativeStore, paths, publish)
    }
    fn with_current_document_sources(
        &self,
        sources: &[(&vfs::SourceUriPath, &[u8])],
        publish: &mut dyn FnMut(),
    ) -> Result<(), vfs::SourceUriPathUnavailable> {
        if !self.native_saved_sources {
            return Err(vfs::SourceUriPathUnavailable::ProviderUnavailable);
        }
        vfs::SourceStore::with_current_document_sources(&vfs::NativeStore, sources, publish)
    }
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        let read = vfs::SourceStore::read(&vfs::NativeStore, path)?;
        if path == self.target {
            let nth = self.reads.fetch_add(1, Ordering::SeqCst);
            if nth < self.pauses.load(Ordering::SeqCst) {
                self.started.notify_one();
                self.resume
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(15))
                    .expect("release actual disk read");
            }
        }
        Ok(read)
    }
    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        vfs::SourceStore::read_to_string(&vfs::NativeStore, path)
    }
    fn read_dir(&self, path: &Path) -> std::io::Result<Vec<vfs::DirEntry>> {
        vfs::SourceStore::read_dir(&vfs::NativeStore, path)
    }
    fn metadata(&self, path: &Path) -> std::io::Result<vfs::Metadata> {
        vfs::SourceStore::metadata(&vfs::NativeStore, path)
    }
    fn write(&self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        vfs::SourceStore::write(&vfs::NativeStore, path, bytes)
    }
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        vfs::SourceStore::create_dir_all(&vfs::NativeStore, path)
    }
}
struct Fixture {
    root: PathBuf,
    uri: Uri,
    backend: Backend,
    store: Arc<ReadPauseStore>,
    resume: std::sync::mpsc::Sender<()>,
}
impl Fixture {
    async fn new(pauses: usize) -> Self {
        Self::build(pauses, false).await
    }
    async fn with_native_saved_sources(pauses: usize) -> Self {
        Self::build(pauses, true).await
    }
    async fn build(pauses: usize, native_saved_sources: bool) -> Self {
        let root = std::env::temp_dir().join(format!(
            "closed-scan-inputs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("source.tcl");
        std::fs::write(&path, "proc p {} {}\n").unwrap();
        std::fs::write(root.join("tcl8.6.tcl.stubs"), "stub old_command {}\n").unwrap();
        let uri = Uri::from_file_path(&path).unwrap();
        let (resume, receiver) = std::sync::mpsc::channel();
        let store = Arc::new(ReadPauseStore {
            target: path,
            reads: AtomicUsize::new(0),
            pauses: AtomicUsize::new(0),
            native_saved_sources,
            started: tokio::sync::Notify::new(),
            resume: std::sync::Mutex::new(receiver),
        });
        let mut backend = crate::tests::test_backend();
        backend.state_mut_for_test().store = store.clone();
        backend
            .document_dialect_overrides
            .lock()
            .await
            .insert(uri.as_str().to_owned(), "tcl8.6".to_owned());
        backend.reindex_index_from_disk(&uri).await.unwrap();
        store.reads.store(0, Ordering::SeqCst);
        store.pauses.store(pauses, Ordering::SeqCst);
        Self {
            root,
            uri,
            backend,
            store,
            resume,
        }
    }
    async fn change_sidecar(&self, command: &str) {
        std::fs::write(
            self.root.join("tcl8.6.tcl.stubs"),
            format!("stub {command} {{}}\n"),
        )
        .unwrap();
        self.backend.invalidate_sidecar_stubs().await;
    }
    async fn assert_current_context(&self, dialect: &str) {
        let analysis = self
            .backend
            .fresh_analysis_for(&self.uri, Arc::from("proc p {} {}\n"), dialect.to_owned())
            .await
            .unwrap();
        let expected =
            core_workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(&analysis)
                .unwrap();
        assert_eq!(
            self.backend
                .workspace_index
                .read()
                .await
                .diagnostic_source_context(self.uri.as_str()),
            Some(&expected)
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
async fn bounded<T>(future: impl Future<Output = T>) -> T {
    crate::rt::timeout(Duration::from_secs(10), future)
        .await
        .expect("closed scan control must not wedge")
}
async fn queued_index_writer(backend: &Backend) {
    bounded(async {
        while !backend
            .workspace_index
            .tracking()
            .waiters
            .values()
            .any(|(access, _)| *access == RwLockAccess::Write)
        {
            crate::rt::yield_now().await;
        }
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn autoload_definition_recaptures_the_whole_query_after_input_publication() {
    let fixture = Fixture::with_native_saved_sources(1).await;
    let caller = Uri::from_file_path(fixture.root.join("caller.tcl")).unwrap();
    let source = "p\n";
    fixture
        .backend
        .documents
        .lock("autoload-recapture-fixture")
        .await
        .insert(
            caller.clone(),
            DocumentState::new(source.to_owned(), "tcl8.6".to_owned()),
        );
    fixture
        .backend
        .db_set_source(&caller, source, "tcl8.6".to_owned())
        .await
        .unwrap();
    let analysis = fixture
        .backend
        .analysis_for(&caller, Arc::from(source), "tcl8.6".to_owned())
        .await
        .unwrap();
    fixture
        .backend
        .workspace_index
        .write()
        .await
        .replace_document(caller.as_str(), &analysis);
    fixture.backend.package_resolver.write().await.add_original_tcl_index(
            "# Tcl autoload index file, version 2.0\nset auto_index(p) [list source [file join $dir source.tcl]]\n",
            &fixture.root, &|_| true);
    let querying = {
        let backend = fixture.backend.clone();
        let caller = caller.clone();
        tokio::spawn(async move {
            backend
                .compute_definition(&caller, Position::new(0, 0))
                .await
                .unwrap()
        })
    };
    tokio::time::timeout(Duration::from_secs(5), fixture.store.started.notified())
        .await
        .unwrap();
    fixture.backend.invalidate_diag_inputs();
    fixture.resume.send(()).unwrap();
    let locations = tokio::time::timeout(Duration::from_secs(10), querying)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        locations.len(),
        1,
        "the first query must reissue its original owner after the input publication"
    );
    assert_eq!(locations[0].uri, fixture.uri);
    assert_eq!(locations[0].range.start.line, 0);
    // The provider's actual source still owns the target after recapture.
    std::fs::write(
        fixture.root.join("source.tcl"),
        "proc replaced_header {} {}\n",
    )
    .unwrap();
    let definitions = fixture
        .backend
        .compute_definition(&caller, Position::new(0, 0))
        .await
        .unwrap();
    assert!(matches!(definitions.as_slice(), []), "{definitions:?}");
}
#[tokio::test]
async fn closed_document_readers_share_actual_saved_read_instrumentation() {
    let fixture = Fixture::new(0).await;
    let document = fixture
        .backend
        .read_document(&fixture.uri)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fixture.store.reads.load(Ordering::SeqCst), 1);
    assert_eq!(
        document.saved_source_bytes.as_deref(),
        Some(b"proc p {} {}\n".as_slice())
    );
    fixture
        .backend
        .closed_workspace_symbol_source(&fixture.uri)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fixture.store.reads.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn single_disk_publication_recomputes_after_actual_input_change_while_queued() {
    let fixture = Fixture::new(0).await;
    let index = fixture.backend.workspace_index.read().await;
    let publishing = crate::rt::spawn({
        let backend = fixture.backend.clone();
        let uri = fixture.uri.clone();
        async move { backend.reindex_index_from_disk(&uri).await }
    });
    queued_index_writer(&fixture.backend).await;
    fixture.change_sidecar("new_command").await;
    drop(index);
    bounded(publishing).await.unwrap().unwrap();
    assert_eq!(
        fixture.store.reads.load(Ordering::SeqCst),
        2,
        "final epoch refusal must trigger a genuine second source read"
    );
    fixture.assert_current_context("tcl8.6").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn a_filtered_batch_row_is_retried_after_actual_midread_sidecar_invalidation() {
    let fixture = Fixture::new(1).await;
    let publishing = crate::rt::spawn({
        let backend = fixture.backend.clone();
        let uri = fixture.uri.clone();
        async move { backend.batch_reindex_from_disk(&[uri]).await }
    });
    bounded(fixture.store.started.notified()).await;
    fixture.change_sidecar("new_command").await;
    fixture.resume.send(()).unwrap();
    bounded(publishing).await.unwrap().unwrap();
    assert_eq!(
        fixture.store.reads.load(Ordering::SeqCst),
        2,
        "a filtered stale row is unavailable, not an empty successful batch"
    );
    fixture.assert_current_context("tcl8.6").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn single_disk_retry_reresolves_dialect_snapshots_instead_of_restamping_old_inputs() {
    let fixture = Fixture::new(1).await;
    let publishing = crate::rt::spawn({
        let backend = fixture.backend.clone();
        let uri = fixture.uri.clone();
        async move { backend.reindex_index_from_disk(&uri).await }
    });
    bounded(fixture.store.started.notified()).await;
    {
        let _inputs = fixture.backend.analyser_inputs_gate.write().await;
        fixture
            .backend
            .document_dialect_overrides
            .lock()
            .await
            .insert(fixture.uri.as_str().to_owned(), "tcl9.0".to_owned());
        fixture.backend.invalidate_diag_inputs();
    }
    fixture.resume.send(()).unwrap();
    bounded(publishing).await.unwrap().unwrap();
    assert_eq!(fixture.store.reads.load(Ordering::SeqCst), 2);
    {
        let db = fixture.backend.db.lock().await;
        let files = fixture.backend.db_files.lock().await;
        assert_eq!(
            files[&fixture.uri].dialect(&*db),
            "tcl9.0",
            "retry must use the actual new dialect decision"
        );
    }
    fixture.assert_current_context("tcl9.0").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn repeated_input_changes_return_typed_unavailable_after_three_actual_reads() {
    let fixture = Fixture::new(3).await;
    let previous = fixture
        .backend
        .workspace_index
        .read()
        .await
        .document_revision(fixture.uri.as_str());
    let mut publishing = crate::rt::spawn({
        let backend = fixture.backend.clone();
        let uri = fixture.uri.clone();
        async move { backend.reindex_index_from_disk(&uri).await }
    });
    for command in ["first_command", "second_command", "third_command"] {
        bounded(async {
                tokio::select! {
                    () = fixture.store.started.notified() => {},
                    early = &mut publishing => panic!("disk operation returned before the required three-read retry schedule: {early:?}"),
                }
            }).await;
        fixture.change_sidecar(command).await;
        fixture.resume.send(()).unwrap();
    }
    let result = bounded(publishing).await.unwrap();
    assert!(
        matches!(result, Err(DocumentSourceUnavailable::ScanWorker { .. })),
        "exhausted retries must remain typed unavailable: {result:?}"
    );
    assert_eq!(fixture.store.reads.load(Ordering::SeqCst), 3);
    assert_eq!(
        fixture
            .backend
            .workspace_index
            .read()
            .await
            .document_revision(fixture.uri.as_str()),
        previous,
        "no rejected worker may publish index records"
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn queued_input_writer_is_awaited_before_retry_instead_of_exhausting_attempts() {
    let fixture = Fixture::new(0).await;
    let index = fixture.backend.workspace_index.read().await;
    let mut publishing = crate::rt::spawn({
        let backend = fixture.backend.clone();
        let uri = fixture.uri.clone();
        async move { backend.reindex_index_from_disk(&uri).await }
    });
    queued_index_writer(&fixture.backend).await;
    let reader = fixture.backend.analyser_inputs_gate.read().await;
    let mut writer = Box::pin(Arc::clone(&fixture.backend.analyser_inputs_gate).write_owned());
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(
        writer.as_mut().poll(&mut context).is_pending(),
        "the actual input writer must be queued"
    );
    drop(index);
    assert!(
        crate::rt::timeout(Duration::from_millis(20), &mut publishing)
            .await
            .is_err(),
        "retry must await the queued legitimate writer, not finish after three immediate refusals"
    );
    assert_eq!(fixture.store.reads.load(Ordering::SeqCst), 1);
    drop(reader);
    let writer = bounded(writer).await;
    fixture
        .backend
        .document_dialect_overrides
        .lock()
        .await
        .insert(fixture.uri.as_str().to_owned(), "tcl9.0".to_owned());
    fixture.backend.invalidate_diag_inputs();
    drop(writer);
    bounded(publishing).await.unwrap().unwrap();
    assert_eq!(fixture.store.reads.load(Ordering::SeqCst), 2);
    fixture.assert_current_context("tcl9.0").await;
}
