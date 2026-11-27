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

#[test]
fn complete_original_arguments_keep_foreign_formals_and_withdraw_partial_arithmetic() {
    for (body, actual, complete) in [
        ("expr {$a + $b}", "1 2", true),
        ("expr {$a + $b}", "abc 2", false),
        ("expr {$a / $b}", "1 0", false),
        ("expr {$a << $b}", "1 -1", false),
        ("expr {$a + [return EARLY]}", "1 2", true),
        ("expr {$a + [return -level 2 EARLY]}", "1 2", false),
        ("unknown_host $a; expr {$a + $b}", "1 2", false),
    ] {
        let source = format!(
            "proc add {{a b}} {{{body}}}; proc f {{}} {{set unused [add {actual}]; puts done}}"
        );
        let (bindings, _) = analyse(&source);
        let offset = u32::try_from(source.find("set unused").unwrap()).unwrap();
        let segment = tcl_source::segment::segment_commands_with_offset_and_config(
            &source[usize::try_from(offset).unwrap()..source.rfind('}').unwrap()],
            offset,
            tcl_lexer::LexerConfig::default(),
        )
        .remove(0);
        let image = tcl_lexer::SourceImage::document(&source);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &image.source_map(),
            tcl_lexer::LexerConfig::default(),
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        let binding = tokens.source_binding.as_ref().unwrap();
        assert_eq!(
            binding.original_arguments_complete_normally(&tokens),
            complete,
            "{source}"
        );
        if complete {
            assert!(
                tokens
                    .variable_accesses
                    .iter()
                    .any(|read| read.original_spelling == "$a")
            );
            let mut changed = tokens.clone();
            changed.word_exprs.clear();
            assert!(!binding.original_arguments_complete_normally(&changed));
            assert!(
                !super::super::SourceInvocationBinding::default()
                    .original_arguments_complete_normally(&tokens)
            );
        }
    }
}

#[test]
fn whole_expression_result_replaces_operand_result() {
    let source = "set result [expr {1 + 2}]; set observed $result";
    let (bindings, _) = analyse(source);
    let registry = tcl_registry::CommandRegistry::build_default();
    let after = bindings.invocation_at_source(
        "set",
        u32::try_from(source.find("set observed").unwrap()).unwrap(),
    );
    assert_eq!(
        after.variable_context.literal_value("result", &registry),
        Some("3")
    );

    let source = "expr {1 + [return DONE]}; set unreachable 1";
    let (bindings, _) = analyse(source);
    assert!(
        bindings
            .invocation_at_source(
                "set",
                u32::try_from(source.find("set unreachable").unwrap()).unwrap(),
            )
            .invocation_site()
            .is_none()
    );
}

#[test]
fn original_expression_read_capture_restores_nested_owner_and_rejects_foreign_frame() {
    let source = "set a [expr {1}]; expr {$a + 1}";
    let (mut bindings, script) = analyse(source);
    let site = u32::try_from(source.rfind("expr").unwrap()).unwrap();
    let proof = bindings
        .expression_preparations_for_script(&script)
        .into_iter()
        .find(|proof| proof.invocation.offset == site)
        .unwrap();
    let read = bindings
        .variable_accesses_during_expression_invocation(site)
        .into_iter()
        .find(|read| read.original_spelling == "$a")
        .unwrap();
    let frame = bindings.invocation_at_source("expr", site).variable_frame;
    let outer = bindings.begin_expression_read_capture(&proof, &frame);
    bindings.capture_current_expression_read(&script.origin, &frame, &read);
    let mut nested = proof.clone();
    nested.invocation.offset += 1;
    let inner = bindings.begin_expression_read_capture(&nested, &frame);
    bindings.capture_current_expression_read(&script.origin, &frame, &read);
    assert_eq!(
        bindings
            .finish_expression_read_capture(inner, &nested)
            .len(),
        0
    );
    assert_eq!(
        bindings.finish_expression_read_capture(outer, &proof),
        vec![read.clone()]
    );
    assert!(bindings.current_expression_reads.0.is_empty());

    let capture = bindings.begin_expression_read_capture(&proof, &frame);
    bindings.capture_current_expression_read(
        &script.origin,
        &crate::var_resolve::VariableExecutionFrame::Unknown,
        &read,
    );
    assert_eq!(
        bindings
            .finish_expression_read_capture(capture, &proof)
            .len(),
        0
    );
    assert!(bindings.current_expression_reads.0.is_empty());
    // The retained lexical inventory survives this transient refusal.
    assert!(
        bindings
            .variable_accesses_during_expression_invocation(site)
            .iter()
            .any(|retained| retained.source == read.source)
    );
}

#[test]
fn numeric_only_preservation_respects_the_selected_comparison_cache_protocol() {
    for profile in ["tcl8.4", "tcl8.6", "tcl9.1", "jim"] {
        let supplied = tcl_registry::model::ingress::static_context_for(profile);
        let registry = supplied.commands();
        let selected = registry.profile().unwrap();
        let policy = crate::tcl_expr_eval::FoldPolicy::for_retained_entry(
            registry,
            Some(tcl_registry::InvocationDialect::of_profile(selected)),
            &tcl_lexer::LexerConfig::from_grammar(selected.grammar),
        );
        for (expression, allowed) in [
            ("$x + 1", true),
            ("$x << 1", true),
            ("$x == $peer", profile != "jim"),
            ("$x < $peer", profile != "jim"),
            ("$x || $peer", profile != "jim"),
        ] {
            let tree = tcl_syntax::expr::parser::parse_expr_with_syntax_context(
                expression,
                &policy.preparation_context().unwrap(),
            );
            assert_eq!(
                expression_has_only_numeric_conversions(&tree, policy),
                allowed,
                "{profile}: {expression}"
            );
        }
    }
}

#[test]
fn a_control_expression_retains_its_actual_original_operand_mapping() {
    let source = "set a [expr {7+0}]; if {$a == \"hello\"} {set result yes}";
    let (bindings, script) = analyse(source);
    let invocation = u32::try_from(source.find("if ").unwrap()).unwrap();
    let proofs = bindings.expression_preparations_for_script(&script);
    let proof = proofs
        .iter()
        .find(|proof| proof.invocation.offset == invocation)
        .unwrap();
    let (origin, base) = proof.original_expression_mapping().unwrap();
    assert_eq!(origin, &script.origin);
    assert_eq!(
        &source[usize::try_from(base).unwrap()..][..proof.source.text.len()],
        proof.source.text.try_text().unwrap()
    );
    let crate::expr_ast::ExprNode::Binary { left, .. } = proof.witness.tree() else {
        panic!("comparison preparation");
    };
    let (variable_origin, variable_site) = proof.original_variable_source(left).unwrap();
    assert_eq!(variable_origin, &script.origin);
    assert_eq!(&source[variable_site.span.as_range()], "$a");
    let reads = bindings.variable_accesses_during_expression_invocation(invocation);
    assert!(reads.iter().any(|read| read.source == variable_site));
    let mut foreign_parent = proof.clone();
    foreign_parent.invocation.source = Arc::new(super::super::SourceOriginId::authored(
        &Arc::from("foreign"),
    ));
    assert!(foreign_parent.original_variable_source(left).is_none());
    let mut missing = proof.clone();
    missing.executed_expression = None;
    if missing.source.mapping == super::super::ExecutedScriptMapping::Materialised {
        assert!(missing.original_variable_source(left).is_none());
    }
}

#[test]
fn a_literal_sharing_footprint_includes_native_canonical_pool_bytes() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let policy = crate::tcl_expr_eval::FoldPolicy::for_retained_entry(
        registry,
        Some(tcl_registry::InvocationDialect::of_profile(profile)),
        &tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );
    for (text, canonical) in [("false", "0"), ("0x10", "16")] {
        let tree = crate::expr_ast::ExprNode::Literal {
            text: text.to_owned(),
            start: 0,
            end: 0,
        };
        let values = expression_coercion_values(
            &tree,
            &crate::var_resolve::ResolveContext::default(),
            registry,
            policy,
            ExpressionCoercionPhase::CompilerPreparation,
        )
        .unwrap();
        assert!(values.iter().any(|value| value == text));
        assert!(values.iter().any(|value| value == canonical));
    }
}

#[test]
fn runtime_variable_coercion_follows_the_original_expression_read() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let profile = registry.profile().unwrap();
    let source = "proc f {} {set hay [dict create a b]; expr {\"a\" in $hay}; set after $hay}";
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        registry,
        super::super::SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let reads = bindings.variable_accesses_during_expression_invocation(
        u32::try_from(source.find("expr").unwrap()).unwrap(),
    );
    let read = reads
        .iter()
        .find(|read| read.original_spelling == "$hay")
        .unwrap();
    for context in read.context_alternatives() {
        let place = crate::var_resolve::resolve_substitution_access(
            &read.original_spelling,
            context,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert!(context.read_produces_value(&place, registry));
        assert_eq!(
            context.contents_representation_at(&place),
            tcl_syntax::value::ValueRepresentation::Dict
        );
    }
    let after = bindings.invocation_at_source(
        "set",
        u32::try_from(source.find("set after").unwrap()).unwrap(),
    );
    let place = crate::var_resolve::resolve_literal_access(
        "hay",
        &after.variable_context,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    assert_eq!(
        after.variable_context.literal_contents_at(&place, registry),
        Some("a b")
    );
    assert_eq!(
        after.variable_context.contents_representation_at(&place),
        tcl_syntax::value::ValueRepresentation::Unknown
    );
}

#[test]
fn compiler_literal_footprints_do_not_borrow_runtime_variable_bytes() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let profile = registry.profile().unwrap();
    let policy = crate::tcl_expr_eval::FoldPolicy::for_retained_entry(
        registry,
        Some(tcl_registry::InvocationDialect::of_profile(profile)),
        &tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );
    let tree = crate::expr_parser::parse_expr("$unbound + 1", Some("tcl8.6"));
    let variables = crate::var_resolve::ResolveContext::default();
    let preparation = expression_coercion_values(
        &tree,
        &variables,
        registry,
        policy,
        ExpressionCoercionPhase::CompilerPreparation,
    )
    .unwrap();
    assert!(preparation.iter().any(|value| value == "1"));
    assert!(
        expression_coercion_values(
            &tree,
            &variables,
            registry,
            policy,
            ExpressionCoercionPhase::RuntimeOperands
        )
        .is_none()
    );
}

fn analyse(source: &str) -> (SourceCommandBindings, ExecutedScriptSource) {
    let bindings = SourceCommandBindings::analyse(
        source,
        tcl_lexer::LexerConfig::default(),
        &tcl_registry::CommandRegistry::build_default(),
    );
    let script =
        ExecutedScriptSource::contiguous(Arc::clone(bindings.source_origin().unwrap()), source, 0)
            .unwrap();
    (bindings, script)
}

#[test]
fn c84_script_preparation_retains_exact_compiler_visits() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let source = "set L {a b c}; lindex $L [expr {[llength $L] - 2}]";
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        &registry,
        super::super::SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let script =
        ExecutedScriptSource::contiguous(Arc::clone(bindings.source_origin().unwrap()), source, 0)
            .unwrap();
    let proofs = bindings.expression_preparations_for_script(&script);
    let [proof] = proofs.as_slice() else {
        panic!("{proofs:#?}");
    };
    assert_eq!(proof.witness.compiled_scripts().len(), 1);
    assert!(proof.has_closed_script_compilation());
    assert!(
        proof
            .script_compilation_dependencies()
            .iter()
            .any(|dependency| { dependency.target.command == "::llength" })
    );
    let mut missing = proof.clone();
    missing.script_compilation = None;
    assert!(!missing.has_closed_script_compilation());
}

#[test]
fn reached_literal_expression_retains_its_prepared_tree() {
    let (bindings, script) = analyse("expr {1 + 2}");
    let proofs = bindings.expression_preparations_for_script(&script);
    assert_eq!(proofs.len(), 1);
    assert_eq!(proofs[0].witness.source(), "1 + 2");
    assert_eq!(proofs[0].source.base(), 6);
    assert_eq!(proofs[0].source.origin, script.origin);
}

fn assert_replaced_expression_target(
    binding: &super::super::SourceInvocationBinding,
    source: &str,
) {
    let target = binding
        .proved_execution_target()
        .expect("actual after-load replacement");
    eprintln!(
        "G4_REPLACED_TARGET kind={:?} registry_backed={} allocation={} authored_match={}",
        target.kind,
        target.registry_backed,
        target.implementation_allocation.is_some(),
        target.matches_authored_implementation(
            source,
            u32::try_from(source.rfind("proc expr").unwrap()).unwrap()
        ),
    );
    assert_eq!(
        binding.runtime_reachability(),
        super::super::SourceRuntimeReachability::Reached
    );
    assert_eq!(target.command, "::expr");
    assert_eq!(target.kind, super::super::BindingKind::Proc);
    assert!(!target.registry_backed);
    assert!(target.matches_authored_implementation(
        source,
        u32::try_from(source.rfind("proc expr").unwrap()).unwrap()
    ));
}

fn assert_nested_owner_refusals(
    binding: &super::super::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    parser: &tcl_syntax::expr::parser::ExprParseContext,
    profile_name: &str,
) {
    let mut wrong_frame = binding.clone();
    wrong_frame.variable_frame = crate::var_resolve::VariableExecutionFrame::Global;
    eprintln!(
        "G4_WRONG_FRAME profile={profile_name} original={:?} mutated={:?} equal={} attached_original={} attached_mutated={} original_normalisation_frame={:?} mutated_normalisation_frame={:?}",
        binding.variable_frame,
        wrong_frame.variable_frame,
        binding == &wrong_frame,
        tokens.source_binding.as_ref() == Some(binding),
        tokens.source_binding.as_ref() == Some(&wrong_frame),
        binding
            .normalisation_expression(registry, tokens)
            .map(|proof| proof.frame),
        wrong_frame
            .normalisation_expression(registry, tokens)
            .map(|proof| proof.frame),
    );
    binding.trace_g4_normal_frame_facts("nested-original", tokens, registry);
    wrong_frame.trace_g4_normal_frame_facts("nested-mutated", tokens, registry);
    assert!(!wrong_frame.nested_expression_normalisation(registry, tokens, parser, None));
    let mut foreign = tokens.clone();
    foreign.word_exprs.clear();
    assert!(!binding.nested_expression_normalisation(registry, &foreign, parser, None));
}

#[test]
fn scoped_nested_normalisation_keeps_original_world_frame_and_source() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    let expression = "expr {[expr {$x * 2}]}";
    for profile_name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let owner = tcl_registry::model::ingress::static_context_for(profile_name);
        let registry = owner.commands();
        let profile = registry.profile().unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let parser =
            tcl_registry::InvocationDialect::of_profile(profile).expression_parse_context(None);
        for (suffix, allowed) in [
            ("", true),
            (
                "; set ::auto_index(unknown_future_entry) {source lib.tcl}; unknown_future_entry",
                false,
            ),
            ("; trace add execution expr enter callback", false),
            (
                "; rename expr saved; proc expr args {return changed}",
                false,
            ),
        ] {
            let source = format!("proc f {{x}} {{{expression}}}{suffix}");
            let offset = u32::try_from(source.find(expression).unwrap()).unwrap();
            let bindings = SourceCommandBindings::analyse_with_options(
                &source,
                config,
                registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..Default::default()
                },
            );
            let command = tcl_source::segment::segment_commands_with_offset_and_config(
                expression, offset, config,
            )
            .remove(0);
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceImage::document(&source).source_map(),
                config,
                &command,
            );
            bindings.stamp_original_tokens(&mut tokens);
            crate::command_binding::trace_original_g4_owner("scoped-nested", &bindings, &tokens);
            let binding = tokens.source_binding.as_ref().unwrap();
            if allowed {
                assert_eq!(
                    binding
                        .proved_execution_target()
                        .map(|target| target.command.as_str()),
                    Some("::expr")
                );
                assert_eq!(
                    binding.runtime_reachability(),
                    super::super::SourceRuntimeReachability::Reached
                );
                assert!(binding.compiler_lookup_state.is_none());
                assert!(bindings.final_state.baseline.native_entry.is_none());
            } else if suffix == "; rename expr saved; proc expr args {return changed}" {
                assert_replaced_expression_target(binding, &source);
            } else {
                assert!(binding.proved_execution_target().is_none());
            }
            assert_eq!(
                binding.nested_expression_normalisation(registry, &tokens, &parser, None),
                allowed,
                "{profile_name}: {source}"
            );
            if allowed {
                assert_nested_owner_refusals(binding, &tokens, registry, &parser, profile_name);
            }
        }
    }
}

#[test]
fn selected_nested_normalisation_projects_actual_operand_and_command_source() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    let expression = "expr {  [expr {$x * 2}]  }";
    for profile_name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let owner = tcl_registry::model::ingress::static_context_for(profile_name);
        let registry = owner.commands();
        let profile = registry.profile().unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let parser =
            tcl_registry::InvocationDialect::of_profile(profile).expression_parse_context(None);
        for (suffix, allowed) in [
            ("", true),
            (
                "; set ::auto_index(unknown_future_entry) {source lib.tcl}; unknown_future_entry",
                false,
            ),
            ("; trace add execution expr enter callback", false),
            (
                "; rename expr saved; proc expr args {return changed}",
                false,
            ),
        ] {
            let source = format!("proc f {{x}} {{{expression}}}{suffix}");
            let offset = u32::try_from(source.find(expression).unwrap()).unwrap();
            let bindings = SourceCommandBindings::analyse_with_options(
                &source,
                config,
                registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..Default::default()
                },
            );
            let command = tcl_source::segment::segment_commands_with_offset_and_config(
                expression, offset, config,
            )
            .remove(0);
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceImage::document(&source).source_map(),
                config,
                &command,
            );
            bindings.stamp_original_tokens(&mut tokens);
            crate::command_binding::trace_original_g4_owner("selected-nested", &bindings, &tokens);
            let binding = tokens.source_binding.as_ref().unwrap();
            if allowed {
                assert_eq!(
                    binding
                        .proved_execution_target()
                        .map(|target| target.command.as_str()),
                    Some("::expr")
                );
                assert_eq!(
                    binding.runtime_reachability(),
                    super::super::SourceRuntimeReachability::Reached
                );
                assert!(binding.compiler_lookup_state.is_none());
                assert!(bindings.final_state.baseline.native_entry.is_none());
            } else if suffix == "; rename expr saved; proc expr args {return changed}" {
                assert_replaced_expression_target(binding, &source);
            } else {
                assert!(binding.proved_execution_target().is_none());
            }
            assert_eq!(
                binding.nested_expression_normalisation(registry, &tokens, &parser, None),
                allowed,
                "{profile_name}: {source}"
            );
            let projection =
                binding.nested_expression_normalisation_source(registry, &tokens, &parser, None);
            if allowed {
                assert_eq!(
                    projection,
                    Some(("$x * 2".to_owned(), "[expr {$x * 2}]".to_owned()))
                );
                assert_nested_owner_refusals(binding, &tokens, registry, &parser, profile_name);
            } else {
                assert!(projection.is_none());
            }
        }
    }
}

#[test]
fn declaration_preparation_failure_does_not_poison_actual_preparation() {
    let registry = tcl_registry::CommandRegistry::build_default();
    for text in ["1 + 2", "1 +"] {
        let authored = format!("expr {{{text}}}");
        let (mut bindings, _) = analyse(&authored);
        let origin = Arc::clone(bindings.source_origin().unwrap());
        let source = ExecutedScriptSource::contiguous(Arc::clone(&origin), text, 6).unwrap();
        let invocation = CommandAllocationSite {
            source: origin,
            offset: 0,
        };
        let actual = bindings.final_state.as_ref().clone();
        let frame = actual.variable_frame.clone();
        let namespace = frame.namespace_identity().cloned();
        let context = SourceExecutionContext {
            original_logical_body: None,
            control: crate::command_binding::source_execution_control::SourceExecutionControl::new(
                tcl_registry::native_compilation::NativeCompilationContext::default(),
                tcl_lexer::SourceSegmentationPurpose::NestedScript,
                crate::command_binding::SourceBodyEntryPurpose::Selected,
                tcl_dialect::model::InvocationRealm::default(),
            ),
            compilation_snapshot: None,
            selected_compilation: None,
            original_variable_compilation: None,
            namespace: "::",
            namespace_key: namespace.as_ref(),
            config: tcl_lexer::LexerConfig::default(),
            registry: &registry,
            depth: 0,
            frame: &frame,
            invocation_offset: 0,
            variable_read_owner: None,
            original_written_projection: None,
            written_arguments: None,
            written_values: None,
            written_name_values: None,
            written_representations: None,
            written_objects: None,
            written_method_prefixes: None,
            written_variable_reads: None,
            expression_source: None,
        };
        bindings.expression_preparations.clear();
        bindings.declaration_preview_depth = 1;
        let mut preview = actual.clone();
        if text == "1 + 2" {
            Arc::make_mut(&mut preview.baseline).dialect = None;
        }
        let (preparation, _) = prepare_reached_expression(&source, &preview, context);
        assert!(matches!(
            preparation,
            ExpressionPreparationProof::Unknown | ExpressionPreparationProof::Rejected(_)
        ));
        bindings.walk_prepared_expression(
            invocation.clone(),
            source.clone(),
            &mut preview,
            context,
        );
        assert!(!bindings.expression_preparations.contains_key(&invocation));
        bindings.declaration_preview_depth = 0;
        let mut entered = actual;
        bindings.walk_prepared_expression(
            invocation.clone(),
            source.clone(),
            &mut entered,
            context,
        );
        if text == "1 + 2" {
            assert_eq!(
                bindings.expression_preparations[&invocation]
                    .as_ref()
                    .unwrap()[0]
                    .witness
                    .source(),
                text,
            );
            Arc::make_mut(&mut entered.baseline).dialect = None;
            bindings.walk_prepared_expression(invocation.clone(), source, &mut entered, context);
        }
        assert_eq!(bindings.expression_preparations[&invocation], None);
    }
}

#[test]
fn evaluated_expression_keeps_derived_origin_and_actual_bytes() {
    let (bindings, script) = analyse("set e {1 + 2}; expr $e");
    let proofs = bindings.expression_preparations_for_script(&script);
    assert_eq!(proofs.len(), 1);
    assert_eq!(proofs[0].witness.source(), "1 + 2");
    assert_ne!(proofs[0].source.origin, script.origin);
    assert_eq!(proofs[0].source.base(), 0);
    assert_eq!(proofs[0].invocation.offset, 15);
}

#[test]
fn rejected_preparation_does_not_record_success_or_later_dispatch() {
    let (bindings, script) = analyse("expr {1 +}; set later 1");
    assert_eq!(
        bindings.expression_preparations_for_script(&script),
        [] as [SourceExpressionPreparation; 0]
    );
    assert!(
        bindings
            .invocation_at("set", "::", 12)
            .proved_execution_target()
            .is_none()
    );
}
#[test]
fn conditional_expression_categories_require_closed_lookup_but_keep_may_tree() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let source = "expr {3.5}";
    let bindings = super::super::SourceCommandBindings::analyse_with_options(
        source,
        config,
        registry,
        super::super::SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            ..Default::default()
        },
    );
    let image = tcl_lexer::SourceImage::document(source);
    let segment =
        tcl_source::segment::segment_commands_image_with_offset_and_config(&image, 0, config)
            .unwrap()
            .remove(0);
    let mut tokens =
        crate::ir::CommandTokens::from_segmented(&image.source_map(), config, &segment);
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    let mut descriptor = binding
        .conditional_expression_evaluation(registry, &tokens)
        .unwrap();
    assert!(descriptor.semantic_lookup_closed());
    assert_eq!(
        descriptor.normal_result_representation(|_| None, |_| None),
        Some(tcl_registry::TclType::Double)
    );
    let tree = descriptor.tree().clone();
    descriptor.lookup_closed = false;
    assert!(!descriptor.semantic_lookup_closed());
    assert_eq!(
        descriptor.normal_result_representation(|_| None, |_| None),
        None
    );
    assert_eq!(
        descriptor.tree(),
        &tree,
        "diagnostic May topology is independent of category authority"
    );
    assert!(binding.original_normal_result(&tokens).is_none());
}

#[test]
fn evaluated_expression_source_cannot_replace_original_advisory_owner() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let source = "expr 3.5";
    let bindings = super::super::SourceCommandBindings::analyse_with_options(
        source,
        config,
        registry,
        super::super::SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            ..Default::default()
        },
    );
    let image = tcl_lexer::SourceImage::document(source);
    let segment =
        tcl_source::segment::segment_commands_image_with_offset_and_config(&image, 0, config)
            .unwrap()
            .remove(0);
    let mut tokens =
        crate::ir::CommandTokens::from_segmented(&image.source_map(), config, &segment);
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    let observations = binding.conditional_expression_evaluations.as_ref().unwrap();
    assert!(observations.iter().all(|observation| {
        observation.issuer == ExpressionEvaluationIssuer::EvaluatedSource
            && matches!(
                observation.source.mapping,
                super::super::ExecutedScriptMapping::Materialised
            )
    }));
    let original = binding
        .conditional_expression_evaluation(registry, &tokens)
        .unwrap();
    assert_eq!(original.issuer, ExpressionEvaluationIssuer::OriginalSource);
    assert_eq!(
        original.source.origin,
        bindings.source_origin().unwrap().clone()
    );
    assert_eq!(original.source.base(), 5);
    assert!(
        bindings
            .expression_preparations
            .values()
            .all(|proofs| proofs.as_ref().is_none_or(Vec::is_empty))
    );
    assert!(binding.original_normal_result(&tokens).is_none());

    let mut conflicting = original;
    conflicting.pool =
        tcl_registry::conditional_expression::ConditionalExpressionPoolState::Unknown;
    let mut changed = tokens.clone();
    changed
        .source_binding
        .as_mut()
        .unwrap()
        .conditional_expression_evaluations = Some(Arc::from(vec![conflicting]));
    assert!(
        changed
            .source_binding
            .as_ref()
            .unwrap()
            .conditional_expression_evaluation(registry, &changed)
            .is_none()
    );
}

#[test]
fn conditional_bare_expression_requires_exact_original_payload_and_handler() {
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        for (source, allowed) in [
            ("expr 3.5", true),
            ("expr abs(-3)", true),
            ("expr 3 + 5", false),
            ("expr 3\\.5", false),
            ("set input 3.5; expr $input", false),
            ("unknown; expr 3.5", false),
            (
                "rename expr original; proc expr args {return custom}; expr 3.5",
                false,
            ),
        ] {
            let bindings = super::super::SourceCommandBindings::analyse_with_options(
                source,
                config,
                registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    ..Default::default()
                },
            );
            let image = tcl_lexer::SourceImage::document(source);
            let segment = tcl_source::segment::segment_commands_image_with_offset_and_config(
                &image, 0, config,
            )
            .unwrap()
            .pop()
            .unwrap();
            let mut tokens =
                crate::ir::CommandTokens::from_segmented(&image.source_map(), config, &segment);
            bindings.stamp_original_tokens(&mut tokens);
            let binding = tokens.source_binding.as_ref().unwrap();
            let receipt = binding.conditional_expression_evaluation(registry, &tokens);
            assert_eq!(receipt.is_some(), allowed, "{name}: {source}");
            if let Some(receipt) = receipt {
                assert_eq!(receipt.source().try_text().unwrap(), tokens.argv_texts[1]);
                let mut changed = tokens.clone();
                let source = changed.word_exprs[1].source().clone();
                changed.word_exprs[1] = tcl_source::words::WordExpr::Literal {
                    text: "99".to_owned(),
                    source,
                };
                assert!(
                    binding
                        .conditional_expression_evaluation(registry, &changed)
                        .is_none()
                );
                assert!(binding.original_normal_result(&tokens).is_none());
            }
        }
    }
}
