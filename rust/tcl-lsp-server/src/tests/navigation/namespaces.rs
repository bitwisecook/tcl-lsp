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

//! Namespace queries across documents.

use super::*;

#[tokio::test]
async fn original_namespace_queries_keep_opaque_slots_across_documents() {
    let backend = test_backend();
    let first = Uri::from_str("file:///opaque-first.tcl").unwrap();
    let second = Uri::from_str("file:///opaque-second.tcl").unwrap();
    let source = r"namespace eval n\uD800 {}
namespace eval n\uD801 {}
namespace exists n\uD800";
    register(&backend, &first, source).await;
    register(&backend, &second, source).await;
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let symbol = Backend::original_namespace_symbol(source, &analysis, Position::new(0, 16))
        .expect("genuine original namespace operand");
    for reference in &mut analysis.namespace_refs {
        reference.qualified_name.clear();
        reference.original_name.clear();
    }
    backend
        .workspace_index
        .write()
        .await
        .add_document(first.as_str(), &analysis);
    backend
        .workspace_index
        .write()
        .await
        .add_document(second.as_str(), &analysis);
    let declarations = backend
        .original_namespace_locations(&first, source, &analysis, &symbol, true, true)
        .await
        .unwrap();
    assert_eq!(declarations.len(), 2);
    assert!(
        declarations
            .iter()
            .all(|location| location.range.start.line == 0)
    );
    let references = backend
        .original_namespace_locations(&first, source, &analysis, &symbol, false, false)
        .await
        .unwrap();
    assert_eq!(references.len(), 2);
    assert!(
        references
            .iter()
            .all(|location| location.range.start.line == 2)
    );
    let hover = backend
        .original_namespace_hover(&first, &analysis, &symbol)
        .await
        .unwrap()
        .expect("namespace address renders through its original policy");
    assert!(hover.value.contains(
        "Declared by 2 `namespace eval` blocks across 2 documents; 2 other reference(s)"
    ));
    assert!(
        Backend::original_namespace_symbol(
            &source.replace("exists", "delete"),
            &analysis,
            Position::new(0, 16)
        )
        .is_none(),
        "stale source cannot issue a cursor selection"
    );
}
