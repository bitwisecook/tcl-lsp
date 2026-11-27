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
use crate::registry_invocation::InvocationWordOrigin;
use crate::registry_invocation::source_structure::{OriginalRegistrySource, source_registry_words};

#[test]
fn logical_original_advice_words_keep_lossless_values_and_native_units_independent() {
    // naming.source.authored-command-transition-advice
    // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
    use super::{AdviceNamingPolicy, original_words};
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    assert!(input.has_logical_source_name_context());
    let source = r"rename café\uFFFD café\uD800";
    let plan = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(source),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        config,
    )
    .unwrap();
    let originals = &plan.commands[0].words;
    let words = original_words(originals, &AdviceNamingPolicy::Logical(input)).unwrap();
    assert_eq!(words[1].value.as_deref(), Some("café�".as_bytes()));
    assert!(words[1].input.as_ref().unwrap().logical_input().is_some());
    assert!(words[1].input.as_ref().unwrap().native_input().is_none());
    assert_eq!(words[1].original, originals[1]);
    assert!(words[2].value.is_none());
    assert!(words[2].input.is_none());
    assert_eq!(words[2].original, originals[2]);
    assert_eq!(
        tcl_syntax::word_rules::original_static_word_source_bytes(&originals[2]).as_deref(),
        Some("café�".as_bytes())
    );

    let dialect = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let native_plan = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(source),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    )
    .unwrap();
    let native = original_words(
        &native_plan.commands[0].words,
        &AdviceNamingPolicy::Native(dialect.authored_name_policy().unwrap()),
    )
    .unwrap();
    assert!(native[2].value.is_some());
    assert!(native[2].input.as_ref().unwrap().native_input().is_some());
    assert!(native[2].input.as_ref().unwrap().logical_input().is_none());
}

#[test]
fn original_move_qualifier_retains_the_complete_operand_and_rejects_other_projections() {
    // naming.interpreter.original-created-handle-source-body
    // docs/design/analysis/name-resolution-proofs/interpreter-original-created-handle-source-body.md
    // This is a source-projection guard, not a successful runtime rename.
    use super::{AdviceNamingPolicy, original_transition_subject_matches, original_words};
    use tcl_registry::{
        CommandBindingTransition, NamespaceTransition, NamespaceTransitionTarget, StateTransition,
        StateTransitions, TransitionSubject,
    };
    let source = "rename s ::N::held";
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = tcl_native_model::InvocationDialect::for_version(version);
        let policy = dialect.authored_name_policy().unwrap();
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap();
        let words =
            original_words(&plan.commands[0].words, &AdviceNamingPolicy::Native(policy)).unwrap();
        let input = words[2].input.as_ref().unwrap();
        let from = TransitionSubject::LocatedNativeBytes {
            value: b"s".to_vec(),
            argument_index: 0,
        };
        let to = TransitionSubject::LocatedNativeBytes {
            value: b"::N::held".to_vec(),
            argument_index: 1,
        };
        let qualifier = TransitionSubject::LocatedNativeBytes {
            value: b"::N".to_vec(),
            argument_index: 1,
        };
        let mut transitions = StateTransitions::default();
        transitions.push(StateTransition::Namespace(NamespaceTransition::Ensure {
            namespace: NamespaceTransitionTarget::Named(qualifier.clone()),
        }));
        transitions.push(StateTransition::CommandBinding(
            CommandBindingTransition::Move { from, to },
        ));
        assert!(original_transition_subject_matches(
            &qualifier,
            input,
            &transitions
        ));
        assert_eq!(input.bytes(), b"::N::held");
        let mut unrelated = StateTransitions::default();
        unrelated.push(StateTransition::Namespace(NamespaceTransition::Ensure {
            namespace: NamespaceTransitionTarget::Named(qualifier.clone()),
        }));
        assert!(!original_transition_subject_matches(
            &qualifier, input, &unrelated
        ));
        let foreign = TransitionSubject::LocatedNativeBytes {
            value: b"::Other".to_vec(),
            argument_index: 1,
        };
        assert!(!original_transition_subject_matches(
            &foreign,
            input,
            &transitions
        ));
    }
}

fn last_words(
    source: &str,
    dialect: &str,
) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
    let analysis = Analyser::new().analyse(source, dialect);
    let config = analysis.body_lexer_config.unwrap();
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    source_registry_words(source, &analysis, commands.last().unwrap())
}

#[test]
fn authored_source_catalogue_barriers_keep_known_shadow_and_unloaded_advice_distinct() {
    // Implementation contract: naming.source.authored-command-transition-advice
    // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
    for source in [
        "package require Tk 8.6; proc entry args {}; entry .e -placeholder text",
        "package require Tk 8.6; proc entry args {}; rename entry {}; entry .e -placeholder text",
        "package require Tk 8.6; proc entry args {}; rename entry moved; entry .e -placeholder text",
    ] {
        assert!(last_words(source, "tcl8.6").is_none(), "{source}");
    }
    for source in [
        "package require Tk 8.6; entry .e -placeholder text",
        "package require Tk 8.6; mystery; entry .e -placeholder text",
    ] {
        assert!(last_words(source, "tcl8.6").is_some(), "{source}");
    }
}

#[test]
fn authored_source_tape_refusals_do_not_reissue_stock_declaration_advice() {
    // Implementation contract: naming.source.authored-command-transition-advice
    // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
    assert_eq!(
        last_words("if {1} {puts original}", "tcl")
            .unwrap()
            .command(),
        "if"
    );
    for source in [
        "proc if {args} {}; if {1} {puts shadowed}",
        "rename if {}; if {1} {puts deleted}",
        "rename if copied; if {1} {puts moved}",
    ] {
        assert!(last_words(source, "tcl").is_none(), "{source}");
    }
    assert_eq!(
        last_words("rename if copied; copied {1} {puts original}", "tcl")
            .unwrap()
            .command(),
        "if"
    );
    assert!(logical_last_body_words("proc p {} {if {1} {puts original}}").is_some());
    assert!(
        logical_last_body_words("proc p {} {proc if {args} {}; if {1} {puts shadowed}}").is_none()
    );
    assert!(logical_last_body_words("proc p {} {rename if {}; if {1} {puts deleted}}").is_none());
}

fn logical_last_body_words(
    source: &str,
) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
    let analysis = Analyser::new().analyse(source, "tcl");
    let config = analysis.body_lexer_config?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let roots = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    let root = source_registry_words(source, &analysis, roots.first()?)?;
    let bodies = root.source_script_bodies(&context);
    let span = bodies.first()?.content_span();
    let children = tcl_source::segment::segment_commands_with_offset_and_config(
        source.get(span.as_range())?,
        span.start(),
        config,
    );
    source_registry_words(source, &analysis, children.last()?)
}

#[test]
fn authored_source_transitions_keep_order_prefixes_and_conditional_authority() {
    // Implementation contract: naming.source.authored-command-transition-advice
    // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let source =
            "opaque\ninterp alias {} pick {} switch -exact\npick subject {default {puts ok}}";
        let mut analysis = Analyser::new().analyse(source, dialect);
        analysis.command_invocations.clear();
        let config = analysis.body_lexer_config.unwrap();
        let commands =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
        assert!(
            source_registry_words(source, &analysis, &commands[0]).is_none(),
            "{dialect}"
        );
        let words = source_registry_words(source, &analysis, &commands[2]).expect(dialect);
        assert_eq!(words.command(), "switch");
        assert_eq!(
            words.origins(),
            [
                InvocationWordOrigin::Written(0),
                InvocationWordOrigin::BindingPrefix(0),
                InvocationWordOrigin::Written(1),
                InvocationWordOrigin::Written(2)
            ]
        );
        assert!(words.operands()[0].is_none());
        assert_eq!(
            words.operands()[1].as_ref().unwrap().span(),
            commands[2].argv[1].span
        );
        assert!(!words.operands_preserve_source_lookup());
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("unavailable lookup borrowed another source purpose: {dialect}");
        };
        assert_eq!(advice.lineage().len(), 1);
        assert_eq!(advice.lineage()[0].original_words().len(), 7);
        assert_eq!(
            advice.uncertain_operations()[0][0].span(),
            commands[0].argv[0].span
        );
        assert!(
            advice
                .obligations()
                .contains(&super::SourceCommandTransitionObligation::UnknownEarlierMutation)
        );
        assert!(analysis.original_completed_command_world().is_none());
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        assert!(
            words
                .with_source_schema(&context, |schema| schema
                    .authored_source_case_invocation()
                    .is_some())
                .unwrap()
        );
        assert_eq!(words.source_script_bodies(&context).len(), 1);
        assert!(
            source_registry_words(&format!("{source}\n# stale"), &analysis, &commands[2]).is_none()
        );
        let foreign = tcl_registry::model::ingress::static_context_for(if dialect == "tcl8.4" {
            "tcl8.5"
        } else {
            "tcl8.4"
        });
        assert!(words.with_source_schema(foreign, |_| ()).is_none());
        let mut changed = analysis.clone();
        let mut same_grammar = config;
        same_grammar.leading_bom = match config.leading_bom {
            tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
            tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
        };
        changed.body_lexer_config = Some(same_grammar);
        assert!(source_registry_words(source, &changed, &commands[2]).is_none());
        if dialect == "tcl8.6" {
            let (_owner, native_entry) =
                crate::environment_ingress::captured_native_entry_with_owner(
                    tcl_dialect::DialectProfile::find(dialect).unwrap(),
                );
            let entry = std::sync::Arc::new(crate::command_binding::SourceAnalysisEntry {
                native_entry: Some(std::sync::Arc::new(native_entry)),
                ..Default::default()
            });
            let supplied = Analyser::new()
                .with_source_analysis_entry(entry)
                .analyse(source, dialect);
            let supplied = source_registry_words(source, &supplied, &commands[2]);
            assert!(
                supplied.is_none_or(|words| !matches!(
                    words.source(),
                    OriginalRegistrySource::SourceTransitions(_)
                )),
                "a supplied current entry cannot borrow fresh authored transition advice"
            );
        }
    }
}

#[test]
fn authored_immediate_bodies_keep_mutation_and_callback_obligations_separate() {
    // Implementation contract: naming.source.original-immediate-body-mutation-advice
    // docs/design/analysis/name-resolution-proofs/original-immediate-body-mutation-advice.md
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = version.dialect_profile_name();
        // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
        let prefix = "set ::auto_index(opaque) {source lib.tcl}; opaque; interp alias {} first {} switch; interp alias {} second {} switch; ";
        let source = format!(
            "{prefix}first subject {{default {{puts child}}}}; second subject {{default {{puts later}}}}"
        );
        let analysis = Analyser::new().analyse(&source, dialect);
        let config = analysis.body_lexer_config.unwrap();
        let commands =
            tcl_source::segment::segment_commands_with_offset_and_config(&source, 0, config);
        let words =
            source_registry_words(&source, &analysis, commands.last().unwrap()).expect(dialect);
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("source purpose changed");
        };
        assert_eq!(advice.lineage().len(), 1);
        assert!(
            advice
                .obligations()
                .contains(&super::SourceCommandTransitionObligation::UnknownEarlierMutation)
        );
        let child = u32::try_from(source.find("puts child").unwrap()).unwrap();
        assert!(advice.uncertain_operations().iter().any(|words| {
            words
                .first()
                .is_some_and(|word| word.span().start() == child)
        }));
        assert!(!words.operands_preserve_source_lookup());
        assert!(analysis.original_completed_command_world().is_none());
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        assert_eq!(words.source_script_bodies(&context).len(), 1);

        // Complete nested source transitions withdraw the prior alias,
        // including a command substitution that runs before its operand.
        for body in [
            "rename second {}",
            "rename second moved",
            "interp alias {} second {} puts",
            "if {1} {rename second {}}",
            "puts [rename second {}]",
            "if {[rename second {}] eq {}} {puts expression}",
            "if {0 && [rename second {}]} {puts lazy}",
        ] {
            let source = format!(
                "{prefix}first subject {{default {{{body}}}}}; second subject {{default {{puts stale}}}}"
            );
            assert!(last_words(&source, dialect).is_none(), "{dialect}: {body}");
        }
        let registration = if dialect == "tcl8.4" {
            "trace variable watched r hook"
        } else {
            "trace add variable watched read hook"
        };
        let source = format!(
            "{prefix}first subject {{default {{{registration}; set watched}}}}; second subject {{default {{puts observed}}}}"
        );
        let words = last_words(&source, dialect);
        if let Some(words) = words {
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("observer borrowed current selection");
            };
            assert!(
                advice
                    .obligations()
                    .contains(&super::SourceCommandTransitionObligation::UnknownEarlierMutation)
            );
            let observer = u32::try_from(source.find(registration).unwrap()).unwrap();
            assert!(
                advice.uncertain_operations().iter().any(|words| {
                    words
                        .first()
                        .is_some_and(|word| word.span().start() == observer)
                }),
                "{dialect}: registration={observer}, uncertain={:?}",
                advice
                    .uncertain_operations()
                    .iter()
                    .map(|words| words.first().map(tcl_lexer::NativeWord::span))
                    .collect::<Vec<_>>()
            );
            assert!(!words.operands_preserve_source_lookup());
        }
    }
}

#[test]
fn authored_dynamic_operands_keep_source_advice_before_the_future_barrier() {
    // Implementation contract: naming.source.original-immediate-body-mutation-advice
    // docs/design/analysis/name-resolution-proofs/original-immediate-body-mutation-advice.md
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = version.dialect_profile_name();
        let source = "opaque; interp alias {} pick {} switch; pick $value {default {puts body}}; pick subject {default {puts after}}";
        let analysis = Analyser::new().analyse(source, dialect);
        let config = analysis.body_lexer_config.unwrap();
        let commands =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
        let words = source_registry_words(source, &analysis, &commands[2]).expect(dialect);
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("dynamic argv changed source purpose");
        };
        assert!(
            advice
                .obligations()
                .contains(&super::SourceCommandTransitionObligation::UnknownEarlierMutation)
        );
        assert!(
            advice
                .uncertain_operations()
                .iter()
                .any(|words| words.as_ref() == advice.original_words())
        );
        assert_eq!(
            words.arguments()[0],
            crate::registry_invocation::EffectiveInvocationWord::Dynamic
        );
        assert!(!words.operands_preserve_source_lookup());
        assert!(source_registry_words(source, &analysis, &commands[3]).is_none());
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        assert_eq!(words.source_script_bodies(&context).len(), 1);
        let source = "opaque; interp alias {} pick {} switch; pick [rename pick {}] {default {puts deleted}}";
        assert!(last_words(source, dialect).is_none());
    }
}

#[test]
fn authored_source_transition_tape_keeps_deletion_rebinding_and_namespace_geometry() {
    // Implementation contract: naming.source.authored-command-transition-advice
    // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
    for source in [
        "opaque; interp alias {} pick {} switch; pick subject {default {puts yes}}",
        "opaque; interp alias {} pick {} puts; interp alias {} pick {}; interp alias {} pick {} switch; pick subject {default {puts yes}}",
        "opaque; rename switch pick; pick subject {default {puts yes}}",
        "opaque; namespace eval ::A {}; interp alias {} ::A::pick {} ::switch; ::A::pick subject {default {puts yes}}",
        "opaque; interp alias {} a {} switch -exact; interp alias {} b {} a subject; b {default {puts yes}}",
    ] {
        let words = last_words(source, "tcl8.6").unwrap_or_else(|| panic!("{source}"));
        assert_eq!(words.command(), "switch", "{source}");
        assert!(
            matches!(words.source(), OriginalRegistrySource::SourceTransitions(_)),
            "{source}"
        );
        assert!(!words.operands_preserve_source_lookup());
    }
    let chain = last_words(
            "opaque; interp alias {} a {} switch -exact; interp alias {} b {} a subject; b {default {puts yes}}",
            "tcl8.6",
        ).unwrap();
    assert_eq!(
        chain.origins(),
        [
            InvocationWordOrigin::Written(0),
            InvocationWordOrigin::BindingPrefix(0),
            InvocationWordOrigin::BindingPrefix(1),
            InvocationWordOrigin::Written(1)
        ]
    );
    assert_eq!(chain.arguments()[0].literal_bytes(), Some(&b"-exact"[..]));
    assert_eq!(chain.arguments()[1].literal_bytes(), Some(&b"subject"[..]));
    assert!(chain.operands()[0].is_none() && chain.operands()[1].is_none());
    let OriginalRegistrySource::SourceTransitions(advice) = chain.source() else {
        panic!("wrong purpose");
    };
    assert_eq!(advice.lineage().len(), 2);
    let source = "opaque; interp alias {} factory {} interp alias; factory {} pick {} switch; pick subject {default {puts yes}}";
    let operation = last_words(source, "tcl8.6").unwrap();
    assert_eq!(operation.command(), "switch");
    let OriginalRegistrySource::SourceTransitions(advice) = operation.source() else {
        panic!("wrong purpose");
    };
    let transition = &advice.lineage()[0];
    assert_eq!(
        transition.original_words()[0].span().start(),
        u32::try_from(source.find("factory {} pick").unwrap()).unwrap()
    );
    assert_eq!(transition.descriptor(), "interp");
    assert_eq!(transition.selection_lineage().len(), 1);
    assert_eq!(
        transition.selection_lineage()[0].site().offset,
        u32::try_from(source.find("interp alias").unwrap()).unwrap()
    );
    assert_eq!(transition.selection_lineage()[0].original_words().len(), 7);
    for source in [
        "opaque; pick subject {default {puts no}}",
        "opaque; interp alias {} pick {} switch; interp alias {} pick {}; pick subject {default {puts no}}",
        "opaque; rename switch pick; rename pick {}; pick subject {default {puts no}}",
        "opaque; proc switch {args} {}; interp alias {} pick {} switch; pick subject {default {puts no}}",
        "opaque; interp alias {} pick {} switch; proc pick {args} {}; pick subject {default {puts no}}",
        "opaque; interp alias {} pick {} switch; unknown; pick subject {default {puts no}}",
        "opaque; interp alias {} pick {} switch; interp alias {} pick {} $target; pick subject {default {puts no}}",
        "opaque; interp alias {} a {} b; interp alias {} b {} a; a subject {default {puts no}}",
        "opaque; interp alias {} ::missing::pick {} switch; ::missing::pick subject {default {puts no}}",
        "opaque; rename switch puts; puts subject {default {puts no}}",
        "opaque; namespace eval A {interp alias {} pick {} switch}; pick subject {default {puts no}}",
    ] {
        assert!(
            last_words(source, "tcl8.6").is_none_or(|words| !matches!(
                words.source(),
                OriginalRegistrySource::SourceTransitions(_)
            )),
            "{source}"
        );
    }
}

#[test]
fn authored_logical_source_advice_keeps_its_explicit_input_and_original_operands() {
    // Implementation contract: naming.source.authored-command-transition-advice
    // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
    for source in [
        "pick subject {default {puts before}}; interp alias {} pick {} switch -exact; pick subject {default {puts after}}",
        "pick subject {default {puts before}}; rename switch pick; pick subject {default {puts after}}",
        "::N::pick subject {default {puts before}}; namespace eval N {}; interp alias {} ::N::pick {} ::switch; ::N::pick subject {default {puts after}}",
    ] {
        let syntax =
            crate::registry_invocation::source_structure::OriginalSourceRegistryContext::capture(
                source,
                input.clone(),
            );
        let commands =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
        assert!(syntax.words(&commands[0]).is_none());
        let foreign_generation =
            context.with_command_store(context.commands().snapshot().shared_registry());
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            commands.last().unwrap(),
        );
        syntax
            .retained_source_realm()
            .stamp_original_tokens(&mut tokens);
        assert!(
            syntax
                .retained_source_realm()
                .original_source_transition_advice(&foreign_generation, &tokens)
                .is_none()
        );
        // A rejected equivalent foreign generation cannot poison the
        // actual input's shared source-advice cache.
        let words = syntax
            .words(commands.last().unwrap())
            .expect("explicit Logical source transition advice");
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("source purpose changed");
        };
        assert!(advice.original_head().native_input().is_none());
        assert!(advice.original_head().logical_input().is_some());
        assert_eq!(advice.logical_source_input(), Some(syntax.editing_input()));
        assert!(words.head_source().unwrap().input().is_none());
        assert!(!words.operands_preserve_source_lookup());
        assert!(!advice.matches_context(&foreign_generation));
        assert!(
            words
                .with_source_schema(&foreign_generation, |_| ())
                .is_none()
        );
        assert!(
            advice
                .lineage()
                .iter()
                .flat_map(|step| step.inputs())
                .all(|(_, input)| input.native_input().is_none())
        );
        assert!(
            words
                .source_script_bodies(&context)
                .iter()
                .any(|body| source.get(body.content_span().as_range()) == Some("puts after"))
        );
        assert!(
            words
                .with_source_schema(
                    &tcl_registry::model::ingress::context_for_profile(
                        tcl_dialect::DialectProfile::find("tcl9.0").unwrap()
                    ),
                    |_| ()
                )
                .is_none()
        );
    }
    for source in [
        "opaque; interp alias {} pick {} switch; interp alias {} pick {}; pick subject {default {puts deleted}}",
        "opaque; interp alias {} pick {} switch; proc pick {args} {}; pick subject {default {puts shadowed}}",
        "opaque; interp alias {} pick {} switch; unknown; pick subject {default {puts changed}}",
        "opaque; interp alias {} pick {} $target; pick subject {default {puts dynamic}}",
    ] {
        let syntax =
            crate::registry_invocation::source_structure::OriginalSourceRegistryContext::capture(
                source,
                input.clone(),
            );
        let commands =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
        assert!(syntax.words(commands.last().unwrap()).is_none(), "{source}");
    }
}

#[test]
fn authored_jim_root_alias_advice_keeps_the_jim_name_recipe() {
    // Implementation contract: naming.source.authored-command-transition-advice
    // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
    let source = "opaque; alias pick puts; pick hello";
    let words = last_words(source, "jim").unwrap();
    assert_eq!(words.command(), "puts");
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        panic!("wrong purpose");
    };
    assert_eq!(
        advice
            .original_head()
            .native_input()
            .unwrap()
            .policy()
            .recipe(),
        tcl_syntax::naming::NativeNameProtocol::Jim084
    );
    assert_eq!(advice.lineage()[0].descriptor(), "alias");
    assert_eq!(advice.lineage()[0].original_words().len(), 3);
    assert_eq!(
        advice.lineage()[0].inputs().len(),
        2,
        "implicit interpreter constants have no original source operands"
    );
    assert!(
        advice.lineage()[0]
            .inputs()
            .iter()
            .all(|(subject, _)| subject.argument_index().is_some())
    );
}
fn original_unicode_source_input() -> crate::analyser::ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        strict_quoting: true,
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, config)
}

#[test]
fn original_logical_unicode_transitions_keep_exact_alias_and_move_inputs() {
    // naming.source.authored-command-transition-advice
    // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
    // Conditional source transitions only; no Native input or successful dispatch.
    let input = original_unicode_source_input();
    assert!(input.has_logical_source_name_context());
    for (source, command, head) in [
        (
            "rename puts \u{feff}puts; \u{feff}puts value",
            "puts",
            "\u{feff}puts",
        ),
        (
            "interp alias {} café {} puts FIXED; café value",
            "puts",
            "café",
        ),
        (
            "namespace eval λ {}; rename puts ::λ::é; ::λ::é value",
            "puts",
            "::λ::é",
        ),
    ] {
        let syntax =
            crate::registry_invocation::source_structure::OriginalSourceRegistryContext::capture(
                source,
                input.clone(),
            );
        let commands = tcl_source::segment::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        );
        let selected = syntax
            .words(commands.last().unwrap())
            .expect("exact Logical UTF-8 source target");
        assert_eq!(selected.command(), command);
        let OriginalRegistrySource::SourceTransitions(advice) = selected.source() else {
            panic!("source transition purpose required")
        };
        assert_eq!(advice.original_head().bytes(), head.as_bytes());
        assert!(advice.original_head().native_input().is_none());
        assert!(advice.original_head().logical_input().is_some());
        assert_eq!(advice.logical_source_input(), Some(&input));
        assert!(!selected.operands_preserve_source_lookup());
        assert!(
            advice
                .lineage()
                .iter()
                .flat_map(|step| step.inputs())
                .all(|(_, input)| input.native_input().is_none())
        );
        assert_eq!(advice.original_words()[0].config(), input.lexer_config());
        assert_eq!(
            advice.original_words()[0].image().bytes(),
            source.as_bytes()
        );
        let context = input.borrowed_context_registry();
        let foreign = context.with_command_store(context.commands().snapshot().shared_registry());
        assert!(selected.with_source_schema(&foreign, |_| ()).is_none());
    }
    for source in [
        "rename puts café; rename café {}; café value",
        "interp alias {} café {} puts; unknown; café value",
        "interp alias {} café {} $target; café value",
        "interp alias {} café {} puts; proc café {args} {}; café value",
    ] {
        let syntax =
            crate::registry_invocation::source_structure::OriginalSourceRegistryContext::capture(
                source,
                input.clone(),
            );
        let commands = tcl_source::segment::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        );
        assert!(syntax.words(commands.last().unwrap()).is_none(), "{source}");
    }
}

#[test]
fn original_logical_nested_expression_keeps_bom_head_and_parent_configuration() {
    // naming.source.authored-command-transition-advice
    // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
    // Original expression child source schema; no entered matcher or output value.
    let input = original_unicode_source_input();
    let source = "\u{feff}interp alias {} \u{feff}match {} regexp\nif {![\u{feff}match λ λ out]} {puts $out}";
    let syntax =
        crate::registry_invocation::source_structure::OriginalSourceRegistryContext::capture(
            source,
            input.clone(),
        );
    let start = u32::try_from(source.find("\u{feff}match λ").unwrap()).unwrap();
    let end = u32::try_from(source.find("]}").unwrap()).unwrap();
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(
        &source[start as usize..end as usize],
        start,
        input.lexer_config().nested(),
    );
    let [child] = commands.as_slice() else {
        panic!("one original expression child")
    };
    let selected = syntax
        .words(child)
        .expect("nested BOM is a retained head unit");
    assert_eq!(selected.command(), "regexp");
    let OriginalRegistrySource::SourceTransitions(advice) = selected.source() else {
        panic!("source transition purpose required")
    };
    assert_eq!(advice.original_head().bytes(), "\u{feff}match".as_bytes());
    assert_eq!(advice.original_words()[0].span().start(), start);
    assert!(
        advice
            .original_words()
            .iter()
            .all(|word| word.config() == input.lexer_config()
                && word.image().bytes() == source.as_bytes())
    );
    assert!(advice.original_head().native_input().is_none());
    assert_eq!(advice.logical_source_input(), Some(&input));
    let changed = tcl_source::segment::segment_commands_with_offset_and_config(
        "match λ λ out",
        start,
        input.lexer_config().nested(),
    );
    assert!(syntax.words(&changed[0]).is_none());
    let foreign_source = format!("{source} ");
    assert!(!advice.matches_source(
        &tcl_lexer::SourceImage::document(&foreign_source),
        input.lexer_config()
    ));
}

#[test]
fn conditional_declaration_memo_reuses_only_actual_owner_and_baseline() {
    use super::*;
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::for_file_grammar(registry.profile().unwrap().grammar);
    let source = "rename $old decl\nproc kept {arg} {list $arg}";
    let bindings = crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
    let owner = bindings.original_analysis_owner().unwrap();
    let seed = bindings.original_source_transition_seed().unwrap();
    let context = seed.source_context(registry).unwrap();
    let first = conditional_procedure_declaration_selections(owner, &seed, &context).unwrap();
    let second = conditional_procedure_declaration_selections(owner, &seed, &context).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    let offset = u32::try_from(source.find("proc kept").unwrap()).unwrap();
    assert!(first.get(&offset).unwrap().is_some());
    let different_baseline = OriginalSourceTransitionSeed {
        baseline: Arc::new((*seed.baseline).clone()),
        ..seed.clone()
    };
    let separate =
        conditional_procedure_declaration_selections(owner, &different_baseline, &context).unwrap();
    assert!(!Arc::ptr_eq(&first, &separate));
    assert_eq!(first, separate);
    assert!(Arc::ptr_eq(
        &first,
        &conditional_procedure_declaration_selections(owner, &seed, &context).unwrap()
    ));
}

#[test]
fn conditional_declaration_memo_cannot_donate_to_foreign_source_or_unavailable_metadata() {
    use super::*;
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::for_file_grammar(registry.profile().unwrap().grammar);
    let source = "rename $old decl\nproc kept {} {list retained}";
    let bindings = crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
    let owner = bindings.original_analysis_owner().unwrap();
    let seed = bindings.original_source_transition_seed().unwrap();
    let context = seed.source_context(registry).unwrap();
    assert!(conditional_procedure_declaration_selections(owner, &seed, &context).is_some());
    let foreign = crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
    assert!(
        conditional_procedure_declaration_selections(
            owner,
            &foreign.original_source_transition_seed().unwrap(),
            &context
        )
        .is_none()
    );
    let mut unavailable = seed.clone();
    Arc::make_mut(&mut unavailable.baseline).metadata_context =
        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
    assert!(conditional_procedure_declaration_selections(owner, &unavailable, &context).is_none());
}
#[test]
fn conditional_memo_does_not_retain_its_actual_analysis_owner() {
    use super::*;
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::for_file_grammar(registry.profile().unwrap().grammar);
    let source = "rename $old decl\nproc kept {} {list retained}";
    let bindings = crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
    let owner = Arc::clone(bindings.original_analysis_owner().unwrap());
    let seed = bindings.original_source_transition_seed().unwrap();
    let context = seed.source_context(registry).unwrap();
    let selected = conditional_procedure_declaration_selections(&owner, &seed, &context).unwrap();
    let weak = Arc::downgrade(&owner);
    let memo = owner.derived_memo();
    let memo_lifetime = Arc::downgrade(&memo);
    let equal = crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
    assert!(!Arc::ptr_eq(
        &memo,
        &equal.original_analysis_owner().unwrap().derived_memo()
    ));
    drop(memo);
    drop(owner);
    drop(seed);
    drop(bindings);
    assert!(weak.upgrade().is_none());
    // Selected source syntax may outlive its analysis without owning it.
    assert!(!selected.is_empty());
    let _ = equal.original_analysis_owner().unwrap().derived_memo();
    assert!(memo_lifetime.upgrade().is_none());
}
