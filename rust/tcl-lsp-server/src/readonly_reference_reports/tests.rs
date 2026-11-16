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
use crate::DocumentState;
use tower_lsp_server::LanguageServer;
use tower_lsp_server::ls_types::{CodeLensParams, TextDocumentIdentifier};

const SOURCE: &str = "oo::class create C {method ping {} {}; method body {} {my ping; my ping}}";

async fn register_current(backend: &Backend, uri: &Uri, source: &str) {
    backend
        .documents
        .lock("readonly reference test")
        .await
        .insert(
            uri.clone(),
            DocumentState::new(source.to_owned(), "tcl8.6".to_owned()),
        );
    backend
        .db_set_source(uri, source, "tcl8.6".to_owned())
        .await
        .unwrap();
    let analysis = backend
        .analysis_for(uri, Arc::from(source), "tcl8.6".to_owned())
        .await
        .unwrap();
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &analysis);
}

async fn ping_identity(backend: &Backend, uri: &Uri) -> OriginalDeclarationIdentity {
    let document = backend.read_document(uri).await.unwrap().unwrap();
    let analysis = backend
        .analysis_for(uri, document.text.clone(), document.dialect)
        .await
        .unwrap();
    let class = analysis.original_class_declarations().next().unwrap();
    let method = class
        .metadata()
        .original_members
        .declarations()
        .find(|method| method.original_name_input().bytes() == b"ping")
        .unwrap();
    OriginalDeclarationIdentity::for_method(uri.as_str(), &document.text, &analysis, class, method)
        .unwrap()
}

async fn snapshot_still_current(
    backend: &Backend,
    retained: &OriginalReadonlyReferenceDocument,
) -> bool {
    let generation = *backend.class_factory_generation.read().await;
    backend
        .finish_readonly_reference_report(
            std::slice::from_ref(retained),
            Vec::new(),
            ReadonlyReferenceCoverage::Partial,
            backend.diag_inputs_epoch(),
            generation,
        )
        .await
        .unwrap()
        .published()
        .is_some()
}

#[tokio::test]
async fn readonly_member_wire_retains_owner_and_partial_coverage() {
    // naming.server.original-reference-and-lens-source-selection
    // docs/design/analysis/name-resolution-proofs/server-original-reference-and-lens-source-selection.md
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///readonly-member-wire.tcl".parse().unwrap();
    register_current(&backend, &uri, SOURCE).await;
    let identity = ping_identity(&backend, &uri).await;
    let report = backend
        .readonly_original_declaration_reference_report(&identity, &uri, false)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        report.coverage,
        ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus
    );
    assert_eq!(report.locations.len(), 2);
    assert!(report.locations.iter().all(|location| location.uri == uri));
    assert_eq!(
        backend
            .original_declaration_reference_locations(&identity, &uri, false)
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Location; 0]
    );
    let lenses = backend
        .code_lens(CodeLensParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: tower_lsp_server::ls_types::WorkDoneProgressParams::default(
            ),
            partial_result_params: tower_lsp_server::ls_types::PartialResultParams::default(),
        })
        .await
        .unwrap()
        .unwrap();
    let mut lens = lenses
        .into_iter()
        .find(|lens| {
            lens.range.start.character == u32::try_from(SOURCE.find("ping").unwrap()).unwrap()
        })
        .unwrap();
    assert_eq!(
        lens.data.as_ref().unwrap()["sourceReferenceCoverage"],
        "complete-source-census"
    );
    let data = lens.data.as_mut().unwrap();
    data["sourceReferenceCoverage"] = serde_json::json!("existing-inventory");
    data["qname"] = serde_json::json!("::unrelated");
    let lens = backend.code_lens_resolve(lens).await.unwrap();
    assert_eq!(
        lens.data.as_ref().unwrap()["sourceReferenceCoverage"],
        "complete-source-census"
    );
    let command = lens.command.unwrap();
    assert_eq!(command.title, "2 references");
    assert_eq!(command.command, "tcl-lsp.showReferences");
    let locations: Vec<Location> =
        serde_json::from_value(command.arguments.unwrap()[2].clone()).unwrap();
    assert_eq!(locations, report.locations);
}

#[tokio::test]
async fn readonly_member_census_stays_partial_beside_other_documents() {
    // A complete census of one document cannot close references that
    // another retained document may hold.
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///readonly-member-census.tcl".parse().unwrap();
    let other: Uri = "file:///readonly-member-other.tcl".parse().unwrap();
    register_current(&backend, &uri, SOURCE).await;
    register_current(&backend, &other, "proc other {} {}").await;
    let identity = ping_identity(&backend, &uri).await;
    let report = backend
        .readonly_original_declaration_reference_report(&identity, &uri, false)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(report.coverage, ReadonlyReferenceCoverage::Partial);
    assert_eq!(report.locations.len(), 2);
    let lenses = backend
        .code_lens(CodeLensParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: tower_lsp_server::ls_types::WorkDoneProgressParams::default(
            ),
            partial_result_params: tower_lsp_server::ls_types::PartialResultParams::default(),
        })
        .await
        .unwrap()
        .unwrap();
    let lens = lenses
        .into_iter()
        .find(|lens| {
            lens.range.start.character == u32::try_from(SOURCE.find("ping").unwrap()).unwrap()
        })
        .unwrap();
    assert_eq!(
        lens.data.as_ref().unwrap()["sourceReferenceCoverage"],
        "partial"
    );
    let lens = backend.code_lens_resolve(lens).await.unwrap();
    assert_eq!(
        lens.data.as_ref().unwrap()["sourceReferenceCoverage"],
        "partial"
    );
}

#[tokio::test]
async fn readonly_member_wire_withdraws_missing_or_replaced_source_owners() {
    // naming.server.original-reference-and-lens-source-selection
    // docs/design/analysis/name-resolution-proofs/server-original-reference-and-lens-source-selection.md
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///readonly-member-current.tcl".parse().unwrap();
    let foreign: Uri = "file:///readonly-member-foreign.tcl".parse().unwrap();
    register_current(&backend, &uri, SOURCE).await;
    let identity = ping_identity(&backend, &uri).await;
    let document = backend.read_document(&uri).await.unwrap().unwrap();
    let retained = backend
        .capture_readonly_reference_document(uri.as_str(), document.text.clone())
        .await
        .unwrap()
        .unwrap();
    assert!(snapshot_still_current(&backend, &retained).await);
    register_current(&backend, &foreign, SOURCE).await;
    let foreign_identity = ping_identity(&backend, &foreign).await;
    assert_ne!(identity, foreign_identity);
    assert!(
        backend
            .readonly_original_declaration_reference_report(&identity, &uri, false)
            .await
            .unwrap()
            .is_none()
    );
    backend.db_files.lock().await.remove(&uri);
    assert!(!snapshot_still_current(&backend, &retained).await);
    assert!(
        backend
            .readonly_original_declaration_reference_report(&identity, &uri, false)
            .await
            .unwrap()
            .is_none()
    );
    register_current(&backend, &uri, "oo::class create C {method changed {} {}}").await;
    assert!(!snapshot_still_current(&backend, &retained).await);
    assert!(
        backend
            .readonly_original_declaration_reference_report(&identity, &uri, false)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn readonly_member_wire_rejects_equal_path_provider_republication() {
    // naming.server.original-reference-and-lens-source-selection
    // docs/design/analysis/name-resolution-proofs/server-original-reference-and-lens-source-selection.md
    let mut backend = crate::tests::test_backend();
    let store = Arc::new(crate::vfs::MemoryStore::new());
    let uri: Uri = "file:///readonly-provider-aba.tcl".parse().unwrap();
    let path = "/same/readonly-provider-aba.tcl";
    store.upsert_uri(uri.as_str(), path, SOURCE.as_bytes().to_vec());
    backend.state_mut_for_test().store = store.clone();
    register_current(&backend, &uri, SOURCE).await;
    let retained = backend
        .capture_readonly_reference_document(uri.as_str(), Arc::from(SOURCE))
        .await
        .unwrap()
        .unwrap();
    assert!(snapshot_still_current(&backend, &retained).await);
    store.upsert_uri(
        uri.as_str(),
        "/other/readonly-provider-aba.tcl",
        SOURCE.as_bytes().to_vec(),
    );
    store.upsert_uri(uri.as_str(), path, SOURCE.as_bytes().to_vec());
    register_current(&backend, &uri, SOURCE).await;
    let current = backend
        .capture_readonly_reference_document(uri.as_str(), Arc::from(SOURCE))
        .await
        .unwrap()
        .unwrap();
    assert!(current.file == retained.file);
    assert_eq!(current.input, retained.input);
    assert_ne!(current.admission.as_ref(), retained.admission.as_ref());
    assert!(!snapshot_still_current(&backend, &retained).await);
    assert!(snapshot_still_current(&backend, &current).await);
    backend.invalidate_live_publication(std::iter::once(&uri));
    assert!(!snapshot_still_current(&backend, &current).await);
    let current = backend
        .capture_readonly_reference_document(uri.as_str(), Arc::from(SOURCE))
        .await
        .unwrap()
        .unwrap();
    assert!(snapshot_still_current(&backend, &current).await);
    backend
        .documents
        .lock("readonly_same_bytes_revision")
        .await
        .get_mut(&uri)
        .unwrap()
        .revision += 1;
    assert!(!snapshot_still_current(&backend, &current).await);
}

#[test]
fn unavailable_member_wire_has_no_clickable_empty_census() {
    // naming.server.original-reference-and-lens-source-selection
    // docs/design/analysis/name-resolution-proofs/server-original-reference-and-lens-source-selection.md
    for coverage in [
        ReadonlyReferenceCoverage::Partial,
        ReadonlyReferenceCoverage::Unavailable,
    ] {
        let report = OriginalReadonlyReferenceReport {
            locations: Vec::new(),
            coverage,
        };
        let command = report.lens_command(
            &"file:///readonly-empty.tcl".parse().unwrap(),
            Position::new(0, 0),
        );
        assert_eq!(command.title, "References unavailable");
        assert_eq!(command.command, "");
        assert!(command.arguments.is_none());
    }
    for coverage in [
        ReadonlyReferenceCoverage::ExistingInventory,
        ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus,
    ] {
        let report = OriginalReadonlyReferenceReport {
            locations: Vec::new(),
            coverage,
        };
        let command = report.lens_command(
            &"file:///readonly-empty.tcl".parse().unwrap(),
            Position::new(0, 0),
        );
        assert_eq!(command.title, "0 references");
        assert_eq!(command.command, "tcl-lsp.showReferences");
    }
}

#[tokio::test]
async fn unrelated_input_writes_recapture_then_withhold_the_census() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///readonly-unrelated-write.tcl".parse().unwrap();
    register_current(&backend, &uri, SOURCE).await;
    let identity = ping_identity(&backend, &uri).await;
    UNRELATED_WRITES_BEFORE_FINAL.set(1);
    let report = backend
        .readonly_original_declaration_reference_report(&identity, &uri, false)
        .await
        .unwrap()
        .expect("one unrelated write is recaptured");
    assert_eq!(UNRELATED_WRITES_BEFORE_FINAL.get(), 0);
    assert_eq!(
        report.coverage,
        ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus
    );
    assert_eq!(report.locations.len(), 2);
    UNRELATED_WRITES_BEFORE_FINAL.set(READONLY_CURRENCY_ATTEMPTS);
    assert!(
        backend
            .readonly_original_declaration_reference_report(&identity, &uri, false)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(UNRELATED_WRITES_BEFORE_FINAL.get(), 0);
    let lens = backend
        .code_lens(CodeLensParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: tower_lsp_server::ls_types::WorkDoneProgressParams::default(
            ),
            partial_result_params: tower_lsp_server::ls_types::PartialResultParams::default(),
        })
        .await
        .unwrap()
        .unwrap()
        .into_iter()
        .find(|lens| {
            lens.range.start.character == u32::try_from(SOURCE.find("ping").unwrap()).unwrap()
        })
        .unwrap();
    UNRELATED_WRITES_BEFORE_FINAL.set(READONLY_CURRENCY_ATTEMPTS);
    let lens = backend.code_lens_resolve(lens).await.unwrap();
    UNRELATED_WRITES_BEFORE_FINAL.set(0);
    assert_eq!(
        lens.data.as_ref().unwrap()["sourceReferenceCoverage"],
        "unavailable"
    );
    let command = lens.command.unwrap();
    assert_eq!(command.title, "References unavailable");
    assert_eq!(command.command, "");
    assert!(command.arguments.is_none());
}
