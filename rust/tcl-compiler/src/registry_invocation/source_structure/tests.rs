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

use crate::interprocedural::test_body_identity::fixture_id_if_present;

#[test]
fn first_tail_command_matches_the_whole_tail_segmentation() {
    let long = "x".repeat(300);
    let sources = [
        format!("set a 1\nset b 2; set c 3\n# note\nputs $a\n{long}\n"),
        format!("proc p {{}} {{\n    set x 1\n    set y {{{long}}}\n}}\np\nset z 2\n"),
        format!("namespace eval :: {{source child.tcl}}; set q {{{long}\n}}\nset r 1\n"),
        format!("set s \"café {long}\"\nset t [list a \\\n b]\nset u 1\n"),
        format!("set v {{unclosed {long}\nset w 1\n"),
    ];
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6").nested();
    for source in &sources {
        for (offset, _) in source.char_indices() {
            let tail = &source[offset..];
            let offset = u32::try_from(offset).unwrap();
            assert_eq!(
                super::first_tail_command(tail, offset, config),
                crate::segmenter::segment_commands_with_offset_and_config(tail, offset, config)
                    .into_iter()
                    .next(),
                "{offset}"
            );
        }
    }
}

use super::*;

fn original_selector_analysis(source: &str) -> crate::analyser::AnalysisResult {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let profile = environment.analyser_profile();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        environment.default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    crate::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

#[test]
fn logical_source_words_keep_complete_receiver_catalogue_vectors_and_current_inputs() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../experiments/name-handling/patterns/composed-callback-prefix/probes/positive.tcl"
    ));
    let mut analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let offset = u32::try_from(source.find("after idle $cb").unwrap()).unwrap();
    assert!(
        analysis
            .retained_command_realm()
            .unwrap()
            .invocation_at_source("", offset)
            .original_recorded_command()
            .is_none(),
        "the source catalogue must supply this complete vector"
    );
    let words = original_logical_source_words_at(source, &analysis, offset)
        .expect("complete original Logical receiver source vector");
    assert_eq!(
        words
            .iter()
            .map(|word| word.try_text().unwrap())
            .collect::<Vec<_>>(),
        ["after", "idle", "$cb"]
    );
    let input = analysis.resolved_input.as_ref().unwrap().clone();
    assert!(words.iter().all(
        |word| word.image() == &tcl_lexer::SourceImage::document(source)
            && word.config() == input.lexer_config()
    ));
    assert!(original_logical_source_words_at(&format!("{source} "), &analysis, offset).is_none());
    assert!(original_logical_source_words_at(source, &analysis, offset + 6).is_none());
    analysis.resolved_input = None;
    assert!(original_logical_source_words_at(source, &analysis, offset).is_none());
    analysis.resolved_input = Some(input.clone());
    let mut changed = input.lexer_config();
    changed.strict_quoting = !changed.strict_quoting;
    analysis.body_lexer_config = Some(changed);
    assert!(original_logical_source_words_at(source, &analysis, offset).is_none());
    analysis.body_lexer_config = Some(input.lexer_config());
    analysis.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        input.lexer_config(),
    ));
    assert!(original_logical_source_words_at(source, &analysis, offset).is_none());
    let source = "list VALUE";
    let logical = crate::analyser::Analyser::new().analyse(source, "tcl");
    assert_eq!(
        original_logical_source_words_at(source, &logical, 0)
            .unwrap()
            .len(),
        2
    );
    let native = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(original_logical_source_words_at(source, &native, 0).is_none());
}

fn selected_source_variable_names(
    invocation: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Vec<(usize, tcl_registry::ArgRole)> {
    invocation.authored_source_variable_name_arguments()
}

#[test]
fn original_source_prefix_catalogue_keeps_fatal_tail_separate() {
    for dialect in ["tcl", "tcl8.6"] {
        let source = "set arr(key) 1\nset v $arr(k more stuff\n";
        let environment = tcl_registry::model::ingress::resolve_environment(dialect);
        let profile = environment.analyser_profile();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            environment.default_context_registry(),
            config,
        );
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        assert!(plan.fatal_tail.is_some());
        assert_eq!(plan.commands.len(), 1);
        let words = source_registry_words_at(source, &analysis, 0)
            .expect("the genuine complete prefix keeps readonly source roles");
        if input.has_logical_source_name_context() {
            assert!(words.head_source().unwrap().input().is_none());
            assert!(!words.operands_preserve_source_lookup());
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("the complete prefix keeps its original Logical source purpose");
            };
            assert!(advice.original_head().logical_input().is_some());
            assert!(advice.original_head().native_input().is_none());
        }
        assert_eq!(
            words.original_argument_word(0),
            Some(&plan.commands[0].words[1])
        );
        assert_eq!(
            words.with_source_schema(&context, selected_source_variable_names),
            Some(vec![(0, tcl_registry::ArgRole::VarWrite)])
        );
        assert!(source_registry_words_at(source, &analysis, 15).is_none());
        assert!(source_registry_words_at(&format!("{source}# changed"), &analysis, 0).is_none());
        let foreign = tcl_registry::model::ingress::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
        );
        assert!(words.with_source_schema(&foreign, |_| ()).is_none());
    }
}

#[test]
fn original_body_catalogue_keeps_source_roles_after_conditional_loop() {
    for dialect in ["tcl", "tcl8.6"] {
        for source in [
            "proc f {n} {while {$n > 0} {set x 1; incr n -1}; puts $x}",
            "oo::class create C {method f {n} {while {$n > 0} {set x 1; incr n -1}; puts $x}}",
        ] {
            let environment = tcl_registry::model::ingress::resolve_environment(dialect);
            let profile = environment.analyser_profile();
            let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                environment.default_context_registry(),
                config,
            );
            let analysis = crate::analyser::Analyser::new()
                .with_resolved_input(input)
                .analyse(source, profile.name);
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            for (head, expected) in [
                ("set x", vec![(0, tcl_registry::ArgRole::VarWrite)]),
                ("incr n", vec![(0, tcl_registry::ArgRole::VarWrite)]),
                ("puts $x", vec![]),
            ] {
                let offset = u32::try_from(source.find(head).unwrap()).unwrap();
                let words =
                    source_registry_words_at(source, &analysis, offset).unwrap_or_else(|| {
                        panic!("conditional original schema absent: {dialect} {head}")
                    });
                let roles = words
                    .with_source_schema(&context, |schema| {
                        schema.authored_source_variable_name_arguments()
                    })
                    .unwrap();
                assert_eq!(roles, expected, "{dialect} {head}");
                assert!(words.matches_source(&tcl_lexer::SourceImage::document(source), config));
            }
        }
    }
    for source in [
        "proc set args {}; proc f {} {set x 1}",
        "rename set saved; proc f {} {set x 1}",
    ] {
        let analysis = original_selector_analysis(source);
        let offset = u32::try_from(source.rfind("set x").unwrap()).unwrap();
        assert!(source_registry_words_at(source, &analysis, offset).is_none());
    }
}

#[test]
fn original_source_schema_keeps_named_selector_and_handler_argument_shape() {
    // naming.source.original-registry-header-advice
    // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../experiments/name-handling/patterns/composed-callback-prefix/probes/positive.tcl"
    ));
    let analysis = original_selector_analysis(source);
    let input = analysis.resolved_input.as_ref().unwrap();
    let config = input.lexer_config();
    let setup = analysis.all_classes["::C"]
        .original_members
        .declarations()
        .find(|member| member.original_name_input().bytes() == b"setup")
        .unwrap();
    let plan = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(source),
        setup.body_word().unwrap().content_span().unwrap(),
        config,
    )
    .unwrap();
    assert!(plan.fatal_tail.is_none());
    let wrapper = original_single_source_command_substitution(&plan.commands[0].words[2]).unwrap();
    assert_eq!(wrapper.command().words[1].bytes(), b"code");
    let span = wrapper.command().span;
    let commands = crate::segmenter::segment_commands_with_offset_and_config(
        &source[span.as_range()],
        span.start(),
        config,
    );
    let [command] = commands.as_slice() else {
        panic!("one original wrapper");
    };
    let realm = analysis.retained_command_realm().unwrap();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        command,
    );
    realm.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(binding.named_invocation().is_some());
    let context = input.context_registry();
    let handler_advice = crate::registry_invocation::registry_invocation_assistance(
        context.commands(),
        None,
        &tokens,
    )
    .unwrap();
    assert_eq!(
        handler_advice
            .unanimous_command_words()
            .unwrap()
            .effective()
            .origins
            .get(1),
        Some(&crate::registry_invocation::InvocationWordOrigin::Written(
            2
        ))
    );
    let words = source_registry_words(source, &analysis, command).unwrap();
    assert_eq!(
        words.origins().get(1),
        Some(&crate::registry_invocation::InvocationWordOrigin::Written(
            1
        ))
    );
    assert_eq!(
        words.original_argument_word(0),
        Some(&wrapper.command().words[1])
    );
    assert_eq!(
        words.original_argument_word(1),
        Some(&wrapper.command().words[2])
    );
    assert_eq!(
        words.with_source_schema(&context, |schema| (
            schema
                .subcommand
                .resolved()
                .map(|selected| selected.canonical_name),
            schema.semantics.argument_offset,
            schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::WRAPS_COMMAND_PREFIX),
            schema.authored_source_plain_script_arguments(),
        )),
        Some((Some("code"), 1, true, Some(vec![1])))
    );
    let changed = format!("{source}# changed complete input\n");
    assert!(source_registry_words(&changed, &analysis, command).is_none());
}

#[test]
fn original_source_schema_withholds_dynamic_and_shadowed_selector_claims() {
    // naming.source.original-registry-header-advice
    // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
    let dynamic = "namespace $selector [list tick]";
    let analysis = original_selector_analysis(dynamic);
    let config = analysis.resolved_input.as_ref().unwrap().lexer_config();
    let command =
        crate::segmenter::segment_commands_with_offset_and_config(dynamic, 0, config).remove(0);
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let wraps = source_registry_words(dynamic, &analysis, &command).and_then(|words| {
        words.with_source_schema(&context, |schema| {
            schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::WRAPS_COMMAND_PREFIX)
        })
    });
    assert_ne!(wraps, Some(true));
    let shadowed = "proc namespace args {}; namespace code [list tick]";
    let analysis = original_selector_analysis(shadowed);
    let config = analysis.resolved_input.as_ref().unwrap().lexer_config();
    let command = crate::segmenter::segment_commands_with_offset_and_config(shadowed, 0, config)
        .pop()
        .unwrap();
    assert!(source_registry_words(shadowed, &analysis, &command).is_none());
}

#[test]
fn original_single_source_child_keeps_whole_operand_and_refuses_literal_brackets() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    for (source, expected) in [
        ("set x [list cb]", true),
        ("set x {[list cb]}", false),
        ("set x \"[list cb]suffix\"", false),
        ("set x [list cb; list other]", false),
    ] {
        let image = tcl_lexer::SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in(
            image.clone(),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert!(plan.fatal_tail.is_none());
        let operand = &plan.commands[0].words[2];
        let child = original_single_source_command_substitution(operand);
        assert_eq!(child.is_some(), expected, "{source}");
        if let Some(child) = child {
            assert_eq!(child.original_operand(), operand);
            assert_eq!(child.command().words.len(), 2);
            assert!(
                child
                    .command()
                    .words
                    .iter()
                    .all(|word| word.image() == &image)
            );
            assert!(child.command().span.start() > operand.span().start());
            assert!(child.command().span.end() < operand.span().end());
        }
    }
}

#[test]
fn original_case_fold_child_keeps_declaration_realm_and_refuses_foreign_source_owners() {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    // Original conditional child syntax, not a Native folded value or body entry.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let input = logical_schema_input(std::sync::Arc::clone(&context));
    let source = "proc f {cmd} {puts [string tolower $cmd]}";
    let mut lowerer =
        crate::lowering::Lowerer::with_config(context.commands(), input.lexer_config())
            .with_resolved_analysis_input(input.clone());
    let module = lowerer.lower(source);
    let body = &fixture_id_if_present(&module.procedures, "::f")
        .and_then(|id| module.procedures.get(&id))
        .unwrap()
        .body;
    let tokens = body
        .retained_source_tokens_for_statement(&body.statements[0])
        .unwrap();
    let metadata =
        super::super::InvocationMetadataContext::for_analysis_input(context.commands(), &input)
            .unwrap();
    let subject = &tokens.words()[1];
    let folded = original_folded_case_subject(tokens, subject, context.commands(), metadata)
        .expect("original Logical declaration child retains its selected schema realm");
    assert!(
        matches!(folded, crate::ir::WordExpr::Variable { ref spelling, .. } if spelling == "$cmd")
    );
    assert_eq!(
        folded.source().span.start(),
        u32::try_from(source.find("$cmd").unwrap()).unwrap()
    );
    let calls = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
        tokens,
        input.lexer_config(),
        context.commands(),
        metadata,
    )
    .unwrap();
    assert_eq!(calls.len(), 1);
    let child = calls[0].tokens.as_ref().unwrap();
    let binding = child.source_binding.as_ref().unwrap();
    assert!(
        binding.invocation_realm().is_none(),
        "conditional declaration is not a reached child"
    );
    assert!(
        binding
            .original_source_realm_for_input(child, &input, context.commands())
            .is_some()
    );
    assert!(
        binding
            .original_source_word_dialect_for_tokens(child)
            .unwrap()
            .native_name_protocol()
            .is_none()
    );
    let mut missing = tokens.clone();
    missing.nested_bindings_mut().clear();
    assert!(
        original_folded_case_subject(&missing, &missing.words()[1], context.commands(), metadata)
            .is_none()
    );
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    let mut foreign_store = tcl_registry::CommandRegistry::build_default();
    foreign_store.insert(tcl_registry::CommandSpec {
        name: "foreign_case_child_axis",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let foreign =
        std::sync::Arc::new(context.with_command_store(std::sync::Arc::new(foreign_store)));
    for changed in [
        crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            std::sync::Arc::clone(&context),
            config,
        ),
        logical_schema_input(foreign),
    ] {
        let metadata = super::super::InvocationMetadataContext::for_analysis_input(
            changed.borrowed_context_registry().commands(),
            &changed,
        )
        .unwrap();
        assert!(
            original_folded_case_subject(tokens, subject, context.commands(), metadata).is_none()
        );
    }
}

fn selected_subcommand_diagnostic(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::AuthoredSourceSubcommandDiagnostic> {
    schema.authored_source_subcommand_diagnostic()
}
fn selected_argument_roles(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> (Vec<(u8, tcl_registry::ArgRole)>, bool) {
    schema.authored_source_argument_roles()
}
fn selected_rule_procedure_operand(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<usize> {
    schema.authored_source_rule_procedure_operand()
}

fn logical_schema_input(
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> crate::analyser::ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    assert!(input.has_logical_source_name_context());
    input
}

fn last_logical_schema(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Option<OriginalRegistryWords> {
    let analysis = crate::analyser::Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, "tcl");
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, input.lexer_config())
            .pop()?;
    source_registry_words(source, &analysis, &segment)
}

#[test]
fn original_logical_schema_matches_selected_descriptors_without_slot_label_equality() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let input = logical_schema_input(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
    );
    for (source, expected) in [
        ("::eval $script", "eval"),
        ("::subst $template", "subst"),
        ("rename eval ::Held; ::Held $script", "eval"),
        ("interp alias {} run {} eval; run $script", "eval"),
        (
            "interp alias {} run {} eval {set local}; run $script",
            "eval",
        ),
    ] {
        let words = last_logical_schema(source, &input).expect(source);
        let expected = input
            .borrowed_context_registry()
            .context()
            .resolve_spec(input.borrowed_context_registry().commands(), expected)
            .unwrap();
        let selected = words
            .with_source_schema(input.borrowed_context_registry(), |schema| {
                schema.authored_source_descriptors().command
            })
            .unwrap();
        assert!(std::ptr::eq(selected, expected), "{source}");
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("independent Logical source purpose")
        };
        assert_eq!(advice.logical_source_input(), Some(&input));
        assert!(words.head_source().unwrap().input().is_none());
        assert!(!words.operands_preserve_source_lookup());
    }
    for source in [
        "proc eval args {}; ::eval $script",
        "interp alias {} run {} eval; rename eval {}; run $script",
        "interp alias {} run {} eval; rename eval Held; run $script",
        "interp alias {} run {} eval; proc eval args {}; run $script",
    ] {
        assert!(last_logical_schema(source, &input).is_none(), "{source}");
    }
}

#[test]
fn original_logical_schema_keeps_same_store_availability_and_exact_source_owner() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut command = registry.get("eval").unwrap().clone();
    command.name = "eval_current";
    command.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(command);
    let current = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(registry)));
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(current.commands())),
    );
    assert!(std::sync::Arc::ptr_eq(current.commands(), older.commands()));
    let input = logical_schema_input(current);
    let source = "::eval_current $script";
    let words = last_logical_schema(source, &input).expect("actual available descriptor");
    assert!(last_logical_schema(source, &logical_schema_input(older)).is_none());
    let original = crate::analyser::Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, "tcl");
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, input.lexer_config())
            .pop()
            .unwrap();
    for axis in 0..4 {
        let mut analysis = original.clone();
        match axis {
            0 => analysis.resolved_input = None,
            1 => {
                analysis.resolved_input = Some(logical_schema_input(
                    tcl_registry::model::ingress::resolve_environment("tcl8.6")
                        .default_context_registry(),
                ));
            }
            2 => {
                analysis.body_lexer_config.as_mut().unwrap().strict_quoting =
                    !input.lexer_config().strict_quoting;
            }
            _ => analysis.body_lexer_config = None,
        }
        assert!(
            source_registry_words(source, &analysis, &segment).is_none(),
            "owner axis {axis}"
        );
    }
    assert!(source_registry_words(&(source.to_owned() + " "), &original, &segment).is_none());
    assert!(
        words
            .with_source_schema(
                &tcl_registry::model::ingress::resolve_environment("tcl8.6")
                    .default_context_registry(),
                |_| ()
            )
            .is_none()
    );
}

#[test]
fn jim_source_trace_filter_selects_only_member_stages() {
    for stage in [
        "constructor.image",
        "jim.alias.receipt",
        "jim.own_member.recipe",
        "jim.own_member.debt.UnknownEarlierMutation",
        "jim.own_member.supported",
        "jim.own_members.recipe",
    ] {
        assert!(jim_source_trace_stage_selected(stage, None));
        assert_eq!(
            jim_source_trace_stage_selected(stage, Some(std::ffi::OsStr::new("member-only-v1"))),
            stage.starts_with("jim.own_member.")
        );
    }
}

#[test]
fn jim_source_trace_filter_preserves_unset_mode_and_selects_only_alias_stages() {
    for stage in [
        "constructor.image",
        "constructor.input",
        "jim.alias.receipt",
        "analyser.proc.shape",
        "jim.alias.publication",
    ] {
        assert!(jim_source_trace_stage_selected(stage, None));
        assert_eq!(
            jim_source_trace_stage_selected(stage, Some(std::ffi::OsStr::new("alias-only-v1"))),
            stage.starts_with("jim.alias.")
        );
        for unknown in ["", "all", "alias-only-v2", "ALIAS-ONLY-V1"] {
            assert!(!jim_source_trace_stage_selected(
                stage,
                Some(std::ffi::OsStr::new(unknown))
            ));
        }
    }
}

fn assert_source_cut_captured_and_expanded_origins_refused(
    input: &crate::analyser::ResolvedAnalysisInput,
) {
    let captured = last_logical_schema(
        "interp alias {} pick {} list {CAPTURE $literal}; pick written",
        input,
    )
    .unwrap();
    assert!(matches!(
        captured.origins().get(1),
        Some(crate::registry_invocation::InvocationWordOrigin::BindingPrefix(0))
    ));
    assert!(!super::super::source_cut_declaration_origins(&captured));
    let expanded = last_logical_schema("list {*}{ONE TWO}", input).unwrap();
    assert!(matches!(
        expanded.origins().get(1),
        Some(crate::registry_invocation::InvocationWordOrigin::ExpandedElement { .. })
    ));
    assert!(!super::super::source_cut_declaration_origins(&expanded));
}

#[test]
fn source_cut_head_origins_keep_written_operands_and_refuse_captured_or_expanded() {
    let input = logical_schema_input(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
    );
    let written = last_logical_schema("proc helper {} {return HELPER}", &input).unwrap();
    assert_eq!(
        written.origins().first(),
        Some(&crate::registry_invocation::InvocationWordOrigin::Written(
            0
        ))
    );
    assert!(super::super::source_cut_declaration_origins(&written));
    let source = "proc helper {} {return HELPER}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let command = crate::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        analysis.body_lexer_config.unwrap(),
    )
    .pop()
    .unwrap();
    let selected = source_registry_words(source, &analysis, &command).unwrap();
    assert_eq!(
        selected.origins().first(),
        Some(&crate::registry_invocation::InvocationWordOrigin::ResolvedHead)
    );
    assert!(super::super::source_cut_declaration_origins(&selected));
    let realm = analysis.retained_command_realm().unwrap();
    let image = realm.original_source_image().unwrap();
    let config = analysis.body_lexer_config.unwrap();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        config,
        &command,
    );
    realm.stamp_original_tokens(&mut tokens);
    let context = analysis
        .resolved_input
        .as_ref()
        .unwrap()
        .borrowed_context_registry();
    let head = super::super::source_cut_resolved_head_correspondence(
        source,
        config,
        analysis.resolved_input.as_ref().unwrap(),
        realm,
        context,
        &tokens,
        &selected,
    )
    .unwrap();
    assert_eq!(head.left().origins().first(), written.origins().first());
    assert_eq!(head.right().origins().first(), selected.origins().first());
    assert!(head.matches_selected_source(&selected));
    assert!(!head.matches_selected_source(&written));
    assert!(
        super::super::source_cut_resolved_head_correspondence(
            &(source.to_owned() + " "),
            config,
            analysis.resolved_input.as_ref().unwrap(),
            realm,
            context,
            &tokens,
            &selected,
        )
        .is_none()
    );
    let mut foreign_config = config;
    foreign_config.strict_quoting = !foreign_config.strict_quoting;
    assert!(
        super::super::source_cut_resolved_head_correspondence(
            source,
            foreign_config,
            analysis.resolved_input.as_ref().unwrap(),
            realm,
            context,
            &tokens,
            &selected,
        )
        .is_none()
    );
    let foreign = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(
        super::super::source_cut_resolved_head_correspondence(
            source,
            config,
            analysis.resolved_input.as_ref().unwrap(),
            foreign.retained_command_realm().unwrap(),
            context,
            &tokens,
            &selected,
        )
        .is_none()
    );

    assert_source_cut_captured_and_expanded_origins_refused(&input);
}

#[test]
fn original_captured_word_projection_keeps_producer_geometry_and_origin() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let source = "interp alias {} pick {} list {CAPTURE $literal}; pick written";
    let logical = logical_schema_input(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
    );
    let native = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let native_segment = crate::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        native.body_lexer_config.unwrap(),
    )
    .pop()
    .unwrap();
    for words in [
        last_logical_schema(source, &logical).unwrap(),
        source_registry_words(source, &native, &native_segment).unwrap(),
    ] {
        assert_eq!(
            words.origins()[1],
            crate::registry_invocation::InvocationWordOrigin::BindingPrefix(0)
        );
        assert!(
            words.operands()[0].is_none(),
            "no written call operand is fabricated"
        );
        let word = words
            .original_argument_word(0)
            .expect("authentic capture allocation word");
        assert_eq!(&source[word.span().as_range()], "{CAPTURE $literal}");
        assert!(word.span().end() < words.head_source().unwrap().span().start());
        assert_eq!(
            words.arguments()[0].literal_bytes(),
            Some(b"CAPTURE $literal".as_slice())
        );
        assert_eq!(
            &source[words.original_argument_word(1).unwrap().span().as_range()],
            "written"
        );
        for axis in 0..4 {
            let mut changed = words.clone();
            match axis {
                0 => {
                    changed.origins[1] =
                        crate::registry_invocation::InvocationWordOrigin::BindingPrefix(1);
                }
                1 => changed.config.strict_quoting = !changed.config.strict_quoting,
                2 => changed.image = tcl_lexer::SourceImage::document("unrelated source"),
                _ => {
                    changed.origins[1] =
                        crate::registry_invocation::InvocationWordOrigin::Written(1);
                }
            }
            assert!(
                changed.original_argument_word(0).is_none(),
                "capture axis {axis}"
            );
        }
    }
    let words = last_logical_schema(source, &logical).unwrap();
    assert!(
        words
            .with_source_value_projection(
                logical.borrowed_context_registry(),
                &[(0, "CHANGED")],
                |_| ()
            )
            .is_none(),
        "capture cannot borrow a CU call operand"
    );
}

#[test]
fn original_logical_payloads_keep_unicode_and_literal_sigils_without_native_names() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let input = logical_schema_input(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
    );
    for (source, captured) in [
        ("list {é $literal}", false),
        ("rename list ::λ; ::λ {é $literal}", false),
        ("interp alias {} κ {} list {é $literal}; κ written", true),
        (
            "interp alias {} pick {} list {é $literal}; rename pick ::κ; ::κ written",
            true,
        ),
    ] {
        let words = last_logical_schema(source, &input).expect(source);
        assert_eq!(
            words.arguments()[0].literal_bytes(),
            Some("é $literal".as_bytes()),
            "{source}"
        );
        let word = words.original_argument_word(0).unwrap();
        assert_eq!(&source[word.span().as_range()], "{é $literal}");
        assert_eq!(words.operands()[0].is_none(), captured);
        assert!(words.original_argument_value_input(0).is_none());
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("Logical source schema")
        };
        assert!(advice.original_head().native_input().is_none());
        assert_eq!(advice.logical_source_input(), Some(&input));
    }
}

#[test]
fn selected_captured_values_keep_the_original_declaration_producer() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let source = "interp alias {} invoke {} apply {{a b} {return ok}}\ninvoke 1";
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let analysis = crate::analyser::Analyser::new().analyse(source, profile);
        let config = analysis.body_lexer_config.unwrap();
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let words =
            source_registry_words(source, &analysis, commands.last().unwrap()).expect(profile);
        assert!(matches!(
            words.origins().get(1),
            Some(crate::registry_invocation::InvocationWordOrigin::BindingPrefix(0))
        ));
        assert!(
            words.operands()[0].is_none(),
            "captured value has no written call operand"
        );
        let input = words.original_argument_value_input(0).unwrap_or_else(|| {
            panic!(
                "missing original captured producer for {profile}; source purpose {:?}",
                std::mem::discriminant(words.source())
            )
        });
        assert_eq!(input.bytes(), b"{a b} {return ok}");
        let parent = input
            .original_word_key()
            .expect("original alias-definition word");
        assert_eq!(&source[parent.span().as_range()], "{{a b} {return ok}}");
        assert!(parent.span().end() < words.head_source().unwrap().span().start());
        let producer = words
            .captured_argument_values()
            .expect("separate conditional source-value purpose");
        assert_ne!(producer.lineage(), []);
        assert_ne!(producer.obligations(), []);
        assert!(
            source_registry_words(
                &(source.to_owned() + " "),
                &analysis,
                commands.last().unwrap()
            )
            .is_none()
        );
    }
    for source in [
        "interp alias {} invoke {} apply $lambda\ninvoke 1",
        "interp alias {} invoke {} apply {{a b} {return ok}}\nproc invoke {x} {}\ninvoke 1",
        "interp alias {} invoke {} apply {{a b} {return ok}}\nrename invoke {}\ninvoke 1",
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analysis.body_lexer_config.unwrap(),
        );
        assert!(
            source_registry_words(source, &analysis, commands.last().unwrap())
                .is_none_or(|words| words.original_argument_value_input(0).is_none())
        );
    }
}

#[test]
fn original_scoped_words_keep_body_issuer_and_selected_subcommand_schema() {
    // naming.source.original-scoped-command-schema
    // docs/design/analysis/name-resolution-proofs/original-scoped-command-schema.md
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    // Source advice only: no report package installation or safe-interpreter
    // aliases are asserted by this candidate schema test.
    let source = "::report::defstyle st {} {\n    top {bogus}\n}\n";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let config = analysis.body_lexer_config.unwrap();
    let start = source.find("top").unwrap();
    let segment = crate::segmenter::segment_commands_with_offset_and_config(
        &source[start..start + 11],
        u32::try_from(start).unwrap(),
        config,
    )
    .remove(0);
    let words = source_registry_words(source, &analysis, &segment).expect("sealed scoped words");
    let OriginalRegistrySource::Scoped(scoped) = words.source() else {
        panic!("scoped issuer required");
    };
    assert_eq!(scoped.command().name, "top");
    assert_eq!(
        scoped.body().original_body().image(),
        &tcl_lexer::SourceImage::document(source)
    );
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    assert!(matches!(
        words
            .with_source_schema(&context, selected_subcommand_diagnostic)
            .flatten(),
        Some(tcl_registry::AuthoredSourceSubcommandDiagnostic::Unknown { .. })
    ));
    assert_eq!(
        words.operands()[0]
            .as_ref()
            .unwrap()
            .word()
            .unwrap()
            .bytes(),
        b"{bogus}"
    );
    assert!(
        source_registry_words(&source.replace("bogus", "other"), &analysis, &segment).is_none()
    );
    assert!(analysis.diagnostics.iter().any(|diagnostic| diagnostic.code
        == tcl_core_types::DiagCode::W001
        && diagnostic.registry_source().is_some()));
}

#[test]
fn original_source_static_values_keep_escapes_separate_from_frozen_runtime_values() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    // This verifies shared source-value capture; it asserts no executed
    // conversion, runtime handler identity or successful native invocation.
    // naming.source.original-static-escape-values
    // docs/design/analysis/name-resolution-proofs/original-source-static-escape-values.md
    // SourceEscapes173 independently caught bare/quoted/braced values on
    // all five C releases and Jim. Ordinary ASCII hex projections establish
    // these values; this test claims no native getter or NUL representation.
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let hex_boundary = if matches!(dialect, "tcl8.4" | "tcl8.5") {
            b"[".as_slice()
        } else {
            b"%b".as_slice()
        };
        for (source, expected) in [
            (r"format \x25b 5", hex_boundary),
            (r#"format "\x25b" 5"#, hex_boundary),
            (r"format {\x25b} 5", br"\x25b".as_slice()),
            (r"format \x25\u0062 5", b"%b".as_slice()),
            (r#"format "\x25\u0062" 5"#, b"%b".as_slice()),
            (r"format {\x25\u0062} 5", br"\x25\u0062".as_slice()),
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
            let config = analysis.body_lexer_config.unwrap();
            let segment =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let words = source_registry_words(source, &analysis, &segment)
                .unwrap_or_else(|| panic!("{dialect}: {source}"));
            assert_eq!(
                words.arguments()[0].literal_bytes(),
                Some(expected),
                "{dialect}: {source}"
            );
            assert_eq!(
                words.operands()[0]
                    .as_ref()
                    .unwrap()
                    .word()
                    .unwrap()
                    .bytes(),
                source.as_bytes().get(7..source.len() - 2).unwrap()
            );
        }
        let source = "set template %b; format $template 5";
        let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
        let config = analysis.body_lexer_config.unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .unwrap();
        let words = source_registry_words(source, &analysis, &segment)
            .unwrap_or_else(|| panic!("{dialect}: dynamic template"));
        assert!(
            words.arguments()[0].literal_bytes().is_none(),
            "{dialect}: source substitution cannot borrow a prior cell value"
        );
    }
}

#[test]
fn original_logical_source_schema_selects_its_positive_input_before_native_advice() {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let source = "expr $left + $right\n";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
    let analysis = crate::analyser::Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, profile.name);
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let words = source_registry_words(source, &analysis, &segment).unwrap();
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        panic!("Logical input must select its own source advice domain");
    };
    assert_eq!(advice.logical_source_input(), Some(&input));
    assert!(advice.original_head().logical_input().is_some());
    assert!(words.head_source().unwrap().input().is_none());
    assert!(!words.operands_preserve_source_lookup());
    assert!(
        words
            .roles()
            .unwrap()
            .iter()
            .all(|(_, role)| *role == tcl_registry::ArgRole::Expr)
    );
    let command = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(source),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        config,
    )
    .unwrap()
    .commands
    .remove(0);
    let realm = analysis.retained_command_realm().unwrap();
    assert_eq!(
        original_logical_registry_words_for_command(realm, &input, &command),
        Some(words)
    );
    let foreign = std::sync::Arc::new(
        context.with_command_store(context.commands().snapshot().shared_registry()),
    );
    let foreign_input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, foreign, config);
    assert!(original_logical_registry_words_for_command(realm, &foreign_input, &command).is_none());
    let mut changed_config = config;
    changed_config.strict_quoting = !changed_config.strict_quoting;
    let changed =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, changed_config);
    assert!(original_logical_registry_words_for_command(realm, &changed, &command).is_none());
    let native = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(
        original_logical_registry_words_for_command(
            native.retained_command_realm().unwrap(),
            native.resolved_input.as_ref().unwrap(),
            &command,
        )
        .is_none()
    );
}

#[test]
fn original_vendor_source_schema_keeps_comment_trivia_out_of_word_identity() {
    // naming.consumer.original-diagnostic-source-actions
    // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
    let source =
        "# proc html_encode is only a comment\nwhen HTTP_REQUEST {HTTP::respond 200 content ok}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "f5-irules");
    let config = analysis.body_lexer_config.unwrap();
    let command =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    assert!(command.preceding_comment.is_some());
    let offset = command.argv[0].span.start();
    let (metadata, projected) =
        selected_vendor_registry_words_at(source, &analysis, offset).unwrap();
    assert!(projected.preceding_comment.is_none());
    assert_ne!(command, projected);
    let words = source_registry_words(source, &analysis, &command).unwrap();
    assert!(matches!(words.source(), OriginalRegistrySource::Vendor(_)));
    assert_eq!(
        words.head_source().unwrap().word().unwrap(),
        metadata.shape().original_words().first().unwrap()
    );
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let bodies = words.source_script_bodies(&context);
    assert_eq!(bodies.len(), 1);
    assert!(bodies[0].matches_source(&tcl_lexer::SourceImage::document(source), config));
    assert_eq!(
        bodies[0].original_container(),
        words.operands()[1].as_ref().unwrap().word().unwrap()
    );

    let mut changed_trivia = command.clone();
    changed_trivia.preceding_comment = Some("proc report_only is presentation".to_owned());
    assert_eq!(
        source_registry_words(source, &analysis, &changed_trivia).unwrap(),
        words
    );
    let mut cut = command;
    let _ = cut.argv.pop();
    let _ = cut.word_fragments.pop();
    let _ = cut.texts.pop();
    let _ = cut.single_token_word.pop();
    assert!(
        source_registry_words(source, &analysis, &cut).is_none(),
        "an incomplete native vector cannot borrow the authentic complete invocation"
    );
}

#[test]
fn original_standalone_source_context_keeps_conditional_bodies_and_alias_order() {
    // Implementation contract: naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    // Implementation contract: naming.source.authored-command-transition-advice
    // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
    for version in tcl_dialect::TclVersion::ALL {
        // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
        let source = "set ::auto_index(pick) {source lib.tcl}; pick subject {default {puts before}}; interp alias {} pick {} switch; pick subject {default {puts after}}; interp alias {} pick {}; pick subject {default {puts deleted}}";
        let profile =
            tcl_registry::model::ingress::resolve_environment(version.dialect_profile_name())
                .analyser_profile();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        let syntax = OriginalSourceRegistryContext::capture(source, input);
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        assert!(syntax.words(&commands[1]).is_none());
        let words = syntax
            .words(&commands[3])
            .expect("actual authored alias must retain conditional source grammar");
        assert!(matches!(
            words.source(),
            OriginalRegistrySource::SourceTransitions(_)
        ));
        assert!(!words.operands_preserve_source_lookup());
        assert!(
            words
                .source_script_bodies(&context)
                .iter()
                .any(|body| source.get(body.content_span().as_range()) == Some("puts after"))
        );
        assert!(syntax.words(&commands[5]).is_none());
    }
}

#[test]
fn original_standalone_selected_alias_roles_keep_source_purpose() {
    // Implementation contract: naming.source.authored-registry-role-projection
    // docs/design/analysis/name-resolution-proofs/authored-registry-role-projection.md
    // Implementation contract: naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    for version in tcl_dialect::TclVersion::ALL {
        let source = "namespace eval ::A {}; interp alias {} ::pick {} ::switch; interp alias {} ::A::pick {} switch; ::pick subject {default {puts first}}; ::A::pick subject {default {puts second}}";
        let profile =
            tcl_registry::model::ingress::resolve_environment(version.dialect_profile_name())
                .analyser_profile();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        let syntax = OriginalSourceRegistryContext::capture(source, input);
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        for (ordinal, body) in [(3, "puts first"), (4, "puts second")] {
            let words = syntax
                .words(&commands[ordinal])
                .expect("genuine selected alias grammar");
            assert!(matches!(words.source(), OriginalRegistrySource::Selected));
            assert!(!words.operands_preserve_source_lookup());
            assert_eq!(
                words.head_source().unwrap().word().unwrap().span(),
                commands[ordinal].argv[0].span
            );
            assert!(
                words
                    .source_script_bodies(&context)
                    .iter()
                    .any(|region| source.get(region.content_span().as_range()) == Some(body))
            );
        }
        for changed in [
            "interp alias {} pick {} switch; interp alias {} pick {}; pick subject {default {puts stale}}",
            "interp alias {} pick {} switch; proc pick args {return other}; pick subject {default {puts stale}}",
        ] {
            let syntax =
                OriginalSourceRegistryContext::capture(changed, syntax.editing_input().clone());
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(changed, 0, config);
            assert!(syntax.words(commands.last().unwrap()).is_none());
        }
    }
}

#[test]
fn original_standalone_source_context_keeps_nested_unknown_selector_source_only() {
    // Implementation contract: naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    let source = "set result [switch $kind {alpha {# physical comment\nputs child}}]";
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
    let syntax = OriginalSourceRegistryContext::capture(source, input);
    let start = source.find("switch").unwrap();
    let end = source.rfind(']').unwrap();
    let command = crate::segmenter::segment_commands_with_offset_and_config(
        &source[start..end],
        u32::try_from(start).unwrap(),
        config,
    )
    .remove(0);
    let words = syntax
        .words(&command)
        .expect("physical case grammar survives an unknown selector value");
    assert!(matches!(
        words.arguments().first(),
        Some(crate::registry_invocation::EffectiveInvocationWord::Dynamic)
    ));
    let original_selector = words
        .operands()
        .first()
        .unwrap()
        .as_ref()
        .unwrap()
        .word()
        .unwrap();
    assert_eq!(
        original_selector.image(),
        &tcl_lexer::SourceImage::document(source)
    );
    assert_eq!(original_selector.config(), config);
    assert!(!words.operands_preserve_source_lookup());
    let bodies = words.source_script_bodies(&context);
    assert_eq!(bodies.len(), 1);
    assert_eq!(
        source.get(bodies[0].content_span().as_range()),
        Some("# physical comment\nputs child")
    );
    assert_eq!(
        syntax.source_image(),
        &tcl_lexer::SourceImage::document(source)
    );
    assert_eq!(syntax.editing_input().lexer_config(), config);
}

#[test]
fn original_deferred_source_schema_keeps_future_entry_and_current_lookup_distinct() {
    // naming.package.original-file-candidate-provenance
    // docs/design/analysis/name-resolution-proofs/original-package-file-candidate-provenance.md
    let source = "package ifneeded P 1.0 {source actual.tcl}";
    for version in tcl_dialect::TclVersion::ALL {
        let analysis =
            crate::analyser::Analyser::new().analyse(source, version.dialect_profile_name());
        let config = analysis.body_lexer_config.unwrap();
        let offset = u32::try_from(source.find("source actual.tcl").unwrap()).unwrap();
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            "source actual.tcl",
            offset,
            config,
        )
        .remove(0);
        let registry = analysis.resolved_registry().unwrap();
        assert!(selected_registry_words(source, &analysis, &command, registry).is_none());
        let words = source_registry_words(source, &analysis, &command)
            .unwrap_or_else(|| panic!("{}", version.dialect_profile_name()));
        let OriginalRegistrySource::Conditional(metadata) = words.source() else {
            panic!("future syntax cannot become selected document lookup");
        };
        assert!(metadata.obligations().contains(
            &crate::command_binding::OriginalCatalogueSourceObligation::UnknownFutureEntry,
        ));
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        assert_eq!(
            words.with_source_schema(&context, |schema| schema.semantics.analyser_hook),
            Some(Some(tcl_registry::hooks::AnalyserHookId::Source))
        );
        assert_eq!(
            words.operands()[0]
                .as_ref()
                .unwrap()
                .input()
                .unwrap()
                .bytes(),
            b"actual.tcl"
        );
        assert_eq!(
            words.head_source().unwrap().word().unwrap().image(),
            &tcl_lexer::SourceImage::document(source)
        );
        assert!(
            source_registry_words(&source.replace("actual", "other"), &analysis, &command)
                .is_none()
        );
        let mut changed = analysis.clone();
        let changed_config = changed.body_lexer_config.as_mut().unwrap();
        changed_config.strict_quoting = !changed_config.strict_quoting;
        assert!(source_registry_words(source, &changed, &command).is_none());
        let mut foreign = analysis.clone();
        let original = foreign.resolved_input.as_ref().unwrap();
        let profile = original.analyser_profile();
        let generation = original.context_registry();
        foreign.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            profile,
            original.unit_profile(),
            std::sync::Arc::new(
                generation.with_command_store(generation.commands().snapshot().shared_registry()),
            ),
            original.lexer_config(),
        ));
        assert!(source_registry_words(source, &foreign, &command).is_none());
    }
}

#[test]
fn original_deferred_source_schema_keeps_shadow_and_produced_script_boundaries() {
    // naming.package.original-file-candidate-provenance
    // docs/design/analysis/name-resolution-proofs/original-package-file-candidate-provenance.md
    for prefix in ["proc source args {return custom}\n", "rename source {}\n"] {
        let source = format!("{prefix}package ifneeded P 1.0 {{source actual.tcl}}");
        let analysis = crate::analyser::Analyser::new().analyse(&source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        let offset = u32::try_from(source.find("source actual.tcl").unwrap()).unwrap();
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            "source actual.tcl",
            offset,
            config,
        )
        .remove(0);
        assert!(source_registry_words(&source, &analysis, &command).is_none());
    }
    let source = "package ifneeded P 1.0 [list source [file join $dir produced.tcl]]";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let config = analysis.body_lexer_config.unwrap();
    let command =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let words = source_registry_words(source, &analysis, &command).unwrap();
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    assert_eq!(
        words.source_script_bodies(&context),
        [] as [crate::registry_invocation::source_scoped_body::OriginalSourceScriptBody; 0]
    );
}

#[test]
fn selected_source_roles_retain_procedure_grammar_under_the_actual_context() {
    // naming.source.authored-registry-role-projection
    // docs/design/analysis/name-resolution-proofs/authored-registry-role-projection.md
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "jim"] {
        let source = "proc p {a b} {return}";
        let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
        let config = analysis.body_lexer_config.unwrap();
        let command =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let registry = analysis.resolved_registry().unwrap();
        let words = selected_registry_words(source, &analysis, &command, registry).expect(dialect);
        assert_eq!(
            words.roles(),
            Some(
                [
                    (0, tcl_registry::ArgRole::Name),
                    (1, tcl_registry::ArgRole::ParamList),
                    (2, tcl_registry::ArgRole::Body),
                ]
                .as_slice()
            ),
            "{dialect}"
        );
        let parameters = words.operands()[1].as_ref().unwrap();
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &command,
        );
        let native = crate::registry_invocation::original_native_compiler_words(
            &tcl_lexer::SourceImage::document(source),
            tokens.words(),
            0,
            config,
        )
        .unwrap();
        assert_eq!(parameters.span(), command.argv[2].span);
        assert_eq!(parameters.word().unwrap(), &native[2]);
        assert!(
            words.operands()[1].as_ref().unwrap().input().is_some(),
            "{dialect}"
        );
        assert!(
            selected_registry_words("proc q {a b} {return}", &analysis, &command, registry)
                .is_none()
        );
        let foreign = tcl_registry::model::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl8.5").unwrap(),
        );
        assert!(words.with_source_schema(&foreign, |_| ()).is_none());
    }
}

#[test]
fn original_source_structure_is_shared_and_owner_checked() {
    // naming.core.original-command-source-schema
    // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
    let source = "set first 1\nset second 2\n";
    let mut analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    analysis.command_invocations.clear();
    analysis.global_scope.variables.clear();
    let config = analysis.body_lexer_config.unwrap();
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let words = source_registry_words(source, &analysis, &commands[1]).unwrap();
    let head = words.head_source().unwrap();
    assert_eq!(head.input().unwrap().bytes(), b"set");
    assert_eq!(head.word().unwrap().span(), commands[1].argv[0].span);
    assert_eq!(words.arguments().len(), 2);
    assert!(
        words
            .roles()
            .unwrap()
            .contains(&(0, tcl_registry::ArgRole::VarWrite))
    );
    assert_eq!(
        words.operands()[0].as_ref().unwrap().span(),
        commands[1].argv[1].span
    );
    assert!(words.matches_source(&tcl_lexer::SourceImage::document(source), config));
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let roles = words
        .with_source_schema(&context, selected_argument_roles)
        .unwrap();
    assert!(roles.1);
    assert!(roles.0.contains(&(0, tcl_registry::ArgRole::VarWrite)));
    assert!(!words.matches_source(
        &tcl_lexer::SourceImage::document(&source.replace("second", "absent")),
        config
    ));
    assert!(!words.matches_source(
        &tcl_lexer::SourceImage::document(source),
        tcl_lexer::LexerConfig {
            expand_syntax: !config.expand_syntax,
            ..config
        }
    ));
    let foreign = tcl_registry::model::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl9.0").unwrap(),
    );
    assert!(words.with_source_schema(&foreign, |_| ()).is_none());
}
#[test]
fn original_hosted_source_schema_keeps_controls_and_unknown_single_slots() {
    // naming.core.original-command-source-schema
    // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
    let source = "call -debug Lib::helper $payload\npool $target\n";
    let analysis = crate::analyser::Analyser::new().analyse(source, "f5-irules");
    let config = analysis.body_lexer_config.unwrap();
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let call = source_registry_words(source, &analysis, &commands[0]).unwrap();
    assert!(matches!(
        call.arguments()[0],
        crate::registry_invocation::EffectiveInvocationWord::ByteLiteral(_)
    ));
    assert!(matches!(
        call.arguments()[2],
        crate::registry_invocation::EffectiveInvocationWord::Dynamic
    ));
    assert_eq!(
        call.with_source_schema(&context, selected_rule_procedure_operand),
        Some(Some(1))
    );
    assert!(matches!(
        call.arguments()[0],
        crate::registry_invocation::EffectiveInvocationWord::ByteLiteral(_)
    ));
    let pool = source_registry_words(source, &analysis, &commands[1]).unwrap();
    assert!(matches!(
        pool.arguments()[0],
        crate::registry_invocation::EffectiveInvocationWord::Dynamic
    ));
    assert_eq!(
        pool.with_source_schema(&context, |schema| schema.words.arguments().exact_argv_len()),
        Some(Some(1))
    );
    assert!(pool.operands()[0].as_ref().unwrap().word().is_some());
}
#[test]
fn original_conditional_operand_keeps_its_static_word_and_list_lineage() {
    // naming.core.original-command-source-schema
    // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    let source =
        "set ::auto_index(unavailable) {source lib.tcl}\nunavailable\n::foreach {a b} {1 2} {}\n";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let commands = crate::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        analysis.body_lexer_config.unwrap(),
    );
    let words = source_registry_words(source, &analysis, &commands[2]).unwrap();
    assert!(matches!(
        words.source(),
        OriginalRegistrySource::Conditional(_)
    ));
    let operand = words.operands()[0].as_ref().unwrap();
    let input = operand.input().unwrap();
    assert_eq!(input.bytes(), b"a b");
    assert_eq!(
        input.original_word_key().unwrap().original_word(),
        operand.word().unwrap()
    );
    let children = input.original_list_elements_with_source_spans().unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].0.bytes(), b"a");
    assert!(
        children
            .iter()
            .all(|(child, span)| child.original_word_key().is_none() && span.is_some())
    );
}
