//! CFG-flavoured walker for var-escape (C33e4/5/6).
//!
//! Pulls together:
//!
//! * **C33e4** — barrier handlers (`eval`, `uplevel`, generic
//!   barriers) plus `escape_every_name_touched_tree` (the
//!   tree-walk variant used inside literal eval bodies, since
//!   those statements aren't part of the enclosing SSA).
//! * **C33e5** — `handle_call` dispatcher + value/expr scans
//!   that thread the per-statement `defs` map through.
//! * **C33e6** — `handle_statement` (per `SsaStatement`),
//!   `walk_block` (per SSA block), `block_order` (deterministic
//!   RPO traversal), and the public [`analyse_cfg_function`]
//!   entry point.
//!
//! Mirrors the bottom half of
//! `core/compiler/var_escape/_cfg_propagation.py`.

use std::collections::HashMap;

use crate::cfg::{Function as CfgFunction, Terminator};
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
#[allow(clippy::implicit_hasher)]
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

#[allow(clippy::implicit_hasher)]
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
#[allow(clippy::implicit_hasher)]
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
#[allow(clippy::implicit_hasher)]
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
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut scanner =
        crate::var_refs::VarReferenceScanner::new(crate::var_refs::VarScanOptions::default());
    for ref_ in scanner.scan_script(&body, &registry) {
        state.escape(&ref_, defs);
    }
    let sub_module = crate::lowering::lower_to_ir(&body, &registry);
    escape_every_name_touched_tree(&sub_module.top_level.statements, state, defs);
}

/// Handle ``uplevel``: only ``#0`` / ``0`` with a literal body
/// is safe; everything else is pessimistic.
#[allow(clippy::implicit_hasher)]
fn handle_uplevel(args: &[String], state: &mut CfgState, _defs: &HashMap<String, Version>) {
    if args.is_empty() {
        state.mark_pessimistic();
        return;
    }
    let first = &args[0];
    let no_dash = first.trim_start_matches('-');
    let is_level_literal = (!no_dash.is_empty() && no_dash.chars().all(|c| c.is_ascii_digit()))
        || (first.starts_with('#') && first[1..].chars().all(|c| c.is_ascii_digit()));
    if !is_level_literal {
        state.mark_pessimistic();
        return;
    }
    if first != "#0" && first != "0" {
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
#[allow(clippy::implicit_hasher)]
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
/// `version_for`).
#[allow(clippy::implicit_hasher)]
#[allow(clippy::too_many_lines)]
pub(crate) fn escape_every_name_touched_tree(
    stmts: &[Statement],
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
) {
    for stmt in stmts {
        if state.dynamic_barrier {
            return;
        }
        match stmt {
            Statement::AssignConst { name, value, .. }
            | Statement::AssignValue { name, value, .. } => {
                if name.is_empty() || is_dynamic_name(name) {
                    state.mark_pessimistic();
                    return;
                }
                state.escape(name, defs);
                apply_value_scan(value, state, defs);
            }
            Statement::AssignExpr { name, expr, .. } => {
                if name.is_empty() || is_dynamic_name(name) {
                    state.mark_pessimistic();
                    return;
                }
                state.escape(name, defs);
                apply_expr_scan(Some(expr), state, defs);
            }
            Statement::Incr { name, amount, .. } => {
                if name.is_empty() || is_dynamic_name(name) {
                    state.mark_pessimistic();
                    return;
                }
                state.escape(name, defs);
                if let Some(a) = amount {
                    apply_value_scan(a, state, defs);
                }
            }
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
            }
            Statement::Barrier { command, args, .. } => {
                handle_barrier(command, args, state, defs);
            }
            Statement::UpFrame { tokens, .. } => {
                let args = synthesise_uplevel_args(tokens.as_ref());
                handle_barrier("uplevel", &args, state, defs);
            }
            Statement::Block { body, tokens, .. } => {
                if is_eval_block(tokens.as_ref()) {
                    let args = synthesise_eval_args(tokens.as_ref());
                    handle_barrier("eval", &args, state, defs);
                } else {
                    escape_every_name_touched_tree(&body.statements, state, defs);
                }
            }
            Statement::Return { value, expr, .. } => {
                if let Some(v) = value {
                    apply_value_scan(v, state, defs);
                }
                apply_expr_scan(expr.as_ref(), state, defs);
            }
            Statement::ExprEval { expr, .. } => apply_expr_scan(Some(expr), state, defs),
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
        }
    }
}

/// Process one [`SsaStatement`] — apply the appropriate
/// per-Statement transfer function.
#[allow(clippy::too_many_lines)]
fn handle_statement(ssa_stmt: &SsaStatement, state: &mut CfgState) {
    let stmt = &ssa_stmt.statement;
    let defs = &ssa_stmt.defs;
    state.remember_versions(ssa_stmt);

    match stmt {
        Statement::Call { .. } => handle_call(stmt, state, defs),
        Statement::Barrier { command, args, .. } => handle_barrier(command, args, state, defs),
        Statement::UpFrame { tokens, .. } => {
            let args = synthesise_uplevel_args(tokens.as_ref());
            handle_barrier("uplevel", &args, state, defs);
        }
        Statement::Block { body, tokens, .. } => {
            if is_eval_block(tokens.as_ref()) {
                let args = synthesise_eval_args(tokens.as_ref());
                handle_barrier("eval", &args, state, defs);
            } else {
                escape_every_name_touched_tree(&body.statements, state, defs);
            }
        }
        Statement::AssignConst { name, value, .. } => {
            if is_dynamic_name(name) {
                if let Some(literal) = state.resolve_literal(name) {
                    state.escape(&literal, defs);
                } else {
                    state.escape_all_known(defs);
                }
            } else {
                state.note_literal_assign(name, value);
            }
            apply_value_scan(value, state, defs);
        }
        Statement::AssignValue { name, value, .. } => {
            if is_dynamic_name(name) {
                if let Some(literal) = state.resolve_literal(name) {
                    state.escape(&literal, defs);
                } else {
                    state.escape_all_known(defs);
                }
            } else if !value.is_empty() && !is_dynamic_token(value) {
                state.note_literal_assign(name, value);
            } else {
                state.invalidate_literal(name);
            }
            apply_value_scan(value, state, defs);
        }
        Statement::AssignExpr { name, expr, .. } => {
            if is_dynamic_name(name) {
                if let Some(literal) = state.resolve_literal(name) {
                    state.escape(&literal, defs);
                } else {
                    state.escape_all_known(defs);
                }
            } else {
                state.invalidate_literal(name);
            }
            apply_expr_scan(Some(expr), state, defs);
        }
        Statement::Incr { name, amount, .. } => {
            if is_dynamic_name(name) {
                if let Some(literal) = state.resolve_literal(name) {
                    state.escape(&literal, defs);
                } else {
                    state.escape_all_known(defs);
                }
            } else {
                state.invalidate_literal(name);
            }
            if let Some(a) = amount {
                apply_value_scan(a, state, defs);
            }
        }
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
    }
}

/// Process one SSA block — every statement plus the terminator's
/// branch condition (so `[info exists ...]` inside an `if`
/// condition isn't missed).
fn walk_block(block: &SsaBlock, state: &mut CfgState, terminator_condition: Option<&ExprNode>) {
    for stmt in &block.statements {
        if state.dynamic_barrier {
            return;
        }
        handle_statement(stmt, state);
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
fn block_order(cfg: &CfgFunction) -> Vec<String> {
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

    for block_name in block_order(cfg) {
        let Some(block) = ssa.blocks.get(&block_name) else {
            continue;
        };
        let term_cond = cfg
            .blocks
            .get(&block_name)
            .and_then(|b| b.terminator.as_ref())
            .and_then(|t| match t {
                Terminator::Branch { condition, .. } => Some(condition),
                _ => None,
            });
        walk_block(block, &mut state, term_cond);
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
        assert!(!r.dynamic_barrier);
        assert!(r.name_tags.is_empty());
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
        assert!(r.dynamic_barrier);
    }

    #[test]
    fn info_exists_literal_escapes_target() {
        let r = analyse("info exists myvar");
        assert_eq!(r.name_tags.get("myvar"), Some(&EscapeTag::Frame));
    }

    #[test]
    fn unknown_command_sets_call_fallback() {
        let r = analyse("some_user_proc arg");
        assert!(r.has_call_fallback);
    }

    #[test]
    fn frameless_runtime_call_does_not_set_call_fallback() {
        let r = analyse("string length foo");
        assert!(!r.has_call_fallback);
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
