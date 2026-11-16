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

//! Declaration targets revalidated against their current documents.

use super::*;

#[tokio::test]
async fn original_indexed_variable_and_namespace_queries_withdraw_stale_foreign_owners() {
    // Implementation contract: naming.editor.original-indexed-source-location
    // docs/design/analysis/name-resolution-proofs/original-indexed-source-location.md
    let backend = test_backend();
    let first = Uri::from_str("file:///current-indexed-first.tcl").unwrap();
    let second = Uri::from_str("file:///current-indexed-second.tcl").unwrap();
    let source = r"namespace eval n\uD800 {}
namespace exists n\uD800
namespace eval N {}
set ::N::v\uD800 1
info exists ::N::v\uD800";
    register(&backend, &first, source).await;
    register(&backend, &second, source).await;
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let symbol = Backend::original_namespace_symbol(source, &analysis, Position::new(0, 16))
        .expect("actual namespace operand");
    let position = Position::new(4, 14);
    assert_eq!(
        backend
            .original_namespace_locations(&first, source, &analysis, &symbol, true, true,)
            .await
            .unwrap()
            .len(),
        2,
        "equal source bytes retain separate URI owners"
    );
    assert_eq!(
        backend
            .cross_document_variable_definition(&first, source, position, &analysis,)
            .await
            .unwrap()
            .len(),
        1
    );
    let retained_context =
        core_workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(&analysis).unwrap();
    let retained_row = analysis
        .original_variable_symbols
        .iter()
        .find(|row| row.is_declaration())
        .unwrap();
    let retained = OriginalIndexedSourceLocation::from_variable(
        second.as_str(),
        &retained_context,
        retained_row,
    )
    .unwrap();
    assert_eq!(
        backend
            .resolve_original_indexed_locations(vec![retained.clone()])
            .await
            .unwrap()
            .len(),
        1
    );

    // Preserve stale index rows while changing equal-length source names.
    backend.documents.lock("test").await.insert(
        second.clone(),
        DocumentState::new(source.replace("D800", "D802"), "tcl8.6".to_owned()),
    );
    let (_declarations, _references, _hover) = assert_stale_original_indexed_source_is_withdrawn(
        &backend, &first, source, &analysis, &symbol, position, &retained,
    )
    .await;

    // Equal source geometry does not make another selected dialect current.
    backend.documents.lock("test").await.insert(
        second.clone(),
        DocumentState::new(source.to_owned(), "tcl9.0".to_owned()),
    );
    assert_eq!(
        backend
            .resolve_original_indexed_locations(vec![retained])
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Location; 0]
    );
    assert_eq!(
        backend
            .cross_document_variable_definition(&first, source, position, &analysis,)
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Location; 0]
    );
    assert_eq!(
        backend
            .original_namespace_locations(&first, source, &analysis, &symbol, true, true,)
            .await
            .unwrap()
            .len(),
        1
    );
}

async fn assert_stale_original_indexed_source_is_withdrawn(
    backend: &Backend,
    first: &Uri,
    source: &str,
    analysis: &AnalysisResult,
    symbol: &core_namespace_symbol::OriginalNamespaceSymbol,
    position: Position,
    retained: &OriginalIndexedSourceLocation,
) -> (Vec<Location>, Vec<Location>, CoreHover) {
    let declarations = backend
        .original_namespace_locations(first, source, analysis, symbol, true, true)
        .await
        .unwrap();
    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].uri, *first);
    let references = backend
        .cross_document_variable_references(source, analysis, position, false)
        .await
        .unwrap();
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].uri, *first);
    assert_eq!(
        backend
            .cross_document_variable_definition(first, source, position, analysis,)
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Location; 0]
    );
    assert!(
        backend
            .cross_document_variable_hover(first, source, position, analysis,)
            .await
            .unwrap()
            .is_none()
    );
    let hover = backend
        .original_namespace_hover(first, analysis, symbol)
        .await
        .unwrap()
        .unwrap();
    assert!(hover.value.contains("1 `namespace eval` block"));
    assert!(hover.value.contains("1 other reference(s)"));
    assert_eq!(
        backend
            .resolve_original_indexed_locations(vec![retained.clone()])
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Location; 0]
    );

    (declarations, references, hover)
}

/// `resolve_target_locations` reads each unique target
/// URI once rather than once per site. Correctness check: several sites
/// interleaved across two documents must all resolve, each against the
/// *right* document's source, and the result must keep the exact input
/// order (some callers, e.g. the code-lens click target, rely on
/// `resolve_target_locations`'s order matching `textDocument/references`'
/// own).
#[tokio::test]
async fn resolve_target_locations_groups_by_uri_and_preserves_site_order() {
    let backend = test_backend();
    let a = Uri::from_str("file:///a.tcl").unwrap();
    let b = Uri::from_str("file:///b.tcl").unwrap();
    // a.tcl: "aaa\nbbb\nccc\n" — three one-word lines.
    // b.tcl: "xxx\nyyy\n" — two one-word lines.
    register(&backend, &a, "aaa\nbbb\nccc\n").await;
    register(&backend, &b, "xxx\nyyy\n").await;
    let span_at = |line_start_byte: u32, len: u32| {
        tcl_lexer::Span::new(line_start_byte, line_start_byte + len)
    };
    // Interleaved: b, a, b, a, a — deliberately not grouped by URI, and
    // not in either document's own line order, so a naive "sort by URI"
    // implementation would be caught by the order assertion below.
    let targets = vec![
        (b.to_string(), span_at(4, 3)), // b.tcl line 1: "yyy"
        (a.to_string(), span_at(8, 3)), // a.tcl line 2: "ccc"
        (b.to_string(), span_at(0, 3)), // b.tcl line 0: "xxx"
        (a.to_string(), span_at(0, 3)), // a.tcl line 0: "aaa"
        (a.to_string(), span_at(4, 3)), // a.tcl line 1: "bbb"
    ];
    let locations = backend.resolve_target_locations(targets).await.unwrap();
    assert_eq!(locations.len(), 5, "{locations:?}");
    let got: Vec<(String, u32)> = locations
        .iter()
        .map(|l| (l.uri.to_string(), l.range.start.line))
        .collect();
    assert_eq!(
        got,
        vec![
            (b.to_string(), 1),
            (a.to_string(), 2),
            (b.to_string(), 0),
            (a.to_string(), 0),
            (a.to_string(), 1),
        ],
        "grouping by URI must not reorder the result",
    );
}

/// A target URI that fails to parse, or has no document, is dropped
/// without disturbing the other targets' resolution or order.
#[tokio::test]
async fn resolve_target_locations_drops_unresolvable_targets_and_keeps_the_rest() {
    let backend = test_backend();
    let a = Uri::from_str("file:///a.tcl").unwrap();
    register(&backend, &a, "aaa\nbbb\n").await;
    let targets = vec![
        ("not a uri".to_owned(), tcl_lexer::Span::new(0, 3)),
        (a.to_string(), tcl_lexer::Span::new(0, 3)),
        (
            "file:///never-opened-and-not-on-disk.tcl".to_owned(),
            tcl_lexer::Span::new(0, 3),
        ),
        (a.to_string(), tcl_lexer::Span::new(4, 7)),
    ];
    let locations = backend.resolve_target_locations(targets).await.unwrap();
    let got: Vec<u32> = locations.iter().map(|l| l.range.start.line).collect();
    assert_eq!(got, vec![0, 1], "{locations:?}");
}
