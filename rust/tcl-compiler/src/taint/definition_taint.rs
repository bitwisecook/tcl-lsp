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

//! The taint a statement gives the variables it defines: call results, written arguments and procedure summaries.

use super::registry_colours::{instance_taints_var_write, source_colour, unique_instance_class};
use super::substitution_taint::{expr_command_taint, expr_yields_boolean};
use super::word_taint::{word_taint, word_taint_at};
use super::{TaintColour, TaintCtx, TaintLattice, dialect_to_point, is_sanitiser};
use crate::cfg::BlockId;
use crate::ir::{CommandTokens, Statement};
use crate::rendered_properties::{RenderedProperties, RenderedValueProps};
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use std::collections::HashMap;

/// Classify one runtime argument without re-substituting alias prefix values.
pub(super) fn positional_argument_taint<S: std::hash::BuildHasher>(
    index: usize,
    args: &[String],
    effective: Option<&crate::registry_invocation::EffectiveCommandWords>,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    depth: u32,
) -> TaintLattice {
    let written = match effective {
        Some(words) => {
            let Some(written) = words.written_argument(index) else {
                return TaintLattice::clean();
            };
            written
        }
        None => index,
    };
    let Some(argument) = args.get(index) else {
        return TaintLattice::tainted();
    };
    let arg_ctx = TaintCtx {
        word_offset: ctx
            .source_tokens
            .and_then(|tokens| tokens.argv.get(written + 1))
            .map(|span| span.start()),
        source_word: ctx
            .source_tokens
            .and_then(|tokens| tokens.word_exprs.get(written + 1)),
        ..ctx
    };
    word_taint_at(argument, uses, taints, arg_ctx, depth)
}

/// When `command` resolves to an internal proc with a known
/// `return_passthrough_param`, return the taint of the corresponding
/// actual argument. Returns `None` when interprocedural summaries are
/// not available or the call doesn't resolve.
pub(super) fn interproc_call_taint<S: std::hash::BuildHasher>(
    command: &str,
    args: &[String],
    effective: Option<&crate::registry_invocation::EffectiveCommandWords>,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    depth: u32,
) -> Option<TaintLattice> {
    let known = ctx.known_procs?;
    let ids = ctx
        .source
        .procedure_ids(ctx.source_tokens?, command, ctx.registry)?;
    let [target] = ids.as_slice() else {
        return None;
    };
    if !known.contains(target) {
        return None;
    }

    // Colour-aware return-summary path: when the interprocedural taint
    // solve has computed per-proc summaries, apply the full transfer.
    // A resolved callee
    // always yields a result (untainted when the arity rejects the call),
    // so the bare argument join below is bypassed for internal calls.
    if let Some(summaries) = ctx.taint_summaries {
        let summary = summaries.get(target)?;
        let arg_taints: Vec<TaintLattice> = (0..args.len())
            .map(|index| {
                positional_argument_taint(index, args, effective, uses, taints, ctx, depth)
            })
            .collect();
        return Some(crate::taint_interproc::apply_proc_return_summary(
            summary,
            &arg_taints,
        ));
    }

    // Conservative single-passthrough rule (no solve summaries available).
    let interproc = ctx.interproc?;
    let summary = interproc.procedures.get(target)?;
    let passthrough = summary.return_passthrough_param.as_ref()?;
    let idx = summary.params.iter().position(|p| p == passthrough)?;
    args.get(idx)?;
    Some(positional_argument_taint(
        idx, args, effective, uses, taints, ctx, 0,
    ))
}

/// Whether this call's callee exposes `defined_name` to external input via a
/// registry-typed instance configuration, including transitive callees.
fn interproc_call_taints_global(command: &str, defined_name: &str, ctx: TaintCtx<'_>) -> bool {
    let (Some(known), Some(interproc)) = (ctx.known_procs, ctx.interproc) else {
        return false;
    };
    ctx.source_tokens
        .and_then(|tokens| ctx.source.procedure_ids(tokens, command, ctx.registry))
        .is_some_and(|ids| {
            ids.iter().filter(|id| known.contains(id)).any(|id| {
                interproc
                    .tainted_global_writes
                    .get(id)
                    .is_some_and(|writes| writes.contains(defined_name))
            })
        })
}

/// Look up taint for a named variable at its current SSA version. A name
/// outside the represented SSA arena has unknown contents and no mitigation.
/// This residual does not establish a read, a store or variable presence.
pub(super) fn var_taint<S: std::hash::BuildHasher>(
    name: &str,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ssa: &SsaFunction,
) -> TaintLattice {
    let Some(sym) = ssa.var_symbol(name) else {
        return TaintLattice::tainted();
    };
    // Only a retained use can select its contents version. An interned
    // spelling elsewhere in the function supplies no value at this read.
    let Some(ver) = uses.get(&sym).copied() else {
        return TaintLattice::tainted();
    };
    taints
        .get(&(sym, ver))
        .copied()
        .unwrap_or(TaintLattice::clean())
}

/// Determine the taint produced by a statement's definition(s).
#[cfg(test)]
pub(super) fn evaluate_taint_def<S: std::hash::BuildHasher>(
    stmt: &Statement,
    defined_var: Symbol,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    ssa: &SsaFunction,
) -> TaintLattice {
    evaluate_taint_def_at(stmt, defined_var, uses, taints, ctx, ssa, None)
}

pub(super) fn evaluate_taint_def_at<S: std::hash::BuildHasher>(
    stmt: &Statement,
    defined_var: Symbol,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    ssa: &SsaFunction,
    point: Option<(BlockId, usize)>,
) -> TaintLattice {
    let ctx = TaintCtx {
        source_tokens: stmt.tokens(),
        word_offset: None,
        source_word: None,
        ..ctx.at(stmt.span().start())
    };
    match stmt {
        Statement::NativeCall { .. } => TaintLattice::tainted(),
        // Expression: join taint from all used variables AND from any command
        // substitution embedded in the expression AST. `join_uses` only sees
        // `$var` SSA uses, so a taint source nested in an `[expr {…}]` command
        // substitution (`set x [expr {[gets stdin] + 1}]`) would otherwise
        // launder the taint — a false-negative security diagnostic.
        // A comparison or logical operator yields only `0`/`1`, whatever
        // its operands held, so the result carries no attacker text.
        Statement::AssignExpr { expr, .. } if expr_yields_boolean(expr) => TaintLattice::clean(),
        Statement::AssignExpr {
            expr, expr_base, ..
        } => {
            let ctx = TaintCtx {
                source_tokens: point.and_then(|(block, index)| {
                    ssa.point_contexts.as_ref()?.source_tokens_at(block, index)
                }),
                ..ctx
            };
            join_uses(uses, taints).join(expr_command_taint(expr, uses, taints, ctx, *expr_base, 0))
        }

        // Value assignment: evaluate the RHS word.
        Statement::AssignValue { value, tokens, .. } => {
            let ctx = TaintCtx {
                word_offset: tokens
                    .as_ref()
                    .and_then(|tokens| tokens.argv.last())
                    .map(|span| span.start()),
                source_word: tokens.as_ref().and_then(|tokens| tokens.word_exprs.last()),
                ..ctx
            };
            word_taint(value, uses, taints, ctx)
        }

        // incr propagates taint from the variable being incremented.
        Statement::Incr { .. } => var_taint(ssa.var_name(defined_var), uses, taints, ssa),

        // This query is made for actual SSA definitions. Successful-handler
        // projections can supply them without adding raw IR `Call::defs`.
        Statement::Call { .. } => {
            call_definition_taint(stmt, defined_var, uses, taints, ctx, ssa, point)
        }

        // Barrier has unknown semantics — propagate taint conservatively
        // from its arguments so attacker-influenced data flowing through
        // an opaque command taints any output variables it defines.
        Statement::Barrier { args, .. } => {
            let mut t = TaintLattice::clean();
            for arg in args {
                t = t.join(word_taint(arg, uses, taints, ctx));
            }
            t
        }

        // `AssignConst` and any unhandled statement variant (e.g.
        // structured control flow that survives in some code paths) are
        // treated as clean: constants cannot introduce taint, and
        // opaque control-flow statements have no value definition to
        // propagate taint through.
        _ => TaintLattice::clean(),
    }
}

fn written_arguments_taint<S: std::hash::BuildHasher>(
    args: &[String],
    tokens: Option<&CommandTokens>,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
) -> TaintLattice {
    args.iter()
        .enumerate()
        .fold(TaintLattice::clean(), |taint, (index, arg)| {
            let arg_ctx = TaintCtx {
                word_offset: tokens
                    .and_then(|tokens| tokens.argv.get(index + 1))
                    .map(|span| span.start()),
                source_word: tokens.and_then(|tokens| tokens.word_exprs.get(index + 1)),
                ..ctx
            };
            taint.join(word_taint(arg, uses, taints, arg_ctx))
        })
}

fn selected_source_write_targets_definition(
    tokens: &CommandTokens,
    defined_var: Symbol,
    ssa: &SsaFunction,
    ctx: TaintCtx<'_>,
    point: Option<(BlockId, usize)>,
) -> Option<bool> {
    let metadata = ctx.source.metadata_context()?;
    let (block, index) = point?;
    let receiver = tokens.words().first().map(|word| (word, ctx.source.config));
    let instance_class = unique_instance_class(receiver, ctx.instance_classes, ctx.source_position);
    // Select exactly one genuine source owner. A registered instance receiver
    // uses its class schema; otherwise the selected registry command schema is
    // required. An unavailable selected owner cannot borrow an empty result
    // from the other dispatch path to claim that this definition is unwritten.
    let writes = if instance_class.is_some() {
        crate::ssa::original_instance_option_writes(tokens, metadata, ctx.registry)?
    } else {
        crate::ssa::original_command_taint_option_writes(tokens, metadata, ctx.registry)?
    };
    if writes.is_empty() {
        return Some(false);
    }
    let target_span = crate::ssa::SsaSourceView::at_statement(ssa, block, index)
        .original_definition_name_with_metadata_context(defined_var, ctx.registry, Some(metadata))?
        .span();
    let selects_target = |write: &crate::ssa::OriginalLinkedOptionWrite| {
        write
            .written_argument
            .checked_add(1)
            .and_then(|index| tokens.words().get(index))
            .is_some_and(|word| word.source().span == target_span)
    };
    Some(writes.iter().any(selects_target))
}

fn call_arguments_taint<S: std::hash::BuildHasher>(
    args: &[String],
    effective: Option<&crate::registry_invocation::EffectiveCommandWords>,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
) -> TaintLattice {
    let mut t = TaintLattice::clean();
    for index in 0..args.len() {
        t = t.join(positional_argument_taint(
            index, args, effective, uses, taints, ctx, 0,
        ));
    }
    t
}

fn call_source_colour(ctx: TaintCtx<'_>, command: &str, arg_refs: &[&str]) -> Option<TaintLattice> {
    source_colour(
        ctx.registry,
        command,
        arg_refs,
        ctx.dialect,
        ctx.instance_classes,
        ctx.source_position,
        ctx.source_tokens
            .and_then(|tokens| tokens.words().first())
            .map(|word| (word, ctx.source.config)),
    )
}

fn call_definition_taint<S: std::hash::BuildHasher>(
    stmt: &Statement,
    defined_var: Symbol,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    ssa: &SsaFunction,
    point: Option<(BlockId, usize)>,
) -> TaintLattice {
    let Statement::Call {
        command,
        args,
        tokens,
        ..
    } = stmt
    else {
        return TaintLattice::clean();
    };
    let tokens = tokens.as_ref();
    if !ctx.source.allows_nominal_metadata() && ctx.source.metadata.is_none() {
        return TaintLattice::tainted();
    }

    if tokens.is_some_and(CommandTokens::has_unproved_source_binding) {
        // A same-spelled sanitizer/source is not implementation proof.
        // Preserve ordinary argument taint without granting mitigation.
        return written_arguments_taint(args, tokens, uses, taints, ctx);
    }
    let resolved = crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
        ctx.registry,
        ctx.source.metadata,
        stmt,
    );
    if (tokens.is_some_and(|tokens| tokens.source_binding.is_some())
        || !ctx.source.allows_nominal_metadata())
        && resolved.is_none()
    {
        return written_arguments_taint(args, tokens, uses, taints, ctx);
    }
    let command = resolved.as_ref().map_or(command.as_str(), |resolved| {
        resolved.facts.canonical_command.as_str()
    });
    let presented;
    let args = if let Some(resolved) = &resolved {
        let Some(arguments) = resolved
            .arguments
            .iter()
            .cloned()
            .collect::<Option<Vec<_>>>()
        else {
            // An unmaterialised captured value has no source spelling or
            // established taint provenance. It cannot license mitigation.
            return TaintLattice::tainted();
        };
        presented = arguments;
        presented.as_slice()
    } else {
        args.as_slice()
    };
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let defined_name = ssa.var_name(defined_var);
    let writes_tainted_target = if ctx.source.allows_nominal_metadata() {
        tcl_registry::taint::taints_var_write(
            ctx.registry,
            command,
            &arg_refs,
            dialect_to_point(ctx.dialect),
            defined_name,
        ) || instance_taints_var_write(ctx, command, &arg_refs, defined_name)
    } else {
        let Some(tokens) = tokens else {
            return TaintLattice::tainted();
        };
        match selected_source_write_targets_definition(tokens, defined_var, ssa, ctx, point) {
            Some(selected) => selected,
            None => return TaintLattice::tainted(),
        }
    };
    if writes_tainted_target {
        return TaintLattice::tainted();
    }
    if interproc_call_taints_global(command, defined_name, ctx) {
        return TaintLattice::tainted();
    }
    if is_sanitiser(ctx.registry, command, &arg_refs) {
        return TaintLattice::clean();
    }
    if let Some(t) = call_source_colour(ctx, command, &arg_refs) {
        return t;
    }
    if let Some(t) = interproc_call_taint(
        command,
        args,
        resolved.as_ref().map(|resolved| &resolved.effective),
        uses,
        taints,
        ctx,
        0,
    ) {
        return t;
    }
    // Binding prefixes are literal values, never a fresh source eval.
    call_arguments_taint(
        args,
        resolved.as_ref().map(|resolved| &resolved.effective),
        uses,
        taints,
        ctx,
    )
}

/// Apply rendered-property-derived colours to a taint lattice.
///
/// Only adds mitigating colours when the rendered-properties lattice
/// provides *positive* evidence for them. A leading `/` (must-bit
/// `STARTS_WITH_SLASH`) proves both `PATH_PREFIXED` and
/// `NON_DASH_PREFIXED`. We deliberately do *not* infer
/// `NON_DASH_PREFIXED` or `CRLF_FREE` from the absence of bits,
/// because phi-joins and unknown command substitutions lose those
/// facts without proving the value is safe — inferring from absence
/// would unsoundly suppress T102 / CRLF-injection warnings.
pub(super) fn colour_from_rendered(lat: TaintLattice, props: RenderedValueProps) -> TaintLattice {
    let mut out = lat;
    if props.must.contains(RenderedProperties::STARTS_WITH_SLASH) {
        out = out.with(TaintColour::PATH_PREFIXED);
        out = out.with(TaintColour::NON_DASH_PREFIXED);
    }
    out
}

/// Join taint from all SSA uses in a statement.
///
/// Version-zero uses contribute according to their typed seed provenance.
/// Conservative global-write seeds do not flow as genuine inputs.
pub(super) fn join_uses<S: std::hash::BuildHasher>(
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
) -> TaintLattice {
    let mut t = TaintLattice::clean();
    for (&sym, &ver) in uses {
        if ver > 0 {
            t = t.join(
                taints
                    .get(&(sym, ver))
                    .copied()
                    .unwrap_or(TaintLattice::clean()),
            );
        } else if let Some(&v0) = taints.get(&(sym, 0))
            && !v0.is_conservative_global_seed_only()
        {
            t = t.join(v0);
        }
    }
    t
}
