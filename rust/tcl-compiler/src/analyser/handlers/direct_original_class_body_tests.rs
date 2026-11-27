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

const OPAQUE: &str = "proc p\\uD800 {} {return A}\nproc p\\uD801 {} {return B}\noo::class create C\\uD800 {method m\\uD800 {} {return A}; renamemethod m\\uD800 moved}\noo::class create C\\uD801 {method m\\uD801 {} {return B}}\nset v\\uD800 1\nset v\\uD801 2\n";

type HeldFactoryBody = (
    u32,
    Token,
    &'static tcl_registry::definer::DefinitionBodyGrammar,
    std::sync::Arc<crate::registry_invocation::OriginalSourceScriptBody>,
);

fn checked_factory_layout(
    declaration: &crate::command_binding::OriginalSourceClassDeclaration,
    expected_name: &[u8],
    context: &tcl_registry::model::ContextRegistry,
    grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
) -> ManufacturerLayout {
    let factory = declaration.factory();
    let descriptor = context
        .context()
        .resolve_spec(context.commands(), factory.command())
        .unwrap();
    assert!(
        descriptor
            .traits
            .contains(tcl_registry::Traits::IS_OO_METACLASS)
    );
    assert!(
        descriptor
            .definition_body
            .is_some_and(|selected| std::ptr::eq(selected, grammar))
    );
    let arguments = factory.arguments();
    // These exact fixtures contain only unchanged written factory
    // arguments: effective domain excludes the original head.
    assert_eq!(factory.original_words().len(), arguments.len() + 1);
    for (index, (argument, original)) in arguments
        .iter()
        .zip(&factory.original_words()[1..])
        .enumerate()
    {
        assert_eq!(
            argument.origin,
            crate::registry_invocation::InvocationWordOrigin::Written(index + 1)
        );
        assert_eq!(&argument.original, original);
        assert!(!argument.original.group().expand);
    }
    let selector = std::str::from_utf8(arguments[0].value.as_deref().unwrap()).unwrap();
    let manufacturer =
        tcl_registry::CommandRegistry::manufacturer_method_for_descriptor(descriptor, selector)
            .filter(|method| method.visibility == tcl_registry::definer::MemberVisibility::Exported)
            .unwrap();
    let layout = ManufacturerLayout::builtin(manufacturer).unwrap();
    assert!(layout.injected.is_empty());
    assert!(!layout.inheritance_unknown);
    let name = &arguments[layout.name_arg];
    assert_eq!(name.value.as_deref(), Some(expected_name));
    let occurrence = crate::signature_scan::original_name::SourceOriginalNameOccurrence::from_original_class_factory(declaration).unwrap();
    assert_eq!(occurrence.name_input().original_word(), &name.original);
    assert_eq!(occurrence.name_input().bytes(), expected_name);
    layout
}

fn assert_body_query_refusals(
    source: &str,
    analysis: &super::super::types::AnalysisResult,
    offset: u32,
    token: Token,
    wrong_token: Token,
    grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
) {
    assert!(
        original_class_definition_body_in_analysis(source, analysis, offset, wrong_token, grammar)
            .is_none()
    );
    assert!(
        original_class_definition_body_in_analysis(
            source,
            analysis,
            offset,
            token,
            &tcl_registry::definer::ITCL_GRAMMAR
        )
        .is_none()
    );
    assert!(
        original_class_definition_body_in_analysis(
            &format!("{source} "),
            analysis,
            offset,
            token,
            grammar
        )
        .is_none()
    );
    assert!(
        original_class_definition_body_in_analysis(
            source,
            &super::super::types::AnalysisResult::default(),
            offset,
            token,
            grammar
        )
        .is_none()
    );
}

fn assert_distinct_factory_bodies(
    source: &str,
    analysis: &super::super::types::AnalysisResult,
    issued_bodies: &[HeldFactoryBody],
) {
    if issued_bodies.len() == 2 {
        assert!(
            original_class_definition_body_in_analysis(
                source,
                analysis,
                issued_bodies[0].0,
                issued_bodies[1].1,
                issued_bodies[0].2
            )
            .is_none()
        );
        assert!(
            original_class_definition_body_in_analysis(
                source,
                analysis,
                issued_bodies[1].0,
                issued_bodies[0].1,
                issued_bodies[1].2
            )
            .is_none()
        );
    }
}

#[test]
fn held_original_factory_receipts_can_query_source_body_without_text_name_or_entry() {
    for (source, offsets, expected_names) in [
        (
            "oo::class create Plain {method m {} {return ASCII}}\n",
            &[0_u32][..],
            &[b"Plain".as_slice()][..],
        ),
        (
            OPAQUE,
            &[54_u32, 138][..],
            &[b"C\xed\xa0\x80".as_slice(), b"C\xed\xa0\x81".as_slice()][..],
        ),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let realm = analysis.retained_command_realm().unwrap();
        let image = realm.original_source_image().unwrap();
        assert_eq!(image, &tcl_lexer::SourceImage::document(source));
        assert!(analysis.matches_original_source_image(image, input.lexer_config()));
        let before = (
            analysis.original_class_declarations().count(),
            analysis.all_classes.len(),
            analysis.all_procs.len(),
        );
        let mut issued_bodies = Vec::new();
        for (&offset, &expected_name) in offsets.iter().zip(expected_names) {
            let declaration =
                crate::registry_invocation::source_structure::source_class_declaration_at(
                    source, &analysis, offset,
                )
                .expect("actual held source declaration, independently of failed text adapter");
            let factory = declaration.factory();
            assert_eq!(factory.site().offset, offset);
            assert!(factory.matches_source(image, input.lexer_config()));
            assert!(factory.matches_context(&context));
            let grammar = declaration.grammar(&context).unwrap();
            let layout = checked_factory_layout(&declaration, expected_name, &context, grammar);
            let arguments = factory.arguments();
            let name = &arguments[layout.name_arg];
            if source == OPAQUE {
                assert!(std::str::from_utf8(expected_name).is_err());
            }
            let original_body = &arguments[layout.body_arg].original;
            assert_eq!(original_body.image(), image);
            assert_eq!(original_body.config(), input.lexer_config());
            let token = *original_body.tokens().first().unwrap();
            // This is an explicit direct SOURCE query, not evidence that
            // the actual handler/definition body was entered.
            let queried = original_class_definition_body_in_analysis(
                source, &analysis, offset, token, grammar,
            );
            assert!(
                queried.is_some(),
                "direct existing body issuer offset={offset}"
            );
            let body = queried.unwrap();
            assert_eq!(body.original_container(), original_body);
            assert_eq!(body.content_span(), original_body.content_span().unwrap());
            assert!(body.matches_source(image, input.lexer_config()));
            assert!(body.matches_context(&context));
            let operand = body
                .source_words()
                .operands()
                .get(layout.body_arg)
                .and_then(Option::as_ref)
                .and_then(
                    crate::registry_invocation::source_structure::OriginalOperandSource::word,
                );
            assert_eq!(operand, Some(original_body));
            let wrong_token = *name.original.tokens().first().unwrap();
            assert_body_query_refusals(source, &analysis, offset, token, wrong_token, grammar);
            let foreign_config = tcl_lexer::LexerConfig::from_grammar(
                tcl_dialect::DialectProfile::find("tcl9.0").unwrap().grammar,
            );
            assert!(!body.matches_source(image, foreign_config));
            issued_bodies.push((offset, token, grammar, body));
        }
        assert_distinct_factory_bodies(source, &analysis, &issued_bodies);
        assert_eq!(
            before,
            (
                analysis.original_class_declarations().count(),
                analysis.all_classes.len(),
                analysis.all_procs.len()
            )
        );
    }
}
#[test]
fn source_factory_parents_keep_their_real_issuers_and_refuse_foreign_inputs() {
    let source = "oo::class create ::Meta {superclass ::oo::class}";
    for dialect in ["tcl", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let analysis = Analyser::new().analyse(source, dialect);
        let declaration =
            crate::registry_invocation::source_structure::source_class_declaration_at(
                source, &analysis, 0,
            )
            .unwrap();
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let grammar = declaration.grammar(&context).unwrap();
        let original = declaration.factory().original_words();
        let token = *original.last().unwrap().tokens().first().unwrap();
        let name_token = *declaration
            .name_input()
            .original_word()
            .unwrap()
            .tokens()
            .first()
            .unwrap();
        let body = original_class_definition_body_in_analysis(source, &analysis, 0, token, grammar)
            .unwrap();
        assert!(matches!(
            body.source_words().source(),
            crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
                _
            )
        ));
        assert_eq!(body.original_container(), original.last().unwrap());
        assert!(
            analysis.class_factories().contains_key("::Meta"),
            "{dialect}"
        );
        let class = &analysis.all_classes["::Meta"];
        let providers = class
            .original_relations
            .effects()
            .flat_map(|effect| effect.values().into_iter().flatten())
            .filter_map(|relation| relation.source_provider())
            .collect::<Vec<_>>();
        assert_eq!(providers.len(), 1);
        assert!(providers[0].matches_analysis(&analysis));
        assert!(providers[0].matches_class_definition(class));
        assert!(providers[0].registry_descriptor(&context).is_some());
        assert_body_query_refusals(source, &analysis, 0, token, name_token, grammar);
        let mut absent = analysis.clone();
        absent.resolved_input = None;
        assert!(
            original_class_definition_body_in_analysis(source, &absent, 0, token, grammar)
                .is_none()
        );
        let mut config = analysis.clone();
        config.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
        assert!(
            original_class_definition_body_in_analysis(source, &config, 0, token, grammar)
                .is_none()
        );
        let mut foreign = analysis.clone();
        foreign.resolved_input = Analyser::new()
            .analyse(
                source,
                if dialect == "tcl9.0" {
                    "tcl8.6"
                } else {
                    "tcl9.0"
                },
            )
            .resolved_input;
        assert!(
            original_class_definition_body_in_analysis(source, &foreign, 0, token, grammar)
                .is_none()
        );
    }
}

#[test]
fn source_factory_layout_cannot_supply_statement_or_proc_completion() {
    use tcl_registry::registry::InvocationCompletion;
    let meta = "oo::class create ::Meta {superclass ::oo::class}";
    for dialect in ["tcl", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for middle in [
            "",
            "proc stop args {exit 0}; trace add execution ::Meta enter ::stop; ",
            "exit 0; ",
        ] {
            let prefix =
                format!("{meta}; proc wrapper {{}} {{::Meta create ::Other {{}}}}; {middle}");
            let source = format!("{prefix}::Meta create ::Made {{}}");
            let mut analyser = Analyser::new();
            analyser.analyse_and_retain_result_for_test(&source, dialect);
            assert!(analyser.result.class_factories().contains_key("::Meta"));
            let commands = tcl_source::segment::segment_commands_with_offset_and_config(
                &source,
                0,
                analyser.file_lexer_config(),
            );
            let command = commands
                .iter()
                .find(|command| command.span.start() == u32::try_from(prefix.len()).unwrap())
                .unwrap();
            assert_eq!(
                analyser.segment_invocation_completion(command),
                InvocationCompletion::Unknown
            );
            assert!(
                !analyser.static_statement_falls_through(command, 0),
                "{dialect}/{middle}"
            );
            let body = analyser.result.all_procs["::wrapper"].body_span;
            assert_ne!(
                analyser.static_proc_body_falls_through("::wrapper", body, 0, &mut Vec::new()),
                Some(true),
                "{dialect}/{middle}"
            );
        }
        for (source, expected, falls) in [
            ("list a b", InvocationCompletion::FallsThrough, true),
            ("exit 0", InvocationCompletion::Terminates, false),
        ] {
            let mut analyser = Analyser::new();
            analyser.analyse_and_retain_result_for_test(source, dialect);
            let command = tcl_source::segment::segment_commands_with_offset_and_config(
                source,
                0,
                analyser.file_lexer_config(),
            )
            .pop()
            .unwrap();
            assert_eq!(analyser.segment_invocation_completion(&command), expected);
            assert_eq!(analyser.static_statement_falls_through(&command, 0), falls);
        }
    }
}
#[test]
fn source_factory_observers_retain_applicability_and_shadows_refuse_metadata() {
    use crate::command_binding::SourceCommandTransitionObligation;
    use crate::registry_invocation::source_structure::OriginalRegistrySource;
    let shadowed = "proc oo::class args {}; oo::class create ::Meta {superclass ::oo::class}";
    let traced = "proc observer args {rename ::oo::class ::held; proc ::oo::class args {}}; trace add execution ::oo::class enter ::observer; oo::class create ::Meta {superclass ::oo::class}";
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        assert!(
            Analyser::new()
                .analyse(shadowed, dialect)
                .class_factories()
                .is_empty()
        );
        let analysis = Analyser::new().analyse(traced, dialect);
        let offset = u32::try_from(traced.rfind("oo::class create").unwrap()).unwrap();
        let declaration =
            crate::registry_invocation::source_structure::source_class_declaration_at(
                traced, &analysis, offset,
            )
            .unwrap();
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let token = *declaration
            .factory()
            .original_words()
            .last()
            .unwrap()
            .tokens()
            .first()
            .unwrap();
        let body = original_class_definition_body_in_analysis(
            traced,
            &analysis,
            offset,
            token,
            declaration.grammar(&context).unwrap(),
        )
        .unwrap();
        let OriginalRegistrySource::SourceTransitions(advice) = body.source_words().source() else {
            panic!("actual source transition parent");
        };
        for obligation in [
            SourceCommandTransitionObligation::UnavailableActualLookup,
            SourceCommandTransitionObligation::RegisteredFactoryApplicability,
        ] {
            assert!(advice.obligations().contains(&obligation));
        }
    }
}
