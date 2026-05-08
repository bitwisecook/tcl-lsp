//! Intra-procedural transfer functions + walker (C33b5/6/7).
//!
//! Pulls together:
//!
//! * **C33b5** — barrier handlers (`eval`, `uplevel`, generic
//!   barriers) plus `escape_every_name_touched` for literal eval
//!   bodies.
//! * **C33b6** — `handle_call` dispatcher that routes a
//!   [`Statement::Call`] to the per-command handlers in
//!   [`super::handlers`], plus value/expr scans for embedded
//!   `[info ...]` hazards and non-frameless command substitutions.
//! * **C33b7** — `walk` (recursive structural traversal) and
//!   [`analyse_script`] (the public per-proc entry point).
//!
//! Mirrors the bottom half of
//! `core/compiler/var_escape/_propagation.py`.

use crate::expr_ast::ExprNode;
use crate::ir::{Script, Statement};
use crate::var_escape::handlers::{
    handle_dynamic_name_first, handle_global, handle_info, handle_namespace_call, handle_upvar,
    handle_variable, has_expand_word,
};
use crate::var_escape::helpers::{
    is_dynamic_name, is_dynamic_token, is_frameless_runtime_command, is_name_first_command,
    normalise_cmd_subst_head, scan_value_for_info_hazards,
};
use crate::var_escape::known_names::collect_known_names;
use crate::var_escape::state::EscapeState;
use crate::var_escape::types::{EscapeTag, ProcEscapeSummary};

// Walk a value text looking for embedded ``[cmd ...]`` substitution
// heads. Apply [`scan_value_for_info_hazards`] for ``info`` shapes
// and flag a fallback when any non-frameless head appears.
pub(crate) fn apply_value_scan(value: &str, state: &mut EscapeState) {
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
        state.escape(&n);
    }
}

/// Apply the value scan to an expression's rendered source.
pub(crate) fn apply_expr_scan(expr: Option<&ExprNode>, state: &mut EscapeState) {
    let Some(expr) = expr else {
        return;
    };
    // ``ExprNode`` doesn't currently carry its own source text on
    // every variant; rendering walks the tree and returns the
    // canonical Tcl source. The hazard scan only needs to find
    // ``[info ...]`` substitutions, which appear verbatim in the
    // rendered text.
    let text = crate::expr_ast::render_expr(expr);
    apply_value_scan(&text, state);
}

/// Find the leading command-word of every `[cmd ...]` substitution
/// in *value*. Trims leading whitespace inside the brackets.
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
        // Optional ``::`` qualifier prefix.
        let head_start = j;
        if value[j..].starts_with("::") {
            j += 2;
        }
        // Identifier head: leading alpha/underscore, then alphanumerics
        // / underscore / colons.
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

/// Handle a generic [`Statement::Call`] — dispatch to the per-
/// command handlers and record callee / fallback flags.
pub(crate) fn handle_call(stmt: &Statement, state: &mut EscapeState) {
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

    // Commands outside the frameless-runtime allow-list could
    // reach the eval fallback via the codegen. Mark accordingly.
    if !is_frameless_runtime_command(cmd) {
        if cmd.is_empty() || is_dynamic_token(cmd) {
            state.record_fallback();
        } else {
            state.record_call_fallback();
        }
    }

    // ``{*}``-expansion in an unknown call defeats argument-index-
    // based analysis (we can't tell where the name arg landed).
    if has_expand_word(tokens.as_ref()) && cmd != "list" && cmd != "concat" {
        use crate::var_escape::types::{Barrier, BarrierKind};
        state.record_barrier(Barrier::with_detail(
            BarrierKind::Expand,
            format!("{{*}}-expansion in {cmd}"),
        ));
        return;
    }

    match cmd {
        "upvar" => handle_upvar(args, state),
        "global" => handle_global(args, state),
        "variable" => handle_variable(args, state),
        "namespace" => handle_namespace_call(args, state),
        "info" => handle_info(args, state),
        c if is_name_first_command(c) => handle_dynamic_name_first(c, args, state),
        _ => {} // defs/reads already cover the static case.
    }

    // Record statically resolvable callees for interprocedural
    // propagation. Bare or ``::``-qualified command words are
    // candidates; substitutions are ignored.
    if !cmd.is_empty() && !is_dynamic_token(cmd) {
        state.record_callee(cmd);
    }
}

/// Handle an ``eval`` barrier: literal body is recursively walked
/// with `escape_every_name_touched`; non-literal body is
/// pessimistic.
fn handle_eval(args: &[String], state: &mut EscapeState) {
    use crate::var_escape::types::{Barrier, BarrierKind, EscapeReason, EscapeReasonKind};
    if args.is_empty() {
        state.record_barrier(Barrier::with_detail(BarrierKind::Eval, "eval (no body)"));
        return;
    }
    let body: String = if args.len() == 1 {
        args[0].clone()
    } else {
        args.join(" ")
    };
    if is_dynamic_token(&body) {
        state.record_barrier(Barrier::with_detail(
            BarrierKind::Eval,
            "eval (dynamic body)",
        ));
        return;
    }
    // Cheap scan first — any ``$var`` reference escapes that name.
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut scanner =
        crate::var_refs::VarReferenceScanner::new(crate::var_refs::VarScanOptions::default());
    for ref_ in scanner.scan_script(&body, &registry) {
        state.escape_with_reason(
            &ref_,
            EscapeReason::with_detail(
                EscapeReasonKind::EvalReference,
                format!("eval body references ${ref_}"),
            ),
        );
    }
    // Recurse into the literal body and escape every name it
    // touches.
    let sub_module = crate::lowering::lower_to_ir(&body, &registry);
    escape_every_name_touched(&sub_module.top_level.statements, state);
}

/// Handle an ``uplevel`` barrier: only ``#0`` / ``0`` with a
/// literal body is safe (body runs at global scope, our locals
/// aren't visible). Everything else is pessimistic.
fn handle_uplevel(args: &[String], state: &mut EscapeState) {
    use crate::var_escape::types::{Barrier, BarrierKind};
    if args.is_empty() {
        state.record_barrier(Barrier::with_detail(BarrierKind::Upvar, "uplevel (no body)"));
        return;
    }
    let first = &args[0];
    let no_dash = first.trim_start_matches('-');
    let is_level_literal = (!no_dash.is_empty() && no_dash.chars().all(|c| c.is_ascii_digit()))
        || (first.starts_with('#') && first[1..].chars().all(|c| c.is_ascii_digit()));
    if !is_level_literal {
        state.record_barrier(Barrier::with_detail(
            BarrierKind::Upvar,
            format!("uplevel {first}"),
        ));
        return;
    }
    if first != "#0" && first != "0" {
        state.record_barrier(Barrier::with_detail(
            BarrierKind::Upvar,
            format!("uplevel {first}"),
        ));
        return;
    }
    let body_parts = &args[1..];
    if body_parts.is_empty() {
        state.record_barrier(Barrier::with_detail(
            BarrierKind::Upvar,
            "uplevel #0 (no body)",
        ));
        return;
    }
    let body: String = if body_parts.len() == 1 {
        body_parts[0].clone()
    } else {
        body_parts.join(" ")
    };
    if is_dynamic_token(&body) {
        state.record_barrier(Barrier::with_detail(
            BarrierKind::Upvar,
            "uplevel #0 (dynamic body)",
        ));
    }
}

/// Dispatch on the barrier command name.
fn handle_barrier(command: &str, args: &[String], state: &mut EscapeState) {
    use crate::var_escape::types::{Barrier, BarrierKind};
    // Any IRBarrier means the codegen can dispatch to the
    // interpreter; the proc prologue must push a frame so the
    // fallback sees locals.
    state.record_fallback();
    match command {
        "eval" => handle_eval(args, state),
        "uplevel" => handle_uplevel(args, state),
        _ => {
            // Any other barrier (subst, trace, catch reraise, …)
            // — be safe.
            state.record_barrier(Barrier::with_detail(
                BarrierKind::Unknown,
                format!("{command} (uncategorised barrier)"),
            ));
        }
    }
}

/// Reconstitute an IRBarrier-style view of an eval-shape
/// `Statement::Block` for the escape walk.
fn synthesise_eval_args(block_tokens: Option<&crate::ir::CommandTokens>) -> Vec<String> {
    block_tokens
        .map(|t| t.argv_texts.iter().skip(1).cloned().collect())
        .unwrap_or_default()
}

fn synthesise_uplevel_args(tokens: Option<&crate::ir::CommandTokens>) -> Vec<String> {
    tokens
        .map(|t| t.argv_texts.iter().skip(1).cloned().collect())
        .unwrap_or_default()
}

/// True when an [`Statement::Block`] was produced by relaxing
/// `eval` (vs `namespace eval`).
fn is_eval_block(tokens: Option<&crate::ir::CommandTokens>) -> bool {
    tokens
        .and_then(|t| t.argv_texts.first())
        .is_some_and(|s| s == "eval")
}

/// Escape every literal Tcl name the body writes, reads, or
/// declares. Used for literal `eval` / `uplevel #0` bodies — the
/// body runs through the interpreter which resolves names against
/// the frame, so any name it touches must be visible there.
// Long match dispatcher over Statement variants in the var_escape walker.
#[allow(clippy::too_many_lines)]
pub(crate) fn escape_every_name_touched(stmts: &[Statement], state: &mut EscapeState) {
    for stmt in stmts {
        if state.dynamic_barrier() {
            return;
        }
        match stmt {
            Statement::AssignConst { name, value, .. }
            | Statement::AssignValue { name, value, .. } => {
                if name.is_empty() || is_dynamic_token(name) {
                    state.mark_pessimistic();
                    return;
                }
                state.escape(name);
                apply_value_scan(value, state);
            }
            Statement::AssignExpr { name, expr, .. } => {
                if name.is_empty() || is_dynamic_token(name) {
                    state.mark_pessimistic();
                    return;
                }
                state.escape(name);
                apply_expr_scan(Some(expr), state);
            }
            Statement::Incr { name, amount, .. } => {
                if name.is_empty() || is_dynamic_token(name) {
                    state.mark_pessimistic();
                    return;
                }
                state.escape(name);
                if let Some(a) = amount {
                    apply_value_scan(a, state);
                }
            }
            Statement::Call { defs, reads, .. } => {
                for n in defs.iter().chain(reads.iter()) {
                    if !n.is_empty() && !is_dynamic_token(n) {
                        state.escape(n);
                    }
                }
                handle_call(stmt, state);
            }
            Statement::Barrier { command, args, .. } => handle_barrier(command, args, state),
            Statement::Return { value, expr, .. } => {
                if let Some(v) = value {
                    apply_value_scan(v, state);
                }
                apply_expr_scan(expr.as_ref(), state);
            }
            Statement::ExprEval { expr, .. } => apply_expr_scan(Some(expr), state),
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    apply_expr_scan(Some(&c.condition), state);
                    escape_every_name_touched(&c.body.statements, state);
                }
                if let Some(b) = else_body {
                    escape_every_name_touched(&b.statements, state);
                }
            }
            Statement::For {
                init,
                condition,
                next,
                body,
                ..
            } => {
                escape_every_name_touched(&init.statements, state);
                apply_expr_scan(Some(condition), state);
                escape_every_name_touched(&next.statements, state);
                escape_every_name_touched(&body.statements, state);
            }
            Statement::While {
                condition, body, ..
            } => {
                apply_expr_scan(Some(condition), state);
                escape_every_name_touched(&body.statements, state);
            }
            Statement::Foreach {
                iterators, body, ..
            } => {
                for it in iterators {
                    apply_value_scan(&it.list_arg, state);
                }
                escape_every_name_touched(&body.statements, state);
            }
            Statement::Catch { body, .. } => escape_every_name_touched(&body.statements, state),
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                escape_every_name_touched(&body.statements, state);
                for h in handlers {
                    escape_every_name_touched(&h.body.statements, state);
                }
                if let Some(f) = finally_body {
                    escape_every_name_touched(&f.statements, state);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        escape_every_name_touched(&b.statements, state);
                    }
                }
                if let Some(d) = default_body {
                    escape_every_name_touched(&d.statements, state);
                }
            }
            Statement::Block { body, tokens, .. } => {
                if is_eval_block(tokens.as_ref()) {
                    let args = synthesise_eval_args(tokens.as_ref());
                    handle_barrier("eval", &args, state);
                } else {
                    escape_every_name_touched(&body.statements, state);
                }
            }
            Statement::UpFrame { tokens, .. } => {
                let args = synthesise_uplevel_args(tokens.as_ref());
                handle_barrier("uplevel", &args, state);
            }
        }
    }
}

/// Walk *stmts* with the standard escape-rule transfer functions.
// Long match dispatcher over Statement variants in the var_escape walker.
#[allow(clippy::too_many_lines)]
fn walk(stmts: &[Statement], state: &mut EscapeState) {
    for stmt in stmts {
        if state.dynamic_barrier() {
            return;
        }
        match stmt {
            Statement::Call { .. } => handle_call(stmt, state),
            Statement::Barrier { command, args, .. } => handle_barrier(command, args, state),
            Statement::UpFrame { tokens, .. } => {
                let args = synthesise_uplevel_args(tokens.as_ref());
                handle_barrier("uplevel", &args, state);
            }
            Statement::AssignConst { name, value, .. } => {
                if is_dynamic_name(name) {
                    if let Some(literal) = state.resolve_literal(name) {
                        state.escape(&literal);
                    } else {
                        state.escape_all_known();
                    }
                } else {
                    state.note_literal_assign(name, value);
                }
                apply_value_scan(value, state);
            }
            Statement::AssignValue { name, value, .. } => {
                if is_dynamic_name(name) {
                    if let Some(literal) = state.resolve_literal(name) {
                        state.escape(&literal);
                    } else {
                        state.escape_all_known();
                    }
                } else if !value.is_empty() && !is_dynamic_token(value) {
                    state.note_literal_assign(name, value);
                } else {
                    state.invalidate_literal(name);
                }
                apply_value_scan(value, state);
            }
            Statement::AssignExpr { name, expr, .. } => {
                if is_dynamic_name(name) {
                    if let Some(literal) = state.resolve_literal(name) {
                        state.escape(&literal);
                    } else {
                        state.escape_all_known();
                    }
                } else {
                    state.invalidate_literal(name);
                }
                apply_expr_scan(Some(expr), state);
            }
            Statement::Incr { name, amount, .. } => {
                if is_dynamic_name(name) {
                    if let Some(literal) = state.resolve_literal(name) {
                        state.escape(&literal);
                    } else {
                        state.escape_all_known();
                    }
                } else {
                    state.invalidate_literal(name);
                }
                if let Some(a) = amount {
                    apply_value_scan(a, state);
                }
            }
            Statement::Return { value, expr, .. } => {
                if let Some(v) = value {
                    apply_value_scan(v, state);
                }
                apply_expr_scan(expr.as_ref(), state);
            }
            Statement::ExprEval { expr, .. } => apply_expr_scan(Some(expr), state),
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    apply_expr_scan(Some(&c.condition), state);
                    walk(&c.body.statements, state);
                }
                if let Some(b) = else_body {
                    walk(&b.statements, state);
                }
            }
            Statement::For {
                init,
                condition,
                next,
                body,
                ..
            } => {
                walk(&init.statements, state);
                apply_expr_scan(Some(condition), state);
                walk(&next.statements, state);
                walk(&body.statements, state);
            }
            Statement::While {
                condition, body, ..
            } => {
                apply_expr_scan(Some(condition), state);
                walk(&body.statements, state);
            }
            Statement::Foreach {
                iterators, body, ..
            } => {
                for it in iterators {
                    apply_value_scan(&it.list_arg, state);
                }
                walk(&body.statements, state);
            }
            Statement::Catch { body, .. } => walk(&body.statements, state),
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                walk(&body.statements, state);
                for h in handlers {
                    walk(&h.body.statements, state);
                }
                if let Some(f) = finally_body {
                    walk(&f.statements, state);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        walk(&b.statements, state);
                    }
                }
                if let Some(d) = default_body {
                    walk(&d.statements, state);
                }
            }
            Statement::Block { body, tokens, .. } => {
                if is_eval_block(tokens.as_ref()) {
                    let args = synthesise_eval_args(tokens.as_ref());
                    handle_barrier("eval", &args, state);
                } else {
                    walk(&body.statements, state);
                }
            }
        }
    }
}

/// Run the intra-procedural escape analysis over *body* and
/// return the resulting [`ProcEscapeSummary`].
///
/// The returned summary is *intra-procedural* — callee-induced
/// escapes haven't been folded in yet. Run the interprocedural
/// pass (C33d) to produce the final summary the codegen should
/// consume.
#[must_use]
pub fn analyse_script<I: IntoIterator<Item = String>>(
    body: &Script,
    params: I,
) -> ProcEscapeSummary {
    let known = collect_known_names(params, body);
    let mut state = EscapeState::new(known);
    walk(&body.statements, &mut state);
    let frame_needed =
        state.dynamic_barrier() || state.tags.values().any(|t| *t == EscapeTag::Frame);
    ProcEscapeSummary {
        tags: state.tags,
        flags: state.flags,
        frame_needed,
        upvar_source_names: state.upvar_source_names,
        direct_callees: state.direct_callees,
        ssa_tags: std::collections::HashMap::new(),
        local_slots: std::collections::BTreeMap::new(),
        // SYNC-JUN-FRAME356-population: thread the structured
        // per-proc barriers + per-name escape reasons recorded by
        // the handlers into the summary so downstream consumers
        // (LSP hover, compiler-explorer surface) can render the
        // specific trigger instead of opaque "dynamic barrier".
        barriers: state.barriers,
        tag_reasons: state.tag_reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir;
    use tcl_registry::CommandRegistry;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn analyse(src: &str) -> ProcEscapeSummary {
        let m = lower_to_ir(src, &reg());
        analyse_script(&m.top_level, std::iter::empty::<String>())
    }

    #[test]
    fn pure_set_does_not_escape() {
        let s = analyse("set x 1");
        assert!(!s.dynamic_barrier());
        assert!(!s.is_frame("x"));
    }

    #[test]
    fn upvar_escapes_local_alias() {
        let s = analyse("upvar 1 caller_x x");
        assert!(s.is_frame("x"));
        assert!(s.upvar_source_names.contains("caller_x"));
    }

    #[test]
    fn global_escapes_named_var() {
        let s = analyse("global g");
        assert!(s.is_frame("g"));
    }

    #[test]
    fn variable_escapes_named_vars() {
        let s = analyse("variable a 1 b 2");
        assert!(s.is_frame("a"));
        assert!(s.is_frame("b"));
    }

    #[test]
    fn info_level_marks_pessimistic() {
        let s = analyse("info level");
        assert!(s.dynamic_barrier());
    }

    #[test]
    fn info_exists_literal_escapes_target() {
        let s = analyse("info exists myvar");
        assert!(s.is_frame("myvar"));
    }

    #[test]
    fn upvar_with_dynamic_level_marks_pessimistic() {
        let s = analyse("upvar $lvl src dst");
        assert!(s.dynamic_barrier());
    }

    #[test]
    fn frameless_runtime_call_does_not_set_call_fallback() {
        let s = analyse("string length foo");
        assert!(!s.has_call_fallback());
    }

    #[test]
    fn unknown_command_sets_call_fallback() {
        let s = analyse("some_user_proc arg");
        assert!(s.has_call_fallback());
    }

    #[test]
    fn dynamic_command_sets_record_fallback() {
        let s = analyse("$cmd arg");
        assert!(s.has_fallback());
    }

    #[test]
    fn descends_into_if_body() {
        let s = analyse("if {1} { upvar 1 caller_x x }");
        assert!(s.is_frame("x"));
    }

    // -- SYNC-JUN-FRAME356-population ---------------------------------
    //
    // Mirrors `015288cf` (PR #356) — `barriers` and `tag_reasons`
    // are populated by the handlers, not synthesised on demand.

    #[test]
    fn population_info_level_records_info_barrier() {
        use crate::var_escape::types::BarrierKind;
        let s = analyse("info level");
        assert!(s.dynamic_barrier());
        assert_eq!(s.barriers.len(), 1, "barriers={:?}", s.barriers);
        assert_eq!(s.barriers[0].kind, BarrierKind::Info);
        assert!(s.barriers[0].detail.contains("info level"));
    }

    #[test]
    fn population_info_exists_records_per_name_reason() {
        use crate::var_escape::types::EscapeReasonKind;
        let s = analyse("info exists myvar");
        assert!(s.is_frame("myvar"));
        let reasons = s.tag_reasons.get("myvar").expect("myvar reasons");
        assert_eq!(reasons.len(), 1);
        assert_eq!(reasons[0].kind, EscapeReasonKind::InfoExists);
        assert!(reasons[0].detail.contains("info exists myvar"));
    }

    #[test]
    fn population_upvar_records_upvar_source_reason() {
        use crate::var_escape::types::EscapeReasonKind;
        let s = analyse("upvar 1 caller_x x");
        assert!(s.is_frame("x"));
        let reasons = s.tag_reasons.get("x").expect("x reasons");
        assert!(
            reasons.iter().any(|r| r.kind == EscapeReasonKind::UpvarSource),
            "expected UpvarSource reason, got {reasons:?}",
        );
    }

    #[test]
    fn population_dynamic_upvar_records_upvar_barrier() {
        use crate::var_escape::types::BarrierKind;
        let s = analyse("upvar $level caller_x x");
        assert!(s.dynamic_barrier());
        assert!(
            s.barriers.iter().any(|b| b.kind == BarrierKind::Upvar),
            "expected Upvar barrier, got {:?}",
            s.barriers,
        );
    }

    #[test]
    fn population_eval_dynamic_records_eval_barrier() {
        use crate::var_escape::types::BarrierKind;
        let s = analyse("eval $body");
        assert!(s.dynamic_barrier());
        assert!(
            s.barriers.iter().any(|b| b.kind == BarrierKind::Eval),
            "expected Eval barrier, got {:?}",
            s.barriers,
        );
    }
}
