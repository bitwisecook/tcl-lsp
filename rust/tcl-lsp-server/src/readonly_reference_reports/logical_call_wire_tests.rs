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

async fn register(backend: &Backend, uri: &Uri, source: &str) {
    backend
        .documents
        .lock("logical call wire test")
        .await
        .insert(
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
        .replace_document(uri.as_str(), &analysis);
}
async fn prepare(
    backend: &Backend,
    uri: &Uri,
    source: &str,
    name: &str,
) -> tower_lsp_server::ls_types::CallHierarchyItem {
    let at = u32::try_from(source.find(name).unwrap()).unwrap();
    let point = tcl_lexer::LineIndex::new_lsp(source).position_at_utf16(at, source);
    let params = serde_json::from_value(serde_json::json!({"textDocument":{"uri":uri},"position":{"line":point.line,"character":point.character.get()}})).unwrap();
    let mut items = backend
        .prepare_call_hierarchy(params)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(items.len(), 1);
    items.remove(0)
}

#[tokio::test]
async fn actual_call_handlers_use_opaque_current_source_selection_not_client_fields() {
    let backend = crate::tests::test_backend();
    let caller: Uri = "file:///tmp/issued-call-caller.tcl".parse().unwrap();
    let target: Uri = "file:///tmp/issued-call-target.tcl".parse().unwrap();
    let caller_source = "proc caller {} { helper }\n";
    register(&backend, &caller, caller_source).await;
    register(&backend, &target, "proc helper {} {}\n").await;
    let mut item = prepare(&backend, &caller, caller_source, "helper").await;
    assert_eq!(item.uri, target);
    assert!(
        item.data
            .as_ref()
            .unwrap()
            .get("logicalCallHierarchy")
            .is_some()
    );
    item.name = "client label has no identity".to_owned();
    item.detail = Some("untrusted".to_owned());
    item.range = Range::default();
    item.selection_range = Range::default();
    let params = serde_json::from_value(serde_json::json!({"item":item})).unwrap();
    let incoming = Box::pin(backend.incoming_calls(params))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].from.uri, caller);
    assert_eq!(incoming[0].from.name, "::caller");
    let params = serde_json::from_value(serde_json::json!({"item":incoming[0].from})).unwrap();
    let outgoing = Box::pin(backend.outgoing_calls(params))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].to.uri, target);
    assert!(outgoing[0].to.data.is_some());
    let mut unissued = item.clone();
    unissued.data = None;
    assert!(
        backend
            .readonly_logical_call_walk(&unissued, true)
            .await
            .unwrap()
            .is_none()
    );
    let mut wrong = item.clone();
    wrong.uri = caller;
    assert!(
        backend
            .readonly_logical_call_walk(&wrong, true)
            .await
            .unwrap()
            .is_none()
    );
    for schema in [1, 2] {
        let mut wrong = item.clone();
        wrong.data.as_mut().unwrap()["logicalCallHierarchy"]["schema"] = serde_json::json!(schema);
        assert!(
            backend
                .readonly_logical_call_walk(&wrong, true)
                .await
                .unwrap()
                .is_none()
        );
    }
}

#[tokio::test]
async fn equal_bytes_republication_and_reindex_cannot_renew_prepared_call_stamp() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/issued-call-epochs.tcl".parse().unwrap();
    let source = "proc caller {} { helper }\nproc helper {} {}\n";
    register(&backend, &uri, source).await;
    let item = prepare(&backend, &uri, source, "helper").await;
    let analysis = backend
        .analysis_for(&uri, Arc::from(source), "tcl".to_owned())
        .await
        .unwrap();
    backend
        .workspace_index
        .write()
        .await
        .replace_document(uri.as_str(), &analysis);
    assert!(
        backend
            .readonly_logical_call_walk(&item, true)
            .await
            .unwrap()
            .is_none()
    );
    let item = prepare(&backend, &uri, source, "helper").await;
    backend.invalidate_live_publication(std::iter::once(&uri));
    assert!(
        backend
            .readonly_logical_call_walk(&item, true)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn typed_call_debt_and_parent_target_stamps_survive_until_final_source_lease() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/issued-call-final-lease.tcl".parse().unwrap();
    let source = "mystery\nproc caller {} {helper; $head}\nproc helper {} {}\n";
    register(&backend, &uri, source).await;
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
    let inventory = logical_call_inventory(&documents);
    let advice = tcl_lsp_core::call_hierarchy::prepare_logical_from_inventory(
        uri.as_str(),
        1,
        16,
        &inventory,
    )
    .unwrap();
    assert!(advice.has_unresolved_applicability());
    assert_eq!(advice.possible_source_items().len(), 1);
    assert!(
        backend
            .finish_logical_call_advice(&documents, &advice, &stamp)
            .await
            .unwrap()
            .published()
            .is_some()
    );
    register(&backend, &uri, &format!("{source}# changed\n")).await;
    assert!(
        backend
            .finish_logical_call_advice(&documents, &advice, &stamp)
            .await
            .unwrap()
            .published()
            .is_none()
    );
}

#[tokio::test]
async fn missing_actual_index_stamp_is_unavailable_even_with_equal_source_and_analysis() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/issued-call-missing-index.tcl".parse().unwrap();
    let source = "proc helper {} {}\nhelper\n";
    register(&backend, &uri, source).await;
    let item = prepare(&backend, &uri, source, "helper").await;
    backend
        .workspace_index
        .write()
        .await
        .remove_document(uri.as_str());
    assert!(
        backend
            .readonly_logical_call_walk(&item, true)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        backend
            .readonly_logical_call_prepare(&uri, Position::new(0, 5))
            .await
            .unwrap()
            .is_none()
    );
}
