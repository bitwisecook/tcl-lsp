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

//! Retained declaration identities behind implementations, lenses and hierarchies.

use super::*;

#[tokio::test]
async fn original_declaration_inventory_excludes_foreign_buffers_and_keeps_unknown_tcl_terminal() {
    // Implementation contract: naming.editor.original-declaration-document-participation
    // docs/design/analysis/name-resolution-proofs/original-declaration-document-participation.md
    let backend = test_backend();
    let base = Uri::from_str("file:///mixed-base.tcl").unwrap();
    let child = Uri::from_str("file:///mixed-child.tcl").unwrap();
    let javascript = Uri::from_str("file:///mixed.js").unwrap();
    let config = Uri::from_str("file:///bigip.conf").unwrap();
    let apl = Uri::from_str("file:///presentation.apl").unwrap();
    let source = r"oo::class create B\uD800 {method m\uD800 {} {}}";
    register(&backend, &base, source).await;
    register(
        &backend,
        &child,
        r"oo::class create Child {superclass B\uD800; method m\uD800 {} {}}",
    )
    .await;
    {
        let mut documents = backend.documents.lock("test mixed participants").await;
        // Counterfactual Tcl-looking text in a JavaScript buffer cannot
        // create a second declaration provider for the same class.
        documents.insert(
            javascript.clone(),
            DocumentState::new(source.to_owned(), "unknown-js".to_owned())
                .with_language_id("javascript".to_owned()),
        );
        documents.insert(
            config.clone(),
            DocumentState::new("ltm virtual example {}".to_owned(), "f5-bigip".to_owned())
                .with_language_id("f5-bigip".to_owned()),
        );
        documents.insert(
            apl.clone(),
            DocumentState::new("section test {}".to_owned(), "f5-iapps".to_owned())
                .with_language_id("apl".to_owned()),
        );
    }
    let documents = backend.original_declaration_documents(&base).await.unwrap();
    assert_eq!(documents.len(), 2);
    assert!(
        documents
            .iter()
            .all(|(uri, _, _)| uri == base.as_str() || uri == child.as_str())
    );
    let params = serde_json::from_value(serde_json::json!({
            "textDocument": {"uri": base.as_str()}, "position": {"line": 0, "character": source.find(r"m\uD800").unwrap()}
        })).unwrap();
    let Some(GotoImplementationResponse::Array(targets)) =
        Box::pin(backend.goto_implementation(params)).await.unwrap()
    else {
        panic!("foreign buffers must not disable exact Tcl implementation advice");
    };
    assert_eq!(targets.len(), 2);
    assert!(targets.iter().any(|location| location.uri == base));
    assert!(targets.iter().any(|location| location.uri == child));
    let unknown = Uri::from_str("file:///unknown-provider.tcl").unwrap();
    backend
        .documents
        .lock("test relevant unknown")
        .await
        .insert(
            unknown,
            DocumentState::new(source.to_owned(), "unknown-tcl-policy".to_owned())
                .with_language_id("tcl".to_owned()),
        );
    assert_eq!(
        backend.original_declaration_documents(&base).await.unwrap(),
        [] as [(
            std::string::String,
            std::sync::Arc<str>,
            std::sync::Arc<tcl_compiler::analyser::AnalysisResult>
        ); 0]
    );
    for dialect in ["tcl8.6", "tcl9.1", "jimtcl", "f5-irules"] {
        let document = DocumentState::new(source.to_owned(), dialect.to_owned())
            .with_language_id("tcl".to_owned());
        assert_eq!(
            Backend::original_declaration_document_participates(&base, &document),
            Some(true),
            "{dialect}"
        );
    }
}

#[tokio::test]
async fn original_implementation_queries_keep_workspace_method_and_class_owners() {
    // Implementation contract: naming.consumer.original-type-and-implementation-navigation
    // docs/design/analysis/name-resolution-proofs/original-type-and-implementation-navigation.md
    let backend = test_backend();
    let base = Uri::from_str("file:///original-implementation-base.tcl").unwrap();
    let child = Uri::from_str("file:///original-implementation-child.tcl").unwrap();
    let other = Uri::from_str("file:///original-implementation-other.tcl").unwrap();
    let base_source = r"oo::class create B\uD800 {method m\uD800 {} {}}";
    register(&backend, &base, base_source).await;
    register(
        &backend,
        &child,
        r"oo::class create Child {superclass B\uD800; method m\uD800 {} {}}",
    )
    .await;
    register(
        &backend,
        &other,
        r"oo::class create Other {method m\uD800 {} {}}",
    )
    .await;
    let params = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": base.as_str()}, "position": {"line": 0, "character": 18}
    }))
    .unwrap();
    let Some(GotoImplementationResponse::Array(classes)) =
        Box::pin(backend.goto_implementation(params)).await.unwrap()
    else {
        panic!("the exact class declaration should have a workspace subtype");
    };
    assert_eq!(classes.len(), 1);
    assert_eq!(classes[0].uri, child);
    let params = serde_json::from_value(serde_json::json!({
            "textDocument": {"uri": base.as_str()}, "position": {"line": 0, "character": base_source.find(r"m\uD800").unwrap()}
        })).unwrap();
    let Some(GotoImplementationResponse::Array(methods)) =
        Box::pin(backend.goto_implementation(params)).await.unwrap()
    else {
        panic!("the exact method declaration should retain its own and descendant definitions");
    };
    assert_eq!(methods.len(), 2);
    assert!(methods.iter().any(|location| location.uri == base));
    assert!(methods.iter().any(|location| location.uri == child));
    assert!(methods.iter().all(|location| location.uri != other));
}

#[tokio::test]
async fn original_workspace_references_and_lenses_share_owned_byte_declarations() {
    // Implementation contract: naming.server.original-reference-and-lens-source-selection
    // docs/design/analysis/name-resolution-proofs/server-original-reference-and-lens-source-selection.md
    let backend = test_backend();
    let provider = Uri::from_str("file:///original-reference-provider.tcl").unwrap();
    let consumer = Uri::from_str("file:///original-reference-consumer.tcl").unwrap();
    let other = Uri::from_str("file:///original-reference-other.tcl").unwrap();
    let sibling = Uri::from_str("file:///original-reference-sibling.tcl").unwrap();
    let source = r"proc p\uD800 {} {}; proc p\uD801 {} {}";
    register(&backend, &provider, source).await;
    register(&backend, &consumer, r"p\uD800").await;
    register(&backend, &other, r"p\uD801").await;
    register(
        &backend,
        &sibling,
        r"namespace eval N {proc p\uD800 {} {}; p\uD800}",
    )
    .await;
    let document = backend.read_document(&consumer).await.unwrap().unwrap();
    let mut analysis = (*backend
        .analysis_for(&consumer, document.text.clone(), document.dialect.clone())
        .await
        .unwrap())
    .clone();
    analysis.all_procs.clear();
    let position = Position {
        line: 0,
        character: 2,
    };
    let identity = backend
        .original_declaration_identity_at(&consumer, &document.text, &analysis, position)
        .await
        .unwrap()
        .expect("a pure consumer retains the exact original provider");
    assert_eq!(identity.uri(), provider.as_str());
    let references =
        Box::pin(backend.reference_locations(&consumer, &document, &analysis, position, false))
            .await
            .unwrap();
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].uri, consumer);
    assert_eq!(
        references,
        backend
            .original_declaration_reference_locations(&identity, &consumer, false)
            .await
            .unwrap()
    );
    let references =
        Box::pin(backend.reference_locations(&consumer, &document, &analysis, position, true))
            .await
            .unwrap();
    assert_eq!(references.len(), 2);
    assert!(references.iter().any(|location| location.uri == provider));
    assert!(
        references
            .iter()
            .all(|location| location.uri != other && location.uri != sibling)
    );
    let definitions = backend
        .cross_document_definition(&consumer, &document.text, position, &analysis)
        .await
        .unwrap();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].uri, provider);

    let duplicate = Uri::from_str("file:///original-reference-duplicate.tcl").unwrap();
    register(&backend, &duplicate, source).await;
    assert_eq!(
        Box::pin(backend.reference_locations(&consumer, &document, &analysis, position, false))
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Location; 0]
    );
}

#[tokio::test]
async fn original_hierarchy_and_lens_wire_data_require_retained_current_declarations() {
    // Implementation contract: naming.consumer.original-declaration-wire-roundtrip
    // docs/design/analysis/name-resolution-proofs/original-declaration-wire-roundtrip.md
    let backend = test_backend();
    let uri = Uri::from_str("file:///original-wire.tcl").unwrap();
    let source = r"proc p\uD800 {} {}; proc p\uD801 {} {}";
    register(&backend, &uri, source).await;
    let doc = backend.read_document(&uri).await.unwrap().unwrap();
    let analysis = backend
        .analysis_for(&uri, doc.text.clone(), doc.dialect)
        .await
        .unwrap();
    let identities =
        tcl_lsp_core::original_declaration::declarations(uri.as_str(), source, &analysis).unwrap();
    assert_eq!(identities.len(), 2);
    let data = backend
        .retain_original_declaration_data(&uri, source, &analysis, Some(&identities[0]))
        .await
        .unwrap();
    let restored = backend
        .resolve_original_declaration_data(Some(&data), &uri, source, &analysis)
        .await
        .unwrap();
    assert_eq!(restored, identities[0]);
    let other = Uri::from_str("file:///same-source-copy.tcl").unwrap();
    assert!(
        backend
            .resolve_original_declaration_data(Some(&data), &other, source, &analysis)
            .await
            .is_none()
    );
    assert!(
        backend
            .resolve_original_declaration_data(Some(&data), &uri, "proc changed {} {}", &analysis)
            .await
            .is_none()
    );
    assert!(
        backend
            .resolve_original_declaration_data(None, &uri, source, &analysis)
            .await
            .is_none()
    );
    let forged = serde_json::json!({"originalDeclaration":{"schema":1,"token":18_446_744_073_709_551_615_u64}});
    assert!(
        backend
            .resolve_original_declaration_data(Some(&forged), &uri, source, &analysis)
            .await
            .is_none()
    );
    let untrusted_labels = serde_json::json!({"originalDeclaration":data["originalDeclaration"],"qname":"unrelated","range":{"start":99}});
    assert_eq!(
        backend
            .resolve_original_declaration_data(Some(&untrusted_labels), &uri, source, &analysis)
            .await
            .unwrap(),
        identities[0]
    );
    let lenses = backend
        .code_lens(CodeLensParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(lenses.len(), 2);
    assert_ne!(lenses[0].data, lenses[1].data);
    assert!(lenses.iter().all(|lens| {
        lens.command.is_none()
            && lens
                .data
                .as_ref()
                .unwrap()
                .get("originalDeclaration")
                .is_some()
    }));
    let mut stale = lenses[0].clone();
    stale.data.as_mut().unwrap()["originalDeclaration"] = forged["originalDeclaration"].clone();
    assert!(
        backend
            .code_lens_resolve(stale)
            .await
            .unwrap()
            .command
            .is_none()
    );
}
