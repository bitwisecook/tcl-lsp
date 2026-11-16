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
use tower_lsp_server::ls_types::{TextDocumentIdentifier, WorkDoneProgressParams};
#[tokio::test]
async fn conditional_rename_requires_client_confirmation_and_preserves_version() {
    let backend = super::tests::test_backend();
    let uri: Uri = "file:///conditional-variable.tcl".parse().unwrap();
    let source = "set {x} 1\nputs ${x}\n";
    let admission = Arc::new(DocumentPathAdmission::capture(&uri, backend.store.as_ref()).unwrap());
    backend.documents.lock("conditional_test").await.insert(
        uri.clone(),
        DocumentState::with_version(source.to_owned(), "tcl".to_owned(), 7)
            .with_path_admission(Arc::clone(&admission)),
    );
    let position = TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        position: Position::new(1, 7),
    };
    let params = RenameParams {
        text_document_position: position.clone(),
        new_name: "new".to_owned(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    assert!(
        backend
            .prepare_rename(position.clone())
            .await
            .unwrap()
            .is_none()
    );
    let error = backend.rename(params.clone()).await.unwrap_err();
    assert_eq!(error.code, jsonrpc::ErrorCode::InvalidRequest);
    backend
        .conditional_variable_rename_supported
        .store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(backend.prepare_rename(position).await.unwrap().is_some());
    let edit = backend
        .rename(params)
        .await
        .unwrap()
        .expect("conditional edit");
    let json = serde_json::to_value(edit).unwrap();
    assert_eq!(json["documentChanges"][0]["textDocument"]["version"], 7);
    assert_eq!(
        json["changeAnnotations"]["conditional-variable-source-alpha"]["needsConfirmation"],
        true
    );
}
#[tokio::test]
async fn conditional_publication_rechecks_document_and_input_ownership() {
    let backend = super::tests::test_backend();
    let uri: Uri = "file:///conditional-currency.tcl".parse().unwrap();
    let source = "set {x} 1\nputs ${x}\n";
    let admission = Arc::new(DocumentPathAdmission::capture(&uri, backend.store.as_ref()).unwrap());
    backend.documents.lock("conditional_test").await.insert(
        uri.clone(),
        DocumentState::with_version(source.to_owned(), "tcl".to_owned(), 7)
            .with_path_admission(Arc::clone(&admission)),
    );
    let uri_generation = backend.live_publication_uri_generation(&uri);
    let doc = backend.read_document(&uri).await.unwrap().unwrap();
    let seed = backend
        .fresh_analysis_seed(&uri, doc.text.clone(), doc.dialect.clone())
        .await
        .unwrap();
    let inputs = backend
        .file_rename_input_snapshot()
        .await
        .expect("published source input");
    assert_eq!(
        backend
            .publish_conditional_if_current(&uri, &doc, &seed, &inputs, uri_generation, 7)
            .await,
        Some(7)
    );
    backend.documents.lock("conditional_test").await.insert(
        uri.clone(),
        DocumentState::with_version(source.to_owned() + " ", "tcl".to_owned(), 8)
            .with_path_admission(Arc::clone(&admission)),
    );
    assert_eq!(
        backend
            .publish_conditional_if_current(&uri, &doc, &seed, &inputs, uri_generation, 7)
            .await,
        None
    );
    backend.documents.lock("conditional_test").await.insert(
        uri.clone(),
        DocumentState::with_version(source.to_owned(), "tcl".to_owned(), 7)
            .with_path_admission(Arc::clone(&admission)),
    );
    backend.invalidate_live_publication([&uri]);
    assert_eq!(
        backend
            .publish_conditional_if_current(&uri, &doc, &seed, &inputs, uri_generation, 7)
            .await,
        None
    );
    let reopened_generation = backend.live_publication_uri_generation(&uri);
    assert_eq!(
        backend
            .publish_conditional_if_current(&uri, &doc, &seed, &inputs, reopened_generation, 7)
            .await,
        Some(7)
    );
    backend.invalidate_diag_inputs();
    assert_eq!(
        backend
            .publish_conditional_if_current(&uri, &doc, &seed, &inputs, reopened_generation, 7)
            .await,
        None
    );
}
#[tokio::test]
async fn conditional_publication_checks_raw_source_not_only_normalised_text() {
    let backend = super::tests::test_backend();
    let uri: Uri = "file:///conditional-line-endings.tcl".parse().unwrap();
    let raw = "set {x} 1\rputs ${x}\r";
    let admission = Arc::new(DocumentPathAdmission::capture(&uri, backend.store.as_ref()).unwrap());
    backend.documents.lock("conditional_test").await.insert(
        uri.clone(),
        DocumentState::with_version(raw.to_owned(), "tcl".to_owned(), 7)
            .with_path_admission(Arc::clone(&admission)),
    );
    let generation = backend.live_publication_uri_generation(&uri);
    let doc = backend.read_document(&uri).await.unwrap().unwrap();
    let seed = backend
        .fresh_analysis_seed(&uri, doc.text.clone(), doc.dialect.clone())
        .await
        .unwrap();
    let inputs = backend
        .file_rename_input_snapshot()
        .await
        .expect("published source input");
    backend.documents.lock("conditional_test").await.insert(
        uri.clone(),
        DocumentState::with_version("set {x} 1\nputs ${x}\n".to_owned(), "tcl".to_owned(), 7)
            .with_path_admission(admission),
    );
    assert_eq!(
        backend
            .publish_conditional_if_current(&uri, &doc, &seed, &inputs, generation, 7)
            .await,
        None
    );
}
#[tokio::test]
async fn conditional_publication_refuses_input_or_sidecar_change_while_queued() {
    for change_sidecar in [false, true] {
        let backend = super::tests::test_backend();
        let uri: Uri = "file:///conditional-queued.tcl".parse().unwrap();
        let source = "set {x} 1\nputs ${x}\n";
        let admission =
            Arc::new(DocumentPathAdmission::capture(&uri, backend.store.as_ref()).unwrap());
        backend.documents.lock("conditional_test").await.insert(
            uri.clone(),
            DocumentState::with_version(source.to_owned(), "tcl".to_owned(), 7)
                .with_path_admission(admission),
        );
        let uri_generation = backend.live_publication_uri_generation(&uri);
        let doc = backend.read_document(&uri).await.unwrap().unwrap();
        let seed = backend
            .fresh_analysis_seed(&uri, doc.text.clone(), doc.dialect.clone())
            .await
            .unwrap();
        let inputs = backend
            .file_rename_input_snapshot()
            .await
            .expect("published source input");
        let gate = backend.analyser_inputs_gate.write().await;
        let pending =
            backend.publish_conditional_if_current(&uri, &doc, &seed, &inputs, uri_generation, 7);
        tokio::pin!(pending);
        tokio::select! { biased; result = &mut pending => panic!("unexpected publication: {result:?}"), () = crate::rt::yield_now() => {} }
        if change_sidecar {
            backend
                .fresh_analysis_cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clear();
        } else {
            backend.invalidate_diag_inputs();
        }
        drop(gate);
        assert_eq!(pending.await, None);
    }
}
#[test]
fn annotations_require_all_three_client_capabilities() {
    let mut params = InitializeParams::default();
    assert!(!client_supports_conditional_rename(&params));
    params.capabilities = serde_json::from_value(serde_json::json!({
        "workspace": {"workspaceEdit": {"documentChanges": true, "changeAnnotationSupport": {}}},
        "textDocument": {"rename": {"honorsChangeAnnotations": true}}
    }))
    .unwrap();
    assert!(client_supports_conditional_rename(&params));
    params
        .capabilities
        .text_document
        .as_mut()
        .unwrap()
        .rename
        .as_mut()
        .unwrap()
        .honors_change_annotations = Some(false);
    assert!(!client_supports_conditional_rename(&params));
}
#[test]
fn conditional_preview_coalesces_all_changes_and_preserves_original_bytes() {
    let source = "set x 1\r\nputs $x\r\n";
    let index = tcl_lexer::LineIndex::new_lsp(source);
    let edits = vec![
        core_rename::TextEdit {
            range: core_definition::LspRange {
                start_line: 0,
                start_character: 4,
                end_line: 0,
                end_character: 5,
            },
            new_text: "new".to_owned(),
        },
        core_rename::TextEdit {
            range: core_definition::LspRange {
                start_line: 1,
                start_character: 6,
                end_line: 1,
                end_character: 7,
            },
            new_text: "new".to_owned(),
        },
    ];
    let combined = coalesce_conditional_variable_edits(
        source,
        source,
        "set new 1\r\nputs $new\r\n",
        &index,
        &edits,
    )
    .unwrap();
    assert_eq!(combined.new_text, "new 1\r\nputs $new");
    assert_eq!(
        combined.range,
        Range::new(Position::new(0, 4), Position::new(1, 7))
    );
    assert!(
        coalesce_conditional_variable_edits(
            source,
            source,
            "set new 1\r\nputs $new\r\n",
            &index,
            &[edits[0].clone(), edits[0].clone()]
        )
        .is_none()
    );
}

#[test]
fn conditional_preview_preserves_unicode_coordinates() {
    let unicode = "😀x\n$x";
    let index = tcl_lexer::LineIndex::new_lsp(unicode);
    let edits = vec![
        core_rename::TextEdit {
            range: core_definition::LspRange {
                start_line: 0,
                start_character: 2,
                end_line: 0,
                end_character: 3,
            },
            new_text: "new".to_owned(),
        },
        core_rename::TextEdit {
            range: core_definition::LspRange {
                start_line: 1,
                start_character: 1,
                end_line: 1,
                end_character: 2,
            },
            new_text: "new".to_owned(),
        },
    ];
    assert_eq!(
        coalesce_conditional_variable_edits(unicode, unicode, "😀new\n$new", &index, &edits)
            .unwrap()
            .new_text,
        "new\n$new"
    );
}

#[test]
fn conditional_preview_preserves_bom_and_lone_cr() {
    let raw = "\u{feff}set x 1\rputs $x\r";
    let normalised = "\u{feff}set x 1\nputs $x\n";
    let index = tcl_lexer::LineIndex::new_lsp(raw);
    let edits = vec![
        core_rename::TextEdit {
            range: core_definition::LspRange {
                start_line: 0,
                start_character: 5,
                end_line: 0,
                end_character: 6,
            },
            new_text: "new".to_owned(),
        },
        core_rename::TextEdit {
            range: core_definition::LspRange {
                start_line: 1,
                start_character: 6,
                end_line: 1,
                end_character: 7,
            },
            new_text: "new".to_owned(),
        },
    ];
    let combined = coalesce_conditional_variable_edits(
        raw,
        normalised,
        "\u{feff}set new 1\nputs $new\n",
        &index,
        &edits,
    )
    .unwrap();
    assert_eq!(combined.new_text, "new 1\rputs $new");
    assert!(coalesce_conditional_variable_edits(raw, "stale", "stale", &index, &edits).is_none());
}
#[test]
fn conditional_edit_is_versioned_and_requires_confirmation() {
    let uri: Uri = "file:///tmp/conditional.tcl".parse().unwrap();
    let edit = annotated_conditional_variable_edit(
        uri,
        7,
        vec![TextEdit::new(Range::default(), "new".to_owned())],
    )
    .unwrap();
    let json = serde_json::to_value(edit).unwrap();
    assert_eq!(json["documentChanges"][0]["textDocument"]["version"], 7);
    assert_eq!(
        json["documentChanges"][0]["edits"][0]["annotationId"],
        "conditional-variable-source-alpha"
    );
    assert_eq!(
        json["changeAnnotations"]["conditional-variable-source-alpha"]["needsConfirmation"],
        true
    );
}
