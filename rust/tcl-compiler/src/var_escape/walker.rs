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

//! Intra-procedural transfer functions + walker.
//!
//! Pulls together:
//!
//! * Barrier handlers (`eval`, `uplevel`, generic
//!   barriers) plus `escape_every_name_touched` for literal eval
//!   bodies.
//! * The `handle_call` dispatcher that routes a
//!   [`Statement::Call`] to the per-command handlers in
//!   [`super::handlers`], plus value/expr scans for embedded
//!   `[info ...]` hazards and non-frameless command substitutions.
//! * `walk` (recursive structural traversal) and
//!   [`analyse_script`] (the public per-proc entry point).

use crate::expr_ast::ExprNode;
use crate::ir::{Script, Statement};
use crate::var_escape::handlers::{
    handle_dynamic_name_first, handle_introspection, handle_variable_aliases, has_expand_word,
};
use crate::var_escape::helpers::{
    default_registry, invocation_facts, invocation_facts_from_tokens, is_dynamic_name,
    is_dynamic_token, is_frameless_runtime_command, normalise_cmd_subst_head,
    scan_value_for_info_hazards,
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
pub(crate) fn handle_call(
    stmt: &Statement,
    state: &mut EscapeState,
    registry: &tcl_registry::CommandRegistry,
) {
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
    let facts = invocation_facts(stmt, registry);

    // Commands outside the frameless-runtime allow-list could
    // reach the eval fallback via the codegen. Mark accordingly.
    if !facts.as_ref().is_some_and(|facts| {
        facts
            .traits
            .contains(tcl_registry::prelude::Traits::FRAMELESS_RUNTIME)
    }) {
        if cmd.is_empty() || is_dynamic_token(cmd) {
            state.record_fallback();
        } else {
            state.record_call_fallback();
        }
    }

    // ``{*}``-expansion in an unknown call defeats argument-index-
    // based analysis (we can't tell where the name arg landed).
    if has_expand_word(tokens.as_ref())
        && !facts.as_ref().is_some_and(|facts| {
            facts
                .traits
                .contains(tcl_registry::prelude::Traits::EXPANSION_ESCAPE_SAFE)
        })
    {
        use crate::var_escape::types::{Barrier, BarrierKind};
        state.record_barrier(Barrier::with_detail(
            BarrierKind::Expand,
            format!("{{*}}-expansion in {cmd}"),
        ));
        return;
    }

    if let Some(facts) = facts.as_deref() {
        handle_variable_aliases(facts, state);
        handle_introspection(facts, args, state);
        if facts
            .traits
            .contains(tcl_registry::prelude::Traits::FIRST_ARG_VARNAME)
        {
            handle_dynamic_name_first(&facts.canonical_command, args, state);
        }
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
fn handle_eval(args: &[String], state: &mut EscapeState, registry: &tcl_registry::CommandRegistry) {
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
    //
    // Deliberately ``scan_word``, not ``scan_script``: escape analysis wants
    // *every name the text mentions*, not the names Tcl's own word-splitting
    // says are substituted at this level. A brace-quoted word inside the body
    // (`eval {if {$x > 1} {...}}` — the expression is braced) suppresses
    // substitution here but is re-parsed and substituted when the inner
    // command runs, so the name really does escape. Missing it would be
    // unsound in the direction that matters: a variable wrongly believed
    // frame-local is one the optimiser may keep in a register.
    //
    // Note the ``is_dynamic_token`` guard above already takes the pessimistic
    // path for *any* body containing ``$`` or ``[``, which is every body this
    // scan could find a reference in — so today the mode is unobservable and
    // the guard is what carries the soundness. It is written as ``scan_word``
    // so that narrowing that guard cannot silently reintroduce the hole.
    let mut scanner =
        crate::var_refs::VarReferenceScanner::new(crate::var_refs::VarScanOptions::default());
    for ref_ in scanner.scan_word(&body, registry) {
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
    let sub_module = crate::lowering::lower_to_ir(&body, registry);
    escape_every_name_touched(&sub_module.top_level.statements, state, registry);
}

/// Handle an ``uplevel`` barrier: only ``#0`` / ``0`` with a
/// literal body is safe (body runs at global scope, our locals
/// aren't visible). Everything else is pessimistic.
fn handle_uplevel(
    args: &[String],
    state: &mut EscapeState,
    registry: &tcl_registry::CommandRegistry,
) {
    use crate::var_escape::types::{Barrier, BarrierKind};
    if args.is_empty() {
        state.record_barrier(Barrier::with_detail(
            BarrierKind::Upvar,
            "uplevel (no body)",
        ));
        return;
    }
    let first = &args[0];
    // The level word's grammar is registry data
    // ([`tcl_registry::frame_effect::FrameLevel`]), not a local digit sniff:
    // C Tcl reads `+0`, `-0`, `#-0`, `0x0`, and `" 0"` as level 0 too, and a
    // sniff that missed them sent a provably-safe body down the pessimistic
    // path.
    let Some(level) = tcl_registry::frame_effect::FrameLevel::parse(first) else {
        state.record_barrier(Barrier::with_detail(
            BarrierKind::Upvar,
            format!("uplevel {first}"),
        ));
        return;
    };
    // `uplevel 0` runs the body in the *current* frame, so a `set x` inside it
    // name-writes our proc's local `x` exactly like `eval`. Walk it the same
    // way and escape every name it touches. Treating `0` as global-safe like
    // `#0` wrongly whitelists our own frame.
    if level.is_current_frame() {
        handle_eval(&args[1..], state, registry);
        return;
    }
    // Only `uplevel #0` (global scope) is safe: our locals aren't visible
    // there, so a literal body can't touch them. Any other level runs in a
    // different caller frame — handled pessimistically.
    if !level.is_global_frame() {
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
fn handle_barrier(
    stmt: &Statement,
    args: &[String],
    state: &mut EscapeState,
    registry: &tcl_registry::CommandRegistry,
) {
    use crate::var_escape::types::{Barrier, BarrierKind};
    // Any IRBarrier means the codegen can dispatch to the
    // interpreter; the proc prologue must push a frame so the
    // fallback sees locals.
    state.record_fallback();
    let operation = invocation_facts(stmt, registry).map(|facts| facts.operation);
    match operation {
        Some(tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Eval,
        )) => handle_eval(args, state, registry),
        Some(tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Uplevel,
        )) => handle_uplevel(args, state, registry),
        _ => {
            // Any other barrier (subst, trace, catch reraise, …)
            // — be safe.
            state.record_barrier(Barrier::with_detail(
                BarrierKind::Unknown,
                "uncategorised registry barrier",
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
fn is_eval_block(
    tokens: Option<&crate::ir::CommandTokens>,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    tokens
        .and_then(|tokens| invocation_facts_from_tokens(tokens, registry))
        .is_some_and(|facts| {
            facts.operation
                == tcl_registry::SemanticOperationId::StructuredLowering(
                    tcl_registry::hooks::LoweringHookId::Eval,
                )
        })
}

/// Escape every literal Tcl name the body writes, reads, or
/// declares. Used for literal `eval` / `uplevel #0` bodies — the
/// body runs through the interpreter which resolves names against
/// the frame, so any name it touches must be visible there.
pub(crate) fn escape_every_name_touched(
    stmts: &[Statement],
    state: &mut EscapeState,
    registry: &tcl_registry::CommandRegistry,
) {
    for stmt in stmts {
        if state.dynamic_barrier() {
            return;
        }
        if escape_assign_or_incr(stmt, state) {
            continue;
        }
        if escape_call_or_barrier(stmt, state, registry) {
            continue;
        }
        escape_structural(stmt, state, registry);
    }
}

/// `escape_every_name_touched` arm: assignment / increment shapes.
/// Returns `true` when *stmt* matched.
fn escape_assign_or_incr(stmt: &Statement, state: &mut EscapeState) -> bool {
    match stmt {
        Statement::AssignConst { name, value, .. } | Statement::AssignValue { name, value, .. } => {
            if name.is_empty() || is_dynamic_token(name) {
                state.mark_pessimistic();
                return true;
            }
            state.escape(name);
            apply_value_scan(value, state);
            true
        }
        Statement::AssignExpr { name, expr, .. } => {
            if name.is_empty() || is_dynamic_token(name) {
                state.mark_pessimistic();
                return true;
            }
            state.escape(name);
            apply_expr_scan(Some(expr), state);
            true
        }
        Statement::Incr { name, amount, .. } => {
            if name.is_empty() || is_dynamic_token(name) {
                state.mark_pessimistic();
                return true;
            }
            state.escape(name);
            if let Some(a) = amount {
                apply_value_scan(a, state);
            }
            true
        }
        _ => false,
    }
}

/// `escape_every_name_touched` arm: Call / Barrier / Return /
/// `ExprEval` shapes.  Returns `true` when *stmt* matched.
fn escape_call_or_barrier(
    stmt: &Statement,
    state: &mut EscapeState,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    match stmt {
        Statement::Call { defs, reads, .. } => {
            for n in defs.iter().chain(reads.iter()) {
                if !n.is_empty() && !is_dynamic_token(n) {
                    state.escape(n);
                }
            }
            handle_call(stmt, state, registry);
            true
        }
        Statement::Barrier { args, .. } => {
            handle_barrier(stmt, args, state, registry);
            true
        }
        Statement::Return { value, expr, .. } => {
            if let Some(v) = value {
                apply_value_scan(v, state);
            }
            apply_expr_scan(expr.as_ref(), state);
            true
        }
        Statement::ExprEval { expr, .. } => {
            apply_expr_scan(Some(expr), state);
            true
        }
        _ => false,
    }
}

/// `escape_every_name_touched` arm: structural recursion (`If` /
/// `For` / `While` / `Foreach` / `Catch` / `Try` / `Switch` /
/// `Block` / `UpFrame`).
fn escape_structural(
    stmt: &Statement,
    state: &mut EscapeState,
    registry: &tcl_registry::CommandRegistry,
) {
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                apply_expr_scan(Some(&c.condition), state);
                escape_every_name_touched(&c.body.statements, state, registry);
            }
            if let Some(b) = else_body {
                escape_every_name_touched(&b.statements, state, registry);
            }
        }
        Statement::For {
            init,
            condition,
            next,
            body,
            ..
        } => {
            escape_every_name_touched(&init.statements, state, registry);
            apply_expr_scan(Some(condition), state);
            escape_every_name_touched(&next.statements, state, registry);
            escape_every_name_touched(&body.statements, state, registry);
        }
        Statement::While {
            condition, body, ..
        } => {
            apply_expr_scan(Some(condition), state);
            escape_every_name_touched(&body.statements, state, registry);
        }
        Statement::Foreach {
            iterators, body, ..
        } => {
            for it in iterators {
                apply_value_scan(&it.list_arg, state);
            }
            escape_every_name_touched(&body.statements, state, registry);
        }
        Statement::Catch { body, .. } => {
            escape_every_name_touched(&body.statements, state, registry);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            escape_every_name_touched(&body.statements, state, registry);
            for h in handlers {
                escape_every_name_touched(&h.body.statements, state, registry);
            }
            if let Some(f) = finally_body {
                escape_every_name_touched(&f.statements, state, registry);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    escape_every_name_touched(&b.statements, state, registry);
                }
            }
            if let Some(d) = default_body {
                escape_every_name_touched(&d.statements, state, registry);
            }
        }
        Statement::Block { body, tokens, .. } => {
            if is_eval_block(tokens.as_ref(), registry) {
                let args = synthesise_eval_args(tokens.as_ref());
                state.record_fallback();
                handle_eval(&args, state, registry);
            } else {
                escape_every_name_touched(&body.statements, state, registry);
            }
        }
        Statement::UpFrame { tokens, .. } => {
            let args = synthesise_uplevel_args(tokens.as_ref());
            state.record_fallback();
            handle_uplevel(&args, state, registry);
        }
        _ => {}
    }
}

/// Resolve a (possibly-dynamic) name and call `escape` — used by
/// the `walk` arms.
fn walk_dynamic_name_escape(state: &mut EscapeState, name: &str) {
    if let Some(literal) = state.resolve_literal(name) {
        state.escape(&literal);
    } else {
        state.escape_all_known();
    }
}

fn walk_assign_or_incr(stmt: &Statement, state: &mut EscapeState) -> bool {
    match stmt {
        Statement::AssignConst { name, value, .. } => {
            if is_dynamic_name(name) {
                walk_dynamic_name_escape(state, name);
            } else {
                state.note_literal_assign(name, value);
            }
            apply_value_scan(value, state);
            true
        }
        Statement::AssignValue { name, value, .. } => {
            if is_dynamic_name(name) {
                walk_dynamic_name_escape(state, name);
            } else if !value.is_empty() && !is_dynamic_token(value) {
                state.note_literal_assign(name, value);
            } else {
                state.invalidate_literal(name);
            }
            apply_value_scan(value, state);
            true
        }
        Statement::AssignExpr { name, expr, .. } => {
            if is_dynamic_name(name) {
                walk_dynamic_name_escape(state, name);
            } else {
                state.invalidate_literal(name);
            }
            apply_expr_scan(Some(expr), state);
            true
        }
        Statement::Incr { name, amount, .. } => {
            if is_dynamic_name(name) {
                walk_dynamic_name_escape(state, name);
            } else {
                state.invalidate_literal(name);
            }
            if let Some(a) = amount {
                apply_value_scan(a, state);
            }
            true
        }
        _ => false,
    }
}

/// Depth cap for [`walk`]'s recursion over nested `if`/`for`/`while`/
/// `foreach`/`catch`/`try`/`switch`/`Block` bodies — issue #996.
/// Transitively bounded today via `MAX_LOWER_NEST_DEPTH` (every `Script`
/// this walk sees was built by `crate::lowering`, which already caps its
/// own construction at 256), capped here independently for
/// defence-in-depth and consistency with every other full-tree walker in
/// this crate.
const MAX_ESCAPE_WALK_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(256);

/// Walk *stmts* with the standard escape-rule transfer functions. `depth`
/// is the nesting level of `stmts` — see [`MAX_ESCAPE_WALK_DEPTH`].
fn walk(
    stmts: &[Statement],
    state: &mut EscapeState,
    depth: u32,
    registry: &tcl_registry::CommandRegistry,
) {
    if MAX_ESCAPE_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in stmts {
        if state.dynamic_barrier() {
            return;
        }
        walk_statement(stmt, state, depth, registry);
    }
}

fn walk_statement(
    stmt: &Statement,
    state: &mut EscapeState,
    depth: u32,
    registry: &tcl_registry::CommandRegistry,
) {
    if walk_assign_or_incr(stmt, state) {
        return;
    }
    match stmt {
        Statement::Call { .. } => handle_call(stmt, state, registry),
        Statement::Barrier { args, .. } => handle_barrier(stmt, args, state, registry),
        Statement::UpFrame { tokens, .. } => {
            let args = synthesise_uplevel_args(tokens.as_ref());
            state.record_fallback();
            handle_uplevel(&args, state, registry);
        }
        Statement::Return { value, expr, .. } => {
            if let Some(v) = value {
                apply_value_scan(v, state);
            }
            apply_expr_scan(expr.as_ref(), state);
        }
        Statement::ExprEval { expr, .. } => apply_expr_scan(Some(expr), state),
        _ => walk_structured_statement(stmt, state, depth, registry),
    }
}

fn walk_structured_statement(
    stmt: &Statement,
    state: &mut EscapeState,
    depth: u32,
    registry: &tcl_registry::CommandRegistry,
) {
    let nested_depth = depth + 1;
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            for clause in clauses {
                apply_expr_scan(Some(&clause.condition), state);
                walk(&clause.body.statements, state, nested_depth, registry);
            }
            if let Some(body) = else_body {
                walk(&body.statements, state, nested_depth, registry);
            }
        }
        Statement::For {
            init,
            condition,
            next,
            body,
            ..
        } => {
            walk(&init.statements, state, nested_depth, registry);
            apply_expr_scan(Some(condition), state);
            walk(&next.statements, state, nested_depth, registry);
            walk(&body.statements, state, nested_depth, registry);
        }
        Statement::While {
            condition, body, ..
        } => {
            apply_expr_scan(Some(condition), state);
            walk(&body.statements, state, nested_depth, registry);
        }
        Statement::Foreach {
            iterators, body, ..
        } => {
            for iterator in iterators {
                apply_value_scan(&iterator.list_arg, state);
            }
            walk(&body.statements, state, nested_depth, registry);
        }
        Statement::Catch { body, .. } => {
            walk(&body.statements, state, nested_depth, registry);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk(&body.statements, state, nested_depth, registry);
            for handler in handlers {
                walk(&handler.body.statements, state, nested_depth, registry);
            }
            if let Some(finally_body) = finally_body {
                walk(&finally_body.statements, state, nested_depth, registry);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for arm in arms {
                if let Some(body) = &arm.body {
                    walk(&body.statements, state, nested_depth, registry);
                }
            }
            if let Some(default_body) = default_body {
                walk(&default_body.statements, state, nested_depth, registry);
            }
        }
        Statement::Block { body, tokens, .. } => {
            if is_eval_block(tokens.as_ref(), registry) {
                let args = synthesise_eval_args(tokens.as_ref());
                state.record_fallback();
                handle_eval(&args, state, registry);
            } else {
                walk(&body.statements, state, nested_depth, registry);
            }
        }
        _ => {}
    }
}

/// Run the intra-procedural escape analysis over *body* and
/// return the resulting [`ProcEscapeSummary`].
///
/// The returned summary is *intra-procedural* — callee-induced
/// escapes haven't been folded in yet. Run the interprocedural
/// pass to produce the final summary the codegen should
/// consume.
#[must_use]
pub fn analyse_script<I: IntoIterator<Item = String>>(
    body: &Script,
    params: I,
) -> ProcEscapeSummary {
    analyse_script_with_registry(body, params, default_registry())
}

/// Registry-aware form of [`analyse_script`]. Production callers that already
/// selected a dialect/profile registry should use this entry point so command
/// availability and invocation facts match lowering exactly.
#[must_use]
pub fn analyse_script_with_registry<I: IntoIterator<Item = String>>(
    body: &Script,
    params: I,
    registry: &tcl_registry::CommandRegistry,
) -> ProcEscapeSummary {
    let known = collect_known_names(params, body);
    let mut state = EscapeState::new(known);
    walk(&body.statements, &mut state, 0, registry);
    let frame_needed =
        state.dynamic_barrier() || state.tags.values().any(|t| *t == EscapeTag::Frame);
    // Tentative pure_leaf — the interprocedural fixpoint can only
    // downgrade it (a proc with an opaque callee loses the flag). Pure
    // means: no escape, no eval/call fallback, no `upvar` source out, no
    // unbounded upvar source.
    let pure_leaf = !frame_needed
        && !state.flags.has_fallback()
        && !state.flags.has_call_fallback()
        && state.upvar_source_names.is_empty()
        && !state.flags.unbounded_upvar_source();
    ProcEscapeSummary {
        tags: state.tags,
        flags: state.flags,
        frame_needed,
        upvar_source_names: state.upvar_source_names,
        direct_callees: state.direct_callees,
        ssa_tags: std::collections::HashMap::new(),
        local_slots: std::collections::BTreeMap::new(),
        pure_leaf,
        // Thread the structured
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

    /// Regression coverage for issue #996: `walk` recurses once per
    /// nested `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`/`Block`
    /// body, with no depth cap of its own before this fix. Transitively
    /// bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering pass today,
    /// so this is defence-in-depth / consistency with every other
    /// full-tree walker in this crate, not a currently-reproducible
    /// crash. 1000 levels of source nesting is comfortably past this new
    /// cap; the assertion is that `analyse_script` returns at all, not
    /// what it returns. Spawns its own big-stack thread since the
    /// lexer/CST/segmenter stages upstream of the lowering cap still walk
    /// the full un-truncated source nesting before that cap trims it —
    /// same rationale as
    /// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
    #[test]
    fn deeply_nested_if_survives_escape_walk() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = String::new();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("set x 1\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let _ = analyse(&src);
            })
            .unwrap()
            .join()
            .unwrap();
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

    /// A name mentioned only inside a *brace-quoted* word of an `eval` body
    /// must still escape.
    ///
    /// `if`'s condition is brace-quoted, so Tcl's own word-splitting
    /// suppresses substitution at this level — but the expression is
    /// re-parsed and `$x` substituted when the inner `if` runs, so `x` is
    /// genuinely reachable from another frame. Believing it frame-local is
    /// unsound in the direction that matters: the optimiser may keep such a
    /// variable in a register.
    ///
    /// Today this holds because `is_dynamic_token` sends *any* body
    /// containing `$` or `[` down the pessimistic barrier path before the
    /// pre-scan runs — which is why the assertion below is on the barrier as
    /// well as the name. The pre-scan's own mode (`scan_word`, the
    /// over-approximating one) is the second line of defence should that
    /// guard ever be narrowed; the mode contract itself is pinned by
    /// `var_refs`' `scan_word_finds_a_var_inside_braces_within_a_value_body`.
    #[test]
    fn eval_body_escapes_a_name_inside_a_brace_quoted_word() {
        let s = analyse("eval {if {$x > 1} { set y 2 }}");
        assert!(
            s.is_frame("x"),
            "a brace-quoted expression inside an eval body still escapes its names",
        );
        assert!(
            s.dynamic_barrier(),
            "and the substitution-bearing body is what makes it pessimistic today",
        );
    }

    #[test]
    fn uplevel_zero_walks_body_like_eval() {
        // `uplevel 0` runs the body in the *current* frame, so
        // it must escape the same names `eval` does — not be treated as
        // global-safe like `#0`. The body's `set x` reaches our proc's local.
        let eval = analyse("eval {set x 2}");
        let up0 = analyse("uplevel 0 {set x 2}");
        assert_eq!(
            up0.is_frame("x"),
            eval.is_frame("x"),
            "uplevel 0 must escape the same names as eval",
        );
        assert!(
            up0.is_frame("x"),
            "uplevel 0 body must escape the current-frame local `x`",
        );
    }

    #[test]
    fn uplevel_global_does_not_escape_current_frame_local() {
        // TN for: `uplevel #0` runs at *global* scope — our
        // local `x` is not visible there, so a literal body's `set x` must NOT
        // mark our local frame-escaping.
        let up_global = analyse("uplevel #0 {set x 2}");
        assert!(
            !up_global.is_frame("x"),
            "uplevel #0 runs globally and must not escape our local `x`",
        );
    }

    // `barriers` and `tag_reasons`
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
            reasons
                .iter()
                .any(|r| r.kind == EscapeReasonKind::UpvarSource),
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
