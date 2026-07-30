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

//! CFG-flavoured walker for var-escape.
//!
//! Pulls together:
//!
//! * barrier handlers (`eval`, `uplevel`, generic
//!   barriers) plus `escape_every_name_touched_tree` (the
//!   tree-walk variant used inside literal eval bodies, since
//!   those statements aren't part of the enclosing SSA).
//! * `handle_call` dispatcher + value/expr scans
//!   that thread the per-statement `defs` map through.
//! * `handle_statement` (per `SsaStatement`),
//!   `walk_block` (per SSA block), `block_order` (deterministic
//!   RPO traversal), and the public [`analyse_cfg_function`]
//!   entry point.

use std::collections::HashMap;

use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;
use crate::ir::{CommandTokens, Statement};
use crate::ssa::{SsaBlock, SsaFunction, SsaStatement, Version};
use crate::var_escape::cfg_propagation::handlers::{
    handle_dynamic_name_first, handle_global, handle_info, handle_namespace_call, handle_upvar,
    handle_variable,
};
use crate::var_escape::cfg_propagation::known_names::collect_known_names_from_cfg;
use crate::var_escape::cfg_propagation::state::{CfgEscapeResult, CfgState};
use crate::var_escape::handlers::has_expand_word;
use crate::var_escape::helpers::{
    is_dynamic_name, is_dynamic_token, is_frameless_runtime_command, is_name_first_command,
    normalise_cmd_subst_head, scan_value_for_info_hazards,
};

/// Walk *value* for embedded `[cmd ...]` substitution heads;
/// flag a fallback when any non-frameless head appears, then
/// run [`scan_value_for_info_hazards`] for embedded `[info ...]`
/// shapes.
fn apply_value_scan(value: &str, state: &mut CfgState, defs: &HashMap<String, Version>) {
    if value.is_empty() {
        return;
    }
    if value.contains('[') {
        for head in extract_cmd_subst_heads(value) {
            let canonical = normalise_cmd_subst_head(&head);
            if !is_frameless_runtime_command(canonical) {
                state.record_fallback();
                break;
            }
        }
    }
    let (pessimistic, names) = scan_value_for_info_hazards(value);
    if pessimistic {
        state.mark_pessimistic();
        return;
    }
    for n in names {
        state.escape(&n, defs);
    }
}

fn apply_expr_scan(expr: Option<&ExprNode>, state: &mut CfgState, defs: &HashMap<String, Version>) {
    let Some(expr) = expr else {
        return;
    };
    let text = crate::expr_ast::render_expr(expr);
    apply_value_scan(&text, state, defs);
}

/// Find the leading command-word of every `[cmd ...]` substitution
/// in *value*. Mirrors the intra-procedural helper.
fn extract_cmd_subst_heads(value: &str) -> Vec<String> {
    let bytes = value.as_bytes();
    let n = bytes.len();
    let mut heads: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < n {
        let Some(open_off) = value[i..].find('[') else {
            break;
        };
        let open = i + open_off;
        let mut j = open + 1;
        while j < n && (bytes[j] == b' ' || bytes[j] == b'\t') {
            j += 1;
        }
        let head_start = j;
        if value[j..].starts_with("::") {
            j += 2;
        }
        let leading = bytes.get(j).copied();
        if !matches!(leading, Some(b) if b.is_ascii_alphabetic() || b == b'_') {
            i = open + 1;
            continue;
        }
        j += 1;
        while j < n && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b':') {
            j += 1;
        }
        heads.push(value[head_start..j].to_string());
        i = j;
    }
    heads
}

/// Generic call dispatcher. Mirrors the intra-procedural variant
/// but threads *defs* through to the per-command handlers.
fn handle_call(stmt: &Statement, state: &mut CfgState, defs: &HashMap<String, Version>) {
    let Statement::Call {
        command,
        args,
        tokens,
        ..
    } = stmt
    else {
        return;
    };
    let cmd = command.as_str();

    if !is_frameless_runtime_command(cmd) {
        if cmd.is_empty() || is_dynamic_token(cmd) {
            state.record_fallback();
        } else {
            state.record_call_fallback();
        }
    }

    if has_expand_word(tokens.as_ref()) && cmd != "list" && cmd != "concat" {
        state.mark_pessimistic();
        return;
    }

    match cmd {
        "upvar" => handle_upvar(args, state, defs),
        "global" => handle_global(args, state, defs),
        "variable" => handle_variable(args, state, defs),
        "namespace" => handle_namespace_call(args, state, defs),
        "info" => handle_info(args, state, defs),
        "catch" => handle_catch(args, state, defs),
        c if is_name_first_command(c) => handle_dynamic_name_first(c, args, state, defs),
        _ => {}
    }

    if !cmd.is_empty() && !is_dynamic_token(cmd) {
        state.record_callee(cmd);
    }
}

/// Handle ``eval``: literal body is recursively walked with
/// [`escape_every_name_touched_tree`]; non-literal body is
/// pessimistic.
fn handle_eval(args: &[String], state: &mut CfgState, defs: &HashMap<String, Version>) {
    if args.is_empty() {
        state.mark_pessimistic();
        return;
    }
    let body: String = if args.len() == 1 {
        args[0].clone()
    } else {
        args.join(" ")
    };
    if is_dynamic_token(&body) {
        state.mark_pessimistic();
        return;
    }
    // [`scan_word`], not `scan_script`: this scan over-approximates on
    // purpose — every name the body text mentions escapes, whether or not
    // Tcl substitutes it at *this* level. A brace-quoted word (`eval {if
    // {$x > 1} {...}}`) is re-parsed and substituted when the inner command
    // runs, so its names escape too. See `var_escape::walker::handle_eval`.
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut scanner =
        crate::var_refs::VarReferenceScanner::new(crate::var_refs::VarScanOptions::default());
    for ref_ in scanner.scan_word(&body, &registry) {
        state.escape(&ref_, defs);
    }
    let sub_module = crate::lowering::lower_to_ir(&body, &registry);
    escape_every_name_touched_tree(&sub_module.top_level.statements, state, defs);
}

/// Handle ``catch``: the CFG lowers ``catch {body}`` to an opaque
/// ``Call`` whose first raw arg is the (brace-stripped) body script, so the
/// flow-sensitive walker never sees an ``upvar``/``global``/``uplevel`` inside
/// it. Mirror [`handle_eval`]: a *literal* body is re-scanned and walked so its
/// scope-crossing escapes — notably the upvar **source name** the
/// interprocedural pass needs to spill the caller's local — are recorded. A
/// non-literal body keeps the call-fallback already recorded by the caller.
fn handle_catch(args: &[String], state: &mut CfgState, defs: &HashMap<String, Version>) {
    let Some(body) = args.first() else {
        return;
    };
    if is_dynamic_token(body) {
        return;
    }
    // [`scan_word`] for the same over-approximation reason as `handle_eval`:
    // a brace-quoted word inside the caught body still escapes its names.
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut scanner =
        crate::var_refs::VarReferenceScanner::new(crate::var_refs::VarScanOptions::default());
    for ref_ in scanner.scan_word(body, &registry) {
        state.escape(&ref_, defs);
    }
    let sub_module = crate::lowering::lower_to_ir(body, &registry);
    escape_every_name_touched_tree(&sub_module.top_level.statements, state, defs);
}

/// Handle ``uplevel``: only ``#0`` (global scope) with a literal body is safe.
/// ``uplevel 0`` runs in the *current* frame and is walked like ``eval``;
/// everything else is pessimistic.
fn handle_uplevel(args: &[String], state: &mut CfgState, defs: &HashMap<String, Version>) {
    if args.is_empty() {
        state.mark_pessimistic();
        return;
    }
    let first = &args[0];
    // Registry level-word grammar, not a local digit sniff — see the twin in
    // `crate::var_escape::walker::handle_uplevel`.
    let Some(level) = tcl_registry::frame_effect::FrameLevel::parse(first) else {
        state.mark_pessimistic();
        return;
    };
    // `uplevel 0` runs the body in the *current* frame — a `set x` there
    // name-writes our proc's local `x` exactly like `eval`. Walk it the same
    // way so those escapes are recorded; treating `0` as global-safe like `#0`
    // wrongly whitelists our own frame (RUST_ISSUE_073).
    if level.is_current_frame() {
        handle_eval(&args[1..], state, defs);
        return;
    }
    // Only `uplevel #0` (global scope) is safe: our locals aren't visible
    // there. Any other level runs in a different caller frame — pessimistic.
    if !level.is_global_frame() {
        state.mark_pessimistic();
        return;
    }
    let body_parts = &args[1..];
    if body_parts.is_empty() {
        state.mark_pessimistic();
        return;
    }
    let body: String = if body_parts.len() == 1 {
        body_parts[0].clone()
    } else {
        body_parts.join(" ")
    };
    if is_dynamic_token(&body) {
        state.mark_pessimistic();
    }
}

/// Dispatch on the barrier command name.
fn handle_barrier(
    command: &str,
    args: &[String],
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
) {
    state.record_fallback();
    match command {
        "eval" => handle_eval(args, state, defs),
        "uplevel" => handle_uplevel(args, state, defs),
        _ => state.mark_pessimistic(),
    }
}

fn synthesise_eval_args(tokens: Option<&CommandTokens>) -> Vec<String> {
    tokens
        .map(|t| t.argv_texts.iter().skip(1).cloned().collect())
        .unwrap_or_default()
}

fn synthesise_uplevel_args(tokens: Option<&CommandTokens>) -> Vec<String> {
    tokens
        .map(|t| t.argv_texts.iter().skip(1).cloned().collect())
        .unwrap_or_default()
}

fn is_eval_block(tokens: Option<&CommandTokens>) -> bool {
    tokens
        .and_then(|t| t.argv_texts.first())
        .is_some_and(|s| s == "eval")
}

/// Tree-walk variant used inside literal `eval` bodies. The
/// statements aren't part of the enclosing SSA, so we tag every
/// name escape at the caller's current version (via *defs* /
fn tree_assign_or_incr(
    stmt: &Statement,
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
) -> bool {
    match stmt {
        Statement::AssignConst { name, value, .. } | Statement::AssignValue { name, value, .. } => {
            if name.is_empty() || is_dynamic_name(name) {
                state.mark_pessimistic();
                return true;
            }
            state.escape(name, defs);
            apply_value_scan(value, state, defs);
            true
        }
        Statement::AssignExpr { name, expr, .. } => {
            if name.is_empty() || is_dynamic_name(name) {
                state.mark_pessimistic();
                return true;
            }
            state.escape(name, defs);
            apply_expr_scan(Some(expr), state, defs);
            true
        }
        Statement::Incr { name, amount, .. } => {
            if name.is_empty() || is_dynamic_name(name) {
                state.mark_pessimistic();
                return true;
            }
            state.escape(name, defs);
            if let Some(a) = amount {
                apply_value_scan(a, state, defs);
            }
            true
        }
        _ => false,
    }
}

fn tree_call_or_barrier(
    stmt: &Statement,
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
) -> bool {
    match stmt {
        Statement::Call {
            defs: call_defs,
            reads,
            ..
        } => {
            for n in call_defs.iter().chain(reads.iter()) {
                if !n.is_empty() && !is_dynamic_token(n) {
                    state.escape(n, defs);
                }
            }
            handle_call(stmt, state, defs);
            true
        }
        Statement::Barrier { command, args, .. } => {
            handle_barrier(command, args, state, defs);
            true
        }
        Statement::UpFrame { tokens, .. } => {
            let args = synthesise_uplevel_args(tokens.as_ref());
            handle_barrier("uplevel", &args, state, defs);
            true
        }
        Statement::Block { body, tokens, .. } => {
            if is_eval_block(tokens.as_ref()) {
                let args = synthesise_eval_args(tokens.as_ref());
                handle_barrier("eval", &args, state, defs);
            } else {
                escape_every_name_touched_tree(&body.statements, state, defs);
            }
            true
        }
        Statement::Return { value, expr, .. } => {
            if let Some(v) = value {
                apply_value_scan(v, state, defs);
            }
            apply_expr_scan(expr.as_ref(), state, defs);
            true
        }
        Statement::ExprEval { expr, .. } => {
            apply_expr_scan(Some(expr), state, defs);
            true
        }
        _ => false,
    }
}

fn tree_structural(stmt: &Statement, state: &mut CfgState, defs: &HashMap<String, Version>) {
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                apply_expr_scan(Some(&c.condition), state, defs);
                escape_every_name_touched_tree(&c.body.statements, state, defs);
            }
            if let Some(b) = else_body {
                escape_every_name_touched_tree(&b.statements, state, defs);
            }
        }
        Statement::For {
            init,
            condition,
            next,
            body,
            ..
        } => {
            escape_every_name_touched_tree(&init.statements, state, defs);
            apply_expr_scan(Some(condition), state, defs);
            escape_every_name_touched_tree(&next.statements, state, defs);
            escape_every_name_touched_tree(&body.statements, state, defs);
        }
        Statement::While {
            condition, body, ..
        } => {
            apply_expr_scan(Some(condition), state, defs);
            escape_every_name_touched_tree(&body.statements, state, defs);
        }
        Statement::Foreach {
            iterators, body, ..
        } => {
            for it in iterators {
                apply_value_scan(&it.list_arg, state, defs);
            }
            escape_every_name_touched_tree(&body.statements, state, defs);
        }
        Statement::Catch { body, .. } => {
            escape_every_name_touched_tree(&body.statements, state, defs);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            escape_every_name_touched_tree(&body.statements, state, defs);
            for h in handlers {
                escape_every_name_touched_tree(&h.body.statements, state, defs);
            }
            if let Some(f) = finally_body {
                escape_every_name_touched_tree(&f.statements, state, defs);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    escape_every_name_touched_tree(&b.statements, state, defs);
                }
            }
            if let Some(d) = default_body {
                escape_every_name_touched_tree(&d.statements, state, defs);
            }
        }
        _ => {}
    }
}

pub(crate) fn escape_every_name_touched_tree(
    stmts: &[Statement],
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
) {
    for stmt in stmts {
        if state.dynamic_barrier() {
            return;
        }
        if tree_assign_or_incr(stmt, state, defs) {
            continue;
        }
        if tree_call_or_barrier(stmt, state, defs) {
            continue;
        }
        tree_structural(stmt, state, defs);
    }
}

/// Resolve a (possibly-dynamic) variable name and call `escape` on
/// the resolved literal — or spill all known names when the dynamic
/// name can't be resolved.  Used by the assign / incr arms.
fn dynamic_name_escape(state: &mut CfgState, defs: &HashMap<String, Version>, name: &str) {
    if let Some(literal) = state.resolve_literal(name) {
        state.escape(&literal, defs);
    } else {
        state.escape_all_known(defs);
    }
}

/// Handle the call / barrier / block / upframe / return / exprEval
/// arms of `handle_statement`.  Returns `true` when *stmt* matched.
fn handle_stmt_call_or_barrier(
    stmt: &Statement,
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
) -> bool {
    match stmt {
        Statement::Call { .. } => {
            handle_call(stmt, state, defs);
            true
        }
        Statement::Barrier { command, args, .. } => {
            handle_barrier(command, args, state, defs);
            true
        }
        Statement::UpFrame { tokens, .. } => {
            let args = synthesise_uplevel_args(tokens.as_ref());
            handle_barrier("uplevel", &args, state, defs);
            true
        }
        Statement::Block { body, tokens, .. } => {
            if is_eval_block(tokens.as_ref()) {
                let args = synthesise_eval_args(tokens.as_ref());
                handle_barrier("eval", &args, state, defs);
            } else {
                escape_every_name_touched_tree(&body.statements, state, defs);
            }
            true
        }
        Statement::Return { value, expr, .. } => {
            if let Some(v) = value {
                apply_value_scan(v, state, defs);
            }
            apply_expr_scan(expr.as_ref(), state, defs);
            true
        }
        Statement::ExprEval { expr, .. } => {
            apply_expr_scan(Some(expr), state, defs);
            true
        }
        _ => false,
    }
}

/// Handle the assign / incr arms of `handle_statement`.  Returns
/// `true` when *stmt* matched.
fn handle_stmt_assign_or_incr(
    stmt: &Statement,
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
) -> bool {
    match stmt {
        Statement::AssignConst { name, value, .. } => {
            if is_dynamic_name(name) {
                dynamic_name_escape(state, defs, name);
            } else {
                state.note_literal_assign(name, value);
            }
            apply_value_scan(value, state, defs);
            true
        }
        Statement::AssignValue { name, value, .. } => {
            if is_dynamic_name(name) {
                dynamic_name_escape(state, defs, name);
            } else if !value.is_empty() && !is_dynamic_token(value) {
                state.note_literal_assign(name, value);
            } else {
                state.invalidate_literal(name);
            }
            apply_value_scan(value, state, defs);
            true
        }
        Statement::AssignExpr { name, expr, .. } => {
            if is_dynamic_name(name) {
                dynamic_name_escape(state, defs, name);
            } else {
                state.invalidate_literal(name);
            }
            apply_expr_scan(Some(expr), state, defs);
            true
        }
        Statement::Incr { name, amount, .. } => {
            if is_dynamic_name(name) {
                dynamic_name_escape(state, defs, name);
            } else {
                state.invalidate_literal(name);
            }
            if let Some(a) = amount {
                apply_value_scan(a, state, defs);
            }
            true
        }
        _ => false,
    }
}

/// Process one [`SsaStatement`] — apply the appropriate
/// per-Statement transfer function.
fn handle_statement(ssa_stmt: &SsaStatement, state: &mut CfgState, ssa: &SsaFunction) {
    let stmt = &ssa_stmt.statement;
    // The SSA per-statement `defs` map is keyed by interned [`Symbol`]; the
    // escape handlers work in display names, so resolve each symbol once here.
    let defs: HashMap<String, Version> = ssa_stmt
        .defs
        .iter()
        .map(|(&sym, &ver)| (ssa.var_name(sym).to_owned(), ver))
        .collect();
    let defs = &defs;
    state.remember_versions(ssa_stmt, ssa);

    if handle_stmt_call_or_barrier(stmt, state, defs) {
        return;
    }
    if handle_stmt_assign_or_incr(stmt, state, defs) {
        return;
    }
    // Structured statements: recurse via the tree walker.  Closure
    // wraps to avoid duplicating the arm patterns.
    match stmt {
        Statement::Return { value, expr, .. } => {
            if let Some(v) = value {
                apply_value_scan(v, state, defs);
            }
            apply_expr_scan(expr.as_ref(), state, defs);
        }
        Statement::ExprEval { expr, .. } => apply_expr_scan(Some(expr), state, defs),
        // Structured statements appear in the SSA stream as flat
        // statements after CFG lowering — but they may still be
        // present for non-flattened compound shapes. Recurse into
        // their inner scripts via the tree walker.
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                apply_expr_scan(Some(&c.condition), state, defs);
                escape_every_name_touched_tree(&c.body.statements, state, defs);
            }
            if let Some(b) = else_body {
                escape_every_name_touched_tree(&b.statements, state, defs);
            }
        }
        Statement::For {
            init,
            condition,
            next,
            body,
            ..
        } => {
            escape_every_name_touched_tree(&init.statements, state, defs);
            apply_expr_scan(Some(condition), state, defs);
            escape_every_name_touched_tree(&next.statements, state, defs);
            escape_every_name_touched_tree(&body.statements, state, defs);
        }
        Statement::While {
            condition, body, ..
        } => {
            apply_expr_scan(Some(condition), state, defs);
            escape_every_name_touched_tree(&body.statements, state, defs);
        }
        Statement::Foreach {
            iterators, body, ..
        } => {
            for it in iterators {
                apply_value_scan(&it.list_arg, state, defs);
            }
            escape_every_name_touched_tree(&body.statements, state, defs);
        }
        Statement::Catch { body, .. } => {
            escape_every_name_touched_tree(&body.statements, state, defs);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            escape_every_name_touched_tree(&body.statements, state, defs);
            for h in handlers {
                escape_every_name_touched_tree(&h.body.statements, state, defs);
            }
            if let Some(f) = finally_body {
                escape_every_name_touched_tree(&f.statements, state, defs);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    escape_every_name_touched_tree(&b.statements, state, defs);
                }
            }
            if let Some(d) = default_body {
                escape_every_name_touched_tree(&d.statements, state, defs);
            }
        }
        // All other Statement variants (Call / Barrier / etc.) are
        // handled by the early-return helpers above.
        _ => {}
    }
}

/// Process one SSA block — every statement plus the terminator's
/// branch condition (so `[info exists ...]` inside an `if`
/// condition isn't missed).
fn walk_block(
    block: &SsaBlock,
    state: &mut CfgState,
    terminator_condition: Option<&ExprNode>,
    ssa: &SsaFunction,
) {
    for stmt in &block.statements {
        if state.dynamic_barrier() {
            return;
        }
        handle_statement(stmt, state, ssa);
    }
    if let Some(cond) = terminator_condition {
        // The terminator's condition doesn't live in any
        // statement's defs, so we have no SSA version to tag —
        // pass an empty defs map and let the escape helper fall
        // back to the latest seen version.
        apply_expr_scan(Some(cond), state, &HashMap::new());
    }
}

/// Reverse-postorder block traversal. The escape analysis is
/// monotone, so any order produces the same final tags; RPO
/// gives tests a deterministic walk order.
fn block_order(cfg: &CfgFunction) -> Vec<BlockId> {
    cfg.reverse_postorder()
}

/// Run the flow-sensitive per-SSA-version escape analysis over
/// *cfg* / *ssa*. Returns the [`CfgEscapeResult`].
#[must_use]
pub fn analyse_cfg_function<I: IntoIterator<Item = String>>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    params: I,
) -> CfgEscapeResult {
    let known = collect_known_names_from_cfg(params, ssa);
    let mut state = CfgState::new(known);

    for block_id in block_order(cfg) {
        let Some(block) = ssa.blocks.get(&block_id) else {
            continue;
        };
        let term_cond = cfg
            .blocks
            .get(&block_id)
            .and_then(|b| b.terminator.as_ref())
            .and_then(|t| match t {
                Terminator::Branch { condition, .. } => Some(condition),
                _ => None,
            });
        walk_block(block, &mut state, term_cond, ssa);
    }

    state.into_result()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg_builder::build_cfg_function;
    use crate::lowering::lower_to_ir;
    use crate::ssa::build_ssa;
    use crate::var_escape::types::EscapeTag;
    use tcl_registry::CommandRegistry;

    fn analyse(src: &str) -> CfgEscapeResult {
        let registry = CommandRegistry::build_default();
        let m = lower_to_ir(src, &registry);
        let cfg = build_cfg_function("::top", &m.top_level, true);
        let ssa = build_ssa(&cfg, &registry);
        analyse_cfg_function(&cfg, &ssa, std::iter::empty::<String>())
    }

    #[test]
    fn pure_set_does_not_escape() {
        let r = analyse("set x 1");
        assert!(!r.dynamic_barrier());
        assert!(r.name_tags.is_empty());
    }

    /// An `eval` body that mentions any variable takes the pessimistic
    /// path, which is what accounts for names Tcl does not substitute at
    /// this level — a brace-quoted `if` condition among them.
    ///
    /// Driven through the handler directly because lowering relaxes a
    /// static-body `eval` into structured IR, so no opaque call reaches it
    /// from [`analyse`]. This is the fallback for every body lowering
    /// declines to relax.
    ///
    /// The pre-scan below the guard uses the over-approximating `scan_word`
    /// mode, but `is_dynamic_token` fires on any `$` or `[` — which is every
    /// body the scan could find a reference in — so the mode is unobservable
    /// here today and the guard carries the soundness. It is written the
    /// over-approximating way so that narrowing the guard cannot silently
    /// open a hole; the mode contract itself is pinned by `var_refs`'
    /// `scan_word_finds_a_var_inside_braces_within_a_value_body`.
    ///
    /// `handle_catch`'s dynamic branch deliberately records nothing here —
    /// its doc states the caller's own call-fallback covers a non-literal
    /// body — so its contract does not hold at this level and is not
    /// asserted.
    #[test]
    fn an_eval_body_mentioning_a_variable_is_pessimistic() {
        let mut state = CfgState::new(std::iter::empty::<String>());
        handle_eval(
            &["if {$x > 1} { set y 2 }".to_string()],
            &mut state,
            &HashMap::new(),
        );
        assert!(
            state
                .flags
                .contains(crate::var_escape::types::EscapeFlags::DYNAMIC_BARRIER),
            "a substitution-bearing eval body is pessimistic, which covers every name",
        );
    }

    #[test]
    fn upvar_escapes_local_alias() {
        let r = analyse("upvar 1 caller_x x");
        assert_eq!(r.name_tags.get("x"), Some(&EscapeTag::Frame));
        assert!(r.upvar_source_names.contains("caller_x"));
    }

    #[test]
    fn global_escapes_named_var() {
        let r = analyse("global g");
        assert_eq!(r.name_tags.get("g"), Some(&EscapeTag::Frame));
    }

    #[test]
    fn variable_escapes_named_vars() {
        let r = analyse("variable a 1 b 2");
        assert_eq!(r.name_tags.get("a"), Some(&EscapeTag::Frame));
        assert_eq!(r.name_tags.get("b"), Some(&EscapeTag::Frame));
    }

    #[test]
    fn info_level_marks_pessimistic() {
        let r = analyse("info level");
        assert!(r.dynamic_barrier());
    }

    #[test]
    fn info_exists_literal_escapes_target() {
        let r = analyse("info exists myvar");
        assert_eq!(r.name_tags.get("myvar"), Some(&EscapeTag::Frame));
    }

    #[test]
    fn unknown_command_sets_call_fallback() {
        let r = analyse("some_user_proc arg");
        assert!(r.has_call_fallback());
    }

    #[test]
    fn frameless_runtime_call_does_not_set_call_fallback() {
        let r = analyse("string length foo");
        assert!(!r.has_call_fallback());
    }

    #[test]
    fn descends_into_if_body() {
        let r = analyse("if {1} { upvar 1 caller_x x }");
        assert_eq!(r.name_tags.get("x"), Some(&EscapeTag::Frame));
    }

    #[test]
    fn ssa_tags_track_per_version() {
        // Two assigns + an upvar — the upvar runs after both
        // assigns, so it tags the latest version.
        let r = analyse("set y 1\nset y 2\nupvar 1 caller_y y");
        // ``y`` is in name_tags as Frame.
        assert_eq!(r.name_tags.get("y"), Some(&EscapeTag::Frame));
    }
}
