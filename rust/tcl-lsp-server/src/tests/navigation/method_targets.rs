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

//! Object-method queries.

use super::*;

async fn assert_current_original_method_owner(
    backend: &Backend,
    provider: &Uri,
    consumer: &Uri,
    source: &str,
    analysis: &AnalysisResult,
    position: Position,
) -> DocumentSourceResult<()> {
    let candidate = backend
        .original_method_candidate_at(consumer, source, analysis, position)
        .await?;
    let std::ops::ControlFlow::Break(Some(candidate)) = candidate else {
        panic!("current original source candidate: {candidate:?}");
    };
    let document = backend.read_document(provider).await?.unwrap();
    assert_eq!(
        candidate.declaration_image(),
        &tcl_lexer::SourceImage::document(&document.text)
    );
    let owner = backend
        .analysis_for(provider, document.text, document.dialect)
        .await?;
    assert!(
        owner.matches_original_source_image(
            candidate.declaration_image(),
            candidate.declaration_config()
        ),
        "current original method owner: selected={:?}, owner={:?}, retained={:?}",
        candidate.declaration_config(),
        owner.body_lexer_config,
        owner.resolved_input,
    );

    Ok(())
}

#[tokio::test]
async fn original_method_queries_keep_opaque_routes_owners_and_terminal_ambiguity() {
    // Implementation contract: naming.server.original-method-query
    // docs/design/analysis/name-resolution-proofs/server-original-method-query.md
    let backend = test_backend();
    let provider = Uri::from_str("file:///opaque-method-provider.tcl").unwrap();
    let consumer = Uri::from_str("file:///opaque-method-consumer.tcl").unwrap();
    let provider_source =
        "oo::class create C\\uD800 {self method m\\uD800 {} {}; self method m\\uD801 {} {}}\n";
    let consumer_source = "C\\uD800 m\\uD800\n";
    register(&backend, &provider, provider_source).await;
    register(&backend, &consumer, consumer_source).await;
    let mut provider_analysis = Analyser::new().analyse(provider_source, "tcl8.6");
    provider_analysis.all_classes.clear();
    provider_analysis.superseded_classes.clear();
    let consumer_analysis = Analyser::new().analyse(consumer_source, "tcl8.6");
    {
        let mut index = backend.workspace_index.write().await;
        index.add_document(provider.as_str(), &provider_analysis);
        index.add_document(consumer.as_str(), &consumer_analysis);
    }
    let position = Position::new(0, 11);
    Box::pin(assert_current_original_method_owner(
        &backend,
        &provider,
        &consumer,
        consumer_source,
        &consumer_analysis,
        position,
    ))
    .await
    .unwrap();
    let std::ops::ControlFlow::Break(definitions) = Box::pin(backend.original_method_locations_at(
        &consumer,
        consumer_source,
        &consumer_analysis,
        position,
        tcl_lsp_core::method_symbol::OriginalMethodNavigation::Declaration,
    ))
    .await
    .unwrap() else {
        panic!("original method provider");
    };
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].uri, provider);
    let std::ops::ControlFlow::Break(references) = Box::pin(backend.original_method_locations_at(
        &consumer,
        consumer_source,
        &consumer_analysis,
        position,
        tcl_lsp_core::method_symbol::OriginalMethodNavigation::References {
            include_declaration: false,
        },
    ))
    .await
    .unwrap() else {
        panic!("original references provider");
    };
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].uri, consumer);
    let std::ops::ControlFlow::Break(Some(hover)) = Box::pin(backend.original_method_hover_at(
        &consumer,
        consumer_source,
        &consumer_analysis,
        position,
    ))
    .await
    .unwrap() else {
        panic!("original method hover");
    };
    assert!(hover.value.contains("Possible source declaration"));
    assert_eq!(
        Box::pin(backend.compute_definition(&consumer, position))
            .await
            .unwrap(),
        definitions
    );
    assert_terminal_original_method_ambiguity(
        &backend,
        &consumer,
        consumer_source,
        &consumer_analysis,
        position,
        provider_source,
        &provider_analysis,
    )
    .await;
}

async fn assert_terminal_original_method_ambiguity(
    backend: &Backend,
    consumer: &Uri,
    consumer_source: &str,
    consumer_analysis: &AnalysisResult,
    position: Position,
    provider_source: &str,
    provider_analysis: &AnalysisResult,
) {
    let duplicate = Uri::from_str("file:///opaque-method-duplicate.tcl").unwrap();
    register(backend, &duplicate, provider_source).await;
    backend
        .workspace_index
        .write()
        .await
        .add_document(duplicate.as_str(), provider_analysis);
    assert!(matches!(
        backend
            .original_method_candidate_at(consumer, consumer_source, consumer_analysis, position,)
            .await
            .unwrap(),
        std::ops::ControlFlow::Break(None)
    ));
    assert_eq!(
        Box::pin(backend.compute_definition(consumer, position))
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Location; 0]
    );
    assert!(matches!(
        Box::pin(backend.original_method_hover_at(
            consumer,
            &format!("#{consumer_source}"),
            consumer_analysis,
            position,
        ))
        .await
        .unwrap(),
        std::ops::ControlFlow::Break(None)
    ));
}

#[tokio::test]
async fn original_method_queries_preserve_lexical_variable_provider_priority() {
    // Implementation contract: naming.server.original-method-query
    // docs/design/analysis/name-resolution-proofs/server-original-method-query.md
    let backend = test_backend();
    let provider = Uri::from_str("file:///variable-priority-method-provider.tcl").unwrap();
    let provider_source = "oo::class create C {self method m {} {}}\n";
    register(&backend, &provider, provider_source).await;
    let provider_analysis = Analyser::new().analyse(provider_source, "tcl8.6");
    backend
        .workspace_index
        .write()
        .await
        .add_document(provider.as_str(), &provider_analysis);
    for (uri_text, source, position, declaration_line) in [
        (
            "file:///variable-priority-unknown.tcl",
            "namespace eval N {}\nset ::N::v VALUE\nunknown $::N::v\n",
            Position::new(2, 13),
            1,
        ),
        (
            "file:///variable-priority-external-class.tcl",
            "set method m\nC $method\n",
            Position::new(1, 4),
            0,
        ),
    ] {
        let uri = Uri::from_str(uri_text).unwrap();
        register(&backend, &uri, source).await;
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(matches!(
            tcl_lsp_core::variable_symbol::select(
                source,
                &analysis,
                position.line,
                position.character
            ),
            std::ops::ControlFlow::Break(Some(_))
        ));
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &analysis);
        assert!(matches!(
            backend
                .original_method_candidate_at(&uri, source, &analysis, position)
                .await
                .unwrap(),
            std::ops::ControlFlow::Continue(())
        ));
        let locations = Box::pin(backend.compute_definition(&uri, position))
            .await
            .unwrap();
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].uri, uri);
        assert_eq!(locations[0].range.start.line, declaration_line);
        assert!(
            Box::pin(backend.original_method_hover_at(&uri, source, &analysis, position))
                .await
                .unwrap()
                .is_continue()
        );
    }
}
