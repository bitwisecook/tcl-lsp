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
use std::sync::Arc;
use tcl_registry::{
    ArgRole, Arity, CommandRegistry, CommandSpec, InvocationArguments, ScriptTiming,
};

#[test]
fn original_bare_script_geometry_keeps_actual_text_and_value_boundaries() {
    for (source, expected) in [
        ("if 1 break", Some("break")),
        ("if 1 $", Some("$")),
        ("if 1 \"break\"", Some("break")),
        ("if 1 $body", None),
        ("if 1 prefix[body]", None),
        (r"if 1 escaped\ body", None),
        (r#"if 1 "br\u0065ak""#, None),
        ("if 1 {*}{break}", None),
    ] {
        let config = LexerConfig {
            expand_syntax: true,
            ..LexerConfig::default()
        };
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let word = &plan.commands[0].words[2];
        let geometry = original_word_script_body_geometry(word);
        assert_eq!(
            geometry
                .as_ref()
                .map(|body| source.get(body.content.as_range()).unwrap()),
            expected,
            "{source}"
        );
        if let Some(geometry) = geometry {
            assert_eq!(&geometry.container, word);
            assert_eq!(geometry.container.config(), config);
            assert!(
                word.span().start() <= geometry.content.start()
                    && geometry.content.end() <= word.span().end()
            );
        }
    }
}

#[test]
fn complete_body_coverage_refuses_unrepresented_lambda_containers() {
    for source in ["apply {{} {puts hidden}}", "apply $lambda"] {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let context = tcl_registry::model::context_for_profile(profile);
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let syntax =
            super::super::source_structure::OriginalSourceRegistryContext::capture(source, input);
        let command =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
                .remove(0);
        let words = syntax.words(&command).unwrap();
        assert!(
            words
                .roles()
                .unwrap()
                .iter()
                .any(|(_, role)| *role == ArgRole::LambdaLiteral)
        );
        assert!(source_script_bodies_for_mutation_coverage(&words, &context).is_none());
        assert_eq!(
            words.source_script_bodies(&context),
            [] as [crate::registry_invocation::source_scoped_body::OriginalSourceScriptBody; 0]
        );
    }
}

#[test]
fn original_lambda_bodies_retain_actual_list_container_and_unchanged_source() {
    for (source, expected) in [
        ("apply {{} {puts visible}}", Some("puts visible")),
        ("apply {{} break}", Some("break")),
        ("apply {{x} {return $x} ::N} value", Some("return $x")),
        (r#"apply {{} "puts visible"}"#, Some("puts visible")),
        (r"apply {{} puts\ visible}", None),
        (r"apply {{} {puts visible}tail}", None),
        ("apply {{} {puts hidden} ::N extra}", None),
        ("apply {{}}", None),
        ("apply $lambda", None),
        ("apply {*}{ {} {puts hidden} }", None),
    ] {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let context = tcl_registry::model::context_for_profile(profile);
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let syntax =
            super::super::source_structure::OriginalSourceRegistryContext::capture(source, input);
        let command =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
                .remove(0);
        let Some(words) = syntax.words(&command) else {
            assert!(expected.is_none(), "missing original schema: {source}");
            continue;
        };
        let body = words.original_lambda_body_geometry(&context);
        assert!(source_script_bodies_for_mutation_coverage(&words, &context).is_none());
        assert_eq!(
            words.source_script_bodies(&context),
            [] as [crate::registry_invocation::source_scoped_body::OriginalSourceScriptBody; 0]
        );
        match expected {
            Some(expected) => {
                let body = body.expect("explicit unchanged original lambda geometry");
                assert_eq!(source.get(body.content_span().as_range()), Some(expected));
                assert_eq!(
                    body.original_container(),
                    words.original_argument_word(0).unwrap()
                );
                assert!(body.matches_source(&SourceImage::document(source), config));
                assert_eq!(body.original_container().config(), config);
                let foreign = tcl_registry::model::context_for_profile(
                    tcl_dialect::DialectProfile::find("tcl8.4").unwrap(),
                );
                assert!(words.original_lambda_body_geometry(&foreign).is_none());
            }
            None => assert!(body.is_none(), "{source}"),
        }
    }
}

fn reference_only(_arguments: InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
    vec![(0, ScriptTiming::ReferenceOnly)]
}

fn original_definition_parent(
    source: &str,
    dialect: &str,
) -> (crate::analyser::AnalysisResult, OriginalSourceScriptBody) {
    let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
    let config = analysis.body_lexer_config.unwrap();
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let command =
        tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let words =
        super::super::source_structure::source_registry_words(source, &analysis, &command).unwrap();
    let parent = words.source_script_bodies(&context).remove(0);
    (analysis, parent)
}

fn definition_child_at(
    source: &str,
    analysis: &crate::analyser::AnalysisResult,
    parent: &OriginalSourceScriptBody,
    head: &str,
) -> OriginalSourceScriptBody {
    let body_region = parent.content_span();
    let command = tcl_source::segment::segment_commands_with_offset_and_config(
        source.get(body_region.as_range()).unwrap(),
        body_region.start(),
        analysis.body_lexer_config.unwrap(),
    )
    .into_iter()
    .find(|command| command.name() == head)
    .unwrap();
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    super::super::source_structure::source_registry_words(source, analysis, &command)
        .unwrap()
        .source_script_bodies(&context)
        .remove(0)
}

#[test]
fn original_member_roles_lend_whole_names_formals_and_wrapper_selectors() {
    let source = "oo::class create C {self self method café {arg {optional 2}} {}; export café}";
    for dialect in ["tcl", "tcl8.6"] {
        let (analysis, parent) = original_definition_parent(source, dialect);
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let region = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        let original = region.original_commands().next().unwrap();
        for (role, indices) in [
            (ArgRole::Name, vec![3]),
            (ArgRole::ParamList, vec![4]),
            (ArgRole::Keyword, vec![1, 2]),
        ] {
            assert_eq!(
                region.argument_words_for_role(original, role).unwrap(),
                indices
                    .into_iter()
                    .map(|index| &original[index])
                    .collect::<Vec<_>>()
            );
            assert!(
                region
                    .argument_words_for_role(&original[..2], role)
                    .is_none()
            );
        }
    }
}

#[test]
fn original_member_variable_words_retain_roles_and_whole_source_geometry() {
    let source = r"oo::class create C {variable a\ b arr($index); self variable wrapped\ name; method run {} {}}";
    for dialect in ["tcl", "tcl8.6"] {
        let (analysis, parent) = original_definition_parent(source, dialect);
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let region = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        let commands = region.original_commands().collect::<Vec<_>>();
        let variables = region.variable_argument_words(commands[0]).unwrap();
        assert_eq!(variables, vec![&commands[0][1], &commands[0][2]]);
        assert_eq!(
            region.variable_argument_words(commands[1]).unwrap(),
            vec![&commands[1][2]]
        );
        assert_eq!(
            region.variable_argument_words(commands[2]).unwrap(),
            [] as [&tcl_lexer::NativeWord; 0]
        );
        assert!(region.variable_argument_words(&commands[0][..2]).is_none());
        let other = source.replace("a\\ b", "different");
        let (_, other_parent) = original_definition_parent(&other, dialect);
        let other_region = other_parent
            .definition_member_region(&context, other_parent.content_span())
            .unwrap();
        assert!(
            region
                .variable_argument_words(other_region.original_commands().next().unwrap())
                .is_none()
        );
    }
}

#[test]
fn original_member_references_retain_whole_words_parent_vocabulary_and_scope() {
    // naming.core.original-member-reference-hazard
    // docs/design/analysis/name-resolution-proofs/core-original-member-reference-hazard.md
    let source = "oo::class create C {export {m space} m\\0n; self export m; export $unknown}";
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let (analysis, parent) = original_definition_parent(source, dialect);
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let region = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        let commands = region.original_commands().collect::<Vec<_>>();
        let references = region.references(commands[0]).unwrap();
        assert_eq!(references.len(), 2, "{dialect}");
        assert!(references[0].matches_reported_name("m space"));
        assert!(references[1].matches_reported_name("m\0n"));
        assert_eq!(references[1].value(), b"m\xc0\x80n");
        assert_eq!(references[0].original_word(), &commands[0][1]);
        assert_eq!(
            references[0].kind(),
            tcl_registry::definer::MemberRefKind::Method
        );
        let wrapped = region.references(commands[1]).unwrap();
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0].original_word(), &commands[1][2]);
        assert_eq!(region.references(commands[2]).unwrap(), [] as [crate::registry_invocation::source_scoped_body::OriginalSourceDefinitionMemberReference; 0]);
        assert!(region.references(&commands[0][..2]).is_none());
        let other = source.replace("m space", "n space");
        let (_, foreign) = original_definition_parent(&other, dialect);
        let foreign_region = foreign
            .definition_member_region(&context, foreign.content_span())
            .unwrap();
        let foreign_command = foreign_region.original_commands().next().unwrap();
        assert!(region.references(foreign_command).is_none());
        let retired =
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
        assert!(
            parent
                .definition_member_region(&retired, parent.content_span())
                .is_none()
        );
    }
}
#[test]
fn mixed_lambda_roles_cannot_donate_immediate_caller_source_context() {
    for (roles, inherits) in [
        (&[(0, ArgRole::Body)][..], true),
        (
            &[(0, ArgRole::Body), (1, ArgRole::LambdaLiteral)][..],
            false,
        ),
    ] {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "mixed-script",
            arity: Arity::exact(2),
            arg_roles: roles,
            ..CommandSpec::DEFAULT
        });
        let context = Arc::new(
            tcl_registry::model::context_for_profile(profile)
                .with_command_store(Arc::new(registry)),
        );
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let source = "mixed-script {puts current} {{} {puts lambda}}";
        let syntax =
            super::super::source_structure::OriginalSourceRegistryContext::capture(source, input);
        let command =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
                .remove(0);
        let words = syntax.words(&command).unwrap();
        let bodies = words.source_script_bodies(&context);
        assert_eq!(bodies.len(), 1);
        assert_eq!(
            bodies[0].inherits_immediate_caller_source_context(&context),
            inherits
        );
    }
}

#[test]
fn original_definition_parent_uses_actual_plain_current_immediate_body_metadata() {
    // naming.core.original-executable-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-executable-region-context.md
    fn immediate(_args: InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
        vec![(0, ScriptTiming::SameInvocation)]
    }
    fn deferred(_args: InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
        vec![(0, ScriptTiming::Deferred)]
    }
    let source = "oo::class create C {source-body {method café {} {puts live}}}";
    for (kind, interpreter, timing, inherits) in [
        (
            tcl_registry::BodyKind::Plain,
            tcl_registry::BodyInterpreter::Current,
            immediate as tcl_registry::ScriptTimingResolver,
            true,
        ),
        (
            tcl_registry::BodyKind::Structural,
            tcl_registry::BodyInterpreter::Current,
            immediate as tcl_registry::ScriptTimingResolver,
            false,
        ),
        (
            tcl_registry::BodyKind::Plain,
            tcl_registry::BodyInterpreter::Argument(0),
            immediate as tcl_registry::ScriptTimingResolver,
            false,
        ),
        (
            tcl_registry::BodyKind::Plain,
            tcl_registry::BodyInterpreter::Current,
            deferred as tcl_registry::ScriptTimingResolver,
            false,
        ),
        (
            tcl_registry::BodyKind::Plain,
            tcl_registry::BodyInterpreter::Current,
            reference_only as tcl_registry::ScriptTimingResolver,
            false,
        ),
    ] {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "source-body",
            arity: Arity::exact(1),
            arg_roles: &[(0, ArgRole::Body)],
            body_kind: kind,
            body_interpreter: interpreter,
            script_timing_resolver: Some(timing),
            ..CommandSpec::DEFAULT
        });
        let context = Arc::new(
            tcl_registry::model::context_for_profile(profile)
                .with_command_store(Arc::new(registry)),
        );
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let command =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
                .remove(0);
        let parent =
            super::super::source_structure::source_registry_words(source, &analysis, &command)
                .unwrap()
                .source_script_bodies(&context)
                .remove(0);
        assert_eq!(
            parent.definition_parent_for(&context, None),
            Some(parent.clone())
        );
        let child = definition_child_at(source, &analysis, &parent, "source-body");
        assert_eq!(
            child
                .definition_parent_for(&context, Some(&parent))
                .is_some(),
            inherits,
            "{kind:?} {interpreter:?}"
        );
        assert!(child.definition_parent_for(&context, None).is_none());
    }
}

#[test]
fn original_definition_parent_declines_structural_foreign_and_uncontained_source() {
    // naming.core.original-executable-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-executable-region-context.md
    let source =
        "oo::class create C {if 1 {method café {} {puts live}}; proc p {} {method ordinary}}";
    let (analysis, parent) = original_definition_parent(source, "tcl8.6");
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let immediate = definition_child_at(source, &analysis, &parent, "if");
    assert_eq!(
        immediate.definition_parent_for(&context, Some(&parent)),
        Some(parent.clone())
    );
    let procedure = definition_child_at(source, &analysis, &parent, "proc");
    assert!(
        procedure
            .definition_parent_for(&context, Some(&parent))
            .is_none()
    );
    let foreign = tcl_registry::model::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl9.0").unwrap(),
    );
    assert!(
        immediate
            .definition_parent_for(&foreign, Some(&parent))
            .is_none()
    );
    let (_, unrelated) =
        original_definition_parent("oo::class create D {method d {} {puts other}}", "tcl8.6");
    assert!(
        immediate
            .definition_parent_for(&context, Some(&unrelated))
            .is_none()
    );
    let source = "oo::class create C {}; if 1 {method outside}";
    let (analysis, parent) = original_definition_parent(source, "tcl8.6");
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let command = tcl_source::segment::segment_commands_with_offset_and_config(
        source,
        0,
        analysis.body_lexer_config.unwrap(),
    )
    .remove(1);
    let outside =
        super::super::source_structure::source_registry_words(source, &analysis, &command)
            .unwrap()
            .source_script_bodies(&context)
            .remove(0);
    assert!(
        outside
            .definition_parent_for(&context, Some(&parent))
            .is_none()
    );
}

#[test]
fn receiver_grammar_belongs_to_the_actual_instance_callable_body() {
    let source = "oo::class create C {method run {} {}; constructor {} {}; destructor {}; self method build {} {}}";
    for dialect in ["tcl", "tcl8.6"] {
        let (analysis, parent) = original_definition_parent(source, dialect);
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let region = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        let commands = region.original_commands().collect::<Vec<_>>();
        assert_eq!(commands.len(), 4);
        let foreign = tcl_registry::model::ingress::resolve_environment("f5-irules")
            .default_context_registry();
        for (index, command) in commands.into_iter().enumerate() {
            let body = region.script_bodies(command).unwrap().remove(0);
            assert_eq!(
                body.receiver_grammar(&context)
                    .map(|grammar| grammar.family),
                (index < 3).then_some(tcl_registry::definer::DefinerFamily::TclOo),
                "{dialect}: {index}"
            );
            assert!(body.receiver_grammar(&foreign).is_none());
        }
    }
}

#[test]
fn original_callable_member_script_keeps_shifted_row_and_original_context() {
    use tcl_registry::definer::{CallableRole, MemberEffect, MemberReceiver};
    let source =
        "oo::class create C {method wire {value} {set x $value}; self method build {} {set x 1}}";
    let (analysis, parent) = original_definition_parent(source, "tcl8.6");
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let region = parent
        .definition_member_region(&context, parent.content_span())
        .unwrap();
    let commands = region.original_commands().collect::<Vec<_>>();
    let instance = region.script_bodies(commands[0]).unwrap().remove(0);
    let class = region.script_bodies(commands[1]).unwrap().remove(0);
    assert_eq!(
        instance.member_row().unwrap().receiver,
        MemberReceiver::Instance
    );
    assert_eq!(
        class.member_row().unwrap().receiver,
        MemberReceiver::TypeObject
    );
    for body in [instance, class] {
        assert!(matches!(
            body.member_row().unwrap().effect,
            MemberEffect::Callable {
                role: CallableRole::Method,
                ..
            }
        ));
        assert!(body.definition_parent().is_none());
        assert!(body.matches_context(&context));
        assert!(body.matches_source(
            &tcl_lexer::SourceImage::document(source),
            analysis.body_lexer_config.unwrap()
        ));
        assert!(!body.matches_source(
            &tcl_lexer::SourceImage::native(source.as_bytes()),
            analysis.body_lexer_config.unwrap()
        ));
        let different =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        assert!(!body.matches_context(&different));
    }
}

#[test]
fn original_member_row_ingress_keeps_unknown_and_counted_values() {
    let source = r"oo::class create C {method $name {} {}; method \uD800 {} {}; method Ω {} {}; method \x00 {} {}; method literal $params {}}";
    let (analysis, parent) = original_definition_parent(source, "tcl8.6");
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let region = parent
        .definition_member_region(&context, parent.content_span())
        .unwrap();
    let bodies = region
        .original_commands()
        .map(|command| region.script_bodies(command).unwrap().remove(0))
        .collect::<Vec<_>>();
    assert_eq!(bodies.len(), 5);
    for body in &bodies[..4] {
        let row = body.member_row().unwrap();
        assert!(row.name.is_none());
        assert!(matches!(
            row.effect,
            tcl_registry::definer::MemberEffect::Callable { .. }
        ));
        assert!(body.matches_source(
            &tcl_lexer::SourceImage::document(source),
            analysis.body_lexer_config.unwrap()
        ));
    }
    let row = bodies[4].member_row().unwrap();
    assert_eq!(row.name.as_deref(), Some("literal"));
    assert!(row.arity.is_none());
}

#[test]
fn original_member_keyword_requires_complete_owned_argv_but_not_body_applicability() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = "oo::class create C {method café $params {puts hidden}}";
    let (analysis, parent) = original_definition_parent(source, "tcl8.6");
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let members = parent
        .definition_member_region(&context, parent.content_span())
        .unwrap();
    let start = u32::try_from(source.find("method").unwrap()).unwrap();
    let words = members.original_command_words_at(start).unwrap();
    assert_eq!(members.member_keyword(words).unwrap().keyword, "method");
    assert!(
        members.script_bodies(words).is_none(),
        "a keyword does not prove its unknown body layout"
    );
    assert!(members.member_keyword(&words[..3]).is_none());
    assert!(members.original_command_words_at(start + 1).is_none());
}

#[test]
fn original_property_member_bodies_skip_option_like_values_and_quoted_options() {
    // naming.tcloo.property-option-value-boundary
    // docs/design/analysis/name-resolution-proofs/tcloo-property-option-value-boundary.md
    // Native observations concern stored Tcl scripts; this control binds
    // their layout to authentic source spans without granting installation.
    for dialect in ["tcl9.0", "tcl9.1"] {
        let source = "oo::configurable create C {property p -get {-set} \"-set\" {set café live}}";
        let (analysis, parent) = original_definition_parent(source, dialect);
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let members = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        let start = u32::try_from(source.find("property").unwrap()).unwrap();
        let words = members.original_command_words_at(start).unwrap();
        let bodies = members.script_bodies(words).unwrap();
        assert_eq!(bodies.len(), 2);
        assert_eq!(bodies[0].argument(), 2);
        assert_eq!(bodies[0].original_container(), &words[3]);
        assert_eq!(
            source.get(bodies[0].content_span().as_range()),
            Some("-set")
        );
        assert_eq!(bodies[1].argument(), 4);
        assert_eq!(bodies[1].original_container(), &words[5]);
        assert_eq!(
            source.get(bodies[1].content_span().as_range()),
            Some("set café live")
        );
        assert!(bodies.iter().all(|body| body.definition_parent().is_none()));
        assert!(
            bodies
                .iter()
                .all(|body| body.original_container() != &words[4])
        );
    }
    let source = "oo::configurable create C {property p -get {return superseded} \"-get\" {-set} -set {set café live}}";
    let (analysis, parent) = original_definition_parent(source, "tcl9.1");
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let members = parent
        .definition_member_region(&context, parent.content_span())
        .unwrap();
    let start = u32::try_from(source.find("property").unwrap()).unwrap();
    let words = members.original_command_words_at(start).unwrap();
    let bodies = members.script_bodies(words).unwrap();
    assert_eq!(
        bodies
            .iter()
            .map(OriginalSourceDefinitionMemberScriptBody::argument)
            .collect::<Vec<_>>(),
        vec![4, 6]
    );
    assert_eq!(
        source.get(bodies[0].content_span().as_range()),
        Some("-set")
    );
    assert!(
        bodies
            .iter()
            .all(|body| body.original_container() != &words[3])
    );
}

#[test]
fn original_member_disposition_keeps_nonmember_unavailable_and_complete_geometry_separate() {
    use OriginalSourceDefinitionMemberDisposition as D;
    let source = "oo::class create C {\nmethod good {} {puts ok}\nsuperclass Base\nif $condition {puts global}\nmethod missing {}\nmethod cooked {} \"puts\\x20cooked\"\n$head body\n}\n";
    let (analysis, parent) = original_definition_parent(source, "tcl");
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let region = parent
        .definition_member_region(&context, parent.content_span())
        .unwrap();
    let commands = region.original_commands().collect::<Vec<_>>();
    assert_eq!(commands.len(), 6);
    let D::CompleteMember(bodies) = region.script_body_disposition(&context, commands[0]) else {
        panic!("the genuine complete member must retain its body");
    };
    assert_eq!(bodies.len(), 1);
    assert_eq!(bodies[0].original_words(), commands[0]);
    assert_eq!(bodies[0].original_container(), &commands[0][3]);
    assert!(
        matches!(region.script_body_disposition(&context, commands[1]), D::CompleteMember(bodies) if bodies.is_empty())
    );
    assert!(matches!(
        region.script_body_disposition(&context, commands[2]),
        D::CompleteNonMember
    ));
    for words in [&commands[0][..3], commands[3], commands[4], commands[5]] {
        assert!(matches!(
            region.script_body_disposition(&context, words),
            D::Unavailable
        ));
    }
    let foreign = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(matches!(
        region.script_body_disposition(
            &foreign.resolved_input.as_ref().unwrap().context_registry(),
            commands[0],
        ),
        D::Unavailable
    ));
    let changed = source.replace("puts ok", "puts no");
    let (changed_analysis, changed_parent) = original_definition_parent(&changed, "tcl");
    let changed_context = changed_analysis
        .resolved_input
        .as_ref()
        .unwrap()
        .context_registry();
    let changed_region = changed_parent
        .definition_member_region(&changed_context, changed_parent.content_span())
        .unwrap();
    assert!(matches!(
        region
            .script_body_disposition(&context, changed_region.original_commands().next().unwrap()),
        D::Unavailable
    ));
}

#[test]
fn original_member_region_owns_complete_argv_and_unicode_body_geometry() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = "oo::class create C {\nmethod café {} {puts café}\nself self {method n {} {puts nested}}\n}\n";
    let (analysis, parent) = original_definition_parent(source, "tcl8.6");
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let members = parent
        .definition_member_region(&context, parent.content_span())
        .unwrap();
    let plan = tcl_lexer::native_script_words_in(
        SourceImage::document(source),
        parent.content_span(),
        analysis.body_lexer_config.unwrap(),
    )
    .unwrap();
    let method = &plan.commands[0].words;
    let bodies = members.script_bodies(method).unwrap();
    assert_eq!(bodies.len(), 1);
    assert_eq!(bodies[0].argument(), 2);
    assert_eq!(bodies[0].original_words(), method);
    assert_eq!(bodies[0].original_container(), &method[3]);
    assert_eq!(
        source.get(bodies[0].content_span().as_range()),
        Some("puts café")
    );
    assert!(bodies[0].definition_parent().is_none());
    assert!(
        members.script_bodies(&method[..3]).is_none(),
        "a trimmed argv is not the complete original command"
    );
    let wrapper = members
        .script_bodies(&plan.commands[1].words)
        .unwrap()
        .remove(0);
    assert_eq!(wrapper.argument(), 1);
    let nested = wrapper
        .definition_parent()
        .unwrap()
        .definition_member_region(&context, wrapper.content_span())
        .unwrap();
    let plan = tcl_lexer::native_script_words_in(
        SourceImage::document(source),
        wrapper.content_span(),
        analysis.body_lexer_config.unwrap(),
    )
    .unwrap();
    let body = nested
        .script_bodies(&plan.commands[0].words)
        .unwrap()
        .remove(0);
    assert_eq!(
        source.get(body.content_span().as_range()),
        Some("puts nested")
    );
}

#[test]
fn original_member_region_declines_foreign_context_words_and_cooked_bodies() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = "oo::class create C {method café {} {puts café}}";
    let (analysis, parent) = original_definition_parent(source, "tcl8.6");
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let config = analysis.body_lexer_config.unwrap();
    let members = parent
        .definition_member_region(&context, parent.content_span())
        .unwrap();
    let foreign = tcl_lexer::native_script_words_in(
        SourceImage::document(&source.replace("café", "cafè")),
        parent.content_span(),
        config,
    )
    .unwrap();
    assert!(members.script_bodies(&foreign.commands[0].words).is_none());
    let mut foreign_config = config;
    foreign_config.strict_quoting ^= true;
    let foreign = tcl_lexer::native_script_words_in(
        SourceImage::document(source),
        parent.content_span(),
        foreign_config,
    )
    .unwrap();
    assert!(members.script_bodies(&foreign.commands[0].words).is_none());
    let foreign_context = tcl_registry::model::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl9.0").unwrap(),
    );
    assert!(
        parent
            .definition_member_region(&foreign_context, parent.content_span())
            .is_none()
    );
    assert!(
        parent
            .definition_member_region(&context, Span::new(0, u32::try_from(source.len()).unwrap()))
            .is_none()
    );
    for (source, unknown_layout) in [
        ("oo::class create C {$member m {} {puts hidden}}", true),
        ("oo::class create C {method m $params {puts hidden}}", true),
        (
            r#"oo::class create C {method m {} "\u0070uts hidden"}"#,
            false,
        ),
        ("oo::class create C {method m {} body[puts hidden]}", false),
    ] {
        let (analysis, parent) = original_definition_parent(source, "tcl8.6");
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let members = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            parent.content_span(),
            analysis.body_lexer_config.unwrap(),
        )
        .unwrap();
        let bodies = members.script_bodies(&plan.commands[0].words);
        if unknown_layout {
            assert!(bodies.is_none(), "{source}");
        } else {
            assert!(bodies.unwrap().is_empty(), "{source}");
        }
    }
}

#[test]
fn original_expression_script_regions_use_actual_grammar_and_container() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    for dialect in ["tcl8.6", "tcl9.0"] {
        let source = "expr {[format café]}";
        let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
        let config = analysis.body_lexer_config.unwrap();
        let command =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
                .remove(0);
        let words =
            super::super::source_structure::source_registry_words(source, &analysis, &command)
                .unwrap();
        let input = analysis.resolved_input.as_ref().unwrap();
        let bodies = words.source_expression_script_bodies(input).unwrap();
        assert_eq!(bodies.len(), 1);
        assert_eq!(
            source.get(bodies[0].content_span().as_range()),
            Some("format café")
        );
        assert_eq!(
            bodies[0].original_container(),
            words.operands()[0].as_ref().unwrap().word().unwrap()
        );
        let mut changed = config;
        changed.strict_quoting ^= true;
        let stale = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            changed,
        );
        assert!(words.source_expression_script_bodies(&stale).is_none());
    }
    for source in [
        "expr {[format hidden]} + 1",
        r#"expr "\u005bformat hidden\u005d""#,
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        let command =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
                .remove(0);
        let words =
            super::super::source_structure::source_registry_words(source, &analysis, &command)
                .unwrap();
        assert!(
            words
                .source_expression_script_bodies(analysis.resolved_input.as_ref().unwrap())
                .is_none(),
            "{source}"
        );
    }
}

#[test]
fn original_script_inventory_separates_reference_syntax_from_potential_evaluation() {
    // naming.source.original-script-region-purpose
    // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
    let source = "reference-script {format reference}\nrun-script {format potential}";
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "reference-script",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Body)],
        script_timing_resolver: Some(reference_only),
        ..CommandSpec::DEFAULT
    });
    registry.insert(CommandSpec {
        name: "run-script",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Body)],
        ..CommandSpec::DEFAULT
    });
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let generation = tcl_registry::model::context_for_profile(profile);
    let context = Arc::new(generation.with_command_store(Arc::new(registry)));
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let syntax =
        super::super::source_structure::OriginalSourceRegistryContext::capture(source, input);
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    let reference = syntax.words(&commands[0]).unwrap();
    assert_eq!(reference.source_script_bodies(&context).len(), 1);
    assert_eq!(
        reference
            .source_script_bodies_for(&context, OriginalSourceScriptPurpose::PotentialEvaluation),
        [] as [crate::registry_invocation::source_scoped_body::OriginalSourceScriptBody; 0]
    );
    let potential = syntax.words(&commands[1]).unwrap();
    assert_eq!(
        potential
            .source_script_bodies_for(&context, OriginalSourceScriptPurpose::PotentialEvaluation)
            .len(),
        1
    );
    assert_eq!(
        potential.with_source_schema(&context, |schema| schema
            .authored_source_script_timing_at(0)),
        Some(Some(ScriptTiming::SameInvocation))
    );
    assert_eq!(
        source_script_bodies_for_mutation_coverage(&reference, &context),
        Some(vec![])
    );
}

#[test]
fn original_definition_coverage_requires_complete_parent_and_member_plans() {
    for dialect in ["tcl", "tcl8.6", "tcl9.0"] {
        let source = "oo::class create C {constructor {} { }; method ping {} {}}";
        let (analysis, parent) = original_definition_parent(source, dialect);
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let region = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        let commands = region.complete_original_commands().expect(dialect);
        assert_eq!(commands.len(), 2);
        let constructor = region.script_bodies(&commands[0]).unwrap().remove(0);
        assert_eq!(constructor.complete_original_commands(), Some(Vec::new()));
        let prefix = parent
            .definition_member_region(
                &context,
                Span::new(
                    parent.content_span().start(),
                    commands[0].last().unwrap().span().end(),
                ),
            )
            .unwrap();
        assert!(prefix.complete_original_commands().is_none(), "{dialect}");

        let (analysis, parent) = original_definition_parent(
            "oo::class create C {constructor {} { }; \"unterminated}",
            dialect,
        );
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let region = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        assert!(region.complete_original_commands().is_none(), "{dialect}");

        let (analysis, parent) = original_definition_parent(
            "oo::class create C {constructor {} {\"unterminated}}",
            dialect,
        );
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let region = parent
            .definition_member_region(&context, parent.content_span())
            .unwrap();
        let command = region.original_commands().next().unwrap();
        let constructor = region.script_bodies(command).unwrap().remove(0);
        assert!(
            constructor.complete_original_commands().is_none(),
            "{dialect}"
        );
    }
}
