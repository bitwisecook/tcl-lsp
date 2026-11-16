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
use crate::workspace_index::WorkspaceIndex;
use tcl_compiler::analyser::Analyser;

fn analysis(source: &str) -> AnalysisResult {
    let mut result = Analyser::new().analyse(source, "tcl8.6").clone();
    result.all_classes.clear();
    result.superseded_classes.clear();
    result
}

#[test]
fn lexical_variable_name_units_do_not_select_a_method_operand() {
    let result = analysis("oo::class create C {method ping {} {}}; set ping VALUE");
    let class = result.original_class_declarations().next().unwrap();
    let method = class
        .metadata()
        .original_members
        .methods(MemberSide::Instance)
        .unwrap()
        .into_iter()
        .find(|method| method.original_name_input().bytes() == b"ping")
        .unwrap();
    let records = [("", class)];
    let selector = method.original_name_input();
    assert!(
        metadata_candidate(&[], &records, 0, (selector, MemberSide::Instance, false)).is_some()
    );
    let image = SourceImage::document("$ping");
    let config = result.body_lexer_config.unwrap();
    let plan = tcl_lexer::native_script_words_in(image, Span::new(0, 5), config).unwrap();
    let word = &plan.commands[0].words[0];
    let root = tcl_compiler::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_word(
            word,
            word.span(),
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            selector.policy(),
        ).unwrap();
    let root = SignatureSourceNameInput::OriginalVariableRoot(root);
    assert_eq!(root.bytes(), selector.bytes());
    assert_eq!(root.policy(), selector.policy());
    assert!(metadata_candidate(&[], &records, 0, (&root, MemberSide::Instance, false)).is_none());
}

#[test]
fn logical_class_calls_keep_their_separate_source_consumer_domain() {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let source = "oo::class create C {self method ping {} {}}; C ping";
    let result = Analyser::new().analyse(source, "tcl");
    assert!(result.allows_retained_logical_declaration_advice());
    assert!(result.original_class_declarations().next().is_none());
    let commands = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        result.body_lexer_config.unwrap(),
    );
    let command = commands.last().unwrap();
    let call = tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
        source,
        &result,
        command.argv[0].span.start(),
    )
    .unwrap();
    assert!(
        call.class_declaration()
            .name_input()
            .native_input()
            .is_none()
    );
    assert!(
        call.class_declaration()
            .logical_source_class(&result)
            .is_some()
    );
    assert!(matches!(
        command_query(source, &result, command),
        ControlFlow::Continue(())
    ));
}

#[test]
fn original_class_call_method_candidates_keep_call_horizons_and_captured_selectors() {
    // naming.source.original-class-publications
    // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
    let source = "oo::class create C {self method ping {} {}}; C ping; rename C M; C ping; M ping; interp alias {} A {} M ping; A; rename M {}; M ping";
    let result = analysis(source);
    assert!(result.original_completed_command_world().is_none());
    let commands = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        result.body_lexer_config.unwrap(),
    );
    let mut selected = Vec::new();
    for command in &commands {
        if !matches!(command.name(), "C" | "M" | "A") {
            continue;
        }
        let query = command_query(source, &result, command);
        let candidate = match query {
            ControlFlow::Break(Some(query)) => {
                if command.name() == "A" {
                    let Target::SourceClassCall(call) = &query.target else {
                        panic!("original captured call");
                    };
                    assert_eq!(call.written_argument(0), None);
                    assert_eq!(query.selector_input().bytes(), b"ping");
                    assert!(query.span().start() < command.argv[0].span.start());
                    assert_ne!(call.obligations(), []);
                }
                candidate_for_analysis(&result, &query)
            }
            _ => ControlFlow::Break(None),
        };
        let found = matches!(candidate, ControlFlow::Break(Some(_)));
        if let ControlFlow::Break(Some(candidate)) = candidate {
            assert_eq!(candidate.method().original_name_input().bytes(), b"ping");
            assert!(!candidate.has_call_point_entry());
        }
        selected.push(found);
    }
    assert_eq!(selected, [true, false, true, true, false]);
}

#[test]
fn original_workspace_class_method_candidates_use_current_slots_and_canonical_targets() {
    // naming.source.original-class-publications
    // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
    let library_source = "oo::class create C {self method ping {} {}}; rename C M";
    let library = analysis(library_source);
    let caller_source = "M ping; C ping";
    let caller = analysis(caller_source);
    assert!(library.original_completed_command_world().is_none());
    let library_uri = "file:///classes.tcl";
    let caller_uri = "file:///caller.tcl";
    let mut index =
        WorkspaceIndex::from_documents([(library_uri, &library), (caller_uri, &caller)]);
    let queries = source_queries(&caller);
    assert_eq!(queries.len(), 2);
    let ControlFlow::Break(Some(selected)) = candidate(&index, caller_uri, &queries[0]) else {
        panic!("current moved source suggestion");
    };
    assert_eq!(selected.uri(), library_uri);
    assert_eq!(selected.class_input.as_ref().unwrap().bytes(), b"C");
    assert_eq!(
        selected.class.as_ref().unwrap(),
        library
            .original_class_declarations()
            .next()
            .unwrap()
            .declaration_site()
    );
    assert!(!selected.has_call_point_entry());
    assert!(matches!(
        candidate(&index, caller_uri, &queries[1]),
        ControlFlow::Continue(())
    ));
    for source in [
        "oo::class create C {self method ping {} {}}; rename C {}",
        "oo::class create C {self method ping {} {}}; proc C {} {}",
    ] {
        index.replace_document(library_uri, &analysis(source));
        for query in &queries {
            assert!(matches!(
                candidate(&index, caller_uri, query),
                ControlFlow::Continue(())
            ));
        }
    }
    let mut missing = library.clone();
    missing.resolved_input = None;
    index.replace_document(library_uri, &missing);
    assert!(matches!(
        candidate(&index, caller_uri, &queries[0]),
        ControlFlow::Continue(())
    ));
}

#[test]
fn original_local_method_providers_share_opaque_declaration_and_whole_source_currency() {
    // Implementation contract: naming.editor.original-method-candidates (docs/design/analysis/name-resolution-proofs/original-method-candidates.md).
    let source =
        "namespace eval N {oo::class create C {method m\\uD800 {a} {}; method m\\uD801 {a b} {}}}";
    let result = analysis(source);
    let cursor = u32::try_from(source.find("m\\uD800").unwrap()).unwrap();
    let ControlFlow::Break(Some(selected)) = local_candidate(source, &result, 0, cursor) else {
        panic!("genuine original nested method declaration");
    };
    assert_eq!(
        selected.method().original_name_input().bytes(),
        b"m\xed\xa0\x80"
    );
    assert_eq!(selected.declaration_image(), &SourceImage::document(source));
    assert_eq!(
        crate::definition::definition(source, 0, cursor, &result).len(),
        1
    );
    assert_eq!(
        crate::references::references(
            source,
            crate::profile_for_dialect("tcl8.6"),
            0,
            cursor,
            &result,
            true
        )
        .len(),
        1
    );
    assert!(
        crate::hover::hover(source, 0, cursor, &result, None)
            .unwrap()
            .value
            .contains("Possible source declaration")
    );
    let stale = source.replacen('N', "M", 1);
    assert!(matches!(
        local_candidate(&stale, &result, 0, cursor),
        ControlFlow::Break(None)
    ));
    assert_eq!(
        crate::definition::definition(&stale, 0, cursor, &result),
        [] as [crate::definition::LspRange; 0]
    );
    assert_eq!(
        crate::references::references(
            &stale,
            crate::profile_for_dialect("tcl8.6"),
            0,
            cursor,
            &result,
            true
        ),
        [] as [crate::definition::LspRange; 0]
    );
    assert!(crate::hover::hover(&stale, 0, cursor, &result, None).is_none());
}

#[test]
fn original_local_method_providers_refuse_reporting_receiver_class_guesses() {
    // Implementation contract: naming.editor.original-method-candidates (docs/design/analysis/name-resolution-proofs/original-method-candidates.md).
    let source = "oo::class create C {method m {} {}}\nset obj unknown\n$obj m\n";
    let mut result = Analyser::new().analyse(source, "tcl8.6").clone();
    result
        .instance_classes
        .insert("obj".to_owned(), "::C".to_owned());
    assert!(matches!(
        local_candidate(source, &result, 2, 5),
        ControlFlow::Break(None)
    ));
    assert_eq!(
        crate::definition::definition(source, 2, 5, &result),
        [] as [crate::definition::LspRange; 0]
    );
    assert_eq!(
        crate::references::references(
            source,
            crate::profile_for_dialect("tcl8.6"),
            2,
            5,
            &result,
            true
        ),
        [] as [crate::definition::LspRange; 0]
    );
    assert!(crate::hover::hover(source, 2, 5, &result, None).is_none());
}

#[test]
fn original_method_selection_leaves_lexical_variable_reads_with_the_variable_owner() {
    // Implementation contract: naming.editor.original-method-candidates (docs/design/analysis/name-resolution-proofs/original-method-candidates.md).
    for (source, line, character) in [
        (
            "namespace eval N {}\nset ::N::v VALUE\nunknown $::N::v\n",
            2,
            13,
        ),
        ("set method m\nC $method\n", 1, 4),
    ] {
        let mut result = analysis(source);
        let ControlFlow::Break(Some(variable)) =
            crate::variable_symbol::select(source, &result, line, character)
        else {
            panic!("genuine lexical variable read");
        };
        let retained_root = variable.span();
        assert!(matches!(
            select(source, &result, line, character),
            ControlFlow::Continue(())
        ));
        // Keep the authentic retained read while its selected symbol is
        // unavailable. Its own provider must refuse instead of borrowing OO.
        result.original_variable_symbols.clear();
        result
            .original_variable_symbol_conflicts
            .push(retained_root);
        assert!(matches!(
            crate::variable_symbol::select(source, &result, line, character),
            ControlFlow::Break(None)
        ));
        assert!(matches!(
            select(source, &result, line, character),
            ControlFlow::Continue(())
        ));
    }
}

#[test]
fn original_method_lookup_owner_absence_is_terminal_before_reporting_assistance() {
    // Implementation contract: naming.editor.original-method-candidates (docs/design/analysis/name-resolution-proofs/original-method-candidates.md).
    let source = "C m\n";
    let mut result = analysis(source);
    assert!(matches!(
        select(source, &result, 0, 2),
        ControlFlow::Break(Some(_))
    ));
    let invocation = result
        .command_invocations
        .iter_mut()
        .find(|invocation| invocation.range.start() == 0)
        .unwrap();
    assert!(invocation.original_name_input.is_some());
    assert!(invocation.original_lookup.take().is_some());
    assert!(matches!(
        select(source, &result, 0, 2),
        ControlFlow::Break(None)
    ));
    assert!(matches!(
        local_candidate(source, &result, 0, 2),
        ControlFlow::Break(None)
    ));
    assert_eq!(
        crate::definition::definition(source, 0, 2, &result),
        [] as [crate::definition::LspRange; 0]
    );
    assert_eq!(
        crate::references::references(
            source,
            crate::profile_for_dialect("tcl8.6"),
            0,
            2,
            &result,
            true
        ),
        [] as [crate::definition::LspRange; 0]
    );
    assert!(crate::hover::hover(source, 0, 2, &result, None).is_none());
    let foreign = analysis("D m\n")
        .command_invocations
        .into_iter()
        .find_map(|invocation| invocation.original_lookup)
        .unwrap();
    result.command_invocations[0].original_lookup = Some(foreign);
    assert!(matches!(
        select(source, &result, 0, 2),
        ControlFlow::Break(None)
    ));
}

#[test]
fn original_workspace_member_candidates_keep_opaque_inputs_and_document_owners() {
    // Implementation contract: naming.editor.original-method-candidates (docs/design/analysis/name-resolution-proofs/original-method-candidates.md).
    let provider = analysis(
        "oo::class create C\\uD800 {self method m\\uD800 {} {}; self method m\\uD801 {} {}}\n",
    );
    let consumer_source = "C\\uD800 m\\uD800\n";
    let consumer = analysis(consumer_source);
    let ControlFlow::Break(Some(query)) = select(consumer_source, &consumer, 0, 11) else {
        panic!("retained original selector");
    };
    assert_eq!(query.class_input().unwrap().bytes(), b"C\xed\xa0\x80");
    assert_eq!(query.selector_input().bytes(), b"m\xed\xa0\x80");
    let mut index = WorkspaceIndex::from_documents([
        ("file:///provider.tcl", &provider),
        ("file:///consumer.tcl", &consumer),
    ]);
    let ControlFlow::Break(Some(selected)) = candidate(&index, "file:///consumer.tcl", &query)
    else {
        panic!("exact own-table source candidate");
    };
    assert_eq!(selected.uri(), "file:///provider.tcl");
    assert_eq!(
        selected.method().original_name_input().bytes(),
        b"m\xed\xa0\x80"
    );
    assert!(!selected.has_call_point_entry());
    assert!(
        selected
            .hover()
            .unwrap()
            .value
            .contains("Possible source declaration")
    );
    index.add_document("file:///provider.tcl", &provider);
    assert!(matches!(
        candidate(&index, "file:///consumer.tcl", &query),
        ControlFlow::Break(Some(_))
    ));
    index.add_document("file:///duplicate.tcl", &provider);
    assert!(matches!(
        candidate(&index, "file:///consumer.tcl", &query),
        ControlFlow::Break(None)
    ));
    assert!(matches!(
        select(&format!("#{consumer_source}"), &consumer, 0, 11),
        ControlFlow::Continue(()) | ControlFlow::Break(None)
    ));
}

#[test]
fn original_instance_member_advice_uses_complete_relations_and_never_class_side_mro() {
    // Implementation contract: naming.editor.original-method-candidates (docs/design/analysis/name-resolution-proofs/original-method-candidates.md).
    let base =
        analysis("oo::class create B\\uD800 {method m\\uD800 {} {}; self method cm {} {}}\n");
    let child = analysis("oo::class create C {superclass B\\uD800}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///base.tcl", &base),
        ("file:///child.tcl", &child),
    ]);
    let root = child.original_class_declarations().next().unwrap();
    let parent = base.original_class_declarations().next().unwrap();
    let input = parent
        .metadata()
        .original_members
        .declarations()
        .find(|method| method.side() == MemberSide::Instance)
        .unwrap()
        .original_name_input();
    let selected = member_metadata_candidate(
        &index,
        "file:///child.tcl",
        root,
        input,
        MemberSide::Instance,
    )
    .unwrap();
    assert_eq!(selected.uri(), "file:///base.tcl");
    assert!(
        member_metadata_candidate(
            &index,
            "file:///child.tcl",
            root,
            input,
            MemberSide::ClassObject
        )
        .is_none()
    );
    let missing = WorkspaceIndex::from_documents([("file:///child.tcl", &child)]);
    assert!(
        member_metadata_candidate(
            &missing,
            "file:///child.tcl",
            root,
            input,
            MemberSide::Instance
        )
        .is_none()
    );
    let mut duplicate = index.clone();
    duplicate.add_document("file:///other-base.tcl", &base);
    assert!(
        member_metadata_candidate(
            &duplicate,
            "file:///child.tcl",
            root,
            input,
            MemberSide::Instance
        )
        .is_none()
    );
}

#[test]
fn original_member_advice_stops_at_earlier_occupied_non_class_headers() {
    // Implementation contract: naming.editor.original-method-candidates (docs/design/analysis/name-resolution-proofs/original-method-candidates.md).
    let provider = analysis("oo::class create C {self method m {} {}}\n");
    let blocker = analysis("namespace eval N {proc C {} {}}\n");
    let source = "namespace eval N {C m}\n";
    let consumer = analysis(source);
    let index = WorkspaceIndex::from_documents([
        ("file:///provider.tcl", &provider),
        ("file:///blocker.tcl", &blocker),
        ("file:///consumer.tcl", &consumer),
    ]);
    let cursor = u32::try_from(source.find("C m").unwrap() + 2).unwrap();
    let ControlFlow::Break(Some(query)) = select_at_offset(source, &consumer, cursor) else {
        panic!("authentic pending class operand");
    };
    assert!(matches!(
        candidate(&index, "file:///consumer.tcl", &query),
        ControlFlow::Continue(())
    ));
}
