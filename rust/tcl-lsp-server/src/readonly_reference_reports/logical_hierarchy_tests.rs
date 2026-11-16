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

async fn register(backend: &Backend, uri: &Uri, source: &str) {
    backend
        .documents
        .lock("logical hierarchy test")
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
        .add_document(uri.as_str(), &analysis);
}

#[tokio::test]
async fn logical_hierarchy_wire_uses_issued_class_and_real_cross_document_owners() {
    let backend = crate::tests::test_backend();
    let base: Uri = "file:///tmp/logical-hierarchy-base.tcl".parse().unwrap();
    let child: Uri = "file:///tmp/logical-hierarchy-child.tcl".parse().unwrap();
    let base_source = "namespace eval N {oo::class create Base {}}";
    let child_source = "namespace eval N {oo::class create Child {superclass Base}}";
    register(&backend, &base, base_source).await;
    register(&backend, &child, child_source).await;
    let offset = u32::try_from(child_source.find("Child").unwrap()).unwrap();
    let mut items = backend
        .readonly_logical_hierarchy_prepare(&child, Position::new(0, offset))
        .await
        .unwrap();
    assert_eq!(items.len(), 1);
    let mut item = items.remove(0);
    assert!(item.data.is_some());
    item.name = "untrusted label".into();
    item.selection_range = Range::default();
    let parents = backend
        .type_hierarchy_walk(item.clone(), false)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(parents.len(), 1);
    assert_eq!(parents[0].name, "::N::Base");
    assert_eq!(parents[0].uri, base);
    assert!(parents[0].data.is_some());
    let children = backend
        .type_hierarchy_walk(parents[0].clone(), true)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].uri, child);
    let mut unissued = item.clone();
    unissued.data = None;
    assert_eq!(
        backend
            .type_hierarchy_walk(unissued, false)
            .await
            .unwrap()
            .unwrap(),
        [] as [tower_lsp_server::ls_types::TypeHierarchyItem; 0]
    );
    let mut foreign = item.clone();
    foreign.uri = base.clone();
    assert_eq!(
        backend
            .type_hierarchy_walk(foreign, false)
            .await
            .unwrap()
            .unwrap(),
        [] as [tower_lsp_server::ls_types::TypeHierarchyItem; 0]
    );
    register(&backend, &child, &format!("{child_source}\n# changed")).await;
    assert_eq!(
        backend
            .type_hierarchy_walk(item, false)
            .await
            .unwrap()
            .unwrap(),
        [] as [tower_lsp_server::ls_types::TypeHierarchyItem; 0]
    );
}

#[tokio::test]
async fn logical_hierarchy_final_snapshot_rejects_changed_relation_parent_with_current_target() {
    let backend = crate::tests::test_backend();
    let base: Uri = "file:///tmp/logical-hierarchy-final-base.tcl"
        .parse()
        .unwrap();
    let child: Uri = "file:///tmp/logical-hierarchy-final-child.tcl"
        .parse()
        .unwrap();
    let base_source = "oo::class create Base {}";
    let child_source = "oo::class create Child {superclass Base}";
    register(&backend, &base, base_source).await;
    register(&backend, &child, child_source).await;
    let epoch = backend.diag_inputs_epoch();
    let generation = *backend.class_factory_generation.read().await;
    let documents = vec![
        backend
            .capture_readonly_reference_document(base.as_str(), Arc::from(base_source))
            .await
            .unwrap()
            .unwrap(),
        backend
            .capture_readonly_reference_document(child.as_str(), Arc::from(child_source))
            .await
            .unwrap()
            .unwrap(),
    ];
    let position = u32::try_from(child_source.find("Child").unwrap()).unwrap();
    let root = tcl_lsp_core::type_hierarchy::prepare_for_document(
        child.as_str(),
        child_source,
        0,
        position,
        &documents[1].analysis,
    )
    .remove(0)
    .logical_declaration
    .unwrap();
    let inventory = documents
        .iter()
        .map(
            |document| tcl_lsp_core::type_hierarchy::OriginalHierarchyDocument {
                uri: document.uri.as_str(),
                source: &document.source,
                analysis: &document.analysis,
            },
        )
        .collect::<Vec<_>>();
    let items =
        tcl_lsp_core::type_hierarchy::related_logical_from_inventory(&root, &inventory, false)
            .unwrap()
            .into_items();
    assert_eq!(items.len(), 1);
    register(&backend, &child, "oo::class create Child {}").await;
    assert_eq!(
        backend
            .finish_logical_hierarchy_items(&documents, &items, epoch, generation)
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::TypeHierarchyItem; 0]
    );
}
