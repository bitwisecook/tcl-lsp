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
use tcl_lexer::Span;

fn command_name_pattern_with_closer(
    word: &str,
    candidate: &str,
    closer: tcl_lexer::BracedVarStyle,
) -> bool {
    let mut input = Analyser::new().prepare_analysis_input("tcl").unwrap();
    input.config.braced_var = closer;
    let config = input.lexer_config();
    let source = format!("marker {word}");
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(&source, 0, config).remove(0);
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(&source),
        config,
        &segment,
    );
    let original = crate::registry_invocation::original_native_compiler_words(
        &tcl_lexer::SourceImage::document(&source),
        tokens.words(),
        0,
        config,
    )
    .unwrap();
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
        input.borrowed_context_registry().commands(),
        &input,
        config,
        Some(input.unit_profile()),
    );
    crate::dynamic_names::original_command_name_overlap(&original[1], candidate, metadata)
        .unwrap()
        .may_equal
}

fn dynamic_command_name_may_equal(word: &str, candidate: &str) -> bool {
    command_name_pattern_with_closer(word, candidate, tcl_lexer::BracedVarStyle::FirstClose)
}

#[test]
fn original_registry_class_definers_require_the_selected_source_and_complete_arguments() {
    for (source, expected) in [
        ("oo::class create Made {}", true),
        ("proc oo::class args {}; oo::class create Made {}", false),
    ] {
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, "tcl");
        analyser.registry = Some(analyser.profile_registry());
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analyser.file_lexer_config(),
        )
        .pop()
        .unwrap();
        assert_eq!(
            analyser
                .resolve_class_creation(segment.args(), segment.arg_tokens(), segment.argv[0])
                .is_some_and(|creation| creation.registry_metaclass.is_some()),
            expected,
            "{source}",
        );
        if expected {
            let mut changed = segment.args().to_vec();
            changed[0] = "new".to_owned();
            assert!(
                analyser
                    .resolve_class_creation(&changed, segment.arg_tokens(), segment.argv[0])
                    .is_none()
            );
            analyser.source = format!("{}{}", analyser.source, ' ').into();
            assert!(
                analyser
                    .resolve_class_creation(segment.args(), segment.arg_tokens(), segment.argv[0])
                    .is_none()
            );
        }
    }
}

#[test]
fn original_factory_overrides_require_readable_original_member_layout() {
    for (source, represented) in [
        (
            "oo::class create Meta {superclass ::oo::class; self method create {name body} {next $name $body}}",
            true,
        ),
        (
            "oo::class create Meta {superclass ::oo::class; self method create {name body} {return opaque}}",
            false,
        ),
        (
            "oo::class create Meta {superclass ::oo::class; self forward create opaque}",
            false,
        ),
    ] {
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, "tcl8.6");
        let class = analyser
            .result
            .all_classes
            .values()
            .find(|class| class.name == "Meta")
            .unwrap();
        let factory = analyser.class_factory_of(class);
        assert_eq!(factory.is_some(), represented, "{source}");
        if let Some(factory) = factory {
            let layout = factory.overrides.get("create").unwrap();
            assert_eq!((layout.name_arg, layout.body_arg), (1, 2));
            let mut report_only = class.clone();
            report_only.class_methods.clear();
            report_only.class_unexports.insert("create".to_owned());
            assert_eq!(analyser.class_factory_of(&report_only), Some(factory));
        }
    }
}

#[test]
fn logical_factory_override_uses_the_retained_original_member_formals() {
    for (body, expected) in [("next $name $body", true), ("return opaque", false)] {
        let source = format!(
            "oo::class create Meta {{superclass ::oo::class; self method create {{name body}} {{{body}}}}}"
        );
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(&source, "tcl");
        let class = analyser
            .result
            .all_classes
            .values()
            .find(|class| class.name == "Meta")
            .unwrap();
        let factory = analyser.class_factory_of(class);
        assert_eq!(factory.is_some(), expected, "{source}");
        if let Some(factory) = factory {
            let layout = factory.overrides.get("create").unwrap();
            assert_eq!((layout.name_arg, layout.body_arg), (1, 2));
        }
    }
}

#[test]
fn original_factory_headers_require_the_actual_graph_occupancy_and_arguments() {
    for (source, key, expected) in [
        ("External create Made {}", "::External", true),
        ("puts create Made {}", "::puts", false),
        (
            "proc External args {}; External create Made {}",
            "::External",
            false,
        ),
        (
            "interp alias {} External {} puts; External create Made {}",
            "::External",
            false,
        ),
        (
            "unknown_effect; External create Made {}",
            "::External",
            false,
        ),
        ("External create [set name] {}", "::External", false),
    ] {
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, "tcl");
        analyser.registry = Some(analyser.profile_registry());
        analyser.workspace_class_factories = Some(std::sync::Arc::new(
            [(
                key.to_owned(),
                ClassFactory {
                    root_metaclass: "::oo::class".to_owned(),
                    ..ClassFactory::default()
                },
            )]
            .into_iter()
            .collect(),
        ));
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analyser.file_lexer_config(),
        )
        .pop()
        .unwrap();
        assert_eq!(
            analyser
                .class_factory_for_command(
                    segment.args(),
                    segment.arg_tokens(),
                    segment.argv[0].span.start(),
                )
                .is_some(),
            expected,
            "{source}",
        );
        if expected {
            let mut changed = segment.arg_tokens().to_vec();
            changed.swap(0, 1);
            assert!(
                analyser
                    .class_factory_for_command(
                        segment.args(),
                        &changed,
                        segment.argv[0].span.start()
                    )
                    .is_none()
            );
            let mut changed = segment.args().to_vec();
            changed[0] = "new".to_owned();
            assert!(
                analyser
                    .class_factory_for_command(
                        &changed,
                        segment.arg_tokens(),
                        segment.argv[0].span.start()
                    )
                    .is_none()
            );
            analyser.source = format!("{}{}", analyser.source, ' ').into();
            assert!(
                analyser
                    .class_factory_for_command(
                        segment.args(),
                        segment.arg_tokens(),
                        segment.argv[0].span.start()
                    )
                    .is_none()
            );
        }
    }
}

#[test]
fn manufacturer_prologue_components_keep_original_reference_and_group_geometry() {
    for (source, expected) in [
        ("next $a:b", vec!["a", "opaque"]),
        ("next ${a:b}", vec!["a:b"]),
        ("next {$body}", vec!["opaque"]),
        (
            "next \"[list superclass {a]b}]\\;$body\"",
            vec!["group", "separator", "body"],
        ),
    ] {
        let image = tcl_lexer::SourceImage::document(source);
        let config = tcl_lexer::LexerConfig::default();
        let plan = tcl_lexer::native_script_words_in(
            image,
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let observed = prologue_pieces(&plan.commands[0].words[1]);
        let labels = observed
            .iter()
            .map(|piece| match piece {
                ProloguePiece::VarRead(name) => *name,
                ProloguePiece::Separator => "separator",
                ProloguePiece::Substitution(_) => "group",
                ProloguePiece::Opaque => "opaque",
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, expected, "{source}");
    }
}

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
fn factory_splices_keep_original_list_data_geometry_and_grammar() {
    let member = FactoryMember {
        words: vec![FactoryWord::CallerSplice(0)],
    };
    for (source, expected) in [
        ("maker {A B}", Some(vec!["A", "B"])),
        (
            "maker {A $literal [data]}",
            Some(vec!["A", "$literal", "[data]"]),
        ),
        (r"maker {escaped\ name}", Some(vec!["escaped name"])),
        ("maker {}", Some(Vec::new())),
        ("maker $dynamic", None),
        ("maker {*}{one two}", None),
        (r"maker {\uD800}", None),
        ("maker {one \"two}", None),
    ] {
        let config = tcl_lexer::LexerConfig {
            expand_syntax: true,
            ..tcl_lexer::LexerConfig::default()
        };
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let original = &plan.commands[0].words[1..];
        let result = resolve_factory_member(&member, Some(original));
        assert_eq!(
            result.as_ref().map(|member| member
                .texts
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()),
            expected,
            "{source}"
        );
        if let Some(result) = result {
            let content = original[0].content_span().unwrap();
            assert_eq!(result.texts.len(), result.argv.len());
            assert!(
                result
                    .argv
                    .iter()
                    .all(|token| content.start() <= token.span.start()
                        && token.span.end() <= content.end())
            );
        }
        assert!(
            resolve_factory_member(&member, None).is_none(),
            "a report-only caller cannot donate an empty splice"
        );
    }
}

#[test]
fn factory_list_templates_require_actual_constructor_and_scalar_formal_splices() {
    for (prefix, constructor, operand, expected) in [
        ("", "list", "$parents", true),
        ("", "::list", "${parents}", true),
        ("", "list", "$parents:tail", false),
        ("", "list", "$parents(key)", false),
        ("", "list", "[set parents]", false),
        ("", "split", "$parents", false),
        ("", "lreverse", "$parents", false),
        (
            "proc list {args} {return opaque}\n",
            "list",
            "$parents",
            false,
        ),
    ] {
        let source = format!(
            "{prefix}oo::class create Meta {{superclass ::oo::class; self method create {{name parents body}} {{next $name \"[{constructor} super\\u0063lass {{*}}{operand}];$body\"}}}}"
        );
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(&source, "tcl8.6");
        let class = analyser
            .result
            .all_classes
            .values()
            .find(|class| class.name == "Meta")
            .unwrap();
        let meta = analyser
            .user_metaclass_of_class(class)
            .expect("actual retained Registry root relation");
        let methods = crate::analyser::class_hierarchy::original_metadata::original_definition_members_for_class(
                &analyser.result, class, MemberSide::ClassObject,
            ).unwrap();
        let method = methods
            .iter()
            .find(|method| method.original_name_input().bytes() == b"create")
            .unwrap();
        let original = method
            .original_parameter_names_and_body(&analyser.result)
            .unwrap();
        let body = original.body_word();
        let plan = tcl_lexer::native_script_words_in(
            body.image().clone(),
            body.content_span().unwrap(),
            body.config(),
        )
        .unwrap();
        let prologue = &plan.commands[0].words[2];
        let group = prologue_pieces(prologue)
            .into_iter()
            .find_map(|part| match part {
                ProloguePiece::Substitution(group) => Some(group),
                _ => None,
            })
            .unwrap();
        let template = analyser.injected_member_from_group(&meta, group, &original);
        assert_eq!(template.is_some(), expected, "{source}");
        if let Some(template) = template {
            assert!(
                matches!(template.words.as_slice(), [FactoryWord::Literal { text, .. }, FactoryWord::CallerSplice(2)] if text == "superclass")
            );
        }
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
fn command_mutation_overlap_retains_the_original_configured_closer() {
    assert!(dynamic_command_name_may_equal("a:::$x:b", "::a::b"));
    assert!(!dynamic_command_name_may_equal("a:::$x:b", "::a::OTHER"));
    assert!(command_name_pattern_with_closer(
        "${a{b}}",
        "::string",
        tcl_lexer::BracedVarStyle::Tcl9Nesting
    ));
    assert!(!command_name_pattern_with_closer(
        "${a{b}}",
        "::string",
        tcl_lexer::BracedVarStyle::FirstClose
    ));
    for (closer, trusted) in [
        (tcl_lexer::BracedVarStyle::FirstClose, true),
        (tcl_lexer::BracedVarStyle::Tcl9Nesting, false),
    ] {
        let mut input = Analyser::new().prepare_analysis_input("tcl").unwrap();
        input.config.braced_var = closer;
        let mut analyser = Analyser::new().with_resolved_input(input);
        analyser.analyse_and_retain_result_for_test("proc ${a{b}} {} {}", "tcl");
        assert_eq!(
            analyser.static_provenance_command_is_trusted("::string"),
            trusted
        );
    }
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

#[test]
fn a_whole_word_loop_variable_records_its_declaration_occurrence() {
    // TP — `foreach name …` declares `name` at the word itself, as
    // `catch {…} res` declares `res`; the read alone left navigation
    // from `$name` with no declaration to answer.
    let source = "foreach name {a b} {puts $name}\nproc p {} {foreach x {1 2} {puts $x}}\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
        let result = Analyser::new().analyse(source, dialect);
        for name in ["name", "x"] {
            let at = u32::try_from(source.find(&format!("foreach {name}")).unwrap() + 8).unwrap();
            let span = Span::new(at, at + u32::try_from(name.len()).unwrap());
            assert!(
                result
                    .original_variable_symbols
                    .iter()
                    .any(|occurrence| occurrence.span() == span),
                "{dialect}: {name}"
            );
        }
    }
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
    let input =
        |context| super::super::ResolvedAnalysisInput::new(profile, profile, context, config);
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
    let changed_input = super::super::ResolvedAnalysisInput::new(
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

fn original_value_use_token(analyser: &Analyser, needle: &str) -> Token {
    let start = u32::try_from(analyser.source.rfind(needle).unwrap()).unwrap();
    let commands = crate::segmenter::segment_commands_with_offset_and_config(
        analyser
            .source
            .get(usize::try_from(start).unwrap()..)
            .unwrap(),
        start,
        analyser.file_lexer_config().nested(),
    );
    commands.first().unwrap().argv[0]
}

#[test]
fn logical_dynamic_word_keeps_exact_braced_name_and_interpolation_sources() {
    let source =
        "set n WRONG; set {$n} RIGHT; set {scalar(open} SCALAR; puts ${n}:${$n}:${scalar(open}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = original_value_use_token(&analyser, "${n}:${$n}:${scalar(open}");
    assert_eq!(
        analyser.resolve_dynamic_word("${n}:${$n}:${scalar(open}", Some(token), false, &[]),
        Some("WRONG:RIGHT:SCALAR".to_owned())
    );
    analyser.source = source.replace("RIGHT", "OTHER").into();
    assert!(
        analyser
            .resolve_dynamic_word("${n}:${$n}:${scalar(open}", Some(token), false, &[])
            .is_none()
    );
}

#[test]
fn logical_nested_constant_fold_uses_original_variable_components() {
    let source = "set n WRONG; set {$n} RIGHT; set result [list $n [list ${$n}]]; puts $result";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = original_value_use_token(&analyser, "$result");
    assert_eq!(
        analyser
            .lookup_logical_const_substitution(token)
            .map(|(value, _)| value),
        Some("WRONG RIGHT")
    );
}

#[test]
fn logical_regex_value_metadata_uses_original_scope_and_source_provenance() {
    let source = "set pattern {ROOT}; proc p {} {set pattern {LOCAL}; regexp $pattern text}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let source_use = source.rfind("$pattern").unwrap();
    assert!(analyser.result.regex_patterns.iter().any(|pattern| {
        pattern.pattern == "LOCAL" && usize::try_from(pattern.range.start()).unwrap() == source_use
    }));
    assert!(
        !analyser
            .result
            .regex_patterns
            .iter()
            .any(|pattern| pattern.pattern == "ROOT")
    );
}

#[test]
fn logical_interpreter_handle_value_keeps_distinct_path_provenance_and_snapshot() {
    let source = "set child [interp create named]; interp eval $child {set x VALUE}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = original_value_use_token(&analyser, "$child");
    assert_eq!(
        analyser.resolve_dynamic_interp_path(Some(token)).as_deref(),
        Some("named")
    );
    let snapshot = analyser.snapshot();
    analyser.source = source.replace("VALUE", "OTHER").into();
    assert!(analyser.resolve_dynamic_interp_path(Some(token)).is_none());
    analyser.source = source.to_owned().into();
    analyser.restore(snapshot);
    assert_eq!(
        analyser.resolve_dynamic_interp_path(Some(token)).as_deref(),
        Some("named")
    );
    let source = "set child named; interp eval $child {set x VALUE}";
    let mut scalar = Analyser::new();
    scalar.analyse_and_retain_result_for_test(source, "tcl");
    let token = original_value_use_token(&scalar, "$child");
    assert!(scalar.resolve_dynamic_interp_path(Some(token)).is_none());
}

#[test]
fn logical_dictionary_keys_have_metadata_and_data_geometry_without_variable_declarations() {
    let source = "set d {alpha A beta B}; dict with d {puts $alpha}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let scope = &analyser.result.global_scope;
    assert!(!scope.variables.contains_key("alpha"));
    assert!(!scope.variables.contains_key("beta"));
    let alpha = scope.readonly_dictionary_keys.get("alpha").unwrap();
    assert_eq!(
        &source[alpha.original_key_span().unwrap().as_range()],
        "alpha"
    );
    let image = tcl_lexer::SourceImage::document(source);
    let advice = analyser
        .result
        .logical_variable_scope_advice(
            &image,
            analyser.file_lexer_config(),
            u32::try_from(source.rfind("$alpha").unwrap()).unwrap(),
        )
        .unwrap();
    assert!(advice.readonly_variable_names().contains(&"alpha"));
    assert!(advice.variable_definition("alpha").is_none());
}

#[test]
fn logical_dictionary_metadata_retains_an_empty_variable_name() {
    let source = "set d {{} EMPTY}; dict with d {puts ${}}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let scope = &analyser.result.global_scope;
    assert!(scope.readonly_dictionary_keys.contains_key(""));
    assert!(!scope.variables.contains_key(""));
}

#[test]
fn logical_dictionary_write_invalidates_the_previous_constant_value() {
    let source = "set x OLD; set d {x NEW}; dict with d {puts $x}; puts $x";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = original_value_use_token(&analyser, "$x");
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
    assert!(analyser.result.global_scope.variables.contains_key("x"));
}

#[test]
fn logical_dictionary_write_invalidates_interpreter_handle_provenance() {
    let source = "set child [interp create named]; set d {child other}; dict with d {}; interp eval $child {set x VALUE}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = original_value_use_token(&analyser, "$child");
    assert!(analyser.resolve_dynamic_interp_path(Some(token)).is_none());
}

#[test]
fn logical_dictionary_metadata_masks_an_outer_procedure_name() {
    let source = "set x ROOT; proc p {} {set d {x LOCAL}; dict with d {puts $x}}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = original_value_use_token(&analyser, "$x");
    let image = tcl_lexer::SourceImage::document(source);
    let advice = analyser
        .result
        .logical_variable_scope_advice(&image, analyser.file_lexer_config(), token.span.start())
        .unwrap();
    assert!(advice.readonly_variable_names().contains(&"x"));
    assert!(advice.variable_definition("x").is_none());
    assert!(analyser.lookup_logical_const_substitution(token).is_none());
}

#[test]
fn dynamic_apply_value_uses_checked_logical_variable_declaration() {
    let source = "namespace eval N {set lambda {{p} {puts $p} ::N}}; apply $::N::lambda VALUE";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = tcl_lexer::Lexer::with_config(source, analyser.file_lexer_config())
        .tokenise_all()
        .unwrap()
        .into_iter()
        .find(|token| token.kind == TokenType::Var)
        .unwrap();
    let elements = analyser
        .resolve_dynamic_apply_lambda(token)
        .expect("original Logical namespace value");
    assert_eq!(
        elements
            .iter()
            .map(|(_, value)| value.as_str())
            .collect::<Vec<_>>(),
        vec!["p", "puts $p", "::N"]
    );
    analyser.source = source.replace("VALUE", "OTHER").into();
    assert!(analyser.resolve_dynamic_apply_lambda(token).is_none());
}

#[test]
fn logical_defined_symbol_word_uses_current_value_provenance() {
    let source = "set name SYMBOL; puts $name";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let token = tcl_lexer::Lexer::with_config(source, analyser.file_lexer_config())
        .tokenise_all()
        .unwrap()
        .into_iter()
        .find(|token| token.kind == TokenType::Var)
        .unwrap();
    assert_eq!(
        analyser.defined_symbol_reporting_name(None, None, "$name", token, true, &[]),
        Some("SYMBOL".to_owned())
    );
    let original = analyser.snapshot();
    analyser.result.resolved_input = None;
    assert!(
        analyser
            .defined_symbol_reporting_name(None, None, "$name", token, true, &[])
            .is_none()
    );
    analyser.restore(original);
    assert_eq!(
        analyser.resolve_const_word("$name", token, true),
        Some("SYMBOL".to_owned())
    );
}

#[test]
fn dict_with_value_uses_checked_logical_scope() {
    let source =
        "set d {root ROOT}; namespace eval N {set d {local LOCAL}; dict with d {puts $local}}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let namespace = analyser
        .result
        .global_scope
        .children
        .iter()
        .find(|scope| scope.kind == super::super::types::ScopeKind::Namespace)
        .unwrap();
    assert!(namespace.readonly_dictionary_keys.contains_key("local"));
    assert!(!namespace.readonly_dictionary_keys.contains_key("root"));
    assert!(!namespace.variables.contains_key("local"));
    let key = &namespace.readonly_dictionary_keys["local"];
    assert_eq!(
        &source[key.original_key_span().unwrap().as_range()],
        "local"
    );
    let source = "set d {root ROOT}; proc p {} {dict with d {puts $root}}";
    let mut procedure = Analyser::new();
    procedure.analyse_and_retain_result_for_test(source, "tcl");
    let local = procedure
        .result
        .global_scope
        .children
        .iter()
        .find(|scope| scope.kind == super::super::types::ScopeKind::Proc)
        .unwrap();
    assert!(!local.variables.contains_key("root"));
}

#[test]
fn dynamic_apply_scanner_uses_irules_brace_boundary_config() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // An original Logical builder with the supplied lexer overlay, without
    // any Native/F5 list execution or lambda-frame claim.
    let source = "set lambda [list {p} {set y}{set z}]";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_dialect("f5-irules");
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let input = super::super::ResolvedAnalysisInput::new(profile, profile, context, config);
    let mut analyser = Analyser::new().with_resolved_input(input);
    analyser.result = analyser.analyse(source, "tcl");
    let token = tcl_lexer::Lexer::with_source_map(tcl_lexer::SourceMap::new(source), config)
        .tokenise_all()
        .expect("valid command substitution")
        .into_iter()
        .find(|token| token.kind == tcl_lexer::TokenType::Cmd)
        .expect("command token");
    let elements = analyser
        .resolve_dynamic_apply_lambda(token)
        .expect("original literal list lambda");
    assert_eq!(elements.len(), 3, "iRules list elements: {elements:?}");
    let mut missing = Analyser::new();
    missing.source = source.to_owned().into();
    missing.profile = tcl_dialect::DialectProfile::irules();
    assert!(
        missing.resolve_dynamic_apply_lambda(token).is_none(),
        "a profile and authored source do not supply the original builder"
    );
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
fn dynamic_command_name_patterns_keep_only_proven_fixed_fragments() {
    assert!(!dynamic_command_name_may_equal(
        "${ns}::define::$method",
        "::string"
    ));
    assert!(dynamic_command_name_may_equal("$name", "::string"));
    assert!(dynamic_command_name_may_equal(
        "${ns}::::helper",
        "::T::helper"
    ));
    assert!(dynamic_command_name_may_equal(
        "$array(index)::define",
        "::anything"
    ));
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

#[test]
fn command_mutation_matching_preserves_constructed_colon_keys() {
    assert!(dynamic_command_name_may_equal("p", ":::::p"));
    assert!(dynamic_command_name_may_equal(":p", ":::p"));
    assert!(!dynamic_command_name_may_equal(":p", ":::::p"));
    // Written separator runs canonicalise once; an already constructed
    // namespace key must not be interpreted as that written spelling.
    assert!(dynamic_command_name_may_equal(":::::p", "::p"));
    assert!(!dynamic_command_name_may_equal(":::::p", ":::::p"));
    assert!(dynamic_command_name_may_equal("x:::y", "::x::y"));
    assert!(dynamic_command_name_may_equal("x:::", "::x::"));
    assert!(!dynamic_command_name_may_equal("x:::y", "::x:::y"));
}

fn class_observation(superclass: Option<&str>, factory: bool) -> ClassDef {
    ClassDef {
        name: "class".to_owned(),
        qualified_name: "::T::D::class".to_owned(),
        superclasses: superclass.into_iter().map(str::to_owned).collect(),
        factory: factory.then(|| ClassFactory {
            root_metaclass: "oo::class".to_owned(),
            ..ClassFactory::default()
        }),
        ..ClassDef::default()
    }
}

#[test]
fn parameterised_class_join_is_monotone_across_incomplete_observations() {
    let proved = class_observation(Some("::T::Mother"), true);
    let incomplete = class_observation(None, false);

    let later_incomplete = join_parameterised_class_observations(&proved, &incomplete).unwrap();
    assert_eq!(later_incomplete.superclasses, vec!["::T::Mother"]);
    assert!(later_incomplete.factory.is_some());

    let later_proof = join_parameterised_class_observations(&incomplete, &proved).unwrap();
    assert_eq!(later_proof.superclasses, vec!["::T::Mother"]);
    assert!(later_proof.factory.is_some());
}

#[test]
fn parameterised_class_join_abstains_on_conflicting_concrete_relations() {
    let left = class_observation(Some("::T::Mother"), true);
    let right = class_observation(Some("::T::OtherMother"), true);
    assert!(join_parameterised_class_observations(&left, &right).is_none());
}

#[test]
fn parameterised_class_join_ignores_an_unobserved_metaclass() {
    // The `oo::define` stub's `"oo::class"` is
    // `ClassDef::default()`'s stand-in, not something a walk read, so it
    // must neither contradict the proved `::T::Mother` nor survive into
    // the joined record.
    let stub = class_observation(None, false);
    assert_ne!(stub.metaclass_provenance, MetaclassProvenance::Observed);
    let proved = ClassDef {
        metaclass: "::T::Mother".to_owned(),
        metaclass_provenance: MetaclassProvenance::Observed,
        ..class_observation(Some("::T::Mother"), true)
    };

    for (existing, observed) in [(&stub, &proved), (&proved, &stub)] {
        let joined = join_parameterised_class_observations(existing, observed)
            .expect("a stand-in metaclass is not conflicting evidence");
        assert_eq!(joined.metaclass, "::T::Mother");
        assert_eq!(joined.metaclass_provenance, MetaclassProvenance::Observed);
        assert_eq!(joined.superclasses, vec!["::T::Mother"]);
        assert!(joined.factory.is_some());
    }
}

#[test]
fn parameterised_class_join_abstains_on_two_observed_metaclasses() {
    // FP guard for the gate above: once *both* sides read a head word,
    // a disagreement is still a real conflict.
    let left = ClassDef {
        metaclass: "::T::Mother".to_owned(),
        metaclass_provenance: MetaclassProvenance::Observed,
        ..class_observation(None, true)
    };
    let right = ClassDef {
        metaclass: "::T::OtherMother".to_owned(),
        metaclass_provenance: MetaclassProvenance::Observed,
        ..class_observation(None, true)
    };
    assert!(join_parameterised_class_observations(&left, &right).is_none());
    // …and identical observed metaclasses still join.
    assert!(join_parameterised_class_observations(&left, &left).is_some());
}

fn esc_tok(span: Span) -> Token {
    Token::new(TokenType::Esc, span)
}

fn str_tok(span: Span) -> Token {
    Token {
        kind: TokenType::Str,
        span,
        content_offset: 1,
        in_quote: false,
    }
}

fn span(start: u32, end: u32) -> Span {
    Span::new(start, end)
}

// Explicit standalone word-shape controls use the shared lexer owner.
fn elements(text: &str) -> Option<Vec<String>> {
    let (head, args) = crate::value_shapes::parse_command_substitution_with_config(
        text,
        tcl_lexer::LexerConfig::default(),
    )?;
    Some(std::iter::once(head).chain(args).collect())
}

#[test]
fn interp_create_value_words_strip_a_single_braced_path_issue_1025() {
    // TP — `{child}` is one Tcl word naming interpreter `child`; the
    // direct handler sees it segmenter-decoded, so the value-flow path
    // must strip the braces too rather than binding to `"{child}"`.
    assert_eq!(
        elements("[interp create {child}]"),
        Some(vec!["interp".into(), "create".into(), "child".into()])
    );
}

#[test]
fn interp_create_value_words_keep_a_nested_path_as_one_word_issue_1025() {
    // TP — `{parent child}` is one word (a descent path), not two.
    // `split_whitespace` would yield `["{parent", "child}"]`, whose first
    // fragment would become the bound key.
    assert_eq!(
        elements("[interp create {parent child}]"),
        Some(vec![
            "interp".into(),
            "create".into(),
            "parent child".into()
        ])
    );
}

#[test]
fn interp_create_value_words_unchanged_for_bare_and_flagged_forms_issue_1025() {
    // TN — the shapes that already worked must be byte-for-byte
    // unchanged by the switch to list parsing.
    assert_eq!(
        elements("[interp create -safe -- name]"),
        Some(vec![
            "interp".into(),
            "create".into(),
            "-safe".into(),
            "--".into(),
            "name".into()
        ])
    );
    assert_eq!(
        elements("[interp create]"),
        Some(vec!["interp".into(), "create".into()])
    );
    // Only one whole bracketed substitution is a call.
    assert_eq!(elements("interp create child"), None);
}

#[test]
fn interp_create_value_words_reject_an_unmatched_brace_issue_1025() {
    // A malformed (mid-edit) path is not a list at all, so the whole
    // substitution is rejected. Returning the prefix parsed before the
    // error instead would read as a bare `interp create`, binding the
    // variable to an auto-named interpreter real Tcl never creates.
    // Later `$i eval` / `interp alias $i …` sites then resolve against
    // that phantom.
    assert_eq!(elements("[interp create {child]"), None);
    assert_eq!(
        elements("[interp create good {child]"),
        None,
        "a valid prefix does not rescue a malformed tail",
    );
}

// `interp create` — the registry's `InterpreterTransition::Create`,
// whose option scan mirrors tclInterp.c (`create_reads_the_c_option_scan`
// in `tcl-registry`'s `interp.rs` pins every row against tclsh 8.4–9.1).

/// Dispatch `interp create WORDS…` and report whether `path` was
/// recorded, and as safe.
fn created_by(words: &[&str], path: &str) -> Option<bool> {
    let mut a = Analyser::new();
    let call: Vec<&str> = ["interp", "create"]
        .into_iter()
        .chain(words.iter().copied())
        .collect();
    dispatch_words(&mut a, &call);
    let key = a.qualified_interp_key(path);
    a.interpreters.get(&key).map(|state| state.safe)
}

#[test]
fn interp_create_flags_are_still_scanned_after_the_path() {
    // C Tcl examines every word and treats a `-` word as a flag until
    // `--`, whether or not the path has been read. tclsh8.6/9.0:
    // `interp create x -safe` yields `interp issafe x` == 1. Stopping at
    // the path recorded `x` as an *unsafe* interpreter.
    assert_eq!(created_by(&["x", "-safe"], "x"), Some(true));
    assert_eq!(created_by(&["-safe", "y"], "y"), Some(true));
    // `-safe` matches by unique prefix.
    assert_eq!(created_by(&["-s", "x"], "x"), Some(true));
}

#[test]
fn interp_create_double_dash_ends_flag_parsing() {
    // tclsh8.6/9.0: `interp create -safe -- z` creates a safe `z`, and
    // `interp create -- -safe` creates an *unsafe* interpreter whose
    // path is the literal `-safe`.
    assert_eq!(created_by(&["-safe", "--", "z"], "z"), Some(true));
    assert_eq!(created_by(&["--", "-safe"], "-safe"), Some(false));
}

#[test]
fn interp_create_two_path_words_record_no_path() {
    // tclsh8.6/9.0: `interp create a b` is `wrong # args`, and `interp
    // create n -bogus` a bad option. No interpreter is created, so
    // recording either word would bind a name that does not exist.
    assert_eq!(created_by(&["a", "b"], "a"), None);
    assert_eq!(created_by(&["a", "b"], "b"), None);
    assert_eq!(created_by(&["n", "-bogus"], "n"), None);
    assert_eq!(
        created_by(&["--", "x", "-safe"], "x"),
        None,
        "after `--` a second `-` word is a path word, so this is the error shape too",
    );
}

#[test]
fn interp_create_bare_and_flag_only_forms_are_unchanged() {
    // TN control: a named child is recorded; an auto-named one has no
    // name this file can reference, so nothing is.
    assert_eq!(created_by(&["child"], "child"), Some(false));
    let mut a = Analyser::new();
    dispatch_words(&mut a, &["interp", "create", "-safe"]);
    assert!(a.interpreters.is_empty());
    // A computed path makes interpreter existence unknowable.
    let mut a = Analyser::new();
    dispatch_words(&mut a, &["interp", "create", "$name"]);
    assert!(a.interpreters.is_empty());
    assert!(a.dynamic_interp_ops);
}

// `set`'s binding through the generic dispatch tail — the role binding
// and `bind_value_word_assignment`; `set` has no analyser hook.

/// Dispatch one command whose words carry the given tokens and
/// single-token flags, through `process_command` exactly as a walked
/// command reaches it.
fn dispatch_tokens(a: &mut Analyser, words: &[(&str, Token, bool)]) {
    let owned: Vec<String> = words.iter().map(|(w, _, _)| (*w).to_string()).collect();
    let toks: Vec<Token> = words.iter().map(|&(_, tok, _)| tok).collect();
    let single: Vec<bool> = words.iter().map(|&(_, _, single)| single).collect();
    let expanded = vec![false; owned.len()];
    a.process_command(&owned, &toks, &single, &expanded, &[]);
}

/// Analyse `source` as Tcl 8.6 and keep the walk's state: the handlers
/// read each word from the analysed source, which a bare token list lacks.
fn analysed(source: &str) -> Analyser {
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl8.6");
    analyser
}

/// A document that does not parse is walked from the recovery
/// segmenter's stream, which no interpretation reaches; a procedure in
/// the recovered tail is still declared. tclsh runs none of it (`missing
/// close-bracket`), so this is an outline fact, not an execution one.
#[test]
fn a_procedure_after_an_unterminated_bracket_is_still_declared() {
    for dialect in ["tcl8.6", "tcl"] {
        let mut analyser = Analyser::new();
        let result = analyser.analyse("set x [foo\nproc recovered {} {}\n", dialect);
        assert!(
            result.all_procs.contains_key("::recovered"),
            "{dialect}: {:?}",
            result.all_procs.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn handle_set_defines_variable() {
    let a = analysed("set x 1\n");
    assert!(a.result.global_scope.variables.contains_key("x"));
    assert!(a.result.global_scope.variables["x"].warn_if_unused);
}

#[test]
fn handle_set_tracks_single_token_literal_value() {
    let a = analysed("set x hello\n");
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        Some("hello")
    );
}

#[test]
fn handle_set_tracks_braced_string_value() {
    let a = analysed("set x {hello world}\n");
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        Some("hello world")
    );
}

#[test]
fn handle_set_clears_const_string_for_interpolated_value() {
    // Re-assigning a value no literal spells clears the constant.
    let a = analysed("set x old\nset x [gets stdin]\n");
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        None
    );
}

#[test]
fn handle_set_no_value_records_read_not_definition() {
    // ``set x`` (one-arg form) is a *read*, not a definition — Tcl
    // returns the current value of ``x`` — recorded by the walk's
    // `VarRead`-role reference pass, with no const-string tracking.
    let src = "set x 1\nset x\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let read = u32::try_from(src.rfind('x').expect("the read")).expect("an offset");
    assert_eq!(
        r.global_scope.variables["x"].references,
        vec![span(read, read + 1)],
    );
    let mut read_only = Analyser::new();
    dispatch_tokens(
        &mut read_only,
        &[
            ("set", esc_tok(span(0, 3)), true),
            ("x", esc_tok(span(4, 5)), true),
        ],
    );
    assert_eq!(
        read_only
            .const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        None
    );
}

/// A literal value word is recorded as Tcl reads it: the
/// `CellWrite` evaluation cooks a bare or quoted token, so `set p
/// "a\\d"` holds `a\d` — tclsh 8.4.20 to 9.1b0 print `a\d`.
#[test]
fn a_literal_value_word_is_recorded_as_tcl_reads_it() {
    let a = analysed("set p \"a\\\\d\"\n");
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("p"))
            .map(|(value, _)| value.as_str()),
        Some("a\\d")
    );
}

/// A computed target defines no variable: `set $n 1` writes
/// the variable `n` names — tclsh 8.4.20 to 9.1b0 leave `n` as it was
/// — so the role binding's static-name rule binds nothing.
#[test]
fn a_computed_set_target_defines_no_variable() {
    let r = Analyser::new().analyse("set $n 1\n", "tcl8.6");
    assert!(!r.global_scope.variables.contains_key("n"));
}

#[test]
fn handle_set_no_value_undefined_var_is_silent() {
    // ``set x`` on an undefined variable is still a read; the read
    // recording silently no-ops when the name isn't in scope, so no
    // spurious binding lands.
    let r = Analyser::new().analyse("set x\n", "tcl8.6");
    assert!(!r.global_scope.variables.contains_key("x"));
}

// Scope aliases through the generic consumer — the dispatch tail's
// `apply_state_transitions`, reached by `process_command` exactly as a
// walked command reaches it.

/// Dispatch one command whose words are bare (`Esc`) tokens four bytes
/// apart — a word carrying `$` or `[` reads as computed, as it would in
/// source.
fn dispatch_words(a: &mut Analyser, words: &[&str]) {
    let owned: Vec<String> = words.iter().map(|s| (*s).to_string()).collect();
    let toks: Vec<Token> = (0..owned.len())
        .map(|i| {
            let at = u32::try_from(i).unwrap() * 4;
            esc_tok(span(at, at + 1))
        })
        .collect();
    let single = vec![true; owned.len()];
    let expanded = vec![false; owned.len()];
    a.process_command(&owned, &toks, &single, &expanded, &[]);
}

#[test]
fn handle_global_defines_each_name() {
    let a = analysed("global x y z\n");
    for name in ["x", "y", "z"] {
        assert!(a.result.global_scope.variables.contains_key(name));
        assert!(!a.result.global_scope.variables[name].warn_if_unused);
        assert_eq!(
            a.result.global_scope.variables[name].link_target.as_deref(),
            Some(format!("::{name}").as_str()),
        );
    }
}

#[test]
fn global_binds_the_tail_of_a_qualified_name_and_skips_a_computed_one() {
    let src = "namespace eval ns {variable v 1}\nset dyn other\nglobal ::ns::v $dyn\n";
    let a = analysed(src);
    let v = &a.result.global_scope.variables["v"];
    assert_eq!(v.link_target.as_deref(), Some("::ns::v"));
    // The declaration word names the cell, so a rename rewrites it.
    let word = u32::try_from(src.find("::ns::v").unwrap()).unwrap();
    assert_eq!(v.link_target_span, Some(span(word, word + 7)));
    assert!(!a.result.global_scope.variables.contains_key("other"));
    assert!(!a.result.global_scope.variables.contains_key("$dyn"));
}

#[test]
fn handle_variable_defines_only_names_skipping_values() {
    // `variable x 1 y 2 z` — names at 0, 2, 4; values at 1, 3.
    let a = analysed("variable x 1 y 2 z\n");
    for name in ["x", "y", "z"] {
        assert!(a.result.global_scope.variables.contains_key(name));
    }
    // Numbers should NOT be variable names.
    assert!(!a.result.global_scope.variables.contains_key("1"));
    assert!(!a.result.global_scope.variables.contains_key("2"));
}

#[test]
fn handle_variable_single_name_no_value() {
    let a = analysed("variable x\n");
    assert!(a.result.global_scope.variables.contains_key("x"));
    // At the top level the current namespace is the global one.
    assert_eq!(
        a.result.global_scope.variables["x"].link_target.as_deref(),
        Some("::x"),
    );
}

#[test]
fn declaration_aliases_use_selected_native_name_owner() {
    for version in tcl_dialect::TclVersion::ALL {
        let mut analyser = Analyser::new();
        analyser.source_analysis_entry = Some(std::sync::Arc::new(
            crate::command_binding::SourceAnalysisEntry {
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(version)),
                ..Default::default()
            },
        ));
        analyser.handle_global_command(&["ns:::v".to_owned()], &[esc_tok(span(0, 6))], &[]);
        assert!(analyser.result.global_scope.variables.contains_key("v"));
        assert!(!analyser.result.global_scope.variables.contains_key(":v"));
        assert_eq!(
            analyser.result.global_scope.variables["v"]
                .link_target
                .as_deref(),
            Some("::ns::v"),
        );
    }
    let mut analyser = Analyser::new();
    analyser.source_analysis_entry = Some(std::sync::Arc::new(
        crate::command_binding::SourceAnalysisEntry {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
            )),
            ..Default::default()
        },
    ));
    analyser.handle_global_command(&["::ns::v".to_owned()], &[esc_tok(span(0, 7))], &[]);
    assert!(!analyser.result.global_scope.variables.contains_key("v"));
    analyser.handle_variable_command(&["::ns:::v".to_owned()], &[esc_tok(span(0, 8))], &[]);
    assert!(analyser.result.global_scope.variables.contains_key("v"));
}

// handle_upvar_command — the `otherVar` link

#[test]
fn namespace_upvar_links_each_local_to_its_namespace_cell() {
    // `namespace upvar ns other local` — `other` resolved within `ns`
    // and kept whole (`b::c` is `::a::b::c`, not the tail-collapsed
    // `::a::c`), `ns` relative to the current namespace; the rename span
    // is the `other` word, not the local.
    let src = "namespace eval a {namespace eval b {variable c 1}; variable d 2}\n\
                   namespace upvar ::a b::c l d m\n";
    let a = analysed(src);
    let l = &a.result.global_scope.variables["l"];
    assert_eq!(l.link_target.as_deref(), Some("::a::b::c"));
    let other = u32::try_from(src.find("b::c l").unwrap()).unwrap();
    assert_eq!(l.link_target_span, Some(span(other, other + 4)));
    assert_eq!(
        a.result.global_scope.variables["m"].link_target.as_deref(),
        Some("::a::d"),
    );
}

#[test]
fn namespace_upvar_with_a_computed_word_keeps_the_cell_as_written() {
    // A computed local binds nothing.
    let mut a = Analyser::new();
    dispatch_words(&mut a, &["namespace", "upvar", "::a", "x", "$local"]);
    assert!(!a.result.global_scope.variables.contains_key("local"));
    // Legacy alias reports preserve computed namespace/otherVar spellings.
    // Core candidates and rename coverage use typed source receipts instead.
    let mut a = Analyser::new();
    dispatch_words(&mut a, &["namespace", "upvar", "$ns", "x", "l"]);
    assert_eq!(
        a.result.global_scope.variables["l"].link_target.as_deref(),
        Some("::$ns::x"),
    );
    // The cell is named by the `otherVar` word, not the local.
    assert_eq!(
        a.result.global_scope.variables["l"].link_target_span,
        Some(span(12, 13)),
    );
    let mut a = Analyser::new();
    dispatch_words(&mut a, &["namespace", "upvar", "::mypkg", "$v", "l"]);
    assert_eq!(
        a.result.global_scope.variables["l"].link_target.as_deref(),
        Some("::mypkg::$v"),
    );
    // An absolute `otherVar` names its cell whatever the namespace word.
    let mut a = Analyser::new();
    dispatch_words(&mut a, &["namespace", "upvar", "$ns", "::abs", "l"]);
    assert_eq!(
        a.result.global_scope.variables["l"].link_target.as_deref(),
        Some("::abs"),
    );
}

#[test]
fn variable_links_keep_a_colon_named_namespace_and_collapse_written_runs() {
    // tclsh 8.6/9.0/9.1: with `set ::v glob; namespace eval : {variable v colon}`,
    // `proc a {} {namespace upvar : v l; namespace upvar ::: v m; global :::v; list $l $m $v}`
    // returns `colon glob glob`; `namespace which -variable v` in `:` is `:::::v`.
    // `upvar 1 :::x y; upvar #0 a:::b z` binds `::x` and `::a::b`.
    let a = analysed(
        "namespace upvar : v l\nnamespace upvar ::: v m\nglobal :::g\nupvar #0 :::x y\nupvar #0 a:::b z\n",
    );
    let variables = &a.result.global_scope.variables;
    let link = |name: &str| variables[name].link_target.clone();
    assert_eq!(link("l").as_deref(), Some(":::::v"));
    assert_eq!(link("m").as_deref(), Some("::v"));
    assert_eq!(link("g").as_deref(), Some("::g"));
    assert_eq!(link("y").as_deref(), Some("::x"));
    assert_eq!(link("z").as_deref(), Some("::a::b"));
}

// `upvar` — the `otherVar` link

/// The link target of `local` in the first procedure body of `src`,
/// analysed under Tcl 8.6.
fn proc_alias_link(src: &str, local: &str) -> Option<String> {
    let a = analysed(src);
    a.result
        .global_scope
        .children
        .iter()
        .find(|scope| scope.kind == crate::analyser::types::ScopeKind::Proc)
        .and_then(|scope| scope.variables.get(local))
        .and_then(|v| v.link_target.clone())
}

/// Dispatch `upvar <args>` and report the alias's link target, if it got
/// one.
fn upvar_link_of(args: &[&str], local: &str) -> Option<String> {
    let mut a = Analyser::new();
    let words: Vec<&str> = std::iter::once("upvar")
        .chain(args.iter().copied())
        .collect();
    dispatch_words(&mut a, &words);
    a.result
        .global_scope
        .variables
        .get(local)
        .and_then(|v| v.link_target.clone())
}

#[test]
fn upvar_links_a_fully_qualified_other_var_at_any_level() {
    // `upvar 1 ::myns::q g` binds `::myns::q` from any call depth
    // (tclsh 9.0.4 / 8.6.14: reached identically through `upvar 1` and
    // `upvar 2`), so the alias has a stable cell to link to.
    for src in [
        "namespace eval myns {variable q 1}\nproc p {} {upvar 1 ::myns::q g; return $g}\np\n",
        "namespace eval myns {variable q 1}\nproc p {} {upvar 2 ::myns::q g; return $g}\n\
             proc outer {} {p}\nouter\n",
    ] {
        assert_eq!(
            proc_alias_link(src, "g"),
            Some("::myns::q".to_string()),
            "{src}"
        );
    }
}

#[test]
fn upvar_links_an_array_other_var_to_its_base() {
    // tk.tcl:177 `upvar ::tk::FocusGrab($index) data` — the element key
    // is runtime data, the array is the named cell every sibling access
    // shares.
    assert_eq!(
        upvar_link_of(&["::tk::FocusGrab($index)", "data"], "data"),
        Some("::tk::FocusGrab".to_string()),
    );
}

#[test]
fn logical_upvar_targets_share_literal_scalar_and_element_geometry() {
    // naming.compiler.logical-upvar-target-element-geometry
    // docs/design/analysis/name-resolution-proofs/logical-upvar-target-element-geometry.md
    // These are readonly Logical source coordinates, not executed links.
    // Literal name parsing must preserve a scalar's unmatched parenthesis
    // and must not strip substitution sigils before the dynamic refusal.
    for (other, expected) in [
        ("a(b", Some("::a(b")),
        ("a(b)c", Some("::a(b)c")),
        ("a(b)", Some("::a")),
        ("π(β", Some("::π(β")),
        ("π(β)", Some("::π")),
        ("a($key)", Some("::a")),
        ("::ns::a(b", Some("::ns::a(b")),
        ("$computed", None),
    ] {
        let source = format!("upvar #0 {{{other}}} local");
        let analysis = Analyser::new().analyse(&source, "tcl");
        assert!(analysis.allows_retained_logical_declaration_advice());
        assert_eq!(
            analysis.global_scope.variables["local"]
                .link_target
                .as_deref(),
            expected,
            "{source}",
        );
    }
}

#[test]
fn logical_alias_source_declarations_keep_typed_targets_and_unknown_obligations() {
    let source = "upvar #0 shared local; puts $local";
    let analysis = Analyser::new().analyse(source, "tcl");
    assert_eq!(
        analysis.global_scope.variables["local"]
            .link_target
            .as_deref(),
        Some("::shared")
    );
    let span = tcl_lexer::Span::new(9, 15);
    let target = analysis
        .original_variable_symbols
        .iter()
        .find(|site| site.span() == span)
        .expect("actual original target occurrence");
    assert!(target.is_declaration());
    assert_eq!(target.original_name_input().bytes(), b"shared");
    assert_eq!(
        target.symbol().checked_c_namespace_key().as_deref(),
        Some("::shared")
    );
    assert_eq!(
        analysis.original_variable_alias_receipts,
        [] as [crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt; 0]
    );
    assert_eq!(
        analysis.original_variable_alias_operands,
        [] as [(
            u32,
            tcl_lexer::Span,
            tcl_lexer::Span,
            crate::registry_invocation::DeclarationVariableAliasPurpose
        ); 0]
    );

    let source = "upvar #0 $unknown local";
    let analysis = Analyser::new().analyse(source, "tcl");
    assert!(
        analysis
            .global_scope
            .variables
            .get("local")
            .is_none_or(|variable| variable.link_target.is_none())
    );
    assert_ne!(
        analysis.original_variable_name_unknowns,
        [] as [tcl_lexer::Span; 0]
    );
    assert_eq!(
        analysis.original_variable_alias_receipts,
        [] as [crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt; 0]
    );
}

#[test]
fn upvar_at_level_hash_zero_links_to_the_global_cell() {
    // tclsh 9.0.4 / 8.6.14: `upvar #0 counter c` reaches `::counter`
    // however deep the stack is.
    assert_eq!(
        proc_alias_link("proc p {} {upvar #0 counter c; set c 1}\np\n", "c"),
        Some("::counter".to_string()),
    );
}

#[test]
fn upvar_does_not_link_a_frame_relative_other_var() {
    // TN — `upvar 1 x y` names whatever local the *caller* happens to
    // have; there is no namespace path for it, so inventing one would
    // unify unrelated cells.
    assert_eq!(upvar_link_of(&["1", "x", "y"], "y"), None);
    assert_eq!(upvar_link_of(&["x", "y"], "y"), None);
    // A computed target has no readable name at all.
    assert_eq!(upvar_link_of(&["1", "$name", "obj"], "obj"), None);
    // A computed *level* could be `#0`, but could equally be `1` — the
    // alias must not be linked on a guess.
    assert_eq!(upvar_link_of(&["$lvl", "counter", "c"], "c"), None);
}

#[test]
fn upvar_level_word_presence_follows_argument_count_parity() {
    // `upvar 1 b` is TWO words, so there is no level word and `1` is the
    // caller-side variable name — the alias is `b` (tclsh 9.0.4 /
    // 8.6.14: with `set 1 ONE` in the caller, the callee reads `ONE`).
    let a = analysed("proc p {} {upvar 1 b; return $b}\nset 1 ONE\np\n");
    let body = &a.result.global_scope.children[0];
    assert!(body.variables.contains_key("b"));
    assert!(!body.variables.contains_key("1"));
}

// handle_proc_command

#[test]
fn handle_proc_command_records_every_declaration_site_even_when_shadowed() {
    // TP: a proc declared twice
    // verbatim at different spans (plain Tcl's own "last redefinition
    // wins" semantics) must still leave BOTH declarations' own name
    // spans in `proc_declaration_sites`, even though `all_procs`
    // (keyed by qualified name) only ever retains the winner's.
    let mut a = Analyser::new();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), "return ONE".to_string()],
        &[
            esc_tok(span(5, 8)),
            esc_tok(span(9, 9)),
            str_tok(span(10, 20)),
        ],
        &[],
        &[],
    );
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), "return TWO".to_string()],
        &[
            esc_tok(span(30, 33)),
            esc_tok(span(34, 34)),
            str_tok(span(35, 45)),
        ],
        &[],
        &[],
    );
    assert_eq!(a.result.all_procs["::foo"].body_span, span(35, 45));
    let sites: Vec<(&str, tcl_lexer::Span)> = a
        .result
        .proc_declaration_sites
        .iter()
        .map(|(q, s)| (q.as_str(), *s))
        .collect();
    assert_eq!(
        sites,
        vec![("::foo", span(5, 8)), ("::foo", span(30, 33))],
        "both declaration sites must be recorded, in source order"
    );
}

#[test]
fn handle_proc_records_proc_at_global() {
    let mut a = Analyser::new();
    let handled = a.handle_proc_command(
        "proc",
        &["foo".to_string(), "a b".to_string(), "set x $a".to_string()],
        &[
            esc_tok(span(5, 8)),
            esc_tok(span(9, 14)),
            str_tok(span(15, 25)),
        ],
        &[],
        &[],
    );
    assert!(handled);
    assert!(a.result.all_procs.contains_key("::foo"));
    let proc = &a.result.all_procs["::foo"];
    assert_eq!(proc.name, "foo");
    assert_eq!(proc.qualified_name, "::foo");
    assert_eq!(proc.params.len(), 2);
    assert_eq!(proc.params[0].name, "a");
    assert_eq!(proc.params[1].name, "b");
}

#[test]
fn handle_proc_qualifies_under_namespace() {
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "ns1"));
    let handled = a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[0],
    );
    assert!(handled);
    assert!(a.result.all_procs.contains_key("::ns1::foo"));
}

#[test]
fn handle_proc_keys_scope_procs_by_simple_name() {
    // ``scope.procs`` is keyed by the simple name so per-scope
    // lookups and shadowing rules work locally. The
    // qualified name lives on ``ProcDef.qualified_name`` and
    // is the key for ``result.all_procs``.
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "ns1"));
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[0],
    );
    let scope = &a.result.global_scope.children[0];
    // Scope's procs map keyed by simple name, NOT qualified.
    assert!(
        scope.procs.contains_key("foo"),
        "scope.procs should be keyed by simple `foo`, got keys: {:?}",
        scope.procs.keys().collect::<Vec<_>>(),
    );
    assert!(
        !scope.procs.contains_key("::ns1::foo"),
        "scope.procs must not be keyed by qualified name",
    );
    // The qualified name is still on the ProcDef.
    assert_eq!(scope.procs["foo"].qualified_name, "::ns1::foo");
    // ...and result.all_procs is keyed by qualified name.
    assert!(a.result.all_procs.contains_key("::ns1::foo"));
}

#[test]
fn handle_proc_absolute_name_rebases() {
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "outer"));
    let handled = a.handle_proc_command(
        "proc",
        &["::other::foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 17)),
            str_tok(span(18, 20)),
            str_tok(span(21, 23)),
        ],
        &[],
        &[0],
    );
    assert!(handled);
    // Absolute name rebases — does NOT nest under outer.
    assert!(a.result.all_procs.contains_key("::other::foo"));
    assert!(!a.result.all_procs.contains_key("::outer::other::foo"));
}

#[test]
fn handle_proc_consumes_last_comment_as_doc() {
    let mut a = Analyser::new();
    a.last_comment = "doc string".to_string();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(0, 3)),
            str_tok(span(4, 6)),
            str_tok(span(7, 9)),
        ],
        &[],
        &[],
    );
    assert_eq!(a.result.all_procs["::foo"].doc, "doc string");
    // last_comment is consumed.
    assert_eq!(a.last_comment, "");
}

#[test]
fn handle_proc_too_few_args_returns_false() {
    let mut a = Analyser::new();
    let handled = a.handle_proc_command(
        "proc",
        &["foo".to_string()],
        &[esc_tok(span(0, 3))],
        &[],
        &[],
    );
    assert!(!handled);
    assert_eq!(a.result.all_procs.len(), 0);
}

// handle_proc_command W113 shadow check

#[test]
fn handle_proc_emits_w113_for_builtin_shadow() {
    // ``proc set {} {}`` — the proc name is a built-in.
    // W113 should anchor at the proc-name span and carry
    // the canonical message shape. A *real* dialect is named in the
    // parenthetical label; the permissive fallback (both the empty
    // string and its canonical "tcl" spelling resolve there —
    // dialect-profile-model.md §8, one sink) carries no label, covered
    // by the sibling no-label test below.
    let mut a = Analyser::new();
    a.profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    a.handle_proc_command(
        "proc",
        &["set".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    let w113s: Vec<&crate::analyser::types::Diagnostic> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W113)
        .collect();
    assert_eq!(w113s.len(), 1);
    assert_eq!(w113s[0].span, span(5, 8));
    assert!(w113s[0].message.contains("'set' shadows built-in"));
    assert!(w113s[0].message.contains("(tcl8.6)"));
    assert_eq!(w113s[0].severity, crate::analyser::types::Severity::Warning);
}

#[test]
fn irules_profile_without_attached_registry_keeps_valid_proc_semantics() {
    let mut analyser = Analyser::new();
    analyser.profile = tcl_dialect::DialectProfile::irules();
    analyser.source = "proc set {} {}".to_owned().into();
    assert!(analyser.registry.is_none());
    let handled = analyser.handle_proc_command(
        "proc",
        &["set".to_owned(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[true, true, true],
        &[],
    );
    assert!(handled);
    assert!(analyser.result.all_procs.contains_key("::set"));
    assert!(
        analyser
            .result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W113)
    );
}

#[test]
fn irules_profile_without_attached_registry_rejects_malformed_proc() {
    let mut analyser = Analyser::new();
    analyser.profile = tcl_dialect::DialectProfile::irules();
    assert!(analyser.registry.is_none());
    let handled = analyser.handle_proc_command(
        "proc",
        &[
            "set".to_owned(),
            String::new(),
            "set leaked 1".to_owned(),
            "extra".to_owned(),
        ],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 26)),
            esc_tok(span(27, 32)),
        ],
        &[],
        &[],
    );
    assert!(handled);
    assert_eq!(analyser.result.all_procs.len(), 0);
    assert_eq!(analyser.result.global_scope.variables.len(), 0);
    assert_eq!(
        analyser.result.diagnostics,
        [] as [crate::analyser::types::Diagnostic; 0]
    );
}

#[test]
fn handle_proc_no_w113_for_non_builtin_name() {
    // ``foo`` is not a built-in — no W113.
    let mut a = Analyser::new();
    a.profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W113),
        "should NOT emit W113 for non-built-in name 'foo'",
    );
}

#[test]
fn handle_proc_w113_matches_qualified_form() {
    // ``proc ::set {} {}`` — qualified form also shadows
    // ``set`` because the registry indexes by bare command
    // name (``::`` is trimmed at lookup).
    let mut a = Analyser::new();
    a.profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    a.handle_proc_command(
        "proc",
        &["::set".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 10)),
            str_tok(span(11, 13)),
            str_tok(span(14, 16)),
        ],
        &[],
        &[],
    );
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W113)
    );
}

#[test]
fn handle_proc_w113_no_dialect_label_when_dialect_empty() {
    // Empty dialect → no parenthetical label in the message.
    let mut a = Analyser::new();
    // dialect intentionally left empty
    a.handle_proc_command(
        "proc",
        &["set".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    let w113 = a
        .result
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W113)
        .expect("W113 expected");
    assert!(w113.message.contains("'set' shadows built-in"));
    assert!(!w113.message.contains('('), "no dialect label expected");
}

#[test]
fn handle_proc_w113_dialect_specific_command_only_shadows_in_that_dialect() {
    // ``pool`` is an *unqualified* iRules-specific command; under the
    // ``f5-irules`` dialect a proc named ``pool`` shadows a built-in,
    // but under plain ``tcl`` it does not.  (A *namespace-qualified*
    // iRules command like ``HTTP::respond`` is never flagged, because a
    // qualified match is a library/package command living in its own
    // namespace, not a core-global shadow.)
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::irules();
    a.source = "proc pool {} {}".to_owned().into();
    a.handle_proc_command(
        "proc",
        &["pool".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 9)),
            str_tok(span(10, 12)),
            str_tok(span(13, 15)),
        ],
        &[true, true, true],
        &[],
    );
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W113),
        "f5-irules dialect should treat pool as built-in",
    );

    // Same proc, plain tcl dialect → no W113.
    let mut b = Analyser::new();
    b.profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    b.handle_proc_command(
        "proc",
        &["pool".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 9)),
            str_tok(span(10, 12)),
            str_tok(span(13, 15)),
        ],
        &[],
        &[],
    );
    assert!(
        !b.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W113),
        "plain tcl dialect should NOT flag pool",
    );
}

#[test]
fn handle_proc_w113_silent_on_namespace_qualified_shadow() {
    // FP: a namespace-qualified match (`HTTP::respond` under f5-irules)
    // is a library/package command in its own namespace, not a
    // core-global shadow — never W113.
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::irules();
    a.handle_proc_command(
        "proc",
        &["HTTP::respond".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 18)),
            str_tok(span(19, 21)),
            str_tok(span(22, 24)),
        ],
        &[],
        &[],
    );
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W113),
        "namespace-qualified HTTP::respond must not fire W113",
    );
}

// handle_proc_command body recursion

#[test]
fn handle_proc_creates_proc_scope_for_braced_body() {
    // ``proc foo {} {}`` — empty braced body still opens a
    // proc scope so subsequent body-walking handlers have a
    // place to record locals.
    let mut a = Analyser::new();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    assert_eq!(a.result.global_scope.children.len(), 1);
    let proc_scope = &a.result.global_scope.children[0];
    assert_eq!(proc_scope.kind, crate::analyser::types::ScopeKind::Proc);
    assert_eq!(proc_scope.name, "foo");
    assert_eq!(proc_scope.body_span, Some(span(12, 14)));
}

#[test]
fn handle_proc_defines_params_in_proc_scope() {
    // ``proc foo {a b} {}`` — a, b become locals in the
    // proc scope, not in the outer scope.
    let mut a = Analyser::new();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), "a b".to_string(), String::new()],
        &[
            esc_tok(span(5, 8)),
            esc_tok(span(9, 14)),
            str_tok(span(15, 17)),
        ],
        &[],
        &[],
    );
    let proc_scope = &a.result.global_scope.children[0];
    assert!(proc_scope.variables.contains_key("a"));
    assert!(proc_scope.variables.contains_key("b"));
    // Outer scope must be untouched.
    assert!(!a.result.global_scope.variables.contains_key("a"));
}

#[test]
fn handle_proc_walks_body_set_defines_local() {
    // The body walk dispatches `set x 1` against the procedure scope, so
    // the local lands there and not in the global scope.
    let a = analysed("proc foo {} {set x 1}\n");
    let proc_scope = &a.result.global_scope.children[0];
    assert!(
        proc_scope.variables.contains_key("x"),
        "body walk should land 'x' in proc scope; vars: {:?}",
        proc_scope.variables.keys().collect::<Vec<_>>()
    );
    assert!(!a.result.global_scope.variables.contains_key("x"));
}

#[test]
fn handle_proc_walks_body_global_falls_into_proc_scope() {
    // `global a b` inside a procedure body binds both names in the
    // procedure scope, so the body's later reads and writes resolve.
    let a = analysed("proc foo {} {global a b}\n");
    let proc_scope = &a.result.global_scope.children[0];
    assert!(proc_scope.variables.contains_key("a"));
    assert!(proc_scope.variables.contains_key("b"));
}

#[test]
fn handle_proc_nested_proc_creates_nested_scopes() {
    // The body walk dispatches `proc` inside the body, creating a nested
    // procedure scope under the outer one.
    let a = analysed("proc outer {} {proc inner {} {}}\n");
    // Outer proc registered.
    assert!(a.result.all_procs.contains_key("::outer"));
    // A nested definition homes to the enclosing proc's *defining*
    // namespace (`command_resolution_namespace` /
    // `advance_command_resolution_namespace`), not to a namespace named
    // after the proc. ``outer`` is unqualified, so its defining namespace
    // is ``::`` and the nested ``inner`` qualifies as ``::inner``.
    assert!(a.result.all_procs.contains_key("::inner"));
    // Outer's proc scope holds the nested proc scope as a child.
    let outer_scope = &a.result.global_scope.children[0];
    assert_ne!(
        outer_scope.children,
        [] as [crate::analyser::types::Scope; 0]
    );
    assert_eq!(
        outer_scope.children[0].kind,
        crate::analyser::types::ScopeKind::Proc,
    );
    assert_eq!(outer_scope.children[0].name, "inner");
}

#[test]
fn handle_proc_dynamic_body_skips_walk() {
    // ``proc foo {} $body`` — the body is a Var token, not a
    // Str token; we cannot statically re-segment a dynamic
    // body, so the body walk is skipped. The proc record
    // itself still lands so downstream signature consumers see
    // the proc; only the inner walk is gated.
    let mut a = Analyser::new();
    let var_tok = Token::new(TokenType::Var, span(13, 18));
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), "$body".to_string()],
        &[esc_tok(span(5, 8)), str_tok(span(9, 11)), var_tok],
        &[],
        &[],
    );
    assert!(a.result.all_procs.contains_key("::foo"));
    // No proc scope opened — Str gate failed.
    assert_eq!(
        a.result.global_scope.children,
        [] as [crate::analyser::types::Scope; 0]
    );
}

#[test]
fn handle_proc_body_walk_increments_body_depth_temporarily() {
    // ``body_depth`` is bumped for the duration of the body
    // walk and restored on exit — top-level-only command
    // checks rely on the depth being zero outside any body.
    let mut a = Analyser::new();
    assert_eq!(a.body_depth, 0);
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    assert_eq!(a.body_depth, 0);
}

#[test]
fn handle_proc_body_walk_does_not_leak_inner_doc_comment() {
    // A trailing comment inside the body should not bleed into
    // ``last_comment`` for whatever follows the proc at the
    // outer scope. The outer comment ("doc string") is consumed
    // as the proc's own doc; after the walk, ``last_comment``
    // is restored to empty.
    let mut a = Analyser::new();
    a.last_comment = "doc string".to_string();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    assert_eq!(a.result.all_procs["::foo"].doc, "doc string");
    assert_eq!(a.last_comment, "");
}

// handle_namespace_eval_command

// handle_namespace_export_command

#[test]
fn handle_namespace_export_records_pattern() {
    let mut a = Analyser::new();
    a.handle_namespace_export_command(
        &["export".to_string(), "bar".to_string()],
        &[esc_tok(span(0, 6)), esc_tok(span(7, 10))],
        &[],
    );
    assert_eq!(a.result.namespace_exports.len(), 1);
    assert_eq!(a.result.namespace_exports[0].ns, "::");
    assert_eq!(a.result.namespace_exports[0].pattern, "bar");
}

#[test]
fn handle_namespace_export_records_multiple_patterns() {
    let mut a = Analyser::new();
    a.handle_namespace_export_command(
        &["export".to_string(), "bar".to_string(), "b*".to_string()],
        &[
            esc_tok(span(0, 6)),
            esc_tok(span(7, 10)),
            esc_tok(span(11, 13)),
        ],
        &[],
    );
    let patterns: Vec<&str> = a
        .result
        .namespace_exports
        .iter()
        .map(|e| e.pattern.as_str())
        .collect();
    assert_eq!(patterns, vec!["bar", "b*"]);
}

/// `NamespaceExportCmd` calls `Tcl_Export` per pattern and returns on the
/// first failure, so a qualified pattern aborts the loop — the patterns
/// *before* it are committed, the ones after are never exported at all.
/// Oracle rows (identical on tclsh 8.6.16 and 9.0.4, `namespace export`
/// read back from the namespace afterwards):
///
/// | script | exports |
/// |---|---|
/// | `namespace export ::bad ok` | nothing |
/// | `namespace export ok ::bad` | `ok` |
/// | `namespace export one ::bad three` | `one` |
#[test]
fn handle_namespace_export_stops_at_the_first_qualified_pattern() {
    let recorded = |words: &[&str]| -> Vec<String> {
        let mut a = Analyser::new();
        let args: Vec<String> = std::iter::once("export".to_string())
            .chain(words.iter().map(|w| (*w).to_string()))
            .collect();
        let tokens: Vec<Token> = (0..args.len())
            .map(|i| {
                let start = u32::try_from(i).expect("small index") * 10;
                esc_tok(span(start, start + 5))
            })
            .collect();
        a.handle_namespace_export_command(&args, &tokens, &[]);
        a.result
            .namespace_exports
            .iter()
            .map(|e| e.pattern.clone())
            .collect()
    };

    // The bad pattern is first: nothing is exported.
    assert_eq!(recorded(&["::bad", "ok"]), [] as [std::string::String; 0]);
    // …and after a valid one: only the valid one survives.
    assert_eq!(recorded(&["ok", "::bad"]), vec!["ok".to_string()]);
    assert_eq!(
        recorded(&["one", "::bad", "three"]),
        vec!["one".to_string()]
    );
    // A *relative* qualifier is equally invalid (oracle: `namespace
    // export sub::pat` errors), so it aborts too.
    assert_eq!(
        recorded(&["sub::pat", "ok"]),
        [] as [std::string::String; 0]
    );
    // A lone colon is not a separator, so `a:b` is a perfectly valid
    // pattern and must not abort (oracle: exports `a:b`).
    assert_eq!(
        recorded(&["a:b", "ok"]),
        vec!["a:b".to_string(), "ok".to_string()]
    );
    // The empty pattern is valid too (oracle: exports `{} ok`).
    assert_eq!(recorded(&["", "ok"]), vec![String::new(), "ok".to_string()]);
}

#[test]
fn handle_namespace_export_clear_commits_before_an_invalid_pattern() {
    // C spends a whole `Tcl_Export(…, "::", 1)` call on `-clear`, which
    // resets the list before any pattern is looked at, so a later bad
    // pattern cannot undo it. Oracle: a namespace exporting `keep`, then
    // `namespace export -clear ::bad`, exports nothing afterwards.
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_namespace_export_command(
        &[
            "export".to_string(),
            "-clear".to_string(),
            "::bad".to_string(),
        ],
        &[
            esc_tok(span(0, 6)),
            esc_tok(span(7, 13)),
            esc_tok(span(14, 19)),
        ],
        &[],
    );
    let events: Vec<(&str, bool)> = a
        .result
        .namespace_exports
        .iter()
        .map(|e| (e.pattern.as_str(), e.clears))
        .collect();
    assert_eq!(events, vec![("", true)]);
}

#[test]
fn handle_namespace_export_clear_records_an_ordered_tombstone() {
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_namespace_export_command(
        &["export".to_string(), "bar".to_string()],
        &[esc_tok(span(0, 6)), esc_tok(span(7, 10))],
        &[],
    );
    a.handle_namespace_export_command(
        &[
            "export".to_string(),
            "-clear".to_string(),
            "baz".to_string(),
        ],
        &[
            esc_tok(span(11, 17)),
            esc_tok(span(18, 24)),
            esc_tok(span(25, 28)),
        ],
        &[],
    );
    // `-clear` is an ordered event, not an eager delete: the earlier
    // `bar` entry stays on the log so a `namespace import` that ran
    // *before* the `-clear` can still see it.
    let events: Vec<(&str, bool, u32)> = a
        .result
        .namespace_exports
        .iter()
        .map(|e| (e.pattern.as_str(), e.clears, e.range.start()))
        .collect();
    assert_eq!(
        events,
        vec![("bar", false, 7), ("", true, 18), ("baz", false, 25)]
    );
}

#[test]
fn handle_namespace_export_consumes_only_one_clear_flag() {
    // `NamespaceExportCmd` compares `objv[1]`
    // against `-clear` once, so a *second* `-clear` is an ordinary export
    // pattern. Oracle (tclsh 8.6.14 / 9.0.4): `namespace export -clear
    // -clear p` leaves exactly `-clear p` exported, and a command really
    // named `-clear` is then importable through `namespace import
    // ::src::*`. Consuming every matching word instead records two
    // tombstones and silently drops the `-clear` export.
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_namespace_export_command(
        &[
            "export".to_string(),
            "-clear".to_string(),
            "-clear".to_string(),
            "p".to_string(),
        ],
        &[
            esc_tok(span(0, 6)),
            esc_tok(span(7, 13)),
            esc_tok(span(14, 20)),
            esc_tok(span(21, 22)),
        ],
        &[],
    );
    let events: Vec<(&str, bool)> = a
        .result
        .namespace_exports
        .iter()
        .map(|e| (e.pattern.as_str(), e.clears))
        .collect();
    assert_eq!(events, vec![("", true), ("-clear", false), ("p", false)]);
}

#[test]
fn handle_namespace_export_flag_after_a_pattern_is_a_pattern() {
    // The flag is only ever the *first* word: oracle `namespace export a
    // -clear` → `namespace export` returns `a -clear`.
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_namespace_export_command(
        &["export".to_string(), "a".to_string(), "-clear".to_string()],
        &[
            esc_tok(span(0, 6)),
            esc_tok(span(7, 8)),
            esc_tok(span(9, 15)),
        ],
        &[],
    );
    let events: Vec<(&str, bool)> = a
        .result
        .namespace_exports
        .iter()
        .map(|e| (e.pattern.as_str(), e.clears))
        .collect();
    assert_eq!(events, vec![("a", false), ("-clear", false)]);
}

#[test]
fn handle_namespace_import_consumes_only_one_force_flag() {
    // Symmetric to the export case: `namespace import -force -force
    // ::src::*` reads the second `-force` as an import *pattern* (and
    // aborts with `no namespace specified in import pattern "-force"`,
    // tclsh 8.6.14/9.0.4). Only the first is skipped as a flag, so the
    // second is recorded as the pattern word it is — one whose empty
    // source namespace both wildcard resolvers already decline.
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_namespace_import_command(
        &[
            "import".to_string(),
            "-force".to_string(),
            "-force".to_string(),
            "::src::*".to_string(),
        ],
        &[
            esc_tok(span(0, 6)),
            esc_tok(span(7, 13)),
            esc_tok(span(14, 20)),
            esc_tok(span(21, 29)),
        ],
        &[],
    );
    let patterns: Vec<&str> = a
        .result
        .namespace_imports
        .iter()
        .map(|i| i.pattern.as_str())
        .collect();
    assert_eq!(patterns, vec!["::-force", "::src::*"]);
}

#[test]
fn handle_namespace_export_bare_clear_records_a_tombstone() {
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_namespace_export_command(
        &["export".to_string(), "-clear".to_string()],
        &[esc_tok(span(0, 6)), esc_tok(span(7, 13))],
        &[],
    );
    assert_eq!(a.result.namespace_exports.len(), 1);
    assert!(a.result.namespace_exports[0].clears);
    assert_eq!(a.result.namespace_exports[0].pattern, "");
}

#[test]
fn handle_namespace_export_skips_dynamic_pattern() {
    let mut a = Analyser::new();
    a.handle_namespace_export_command(
        &["export".to_string(), "$dyn".to_string()],
        &[esc_tok(span(0, 6)), esc_tok(span(7, 11))],
        &[],
    );
    assert_eq!(
        a.result.namespace_exports.len(),
        0,
        "a dynamic pattern can't be statically recorded"
    );
}

// handle_namespace_path_command

/// The two-word literal set form records entries under the declaring
/// namespace; a later declaration replaces the whole path (C Tcl
/// semantics), and entries are kept as written (rooting happens in the
/// shared candidate builder).
#[test]
fn handle_namespace_path_records_and_replaces() {
    let mut a = Analyser::new();
    a.handle_namespace_path_command(&["path".to_string(), "::u ::v".to_string()], &[], &[]);
    assert_eq!(
        a.namespace_paths.get("::").map(Vec::as_slice),
        Some(&["::u".to_string(), "::v".to_string()][..]),
    );
    a.handle_namespace_path_command(&["path".to_string(), "inner".to_string()], &[], &[]);
    assert_eq!(
        a.namespace_paths.get("::").map(Vec::as_slice),
        Some(&["inner".to_string()][..]),
        "each namespace path declaration replaces the whole path",
    );
}

/// A **braced** path word is literal, not dynamic. The word
/// arrives with braces stripped, so a text-only dynamism scan sees the
/// `$` and abstains from the whole command; the token kind is the
/// authority.
///
/// tclsh-proof: tclsh8.6.14 —
/// `set nn {::$ns}; namespace eval $nn {}; namespace eval a {}` then
/// `namespace eval n { namespace path {::$ns ::a} }` and
/// `namespace eval n { namespace path }` answers `{::$ns} ::a` — the
/// entry is a namespace literally *named* `::$ns`.
#[test]
fn handle_namespace_path_records_a_braced_dollar_bearing_word() {
    let mut a = Analyser::new();
    a.handle_namespace_path_command(
        &["path".to_string(), "::$ns ::a".to_string()],
        &[esc_tok(span(0, 4)), str_tok(span(5, 16))],
        &[],
    );
    assert_eq!(
        a.namespace_paths.get("::").map(Vec::as_slice),
        Some(&["::$ns".to_string(), "::a".to_string()][..]),
        "a braced word never substitutes, so its entries are literal names",
    );
}

/// TN control — the same text **unbraced** really is a substitution, so
/// the command still abstains.
#[test]
fn handle_namespace_path_still_skips_an_unbraced_dollar_word() {
    let mut a = Analyser::new();
    a.handle_namespace_path_command(
        &["path".to_string(), "$entries".to_string()],
        &[esc_tok(span(0, 4)), Token::new(TokenType::Var, span(5, 13))],
        &[],
    );
    assert_eq!(a.namespace_paths.len(), 0);
}

/// The query form (no list) and a dynamic list (`$var` / `[cmd]`)
/// record nothing — the conservative empty path stands.
#[test]
fn handle_namespace_path_skips_query_and_dynamic_forms() {
    let mut a = Analyser::new();
    a.handle_namespace_path_command(&["path".to_string()], &[], &[]);
    a.handle_namespace_path_command(&["path".to_string(), "$entries".to_string()], &[], &[]);
    a.handle_namespace_path_command(
        &["path".to_string(), "[current_path]".to_string()],
        &[],
        &[],
    );
    assert_eq!(a.namespace_paths.len(), 0);
}

/// A declaration inside a namespace scope keys to that namespace's
/// accumulated name, so settlement applies it to the right call sites.
#[test]
fn handle_namespace_path_keys_to_declaring_namespace() {
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(crate::analyser::types::Scope::new(
            crate::analyser::types::ScopeKind::Namespace,
            "outer",
        ));
    a.handle_namespace_path_command(&["path".to_string(), "::helpers".to_string()], &[], &[0]);
    assert_eq!(
        a.namespace_paths.get("::outer").map(Vec::as_slice),
        Some(&["::helpers".to_string()][..]),
    );
}

/// The `namespace path` resolution tier is 8.5+: a bare call under a path
/// gains the path namespace as a candidate from 8.5 on, but under 8.4 it
/// must not (8.4 has no path tier, so the call never reaches it).
#[test]
fn bare_call_honours_namespace_path_only_from_8_5() {
    let src = "namespace eval ::app { namespace path ::mymod\n    proc run {} { helper } }\n";
    let candidates = |dialect: &str| {
        let mut a = Analyser::new();
        a.analyse(src, dialect)
            .command_invocations
            .iter()
            .find(|i| i.name == "helper")
            .map_or_default(|i| i.resolution_candidates.clone())
    };
    assert!(
        candidates("tcl8.6").iter().any(|c| c == "::mymod::helper"),
        "8.6 should add the path namespace as a candidate: {:?}",
        candidates("tcl8.6"),
    );
    assert!(
        !candidates("tcl8.4").iter().any(|c| c == "::mymod::helper"),
        "8.4 has no path tier, so it must not: {:?}",
        candidates("tcl8.4"),
    );
}

/// A command-table mutation inside a child interpreter's eval body
/// belongs to *that* interpreter, not to a flat file-wide map.
///
/// tclsh 9.0.4, verified: after `interp create c; c eval { rename puts
/// myputs }` the parent's `puts` still works, the parent has no `myputs`,
/// the child has `myputs` and no `puts`; and `c eval { rename set {} }`
/// leaves the parent's `set` alone.
#[test]
fn rename_inside_a_child_body_does_not_touch_the_parent_command_table() {
    let src = "interp create c\nc eval { rename puts myputs }\nputs hi\n";
    let r = Analyser::new().analyse(src, "tcl8.6");
    assert_eq!(
        r.renamed_commands.len(),
        0,
        "a child's rename must not enter the parent's rename map: {:?}",
        r.renamed_commands
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.message.contains("'puts'")),
        "the parent's own `puts` is still a live builtin: {:?}",
        r.diagnostics
            .iter()
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect::<Vec<_>>()
    );
}

/// The incremental per-item path must agree with full analysis here: a
/// deferred proc body is walked by a fresh `Analyser` that does not
/// inherit the interpreter stack, so a `rename` buried in a proc defined
/// inside a child body is the shape most likely to diverge.
#[test]
fn per_item_agrees_on_a_rename_nested_in_a_child_body() {
    let src = "interp create c\n\
                   c eval { proc setup {} { rename puts myputs } }\n\
                   puts hi\n";
    let codes = |r: &super::super::types::AnalysisResult| {
        let mut v: Vec<(String, u32)> = r
            .diagnostics
            .iter()
            .map(|d| (d.code.to_string(), d.span.start()))
            .collect();
        v.sort();
        v
    };
    let full = Analyser::new().analyse(src, "tcl8.6");
    let per_item = Analyser::new().analyse_per_item(src, "tcl8.6");
    assert_eq!(codes(&full), codes(&per_item));
    assert!(
        !full
            .diagnostics
            .iter()
            .any(|d| d.message.contains("'puts'")),
        "{:?}",
        codes(&full)
    );
}

/// The same rename written in the parent still records normally — the
/// guard narrows the fact to its interpreter, it does not disable it.
#[test]
fn rename_in_the_parent_still_records_the_command_move() {
    let src = "interp create c\nrename puts myputs\n";
    let r = Analyser::new().analyse(src, "tcl8.6");
    assert_eq!(
        r.renamed_commands.get("::myputs").map(String::as_str),
        Some("::puts"),
        "{:?}",
        r.renamed_commands
    );
}

/// `interp alias {} A {} B` inside a child's eval body creates the alias
/// in the *child* (an empty srcPath is "the interpreter I am running
/// in"), so it must home under that child's `@interp@` domain rather than
/// at the parent's global root.
#[test]
fn empty_path_interp_alias_inside_a_child_body_homes_in_that_child() {
    let src = "interp create c\nc eval { interp alias {} shout {} puts }\n";
    let r = Analyser::new().analyse(src, "tcl8.6");
    assert!(
        !r.command_aliases.contains_key("::shout"),
        "the parent gained no alias: {:?}",
        r.command_aliases.keys().collect::<Vec<_>>()
    );
    assert!(
        r.command_aliases.contains_key("::@interp@c::shout"),
        "the child's alias homes under its own domain: {:?}",
        r.command_aliases.keys().collect::<Vec<_>>()
    );
}

/// The same alias written at the top level is unchanged.
#[test]
fn empty_path_interp_alias_at_top_level_still_homes_globally() {
    let src = "interp alias {} shout {} puts\n";
    let r = Analyser::new().analyse(src, "tcl8.6");
    assert!(
        r.command_aliases.contains_key("::shout"),
        "{:?}",
        r.command_aliases.keys().collect::<Vec<_>>()
    );
}

/// `interp eval child { proc foo }` runs in a child interpreter, so `foo`
/// is isolated from the parent's `::foo` — the two stay distinct commands,
/// so a parent rename of `::foo` can never reach the child body.
#[test]
fn interp_eval_child_isolates_definitions_from_the_parent() {
    // Runnable Tcl (tclsh 9.0.4): the child is created before the
    // eval.  The child's `foo` homes under the synthetic
    // `@interp@child` domain — unrepresentable as a real namespace,
    // so a parent namespace literally named `child` can never
    // collide with the interpreter's definitions (the
    // child-global-vs-parent-namespace identity split).
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create child\nproc foo {} {}\ninterp eval child { proc foo {} {} }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(r.all_procs.contains_key("::foo"), "parent proc: {keys:?}");
    assert!(
        r.all_procs.contains_key("::@interp@child::foo"),
        "child proc isolated under the interpreter domain: {keys:?}",
    );
    assert!(
        !r.all_procs.contains_key("::child::foo"),
        "the child's global is not the parent namespace `::child`: {keys:?}",
    );
}

/// `sandbox eval { proc foo }` via the interpreter's own object command
/// (`interp create sandbox` binds `sandbox` itself as a callable
/// command) isolates definitions from the parent exactly like the
/// literal `interp eval sandbox { … }` form above — the far more common
/// real-world spelling (`docstrip_util.tcl`'s `$c eval {...}`, reduced to
/// a literal name here so it resolves statically).
#[test]
fn interp_handle_eval_isolates_definitions_from_the_parent() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create sandbox\nproc foo {} {}\nsandbox eval { proc foo {} {} }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(r.all_procs.contains_key("::foo"), "parent proc: {keys:?}");
    assert!(
        r.all_procs.contains_key("::@interp@sandbox::foo"),
        "child proc isolated under the interpreter domain: {keys:?}",
    );
}

#[test]
fn renamed_away_interp_handle_reused_as_a_proc_is_not_treated_as_interp_eval() {
    // TP: `self.interpreters` is only cleared by `interp delete`, so a plain
    // `rename sandbox {}` (deleting the interpreter's own object
    // command — confirmed against tclsh9.0: afterwards `info commands
    // sandbox` is empty and the child interpreter is only reachable
    // through whatever name, if any, it was renamed *to*) left a stale
    // `sandbox` entry. A later, wholly unrelated `proc sandbox {sub
    // body} {...}` reusing the freed name would then have any
    // `sandbox eval …` call misidentified as isolated interpreter-eval
    // (walking the literal second argument as a *script*) instead of an
    // ordinary two-arg proc call — confirmed against tclsh9.0 that the
    // real dispatch reaches the new proc, never the (now nameless,
    // orphaned) interpreter.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create sandbox\n\
             rename sandbox {}\n\
             proc sandbox {sub body} {}\n\
             sandbox eval { proc shouldNotBeIsolated {} {} }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        !r.all_procs
            .contains_key("::@interp@sandbox::shouldNotBeIsolated"),
        "the call must not be isolated into the (deleted) interpreter's \
             domain once `sandbox` has been renamed away and reused: {keys:?}",
    );
}

#[test]
fn renamed_interp_handle_still_isolates_under_its_new_name() {
    // TP — the other half of the same fix: a rename that *moves* the
    // interpreter handle (`NEW` non-empty) rather than deleting it must
    // keep the tracking, now keyed by `NEW` — confirmed against
    // tclsh9.0 that `rename sandbox t; t eval {...}` still talks to the
    // same child interpreter. A name-only invalidation (removing `OLD`
    // without transferring state to `NEW`) would wrongly stop isolating
    // `t eval { ... }` too (falling through to an ordinary, unisolated
    // command call). The domain name itself is always derived from
    // whichever key the call site actually writes — `@interp@t`, not
    // the original `@interp@sandbox` — exactly like every other
    // interpreter-domain lookup in this module (e.g. the literal
    // `interp eval` form re-derives its domain from its own `PATH`
    // argument text the same way); this test only asserts that the
    // call is isolated *at all* under the new name, not which domain
    // string it lands in.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create sandbox\n\
             rename sandbox t\n\
             t eval { proc shouldBeIsolated {} {} }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs.contains_key("::@interp@t::shouldBeIsolated"),
        "the renamed handle must still be tracked as an interpreter, \
             isolating the eval body under its new name's domain: {keys:?}",
    );
}

#[test]
fn interp_handle_eval_and_literal_interp_eval_agree_on_the_same_domain() {
    // The two spellings of the same operation must home under the
    // *same* synthetic domain — the analyser can't tell them apart at
    // run time (both dispatch on the identical interpreter object
    // command), so it must not tell them apart statically either.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create sandbox\ninterp eval sandbox { proc viaLiteral {} {} }\n\
             sandbox eval { proc viaHandle {} {} }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs.contains_key("::@interp@sandbox::viaLiteral"),
        "{keys:?}"
    );
    assert!(
        r.all_procs.contains_key("::@interp@sandbox::viaHandle"),
        "{keys:?}"
    );
}

#[test]
fn two_interp_handles_never_cross_contaminate_same_named_procs() {
    // TP: two
    // separate safe child interpreters, each independently defining
    // their own same-named `helper`, must never merge — a call inside
    // one script must resolve to *that* interpreter's helper, never
    // the other's.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create sandboxA\ninterp create sandboxB\n\
             interp eval sandboxA { proc helper {} { return A } }\n\
             sandboxB eval { proc helper {} { return B } }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs.contains_key("::@interp@sandboxA::helper"),
        "{keys:?}"
    );
    assert!(
        r.all_procs.contains_key("::@interp@sandboxB::helper"),
        "{keys:?}"
    );
    // Each interpreter's own `helper` is a genuinely distinct
    // ProcDef, not the same entry aliased twice.
    assert_ne!(
        r.all_procs["::@interp@sandboxA::helper"].body_span,
        r.all_procs["::@interp@sandboxB::helper"].body_span,
    );
}

#[test]
fn untracked_bareword_eval_is_not_isolated() {
    // TN / FN guard — a bareword shaped exactly like the tracked-handle
    // idiom (`NAME eval { script }`) but never created via `interp
    // create` must fall through to the generic dispatch untouched: no
    // isolated scope, no crash.
    let mut a = Analyser::new();
    let r = a.analyse("untracked eval { proc foo {} {} }\n", "tcl8.6");
    assert!(
        !r.all_procs.keys().any(|k| k.contains("@interp@")),
        "an untracked head must never open an interpreter domain: {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn ordinary_proc_with_eval_as_sole_argument_is_untouched() {
    // FP guard — an unrelated proc whose single argument merely happens
    // to be the literal text `eval` must resolve as an ordinary call,
    // never trip the arity/shape check into isolating anything.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {a} { return $a }\nfoo eval\n", "tcl8.6");
    assert!(
        !r.all_procs.keys().any(|k| k.contains("@interp@")),
        "a 1-arg call must never be treated as NAME eval SCRIPT: {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn interp_eval_into_uncreated_interp_warns_and_multiword_stays_isolated_945() {
    // `interp eval ghost { … }` with no `interp create ghost` anywhere
    // raises `could not find interpreter` at run time — W140.
    let mut a = Analyser::new();
    let r = a.analyse("interp eval ghost { proc foo {} {} }\n", "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W140),
        "eval into a never-created interpreter warns: {:?}",
        r.diagnostics
    );
    // A dynamic create makes existence unknowable — W140 abstains.
    let mut a2 = Analyser::new();
    let r2 = a2.analyse(
        "interp create $name\ninterp eval ghost { proc foo {} {} }\n",
        "tcl8.6",
    );
    assert!(
        !r2.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W140),
        "dynamic interp creation abstains: {:?}",
        r2.diagnostics
    );
    // Multi-word scripts concatenate at run time — the words must not
    // be analysed in the *parent* scope, nor
    // walked per-word (commands span word boundaries).
    let mut a3 = Analyser::new();
    let r3 = a3.analyse(
        "interp create c\ninterp eval c {proc leak {} {}} {puts hi}\n",
        "tcl8.6",
    );
    assert!(
        !r3.all_procs.contains_key("::leak"),
        "multi-word eval must not leak definitions into the parent: {:?}",
        r3.all_procs.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn child_created_in_a_namespace_proc_behind_a_glob_import_is_not_w140_2289() {
    let source = "package require tcltest\n\
             namespace eval vl {\n\
             namespace import ::tcltest::*\n\
             proc New {} {\n\
             catch {interp delete c}\n\
             interp create c\n\
             interp eval c [list set auto_path $::auto_path]\n\
             }\n\
             }\n\
             interp eval c {set x 1}\n";
    let r = Analyser::new().analyse(source, "tcl8.6");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W140),
        "{:?}",
        r.diagnostics
    );
}

#[test]
fn deleted_and_recreated_interp_is_a_fresh_domain_945() {
    // C: `interp delete s; interp create s` starts with an empty
    // command table — the first lifetime's definitions are gone.  The
    // epoch-stamped
    // domain keeps the two lifetimes apart: the first eval's `foo`
    // homes under `@interp@s`, the recreated interpreter's under
    // `@interp@s#1`, and they never merge.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create s\ninterp eval s { proc foo {} {} }\n\
             interp delete s\ninterp create s\n\
             interp eval s { proc bar {} {} }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs.contains_key("::@interp@s::foo"),
        "first lifetime's proc: {keys:?}"
    );
    assert!(
        r.all_procs.contains_key("::@interp@s#1::bar"),
        "recreated interpreter is a fresh domain: {keys:?}"
    );
    assert!(
        !r.all_procs.contains_key("::@interp@s::bar"),
        "the recreated child never merges with its predecessor: {keys:?}"
    );
}

#[test]
fn nested_interp_ops_qualify_against_the_enclosing_child_945() {
    // `interp create t` inside `interp eval s {…}` creates `s t` —
    // paths are relative to the *current* interpreter, so a top-level
    // `interp eval {s t} {…}` reaches the grandchild without W140 and
    // its definitions home under the composed domain.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create s\n\
             interp eval s { interp create t }\n\
             interp eval {s t} { proc deep {} {} }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W140),
        "the nested create makes `s t` exist: {:?}",
        r.diagnostics
    );
    assert!(
        r.all_procs.contains_key("::@interp@s t::deep"),
        "grandchild definitions home under the composed domain: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

#[test]
fn cross_domain_interp_alias_links_child_calls_to_the_parent_target_945() {
    // `interp alias s helper {} ::parent_helper` makes `helper`
    // callable *inside the child* running the parent's proc — the
    // alias deliberately crosses interpreter domains while
    // definitions do not.  The child-side call
    // resolves through the alias link to the parent target.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc parent_helper {} {}\ninterp create s\n\
             interp alias s helper {} parent_helper\n\
             interp eval s { helper }\n",
        "tcl8.6",
    );
    let alias = r
        .command_aliases
        .get("::@interp@s::helper")
        .unwrap_or_else(|| {
            panic!(
                "child-domain alias recorded: {:?}",
                r.command_aliases.keys().collect::<Vec<_>>()
            )
        });
    assert_eq!(
        alias.target, "::parent_helper",
        "the alias target resolves in the parent domain",
    );
}

#[test]
fn safe_interp_hides_unsafe_commands_and_expose_restores_945() {
    // naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    // tclsh 9.0.4: a `-safe` child hides `source` (and the rest of the
    // non-CMD_IS_SAFE set) — calling it raises `invalid command name`.
    // The source warning retains its explicit child/source assumptions.
    // It does not establish an actual selected hidden command, so the
    // possible source edge remains available to workspace consumers.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\ninterp eval s { source b.tcl }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "hidden `source` in a safe interp warns: {:?}",
        r.diagnostics
    );
    assert_eq!(
        r.source_targets.len(),
        1,
        "conditional visibility does not discard the possible source edge: {:?}",
        r.source_targets
    );
    // A safe command (`puts`) draws no W129; a non-safe interp draws
    // none either.
    let mut a2 = Analyser::new();
    let r2 = a2.analyse(
        "interp create -safe s\ninterp eval s { puts hi }\n\
             interp create n\ninterp eval n { source b.tcl }\n",
        "tcl8.6",
    );
    assert!(
        !r2.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "safe commands and non-safe interps draw no W129: {:?}",
        r2.diagnostics
    );
    // `interp expose s source` restores the command.
    let mut a3 = Analyser::new();
    let r3 = a3.analyse(
        "interp create -safe s\ninterp expose s source\n\
             interp eval s { source b.tcl }\n",
        "tcl8.6",
    );
    assert!(
        !r3.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "an exposed command is callable again: {:?}",
        r3.diagnostics
    );
    // `interp hide n source` hides it in a *normal* interp too.
    let mut a4 = Analyser::new();
    let r4 = a4.analyse(
        "interp create n\ninterp hide n source\n\
             interp eval n { source b.tcl }\n",
        "tcl8.6",
    );
    assert!(
        r4.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "an explicitly hidden command warns in a normal interp: {:?}",
        r4.diagnostics
    );
}

#[test]
fn safe_interp_child_redefinition_of_a_hidden_builtin_is_callable_945() {
    // naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    // tclsh 9.0.4: `proc source {} {…}` inside a safe interp creates a
    // real command in the ordinary command table — a table entirely
    // separate from the hidden set — so calling `source` afterwards
    // runs the user's proc, never `invalid command name`.  The gate
    // must not draw W129 for the call, and the call must still be
    // fully analysed (resolving to the local proc), not skipped as an
    // effect-free hidden call.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s { proc source {} { return ok }; source }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a locally-redefined name is callable, not hidden: {:?}",
        r.diagnostics
    );
    assert!(
        r.all_procs.contains_key("::@interp@s::source"),
        "the local proc is recorded, not skipped: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    // A call *preceding* the redefinition is unaffected by this file's
    // whole-body approximation in the other direction — still hidden
    // until the walk has seen a local definition — is not asserted
    // here (the analyser's per-body model is not that finely ordered);
    // what matters is that a real, common redefinition idiom never
    // produces a false-positive W129 that also drops the call's facts.
}

#[test]
fn interp_expose_with_a_new_name_tracks_the_new_name_945() {
    // tclsh 9.0.4: `interp expose s source src` restores the hidden
    // `source` implementation under the name `src` — `source` itself
    // stays absent from ordinary lookup, and `src` becomes callable.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\ninterp expose s source src\n\
             interp eval s { source b.tcl }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "the hidden name itself stays hidden after an exposed rename: {:?}",
        r.diagnostics
    );
    let mut a2 = Analyser::new();
    let r2 = a2.analyse(
        "interp create -safe s\ninterp expose s source src\n\
             interp eval s { src b.tcl }\n",
        "tcl8.6",
    );
    assert!(
        !r2.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "the new exposed name is callable: {:?}",
        r2.diagnostics
    );
}

// W129 through `[...]` bracket-substitution indirection.

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_list_quoted_apply_lambda_body_reports_hidden_source_1001() {
    // naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 package ifneeded myPackage 1.0 [list apply {dir {\n\
                     source [file join $dir font.tcl]\n\
                 }} $dir]\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "hidden `source` nested inside a list-quoted apply lambda body \
             (reached only via `package ifneeded`'s deferred script) warns: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_list_quoted_apply_in_command_prefix_position_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 trace add variable x write [list apply {{a b c} {\n\
                     exec ls\n\
                 }} $x]\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "hidden `exec` nested inside a list-quoted apply lambda body \
             reached via a CommandPrefix (`trace add … command`) position warns: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_list_quoted_apply_after_idle_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 after idle [list apply {x {\n\
                     file delete $x\n\
                 }} val]\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "hidden `file` nested inside a list-quoted apply lambda body \
             reached via `after idle` warns: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_list_quoted_hidden_command_directly_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s { after idle [list source b.tcl] }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "a bare `[list source …]` in a deferred-call position warns \
             directly, with no `apply` indirection needed: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_list_quoted_apply_in_plain_data_is_not_flagged_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 set data [list apply {x { source $x }} value]\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a `[list apply …]` value that is only ever stored, never \
             invoked, must not warn: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn list_quoted_apply_lambda_outside_any_safe_interp_is_untouched_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "package ifneeded myPackage 1.0 [list apply {dir {source [file join $dir x]}} $dir]\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "no safe interpreter is involved, so no W129 can ever fire: {:?}",
        r.diagnostics
    );
    assert_eq!(
        r.all_procs.len(),
        0,
        "this fix must not widen the general analyser's scope — \
             the list-quoted lambda body stays un-analysed outside a \
             safe-interpreter context, exactly as before #1001: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_direct_nested_bracket_substitution_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\ninterp eval s { set x [source b.tcl] }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "a directly nested `[source …]` substitution warns: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_direct_nested_bracket_substitution_deep_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s { if {[catch {set y [exec ls]} err]} { puts $err } }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "a deeply nested `[exec …]` substitution still warns: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_expand_list_quoted_head_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\ninterp eval s { {*}[list source b.tcl] }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "`{{*}}[list source …]` used as the whole statement warns \
             on the effective (expanded) head: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_expand_dynamic_var_head_not_flagged_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s { set cmdList [list source b.tcl]; {*}$cmdList }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "an opaque `{{*}}$var` expansion is not statically resolvable \
             and must not warn: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_dynamic_variable_dispatch_not_flagged_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s { set cmd source; $cmd b.tcl }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "dynamic dispatch through a variable is not statically \
             resolvable and must not warn: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_eval_list_quoted_hidden_command_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\ninterp eval s { eval [list source b.tcl] }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "`eval [list source …]` warns on the resolved head: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_uplevel_list_quoted_hidden_command_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\ninterp eval s { uplevel [list source b.tcl] }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "`uplevel [list source …]` warns on the resolved head: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_list_quoted_apply_safe_command_not_flagged_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s { after idle [list apply {x { puts $x }} val] }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a safe command (`puts`) reached via list-quoted apply must \
             not warn: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_redefined_command_not_flagged_through_indirection_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 proc source {} { return ok }\n\
                 after idle [list apply {{} { source }}]\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a locally-redefined name reached via list-quoted apply is \
             callable, not hidden: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_alias_bridged_command_not_flagged_through_indirection_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp alias s foo {} source\n\
             interp eval s { after idle [list apply {{} { foo }}] }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "an alias bridging a capability in from the trusted parent is \
             not itself a hidden registry name, so it must not warn: {:?}",
        r.diagnostics
    );
}

/// `namespace inscope ::x { proc foo }` runs the body in `::x`, so `foo`
/// homes to `::x::foo` — the same namespace frame as `namespace eval`, not
/// the caller's scope.
#[test]
fn namespace_inscope_runs_the_body_in_the_named_namespace() {
    let mut a = Analyser::new();
    let r = a.analyse("namespace inscope ::x { proc foo {} {} }\n", "tcl8.6");
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs.contains_key("::x::foo"),
        "inscope body should home to the named namespace: {keys:?}",
    );
    assert!(
        !r.all_procs.contains_key("::foo"),
        "the body must not resolve in the caller's global scope: {keys:?}",
    );
}

/// `namespace code { proc foo }` captures the *current* namespace, so its
/// script is analysed in this scope — inside `::x`, `foo` homes to
/// `::x::foo`.
#[test]
fn namespace_code_analyses_the_script_in_the_current_namespace() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::x { namespace code { proc foo {} {} } }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs.contains_key("::x::foo"),
        "namespace code script should analyse in the current namespace: {keys:?}",
    );
}

/// `interp eval {} { proc foo }` targets the *current* interpreter, so
/// `foo` is the parent's `::foo` (no isolation, no synthetic namespace).
#[test]
fn interp_eval_empty_path_is_the_current_interpreter() {
    let mut a = Analyser::new();
    let r = a.analyse("interp eval {} { proc foo {} {} }\n", "tcl8.6");
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs.contains_key("::foo"),
        "current-interp proc: {keys:?}"
    );
    assert!(
        !keys.iter().any(|k| k.contains("::::")),
        "an empty path must not open a synthetic namespace: {keys:?}",
    );
}

#[test]
fn handle_namespace_eval_creates_child_scope() {
    let mut a = Analyser::new();
    let handled = a.handle_namespace_eval_command(
        &[
            "eval".to_string(),
            "ns1".to_string(),
            "proc inner {} {}".to_string(),
        ],
        &[
            esc_tok(span(10, 14)),
            esc_tok(span(15, 18)),
            str_tok(span(19, 35)),
        ],
        &[],
        &[],
    );
    assert!(handled);
    assert_eq!(a.result.global_scope.children.len(), 1);
    assert_eq!(a.result.global_scope.children[0].name, "ns1");
    assert_eq!(
        a.result.global_scope.children[0].kind,
        crate::analyser::types::ScopeKind::Namespace,
    );
}

#[test]
fn namespace_scope_geometry_retains_the_actual_point_operand() {
    // naming.source.original-point-operand-projection
    // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
    let source = "set target A; namespace eval $target {namespace eval B {set x 1}}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let outer = &result.global_scope.children[0];
    let inner = &outer.children[0];
    let outer_input = result
        .namespace_refs
        .iter()
        .find(|reference| {
            reference
                .original_name_input
                .as_ref()
                .is_some_and(|input| input.bytes() == b"A")
        })
        .unwrap()
        .original_name_input
        .as_ref()
        .unwrap();
    let inner_input = result
        .namespace_refs
        .iter()
        .find(|reference| {
            reference
                .original_name_input
                .as_ref()
                .is_some_and(|input| input.bytes() == b"B")
        })
        .unwrap()
        .original_name_input
        .as_ref()
        .unwrap();
    assert_eq!(
        outer.naming_scope,
        result
            .global_scope
            .naming_scope
            .as_ref()
            .unwrap()
            .child_from_input(outer_input)
    );
    assert!(outer.naming_scope.is_some());
    assert_eq!(
        inner.naming_scope,
        outer
            .naming_scope
            .as_ref()
            .unwrap()
            .child_from_input(inner_input)
    );
    assert!(inner.naming_scope.is_some());
    let unknown = Analyser::new().analyse("namespace eval $missing {set x 1}", "tcl8.6");
    assert!(unknown.global_scope.children[0].naming_scope.is_none());
}

#[test]
fn handle_namespace_eval_records_body_span() {
    let mut a = Analyser::new();
    a.handle_namespace_eval_command(
        &["eval".to_string(), "ns1".to_string(), String::new()],
        &[
            esc_tok(span(10, 14)),
            esc_tok(span(15, 18)),
            str_tok(span(19, 35)),
        ],
        &[],
        &[],
    );
    assert_eq!(
        a.result.global_scope.children[0].body_span,
        Some(span(19, 35))
    );
}

#[test]
fn handle_namespace_eval_dynamic_target_gets_a_synthetic_span_keyed_name() {
    // TP (corpus shape: irc.tcl's per-connection
    // `namespace eval $name { … }` idiom): a
    // dynamic target must never become the scope's `.name` verbatim (two
    // unrelated occurrences sharing the same variable name would then
    // collapse into one scope), so it's replaced with a synthetic name
    // keyed on this occurrence's own token offset.
    let mut a = Analyser::new();
    a.handle_namespace_eval_command(
        &[
            "eval".to_string(),
            "$name".to_string(),
            "proc inner {} {}".to_string(),
        ],
        &[
            esc_tok(span(10, 14)),
            esc_tok(span(15, 20)),
            str_tok(span(21, 37)),
        ],
        &[],
        &[],
    );
    assert_eq!(a.result.global_scope.children[0].name, "@dynns@15");
}

#[test]
fn handle_namespace_eval_literal_target_keeps_its_written_name() {
    // FN guard — a literal (non-dynamic) target must still use its own
    // written text verbatim.
    let mut a = Analyser::new();
    a.handle_namespace_eval_command(
        &["eval".to_string(), "ns1".to_string(), String::new()],
        &[
            esc_tok(span(10, 14)),
            esc_tok(span(15, 18)),
            str_tok(span(19, 35)),
        ],
        &[],
        &[],
    );
    assert_eq!(a.result.global_scope.children[0].name, "ns1");
}

#[test]
fn two_dynamic_namespace_eval_blocks_sharing_a_variable_name_never_collide() {
    // TP — the full-pipeline shape from irc.tcl: two lexically
    // unrelated procs each open `namespace eval $name { … }` with a
    // same-named local `name` (an unremarkable choice for this exact
    // idiom) and each define their own `helper`. Keying both blocks'
    // scope on the literal text "$name" would let the second `helper`
    // silently overwrite the first in the flat `all_procs` map.
    let mut a = Analyser::new();
    let r = a.analyse(
            "proc setupA {} {\n    set name connA\n    namespace eval $name {\n        proc helper {} { return A }\n    }\n}\n\
             proc setupB {} {\n    set name connB\n    namespace eval $name {\n        proc helper {} { return B }\n    }\n}\n",
            "tcl8.6",
        );
    // drift-ok: test assertion counting distinct entries by simple name
    // to prove no cross-occurrence merge, not a resolution decision.
    let helpers: Vec<_> = r
        .all_procs
        .iter()
        .filter(|(_, p)| p.name == "helper")
        .collect();
    assert_eq!(
        helpers.len(),
        2,
        "both blocks' helper must survive as distinct entries: {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );
    assert_ne!(
        helpers[0].0, helpers[1].0,
        "distinct call sites must get distinct qualified keys: {helpers:?}",
    );
    assert_ne!(
        helpers[0].1.body_span, helpers[1].1.body_span,
        "each is genuinely its own definition, not the same one twice: {helpers:?}",
    );
}

// handle_namespace_ensemble

#[test]
fn handle_namespace_ensemble_records_in_set() {
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "myns"));
    a.handle_namespace_ensemble(
        &["ensemble".to_string(), "create".to_string()],
        &[],
        &[0],
        None,
    );
    assert!(a.ensemble_namespaces.contains("::myns"));
}

#[test]
fn handle_namespace_ensemble_command_option_recorded() {
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    // `-command` recognition is registry-driven (`ENSEMBLE_CREATE_OPTIONS`).
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "myns"));
    a.handle_namespace_ensemble(
        &[
            "ensemble".to_string(),
            "create".to_string(),
            "-command".to_string(),
            "::ens".to_string(),
        ],
        &[],
        &[0],
        None,
    );
    assert!(a.ensemble_namespaces.contains("::myns"));
    assert!(a.ensemble_namespaces.contains("::ens"));
}

#[test]
fn handle_namespace_ensemble_command_option_qualifies_relative_name() {
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    // `-command` recognition is registry-driven (`ENSEMBLE_CREATE_OPTIONS`).
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "myns"));
    a.handle_namespace_ensemble(
        &[
            "ensemble".to_string(),
            "create".to_string(),
            "-command".to_string(),
            "ens".to_string(),
        ],
        &[],
        &[0],
        None,
    );
    assert!(a.ensemble_namespaces.contains("::myns::ens"));
}

#[test]
fn handle_namespace_ensemble_command_option_dynamic_value_not_recorded() {
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "myns"));
    a.handle_namespace_ensemble(
        &[
            "ensemble".to_string(),
            "create".to_string(),
            "-command".to_string(),
            "$dyn".to_string(),
        ],
        &[],
        &[0],
        None,
    );
    assert_eq!(
        a.ensemble_namespaces,
        std::collections::HashSet::from(["::myns".to_string()])
    );
}

#[test]
fn namespace_ensemble_map_and_subcommands_record_command_references() {
    // `-map {get ::foo::getImpl}` — the odd element is the target command;
    // `-subcommands {show}` maps `show` to `::foo::show`.  Both become
    // command references so navigation reaches the implementing procs.
    let mut a = Analyser::new();
    let src = "namespace eval ::foo {\n    \
                   proc getImpl {} {}\n    \
                   proc show {} {}\n    \
                   namespace ensemble create -map {get ::foo::getImpl} -subcommands {show}\n\
                   }\n";
    let r = a.analyse(src, "tcl8.6");
    let resolved: Vec<&str> = r
        .command_invocations
        .iter()
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        resolved.contains(&"::foo::getImpl"),
        "the -map target should be a command reference: {resolved:?}",
    );
    assert!(
        resolved.contains(&"::foo::show"),
        "the -subcommands name should map to `<ns>::show`: {resolved:?}",
    );
}

/// A tk-shaped ensemble: the ensemble-creating command runs inside
/// a proc whose *qualified name* homes it to `::tk`, but which is declared
/// at the top level with no enclosing `namespace eval`.  `namespace
/// current` there is `::tk` (tclsh 8.6.16 / 9.0.4-verified: the ensemble
/// created is `::tk` and `tk alpha` dispatches to `::tk::alpha`), so the
/// subcommand must map to `::tk::alpha`; a purely lexical namespace walk
/// skips proc scopes and would map it to `::alpha`.
#[test]
fn namespace_ensemble_create_in_a_qualified_name_proc_homes_to_that_namespace_923_idx85() {
    let mut a = Analyser::new();
    let src = "namespace eval ::tk {}\n\
                   proc ::tk::alpha {} {}\n\
                   proc ::tk::SetupEnsemble {} {\n    \
                   namespace ensemble create -subcommands {alpha}\n\
                   }\n";
    let r = a.analyse(src, "tcl8.6");
    let resolved: Vec<&str> = r
        .command_invocations
        .iter()
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        resolved.contains(&"::tk::alpha"),
        "the subcommand should map to the proc's defining namespace: {resolved:?}",
    );
    assert!(
        !resolved.contains(&"::alpha"),
        "the lexical (global) namespace is the wrong home: {resolved:?}",
    );
    assert!(
        a.ensemble_namespaces.contains("::tk"),
        "the ensemble command itself is `::tk`: {:?}",
        a.ensemble_namespaces,
    );
}

/// TN control for the above: the ordinary lexical case — the ensemble is
/// created directly inside `namespace eval ::foo`, so both the lexical and
/// the command-resolution namespace agree on `::foo`.  Pinned so the common
/// shape cannot regress.
#[test]
fn namespace_ensemble_create_lexically_inside_namespace_eval_is_unchanged_923_idx85() {
    let mut a = Analyser::new();
    let src = "namespace eval ::foo {\n    \
                   proc alpha {} {}\n    \
                   namespace ensemble create -subcommands {alpha}\n\
                   }\n";
    let r = a.analyse(src, "tcl8.6");
    let resolved: Vec<&str> = r
        .command_invocations
        .iter()
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        resolved.contains(&"::foo::alpha"),
        "lexical case still maps to `::foo::alpha`: {resolved:?}",
    );
    assert!(a.ensemble_namespaces.contains("::foo"));
}

/// The definition-homing sweep: every analyser site that asks "which
/// namespace is current here?" answers with
/// [`Analyser::command_resolution_namespace`], so a definition made inside
/// a qualified-name proc's body homes to that proc's *defining* namespace.
/// These are cases (A)–(D) from `docs/design/analysis/name-resolution.md`
/// §3.5, each with the real interpreter's answer as the expectation.
#[test]
fn definitions_inside_a_qualified_name_proc_home_to_its_defining_namespace_923_idx85() {
    // (A) The already-correct lexical control.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::x { proc mk {} { proc helper {} {} } }\n",
        "tcl8.6",
    );
    assert!(
        r.all_procs.contains_key("::x::helper"),
        "(A) lexical case: {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );

    // (B) A qualified-name proc with no enclosing `namespace eval`. Each
    // qualifier exists first: tclsh 8.6/9.0 refuse `proc ::x::mk` with
    // `unknown namespace` otherwise.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::x {}\nproc ::x::mk {} { proc helper {} {} }\n",
        "tcl8.6",
    );
    assert!(
        r.all_procs.contains_key("::x::helper"),
        "(B) qualified encloser: {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );

    // (C) A qualified-name proc whose own name overrides the lexical
    // `namespace eval` it sits in.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::y {}\nnamespace eval ::x { proc ::y::mk {} { proc helper {} {} } }\n",
        "tcl8.6",
    );
    assert!(
        r.all_procs.contains_key("::y::helper"),
        "(C) absolute name rebases: {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );

    // (D) The same rule for a class, not a proc.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::x {}\nproc ::x::mk {} { oo::class create Helper {} }\n",
        "tcl8.6",
    );
    assert!(
        r.all_classes.contains_key("::x::Helper"),
        "(D) class create: {:?}",
        r.all_classes.keys().collect::<Vec<_>>(),
    );
}

/// A nested definition that mis-homes to `::` collides with a real global
/// of the same name, and one silently overwrites the other.  Both must
/// survive under their own keys.
#[test]
fn a_nested_definition_no_longer_overwrites_a_same_named_global_923_idx85() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc helper {} { return global }\n\
             proc ::x::mk {} { proc helper {} { return nested } }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs.contains_key("::helper"),
        "global lost: {keys:?}"
    );
    assert!(
        r.all_procs.contains_key("::x::helper"),
        "nested lost: {keys:?}",
    );
}

/// `namespace import` written inside a qualified-name proc imports into
/// that proc's defining namespace, not the global one.
#[test]
fn namespace_import_inside_a_qualified_name_proc_targets_that_namespace_923_idx85() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::src { proc thing {} {} namespace export thing }\n\
             namespace eval ::dst {}\n\
             proc ::dst::setup {} { namespace import ::src::thing }\n",
        "tcl8.6",
    );
    let targets: Vec<&str> = r.namespace_imports.iter().map(|i| i.ns.as_str()).collect();
    assert!(
        targets.contains(&"::dst"),
        "the import lands in the proc's defining namespace: {targets:?}",
    );
}

/// A `-map` target inside the same qualified-name proc resolves the same
/// way — the `-map` half.
#[test]
fn namespace_ensemble_map_in_a_qualified_name_proc_resolves_targets_923_idx85() {
    let mut a = Analyser::new();
    let src = "namespace eval ::tk {}\n\
                   proc ::tk::getImpl {} {}\n\
                   proc ::tk::SetupEnsemble {} {\n    \
                   namespace ensemble create -map {get getImpl}\n\
                   }\n";
    let r = a.analyse(src, "tcl8.6");
    let resolved: Vec<&str> = r
        .command_invocations
        .iter()
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        resolved.contains(&"::tk::getImpl"),
        "the -map target resolves in the proc's defining namespace: {resolved:?}",
    );
}

// The tk-shaped ensemble's *call-site* half.  Everything above pins the
// ensemble's own declaration (where the `-map`/`-subcommands` targets
// home to); these pin the downstream `<ensemble> <sub>` dispatch sites,
// which is what find-references / rename / code-lens / call-hierarchy
// enumerate.  Oracle — tclsh 8.6.16 and 9.0.4 both print
// `shown` then `configured:-x 1` for `VIAPROC_SRC`, so
// `::app::widget show` really does dispatch to `::app::widget::Show`.

/// The shape: `namespace ensemble create -map` inside a
/// proc declared with a fully-qualified name at top level, with no
/// enclosing `namespace eval`, and the dispatch call sites written after
/// it — one nested in a `[…]` substitution, one at the top level.
const VIAPROC_SRC: &str = "namespace eval ::app::widget {\n    \
         variable state 0\n\
         }\n\
         proc ::app::widget::Setup {} {\n    \
         namespace ensemble create -map {\n        \
         show      ::app::widget::Show\n        \
         configure ::app::widget::Configure\n    \
         }\n\
         }\n\
         proc ::app::widget::Show {} { puts \"shown\" }\n\
         proc ::app::widget::Configure {args} { puts \"configured:$args\" }\n\
         ::app::widget::Setup\n\
         puts [::app::widget show]\n\
         ::app::widget configure -x 1\n";

/// Every `(written name, resolved name)` pair recorded as an
/// existence-probed reference — the shape
/// `record_ensemble_subcommand_invocation` gives a subcommand word.
fn probe_refs(r: &super::super::types::AnalysisResult) -> Vec<(String, String)> {
    r.command_invocations
        .iter()
        .filter(|i| i.existence_probe)
        .map(|i| {
            (
                i.name.clone(),
                i.resolved_qualified_name.clone().unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn ensemble_call_sites_resolve_when_the_ensemble_is_created_in_a_qualified_proc_923_idx85() {
    let r = Analyser::new().analyse(VIAPROC_SRC, "tcl8.6").clone();
    let probes = probe_refs(&r);
    assert!(
        probes.contains(&("show".to_owned(), "::app::widget::Show".to_owned())),
        "the `[::app::widget show]` dispatch must be a reference to Show: {probes:?}",
    );
    assert!(
        probes.contains(&(
            "configure".to_owned(),
            "::app::widget::Configure".to_owned()
        )),
        "the top-level `::app::widget configure` dispatch must be a reference \
             to Configure: {probes:?}",
    );
}

#[test]
fn per_item_records_the_same_ensemble_call_sites_as_the_whole_file_walk_923_idx85() {
    // The per-item shell pass defers `::app::widget::Setup`'s body, so the
    // ensemble map is empty when the two call sites below it are
    // walked: without the deferred replay neither subcommand reference
    // is recorded — go-to-definition (an on-demand lookup against the
    // finished analysis) still answers, find-references cannot.  Comparing
    // the two walk strategies directly is the assertion that cannot drift.
    let whole = Analyser::new().analyse(VIAPROC_SRC, "tcl8.6").clone();
    let per_item = Analyser::new().analyse_per_item(VIAPROC_SRC, "tcl8.6");
    assert_eq!(
        probe_refs(&per_item),
        probe_refs(&whole),
        "per-item must record the same ensemble dispatch references as the \
             whole-file walk",
    );
    assert!(
        probe_refs(&per_item)
            .iter()
            .any(|(_, res)| res == "::app::widget::Show"),
        "and both must actually find Show's call site: {:?}",
        probe_refs(&per_item),
    );
}

#[test]
fn a_dynamic_ensemble_map_records_no_call_site_reference_923_idx85() {
    // TN — deliberate abstention.  `-map [list …]` is not a literal
    // list, so no `subcommand -> target` fact exists at all; the
    // dispatch site must stay unattributed rather than being guessed
    // at from the surrounding text.  Both walk strategies abstain.
    let src = "proc ::app::widget::Setup {} {\n    \
                   namespace ensemble create -map [list show ::app::widget::Show]\n\
                   }\n\
                   proc ::app::widget::Show {} {}\n\
                   ::app::widget::Setup\n\
                   ::app::widget show\n";
    for r in [
        Analyser::new().analyse(src, "tcl8.6").clone(),
        Analyser::new().analyse_per_item(src, "tcl8.6"),
    ] {
        assert!(
            !probe_refs(&r)
                .iter()
                .any(|(_, res)| res == "::app::widget::Show"),
            "a dynamic -map must not attribute the dispatch site: {:?}",
            probe_refs(&r),
        );
    }
}

#[test]
fn an_ensemble_call_site_above_its_declaration_is_not_attributed_923_idx85() {
    // TN — the whole-file DFS walks a proc body at its *definition*
    // point, so a dispatch written above the proc that creates the
    // ensemble sees no map.  The deferred replay reproduces that
    // visibility rule rather than widening it, which is what keeps the
    // two walk strategies byte-identical.
    let src = "proc ::app::widget::Show {} {}\n\
                   ::app::widget show\n\
                   proc ::app::widget::Setup {} {\n    \
                   namespace ensemble create -map {show ::app::widget::Show}\n\
                   }\n";
    for r in [
        Analyser::new().analyse(src, "tcl8.6").clone(),
        Analyser::new().analyse_per_item(src, "tcl8.6"),
    ] {
        assert!(
            !probe_refs(&r)
                .iter()
                .any(|(_, res)| res == "::app::widget::Show"),
            "a dispatch above the declaration must not be attributed: {:?}",
            probe_refs(&r),
        );
    }
}

// `namespace ensemble configure`: the real
// `tk/library/systray.tcl` idiom splices new subcommands onto a
// *pre-existing* ensemble via `configure`, not `create`, so
// `handle_namespace_ensemble` has to recognise both.

#[test]
fn namespace_ensemble_configure_extends_a_preexisting_ensembles_map() {
    // TP — the mechanism in isolation, with a literal `-map` value and a
    // deliberately non-tk ensemble name (`myens`), proving the mechanism is
    // registry-agnostic rather than hardcoded to `tk`.
    let mut a = Analyser::new();
    let src = "namespace eval ::myens {\n    \
                   namespace ensemble create -subcommands {}\n\
                   }\n\
                   proc ::myens::extra {args} {}\n\
                   namespace ensemble configure ::myens -map {extra ::myens::extra}\n\
                   myens extra\n";
    let r = a.analyse(src, "tcl8.6");
    let resolved: Vec<&str> = r
        .command_invocations
        .iter()
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        resolved.contains(&"::myens::extra"),
        "a configure-spliced -map target should be a command reference: {resolved:?}",
    );
}

#[test]
fn namespace_ensemble_configure_dynamic_name_is_not_recorded() {
    // FP-guard — `namespace ensemble configure $x -map {...}` can't be
    // resolved statically; the whole call must abstain rather than
    // recording a `-map` splice under a guessed/wrong key.
    let mut a = Analyser::new();
    a.handle_namespace_ensemble(
        &[
            "ensemble".to_string(),
            "configure".to_string(),
            "$x".to_string(),
            "-map".to_string(),
            "sub".to_string(),
            "::real::target".to_string(),
        ],
        &[],
        &[],
        None,
    );
    assert_eq!(a.result.ensemble_subcommand_targets.len(), 0);
}

#[test]
fn namespace_ensemble_configure_with_no_name_argument_is_a_no_op() {
    // TN — the bare query form `namespace ensemble configure` (no NAME,
    // no options) must not panic on `args[2]` indexing.
    let mut a = Analyser::new();
    a.handle_namespace_ensemble(
        &["ensemble".to_string(), "configure".to_string()],
        &[],
        &[],
        None,
    );
    assert_eq!(a.result.ensemble_subcommand_targets.len(), 0);
}

#[test]
fn namespace_ensemble_configure_dict_merge_literal_tail_is_extracted() {
    // TP — the real, exact idiom:
    // `namespace ensemble configure NAME -map [dict merge [namespace
    // ensemble configure NAME -map] {literal}]`. The self-referential
    // query argument is left unknown, but the literal tail's own pairs
    // are statically known regardless of what it evaluates to.
    let mut a = Analyser::new();
    let src = "namespace eval ::myens {\n    \
                   namespace ensemble create -subcommands {}\n\
                   }\n\
                   proc ::myens::extra {args} {}\n\
                   proc ::myens::other {args} {}\n\
                   namespace ensemble configure ::myens -map \
                   [dict merge [namespace ensemble configure ::myens -map] \
                   {extra ::myens::extra other ::myens::other}]\n\
                   myens extra\n\
                   myens other\n";
    let r = a.analyse(src, "tcl8.6");
    let resolved: Vec<&str> = r
        .command_invocations
        .iter()
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        resolved.contains(&"::myens::extra"),
        "the dict-merge literal tail's first pair should resolve: {resolved:?}",
    );
    assert!(
        resolved.contains(&"::myens::other"),
        "the dict-merge literal tail's second pair should resolve too, \
             proving the extraction isn't overfit to a single pair: {resolved:?}",
    );
}

#[test]
fn namespace_ensemble_configure_unrecognised_dynamic_map_shape_abstains_safely() {
    // Safety guard: a `-map` value that is itself one whole
    // dynamic `[...]` substitution NOT matching the narrow `dict merge
    // ARG {literal}` shape must abstain entirely — not naively
    // word-split the expression's own source text into bogus
    // subcommand/target pairs. `[linsert {} 0 foo bar]` evaluates to
    // the *string* "foo bar" at runtime (one call to `linsert`; "foo"/
    // "bar" are plain data words, never independently invoked) — were
    // the value's raw text naively whitespace-split instead of
    // abstained on, "foo" would land at an odd (target) index and
    // wrongly resolve to the real `::myens::foo` defined below,
    // recording a spurious command reference to it.
    let mut a = Analyser::new();
    let src = "namespace eval ::myens {\n    \
                   namespace ensemble create -subcommands {}\n\
                   }\n\
                   proc ::myens::foo {} {}\n\
                   namespace ensemble configure ::myens -map [linsert {} 0 foo bar]\n";
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.ensemble_subcommand_targets
            .get("::myens")
            .is_none_or(std::collections::HashMap::is_empty),
        "an unrecognised dynamic -map shape must record no subcommand \
             targets: {:?}",
        r.ensemble_subcommand_targets.get("::myens"),
    );
    let resolved: Vec<&str> = r
        .command_invocations
        .iter()
        .filter_map(|i| i.resolved_qualified_name.as_deref())
        .collect();
    assert!(
        !resolved.contains(&"::myens::foo"),
        "must not record a spurious command reference from splitting \
             the dynamic expression's own text: {resolved:?}",
    );
}

#[test]
fn ensemble_subcommand_targets_record_which_option_declared_them() {
    // The two options bind the subcommand word to its
    // target in opposite ways, so the recorded fact has to say which one
    // wrote it. Oracle (tclsh 8.6.14 / 9.0.4, identical): with `-map
    // {show ::app::widget::Show}` the call `::app::widget Show` is
    // `unknown or ambiguous subcommand "Show": must be show` — the key is
    // arbitrary; with `-subcommands {alpha}` the ensemble derives
    // `::app::widget::alpha` from the entry, and a `-subcommands {alpha}`
    // whose proc is named `beta` is `invalid command name "alpha"`.
    use crate::signature_scan::types::EnsembleSubcommandProvenance;

    let mut a = Analyser::new();
    let map_src = "namespace eval ::app::widget {}\n\
                       proc ::app::widget::Show {} {}\n\
                       namespace ensemble create -command ::app::widget \
                       -map {show ::app::widget::Show}\n";
    let r = a.analyse(map_src, "tcl8.6");
    let entry = r
        .ensemble_subcommand_targets
        .get("::app::widget")
        .and_then(|subs| subs.get("show"))
        .expect("the literal -map pair is recorded");
    assert_eq!(entry.target, "::app::widget::Show");
    assert_eq!(
        entry.provenance,
        EnsembleSubcommandProvenance::Map,
        "a -map pair must be tagged as such",
    );

    let mut a = Analyser::new();
    let sub_src = "namespace eval ::app::widget {\n    \
                       proc alpha {} {}\n    \
                       namespace ensemble create -command ::app::widget \
                       -subcommands {alpha}\n\
                       }\n";
    let r = a.analyse(sub_src, "tcl8.6");
    let entry = r
        .ensemble_subcommand_targets
        .get("::app::widget")
        .and_then(|subs| subs.get("alpha"))
        .expect("the literal -subcommands entry is recorded");
    assert_eq!(entry.target, "::app::widget::alpha");
    assert_eq!(
        entry.provenance,
        EnsembleSubcommandProvenance::Subcommands,
        "a -subcommands entry must be tagged as such",
    );
}

#[test]
fn ensemble_dispatch_call_sites_carry_their_mapping_provenance() {
    // The dispatch word's own invocation record carries the provenance,
    // because only the recording site knows *this span* is
    // a subcommand word: a `-map {Show ::app::widget::Show}` whose key
    // happens to equal the target's tail is textually indistinguishable
    // from an ordinary bare call to the target, so a consumer that
    // re-derived the fact by name would gate the wrong spans.
    use crate::signature_scan::types::EnsembleSubcommandProvenance;

    let mut a = Analyser::new();
    let map_src = "namespace eval ::app::widget {}\n\
                       proc ::app::widget::Show {} {}\n\
                       namespace ensemble create -command ::app::widget \
                       -map {show ::app::widget::Show}\n\
                       ::app::widget show\n";
    let r = a.analyse(map_src, "tcl8.6");
    let tagged: Vec<(&str, Option<EnsembleSubcommandProvenance>)> = r
        .command_invocations
        .iter()
        .filter(|i| i.resolved_qualified_name.as_deref() == Some("::app::widget::Show"))
        .map(|i| (i.name.as_str(), i.ensemble_dispatch))
        .collect();
    assert!(
        tagged.contains(&("show", Some(EnsembleSubcommandProvenance::Map))),
        "the dispatch word is tagged with the -map provenance: {tagged:?}",
    );
    assert!(
        tagged
            .iter()
            .any(|(name, prov)| *name == "::app::widget::Show" && prov.is_none()),
        "the -map *value* is an ordinary reference, not a dispatch word — \
             it carries the target's own name and rename must rewrite it: {tagged:?}",
    );

    let mut a = Analyser::new();
    let sub_src = "namespace eval ::app::widget {\n    \
                       proc alpha {} {}\n    \
                       namespace ensemble create -command ::app::widget \
                       -subcommands {alpha}\n\
                       }\n\
                       ::app::widget alpha\n";
    let r = a.analyse(sub_src, "tcl8.6");
    let tagged: Vec<(&str, Option<EnsembleSubcommandProvenance>)> = r
        .command_invocations
        .iter()
        .filter(|i| i.resolved_qualified_name.as_deref() == Some("::app::widget::alpha"))
        .map(|i| (i.name.as_str(), i.ensemble_dispatch))
        .collect();
    assert!(
        tagged.contains(&("alpha", Some(EnsembleSubcommandProvenance::Subcommands))),
        "the dispatch word is tagged with the -subcommands provenance: {tagged:?}",
    );
}

#[test]
fn handle_namespace_ensemble_other_options_value_word_is_not_mistaken_for_command_flag() {
    // `-map`'s value here is (pathologically, but syntactically legally)
    // the string `-command`. A scan checking *every* word for literal
    // equality with `-command`, another option's own value word
    // included, misreads that value as the
    // `-command` flag itself and steals the *next* word (the real
    // `-command`'s own flag) as if it were a namespace name. Walking
    // by each option's declared value arity instead correctly skips
    // `-map`'s whole value before ever looking for `-command` again,
    // so only the genuine `-command ::real::target` is recorded.
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "myns"));
    a.handle_namespace_ensemble(
        &[
            "ensemble".to_string(),
            "create".to_string(),
            "-map".to_string(),
            "-command".to_string(),
            "-command".to_string(),
            "::real::target".to_string(),
        ],
        &[],
        &[0],
        None,
    );
    assert_eq!(
        a.ensemble_namespaces,
        std::collections::HashSet::from(["::myns".to_string(), "::real::target".to_string()]),
        "only the genuine -command flag's value must be recorded, \
             not -map's value word that happens to read \"-command\""
    );
}

#[test]
fn handle_namespace_ensemble_global_scope_no_op() {
    let mut a = Analyser::new();
    a.handle_namespace_ensemble(
        &["ensemble".to_string(), "create".to_string()],
        &[],
        &[],
        None,
    );
    assert_eq!(a.ensemble_namespaces.len(), 0);
}

#[test]
fn handle_namespace_ensemble_wrong_subcommand_no_op() {
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "myns"));
    a.handle_namespace_ensemble(&["eval".to_string(), "myns".to_string()], &[], &[0], None);
    assert_eq!(a.ensemble_namespaces.len(), 0);
}

// handle_foreach_command

#[test]
fn handle_foreach_defines_single_loop_var() {
    let result = Analyser::new().analyse("foreach i {1 2 3} {puts $i}", "tcl");
    assert!(result.global_scope.variables.contains_key("i"));
}

#[test]
fn handle_foreach_defines_multiple_loop_vars() {
    let source = "foreach {k v} {a 1 b 2} {}";
    let result = Analyser::new().analyse(source, "tcl");
    let k = &result.global_scope.variables["k"];
    let v = &result.global_scope.variables["v"];
    assert_eq!(k.definition_span, span(9, 10));
    assert_eq!(v.definition_span, span(11, 12));
    assert!(k.definition_span.start() < v.definition_span.start());
}

#[test]
fn handle_foreach_varlist_uses_tcl_list_names_and_source_ranges() {
    let source = r#"foreach {{one name} "two name" three\ name plain} {values} {}"#;
    let result = Analyser::new().analyse(source, "tcl");
    let variables = &result.global_scope.variables;
    for (name, spelling) in [
        ("one name", "one name"),
        ("two name", "two name"),
        ("three name", r"three\ name"),
        ("plain", "plain"),
    ] {
        let start = u32::try_from(source.find(spelling).expect("original element")).unwrap();
        let end = start + u32::try_from(spelling.len()).unwrap();
        assert_eq!(variables[name].definition_span, span(start, end));
    }
}

#[test]
fn malformed_foreach_varlist_defines_no_variables() {
    let result = Analyser::new().analyse(r#"foreach "{unterminated" values {}"#, "tcl");
    assert!(result.global_scope.variables.is_empty());
}

#[test]
fn literal_foreach_binding_parses_single_variable_as_tcl_list() {
    assert_eq!(
        Analyser::literal_foreach_binding(
            r"{one name}",
            "alpha beta",
            true,
            tcl_syntax::word_rules::WordValueRules::TCL
        ),
        Some((
            "one name".to_string(),
            vec!["alpha".to_string(), "beta".to_string()]
        ))
    );
    assert!(
        Analyser::literal_foreach_binding(
            "{unterminated",
            "alpha",
            true,
            tcl_syntax::word_rules::WordValueRules::TCL
        )
        .is_none()
    );
    assert!(
        Analyser::literal_foreach_binding(
            "one two",
            "alpha",
            true,
            tcl_syntax::word_rules::WordValueRules::TCL
        )
        .is_none()
    );
}

#[test]
fn handle_foreach_defines_every_varlist_in_the_multi_list_lock_step_form() {
    let result = Analyser::new().analyse(
        "foreach dirName {src src {src core}} name {alpha beta gamma} {puts $dirName-$name}",
        "tcl",
    );
    assert!(result.global_scope.variables.contains_key("dirName"));
    assert!(result.global_scope.variables.contains_key("name"));
}

#[test]
fn handle_foreach_multi_list_form_binds_a_third_pair_too() {
    let result = Analyser::new().analyse("foreach a {1 2} b {3 4} c {5 6} {puts $a$b$c}", "tcl");
    for name in ["a", "b", "c"] {
        assert!(result.global_scope.variables.contains_key(name));
    }
}

#[test]
fn handle_foreach_too_few_args_returns_false() {
    let mut a = Analyser::new();
    let handled = a.handle_foreach_command(
        "foreach",
        &["i".to_string(), "{1 2}".to_string()],
        &[esc_tok(span(0, 1)), str_tok(span(2, 7))],
        &[],
    );
    assert!(!handled);
}

// Dynamic proc names + the foreach rename-and-reinstall idiom:
// `tk/library/accessibility.tcl`'s `foreach wtype {...} {
// rename ::$wtype ::tk::accessible::orig_$wtype ; proc ::$wtype {args}
// {...} }` renames each classic widget command away and reinstalls a
// wrapper under the same original name.

#[test]
fn handle_proc_command_dynamic_name_resolves_via_constant_fold() {
    // TP — no loop at all: a plain `set` constant. `proc ::$wtype {...}` must
    // constant-fold its name, as `rename` does, rather than register under
    // the literal garbled text.
    let mut a = Analyser::new();
    let src = "set wtype button\nproc ::$wtype {} {return ok}\n";
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.all_procs.contains_key("::button"),
        "{:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    assert!(
        !r.all_procs.keys().any(|k| k.contains('$')),
        "{:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

#[test]
fn handle_proc_command_unresolvable_dynamic_name_keeps_raw_text() {
    // TN — a genuinely dynamic name (no constant to fold against) keeps the
    // raw-text fallback behaviour; only the resolvable case folds.
    let mut a = Analyser::new();
    let src = "proc ::$wtype {} {return ok}\n";
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.all_procs.keys().any(|k| k.contains("wtype")),
        "{:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

#[test]
fn handle_foreach_rename_reinstall_idiom_resolves_every_literal_element() {
    // TP — `tk/library/accessibility.tcl` renames each classic widget
    // command away and
    // reinstalls a wrapper proc under the same name, once per element
    // of a literal `foreach` list. tclsh9.0/8.6 both prove `button`/
    // `entry` are the *new* wrapper procs afterwards; the original bodies
    // live only at `::tk::accessible::orig_button`/`orig_entry`.
    let mut a = Analyser::new();
    let src = "proc button {args} {return orig_button}\n\
                   proc entry {args} {return orig_entry}\n\
                   namespace eval ::tk::accessible {\n    \
                   foreach wtype {button entry} {\n        \
                   rename ::$wtype ::tk::accessible::orig_$wtype\n        \
                   proc ::$wtype {args} {return wrapped}\n    \
                   }\n\
                   }\n";
    let r = a.analyse(src, "tcl8.6");
    // Both wrapper redefinitions are registered, at the *same* physical
    // source location (the one templated `proc ::$wtype ...`
    // statement) — genuinely correct: there is only one place in the
    // source that defines either of them.
    let button_def = r.all_procs.get("::button").expect("::button registered");
    let entry_def = r.all_procs.get("::entry").expect("::entry registered");
    assert_eq!(button_def.body_span, entry_def.body_span);
    // No garbled `${wtype}`-named entry left behind.
    assert!(
        !r.all_procs.keys().any(|k| k.contains('$')),
        "{:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
    // The old, pre-rename bodies are reachable only via their new
    // `orig_*` names.
    assert_eq!(
        r.renamed_commands.get("::tk::accessible::orig_button"),
        Some(&"::button".to_string())
    );
    assert_eq!(
        r.renamed_commands.get("::tk::accessible::orig_entry"),
        Some(&"::entry".to_string())
    );
}

#[test]
fn handle_foreach_rename_reinstall_idiom_is_not_overfit_to_two_elements() {
    // FP guard — a third element proves the per-iteration simulation
    // isn't hardcoded to exactly the corpus's own two widget names.
    let mut a = Analyser::new();
    let src = "namespace eval ::ns {\n    \
                   foreach wtype {button entry checkbutton} {\n        \
                   rename ::$wtype ::ns::orig_$wtype\n        \
                   proc ::$wtype {args} {return wrapped}\n    \
                   }\n\
                   }\n";
    let r = a.analyse(src, "tcl8.6");
    for name in ["::button", "::entry", "::checkbutton"] {
        assert!(
            r.all_procs.contains_key(name),
            "{name} missing: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn handle_foreach_multi_pair_form_does_not_trigger_the_literal_simulation() {
    // TN — the multi-list lock-step form (`foreach v1 l1 v2 l2
    // body`) is a different arg shape; the single-pair-only literal
    // simulation must not misfire on it.
    let mut a = Analyser::new();
    let src = "foreach a {1 2} b {3 4} {\n    rename ::$a ::orig_$a\n}\n";
    let r = a.analyse(src, "tcl8.6");
    assert_eq!(r.renamed_commands.len(), 0, "{:?}", r.renamed_commands);
}

#[test]
fn handle_foreach_dynamic_list_does_not_trigger_the_literal_simulation() {
    // TN — a non-literal list (a variable, not a literal element list)
    // can't be simulated; must abstain.
    let mut a = Analyser::new();
    let src =
        "set items {button entry}\nforeach wtype $items {\n    rename ::$wtype ::orig_$wtype\n}\n";
    let r = a.analyse(src, "tcl8.6");
    assert_eq!(r.renamed_commands.len(), 0, "{:?}", r.renamed_commands);
}

#[test]
fn handle_foreach_non_idiom_body_is_unaffected() {
    // TN — an ordinary literal-list foreach with no rename/proc inside
    // must see no new side effects from the idiom-recognition code.
    let mut a = Analyser::new();
    let src = "foreach x {a b c} {\n    puts $x\n}\n";
    let r = a.analyse(src, "tcl8.6");
    assert_eq!(r.renamed_commands.len(), 0);
    assert!(!r.all_procs.keys().any(|k| k.contains('$')));
}

// handle_switch_command

fn switch_analyser() -> Analyser {
    let mut analyser = Analyser::new();
    analyser.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    analyser
}

#[test]
fn handle_switch_returns_true_for_canonical_shape() {
    let mut a = switch_analyser();
    let handled = a.handle_switch_command(
        "switch",
        &["$x".to_string(), "{a {puts a} b {puts b}}".to_string()],
        &[esc_tok(span(7, 9)), str_tok(span(10, 36))],
        &[],
    );
    assert!(handled);
}

#[test]
fn handle_switch_too_few_args_returns_false() {
    let mut a = switch_analyser();
    let handled = a.handle_switch_command("switch", &["$x".to_string()], &[], &[]);
    assert!(!handled);
}

#[test]
fn handle_switch_form1_walks_each_arm_body() {
    // Form 1: each arm body lands its `set` in the surrounding scope.
    let a = analysed("set x a\nswitch $x a {set y 1} b {set z 2}\n");
    assert!(a.result.global_scope.variables.contains_key("y"));
    assert!(a.result.global_scope.variables.contains_key("z"));
}

#[test]
fn handle_switch_form2_braced_body_walks_each_arm() {
    // Form 2: the single braced word holds all pattern/body pairs, and
    // each body is walked.
    let a = analysed("set x a\nswitch $x { a {set y 1} b {set z 2} }\n");
    assert!(a.result.global_scope.variables.contains_key("y"));
    assert!(a.result.global_scope.variables.contains_key("z"));
}

#[test]
fn handle_switch_form1_skips_fallthrough_marker() {
    // ``switch $x a - b {set y 1}`` — the ``-`` body for
    // pattern ``a`` is fall-through (next arm fires); only
    // ``b``'s body should be walked.
    let a = analysed("set x a\nswitch $x a - b {set y 1}\n");
    assert!(a.result.global_scope.variables.contains_key("y"));
}

#[test]
fn handle_switch_recognises_dashdash_options_terminator() {
    // ``switch -- $x a {set y 1}`` — ``--`` ends the option
    // section; the string arg follows.  Walker still finds
    // the arm body and lands ``y``.
    let a = analysed("set x a\nswitch -- $x a {set y 1}\n");
    assert!(a.result.global_scope.variables.contains_key("y"));
}

#[test]
fn handle_switch_dynamic_form2_body_skips_walk() {
    // Form 2 with a dynamic body (``$body`` instead of a
    // braced literal) yields no elements; the walk no-ops.
    let mut a = switch_analyser();
    let var_tok = Token::new(TokenType::Var, span(10, 15));
    a.handle_switch_command(
        "switch",
        &["$x".to_string(), "$body".to_string()],
        &[esc_tok(span(7, 9)), var_tok],
        &[],
    );
    // No body walked → no vars defined.
    assert_eq!(a.result.global_scope.variables.len(), 0);
}

// handle_catch_command

#[test]
fn handle_catch_canonical_returns_true() {
    let mut a = Analyser::new();
    let handled = a.handle_catch_command(&["body".to_string()], &[esc_tok(span(0, 4))], &[]);
    assert!(handled);
}

#[test]
fn handle_catch_with_result_var_defines_it() {
    let mut a = Analyser::new();
    // The binding positions come from the registry's VarWrite roles.
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_catch_command(
        &["body".to_string(), "res".to_string()],
        &[esc_tok(span(0, 4)), esc_tok(span(5, 8))],
        &[],
    );
    assert!(a.result.global_scope.variables.contains_key("res"));
}

#[test]
fn handle_catch_with_options_var_defines_both() {
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
    ));
    a.handle_catch_command(
        &["body".to_string(), "res".to_string(), "opts".to_string()],
        &[
            esc_tok(span(0, 4)),
            esc_tok(span(5, 8)),
            esc_tok(span(9, 13)),
        ],
        &[],
    );
    assert!(a.result.global_scope.variables.contains_key("res"));
    assert!(a.result.global_scope.variables.contains_key("opts"));
}

#[test]
fn handle_catch_no_args_returns_false() {
    let mut a = Analyser::new();
    let handled = a.handle_catch_command(&[], &[], &[]);
    assert!(!handled);
}

// try (no handler of its own: it goes through the generic dispatch `for`
// uses; `try`'s own clause-timing resolution is `clause_grammar.rs`'s
// `try_grammar_agrees_with_the_retired_walk` and its neighbours)

#[test]
fn handle_try_walks_main_body() {
    // ``try {set y 1}`` — main body walks and lands ``y``.
    let mut a = Analyser::new();
    let r = a.analyse("try {set y 1}\n", "tcl8.6");
    assert!(r.global_scope.variables.contains_key("y"));
}

#[test]
fn handle_try_walks_finally_body() {
    // ``try {} finally {set z 1}`` — finally clause body walks.
    let mut a = Analyser::new();
    let r = a.analyse("try {} finally {set z 1}\n", "tcl8.6");
    assert!(r.global_scope.variables.contains_key("z"));
}

#[test]
fn handle_try_walks_on_handler_body() {
    // ``try {} on error {result options} {set q 1}`` — the handler
    // body walks and its var-list binds both names (so completion
    // offers `$result` / `$options`).
    let mut a = Analyser::new();
    let r = a.analyse("try {} on error {result options} {set q 1}\n", "tcl8.6");
    assert!(r.global_scope.variables.contains_key("q"));
    assert!(r.global_scope.variables.contains_key("result"));
    assert!(r.global_scope.variables.contains_key("options"));
}

#[test]
fn handle_try_walks_trap_handler_body() {
    // ``try {} trap NONE {result} {set q 1}`` — same shape as ``on``,
    // but the keyword is ``trap`` and the var-list has one name.
    let mut a = Analyser::new();
    let r = a.analyse("try {} trap NONE {result} {set q 1}\n", "tcl8.6");
    assert!(r.global_scope.variables.contains_key("q"));
    assert!(r.global_scope.variables.contains_key("result"));
}

// resolve_proc_call

#[test]
fn resolve_proc_call_absolute_qualified_name() {
    // ``::foo`` resolves directly when registered.
    let mut a = Analyser::new();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    let resolved = a.resolve_proc_call("::foo", &[]);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().qualified_name, "::foo");
}

#[test]
fn resolve_proc_call_bare_name_walks_namespace_chain() {
    // ``foo`` declared inside ``ns1`` is found when resolved
    // from ``ns1``'s scope.
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "ns1"));
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[0],
    );
    // Resolve from inside ns1 — should find ::ns1::foo.
    let resolved = a.resolve_proc_call("foo", &[0]);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().qualified_name, "::ns1::foo");
}

#[test]
fn resolve_proc_call_falls_back_to_global() {
    // Bare ``foo`` declared at global is found from any scope.
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[],
    );
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "ns1"));
    // Resolve from inside ns1 — chain misses ::ns1::foo,
    // falls back to ::foo.
    let resolved = a.resolve_proc_call("foo", &[0]);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().qualified_name, "::foo");
}

#[test]
fn resolve_proc_call_qualified_relative_name() {
    // ``a::b`` (qualified but not absolute) prepends ``::``.
    let mut a = Analyser::new();
    a.result.all_procs.insert(
        "::a::b".to_string(),
        super::ProcDef {
            source_name: None,
            name: "b".to_string(),
            qualified_name: "::a::b".to_string(),
            params: Vec::new(),
            params_computed: false,
            formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                tcl_dialect::ParameterGrammar::Tcl,
            ),
            name_span: span(0, 0),
            body_span: span(0, 0),
            doc: String::new(),
            param_traits: std::collections::HashMap::new(),
            caller_frame_params: std::collections::HashSet::new(),
            caller_frame_literals: std::collections::HashMap::new(),
        },
    );
    let resolved = a.resolve_proc_call("a::b", &[]);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().qualified_name, "::a::b");
}

#[test]
fn resolve_proc_call_relative_dotted_name_prefers_current_namespace() {
    // A relative dotted word (`ns2::inner`, containing `::` but not
    // starting with it) must resolve against the current namespace
    // first, not be rooted straight at global (confirmed against tclsh
    // 9.0.4 — see `bareword_resolution_candidates`).
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    for qname in ["::ns2::inner", "::ns::ns2::inner"] {
        a.result.all_procs.insert(
            qname.to_string(),
            super::ProcDef {
                source_name: None,
                name: "inner".to_string(),
                qualified_name: qname.to_string(),
                params: Vec::new(),
                params_computed: false,
                formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                    tcl_dialect::ParameterGrammar::Tcl,
                ),
                name_span: span(0, 0),
                body_span: span(0, 0),
                doc: String::new(),
                param_traits: std::collections::HashMap::new(),
                caller_frame_params: std::collections::HashSet::new(),
                caller_frame_literals: std::collections::HashMap::new(),
            },
        );
    }
    a.result
        .global_scope
        .children
        .push(Scope::new(ScopeKind::Namespace, "ns"));
    let resolved = a.resolve_proc_call("ns2::inner", &[0]);
    assert_eq!(
        resolved.unwrap().qualified_name,
        "::ns::ns2::inner",
        "must prefer the current-namespace proc over the root one",
    );
    // Falls back to the root proc when there is no current-namespace
    // candidate.
    let resolved = a.resolve_proc_call("ns2::inner", &[]);
    assert_eq!(resolved.unwrap().qualified_name, "::ns2::inner");
}

#[test]
fn resolve_proc_call_does_not_walk_ancestor_namespaces() {
    // Real Tcl bareword resolution is exactly two levels — current
    // namespace, then global — absent an explicit `namespace path`. A
    // proc defined in a *grandparent* namespace must not resolve for a
    // bare call from a nested `::a::b::c` scope.
    use crate::analyser::types::{Scope, ScopeKind};
    let mut a = Analyser::new();
    let mut ns_a = Scope::new(ScopeKind::Namespace, "a");
    let mut ns_b = Scope::new(ScopeKind::Namespace, "b");
    ns_b.children.push(Scope::new(ScopeKind::Namespace, "c"));
    ns_a.children.push(ns_b);
    a.result.global_scope.children.push(ns_a);
    // scope_path [0] = ::a — define `foo` there.
    a.handle_proc_command(
        "proc",
        &["foo".to_string(), String::new(), String::new()],
        &[
            esc_tok(span(5, 8)),
            str_tok(span(9, 11)),
            str_tok(span(12, 14)),
        ],
        &[],
        &[0],
    );
    // Resolving from ::a::b::c (scope_path [0, 0, 0]) must NOT reach
    // the grandparent's ::a::foo.
    assert!(
        a.resolve_proc_call("foo", &[0, 0, 0]).is_none(),
        "a grandparent-namespace proc must not resolve for a bare call",
    );
    // Control: resolving directly from ::a still finds it.
    assert!(a.resolve_proc_call("foo", &[0]).is_some());
}

#[test]
fn resolve_proc_call_unknown_name_returns_none() {
    let a = Analyser::new();
    assert!(a.resolve_proc_call("nope", &[]).is_none());
}

#[test]
fn resolve_proc_call_empty_name_returns_none() {
    let a = Analyser::new();
    assert!(a.resolve_proc_call("", &[]).is_none());
}

// handle_interp_alias

#[test]
fn handle_interp_alias_records_canonical_form() {
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_interp_alias(
        &[
            "alias".to_string(),
            String::new(),
            "myset".to_string(),
            String::new(),
            "set".to_string(),
        ],
        &[],
        42,
    );
    assert!(a.command_aliases.contains_key("::myset"));
    assert!(a.result.command_aliases.contains_key("::myset"));
    let (target, prepended) = &a.command_aliases["::myset"];
    assert_eq!(target, "set");
    assert_eq!(prepended.as_slice(), [] as [String; 0]);
    assert_eq!(a.alias_offsets.get("::myset"), Some(&42));
    // The offset is also promoted onto the finalised `AnalysisResult`
    // (not just the in-progress `Analyser`), so a cross-document
    // consumer such as `tcl_lsp_core::WorkspaceIndex` can tell an
    // unconditional alias apart from one nested in a proc/class body.
    assert_eq!(a.result.alias_offsets.get("::myset"), Some(&42));
}

#[test]
fn handle_interp_alias_with_prepended_args() {
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_interp_alias(
        &[
            "alias".to_string(),
            String::new(),
            "logerr".to_string(),
            String::new(),
            "puts".to_string(),
            "stderr".to_string(),
        ],
        &[],
        0,
    );
    let (target, prepended) = &a.command_aliases["::logerr"];
    assert_eq!(target, "puts");
    assert_eq!(prepended, &vec!["stderr".to_string()]);
}

#[test]
fn handle_interp_alias_wrong_shape_no_op() {
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_interp_alias(&["alias".to_string()], &[], 0);
    assert_eq!(a.command_aliases.len(), 0);
    assert_eq!(a.alias_offsets.len(), 0);
}

// resolve_dynamic_word

#[test]
fn resolve_dynamic_word_braced_literal_dollar_is_not_mistaken_for_dynamic() {
    // A brace-quoted word containing a literal `$`/`[` character is
    // legitimately STATIC — Tcl suppresses substitution inside
    // `{}` — so this must return it as-is via `resolve_const_word`'s
    // `Str`/`Esc` branch, even though `is_dynamic_word`'s naive text
    // scan alone would call it dynamic.
    let a = Analyser::new();
    let tok = Token {
        kind: TokenType::Str,
        span: span(0, 10),
        content_offset: 1,
        in_quote: false,
    };
    assert_eq!(
        a.resolve_dynamic_word("::pkg::$c", Some(tok), true, &[]),
        Some("::pkg::$c".to_string())
    );
}

#[test]
fn resolve_dynamic_word_missing_token_is_none() {
    // A dynamic word with no token to inspect (the handful of
    // existing handle_rename unit tests pass `arg_tokens: &[]`)
    // must fall through to None, not panic.
    let a = Analyser::new();
    assert_eq!(a.resolve_dynamic_word("$x", None, false, &[]), None);
}

// handle_rename

#[test]
fn handle_rename_records_static_move() {
    let mut a = Analyser::new();
    let dynamic = a.handle_rename(
        &["target".to_string(), "target_orig".to_string()],
        &[],
        &[],
        &[],
        42,
    );
    assert!(!dynamic, "a fully static rename is not dynamic");
    assert_eq!(
        a.renamed_commands.get("::target_orig").map(String::as_str),
        Some("::target")
    );
    assert_eq!(
        a.result
            .renamed_commands
            .get("::target_orig")
            .map(String::as_str),
        Some("::target")
    );
    assert_eq!(a.rename_offsets.get("::target_orig"), Some(&42));
    // The old name is recorded as deleted from this offset —
    // confirmed against tclsh 9.0.4: `target` becomes an unknown
    // command, not still callable under its original arity.
    assert_eq!(a.deleted_commands.get("::target"), Some(&42));
}

#[test]
fn handle_rename_deletion_records_old_name_as_deleted() {
    // `rename OLD {}` deletes OLD — no new binding to record (it is
    // not a "dynamic" rename either, nothing to widen for), but OLD
    // itself must be recorded as gone (confirmed against tclsh
    // 9.0.4: also "invalid command name" afterwards).
    let mut a = Analyser::new();
    let dynamic = a.handle_rename(&["target".to_string(), String::new()], &[], &[], &[], 7);
    assert!(!dynamic);
    assert_eq!(a.renamed_commands.len(), 0);
    assert_eq!(a.deleted_commands.get("::target"), Some(&7));
}

#[test]
fn handle_rename_dynamic_old_name_reports_dynamic() {
    let mut a = Analyser::new();
    let dynamic = a.handle_rename(&["$x".to_string(), "y".to_string()], &[], &[], &[], 0);
    assert!(dynamic, "rename $x y cannot be resolved statically");
    assert_eq!(a.renamed_commands.len(), 0);
    assert_eq!(a.deleted_commands.len(), 0);
}

#[test]
fn handle_rename_dynamic_new_name_reports_dynamic() {
    let mut a = Analyser::new();
    let dynamic = a.handle_rename(&["x".to_string(), "y[z]".to_string()], &[], &[], &[], 0);
    assert!(dynamic, "rename x y[z] cannot be resolved statically");
    assert_eq!(a.renamed_commands.len(), 0);
    assert_eq!(a.deleted_commands.len(), 0);
}

#[test]
fn handle_rename_wrong_shape_no_op() {
    let mut a = Analyser::new();
    let dynamic = a.handle_rename(&["onlyone".to_string()], &[], &[], &[], 0);
    assert!(!dynamic);
    assert_eq!(a.renamed_commands.len(), 0);
    assert_eq!(a.deleted_commands.len(), 0);
}

// handle_source_command

#[test]
fn handle_source_command_resolves_a_same_file_constant_variable() {
    // The simplest possible case: a straight-line `set`, zero branches, zero
    // external input, immediately followed by `source $var` — the real
    // corpus's `set p "e.tcl"; source $p` shape, resolved through the
    // same constant-string lattice `rename`'s OLD/NEW words use, via the
    // shared `resolve_dynamic_word`.
    let mut a = Analyser::new();
    let r = a.analyse("namespace eval ::z { set p \"e.tcl\"; source $p }\n", "tcl");
    let target = r
        .source_targets
        .iter()
        .find(|s| s.raw_path == "e.tcl")
        .unwrap_or_else(|| panic!("resolved literal not found: {:?}", r.source_targets));
    assert!(target.is_literal, "{target:?}");
}

#[test]
fn handle_source_command_resolves_a_concatenated_constant_variable() {
    // A multi-token word (`${base}.tcl`, a Var fragment concatenated
    // with a literal) goes through `fold_interpolation_single` rather
    // than the single-token `resolve_const_word` fast path — covered
    // separately since it's a different code path inside
    // `resolve_dynamic_word`.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::z { set base \"helper\"; source ${base}.tcl }\n",
        "tcl",
    );
    let target = r
        .source_targets
        .iter()
        .find(|s| s.raw_path == "helper.tcl")
        .unwrap_or_else(|| panic!("resolved literal not found: {:?}", r.source_targets));
    assert!(target.is_literal, "{target:?}");
}

#[test]
fn handle_source_command_a_dynamic_variable_with_no_known_value_stays_dynamic() {
    // A proc parameter is never constant-tracked (the same limitation
    // `resolve_dynamic_word` already documents for `rename`) — `source
    // $p` here must stay conservatively dynamic rather than guess.
    let mut a = Analyser::new();
    let r = a.analyse("proc f {p} { source $p }\n", "tcl");
    assert_eq!(r.source_targets.len(), 1, "{:?}", r.source_targets);
    assert!(!r.source_targets[0].is_literal, "{:?}", r.source_targets[0]);
}

#[test]
fn handle_source_command_leaves_a_bracket_wrapped_variable_dynamic() {
    // A same-file constant wrapped inside a `[file join ...]` command
    // substitution — the real corpus's `source [file join $edir
    // extra.tcl]` shape — can't be folded by the same-word constant
    // lattice (`fold_interpolation_single` rejects any word containing
    // `[` outright, by design — a command substitution can have
    // arbitrary side effects), and `evaluate_auto_path_expr`'s
    // separate `[info script]`-anchored folder doesn't evaluate `$var`
    // at all.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::z { set dir \"lib\"; source [file join $dir extra.tcl] }\n",
        "tcl",
    );
    assert_eq!(r.source_targets.len(), 1, "{:?}", r.source_targets);
    assert!(!r.source_targets[0].is_literal, "{:?}", r.source_targets[0]);
}

// The `INSTALLS_NAMED_DEFINITION` re-dispatch contract.

#[test]
fn every_installer_spec_lands_on_a_live_redispatch_arm() {
    // Drift gate — the trait is a *promise* that
    // `simulate_remaining_foreach_iterations` re-runs the command once
    // per literal `foreach` element.  A spec that carries it but whose
    // analyser hook has no arm in that match silently promises nothing
    // (`oo::objdefine` is exactly that shape).
    // Registry-driven, so a newly-stamped spec fails here rather than
    // in a corpus months later.
    use tcl_registry::hooks::AnalyserHookId as Hook;
    let redispatched = [Hook::Proc, Hook::Rename, Hook::OoDefine, Hook::OoObjdefine];
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut checked = 0usize;
    for name in registry.command_names() {
        for spec in registry.specs(name) {
            if !spec
                .traits
                .contains(tcl_registry::Traits::INSTALLS_NAMED_DEFINITION)
            {
                continue;
            }
            checked += 1;
            let hook = spec.analyser_hook.unwrap_or_else(|| {
                panic!("{name} carries INSTALLS_NAMED_DEFINITION but has no analyser hook")
            });
            assert!(
                redispatched.contains(&hook),
                "{name} carries INSTALLS_NAMED_DEFINITION but its hook {hook:?} has no \
                     arm in the foreach re-dispatch — the trait promises a re-dispatch \
                     the match does not deliver",
            );
        }
    }
    assert!(
        checked >= 4,
        "expected proc/rename/oo::define/oo::objdefine, saw {checked}"
    );
}

#[test]
fn foreach_objdefine_records_per_object_facts_for_every_literal_element() {
    // TP — the sibling shape.  tclsh 9.0.4 and 8.6.16
    // both run `foreach o {::a ::b} { oo::objdefine $o { method probe
    // {} {…} } }` and report `probe` in `info object methods` for
    // *both* objects, so both must carry the per-object method fact.
    let mut a = Analyser::new();
    let r = a.analyse(
            "oo::class create Base { method hello {} { return hi } }\n\
             Base create ::a\n\
             Base create ::b\n\
             foreach o {::a ::b} {\n    oo::objdefine $o {\n        method probe {} { return probed }\n    }\n}\n",
            "tcl9.0",
        );
    for obj in ["::a", "::b"] {
        let methods = r
            .object_methods
            .get(obj)
            .unwrap_or_else(|| panic!("{obj} has per-object methods: {:?}", r.object_methods));
        assert!(
            methods.iter().any(|m| m.def.name == "probe"),
            "{obj} gained probe: {methods:?}",
        );
    }
}

#[test]
fn foreach_objdefine_does_not_double_record_the_first_element() {
    // FP guard — the ordinary body walk covers the first element and
    // the simulation covers the rest, so the first element's site is
    // visited twice under the loop variable's own key.  One source site
    // can never declare the same member twice, so it must appear once.
    let mut a = Analyser::new();
    let r = a.analyse(
            "oo::class create Base { method hello {} { return hi } }\n\
             Base create ::a\n\
             Base create ::b\n\
             foreach o {::a ::b} {\n    oo::objdefine $o {\n        method probe {} { return probed }\n    }\n}\n",
            "tcl9.0",
        );
    for (key, methods) in &r.object_methods {
        let probes = methods.iter().filter(|m| m.def.name == "probe").count();
        assert!(
            probes <= 1,
            "{key} recorded probe {probes} times: {methods:?}",
        );
    }
}

#[test]
fn foreach_objdefine_does_not_duplicate_body_diagnostics() {
    // FP guard — re-running the installer per element re-walks the
    // per-object method bodies, so a diagnostic raised inside one must
    // still be reported once per site, never once per iteration.
    let mut a = Analyser::new();
    let r = a.analyse(
            "oo::class create Base { method hello {} { return hi } }\n\
             Base create ::a\n\
             Base create ::b\n\
             foreach o {::a ::b} {\n    oo::objdefine $o {\n        method probe {} { return $undefinedVar }\n    }\n}\n",
            "tcl9.0",
        );
    let mut seen = std::collections::HashSet::new();
    for diag in &r.diagnostics {
        assert!(
            seen.insert((diag.code.to_string(), diag.span)),
            "duplicate diagnostic {:?} at {:?}",
            diag.code,
            diag.span,
        );
    }
}

#[test]
fn foreach_objdefine_over_a_dynamic_list_abstains() {
    // TN — a runtime element list names no object the walk can know, so
    // the per-object facts stay under the loop variable's own key and
    // are never attributed to a literal object.
    let mut a = Analyser::new();
    let r = a.analyse(
            "oo::class create Base { method hello {} { return hi } }\n\
             Base create ::a\n\
             foreach o $objects {\n    oo::objdefine $o {\n        method probe {} { return probed }\n    }\n}\n",
            "tcl9.0",
        );
    assert!(
        !r.object_methods.contains_key("::a"),
        "a runtime receiver must not be attributed to a literal object: {:?}",
        r.object_methods,
    );
    assert!(
        r.object_methods.contains_key("o"),
        "the facts stay under the receiver variable: {:?}",
        r.object_methods,
    );
}

// handle_oo_objdefine

/// `oo::objdefine` records its receiver word: a variable by its name,
/// braced or not, and a literal object by its spelling. tclsh 8.6.18 and
/// 9.0.4 print `1 2 3` for the three forms over one object.
#[test]
fn handle_oo_objdefine_records_its_receiver_word() {
    for receiver in ["$obj", "${obj}", "obj"] {
        let a = analysed(&format!(
            "oo::object create obj\nset obj obj\noo::objdefine {receiver} {{method m {{}} {{return 1}}}}\n"
        ));
        assert!(a.objdefined_vars.contains("obj"), "{receiver}");
    }
}

#[test]
fn oo_objdefine_body_methods_are_analysed() {
    // Before: the `oo::objdefine` body was never parsed, so nothing inside
    // a per-object method resolved.  Now the body walks like any method
    // body — the call it makes is recorded as an invocation.
    let mut a = Analyser::new();
    let src = "proc helper {} {}\n\
                   oo::class create Foo {}\n\
                   set o [Foo new]\n\
                   oo::objdefine $o {\n    \
                       method greet {} { helper }\n\
                   }\n";
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.command_invocations.iter().any(|i| i.name == "helper"),
        "the call inside the per-object method body should be analysed: {:?}",
        r.command_invocations
            .iter()
            .map(|i| &i.name)
            .collect::<Vec<_>>(),
    );
}

// handle_oo_class_command

#[test]
fn handle_oo_class_create_records_class() {
    let a = analysed("oo::class create MyClass\n");
    assert!(a.result.all_classes.contains_key("::MyClass"));
    let cls = &a.result.all_classes["::MyClass"];
    assert_eq!(cls.name, "MyClass");
}

#[test]
fn handle_oo_class_create_with_body() {
    let src = "oo::class create MyClass {method m {} {}}\n";
    let a = analysed(src);
    let open = u32::try_from(src.find('{').unwrap()).unwrap();
    assert_eq!(
        a.result.all_classes["::MyClass"].body_span,
        span(open, open + 15)
    );
}

#[test]
fn handle_oo_class_wrong_subcommand_returns_false() {
    let mut a = Analyser::new();
    let handled = a.handle_oo_class_command(
        "oo::class",
        &["destroy".to_string(), "MyClass".to_string()],
        &[
            esc_tok(span(0, 9)),
            esc_tok(span(10, 17)),
            esc_tok(span(18, 25)),
        ],
        &[],
        esc_tok(span(0, 9)),
    );
    assert!(!handled);
    assert_eq!(a.result.all_classes.len(), 0);
}

// handle_oo_class_command body walking

#[test]
fn analyse_oo_class_body_records_superclass_and_methods() {
    // End-to-end: ``oo::class create Sub`` with a body
    // declaring a superclass and a method.  After analyse
    // ``::Sub`` should carry both fields.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "oo::class create Sub { superclass ::Base\nmethod greet {} { puts hi } }",
        "tcl",
    );
    assert!(r.all_classes.contains_key("::Sub"));
    let cls = &r.all_classes["::Sub"];
    assert_eq!(cls.superclasses, vec!["::Base"]);
    assert!(cls.methods.contains_key("greet"));
    assert_eq!(cls.methods["greet"].kind, "method");
}

#[test]
fn analyse_oo_class_body_records_classmethod_and_mixin() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "oo::class create C { mixin ::M\nclassmethod build {} { return ok } }",
        "tcl",
    );
    let cls = &r.all_classes["::C"];
    assert_eq!(cls.mixins, vec!["::M"]);
    assert!(cls.class_methods.contains_key("build"));
    assert!(!cls.methods.contains_key("build"));
}

// handle_oo_define_command body walking

#[test]
fn analyse_oo_define_body_extends_existing_class() {
    // ``oo::class create C {}`` followed by ``oo::define
    // C { method m {} {} }`` — the method ends up in the
    // already-recorded class.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "oo::class create C {}\noo::define C { method m {} {} }",
        "tcl",
    );
    assert!(r.all_classes.contains_key("::C"));
    let cls = &r.all_classes["::C"];
    assert!(cls.methods.contains_key("m"));
}

#[test]
fn oo_define_extending_an_existing_class_adds_its_own_class_body_span() {
    // `class_body_spans` must record BOTH the
    // creation site's own span AND the separate `oo::define` block's
    // own span for the same qualified class — a class extended via a
    // *separate* `oo::define ClassName { ... }` block has textually
    // disjoint body spans, not one contiguous range, and `my`-dispatch
    // resolution (a lexical "which class's body am I inside" query)
    // needs every contributing span, not just the first one recorded.
    let mut a = crate::analyser::Analyser::new();
    let src = "oo::class create C {}\noo::define C { method m {} {} }";
    let r = a.analyse(src, "tcl");
    let spans: Vec<tcl_lexer::Span> = r
        .class_body_spans
        .iter()
        .filter(|(name, _)| name == "::C")
        .map(|(_, span)| *span)
        .collect();
    assert_eq!(spans.len(), 2, "{spans:?}");
    // One of the two recorded spans must cover the `oo::define`
    // block's own `method m {} {}` text — the token whose containment
    // `enclosing_class_at` checks for a cursor inside it.
    let inside_define_block = u32::try_from(src.find("method m").unwrap()).unwrap();
    assert!(
        spans
            .iter()
            .any(|span| span.start() <= inside_define_block && inside_define_block < span.end()),
        "no recorded span covers the oo::define block's own body: {spans:?}"
    );
}

#[test]
fn oo_define_extends_class_named_like_a_create_subcommand() {
    // `oo::define` / `oo::objdefine` carry a TclOO
    // `definition_body` but no `IS_OO_METACLASS` trait, so a class literally
    // named `create` must be *extended* by `handle_oo_define_command`, not
    // stolen by `handle_oo_class_command` (which would record a bogus class
    // named `method`).  The class-creator check requires the metaclass
    // trait, which define/objdefine lack.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("oo::define create method greet {} { return hi }", "tcl");
    // The real class is `create`, with method `greet` — not a bogus class.
    let cls = r
        .all_classes
        .get("::create")
        .expect("class `create` recorded by oo::define");
    assert!(
        cls.methods.contains_key("greet"),
        "method greet must be recorded on class `create`: {:?}",
        cls.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        !r.all_classes.contains_key("::method"),
        "no bogus class named `method` should be created",
    );
}

#[test]
fn analyse_oo_define_inline_form_extends_class() {
    // ``oo::define C method m {} {}`` — inline form,
    // single subcommand.  Works whether or not the class
    // was already declared (creates a stub if absent).
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("oo::define MyClass method greet {} { puts hi }", "tcl");
    assert!(r.all_classes.contains_key("::MyClass"));
    let cls = &r.all_classes["::MyClass"];
    assert!(cls.methods.contains_key("greet"));
}

#[test]
fn two_dynamic_oo_define_targets_sharing_a_variable_name_never_merge() {
    // TP — clay.tcl's `current_class`-based Ensemble DSL shape: two lexically
    // unrelated procs each write `oo::define $class method ... ` with a
    // same-named local `class` (an unremarkable choice for this exact
    // idiom) and each add their own method. Keying such a call on the raw
    // text `$class` gives both calls the identical key, so the second
    // `remove`+`insert` round-trip silently merges the first proc's
    // method into the second's ClassDef (and vice versa via the
    // dual-registration into `scope.classes`), producing a document
    // symbol whose child method range sits outside its own parent's.
    // The targets here are *parameters*, so no constant dominates
    // either call and both stay genuinely dynamic — the shape this
    // guard is about.  (A target bound to a dominating constant is a
    // different, statically-resolvable case: see
    // `oo_define_resolves_a_dominating_constant_target`.)
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc addFoo {class} {\n    oo::define $class method foo {} { return foo }\n}\n\
             proc addBar {class} {\n    oo::define $class method bar {} { return bar }\n}\n",
        "tcl8.6",
    );
    let synthetic: Vec<_> = r
        .all_classes
        .iter()
        .filter(|(k, _)| k.contains("@dynclass@"))
        .collect();
    assert_eq!(
        synthetic.len(),
        2,
        "each dynamic oo::define call site gets its own ClassDef: {:?}",
        r.all_classes.keys().collect::<Vec<_>>(),
    );
    let has_foo_only = synthetic
        .iter()
        .any(|(_, c)| c.methods.contains_key("foo") && !c.methods.contains_key("bar"));
    let has_bar_only = synthetic
        .iter()
        .any(|(_, c)| c.methods.contains_key("bar") && !c.methods.contains_key("foo"));
    assert!(
        has_foo_only && has_bar_only,
        "neither call site's method may leak into the other's ClassDef: {:?}",
        synthetic
            .iter()
            .map(|(k, c)| (k, c.methods.keys().collect::<Vec<_>>()))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn dynamic_oo_define_target_does_not_touch_a_same_named_literal_class() {
    // FP guard — a class whose *written* name matches nothing knowable
    // (`$class` bound to a parameter) must never be found/extended
    // through a literal class that happens to share the variable's
    // source text: the synthetic key can never collide with a real
    // qualified name.  A target bound to a dominating constant *is*
    // resolvable and does extend the real class — tclsh 9.0.4/8.6.16
    // agree — which is
    // `oo_define_resolves_a_dominating_constant_target`'s case, not
    // this one.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Widget {\n    method real {} {}\n}\n\
             proc addDynamic {class} {\n    oo::define $class method dynamic {} {}\n}\n",
        "tcl8.6",
    );
    let widget = &r.all_classes["::Widget"];
    assert!(widget.methods.contains_key("real"), "{widget:?}");
    assert!(
        !widget.methods.contains_key("dynamic"),
        "the dynamic call must not extend the literal same-named class: {widget:?}",
    );
}

// handle_oo_define_command

#[test]
fn handle_oo_define_recognises_canonical_form() {
    let mut a = Analyser::new();
    let handled = a.handle_oo_define_command(
        "oo::define",
        &["MyClass".to_string(), "method m {} {}".to_string()],
        &[],
        &[],
        &[],
    );
    assert!(handled);
}

#[test]
fn handle_oo_define_no_args_returns_false() {
    let mut a = Analyser::new();
    let handled = a.handle_oo_define_command("oo::define", &[], &[], &[], &[]);
    assert!(!handled);
}

// `incr` / `append` / `lappend` / `dict` — bound by role

#[test]
fn incr_binds_its_target_by_role() {
    let a = analysed("incr counter\n");
    assert!(a.result.global_scope.variables.contains_key("counter"));
    // A read-modify-write target is read before it is written, so the
    // binding is a side effect, never a "set but unused" write.
    assert!(!a.result.global_scope.variables["counter"].warn_if_unused);
}

#[test]
fn handle_incr_with_amount() {
    let a = analysed("incr counter 5\n");
    assert!(a.result.global_scope.variables.contains_key("counter"));
    assert!(!a.result.global_scope.variables.contains_key("5"));
}

#[test]
fn handle_incr_no_args_no_op() {
    let mut a = Analyser::new();
    dispatch_words(&mut a, &["incr"]);
    assert!(a.result.global_scope.variables.is_empty());
}

#[test]
fn a_written_array_element_binds_its_array_and_a_computed_name_binds_nothing() {
    // `incr hits($word)` writes an element of `hits` whatever the key;
    // `append $name x` and `incr $count` write a variable no static name
    // spells.
    let a = analysed(
        "set word k\nincr hits($word)\nset name v\nappend $name x\n\
             set count n\nset n 1\nincr $count\nlappend items x\n",
    );
    assert!(a.result.global_scope.variables.contains_key("hits"));
    assert!(a.result.global_scope.variables.contains_key("items"));
    assert!(!a.result.global_scope.variables.contains_key("v"));
    assert!(!a.result.global_scope.variables.contains_key("$name"));
    assert!(!a.result.global_scope.variables.contains_key("$count"));
}

#[test]
fn dict_for_and_dict_update_bind_their_loop_variables_by_role() {
    let a = analysed("set d {}\ndict for {k v} $d {}\ndict update d key w {}\n");
    for name in ["k", "v", "w"] {
        assert!(
            a.result.global_scope.variables.contains_key(name),
            "{name}: {:?}",
            a.result.global_scope.variables.keys().collect::<Vec<_>>()
        );
    }
    // Neither the key word nor the dictionary variable is a loop variable.
    assert!(!a.result.global_scope.variables.contains_key("key"));
}

// ClassDef extended fields + UnknownProcInfo

#[test]
fn analyse_oo_class_records_metaclass_from_command_name() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("oo::class create C {}", "tcl");
    let cls = &r.all_classes["::C"];
    assert_eq!(cls.metaclass, "oo::class");
}

#[test]
fn analyse_oo_class_body_records_constructors_and_destructor() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "oo::class create C { constructor args { puts ctor }\ndestructor { puts dtor } }",
        "tcl",
    );
    let cls = &r.all_classes["::C"];
    assert_eq!(cls.constructors.len(), 1);
    assert_eq!(cls.constructors[0].kind, "constructor");
    assert!(cls.destructor.is_some());
    assert_eq!(cls.destructor.as_ref().unwrap().kind, "destructor");
}

#[test]
fn analyse_oo_class_body_records_variables_filters_exports() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "oo::class create C { variable x y\nfilter log\nexport foo bar\nunexport hidden }",
        "tcl",
    );
    let cls = &r.all_classes["::C"];
    assert_eq!(cls.variables, vec!["x", "y"]);
    assert_eq!(cls.filters, vec!["log"]);
    assert!(cls.exports.contains("foo"));
    assert!(cls.exports.contains("bar"));
    assert!(cls.unexports.contains("hidden"));
}

#[test]
fn analyse_oo_class_body_records_property_def() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "oo::class create C { property colour -kind readwrite -get { return red } }",
        "tcl",
    );
    let cls = &r.all_classes["::C"];
    let pd = cls.properties.get("colour").expect("colour recorded");
    assert_eq!(pd.kind, "readwrite");
    assert!(pd.has_getter);
    assert!(!pd.has_setter);
}

#[test]
fn analyse_unknown_proc_records_dispatch_targets_end_to_end() {
    // End-to-end: a ``proc unknown {cmd args} {...}`` with
    // an exact-match switch should populate
    // ``result.unknown_proc_info`` with the arm labels as
    // dispatch targets.  This is what gates W123.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } bar { return 2 } } }",
        "tcl",
    );
    let info = r.unknown_proc_info.expect("unknown_proc_info populated");
    assert!(!info.empty_stub);
    assert!(info.dispatch_targets.contains("foo"));
    assert!(info.dispatch_targets.contains("bar"));
}

#[test]
fn analyse_without_unknown_proc_leaves_unknown_proc_info_none() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("proc foo {} { return 1 }", "tcl");
    assert!(r.unknown_proc_info.is_none());
}

#[test]
fn analyse_unknown_proc_with_empty_body_marks_empty_stub() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("proc unknown {cmd args} {}", "tcl");
    let info = r.unknown_proc_info.expect("unknown_proc_info populated");
    assert!(info.empty_stub);
}

/// `conditional_depth` is driven by
/// `Traits::BRANCH_SELECTED_BODY`, so exactly the branch-selected bodies
/// mark a `package require` conditional.
///
/// `if` and `try` bodies are branch-selected: at most one runs, chosen at
/// run time, so nothing inside dominates the code after the command. A
/// `while` body is skippable *and* repeatable — a different question,
/// owned by `control_flow_body_depth` — and does **not** mark the require
/// conditional. A top-level require is unconditional.
#[test]
fn package_require_conditionality_follows_the_branch_selected_body_trait() {
    let conditional_flags = |src: &str| -> Vec<bool> {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, "tcl8.6")
            .package_requires
            .iter()
            .map(|p| p.conditional)
            .collect()
    };
    assert_eq!(conditional_flags("package require Tcl 8.6\n"), vec![false]);
    assert_eq!(
        conditional_flags("if {$x} { package require Tcl 8.6 }\n"),
        vec![true],
    );
    assert_eq!(
        conditional_flags("while {$x} { package require Tcl 8.6 }\n"),
        vec![false],
        "a loop body is skippable-and-repeatable, a different question",
    );
    // `try`'s bodies take the generic body walk at their clause's timing:
    // the protected body is a guarded probe, so it is conditional like
    // `if`'s.
    assert_eq!(
        conditional_flags("try { package require Tcl 8.6 } on error {} {}\n"),
        vec![true],
    );
}

/// The generic body walk raises the branch-selected depth per clause
/// timing (`body_depths`), so a `package require` records the right
/// conditionality wherever in a `try` it sits.
///
/// Clause semantics are C Tcl's (Tcl 9.0.4 `try(n)`, `TclNRTryObjCmd` in
/// `generic/tclCmdMZ.c`): the main body may be cut short by an exception a
/// handler swallows and the `on`/`trap` handlers run only on a match — both
/// branch-selected — while `finally` always runs, so it is not.
#[test]
fn package_require_conditionality_per_try_clause_kind() {
    let conditional_flags = |src: &str| -> Vec<bool> {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, "tcl8.6")
            .package_requires
            .iter()
            .map(|p| p.conditional)
            .collect()
    };
    // TP — the guarded optional-dependency idiom.
    assert_eq!(
        conditional_flags("try { package require Foo } on error {} {}\n"),
        vec![true],
        "a require in the main try body is conditional",
    );
    // TP — an `on` handler body.
    assert_eq!(
        conditional_flags("try { set x 1 } on error {} { package require Foo }\n"),
        vec![true],
        "a require in an `on` handler is conditional",
    );
    // TP — a `trap` handler body.
    assert_eq!(
        conditional_flags("try { set x 1 } trap {POSIX ENOENT} {m o} { package require Foo }\n"),
        vec![true],
        "a require in a `trap` handler is conditional",
    );
    // TN — a `finally` body always runs, so it is not branch-selected.
    assert_eq!(
        conditional_flags("try { set x 1 } finally { package require Foo }\n"),
        vec![false],
        "a `finally` body always runs — the require is unconditional",
    );
    // TN — a top-level require, the control.
    assert_eq!(conditional_flags("package require Foo\n"), vec![false]);
    // FN guard — the depth is restored after the walk, so a require that
    // follows the whole `try` command is still unconditional.
    assert_eq!(
        conditional_flags(
            "try { package require A } on error {} { package require B } finally { package require C }\npackage require D\n"
        ),
        vec![true, true, false, false],
        "main body + handler conditional; finally and the following \
             top-level require unconditional",
    );
    // FP guard — nesting a `try` inside an `if` must not leave the depth
    // stuck: the require after both is still unconditional.
    assert_eq!(
        conditional_flags(
            "if {$x} { try { package require A } finally { set y 1 } }\npackage require B\n"
        ),
        vec![true, false],
    );
}

/// `for`'s bodies take the generic body walk at their clause's timing
/// (D2.55), the reading the retired `handle_for_command` gave: `start`
/// runs once, at the enclosing depth; `next` and the body run per
/// iteration, at a control-flow depth. None is branch-selected.
#[test]
fn for_start_runs_at_the_enclosing_depth_and_next_and_body_per_iteration() {
    let mut a = crate::analyser::Analyser::new();
    let depths: Vec<(bool, bool)> = a
        .analyse(
            "for {package require A} {$i < 3} {package require B} {package require C}\n\
                 package require D\n",
            "tcl8.6",
        )
        .package_requires
        .iter()
        .map(|p| (p.conditional, p.control_flow))
        .collect();
    assert_eq!(
        depths,
        vec![(false, false), (false, true), (false, true), (false, false)],
        "start straight-line; next and body control flow; the depth \
             restored after the loop",
    );
}

/// A `try` handler body is `Selected` — conditional and control flow — so
/// a `rename` there may never run and is not a straight-line deletion,
/// as one inside a loop body is not. A
/// `finally` body always runs, so its `rename` still is one.
#[test]
fn a_rename_in_a_try_handler_is_not_a_straight_line_deletion() {
    let destroys_foo = |src: &str| {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, "tcl8.6")
            .destroyed_commands
            .contains_key("::foo")
    };
    assert!(!destroys_foo(
        "proc foo {} {}\ntry { set x 1 } on error {} { rename foo {} }\n"
    ));
    assert!(!destroys_foo(
        "proc foo {} {}\ntry { set x 1 } trap {POSIX ENOENT} {} { rename foo {} }\n"
    ));
    assert!(destroys_foo(
        "proc foo {} {}\ntry { set x 1 } finally { rename foo {} }\n"
    ));
    assert!(destroys_foo("proc foo {} {}\nrename foo {}\n"));
}

/// A `proc unknown` nested inside `namespace eval`
/// is an ordinary namespace proc, not the interpreter's handler, so it
/// must not seed `unknown_proc_info` and suppress W123 file-wide.
///
/// The body is the dynamic (`exec`-dispatching) shape that *would*
/// suppress W123 file-wide if this were the global handler, so the test
/// isolates the scope question rather than the body-shape one.
///
/// Oracle (tclsh8.6.14 and tclsh9.0.4): with
/// `namespace eval ::mylib { proc unknown {args} { return handled } }`,
/// calling `totallyBogusCommand` still fails
/// `invalid command name "totallyBogusCommand"` — only a *global*
/// `proc unknown` makes it return `handled`.
#[test]
fn analyse_namespace_local_unknown_proc_does_not_seed_the_global_handler() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "namespace eval ::mylib { proc unknown {cmd args} { exec $cmd {*}$args } }\n\
             totallyBogusCommand\n",
        "tcl9.0",
    );
    assert!(
        r.unknown_proc_info.is_none(),
        "a namespace-local proc named unknown is not the global handler",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W123),
        "so the bogus command is still unresolved; got {:?}",
        r.diagnostics,
    );
}

/// TP guard — the global handler still seeds the info (and still
/// suppresses W123), which is the behaviour the C2 fix must not disturb.
#[test]
fn analyse_global_unknown_proc_still_seeds_the_handler() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc unknown {cmd args} { exec $cmd {*}$args }\ntotallyBogusCommand\n",
        "tcl9.0",
    );
    assert!(
        r.unknown_proc_info.is_some(),
        "a global proc unknown is the handler",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W123),
        "and it suppresses the unresolved-command warning; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_qualified_unknown_proc_also_populates_info() {
    // ``::tcl::unknown`` (the canonical fully-qualified
    // name) should trigger detection too.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("proc ::tcl::unknown {cmd args} { exec $cmd }", "tcl");
    let info = r.unknown_proc_info.expect("unknown_proc_info populated");
    assert!(info.has_exec);
}

// stray-close-bracket recovery

#[test]
fn analyse_top_level_repairs_stray_close_bracket() {
    // ``set x string]`` is a typo for ``set x [string ...]``.
    // The recovery should rewrite the third argv entry into
    // a virtual ``CMD`` token before dispatch so the var
    // record is registered with the recovered shape.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("set x string]", "tcl");
    // ``x`` ends up in scope as a single-arg ``set`` (a var
    // read), not as a two-arg ``set`` with the broken text
    // — recovery yields the synthetic ``[string]`` command
    // word so dispatch sees the intended shape.
    assert!(r.global_scope.variables.contains_key("x"));
}

// unknown_proc_info / package require

#[test]
fn analyse_records_package_require() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("package require Tcl 8.6", "tcl");
    assert_eq!(r.package_requires.len(), 1);
    let p = &r.package_requires[0];
    assert_eq!(p.name, "Tcl");
    assert_eq!(p.version.as_deref(), Some("8.6"));
    assert_eq!(p.requirements, ["8.6"]);
    assert!(!p.conditional);
    assert!(!p.control_flow);
    assert!(!p.exact, "no `-exact` word means the ranged requirement");
}

#[test]
fn analyse_records_every_package_requirement_alternative() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("package require Foo 1.0 2.0-2.5", "tcl8.6");
    let p = &r.package_requires[0];
    assert_eq!(p.version.as_deref(), Some("1.0"));
    assert_eq!(p.requirements, ["1.0", "2.0-2.5"]);
}

#[test]
fn analyse_marks_package_requires_in_control_flow() {
    let cases = [
        (
            "while {$enabled} { package require WhileDep 1.0 }",
            "tcl8.6",
            "WhileDep",
        ),
        (
            "foreach item $items { package require ForeachDep 1.0 }",
            "tcl8.6",
            "ForeachDep",
        ),
        (
            "lmap item $items { package require LmapDep 1.0 }",
            "tcl8.6",
            "LmapDep",
        ),
        (
            "dict for {key value} {} { package require DictForDep 1.0 }",
            "tcl8.6",
            "DictForDep",
        ),
        (
            "dict map {key value} {} { package require DictMapDep 1.0 }",
            "tcl8.6",
            "DictMapDep",
        ),
        (
            "::tcl::dict::for {key value} {} { package require QualifiedDictForDep 1.0 }",
            "tcl8.6",
            "QualifiedDictForDep",
        ),
        (
            "array for {key value} values { package require ArrayForDep 1.0 }",
            "tcl9.0",
            "ArrayForDep",
        ),
        (
            "switch $kind a { package require SwitchDep 1.0 }",
            "tcl8.6",
            "SwitchDep",
        ),
        (
            "timerate { package require TimerateDep 1.0 } 10 1",
            "tcl9.0",
            "TimerateDep",
        ),
    ];
    for (source, dialect, name) in cases {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(source, dialect);
        let p = r
            .package_requires
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("missing {name} from {source:?}"));
        assert!(
            p.control_flow,
            "{name} is inside a skippable or repeatable body: {source:?}",
        );
    }
}

#[test]
fn analyse_marks_only_the_repeatable_for_scripts_as_control_flow() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "for {package require InitDep 1.0} {$enabled} \
             {package require NextDep 1.0} {package require BodyDep 1.0}\n\
             package require TailDep 1.0",
        "tcl8.6",
    );
    let flags: Vec<_> = r
        .package_requires
        .iter()
        .map(|p| (p.name.as_str(), p.control_flow))
        .collect();
    assert_eq!(
        flags,
        [
            ("InitDep", false),
            ("NextDep", true),
            ("BodyDep", true),
            ("TailDep", false),
        ],
    );
}

#[test]
fn analyse_records_package_require_exact_flag() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("package require -exact Tcl 8.6", "tcl");
    let p = &r.package_requires[0];
    assert_eq!(p.name, "Tcl");
    assert_eq!(p.version.as_deref(), Some("8.6"));
    // TP: the flag reaches the record, so the resolver can
    // narrow `8.6` to the degenerate range `8.6-8.6`.
    assert!(p.exact);
    // A lone `-exact` is the option with no package after it — tclsh
    // 8.4–9.0: `wrong # args` (`PkgRequireCore` compares the word with
    // `strcmp(…, "-exact")` whatever follows) — so nothing is required.
    let r = a.analyse("package require -exact", "tcl");
    assert!(r.package_requires.is_empty(), "{:?}", r.package_requires);
    // FP guard — only the exact spelling is the flag: `-e` is not
    // `-exact` (tclsh: `package require -e Tcl` asks for a package named
    // `-e`). The registry's option scan reads a dash word naming no
    // option as a call Tcl rejects, so the layout is unproven and no
    // requirement — exact or not — is recorded for it.
    let r = a.analyse("package require -e 1.0", "tcl");
    assert!(
        !r.package_requires.iter().any(|p| p.exact),
        "{:?}",
        r.package_requires
    );
}

/// `package ifneeded NAME VER SCRIPT` registers a load script; the
/// two-word query form registers nothing.
#[test]
fn analyse_records_package_ifneeded_registrations_only() {
    let mut a = crate::analyser::Analyser::new();
    // TP — the setter form, the shape every `pkgIndex.tcl` writes.
    let r = a.analyse(
        "package ifneeded base64 2.5 [list source [file join $dir base64.tcl]]",
        "tcl",
    );
    assert_eq!(r.package_ifneededs.len(), 1);
    assert_eq!(r.package_ifneededs[0].name, "base64");
    assert_eq!(r.package_ifneededs[0].version, "2.5");
    // TP — a braced script body is equally a registration.
    let r = a.analyse("package ifneeded mylib 1.0 {source lib.tcl}", "tcl");
    assert_eq!(r.package_ifneededs.len(), 1);
    // TN — the two-word form is a *query*; it registers nothing, so a
    // document that merely asks what is registered must not be read as
    // making the package's loading dynamic.
    let r = a.analyse("package ifneeded base64 2.5", "tcl");
    assert_eq!(
        r.package_ifneededs,
        [] as [crate::analyser::types::PackageIfneeded; 0]
    );
}

/// `package prefer latest` is recorded; every other spelling of the
/// subcommand changes no state and is not.
#[test]
fn analyse_records_package_prefer_latest() {
    let mut a = crate::analyser::Analyser::new();
    // TP — the one transition that exists.
    let r = a.analyse("package prefer latest", "tcl");
    assert_eq!(r.package_prefer_latest.len(), 1);
    assert!(!r.package_prefer_latest[0].conditional);
    // TP — inside a guarded branch the record carries `conditional`, so a
    // consumer can refuse to flip the selection rule on it.
    let r = a.analyse("catch {package prefer latest}", "tcl");
    assert_eq!(r.package_prefer_latest.len(), 1);
    assert!(r.package_prefer_latest[0].conditional);
    // TN — `stable` is a no-op from the default and silently ineffective
    // afterwards; the bare form is a query; a dynamic mode word is not
    // guessed at.
    for src in [
        "package prefer stable",
        "package prefer",
        "package prefer $mode",
        "package prefer [mode]",
    ] {
        let r = a.analyse(src, "tcl");
        assert_eq!(r.package_prefer_latest.len(), 0, "{src}");
    }
}

#[test]
fn analyse_w123_suppressed_when_package_require_seen() {
    // W123 is suppressed when any package require is on
    // file — package may load arbitrary commands.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("package require Foo\nbogus_command arg", "tcl");
    assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
}

#[test]
fn analyse_w123_suppressed_when_unknown_proc_chains_original() {
    // ``proc unknown`` that chains the original handler is
    // a *dynamic* shape — W123 is suppressed entirely
    // because runtime can resolve any command name.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc unknown {cmd args} { _original_unknown $cmd {*}$args }\nbogus_command arg",
        "tcl",
    );
    assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
}

#[test]
fn analyse_w123_suppressed_for_every_chain_spelling_of_the_handler() {
    // `chains_original` is the registry's
    // `Traits::UNRESOLVED_COMMAND_HANDLER`, so every spelling of the
    // handler counts.  The old `CHAIN_TARGETS` list named the `tcl::`
    // pair by hand and omitted `unknown` / `::unknown` entirely, so a
    // handler chaining through the plain spelling read as
    // self-contained and every unresolved command in the file collected
    // a spurious W123.
    for chain in ["unknown", "::unknown", "::tcl::unknown", "_orig_unknown"] {
        let mut a = crate::analyser::Analyser::new();
        let src = format!("proc unknown {{cmd args}} {{ {chain} $cmd {{*}}$args }}\nbogus_cmd arg");
        let r = a.analyse(&src, "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
            "chaining through {chain} should suppress W123",
        );
    }
}

#[test]
fn analyse_w123_suppressed_when_unknown_proc_calls_exec() {
    // ``exec $cmd`` inside ``unknown`` is a dynamic shape;
    // any command may be a real binary on PATH.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc unknown {cmd args} { exec $cmd {*}$args }\nbogus_command arg",
        "tcl",
    );
    assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
}

#[test]
fn analyse_w123_quiet_outside_explicit_dispatch_targets() {
    // A global ``proc unknown`` receives every unresolved word, so
    // ``bogus_command`` runs it and returns normally (tclsh 8.4.20, 8.6
    // and 9.0: `catch` gives 0): nothing is proved unresolved.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } } }\nbogus_command arg",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "the global unknown handler runs ``bogus_command``; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w123_suppressed_for_explicit_dispatch_target() {
    // ``foo`` is in the explicit dispatch_targets — even
    // for the non-dynamic shape, the per-invocation loop
    // suppresses W123 for it.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } } }\nfoo arg",
        "tcl",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W123 && d.message.contains("'foo'")),
        "W123 should not fire for command listed in dispatch_targets; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w123_quiet_for_empty_unknown_stub() {
    // An empty global ``unknown`` still receives the call, which then
    // returns an empty result (tclsh 8.4.20, 8.6 and 9.0).
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("proc unknown {cmd args} {}\nbogus_command arg", "tcl8.6");
    assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
}

// deep_param_traits plumbing
//
// These tests pin the contract that flipping
// `Analyser::deep_param_traits` actually changes the
// `ProcDef.param_traits` surface: the shallow pass alone
// misses traits hidden inside braced bodies, the deep pass
// catches them, and the union of both is what the analyser
// exposes.  The call-graph / symbol-graph / dataflow-graph /
// semantic-graph builders flip this on.

#[test]
fn analyse_with_deep_param_traits_surfaces_nested_eval() {
    // `$body` is buried inside a `foreach` body — the
    // shallow pass walks only top-level commands and misses
    // it.  Flipping `deep_param_traits` on must surface
    // `Eval`.
    let source = "proc f {items body} {\n  foreach item $items {\n    uplevel 1 $body\n  }\n}";
    let mut shallow = crate::analyser::Analyser::new();
    let shallow_r = shallow.analyse(source, "tcl");
    let shallow_proc = shallow_r.all_procs.get("::f").expect("::f proc registered");
    let shallow_body_traits = shallow_proc.param_traits.get("body");
    assert!(
        !shallow_body_traits.is_some_and(|s| s.contains(&crate::analyser::ProcArgTrait::Eval)),
        "shallow pass should miss nested Eval, got {shallow_body_traits:?}",
    );

    let mut deep = crate::analyser::Analyser::new();
    deep.deep_param_traits = true;
    let deep_r = deep.analyse(source, "tcl");
    let deep_proc = deep_r.all_procs.get("::f").expect("::f proc registered");
    let deep_body_traits = deep_proc
        .param_traits
        .get("body")
        .expect("body traits present with deep_param_traits on");
    assert!(
        deep_body_traits.contains(&crate::analyser::ProcArgTrait::Eval),
        "deep pass should surface nested Eval, got {deep_body_traits:?}",
    );
}

#[test]
fn deep_lambda_traits_follow_selected_original_words_and_document_offsets() {
    for prefix in ["", "# λ shifts the original byte offsets\n", "\u{feff}"] {
        let source = format!("{prefix}proc f {{body}} {{apply {{x {{eval $x}}}} $body}}");
        let mut analyser = crate::analyser::Analyser::new();
        analyser.deep_param_traits = true;
        let result = analyser.analyse(&source, "tcl9.0");
        let traits = result
            .all_procs
            .get("::f")
            .and_then(|procedure| procedure.param_traits.get("body"));
        assert!(
            traits.is_some_and(|traits| traits.contains(&crate::analyser::ProcArgTrait::Eval)),
            "selected original lambda source should carry Eval through its actual argument; prefix={prefix:?}, got {traits:?}",
        );
    }
}

#[test]
fn analyse_with_deep_param_traits_off_matches_shallow() {
    // For procs without nested-body usage, deep + shallow
    // produce the same trait map.  This pins that the deep
    // pass doesn't accidentally lose any shallow trait.
    let source = "proc g {body} { uplevel 1 $body }";
    let mut shallow = crate::analyser::Analyser::new();
    let shallow_r = shallow.analyse(source, "tcl");

    let mut deep = crate::analyser::Analyser::new();
    deep.deep_param_traits = true;
    let deep_r = deep.analyse(source, "tcl");

    let shallow_traits = shallow_r
        .all_procs
        .get("::g")
        .expect("::g registered")
        .param_traits
        .clone();
    let deep_traits = deep_r
        .all_procs
        .get("::g")
        .expect("::g registered")
        .param_traits
        .clone();
    assert_eq!(
        shallow_traits, deep_traits,
        "deep + shallow should match for top-level-only bodies",
    );
}

#[test]
fn uplevel_nonzero_isolates_body_vars_from_proc() {
    // `uplevel 1 {set x 5}` runs in the caller's frame, not the enclosing
    // proc's — its `x` must not merge into the proc's locals (they are
    // different variables), and it lands in an isolated `Uplevel` child
    // scope tagged with the level word.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "proc f {} {\n    uplevel 1 {set x 5}\n    set y 1\n}\n",
        "tcl8.6",
    );
    let f_scope = r
        .global_scope
        .children
        .iter()
        .find(|c| c.name == "f")
        .expect("f proc scope");
    assert!(f_scope.variables.contains_key("y"), "y is a proc local");
    assert!(
        !f_scope.variables.contains_key("x"),
        "uplevel body's `x` must not merge into the proc scope",
    );
    let up = f_scope
        .children
        .iter()
        .find(|c| c.kind == crate::analyser::ScopeKind::Uplevel)
        .expect("isolated uplevel scope");
    assert_eq!(up.name, "1", "the level word tags the frame");
    assert!(up.variables.contains_key("x"));
}

#[test]
fn nested_def_in_qualified_encloser_does_not_overwrite_global() {
    // `proc a::outer { proc helper ... }`: the nested `helper` homes to the
    // encloser's *defining* namespace (`::a::helper`), not the lexical
    // global — so the real global `proc helper` is preserved rather than
    // overwritten in `all_procs`.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
            "proc helper {} { puts global }\nproc a::outer {} {\n    proc helper {} { puts nested }\n}\n",
            "tcl8.6",
        );
    assert!(
        r.all_procs.contains_key("::a::helper"),
        "nested helper must home to ::a::helper: {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );
    let global = r
        .all_procs
        .get("::helper")
        .expect("global ::helper preserved");
    assert_eq!(
        global.name_span.start(),
        5,
        "::helper must remain the global declaration (line 0), not the nested one",
    );
    // The same for a nested class.
    let mut b = crate::analyser::Analyser::new();
    let rc = b.analyse(
        "oo::class create Widget {}\nproc a::outer {} {\n    oo::class create Widget {}\n}\n",
        "tcl8.6",
    );
    assert!(
        rc.all_classes.contains_key("::a::Widget"),
        "nested class homes to ::a::Widget: {:?}",
        rc.all_classes.keys().collect::<Vec<_>>(),
    );
    assert!(
        rc.all_classes.contains_key("::Widget"),
        "global ::Widget preserved",
    );
}

#[test]
fn variable_global_upvar_skip_dynamic_names() {
    // A `variable` / `global` / `upvar` / `namespace upvar` whose name word
    // is computed (`$dyn` / `[f]`) is not a static declaration — the literal
    // substitution text must not be recorded as a variable.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
            "namespace eval ns {\n    variable $dyn\n    variable realvar 1\n}\nproc p {} {\n    global $g\n    upvar 1 other $loc\n}\n",
            "tcl8.6",
        );
    let ns = r
        .global_scope
        .children
        .iter()
        .find(|c| c.name == "ns")
        .expect("ns scope");
    assert!(
        ns.variables.contains_key("realvar"),
        "a static `variable` name is still recorded",
    );
    assert!(
        !ns.variables.contains_key("dyn"),
        "`variable $dyn` must not record `dyn`: {:?}",
        ns.variables.keys().collect::<Vec<_>>(),
    );
    let p = r
        .global_scope
        .children
        .iter()
        .find(|c| c.name == "p")
        .expect("p scope");
    assert!(
        !p.variables.contains_key("g") && !p.variables.contains_key("loc"),
        "dynamic `global`/`upvar` names must not be recorded: {:?}",
        p.variables.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn uplevel_zero_still_resets_to_global_frame() {
    // `uplevel #0` keeps the global-frame tag so variable resolution
    // resolves outward to the global namespace (not the caller-frame
    // abstention a non-`#0` level uses).
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("proc g {} {\n    uplevel #0 {set z 5}\n}\n", "tcl8.6");
    let g_scope = r
        .global_scope
        .children
        .iter()
        .find(|c| c.name == "g")
        .expect("g proc scope");
    let up = g_scope
        .children
        .iter()
        .find(|c| c.kind == crate::analyser::ScopeKind::Uplevel)
        .expect("uplevel scope");
    assert_eq!(up.name, "#0");
    assert!(up.variables.contains_key("z"));
}

// stub-overlay end-to-end
//
// These tests pin the contract that the stub overlay built
// from `# tcl-lsp: stub` directives during `analyse()`
// propagates into the per-proc `param_traits` map.

#[test]
fn analyse_with_declared_stub_propagates_role_to_param_traits() {
    // The source declares a `# tcl-lsp: stub my_eval
    // {script:body}` directive, then defines a proc that
    // invokes `my_eval $body`.  The body arg's role flows
    // from the stub overlay → `param_traits["body"]
    // .contains(Body)`.
    let source = "\
# tcl-lsp: stubs-begin\n\
# tcl-lsp: stub my_eval {script:body}\n\
# tcl-lsp: stubs-end\n\
proc runs {body} {\n\
    my_eval $body\n\
}\n";
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(source, "tcl8.6");
    let proc = r.all_procs.get("::runs").expect("::runs registered");
    let body_traits = proc
        .param_traits
        .get("body")
        .expect("body param has traits");
    assert!(
        body_traits.contains(&crate::analyser::ProcArgTrait::Body),
        "expected Body trait via stub overlay, got {body_traits:?}",
    );
}

#[test]
fn analyse_without_stub_directive_leaves_body_untyped() {
    // Same proc, no stub directive — without the overlay
    // entry, `my_eval` isn't a known command, so the body
    // arg has no recorded role and `param_traits` is empty
    // for `body`.  This pins that the stub directive (not
    // some background heuristic) is what gives the
    // parameter its `Body` trait.
    let source = "proc runs {body} { my_eval $body }";
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(source, "tcl");
    let proc = r.all_procs.get("::runs").expect("::runs registered");
    assert!(
        !proc
            .param_traits
            .get("body")
            .is_some_and(|s| s.contains(&crate::analyser::ProcArgTrait::Body)),
        "expected NO Body trait without stub directive, got {:?}",
        proc.param_traits.get("body"),
    );
}

// Safe-interp visibility survives `analyse_per_item`.

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_reaches_deferred_proc_body_under_per_item_1001() {
    let mut a = Analyser::new();
    let r = a.analyse_per_item(
        "interp create -safe s\ninterp eval s { proc f {} { source foo }; f }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "a hidden call inside a deferred proc body warns under \
             incremental (per-item) analysis, matching `Analyser::analyse`: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_reaches_deferred_apply_body_under_per_item_1001() {
    // naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    let mut a = Analyser::new();
    let r = a.analyse_per_item(
        "interp create -safe s\ninterp eval s { apply {{} { source foo }} }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "a hidden call inside a deferred apply-lambda body warns under \
             incremental (per-item) analysis: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_reaches_deferred_method_body_under_per_item_1001() {
    // naming.interpreter.original-source-visibility-advice
    // docs/design/analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md
    let mut a = Analyser::new();
    let r = a.analyse_per_item(
        "interp create -safe s\n\
             interp eval s {\n\
                 oo::class create C { method m {} { source foo } }\n\
                 [C new] m\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "a hidden call inside a deferred TclOO method body warns under \
             incremental (per-item) analysis: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn deferred_proc_body_outside_any_safe_interp_is_untouched_under_per_item_1001() {
    let mut a = Analyser::new();
    let r = a.analyse_per_item("proc f {} { source foo }\nf\n", "tcl8.6");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "no safe interpreter is involved, so no W129 can ever fire: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_deferred_body_redefinition_updates_its_source_visibility() {
    let mut a = Analyser::new();
    let r = a.analyse_per_item(
        "interp create -safe s\n\
             interp eval s {\n\
                 proc f {} { proc source {} { return ok }; source }\n\
                 f\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "the deferred body retains its own original source definition before the later call: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_expand_list_quoted_apply_lambda_body_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s { {*}[list apply {dir {source $dir/evil.tcl}} $env(HOME)] }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "hidden `source` nested inside a `{{*}}[list apply ...]`-invoked \
             lambda body warns: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// Original command/list operands and the enclosing source child are required.
#[test]
fn safe_interp_w129_list_quoted_apply_package_ifneeded_then_require_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 package ifneeded evil 1.0 [list apply {dir {source $dir/evil.tcl}} $env(HOME)]\n\
                 package require evil\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129
                && d.conditional_interpreter_visibility().is_some()),
        "the deferred script's hidden `source` warns once `package \
             require` is present too: {:?}",
        r.diagnostics
    );
}

// Namespace ensemble -map redirection.

/// Conditional source visibility, independently of actual dispatch.
/// A reporting ensemble map does not own the original mapped target.
#[test]
fn safe_interp_w129_ensemble_create_map_redirect_to_hidden_command_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens -map {go source}\n\
                 myens go pkg.tcl\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a reporting map does not supply an original mapped-target receipt: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// A reporting ensemble map does not own the original mapped target.
#[test]
fn safe_interp_w129_ensemble_configure_map_redirect_to_hidden_command_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens\n\
                 namespace ensemble configure myens -map {go source}\n\
                 myens go pkg.tcl\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a reporting map does not supply an original mapped-target receipt: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// A reporting ensemble map does not own the original mapped target.
#[test]
fn safe_interp_w129_ensemble_default_name_map_redirect_to_hidden_command_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 namespace eval myns { namespace ensemble create -map {go source} }\n\
                 myns go pkg.tcl\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a reporting map does not supply an original mapped-target receipt: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// A reporting ensemble map does not own the original mapped target.
#[test]
fn safe_interp_w129_ensemble_map_redirect_to_safe_command_not_flagged_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens -map {go puts}\n\
                 myens go hi\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "an ensemble redirect to a safe command must not warn: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// A reporting ensemble map does not own the original mapped target.
#[test]
fn ensemble_map_redirect_outside_any_safe_interp_is_untouched_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace ensemble create -command myens -map {go source}\n\
             myens go pkg.tcl\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "no safe interpreter is involved, so no W129 can ever fire: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// A reporting ensemble map does not own the original mapped target.
#[test]
fn safe_interp_w129_ensemble_map_redirect_multiword_target_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens -map {go {source b.tcl}}\n\
                 myens go\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a multiword reporting prefix does not own its effective head: {:?}",
        r.diagnostics
    );
}

/// Conditional source visibility, independently of actual dispatch.
/// A reporting ensemble map does not own the original mapped target.
#[test]
fn safe_interp_w129_ensemble_configure_map_replaces_not_merges_1001() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens \
                     -map {bad source ok puts}\n\
                 namespace ensemble configure myens -map {ok puts}\n\
                 myens bad\n\
             }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W129),
        "a subcommand dropped by a later `-map` must not keep warning \
             through its stale mapping: {:?}",
        r.diagnostics
    );
}
