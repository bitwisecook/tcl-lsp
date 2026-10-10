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

//! Conservative static evaluation of simple Tcl `for`-loops.
//!
//! Supports a narrow, side-effect-free subset so callers can
//! infer post-loop constants without changing semantics. Uses
//! the expression evaluator to fold conditions and bounded
//! iteration (capped by `DEFAULT_MAX_STATIC_LOOP_ITERS`) to
//! catch pathological inputs.

use std::collections::HashMap;

use crate::expr_ast::ExprNode;
use crate::ir::{IfClause, Script, Statement, SwitchArm, SwitchMode};
use crate::tcl_expr_eval::{Env, EnvValue, FoldPolicy, TclValue};

use super::{DEFAULT_MAX_STATIC_LOOP_ITERS, StaticEnv, StaticValue, simple_var_ref};

impl StaticValue {
    /// Project mathematical contents, without proving a fresh native object.
    pub(crate) fn to_env_value(&self) -> EnvValue {
        match self {
            Self::Int(i) => EnvValue::Int(*i),
            Self::Float(f) => EnvValue::Float(*f),
            Self::Bool(b) => EnvValue::Int(i64::from(*b)),
            Self::Str(s) => EnvValue::Str(s.clone()),
        }
    }
}

fn env_as_tcl_env(env: &StaticEnv) -> Env {
    env.iter()
        .map(|(k, v)| (k.clone(), v.to_env_value()))
        .collect()
}

/// Parse a literal text as [`StaticValue`]. Prefers integer,
/// then `true`/`false`, then string fallback.
#[must_use]
pub fn parse_literal_value(text: &str) -> StaticValue {
    let stripped = text.trim();
    if let Ok(i) = stripped.parse::<i64>() {
        return StaticValue::Int(i);
    }
    match stripped.to_ascii_lowercase().as_str() {
        "true" => return StaticValue::Bool(true),
        "false" => return StaticValue::Bool(false),
        _ => {}
    }
    StaticValue::Str(stripped.to_owned())
}

/// Evaluate an expression string under `env`.
///
/// Returns `Some(int)` when the result folds to an integer (or an
/// integer-valued float), `None` otherwise. Booleans collapse into
/// `i64` (0/1) for simpler call-sites.
#[must_use]
pub fn evaluate_expr_with_constants(
    expr: &ExprNode,
    env: &StaticEnv,
    policy: FoldPolicy,
) -> Option<i64> {
    evaluate_expr_with_constants_and_math_bindings(expr, env, policy, None)
}

/// Evaluate with exact retained reached-call evidence. A missing query is an
/// unknown execution binding, including in the compatibility entry above.
#[must_use]
pub fn evaluate_expr_with_constants_and_math_bindings(
    expr: &ExprNode,
    env: &StaticEnv,
    policy: FoldPolicy,
    bindings: Option<crate::math_function_binding::ExpressionMathBindings<'_>>,
) -> Option<i64> {
    let dependencies = std::cell::RefCell::new(Vec::new());
    let preparations = std::cell::RefCell::new(Vec::new());
    let observations = std::cell::RefCell::new(Vec::new());
    let increments = std::cell::RefCell::new(Vec::new());
    let evaluation_order = std::cell::Cell::new(0);
    StaticSimulation {
        purpose: SimulationPurpose::Execution,
        observations: &observations,
        increments: &increments,
        evaluation_order: &evaluation_order,
        policy,
        dependencies: &dependencies,
        preparations: &preparations,
    }
    .evaluate(expr, env, bindings)
}

/// Convert a finite, integer-valued `f64` to `i64`, saturating to
/// `i64::MIN` / `i64::MAX` when the value is out of range.
///
/// Avoids a lossy `as` cast: the value is rendered to its exact integer
/// decimal (`f` is integral by contract) and parsed. Out-of-range
/// magnitudes fail to parse and saturate by sign, as an `f as i64` cast
/// does.
fn saturating_f64_to_i64(f: f64) -> i64 {
    // `+ 0.0` normalises `-0.0` to `0.0` so it renders/parses as `0`, as an
    // `as i64` cast does.
    match format!("{:.0}", f + 0.0).parse::<i64>() {
        Ok(i) => i,
        Err(_) if f.is_sign_negative() => i64::MIN,
        Err(_) => i64::MAX,
    }
}

// For-loop summarisation

/// Summarise a simple static `for`-loop from its structured IR
/// form. Returns the post-loop variable environment on success,
/// `None` if the loop escapes the supported subset or exceeds
/// `max_iterations`.
#[must_use]
pub fn summarise_static_for(
    init: &Script,
    condition: &ExprNode,
    next_script: &Script,
    body: &Script,
    initial_constants: &StaticEnv,
    max_iterations: u64,
    policy: FoldPolicy,
) -> Option<StaticEnv> {
    summarise_static_for_selected(
        init,
        StaticForCondition {
            purpose: SimulationPurpose::Execution,
            expression: condition,
            bindings: None,
        },
        next_script,
        body,
        initial_constants,
        max_iterations,
        policy,
    )
    .map(|summary| summary.values)
}

#[derive(Clone, Copy)]
struct StaticForCondition<'a> {
    purpose: SimulationPurpose,
    expression: &'a ExprNode,
    bindings: Option<crate::math_function_binding::ExpressionMathBindings<'a>>,
}

fn summarise_static_for_selected(
    init: &Script,
    condition: StaticForCondition<'_>,
    next_script: &Script,
    body: &Script,
    initial_constants: &StaticEnv,
    max_iterations: u64,
    policy: FoldPolicy,
) -> Option<StaticLoopAnalysis> {
    let mut env: StaticEnv = initial_constants.clone();
    let dependencies = std::cell::RefCell::new(Vec::new());
    let preparations = std::cell::RefCell::new(Vec::new());
    let observations = std::cell::RefCell::new(Vec::new());
    let increments = std::cell::RefCell::new(Vec::new());
    let evaluation_order = std::cell::Cell::new(0);
    let simulation = StaticSimulation {
        policy,
        purpose: condition.purpose,
        observations: &observations,
        increments: &increments,
        evaluation_order: &evaluation_order,
        dependencies: &dependencies,
        preparations: &preparations,
    };

    if !exec_script(init, &mut env, simulation) {
        return None;
    }
    let mut iterations: u64 = 0;
    loop {
        let cond = simulation.evaluate(condition.expression, &env, condition.bindings)?;
        if cond == 0 {
            break;
        }
        iterations += 1;
        if iterations > max_iterations {
            return None;
        }
        if !exec_script(body, &mut env, simulation) {
            return None;
        }
        if !exec_script(next_script, &mut env, simulation) {
            return None;
        }
    }
    Some(StaticLoopAnalysis {
        expression_obligations: observations.into_inner(),
        increment_obligations: increments.into_inner(),
        values: env,
        required_math_invocations: dependencies.into_inner(),
        required_expression_preparations: preparations.into_inner(),
    })
}

/// Convenience entry point that extracts the init/condition/next/
/// body from a [`Statement::For`] and forwards to
/// [`summarise_static_for`].
#[must_use]
pub fn summarise_for_statement(
    stmt: &Statement,
    initial_constants: &StaticEnv,
    max_iterations: u64,
    policy: FoldPolicy,
) -> Option<StaticEnv> {
    summarise_for_statement_with_math_bindings(
        stmt,
        initial_constants,
        max_iterations,
        policy,
        None,
    )
}

/// Summarise under an exact original condition query; body expressions carry
/// their own Script source inventories. Never reparse rendered conditions to
/// reuse an AST offset from another expression.
#[must_use]
pub fn summarise_for_statement_with_math_bindings(
    stmt: &Statement,
    initial_constants: &StaticEnv,
    max_iterations: u64,
    policy: FoldPolicy,
    condition_bindings: Option<crate::math_function_binding::ExpressionMathBindings<'_>>,
) -> Option<StaticEnv> {
    summarise_for_statement_with_dependencies(
        stmt,
        initial_constants,
        max_iterations,
        policy,
        condition_bindings,
    )
    .map(|summary| summary.values)
}

/// Preserve dependencies consumed by successful loop simulation for artifacts.
#[must_use]
pub fn summarise_for_statement_with_dependencies(
    stmt: &Statement,
    initial_constants: &StaticEnv,
    max_iterations: u64,
    policy: FoldPolicy,
    condition_bindings: Option<crate::math_function_binding::ExpressionMathBindings<'_>>,
) -> Option<StaticLoopSummary> {
    let Statement::For {
        init,
        condition,
        next,
        body,
        ..
    } = stmt
    else {
        return None;
    };
    summarise_static_for_selected(
        init,
        StaticForCondition {
            purpose: SimulationPurpose::Execution,
            expression: condition,
            bindings: condition_bindings,
        },
        next,
        body,
        initial_constants,
        max_iterations,
        policy,
    )
    .map(|analysis| StaticLoopSummary {
        values: analysis.values,
        required_math_invocations: analysis.required_math_invocations,
        required_expression_preparations: analysis.required_expression_preparations,
    })
}

/// Analyse reached native loop values while retaining conversion obligations.
/// This distinct result cannot be used as an executable static-loop summary.
#[must_use]
pub fn analyse_for_statement_with_dependencies(
    statement: &Statement,
    initial_constants: &StaticEnv,
    max_iterations: u64,
    policy: FoldPolicy,
    condition_bindings: Option<crate::math_function_binding::ExpressionMathBindings<'_>>,
) -> Option<StaticLoopAnalysis> {
    let Statement::For {
        init,
        condition,
        next,
        body,
        ..
    } = statement
    else {
        return None;
    };
    summarise_static_for_selected(
        init,
        StaticForCondition {
            purpose: SimulationPurpose::Analysis,
            expression: condition,
            bindings: condition_bindings,
        },
        next,
        body,
        initial_constants,
        max_iterations,
        policy,
    )
}

#[cfg(test)]
mod tests {
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
    fn positioned_simulation_requires_matching_whole_expression_preparation() {
        use crate::command_binding::{
            CommandAllocationSite, ExecutedScriptSource, SourceExpressionPreparation,
            SourceOriginId,
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
            summarise_for_statement_with_dependencies(
                &statement,
                &StaticEnv::new(),
                20,
                policy,
                None,
            )
            .is_none()
        );
        let analysis = analyse_for_statement_with_dependencies(
            &statement,
            &StaticEnv::new(),
            20,
            policy,
            None,
        )
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
    fn semantic_selected_result_preserves_native_bytes() {
        let policy = FoldPolicy::default().with_invocation_dialect(
            tcl_registry::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
                tcl_dialect::model::Release::JIM_0_84,
            )),
        );
        let observations = std::cell::RefCell::new(Vec::new());
        let increments = std::cell::RefCell::new(Vec::new());
        let preparations = std::cell::RefCell::new(Vec::new());
        let dependencies = std::cell::RefCell::new(Vec::new());
        let evaluation_order = std::cell::Cell::new(0);
        let simulation = StaticSimulation {
            policy,
            purpose: SimulationPurpose::Analysis,
            observations: &observations,
            increments: &increments,
            preparations: &preparations,
            dependencies: &dependencies,
            evaluation_order: &evaluation_order,
        };
        let environment = StaticEnv::from([("x".to_owned(), StaticValue::Str("003".to_owned()))]);
        let result = simulation
            .evaluate_contents(&parse_expr("$x", None), &environment, None)
            .expect("native selected result is known");
        assert_eq!(result.0, StaticValue::Str("003".to_owned()));
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

    // parse_literal_value

    #[test]
    fn parse_literal_int_bool_string() {
        assert_eq!(parse_literal_value("42"), StaticValue::Int(42));
        assert_eq!(parse_literal_value("-7"), StaticValue::Int(-7));
        assert_eq!(parse_literal_value("true"), StaticValue::Bool(true));
        assert_eq!(parse_literal_value("False"), StaticValue::Bool(false));
        assert_eq!(
            parse_literal_value("hello"),
            StaticValue::Str("hello".into())
        );
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
            subject_braced: false,
            raw_arg_braced: Vec::new(),
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
        assert_eq!(env.get("x"), Some(&StaticValue::Str("20".to_owned())));
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
        assert_eq!(env.get("v"), Some(&StaticValue::Str("1".to_owned())));
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
}
