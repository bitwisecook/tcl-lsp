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

use crate::expr_ast::{ExprNode, expr_text};
use crate::ir::{IfClause, Script, Statement, SwitchArm, SwitchMode};
use crate::naming::normalise_var_name;
use crate::tcl_expr_eval::{Env, EnvValue, TclValue, eval_tcl_expr_with_octal};

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
    octal: Option<bool>,
) -> Option<i64> {
    let tcl_env = env_as_tcl_env(env);
    let v = eval_tcl_expr_with_octal(expr, &tcl_env, octal)?;
    match v {
        TclValue::Int(i) => Some(i),
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
/// magnitudes fail to parse and saturate by sign, matching the previous
/// `f as i64` saturating-cast behaviour.
fn saturating_f64_to_i64(f: f64) -> i64 {
    // `+ 0.0` normalises `-0.0` to `0.0` so it renders/parses as `0`,
    // matching the previous `as i64` saturating cast.
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
        if !valid_ident(inner) {
            return None;
        }
        inner
    } else {
        let rest = stripped.strip_prefix('$')?;
        if !valid_ident(rest) {
            return None;
        }
        rest
    };
    Some(normalise_var_name(name).to_owned())
}

fn valid_ident(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut bytes = s.bytes();
    let first = bytes.next().unwrap();
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
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

fn resolve_switch_subject(text: &str, env: &StaticEnv) -> Option<String> {
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

fn resolve_switch_pattern(pattern: &str) -> String {
    strip_word_delimiters(pattern)
}

// Simulator

/// Execute one IR statement in the simulator, updating `env`.
///
/// Returns `true` when the statement is in the supported subset;
/// `false` when it should abort the whole summarisation (call,
/// barrier, unhandled structured form, etc.).
fn exec_statement(stmt: &Statement, env: &mut StaticEnv, octal: Option<bool>) -> bool {
    match stmt {
        Statement::AssignConst { name, value, .. } => {
            env.insert(name.clone(), parse_literal_value(value));
            true
        }
        Statement::AssignExpr { name, expr, .. } => {
            match evaluate_expr_with_constants(expr, env, octal) {
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
        Statement::Incr { name, amount, .. } => {
            let base = env.get(name).cloned();
            let Some(StaticValue::Int(b)) = base else {
                return false;
            };
            let amt: i64 = match amount.as_deref() {
                None => 1,
                Some(text) => {
                    let t = text.trim();
                    if let Ok(i) = t.parse::<i64>() {
                        i
                    } else if let Some(var) = simple_var_ref(t) {
                        match env.get(&var) {
                            Some(StaticValue::Int(i)) => *i,
                            Some(StaticValue::Bool(b)) => i64::from(*b),
                            _ => return false,
                        }
                    } else {
                        return false;
                    }
                }
            };
            let Some(sum) = b.checked_add(amt) else {
                return false;
            };
            env.insert(name.clone(), StaticValue::Int(sum));
            true
        }
        Statement::If {
            clauses, else_body, ..
        } => exec_if(clauses, else_body.as_ref(), env, octal),
        Statement::Switch {
            subject,
            arms,
            default_body,
            mode,
            ..
        } => exec_switch(subject, arms, default_body.as_ref(), *mode, env, octal),
        // Calls, barriers, returns, loops (other than the
        // top-level summarised `for`) — out of supported subset.
        _ => false,
    }
}

fn exec_script(script: &Script, env: &mut StaticEnv, octal: Option<bool>) -> bool {
    for stmt in &script.statements {
        if !exec_statement(stmt, env, octal) {
            return false;
        }
    }
    true
}

fn exec_if(
    clauses: &[IfClause],
    else_body: Option<&Script>,
    env: &mut StaticEnv,
    octal: Option<bool>,
) -> bool {
    for clause in clauses {
        let Some(cond) = evaluate_expr_with_constants(&clause.condition, env, octal) else {
            return false;
        };
        if cond != 0 {
            return exec_script(&clause.body, env, octal);
        }
    }
    match else_body {
        None => true,
        Some(body) => exec_script(body, env, octal),
    }
}

fn exec_switch(
    subject: &str,
    arms: &[SwitchArm],
    default_body: Option<&Script>,
    _mode: SwitchMode,
    env: &mut StaticEnv,
    octal: Option<bool>,
) -> bool {
    let Some(subject_value) = resolve_switch_subject(subject, env) else {
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
        Some(b) => exec_script(b, env, octal),
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
    octal: Option<bool>,
) -> Option<StaticEnv> {
    let mut env: StaticEnv = initial_constants.clone();

    if !exec_script(init, &mut env, octal) {
        return None;
    }
    let mut iterations: u64 = 0;
    loop {
        let cond = evaluate_expr_with_constants(condition, &env, octal)?;
        if cond == 0 {
            break;
        }
        iterations += 1;
        if iterations > max_iterations {
            return None;
        }
        if !exec_script(body, &mut env, octal) {
            return None;
        }
        if !exec_script(next_script, &mut env, octal) {
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
    octal: Option<bool>,
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
    let _ = expr_text(condition); // reserved for diagnostics
    summarise_static_for(
        init,
        condition,
        next,
        body,
        initial_constants,
        max_iterations,
        octal,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr_parser::parse_expr;
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

    // -- saturating_f64_to_i64 --

    #[test]
    fn saturating_f64_to_i64_in_range_and_saturates() {
        // In-range integral floats convert exactly.
        assert_eq!(saturating_f64_to_i64(0.0), 0);
        assert_eq!(saturating_f64_to_i64(42.0), 42);
        assert_eq!(saturating_f64_to_i64(-42.0), -42);
        // `-0.0` normalises to 0 (tclsh `int(-0.0)` == 0), not "-0".
        assert_eq!(saturating_f64_to_i64(-0.0), 0);
        // Out-of-range magnitudes saturate by sign (matches the prior
        // `as i64` saturating-cast behaviour).
        assert_eq!(saturating_f64_to_i64(1e30), i64::MAX);
        assert_eq!(saturating_f64_to_i64(-1e30), i64::MIN);
    }

    // -- parse_literal_value --

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

    // -- simple_var_ref --

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

    // -- summarise_static_for --

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
            None,
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
            None,
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
            None,
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
                None
            )
            .is_none()
        );
    }

    /// `switch $mode { a {set v 1} default {set v 9} }` — shared by the
    /// switch-dispatch case and its unresolvable-subject counterpart.
    fn mode_switch() -> Statement {
        Statement::Switch {
            span: sp(),
            subject: "$mode".into(),
            subject_span: sp(),
            arms: vec![SwitchArm {
                pattern: "a".into(),
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
            None,
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
            None,
        )
        .expect("summarised");
        assert_eq!(env.get("v"), Some(&StaticValue::Int(1)));
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
            None,
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
        let env =
            summarise_for_statement(&for_stmt, &StaticEnv::new(), 100, None).expect("summarised");
        assert_eq!(env.get("i"), Some(&StaticValue::Int(2)));
    }

    // -- evaluate_expr_with_constants --

    #[test]
    fn evaluate_expr_integer() {
        let mut env = StaticEnv::new();
        env.insert("x".into(), StaticValue::Int(5));
        assert_eq!(
            evaluate_expr_with_constants(&parse_expr("$x + 3", None), &env, None),
            Some(8)
        );
    }

    #[test]
    fn evaluate_expr_integer_valued_float() {
        assert_eq!(
            evaluate_expr_with_constants(&parse_expr("6.0 / 2", None), &StaticEnv::new(), None),
            Some(3)
        );
    }

    #[test]
    fn evaluate_expr_fractional_float_none() {
        assert_eq!(
            evaluate_expr_with_constants(&parse_expr("1.5", None), &StaticEnv::new(), None),
            None
        );
    }
}
