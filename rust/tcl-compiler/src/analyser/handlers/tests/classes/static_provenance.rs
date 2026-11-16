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

//! The static reading behind class-creation proofs: `if` conditions, control arms, completions and literal values.

use super::*;

#[test]
fn original_if_conditions_keep_selected_grammar_and_unknown_evaluation_boundaries() {
    for (dialect, condition, expected) in [
        ("tcl8.6", "1 + 2 * 3", Some(true)),
        ("tcl8.6", "0", Some(false)),
        ("tcl8.6", "1 / 0", None),
        ("tcl8.6", "$unknown", None),
        ("tcl8.6", "[unknown]", None),
        ("tcl8.6", "abs(1)", None),
        ("tcl8.6", "\"1\"", None),
        ("tcl8.6", "1_0", None),
        ("tcl9.0", "1_0", Some(true)),
    ] {
        let source = format!("if {{{condition}}} {{set x 1}}");
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(&source, dialect);
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            &source,
            0,
            analyser.file_lexer_config(),
        )
        .remove(0);
        assert_eq!(
            analyser.original_condition_boolean(&segment, 1),
            expected,
            "{dialect}: {source}"
        );
        assert!(
            analyser.original_condition_boolean(&segment, 2).is_none(),
            "a Body role cannot donate an expression"
        );
        let mut foreign = segment.clone();
        foreign.texts[1] = "0".to_owned();
        if condition != "0" {
            assert!(
                analyser.original_condition_boolean(&foreign, 1).is_none(),
                "foreign operand data cannot borrow the original expression owner"
            );
        }
        let mut input = analyser.result.resolved_input.clone().unwrap();
        input.config.strict_quoting = !input.config.strict_quoting;
        analyser.resolved_input = Some(input);
        assert!(analyser.original_condition_boolean(&segment, 1).is_none());
    }
    let mut prepared = Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.0").unwrap();
    for escapes in [
        tcl_dialect::EscapeSyntax::Tcl84,
        tcl_dialect::EscapeSyntax::Tcl90,
        tcl_dialect::EscapeSyntax::Jim,
    ] {
        let mut input = input.clone();
        input.config.escapes = escapes;
        let mut analyser = Analyser::new().with_resolved_input(input.clone());
        let source = "if {1} break";
        analyser.analyse_and_retain_result_for_test(source, "display-only");
        assert_eq!(analyser.result.resolved_input.as_ref(), Some(&input));
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .remove(0);
        assert_eq!(analyser.original_condition_boolean(&segment, 1), Some(true));
    }
}

#[test]
fn original_if_branches_use_clause_order_and_complete_original_bodies() {
    for (source, selected_bodies, fallthrough) in [
        ("if {1} {set x 1} else {set x 2}", vec!["set x 1"], false),
        (
            "if {0} {set x 1} elseif {1} {set x 2} else {set x 3}",
            vec!["set x 2"],
            false,
        ),
        ("if {0} {set x 1}", Vec::new(), true),
        (
            "if {$unknown} {set x 1} else {set x 2}",
            vec!["set x 1", "set x 2"],
            false,
        ),
        ("if {1 / 0} {set x 1}", vec!["set x 1"], true),
        (
            "if {1} {set x 1} elseif {1 / 0} {set x 2}",
            vec!["set x 1"],
            false,
        ),
        ("if {0} break else continue", vec!["continue"], false),
    ] {
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, "tcl8.6");
        let config = analyser.file_lexer_config();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let (spans, actual_fallthrough) = analyser
            .possible_if_body_spans(&segment, "unused")
            .expect("actual selected source If and complete original bodies");
        let image = tcl_lexer::SourceImage::document(source);
        let original = crate::registry_invocation::original_native_compiler_words(
            &image,
            crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::from_image(&image),
                config,
                &segment,
            )
            .words(),
            0,
            config,
        )
        .unwrap();
        let selected = spans
            .iter()
            .map(|span| {
                let word = original
                    .iter()
                    .find(|word| {
                        word.tokens()
                            .first()
                            .is_some_and(|token| token.span == *span)
                    })
                    .unwrap();
                source.get(word.content_span().unwrap().as_range()).unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(selected, selected_bodies, "{source}");
        assert_eq!(actual_fallthrough, fallthrough, "{source}");
        analyser.source = format!("{}{}", analyser.source, ' ').into();
        assert!(
            analyser
                .possible_if_body_spans(&segment, "unused")
                .is_none()
        );
    }
}

#[test]
fn control_arm_metadata_requires_the_actual_selected_body_source() {
    use tcl_registry::registry::ControlArmSemantics as Arm;
    for (source, body_ordinal, expected) in [
        ("if {1} {set x 1}", 1, Some(Arm::Selected)),
        (
            "interp alias {} choose {} if; choose {1} {set x 1}",
            1,
            Some(Arm::Selected),
        ),
        ("interp alias {} choose {} if 1; choose {set x 1}", 0, None),
        ("proc if args {}; if {1} {set x 1}", 1, None),
        ("namespace eval N {set x 1}", 2, Some(Arm::FrameBoundary)),
        ("namespace eval N {set x 1;} {set y 2}", 2, None),
        ("apply {{} {set x 1}}", 0, None),
    ] {
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, "tcl8.6");
        let controller = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analyser.file_lexer_config(),
        )
        .pop()
        .unwrap();
        let mut arm = ControlArm {
            body_span: controller.arg_tokens()[body_ordinal].span,
            controller,
        };
        assert_eq!(
            analyser.control_arm_semantics_for(&arm),
            expected,
            "{source}"
        );
        let supplied = analyser.resolved_input.clone();
        let mut foreign = analyser.result.resolved_input.clone().unwrap();
        foreign.config.strict_quoting = !foreign.config.strict_quoting;
        analyser.resolved_input = Some(foreign);
        assert!(
            analyser.control_arm_semantics_for(&arm).is_none(),
            "a changed current input cannot borrow old structural advice"
        );
        analyser.resolved_input = supplied;
        let actual_span = arm.body_span;
        arm.body_span = Span::new(arm.body_span.start(), arm.body_span.start());
        assert!(
            analyser.control_arm_semantics_for(&arm).is_none(),
            "an arbitrary contained point is not an original body"
        );
        arm.body_span = actual_span;
        analyser.source = format!("{}{}", analyser.source, ' ').into();
        assert!(analyser.control_arm_semantics_for(&arm).is_none());
    }
}

#[test]
fn control_arm_positions_require_actual_original_body_containers() {
    for (source, expected) in [
        (
            "if {0} {set x 1} else {set x 2}",
            vec![(1, Some(false)), (3, Some(true))],
        ),
        (
            "switch x {x {set a 1} default {set a 2}}",
            vec![(1, None), (1, None)],
        ),
        ("apply {{} {puts body}}", vec![(0, None)]),
    ] {
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, "tcl8.6");
        let config = analyser.file_lexer_config();
        let controller =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let context = analyser.retained_static_source_context().unwrap();
        let words = crate::registry_invocation::source_structure::source_registry_words_at(
            source,
            &analyser.result,
            0,
        )
        .unwrap();
        let bodies = words
            .source_script_bodies_for(
                &context,
                crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
            )
            .into_iter()
            .chain(words.original_lambda_body_geometry(&context))
            .collect::<Vec<_>>();
        assert_eq!(bodies.len(), expected.len(), "{source}");
        for (body, (ordinal, selected)) in bodies.iter().zip(expected) {
            let mut arm = ControlArm {
                controller: controller.clone(),
                body_span: body.extent_span(),
            };
            assert_eq!(
                analyser.control_arm_body_index(&arm),
                Some(ordinal),
                "{source}"
            );
            if let Some(selected) = selected {
                assert_eq!(
                    analyser.control_body_is_selected(
                        "",
                        &arm,
                        &std::collections::HashMap::new(),
                        0
                    ),
                    Some(selected),
                    "{source}"
                );
            }
            let mut foreign = arm.clone();
            foreign.controller.texts[0] = "another-head".to_owned();
            assert!(analyser.control_arm_body_index(&foreign).is_none());
            let mut foreign = arm.clone();
            foreign.controller.texts[1] = "altered-argument".to_owned();
            assert!(analyser.control_arm_body_index(&foreign).is_none());
            let mut foreign = arm.clone();
            foreign.controller.argv[1].span = Span::new(0, 0);
            assert!(analyser.control_arm_body_index(&foreign).is_none());
            arm.body_span = body.container_extent_span();
            if arm.body_span != body.extent_span() {
                assert!(
                    analyser.control_arm_body_index(&arm).is_none(),
                    "container is not its script child"
                );
            }
            arm.body_span = Span::new(body.content_span().start(), body.content_span().start());
            assert!(
                analyser.control_arm_body_index(&arm).is_none(),
                "interior point cannot donate body authority"
            );
        }
        analyser.source = format!("{}{}", analyser.source, ' ').into();
        let arm = ControlArm {
            controller,
            body_span: bodies[0].extent_span(),
        };
        assert!(analyser.control_arm_body_index(&arm).is_none());
    }
}

#[test]
fn bound_parameter_substitution_uses_the_complete_shared_reference_grammar() {
    let parameters = vec![
        "a".to_owned(),
        "a:b".to_owned(),
        "a{b}".to_owned(),
        "a{b".to_owned(),
    ];
    let arguments = vec![
        "SCALAR".to_owned(),
        "LITERAL".to_owned(),
        "NESTED".to_owned(),
        "FIRST".to_owned(),
    ];
    let mut config = Analyser::new()
        .prepare_analysis_input("tcl")
        .unwrap()
        .lexer_config();
    assert_eq!(
        substitute_bound_words("$a:b", &parameters, &arguments, config).as_deref(),
        Some("SCALAR:b")
    );
    assert_eq!(
        substitute_bound_words("${a:b}", &parameters, &arguments, config).as_deref(),
        Some("LITERAL")
    );
    assert!(substitute_bound_words("$a(index)", &parameters, &arguments, config).is_none());
    config.braced_var = tcl_lexer::BracedVarStyle::Tcl9Nesting;
    assert_eq!(
        substitute_bound_words("${a{b}}", &parameters, &arguments, config).as_deref(),
        Some("NESTED")
    );
    config.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
    assert_eq!(
        substitute_bound_words("${a{b}}", &parameters, &arguments, config).as_deref(),
        Some("FIRST}")
    );
}

#[test]
fn segment_completion_retains_return_depth_and_implementation_identity() {
    use tcl_registry::registry::InvocationCompletion;
    for (source, expected) in [
        ("return VALUE", InvocationCompletion::ReturnsResult(Some(0))),
        ("return -level 2 VALUE", InvocationCompletion::Terminates),
        (
            "proc p {depth} {return -level $depth VALUE}",
            InvocationCompletion::Unknown,
        ),
        (
            "proc return args {list CUSTOM}; return VALUE",
            InvocationCompletion::Unknown,
        ),
        (
            "rename return savedReturn; savedReturn VALUE",
            InvocationCompletion::ReturnsResult(Some(0)),
        ),
    ] {
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, "tcl9.0");
        // The public walk clears transient registry state after returning.
        analyser.registry = Some(analyser.profile_registry());
        let segments = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analyser.lexer_config(),
        );
        let segment = segments.last().unwrap();
        let segment = if source.starts_with("proc p") {
            analyser
                .direct_statements_in_span(segment.arg_tokens()[2].span)
                .unwrap()
                .pop()
                .unwrap()
        } else {
            segment.clone()
        };
        let binding = analyser
            .head_identities
            .invocation_at_source(segment.name(), segment.argv[0].span.start());
        assert_eq!(
            analyser.segment_invocation_completion(&segment),
            expected,
            "{source}; unknown={} absent={} targets={:?} execution={:?} handler={:?} compilation={:?}",
            binding.unknown,
            binding.may_be_absent,
            binding
                .targets
                .iter()
                .map(|target| (&target.command, target.registry_backed))
                .collect::<Vec<_>>(),
            binding
                .proved_execution_target()
                .map(|target| &target.command),
            binding
                .proved_handler_target()
                .map(|target| &target.command),
            binding.native_compilation_selection(),
        );
    }
}

#[test]
fn original_static_proc_selection_does_not_borrow_final_names_or_caller_labels() {
    // Implementation contract: naming.compiler.original-static-procedure-selection
    // docs/design/analysis/name-resolution-proofs/original-static-procedure-selection.md
    let sources = [
        "namespace eval N {proc helper {} {return OLD}}; proc N::helper {} {return NEW}",
        "namespace eval N {proc helper {} {return OLD}}; rename N::helper N::saved",
        "namespace eval N {proc helper {} {return OLD}}; interp alias {} N::helper {} puts",
        "namespace eval N {proc helper {} {return NAMESPACE}}; proc helper {} {return GLOBAL}",
    ];
    for dialect in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jim",
        "f5-irules",
    ] {
        for source in sources {
            let mut analyser = Analyser::new();
            analyser.analyse_and_retain_result_for_test(source, dialect);
            analyser.registry = Some(analyser.profile_registry());
            assert!(
                analyser
                    .resolve_static_proc_name("::N::caller", "helper")
                    .is_none(),
                "{dialect}: {source}"
            );
            assert!(
                analyser
                    .static_proc_result("::N::helper", &[], &mut Vec::new(), 0)
                    .is_none(),
                "{dialect}: {source}"
            );
            assert!(
                analyser
                    .static_command_value(
                        "helper",
                        &[],
                        "::N::caller",
                        &std::collections::HashMap::new(),
                        &mut Vec::new(),
                        0,
                    )
                    .is_none(),
                "{dialect}: source-free command folding has no occupied target"
            );
        }
    }
    // A replaced Registry head is also an occupied implementation, not
    // authority supplied by the builtin spelling in a fold table.
    let mut shadowed = Analyser::new();
    shadowed.analyse_and_retain_result_for_test(
        "proc string args {return USER}; string length abc",
        "tcl8.6",
    );
    shadowed.registry = Some(shadowed.profile_registry());
    assert!(
        shadowed
            .static_command_value(
                "string",
                &[Some("length".to_owned()), Some("abc".to_owned())],
                "::caller",
                &std::collections::HashMap::new(),
                &mut Vec::new(),
                0,
            )
            .is_none()
    );
    let mut logical = Analyser::new();
    logical.analyse_and_retain_result_for_test(sources[3], "tcl");
    logical.registry = Some(logical.profile_registry());
    assert_eq!(
        logical
            .resolve_static_proc_name("::N::caller", "helper")
            .as_deref(),
        Some("::N::helper")
    );
    assert_eq!(
        logical
            .static_proc_result("::N::helper", &[], &mut Vec::new(), 0)
            .as_deref(),
        Some("NAMESPACE")
    );
}

fn assert_no_static_proc_completion(analyser: &mut Analyser) {
    assert!(
        analyser
            .static_proc_result("::p", &[], &mut Vec::new(), 0)
            .is_none()
    );
}

#[test]
fn original_logical_static_completion_keeps_alias_ordinals_and_source_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional source result advice, without Native return completion.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = |context| {
        super::super::super::super::ResolvedAnalysisInput::new(profile, profile, context, config)
    };
    let analyse = |source: &str, input| {
        let mut analyser = Analyser::new().with_resolved_input(input);
        analyser.analyse_and_retain_result_for_test(source, "tcl");
        analyser
    };
    for (source, expected) in [
        ("proc p {} {return café}", Some("café")),
        (
            "interp alias {} answer {} return; proc p {} {answer WRITTEN}",
            Some("WRITTEN"),
        ),
        (
            "interp alias {} answer {} return CAPTURED; proc p {} {answer}",
            None,
        ),
        (
            "interp alias {} answer {} return -level 2; proc p {} {answer VALUE}",
            None,
        ),
        (
            "proc return args {return CHANGED}; proc p {} {return VALUE}",
            None,
        ),
    ] {
        let mut analyser = analyse(source, input(std::sync::Arc::clone(&context)));
        assert_eq!(
            analyser
                .static_proc_result("::p", &[], &mut Vec::new(), 0)
                .as_deref(),
            expected,
            "{source}"
        );
    }
    let mut availability_store = tcl_registry::CommandRegistry::build_default();
    availability_store.insert(tcl_registry::CommandSpec {
        name: "metadata_return_current",
        surface: Some(tcl_dialect::model::SpecSurface::TCL86_PLUS),
        ..availability_store.get("return").unwrap().clone()
    });
    let shared = availability_store.snapshot().shared_registry();
    let current = std::sync::Arc::new(context.with_command_store(std::sync::Arc::clone(&shared)));
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::resolve_environment("tcl8.4")
            .default_context_registry()
            .with_command_store(shared),
    );
    assert!(std::sync::Arc::ptr_eq(current.commands(), older.commands()));
    let selected_source = "proc p {} {metadata_return_current VALUE}";
    let mut selected = analyse(selected_source, input(current));
    assert_eq!(
        selected
            .static_proc_result("::p", &[], &mut Vec::new(), 0)
            .as_deref(),
        Some("VALUE")
    );
    let mut unavailable = analyse(selected_source, input(older));
    assert_no_static_proc_completion(&mut unavailable);
    let source = "proc p {} {return VALUE}";
    let mut missing = analyse(source, input(std::sync::Arc::clone(&context)));
    missing.resolved_input = None;
    missing.result.resolved_input = None;
    assert_no_static_proc_completion(&mut missing);
    let mut changed = analyse(source, input(std::sync::Arc::clone(&context)));
    let mut altered = config;
    altered.strict_quoting = !altered.strict_quoting;
    let changed_input = super::super::super::super::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        altered,
    );
    changed.resolved_input = Some(changed_input.clone());
    changed.result.resolved_input = Some(changed_input);
    assert_no_static_proc_completion(&mut changed);
    let mut store = tcl_registry::CommandRegistry::build_default();
    store.insert(tcl_registry::CommandSpec {
        name: "foreign_completion_axis",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let foreign = std::sync::Arc::new(context.with_command_store(std::sync::Arc::new(store)));
    let mut changed = analyse(source, input(std::sync::Arc::clone(&context)));
    let foreign_input = input(foreign);
    changed.resolved_input = Some(foreign_input.clone());
    changed.result.resolved_input = Some(foreign_input);
    assert_no_static_proc_completion(&mut changed);
}

#[test]
fn builtin_next_completion_requires_the_actual_mro_end() {
    assert_eq!(
        Analyser::builtin_next_completion(&tcl_registry::definer::TCLOO_GRAMMAR, "unknown", 0, 2,),
        None
    );
    assert_eq!(
        Analyser::builtin_next_completion(&tcl_registry::definer::TCLOO_GRAMMAR, "unknown", 1, 2,),
        Some(true)
    );
}

#[test]
fn static_write_invalidation_preserves_scalar_parenthesis_names() {
    for name in ["a(b", "a(b)tail", "a(b)"] {
        let source = format!("set {name} VALUE");
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(&source, "tcl");
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            &source,
            0,
            analyser.file_lexer_config().nested(),
        )
        .remove(0);
        let mut values = std::collections::HashMap::from([
            ("a".to_owned(), "ROOT".to_owned()),
            ("a(b".to_owned(), "OPEN".to_owned()),
            ("a(b)tail".to_owned(), "SUFFIX".to_owned()),
        ]);
        analyser
            .invalidate_static_facts_written_by_segment(&command, &mut values, 0)
            .unwrap();
        let root = if name == "a(b)" { "a" } else { name };
        assert!(!values.contains_key(root));
        assert_eq!(values.len(), 2);
    }
}
