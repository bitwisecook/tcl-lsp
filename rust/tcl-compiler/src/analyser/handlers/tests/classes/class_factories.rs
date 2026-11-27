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

//! Class factories: registry definers, user metaclass overrides and the members a manufacturer splices in.

use super::*;

#[test]
fn original_registry_class_definers_require_the_selected_source_and_complete_arguments() {
    for (source, expected) in [
        ("oo::class create Made {}", true),
        ("proc oo::class args {}; oo::class create Made {}", false),
    ] {
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, "tcl");
        analyser.registry = Some(analyser.profile_registry());
        let segment = tcl_source::segment::segment_commands_with_offset_and_config(
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
        let segment = tcl_source::segment::segment_commands_with_offset_and_config(
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
