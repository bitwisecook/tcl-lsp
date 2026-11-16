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

use crate::interprocedural::test_body_identity::fixture_id;

use super::*;

#[test]
fn existence_inventory_keeps_literal_destination_names_and_unknown_targets() {
    let mut cfg = CfgFunction::new("source body", "entry");
    let entry = cfg.entry;
    for name in ["", "$literal", "${braced}", "$array(key)"] {
        cfg.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: tcl_lexer::Span::new(0, 0),
                name: name.into(),
                name_braced: true,
                value: "1".into(),
                value_span: None,
            });
    }
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    let metadata = ExistenceMetadataInputs {
        registry: &registry,
        context: registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(Into::into),
    };
    let (defined, _, unknown) = scan_defined_and_unset(&cfg, metadata);
    assert_eq!(
        defined,
        FxHashSet::from_iter(
            ["", "$literal", "${braced}", "$array"]
                .into_iter()
                .map(str::to_owned)
        )
    );
    assert!(!unknown.writes);
    assert!(
        !defined.contains("literal") && !defined.contains("braced") && !defined.contains("array")
    );
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: tcl_lexer::Span::new(0, 0),
            name: "$target".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        });
    let (defined, _, unknown) = scan_defined_and_unset(&cfg, metadata);
    assert!(unknown.writes);
    assert!(!defined.contains("target"));
}

#[test]
fn source_destroy_inventory_keeps_original_empty_sigil_and_array_name_data() {
    let (context, unit) = logical_callback_unit("unset {} {$literal} {arr(key)}");
    let registry = context.commands();
    let metadata = ExistenceMetadataInputs {
        registry,
        context: crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry,
            unit.ir_module.source_metadata_input.as_ref().unwrap(),
        ),
    };
    let (_, unset, unknown) = scan_defined_and_unset(&unit.top_level.cfg, metadata);
    for name in ["", "$literal", "arr(key)", "arr"] {
        assert!(unset.contains(name), "missing original {name:?}: {unset:?}");
    }
    assert!(!unset.contains("literal"));
    assert!(unknown.destroys);
    let (_, _, refused) = scan_defined_and_unset(
        &unit.top_level.cfg,
        ExistenceMetadataInputs {
            registry,
            context: None,
        },
    );
    assert!(refused.writes && refused.destroys);
}

#[test]
fn nominal_destroy_inventory_uses_selected_roles_and_keeps_unknown_argv() {
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    let metadata = ExistenceMetadataInputs {
        registry: &registry,
        context: registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(Into::into),
    };
    let mut cfg = CfgFunction::new("explicit standalone", "entry");
    cfg.blocks
        .get_mut(&cfg.entry)
        .unwrap()
        .statements
        .push(Statement::Call {
            span: tcl_lexer::Span::new(0, 0),
            command: "::unset".into(),
            canonical_command: None,
            args: vec![
                String::new(),
                "array(key)".into(),
                "$computed".into(),
                "nul\0tail".into(),
            ],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        });
    let (_, unset, unknown) = scan_defined_and_unset(&cfg, metadata);
    assert!(unset.contains("") && unset.contains("array") && unset.contains("array(key)"));
    assert!(!unset.contains("computed") && !unset.contains("nul") && !unset.contains("nul\0tail"));
    assert!(unknown.destroys);
    cfg.metadata_context = crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
    let (_, unset, unknown) = scan_defined_and_unset(&cfg, metadata);
    assert!(unset.is_empty() && unknown.writes && unknown.destroys);
}

fn logical_callback_unit(
    source: &str,
) -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::compilation_unit::CompilationUnit,
) {
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use std::sync::Arc;
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    (context, unit)
}

#[test]
fn lazy_semantic_projection_retains_original_callback_inventory_and_carrierless_refusal() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Callback names are conditional module exposure, never an executed callback or store.
    for (source, named, any) in [
        (
            "after 0 {set callback_only CALLBACK}; set local VALUE",
            true,
            false,
        ),
        (
            "after 0 {set $computed CALLBACK}; set local VALUE",
            false,
            true,
        ),
    ] {
        let (context, unit) = logical_callback_unit(source);
        let registry = context.commands();
        let module = &unit.ir_module;
        assert_eq!(
            module.deferred_writes.names.contains("callback_only"),
            named,
            "{source}"
        );
        assert_eq!(module.deferred_writes.any, any, "{source}");
        let extra = HashSet::new();
        let inputs = ValueFactInputs {
            param_constants: None,
            policy: FoldPolicy::from_registry(registry),
            extra_escaping: &extra,
            trace: TraceInputs {
                source_metadata_input: module.source_metadata_input.as_ref(),
                registry,
                traced_variables: &module.traced_variables,
                has_dynamic_variable_trace: module.has_dynamic_variable_trace,
                deferred_writes: &module.deferred_writes,
                analysis_context: None,
                existence: None,
            },
            folds: None,
        };
        let projection = SemanticValueProjection::new(inputs);
        let clear = SemanticValueProjection::new(ValueFactInputs {
            trace: TraceInputs {
                deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
                ..inputs.trace
            },
            ..inputs
        });
        assert_ne!(
            projection, clear,
            "the retained callback inventory rekeys the lazy projection"
        );
        let detached = projection.uncached();
        assert_eq!(projection, detached);
        let function = &unit.top_level;
        let mut carrierless = function.ssa.clone();
        carrierless.point_contexts = None;
        let escaping = compatibility_escaping(&function.cfg, &carrierless, inputs.trace, &extra);
        assert_eq!(escaping.contains("callback_only"), named, "{source}");
        assert_eq!(inputs.trace.writes_any_variable(), any);
        assert_eq!(
            detached.get(&function.cfg, &carrierless),
            Some(&semantic_value_facts(&function.cfg, &carrierless, inputs)),
        );
        assert!(
            compatibility_escaping(&function.cfg, &function.ssa, inputs.trace, &extra).is_empty(),
            "positioned source worlds retain their temporal rules"
        );
    }
}

#[test]
fn lazy_semantic_projection_preserves_independent_fold_owner_and_purity() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Cache replay retains conditional value inputs, never a native result/handler.
    let (context, unit) = logical_callback_unit("set result [string length abc]");
    let registry = context.commands();
    let module = &unit.ir_module;
    let input = module.source_metadata_input.as_ref().unwrap();
    let mut foreign_registry = CommandRegistry::build_default();
    foreign_registry.insert(tcl_registry::CommandSpec {
        name: "foreign_projection_owner_axis",
        ..foreign_registry.get("set").unwrap().clone()
    });
    let foreign_context = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl9.0")
            .with_command_store(foreign_registry.snapshot().shared_registry()),
    );
    assert_ne!(
        registry.snapshot().semantic_key(),
        foreign_context.commands().snapshot().semantic_key()
    );
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        foreign_context,
        input.lexer_config(),
    );
    let extra = HashSet::new();
    let trace = TraceInputs {
        source_metadata_input: Some(input),
        registry,
        traced_variables: &module.traced_variables,
        has_dynamic_variable_trace: module.has_dynamic_variable_trace,
        deferred_writes: &module.deferred_writes,
        analysis_context: None,
        existence: None,
    };
    let mut previous = Vec::new();
    for fold_input in [Some(input), None, Some(&foreign)] {
        for proven_pure_parameters in [false, true] {
            let inputs = ValueFactInputs {
                param_constants: None,
                policy: FoldPolicy::from_registry(registry),
                extra_escaping: &extra,
                trace,
                folds: Some(BuiltinFoldInputs {
                    source_metadata_input: fold_input,
                    registry,
                    mutations: &unit.command_mutations,
                    dialect: registry.profile(),
                    defining_class: None,
                    registry_engine: false,
                    trust: FoldTrust::ObservedBindings,
                    proven_pure_parameters,
                }),
            };
            let projection = SemanticValueProjection::new(inputs);
            let retained = projection.inputs.as_ref().unwrap();
            assert_eq!(retained.source_metadata_input.as_ref(), Some(input));
            assert_eq!(retained.fold_source_metadata_input.as_ref(), fold_input);
            assert_eq!(retained.proven_pure_parameters, proven_pure_parameters);
            assert_eq!(
                projection.get(&unit.top_level.cfg, &unit.top_level.ssa),
                Some(&semantic_value_facts(
                    &unit.top_level.cfg,
                    &unit.top_level.ssa,
                    inputs
                )),
            );
            assert_eq!(
                projection.matches_source_input(registry, input),
                fold_input == Some(input)
            );
            assert!(!projection.matches_source_input(registry, &foreign));
            assert_eq!(
                projection.uncached().matches_source_input(registry, input),
                fold_input == Some(input)
            );
            for other in &previous {
                assert_ne!(&projection, other);
            }
            previous.push(projection);
        }
    }
}

#[test]
fn semantic_math_dependencies_survive_only_successful_reached_evaluations() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    for (source, expected) in [
        ("set x [expr {abs(-3)}]", 1),
        ("set x [expr {0 && abs(-3)}]", 0),
        ("set x [expr {sqrt(-1)}]", 0),
        (
            "proc ::tcl::mathfunc::abs {x} {return 99}; set x [expr {abs(-3)}]",
            0,
        ),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source, &registry, false, profile,
        );
        let semantic = semantic_facts_for_unit(&unit, &registry);
        let requirements = &semantic.result.required_math_invocations;
        assert_eq!(requirements.len(), expected, "{source}");
        for requirement in requirements {
            assert_eq!(requirement.function, "abs");
            assert!(
                semantic.expressions.values().any(|expression| {
                    expression.required_math_invocations.contains(requirement)
                })
            );
        }
        assert_eq!(
            unit.top_level.sccp.required_math_invocations,
            [] as [crate::command_binding::SourceMathInvocation; 0]
        );
    }
}

#[test]
fn successful_lazy_expression_analysis_retains_preparation_without_reached_calls() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
        "set x [expr {0 && future_function(1)}]",
        &registry,
        false,
        profile,
    );
    let semantic = semantic_facts_for_unit(&unit, &registry);
    let result = &semantic.result;
    assert_eq!(result.required_math_invocations.len(), 0);
    assert_eq!(result.required_expression_preparations.len(), 1);
    assert_eq!(
        unit.top_level.sccp.required_expression_preparations,
        [] as [crate::command_binding::SourceExpressionPreparation; 0]
    );
    let preparation = &result.required_expression_preparations[0];
    assert_eq!(preparation.witness.source(), "0 && future_function(1)");
    assert!(semantic.expressions.values().any(|expression| {
        expression
            .required_expression_preparations
            .contains(preparation)
    }));
    let bindings = ExpressionMathBindings::for_origin(
        &unit.top_level.cfg.implicit_math_invocations,
        unit.top_level.cfg.executed_source.as_deref(),
        Some(preparation.source.base()),
    )
    .with_preparations(&unit.top_level.cfg.expression_preparations);
    let wrong_profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let dependencies = MathDependencies {
        purpose: SolverPurpose::SemanticAnalysis,
        ..MathDependencies::default()
    };
    assert!(
        fold_math_expression(
            preparation.witness.tree(),
            &Env::new(),
            FoldPolicy::for_profile(Some(true), Some(wrong_profile)),
            Some(MathFoldContext {
                point: None,
                bindings,
                dependencies: Some(&dependencies),
                incoming_reads: None,
            }),
        )
        .is_none()
    );
    assert_eq!(dependencies.preparations.borrow().len(), 0);
}

#[test]
fn analysis_value_keeps_native_conversion_obligations_out_of_execution_constants() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
        "set x 3; expr {$x + 1}",
        &registry,
        false,
        profile,
    );
    let function = &unit.top_level;
    let point = function
        .ssa
        .blocks
        .iter()
        .find_map(|(&block, body)| {
            body.statements
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    matches!(statement.statement, Statement::ExprEval { .. })
                        .then_some(ExpressionEvaluationPoint::Statement { block, index })
                })
        })
        .expect("retained full expression producer");
    let original_values = function.sccp.values.clone();
    let analysis = function
        .sccp
        .expression_analysis(
            &function.cfg,
            &function.ssa,
            point,
            FoldPolicy::for_profile(Some(false), Some(profile)),
        )
        .expect("known numerical input at exact native read");
    assert_eq!(analysis.evaluation.value, TclValue::Int(4));
    assert_ne!(
        analysis.evaluation.coercions,
        [] as [crate::tcl_expr_eval::NativeCoercionObligation; 0]
    );
    assert!(!analysis.evaluation.native_value_effects_are_proved());
    assert_eq!(analysis.required_expression_preparations.len(), 1);
    assert_eq!(analysis.required_math_invocations.len(), 0);
    assert_eq!(function.sccp.values, original_values);
}

fn semantic_facts_for_unit(
    unit: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
) -> SemanticValueFacts {
    semantic_value_facts(
        &unit.top_level.cfg,
        &unit.top_level.ssa,
        ValueFactInputs {
            param_constants: None,
            policy: FoldPolicy::from_registry(registry),
            extra_escaping: &HashSet::new(),
            trace: TraceInputs {
                deferred_writes: &unit.ir_module.deferred_writes,
                analysis_context: None,
                existence: None,
                source_metadata_input: unit.ir_module.source_metadata_input.as_ref(),
                registry,
                traced_variables: &unit.ir_module.traced_variables,
                has_dynamic_variable_trace: unit.ir_module.has_dynamic_variable_trace,
            },
            folds: Some(BuiltinFoldInputs {
                source_metadata_input: unit.ir_module.source_metadata_input.as_ref(),
                registry,
                mutations: &unit.command_mutations,
                dialect: registry.profile(),
                defining_class: None,
                registry_engine: false,
                trust: FoldTrust::ObservedBindings,
                proven_pure_parameters: false,
            }),
        },
    )
}

#[test]
fn semantic_solver_propagates_numeric_contents_without_erasing_producer_effects() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
        "set x 3; set y [expr {$x + 1}]; set z [expr {$y + 1}]; expr {$z + 1}",
        &registry,
        false,
        profile,
    );
    let facts = semantic_facts_for_unit(&unit, &registry);
    let analyses: Vec<_> = facts.expressions.values().collect();
    for expected in [4, 5, 6] {
        assert!(
            analyses
                .iter()
                .any(|analysis| analysis.evaluation.value == TclValue::Int(expected)),
            "missing chained value {expected}: {facts:#?}"
        );
    }
    assert!(
        analyses
            .iter()
            .all(|analysis| !analysis.evaluation.coercions.is_empty())
    );
    assert!(
        analyses
            .iter()
            .all(|analysis| analysis.required_expression_preparations.len() == 1)
    );
    let final_point = unit
        .top_level
        .ssa
        .blocks
        .iter()
        .find_map(|(&block, body)| {
            body.statements
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    matches!(statement.statement, Statement::ExprEval { .. })
                        .then_some(ExpressionEvaluationPoint::Statement { block, index })
                })
        })
        .expect("retained standalone expression");
    assert_eq!(
        facts.expression(final_point).unwrap().evaluation.value,
        TclValue::Int(6)
    );
    assert!(
        unit.top_level
            .sccp
            .expression_analysis(
                &unit.top_level.cfg,
                &unit.top_level.ssa,
                final_point,
                FoldPolicy::from_registry(&registry),
            )
            .is_none()
    );
}

#[test]
fn native_result_protocols_do_not_become_contents_only_execution_constants() {
    // An integer result has one canonical spelling, so it may become an
    // execution constant. A double from an executed operator keeps its
    // operation (C 8.6 spells it under `tcl_precision` when its string is
    // generated); a C 8.5+ constant tree is already a pooled literal.
    for (selected, source, contents, constant) in [
        (
            "tcl8.4",
            "set result [expr {2 + 2}]",
            ConstValue::Int(4),
            true,
        ),
        (
            "tcl8.6",
            "set result [expr {2 + 2}]",
            ConstValue::Int(4),
            true,
        ),
        ("jim", "set result [expr {2 + 2}]", ConstValue::Int(4), true),
        (
            "tcl8.4",
            "set result [expr {1 / 4.0}]",
            ConstValue::Float(0.25),
            false,
        ),
        (
            "jim",
            "set result [expr {1 / 4.0}]",
            ConstValue::Float(0.25),
            false,
        ),
        (
            "tcl8.6",
            "set result [expr {1 / 4.0}]",
            ConstValue::Float(0.25),
            true,
        ),
    ] {
        let owner = tcl_registry::model::ingress::static_context_for(selected);
        let registry = owner.commands();
        let profile = registry.profile().expect("actual selected native profile");
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source, registry, false, profile,
        );
        let semantic = semantic_facts_for_unit(&unit, registry);
        let producer = unit
                .top_level
                .ssa
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .find(|statement| {
                    matches!(&statement.statement, Statement::AssignExpr { name, .. } if name == "result")
                })
                .expect("retained original arithmetic result producer");
        assert!(!producer.defs.is_empty());
        for (&symbol, &version) in &producer.defs {
            let key = (symbol, version);
            assert_eq!(
                semantic.contents(key),
                Some(&LatticeValue::Const(contents.clone())),
                "numeric contents remain available for {selected} {source}",
            );
            assert_eq!(
                matches!(
                    unit.top_level.sccp.values.get(&key),
                    Some(LatticeValue::Const(_))
                ),
                constant,
                "execution constant for {selected} {source}",
            );
        }
    }
}

#[test]
fn procedure_body_execution_constants_use_retained_preparation() {
    for (expression, expected_constant) in [("2 + 2", true), ("1 / 4.0", false)] {
        let owner = tcl_registry::model::ingress::static_context_for("tcl8.4");
        let registry = owner.commands();
        let profile = registry.profile().expect("selected profile");
        let source = format!("proc p {{}} {{set result [expr {{{expression}}}]; return $result}}");
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            &source, registry, false, profile,
        );
        let mut found = false;
        for function in unit.procedures.values().chain(unit.body_units.values()) {
            for statement in function
                .ssa
                .blocks
                .values()
                .flat_map(|block| &block.statements)
            {
                if !matches!(&statement.statement, Statement::AssignExpr { name, .. } if name == "result")
                {
                    continue;
                }
                found = true;
                assert!(!statement.defs.is_empty());
                for (&symbol, &version) in &statement.defs {
                    assert_eq!(
                        matches!(
                            function.sccp.values.get(&(symbol, version)),
                            Some(LatticeValue::Const(_))
                        ),
                        expected_constant,
                        "procedure expression {expression}",
                    );
                }
            }
        }
        assert!(found, "retained procedure expression {expression}");
    }
}

#[test]
fn lambda_body_execution_constants_use_retained_preparation() {
    let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = owner.commands();
    let profile = registry.profile().expect("selected profile");
    let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
        "apply {{} {set result [expr {2 + 2}]; return $result}}",
        registry,
        false,
        profile,
    );
    let mut found = false;
    for function in unit.body_units.values() {
        for statement in function
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
        {
            if !matches!(&statement.statement, Statement::AssignExpr { name, .. } if name == "result")
            {
                continue;
            }
            found = true;
            assert!(!statement.defs.is_empty());
            for (&symbol, &version) in &statement.defs {
                assert!(matches!(
                    function.sccp.values.get(&(symbol, version)),
                    Some(LatticeValue::Const(ConstValue::Int(4)))
                ));
            }
        }
    }
    assert!(found, "retained lambda expression");
}

#[test]
fn semantic_solver_keeps_selected_native_result_bytes_separate_from_number() {
    let profile = tcl_registry::model::ingress::static_context_for("jim")
        .commands()
        .profile()
        .unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
        "set x 003; set y [expr {$x}]; expr {$y + 1}",
        &registry,
        false,
        profile,
    );
    let facts = semantic_facts_for_unit(&unit, &registry);
    let selected = facts
        .expressions
        .values()
        .find(|analysis| {
            matches!(
                analysis.evaluation.result_dependency,
                Some(
                    crate::tcl_expr_eval::NativeExpressionResultDependency::SelectedOperand { .. }
                )
            )
        })
        .expect("retained selected-object producer");
    assert_eq!(selected.evaluation.value, TclValue::Int(3));
    assert!(matches!(&selected.evaluation.result_dependency,
            Some(crate::tcl_expr_eval::NativeExpressionResultDependency::SelectedOperand {
                existing_bytes: Some(bytes), ..
            }) if bytes == "003"));
    assert!(
        facts
            .result
            .values
            .values()
            .any(|value| { *value == LatticeValue::Const(ConstValue::String("003".to_owned())) })
    );
    assert!(
        facts
            .expressions
            .values()
            .any(|analysis| analysis.evaluation.value == TclValue::Int(4))
    );
}

/// Whitespace inside a Tcl word is data, so the lattice keeps the exact
/// spelling (#2052).
///
/// Trimming corrupted every consumer: `set p { again}` reached the lattice
/// as `again`, so `append s $p` was rewritten to `append s again` and the
/// program printed `helloagain` where tclsh 9.0.4 prints `hello again`.
/// `string length $p` folded to 5 against the true 6.
#[test]
fn a_literal_keeps_its_surrounding_whitespace() {
    assert_eq!(
        parse_literal_value(" again"),
        ConstValue::String(" again".to_owned()),
    );
    assert_eq!(
        parse_literal_value("trailing "),
        ConstValue::String("trailing ".to_owned()),
    );
    // An integer with whitespace around it is not the integer: rendering
    // it back as `5` would drop the space just as surely.
    assert_eq!(
        parse_literal_value(" 5"),
        ConstValue::String(" 5".to_owned()),
    );
    // The bare integer still folds.
    assert_eq!(parse_literal_value("5"), ConstValue::Int(5));
    assert_eq!(parse_literal_value("-17"), ConstValue::Int(-17));
}
use crate::cfg::{Block, BlockId, Function, Terminator};
use crate::expr_ast::ExprNode;

fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

/// [`evaluate_fixture_def_with_folds`] under a module that mutates no command
/// binding — the stance the shared per-unit lattice takes for an
/// untouched file ([`FoldTrust::ObservedBindings`] over
/// [`crate::command_binding::ModuleCommandMutations`]'s `Default`).
///
/// Plain [`evaluate_fixture_def`] holds no whole-module command view at all and
/// therefore folds no builtin command substitution, so a fold-arm test
/// has to state the trust fact it folds under.
fn evaluate_pristine<S: std::hash::BuildHasher>(
    stmt: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: &SsaFunction,
    policy: FoldPolicy,
) -> LatticeValue {
    evaluate_fixture_def_with_folds(
        stmt,
        values,
        ssa,
        policy,
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry: &registry(),
            mutations: &crate::command_binding::ModuleCommandMutations::default(),
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: FoldTrust::ObservedBindings,
            proven_pure_parameters: false,
        }),
        tcl_lexer::LexerConfig::default(),
    )
}

/// Convenience wrapper over [`crate::sccp::sccp`] for tests with no `Module` in
/// hand — no traced variables, no dynamic variable trace.
fn sccp_no_traces(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
    policy: FoldPolicy,
) -> SccpResult {
    crate::sccp::sccp(
        cfg,
        ssa,
        param_constants,
        policy,
        TraceInputs {
            deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
            analysis_context: None,
            existence: None,
            source_metadata_input: None,
            registry: &registry(),
            traced_variables: &BTreeSet::new(),
            has_dynamic_variable_trace: false,
        },
    )
}

/// Intern `name` and insert a fresh block; returns its [`BlockId`].
fn block(f: &mut Function, name: &str) -> BlockId {
    let id = f.intern_block(name);
    f.blocks.insert(id, Block::new(name));
    id
}

fn id_of(f: &Function, name: &str) -> BlockId {
    f.block_id(name).expect("interned")
}

fn goto(target: BlockId) -> Terminator {
    Terminator::Goto { target, span: None }
}

fn branch(cond: ExprNode, tt: BlockId, ft: BlockId) -> Terminator {
    Terminator::Branch {
        condition: cond,
        true_target: tt,
        false_target: ft,
        span: None,
        condition_base: None,
    }
}

fn literal(text: &str) -> ExprNode {
    ExprNode::Literal {
        text: text.into(),
        start: 0,
        end: u32::try_from(text.len()).unwrap_or(0),
    }
}

// Join.

#[test]
fn join_unknown_absorbs() {
    let c = LatticeValue::Const(ConstValue::Int(7));
    assert_eq!(join(&LatticeValue::Unknown, &c), c);
    assert_eq!(join(&c, &LatticeValue::Unknown), c);
}

#[test]
fn join_overdefined_is_absorbing() {
    let c = LatticeValue::Const(ConstValue::Int(7));
    assert_eq!(
        join(&LatticeValue::Overdefined, &c),
        LatticeValue::Overdefined
    );
    assert_eq!(
        join(&c, &LatticeValue::Overdefined),
        LatticeValue::Overdefined
    );
}

#[test]
fn join_identical_const_stays_const() {
    let c = LatticeValue::Const(ConstValue::Int(7));
    assert_eq!(join(&c, &c), c);
}

#[test]
fn join_distinct_consts_widens_to_set() {
    let a = LatticeValue::Const(ConstValue::Int(1));
    let b = LatticeValue::Const(ConstValue::Int(2));
    let merged = join(&a, &b);
    match merged {
        LatticeValue::ConstSet(ref vs) => {
            assert_eq!(vs.len(), 2);
        }
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn join_existing_set_absorbs_member() {
    let a = LatticeValue::ConstSet(vec![ConstValue::Int(1), ConstValue::Int(2)]);
    let b = LatticeValue::Const(ConstValue::Int(1));
    // Adding a member already in the set returns `old` unchanged.
    assert_eq!(join(&a, &b), a);
}

#[test]
fn join_widens_large_sets_to_overdefined() {
    let big: Vec<ConstValue> = (0..MAX_CONSTSET_SIZE)
        .map(|i| ConstValue::Int(i64::try_from(i).unwrap()))
        .collect();
    let a = LatticeValue::ConstSet(big);
    let b = LatticeValue::Const(ConstValue::Int(999));
    assert_eq!(join(&a, &b), LatticeValue::Overdefined);
}

#[test]
fn set_value_tracks_change() {
    let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
    let key: ValueKey = (Symbol(0), 1);
    assert!(set_value(
        &mut values,
        key,
        &LatticeValue::Const(ConstValue::Int(1))
    ));
    assert!(!set_value(
        &mut values,
        key,
        &LatticeValue::Const(ConstValue::Int(1))
    ));
    assert!(set_value(
        &mut values,
        key,
        &LatticeValue::Const(ConstValue::Int(2))
    ));
}

// Predecessors + cfg_order.

#[test]
fn predecessors_simple_chain() {
    let mut f = Function::new("::top", "a");
    let a = f.entry;
    let b = block(&mut f, "b");
    f.blocks.get_mut(&a).unwrap().terminator = Some(goto(b));
    f.blocks.get_mut(&b).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let p = compute_predecessors(&f);
    assert!(p.get(&b).unwrap().contains(&a));
    assert!(p.get(&a).is_none_or(HashSet::is_empty));
}

#[test]
fn cfg_order_starts_at_entry() {
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    let t = block(&mut f, "t");
    let e = block(&mut f, "e");
    let join = block(&mut f, "join");
    f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(literal("1"), t, e));
    f.blocks.get_mut(&t).unwrap().terminator = Some(goto(join));
    f.blocks.get_mut(&e).unwrap().terminator = Some(goto(join));
    f.blocks.get_mut(&join).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let order = cfg_order(&f);
    assert_eq!(order[0], entry);
    // join must appear after both branches.
    let join_pos = order.iter().position(|b| *b == join).unwrap();
    let t_pos = order.iter().position(|b| *b == t).unwrap();
    let e_pos = order.iter().position(|b| *b == e).unwrap();
    assert!(join_pos > t_pos);
    assert!(join_pos > e_pos);
}

#[test]
fn point_expression_environment_preserves_alias_spelling_and_retargeting() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let source =
        include_str!("../../../../tcl-syntax/tests/data/resolution/alias-retarget-expression.tcl");
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl9.0",
    );
    let ssa = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .ssa;
    let mut seen = 0;
    for (&block, body) in &ssa.blocks {
        for (index, statement) in body.statements.iter().enumerate() {
            let (Statement::AssignExpr { name, .. } | Statement::AssignValue { name, .. }) =
                &statement.statement
            else {
                continue;
            };
            let expected = match name.as_str() {
                "first" => 1,
                "second" => 9,
                _ => continue,
            };
            let lookup = SsaSourceView::at_statement(ssa, block, index);
            let symbol = lookup.symbol("y").expect("point-specific alias read");
            let version = statement.uses[&symbol];
            let values = HashMap::from([(
                (symbol, version),
                LatticeValue::Const(ConstValue::Int(expected)),
            )]);
            let env = env_from_uses(&statement.uses, &values, lookup);
            assert!(
                matches!(env.get("y"), Some(EnvValue::Int(value)) if *value == expected),
                "source alias must remain available: {env:?}"
            );
            assert_eq!(
                env_from_uses(
                    &statement.uses,
                    &values,
                    SsaSourceView::at_statement(ssa, BlockId(u32::MAX), index)
                )
                .len(),
                0,
                "missing exact point cannot recover a display-name environment"
            );
            seen += 1;
        }
    }
    assert_eq!(
        seen,
        2,
        "both source expressions must be exercised: {}",
        ssa.outline()
    );
}

/// A literal body resolves its actual selected-frame mutation; a dynamic
/// body retains unknown contents and presence on its reached continuation.
fn native_existence_unit(
    source: &str,
    registry: &CommandRegistry,
) -> crate::environment_ingress::RetainedNativeUnit {
    let profile = registry.profile().expect("selected native fixture profile");
    let (owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(captured)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
            ..Default::default()
        },
        ..crate::command_binding::SourceAnalysisEntry::default()
    };
    let unit = crate::compilation_unit::CompilationUnit::build_with_source_entry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        &entry,
    );
    crate::environment_ingress::RetainedNativeUnit::new(unit, owner)
}

#[test]
fn existence_fold_resolves_literal_upframe_and_declines_dynamic_body() {
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = native_existence_unit(
        "proc f {} { uplevel 0 { set created 1 }; if {[info exists created]} { return yes } else { return no } }; f",
        &registry,
    );
    let f = cu.function("::f").expect("procedure analysed");
    assert!(
        f.cfg
            .blocks
            .values()
            .flat_map(|block| block.statements.iter())
            .any(|statement| matches!(statement, Statement::UpFrame { .. })),
        "the regression requires the static uplevel lowering path"
    );
    assert!(
        f.sccp.constant_branches.iter().any(|branch| branch.value),
        "the literal selected-frame store creates the queried cell: {:?}",
        f.sccp.constant_branches,
    );
    let dynamic = native_existence_unit(
        "proc f {script} {uplevel 0 $script; if {[info exists created]} {return yes} else {return no}}; f [mystery]",
        &registry,
    );
    assert!(
        dynamic
            .function("::f")
            .unwrap()
            .sccp
            .constant_branches
            .is_empty(),
        "a caller-supplied body may create or remove the queried cell",
    );
}

#[test]
fn existence_fold_resolves_nested_upframe_alias_unset_mutation() {
    // `uplevel 0` evaluates in this procedure's frame. The nested body
    // aliases that frame's parameter and unsets it, so Tcl observes the else
    // branch. The no-uplevel twin proves the branch is otherwise foldable;
    // this tests the resolved mutation rather than a whole-function barrier.
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let stable = native_existence_unit(
        "proc f {local} { if {[info exists local]} { return yes } else { return no } }; f VALUE",
        &registry,
    );
    assert!(
        !stable
            .function("::f")
            .expect("control procedure analysed")
            .sccp
            .constant_branches
            .is_empty(),
        "the unmutated control must be foldable"
    );

    let mutated = native_existence_unit(
        "proc f {local} { uplevel 0 { uplevel 0 { upvar 0 local alias; unset alias } }; if {[info exists local]} { return yes } else { return no } }; f VALUE",
        &registry,
    );
    let f = mutated.function("::f").expect("mutated procedure analysed");
    let outer = f
        .cfg
        .blocks
        .values()
        .flat_map(|block| block.statements.iter())
        .find_map(|statement| match statement {
            Statement::UpFrame { body, .. } => Some(body),
            _ => None,
        })
        .expect("outer literal uplevel must lower to UpFrame");
    assert!(
        outer
            .statements
            .iter()
            .any(|statement| matches!(statement, Statement::UpFrame { .. })),
        "the nested literal uplevel must remain an UpFrame in its parent body"
    );
    let branches = &f.sccp.constant_branches;
    assert!(
        branches.iter().any(|branch| !branch.value),
        "the literal alias unset removes the selected local: {branches:?}",
    );
    assert!(branches.iter().all(|branch| !branch.value));
}

#[test]
fn existence_folding_requires_actual_entry_and_selected_query() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let declaration = "proc f {local} {if {[info exists local]} {return yes} else {return no}}";
    let missing_entry = crate::compilation_unit::CompilationUnit::build_with_source_entry(
        &format!("{declaration}; f VALUE"),
        crate::compilation_unit::UnitBuildOptions {
            registry: &registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        &crate::command_binding::SourceAnalysisEntry {
            unknown_entry: true,
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            ..Default::default()
        },
    );
    assert!(missing_entry.function("::f").is_none());
    assert_missing_entry_retains_only_declared_query(declaration, &registry, profile);
    for source in [
        declaration.to_owned(),
        format!("rename info {{}}; proc info args {{return UNKNOWN}}; {declaration}; f VALUE"),
        format!("{declaration}; mystery; f VALUE"),
    ] {
        let unit = native_existence_unit(&source, &registry);
        assert!(
            unit.function("::f")
                .unwrap()
                .sccp
                .constant_branches
                .is_empty(),
            "unentered or unselected query must not fold: {source}"
        );
    }
}

fn assert_missing_entry_retains_only_declared_query(
    declaration: &str,
    registry: &CommandRegistry,
    profile: &tcl_dialect::DialectProfile,
) {
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
        declaration,
        config,
        registry,
        crate::command_binding::SourceAnalysisOptions {
            unknown_entry: true,
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        },
    );
    let query = "info exists local";
    let offset = u32::try_from(declaration.find(query).unwrap()).unwrap();
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(query, offset, config).remove(0);
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(declaration),
        config,
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    let layout = binding
        .declaration_operand_layout_advice(&tokens)
        .expect("the original declaration retains conditional query geometry");
    assert!(!layout.closed_lookup());
    assert!(binding.unknown);
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.compiler_lookup_state.is_none());
    let site = binding.invocation_site().unwrap();
    let lexical_body = bindings
        .conditional_body_entry_at(&site.source, site.offset)
        .expect("conditional lexical source is independent of actual entry");
    assert!(lexical_body.owns_source(&site.source, site.offset));
    assert!(!bindings.has_actual_procedure_entry_at(&site.source, site.offset));
    assert_eq!(
        binding.native_compilation_admission_selection(),
        tcl_registry::native_compilation::NativeCompilationSelection::Unknown
    );
}

#[test]
fn upframe_body_models_upvar_alias_unset_in_the_caller_frame() {
    // tclsh: `proc caller {} {set caller_value 1; f; info exists
    // caller_value}; proc f {} {uplevel 1 {upvar 0 caller_value alias;
    // unset alias}}; caller` returns 0. The literal UpFrame body therefore
    // carries a real caller-frame mutation, not merely a control-flow
    // wrapper, and is a separate vector from the nested local-frame test.
    let registry = CommandRegistry::build_default();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {} { uplevel 1 { upvar 0 caller_value alias; unset alias } }",
        &registry,
        false,
    );
    let f = cu.function("::f").expect("procedure analysed");
    let body = f
        .cfg
        .blocks
        .values()
        .flat_map(|block| block.statements.iter())
        .find_map(|statement| match statement {
            Statement::UpFrame { body, .. } => Some(body),
            _ => None,
        })
        .expect("literal caller-frame body must lower to UpFrame");
    assert!(body.statements.iter().any(|statement| {
        matches!(statement, Statement::Call { command, .. } if command == "upvar")
    }));
    assert!(body.statements.iter().any(|statement| {
        matches!(statement, Statement::Call { command, .. } if command == "unset")
    }));
}

// Driver.

use crate::ir::Statement;
use crate::ssa::{SsaBlock, SsaStatement};
use tcl_lexer::Span;

/// A block-less SSA function used purely as a variable-name interner for
/// the hand-built statement / lattice tests.
fn bare_ssa() -> SsaFunction {
    SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()])
}

fn assign_const_stmt(ssa: &mut SsaFunction, name: &str, value: &str, ver: u32) -> SsaStatement {
    let mut defs = HashMap::new();
    defs.insert(ssa.intern_var(name), ver);
    SsaStatement {
        statement: Statement::AssignConst {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: false,
            value: value.into(),
            value_span: None,
        },
        uses: HashMap::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

fn empty_ssa_block(name: &str) -> SsaBlock {
    SsaBlock {
        name: name.into(),
        phis: Vec::new(),
        statements: Vec::new(),
        entry_versions: HashMap::new(),
        exit_versions: HashMap::new(),
    }
}

/// Build an [`SsaFunction`] over `f`'s interner with the given per-name SSA
/// blocks; any CFG block not listed gets an empty SSA block.
fn make_ssa(f: &Function, named: Vec<(&str, SsaBlock)>) -> SsaFunction {
    let mut ssa = SsaFunction::trivial("::top", f.entry, f.block_names().to_vec());
    let mut provided: std::collections::HashSet<BlockId> = std::collections::HashSet::new();
    for (name, blk) in named {
        let id = id_of(f, name);
        provided.insert(id);
        ssa.blocks.insert(id, blk);
    }
    for id in f.blocks.keys() {
        if !provided.contains(id) {
            ssa.blocks.insert(*id, empty_ssa_block(f.block_name(*id)));
        }
    }
    ssa
}

#[test]
fn phi_join_widens_version_zero_livein_to_overdefined() {
    // A phi merging a version-0 live-in (a proc parameter / global /
    // other caller-supplied root) with a defined-arm constant must widen
    // to Overdefined — the caller's value cannot vanish from the merge.
    // Otherwise `if {$c} { set x 5 }; if {$x == 5} {A} else {B}` folds the
    // second test always-true and deletes the else arm. Mirrors the
    // interval pass joining TOP for `inc == 0` (intervals.rs:570-574).
    let mut ssa = bare_ssa();
    let x = ssa.intern_var("x");
    let mut block = empty_ssa_block("merge");
    block.phis.push(crate::ssa::Phi {
        name: x,
        version: 2,
        incoming: HashMap::from([(BlockId(1), 0), (BlockId(2), 1)]),
    });
    // `seed_live_in_roots` pins the version-0 root Overdefined.
    let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
    values.insert((x, 0), LatticeValue::Overdefined);
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(5)));
    assert!(sccp_process_phis(
        &mut values,
        &block,
        &[BlockId(1), BlockId(2)]
    ));
    assert_eq!(values.get(&(x, 2)), Some(&LatticeValue::Overdefined));
}

#[test]
fn barrier_own_defs_become_overdefined() {
    // A barrier (e.g. `dict for {x y} $d {}`) defines its
    // own variables. Those defs must be set Overdefined so they participate
    // in a downstream phi join; otherwise the def key is never inserted and
    // a phi merging the barrier-def with a constant folds to that constant.
    let mut ssa = bare_ssa();
    let x = ssa.intern_var("x");
    let mut block = empty_ssa_block("barrier_block");
    let mut defs: HashMap<Symbol, crate::ssa::Version> = HashMap::new();
    defs.insert(x, 2);
    block.statements.push(SsaStatement {
        statement: Statement::Barrier {
            span: Span::new(0, 0),
            reason: "dict-for".into(),
            command: "::tcl::dict::for".into(),
            canonical_command: None,
            args: vec!["d".into()],
            tokens: None,
        },
        uses: HashMap::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    });
    let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
    let escaping: HashSet<String> = HashSet::new();
    assert!(sccp_process_statements(
        &mut values,
        &block,
        &ssa,
        StatementInputs {
            clobbers: None,
            store: &StoreInputs {
                reference_config: Some(tcl_lexer::LexerConfig::default()),
                math_dependencies: None,
                cfg: None,
                escaping: &escaping,
                policy: FoldPolicy::default(),
                has_dynamic_variable_trace: false,
                opaque_native_accesses: false,
                folds: None,
                registry: &CommandRegistry::build_default(),
            },
        }
    ));
    assert_eq!(
        values.get(&(x, 2)),
        Some(&LatticeValue::Overdefined),
        "a barrier-defined variable must be Overdefined, not absent/Unknown"
    );
}

#[test]
fn phi_join_defaults_missing_version_zero_feed_to_overdefined() {
    // Defence in depth: even if a version-0 feed was never seeded, the
    // join must treat it as Overdefined rather than skipping it (which
    // would leave the phi holding the defined-arm constant).
    let mut ssa = bare_ssa();
    let x = ssa.intern_var("x");
    let mut block = empty_ssa_block("merge");
    block.phis.push(crate::ssa::Phi {
        name: x,
        version: 2,
        incoming: HashMap::from([(BlockId(1), 0), (BlockId(2), 1)]),
    });
    let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(5)));
    assert!(sccp_process_phis(
        &mut values,
        &block,
        &[BlockId(1), BlockId(2)]
    ));
    assert_eq!(values.get(&(x, 2)), Some(&LatticeValue::Overdefined));
}

#[test]
fn sccp_marks_entry_executable_and_propagates_const() {
    // entry: set x 42
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    f.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let mut ssa = make_ssa(&f, vec![]);
    let stmt = assign_const_stmt(&mut ssa, "x", "42", 1);
    ssa.blocks.get_mut(&entry).unwrap().statements.push(stmt);
    let x = ssa.var_symbol("x").unwrap();

    let r = sccp_no_traces(&f, &ssa, None, FoldPolicy::default());
    assert!(r.executable_blocks.contains(&entry));
    assert_eq!(
        r.values.get(&(x, 1)),
        Some(&LatticeValue::Const(ConstValue::Int(42)))
    );
}

#[test]
fn sccp_constant_branch_detected_and_taken_target_marked() {
    // entry: branch on literal "1" → true → "t", false → "e"
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    let t = block(&mut f, "t");
    let e = block(&mut f, "e");
    f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(literal("1"), t, e));
    f.blocks.get_mut(&t).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let ssa = make_ssa(&f, vec![]);

    let r = sccp_no_traces(&f, &ssa, None, FoldPolicy::default());
    assert!(r.executable_blocks.contains(&t));
    assert!(!r.executable_blocks.contains(&e));
    assert_eq!(r.constant_branches.len(), 1);
    let cb = &r.constant_branches[0];
    assert!(cb.value);
    assert_eq!(cb.taken_target, "t");
    assert_eq!(cb.not_taken_target, "e");
}

#[test]
fn sccp_false_branch_prunes_true_target() {
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    let t = block(&mut f, "t");
    let e = block(&mut f, "e");
    f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(literal("0"), t, e));
    f.blocks.get_mut(&t).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let ssa = make_ssa(&f, vec![]);

    let r = sccp_no_traces(&f, &ssa, None, FoldPolicy::default());
    assert!(!r.executable_blocks.contains(&t));
    assert!(r.executable_blocks.contains(&e));
}

#[test]
fn sccp_unknown_branch_executes_both_targets() {
    // Var reference — lattice value defaults to Unknown → decision None.
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    let t = block(&mut f, "t");
    let e = block(&mut f, "e");
    let cond = ExprNode::Var {
        text: "$z".into(),
        name: "z".into(),
        start: 0,
        end: 2,
    };
    f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(cond, t, e));
    f.blocks.get_mut(&t).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let ssa = make_ssa(&f, vec![]);

    let r = sccp_no_traces(&f, &ssa, None, FoldPolicy::default());
    assert!(r.executable_blocks.contains(&t));
    assert!(r.executable_blocks.contains(&e));
    assert_eq!(r.constant_branches.len(), 0);
}

#[test]
fn evaluate_def_assign_const_produces_int_or_string() {
    let mut ssa = bare_ssa();
    let s_int = assign_const_stmt(&mut ssa, "x", "42", 1);
    assert_eq!(
        evaluate_fixture_def(
            &s_int,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(42))
    );
    let s_str = assign_const_stmt(&mut ssa, "x", "hello", 1);
    assert_eq!(
        evaluate_fixture_def(
            &s_str,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::String("hello".into()))
    );
}

#[test]
fn evaluate_def_assign_expr_retains_numeric_precision_and_conversion() {
    let unit = cu("set a 2; set x [expr {$a + 3}]");
    let facts = unit.top_level.semantic_values().unwrap();
    let analysis = facts
        .expression_iter()
        .find_map(|(_, analysis)| {
            (analysis.evaluation.value == TclValue::Int(5)).then_some(analysis)
        })
        .expect("the actual prepared expression computes 5");
    assert_ne!(
        analysis.evaluation.coercions,
        [] as [crate::tcl_expr_eval::NativeCoercionObligation; 0]
    );
    assert!(
        !unit
            .top_level
            .sccp
            .values
            .values()
            .any(|value| { *value == LatticeValue::Const(ConstValue::Int(5)) }),
        "a contents value alone cannot erase its retained-object conversion"
    );
}

/// SCCP folds the iRules word operators when — and only when — the
/// policy says the dialect has them.
///
/// Evaluating through a dialect-blind entry point (a bare
/// `octal: Option<bool>`) leaves `FoldOps::is_irules` `false`, so every
/// word operator silently declines while the `eq` control on the same
/// shape folds, because plain Tcl shares it.
#[test]
fn evaluate_def_folds_irules_word_operator_only_under_an_irules_policy() {
    let mut ssa = bare_ssa();
    let subject = ssa.intern_var("s");
    let out = ssa.intern_var("hit");
    let mut uses = HashMap::new();
    uses.insert(subject, 1);
    let mut defs = HashMap::new();
    defs.insert(out, 1);

    let expr = ExprNode::Binary {
        op: crate::expr_ast::BinOp::Contains,
        left: Box::new(ExprNode::Var {
            text: "$s".into(),
            name: "s".into(),
            start: 0,
            end: 2,
        }),
        right: Box::new(ExprNode::String {
            text: "cd".into(),
            start: 3,
            end: 7,
        }),
    };
    let stmt_ssa = SsaStatement {
        statement: Statement::AssignExpr {
            span: Span::new(0, 0),
            name: "hit".into(),
            name_braced: false,
            expr,
            command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
            expr_base: None,
            fallback_value: "[expr {$s contains \"cd\"}]".into(),
        },
        uses,
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    };
    let mut values = HashMap::new();
    values.insert(
        (subject, 1),
        LatticeValue::Const(ConstValue::String("abcde".into())),
    );

    let irules = FoldPolicy::for_profile(Some(true), Some(tcl_dialect::DialectProfile::irules()));
    assert_eq!(
        evaluate_fixture_def(
            &stmt_ssa,
            &values,
            &ssa,
            irules,
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(1)),
        "`$s contains \"cd\"` with $s = abcde must fold to 1 under an iRules policy"
    );
    // TN: without the dialect fact the fold is declined, not guessed.
    assert_eq!(
        evaluate_fixture_def(
            &stmt_ssa,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Overdefined,
        "a dialect-blind policy must decline the word-operator fold"
    );
}

// evaluate_fixture_def for Incr.

fn incr_stmt(
    ssa: &mut SsaFunction,
    name: &str,
    amount: Option<&str>,
    old_ver: u32,
    new_ver: u32,
) -> SsaStatement {
    let sym = ssa.intern_var(name);
    let mut uses = HashMap::new();
    uses.insert(sym, old_ver);
    let mut defs = HashMap::new();
    defs.insert(sym, new_ver);
    SsaStatement {
        statement: Statement::Incr {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: false,
            amount: amount.map(String::from),
            amount_braced: false,
            safe_on_uninit: false,
        },
        uses,
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

#[test]
fn evaluate_def_incr_default_amount() {
    // x@1 = Const(Int(5)); `incr x` → x@2 = Const(Int(6)).
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(5)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(6))
    );
}

#[test]
fn evaluate_def_incr_integer_literal_amount() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", Some("10"), 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(3)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(13))
    );
}

#[test]
fn evaluate_def_incr_negative_literal_amount() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", Some("-2"), 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(10)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(8))
    );
}

#[test]
fn evaluate_def_incr_var_ref_amount() {
    // `incr x $y` where $y resolves to 4.
    let mut ssa = bare_ssa();
    let mut stmt = incr_stmt(&mut ssa, "x", Some("$y"), 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let y = ssa.intern_var("y");
    stmt.uses.insert(y, 1);
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(6)));
    values.insert((y, 1), LatticeValue::Const(ConstValue::Int(4)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(10))
    );
}

#[test]
fn evaluate_def_incr_unknown_base_propagates_unknown() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
    let values = HashMap::new();
    // No entry for x@1 → base is Unknown → result Unknown.
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Unknown
    );
}

#[test]
fn evaluate_def_incr_overdefined_base_widens() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Overdefined);
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Overdefined
    );
}

#[test]
fn evaluate_def_incr_non_integer_amount_widens() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", Some("2.5"), 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(1)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Overdefined
    );
}

fn assert_profile_reference_shapes(
    profile: &str,
    config: tcl_lexer::LexerConfig,
    rules: tcl_syntax::word_rules::WordValueRules,
    uses: &HashMap<Symbol, crate::ssa::Version>,
    values: &HashMap<ValueKey, LatticeValue>,
    ssa: &SsaFunction,
) {
    assert_eq!(
        resolve_simple_var_ref("$café", uses, values, ssa, Some(config)).is_some(),
        profile == "jim"
    );
    assert_eq!(
        resolve_simple_var_ref("${a{b}c}", uses, values, ssa, Some(config)).is_some(),
        matches!(profile, "tcl9.0" | "tcl9.1")
    );
    assert_eq!(
        resolve_simple_var_ref("$a", &HashMap::new(), values, ssa, Some(config)),
        None
    );
    assert_eq!(
        resolve_foreach_list_via_lattice("$a", &HashMap::new(), values, ssa, rules, Some(config)),
        None
    );
}

#[test]
fn original_value_reference_consumers_require_grammar_and_scalar_shape() {
    // Original inputs: experiments/name-handling/oracles/whole-variable-reference-shapes-20261014.
    let mut ssa = bare_ssa();
    let mut uses = HashMap::new();
    let mut values = HashMap::new();
    for name in ["a", "a:b", "café", "", "scalar(open", "arr(key)", "a{b}c"] {
        let symbol = ssa.intern_var(name);
        uses.insert(symbol, 1);
        values.insert(
            (symbol, 1),
            LatticeValue::Const(ConstValue::String("one two".into())),
        );
    }
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let config = tcl_lexer::LexerConfig::for_dialect(profile);
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
        for text in ["$a", "${}", "${scalar(open}"] {
            assert!(
                resolve_simple_var_ref(text, &uses, &values, &ssa, Some(config)).is_some(),
                "{profile}: {text}"
            );
            assert_eq!(
                resolve_foreach_list_via_lattice(text, &uses, &values, &ssa, rules, Some(config)),
                Some(vec!["one".into(), "two".into()]),
                "{profile}: {text}"
            );
            assert_eq!(
                resolve_const_string(
                    text,
                    &uses,
                    &values,
                    &ssa,
                    FoldPolicy::default(),
                    Some(config)
                ),
                Some("one two".into()),
                "{profile}: {text}"
            );
            assert_eq!(
                resolve_simple_var_ref(text, &uses, &values, &ssa, None),
                None
            );
            assert_eq!(
                resolve_foreach_list_via_lattice(text, &uses, &values, &ssa, rules, None),
                None
            );
            assert_eq!(
                resolve_const_string(text, &uses, &values, &ssa, FoldPolicy::default(), None),
                None
            );
        }
        for text in ["$a:b", "$arr(key)", "${arr(key)}", "$a$b"] {
            assert_eq!(
                resolve_simple_var_ref(text, &uses, &values, &ssa, Some(config)),
                None,
                "{profile}: {text}"
            );
            assert_eq!(
                resolve_foreach_list_via_lattice(text, &uses, &values, &ssa, rules, Some(config)),
                None,
                "{profile}: {text}"
            );
            assert_eq!(
                resolve_const_string(
                    text,
                    &uses,
                    &values,
                    &ssa,
                    FoldPolicy::default(),
                    Some(config)
                ),
                None,
                "{profile}: {text}"
            );
        }
        assert_profile_reference_shapes(profile, config, rules, &uses, &values, &ssa);
    }
}

#[test]
fn resolve_simple_var_ref_accepts_bare_and_braced() {
    let mut ssa = bare_ssa();
    let x = ssa.intern_var("x");
    let mut uses = HashMap::new();
    uses.insert(x, 1);
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(7)));
    assert_eq!(
        resolve_simple_var_ref(
            "$x",
            &uses,
            &values,
            &ssa,
            Some(tcl_lexer::LexerConfig::default())
        ),
        Some(LatticeValue::Const(ConstValue::Int(7)))
    );
    assert_eq!(
        resolve_simple_var_ref(
            "${x}",
            &uses,
            &values,
            &ssa,
            Some(tcl_lexer::LexerConfig::default())
        ),
        Some(LatticeValue::Const(ConstValue::Int(7)))
    );
    assert_eq!(
        resolve_simple_var_ref(
            "$y",
            &uses,
            &values,
            &ssa,
            Some(tcl_lexer::LexerConfig::default())
        ),
        None
    );
    assert_eq!(
        resolve_simple_var_ref(
            "plain",
            &uses,
            &values,
            &ssa,
            Some(tcl_lexer::LexerConfig::default())
        ),
        None
    );
}

// Foreach constset extraction.

fn foreach_stmt(ssa: &mut SsaFunction, var: &str, list: &str, new_ver: u32) -> SsaStatement {
    let mut defs = HashMap::new();
    defs.insert(ssa.intern_var(var), new_ver);
    SsaStatement {
        statement: Statement::Call {
            span: Span::new(0, 0),
            command: "foreach".into(),
            canonical_command: None,
            args: vec![list.into()],
            defs: vec![var.into()],
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        },
        uses: HashMap::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

#[test]
fn extract_foreach_elements_literal_list() {
    // `list_text` is already delimiter-stripped by the segmenter, matching
    // the shape `foreach v {a b c}`'s `list_arg` is built in: the word's
    // own `{…}` is gone, leaving plain whitespace-separated text.
    assert_eq!(
        extract_foreach_elements("a b c", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a".into(), "b".into(), "c".into()])
    );
}

#[test]
fn extract_foreach_elements_rejects_substitutions() {
    assert_eq!(
        extract_foreach_elements("$lst", tcl_syntax::word_rules::WordValueRules::TCL),
        None
    );
    assert_eq!(
        extract_foreach_elements("[list a b c]", tcl_syntax::word_rules::WordValueRules::TCL),
        None
    );
}

#[test]
fn extract_foreach_elements_splits_nested_braces_as_tcl_list() {
    // `a {b c} d` is a three-element Tcl list — `a`, `b c`, `d` — not the
    // four whitespace runs `a`, `{b`, `c}`, `d`. A naive `split_ascii_whitespace`
    // corrupted the CONSTSET; the list-aware split fixes it.
    assert_eq!(
        extract_foreach_elements("a {b c} d", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a".into(), "b c".into(), "d".into()])
    );
    // Backslash-escaped whitespace groups an element too: `a\ b c` is two
    // elements `a b` and `c`.
    assert_eq!(
        extract_foreach_elements("a\\ b c", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a b".into(), "c".into()])
    );
}

#[test]
fn extract_foreach_elements_empty_list_returns_empty() {
    assert_eq!(
        extract_foreach_elements("", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(Vec::new())
    );
}

// `list_text` already has its outer
// `foreach`-word delimiter removed by the segmenter (see
// `Statement::Foreach::list_arg`'s construction in
// `lowering::structured`), so a value that itself starts with `{` / `"`
// is a *nested* list element, not a delimiter to peel a second time.
#[test]
fn extract_foreach_elements_single_element_braced_list_not_double_stripped() {
    // Source `foreach v {{a b c}}`: the segmenter strips the word's own
    // outer `{…}`, leaving the single-element list `{a b c}` as
    // `list_text`. Splitting it as a Tcl list yields the one element
    // `a b c`, not the three whitespace-separated words `a`, `b`, `c`
    // a second brace-strip-then-split would wrongly produce.
    assert_eq!(
        extract_foreach_elements("{a b c}", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a b c".into()])
    );
}

#[test]
fn extract_foreach_elements_two_braced_elements_not_double_stripped() {
    // Source `foreach v {{a b} {c d}}`: `list_text` is `{a b} {c d}`.
    // The pre-fix double-strip peeled the text's own outer `{`/`}` too,
    // yielding the corrupted split `a`, `b}`, `{c`, `d` (lenient split of
    // `a b} {c d`). The correct split is the two nested-list elements.
    assert_eq!(
        extract_foreach_elements("{a b} {c d}", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a b".into(), "c d".into()])
    );
}

#[test]
fn extract_foreach_elements_quoted_word_already_delimiter_stripped() {
    // Source `foreach v "a b c"`: the segmenter strips the word's own
    // `"…"` delimiters the same way it strips `{…}`, so `list_text` is
    // plain `a b c` — no quote handling is needed (or wanted) here.
    assert_eq!(
        extract_foreach_elements("a b c", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a".into(), "b".into(), "c".into()])
    );
}

#[test]
fn evaluate_def_foreach_literal_list_folds_constset() {
    // `list` mirrors `list_arg` as the segmenter hands it: source
    // `foreach v {1 2 3}`'s outer `{…}` is already stripped.
    let mut ssa = bare_ssa();
    let stmt = foreach_stmt(&mut ssa, "v", "1 2 3", 1);
    let result = evaluate_fixture_def(
        &stmt,
        &HashMap::new(),
        &ssa,
        FoldPolicy::default(),
        tcl_lexer::LexerConfig::default(),
    );
    match result {
        LatticeValue::ConstSet(ref vs) => {
            assert_eq!(vs.len(), 3);
            assert!(vs.contains(&ConstValue::Int(1)));
            assert!(vs.contains(&ConstValue::Int(3)));
        }
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn evaluate_def_foreach_list_cmd_subst_folds_constset() {
    // `foreach v [list a b c]` folds through `try_fold_cmd_subst` to the
    // same element CONSTSET as the braced-literal form.
    let mut ssa = bare_ssa();
    let stmt = foreach_stmt(&mut ssa, "v", "[list a b c]", 1);
    let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
    match result {
        LatticeValue::ConstSet(ref vs) => {
            assert_eq!(vs.len(), 3);
            assert!(vs.contains(&ConstValue::String("a".into())));
            assert!(vs.contains(&ConstValue::String("c".into())));
        }
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn evaluate_def_foreach_single_element_folds_const() {
    // `list` mirrors `list_arg` for source `foreach v {only}`: the
    // segmenter's delimiter strip leaves the bare word `only`.
    let mut ssa = bare_ssa();
    let stmt = foreach_stmt(&mut ssa, "v", "only", 1);
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::String("only".into()))
    );
}

#[test]
fn evaluate_def_foreach_nested_braced_elements_folds_constset() {
    // Source `foreach v {{a b} {c d}}` hands `list_arg` = `{a b} {c d}`
    // (only the word's own outer `{…}` is stripped by the segmenter).
    // Peeling a *second* level of bracing off this already
    // delimiter-stripped text would fold `v` to the corrupted CONSTSET
    // {"a", "b}"} instead of the two list elements.
    let mut ssa = bare_ssa();
    let stmt = foreach_stmt(&mut ssa, "v", "{a b} {c d}", 1);
    let result = evaluate_fixture_def(
        &stmt,
        &HashMap::new(),
        &ssa,
        FoldPolicy::default(),
        tcl_lexer::LexerConfig::default(),
    );
    match result {
        LatticeValue::ConstSet(ref vs) => {
            assert_eq!(vs.len(), 2);
            assert!(vs.contains(&ConstValue::String("a b".into())));
            assert!(vs.contains(&ConstValue::String("c d".into())));
            assert!(!vs.contains(&ConstValue::String("b}".into())));
        }
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn evaluate_def_foreach_via_lattice_var() {
    let mut ssa = bare_ssa();
    let mut stmt = foreach_stmt(&mut ssa, "v", "$lst", 1);
    let lst = ssa.intern_var("lst");
    stmt.uses.insert(lst, 1);
    let mut values = HashMap::new();
    values.insert(
        (lst, 1),
        LatticeValue::Const(ConstValue::String("a b c".into())),
    );
    let result = evaluate_fixture_def(
        &stmt,
        &values,
        &ssa,
        FoldPolicy::default(),
        tcl_lexer::LexerConfig::default(),
    );
    match result {
        LatticeValue::ConstSet(ref vs) => assert_eq!(vs.len(), 3),
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn evaluate_def_foreach_unbound_var_widens() {
    let mut ssa = bare_ssa();
    let mut stmt = foreach_stmt(&mut ssa, "v", "$lst", 1);
    let lst = ssa.intern_var("lst");
    stmt.uses.insert(lst, 1);
    // Empty lattice — var not bound.
    let result = evaluate_fixture_def(
        &stmt,
        &HashMap::new(),
        &ssa,
        FoldPolicy::default(),
        tcl_lexer::LexerConfig::default(),
    );
    assert_eq!(result, LatticeValue::Overdefined);
}

#[test]
fn evaluate_def_foreach_multi_var_widens() {
    // 2-element defs → no constset extraction.
    let mut ssa = bare_ssa();
    let mut stmt = foreach_stmt(&mut ssa, "v", "a b", 1);
    let Statement::Call { defs, .. } = &mut stmt.statement else {
        panic!();
    };
    defs.push("w".into());
    let result = evaluate_fixture_def(
        &stmt,
        &HashMap::new(),
        &ssa,
        FoldPolicy::default(),
        tcl_lexer::LexerConfig::default(),
    );
    assert_eq!(result, LatticeValue::Overdefined);
}

// AssignValue + command-substitution folding.

fn assign_value_stmt(ssa: &mut SsaFunction, name: &str, value: &str, ver: u32) -> SsaStatement {
    let mut defs = HashMap::new();
    defs.insert(ssa.intern_var(name), ver);
    SsaStatement {
        statement: Statement::AssignValue {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: false,
            value: value.into(),
            value_needs_backsubst: false,
            tokens: None,
        },
        uses: HashMap::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

#[test]
fn evaluate_def_assign_value_plain_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "hello", 1);
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::String("hello".into()))
    );
}

#[test]
fn evaluate_def_assign_value_integer_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "42", 1);
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(42))
    );
}

#[test]
fn evaluate_def_assign_value_resolves_var_ref() {
    let mut ssa = bare_ssa();
    let mut stmt = assign_value_stmt(&mut ssa, "y", "$x", 1);
    let x = ssa.intern_var("x");
    stmt.uses.insert(x, 1);
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(7)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(7))
    );
}

#[test]
fn evaluate_def_assign_value_folds_list_cmd() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[list a b c]", 1);
    let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
    match result {
        LatticeValue::Const(ConstValue::String(s)) => assert_eq!(s, "a b c"),
        other => panic!("expected Const(String), got {other:?}"),
    }
}

#[test]
fn evaluate_def_assign_value_folds_llength_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c d}]", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(4))
    );
}

#[test]
fn evaluate_def_assign_value_folds_string_length() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[string length \"hello\"]", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(5))
    );
}

// The fold arms answer to the command-binding trust fact.

/// The whole-module mutation summary for `src`.
fn mutations_for(src: &str) -> crate::command_binding::ModuleCommandMutations {
    let reg = registry();
    let ir = crate::lowering::lower_to_ir(src, &reg);
    crate::command_binding::scan_module_command_mutations(&ir, &reg)
}

/// Evaluate `stmt` under the whole-module trust fact `mutations`.
fn evaluate_under_unit(
    stmt: &SsaStatement,
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    mutations: &crate::command_binding::ModuleCommandMutations,
) -> LatticeValue {
    evaluate_fixture_def_with_folds(
        stmt,
        &HashMap::new(),
        ssa,
        FoldPolicy::default(),
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry,
            mutations,
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: FoldTrust::WholeModule,
            proven_pure_parameters: false,
        }),
        tcl_lexer::LexerConfig::default(),
    )
}

/// The `[list …]` / `[format …]` arms run ahead of the registry engine, so
/// they must apply its trust gate themselves — otherwise a unit that
/// renamed `list` still gets the builtin's answer.
///
/// tclsh 8.6.16 / 9.0.4 (identical): after `rename list mylist`, evaluating
/// `list a b c` raises rather than returning `a b c`.
#[test]
fn list_arm_declines_once_the_unit_renames_list() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[list a b c]", 1);

    let untouched = mutations_for("set y 1\n");
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &untouched),
        LatticeValue::Const(ConstValue::String("a b c".into())),
        "an untouched `list` still folds"
    );

    let renamed = mutations_for("rename list mylist\n");
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &renamed),
        LatticeValue::Overdefined,
        "a renamed `list` must not fold to the builtin's answer"
    );
}

/// The same gate, on an arm reached through the head-word check rather
/// than through its own early return — and per name: renaming `llength`
/// must not cost `list` its fold.
#[test]
fn head_word_arms_decline_only_for_the_name_the_unit_touched() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let count = assign_value_stmt(&mut ssa, "n", "[llength {a b c d}]", 1);
    let listing = assign_value_stmt(&mut ssa, "x", "[list a b c]", 2);

    let renamed = mutations_for("rename llength myll\n");
    assert_eq!(
        evaluate_under_unit(&count, &ssa, &reg, &renamed),
        LatticeValue::Overdefined,
        "a renamed `llength` must not fold to the builtin's answer"
    );
    assert_eq!(
        evaluate_under_unit(&listing, &ssa, &reg, &renamed),
        LatticeValue::Const(ConstValue::String("a b c".into())),
        "`list` is untouched by a `rename llength` and keeps its fold"
    );
}

/// Evaluate `stmt` under `mutations` with the shared per-unit lattice's
/// stance ([`FoldTrust::ObservedBindings`]).
fn evaluate_under_lattice_stance(
    stmt: &SsaStatement,
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    mutations: &crate::command_binding::ModuleCommandMutations,
) -> LatticeValue {
    evaluate_fixture_def_with_folds(
        stmt,
        &HashMap::new(),
        ssa,
        FoldPolicy::default(),
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry,
            mutations,
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: FoldTrust::ObservedBindings,
            proven_pure_parameters: false,
        }),
        tcl_lexer::LexerConfig::default(),
    )
}

/// A user `proc` shadowing a builtin is a *named* takeover, so both
/// stances decline it. This is the root of #2164: the shared per-unit
/// lattice took no trust fact at all and answered `3` for a
/// `[llength {a b c}]` the module resolves to a proc returning 99, which
/// the optimiser then published as `return 3` beside its own `set v 99`.
///
/// tclsh 8.4.20 / 8.5.19 / 8.6.18 / 9.0.4 / 9.1b0 (unanimous): with
/// `proc llength {l} {return 99}` in scope, `[llength {a b c}]` is 99.
#[test]
fn both_stances_decline_a_builtin_a_proc_shadows() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

    let shadowed = mutations_for("proc llength {l} { return 99 }\n");
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &shadowed),
        LatticeValue::Overdefined,
        "the lattice must not answer with builtin semantics for a shadowed name"
    );
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &shadowed),
        LatticeValue::Overdefined,
        "nor may the rewrite stance"
    );

    // Positive control: the same statement, the same two stances, a module
    // that shadows nothing — both fold, so the declines above are the
    // shadow's doing and not a dead arm.
    let untouched = mutations_for("set y 1\n");
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &untouched),
        LatticeValue::Const(ConstValue::Int(3)),
    );
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &untouched),
        LatticeValue::Const(ConstValue::Int(3)),
    );
}

/// Where the two stances deliberately differ: a command head an autoload
/// index in view may define raises the summary's unbounded `dynamic` top,
/// which names no subject. The rewrite stance declines on it (a rewrite
/// must be certain); the lattice stance keeps folding, or a single
/// unknown library call would cost a file every constant it has.
#[test]
fn only_the_rewrite_stance_declines_on_the_unbounded_top() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

    let opaque = mutations_for("someUnknownLibraryCall x\n");
    assert!(
        opaque.has_dynamic_mutation(),
        "an unresolved head is expected to raise the unbounded top"
    );
    assert!(
        opaque.observed_binding_is_the_builtin("llength"),
        "but it names no subject, so `llength`'s observed binding is intact"
    );
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &opaque),
        LatticeValue::Const(ConstValue::Int(3)),
    );
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &opaque),
        LatticeValue::Overdefined,
        "a fold that becomes a rewrite declines on the unbounded top"
    );
}

/// A `rename` whose **subject** the scan cannot name distrusts every
/// builtin, where an unresolved command *head* does not (#2168).
///
/// Both raise the unbounded top, which is why gating the lattice on the
/// whole of it was rejected in #2164 — it would take the fold in
/// [`only_the_rewrite_stance_declines_on_the_unbounded_top`] with it. The
/// distinction is whether something was definitely rebound: `rename $a {}`
/// moved *some* command, so no name is claimable; `someUnknownLibraryCall
/// x` moved nothing.
///
/// tclsh 8.6.18 and 9.0.4 both print 99 for the repro on #2168, where the
/// optimiser rewrote the body to `return 3` — and its own output literally
/// read `rename llength {}`, having constant-propagated the operands in
/// the same run.
#[test]
fn an_unnameable_rename_subject_distrusts_where_an_unknown_head_does_not() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

    let computed = mutations_for("rename $a {}\n");
    assert!(
        computed.has_dynamic_mutation(),
        "an unresolved rename subject raises the unbounded top"
    );
    assert!(
        !computed.observed_binding_is_the_builtin("llength"),
        "and, unlike an unknown head, it leaves no name claimable"
    );
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &computed),
        LatticeValue::Overdefined,
        "the lattice must not fold a builtin a computed rename may have moved"
    );

    // A value actually established by the shared source owner resolves
    // the same operand to an exact mutation, without widening unrelated names.
    let exact = mutations_for("set a llength\nrename $a {}\n");
    assert!(
        !exact.has_dynamic_mutation(),
        "resolved subject is an exact rename"
    );
    assert!(!exact.observed_binding_is_the_builtin("llength"));
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &exact),
        LatticeValue::Overdefined
    );

    // Positive control, and the coverage #2164 measured: an unresolved
    // head still raises the top yet still folds, so this change cannot
    // have been bought by gating on `dynamic`.
    let opaque = mutations_for(
        "someUnknownLibraryCall x
",
    );
    assert!(opaque.has_dynamic_mutation());
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &opaque),
        LatticeValue::Const(ConstValue::Int(3)),
    );
}

/// The same rule for a body the source-recursive walk never sees. A proc
/// installed through an alias prefix is recovered only by the closed
/// command lattice, so its unnameable rename subject reaches the optimiser
/// through `mutation_projection` or not at all (#2168).
///
/// The renamed implementation returns **99** on tclsh 8.6.18 and 9.0.4;
/// retaining the stock `llength` result **3** would change its result.
#[test]
fn an_unnameable_rename_subject_inside_an_alias_defined_body_also_distrusts() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

    // `makep {} BODY` is `proc p {} BODY`; BODY is never walked as source.
    let aliased = mutations_for(
        "proc mylen {l} { return 99 }
interp alias {} makep {} proc p
makep {} {set a llength; set b mylen; rename $a {}; rename $b $a}
p
",
    );
    assert!(
        !aliased.observed_binding_is_the_builtin("llength"),
        "the projection is the only witness that the subject was unnameable"
    );
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &aliased),
        LatticeValue::Overdefined,
        "folding here rewrites a program meaning 99 into one meaning 3"
    );
}

#[test]
fn string_length_fold_counts_in_the_selected_dialects_character_model() {
    // U+1D11E is one Tcl 9 scalar but two Tcl 8 `Tcl_UniChar` units, so the
    // compile-time fold must answer as the selected runtime would — and
    // decline when no release is selected, leaving the width ambiguous.
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[string length \"\u{1D11E}\"]", 1);
    let fold = |dialect: Option<&'static tcl_dialect::DialectProfile>| {
        evaluate_pristine(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::for_profile(Some(false), dialect),
        )
    };
    assert_eq!(
        fold(Some(
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
        )),
        LatticeValue::Const(ConstValue::Int(1))
    );
    assert_eq!(
        fold(Some(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
        )),
        LatticeValue::Const(ConstValue::Int(2))
    );
    assert_eq!(
        fold(None),
        LatticeValue::Overdefined,
        "no selected release leaves a supplementary width ambiguous"
    );

    // A string both models count identically still folds with no selected
    // release — declining those would drop an ordinary optimisation.
    let mut ssa = bare_ssa();
    let ascii = assign_value_stmt(&mut ssa, "n", "[string length \"hello\"]", 1);
    assert_eq!(
        evaluate_pristine(
            &ascii,
            &HashMap::new(),
            &ssa,
            FoldPolicy::for_profile(Some(false), None)
        ),
        LatticeValue::Const(ConstValue::Int(5))
    );
}

#[test]
fn evaluate_def_assign_value_folds_expr_cmd_subst() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[expr {1 + 2}]", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(3))
    );
}

#[test]
fn evaluate_def_assign_value_folds_format_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "s", "[format \"%d-%d\" 1 2]", 1);
    match evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()) {
        LatticeValue::Const(ConstValue::String(s)) => assert_eq!(s, "1-2"),
        other => panic!("expected Const(String), got {other:?}"),
    }
}

#[test]
fn quoted_expr_with_string_var_does_not_fold() {
    // `set r [expr "$a == $b"]` with a="alpha", b="beta": Tcl substitutes
    // the values textually before parsing, so `expr "alpha == beta"`
    // errors (`invalid bareword`). The fold must bail rather than treat
    // the strings as operands and return 0.
    let mut ssa = bare_ssa();
    let mut stmt = assign_value_stmt(&mut ssa, "r", "[expr \"$a == $b\"]", 1);
    let a = ssa.intern_var("a");
    let b = ssa.intern_var("b");
    stmt.uses.insert(a, 1);
    stmt.uses.insert(b, 1);
    let mut values = HashMap::new();
    values.insert(
        (a, 1),
        LatticeValue::Const(ConstValue::String("alpha".into())),
    );
    values.insert(
        (b, 1),
        LatticeValue::Const(ConstValue::String("beta".into())),
    );
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Overdefined
    );
}

#[test]
fn quoted_expr_with_numeric_var_retains_known_contents() {
    let unit = cu(r#"set a 3; set b 4; set r [expr "$a + $b"]"#);
    let facts = unit.top_level.semantic_values().unwrap();
    let known_seven = facts
        .contents_iter()
        .any(|(_, value)| *value == LatticeValue::Const(ConstValue::Int(7)));
    if !known_seven {
        eprintln!(
            "quoted expr statement kinds={:?} failure={} admission={}",
            unit.ir_module
                .top_level
                .statements
                .iter()
                .map(std::mem::discriminant)
                .collect::<Vec<_>>(),
            unit.ir_module
                .top_level
                .native_compilation_failure
                .is_some(),
            unit.ir_module
                .top_level
                .native_compilation_admission
                .is_some()
        );
        let show = |binding: &crate::command_binding::SourceInvocationBinding| {
            format!(
                "targets={:?} unknown={} absent={} residual={:?} handler={:?} bindings={} traces={} argv={:?} compiled={:?}",
                binding
                    .targets
                    .iter()
                    .map(|target| (&target.command, target.registry_backed))
                    .collect::<Vec<_>>(),
                binding.unknown,
                binding.may_be_absent,
                binding.compiled_execution_residual,
                binding
                    .native_handler_envelope
                    .as_ref()
                    .map(|target| &target.command),
                binding.variable_context.dynamic_bindings,
                binding.variable_context.dynamic_traces,
                binding.evaluated_argument_values,
                binding
                    .compiled_candidates
                    .iter()
                    .map(|proof| (proof.operation, proof.certainty))
                    .collect::<Vec<_>>()
            )
        };
        for site in unit.ir_module.top_level.command_binding_sites.iter() {
            if let Some(tokens) = &site.source_tokens {
                eprintln!(
                    "quoted expr point {:?} argv={:?} proof={:?}",
                    site.span,
                    tokens.argv_texts,
                    tokens.source_binding.as_ref().map(show)
                );
                for (offset, binding) in &tokens.nested_bindings {
                    eprintln!("quoted expr nested {offset} {}", show(binding));
                }
            }
        }
        for proof in &unit.top_level.cfg.expression_preparations {
            eprintln!(
                "quoted expr preparation site={} base={} bytes={:?} axes={:?}",
                proof.invocation.offset,
                proof.source.base(),
                proof.witness.source(),
                proof.witness.context()
            );
        }
    }
    assert!(
        known_seven,
        "textual substitution computes native contents 7: {facts:#?}"
    );
    assert!(
        !unit
            .top_level
            .sccp
            .values
            .values()
            .any(|value| { *value == LatticeValue::Const(ConstValue::Int(7)) }),
        "unproved preparation/producer effects do not license erasure"
    );
}

#[test]
fn materialised_condition_values_do_not_license_branch_erasure() {
    let unit = cu("set n 1; if $n {set result YES} else {set result NO}");
    let function = &unit.top_level;
    let known = function.semantic_values().unwrap();
    assert!(
        known
            .result
            .constant_branches
            .iter()
            .any(|branch| { branch.condition == "$n" && branch.value })
    );
    assert_eq!(function.sccp.constant_branches, [] as [ConstantBranch; 0]);
    let analysis = known
        .expression_iter()
        .find_map(|(point, analysis)| {
            matches!(point, ExpressionEvaluationPoint::Branch { .. }).then_some(analysis)
        })
        .expect("the reached condition retains its actual preparation");
    assert_eq!(analysis.required_expression_preparations.len(), 1);
    assert_eq!(
        analysis.required_expression_preparations[0]
            .witness
            .source(),
        "1"
    );
}

#[test]
fn braced_expr_with_string_var_retains_numeric_comparison_obligations() {
    let unit = cu("set a alpha; set b beta; set r [expr {$a == $b}]");
    let facts = unit.top_level.semantic_values().unwrap();
    let analysis = facts
        .expression_iter()
        .find_map(|(_, analysis)| {
            (analysis.evaluation.value == TclValue::Int(0)).then_some(analysis)
        })
        .expect("the actual prepared string comparison computes false");
    assert_ne!(
        analysis.evaluation.coercions,
        [] as [crate::tcl_expr_eval::NativeCoercionObligation; 0]
    );
    assert!(
        !unit
            .top_level
            .sccp
            .values
            .values()
            .any(|value| { *value == LatticeValue::Const(ConstValue::Int(0)) }),
        "numeric comparison may change shared string representations"
    );
}

#[test]
fn evaluate_def_assign_value_unknown_cmd_widens() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[nonexistent_fold args]", 1);
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Overdefined
    );
}

#[test]
fn evaluate_def_assign_value_llength_via_lattice_var() {
    let mut ssa = bare_ssa();
    let mut stmt = assign_value_stmt(&mut ssa, "n", "[llength $lst]", 1);
    let lst = ssa.intern_var("lst");
    stmt.uses.insert(lst, 1);
    let mut values = HashMap::new();
    values.insert(
        (lst, 1),
        LatticeValue::Const(ConstValue::String("a b c".into())),
    );
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(3))
    );
}

#[test]
fn split_head_basic() {
    assert_eq!(split_head("cmd arg1 arg2"), ("cmd", Some("arg1 arg2")));
    assert_eq!(split_head("  cmd"), ("cmd", None));
    assert_eq!(split_head(""), ("", None));
}

#[test]
fn strip_one_level_braces_and_quotes() {
    assert_eq!(strip_one_level("{abc}"), "abc");
    assert_eq!(strip_one_level("\"abc\""), "abc");
    assert_eq!(strip_one_level("bare"), "bare");
    assert_eq!(strip_one_level("{}"), "");
}

#[test]
fn parse_literal_value_prefers_int() {
    assert_eq!(parse_literal_value("42"), ConstValue::Int(42));
    assert_eq!(parse_literal_value("-5"), ConstValue::Int(-5));
    assert_eq!(parse_literal_value("0"), ConstValue::Int(0));
    assert_eq!(
        parse_literal_value("hello"),
        ConstValue::String("hello".into())
    );
    // Leading-zero and non-canonical integer forms do not round-trip, so
    // they stay strings (lets SCCP apply the per-dialect leading-zero rule).
    assert_eq!(parse_literal_value("08"), ConstValue::String("08".into()));
    assert_eq!(parse_literal_value("010"), ConstValue::String("010".into()));
    assert_eq!(parse_literal_value("+5"), ConstValue::String("+5".into()));
    assert_eq!(parse_literal_value("-0"), ConstValue::String("-0".into()));
}

#[test]
fn cfg_order_appends_unreachable_blocks() {
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    let dead = block(&mut f, "dead");
    f.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    f.blocks.get_mut(&dead).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let order = cfg_order(&f);
    assert!(order.contains(&entry));
    assert!(order.contains(&dead));
}

fn cu(src: &str) -> crate::compilation_unit::CompilationUnit {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    crate::compilation_unit::CompilationUnit::build_for_profile(src, &registry, false, profile)
}

#[test]
fn sccp_folds_post_loop_branch_via_static_summary() {
    // After `for {set i 0} {$i < 10} {incr i} {}` tclsh leaves `i == 10`,
    // so the following `if {$i == 10}` is statically true. SCCP cannot fold
    // a loop-carried phi, but the static-loop summary simulates the loop
    // and folds the branch. Verified against tclsh 8.4-9.0 (i == 10, and
    // the accumulator j == 5, hold after the loops).
    let c = cu(
        "proc ::p {} { for {set i 0} {$i < 10} {incr i} {}\n if {$i == 10} { return yes } else { return no } }",
    );
    let fu = c.function("::p").unwrap();
    let r = &fu.semantic_values().unwrap().result;
    let cb = r
            .constant_branches
            .iter()
            .find(|cb| cb.condition.contains("$i == 10"))
            .unwrap_or_else(|| {
                panic!(
                    "post-loop branch must fold via the static-loop summary: facts={:#?} loops={:#?} preparations={:#?} sources={:#?}",
                    fu.semantic_values(),
                    fu.cfg.loop_nodes,
                    fu.cfg.expression_preparations,
                    fu.cfg.terminator_sources
                )
            });
    assert!(cb.value, "i == 10 after the loop, so the branch is true");
    let facts = fu.semantic_values().unwrap();
    assert!(
        facts.loops.values().any(|analysis| {
            !analysis.expression_obligations.is_empty()
                && !analysis.increment_obligations.is_empty()
        }),
        "semantic loop contents retain native operations in evaluation order"
    );
    assert!(
        !fu.sccp
            .constant_branches
            .iter()
            .any(|branch| branch.condition.contains("$i == 10")),
        "numeric simulation does not erase loop or branch conversions"
    );

    // Body side effects are simulated too: j accumulates to 5.
    let ca = cu(
        "proc ::a {} { set j 0\n for {set k 5} {$k > 0} {incr k -1} { incr j }\n if {$j == 5} { return yes } else { return no } }",
    );
    let fa = ca.function("::a").unwrap();
    let ra = &fa.semantic_values().unwrap().result;
    let cba = ra
        .constant_branches
        .iter()
        .find(|cb| cb.condition.contains("$j == 5"))
        .expect("accumulator branch must fold via the static-loop summary");
    assert!(cba.value, "j == 5 after the loop");
    assert!(
        !fa.sccp
            .constant_branches
            .iter()
            .any(|branch| branch.condition.contains("$j == 5")),
        "accumulator precision is not execution-erasure proof"
    );

    // A loop with an unknown (parameter) bound cannot be summarised, so the
    // post-loop branch stays unfolded (conservative).
    let cq = cu(
        "proc ::q {n} { for {set i 0} {$i < $n} {incr i} {}\n if {$i == 10} { return yes } else { return no } }",
    );
    let fq = cq.function("::q").unwrap();
    let rq = &fq.semantic_values().unwrap().result;
    assert!(
        !rq.constant_branches
            .iter()
            .any(|cb| cb.condition.contains("$i == 10")),
        "an unknown loop bound must not fold the post-loop branch"
    );
}

#[test]
fn sccp_foreach_nested_braced_list_constset_not_corrupted() {
    // End to end: `foreach v {{a b} {c d}}` lowers `list_arg` to the
    // segmenter-stripped text `{a b} {c d}` (only the word's own outer
    // braces are gone). Peeling a *second* level of bracing off this
    // already-stripped text corrupts `v`'s CONSTSET to the two elements
    // `a` and `b}` (from the lenient split of `a b} {c d`) and wrongly
    // proves `if {$v eq "c d"}` false. It must fold to the two correct
    // elements `a b` and `c d` instead.
    let c = cu(
        "proc ::p {} { foreach v {{a b} {c d}} { if {$v eq \"c d\"} { set r yes } else { set r no } } }",
    );
    let fu = c.function("::p").unwrap();
    let r = sccp_no_traces(&fu.cfg, &fu.ssa, None, FoldPolicy::default());
    let const_sets: Vec<_> = r
        .values
        .iter()
        .filter(|((symbol, _), _)| fu.ssa.var_name(*symbol) == "v")
        .filter_map(|(_, val)| match val {
            LatticeValue::ConstSet(vs) => Some(vs.clone()),
            _ => None,
        })
        .collect();
    assert!(
        !const_sets
            .iter()
            .any(|vs| vs.contains(&ConstValue::String("b}".into()))),
        "v's CONSTSET must not contain the corrupted element \"b}}\": {const_sets:?}"
    );
    assert!(
        const_sets.iter().any(|vs| vs.len() == 2
            && vs.contains(&ConstValue::String("a b".into()))
            && vs.contains(&ConstValue::String("c d".into()))),
        "expected a CONSTSET {{\"a b\", \"c d\"}} among v's lattice values, got {const_sets:?}"
    );
}

#[test]
fn branch_deferrable_optimism() {
    let cond = ExprNode::Var {
        text: "$x".into(),
        name: "x".into(),
        start: 0,
        end: 2,
    };
    let mut ssa = bare_ssa();
    let x = ssa.intern_var("x");
    let mut sb = empty_ssa_block("b");
    sb.exit_versions.insert(x, 1);
    let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();

    // Defined operand (version 1) not yet computed → defer.
    assert!(branch_deferrable(
        &sb,
        &cond,
        &values,
        &ssa,
        tcl_dialect::LexerGrammar::default()
    ));
    values.insert((x, 1), LatticeValue::Unknown);
    assert!(branch_deferrable(
        &sb,
        &cond,
        &values,
        &ssa,
        tcl_dialect::LexerGrammar::default()
    ));

    // An `Overdefined` operand proves the condition genuinely
    // non-constant → never defer.
    values.insert((x, 1), LatticeValue::Overdefined);
    assert!(!branch_deferrable(
        &sb,
        &cond,
        &values,
        &ssa,
        tcl_dialect::LexerGrammar::default()
    ));

    // A constant operand folds via `evaluate_branch`, so the `None`
    // arm is never reached → not deferrable here.
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(1)));
    assert!(!branch_deferrable(
        &sb,
        &cond,
        &values,
        &ssa,
        tcl_dialect::LexerGrammar::default()
    ));

    // Version-0 operands (parameters / globals / live-in roots) are
    // already `Overdefined` and excluded from the deferral test.
    let mut sb0 = empty_ssa_block("b");
    sb0.exit_versions.insert(x, 0);
    assert!(!branch_deferrable(
        &sb0,
        &cond,
        &HashMap::new(),
        &ssa,
        tcl_dialect::LexerGrammar::default()
    ));
}

#[test]
fn canonical_root_cells_fold_until_a_proved_mutation_dependency() {
    let stable = cu("set a 3; set b 4; if {$a+$b == 7} {set result YES} else {set result NO}");
    assert!(
        !stable
            .top_level
            .semantic_values()
            .unwrap()
            .result
            .constant_branches
            .is_empty(),
        "root-qualified identity alone does not imply mutation; values={:?}; cells={:?}",
        stable.top_level.sccp.values,
        stable.top_level.ssa.cell_names()
    );
    assert!(
        stable.top_level.sccp.constant_branches.is_empty(),
        "known branch values do not prove retained coercion erasure"
    );
    let observed = cu(
        "set a 3; proc observer {args} {set ::a 4}; trace add variable a read observer; if {$a == 3} {set result YES} else {set result NO}",
    );
    assert!(
        observed
            .top_level
            .semantic_values()
            .unwrap()
            .result
            .constant_branches
            .is_empty(),
        "a read observer may mutate the stored value"
    );
    let mutated = cu(
        "set a 3; proc mutate {} {global a; set a 4}; mutate; if {$a == 3} {set result YES} else {set result NO}",
    );
    assert!(
        !mutated
            .top_level
            .semantic_values()
            .unwrap()
            .result
            .constant_branches
            .iter()
            .any(|branch| branch.value),
        "re-entry must never reuse the stale root value"
    );
}

#[test]
fn positioned_constants_do_not_cross_unrepresented_mutations_or_observers() {
    for source in [
        "proc f {cmd} {set x 3; $cmd; if {$x == 3} {return YES} else {return NO}}",
        "proc f {n} {set x 3; set $n 4; if {$x == 3} {return YES} else {return NO}}",
        "proc f {n} {set x 3; unset $n; if {$x == 3} {return YES} else {return NO}}",
        "proc f {n} {set x 3; trace add variable $n read observer; if {$x == 3} {return YES} else {return NO}}",
    ] {
        let unit = cu(source);
        let function = unit.function("::f").expect("procedure retained");
        assert!(
            function
                .sccp
                .constant_branches
                .iter()
                .all(|branch| !branch.value),
            "an uncertain later read must not reuse the earlier value: {source}; branches={:?}",
            function.sccp.constant_branches,
        );
    }
}

#[test]
fn later_opaque_effects_do_not_destroy_an_earlier_branch_proof() {
    let unit = cu(
        "proc f {cmd} {set x 3; if {$x == 3} {set first YES} else {set first NO}; $cmd; if {$x == 4} {set second YES} else {set second NO}}",
    );
    let function = unit.function("::f").expect("procedure retained");
    assert!(
        function.sccp.constant_branches.is_empty(),
        "unknown retained conversions prevent execution erasure"
    );
    assert!(
        function
            .semantic_values()
            .unwrap()
            .result
            .constant_branches
            .iter()
            .any(|branch| branch.value && branch.condition.contains('3')),
        "the earlier read remains precise: {:?}",
        function.semantic_values().unwrap().result.constant_branches
    );
    assert!(
        !function
            .semantic_values()
            .unwrap()
            .result
            .constant_branches
            .iter()
            .any(|branch| branch.value && branch.condition.contains('4')),
        "the later read has an unknown contents origin: {:?}",
        function.semantic_values().unwrap().result.constant_branches
    );
}

#[test]
fn global_alias_contents_are_precise_until_an_actual_reentry_dependency() {
    // Both controls are independently executed in the six-engine temporal
    // vector corpus. Sharing a cell alone does not change its contents.
    let direct = cu("proc p {} {global g; set g 5; if {$g == 5} {return YES} else {return NO}}; p");
    let function = direct.function("::p").unwrap();
    assert!(
        function.sccp.constant_branches.is_empty(),
        "alias contents alone do not erase numeric read conversions"
    );
    assert!(
        function
            .semantic_values()
            .unwrap()
            .result
            .constant_branches
            .iter()
            .any(|branch| branch.value),
        "an unobserved literal store followed immediately by its read is precise"
    );

    let mutated = cu(
        "proc mutate {} {global g; set g 6}; proc p {} {global g; set g 5; mutate; if {$g == 5} {return YES} else {return NO}}; p",
    );
    let function = mutated.function("::p").unwrap();
    assert!(
        !function
            .semantic_values()
            .unwrap()
            .result
            .constant_branches
            .iter()
            .any(|branch| branch.value),
        "the alias read must use the contents after reentry"
    );
}

/// Regression: a literal-body `uplevel #0 {…}` (the CFG shape `Statement::
/// UpFrame`) evaluates its body in the absolute global frame, which can
/// reassign any name visible there — including one with no `global`/
/// `variable`/`upvar`/`trace` declaration at all. SCCP must widen every
/// tracked value across it exactly as it already does for a plain
/// `Statement::Barrier`. Confirmed against tclsh 8.6/9.0: `set n 5;
/// uplevel #0 {set n 99}; if {$n == 5} {…}` takes the *else* branch
/// (`n` is 99), so SCCP must not fold this to a constant-true branch.
#[test]
fn sccp_widens_across_upframe_from_literal_uplevel() {
    let with_upframe =
        cu("set n 5\nuplevel #0 { set n 99 }\nif {$n == 5} { set r yes } else { set r no }\n");
    let f = with_upframe.function("::top").unwrap();
    let r = sccp_no_traces(&f.cfg, &f.ssa, None, FoldPolicy::default());
    assert_eq!(
        r.constant_branches.len(),
        0,
        "a value reachable through an UpFrame must not fold a constant branch, got {:?}",
        r.constant_branches,
    );
}
