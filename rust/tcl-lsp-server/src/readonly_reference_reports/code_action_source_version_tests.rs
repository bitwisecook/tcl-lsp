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
use tower_lsp_server::ls_types::{
    CodeActionContext, CodeActionOrCommand, CodeActionParams, DocumentChanges,
    TextDocumentIdentifier,
};

const SOURCE: &str = "unset xs\n";

async fn open(backend: &Backend, uri: &Uri, source: &str) -> DocumentState {
    let document = DocumentState::with_version(source.to_owned(), "tcl8.6".to_owned(), 23);
    backend
        .db_set_source(uri, source, document.dialect.clone())
        .await
        .expect("publish the fixture through the backend source API");
    backend
        .documents
        .lock("source action fixture")
        .await
        .insert(uri.clone(), document.clone());
    document
}

fn assert_original_fold_matches_project_evidence(
    owner: &OriginalCodeActionSource,
    source: &str,
    evidence: Option<&tcl_compiler::unit_scope::CallSiteEvidence>,
) {
    let WorkspaceSymbolAnalysisKind::Database(document) = &owner.analysis.kind else {
        panic!("the fixture must exercise the actual database-backed readonly capture");
    };
    assert_eq!(document.external_call_sites.as_deref(), evidence);
    let analysis = owner.analysis();
    let registry = analysis.resolved_registry().unwrap();
    assert!(
        analysis
            .original_optimiser_source_unit(source, registry, evidence)
            .is_some()
    );
    let checks = tcl_lsp_db::compiler_check_diagnostics_uncached_from_analysis(
        source, registry, &analysis, None, evidence,
    );
    let rewrite = checks
        .optimisations
        .into_iter()
        .find(|row| row.code == tcl_core_types::DiagCode::O101)
        .expect("the actual source worker must issue the constant fold");
    assert_eq!(rewrite.replacement, "set x 3");
    assert!(rewrite.original_rewrite_batch().is_some());
    let finding = tcl_lsp_core::diagnostic_policy::Finding::from(rewrite);
    let current =
        tcl_lsp_core::code_actions::DiagnosticEditSource::for_analysis(source, &analysis).unwrap();
    assert!(current.matches_optimiser_rewrite_group(&[&finding]));
}

#[tokio::test]
async fn readonly_action_producer_retains_exact_optional_project_evidence() {
    use salsa::Setter as _;
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///source-action-project-evidence.tcl"
        .parse()
        .unwrap();
    let source = "set x [expr {1 + 2}]\n";
    let document = open(&backend, &uri, source).await;
    let mut nonempty = tcl_compiler::unit_scope::CallSiteEvidence::default();
    nonempty.record_opaque_caller("::outside");
    assert!(!nonempty.is_empty());
    let versions = [
        None,
        Some(Arc::new(
            tcl_compiler::unit_scope::CallSiteEvidence::default(),
        )),
        Some(Arc::new(nonempty)),
    ];
    for (index, evidence) in versions.iter().enumerate() {
        let file = *backend.db_files.lock().await.get(&uri).unwrap();
        file.set_external_call_sites(&mut *backend.db.lock().await)
            .to(evidence.clone());
        let owner = backend
            .capture_code_action_source(&uri, &document)
            .await
            .unwrap()
            .unwrap();
        assert_original_fold_matches_project_evidence(&owner, source, evidence.as_deref());
        assert!(
            backend
                .finish_code_action_response(&owner, Vec::new(), None)
                .await
                .unwrap()
                .is_some()
        );
        let original_input = owner.analysis().resolved_input.clone().unwrap();
        let next = versions[(index + 1) % versions.len()].clone();
        assert_ne!(evidence, &next);
        file.set_external_call_sites(&mut *backend.db.lock().await)
            .to(next);
        let config = backend.resolved_db_config(&uri).await;
        let input =
            tcl_lsp_db::document_analysis_input(&*backend.db.lock().await, file, config).unwrap();
        assert_eq!(input.as_ref(), &original_input);
        assert_eq!(
            backend
                .documents
                .lock("unchanged action evidence source")
                .await[&uri]
                .text
                .as_ref(),
            source
        );
        assert!(
            backend
                .finish_code_action_response(&owner, Vec::new(), None)
                .await
                .unwrap()
                .is_none()
        );
    }
}

#[tokio::test]
async fn actual_code_action_endpoint_publishes_the_original_document_version() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///source-action-version.tcl".parse().unwrap();
    open(&backend, &uri, SOURCE).await;
    let actions = backend
        .code_action(CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range: Range::new(Position::new(0, 0), Position::new(0, 8)),
            context: CodeActionContext::default(),
            work_done_progress_params: tower_lsp_server::ls_types::WorkDoneProgressParams::default(
            ),
            partial_result_params: tower_lsp_server::ls_types::PartialResultParams::default(),
        })
        .await
        .unwrap()
        .expect("actual source action");
    let action = actions
        .iter()
        .find_map(|item| match item {
            CodeActionOrCommand::CodeAction(action)
                if action.title == "Add '-nocomplain' to unset" =>
            {
                Some(action)
            }
            _ => None,
        })
        .expect("actual Core unset subject");
    let edit = action.edit.as_ref().unwrap();
    assert!(edit.changes.is_none());
    let Some(DocumentChanges::Edits(documents)) = &edit.document_changes else {
        panic!("expected a versioned edit");
    };
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].text_document.uri, uri);
    assert_eq!(documents[0].text_document.version, Some(23));
    assert_ne!(documents[0].edits.len(), 0);
    assert_eq!(
        backend.documents.lock("unchanged source action").await[&uri]
            .text
            .as_ref(),
        SOURCE
    );
}

#[tokio::test]
async fn equal_source_does_not_replace_the_original_version_input_or_export_generation() {
    use salsa::Setter as _;
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///source-action-currentness.tcl".parse().unwrap();
    let document = open(&backend, &uri, SOURCE).await;
    let owner = backend
        .capture_code_action_source(&uri, &document)
        .await
        .unwrap()
        .unwrap();
    assert!(
        backend
            .finish_code_action_response(&owner, Vec::new(), None)
            .await
            .unwrap()
            .is_some()
    );
    backend
        .documents
        .lock("version change")
        .await
        .get_mut(&uri)
        .unwrap()
        .version = Some(24);
    assert!(
        backend
            .finish_code_action_response(&owner, Vec::new(), None)
            .await
            .unwrap()
            .is_none()
    );
    let current = backend.documents.lock("current version").await[&uri].clone();
    let owner = backend
        .capture_code_action_source(&uri, &current)
        .await
        .unwrap()
        .unwrap();
    let config = backend.resolved_db_config(&uri).await;
    config
        .set_extra_commands(&mut *backend.db.lock().await)
        .to(vec!["new_action_input".to_owned()]);
    assert!(
        backend
            .finish_code_action_response(&owner, Vec::new(), None)
            .await
            .unwrap()
            .is_none()
    );
    let owner = backend
        .capture_code_action_source(&uri, &current)
        .await
        .unwrap()
        .unwrap();
    assert!(
        backend
            .finish_code_action_response(&owner, Vec::new(), None)
            .await
            .unwrap()
            .is_some()
    );
    let external = tcl_compiler::analyser::Analyser::new().analyse("proc external {} {}", "tcl8.6");
    backend
        .workspace_index
        .write()
        .await
        .add_document("file:///action-external.tcl", &external);
    assert!(
        backend
            .finish_code_action_response(&owner, Vec::new(), None)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn missing_versions_and_clamped_utf16_positions_cannot_issue_source_actions() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "untitled:source-action-unicode".parse().unwrap();
    let document = open(&backend, &uri, "set x 😀\r\n").await;
    let owner = backend
        .capture_code_action_source(&uri, &document)
        .await
        .unwrap()
        .unwrap();
    assert!(owner.contains_range(Range::new(Position::new(0, 6), Position::new(0, 8))));
    assert!(!owner.contains_range(Range::new(Position::new(0, 7), Position::new(0, 8))));
    assert!(!owner.contains_range(Range::new(Position::new(0, 0), Position::new(0, 99))));
    assert!(!owner.contains_range(Range::new(Position::new(9, 0), Position::new(9, 0))));
    let mut unversioned = document;
    unversioned.version = None;
    assert!(
        backend
            .capture_code_action_source(&uri, &unversioned)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn equal_provider_replacement_cannot_renew_the_action_source_receipt() {
    let mut backend = crate::tests::test_backend();
    let store = Arc::new(crate::vfs::MemoryStore::new());
    let uri: Uri = "file:///source-action-provider.tcl".parse().unwrap();
    store.upsert_uri_aliases(
        &[uri.as_str()],
        "/source-action-provider.tcl",
        SOURCE.as_bytes().to_vec(),
    );
    backend.state_mut_for_test().store = store.clone();
    let mut document = open(&backend, &uri, SOURCE).await;
    document.path_admission = Some(backend.document_path_admission(&uri).await.unwrap());
    backend
        .documents
        .lock("actual source provider receipt")
        .await
        .insert(uri.clone(), document.clone());
    let owner = backend
        .capture_code_action_source(&uri, &document)
        .await
        .unwrap()
        .unwrap();
    assert!(
        backend
            .finish_code_action_response(&owner, Vec::new(), None)
            .await
            .unwrap()
            .is_some()
    );
    assert!(store.remove_uri(uri.as_str()));
    store.upsert_uri_aliases(
        &[uri.as_str()],
        "/source-action-provider.tcl",
        SOURCE.as_bytes().to_vec(),
    );
    assert!(
        backend
            .finish_code_action_response(&owner, Vec::new(), None)
            .await
            .is_err()
    );
}
