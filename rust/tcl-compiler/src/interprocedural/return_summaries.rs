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

//! The return shape of a procedure: classifying each `return`, whether a body always or may return, and the summary fields derived from them.

use super::{ConstantReturn, MAX_INTERPROCEDURAL_WALK_DEPTH, ReturnKind};
use crate::ir::ProcedureId;
use std::collections::{HashMap, HashSet};
use tcl_registry::value_transfer::ExactValue;
use tcl_source::limits::MAX_EXPR_NODE_DEPTH;

/// Classify a single `Statement::Return` by its word, for the summary's
/// return shapes: a literal is its value through the exact value ingress —
/// a braced word its content, a bare or quoted one its escapes decoded,
/// nothing trimmed — `$param` a passthrough, and `return [expr {…}]` as
/// [`classify_return_expr`] reads it.
pub(super) fn classify_return(
    value: Option<&str>,
    braced: bool,
    word: Option<&tcl_source::words::WordExpr>,
    expr: Option<&crate::expr_ast::ExprNode>,
    params: &HashSet<String>,
    config: Option<tcl_lexer::LexerConfig>,
) -> ReturnKind {
    // Prefer the structured `expr` when the return was `return
    // [expr {…}]` or similar — the AST gives precise information.
    if let Some(node) = expr {
        return classify_return_expr(node, params, config);
    }
    let Some(raw) = value else {
        return ReturnKind::Other;
    };
    if let Some(config) = config
        && let Some(text) = crate::value_transfer::recorded_word_value(raw, braced, &config)
    {
        return ReturnKind::Literal(ExactValue::from_literal(&text));
    }
    if !braced
        && let Some(config) = config
        && let Some(tcl_source::words::WordExpr::Variable { spelling, .. }) = word
        && let Some(name) =
            tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, config)
        && params.contains(name)
    {
        return ReturnKind::Passthrough(name.to_owned());
    }
    ReturnKind::Other
}

fn classify_return_expr(
    node: &crate::expr_ast::ExprNode,
    params: &HashSet<String>,
    config: Option<tcl_lexer::LexerConfig>,
) -> ReturnKind {
    use crate::expr_ast::ExprNode;

    // A literal operand is the expression's value only where it is a
    // canonical decimal integer: `0x10` is 16, `010` is 8 or 10 by release,
    // and `true` stays `true`, which the expression route decides.
    if let ExprNode::Literal { text, .. } = node {
        let value = ExactValue::from_literal(text);
        return if value.as_int().is_some() {
            ReturnKind::Literal(value)
        } else {
            ReturnKind::Other
        };
    }
    if let ExprNode::String { text, .. } = node {
        // The operand's text is its value only when it is fixed: a `"…"` one
        // substitutes, so `return [expr {"pre$x"}]` returns `pre5` in tclsh
        // 8.6.18 and 9.0.4 where O103 folded the call to the literal `pre$x`;
        // a `{…}` one folds its backslash-newlines, so `{a\<newline> b}` is
        // `a b`, not the raw bytes (#2227, found in review).
        return tcl_syntax::expr::fixed_string_operand(text).map_or(ReturnKind::Other, |value| {
            ReturnKind::Literal(ExactValue::from_literal(value))
        });
    }
    if let ExprNode::Var { text, .. } = node
        && let Some(config) = config
        && let Ok(crate::native_lowering::cells::CellPlace::Named { name }) =
            crate::native_lowering::cells::variable_reference_place(text, config)
        && params.contains(&name)
    {
        return ReturnKind::Passthrough(name);
    }
    // Walk the AST collecting var references against the param
    // set; any match → UsesParam.
    let mut referenced: Vec<String> = Vec::new();
    if !walk_collect_param_refs(node, params, &mut referenced, config, 0) {
        return ReturnKind::Other;
    }
    if !referenced.is_empty() {
        referenced.sort();
        referenced.dedup();
        return ReturnKind::UsesParam(referenced);
    }
    ReturnKind::Other
}

pub(super) fn walk_collect_param_refs(
    node: &crate::expr_ast::ExprNode,
    params: &HashSet<String>,
    out: &mut Vec<String>,
    config: Option<tcl_lexer::LexerConfig>,
    depth: u32,
) -> bool {
    use crate::expr_ast::ExprNode;
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return false;
    }
    match node {
        ExprNode::Var { .. } => {
            let Some(config) = config else {
                return false;
            };
            let Ok(Some(reference)) = node.variable_reference(config) else {
                return false;
            };
            let Ok(name) = std::str::from_utf8(reference.name) else {
                return false;
            };
            if params.contains(name) {
                out.push(name.to_owned());
            }
            reference.index.is_none()
        }
        ExprNode::Binary { left, right, .. } => {
            let left = walk_collect_param_refs(left, params, out, config, depth + 1);
            walk_collect_param_refs(right, params, out, config, depth + 1) && left
        }
        ExprNode::Unary { operand, .. } => {
            walk_collect_param_refs(operand, params, out, config, depth + 1)
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            let condition = walk_collect_param_refs(condition, params, out, config, depth + 1);
            let left = walk_collect_param_refs(true_branch, params, out, config, depth + 1);
            walk_collect_param_refs(false_branch, params, out, config, depth + 1)
                && left
                && condition
        }
        ExprNode::Call { args, .. } => {
            let mut complete = true;
            for arg in args {
                complete &= walk_collect_param_refs(arg, params, out, config, depth + 1);
            }
            complete
        }
        ExprNode::Literal { .. } => true,
        ExprNode::Command { .. }
        | ExprNode::String { .. }
        | ExprNode::CompiledWord { .. }
        | ExprNode::Raw { .. } => false,
    }
}

/// Whether every path through `script` reaches an explicit `return`, so
/// the proc cannot fall off the end. A fall-through exit returns the
/// result of the body's last command — generally **not** a compile-time
/// constant — so a proc that returns a literal on one path but can fall
/// through on another is not constant-returning (O103).
///
/// Deliberately conservative: it recognises only `return` and a
/// fully-covered `if`/`elseif`/`else` whose every arm returns. Anything
/// else is treated as "may fall through", which costs only a missed fold,
/// never a miscompile.
/// `depth` is the nesting level of `script` — see
/// [`MAX_INTERPROCEDURAL_WALK_DEPTH`]. Past the cap, conservatively answers
/// `false` ("may fall through") — the same conservative direction this
/// function's doc comment already commits to for anything it can't prove,
/// so an unresolved deep answer costs only a missed constant-return fold,
/// never a miscompile.
pub(super) fn script_always_returns(script: &crate::ir::Script, depth: u32) -> bool {
    if MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth) {
        return false;
    }
    script
        .statements
        .iter()
        .any(|s| stmt_always_returns(s, depth))
}

pub(super) fn stmt_always_returns(stmt: &crate::ir::Statement, depth: u32) -> bool {
    use crate::ir::Statement;
    match stmt {
        Statement::Return { .. } => true,
        Statement::If {
            clauses, else_body, ..
        } => {
            else_body
                .as_ref()
                .is_some_and(|b| script_always_returns(b, depth + 1))
                && clauses
                    .iter()
                    .all(|c| script_always_returns(&c.body, depth + 1))
        }
        _ => false,
    }
}

/// Whether running `stmt` may run a `return` that leaves the procedure from
/// inside it: a `return` itself, or one in a script the statement runs — an
/// `if`'s, a loop's, a `try`'s body, handlers or `finally`, a `switch`'s arms,
/// a block's or an `uplevel`'s, a call's word the registry gives the
/// [`tcl_registry::ArgRole::Body`] role, or a barrier's unseen code — but not
/// one in a `catch` body, which the `catch` absorbs (as the registry's plan
/// says of a `catch` kept as a call), nor in the body of a loop the flow
/// graph lowers, whose synthetic header holds only the list words. `depth`
/// is the nesting level of the script holding `stmt`; past
/// [`MAX_INTERPROCEDURAL_WALK_DEPTH`] it answers that it may.
pub(crate) fn statement_may_return(
    stmt: &crate::ir::Statement,
    registry: &tcl_registry::CommandRegistry,
    depth: u32,
) -> bool {
    use crate::ir::Statement;
    let runs = |script: &crate::ir::Script| script_may_return(script, registry, depth + 1);
    match stmt {
        Statement::Return { .. } | Statement::Barrier { .. } => true,
        Statement::Call {
            args,
            foreach_groups: None,
            ..
        } => {
            let head = stmt.canonical_command_or_source();
            let words: Vec<&str> = args.iter().map(String::as_str).collect();
            !registry
                .arg_indices_for_role(head, &words, tcl_registry::ArgRole::Body)
                .is_empty()
                && !crate::value_transfer::resolved_body_absorbs_completion(registry, head, args)
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses.iter().any(|clause| runs(&clause.body)) || else_body.as_ref().is_some_and(runs)
        }
        Statement::For {
            init, next, body, ..
        } => runs(init) || runs(next) || runs(body),
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Block { body, .. }
        | Statement::UpFrame { body, .. } => runs(body),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            runs(body)
                || handlers.iter().any(|handler| runs(&handler.body))
                || finally_body.as_ref().is_some_and(runs)
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            arms.iter().any(|arm| arm.body.as_ref().is_some_and(runs))
                || default_body.as_ref().is_some_and(runs)
        }
        _ => false,
    }
}

/// [`statement_may_return`] over every statement of `script`, nested
/// `depth` levels deep.
fn script_may_return(
    script: &crate::ir::Script,
    registry: &tcl_registry::CommandRegistry,
    depth: u32,
) -> bool {
    MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth)
        || script
            .statements
            .iter()
            .any(|stmt| statement_may_return(stmt, registry, depth))
}

/// What a procedure's seedless run says it returns, as
/// [`summarise_returns`] reads it.
#[derive(Clone, Copy)]
pub(super) enum SeedlessAnswer<'a> {
    /// No run was made: the return shapes answer.
    NotRun,
    /// The one value every exit gives.
    Value(&'a ExactValue),
    /// The run proved no one value.
    NoValue,
}

impl<'a> SeedlessAnswer<'a> {
    /// The answer [`seedless_returns`] recorded for `qname`.
    pub(super) fn of(
        seedless: &'a HashMap<ProcedureId, Option<ExactValue>>,
        qname: &ProcedureId,
    ) -> Self {
        match seedless.get(qname) {
            None => Self::NotRun,
            Some(Some(value)) => Self::Value(value),
            Some(None) => Self::NoValue,
        }
    }
}

/// Derive the return-value summary fields from a proc's collected
/// [`ReturnKind`] list and its seedless run's answer: `(returns_constant,
/// constant_return, passthrough_param, depends_on_params)`.
///
/// Where a run was made its answer is the constant, and where it proved no
/// one value the shapes answer only a passthrough and the parameters the
/// value depends on, never a constant, since a value the run could not prove
/// may be one it rules out. With no run, every return the same literal is
/// the constant.
pub(super) fn summarise_returns(
    returns: &[ReturnKind],
    answer: SeedlessAnswer<'_>,
) -> (bool, Option<ConstantReturn>, Option<String>, Vec<String>) {
    let constant = match answer {
        SeedlessAnswer::Value(value) => Some(value),
        SeedlessAnswer::NoValue => None,
        SeedlessAnswer::NotRun => match returns.first() {
            Some(ReturnKind::Literal(first))
                if returns
                    .iter()
                    .all(|r| matches!(r, ReturnKind::Literal(v) if v.bytes == first.bytes)) =>
            {
                Some(first)
            }
            _ => None,
        },
    };
    if let Some(constant) = constant.and_then(ConstantReturn::from_exact) {
        return (true, Some(constant), None, Vec::new());
    }
    // Passthrough: every return is Passthrough of the same param.
    if let Some(ReturnKind::Passthrough(first)) = returns.first()
        && returns
            .iter()
            .all(|r| matches!(r, ReturnKind::Passthrough(v) if v == first))
    {
        return (false, None, Some(first.clone()), vec![first.clone()]);
    }
    // Depends on params: union of all ParamRefs + Passthrough
    // targets.
    let mut depends: Vec<String> = Vec::new();
    for r in returns {
        match r {
            ReturnKind::Passthrough(p) => depends.push(p.clone()),
            ReturnKind::UsesParam(ps) => depends.extend(ps.iter().cloned()),
            _ => {}
        }
    }
    depends.sort();
    depends.dedup();
    (false, None, None, depends)
}
