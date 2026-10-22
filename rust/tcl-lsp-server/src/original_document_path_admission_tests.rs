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

use super::tests::test_backend;
use super::*;
use tower_lsp_server::ls_types::{
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem,
    VersionedTextDocumentIdentifier,
};

fn uri(raw: &str) -> Uri {
    Uri::from_str(raw).unwrap()
}

fn native_uri(name: &str) -> Uri {
    canonical_file_uri(std::env::temp_dir().join(name)).unwrap()
}

fn opened(uri: &Uri, text: &str) -> DidOpenTextDocumentParams {
    DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "tcl8.6".to_owned(),
            version: 1,
            text: text.to_owned(),
        },
    }
}

fn changed(uri: &Uri, text: &str) -> DidChangeTextDocumentParams {
    DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: text.to_owned(),
        }],
    }
}

fn retained_analysis() -> AnalysisResult {
    Analyser::new()
        .analyse("proc retained_path_row {} {}", "tcl8.6")
        .clone()
}

#[test]
fn source_path_admission_distinguishes_nonfile_unicode_and_refused_file_data() {
    let nonfile = uri("untitled:retained-source");
    let admitted = DocumentPathAdmission::capture(&nonfile, &vfs::NativeStore).unwrap();
    assert!(matches!(
        admitted,
        DocumentPathAdmission::NoFilesystemPath { .. }
    ));
    assert_eq!(admitted.analyser_path(), None);
    let unicode = native_uri("source-é-�.tcl");
    let admitted = DocumentPathAdmission::capture(&unicode, &vfs::NativeStore).unwrap();
    assert_eq!(admitted.path().unwrap().to_str(), admitted.analyser_path());
    assert!(admitted.analyser_path().unwrap().contains("source-é-�.tcl"));
    for raw in [
        "file:///source-%FF.tcl",
        "file:///source-%00.tcl",
        "file:///a%2Fb.tcl",
        "file://user@host/share/a.tcl",
    ] {
        assert!(
            matches!(
                DocumentPathAdmission::capture(&uri(raw), &vfs::NativeStore),
                Err(DocumentSourceUnavailable::Path { .. })
            ),
            "{raw}"
        );
    }
    assert!(matches!(
        admitted.check_uri(&nonfile),
        Err(DocumentSourceUnavailable::ForeignReceipt { .. })
    ));
}

#[tokio::test]
async fn refused_open_and_unknown_change_publish_no_document_source_or_index() {
    let backend = test_backend();
    for raw in [
        "file:///open-%FF.tcl",
        "file:///open-%00.tcl",
        "file:///open%2Fsplit.tcl",
    ] {
        let target = uri(raw);
        Box::pin(backend
            .did_open(opened(&target, "proc rejected {} {}")))
            .await;
        backend
            .did_change(changed(&target, "proc changed_rejected {} {}"))
            .await;
        assert!(
            !backend
                .documents
                .lock("path_admission_test")
                .await
                .contains_key(&target)
        );
        assert!(!backend.db_files.lock().await.contains_key(&target));
        assert!(
            !backend
                .workspace_index
                .read()
                .await
                .contains_document(target.as_str())
        );
        assert!(
            backend
                .fresh_analysis_for(
                    &target,
                    Arc::from("proc refused {} {}"),
                    "tcl8.6".to_owned()
                )
                .await
                .is_err()
        );
        assert!(backend.read_document(&target).await.is_err());
    }
}

#[tokio::test]
async fn refused_known_change_retains_exact_prior_document_and_index() {
    let backend = test_backend();
    let target = uri("file:///known-%FF.tcl");
    let prior = DocumentState::with_version(
        "proc retained_path_row {} {}".to_owned(),
        "tcl8.6".to_owned(),
        17,
    );
    backend
        .documents
        .lock("path_admission_test")
        .await
        .insert(target.clone(), prior.clone());
    backend
        .workspace_index
        .write()
        .await
        .add_document(target.as_str(), &retained_analysis());
    backend
        .did_change(changed(&target, "proc must_not_replace {} {}"))
        .await;
    let current = backend
        .documents
        .lock("path_admission_test")
        .await
        .get(&target)
        .unwrap()
        .clone();
    assert!(Arc::ptr_eq(&current.text, &prior.text));
    assert_eq!(current.version, prior.version);
    assert_eq!(current.revision, prior.revision);
    assert_eq!(current.publication, prior.publication);
    assert_eq!(current.path_admission, prior.path_admission);
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .contains_document(target.as_str())
    );
    assert!(!backend.db_files.lock().await.contains_key(&target));
    backend
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: target.clone(),
            },
        })
        .await;
    backend
        .did_change_watched_files(DidChangeWatchedFilesParams {
            changes: vec![tower_lsp_server::ls_types::FileEvent {
                uri: target.clone(),
                typ: FileChangeType::DELETED,
            }],
        })
        .await;
    let current = backend
        .documents
        .lock("path_admission_test")
        .await
        .get(&target)
        .unwrap()
        .clone();
    assert!(Arc::ptr_eq(&current.text, &prior.text));
    assert_eq!(current.version, prior.version);
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .contains_document(target.as_str())
    );
}

#[tokio::test]
async fn foreign_live_receipt_is_terminal_before_change_or_fresh_cache_reuse() {
    let backend = test_backend();
    let target = native_uri("actual-source.tcl");
    let foreign = Arc::new(
        DocumentPathAdmission::capture(&native_uri("foreign-source.tcl"), backend.store.as_ref())
            .unwrap(),
    );
    let prior = DocumentState::with_version("set retained 1".to_owned(), "tcl8.6".to_owned(), 7)
        .with_path_admission(foreign);
    backend
        .documents
        .lock("path_admission_test")
        .await
        .insert(target.clone(), prior.clone());
    backend.did_change(changed(&target, "set replaced 2")).await;
    let current = backend
        .documents
        .lock("path_admission_test")
        .await
        .get(&target)
        .unwrap()
        .clone();
    assert!(Arc::ptr_eq(&current.text, &prior.text));
    assert_eq!(current.version, prior.version);
    assert!(matches!(
        backend
            .fresh_analysis_seed(&target, Arc::clone(&prior.text), prior.dialect.clone())
            .await,
        Err(DocumentSourceUnavailable::ForeignReceipt { .. })
    ));
    assert!(!backend.db_files.lock().await.contains_key(&target));
}

#[tokio::test]
async fn unicode_and_nonfile_receipts_reach_salsa_and_fresh_analysis() {
    let backend = test_backend();
    for target in [
        native_uri("checked-é-�.tcl"),
        uri("untitled:checked-source"),
    ] {
        let source = "set retained 1";
        Box::pin(backend.did_open(opened(&target, source))).await;
        let state = backend
            .documents
            .lock("path_admission_test")
            .await
            .get(&target)
            .unwrap()
            .clone();
        let admission = state.path_admission.as_ref().unwrap();
        admission.check_uri(&target).unwrap();
        let file = *backend.db_files.lock().await.get(&target).unwrap();
        let actual_path = {
            let db = backend.db.lock().await;
            file.path(&*db).clone()
        };
        assert_eq!(actual_path.as_deref(), admission.analyser_path());
        let seed = backend
            .fresh_analysis_seed(&target, Arc::clone(&state.text), state.dialect.clone())
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&seed.path_admission, admission));
        assert_eq!(seed.path_admission.analyser_path(), actual_path.as_deref());
        assert!(tcl_compiler::source_graph::current_analysis(source, &seed.analysis).is_some());
        backend.did_change(changed(&target, "set retained 2")).await;
        let current = backend
            .documents
            .lock("path_admission_test")
            .await
            .get(&target)
            .unwrap()
            .clone();
        assert_eq!(current.version, Some(2));
        assert_eq!(current.text.as_ref(), "set retained 2");
        assert!(Arc::ptr_eq(
            current.path_admission.as_ref().unwrap(),
            admission
        ));
    }
}

#[tokio::test]
async fn disk_unavailability_preserves_prior_rows_but_genuine_missing_file_retires() {
    let backend = test_backend();
    let invalid = uri("file:///disk-%FF.tcl");
    backend
        .workspace_index
        .write()
        .await
        .add_document(invalid.as_str(), &retained_analysis());
    assert!(backend.reindex_index_from_disk(&invalid).await.is_err());
    assert!(
        backend
            .batch_reindex_from_disk(std::slice::from_ref(&invalid))
            .await
            .is_err()
    );
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .contains_document(invalid.as_str())
    );

    let missing_path = std::env::temp_dir().join(format!(
        "tcl-lsp-admission-missing-{}-{}.tcl",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    assert!(!missing_path.exists());
    let missing = canonical_file_uri(&missing_path).unwrap();
    backend
        .db_set_source(
            &missing,
            "proc retained_path_row {} {}",
            "tcl8.6".to_owned(),
        )
        .await
        .unwrap();
    backend
        .workspace_index
        .write()
        .await
        .add_document(missing.as_str(), &retained_analysis());
    // An invalid member refuses the whole prepared batch before the valid
    // missing member can remove its previous source/index row.
    assert!(
        backend
            .batch_reindex_from_disk(&[missing.clone(), invalid.clone()])
            .await
            .is_err()
    );
    assert!(backend.db_files.lock().await.contains_key(&missing));
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .contains_document(missing.as_str())
    );
    backend.reindex_index_from_disk(&missing).await.unwrap();
    assert!(!backend.db_files.lock().await.contains_key(&missing));
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .contains_document(missing.as_str())
    );
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .contains_document(invalid.as_str())
    );
}

#[tokio::test]
async fn disk_replacement_and_removal_preflight_is_atomic_on_refusal() {
    let backend = test_backend();
    let retained = native_uri("disk-retained.tcl");
    backend
        .db_set_source(
            &retained,
            "proc retained_path_row {} {}",
            "tcl8.6".to_owned(),
        )
        .await
        .unwrap();
    backend
        .workspace_index
        .write()
        .await
        .add_document(retained.as_str(), &retained_analysis());
    let invalid = uri("file:///disk-replacement-%FF.tcl");
    let replacements = vec![(
        invalid.clone(),
        "proc refused {} {}".to_owned(),
        "tcl8.6".to_owned(),
        retained_analysis(),
    )];
    assert!(
        backend
            .publish_disk_results(
                &replacements,
                std::slice::from_ref(&retained),
                DiskRemovalPolicy::ClosedOnly,
                "path_admission_test"
            )
            .await
            .is_err()
    );
    assert!(backend.db_files.lock().await.contains_key(&retained));
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .contains_document(retained.as_str())
    );
    assert!(!backend.db_files.lock().await.contains_key(&invalid));
}

#[tokio::test]
async fn malformed_uri_never_reads_the_valid_replacement_character_neighbour() {
    let mut backend = test_backend();
    let neighbour_path = std::env::temp_dir().join("source-admission-�-neighbour.tcl");
    let neighbour = canonical_file_uri(&neighbour_path).unwrap();
    assert!(neighbour.as_str().contains("%EF%BF%BD"));
    let malformed = uri(&neighbour.as_str().replace("%EF%BF%BD", "%FF"));
    let store = vfs::MemoryStore::new();
    store.upsert(neighbour_path, b"proc replacement_neighbour {} {}".to_vec());
    backend.store = Arc::new(store);
    let valid = backend.read_document(&neighbour).await.unwrap().unwrap();
    assert_eq!(valid.text.as_ref(), "proc replacement_neighbour {} {}");
    assert_eq!(valid.path_admission.as_ref().unwrap().uri(), &neighbour);
    assert!(matches!(
        backend.read_document(&malformed).await,
        Err(DocumentSourceUnavailable::Path { .. })
    ));
    assert!(backend.reindex_index_from_disk(&malformed).await.is_err());
    assert!(!backend.db_files.lock().await.contains_key(&malformed));
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .contains_document(malformed.as_str())
    );
}

#[tokio::test]
async fn virtual_registered_path_reaches_actual_source_and_fresh_analysis() {
    let mut backend = test_backend();
    let store = Arc::new(vfs::MemoryStore::new());
    let target = uri("file:///client/source.tcl");
    let actual = PathBuf::from("/actual-browser-mount/distinct-é.tcl");
    store.upsert_uri(target.as_str(), &actual, b"set retained 1".to_vec());
    backend.store = store.clone();
    Box::pin(backend.did_open(opened(&target, "set retained 1"))).await;
    let doc = backend
        .documents
        .lock("virtual_path_test")
        .await
        .get(&target)
        .unwrap()
        .clone();
    let admission = doc.path_admission.as_ref().unwrap();
    assert!(matches!(
        admission.as_ref(),
        DocumentPathAdmission::Virtual { .. }
    ));
    assert_eq!(admission.path(), Some(actual.as_path()));
    let file = *backend.db_files.lock().await.get(&target).unwrap();
    assert_eq!(
        file.path(&*backend.db.lock().await).as_deref(),
        actual.to_str()
    );
    let seed = backend
        .fresh_analysis_seed(&target, Arc::clone(&doc.text), doc.dialect.clone())
        .await
        .unwrap();
    assert_eq!(seed.path_admission.analyser_path(), actual.to_str());
    store.upsert_uri(
        target.as_str(),
        "/replacement-browser-mount/other.tcl",
        b"set replaced 2".to_vec(),
    );
    backend
        .did_change(changed(&target, "set must_not_publish 3"))
        .await;
    let current = backend
        .documents
        .lock("virtual_path_test")
        .await
        .get(&target)
        .unwrap()
        .clone();
    assert!(Arc::ptr_eq(&current.text, &doc.text));
    assert_eq!(current.version, doc.version);
    assert!(
        backend
            .fresh_analysis_seed(&target, Arc::clone(&doc.text), doc.dialect.clone())
            .await
            .is_err()
    );
    assert_eq!(
        file.path(&*backend.db.lock().await).as_deref(),
        actual.to_str()
    );
}

#[tokio::test]
async fn unregistered_virtual_open_requires_no_native_host_or_vfs_upsert() {
    let mut backend = test_backend();
    backend.store = Arc::new(vfs::MemoryStore::new());
    let target = uri(&tcl_lsp_core::file_uri::from_path(
        std::path::Path::new("/browser/open-é.tcl"),
        tcl_lsp_core::file_uri::FileUriEscaping::Unreserved,
    )
    .unwrap());
    Box::pin(backend.did_open(opened(&target, "set browser 1"))).await;
    let docs = backend.documents.lock("virtual_open_test").await;
    let admission = docs.get(&target).unwrap().path_admission.as_ref().unwrap();
    assert!(matches!(
        admission.as_ref(),
        DocumentPathAdmission::Virtual { .. }
    ));
    assert_eq!(admission.analyser_path(), Some("/browser/open-é.tcl"));
    admission
        .check_for(&target, backend.store.as_ref())
        .unwrap();
}

#[derive(Debug)]
struct RefusingReadStore {
    actual: Arc<vfs::MemoryStore>,
}
impl vfs::SourceStore for RefusingReadStore {
    fn document_uri_path(
        &self,
        uri: &str,
    ) -> Result<vfs::SourceUriPath, vfs::SourceUriPathUnavailable> {
        self.actual.document_uri_path(uri)
    }
    fn source_path_origin(
        &self,
        path: &Path,
    ) -> Result<vfs::SourcePathOrigin, vfs::SourceUriPathUnavailable> {
        self.actual.source_path_origin(path)
    }
    fn validate_source_path_origin(
        &self,
        origin: &vfs::SourcePathOrigin,
    ) -> Result<(), vfs::SourceUriPathUnavailable> {
        self.actual.validate_source_path_origin(origin)
    }
    fn validate_document_uri_path(
        &self,
        selected: &vfs::SourceUriPath,
    ) -> Result<(), vfs::SourceUriPathUnavailable> {
        self.actual.validate_document_uri_path(selected)
    }
    fn with_current_document_paths(
        &self,
        selected: &[&vfs::SourceUriPath],
        publish: &mut dyn FnMut(),
    ) -> Result<(), vfs::SourceUriPathUnavailable> {
        self.actual.with_current_document_paths(selected, publish)
    }
    fn read(&self, _path: &Path) -> std::io::Result<Vec<u8>> {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "actual provider read refused",
        ))
    }
    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        self.read(path)
            .map(|bytes| String::from_utf8(bytes).unwrap())
    }
    fn read_dir(&self, path: &Path) -> std::io::Result<Vec<vfs::DirEntry>> {
        self.actual.read_dir(path)
    }
    fn metadata(&self, path: &Path) -> std::io::Result<vfs::Metadata> {
        self.actual.metadata(path)
    }
    fn write(&self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        self.actual.write(path, bytes)
    }
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        self.actual.create_dir_all(path)
    }
}

#[tokio::test]
async fn admitted_provider_read_failure_is_unavailable_and_cannot_retire_rows() {
    let mut backend = test_backend();
    let store = Arc::new(vfs::MemoryStore::new());
    let target = uri("file:///browser/blocked.tcl");
    store.upsert_uri(
        target.as_str(),
        "/browser/blocked.tcl",
        b"proc retained_path_row {} {}".to_vec(),
    );
    backend.store = store.clone();
    backend
        .db_set_source(&target, "proc retained_path_row {} {}", "tcl8.6".to_owned())
        .await
        .unwrap();
    backend
        .workspace_index
        .write()
        .await
        .add_document(target.as_str(), &retained_analysis());
    let file = *backend.db_files.lock().await.get(&target).unwrap();
    backend.store = Arc::new(RefusingReadStore { actual: store });
    assert!(
        matches!(backend.read_document(&target).await, Err(DocumentSourceUnavailable::Read { cause, .. }) if cause.kind() == std::io::ErrorKind::PermissionDenied)
    );
    assert!(matches!(
        backend.reindex_index_from_disk(&target).await,
        Err(DocumentSourceUnavailable::Read { .. })
    ));
    assert!(
        backend
            .batch_reindex_from_disk(std::slice::from_ref(&target))
            .await
            .is_err()
    );
    assert!(backend.db_files.lock().await.get(&target) == Some(&file));
    assert_eq!(
        file.text(&*backend.db.lock().await),
        "proc retained_path_row {} {}"
    );
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .contains_document(target.as_str())
    );
}

#[tokio::test]
async fn missing_scan_receipt_cannot_retire_a_remapped_uri() {
    let mut backend = test_backend();
    let store = Arc::new(vfs::MemoryStore::new());
    let target = uri("file:///browser/missing-before-remap.tcl");
    let original_path = Path::new("/original/missing.tcl");
    store.upsert_uri(
        target.as_str(),
        original_path,
        b"proc retained_path_row {} {}".to_vec(),
    );
    backend.store = store.clone();
    backend
        .db_set_source(&target, "proc retained_path_row {} {}", "tcl8.6".to_owned())
        .await
        .unwrap();
    backend
        .workspace_index
        .write()
        .await
        .add_document(target.as_str(), &retained_analysis());
    let file = *backend.db_files.lock().await.get(&target).unwrap();
    store.remove(original_path);
    let admission = backend.document_path_admission(&target).await.unwrap();
    let driver = backend.fresh_analysis_driver(admission).await;
    let scanned = Backend::scan_disk_file(
        backend.store.clone(),
        target.clone(),
        Arc::new(HashMap::new()),
        Arc::new(Vec::new()),
        Arc::new("tcl8.6".to_owned()),
        driver,
    )
    .await;
    let DiskScanOutcome::Absent(admission) = scanned else {
        panic!("the actual selected original path is missing");
    };
    store.upsert_uri(
        target.as_str(),
        "/replacement/live.tcl",
        b"proc replacement {} {}".to_vec(),
    );
    let batch = AdmittedDiskBatch {
        removal_admissions: vec![admission],
        ..Default::default()
    };
    assert!(
        backend
            .publish_scanned_disk_results(
                &batch,
                std::slice::from_ref(&target),
                DiskRemovalPolicy::ClosedOnly,
                "withdrawn_missing_scan"
            )
            .await
            .is_err()
    );
    assert!(backend.db_files.lock().await.get(&target) == Some(&file));
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .contains_document(target.as_str())
    );
}

#[tokio::test]
async fn opening_waiting_for_index_cannot_retire_after_mapping_withdrawal() {
    let mut backend = test_backend();
    let store = Arc::new(vfs::MemoryStore::new());
    let target = uri("file:///browser/open-waits-for-index.tcl");
    store.upsert_uri(
        target.as_str(),
        "/original/open.tcl",
        b"proc retained_path_row {} {}".to_vec(),
    );
    backend.store = store.clone();
    backend
        .workspace_index
        .write()
        .await
        .add_document(target.as_str(), &retained_analysis());
    let backend = Arc::new(backend);
    let index = backend.workspace_index.read().await;
    let worker_backend = Arc::clone(&backend);
    let worker_target = target.clone();
    let opening = tokio::spawn(async move {
        Box::pin(worker_backend
            .did_open(opened(&worker_target, "set new_open 1")))
            .await;
    });
    tokio::time::timeout(std::time::Duration::from_secs(180), async {
        loop {
            if backend
                .documents
                .lock("wait_for_leased_open")
                .await
                .contains_key(&target)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    store.upsert_uri(
        target.as_str(),
        "/replacement/open.tcl",
        b"set replacement 1".to_vec(),
    );
    drop(index);
    tokio::time::timeout(std::time::Duration::from_secs(180), opening)
        .await
        .unwrap()
        .unwrap();
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .contains_document(target.as_str())
    );
    assert!(!backend.db_files.lock().await.contains_key(&target));
}

#[tokio::test]
async fn closed_cached_source_keeps_its_original_mapping_until_genuine_republication() {
    let mut backend = test_backend();
    let store = Arc::new(vfs::MemoryStore::new());
    let target = uri("file:///browser/closed-cached.tcl");
    let original = "set original_path 1";
    store.upsert_uri(
        target.as_str(),
        "/original/cached.tcl",
        original.as_bytes().to_vec(),
    );
    backend.store = store.clone();
    backend
        .db_set_source(&target, original, "tcl8.6".to_owned())
        .await
        .unwrap();
    let retained = backend
        .analysis_for(&target, Arc::from(original), "tcl8.6".to_owned())
        .await
        .unwrap();
    let file = *backend.db_files.lock().await.get(&target).unwrap();
    assert_eq!(
        file.path(&*backend.db.lock().await).as_deref(),
        Some("/original/cached.tcl")
    );
    store.upsert_uri(
        target.as_str(),
        "/replacement/cached.tcl",
        b"set replacement_path 2".to_vec(),
    );
    assert!(backend.read_document(&target).await.is_err());
    assert!(
        backend
            .analysis_for(&target, Arc::from(original), "tcl8.6".to_owned())
            .await
            .is_err()
    );
    assert_eq!(
        file.path(&*backend.db.lock().await).as_deref(),
        Some("/original/cached.tcl")
    );
    backend.reindex_index_from_disk(&target).await.unwrap();
    let current = backend.read_document(&target).await.unwrap().unwrap();
    assert_eq!(current.text.as_ref(), "set replacement_path 2");
    assert_eq!(
        file.path(&*backend.db.lock().await).as_deref(),
        Some("/replacement/cached.tcl")
    );
    let replacement = backend
        .analysis_for(&target, Arc::clone(&current.text), current.dialect)
        .await
        .unwrap();
    assert!(!Arc::ptr_eq(&retained, &replacement));
}

#[tokio::test]
async fn equal_path_aba_and_equal_foreign_provider_cannot_authenticate_old_cached_source() {
    let mut backend = test_backend();
    let store = Arc::new(vfs::MemoryStore::new());
    let target = uri("file:///browser/closed-aba.tcl");
    let path = "/same/closed-aba.tcl";
    let source = "set same_bytes 1";
    store.upsert_uri(target.as_str(), path, source.as_bytes().to_vec());
    backend.store = store.clone();
    backend
        .db_set_source(&target, source, "tcl8.6".to_owned())
        .await
        .unwrap();
    backend
        .analysis_for(&target, Arc::from(source), "tcl8.6".to_owned())
        .await
        .unwrap();
    store.upsert_uri(
        target.as_str(),
        "/different/closed-aba.tcl",
        source.as_bytes().to_vec(),
    );
    store.upsert_uri(target.as_str(), path, source.as_bytes().to_vec());
    assert!(
        backend
            .analysis_for(&target, Arc::from(source), "tcl8.6".to_owned())
            .await
            .is_err()
    );
    backend.reindex_index_from_disk(&target).await.unwrap();
    backend
        .analysis_for(&target, Arc::from(source), "tcl8.6".to_owned())
        .await
        .unwrap();
    let foreign = Arc::new(vfs::MemoryStore::new());
    foreign.upsert_uri(target.as_str(), path, source.as_bytes().to_vec());
    backend.store = foreign;
    assert!(
        backend
            .analysis_for(&target, Arc::from(source), "tcl8.6".to_owned())
            .await
            .is_err()
    );
}

#[cfg(windows)]
#[test]
fn actual_windows_scan_path_keeps_drive_and_verbatim_spelling_separate_from_uri() {
    for path in [
        Path::new(r"C:\MiXeD\Probe.tcl"),
        Path::new(r"\\?\C:\MiXeD\Probe.tcl"),
    ] {
        let target = canonical_file_uri(path).unwrap();
        assert!(
            target.as_str().starts_with("file:///c%3A/")
                || target.as_str().starts_with("file:///c:/")
        );
        let raw = DocumentPathAdmission::capture(&target, &vfs::NativeStore).unwrap();
        assert_ne!(raw.path().unwrap().as_os_str(), path.as_os_str());
        let produced =
            DocumentPathAdmission::from_selected_path(&target, path, &vfs::NativeStore).unwrap();
        assert_eq!(produced.path().unwrap().as_os_str(), path.as_os_str());
        assert_eq!(produced.analyser_path(), path.to_str());
        produced.check_for(&target, &vfs::NativeStore).unwrap();
        let DocumentPathAdmission::Native {
            selected,
            source_origin: Some(origin),
            ..
        } = produced
        else {
            panic!("actual Windows provider issues the native Path origin");
        };
        assert_eq!(selected.path().as_os_str(), raw.path().unwrap().as_os_str());
        assert_eq!(origin.path().as_os_str(), path.as_os_str());
    }
}

#[cfg(unix)]
#[test]
fn actual_virtual_scan_path_refuses_a_stronger_uri_alias_to_another_path() {
    let store = vfs::MemoryStore::new();
    let actual = Path::new("/actual/scan-source.tcl");
    let target = canonical_file_uri(actual).unwrap();
    let produced = DocumentPathAdmission::from_selected_path(&target, actual, &store).unwrap();
    assert_eq!(produced.path().unwrap().as_os_str(), actual.as_os_str());
    assert!(matches!(
        &produced,
        DocumentPathAdmission::Virtual {
            source_origin: Some(_),
            ..
        }
    ));
    store.upsert_uri(
        target.as_str(),
        "/stronger/other-source.tcl",
        b"set other 1".to_vec(),
    );
    assert!(matches!(
        DocumentPathAdmission::from_selected_path(&target, actual, &store),
        Err(DocumentSourceUnavailable::Store {
            cause: vfs::SourceUriPathUnavailable::PathOriginMismatch,
            ..
        })
    ));
    assert!(produced.check_for(&target, &store).is_err());
}
