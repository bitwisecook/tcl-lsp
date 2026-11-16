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

//! The semantic-value projection: lazy construction, retained math dependencies, native result protocols and the expression environment at a point.

use super::*;

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

#[test]
fn point_expression_environment_preserves_alias_spelling_and_retargeting() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let source = include_str!(
        "../../../../../tcl-syntax/tests/data/resolution/alias-retarget-expression.tcl"
    );
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
