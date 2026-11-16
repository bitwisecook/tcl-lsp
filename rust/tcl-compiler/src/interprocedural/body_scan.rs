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

//! The recursive scan of one procedure body: scripts, statements, control flow, `return` and up-frame scripts.

use super::call_facts::{
    mark_upvar_alias_write, scan_call_facts, scan_call_statement, scan_role_code_arguments,
};
use super::parameter_observations::{
    note_original_authored_alias_parameters, note_original_authored_body_call_parameters,
    note_original_command_references, note_original_expression_params,
    note_original_return_parameter, original_parameter_expression_config,
    original_parameter_observation_owner,
};
use super::positioned_invocations::{
    scan_normal_visible_reads, scan_normal_visible_writes, scan_original_variable_read_effects,
    scan_positioned_invocation, scan_retained_substitutions,
};
use super::return_summaries::{classify_return, script_always_returns, walk_collect_param_refs};
use super::substitution_calls::{scan_expr_for_calls, scan_value_substitutions};
use super::{LocalFacts, MAX_INTERPROCEDURAL_WALK_DEPTH, ProcScan, ReturnKind, ScanCtx};
use crate::ir::ProcedureId;
use crate::side_effects::EffectRegion;
use std::collections::HashSet;

pub(super) fn scan_proc(scan: ProcScan<'_>) -> LocalFacts {
    let parameter_body = original_parameter_observation_owner(scan);
    let parameter_owner = parameter_body.is_some();
    let declaration_source_unclosed = parameter_body
        .and_then(|id| {
            scan.source_module
                .original_authored_declaration_body_units
                .get(&id)
        })
        .is_some_and(|card| card.has_unclosed_declaration_source_coverage());
    let ProcScan {
        proc,
        unit: _,
        known,
        registry,
        dialect,
        object_types,
        identities,
        declared,
        source_input,
        source_procedures,
        source_module,
    } = scan;
    let mut facts = LocalFacts {
        local_pure: true,
        impure_beyond_links: false,
        param_observations_incomplete: !parameter_owner || declaration_source_unclosed,
        ..LocalFacts::default()
    };
    let params: HashSet<String> = proc
        .params
        .iter()
        .filter(|_| parameter_owner)
        .cloned()
        .collect();
    let ctx = ScanCtx {
        known,
        registry,
        dialect,
        params: &params,
        object_types,
        identities,
        declared,
        positioned_substitutions: false,
        source_input,
        source_procedures,
        source_module,
        current_tokens: None,
        parameter_body,
    };
    scan_script(&proc.body, ctx, &mut facts, 0);
    // If the body can fall off the end, its implicit exit returns the
    // result of the last command — not a constant. Record a non-constant
    // exit so `summarise_returns` won't fold a conditional `return CONST`
    // to a constant when a fall-through path returns something else
    // (O103).
    if !script_always_returns(&proc.body, 0) {
        facts.returns.push(ReturnKind::Other);
    }
    facts.name_params = proc
        .params
        .iter()
        .enumerate()
        .filter(|(_, param)| facts.upvar_aliases.values().any(|named| named == *param))
        .map(|(index, _)| index)
        .collect();
    facts
}

pub(super) fn possible_body_id(word: &str, ctx: ScanCtx<'_>) -> Option<ProcedureId> {
    let ids = crate::registry_invocation::original_source_procedure_ids_for_word_for_module(
        ctx.current_tokens?,
        word,
        ctx.source_module,
        ctx.registry,
    )?;
    let [id] = ids.as_slice() else {
        return None;
    };
    ctx.known.contains(id).then_some(*id)
}

/// `depth` is the nesting level of `script` — see
/// [`MAX_INTERPROCEDURAL_WALK_DEPTH`]. Past the cap, `script`'s statements
/// are not scanned; `facts` is instead marked exactly as conservatively as
/// [`scan_statement`]'s own `Statement::Barrier` arm (unscanned code must
/// not be silently under-counted as pure / effect-free).
pub(super) fn scan_script(
    script: &crate::ir::Script,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
    depth: u32,
) {
    // Different scripts/branches cannot donate conditional alias continuation.
    facts.source_parameter_aliases.clear();
    if MAX_INTERPROCEDURAL_WALK_DEPTH.exceeded(depth) {
        facts.param_observations_incomplete = true;
        facts.has_barrier = true;
        facts.mark_impure();
        facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
        facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
        return;
    }
    for stmt in &script.statements {
        scan_statement(
            stmt,
            script.retained_source_tokens_for_statement(stmt),
            ctx,
            facts,
            depth,
        );
    }
    facts.source_parameter_aliases.clear();
}

fn scan_return_statement(
    stmt: &crate::ir::Statement,
    retained_tokens: Option<&crate::ir::CommandTokens>,
    ctx: ScanCtx<'_>,
    params: &HashSet<String>,
    facts: &mut LocalFacts,
) {
    let crate::ir::Statement::Return {
        value,
        value_word,
        expr,
        expr_base,
        braced,
        ..
    } = stmt
    else {
        return;
    };
    note_original_return_parameter(value_word.as_ref(), ctx, facts);
    let config = retained_tokens.and_then(|tokens| {
        let binding = tokens.source_binding.as_ref()?;
        binding.original_invocation_metadata_for_module(tokens, ctx.source_module, ctx.registry)?;
        let config = binding.original_lexer_config_for_tokens(tokens)?;
        if let Some(expression) = expr {
            let (original_config, _) =
                crate::registry_invocation::original_lowered_expression_syntax(
                    expression, *expr_base, tokens,
                )?;
            (original_config == config).then_some(())?;
        }
        Some(config)
    });
    if let Some(expression) = expr {
        let mut references = Vec::new();
        let source_config = original_parameter_expression_config(expression, *expr_base, ctx);
        if !walk_collect_param_refs(expression, params, &mut references, source_config, 0) {
            facts.param_observations_incomplete = true;
        }
        facts.param_references.extend(references);
    }
    let kind = classify_return(
        value.as_deref(),
        *braced,
        value_word.as_ref(),
        expr.as_ref(),
        params,
        config,
    );
    facts.returns.push(kind);
    // For a return, scan `[cmd …]` substitutions in the return
    // value (`return [add $x $x]`) for call-graph edges.
    if let Some(value) = value
        && value.contains('[')
    {
        scan_value_substitutions(value, ctx, facts, 0);
    }
}

fn scan_upframe_script(
    body: &crate::ir::Script,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
    depth: u32,
) {
    facts.param_observations_incomplete = true;
    // Static-body uplevel runs the inner script in the
    // caller's frame — for interprocedural purposes it can
    // touch any caller-scope variable, so treat it as a
    // barrier conservatively. Any reads/writes inside
    // ``body`` propagate up.
    facts.has_barrier = true;
    facts.mark_impure();
    facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
    facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
    let outer_parameters = HashSet::new();
    scan_script(
        body,
        ScanCtx {
            params: &outer_parameters,
            parameter_body: None,
            ..ctx
        },
        facts,
        depth + 1,
    );
}

/// `depth` is `stmt`'s own nesting level — see
/// [`MAX_INTERPROCEDURAL_WALK_DEPTH`]. Dispatch-only (no nested `Script`
/// entered here) keeps the same `depth`; recursing into a nested body
/// (`UpFrame`/`Block`'s inner statements, or any control-flow body via
/// [`scan_control_flow_statement`]) passes `depth + 1`.
pub(super) fn scan_statement(
    stmt: &crate::ir::Statement,
    retained_tokens: Option<&crate::ir::CommandTokens>,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
    depth: u32,
) {
    use crate::ir::Statement;
    if !stmt.is_executable_invocation() {
        return;
    }
    let ScanCtx { params, .. } = ctx;
    let ctx = ScanCtx {
        current_tokens: retained_tokens,
        ..ctx
    };
    note_original_command_references(ctx, facts);
    note_original_authored_alias_parameters(ctx, facts);
    note_original_authored_body_call_parameters(ctx, facts);
    let ctx = scan_retained_substitutions(retained_tokens, ctx, facts);
    if let Some(tokens) = retained_tokens {
        scan_original_variable_read_effects(tokens, ctx, facts);
        scan_normal_visible_writes(tokens, ctx, facts);
        scan_normal_visible_reads(tokens, ctx, facts);
    }
    if scan_positioned_invocation(stmt, ctx, facts) {
        return;
    }
    match stmt {
        Statement::NativeCall { .. } => {
            facts.param_observations_incomplete = true;
            facts.has_barrier = true;
            facts.local_pure = false;
            facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
            facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
        }

        Statement::Barrier {
            command,
            args,
            tokens,
            ..
        } => {
            // A barrier makes effects opaque; it does not erase the call's
            // registry-declared identities.  In particular, same-invocation
            // command prefixes such as `lsort -command cb` still make `cb`
            // reachable, while deferred registrations that conservatively
            // lower to a barrier retain their callback edge too — and neither
            // does it erase the script its `ArgRole::Body` words carry.
            scan_call_facts(command, args, ctx, facts);
            scan_role_code_arguments(command, args, tokens.as_ref(), ctx, facts);
            facts.has_barrier = true;
            facts.mark_impure();
            facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
            facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
        }
        Statement::UpFrame { body, .. } => {
            scan_upframe_script(body, ctx, facts, depth);
        }
        Statement::Block { body, .. } => {
            // ``Block`` is a transparent splice: walk through to the
            // inner statements without flagging a barrier.
            scan_script(body, ctx, facts, depth + 1);
        }
        Statement::AssignConst { name, .. } => {
            note_assign_global_write(name, facts);
        }
        Statement::AssignExpr {
            name,
            expr,
            expr_base,
            ..
        } => {
            note_assign_global_write(name, facts);
            note_original_expression_params(expr, *expr_base, ctx, facts);
        }
        Statement::AssignValue { name, value, .. } => {
            note_assign_global_write(name, facts);
            // For an assigned value, a `[cmd …]` substitution in that
            // value is a call site (`set y [double $x]`), so its callees
            // become call-graph edges.
            if value.contains('[') {
                scan_value_substitutions(value, ctx, facts, 0);
            }
        }
        Statement::Incr { name, amount, .. } => {
            note_assign_global_write(name, facts);
            if let Some(amount) = amount
                && amount.contains('[')
            {
                scan_value_substitutions(amount, ctx, facts, 0);
            }
        }
        Statement::Return { .. } => {
            scan_return_statement(stmt, retained_tokens, ctx, params, facts);
        }
        Statement::Call {
            command,
            args,
            defs,
            tokens,
            ..
        } => {
            scan_call_statement(command, args, defs, tokens.as_ref(), ctx, facts);
        }
        Statement::If { .. }
        | Statement::For { .. }
        | Statement::While { .. }
        | Statement::ExprEval { .. }
        | Statement::Foreach { .. }
        | Statement::Catch { .. }
        | Statement::Try { .. }
        | Statement::Switch { .. } => {
            scan_control_flow_statement(stmt, ctx, facts, depth);
        }
    }
}

/// Recurse into the bodies / conditions of a control-flow statement (`if` /
/// `for` / `while` / `expr` / `foreach` / `catch` / `try` / `switch`),
/// recording param-touching conditions and embedded call edges. Extracted from
/// [`scan_statement`]. `depth` is `stmt`'s own nesting level — see
/// [`MAX_INTERPROCEDURAL_WALK_DEPTH`]; every nested body passed to
/// [`scan_script`] descends one level (`depth + 1`).
pub(super) fn scan_control_flow_statement(
    stmt: &crate::ir::Statement,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
    depth: u32,
) {
    use crate::ir::Statement;
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                note_original_expression_params(&c.condition, c.condition_base, ctx, facts);
                scan_expr_for_calls(&c.condition, ctx, facts, 0);
                scan_script(&c.body, ctx, facts, depth + 1);
            }
            if let Some(body) = else_body {
                scan_script(body, ctx, facts, depth + 1);
            }
        }
        Statement::For {
            init,
            condition,
            condition_base,
            next,
            body,
            ..
        } => {
            note_original_expression_params(condition, *condition_base, ctx, facts);
            scan_expr_for_calls(condition, ctx, facts, 0);
            scan_script(init, ctx, facts, depth + 1);
            scan_script(next, ctx, facts, depth + 1);
            scan_script(body, ctx, facts, depth + 1);
        }
        Statement::While {
            condition,
            condition_base,
            body,
            ..
        } => {
            note_original_expression_params(condition, *condition_base, ctx, facts);
            scan_expr_for_calls(condition, ctx, facts, 0);
            scan_script(body, ctx, facts, depth + 1);
        }
        Statement::ExprEval {
            expr, expr_base, ..
        } => {
            note_original_expression_params(expr, *expr_base, ctx, facts);
            scan_expr_for_calls(expr, ctx, facts, 0);
        }
        Statement::Foreach { body, .. } | Statement::Catch { body, .. } => {
            scan_script(body, ctx, facts, depth + 1);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            scan_script(body, ctx, facts, depth + 1);
            for h in handlers {
                scan_script(&h.body, ctx, facts, depth + 1);
            }
            if let Some(fb) = finally_body {
                scan_script(fb, ctx, facts, depth + 1);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    scan_script(b, ctx, facts, depth + 1);
                }
            }
            if let Some(db) = default_body {
                scan_script(db, ctx, facts, depth + 1);
            }
        }
        _ => {}
    }
}

fn is_global_or_namespace(name: &str) -> bool {
    tcl_syntax::naming::is_qualified(name.as_bytes())
}

/// Segment a raw Tcl source string into top-level commands and
/// record call-graph edges for each one.  Recurses into
/// `ArgRole::Body`-role arguments so `if {[catch {p}]} ...` also
/// sees `p` as a call.
/// Record the caller-visible-state effect of an assignment to `name`.
/// A write to a `::`-qualified name OR a name aliased into global /
/// namespace scope earlier in the body (`global g; set g 5`) mutates
/// caller-visible state. The write is recorded via
/// `writes_global` ONLY — it does not enter the coarse [`EffectRegion`]
/// set (which tracks HTTP / response-lifecycle / unknown state, not plain
/// variable storage), so the `callgraph`/`dataflow` effect string stays
/// `NONE` for a purely global-mutating proc. Also upgrades a level-1
/// upvar alias write to `VarWrite`.
fn note_assign_global_write(name: &str, facts: &mut LocalFacts) {
    if is_global_or_namespace(name) || facts.global_aliases.contains(name) {
        facts.writes_global = true;
        facts.mark_impure();
    }
    mark_upvar_alias_write(name, facts);
}
