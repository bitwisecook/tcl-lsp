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
        .lock("conditional_navigation_test")
        .await
        .insert(
            uri.clone(),
            DocumentState::new(source.to_owned(), "tcl".to_owned()),
        );
    backend
        .db_set_source(uri, source, "tcl".to_owned())
        .await
        .unwrap();
}

#[tokio::test]
async fn typed_hierarchy_debt_survives_wire_publication_and_parent_currentness() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/conditional-hierarchy-debt.tcl"
        .parse()
        .unwrap();
    let source = "oo::class create Base {}; mystery; oo::class create Child {superclass Base}";
    register(&backend, &uri, source).await;
    let offset = u32::try_from(source.rfind("Base").unwrap()).unwrap();
    let items = backend
        .readonly_logical_hierarchy_prepare(&uri, Position::new(0, offset))
        .await
        .unwrap();
    assert_eq!(items.len(), 1);
    let document = backend
        .capture_readonly_reference_document(uri.as_str(), Arc::from(source))
        .await
        .unwrap()
        .unwrap();
    let handle =
        serde_json::from_value(items[0].data.as_ref().unwrap()["logicalHierarchy"].clone())
            .unwrap();
    let identity = backend
        .original_declaration_registry
        .lock()
        .await
        .resolve_logical_class(handle, uri.as_str(), &document.source, &document.analysis)
        .unwrap();
    assert!(identity.has_unresolved_applicability());
    let advice = tcl_lsp_core::type_hierarchy::prepare_logical_advice_for_document(
        uri.as_str(),
        source,
        0,
        offset,
        &document.analysis,
    )
    .unwrap();
    assert!(advice.has_unresolved_applicability());
    let epoch = backend.diag_inputs_epoch();
    let generation = *backend.class_factory_generation.read().await;
    register(
        &backend,
        &uri,
        "oo::class create Base {}; oo::class create Child {}",
    )
    .await;
    assert_eq!(
        backend
            .finish_logical_hierarchy_advice(&[document], &advice, epoch, generation)
            .await
            .unwrap()
            .published()
            .unwrap_or_default(),
        [] as [tower_lsp_server::ls_types::TypeHierarchyItem; 0]
    );
}

#[tokio::test]
async fn implementation_possible_locations_keep_typed_debt_until_final_source_lease() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/conditional-implementation-debt.tcl"
        .parse()
        .unwrap();
    let source = "oo::class create Base {method run {} {}}\nmystery\noo::class create Child {superclass Base; method run {} {}}\noo::class create Other {superclass $missing}";
    register(&backend, &uri, source).await;
    let offset = u32::try_from(source.find("run").unwrap()).unwrap();
    let locations = backend
        .readonly_logical_implementation(&uri, Position::new(0, offset))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        locations
            .iter()
            .map(|location| location.range.start.line)
            .collect::<Vec<_>>(),
        vec![0, 2]
    );
    let document = backend
        .capture_readonly_reference_document(uri.as_str(), Arc::from(source))
        .await
        .unwrap()
        .unwrap();
    let advice = tcl_lsp_core::implementation::logical_implementation_advice(
        uri.as_str(),
        source,
        0,
        offset,
        &document.analysis,
    )
    .unwrap();
    assert!(advice.has_unresolved_applicability());
    let epoch = backend.diag_inputs_epoch();
    let generation = *backend.class_factory_generation.read().await;
    register(&backend, &uri, &format!("{source}\n# changed")).await;
    assert!(
        backend
            .finish_logical_implementation_advice(&[document], &advice, epoch, generation)
            .await
            .unwrap()
            .published()
            .is_none()
    );
}
