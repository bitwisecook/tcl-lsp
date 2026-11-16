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

async fn register(backend: &Backend, uri: &Uri, source: &str) {
    backend.documents.lock("logical lens test").await.insert(
        uri.clone(),
        DocumentState::new(source.to_owned(), "tcl".to_owned()),
    );
    backend
        .db_set_source(uri, source, "tcl".to_owned())
        .await
        .unwrap();
    let analysis = backend
        .analysis_for(uri, Arc::from(source), "tcl".to_owned())
        .await
        .unwrap();
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &analysis);
}

async fn helper_lens(backend: &Backend, uri: &Uri) -> tower_lsp_server::ls_types::CodeLens {
    backend
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
        .find(|lens| lens.range.start.line == 0)
        .unwrap()
}

#[tokio::test]
async fn logical_lens_reuses_opaque_source_selection_not_client_range_or_label() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/logical-lens-wire.tcl".parse().unwrap();
    register(
        &backend,
        &uri,
        "proc helper {} {}\nproc caller {} {helper}\n",
    )
    .await;
    let mut lens = helper_lens(&backend, &uri).await;
    let original = lens.range;
    lens.range = Range::new(Position::new(99, 99), Position::new(99, 100));
    lens.data.as_mut().unwrap()["qname"] = serde_json::json!("::caller");
    let resolved = backend.code_lens_resolve(lens).await.unwrap();
    assert_eq!(resolved.range, original);
    let command = resolved.command.unwrap();
    assert_eq!(command.title, "at least 1 reference");
    assert_eq!(command.command, "tcl-lsp.showReferences");
    assert_eq!(resolved.data.unwrap()["sourceReferenceCoverage"], "partial");
}

#[tokio::test]
async fn logical_lens_old_publication_and_legacy_or_unknown_handles_stay_inert() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/logical-lens-stale.tcl".parse().unwrap();
    register(
        &backend,
        &uri,
        "proc helper {} {}\nproc caller {} {helper}\n",
    )
    .await;
    let lens = helper_lens(&backend, &uri).await;
    let mut legacy = lens.clone();
    legacy.data = Some(serde_json::json!({"qname": "::helper", "uri": uri.as_str()}));
    assert!(
        backend
            .code_lens_resolve(legacy)
            .await
            .unwrap()
            .command
            .is_none()
    );
    let mut unknown = lens.clone();
    unknown.data.as_mut().unwrap()["logicalSourceLens"] =
        serde_json::json!({"schema": 4, "token": 18_446_744_073_709_551_615_u64});
    assert!(
        backend
            .code_lens_resolve(unknown)
            .await
            .unwrap()
            .command
            .is_none()
    );
    register(&backend, &uri, "proc other {} {}\nproc caller {} {other}\n").await;
    assert!(
        backend
            .code_lens_resolve(lens)
            .await
            .unwrap()
            .command
            .is_none()
    );
}
#[tokio::test]
async fn logical_lens_equal_counter_from_another_backend_cannot_select_this_session() {
    let first = crate::tests::test_backend();
    let second = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/logical-lens-session.tcl".parse().unwrap();
    let source = "proc helper {} {}\nproc caller {} {helper}\n";
    register(&first, &uri, source).await;
    register(&second, &uri, source).await;
    let first_lens = helper_lens(&first, &uri).await;
    let second_lens = helper_lens(&second, &uri).await;
    let first_handle = &first_lens.data.as_ref().unwrap()["logicalSourceLens"];
    let second_handle = &second_lens.data.as_ref().unwrap()["logicalSourceLens"];
    assert_eq!(first_handle["schema"], second_handle["schema"]);
    assert_eq!(first_handle["token"], second_handle["token"]);
    assert_ne!(first_handle["session"], second_handle["session"]);
    assert!(
        first
            .code_lens_resolve(first_lens.clone())
            .await
            .unwrap()
            .command
            .is_some()
    );
    assert!(
        second
            .code_lens_resolve(second_lens)
            .await
            .unwrap()
            .command
            .is_some()
    );
    assert!(
        second
            .code_lens_resolve(first_lens)
            .await
            .unwrap()
            .command
            .is_none()
    );
}
