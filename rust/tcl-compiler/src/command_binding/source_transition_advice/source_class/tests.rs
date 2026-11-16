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

use crate::analyser::Analyser;
use crate::registry_invocation::source_structure::source_constructor_call_at;

#[test]
fn c_source_class_query_preserves_independently_completed_world() {
    use crate::registry_invocation::source_structure::source_class_declaration_at;
    let source =
        "namespace eval outer {namespace eval inner {oo::class create Widget {method m {} {}}}}";
    let offset = u32::try_from(source.find("oo::class").unwrap()).unwrap();
    for dialect in ["tcl8.6", "tcl9.0"] {
        let analysis = Analyser::new().analyse(source, dialect);
        assert!(!analysis.allows_lexical_declaration_advice());
        let completed = analysis
            .original_completed_command_world()
            .cloned()
            .expect("independently retained closed Normal source root before the query");
        let declaration = source_class_declaration_at(source, &analysis, offset)
            .expect("genuine original namespace-body class declaration");
        assert_eq!(
            declaration
                .source_class(&analysis)
                .unwrap()
                .metadata()
                .qualified_name,
            "::outer::inner::Widget"
        );
        assert!(declaration.factory.c_source_body.is_some());
        assert!(declaration.factory.logical_source_input().is_none());
        assert_eq!(
            analysis.original_completed_command_world(),
            Some(&completed)
        );
    }
}

#[test]
fn c_source_namespace_class_after_unentered_definition_body_keeps_unknown_mutation() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    // An unentered non-empty `TclOO` definition body leaves the
    // unresolved-command uncertainty: the later relative factory head in
    // the same namespace is still a source class, with that obligation.
    // `uplevel 1 {namespace path ::P}` in the body reaches the caller
    // frame (tclsh 8.6.18 / 9.0.4 / 9.1.0 then dispatch the later
    // `oo::class` to `::P::oo::class`), so the obligation must remain.
    use super::super::SourceCommandTransitionObligation;
    use crate::registry_invocation::source_structure::source_class_declaration_at;
    for body in ["method m {} {}", "uplevel 1 {namespace path ::P}"] {
        let source = format!(
            "namespace eval ::P {{}}\nnamespace eval outer {{oo::class create A {{{body}}}; \
                 oo::class create B {{}}}}\n"
        );
        let offset = u32::try_from(source.rfind("oo::class create").unwrap()).unwrap();
        for dialect in ["tcl8.6", "tcl9.0"] {
            let analysis = Analyser::new().analyse(&source, dialect);
            let declaration = source_class_declaration_at(&source, &analysis, offset)
                .unwrap_or_else(|| panic!("{dialect}: {source}"));
            assert_eq!(
                declaration
                    .source_class(&analysis)
                    .expect("canonical source class card")
                    .metadata()
                    .qualified_name,
                "::outer::B",
                "{dialect}: {source}"
            );
            assert!(
                declaration
                    .factory
                    .obligations()
                    .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation),
                "{dialect}: {source}"
            );
        }
    }
}

#[test]
fn c_source_namespace_class_declarations_retain_original_body_and_conditional_scope() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    use super::super::SourceCommandTransitionObligation;
    use crate::registry_invocation::source_structure::source_class_declaration_at;
    let source =
        "namespace eval outer {namespace eval inner {oo::class create Widget {method m {} {}}}}";
    let offset = u32::try_from(source.find("oo::class").unwrap()).unwrap();
    for dialect in ["tcl8.6", "tcl9.0"] {
        let analysis = Analyser::new().analyse(source, dialect);
        assert!(!analysis.allows_lexical_declaration_advice());
        // tclsh 8.6.18 / 9.0.4 / 9.1.0 complete this script with code 0
        // and one fixed command world (`::outer::inner::Widget` added,
        // nothing removed), so the completed world is present.
        assert!(analysis.original_completed_command_world().is_some());
        let declaration = source_class_declaration_at(source, &analysis, offset)
            .expect("original namespace-body class declaration");
        let class = declaration
            .source_class(&analysis)
            .expect("canonical source class card");
        assert_eq!(class.metadata().qualified_name, "::outer::inner::Widget");
        assert!(class.metadata().methods.contains_key("m"));
        assert!(declaration.factory.c_source_body.is_some());
        assert!(declaration.factory.logical_source_input().is_none());
        for obligation in [
            SourceCommandTransitionObligation::ConditionalCNamespaceApplicability,
            SourceCommandTransitionObligation::ConditionalCBodyApplicability,
            SourceCommandTransitionObligation::RegisteredFactoryApplicability,
        ] {
            assert!(declaration.factory.obligations().contains(&obligation));
        }
        assert!(
            source_class_declaration_at(&source.replace("Widget", "Gadget"), &analysis, offset)
                .is_none()
        );
        let mut foreign = analysis.clone();
        foreign.resolved_input = Analyser::new()
            .analyse("puts foreign", "tcl9.1")
            .resolved_input;
        assert!(source_class_declaration_at(source, &foreign, offset).is_none());
    }
    for source in [
        "namespace eval $unknown {oo::class create Widget {}}",
        "namespace eval outer {proc ::oo::class args {}; oo::class create Widget {}}",
    ] {
        let offset = u32::try_from(source.rfind("oo::class create").unwrap()).unwrap();
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            source_class_declaration_at(source, &analysis, offset).is_none(),
            "{source}"
        );
    }
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let (_owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let analysis = Analyser::new()
        .with_source_analysis_entry(std::sync::Arc::new(
            crate::command_binding::SourceAnalysisEntry {
                native_entry: Some(std::sync::Arc::new(entry)),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        ))
        .analyse(source, profile.name);
    assert!(
        source_class_declaration_at(source, &analysis, offset).is_none(),
        "closed Native baseline cannot borrow conditional source-body ownership"
    );
}

#[test]
fn logical_class_facet_retains_a_same_purpose_occupied_target_refusal() {
    use super::*;
    use crate::registry_invocation::source_structure::{
        OriginalRegistrySource, source_registry_words_at,
    };
    let source = "oo::class create C {}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let words = source_registry_words_at(source, &analysis, 0).unwrap();
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        panic!("actual Logical schema");
    };
    let input = analysis.resolved_input.as_ref().unwrap();
    let context = input.context_registry();
    let policy = AdviceNamingPolicy::Logical(input.clone());
    let owner = AdviceInvocationContext {
        origin: &advice.site.source,
        context: &context,
        config: advice.config,
        dialect: advice.dialect,
        policy: &policy,
        query: None,
    };
    let mut graph = AdviceGraph::new(&context, &policy, advice.dialect).unwrap();
    let mut tape = OriginalSourceTransitionAdviceTape::default();
    let invocation = AdviceInvocation {
        native: advice.original_words(),
        head: &advice.head,
        arguments: &advice.arguments,
        lineage: &advice.lineage,
    };
    words
        .with_source_schema(&context, |schema| {
            let first = owner
                .retain_source_class_declaration(&mut tape, &graph, invocation, schema)
                .unwrap();
            let slot = first.source_slot().clone();
            assert_eq!(tape.source_class_for_advice(advice), Some(first.as_ref()));
            AdviceInvocationContext::install_source_class(&mut tape, &mut graph, Some(first));
            assert!(
                owner
                    .retain_source_class_declaration(&mut tape, &graph, invocation, schema)
                    .is_none()
            );
            assert!(
                tape.source_class_for_advice(advice).is_none(),
                "the same real facet cannot erase occupied-target refusal"
            );
            graph.cells.remove(&slot);
            assert!(
                owner
                    .retain_source_class_declaration(&mut tape, &graph, invocation, schema)
                    .is_some()
            );
            assert!(
                tape.source_class_for_advice(advice).is_none(),
                "same-facet refusal remains terminal"
            );
            Some(())
        })
        .unwrap();
}

#[test]
fn logical_class_declarations_reborrow_the_exact_source_purpose_facet() {
    use super::*;
    use crate::registry_invocation::source_structure::{
        OriginalRegistrySource, source_class_declaration_at, source_registry_words_at,
    };
    let source = "namespace eval left {oo::class create Widget {}}\nnamespace eval right {oo::class create Widget {}}\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    assert_eq!(analysis.all_classes.len(), 2);
    let input = analysis.resolved_input.as_ref().unwrap();
    let context = input.context_registry();
    let realm = analysis.retained_command_realm().unwrap();
    for (offset, _) in source.match_indices("oo::class create") {
        let offset = u32::try_from(offset).unwrap();
        let words = source_registry_words_at(source, &analysis, offset).unwrap();
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("the authentic original Logical source schema");
        };
        let declaration = source_class_declaration_at(source, &analysis, offset)
            .expect("the actual source class issuer facet");
        assert!(declaration.logical_source_class(&analysis).is_some());
        assert!(declaration.source_class(&analysis).is_none());
        let mut tape = realm
            .source_bindings_ref()
            .original_source_transition_advice_tape(&context)
            .unwrap();
        assert_eq!(tape.source_class_for_advice(advice), Some(&declaration));
        // A generic purpose conflict cannot manufacture or erase the
        // independently retained exact source facet.
        tape.class_declarations.insert(offset, None);
        assert!(tape.source_class(offset).is_none());
        assert_eq!(tape.source_class_for_advice(advice), Some(&declaration));
        let mut changed = advice.as_ref().clone();
        changed
            .uncertain_operations
            .push(Arc::from(advice.original_words()));
        assert!(tape.source_class_for_advice(&changed).is_none());
        let mut conflicting = declaration.clone();
        conflicting.slot.simple = tcl_core_types::NameBytes::from(b"Other".as_slice());
        tape.retain_class_declaration_facet(SourceClassDeclarationFacet {
            factory: Arc::clone(&declaration.factory),
            class: Some(Arc::new(conflicting)),
        });
        assert!(tape.source_class_for_advice(advice).is_none());
        let mut tape = realm
            .source_bindings_ref()
            .original_source_transition_advice_tape(&context)
            .unwrap();
        tape.retain_class_declaration_facet(SourceClassDeclarationFacet {
            factory: Arc::clone(&declaration.factory),
            class: None,
        });
        assert!(tape.source_class_for_advice(advice).is_none());
        tape.class_declaration_facets.clear();
        assert!(tape.source_class_for_advice(advice).is_none());
        assert!(
            source_class_declaration_at(&source.replace("Widget", "Other"), &analysis, offset)
                .is_none()
        );
    }
}

#[test]
fn original_abstract_constructor_defaults_and_exports_keep_selected_native_source_grammar() {
    // naming.tcloo.original-abstract-manufacturer-public-visibility
    // docs/design/analysis/name-resolution-proofs/tcloo-original-abstract-manufacturer-public-visibility.md
    // A Native source-name model is not a entered Native constructor receipt.
    for dialect in ["tcl9.0", "tcl9.1"] {
        let environment = tcl_registry::model::ingress::resolve_known_environment(dialect).unwrap();
        let profile = environment.analyser_profile();
        let mut config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        config.strict_quoting = false;
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            environment.default_context_registry(),
            config,
        );
        for (source, marker, visible) in [
            ("oo::abstract create Base {}; Base new", "Base new", false),
            (
                "oo::abstract create Base {}; Base create object",
                "Base create object",
                false,
            ),
            (
                "oo::abstract create Base {}; Base createWithNamespace object ::ns",
                "Base createWithNamespace",
                false,
            ),
            (
                "oo::abstract create Base {}; oo::objdefine Base export new; Base new",
                "Base new",
                true,
            ),
            (
                "oo::abstract create Base {}; oo::class create Concrete {superclass Base}; Concrete new",
                "Concrete new",
                true,
            ),
            (
                "rename oo::abstract Factory; Factory create Base {}; Base new",
                "Base new",
                false,
            ),
            (
                "interp alias {} Factory {} oo::abstract create; Factory Base {}; Base new",
                "Base new",
                false,
            ),
            (
                "oo::abstract create Base {}; rename Base Moved; Moved new",
                "Moved new",
                false,
            ),
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, dialect);
            let offset = u32::try_from(source.rfind(marker).unwrap()).unwrap();
            let call = source_constructor_call_at(source, &analysis, offset).expect(source);
            assert_eq!(
                call.constructor_shape(&analysis).is_some(),
                visible,
                "{dialect}: {source}"
            );
            assert!(call.logical_constructor_shape(&analysis).is_none());
            let mut changed = analysis.clone();
            let mut changed_config = config;
            changed_config.strict_quoting = true;
            changed.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                input.context_registry(),
                changed_config,
            ));
            assert!(call.constructor_shape(&changed).is_none());
            let mut missing = analysis.clone();
            missing.resolved_input = None;
            assert!(call.constructor_shape(&missing).is_none());
            assert!(
                source_constructor_call_at(&source.replace("Base", "Foreign"), &analysis, offset)
                    .is_none()
            );
        }
    }
}

#[test]
fn original_abstract_constructor_logical_advice_keeps_defaults_without_native_identity() {
    // naming.tcloo.original-abstract-manufacturer-public-visibility
    // docs/design/analysis/name-resolution-proofs/tcloo-original-abstract-manufacturer-public-visibility.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
            .unwrap()
            .default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    for (source, visible) in [
        ("oo::abstract create Base {}; Base new", false),
        (
            "oo::abstract create Base {}; oo::objdefine Base export new; Base new",
            true,
        ),
        (
            "oo::abstract create Base {}; oo::class create Concrete {superclass Base}; Concrete new",
            true,
        ),
        (
            "rename oo::abstract Factory; Factory create Base {}; Base new",
            false,
        ),
    ] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        let offset = u32::try_from(source.rfind(';').unwrap() + 2).unwrap();
        let call = source_constructor_call_at(source, &analysis, offset).expect(source);
        assert_eq!(
            call.logical_constructor_shape(&analysis).is_some(),
            visible,
            "{source}"
        );
        assert!(call.constructor_shape(&analysis).is_none());
        assert!(call.class_declaration().source_class(&analysis).is_none());
        assert!(
            call.class_declaration()
                .logical_source_class(&analysis)
                .is_some()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(call.logical_constructor_shape(&missing).is_none());
    }
}

#[test]
fn original_logical_constructor_shape_keeps_canonical_factory_without_native_authority() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    for (source, expected) in [
        ("oo::class create C {method ping {} {}}; [C new] ping", true),
        (
            "oo::class create C {method ping {} {}}; interp alias {} make {} C new; [make] ping",
            true,
        ),
        (
            "oo::class create C {self method new {} {}}; [C new] ping",
            false,
        ),
        (
            "oo::class create C {}; oo::objdefine C unexport new; [C new] ping",
            false,
        ),
    ] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "tcl");
        let offset = u32::try_from(source.rfind('[').unwrap() + 1).unwrap();
        let call = source_constructor_call_at(source, &analysis, offset).expect(source);
        assert_eq!(
            call.logical_constructor_shape(&analysis).is_some(),
            expected,
            "{source}"
        );
        assert!(call.constructor_shape(&analysis).is_none());
        assert!(call.class_declaration().source_class(&analysis).is_none());
        assert!(
            call.class_declaration()
                .logical_source_class(&analysis)
                .is_some()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(call.logical_constructor_shape(&missing).is_none());
        let mut unavailable = analysis.clone();
        unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
            environment: "tcl8.6".into(),
            overlay: 999,
        });
        assert!(call.logical_constructor_shape(&unavailable).is_none());
    }
}

#[test]
fn original_class_constructor_call_keeps_alias_prefix_move_and_canonical_declaration() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    let source = "oo::class create C {constructor {x} {}}; rename C Held; interp alias {} make {} Held create; make c value";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let offset = u32::try_from(source.find("make c value").unwrap()).unwrap();
    let call = source_constructor_call_at(source, &analysis, offset).expect("original class call");
    assert_eq!(call.original_words().len(), 3);
    assert_eq!(
        call.arguments()[0].literal_bytes(),
        Some(b"create".as_slice())
    );
    assert_eq!(call.written_argument(0), None);
    assert_eq!(
        call.argument_input(0).unwrap().original_word(),
        call.argument_word(0)
    );
    assert_eq!(call.argument_input(1).unwrap().bytes(), b"c");
    assert_eq!(call.written_argument(1), Some(0));
    assert_eq!(call.class_declaration().lineage().len(), 2);
    assert_eq!(call.class_declaration().name_input().bytes(), b"C");
    assert_eq!(
        call.class_declaration().source_slot().simple.as_bytes(),
        b"Held"
    );
    let class = call.class_declaration().source_class(&analysis).unwrap();
    assert_eq!(class.name_input().bytes(), b"C");
    assert_eq!(
        call.constructor_shape(&analysis)
            .unwrap()
            .constructor_args_from(),
        2
    );
    assert!(
        source_constructor_call_at(&source.replace("value", "other"), &analysis, offset).is_none()
    );
}

#[test]
fn original_constructor_substitution_keeps_whole_child_words_and_selected_body() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    for source in [
        "oo::class create C {constructor {x} {}}; set o [C new value]",
        "oo::class create C {constructor {x} {}}; interp alias {} make {} C new; set o [make value]",
        "proc p {} {set o [C new value]}; oo::class create C {constructor {x} {}}; p",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(
            source
                .find("[C new")
                .or_else(|| source.find("[make value"))
                .unwrap()
                + 1,
        )
        .unwrap();
        let call = source_constructor_call_at(source, &analysis, offset).expect(source);
        assert_eq!(call.arguments()[0].literal_bytes(), Some(b"new".as_slice()));
        assert_eq!(
            call.arguments()[1].literal_bytes(),
            Some(b"value".as_slice())
        );
        assert_eq!(
            call.constructor_shape(&analysis)
                .unwrap()
                .constructor_args_from(),
            1
        );
        assert_eq!(
            call.class_declaration()
                .source_class(&analysis)
                .unwrap()
                .name_input()
                .bytes(),
            b"C"
        );
        if source.starts_with("proc") {
            assert!(call.original_source_body().is_some());
            assert!(call.obligations().contains(
                &super::SourceCommandTransitionObligation::OriginalProcedureBodyApplicability
            ));
        }
        assert!(
            source_constructor_call_at(&source.replace("value", "other"), &analysis, offset)
                .is_none()
        );
    }
}

#[test]
fn original_constructor_substitution_refuses_compound_words_and_multiple_commands() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    for source in [
        "oo::class create C {}; set o prefix[C new value]",
        "oo::class create C {}; set o [C new value; puts other]",
        "oo::class create C {}; rename C {}; set o [C new value]",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("[C new").unwrap() + 1).unwrap();
        assert!(
            source_constructor_call_at(source, &analysis, offset).is_none(),
            "{source}"
        );
    }
}

#[test]
fn original_logical_constructor_calls_keep_real_deferred_and_immediate_parent_bodies() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    for (source, obligation) in [
        (
            "oo::class create C {}; proc p {} {set o [C new value]}",
            super::SourceCommandTransitionObligation::DeferredLogicalBodyApplicability,
        ),
        (
            "oo::class create C {}; if {1} {set o [C new value]}",
            super::SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability,
        ),
        (
            "proc p {} {oo::class create C {}; set o [C new value]}",
            super::SourceCommandTransitionObligation::DeferredLogicalBodyApplicability,
        ),
    ] {
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let offset = u32::try_from(source.find("[C new").unwrap() + 1).unwrap();
        let call = source_constructor_call_at(source, &analysis, offset).expect(source);
        assert!(call.obligations().contains(&obligation));
        let body = call
            .original_source_body()
            .expect("genuine selected enclosing body");
        assert!(body.content_span().start() <= offset);
        assert!(
            call.original_words()
                .iter()
                .all(|word| word.span().end() <= body.content_span().end())
        );
        assert!(
            call.class_declaration()
                .name_input()
                .native_input()
                .is_none()
        );
        assert!(call.constructor_shape(&analysis).is_none());
    }
}

#[test]
fn original_source_class_keeps_conditional_factory_after_unknown_call_but_known_replacement_blocks()
{
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    let source = "oo::class create C {constructor {x} { }}; C new one; C new two";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    for (ordinal, marker) in ["C new one", "C new two"].into_iter().enumerate() {
        let offset = u32::try_from(source.find(marker).unwrap()).unwrap();
        let call = source_constructor_call_at(source, &analysis, offset).expect(marker);
        assert_eq!(
            call.constructor_shape(&analysis)
                .unwrap()
                .constructor_args_from(),
            1
        );
        if ordinal == 1 {
            assert!(
                call.obligations()
                    .contains(&super::SourceCommandTransitionObligation::UnknownEarlierMutation)
            );
        }
    }
    for source in [
        "oo::class create C {}; C new one; proc C args {}; C new two",
        "oo::class create C {}; C new one; rename C {}; C new two",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("C new two").unwrap()).unwrap();
        assert!(
            source_constructor_call_at(source, &analysis, offset).is_none(),
            "{source}"
        );
    }
}

#[test]
fn original_constructor_manufacturer_visibility_keeps_call_source_horizon() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    for (source, selector, visible) in [
        (
            "oo::class create C {}; C createWithNamespace obj ::ns value; oo::objdefine C export createWithNamespace",
            "C createWithNamespace",
            false,
        ),
        (
            "oo::class create C {}; C new value; oo::objdefine C unexport new",
            "C new",
            true,
        ),
        (
            "oo::class create C {}; oo::objdefine C unexport new; C new value; oo::objdefine C export new",
            "C new",
            false,
        ),
        (
            "oo::class create C {}; C new value; oo::objdefine C method new args {return replacement}",
            "C new",
            true,
        ),
        (
            "oo::class create C {}; oo::objdefine C method new args {return replacement}; C new value",
            "C new",
            false,
        ),
        (
            "oo::class create C {}; proc p {} {C createWithNamespace obj ::ns value}; oo::objdefine C export createWithNamespace",
            "C createWithNamespace",
            true,
        ),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find(selector).unwrap()).unwrap();
        let call = source_constructor_call_at(source, &analysis, offset).expect(source);
        assert_eq!(
            call.constructor_shape(&analysis).is_some(),
            visible,
            "{source}"
        );
    }
}

#[test]
fn original_class_constructor_call_refuses_known_factory_or_class_replacement_and_cycles() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    for source in [
        "proc oo::class args {}; oo::class create C {}; C new value",
        "oo::class create C {}; proc C args {}; C new value",
        "oo::class create C {}; rename C {}; C new value",
        "oo::class create C {}; interp alias {} again {} again; again new value",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.rfind(';').unwrap() + 2).unwrap();
        assert!(
            source_constructor_call_at(source, &analysis, offset).is_none(),
            "{source}"
        );
    }
}

#[test]
fn original_class_constructor_call_preserves_expansion_and_refuses_private_or_overridden_makers() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    let source = "oo::class create C {constructor args {}}; C new {*}$values";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let offset = u32::try_from(source.find("C new").unwrap()).unwrap();
    let call = source_constructor_call_at(source, &analysis, offset).unwrap();
    assert_eq!(
        call.arguments()[1],
        crate::registry_invocation::EffectiveInvocationWord::Expanded
    );
    assert_eq!(
        call.constructor_shape(&analysis)
            .unwrap()
            .constructor_args_from(),
        1
    );
    for source in [
        "oo::class create C {}; C createWithNamespace c ::NS value",
        "oo::class create C {self method create args {}}; C create c value",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.rfind(';').unwrap() + 2).unwrap();
        let call = source_constructor_call_at(source, &analysis, offset).unwrap();
        assert!(call.constructor_shape(&analysis).is_none(), "{source}");
    }
}

#[test]
fn original_source_class_family_uses_selected_definer_not_metaclass_reporting_label() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    let source = "rename oo::class Maker; Maker create C {}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let record = analysis.original_class_declarations().next().unwrap();
    let receipt =
        super::OriginalSourceClassDeclaration::from_class(source, &analysis, record).unwrap();
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    assert_eq!(
        receipt.grammar(&context).unwrap().family,
        tcl_registry::definer::DefinerFamily::TclOo
    );
    assert_eq!(receipt.factory().command(), "oo::class");
    assert_eq!(receipt.factory().lineage().len(), 1);
}

#[test]
fn original_snit_constructor_call_retains_explicit_and_bare_provider_layouts() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    for (invocation, from) in [("T create t value", 2), ("T t value", 1)] {
        let source =
            format!("package require snit; snit::type T {{constructor {{x}} {{}}}}; {invocation}");
        let analysis = Analyser::new().analyse(&source, "tcl8.6");
        let offset = u32::try_from(source.find(invocation).unwrap()).unwrap();
        let call = source_constructor_call_at(&source, &analysis, offset).expect(invocation);
        assert_eq!(
            call.constructor_shape(&analysis)
                .unwrap()
                .constructor_args_from(),
            from
        );
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        assert_eq!(
            call.class_declaration().grammar(&context).unwrap().family,
            tcl_registry::definer::DefinerFamily::Snit
        );
    }
}
#[test]
fn original_native_class_names_keep_distinct_source_units_without_ascii_report_gate() {
    // docs/design/analysis/name-resolution-proofs/corner-command-distinct-unicode.md

    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    // Native Unicode name observation is separate: naming.corner.command-distinct-unicode.
    // These assertions check source producer/publication geometry, not execution.
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let source = "oo::class create é {}; oo::class create e\u{301} {}; é new; e\u{301} new";
        let analysis = Analyser::new().analyse(source, dialect);
        let calls: Vec<_> = ["é new", "e\u{301} new"]
            .into_iter()
            .map(|marker| {
                source_constructor_call_at(
                    source,
                    &analysis,
                    u32::try_from(source.find(marker).unwrap()).unwrap(),
                )
                .unwrap()
            })
            .collect();
        assert_ne!(
            calls[0].class_declaration().source_slot(),
            calls[1].class_declaration().source_slot()
        );
        assert_eq!(
            calls[0].class_declaration().name_input().bytes(),
            "é".as_bytes()
        );
        assert_eq!(
            calls[1].class_declaration().name_input().bytes(),
            "e\u{301}".as_bytes()
        );
        for call in calls {
            assert!(
                call.class_declaration()
                    .name_input()
                    .native_input()
                    .is_some()
            );
            assert!(call.class_declaration().source_class(&analysis).is_some());
        }
    }
    let source = "class {Base α} {}; class Derived {{Base α}} {}";
    let analysis = Analyser::new().analyse(source, "jim");
    let class = crate::registry_invocation::source_structure::source_class_declaration_at(
        source, &analysis, 0,
    )
    .unwrap();
    assert_eq!(class.name_input().bytes(), "Base α".as_bytes());
    assert!(class.source_class(&analysis).is_some());
}
#[test]
fn original_factory_attempt_keeps_source_incarnation_without_constructor_visibility() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    for (tail, selector, name) in [
        ("Base new", "new", b"Base".as_slice()),
        ("Base create object", "create", b"Base".as_slice()),
        (
            "Base createWithNamespace object ::ns",
            "createWithNamespace",
            b"Base".as_slice(),
        ),
        ("set result [Base new]", "new", b"Base".as_slice()),
        (
            "interp alias {} make {} Base new; make",
            "new",
            b"Base".as_slice(),
        ),
        ("rename Base Moved; Moved new", "new", b"Moved".as_slice()),
    ] {
        let source = format!("oo::abstract create Base {{}}; {tail}");
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(&source, "tcl9.0");
        let marker = if tail == "set result [Base new]" {
            "Base new"
        } else if tail.starts_with("interp alias") {
            "make"
        } else if tail.starts_with("rename") {
            "Moved new"
        } else {
            tail
        };
        let offset = u32::try_from(source.rfind(marker).unwrap()).unwrap();
        let call = source_constructor_call_at(&source, &analysis, offset).expect(tail);
        let attempt = call.source_factory_attempt(&analysis).expect(tail);
        assert_eq!(attempt.operation().keyword, selector);
        assert_eq!(
            attempt.class_declaration().source_slot().simple.as_bytes(),
            name
        );
        assert!(
            attempt
                .factory_traits()
                .contains(tcl_registry::Traits::ABSTRACT_CLASS_FACTORY)
        );
        assert!(
            call.constructor_shape(&analysis).is_none(),
            "source attempt grants no Native constructor"
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(call.source_factory_attempt(&missing).is_none());
        let mut changed = analysis.clone();
        let mut config = input.lexer_config();
        config.strict_quoting ^= true;
        changed.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            input.context_registry(),
            config,
        ));
        assert!(call.source_factory_attempt(&changed).is_none());
    }
    for mutation in [
        "proc Base args {return replacement}",
        "oo::objdefine Base method new args {return custom}",
        "oo::objdefine Base export new",
        "unknownMutation",
    ] {
        let source = format!("oo::abstract create Base {{}}; {mutation}; Base new");
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(&source, "tcl9.0");
        let offset = u32::try_from(source.rfind("Base new").unwrap()).unwrap();
        let call = source_constructor_call_at(&source, &analysis, offset);
        if mutation == "unknownMutation" {
            assert!(
                call.is_some(),
                "the original conditional call remains retained"
            );
        }
        assert!(
            call.as_ref()
                .and_then(|call| call.source_factory_attempt(&analysis))
                .is_none(),
            "{mutation}"
        );
    }
}

fn class_object_member_names(analysis: &crate::analyser::AnalysisResult) -> Vec<Vec<u8>> {
    let mut records = analysis.original_class_declarations();
    let record = records.next().expect("one source class");
    assert!(records.next().is_none());
    record
        .metadata()
        .original_members
        .methods(crate::analyser::MemberSide::ClassObject)
        .expect("complete class-object member inventory")
        .iter()
        .map(|method| method.original_name_input().bytes().to_vec())
        .collect()
}

/// Contract D: snit's type command is published with its class-side
/// `typemethod` as the dispatch target, as `oo::class create` publishes a
/// class. `Factory make` joins the source class whose class-object member
/// table declares `make`, so it is that typemethod's dispatch, not an
/// instance construction.
#[test]
fn snit_type_command_dispatches_its_typemethod_through_the_source_class() {
    let source = "snit::type Factory {\n    typemethod make {} {\n        return [Factory create x]\n    }\n}\nFactory make\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    let offset = u32::try_from(source.rfind("Factory make").unwrap()).unwrap();
    let call = source_constructor_call_at(source, &analysis, offset)
        .expect("the type command joins its source class");
    assert!(call.class_declaration().source_class(&analysis).is_some());
    assert_eq!(class_object_member_names(&analysis), vec![b"make".to_vec()]);
    assert!(
        call.constructor_shape(&analysis).is_none(),
        "a declared typemethod is a dispatch, not a bare-word construction"
    );
}

/// Contract D: [incr Tcl] publishes each class-scoped `proc` as the
/// command `Class::proc` in the class's namespace. A qualified call joins
/// the source class its qualifier names, with the written member as its
/// class-side target; a qualifier naming no source class joins nothing.
#[test]
fn itcl_class_proc_calls_join_the_qualifying_source_class() {
    use crate::registry_invocation::source_structure::source_class_proc_call_at;
    let source = concat!(
        "itcl::class Factory {\n",
        "    proc make {} { return 1 }\n",
        "}\n",
        "::Factory::make\n",
        "Factory::make\n",
        "Other::make\n",
    );
    let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    // The class proc is declared, but never in the class object's
    // two-word dispatch table.
    assert_eq!(
        class_object_member_names(&analysis),
        [] as [std::vec::Vec<u8>; 0]
    );
    assert!(
        analysis
            .original_class_declarations()
            .flat_map(|record| record.metadata().original_members.declarations())
            .any(|method| method.original_name_input().bytes() == b"make")
    );
    for spelling in ["::Factory::make\n", "\nFactory::make\n"] {
        let offset =
            u32::try_from(source.find(spelling).unwrap() + usize::from(spelling.starts_with('\n')))
                .unwrap();
        let call = source_class_proc_call_at(source, &analysis, offset)
            .unwrap_or_else(|| panic!("{spelling:?} dispatches the class proc"));
        assert_eq!(call.member_name(), b"make");
        let end = offset + u32::try_from(spelling.trim_matches('\n').len()).unwrap();
        assert_eq!(call.member_span(), tcl_lexer::Span::new(end - 4, end));
        assert!(call.class_declaration().source_class(&analysis).is_some());
    }
    // A plain procedure at the qualified name shadows the class proc.
    let shadowed = format!("{source}proc Factory::make {{}} {{ return 2 }}\nFactory::make\n");
    let shadowed_analysis = Analyser::new().analyse(&shadowed, "tcl8.6").clone();
    let offset = u32::try_from(shadowed.rfind("Factory::make").unwrap()).unwrap();
    assert!(source_class_proc_call_at(&shadowed, &shadowed_analysis, offset).is_none());
    // A qualifier naming no source class: tclsh answers
    // `invalid command name "Other::make"`.
    let offset = u32::try_from(source.find("Other::make").unwrap()).unwrap();
    assert!(source_class_proc_call_at(source, &analysis, offset).is_none());
}

/// Itcl's two-word `Class name` form constructs an object called `name`
/// even when the class declares a class-scoped `proc name`; that proc is
/// reachable only as `Class::name`.
#[test]
fn itcl_two_word_form_is_construction_even_over_a_class_proc_name() {
    use crate::registry_invocation::source_structure::source_class_proc_call_at;
    let source =
        "itcl::class Factory {\n    proc make {} {\n        return 1\n    }\n}\nFactory make\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
    let offset = u32::try_from(source.rfind("Factory make").unwrap()).unwrap();
    assert!(source_class_proc_call_at(source, &analysis, offset).is_none());
    let call = source_constructor_call_at(source, &analysis, offset)
        .expect("the class command joins its source class");
    assert!(matches!(
        call.constructor_shape(&analysis),
        Some(super::OriginalSourceConstructorShape::BareWord {
            constructor_args_from: 1
        })
    ));
}
