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

fn original(
    source: &str,
    command: &str,
    offset: usize,
) -> (SourceCommandBindings, crate::ir::CommandTokens) {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        config,
        registry,
        SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        },
    );
    let source_part = command;
    let segments = crate::segmenter::segment_commands_with_offset_and_config(
        source_part,
        u32::try_from(offset).unwrap(),
        config,
    );
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        &segments[0],
    );
    bindings.stamp_original_tokens(&mut tokens);
    (bindings, tokens)
}

fn inspect_native_root_model(
    source: &str,
    inspect: impl FnOnce(&SourceCommandBindings, &tcl_registry::CommandRegistry),
) {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let profile = registry.profile().unwrap();
    let (_owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        registry,
        SourceAnalysisOptions {
            native_entry: Some(&entry),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        },
    );
    inspect(&bindings, registry);
}

#[test]
fn completed_command_values_preserve_short_circuit_mutation_order() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/data/w210-short-circuit.json")).unwrap();
    for case in fixtures["mutations"].as_array().unwrap() {
        let Some(expected) = case["expected_model_rhs_dispatch"].as_bool() else {
            continue;
        };
        let source = case["source"].as_str().unwrap();
        let rhs = u32::try_from(source.find("[set x 9]").unwrap() + 1).unwrap();
        inspect_native_root_model(source, |bindings, _| {
            assert_eq!(
                bindings.has_site(rhs),
                expected,
                "{}: original model RHS dispatch at {rhs}",
                case["id"]
            );
        });
    }
}

#[test]
fn unsupported_completions_do_not_donate_a_child_or_later_store() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/data/w210-short-circuit.json")).unwrap();
    for case in fixtures["mutations"].as_array().unwrap() {
        if case["native_status"] != 0 || case["expected_model_rhs_dispatch"].is_boolean() {
            continue;
        }
        let source = case["source"].as_str().unwrap();
        inspect_native_root_model(source, |bindings, registry| {
            let value = bindings
                .final_state
                .source_variables
                .substitution_literal_value("$x", registry);
            assert!(
                value.is_none() || value == case["native_result"].as_str(),
                "{}: model value {value:?} disagrees with the native result",
                case["id"]
            );
        });
    }
}

#[test]
fn completed_source_values_keep_expression_and_admission_owners() {
    use crate::analyser::Analyser;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};

    let cases = [
        ("set x 7; if {[set x 0] && [set x 9]} {}; set x", "0"),
        ("set x 7; if {![set x 0] || [set x 9]} {}; set x", "0"),
        ("expr {[set x 1] && [set x 9]}", "1"),
        ("expr {[set x 0] || [set x 9]}", "1"),
        ("expr {![set x 0] || [set x 9]}", "1"),
        ("expr {!![set x 0] && [set x 9]}", "0"),
    ];
    for (source, expected) in cases {
        for dialect in ["tcl8.4", "tcl9.0"] {
            let mut analyser = Analyser::new();
            let _ = analyser.analyse(source, dialect);
            let input = analyser.resolved_analysis_input();
            let settings = analyser.source_compilation_settings();
            let registry = input.borrowed_context_registry().commands();
            let unit = CompilationUnit::build_with_analysis_input_and_source_settings(
                source,
                UnitBuildOptions {
                    registry,
                    defer_top_level: false,
                    config: input.lexer_config(),
                    dialect: Some(input.unit_profile()),
                    external_call_sites: None,
                    declared_commands: None,
                },
                None,
                &input,
                &settings,
            );
            let body = &unit.ir_module.top_level;
            let tokens = body
                .retained_source_tokens_for_statement(body.statements.last().unwrap())
                .unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            let metadata = unit
                .ir_module
                .source_entry
                .metadata_context
                .metadata_context(registry)
                .and_then(crate::registry_invocation::InvocationMetadataSelection::context);
            let value = binding
                .original_normal_result(tokens)
                .map(SourceNormalResult::text);
            assert_eq!(
                unit.ir_module
                    .source_entry
                    .execution_name_policy
                    .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                    .map(tcl_syntax::naming::NamePolicyProtocol::authority),
                Some(tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation)
            );
            assert!(metadata.is_some());
            assert!(unit.ir_module.source_entry.native_entry.is_none());
            assert!(unit.ir_module.source_entry.source_point_entry.is_none());
            assert!(binding.admitted_inline_invocation().is_none());
            assert!(binding.admitted_named_invocation().is_none());
            println!(
                "SHORT_CIRCUIT_GATES {}",
                serde_json::json!({
                    "source": source, "dialect": dialect, "result": value,
                    "complete_normally": binding.original_invocation_completes_normally(tokens),
                    "metadata_available": metadata.is_some(),
                    "compiler_inline_admitted": binding.admitted_inline_invocation().is_some(),
                    "compiler_named_admitted": binding.admitted_named_invocation().is_some(),
                    "source_authority": unit.ir_module.source_entry.execution_name_policy.and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe).map(|policy| format!("{:?}", policy.authority())),
                    "native_entry": unit.ir_module.source_entry.native_entry.is_some(),
                    "source_point_entry": unit.ir_module.source_entry.source_point_entry.is_some(),
                    "compilation_mode": format!("{:?}", unit.ir_module.source_entry.native_compilation.mode),
                })
            );
            assert!(
                value.is_none() || value == Some(expected),
                "{dialect}: {source}: {value:?}"
            );
            if source.starts_with("expr ") {
                // The existing expression owner refuses these command-bearing
                // roots; a child's completion cannot replace its own producer.
                assert!(value.is_none());
                assert!(!binding.original_invocation_completes_normally(tokens));
            } else {
                assert_eq!(value, Some(expected));
                assert!(binding.original_invocation_completes_normally(tokens));
            }
        }
    }
}

#[test]
fn condition_read_guards_expire_at_intervening_source_effects() {
    let cases = [
        ("[info exists x] && $x > 0", "$x", "", true),
        ("![info exists x] || $x > 0", "$x", "", true),
        ("!![info exists x] && $x > 0", "$x", "", true),
        ("[info exists x] || $x > 0", "$x", "", false),
        ("![info exists x] && $x > 0", "$x", "", false),
        ("[info exists x] && ([set x 0] && $x > 0)", "$x", "", false),
        (
            "[info exists x] && ([unset x; set ready 1] && $x > 0)",
            "$x",
            "",
            false,
        ),
        ("[info exists x] && ([opaque] && $x > 0)", "$x", "", false),
        (
            "[info exists x] && ([upvar 0 other x; set ready 1] && $x > 0)",
            "$x",
            "",
            false,
        ),
        ("[info exists ::x] && $x > 0", "$x", "", false),
        (
            "[info exists a(safe)] && $a(safe) > 0",
            "$a(safe)",
            "",
            true,
        ),
        (
            "[info exists a(safe)] && $a(missing) > 0",
            "$a(missing)",
            "",
            false,
        ),
        ("[info exists x] && ($other && $x > 0)", "$x", "", true),
        (
            "[info exists x] && ($other && $x > 0)",
            "$x",
            "trace add variable other read ::trace_read;",
            false,
        ),
    ];
    for (condition, read, prefix, expected) in cases {
        let source = format!(
            "proc trace_read args {{}}; proc p {{c other}} {{if {{$c}} {{set x 1; set a(safe) 1}}; {prefix} if {{{condition}}} {{puts yes}}}}"
        );
        assert_eq!(
            condition_has_original_read_guard(&source, read),
            expected,
            "{condition}"
        );
    }
}

fn condition_has_original_read_guard(source: &str, read: &str) -> bool {
    use crate::analyser::Analyser;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    let mut analyser = Analyser::new();
    let _ = analyser.analyse(source, "tcl9.0");
    let input = analyser.resolved_analysis_input();
    let settings = analyser.source_compilation_settings();
    let registry = input.borrowed_context_registry().commands();
    let unit = CompilationUnit::build_with_analysis_input_and_source_settings(
        source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(input.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
        &settings,
    );
    let target = u32::try_from(source.rfind(read).unwrap()).unwrap();
    for function in unit.analysable_body_function_units() {
        for (&block, data) in &function.cfg.blocks {
            let Some(crate::cfg::Terminator::Branch {
                condition,
                condition_base: Some(base),
                ..
            }) = &data.terminator
            else {
                continue;
            };
            let view = crate::ssa::SsaSourceView::at_statement(&function.ssa, block, usize::MAX);
            let tokens = view.source_tokens();
            let reading = crate::tcl_expr_eval::ConditionReading {
                registry,
                config: input.lexer_config(),
                policy: crate::tcl_expr_eval::FoldPolicy::from_registry(registry),
                trusted: &|_| false,
                existence: crate::existence_query::QueryReading::for_cfg(
                    &function.cfg,
                    registry,
                    input.lexer_config(),
                    tokens,
                    Some(*base),
                )
                .for_diagnostics(),
            };
            if crate::tcl_expr_eval::condition_read_guards(condition, *base, reading)
                .iter()
                .any(|(span, cell)| {
                    function.abs_span(*span).start() == target
                        && tokens.is_some_and(|tokens| {
                            tokens.variable_accesses.iter().any(|access| {
                                access.source.span == *span
                                    && !access.context_alternatives().is_empty()
                                    && access.context_alternatives().iter().all(|context| {
                                        let place = access.place_in_context(context, registry);
                                        crate::var_resolve::canonical_place_key(&place).as_ref()
                                            == Some(cell)
                                    })
                            })
                        })
                })
            {
                return true;
            }
        }
    }
    false
}

#[test]
fn conditional_expression_template_retains_grammar_without_actual_entry() {
    let source = "proc f {} {expr {7 << 1}}";
    let (bindings, tokens) = original(source, "expr {7 << 1}", source.find("expr").unwrap());
    crate::command_binding::trace_original_g4_owner("conditional-expression", &bindings, &tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let receipt = binding
        .conditional_expression_evaluation(registry, &tokens)
        .expect("exact conditional expression receipt");
    assert_eq!(
        receipt.normal_result_representation(|_| None),
        Some(tcl_registry::TclType::Int)
    );
    assert_eq!(
        binding.runtime_reachability(),
        SourceRuntimeReachability::Reached
    );
    assert!(!binding.execution_is_unknown());
    assert_eq!(
        binding
            .proved_execution_target()
            .map(|target| target.command.as_str()),
        Some("::expr")
    );
    assert!(binding.compiler_lookup_state.is_none());
    assert!(bindings.final_state.baseline.native_entry.is_none());
    binding.trace_g4_normal_frame_facts("conditional-expression", &tokens, registry);
    assert_eq!(
        binding.original_normal_result(&tokens).unwrap().text(),
        "14"
    );
    assert!(binding.original_invocation_completes_normally(&tokens));
    assert!(binding.expression_preparation(&[]).is_none());
}

#[test]
fn conditional_expression_pool_representation_is_withdrawn_by_shared_getters() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    for source in [
        "set held 14; llength $held; expr {7 << 1}",
        "unknown_writer; expr {7 << 1}",
    ] {
        let (_, tokens) = original(
            source,
            &source[source.find("expr").unwrap()..],
            source.find("expr").unwrap(),
        );
        let answer = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.conditional_expression_evaluation(registry, &tokens))
            .and_then(|receipt| receipt.normal_result_representation(|_| None));
        assert!(answer.is_none(), "{source}");
    }
}

#[test]
fn conditional_expression_source_and_replaced_handler_do_not_borrow_receipts() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let (_, tokens) = original("expr {7 << 1}", "expr {7 << 1}", 0);
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(
        binding
            .conditional_expression_evaluation(registry, &tokens)
            .is_some()
    );
    let (_, foreign) = original("expr {8 << 1}", "expr {8 << 1}", 0);
    assert!(
        binding
            .conditional_expression_evaluation(registry, &foreign)
            .is_none()
    );
    let source = "proc expr args {return arbitrary}; expr {7 << 1}";
    let (_, replaced) = original(source, "expr {7 << 1}", source.rfind("expr").unwrap());
    assert!(
        replaced
            .source_binding
            .as_ref()
            .unwrap()
            .conditional_expression_evaluation(registry, &replaced)
            .is_none()
    );
}
