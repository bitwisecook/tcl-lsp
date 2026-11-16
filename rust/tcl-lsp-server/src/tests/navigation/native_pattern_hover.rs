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

use super::super::*;

async fn open_into(backend: &Backend, uri: &Uri, source: &str) {
    Box::pin(backend.did_open(DidOpenTextDocumentParams {
        text_document: tower_lsp_server::ls_types::TextDocumentItem {
            uri: uri.clone(),
            language_id: "tcl".to_owned(),
            version: 1,
            text: source.to_owned(),
        },
    }))
    .await;
    let document = backend.read_document(uri).await.unwrap().unwrap();
    crate::rt::timeout(std::time::Duration::from_secs(30), async {
        loop {
            if backend
                .pull_diag_cache
                .lock()
                .await
                .get(uri)
                .is_some_and(|entry| entry.revision == document.revision)
            {
                break;
            }
            crate::rt::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("actual matching diagnostics publication");
}

async fn open_settled(source: &str) -> (Backend, Uri) {
    let backend = test_backend();
    let uri = Uri::from_str("file:///native-pattern-hover.tcl").unwrap();
    open_into(&backend, &uri, source).await;
    (backend, uri)
}

async fn request(backend: &Backend, uri: &Uri, position: Position) -> Option<Hover> {
    backend
        .hover(
            serde_json::from_value(serde_json::json!({
                "textDocument": {"uri": uri.as_str()},
                "position": {"line": position.line, "character": position.character}
            }))
            .unwrap(),
        )
        .await
        .unwrap()
}

#[derive(Clone, Copy)]
enum ExternalOccupant {
    Class,
    Procedure,
    Object,
}

fn external_occupant_matches(
    index: &core_workspace_index::WorkspaceIndex,
    owner: &Uri,
    metadata: &tcl_compiler::registry_invocation::OriginalConditionalRegistryMetadata,
    kind: ExternalOccupant,
) -> bool {
    let policy = metadata.original_head().name_input().policy();
    let paths = metadata.possible_source_command_paths();
    let matches = |found_owner: &str, slot: &tcl_core_types::ByteCommandSlot, found| {
        found_owner == owner.as_str()
            && found == policy
            && paths.iter().any(|path| path.contains(slot))
    };
    match kind {
        ExternalOccupant::Class => index
            .original_class_source_candidates()
            .any(|(owner, _, slot, policy)| matches(owner, slot, policy)),
        ExternalOccupant::Procedure => index
            .original_procedure_source_candidates()
            .any(|(owner, _, slot, policy)| matches(owner, slot, policy)),
        ExternalOccupant::Object => {
            index
                .original_command_publications()
                .any(|(owner, publication)| {
                    publication.kind()
                        == tcl_compiler::command_binding::OriginalCommandPublicationKind::Object
                        && matches(owner, publication.slot(), publication.policy())
                })
        }
    }
}

async fn assert_external_occupant_refused(provider_source: &str, kind: ExternalOccupant) {
    let source = "regexp {^foo.*$} $line\n";
    let (backend, uri) = open_settled(source).await;
    let document = backend.read_document(&uri).await.unwrap().unwrap();
    let analysis = backend
        .analysis_for(&uri, document.text.clone(), document.dialect.clone())
        .await
        .unwrap();
    let generation = backend.live_publication_uri_generation(&uri);
    assert!(
        backend
            .original_pattern_hover_for_snapshot(
                (uri.clone(), document.clone(), generation),
                &analysis,
                Position::new(0, 10),
                backend.diag_inputs_epoch()
            )
            .await
            .unwrap()
            .is_some(),
        "actual conditional pattern before independent occupation"
    );
    let metadata = analysis
        .original_conditional_registry_metadata_in_source(
            &tcl_lexer::SourceImage::document(source),
            analysis.body_lexer_config.unwrap(),
            0,
        )
        .expect("actual retained conditional catalogue");
    let paths = metadata.possible_source_command_paths();
    assert!(!paths.is_empty() && paths.iter().all(|path| !path.is_empty()));
    let provider = Uri::from_str("file:///external-pattern-occupant.tcl").unwrap();
    open_into(&backend, &provider, provider_source).await;
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    {
        let index = backend.workspace_index.read().await;
        assert_eq!(
            index.source_analysis_input(uri.as_str()),
            analysis.resolved_input.as_ref()
        );
        let occupied = external_occupant_matches(&index, &provider, metadata, kind);
        assert!(
            occupied,
            "real provider publication must occupy an authentic conditional path"
        );
        assert!(
            core_hover::registry_pattern_format_hover_in_workspace(
                source,
                0,
                10,
                &analysis,
                context.commands(),
                &index
            )
            .is_none(),
            "exact workspace occupation is terminal"
        );
    }
    assert!(
        backend
            .original_pattern_hover_for_snapshot(
                (uri, document, generation),
                &analysis,
                Position::new(0, 10),
                backend.diag_inputs_epoch()
            )
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn native_pattern_hover_keeps_first_settled_registry_operand() {
    let source = "regexp {^(\\d+)\\s+(\\w+)$} $line\n";
    let (backend, uri) = open_settled(source).await;
    let document = backend.read_document(&uri).await.unwrap().unwrap();
    let analysis = backend
        .analysis_for(&uri, document.text.clone(), document.dialect.clone())
        .await
        .unwrap();
    assert!(!analysis.allows_lexical_declaration_advice());
    assert!(
        !analysis
            .resolved_input
            .as_ref()
            .unwrap()
            .has_logical_source_name_context()
    );
    assert!(
        matches!(
            tcl_lsp_core::method_symbol::select(source, &analysis, 0, 10),
            ControlFlow::Break(None)
        ),
        "actual local method abstention"
    );
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    assert!(
        core_hover::registry_pattern_format_hover(source, 0, 10, &analysis, context.commands())
            .is_some(),
        "genuine retained Registry pattern operand"
    );
    let metadata = analysis
        .original_conditional_registry_metadata_in_source(
            &tcl_lexer::SourceImage::document(source),
            analysis.body_lexer_config.unwrap(),
            0,
        )
        .expect("actual conditional catalogue before Server alternative");
    assert!(!metadata.possible_source_command_paths().is_empty());
    let first = request(&backend, &uri, Position::new(0, 10))
        .await
        .expect("first actual Backend pattern hover");
    let text = format!("{:?}", first.contents);
    for expected in ["Regex pattern", "Capture group", "Digit"] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
}

#[tokio::test]
async fn native_pattern_hover_preserves_source_shadows_and_retired_snapshots() {
    assert_external_occupant_refused(
        "rename regexp {}\noo::class create regexp {}\n",
        ExternalOccupant::Class,
    )
    .await;
    assert_external_occupant_refused("proc regexp {args} {}\n", ExternalOccupant::Procedure).await;
    assert_external_occupant_refused(
        "rename regexp {}\noo::object create regexp\n",
        ExternalOccupant::Object,
    )
    .await;
    for (source, character) in [
        (
            "proc regexp {args} {return 0}\nregexp {^foo.*$} $line\n",
            10,
        ),
        (
            "rename regexp {}\noo::class create regexp {}\nregexp {^foo.*$} $line\n",
            10,
        ),
        ("oo::class create C {}\nC {^foo.*$} $line\n", 5),
        (
            "oo::class create C {method m {} {}}\nset obj unknown\n$obj m\n",
            5,
        ),
    ] {
        let (backend, uri) = open_settled(source).await;
        let line = u32::try_from(source.lines().count() - 1).unwrap();
        let first = request(&backend, &uri, Position::new(line, character)).await;
        assert!(
            first.is_none_or(|hover| !format!("{:?}", hover.contents).contains("Regex pattern")),
            "a shadow or unresolved OO source cannot become a Registry pattern: {source}"
        );
    }
    let source = "regexp {^foo.*$} $line\n";
    let (backend, uri) = open_settled(source).await;
    let document = backend.read_document(&uri).await.unwrap().unwrap();
    let analysis = backend
        .analysis_for(&uri, document.text.clone(), document.dialect.clone())
        .await
        .unwrap();
    let epoch = backend.diag_inputs_epoch();
    let generation = backend.live_publication_uri_generation(&uri);
    assert!(
        backend
            .original_pattern_hover_for_snapshot(
                (uri.clone(), document.clone(), generation),
                &analysis,
                Position::new(0, 10),
                epoch
            )
            .await
            .unwrap()
            .is_some(),
        "actual source lease before withdrawal"
    );
    Box::pin(backend.did_change(DidChangeTextDocumentParams {
        text_document: tower_lsp_server::ls_types::VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![tower_lsp_server::ls_types::TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "# retired source\n".to_owned(),
        }],
    }))
    .await;
    assert!(
        backend
            .original_pattern_hover_for_snapshot(
                (uri, document, generation),
                &analysis,
                Position::new(0, 10),
                epoch
            )
            .await
            .unwrap()
            .is_none(),
        "actual changed publication cannot renew old advice"
    );
}
