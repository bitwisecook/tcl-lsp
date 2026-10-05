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

use std::collections::{BTreeMap, BTreeSet, HashMap};

use tcl_registry::CommandRegistry;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::value_transfer::{
    Budget, BudgetLimit, CompletionOutcome, DeclineReason, ExactValue, ExitRule, IterationPlan,
    LoopStep, PlaceKind, StoreOutcome,
};

use crate::expr_ast::ExprNode;
use crate::ir::{IfClause, Script, Statement};
use crate::naming::normalise_var_name;
use crate::tcl_expr_eval::{Env, EnvValue, FoldPolicy, TclValue, eval_tcl_expr_with_policy};
use crate::value_shapes::is_static_var_word;
use crate::value_transfer::{LatticeDriver, StateStep};

/// Default cap on iteration count — beyond this we give up and
/// return `None` rather than simulate any further.
pub const DEFAULT_MAX_STATIC_LOOP_ITERS: u64 = 4096;

/// A value the static simulator tracks: integer, float, boolean,
/// or string.
#[derive(Debug, Clone, PartialEq)]
pub enum StaticValue {
    /// Integer.
    Int(i64),
    /// Floating-point.
    Float(f64),
    /// Boolean (maps to integer 0/1 on numeric contexts).
    Bool(bool),
    /// String.
    Str(String),
}

impl StaticValue {
    fn to_env_value(&self) -> EnvValue {
        match self {
            Self::Int(i) => EnvValue::Int(*i),
            Self::Float(f) => EnvValue::Float(*f),
            Self::Bool(b) => EnvValue::Int(i64::from(*b)),
            Self::Str(s) => EnvValue::Str(s.clone()),
        }
    }
}

/// Environment tracked by the simulator — variable name → current
/// constant value.
pub type StaticEnv = HashMap<String, StaticValue>;

/// What a simulation evaluates under: the fold policy for its expressions
/// and the registry whose declared routes evaluate its cell updates.
#[derive(Clone, Copy)]
pub struct LoopSemantics<'a> {
    /// The expression fold policy.
    pub policy: FoldPolicy,
    /// The registry the cell updates resolve against.
    pub registry: &'a CommandRegistry,
}

fn env_as_tcl_env(env: &StaticEnv) -> Env {
    env.iter()
        .map(|(k, v)| (k.clone(), v.to_env_value()))
        .collect()
}

/// Parse a literal text as [`StaticValue`]: an integer under the one
/// value-ingress rule (`ExactValue::from_literal` — the canonical spelling
/// round-trips, so `010` and `+5` stay text for the release's numeral
/// grammar to read), then `true`/`false`, then the exact text.
#[must_use]
pub fn parse_literal_value(text: &str) -> StaticValue {
    if let Some(i) = tcl_registry::value_transfer::ExactValue::from_literal(text).as_int() {
        return StaticValue::Int(i);
    }
    match text.trim().to_ascii_lowercase().as_str() {
        "true" => return StaticValue::Bool(true),
        "false" => return StaticValue::Bool(false),
        _ => {}
    }
    StaticValue::Str(text.to_owned())
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
    let tcl_env = env_as_tcl_env(env);
    let v = eval_tcl_expr_with_policy(expr, &tcl_env, policy)?;
    match v {
        TclValue::Int(i) => Some(i),
        // A beyond-wide loop bound is pathological — decline static
        // unrolling rather than saturate.
        TclValue::Big(_) => None,
        TclValue::Float(f) => {
            if !f.is_finite() {
                return None;
            }
            if f.fract() == 0.0 {
                // Finite integral float: saturate to the `i64` range, an
                // acceptable cap for loop-bound evaluation.
                Some(saturating_f64_to_i64(f))
            } else {
                None
            }
        }
    }
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

/// Extract a simple variable reference. `$name` / `${name}` with
/// no surrounding text → `Some(normalised_name)`; anything else
/// → `None`.
#[must_use]
pub fn simple_var_ref(text: &str) -> Option<String> {
    let stripped = text.trim();
    let name = if let Some(inner) = stripped
        .strip_prefix("${")
        .and_then(|s| s.strip_suffix('}'))
    {
        if !is_static_var_word(inner) {
            return None;
        }
        inner
    } else {
        let rest = stripped.strip_prefix('$')?;
        if !is_static_var_word(rest) {
            return None;
        }
        rest
    };
    Some(normalise_var_name(name).to_owned())
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

/// The value a `switch` subject holds in the simulator: a braced word's
/// own text, a lone `$name` read from the environment, or a word with no
/// substitution as written.
fn resolve_switch_subject(text: &str, braced: bool, env: &StaticEnv) -> Option<String> {
    if braced {
        return Some(text.to_owned());
    }
    let stripped = text.trim();
    if stripped.contains('$') || stripped.contains('[') {
        let name = simple_var_ref(stripped)?;
        let v = env.get(&name)?;
        return Some(match v {
            StaticValue::Int(i) => i.to_string(),
            StaticValue::Float(f) => f.to_string(),
            StaticValue::Bool(b) => (if *b { "1" } else { "0" }).to_string(),
            StaticValue::Str(s) => s.clone(),
        });
    }
    Some(strip_word_delimiters(stripped))
}

// Simulator

/// Execute one IR statement in the simulator, updating `env`.
///
/// Returns `true` when the statement is in the supported subset;
/// `false` when it should abort the whole summarisation (call,
/// barrier, unhandled structured form, etc.).
fn exec_statement(stmt: &Statement, env: &mut StaticEnv, semantics: LoopSemantics<'_>) -> bool {
    let policy = semantics.policy;
    match stmt {
        Statement::AssignConst { name, value, .. } => {
            env.insert(name.clone(), parse_literal_value(value));
            true
        }
        Statement::AssignExpr { name, expr, .. } => {
            match evaluate_expr_with_constants(expr, env, policy) {
                Some(v) => {
                    env.insert(name.clone(), StaticValue::Int(v));
                    true
                }
                None => false,
            }
        }
        Statement::AssignValue { name, value, .. } => {
            if value.contains('[') {
                return false;
            }
            if let Some(var) = simple_var_ref(value) {
                let Some(existing) = env.get(&var).cloned() else {
                    return false;
                };
                env.insert(name.clone(), existing);
                return true;
            }
            env.insert(name.clone(), parse_literal_value(value));
            true
        }
        // The typed cell update: the registry's declared route evaluates
        // it over the environment, under the same target semantics as the
        // lattice — one increment model, not a second one here.
        Statement::Incr {
            name,
            amount,
            amount_braced,
            ..
        } => crate::value_transfer::exec_cell_update_in_env(
            semantics.registry,
            policy.dialect,
            name,
            amount.as_deref().map(|text| (text, *amount_braced)),
            env,
        ),
        Statement::If {
            clauses, else_body, ..
        } => exec_if(clauses, else_body.as_ref(), env, semantics),
        Statement::Switch { .. } => exec_switch(stmt, env, semantics),
        // Calls, barriers, returns, loops (other than the
        // top-level summarised `for`) — out of supported subset.
        _ => false,
    }
}

fn exec_script(script: &Script, env: &mut StaticEnv, semantics: LoopSemantics<'_>) -> bool {
    for stmt in &script.statements {
        if !exec_statement(stmt, env, semantics) {
            return false;
        }
    }
    true
}

fn exec_if(
    clauses: &[IfClause],
    else_body: Option<&Script>,
    env: &mut StaticEnv,
    semantics: LoopSemantics<'_>,
) -> bool {
    let policy = semantics.policy;
    for clause in clauses {
        let Some(cond) = evaluate_expr_with_constants(&clause.condition, env, policy) else {
            return false;
        };
        if cond != 0 {
            return exec_script(&clause.body, env, semantics);
        }
    }
    match else_body {
        None => true,
        Some(body) => exec_script(body, env, semantics),
    }
}

/// Run a `switch` over the environment: the registry's selection for the
/// statement's own command and options, with the subject holding the value
/// the environment gives it, chooses the body — mode, case folding and
/// fall-through as the command reads them. A selection the registry does not
/// make (a pattern the environment cannot state, an error the command
/// raises) ends the simulation.
fn exec_switch(stmt: &Statement, env: &mut StaticEnv, semantics: LoopSemantics<'_>) -> bool {
    let Statement::Switch {
        subject,
        subject_braced,
        arms,
        default_body,
        ..
    } = stmt
    else {
        return false;
    };
    let Some(value) = resolve_switch_subject(subject, *subject_braced, env) else {
        return false;
    };
    let Some(fact) = crate::value_transfer::statement_selection(semantics.registry, stmt, &value)
    else {
        return false;
    };
    let ([body], [writes]) = (fact.bodies.as_slice(), fact.writes.as_slice()) else {
        return false;
    };
    if !writes.is_empty() {
        return false;
    }
    let chosen = match body {
        None => return true,
        Some(arm) if *arm == arms.len() => default_body.as_ref(),
        Some(arm) => arms.get(*arm).and_then(|arm| arm.body.as_ref()),
    };
    chosen.is_some_and(|script| exec_script(script, env, semantics))
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
    semantics: LoopSemantics<'_>,
) -> Option<StaticEnv> {
    let mut env: StaticEnv = initial_constants.clone();
    let policy = semantics.policy;

    if !exec_script(init, &mut env, semantics) {
        return None;
    }
    let mut iterations: u64 = 0;
    loop {
        let cond = evaluate_expr_with_constants(condition, &env, policy)?;
        if cond == 0 {
            break;
        }
        iterations += 1;
        if iterations > max_iterations {
            return None;
        }
        if !exec_script(body, &mut env, semantics) {
            return None;
        }
        if !exec_script(next_script, &mut env, semantics) {
            return None;
        }
    }
    Some(env)
}

/// Convenience entry point that extracts the init/condition/next/
/// body from a [`Statement::For`] and forwards to
/// [`summarise_static_for`].
#[must_use]
pub fn summarise_for_statement(
    stmt: &Statement,
    initial_constants: &StaticEnv,
    max_iterations: u64,
    semantics: LoopSemantics<'_>,
) -> Option<StaticEnv> {
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
    summarise_static_for(
        init,
        condition,
        next,
        body,
        initial_constants,
        max_iterations,
        semantics,
    )
}

// Bounded-loop enumeration

/// What a loop enumeration holds of one place: bound to an exact value, a
/// scalar whose value it does not know, or not bound.
#[derive(Debug, Clone, PartialEq)]
pub enum Slot {
    /// Bound to this exact value.
    Value(ExactValue),
    /// Bound to a scalar whose value the enumeration does not know: a store
    /// replaces it, and a read of it declines.
    Scalar,
    /// Not bound.
    Unbound,
}

/// The state a loop enumeration runs over: what it holds of each place it
/// knows. A place it does not hold is one it knows nothing of — it may be an
/// array — and a read of one, or a store to one, declines the enumeration.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LoopState {
    slots: BTreeMap<String, Slot>,
}

impl LoopState {
    /// What the state holds of `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Slot> {
        self.slots.get(name)
    }

    /// Hold `slot` for `name`.
    pub fn set(&mut self, name: String, slot: Slot) {
        self.slots.insert(name, slot);
    }

    /// Every place the state holds, in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Slot)> {
        self.slots.iter()
    }
}

/// One loop run to its exit over exact state
/// (`docs/design/compiler/value-transfers.md` § *Bounded-loop enumeration*):
/// the plan the registry declares for it, what it found and left of each
/// place it wrote, how many passes it ran and how it left.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopEnumeration {
    /// The loop's iteration plan.
    pub plan: IterationPlan,
    /// What each place the loop wrote held before it ran.
    pub entry: Vec<(String, Slot)>,
    /// The passes it ran.
    pub iterations: u64,
    /// How it left: exhaustion, a false condition, `break`, or a completion
    /// the plan does not absorb.
    pub exit: ExitRule,
    /// What each place the loop wrote holds where it left.
    pub state: Vec<(String, Slot)>,
    /// How the loop completed: normally, or with the completion the plan does
    /// not absorb (an error, a `return`), whose state holds only on that path.
    pub completion: CompletionOutcome,
}

/// One enumeration's context: the run's driver, which places another actor
/// may write, the budget every statement of every pass is charged to, the
/// passes left, and every place written.
struct Enumerator<'a> {
    driver: &'a LatticeDriver<'a>,
    external: &'a dyn Fn(&str) -> bool,
    budget: Budget,
    passes_left: u64,
    written: BTreeSet<String>,
    /// The head each typed statement kind resolves through, once asked.
    heads: HashMap<LoweringHookId, Option<String>>,
}

/// Run the loop `stmt` — a `for`, `while` or `foreach` statement — to its exit
/// over `state`, its every statement evaluated by the registry's routes through
/// `driver`, under at most `max_iterations` passes in all, a nested loop's
/// included. `state` holds what the loop finds before it runs, and for a `for`
/// what its start script left when `start_ran` (the CFG runs it ahead of the
/// loop). `external` names the places another actor may write: the enumeration
/// writes none. A pass past the cap is the `Iterations` budget decline, and
/// nothing is published; so is any statement no route evaluates exactly.
pub(crate) fn enumerate_loop(
    driver: &LatticeDriver<'_>,
    stmt: &Statement,
    mut state: LoopState,
    (start_ran, max_iterations): (bool, u64),
    external: &dyn Fn(&str) -> bool,
) -> Result<LoopEnumeration, DeclineReason> {
    let mut enumerator = Enumerator {
        driver,
        external,
        budget: driver.enumeration_budget(),
        passes_left: max_iterations,
        written: BTreeSet::new(),
        heads: HashMap::new(),
    };
    let before = state.clone();
    let run = enumerator.run_loop(stmt, &mut state, start_ran)?;
    let mut entry = Vec::new();
    let mut after = Vec::new();
    for name in &enumerator.written {
        if let (Some(found), Some(left)) = (before.get(name), state.get(name)) {
            entry.push((name.clone(), found.clone()));
            after.push((name.clone(), left.clone()));
        }
    }
    Ok(LoopEnumeration {
        plan: run.plan,
        entry,
        iterations: run.iterations,
        exit: run.exit,
        state: after,
        completion: run.completion,
    })
}

/// What a script left when an enumeration ran it whole
/// ([`enumerate_script`]): the state, every place it wrote, and how it
/// completed.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptRun {
    /// The state it left.
    pub state: LoopState,
    /// Every place it wrote.
    pub written: BTreeSet<String>,
    /// How it completed.
    pub completion: CompletionOutcome,
}

/// Run `script` to its end over `state` — the body of an opaque `catch`,
/// whose completion the `catch` absorbs — as a loop's passes are run, under
/// at most `max_iterations` passes of the loops it holds: what it leaves,
/// written where it stopped when it completed otherwise.
pub(crate) fn enumerate_script(
    driver: &LatticeDriver<'_>,
    script: &Script,
    mut state: LoopState,
    max_iterations: u64,
    external: &dyn Fn(&str) -> bool,
) -> Result<ScriptRun, DeclineReason> {
    let mut enumerator = Enumerator {
        driver,
        external,
        budget: driver.enumeration_budget(),
        passes_left: max_iterations,
        written: BTreeSet::new(),
        heads: HashMap::new(),
    };
    let completion = enumerator.exec_script(script, &mut state)?;
    Ok(ScriptRun {
        state,
        written: enumerator.written,
        completion,
    })
}

/// What one loop's run left: its plan, the passes, the exit and the
/// completion.
struct LoopRun {
    plan: IterationPlan,
    iterations: u64,
    exit: ExitRule,
    completion: CompletionOutcome,
}

impl Enumerator<'_> {
    /// Run one loop statement over `state` to its exit.
    fn run_loop(
        &mut self,
        stmt: &Statement,
        state: &mut LoopState,
        start_ran: bool,
    ) -> Result<LoopRun, DeclineReason> {
        let (hook, words) = loop_words(stmt)?;
        let head = self.typed_head(hook)?;
        let borrowed: Vec<(&str, bool)> = words
            .iter()
            .map(|(text, braced)| (text.as_str(), *braced))
            .collect();
        let plan = self.driver.loop_plan_in_state(state, &head, &borrowed)?;
        match stmt {
            Statement::For {
                init,
                condition,
                next,
                body,
                ..
            } => {
                if !start_ran && self.exec_script(init, state)? != CompletionOutcome::Normal {
                    return Err(DeclineReason::Unsupported);
                }
                self.run_passes(plan, state, |this, state| {
                    if !this
                        .driver
                        .condition_in_state(state, condition, &mut this.budget)?
                    {
                        return Ok(Pass::Done(ExitRule::FalseCondition));
                    }
                    Ok(Pass::Body(body, Some(next)))
                })
            }
            Statement::While {
                condition, body, ..
            } => self.run_passes(plan, state, |this, state| {
                if !this
                    .driver
                    .condition_in_state(state, condition, &mut this.budget)?
                {
                    return Ok(Pass::Done(ExitRule::FalseCondition));
                }
                Ok(Pass::Body(body, None))
            }),
            Statement::Foreach {
                iterators, body, ..
            } => {
                let groups = self.list_groups(iterators, state)?;
                let passes = groups
                    .iter()
                    .map(|(vars, elements)| elements.len().div_ceil(vars.len().max(1)))
                    .max()
                    .unwrap_or(0);
                let mut pass = 0;
                self.run_passes(plan, state, |this, state| {
                    if pass == passes {
                        return Ok(Pass::Done(ExitRule::Exhaustion));
                    }
                    for (vars, elements) in &groups {
                        for (at, var) in vars.iter().enumerate() {
                            let element = elements
                                .get(pass * vars.len() + at)
                                .map_or("", String::as_str);
                            this.write(state, var, Slot::Value(ExactValue::from_literal(element)))?;
                        }
                    }
                    pass += 1;
                    Ok(Pass::Body(body, None))
                })
            }
            _ => Err(DeclineReason::Unsupported),
        }
    }

    /// Each `foreach` group's variables and the elements of its list, read
    /// over `state` under the target's list rules.
    fn list_groups(
        &self,
        iterators: &[crate::ir::ForeachIterator],
        state: &LoopState,
    ) -> Result<Vec<ListGroupRun>, DeclineReason> {
        iterators
            .iter()
            .map(|iterator| {
                if iterator.vars.is_empty() {
                    return Err(DeclineReason::WrongRepresentation);
                }
                let list =
                    self.driver
                        .word_in_state(state, &iterator.list_arg, iterator.list_braced)?;
                let text = list.as_str().map_err(|_| DeclineReason::NotText)?;
                let elements = self
                    .driver
                    .list_elements(text)
                    .ok_or(DeclineReason::WrongRepresentation)?;
                Ok((iterator.vars.clone(), elements))
            })
            .collect()
    }

    /// Run passes of a loop: `next_pass` decides whether another pass runs
    /// (and binds what it binds) and names its body and step script; the
    /// plan's completion protocol reads how each body completed.
    fn run_passes<'s>(
        &mut self,
        plan: IterationPlan,
        state: &mut LoopState,
        mut next_pass: impl FnMut(&mut Self, &mut LoopState) -> Result<Pass<'s>, DeclineReason>,
    ) -> Result<LoopRun, DeclineReason> {
        let mut iterations = 0;
        loop {
            let (body, step) = match next_pass(self, state)? {
                Pass::Done(exit) => {
                    return Ok(LoopRun {
                        plan,
                        iterations,
                        exit,
                        completion: CompletionOutcome::Normal,
                    });
                }
                Pass::Body(body, step) => (body, step),
            };
            if self.passes_left == 0 {
                return Err(DeclineReason::Budget(BudgetLimit::Iterations));
            }
            self.passes_left -= 1;
            iterations += 1;
            let completion = self.exec_script(body, state)?;
            match plan.step(&completion) {
                LoopStep::Next | LoopStep::Skip => {}
                LoopStep::Exit => {
                    return Ok(LoopRun {
                        plan,
                        iterations,
                        exit: ExitRule::Break,
                        completion: CompletionOutcome::Normal,
                    });
                }
                LoopStep::Leave => {
                    return Ok(LoopRun {
                        plan,
                        iterations,
                        exit: ExitRule::NonNormalCompletion,
                        completion,
                    });
                }
            }
            if let Some(step) = step {
                match self.exec_script(step, state)? {
                    CompletionOutcome::Normal => {}
                    // `break` in the step script ends the loop as in the body.
                    completion if plan.step(&completion) == LoopStep::Exit => {
                        return Ok(LoopRun {
                            plan,
                            iterations,
                            exit: ExitRule::Break,
                            completion: CompletionOutcome::Normal,
                        });
                    }
                    _ => return Err(DeclineReason::Unsupported),
                }
            }
        }
    }

    /// Run `script`'s statements in order until one completes otherwise:
    /// how the script completed.
    fn exec_script(
        &mut self,
        script: &Script,
        state: &mut LoopState,
    ) -> Result<CompletionOutcome, DeclineReason> {
        for stmt in &script.statements {
            let completion = self.exec_statement(stmt, state)?;
            if completion != CompletionOutcome::Normal {
                return Ok(completion);
            }
        }
        Ok(CompletionOutcome::Normal)
    }

    /// Run one statement over `state` through the registry's routes: how it
    /// completed.
    fn exec_statement(
        &mut self,
        stmt: &Statement,
        state: &mut LoopState,
    ) -> Result<CompletionOutcome, DeclineReason> {
        match stmt {
            Statement::AssignConst { .. }
            | Statement::AssignValue { .. }
            | Statement::AssignExpr { .. }
            | Statement::ExprEval { .. } => self.exec_assignment(stmt, state),
            Statement::Incr { .. } | Statement::Call { .. } => self.exec_command(stmt, state),
            Statement::If {
                clauses, else_body, ..
            } => {
                for clause in clauses {
                    if self
                        .driver
                        .condition_in_state(state, &clause.condition, &mut self.budget)?
                    {
                        return self.exec_script(&clause.body, state);
                    }
                }
                match else_body {
                    Some(body) => self.exec_script(body, state),
                    None => Ok(CompletionOutcome::Normal),
                }
            }
            Statement::Switch { .. } => self.exec_switch(stmt, state),
            Statement::For { .. } | Statement::While { .. } | Statement::Foreach { .. } => {
                Ok(self.run_loop(stmt, state, false)?.completion)
            }
            _ => Err(DeclineReason::Unsupported),
        }
    }

    /// Run an assignment or an `expr` over `state`: the value a literal, a
    /// word or the expression route gives, written through the registry's
    /// cell write.
    fn exec_assignment(
        &mut self,
        stmt: &Statement,
        state: &mut LoopState,
    ) -> Result<CompletionOutcome, DeclineReason> {
        let (name, value) = match stmt {
            Statement::AssignConst {
                name,
                name_braced,
                value,
                ..
            } => {
                let value = self
                    .driver
                    .literal_value(value, tcl_lexer::TokenType::Str)
                    .ok_or(DeclineReason::NotExact)?
                    .into_owned();
                ((name.as_str(), *name_braced), value)
            }
            Statement::AssignValue {
                name,
                name_braced,
                value,
                ..
            } => {
                let value = self.driver.word_in_state(state, value, false)?;
                let text = value.as_str().map_err(|_| DeclineReason::NotText)?;
                ((name.as_str(), *name_braced), text.to_owned())
            }
            Statement::AssignExpr {
                name,
                name_braced,
                expr,
                command_binding,
                ..
            } => {
                let head = command_binding
                    .as_ref()
                    .map(|binding| (binding.name.as_str(), binding.identity.as_str()));
                match self
                    .driver
                    .expression_in_state(state, expr, head, &mut self.budget)?
                {
                    Ok(value) => {
                        let text = value.as_str().map_err(|_| DeclineReason::NotText)?;
                        ((name.as_str(), *name_braced), text.to_owned())
                    }
                    Err(completion) => return Ok(completion),
                }
            }
            Statement::ExprEval {
                expr,
                command_binding,
                ..
            } => {
                let head = Some((
                    command_binding.name.as_str(),
                    command_binding.identity.as_str(),
                ));
                return Ok(self
                    .driver
                    .expression_in_state(state, expr, head, &mut self.budget)?
                    .err()
                    .unwrap_or(CompletionOutcome::Normal));
            }
            _ => return Err(DeclineReason::Unsupported),
        };
        self.assign(state, name, (&value, true))
    }

    /// Run an `incr` or a command over `state` through the route its head
    /// resolves to, applying the stores it makes.
    fn exec_command(
        &mut self,
        stmt: &Statement,
        state: &mut LoopState,
    ) -> Result<CompletionOutcome, DeclineReason> {
        let step = match stmt {
            Statement::Incr {
                name,
                amount,
                amount_braced,
                ..
            } => {
                let head = self.typed_head(LoweringHookId::Incr)?;
                let mut words = vec![(name.as_str(), true)];
                words.extend(amount.as_deref().map(|text| (text, *amount_braced)));
                self.driver
                    .invoke_in_state(state, &head, &words, &mut self.budget)?
            }
            Statement::Call {
                command,
                canonical_command,
                args,
                tokens,
                ..
            } => {
                let head = canonical_command.as_deref().unwrap_or(command);
                let words: Vec<(&str, bool)> = args
                    .iter()
                    .enumerate()
                    .map(|(at, text)| {
                        let braced = tokens
                            .as_ref()
                            .is_some_and(|tokens| tokens.arg_is_braced_literal(at));
                        (text.as_str(), braced)
                    })
                    .collect();
                self.driver
                    .invoke_in_state(state, head, &words, &mut self.budget)?
            }
            _ => return Err(DeclineReason::Unsupported),
        };
        self.apply(state, step)
    }

    /// Run a `switch` over `state`: the registry's selection for the
    /// statement's own command and options, the subject holding the value
    /// the state gives its word, chooses the body.
    fn exec_switch(
        &mut self,
        stmt: &Statement,
        state: &mut LoopState,
    ) -> Result<CompletionOutcome, DeclineReason> {
        let Statement::Switch {
            subject,
            subject_braced,
            arms,
            default_body,
            ..
        } = stmt
        else {
            return Err(DeclineReason::Unsupported);
        };
        let value = self.driver.word_in_state(state, subject, *subject_braced)?;
        let text = value.as_str().map_err(|_| DeclineReason::NotText)?;
        let fact = self
            .driver
            .switch_selection(stmt, text)
            .ok_or(DeclineReason::NotExact)?;
        let ([body], [writes]) = (fact.bodies.as_slice(), fact.writes.as_slice()) else {
            return Err(DeclineReason::NotExact);
        };
        if !writes.is_empty() {
            return Err(DeclineReason::Unsupported);
        }
        let chosen = match body {
            None => return Ok(CompletionOutcome::Normal),
            Some(arm) if *arm == arms.len() => default_body.as_ref(),
            Some(arm) => arms.get(*arm).and_then(|arm| arm.body.as_ref()),
        };
        match chosen {
            Some(script) => self.exec_script(script, state),
            None => Err(DeclineReason::NotExact),
        }
    }

    /// `set name value` through the registry's cell write.
    fn assign(
        &mut self,
        state: &mut LoopState,
        name: (&str, bool),
        value: (&str, bool),
    ) -> Result<CompletionOutcome, DeclineReason> {
        let head = self.typed_head(LoweringHookId::Set)?;
        let step = self
            .driver
            .invoke_in_state(state, &head, &[name, value], &mut self.budget)?;
        self.apply(state, step)
    }

    /// Apply what an invocation did to `state`: each store its completion ran,
    /// in order, to a scalar place no other actor may write; and how it
    /// completed.
    fn apply(
        &mut self,
        state: &mut LoopState,
        step: StateStep,
    ) -> Result<CompletionOutcome, DeclineReason> {
        let ran = match &step.completion {
            CompletionOutcome::Error { written, .. } => *written,
            _ => step.stores.len(),
        };
        for (place, store) in step.stores.into_iter().take(ran) {
            if !matches!(place.kind, PlaceKind::Scalar) || (self.external)(&place.name) {
                return Err(DeclineReason::EscapingPlace);
            }
            match store {
                StoreOutcome::Write { value, .. } => {
                    self.write(state, &place.name, Slot::Value(value))?;
                }
                StoreOutcome::Unbind { .. } => self.write(state, &place.name, Slot::Unbound)?,
                StoreOutcome::Preserve { .. } => {}
                _ => return Err(DeclineReason::NotExact),
            }
        }
        Ok(step.completion)
    }

    /// The head the typed statement of `hook` resolves through, asked of the
    /// driver once per enumeration.
    fn typed_head(&mut self, hook: LoweringHookId) -> Result<String, DeclineReason> {
        let driver = self.driver;
        self.heads
            .entry(hook)
            .or_insert_with(|| driver.enumeration_typed_head(hook).map(str::to_owned))
            .clone()
            .ok_or(DeclineReason::RebindingSuspected)
    }

    /// Hold `slot` for `name`, a place no other actor may write and whose
    /// kind the state proves: one it holds a scalar of, or holds unbound. A
    /// place it does not hold may be an array, on which a scalar store raises
    /// after the stores before it, so the enumeration declines.
    fn write(
        &mut self,
        state: &mut LoopState,
        name: &str,
        slot: Slot,
    ) -> Result<(), DeclineReason> {
        if (self.external)(name) {
            return Err(DeclineReason::EscapingPlace);
        }
        if state.get(name).is_none() {
            return Err(DeclineReason::NotExact);
        }
        self.written.insert(name.to_owned());
        state.set(name.to_owned(), slot);
        Ok(())
    }
}

/// One `foreach` group as a run reads it: its variables, and the elements of
/// its list.
type ListGroupRun = (Vec<String>, Vec<String>);

/// What one pass of a loop runs: its body and, for a `for`, its step script —
/// or the exit the loop takes before the pass.
enum Pass<'s> {
    Body(&'s Script, Option<&'s Script>),
    Done(ExitRule),
}

/// The lowering hook a loop statement is the typed form of, and its source
/// words with whether each was braced, as its plan reads them.
fn loop_words(stmt: &Statement) -> Result<(LoweringHookId, Vec<(String, bool)>), DeclineReason> {
    match stmt {
        Statement::For { raw_args, .. } | Statement::While { raw_args, .. } => {
            let hook = if matches!(stmt, Statement::For { .. }) {
                LoweringHookId::For
            } else {
                LoweringHookId::While
            };
            Ok((
                hook,
                raw_args.iter().map(|text| (text.clone(), true)).collect(),
            ))
        }
        Statement::Foreach {
            iterators,
            raw_args,
            is_dict_iteration: false,
            is_array_iteration: false,
            ..
        } => {
            let mut words: Vec<(String, bool)> = Vec::new();
            for iterator in iterators {
                words.push((tcl_syntax::list::join_list(&iterator.vars), true));
                words.push((iterator.list_arg.clone(), iterator.list_braced));
            }
            words.push((raw_args.last().cloned().unwrap_or_default(), true));
            Ok((LoweringHookId::Foreach, words))
        }
        _ => Err(DeclineReason::Unsupported),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr_parser::parse_expr;
    use crate::ir::{SwitchArm, SwitchMode};
    use tcl_lexer::Span;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

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

    /// The simulator's `incr` is the registry's route: a leading-zero
    /// counter reads as octal up to 8.6 and decimal from 9.0, and a profile
    /// naming no release (the lenient `tcl`) cannot simulate it at all.
    /// `f5-irules` counts as its declared 8.4 base does (ruling 8): tclsh
    /// 8.4 to 9.1 all end the loop at 20.
    #[test]
    fn simulated_incr_reads_the_counter_under_the_release() {
        let for_stmt = Statement::For {
            span: sp(),
            init: script_of(vec![assign_const("i", "010")]),
            init_span: sp(),
            condition: parse_expr("$i < 20", None),
            condition_span: sp(),
            next: script_of(vec![incr("i", None)]),
            next_span: sp(),
            body: empty_script(),
            body_span: sp(),
            raw_args: Vec::new(),
            raw_tokens: None,
            condition_base: None,
        };
        let under = |dialect: &str| {
            let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
            summarise_for_statement(
                &for_stmt,
                &StaticEnv::new(),
                100,
                LoopSemantics {
                    policy: FoldPolicy::from_registry(registry),
                    registry,
                },
            )
            .map(|env| env.get("i").cloned())
        };
        assert_eq!(under("tcl8.6"), Some(Some(StaticValue::Int(20))));
        assert_eq!(under("tcl9.0"), Some(Some(StaticValue::Int(20))));
        assert_eq!(under("f5-irules"), Some(Some(StaticValue::Int(20))));
        assert_eq!(under("tcl"), None, "no release: the counter is ambiguous");
    }

    /// The literal ingress is the one rule: a leading-zero or signed
    /// numeral stays text for the release's grammar to read, and nothing
    /// is trimmed.
    #[test]
    fn parse_literal_value_keeps_release_dependent_spellings_as_text() {
        assert_eq!(parse_literal_value("42"), StaticValue::Int(42));
        assert_eq!(parse_literal_value("-7"), StaticValue::Int(-7));
        assert_eq!(parse_literal_value("010"), StaticValue::Str("010".into()));
        assert_eq!(parse_literal_value("+5"), StaticValue::Str("+5".into()));
        assert_eq!(parse_literal_value(" 5"), StaticValue::Str(" 5".into()));
        assert_eq!(parse_literal_value("true"), StaticValue::Bool(true));
        assert_eq!(parse_literal_value(" a "), StaticValue::Str(" a ".into()));
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
    fn summarise_counts_iterations_to_five() {
        // for {set i 0} {$i < 5} {incr i} { /* nothing */ }
        let init = script_of(vec![assign_const("i", "0")]);
        let cond = parse_expr("$i < 5", None);
        let next_script = script_of(vec![incr("i", None)]);
        let body = empty_script();
        let env = summarise_static_for(
            &init,
            &cond,
            &next_script,
            &body,
            &StaticEnv::new(),
            1000,
            LoopSemantics {
                policy: FoldPolicy::default(),
                registry: &registry(),
            },
        )
        .expect("summarised");
        assert_eq!(env.get("i"), Some(&StaticValue::Int(5)));
    }

    #[test]
    fn summarise_body_accumulates_counter() {
        // for {set i 0; set total 0} {$i < 3} {incr i} { incr total }
        let init = script_of(vec![assign_const("i", "0"), assign_const("total", "0")]);
        let cond = parse_expr("$i < 3", None);
        let next_script = script_of(vec![incr("i", None)]);
        let body = script_of(vec![incr("total", None)]);
        let env = summarise_static_for(
            &init,
            &cond,
            &next_script,
            &body,
            &StaticEnv::new(),
            1000,
            LoopSemantics {
                policy: FoldPolicy::default(),
                registry: &registry(),
            },
        )
        .expect("summarised");
        assert_eq!(env.get("total"), Some(&StaticValue::Int(3)));
        assert_eq!(env.get("i"), Some(&StaticValue::Int(3)));
    }

    #[test]
    fn summarise_respects_iteration_cap() {
        let init = script_of(vec![assign_const("i", "0")]);
        let cond = parse_expr("$i < 10000", None);
        let next_script = script_of(vec![incr("i", None)]);
        let body = empty_script();
        let result = summarise_static_for(
            &init,
            &cond,
            &next_script,
            &body,
            &StaticEnv::new(),
            100,
            LoopSemantics {
                policy: FoldPolicy::default(),
                registry: &registry(),
            },
        );
        assert!(result.is_none(), "should exceed the 100-iter cap");
    }

    #[test]
    fn summarise_unsupported_statement_returns_none() {
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
            summarise_static_for(
                &init,
                &cond,
                &next_script,
                &body,
                &StaticEnv::new(),
                1000,
                LoopSemantics {
                    policy: FoldPolicy::default(),
                    registry: &registry()
                }
            )
            .is_none()
        );
    }

    /// `switch $mode { a {set v 1} default {set v 9} }` — shared by the
    /// switch-dispatch case and its unresolvable-subject counterpart.
    fn mode_switch() -> Statement {
        mode_switch_with(&[], SwitchMode::Exact, false, "a")
    }

    /// `switch <options> $mode { <pattern> {set v 1} default {set v 9} }` as
    /// the lowering records it: the words as written with how each was
    /// delimited, beside the arms and the mode the options select.
    fn mode_switch_with(
        options: &[&str],
        mode: SwitchMode,
        nocase: bool,
        pattern: &str,
    ) -> Statement {
        let list = format!("{pattern} {{set v 1}} default {{set v 9}}");
        let raw_args: Vec<String> = options
            .iter()
            .copied()
            .chain(["$mode", list.as_str()])
            .map(str::to_owned)
            .collect();
        let mut braced = vec![false; raw_args.len()];
        if let Some(last) = braced.last_mut() {
            *last = true;
        }
        Statement::Switch {
            subject_braced: false,
            raw_arg_braced: braced,
            raw_arg_quoted: vec![false; raw_args.len()],
            command: "switch".into(),
            span: sp(),
            subject: "$mode".into(),
            subject_span: sp(),
            arms: vec![SwitchArm {
                pattern: pattern.into(),
                pattern_braced: true,
                pattern_span: sp(),
                body: Some(script_of(vec![assign_const("v", "1")])),
                body_span: Some(sp()),
                fallthrough: false,
            }],
            default_body: Some(script_of(vec![assign_const("v", "9")])),
            default_span: None,
            mode,
            nocase,
            raw_args,
            patterns_braced: true,
        }
    }

    /// What `v` holds after `switch <options> $mode …` runs once with
    /// `mode` set to `value` under Tcl 8.6, or `None` where the simulation
    /// gives up.
    fn v_after(switch: Statement, value: &str) -> Option<StaticValue> {
        let body = script_of(vec![switch]);
        let init = script_of(vec![assign_const("i", "0"), assign_const("mode", value)]);
        summarise_static_for(
            &init,
            &parse_expr("$i < 1", None),
            &script_of(vec![incr("i", None)]),
            &body,
            &StaticEnv::new(),
            1000,
            LoopSemantics {
                policy: FoldPolicy::default(),
                registry: tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            },
        )?
        .remove("v")
    }

    #[test]
    fn summarise_resolves_if_else_branch_in_body() {
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
        let env = summarise_static_for(
            &init,
            &cond,
            &next_script,
            &body,
            &StaticEnv::new(),
            1000,
            LoopSemantics {
                policy: FoldPolicy::default(),
                registry: &registry(),
            },
        )
        .expect("summarised");
        assert_eq!(env.get("i"), Some(&StaticValue::Int(3)));
        assert_eq!(env.get("x"), Some(&StaticValue::Int(20)));
    }

    #[test]
    fn summarise_resolves_switch_dispatch_in_body() {
        // for {set i 0; set mode a} {$i < 1} {incr i} { switch … } → v = 1.
        let init = script_of(vec![assign_const("i", "0"), assign_const("mode", "a")]);
        let cond = parse_expr("$i < 1", None);
        let next_script = script_of(vec![incr("i", None)]);
        let body = script_of(vec![mode_switch()]);
        let env = summarise_static_for(
            &init,
            &cond,
            &next_script,
            &body,
            &StaticEnv::new(),
            1000,
            LoopSemantics {
                policy: FoldPolicy::default(),
                registry: &registry(),
            },
        )
        .expect("summarised");
        assert_eq!(env.get("v"), Some(&StaticValue::Int(1)));
    }

    /// The simulator runs the command's own selection, so the mode its
    /// options select is honoured where it had compared every pattern as a
    /// string: `-glob` matches `a*`, `-exact` does not, `-nocase` folds
    /// case, `-regexp` runs the engine, and a `-` body supplies the next.
    #[test]
    fn summarise_honours_the_mode_of_a_switch() {
        let taken = Some(StaticValue::Int(1));
        let default = Some(StaticValue::Int(9));
        let cases = [
            (
                &["-glob", "--"][..],
                SwitchMode::Glob,
                false,
                "a*",
                "abc",
                &taken,
            ),
            (
                &["-exact", "--"],
                SwitchMode::Exact,
                false,
                "a*",
                "abc",
                &default,
            ),
            (
                &["-nocase", "--"],
                SwitchMode::Exact,
                true,
                "ABC",
                "abc",
                &taken,
            ),
            (
                &["-regexp", "--"],
                SwitchMode::Regexp,
                false,
                "^a.c$",
                "abc",
                &taken,
            ),
            (
                &["-regexp", "--"],
                SwitchMode::Regexp,
                false,
                "^b",
                "abc",
                &default,
            ),
        ];
        for (options, mode, nocase, pattern, value, expected) in cases {
            assert_eq!(
                v_after(mode_switch_with(options, mode, nocase, pattern), value),
                *expected,
                "{options:?} {pattern} against {value}"
            );
        }
    }

    /// A statement whose words the lowering did not record, or whose subject
    /// the environment cannot state, is outside the supported subset.
    #[test]
    fn summarise_bails_where_the_selection_is_not_made() {
        let mut unrecorded = mode_switch();
        if let Statement::Switch { raw_args, .. } = &mut unrecorded {
            raw_args.clear();
        }
        assert_eq!(v_after(unrecorded, "a"), None);
        // The separate-words form: a literal pattern word selects, and a
        // pattern the command would substitute is no value to match.
        let inline = |pattern: &str| {
            let mut statement = mode_switch();
            if let Statement::Switch {
                raw_args,
                raw_arg_braced,
                raw_arg_quoted,
                patterns_braced,
                ..
            } = &mut statement
            {
                *raw_args = ["$mode", pattern, "set v 1", "default", "set v 9"]
                    .map(str::to_owned)
                    .to_vec();
                *raw_arg_braced = vec![false, false, true, false, true];
                *raw_arg_quoted = vec![false; 5];
                *patterns_braced = false;
            }
            statement
        };
        assert_eq!(v_after(inline("a"), "a"), Some(StaticValue::Int(1)));
        assert_eq!(v_after(inline("$pat"), "a"), None);
    }

    #[test]
    fn summarise_bails_on_unresolvable_switch_subject() {
        // `$mode` is never set, so the subject can't resolve → summary bails.
        let init = script_of(vec![assign_const("i", "0")]);
        let cond = parse_expr("$i < 1", None);
        let next_script = script_of(vec![incr("i", None)]);
        let body = script_of(vec![mode_switch()]);
        let result = summarise_static_for(
            &init,
            &cond,
            &next_script,
            &body,
            &StaticEnv::new(),
            1000,
            LoopSemantics {
                policy: FoldPolicy::default(),
                registry: &registry(),
            },
        );
        assert!(result.is_none(), "unresolvable switch subject should bail");
    }

    #[test]
    fn summarise_forwards_for_statement_helper() {
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
        let env = summarise_for_statement(
            &for_stmt,
            &StaticEnv::new(),
            100,
            LoopSemantics {
                policy: FoldPolicy::default(),
                registry: &registry(),
            },
        )
        .expect("summarised");
        assert_eq!(env.get("i"), Some(&StaticValue::Int(2)));
    }

    // evaluate_expr_with_constants

    #[test]
    fn evaluate_expr_integer() {
        let mut env = StaticEnv::new();
        env.insert("x".into(), StaticValue::Int(5));
        assert_eq!(
            evaluate_expr_with_constants(&parse_expr("$x + 3", None), &env, FoldPolicy::default()),
            Some(8)
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
