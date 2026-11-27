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

use tcl_registry::value_transfer::NumericValue;

use crate::expr_ast::ExprNode;
use crate::ir::{IfClause, Script, Statement, SwitchArm, SwitchMode};
use crate::tcl_expr_eval::{Env, FoldPolicy, TclValue, const_to_env_value};
use crate::value_transfer::format_constant_with_policy;

#[cfg(test)]
use super::simple_var_ref;
use super::{StaticEnv, StaticValue};

fn env_as_tcl_env(env: &StaticEnv) -> Env {
    env.iter()
        .map(|(k, v)| (k.clone(), const_to_env_value(v)))
        .collect()
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
        source: None,
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

fn strip_word_delimiters(text: &str) -> String {
    let stripped = text.trim();
    if stripped.len() >= 2 {
        let bytes = stripped.as_bytes();
        let first = bytes[0];
        let last = bytes[stripped.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'{' && last == b'}') {
            return stripped[1..stripped.len() - 1].to_owned();
        }
    }
    stripped.to_owned()
}

fn resolve_switch_subject(text: &str, env: &StaticEnv, policy: FoldPolicy) -> Option<String> {
    let stripped = text.trim();
    if stripped.contains('$') || stripped.contains('[') {
        let config =
            tcl_lexer::LexerConfig::from_grammar(policy.preparation_context()?.lexer_grammar);
        let name = tcl_syntax::naming::whole_scalar_variable_reference_name(stripped, config)?;
        return format_constant_with_policy(env.get(name)?, policy);
    }
    Some(strip_word_delimiters(stripped))
}

fn resolve_switch_pattern(pattern: &str) -> String {
    strip_word_delimiters(pattern)
}

// Simulator

/// A successful loop simulation and every implicit call it consumed.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticLoopSummary {
    /// Values established at the loop exit.
    pub values: StaticEnv,
    /// Guard obligations survive removal of the original expression.
    pub required_math_invocations: Vec<crate::command_binding::SourceMathInvocation>,
    /// Whole-expression entry validation survives removal of its evaluation.
    pub required_expression_preparations: Vec<crate::command_binding::SourceExpressionPreparation>,
}

/// A reached expression whose native object effects remain represented.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticExpressionObligation {
    /// Evaluation order across all iterations, including repeated source sites.
    pub evaluation_order: usize,
    /// Reached conversions; these never become fresh-object evidence.
    pub coercions: Vec<crate::tcl_expr_eval::NativeCoercionObligation>,
    /// Existing native object or bytes preserved by expression completion.
    pub result_dependency: Option<crate::tcl_expr_eval::NativeExpressionResultDependency>,
    /// Exact native expression preparation, when positioned evidence exists.
    pub preparation: Option<crate::command_binding::SourceExpressionPreparation>,
}

/// A native increment whose retained operand conversion must still execute.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticIncrementObligation {
    /// Order shared with reached expression evaluations.
    pub evaluation_order: usize,
    /// Original increment statement, including its operand syntax and extent.
    pub statement: Statement,
}

/// Semantic loop values without a licence to erase native evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticLoopAnalysis {
    /// Native storage contents tracked by the same bounded simulator.
    pub values: StaticEnv,
    /// Reached implicit dispatch requirements.
    pub required_math_invocations: Vec<crate::command_binding::SourceMathInvocation>,
    /// Whole-expression native entry requirements.
    pub required_expression_preparations: Vec<crate::command_binding::SourceExpressionPreparation>,
    /// Effects and result preservation at each reached evaluation.
    pub expression_obligations: Vec<StaticExpressionObligation>,
    /// Reached increment operations without native-object conversion evidence.
    pub increment_obligations: Vec<StaticIncrementObligation>,
}

#[derive(Clone, Copy)]
enum SimulationPurpose {
    Execution,
    Analysis,
}

/// Exact source metadata retained by the current CFG, separate from catalogue simulation.
#[derive(Clone, Copy)]
pub(crate) struct StaticLoopSource<'a> {
    registry: &'a tcl_registry::CommandRegistry,
    metadata: &'a crate::registry_invocation::OwnedInvocationMetadataContext,
}

impl<'a> StaticLoopSource<'a> {
    pub(crate) const fn new(
        registry: &'a tcl_registry::CommandRegistry,
        metadata: &'a crate::registry_invocation::OwnedInvocationMetadataContext,
    ) -> Self {
        Self { registry, metadata }
    }

    fn case(
        self,
        script: &Script,
        statement: &Statement,
    ) -> Option<crate::registry_invocation::source_structure::OriginalSourceCase> {
        let metadata = self.metadata.metadata_context(self.registry)?.context()?;
        let tokens = script.retained_source_tokens_for_statement(statement)?;
        let binding = tokens.source_binding.as_ref()?;
        let metadata = binding.original_invocation_metadata_for_input(
            tokens,
            metadata.source_analysis_input()?,
            self.registry,
        )?;
        if binding.original_invocation_source_purpose(tokens, metadata, self.registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical
        {
            crate::registry_invocation::source_structure::original_logical_source_case_for_tokens(
                tokens,
                self.registry,
                metadata,
            )
        } else {
            crate::registry_invocation::source_structure::original_native_source_case_for_statement(
                script,
                statement,
                self.registry,
                metadata,
            )
        }
    }
}

#[derive(Clone, Copy)]
struct StaticSimulation<'a> {
    source: Option<StaticLoopSource<'a>>,
    policy: FoldPolicy,
    purpose: SimulationPurpose,
    observations: &'a std::cell::RefCell<Vec<StaticExpressionObligation>>,
    increments: &'a std::cell::RefCell<Vec<StaticIncrementObligation>>,
    evaluation_order: &'a std::cell::Cell<usize>,
    dependencies: &'a std::cell::RefCell<Vec<crate::command_binding::SourceMathInvocation>>,
    preparations: &'a std::cell::RefCell<Vec<crate::command_binding::SourceExpressionPreparation>>,
}

impl StaticSimulation<'_> {
    fn evaluate(
        self,
        expression: &ExprNode,
        environment: &StaticEnv,
        bindings: Option<crate::math_function_binding::ExpressionMathBindings<'_>>,
    ) -> Option<i64> {
        self.evaluate_contents(expression, environment, bindings)?.1
    }

    fn resolve_math_call(
        self,
        bindings: crate::math_function_binding::ExpressionMathBindings<'_>,
        preparation: Option<&crate::command_binding::SourceExpressionPreparation>,
        consumed: &std::cell::RefCell<Vec<crate::command_binding::SourceMathInvocation>>,
        function: &str,
        start: u32,
    ) -> Option<crate::tcl_expr_eval::NativeMathFunctionTarget> {
        let call = bindings.resolved_call(function, start)?;
        crate::math_function_binding::native_fold_dependency(call.invocation)?;
        let required = call.invocation.fixed_prerequisite();
        if self
            .dependencies
            .borrow()
            .iter()
            .chain(consumed.borrow().iter())
            .any(|previous: &crate::command_binding::SourceMathInvocation| {
                !crate::math_function_binding::native_math_prerequisites_compatible(
                    previous.fixed_prerequisite(),
                    required,
                )
            })
            || self.preparations.borrow().iter().any(|previous| {
                !crate::math_function_binding::native_math_prerequisites_compatible(
                    previous.witness.fixed_functions(),
                    required,
                )
            })
            || preparation.is_some_and(|preparation| {
                !crate::math_function_binding::native_math_prerequisites_compatible(
                    preparation.witness.fixed_functions(),
                    required,
                )
            })
        {
            return None;
        }
        consumed.borrow_mut().push(call.invocation.clone());
        Some(call.target())
    }

    fn evaluate_contents(
        self,
        expression: &ExprNode,
        environment: &StaticEnv,
        bindings: Option<crate::math_function_binding::ExpressionMathBindings<'_>>,
    ) -> Option<(StaticValue, Option<i64>)> {
        let preparation = bindings.and_then(|bindings| bindings.preparation());
        if bindings.is_some_and(|bindings| bindings.is_positioned()) && preparation.is_none() {
            return None;
        }
        if let Some(preparation) = preparation
            && (self.policy.preparation_context().as_ref() != Some(preparation.witness.context())
                || self.preparations.borrow().iter().any(|previous| {
                    !crate::math_function_binding::native_math_prerequisites_compatible(
                        previous.witness.fixed_functions(),
                        preparation.witness.fixed_functions(),
                    )
                })
                || self.dependencies.borrow().iter().any(|previous| {
                    !crate::math_function_binding::native_math_prerequisites_compatible(
                        previous.fixed_prerequisite(),
                        preparation.witness.fixed_functions(),
                    )
                }))
        {
            return None;
        }
        let expression = preparation.map_or(expression, |preparation| preparation.witness.tree());
        let consumed = std::cell::RefCell::new(Vec::new());
        let evaluation = crate::tcl_expr_eval::analyse_tcl_expr_with_resolved_math_bindings(
            expression,
            &env_as_tcl_env(environment),
            self.policy,
            &|function, start| {
                self.resolve_math_call(bindings?, preparation, &consumed, function, start)
            },
            None,
        )?;
        if matches!(self.purpose, SimulationPurpose::Execution)
            && !evaluation.native_value_effects_are_proved()
        {
            return None;
        }
        let numeric = match evaluation.value {
            TclValue::Int(value) => Some(value),
            TclValue::Float(value) if value.is_finite() && value.fract() == 0.0 => {
                Some(saturating_f64_to_i64(value))
            }
            _ => None,
        };
        let grammar = preparation
            .map(|preparation| preparation.witness.context().lexer_grammar)
            .or_else(|| {
                self.policy
                    .preparation_context()
                    .map(|context| context.lexer_grammar)
            });
        let value = static_expression_contents(
            &evaluation,
            environment,
            grammar.map(tcl_lexer::LexerConfig::from_grammar),
        )?;
        let order = self.evaluation_order.get();
        self.evaluation_order.set(order.checked_add(1)?);
        if !evaluation.native_value_effects_are_proved() {
            self.observations
                .borrow_mut()
                .push(StaticExpressionObligation {
                    evaluation_order: order,
                    coercions: evaluation.coercions,
                    result_dependency: evaluation.result_dependency,
                    preparation: preparation.cloned(),
                });
        }
        let mut ledger = self.dependencies.borrow_mut();
        for proof in consumed.into_inner() {
            if !ledger.contains(&proof) {
                ledger.push(proof);
            }
        }
        if let Some(preparation) = preparation {
            let mut ledger = self.preparations.borrow_mut();
            if !ledger.contains(preparation) {
                ledger.push(preparation.clone());
            }
        }
        Some((value, numeric))
    }
}

fn static_integer(value: &StaticValue, policy: FoldPolicy) -> Option<i64> {
    match value {
        StaticValue::Number(NumericValue::Int(value)) => Some(*value),
        StaticValue::Number(NumericValue::Bool(value)) => Some(i64::from(*value)),
        StaticValue::String(value) => {
            let parse = |syntax| {
                tcl_syntax::number::parse_whole_with(
                    value.as_str(),
                    tcl_syntax::number::ParseFlags {
                        integer_only: true,
                        ..tcl_syntax::number::ParseFlags::for_syntax(syntax)
                    },
                )
            };
            let number = match policy.numbers {
                Some(syntax) => parse(syntax)?,
                None => tcl_dialect::NumberSyntax::unanimous(parse).flatten()?,
            };
            match policy.arithmetic {
                Some(
                    tcl_dialect::NativeArithmetic::Tcl84Wide
                    | tcl_dialect::NativeArithmetic::JimWide,
                ) => tcl_syntax::expr::wide::parsed_literal(policy.arithmetic?, &number).ok(),
                _ => match number {
                    tcl_syntax::number::Number::Int(value) => Some(value),
                    _ => None,
                },
            }
        }
        StaticValue::Number(NumericValue::Float(_)) => None,
    }
}

fn static_expression_contents(
    evaluation: &crate::tcl_expr_eval::FoldEvaluation,
    environment: &StaticEnv,
    config: Option<tcl_lexer::LexerConfig>,
) -> Option<StaticValue> {
    use crate::tcl_expr_eval::NativeExpressionResultDependency as Dependency;
    match &evaluation.result_dependency {
        Some(Dependency::StringResult { value }) => {
            Some(StaticValue::String(value.bytes.as_str().into()))
        }
        Some(Dependency::SelectedOperand {
            reference,
            existing_value,
            ..
        }) => {
            if let Some(value) = existing_value {
                return Some(StaticValue::String(value.bytes.as_str().into()));
            }
            let name =
                tcl_syntax::naming::whole_scalar_variable_reference_name(reference, config?)?;
            environment.get(name).cloned()
        }
        None => match evaluation.value {
            TclValue::Int(value) => Some(StaticValue::int(value)),
            TclValue::Float(value) if value.is_finite() => Some(StaticValue::float(value)),
            _ => None,
        },
    }
}

/// Execute one IR statement in the simulator, updating `env`.
///
/// Returns `true` when the statement is in the supported subset;
/// `false` when it should abort the whole summarisation (call,
/// barrier, unhandled structured form, etc.).
fn exec_statement(
    stmt: &Statement,
    source: &Script,
    env: &mut StaticEnv,
    simulation: StaticSimulation<'_>,
) -> bool {
    match stmt {
        Statement::AssignConst { name, value, .. } => {
            env.insert(name.clone(), StaticValue::String(value.as_str().into()));
            true
        }
        Statement::AssignExpr {
            name,
            expr,
            expr_base,
            ..
        } => {
            let bindings =
                crate::math_function_binding::ExpressionMathBindings::new(source, *expr_base);
            match simulation.evaluate_contents(expr, env, Some(bindings)) {
                Some((value, _)) => {
                    env.insert(name.clone(), value);
                    true
                }
                None => false,
            }
        }
        Statement::AssignValue { name, value, .. } => {
            let Some(argument) = crate::registry_invocation::original_source_argument_for_script(
                source, stmt, 1, value,
            ) else {
                return false;
            };
            let value = argument
                .literal_data()
                .map(|text| StaticValue::String(text.into()))
                .or_else(|| env.get(argument.scalar_reference_name()?).cloned());
            let Some(value) = value else {
                return false;
            };
            env.insert(name.clone(), value);
            true
        }
        Statement::Incr {
            name,
            amount,
            amount_braced,
            ..
        } => exec_increment(
            stmt,
            source,
            env,
            simulation,
            name,
            amount.as_deref(),
            *amount_braced,
        ),
        Statement::If {
            clauses, else_body, ..
        } => exec_if(source, clauses, else_body.as_ref(), env, simulation),
        Statement::Switch {
            subject,
            arms,
            default_body,
            mode,
            nocase,
            ..
        } => {
            if simulation.source.is_some() {
                exec_original_switch(source, stmt, env, simulation)
            } else if *mode == SwitchMode::Exact && !nocase {
                exec_switch(subject, arms, default_body.as_ref(), env, simulation)
            } else {
                false
            }
        }
        // Calls, barriers, returns, loops (other than the
        // top-level summarised `for`) — out of supported subset.
        _ => false,
    }
}

fn exec_increment(
    stmt: &Statement,
    source: &Script,
    env: &mut StaticEnv,
    simulation: StaticSimulation<'_>,
    name: &str,
    amount: Option<&str>,
    amount_braced: bool,
) -> bool {
    let Some(b) = env
        .get(name)
        .and_then(|value| static_integer(value, simulation.policy))
    else {
        return false;
    };
    let amt = match amount {
        None => 1,
        Some(text) => {
            let operand = if amount_braced {
                StaticValue::String(text.into())
            } else if let Some(argument) =
                crate::registry_invocation::original_source_argument_for_script(
                    source, stmt, 1, text,
                )
            {
                let value = argument
                    .literal_data()
                    .map(|text| StaticValue::String(text.into()))
                    .or_else(|| env.get(argument.scalar_reference_name()?).cloned());
                let Some(value) = value else {
                    return false;
                };
                value
            } else {
                StaticValue::String(text.into())
            };
            let Some(amount) = static_integer(&operand, simulation.policy) else {
                return false;
            };
            amount
        }
    };
    let sum = match simulation.policy.arithmetic {
        Some(
            policy @ (tcl_dialect::NativeArithmetic::Tcl84Wide
            | tcl_dialect::NativeArithmetic::JimWide),
        ) => tcl_syntax::expr::wide::binary(policy, crate::expr_ast::BinOp::Add, b, amt).ok(),
        _ => b.checked_add(amt),
    };
    let Some(sum) = sum else {
        return false;
    };
    if matches!(simulation.purpose, SimulationPurpose::Execution) {
        return false;
    }
    let order = simulation.evaluation_order.get();
    let Some(next_order) = order.checked_add(1) else {
        return false;
    };
    simulation.evaluation_order.set(next_order);
    simulation
        .increments
        .borrow_mut()
        .push(StaticIncrementObligation {
            evaluation_order: order,
            statement: stmt.clone(),
        });
    env.insert(name.to_owned(), StaticValue::int(sum));
    true
}

fn exec_script(script: &Script, env: &mut StaticEnv, simulation: StaticSimulation<'_>) -> bool {
    for stmt in &script.statements {
        if !exec_statement(stmt, script, env, simulation) {
            return false;
        }
    }
    true
}

fn exec_if(
    source: &Script,
    clauses: &[IfClause],
    else_body: Option<&Script>,
    env: &mut StaticEnv,
    simulation: StaticSimulation<'_>,
) -> bool {
    for clause in clauses {
        let bindings = crate::math_function_binding::ExpressionMathBindings::new(
            source,
            clause.condition_base,
        );
        let Some(cond) = simulation.evaluate(&clause.condition, env, Some(bindings)) else {
            return false;
        };
        if cond != 0 {
            return exec_script(&clause.body, env, simulation);
        }
    }
    match else_body {
        None => true,
        Some(body) => exec_script(body, env, simulation),
    }
}

fn exec_original_switch(
    source: &Script,
    statement: &Statement,
    environment: &mut StaticEnv,
    simulation: StaticSimulation<'_>,
) -> bool {
    let Statement::Switch {
        arms,
        default_body,
        subject,
        subject_braced,
        mode,
        nocase,
        ..
    } = statement
    else {
        return false;
    };
    let Some(case) = simulation
        .source
        .and_then(|source_owner| source_owner.case(source, statement))
    else {
        return false;
    };
    if *mode != SwitchMode::Exact
        || *nocase
        || !case.matches_subject_presentation(subject, *subject_braced)
        || case.mode() != tcl_registry::spec::CaseMatchMode::Exact
        || case.folds_case()
        || case.body_operands().selection_unknown
        || !case
            .patterns()
            .iter()
            .map(String::as_str)
            .eq(arms.iter().map(|arm| arm.pattern.as_str()))
        || !case.matches_statement_bodies(arms, default_body.as_ref())
        || matches!(simulation.purpose, SimulationPurpose::Execution)
    {
        return false;
    }
    let subject = case.subject_data().map(str::to_owned).or_else(|| {
        let value = environment.get(case.scalar_subject_name()?)?;
        format_constant_with_policy(value, simulation.policy)
    });
    let Some(subject) = subject.filter(|value| !value.contains('\0')) else {
        return false;
    };
    let mut fallthrough = false;
    for (pattern, arm) in case.patterns().iter().zip(arms) {
        if *pattern != subject && !fallthrough {
            continue;
        }
        if let Some(body) = &arm.body {
            return exec_script(body, environment, simulation);
        }
        fallthrough = true;
    }
    default_body
        .as_ref()
        .is_none_or(|body| exec_script(body, environment, simulation))
}

fn exec_switch(
    subject: &str,
    arms: &[SwitchArm],
    default_body: Option<&Script>,
    env: &mut StaticEnv,
    simulation: StaticSimulation<'_>,
) -> bool {
    let Some(subject_value) = resolve_switch_subject(subject, env, simulation.policy) else {
        return false;
    };
    let mut pending_fallthrough = false;
    let mut selected_body: Option<&Script> = None;
    for arm in arms {
        let pattern = resolve_switch_pattern(&arm.pattern);
        let matches = pattern == subject_value;
        if !(matches || pending_fallthrough) {
            continue;
        }
        if let Some(body) = arm.body.as_ref() {
            selected_body = Some(body);
            break;
        }
        pending_fallthrough = true;
    }
    let body = selected_body.or(default_body);
    match body {
        None => true,
        Some(b) => exec_script(b, env, simulation),
    }
}

// For-loop summarisation

#[derive(Clone, Copy)]
struct StaticForCondition<'a> {
    source: Option<StaticLoopSource<'a>>,
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
        source: condition.source,
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

/// Analysis retains original case/source ownership without licensing loop erasure.
pub(crate) fn analyse_for_statement_with_source_dependencies(
    statement: &Statement,
    initial: &StaticEnv,
    maximum: u64,
    policy: FoldPolicy,
    bindings: Option<crate::math_function_binding::ExpressionMathBindings<'_>>,
    source: StaticLoopSource<'_>,
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
            source: (!source.metadata.is_standalone()).then_some(source),
            purpose: SimulationPurpose::Analysis,
            expression: condition,
            bindings,
        },
        next,
        body,
        initial,
        maximum,
        policy,
    )
}

/// Supplied execution keeps the source owner; readonly case syntax cannot
/// stand in for a retained Native Switch execution obligation.
pub(crate) fn summarise_for_statement_with_source_dependencies(
    statement: &Statement,
    initial: &StaticEnv,
    maximum: u64,
    policy: FoldPolicy,
    bindings: Option<crate::math_function_binding::ExpressionMathBindings<'_>>,
    source: StaticLoopSource<'_>,
) -> Option<StaticLoopSummary> {
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
            source: (!source.metadata.is_standalone()).then_some(source),
            purpose: SimulationPurpose::Execution,
            expression: condition,
            bindings,
        },
        next,
        body,
        initial,
        maximum,
        policy,
    )
    .map(|analysis| StaticLoopSummary {
        values: analysis.values,
        required_math_invocations: analysis.required_math_invocations,
        required_expression_preparations: analysis.required_expression_preparations,
    })
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
            source: None,
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
            source: None,
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
mod tests;
