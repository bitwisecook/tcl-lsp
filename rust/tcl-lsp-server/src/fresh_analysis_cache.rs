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

//! Reuse of per-document fresh analyses keyed on their complete input
//! revision (user decision 2026-10-18T16:00Z, `Server108`; closed documents
//! included in a size-bounded LRU, decision 2026-10-19T11:00Z
//! `closed_document_analysis_cache`).
//!
//! A fresh analysis is a pure function of the document's bytes and dialect,
//! its checked path admission (which fixes the analyser's file path), and the
//! configured analyser inputs the driver resolved under one stable
//! analyser-inputs epoch and class-factory generation. A sidecar stub file is
//! the one disk input the analyser reads on its own; a sidecar change clears
//! the cache ([`FreshAnalysisCache::clear`]). Every hit is therefore the
//! analysis a recomputation would produce, so read-only requests re-analyse
//! only the documents whose input revision changed. A hit still passes the
//! same path-admission checks a fresh run does before it is handed out.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use tcl_compiler::analyser::NonAsciiMode;
use tower_lsp_server::ls_types::Uri;

use super::{DocumentPathAdmission, FreshAnalysisSeed, ResourceAnalyserInputs};

/// Entry cap. Open documents are a handful; the rest are closed documents a
/// workspace query touched.
const MAX_ENTRIES: usize = 512;

/// Cap on the retained source bytes, so a few very large files cannot pin an
/// unbounded amount of memory through their analyses.
const MAX_SOURCE_BYTES: usize = 64 * 1024 * 1024;

/// Structure-only analyses serve the coarse semantic-token tier of open
/// documents, so a handful of slots is enough.
const MAX_STRUCTURE_ENTRIES: usize = 32;

/// The configured analyser inputs a [`super::FreshAnalysisDriver`] was built
/// from, other than the epoch/generation stamps it also carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FreshAnalyserInputs {
    pub(super) disabled: HashSet<String>,
    pub(super) non_ascii_mode: NonAsciiMode,
    pub(super) extra_commands: HashSet<String>,
    pub(super) pack_key: u64,
    pub(super) resource: ResourceAnalyserInputs,
    pub(super) workspace_declared_commands: Option<Arc<tcl_registry::model::DeclaredSurface>>,
}

/// Everything a fresh analysis of one document depends on.
#[derive(Debug, Clone)]
pub(super) struct FreshAnalysisKey {
    pub(super) text: Arc<str>,
    pub(super) dialect: String,
    pub(super) path_admission: Arc<DocumentPathAdmission>,
    pub(super) analyser_inputs_epoch: u64,
    pub(super) class_factory_generation: u64,
    pub(super) inputs: FreshAnalyserInputs,
    pub(super) resolved_input: Option<tcl_compiler::analyser::ResolvedAnalysisInput>,
}

impl PartialEq for FreshAnalysisKey {
    fn eq(&self, other: &Self) -> bool {
        self.analyser_inputs_epoch == other.analyser_inputs_epoch
            && self.class_factory_generation == other.class_factory_generation
            && self.dialect == other.dialect
            && (Arc::ptr_eq(&self.text, &other.text) || self.text == other.text)
            && self.path_admission == other.path_admission
            && self.inputs == other.inputs
            && self.resolved_input == other.resolved_input
    }
}

struct Entry {
    key: FreshAnalysisKey,
    seed: FreshAnalysisSeed,
}

pub(super) struct SidecarRevision {
    published: std::sync::atomic::AtomicBool,
}

impl Default for SidecarRevision {
    fn default() -> Self {
        Self {
            published: std::sync::atomic::AtomicBool::new(true),
        }
    }
}

impl SidecarRevision {
    pub(super) fn is_published_current(&self) -> bool {
        self.published.load(std::sync::atomic::Ordering::Acquire)
    }
}

/// Bounded least-recently-used map from a document URI to its latest fresh
/// analysis and the input revision it was computed from.
#[derive(Default)]
pub(super) struct FreshAnalysisCache {
    entries: HashMap<Uri, Entry>,
    /// Least recently used first.
    order: VecDeque<Uri>,
    source_bytes: usize,
    revision: Arc<SidecarRevision>,
    /// Structure-only analyses for the coarse token tier, which otherwise
    /// re-analysed the same text for every capture attempt of every request.
    structure: HashMap<Uri, Entry>,
}

impl FreshAnalysisCache {
    /// The cached analysis of `uri` when it was computed from exactly `key`.
    pub(super) fn get(&mut self, uri: &Uri, key: &FreshAnalysisKey) -> Option<FreshAnalysisSeed> {
        let entry = self.entries.get(uri)?;
        if entry.key != *key {
            return None;
        }
        let seed = entry.seed.clone();
        self.touch(uri);
        Some(seed)
    }

    /// Record `seed` as `uri`'s analysis for `key`, replacing any older
    /// revision of the same document.
    pub(super) fn insert(&mut self, uri: &Uri, key: FreshAnalysisKey, seed: FreshAnalysisSeed) {
        let bytes = key.text.len();
        if bytes > MAX_SOURCE_BYTES {
            self.remove(uri);
            return;
        }
        self.remove(uri);
        self.source_bytes += bytes;
        self.entries.insert(uri.clone(), Entry { key, seed });
        self.order.push_back(uri.clone());
        while self.entries.len() > MAX_ENTRIES || self.source_bytes > MAX_SOURCE_BYTES {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&oldest) {
                self.source_bytes -= entry.key.text.len();
            }
        }
    }

    /// The cached structure-only analysis of `uri` for exactly `key`.
    pub(super) fn get_structure(
        &self,
        uri: &Uri,
        key: &FreshAnalysisKey,
    ) -> Option<FreshAnalysisSeed> {
        let entry = self.structure.get(uri)?;
        (entry.key == *key).then(|| entry.seed.clone())
    }

    /// Record `seed` as `uri`'s structure-only analysis for `key`.
    pub(super) fn insert_structure(
        &mut self,
        uri: &Uri,
        key: FreshAnalysisKey,
        seed: FreshAnalysisSeed,
    ) {
        if key.text.len() > MAX_SOURCE_BYTES {
            self.structure.remove(uri);
            return;
        }
        if self.structure.len() >= MAX_STRUCTURE_ENTRIES && !self.structure.contains_key(uri) {
            self.structure.clear();
        }
        self.structure.insert(uri.clone(), Entry { key, seed });
    }

    /// Drop `uri`'s entry, if any.
    pub(super) fn remove(&mut self, uri: &Uri) {
        if let Some(entry) = self.entries.remove(uri) {
            self.source_bytes -= entry.key.text.len();
            self.order.retain(|queued| queued != uri);
        }
    }

    /// Retain custody of the sidecar state before starting a worker.
    pub(super) fn revision(&self) -> Arc<SidecarRevision> {
        Arc::clone(&self.revision)
    }

    pub(super) fn is_current(&self, revision: &Arc<SidecarRevision>) -> bool {
        Arc::ptr_eq(&self.revision, revision)
    }

    pub(super) fn published_revision(&self) -> Option<Arc<SidecarRevision>> {
        self.revision
            .is_published_current()
            .then(|| self.revision())
    }

    pub(super) fn published_revision_is_current(&self, revision: &Arc<SidecarRevision>) -> bool {
        self.revision.is_published_current() && self.is_current(revision)
    }

    pub(super) fn complete_sidecar_invalidation(&mut self, revision: &Arc<SidecarRevision>) {
        if self.is_current(revision) {
            self.revision
                .published
                .store(true, std::sync::atomic::Ordering::Release);
        }
    }

    /// Drop every entry and retire workers started before the invalidation.
    /// Published database reads resume only after their sidecar epochs advance.
    pub(super) fn clear(&mut self) {
        self.revision
            .published
            .store(false, std::sync::atomic::Ordering::Release);
        self.revision = Arc::new(SidecarRevision {
            published: std::sync::atomic::AtomicBool::new(false),
        });
        self.entries.clear();
        self.order.clear();
        self.source_bytes = 0;
        self.structure.clear();
    }

    fn touch(&mut self, uri: &Uri) {
        if let Some(position) = self.order.iter().position(|queued| queued == uri)
            && let Some(queued) = self.order.remove(position)
        {
            self.order.push_back(queued);
        }
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;
    use std::sync::Arc;

    use tower_lsp_server::ls_types::Uri;

    /// Server108: an unchanged input revision is not analysed again; a
    /// changed text or analyser-inputs epoch is.
    #[tokio::test]
    async fn fresh_analysis_is_reused_only_for_an_unchanged_input_revision() {
        let backend = crate::tests::test_backend();
        let uri = Uri::from_str("file:///workspace/fresh-cache.tcl").unwrap();
        let source: Arc<str> = Arc::from("proc a {} {}\na\n");
        let first = backend
            .fresh_analysis_seed(&uri, Arc::clone(&source), "tcl8.6".to_owned())
            .await
            .unwrap();
        let again = backend
            .fresh_analysis_seed(&uri, Arc::from("proc a {} {}\na\n"), "tcl8.6".to_owned())
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&first.analysis, &again.analysis));
        let edited = backend
            .fresh_analysis_seed(&uri, Arc::from("proc b {} {}\nb\n"), "tcl8.6".to_owned())
            .await
            .unwrap();
        assert!(!Arc::ptr_eq(&first.analysis, &edited.analysis));
        let other_dialect = backend
            .fresh_analysis_seed(&uri, Arc::from("proc b {} {}\nb\n"), "tcl9.0".to_owned())
            .await
            .unwrap();
        assert!(!Arc::ptr_eq(&edited.analysis, &other_dialect.analysis));
        backend.invalidate_diag_inputs();
        let reconfigured = backend
            .fresh_analysis_seed(&uri, Arc::from("proc b {} {}\nb\n"), "tcl9.0".to_owned())
            .await
            .unwrap();
        assert!(!Arc::ptr_eq(
            &other_dialect.analysis,
            &reconfigured.analysis
        ));
        assert_eq!(
            reconfigured.analyser_inputs_epoch,
            backend.diag_inputs_epoch()
        );
        assert_eq!(
            backend
                .fresh_analysis_cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .len(),
            1
        );
    }
}

#[cfg(test)]
mod input_custody_tests {
    use std::future::Future as _;
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use crate::{FreshAnalysisSeed, vfs};
    use tower_lsp_server::ls_types::Uri;

    #[tokio::test]
    async fn published_analysis_obeys_the_requested_dialect_and_reuses_matching_input() {
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///workspace/cache-requested-input.tcl"
            .parse()
            .unwrap();
        let source: Arc<str> = Arc::from("proc p {} {}\np\n");
        backend
            .db_set_source(&uri, &source, "tcl8.6".to_owned())
            .await
            .unwrap();
        let published = backend.cached_analysis(&uri).await.unwrap();
        let requested = backend
            .analysis_for(&uri, Arc::clone(&source), "tcl9.0".to_owned())
            .await
            .unwrap();
        assert_eq!(requested.resolved_profile().unwrap().name, "tcl9.0");
        assert_ne!(published.resolved_input, requested.resolved_input);
        let again = backend
            .analysis_for(&uri, Arc::clone(&source), "tcl9.0".to_owned())
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&requested, &again));
        let original = backend
            .analysis_for(&uri, source, "tcl8.6".to_owned())
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&published, &original));
    }

    #[tokio::test]
    async fn unavailable_requested_overlay_does_not_borrow_a_published_analysis() {
        use salsa::Setter as _;
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///workspace/cache-unavailable-overlay.tcl"
            .parse()
            .unwrap();
        let source: Arc<str> = Arc::from("set value 1\n");
        backend
            .db_set_source(&uri, &source, "tcl8.6".to_owned())
            .await
            .unwrap();
        let published = backend.cached_analysis(&uri).await.unwrap();
        assert!(published.resolved_input.is_some());
        {
            let mut db = backend.db.lock().await;
            let config = *backend.db_config.lock().await;
            config.set_spec_pack_key(&mut *db).to(u64::MAX);
        }
        backend.invalidate_diag_inputs();
        let unavailable = backend
            .analysis_for(&uri, source, "tcl8.6".to_owned())
            .await
            .unwrap();
        assert!(unavailable.analysis_context_unavailable.is_some());
        assert!(unavailable.resolved_input.is_none());
        assert!(!Arc::ptr_eq(&published, &unavailable));
    }

    #[tokio::test]
    async fn epoch_is_checked_after_waiting_for_factory_generation() {
        let backend = crate::tests::test_backend();
        let uri: Uri = "untitled:cache-epoch".parse().unwrap();
        let seed: FreshAnalysisSeed = backend
            .fresh_analysis_seed(&uri, Arc::from("set x 1\n"), "tcl8.6".to_owned())
            .await
            .unwrap();
        let generation = backend.class_factory_generation.write().await;
        let mut current = Box::pin(backend.fresh_index_generation_is_current(&seed));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(current.as_mut().poll(&mut context).is_pending());
        backend.invalidate_diag_inputs();
        drop(generation);
        assert!(!current.await);
    }

    #[derive(Debug)]
    struct WorkerCompletionStore {
        caller: std::thread::ThreadId,
        checks: AtomicUsize,
        completed: tokio::sync::Notify,
        resume: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
    }

    impl vfs::SourceStore for WorkerCompletionStore {
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
            if std::thread::current().id() != self.caller
                && self.checks.fetch_add(1, Ordering::SeqCst) == 1
            {
                self.completed.notify_one();
                self.resume
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(60))
                    .expect("release the completed analysis worker");
            }
            vfs::SourceStore::validate_document_uri_path(&vfs::NativeStore, selected)
        }
        fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
            vfs::SourceStore::read(&vfs::NativeStore, path)
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
        fn write(&self, path: &Path, contents: &[u8]) -> std::io::Result<()> {
            vfs::SourceStore::write(&vfs::NativeStore, path, contents)
        }
        fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
            vfs::SourceStore::create_dir_all(&vfs::NativeStore, path)
        }
    }

    #[tokio::test]
    async fn sidecar_invalidation_retires_an_already_computed_worker() {
        struct Directory(std::path::PathBuf);
        impl Drop for Directory {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root = Directory(std::env::temp_dir().join(format!(
                "tcl-lsp-cache-sidecar-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )));
        std::fs::create_dir(&root.0).unwrap();
        let file = root.0.join("source.tcl");
        let sidecar = root.0.join("tcl8.6.tcl.stubs");
        std::fs::write(&file, "old_command\n").unwrap();
        std::fs::write(&sidecar, "stub old_command {}\n").unwrap();
        let uri = Uri::from_file_path(&file).unwrap();
        let (resume, receiver) = std::sync::mpsc::channel();
        let store = Arc::new(WorkerCompletionStore {
            caller: std::thread::current().id(),
            checks: AtomicUsize::new(0),
            completed: tokio::sync::Notify::new(),
            resume: std::sync::Mutex::new(receiver),
        });
        let mut backend = crate::tests::test_backend();
        backend.state_mut_for_test().store = store.clone();
        let backend = Arc::new(backend);
        let analysis = crate::rt::spawn({
            let backend = Arc::clone(&backend);
            let uri = uri.clone();
            async move {
                backend
                    .fresh_analysis_seed(&uri, Arc::from("old_command\n"), "tcl8.6".to_owned())
                    .await
                    .unwrap()
            }
        });
        crate::rt::timeout(Duration::from_secs(30), store.completed.notified())
            .await
            .expect("the actual worker finishes analysis before invalidation");
        std::fs::write(&sidecar, "stub new_command {}\n").unwrap();
        backend.invalidate_sidecar_stubs().await;
        resume.send(()).unwrap();
        let seed = crate::rt::timeout(Duration::from_secs(30), analysis)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            seed.analysis
                .stub_commands
                .iter()
                .filter(|stub| stub.from_sidecar)
                .map(|stub| stub.name.as_str())
                .collect::<Vec<_>>(),
            vec!["new_command"]
        );
        let again = backend
            .fresh_analysis_seed(&uri, Arc::from("old_command\n"), "tcl8.6".to_owned())
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&seed.analysis, &again.analysis));
        assert!(
            store.checks.load(Ordering::SeqCst) >= 4,
            "the worker must genuinely recompute after invalidation"
        );
    }
}

#[cfg(test)]
mod sidecar_publication_tests {
    use std::future::Future as _;
    use std::sync::Arc;

    use crate::{AnalysisResult, Backend, DocumentPublication, DocumentState};
    use tower_lsp_server::ls_types::Uri;

    struct Documents {
        root: std::path::PathBuf,
        uris: [Uri; 2],
        source: Arc<str>,
    }
    impl Documents {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "tcl-lsp-sidecar-publication-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
            ));
            std::fs::create_dir(&root).unwrap();
            std::fs::write(root.join("tcl8.6.tcl.stubs"), "stub old_command {}\n").unwrap();
            let source: Arc<str> = Arc::from("proc p {} {}\n");
            let uris = ["first.tcl", "second.tcl"].map(|name| {
                let path = root.join(name);
                std::fs::write(&path, source.as_bytes()).unwrap();
                Uri::from_file_path(path).unwrap()
            });
            Self { root, uris, source }
        }
        fn replace_sidecar(&self) {
            std::fs::write(self.root.join("tcl8.6.tcl.stubs"), "stub new_command {}\n").unwrap();
        }
        async fn publish(&self, backend: &Backend) {
            for uri in &self.uris {
                backend
                    .db_set_source(uri, &self.source, "tcl8.6".to_owned())
                    .await
                    .unwrap();
                assert_eq!(
                    sidecar_names(&backend.cached_analysis(uri).await.unwrap()),
                    vec!["old_command"]
                );
            }
        }
    }
    impl Drop for Documents {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
    fn sidecar_names(analysis: &AnalysisResult) -> Vec<&str> {
        analysis
            .stub_commands
            .iter()
            .filter(|stub| stub.from_sidecar)
            .map(|stub| stub.name.as_str())
            .collect()
    }

    #[tokio::test]
    async fn cancelled_sidecar_invalidation_refuses_stale_database_analyses_for_all_documents() {
        let documents = Documents::new();
        let backend = crate::tests::test_backend();
        documents.publish(&backend).await;
        documents.replace_sidecar();
        let database = backend.db.lock().await;
        let mut invalidating = Box::pin(backend.invalidate_sidecar_stubs());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(invalidating.as_mut().poll(&mut context).is_pending());
        drop(invalidating);
        drop(database);
        for uri in &documents.uris {
            let old = {
                let file = *backend.db_files.lock().await.get(uri).unwrap();
                let config = backend.resolved_db_config(uri).await;
                let snapshot = backend.db.snapshot("sidecar_cancelled_raw_memo").await;
                tcl_lsp_db::file_analysis_incremental(&*snapshot, file, config)
            };
            assert_eq!(
                sidecar_names(&old),
                vec!["old_command"],
                "cancellation leaves the actual DB input unrefreshed"
            );
            let current = backend
                .analysis_for(uri, Arc::clone(&documents.source), "tcl8.6".to_owned())
                .await
                .unwrap();
            assert_eq!(sidecar_names(&current), vec!["new_command"]);
            assert!(!Arc::ptr_eq(&old, &current));
        }
        backend.invalidate_sidecar_stubs().await;
        for uri in &documents.uris {
            let published = backend.cached_analysis(uri).await.unwrap();
            assert_eq!(sidecar_names(&published), vec!["new_command"]);
            let current = backend
                .analysis_for(uri, Arc::clone(&documents.source), "tcl8.6".to_owned())
                .await
                .unwrap();
            assert!(
                Arc::ptr_eq(&published, &current),
                "a completed invalidation restores published reuse"
            );
        }
    }

    #[tokio::test]
    async fn overlapping_sidecar_invalidation_waits_for_the_input_owner_and_recovers_after_cancellation()
     {
        let documents = Documents::new();
        let backend = crate::tests::test_backend();
        documents.publish(&backend).await;
        documents.replace_sidecar();
        let database = backend.db.lock().await;
        let mut first = Box::pin(backend.invalidate_sidecar_stubs());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(first.as_mut().poll(&mut context).is_pending());
        let revision = backend.fresh_analysis_cache.lock().unwrap().revision();
        let epoch = backend.diag_inputs_epoch();
        let mut second = Box::pin(backend.invalidate_sidecar_stubs());
        assert!(second.as_mut().poll(&mut context).is_pending());
        assert!(
            backend
                .fresh_analysis_cache
                .lock()
                .unwrap()
                .is_current(&revision),
            "the waiting invalidation cannot mutate another owner's revision"
        );
        assert_eq!(backend.diag_inputs_epoch(), epoch);
        drop(first);
        assert!(second.as_mut().poll(&mut context).is_pending());
        assert!(
            !backend
                .fresh_analysis_cache
                .lock()
                .unwrap()
                .is_current(&revision)
        );
        assert_ne!(backend.diag_inputs_epoch(), epoch);
        drop(database);
        second.await;
        for uri in &documents.uris {
            let published = backend.cached_analysis(uri).await.unwrap();
            assert_eq!(sidecar_names(&published), vec!["new_command"]);
            let current = backend
                .analysis_for(uri, Arc::clone(&documents.source), "tcl8.6".to_owned())
                .await
                .unwrap();
            assert!(Arc::ptr_eq(&published, &current));
        }
    }

    #[tokio::test]
    async fn previously_issued_seed_is_recomputed_before_open_index_publication() {
        let documents = Documents::new();
        let backend = crate::tests::test_backend();
        let uri = &documents.uris[0];
        let seed = backend
            .fresh_analysis_seed(uri, Arc::clone(&documents.source), "tcl8.6".to_owned())
            .await
            .unwrap();
        assert_eq!(sidecar_names(&seed.analysis), vec!["old_command"]);
        let old = Arc::clone(&seed.analysis);
        let mut document =
            DocumentState::with_version(documents.source.to_string(), "tcl8.6".to_owned(), 1);
        document.publication = DocumentPublication::Salsa;
        backend
            .documents
            .lock("sidecar seed test")
            .await
            .insert(uri.clone(), document);
        documents.replace_sidecar();
        backend.invalidate_sidecar_stubs().await;
        assert_eq!(backend.fresh_analysis_cache.lock().unwrap().len(), 0);
        assert!(
            backend
                .publish_open_index_if_current(uri, &documents.source, "tcl8.6", 1, seed)
                .await
                .unwrap()
        );
        assert_eq!(
            backend.fresh_analysis_cache.lock().unwrap().len(),
            1,
            "the publisher must perform an actual replacement analysis"
        );
        let current = backend
            .fresh_analysis_seed(uri, Arc::clone(&documents.source), "tcl8.6".to_owned())
            .await
            .unwrap();
        assert!(!Arc::ptr_eq(&old, &current.analysis));
        assert_eq!(sidecar_names(&current.analysis), vec!["new_command"]);
    }
    #[derive(Debug)]
    struct PublishedReadStore {
        armed: std::sync::atomic::AtomicBool,
        checks: std::sync::atomic::AtomicUsize,
        selected: tokio::sync::Notify,
        resume: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
    }
    impl crate::vfs::SourceStore for PublishedReadStore {
        fn document_uri_path(
            &self,
            uri: &str,
        ) -> Result<crate::vfs::SourceUriPath, crate::vfs::SourceUriPathUnavailable> {
            crate::vfs::SourceStore::document_uri_path(&crate::vfs::NativeStore, uri)
        }
        fn source_path_origin(
            &self,
            path: &std::path::Path,
        ) -> Result<crate::vfs::SourcePathOrigin, crate::vfs::SourceUriPathUnavailable> {
            crate::vfs::SourceStore::source_path_origin(&crate::vfs::NativeStore, path)
        }
        fn validate_source_path_origin(
            &self,
            origin: &crate::vfs::SourcePathOrigin,
        ) -> Result<(), crate::vfs::SourceUriPathUnavailable> {
            crate::vfs::SourceStore::validate_source_path_origin(&crate::vfs::NativeStore, origin)
        }
        fn validate_document_uri_path(
            &self,
            selected: &crate::vfs::SourceUriPath,
        ) -> Result<(), crate::vfs::SourceUriPathUnavailable> {
            use std::sync::atomic::Ordering;
            if self.armed.load(Ordering::SeqCst) && self.checks.fetch_add(1, Ordering::SeqCst) == 2
            {
                self.selected.notify_one();
                self.resume
                    .lock()
                    .unwrap()
                    .recv_timeout(std::time::Duration::from_secs(60))
                    .expect("release the reader retaining the actual cached Arc");
            }
            crate::vfs::SourceStore::validate_document_uri_path(&crate::vfs::NativeStore, selected)
        }
        fn with_current_document_paths(
            &self,
            selected: &[&crate::vfs::SourceUriPath],
            publish: &mut dyn FnMut(),
        ) -> Result<(), crate::vfs::SourceUriPathUnavailable> {
            crate::vfs::SourceStore::with_current_document_paths(
                &crate::vfs::NativeStore,
                selected,
                publish,
            )
        }
        fn with_current_source_inputs(
            &self,
            selected: &[&crate::vfs::SourceUriPath],
            documents: &[(&crate::vfs::SourceUriPath, &[u8])],
            paths: &[(&crate::vfs::SourcePathOrigin, &[u8])],
            publish: &mut dyn FnMut(),
        ) -> Result<(), crate::vfs::SourceUriPathUnavailable> {
            crate::vfs::SourceStore::with_current_source_inputs(
                &crate::vfs::NativeStore,
                selected,
                documents,
                paths,
                publish,
            )
        }
        fn read(&self, path: &std::path::Path) -> std::io::Result<Vec<u8>> {
            crate::vfs::SourceStore::read(&crate::vfs::NativeStore, path)
        }
        fn read_to_string(&self, path: &std::path::Path) -> std::io::Result<String> {
            crate::vfs::SourceStore::read_to_string(&crate::vfs::NativeStore, path)
        }
        fn read_dir(&self, path: &std::path::Path) -> std::io::Result<Vec<crate::vfs::DirEntry>> {
            crate::vfs::SourceStore::read_dir(&crate::vfs::NativeStore, path)
        }
        fn metadata(&self, path: &std::path::Path) -> std::io::Result<crate::vfs::Metadata> {
            crate::vfs::SourceStore::metadata(&crate::vfs::NativeStore, path)
        }
        fn write(&self, path: &std::path::Path, contents: &[u8]) -> std::io::Result<()> {
            crate::vfs::SourceStore::write(&crate::vfs::NativeStore, path, contents)
        }
        fn create_dir_all(&self, path: &std::path::Path) -> std::io::Result<()> {
            crate::vfs::SourceStore::create_dir_all(&crate::vfs::NativeStore, path)
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn sidecar_invalidation_withdraws_a_published_read_captured_before_clear() {
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        let documents = Documents::new();
        let (resume, receiver) = std::sync::mpsc::channel();
        let store = Arc::new(PublishedReadStore {
            armed: AtomicBool::new(false),
            checks: AtomicUsize::new(0),
            selected: tokio::sync::Notify::new(),
            resume: std::sync::Mutex::new(receiver),
        });
        let mut backend = crate::tests::test_backend();
        backend.state_mut_for_test().store = store.clone();
        let backend = Arc::new(backend);
        documents.publish(&backend).await;
        let uri = documents.uris[0].clone();
        let old = backend.cached_analysis(&uri).await.unwrap();
        store.armed.store(true, Ordering::SeqCst);
        let reading = crate::rt::spawn({
            let backend = Arc::clone(&backend);
            let uri = uri.clone();
            let source = Arc::clone(&documents.source);
            async move {
                backend
                    .analysis_for(&uri, source, "tcl8.6".to_owned())
                    .await
                    .unwrap()
            }
        });
        // Initial capture and retained-admission validation precede the DB
        // read; its third validation runs only after cached_analysis returns.
        crate::rt::timeout(
            std::time::Duration::from_secs(10),
            store.selected.notified(),
        )
        .await
        .expect("the reader must retain its actual cached result before invalidation");
        assert_eq!(store.checks.load(Ordering::SeqCst), 3);
        documents.replace_sidecar();
        backend.invalidate_sidecar_stubs().await;
        resume.send(()).unwrap();
        let current = crate::rt::timeout(std::time::Duration::from_secs(10), reading)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(sidecar_names(&old), vec!["old_command"]);
        assert_eq!(sidecar_names(&current), vec!["new_command"]);
        assert!(!Arc::ptr_eq(&old, &current));
    }

    #[tokio::test]
    async fn sidecar_invalidation_waits_for_an_active_index_publication() {
        let documents = Documents::new();
        let backend = Arc::new(crate::tests::test_backend());
        let uri = documents.uris[0].clone();
        let seed = backend
            .fresh_analysis_seed(&uri, Arc::clone(&documents.source), "tcl8.6".to_owned())
            .await
            .unwrap();
        assert_eq!(sidecar_names(&seed.analysis), vec!["old_command"]);
        let mut document =
            DocumentState::with_version(documents.source.to_string(), "tcl8.6".to_owned(), 1);
        document.publication = DocumentPublication::Salsa;
        backend
            .documents
            .lock("sidecar publication overlap test")
            .await
            .insert(uri.clone(), document);
        let index = backend.workspace_index.read().await;
        let publishing = crate::rt::spawn({
            let backend = Arc::clone(&backend);
            let source = Arc::clone(&documents.source);
            let seed = seed.clone();
            let uri = uri.clone();
            async move {
                backend
                    .publish_open_index_if_current(&uri, &source, "tcl8.6", 1, seed)
                    .await
                    .unwrap()
            }
        });
        crate::rt::timeout(std::time::Duration::from_secs(5), async {
            while backend.workspace_index.try_read().is_ok() {
                crate::rt::yield_now().await;
            }
        })
        .await
        .expect(
            "the active publisher must queue for the index while retaining its input read guard",
        );
        assert!(backend.analyser_inputs_gate.try_write().is_err());
        let revision = backend.fresh_analysis_cache.lock().unwrap().revision();
        let epoch = backend.diag_inputs_epoch();
        documents.replace_sidecar();
        let mut invalidating = Box::pin(backend.invalidate_sidecar_stubs());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(invalidating.as_mut().poll(&mut context).is_pending());
        assert!(
            backend
                .fresh_analysis_cache
                .lock()
                .unwrap()
                .is_current(&revision)
        );
        assert_eq!(backend.diag_inputs_epoch(), epoch);
        drop(index);
        assert!(
            crate::rt::timeout(std::time::Duration::from_secs(5), publishing)
                .await
                .unwrap()
                .unwrap()
        );
        invalidating.await;
        assert_ne!(backend.diag_inputs_epoch(), epoch);
        assert!(
            !backend
                .fresh_analysis_cache
                .lock()
                .unwrap()
                .is_current(&revision)
        );
        assert_eq!(backend.fresh_analysis_cache.lock().unwrap().len(), 0);
        assert!(
            backend
                .publish_open_index_if_current(&uri, &documents.source, "tcl8.6", 1, seed)
                .await
                .unwrap()
        );
        assert_eq!(backend.fresh_analysis_cache.lock().unwrap().len(), 1);
        let current = backend
            .fresh_analysis_seed(&uri, Arc::clone(&documents.source), "tcl8.6".to_owned())
            .await
            .unwrap();
        assert_eq!(sidecar_names(&current.analysis), vec!["new_command"]);
    }
    #[test]
    fn retained_semantic_query_workers_abstain_after_cancelled_sidecar_invalidation() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .max_blocking_threads(1)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let documents = Documents::new();
            let backend = crate::tests::test_backend();
            documents.publish(&backend).await;
            let uri = &documents.uris[0];
            let old = backend.db_file_analysis_handle(uri).await.unwrap().await.unwrap().unwrap();
            assert_eq!(sidecar_names(&old), vec!["old_command"]);
            assert!(backend.db_compilation_unit_handle(uri).await.unwrap().await.unwrap().is_some());
            assert!(backend.db_semantic_tokens(uri).await.unwrap().await.unwrap().is_some());
            let (resume, receiver) = std::sync::mpsc::channel();
            let entered = Arc::new(tokio::sync::Notify::new());
            let blocker = crate::rt::spawn_blocking({
                let entered = Arc::clone(&entered);
                move || {
                    entered.notify_one();
                    receiver.recv_timeout(std::time::Duration::from_secs(60)).expect("release the blocking pool after invalidation");
                }
            });
            crate::rt::timeout(std::time::Duration::from_secs(10), entered.notified()).await.expect("the blocking pool barrier must start");
            let analysis = backend.db_file_analysis_handle(uri).await.unwrap();
            let unit = backend.db_compilation_unit_handle(uri).await.unwrap();
            let tokens = backend.db_semantic_tokens(uri).await.unwrap();
            documents.replace_sidecar();
            let database = backend.db.lock().await;
            let revision = tcl_lsp_db::database_revision(&database);
            let mut invalidating = Box::pin(backend.invalidate_sidecar_stubs());
            let mut context = std::task::Context::from_waker(std::task::Waker::noop());
            assert!(invalidating.as_mut().poll(&mut context).is_pending());
            drop(invalidating);
            assert_eq!(tcl_lsp_db::database_revision(&database), revision);
            drop(database);
            resume.send(()).unwrap();
            blocker.await.unwrap();
            let abstained = [
                analysis.await.unwrap().is_none(),
                unit.await.unwrap().is_none(),
                tokens.await.unwrap().is_none(),
            ];
            assert_eq!(abstained, [true; 3], "all real workers must reject their retained pre-clear snapshots without relying on Salsa cancellation");
        });
    }

    #[tokio::test]
    async fn semantic_query_handles_abstain_during_invalidation_and_resume_after_recovery() {
        let documents = Documents::new();
        let backend = crate::tests::test_backend();
        documents.publish(&backend).await;
        documents.replace_sidecar();
        let database = backend.db.lock().await;
        let mut invalidating = Box::pin(backend.invalidate_sidecar_stubs());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(invalidating.as_mut().poll(&mut context).is_pending());
        let mut analysis = Box::pin(backend.db_file_analysis_handle(&documents.uris[0]));
        assert!(matches!(
            analysis.as_mut().poll(&mut context),
            std::task::Poll::Ready(None)
        ));
        let mut unit = Box::pin(backend.db_compilation_unit_handle(&documents.uris[0]));
        assert!(matches!(
            unit.as_mut().poll(&mut context),
            std::task::Poll::Ready(None)
        ));
        let mut tokens = Box::pin(backend.db_semantic_tokens(&documents.uris[0]));
        assert!(matches!(
            tokens.as_mut().poll(&mut context),
            std::task::Poll::Ready(None)
        ));
        drop(invalidating);
        drop(database);
        backend.invalidate_sidecar_stubs().await;
        for uri in &documents.uris {
            let analysis = backend
                .db_file_analysis_handle(uri)
                .await
                .unwrap()
                .await
                .unwrap()
                .unwrap();
            assert_eq!(sidecar_names(&analysis), vec!["new_command"]);
            assert!(
                backend
                    .db_compilation_unit_handle(uri)
                    .await
                    .unwrap()
                    .await
                    .unwrap()
                    .is_some()
            );
            assert!(
                backend
                    .db_semantic_tokens(uri)
                    .await
                    .unwrap()
                    .await
                    .unwrap()
                    .is_some()
            );
        }
    }
    async fn cancelled_sidecar_for_diagnostic_fixture(documents: &Documents, backend: &Backend) {
        documents.replace_sidecar();
        let database = backend.db.lock().await;
        let mut invalidating = Box::pin(backend.invalidate_sidecar_stubs());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(invalidating.as_mut().poll(&mut context).is_pending());
        drop(invalidating);
        drop(database);
    }

    async fn open_empty_diagnostic_fixture(documents: &Documents, backend: &Backend) {
        let uri = &documents.uris[0];
        std::fs::write(documents.root.join("first.tcl"), "").unwrap();
        backend
            .documents
            .lock("sidecar diagnostic fixture")
            .await
            .insert(
                uri.clone(),
                DocumentState::with_version(String::new(), "tcl8.6".to_owned(), 1),
            );
        backend
            .db_set_source(uri, "", "tcl8.6".to_owned())
            .await
            .unwrap();
        assert_eq!(
            sidecar_names(&backend.cached_analysis(uri).await.unwrap()),
            vec!["old_command"]
        );
    }

    #[tokio::test]
    async fn cancelled_sidecar_invalidation_does_not_issue_a_complete_push_report_under_the_new_epoch()
     {
        let documents = Documents::new();
        let backend = crate::tests::test_backend();
        open_empty_diagnostic_fixture(&documents, &backend).await;
        let uri = &documents.uris[0];
        let expected = [serde_json::json!({"uri": uri, "version": 1, "source": ""})];
        crate::rt::timeout(
            std::time::Duration::from_secs(30),
            backend.publish_analyser_diagnostics(
                uri.clone(),
                String::new(),
                "tcl8.6".to_owned(),
                0,
                Some(1),
            ),
        )
        .await
        .unwrap();
        let before = backend
            .current_diagnostics_command(&expected)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            before["status"], "complete",
            "the actual provider must first establish a complete report"
        );
        cancelled_sidecar_for_diagnostic_fixture(&documents, &backend).await;
        crate::rt::timeout(
            std::time::Duration::from_secs(30),
            backend.publish_analyser_diagnostics(
                uri.clone(),
                String::new(),
                "tcl8.6".to_owned(),
                0,
                Some(1),
            ),
        )
        .await
        .unwrap();
        let after = backend
            .current_diagnostics_command(&expected)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after["status"], "unavailable");
        assert_eq!(after["reason"], "stale");
        let document = backend
            .documents
            .lock("sidecar diagnostic recovery")
            .await
            .remove(uri)
            .unwrap();
        backend.invalidate_sidecar_stubs().await;
        backend
            .documents
            .lock("sidecar diagnostic recovery")
            .await
            .insert(uri.clone(), document);
        crate::rt::timeout(
            std::time::Duration::from_secs(30),
            backend.publish_analyser_diagnostics(
                uri.clone(),
                String::new(),
                "tcl8.6".to_owned(),
                0,
                Some(1),
            ),
        )
        .await
        .unwrap();
        let recovered = backend
            .current_diagnostics_command(&expected)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(recovered["status"], "complete");
    }

    #[tokio::test]
    async fn direct_pull_refuses_cancelled_sidecar_inputs_and_recovers_after_the_database_refresh()
    {
        use tower_lsp_server::LanguageServer as _;
        let documents = Documents::new();
        let backend = crate::tests::test_backend();
        open_empty_diagnostic_fixture(&documents, &backend).await;
        let uri = &documents.uris[0];
        let params =
            || serde_json::from_value(serde_json::json!({"textDocument": {"uri": uri}})).unwrap();
        assert!(
            crate::rt::timeout(
                std::time::Duration::from_secs(30),
                backend.diagnostic(params())
            )
            .await
            .unwrap()
            .is_ok()
        );
        cancelled_sidecar_for_diagnostic_fixture(&documents, &backend).await;
        let response = crate::rt::timeout(
            std::time::Duration::from_secs(30),
            backend.diagnostic(params()),
        )
        .await
        .unwrap();
        let error =
            response.expect_err("an unrefreshed database must not issue a full direct report");
        assert_eq!(
            error.code,
            tower_lsp_server::jsonrpc::ErrorCode::ContentModified
        );
        assert_eq!(error.data.unwrap()["reason"], "stale");
        let document = backend
            .documents
            .lock("sidecar pull recovery")
            .await
            .remove(uri)
            .unwrap();
        backend.invalidate_sidecar_stubs().await;
        backend
            .documents
            .lock("sidecar pull recovery")
            .await
            .insert(uri.clone(), document);
        assert!(
            crate::rt::timeout(
                std::time::Duration::from_secs(30),
                backend.diagnostic(params())
            )
            .await
            .unwrap()
            .is_ok()
        );
    }

    #[tokio::test]
    async fn project_callback_projection_refuses_the_cancelled_sidecar_generation() {
        let documents = Documents::new();
        let backend = crate::tests::test_backend();
        documents.publish(&backend).await;
        let old = backend.cached_analysis(&documents.uris[0]).await.unwrap();
        let disabled = std::collections::HashSet::new();
        assert!(
            backend
                .project_callback_diagnostics_if(true, &documents.source, &old, &disabled)
                .await
                .is_some(),
            "the actual project projection must first be available"
        );
        cancelled_sidecar_for_diagnostic_fixture(&documents, &backend).await;
        assert!(
            backend
                .project_callback_diagnostics_if(true, &documents.source, &old, &disabled)
                .await
                .is_none()
        );
        backend.invalidate_sidecar_stubs().await;
        let current = backend.cached_analysis(&documents.uris[0]).await.unwrap();
        assert_eq!(sidecar_names(&current), vec!["new_command"]);
        assert!(
            backend
                .project_callback_diagnostics_if(true, &documents.source, &current, &disabled)
                .await
                .is_some()
        );
    }

    #[tokio::test]
    async fn empty_dependency_diagnostic_commit_cannot_publish_after_a_queued_sidecar_writer() {
        let documents = Documents::new();
        let backend = crate::tests::test_backend();
        documents.publish(&backend).await;
        let uri = &documents.uris[0];
        let old = backend.cached_analysis(uri).await.unwrap();
        let inputs = backend
            .diag_inputs(uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
            .await;
        backend.closed_diag_gen.lock().await.insert(uri.clone(), 1);
        let delivery = inputs.delivery_for_job(
            None,
            &documents.source,
            uri,
            crate::DiagCurrency::ClosedFromDisk(1),
            None,
        );
        let dependencies = Arc::new(crate::WorkspaceDiagnosticSources::default());
        assert!(dependencies.is_empty());
        assert_eq!(inputs.inputs_epoch, Some(backend.diag_inputs_epoch()));
        assert!(
            delivery
                .is_current(&*backend.documents.lock("empty dependency currentness").await)
                .await
        );
        assert!(
            !backend
                .workspace_index
                .read()
                .await
                .contains_document(uri.as_str())
        );
        let rehoming = backend.rehoming_gate.lock().await;
        let reader = backend.analyser_inputs_gate.read().await;
        documents.replace_sidecar();
        let mut invalidating = Box::pin(backend.invalidate_sidecar_stubs());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(invalidating.as_mut().poll(&mut context).is_pending());
        let mut publishing = Box::pin(crate::publish_diagnostics_result_with_sources(
            &delivery,
            &backend.workspace_index,
            &backend.rehomed_source_seeds,
            &backend.rehoming_gate,
            &old,
            crate::DiagnosticPublicationSources {
                result: (
                    Err(crate::DiagnosticUnavailable::Context),
                    Ok(Vec::new()),
                    None,
                ),
                workspace_sources: Some(dependencies),
            },
            crate::PublishTiming {
                started: crate::rt::Instant::now(),
                uri_str: uri.as_str(),
                line_count: 1,
            },
        ));
        assert!(publishing.as_mut().poll(&mut context).is_pending());
        drop(reader);
        crate::rt::timeout(std::time::Duration::from_secs(10), invalidating)
            .await
            .unwrap();
        drop(rehoming);
        let _settled = crate::rt::timeout(std::time::Duration::from_secs(10), publishing)
            .await
            .unwrap();
        assert!(
            !backend
                .workspace_index
                .read()
                .await
                .contains_document(uri.as_str()),
            "the old empty-dependency publication must not mutate the index after invalidation"
        );
        assert!(
            !backend.pull_diag_cache.lock().await.contains_key(uri),
            "the stale run must not deposit a pull-cache or mailbox publication"
        );
    }

    #[tokio::test]
    async fn fast_tier_joins_input_writer_order_and_refuses_a_retired_sidecar_token() {
        let documents = Documents::new();
        let backend = crate::tests::test_backend();
        documents.publish(&backend).await;
        let uri = documents.uris[0].clone();
        backend
            .documents
            .lock("sidecar fast-tier fixture")
            .await
            .insert(
                uri.clone(),
                DocumentState::with_version(documents.source.to_string(), "tcl8.6".to_owned(), 1),
            );
        let inputs = backend
            .diag_inputs(&uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
            .await;
        let delivery = inputs.delivery_for_job(
            None,
            &documents.source,
            &uri,
            crate::DiagCurrency::Open(0),
            Some(1),
        );
        assert!(!delivery.client_supports_pull);
        assert_eq!(inputs.inputs_epoch, Some(backend.diag_inputs_epoch()));
        assert!(
            delivery
                .is_current(&*backend.documents.lock("fast-tier currentness").await)
                .await
        );
        // Coalesce the replacement into its existing scheduler slot; this
        // control drives the old fast-tier publisher itself.
        backend.diag_slots.lock().await.insert(
            uri.clone(),
            crate::DiagSlot {
                running: true,
                ..Default::default()
            },
        );
        let reader = backend.analyser_inputs_gate.read().await;
        documents.replace_sidecar();
        let mut invalidating = Box::pin(backend.invalidate_sidecar_stubs());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(invalidating.as_mut().poll(&mut context).is_pending());
        let mut fast = Box::pin(delivery.deliver_fast_tier_if_current(Vec::new()));
        assert!(
            fast.as_mut().poll(&mut context).is_pending(),
            "a fast-tier deposit must join the already queued input writer"
        );
        drop(reader);
        crate::rt::timeout(std::time::Duration::from_secs(10), invalidating)
            .await
            .unwrap();
        assert!(
            crate::rt::timeout(std::time::Duration::from_secs(10), fast)
                .await
                .unwrap()
                .is_none()
        );
        assert!(!backend.pull_diag_cache.lock().await.contains_key(&uri));
    }

    #[tokio::test]
    async fn suppressed_delivery_uses_its_original_configuration_epoch_without_a_registry_donor() {
        for excluded in [false, true] {
            let documents = Documents::new();
            let backend = crate::tests::test_backend();
            open_empty_diagnostic_fixture(&documents, &backend).await;
            let uri = &documents.uris[0];
            assert_selected_registry(&backend, uri, false).await;
            select_suppressed_missing_registry(&backend, uri, excluded).await;
            assert_selected_registry(&backend, uri, true).await;
            let make_prior = || crate::PullDiagEntry {
                currency: None,
                result_id: "before-suppression".to_owned(),
                revision: 0,
                diagnostics: vec![tower_lsp_server::ls_types::Diagnostic {
                    message: "previous squiggle".to_owned(),
                    ..Default::default()
                }],
            };
            backend
                .pull_diag_cache
                .lock()
                .await
                .insert(uri.clone(), make_prior());
            let inputs = backend
                .diag_inputs(uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
                .await;
            assert_eq!(inputs.inputs_epoch, Some(inputs.configuration_epoch));
            assert_eq!(inputs.configuration_epoch, backend.diag_inputs_epoch());
            assert_eq!(inputs.toggles.excluded, excluded);
            assert_eq!(inputs.toggles.diagnostics_enabled, excluded);
            let job = inputs.capture_job(uri).await.unwrap();
            assert!(
                job.diagnostic_pass.is_some(),
                "current policy-clear receipt"
            );
            let _settled = crate::rt::timeout(
                std::time::Duration::from_secs(10),
                crate::run_diagnostics_core(inputs, uri, job),
            )
            .await
            .unwrap();
            let cache = backend.pull_diag_cache.lock().await;
            let cleared = cache.get(uri).unwrap();
            assert!(
                cleared.diagnostics.is_empty(),
                "registry unavailability must not block the selected suppression policy"
            );
            assert_ne!(cleared.result_id, "before-suppression");
            drop(cache);
            backend
                .pull_diag_cache
                .lock()
                .await
                .insert(uri.clone(), make_prior());
            let old_inputs = backend
                .diag_inputs(uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
                .await;
            let job = old_inputs.capture_job(uri).await.unwrap();
            {
                let _inputs = backend.analyser_inputs_gate.write().await;
                backend
                    .feature_toggles
                    .lock()
                    .await
                    .set
                    .insert("diagnostics".to_owned(), true);
                backend.diagnostics_exclude.lock().await.clear();
                backend.invalidate_diag_inputs();
            }
            let _settled = crate::rt::timeout(
                std::time::Duration::from_secs(10),
                crate::run_diagnostics_core(old_inputs, uri, job),
            )
            .await
            .unwrap();
            let cache = backend.pull_diag_cache.lock().await;
            let retained = cache.get(uri).unwrap();
            assert_eq!(
                retained.result_id, "before-suppression",
                "old toggles cannot borrow the new configuration epoch"
            );
            assert_eq!(retained.diagnostics.len(), 1);
            drop(cache);
            let enabled = backend
                .diag_inputs(uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
                .await;
            assert!(enabled.inputs_epoch.is_none());
            assert!(
                enabled
                    .capture_job(uri)
                    .await
                    .unwrap()
                    .diagnostic_pass
                    .is_none()
            );
            assert_selected_registry(&backend, uri, true).await;
        }
    }

    async fn assert_selected_registry(backend: &Backend, uri: &Uri, unavailable: bool) {
        let config = backend.resolved_db_config(uri).await;
        let file = *backend.db_files.lock().await.get(uri).unwrap();
        let db = backend.db.lock().await;
        let input = tcl_lsp_db::document_analysis_input(&*db, file, config);
        if unavailable {
            let miss = input.expect_err("the actual selected overlay was not installed");
            assert_eq!(miss.overlay, u64::MAX);
            assert_eq!(miss.environment, "tcl8.6");
        } else {
            assert_eq!(input.unwrap().unit_profile().name, "tcl8.6");
        }
    }

    async fn select_suppressed_missing_registry(backend: &Backend, uri: &Uri, excluded: bool) {
        use salsa::Setter as _;
        let _inputs = backend.analyser_inputs_gate.write().await;
        if excluded {
            *backend.diagnostics_exclude.lock().await = vec!["*.tcl".to_owned()];
        } else {
            backend
                .feature_toggles
                .lock()
                .await
                .set
                .insert("diagnostics".to_owned(), false);
        }
        let config = backend.resolved_db_config(uri).await;
        let mut db = backend.db.lock().await;
        config.set_spec_pack_key(&mut *db).to(u64::MAX);
        backend.invalidate_diag_inputs();
    }
}
