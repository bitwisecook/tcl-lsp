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

#[tokio::test]
async fn newly_indexed_provider_invalidates_old_call_stamp_with_all_retained_rows_unchanged() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/call-provider-generation.tcl".parse().unwrap();
    let source = "proc caller {} {helper}\nproc helper {} {}\n";
    backend
        .documents
        .lock("call provider generation test")
        .await
        .insert(
            uri.clone(),
            DocumentState::new(source.to_owned(), "tcl".to_owned()),
        );
    backend
        .db_set_source(&uri, source, "tcl".to_owned())
        .await
        .unwrap();
    let analysis = backend
        .analysis_for(&uri, Arc::from(source), "tcl".to_owned())
        .await
        .unwrap();
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &analysis);
    let documents = backend.logical_call_documents(&uri).await.unwrap().unwrap();
    let epoch = backend.diag_inputs_epoch();
    let generation = *backend.class_factory_generation.read().await;
    let stamp = LogicalCallPublicationStamp::capture(
        &documents,
        &*backend.workspace_index.read().await,
        epoch,
        generation,
    )
    .unwrap();
    let advice = tcl_lsp_core::call_hierarchy::prepare_logical_from_inventory(
        uri.as_str(),
        0,
        16,
        &logical_call_inventory(&documents),
    )
    .unwrap();
    let projection = backend
        .finish_logical_call_advice(&documents, &advice, &stamp)
        .await
        .unwrap()
        .published()
        .unwrap();
    assert_eq!(projection.items.len(), 1);
    let handle = serde_json::from_value(
        projection.items[0].data.as_ref().unwrap()["logicalCallHierarchy"].clone(),
    )
    .unwrap();
    let extra = tcl_compiler::analyser::Analyser::new().analyse("proc helper {} {}", "tcl");
    backend
        .workspace_index
        .write()
        .await
        .add_document("file:///tmp/new-call-provider.tcl", &extra);
    let current = LogicalCallPublicationStamp::capture(
        &documents,
        &*backend.workspace_index.read().await,
        epoch,
        generation,
    )
    .unwrap();
    // This isolates a provider-roster mutation: old row source, input,
    // admission, DB/publication and indexed revision/context stay exact.
    assert_eq!(stamp.documents, current.documents);
    assert_ne!(stamp.index_generation, current.index_generation);
    let publication =
        tcl_lsp_core::original_declaration::LogicalCallHostPublication::retain(current);
    assert!(
        backend
            .original_declaration_registry
            .lock()
            .await
            .resolve_logical_call(
                handle,
                uri.as_str(),
                &logical_call_inventory(&documents),
                &publication
            )
            .is_none()
    );
    assert!(
        backend
            .finish_logical_call_advice(&documents, &advice, &stamp)
            .await
            .unwrap()
            .published()
            .is_none()
    );
}
