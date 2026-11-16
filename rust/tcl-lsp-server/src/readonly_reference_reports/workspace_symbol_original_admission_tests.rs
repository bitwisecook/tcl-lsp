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
use std::collections::HashMap;
use tower_lsp_server::LanguageServer;
use tower_lsp_server::ls_types::{
    SymbolInformation, WorkspaceSymbolParams, WorkspaceSymbolResponse,
};

const SOURCE: &str = "proc first_symbol {} {}\nproc second_symbol {} {}\n";

async fn actual_symbols(backend: &Backend) -> Vec<SymbolInformation> {
    let response = backend
        .symbol(WorkspaceSymbolParams {
            query: "symbol".to_owned(),
            work_done_progress_params: tower_lsp_server::ls_types::WorkDoneProgressParams::default(
            ),
            partial_result_params: tower_lsp_server::ls_types::PartialResultParams::default(),
        })
        .await
        .unwrap();
    let Some(WorkspaceSymbolResponse::Flat(symbols)) = response else {
        panic!("expected genuine current symbols, got {response:?}");
    };
    assert_eq!(symbols.len(), 2);
    symbols
}

async fn assert_alias_has_no_open_rows(backend: &Backend, uri: &Uri) {
    assert!(
        !backend
            .documents
            .lock("original alias no open row")
            .await
            .contains_key(uri)
    );
    assert!(!backend.db_files.lock().await.contains_key(uri));
}

fn assert_alias_admission(
    selected: &WorkspaceSymbolSourceAnalysis,
    retained: &DocumentPathAdmission,
    snapshot: &super::super::DocumentState,
    uri: &Uri,
    store: &crate::vfs::MemoryStore,
) {
    assert_ne!(WorkspaceSymbolSourceAnalysis::admission(selected), retained);
    WorkspaceSymbolSourceAnalysis::admission(selected)
        .check_for(uri, store)
        .unwrap();
    assert_eq!(snapshot.text.as_ref(), SOURCE);
    assert_eq!(
        retained.path(),
        WorkspaceSymbolSourceAnalysis::admission(selected).path()
    );
}

async fn assert_alias_generations(
    backend: &Backend,
    uri: &Uri,
    revision: Option<u64>,
    inputs_epoch: u64,
    factory_generation: u64,
) {
    assert_eq!(
        backend
            .workspace_index
            .read()
            .await
            .document_revision(uri.as_str()),
        revision
    );
    assert_eq!(backend.diag_inputs_epoch(), inputs_epoch);
    assert_eq!(
        *backend.class_factory_generation.read().await,
        factory_generation
    );
}

async fn assert_alias_response_withdrawn(
    backend: &Backend,
    analyses: &HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
    rows: &[(
        SymbolInformation,
        Uri,
        super::super::DocumentState,
        Option<u64>,
    )],
    inputs_epoch: u64,
    factory_generation: u64,
) {
    assert!(matches!(
        backend
            .finish_workspace_symbol_response(analyses, rows, inputs_epoch, factory_generation)
            .await,
        Err(super::super::DocumentSourceUnavailable::Store {
            cause: crate::vfs::SourceUriPathUnavailable::WithdrawnMapping,
            ..
        })
    ));
}

#[tokio::test]
async fn original_closed_source_mapping_cannot_borrow_equal_bytes_from_a_new_alias_allocation() {
    let mut backend = crate::tests::test_backend();
    let store = Arc::new(crate::vfs::MemoryStore::new());
    let uri: Uri = "file:///workspace-symbol-original-alias.tcl"
        .parse()
        .unwrap();
    let alias = "file:///workspace-symbol-service-alias.tcl";
    let path = "/same/workspace-symbol-original-alias.tcl";
    store.upsert_uri_aliases(&[uri.as_str(), alias], path, SOURCE.as_bytes().to_vec());
    backend.state_mut_for_test().store = store.clone();
    let original = tcl_compiler::analyser::Analyser::new().analyse(SOURCE, "tcl8.6");
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &original);
    assert_alias_has_no_open_rows(&backend, &uri).await;
    let symbols = actual_symbols(&backend).await;
    let snapshot = backend.read_document(&uri).await.unwrap().unwrap();
    let retained = snapshot.path_admission.as_ref().unwrap().clone();
    let revision = backend
        .workspace_index
        .read()
        .await
        .document_revision(uri.as_str());
    let inputs_epoch = backend.diag_inputs_epoch();
    let factory_generation = *backend.class_factory_generation.read().await;
    let rows = symbols
        .iter()
        .cloned()
        .map(|symbol| (symbol, uri.clone(), snapshot.clone(), revision))
        .collect::<Vec<_>>();
    let selected = backend
        .capture_workspace_symbol_analysis(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    let analyses = HashMap::from([(uri.as_str().to_owned(), Some(selected))]);
    assert!(
        backend
            .finish_workspace_symbol_response(&analyses, &rows, inputs_epoch, factory_generation)
            .await
            .unwrap()
            .is_some()
    );

    assert!(store.remove_uri(alias));
    store.upsert_uri_aliases(&[uri.as_str(), alias], path, SOURCE.as_bytes().to_vec());
    let selected = backend
        .capture_workspace_symbol_analysis(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    assert_alias_admission(
        &selected,
        retained.as_ref(),
        &snapshot,
        &uri,
        store.as_ref(),
    );
    assert_alias_generations(&backend, &uri, revision, inputs_epoch, factory_generation).await;
    let analyses = HashMap::from([(uri.as_str().to_owned(), Some(selected))]);
    assert_alias_response_withdrawn(&backend, &analyses, &rows, inputs_epoch, factory_generation)
        .await;

    // A new genuine source read may use the replacement allocation. The
    // refusal above is currency of the old source issuer, not path absence.
    let current = backend.read_document(&uri).await.unwrap().unwrap();
    assert_eq!(current.text, snapshot.text);
    assert_eq!(current.revision, snapshot.revision);
    assert_eq!(current.version, snapshot.version);
    let selected = backend
        .capture_workspace_symbol_analysis(&uri, &current)
        .await
        .unwrap()
        .unwrap();
    let analyses = HashMap::from([(uri.as_str().to_owned(), Some(selected))]);
    let current_rows = symbols
        .into_iter()
        .map(|symbol| (symbol, uri.clone(), current.clone(), revision))
        .collect::<Vec<_>>();
    assert!(
        backend
            .finish_workspace_symbol_response(
                &analyses,
                &current_rows,
                inputs_epoch,
                factory_generation
            )
            .await
            .unwrap()
            .is_some()
    );
}

async fn saved_symbol_source_fixture() -> (Backend, Arc<crate::vfs::MemoryStore>, Uri, &'static str)
{
    let mut backend = crate::tests::test_backend();
    let store = Arc::new(crate::vfs::MemoryStore::new());
    let uri: Uri = "file:///workspace-symbol-saved-image.tcl".parse().unwrap();
    let path = "/workspace-symbol-saved-image.tcl";
    store.upsert_uri(uri.as_str(), path, SOURCE.as_bytes().to_vec());
    backend.state_mut_for_test().store = store.clone();
    let original = tcl_compiler::analyser::Analyser::new().analyse(SOURCE, "tcl8.6");
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &original);
    assert_alias_has_no_open_rows(&backend, &uri).await;
    (backend, store, uri, path)
}

#[tokio::test]
async fn closed_symbol_saved_bytes_cannot_borrow_a_same_binding_upsert() {
    let (backend, store, uri, path) = saved_symbol_source_fixture().await;
    let symbols = actual_symbols(&backend).await;
    let snapshot = backend
        .closed_workspace_symbol_source(&uri)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        snapshot.saved_source_bytes.as_deref(),
        Some(SOURCE.as_bytes())
    );
    let (_, report) = tcl_lsp_core::source_decode::decode_source(SOURCE.as_bytes());
    assert_eq!(snapshot.decode_report, Some(report));
    let admission = snapshot.path_admission.as_ref().unwrap().clone();
    let revision = backend
        .workspace_index
        .read()
        .await
        .document_revision(uri.as_str());
    let inputs_epoch = backend.diag_inputs_epoch();
    let factory_generation = *backend.class_factory_generation.read().await;
    let selected = backend
        .capture_workspace_symbol_analysis(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    let analyses = HashMap::from([(uri.as_str().to_owned(), Some(selected))]);
    let rows = symbols
        .iter()
        .cloned()
        .map(|symbol| (symbol, uri.clone(), snapshot.clone(), revision))
        .collect::<Vec<_>>();
    assert!(
        backend
            .finish_workspace_symbol_response(&analyses, &rows, inputs_epoch, factory_generation)
            .await
            .unwrap()
            .is_some()
    );

    let changed = SOURCE.replace('\n', "\r");
    store.upsert_uri(uri.as_str(), path, changed.as_bytes().to_vec());
    admission.check_for(&uri, store.as_ref()).unwrap();
    assert_alias_generations(&backend, &uri, revision, inputs_epoch, factory_generation).await;
    assert!(matches!(
        backend
            .finish_workspace_symbol_response(&analyses, &rows, inputs_epoch, factory_generation)
            .await,
        Err(super::super::DocumentSourceUnavailable::Store {
            cause: crate::vfs::SourceUriPathUnavailable::SourceImageUnavailable,
            ..
        })
    ));

    let current = backend
        .closed_workspace_symbol_source(&uri)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.path_admission.as_ref(), Some(&admission));
    assert_eq!(current.text, snapshot.text);
    assert_ne!(current.raw(), snapshot.raw());
    assert_eq!(
        current.saved_source_bytes.as_deref(),
        Some(changed.as_bytes())
    );
    let selected = backend
        .capture_workspace_symbol_analysis(&uri, &current)
        .await
        .unwrap()
        .unwrap();
    let analyses = HashMap::from([(uri.as_str().to_owned(), Some(selected))]);
    let rows = symbols
        .into_iter()
        .map(|symbol| (symbol, uri.clone(), current.clone(), revision))
        .collect::<Vec<_>>();
    assert!(
        backend
            .finish_workspace_symbol_response(&analyses, &rows, inputs_epoch, factory_generation)
            .await
            .unwrap()
            .is_some()
    );
}

struct MixedSavedRows {
    backend: Backend,
    store: Arc<crate::vfs::MemoryStore>,
    first: Uri,
    current: Uri,
    analyses: HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
    rows: Vec<WorkspaceSymbolRow>,
    inputs_epoch: u64,
    factory_generation: u64,
}

impl MixedSavedRows {
    async fn new() -> Self {
        let mut backend = crate::tests::test_backend();
        let store = Arc::new(crate::vfs::MemoryStore::new());
        let first: Uri = "file:///workspace-symbol-first-saved.tcl".parse().unwrap();
        let current: Uri = "file:///workspace-symbol-current-saved.tcl"
            .parse()
            .unwrap();
        backend.state_mut_for_test().store = store.clone();
        for (uri, path, source) in [
            (
                &first,
                "/workspace-symbol-first-saved.tcl",
                "proc first_symbol {} {}\n",
            ),
            (
                &current,
                "/workspace-symbol-current-saved.tcl",
                "proc current_symbol {} {}\n",
            ),
        ] {
            store.upsert_uri(uri.as_str(), path, source.as_bytes().to_vec());
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
            backend
                .workspace_index
                .write()
                .await
                .add_document(uri.as_str(), &analysis);
        }
        let symbols = actual_symbols(&backend).await;
        let inputs_epoch = backend.diag_inputs_epoch();
        let factory_generation = *backend.class_factory_generation.read().await;
        let mut analyses = HashMap::new();
        let mut snapshots = HashMap::new();
        for uri in [&first, &current] {
            let snapshot = backend
                .closed_workspace_symbol_source(uri)
                .await
                .unwrap()
                .unwrap();
            let selected = backend
                .capture_workspace_symbol_analysis(uri, &snapshot)
                .await
                .unwrap()
                .unwrap();
            analyses.insert(uri.as_str().to_owned(), Some(selected));
            snapshots.insert(uri.clone(), snapshot);
        }
        let mut rows = Vec::new();
        for symbol in symbols {
            let uri = symbol.location.uri.clone();
            let revision = backend
                .workspace_index
                .read()
                .await
                .document_revision(uri.as_str());
            rows.push((
                symbol,
                uri.clone(),
                snapshots.remove(&uri).unwrap(),
                revision,
            ));
        }
        let fixture = Self {
            backend,
            store,
            first,
            current,
            analyses,
            rows,
            inputs_epoch,
            factory_generation,
        };
        let Some(WorkspaceSymbolResponse::Flat(rows)) = fixture.response().await.unwrap() else {
            panic!("both genuine captured saved rows are initially available")
        };
        assert_eq!(rows.len(), 2);
        fixture
    }

    async fn response(&self) -> DocumentSourceResult<Option<WorkspaceSymbolResponse>> {
        self.backend
            .finish_workspace_symbol_response(
                &self.analyses,
                &self.rows,
                self.inputs_epoch,
                self.factory_generation,
            )
            .await
    }

    fn change_first_saved_bytes(&self) {
        self.store.upsert_uri(
            self.first.as_str(),
            "/workspace-symbol-first-saved.tcl",
            b"proc first_symbol {} {}\r".to_vec(),
        );
        let snapshot = self
            .rows
            .iter()
            .find(|(_, uri, _, _)| uri == &self.first)
            .unwrap();
        snapshot
            .2
            .path_admission
            .as_ref()
            .unwrap()
            .check_for(&self.first, self.store.as_ref())
            .unwrap();
        assert_eq!(snapshot.2.text.as_ref(), "proc first_symbol {} {}\n");
    }
}

#[tokio::test]
async fn closed_symbol_withdrawn_index_row_does_not_lease_its_changed_saved_bytes() {
    let fixture = MixedSavedRows::new().await;
    fixture.change_first_saved_bytes();
    fixture
        .backend
        .workspace_index
        .write()
        .await
        .remove_document(fixture.first.as_str());
    let Some(WorkspaceSymbolResponse::Flat(rows)) = fixture.response().await.unwrap() else {
        panic!("the independently current saved row remains available")
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].location.uri, fixture.current);
}

#[tokio::test]
async fn closed_symbol_selected_byte_change_refuses_the_joint_saved_answer() {
    let fixture = MixedSavedRows::new().await;
    fixture.change_first_saved_bytes();
    for (_, uri, _, revision) in &fixture.rows {
        assert_eq!(
            fixture
                .backend
                .workspace_index
                .read()
                .await
                .document_revision(uri.as_str()),
            *revision
        );
    }
    assert_eq!(fixture.backend.diag_inputs_epoch(), fixture.inputs_epoch);
    assert_eq!(
        *fixture.backend.class_factory_generation.read().await,
        fixture.factory_generation
    );
    assert!(matches!(
        fixture.response().await,
        Err(super::super::DocumentSourceUnavailable::Store {
            cause: crate::vfs::SourceUriPathUnavailable::SourceImageUnavailable,
            ..
        })
    ));
}

#[tokio::test]
async fn host_reported_file_requires_its_original_source_read_admission() {
    let mut backend = crate::tests::test_backend();
    let store = Arc::new(crate::vfs::MemoryStore::new());
    let uri: Uri = "file:///workspace-symbol-missing-original-admission.tcl"
        .parse()
        .unwrap();
    store.upsert_uri(
        uri.as_str(),
        "/actual/workspace-symbol-missing-original-admission.tcl",
        SOURCE.as_bytes().to_vec(),
    );
    backend.state_mut_for_test().store = store;
    let original = tcl_compiler::analyser::Analyser::new().analyse(SOURCE, "tcl8.6");
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &original);
    let symbols = actual_symbols(&backend).await;
    let without_receipt = super::super::DocumentState::new(SOURCE.to_owned(), "tcl8.6".to_owned());
    assert!(without_receipt.path_admission.is_none());
    // A genuine independently current analysis is not the missing source
    // read receipt. It must not repair that publication boundary.
    let selected = backend
        .capture_workspace_symbol_analysis(&uri, &without_receipt)
        .await
        .unwrap()
        .unwrap();
    let analyses = HashMap::from([(uri.as_str().to_owned(), Some(selected))]);
    let revision = backend
        .workspace_index
        .read()
        .await
        .document_revision(uri.as_str());
    let rows = symbols
        .into_iter()
        .map(|symbol| (symbol, uri.clone(), without_receipt.clone(), revision))
        .collect::<Vec<_>>();
    let inputs_epoch = backend.diag_inputs_epoch();
    let factory_generation = *backend.class_factory_generation.read().await;
    assert!(
        backend
            .finish_workspace_symbol_response(&analyses, &rows, inputs_epoch, factory_generation,)
            .await
            .unwrap()
            .is_none()
    );
}
