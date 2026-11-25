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
use crate::expr_parser::parse_expr;

fn selected_analysis_policy() -> FoldPolicy {
    let supplied = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = supplied.commands();
    let profile = registry.profile().unwrap();
    FoldPolicy::for_retained_entry(
        registry,
        Some(tcl_registry::InvocationDialect::of_profile(profile)),
        &tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    )
}
use tcl_lexer::Span;

fn sp() -> Span {
    Span::new(0, 0)
}

fn empty_script() -> Script {
    Script::new()
}

fn script_of(stmts: Vec<Statement>) -> Script {
    let mut s = Script::new();
    for st in stmts {
        s.statements.push(st);
    }
    s
}

fn assign_const(name: &str, value: &str) -> Statement {
    Statement::AssignConst {
        span: sp(),
        name: name.into(),
        name_braced: false,
        value: value.into(),
        value_span: None,
    }
}

fn incr(name: &str, amount: Option<&str>) -> Statement {
    Statement::Incr {
        span: sp(),
        name: name.into(),
        name_braced: false,
        amount: amount.map(String::from),
        amount_braced: false,
        safe_on_uninit: false,
    }
}

fn analyse_static_for(
    init: &Script,
    condition: &ExprNode,
    next: &Script,
    body: &Script,
    initial: &StaticEnv,
    max_iterations: u64,
    policy: FoldPolicy,
) -> Option<StaticEnv> {
    summarise_static_for_selected(
        init,
        StaticForCondition {
            source: None,
            purpose: SimulationPurpose::Analysis,
            expression: condition,
            bindings: None,
        },
        next,
        body,
        initial,
        max_iterations,
        policy,
    )
    .map(|analysis| analysis.values)
}

#[test]
fn selected_operand_contents_keep_the_complete_scalar_reference_grammar() {
    use tcl_syntax::expr::parser::{CheckedExprParse, parse_expr_checked_with_context};
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let environment =
            tcl_registry::model::ingress::resolve_known_environment(environment).unwrap();
        let context = environment.default_context_registry();
        let profile = environment.unit_profile();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let policy = FoldPolicy::for_retained_entry(
            context.commands(),
            Some(tcl_registry::InvocationDialect::of_profile(profile)),
            &config,
        );
        let parse_context = policy.preparation_context().unwrap();
        for name in ["$literal", "[literal]", "with space", " with space "] {
            let source = format!("1?${{{name}}}:0");
            let CheckedExprParse::Parsed(expression) =
                parse_expr_checked_with_context(&source, &parse_context)
            else {
                panic!("fixed complete scalar expression: {source}");
            };
            let initial = StaticEnv::from([(name.to_owned(), StaticValue::Int(0))]);
            let empty = empty_script();
            let summary = summarise_static_for_selected(
                &empty,
                StaticForCondition {
                    source: None,
                    purpose: SimulationPurpose::Analysis,
                    expression: &expression,
                    bindings: None,
                },
                &empty,
                &empty,
                &initial,
                1,
                policy,
            )
            .expect("the selected zero operand ends the loop without changing its name");
            assert_eq!(summary.values.get(name), Some(&StaticValue::Int(0)));
            if policy.native_family == Some(tcl_dialect::model::Family::Jim) {
                let evaluated = crate::tcl_expr_eval::analyse_tcl_expr_with_resolved_math_bindings(
                    &expression,
                    &env_as_tcl_env(&initial),
                    policy,
                    &|_, _| None,
                    None,
                )
                .unwrap();
                assert!(static_expression_contents(&evaluated, &initial, None).is_none());
                assert!(summary.expression_obligations.iter().any(|obligation| matches!(
                        &obligation.result_dependency,
                        Some(crate::tcl_expr_eval::NativeExpressionResultDependency::SelectedOperand { reference, existing_bytes: None, .. })
                        if reference == &format!("${{{name}}}")
                    )));
            }
        }
    }
}

#[test]
fn positioned_simulation_requires_matching_whole_expression_preparation() {
    use crate::command_binding::{
        CommandAllocationSite, ExecutedScriptSource, SourceExpressionPreparation, SourceOriginId,
    };
    use crate::math_function_binding::ExpressionMathBindings;
    use std::sync::Arc;
    use tcl_registry::runtime_expr_validation::{
        ExpressionPreparationProof, prepare_expression_witness,
    };
    let origin = Arc::new(SourceOriginId::authored(&Arc::from("0")));
    let source = ExecutedScriptSource::contiguous(Arc::clone(&origin), "0", 0).unwrap();
    let policy = FoldPolicy::default().with_invocation_dialect(
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1),
    );
    let ExpressionPreparationProof::Prepared(witness) =
        prepare_expression_witness("0", &policy.preparation_context().unwrap(), None)
    else {
        panic!("literal preparation succeeds")
    };
    let preparation = SourceExpressionPreparation {
        namespace_key: crate::command_binding::SourceNamespaceKey::authored("::"),
        invocation: CommandAllocationSite {
            source: origin,
            offset: 0,
        },
        source: Arc::new(source.clone()),
        witness: Arc::from(witness),
        script_compilation: None,
        executed_expression: None,
    };
    let expression = parse_expr("0", None);
    let positioned = ExpressionMathBindings::for_origin(&[], Some(&source), Some(0));
    assert_eq!(
        evaluate_expr_with_constants_and_math_bindings(
            &expression,
            &StaticEnv::new(),
            policy,
            Some(positioned),
        ),
        None
    );
    let inventory = [preparation.clone()];
    let positioned = positioned.with_preparations(&inventory);
    let summary = summarise_static_for_selected(
        &empty_script(),
        StaticForCondition {
            source: None,
            purpose: SimulationPurpose::Execution,
            expression: &expression,
            bindings: Some(positioned),
        },
        &empty_script(),
        &empty_script(),
        &StaticEnv::new(),
        10,
        policy,
    )
    .unwrap();
    assert_eq!(summary.required_expression_preparations, vec![preparation]);
    assert_eq!(
        summary.required_math_invocations,
        [] as [crate::command_binding::SourceMathInvocation; 0]
    );
    let other_engine = FoldPolicy::default().with_invocation_dialect(
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4),
    );
    assert_eq!(
        evaluate_expr_with_constants_and_math_bindings(
            &expression,
            &StaticEnv::new(),
            other_engine,
            Some(positioned),
        ),
        None
    );
}

#[test]
fn semantic_loop_values_retain_each_native_conversion() {
    let condition = parse_expr("$i < 10", None);
    let statement = Statement::For {
        span: sp(),
        init: script_of(vec![assign_const("i", "0"), assign_const("j", "0")]),
        init_span: sp(),
        condition,
        condition_span: sp(),
        next: script_of(vec![incr("i", None)]),
        next_span: sp(),
        body: script_of(vec![Statement::If {
            span: sp(),
            clauses: vec![IfClause {
                condition: parse_expr("$i < 5", None),
                condition_span: sp(),
                condition_base: None,
                body: script_of(vec![incr("j", None)]),
                body_span: sp(),
            }],
            else_body: None,
            else_span: None,
        }]),
        body_span: sp(),
        raw_args: Vec::new(),
        raw_tokens: None,
        condition_base: None,
    };
    let policy = FoldPolicy::default().with_invocation_dialect(
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1),
    );
    assert!(
        summarise_for_statement_with_dependencies(&statement, &StaticEnv::new(), 20, policy, None,)
            .is_none()
    );
    let analysis =
        analyse_for_statement_with_dependencies(&statement, &StaticEnv::new(), 20, policy, None)
            .expect("semantic values retain native evaluation");
    assert_eq!(analysis.values.get("i"), Some(&StaticValue::Int(10)));
    assert_eq!(analysis.values.get("j"), Some(&StaticValue::Int(5)));
    assert_eq!(analysis.increment_obligations.len(), 15);
    assert_eq!(analysis.expression_obligations.len(), 21);
    assert!(
        analysis
            .expression_obligations
            .iter()
            .all(|obligation| !obligation.coercions.is_empty())
    );
    let mut orders: Vec<_> = analysis
        .expression_obligations
        .iter()
        .map(|obligation| obligation.evaluation_order)
        .chain(
            analysis
                .increment_obligations
                .iter()
                .map(|obligation| obligation.evaluation_order),
        )
        .collect();
    orders.sort_unstable();
    assert_eq!(orders, (0..36).collect::<Vec<_>>());
}

#[test]
fn original_assignment_values_use_whole_words_without_promoting_literal_data() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        for (value, expected) in [
            ("$input", Some("VALUE")),
            ("\"$input\"", Some("VALUE")),
            (r"\$input", Some("$input")),
            ("{$input}", Some("$input")),
            ("\"$input tail\"", None),
            ("$input(key)", None),
            ("[unknown_command]", None),
        ] {
            let source = format!("set input VALUE; set result {value}");
            let fixture = crate::provider_fixtures::original_native_module(&source, dialect);
            let (context, module) = &*fixture;
            let policy = FoldPolicy::for_retained_entry(
                context.commands(),
                Some(tcl_registry::InvocationDialect::of_profile(
                    module
                        .source_metadata_input
                        .as_ref()
                        .unwrap()
                        .unit_profile(),
                )),
                &module.native_lexer_config(),
            );
            let observations = std::cell::RefCell::new(Vec::new());
            let increments = std::cell::RefCell::new(Vec::new());
            let preparations = std::cell::RefCell::new(Vec::new());
            let dependencies = std::cell::RefCell::new(Vec::new());
            let evaluation_order = std::cell::Cell::new(0);
            let simulation = StaticSimulation {
                source: None,
                policy,
                purpose: SimulationPurpose::Analysis,
                observations: &observations,
                increments: &increments,
                preparations: &preparations,
                dependencies: &dependencies,
                evaluation_order: &evaluation_order,
            };
            let mut environment = StaticEnv::new();
            assert_eq!(
                exec_script(&module.top_level, &mut environment, simulation),
                expected.is_some(),
                "{dialect}: {source}"
            );
            assert_eq!(
                environment.get("result"),
                expected
                    .map(|value| StaticValue::String(value.to_owned()))
                    .as_ref(),
                "{dialect}: {source}"
            );
        }
    }
}

#[test]
fn original_increment_amount_keeps_braced_data_separate_from_reads() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        for (amount, braced, supported) in [
            ("{$delta}", true, false),
            ("$delta", false, true),
            ("{2}", true, true),
        ] {
            let source = format!("set n 1; set delta 2; incr n {amount}");
            let fixture = crate::provider_fixtures::original_native_module(&source, dialect);
            let (context, module) = &*fixture;
            let statement = module.top_level.statements.last().unwrap();
            let Statement::Incr { amount_braced, .. } = statement else {
                panic!("actual source increment must retain its typed amount");
            };
            assert_eq!(*amount_braced, braced);
            assert!(
                module
                    .top_level
                    .retained_source_tokens_for_statement(statement)
                    .is_some()
            );
            let policy = FoldPolicy::for_retained_entry(
                context.commands(),
                Some(tcl_registry::InvocationDialect::of_profile(
                    module
                        .source_metadata_input
                        .as_ref()
                        .unwrap()
                        .unit_profile(),
                )),
                &module.native_lexer_config(),
            );
            let observations = std::cell::RefCell::new(Vec::new());
            let increments = std::cell::RefCell::new(Vec::new());
            let preparations = std::cell::RefCell::new(Vec::new());
            let dependencies = std::cell::RefCell::new(Vec::new());
            let evaluation_order = std::cell::Cell::new(0);
            let simulation = StaticSimulation {
                source: None,
                policy,
                purpose: SimulationPurpose::Analysis,
                observations: &observations,
                increments: &increments,
                preparations: &preparations,
                dependencies: &dependencies,
                evaluation_order: &evaluation_order,
            };
            let mut environment = StaticEnv::new();
            assert_eq!(
                exec_script(&module.top_level, &mut environment, simulation),
                supported
            );
            if supported {
                assert_eq!(environment.get("n"), Some(&StaticValue::Int(3)));
                assert_eq!(increments.borrow().len(), 1);
            } else {
                assert_eq!(
                    environment.get("n"),
                    Some(&StaticValue::String("1".to_owned()))
                );
                assert!(increments.borrow().is_empty());
            }
        }
    }
}

#[test]
fn original_increment_reads_use_whole_words_and_decode_literal_escapes() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        for (amount, braced, supported) in [
            ("\"$delta\"", false, true),
            (r"\x32", false, true),
            ("\"$delta tail\"", false, false),
            ("$missing", false, false),
        ] {
            let source = format!("set n 1; set delta 2; incr n {amount}");
            let fixture = crate::provider_fixtures::original_native_module(&source, dialect);
            let (context, module) = &*fixture;
            let statement = module.top_level.statements.last().unwrap();
            let Statement::Incr { amount_braced, .. } = statement else {
                panic!("actual source increment must retain its typed amount");
            };
            assert_eq!(*amount_braced, braced);
            assert!(
                module
                    .top_level
                    .retained_source_tokens_for_statement(statement)
                    .is_some()
            );
            let policy = FoldPolicy::for_retained_entry(
                context.commands(),
                Some(tcl_registry::InvocationDialect::of_profile(
                    module
                        .source_metadata_input
                        .as_ref()
                        .unwrap()
                        .unit_profile(),
                )),
                &module.native_lexer_config(),
            );
            let observations = std::cell::RefCell::new(Vec::new());
            let increments = std::cell::RefCell::new(Vec::new());
            let preparations = std::cell::RefCell::new(Vec::new());
            let dependencies = std::cell::RefCell::new(Vec::new());
            let evaluation_order = std::cell::Cell::new(0);
            let simulation = StaticSimulation {
                source: None,
                policy,
                purpose: SimulationPurpose::Analysis,
                observations: &observations,
                increments: &increments,
                preparations: &preparations,
                dependencies: &dependencies,
                evaluation_order: &evaluation_order,
            };
            let mut environment = StaticEnv::new();
            assert_eq!(
                exec_script(&module.top_level, &mut environment, simulation),
                supported
            );
            if supported {
                assert_eq!(environment.get("n"), Some(&StaticValue::Int(3)));
                assert_eq!(increments.borrow().len(), 1);
            } else {
                assert_eq!(
                    environment.get("n"),
                    Some(&StaticValue::String("1".to_owned()))
                );
                assert!(increments.borrow().is_empty());
            }
        }
    }
}

fn simulate_case_source(
    script: &Script,
    registry: &tcl_registry::CommandRegistry,
    metadata: &crate::registry_invocation::OwnedInvocationMetadataContext,
    policy: FoldPolicy,
    purpose: SimulationPurpose,
    environment: &mut StaticEnv,
) -> bool {
    let observations = std::cell::RefCell::new(Vec::new());
    let increments = std::cell::RefCell::new(Vec::new());
    let preparations = std::cell::RefCell::new(Vec::new());
    let dependencies = std::cell::RefCell::new(Vec::new());
    let evaluation_order = std::cell::Cell::new(0);
    exec_script(
        script,
        environment,
        StaticSimulation {
            source: Some(StaticLoopSource::new(registry, metadata)),
            policy,
            purpose,
            observations: &observations,
            increments: &increments,
            preparations: &preparations,
            dependencies: &dependencies,
            evaluation_order: &evaluation_order,
        },
    )
}

#[test]
fn original_exact_case_simulation_keeps_subject_names_and_literal_patterns() {
    for name in ["$literal", "[literal]", "with space", " with space "] {
        let source = format!(
            "switch -exact -- ${{{name}}} {{{{ a }} {{set result MATCH}} default {{set result MISS}}}}"
        );
        let fixture = crate::provider_fixtures::original_native_module(&source, "tcl8.6");
        let (context, module) = &*fixture;
        assert!(matches!(
            module.top_level.statements[0],
            Statement::Switch { .. }
        ));
        let input = module.source_metadata_input.as_ref().unwrap();
        let metadata = crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(
            Some(input),
        );
        let policy = FoldPolicy::for_retained_entry(
            context.commands(),
            Some(tcl_registry::InvocationDialect::of_profile(
                input.unit_profile(),
            )),
            &module.native_lexer_config(),
        );
        let mut environment =
            StaticEnv::from([(name.to_owned(), StaticValue::String(" a ".to_owned()))]);
        assert!(
            simulate_case_source(
                &module.top_level,
                context.commands(),
                &metadata,
                policy,
                SimulationPurpose::Analysis,
                &mut environment
            ),
            "{source}"
        );
        assert_eq!(
            environment.get("result"),
            Some(&StaticValue::String("MATCH".to_owned())),
            "{source}"
        );
    }
}

#[test]
fn original_case_simulation_refuses_changed_subject_body_and_missing_owner() {
    let fixture = crate::provider_fixtures::original_native_module(
        "switch -exact -- $value {a {set result MATCH} default {set result MISS}}",
        "tcl8.6",
    );
    let (context, module) = &*fixture;
    let input = module.source_metadata_input.as_ref().unwrap();
    let metadata =
        crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(input));
    let policy = FoldPolicy::for_retained_entry(
        context.commands(),
        Some(tcl_registry::InvocationDialect::of_profile(
            input.unit_profile(),
        )),
        &module.native_lexer_config(),
    );
    let environment = StaticEnv::from([("value".to_owned(), StaticValue::String("a".to_owned()))]);
    assert!(simulate_case_source(
        &module.top_level,
        context.commands(),
        &metadata,
        policy,
        SimulationPurpose::Analysis,
        &mut environment.clone()
    ));
    let mut changed = module.top_level.clone();
    let Statement::Switch { subject, .. } = &mut changed.statements[0] else {
        panic!("original typed Switch")
    };
    *subject = "DIFFERENT".to_owned();
    assert!(!simulate_case_source(
        &changed,
        context.commands(),
        &metadata,
        policy,
        SimulationPurpose::Analysis,
        &mut environment.clone()
    ));
    let mut changed = module.top_level.clone();
    let Statement::Switch {
        arms, default_body, ..
    } = &mut changed.statements[0]
    else {
        panic!("original typed Switch")
    };
    std::mem::swap(&mut arms[0].body, default_body);
    assert!(!simulate_case_source(
        &changed,
        context.commands(),
        &metadata,
        policy,
        SimulationPurpose::Analysis,
        &mut environment.clone()
    ));
    assert!(!simulate_case_source(
        &module.top_level,
        context.commands(),
        &crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable,
        policy,
        SimulationPurpose::Analysis,
        &mut environment.clone()
    ));
    assert!(!simulate_case_source(
        &module.top_level,
        context.commands(),
        &metadata,
        policy,
        SimulationPurpose::Execution,
        &mut environment.clone()
    ));
}

#[test]
fn standalone_case_simulation_declines_unsupported_matching_modes() {
    for mode in [SwitchMode::Glob, SwitchMode::Regexp] {
        let mut statement = mode_switch();
        let Statement::Switch { mode: selected, .. } = &mut statement else {
            unreachable!()
        };
        *selected = mode;
        let body = script_of(vec![statement]);
        assert!(
            analyse_static_for(
                &script_of(vec![assign_const("i", "0")]),
                &parse_expr("$i < 1", None),
                &script_of(vec![incr("i", None)]),
                &body,
                &StaticEnv::from([("mode".to_owned(), StaticValue::String("a".to_owned()))]),
                1,
                selected_analysis_policy()
            )
            .is_none()
        );
    }
    let mut statement = mode_switch();
    let Statement::Switch { nocase, .. } = &mut statement else {
        unreachable!()
    };
    *nocase = true;
    assert!(
        analyse_static_for(
            &script_of(vec![assign_const("i", "0")]),
            &parse_expr("$i < 1", None),
            &script_of(vec![incr("i", None)]),
            &script_of(vec![statement]),
            &StaticEnv::from([("mode".to_owned(), StaticValue::String("a".to_owned()))]),
            1,
            selected_analysis_policy()
        )
        .is_none()
    );
}

#[test]
fn semantic_selected_result_preserves_native_bytes() {
    let policy =
        FoldPolicy::default().with_invocation_dialect(tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        ));
    let observations = std::cell::RefCell::new(Vec::new());
    let increments = std::cell::RefCell::new(Vec::new());
    let preparations = std::cell::RefCell::new(Vec::new());
    let dependencies = std::cell::RefCell::new(Vec::new());
    let evaluation_order = std::cell::Cell::new(0);
    let simulation = StaticSimulation {
        source: None,
        policy,
        purpose: SimulationPurpose::Analysis,
        observations: &observations,
        increments: &increments,
        preparations: &preparations,
        dependencies: &dependencies,
        evaluation_order: &evaluation_order,
    };
    let environment = StaticEnv::from([("x".to_owned(), StaticValue::String("003".to_owned()))]);
    let result = simulation
        .evaluate_contents(&parse_expr("$x", None), &environment, None)
        .expect("native selected result is known");
    assert_eq!(result.0, StaticValue::String("003".to_owned()));
    assert_eq!(result.1, Some(3));
    assert!(observations.borrow()[0].result_dependency.is_some());
}

// saturating_f64_to_i64

#[test]
fn saturating_f64_to_i64_in_range_and_saturates() {
    // In-range integral floats convert exactly.
    assert_eq!(saturating_f64_to_i64(0.0), 0);
    assert_eq!(saturating_f64_to_i64(42.0), 42);
    assert_eq!(saturating_f64_to_i64(-42.0), -42);
    // `-0.0` normalises to 0 (tclsh `int(-0.0)` == 0), not "-0".
    assert_eq!(saturating_f64_to_i64(-0.0), 0);
    // Out-of-range magnitudes saturate by sign, as an `as i64` cast
    // does.
    assert_eq!(saturating_f64_to_i64(1e30), i64::MAX);
    assert_eq!(saturating_f64_to_i64(-1e30), i64::MIN);
}

// simple_var_ref

#[test]
fn simple_var_ref_bare_and_braced() {
    assert_eq!(simple_var_ref("$x"), Some("x".into()));
    assert_eq!(simple_var_ref("${x}"), Some("x".into()));
    assert_eq!(simple_var_ref("$foo::bar"), Some("foo::bar".into()));
}

#[test]
fn simple_var_ref_rejects_non_var() {
    assert_eq!(simple_var_ref("hello"), None);
    assert_eq!(simple_var_ref("$x extra"), None);
    assert_eq!(simple_var_ref("$1bad"), None);
}

// summarise_static_for

#[test]
fn analyse_counts_iterations_to_five() {
    // for {set i 0} {$i < 5} {incr i} { /* nothing */ }
    let init = script_of(vec![assign_const("i", "0")]);
    let cond = parse_expr("$i < 5", None);
    let next_script = script_of(vec![incr("i", None)]);
    let body = empty_script();
    let env = analyse_static_for(
        &init,
        &cond,
        &next_script,
        &body,
        &StaticEnv::new(),
        1000,
        selected_analysis_policy(),
    )
    .expect("summarised");
    assert_eq!(env.get("i"), Some(&StaticValue::Int(5)));
}

#[test]
fn analyse_body_accumulates_counter() {
    // for {set i 0; set total 0} {$i < 3} {incr i} { incr total }
    let init = script_of(vec![assign_const("i", "0"), assign_const("total", "0")]);
    let cond = parse_expr("$i < 3", None);
    let next_script = script_of(vec![incr("i", None)]);
    let body = script_of(vec![incr("total", None)]);
    let env = analyse_static_for(
        &init,
        &cond,
        &next_script,
        &body,
        &StaticEnv::new(),
        1000,
        selected_analysis_policy(),
    )
    .expect("summarised");
    assert_eq!(env.get("total"), Some(&StaticValue::Int(3)));
    assert_eq!(env.get("i"), Some(&StaticValue::Int(3)));
}

#[test]
fn analyse_respects_iteration_cap() {
    let init = script_of(vec![assign_const("i", "0")]);
    let cond = parse_expr("$i < 10000", None);
    let next_script = script_of(vec![incr("i", None)]);
    let body = empty_script();
    let result = analyse_static_for(
        &init,
        &cond,
        &next_script,
        &body,
        &StaticEnv::new(),
        100,
        selected_analysis_policy(),
    );
    assert!(result.is_none(), "should exceed the 100-iter cap");
}

#[test]
fn analyse_unsupported_statement_returns_none() {
    // Body contains a `Call` → out of the supported subset.
    let init = script_of(vec![assign_const("i", "0")]);
    let cond = parse_expr("$i < 3", None);
    let next_script = script_of(vec![incr("i", None)]);
    let body = script_of(vec![Statement::Call {
        span: sp(),
        command: "puts".into(),
        canonical_command: None,
        args: vec!["$i".into()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }]);
    assert!(
        analyse_static_for(
            &init,
            &cond,
            &next_script,
            &body,
            &StaticEnv::new(),
            1000,
            selected_analysis_policy()
        )
        .is_none()
    );
}

/// `switch $mode { a {set v 1} default {set v 9} }` — shared by the
/// switch-dispatch case and its unresolvable-subject counterpart.
fn mode_switch() -> Statement {
    Statement::Switch {
        command: "switch".into(),
        subject_braced: false,
        raw_arg_braced: Vec::new(),
        raw_arg_quoted: Vec::new(),
        span: sp(),
        subject: "$mode".into(),
        subject_span: sp(),
        arms: vec![SwitchArm {
            pattern: "a".into(),
            pattern_braced: true,
            pattern_span: sp(),
            body: Some(script_of(vec![assign_const("v", "1")])),
            body_span: Some(sp()),
            fallthrough: false,
        }],
        default_body: Some(script_of(vec![assign_const("v", "9")])),
        default_span: None,
        mode: SwitchMode::Exact,
        nocase: false,
        raw_args: Vec::new(),
        patterns_braced: true,
    }
}

#[test]
fn analyse_resolves_if_else_branch_in_body() {
    // for {set i 0} {$i < 3} {incr i} {
    //     if {$i == 1} {set x 10} else {set x 20}
    // }  →  i ends at 3; the last iteration (i = 2) takes the else.
    let init = script_of(vec![assign_const("i", "0")]);
    let cond = parse_expr("$i < 3", None);
    let next_script = script_of(vec![incr("i", None)]);
    let body = script_of(vec![Statement::If {
        span: sp(),
        clauses: vec![IfClause {
            condition: parse_expr("$i == 1", None),
            condition_span: sp(),
            body: script_of(vec![assign_const("x", "10")]),
            body_span: sp(),
            condition_base: None,
        }],
        else_body: Some(script_of(vec![assign_const("x", "20")])),
        else_span: None,
    }]);
    let env = analyse_static_for(
        &init,
        &cond,
        &next_script,
        &body,
        &StaticEnv::new(),
        1000,
        selected_analysis_policy(),
    )
    .expect("summarised");
    assert_eq!(env.get("i"), Some(&StaticValue::Int(3)));
    assert_eq!(env.get("x"), Some(&StaticValue::String("20".to_owned())));
}

#[test]
fn analyse_resolves_switch_dispatch_in_body() {
    // for {set i 0; set mode a} {$i < 1} {incr i} { switch … } → v = 1.
    let init = script_of(vec![assign_const("i", "0"), assign_const("mode", "a")]);
    let cond = parse_expr("$i < 1", None);
    let next_script = script_of(vec![incr("i", None)]);
    let body = script_of(vec![mode_switch()]);
    let env = analyse_static_for(
        &init,
        &cond,
        &next_script,
        &body,
        &StaticEnv::new(),
        1000,
        selected_analysis_policy(),
    )
    .expect("summarised");
    assert_eq!(env.get("v"), Some(&StaticValue::String("1".to_owned())));
}

#[test]
fn analyse_bails_on_unresolvable_switch_subject() {
    // `$mode` is never set, so the subject can't resolve → summary bails.
    let init = script_of(vec![assign_const("i", "0")]);
    let cond = parse_expr("$i < 1", None);
    let next_script = script_of(vec![incr("i", None)]);
    let body = script_of(vec![mode_switch()]);
    let result = analyse_static_for(
        &init,
        &cond,
        &next_script,
        &body,
        &StaticEnv::new(),
        1000,
        selected_analysis_policy(),
    );
    assert!(result.is_none(), "unresolvable switch subject should bail");
}

#[test]
fn analyse_forwards_for_statement_helper() {
    let for_stmt = Statement::For {
        span: sp(),
        init: script_of(vec![assign_const("i", "0")]),
        init_span: sp(),
        condition: parse_expr("$i < 2", None),
        condition_span: sp(),
        next: script_of(vec![incr("i", None)]),
        next_span: sp(),
        body: empty_script(),
        body_span: sp(),
        raw_args: Vec::new(),
        raw_tokens: None,
        condition_base: None,
    };
    let env = analyse_for_statement_with_dependencies(
        &for_stmt,
        &StaticEnv::new(),
        100,
        selected_analysis_policy(),
        None,
    )
    .expect("analysed")
    .values;
    assert_eq!(env.get("i"), Some(&StaticValue::Int(2)));
}

// evaluate_expr_with_constants

#[test]
fn evaluate_expr_integer() {
    let mut env = StaticEnv::new();
    env.insert("x".into(), StaticValue::Int(5));
    assert_eq!(
        evaluate_expr_with_constants(&parse_expr("$x + 3", None), &env, FoldPolicy::default()),
        None
    );
    let evaluation = crate::tcl_expr_eval::analyse_tcl_expr_with_resolved_math_bindings(
        &parse_expr("$x + 3", None),
        &env_as_tcl_env(&env),
        selected_analysis_policy(),
        &|_, _| None,
        None,
    )
    .expect("analysis retains numeric conversion");
    assert_eq!(evaluation.value, TclValue::Int(8));
    assert_ne!(
        evaluation.coercions,
        [] as [crate::tcl_expr_eval::NativeCoercionObligation; 0]
    );
}

#[test]
fn evaluate_expr_integer_valued_float() {
    assert_eq!(
        evaluate_expr_with_constants(
            &parse_expr("6.0 / 2", None),
            &StaticEnv::new(),
            FoldPolicy::default()
        ),
        Some(3)
    );
}

#[test]
fn evaluate_expr_fractional_float_none() {
    assert_eq!(
        evaluate_expr_with_constants(
            &parse_expr("1.5", None),
            &StaticEnv::new(),
            FoldPolicy::default()
        ),
        None
    );
}
