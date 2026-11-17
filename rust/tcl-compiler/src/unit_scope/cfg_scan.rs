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

//! The walk over each caller's CFG: statements, lifted substitutions, command surfaces and terminators.

use super::call_recording::{
    record_call_site_evidence, record_unenumerable_targets, unseen_code_is_scanned_host,
};
use super::{CallSiteEvidence, CallSiteScanCtx, CallerFrame, CallerScopeId};
use crate::cfg::Function as CfgFunction;
use crate::ir::Statement;

/// A marker that widens the caller frame for, or states the substitution
/// effects of, the host statement beside it, which performs (and is scanned
/// for) the invocation.
fn accompanies_a_scanned_host(statement: &Statement) -> bool {
    statement.tokens().is_some_and(|tokens| {
        matches!(
            tokens.synthetic,
            Some(
                crate::ir::SyntheticMarker::CallerFrameOpaque
                    | crate::ir::SyntheticMarker::WordEffects(_)
            )
        )
    })
}

/// Walk each `(caller qname, CFG)` pair's flattened statements, recording
/// every `Call`/`Barrier` as a call site.  Shared by the in-unit scan and
/// [`scan_source_call_sites`] so the two can never diverge on what counts.
pub(super) fn scan_cfg_callers<'a>(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    funcs: impl Iterator<Item = (CallerScopeId, &'a CfgFunction)>,
) {
    // The unresolved-command handler's caller set is *never* enumerable, so
    // it is poisoned before a single statement is read.
    //
    // Tcl routes to it every command word that resolves to nothing at the
    // moment of the call — a name typed at a prompt, a name a package
    // autoloads, a name another file introduces, a name produced by string
    // arithmetic. `record_unresolved_word_dispatch` can name *some* of those
    // words, but naming some of a set is not enumerating it: treating those
    // as the complete caller set let a coincidentally-uniform handful seed
    // the handler's parameters and fold its body against values real Tcl
    // never passes. The concrete dispatches stay, purely as extra retracting
    // evidence.
    if let Some(handler) = ctx.unresolved_handler {
        out.record_opaque_caller(handler);
    }
    for (scope, func) in funcs {
        // The typed caller coordinate owns variable facts. A retained procedure is
        // one this evidence map attributes calls *to*; a method or `apply`
        // lambda is not, so its parameters may hold values never seen here.
        let declared = match scope {
            CallerScopeId::Procedure(id) => ctx.procedures.get(&id),
            _ => None,
        };
        let caller = CallerFrame {
            metadata_owner: &func.metadata_context,
            scope,
            params: declared.map_or(&[][..], |p| p.params.as_slice()),
            callers_tracked: declared.is_some() || scope == CallerScopeId::TopLevel,
        };
        let standalone = ctx.source_input.is_none()
            && ctx.module.source_entry.metadata_context.is_standalone()
            && func.metadata_context.is_standalone();
        let config = ctx.module.lexer_config;
        for (&block_id, block) in &func.blocks {
            for (index, stmt) in block.statements.iter().enumerate() {
                if stmt
                    .tokens()
                    .is_some_and(|tokens| !tokens.evaluates_words())
                {
                    continue;
                }
                if accompanies_a_scanned_host(stmt) {
                    continue;
                }
                if crate::ssa::is_unseen_call_marker(stmt)
                    && unseen_code_is_scanned_host(func, block_id, &block.statements, index)
                {
                    continue;
                }
                let tokens = func.source_tokens_at(block_id, index);
                if let Statement::Call { command, args, .. }
                | Statement::Barrier { command, args, .. } = stmt
                {
                    record_call_site_evidence(out, ctx, &caller, command, args, tokens, 0);
                }
                if standalone {
                    // Explicit compatibility uses local scalar syntax only.
                    for lifted in
                        crate::word_subst::lifted_calls_with_surface(tokens, config, &ctx.surface())
                    {
                        record_call_site_evidence(
                            out,
                            ctx,
                            &caller,
                            &lifted.command,
                            &lifted.args,
                            lifted.tokens.as_ref(),
                            0,
                        );
                    }
                    record_surface_call_sites(out, ctx, &caller, config, stmt);
                } else {
                    record_original_lifted_call_sites(out, ctx, &caller, tokens);
                }
            }
            let tokens = func.source_tokens_at(block_id, usize::MAX);
            if standalone {
                record_terminator_call_sites(
                    out,
                    ctx,
                    &caller,
                    config,
                    block.terminator.as_ref(),
                    tokens,
                );
            } else if matches!(
                block.terminator,
                Some(crate::cfg::Terminator::Return {
                    tokens: None,
                    value: None,
                    value_word: None,
                    span: None,
                    expr: None,
                    ..
                })
            ) || (matches!(
                block.terminator,
                Some(crate::cfg::Terminator::Branch { .. })
            ) && block.statements.iter().any(|statement| {
                statement.tokens().is_some_and(|tokens| {
                    matches!(
                        tokens.synthetic,
                        Some(crate::ir::SyntheticMarker::IterationBindings(_))
                    )
                })
            })) {
                // An implicit exit and typed frozen iterator boundary do not
                // evaluate another original command or substitution.
            } else if matches!(
                block.terminator,
                Some(crate::cfg::Terminator::Return { .. } | crate::cfg::Terminator::Branch { .. })
            ) {
                // The CFG owner is the sole carrier door, including consumed
                // expression conditions and returns. A failed consensus does
                // not reopen inline tokens or parsed AST text.
                record_original_lifted_call_sites(out, ctx, &caller, tokens);
            }
        }
    }
}

/// Complete caller substitutions from the exact retained original vector.
/// Body and expression roles remain source-only selected metadata; missing
/// inventory retracts seeds rather than constructing a source offset.
pub(super) fn record_original_lifted_call_sites(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    tokens: Option<&crate::ir::CommandTokens>,
) {
    let inventory = (|| {
        let tokens = tokens?;
        if !crate::place_bridge::evaluates_arguments(tokens) {
            return Some(Vec::new());
        }
        if tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments) {
            let metadata = ctx
                .original_argument_metadata(tokens, caller.metadata_owner)
                .ok()??;
            let mut calls = crate::word_subst::checked_original_argument_lifted_calls(
                tokens,
                ctx.registry,
                metadata,
            )?;
            // The enclosing argv has no expression/body-role authority. Each
            // actual nested invocation independently retains its own selected
            // expression roles and complete source inventory.
            for call in calls.clone() {
                let child = call.tokens.as_ref()?;
                let metadata = ctx.original_metadata(child, caller.metadata_owner).ok()??;
                let config = child
                    .source_binding
                    .as_ref()?
                    .original_lexer_config_for_tokens(child)?;
                for nested in
                    crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                        child,
                        config,
                        ctx.registry,
                        metadata,
                    )?
                {
                    if let Some(previous) =
                        calls.iter().find(|previous| previous.span == nested.span)
                    {
                        if previous != &nested {
                            return None;
                        }
                    } else {
                        calls.push(nested);
                    }
                }
            }
            return Some(calls);
        }
        let metadata = ctx
            .original_metadata(tokens, caller.metadata_owner)
            .ok()??;
        let binding = tokens.source_binding.as_ref()?;
        let config = binding.original_lexer_config_for_tokens(tokens)?;
        let input = metadata.source_analysis_input()?;
        if binding
            .original_source_procedure_argument_roles(tokens, input, ctx.registry)
            .is_some()
        {
            // An authenticated original procedure call evaluates lexical
            // argument substitutions; braces cannot become expression roles.
            let mut calls = crate::word_subst::checked_lifted_calls(tokens, config)?;
            for call in &mut calls {
                let child = call.tokens.as_mut()?;
                child.inherit_nested_bindings(tokens);
                call.words = Some(child.clone());
            }
            Some(calls)
        } else {
            crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                tokens,
                config,
                ctx.registry,
                metadata,
            )
        }
    })();
    let Some(inventory) = inventory else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    for lifted in inventory {
        // record_call_site_evidence independently joins every original child
        // to the current Module and this CFG's tagged metadata owner.
        record_call_site_evidence(
            out,
            ctx,
            caller,
            &lifted.command,
            &lifted.args,
            lifted.tokens.as_ref(),
            0,
        );
    }
}

/// Record the call sites a statement runs through a surface its **words** do
/// not carry.
///
/// A fused `AssignExpr`, `ExprEval` or `Incr` keeps a parsed expression or an
/// amount string in place of a `CommandTokens`, so
/// [`crate::word_subst::lifted_calls_with_surface`] above sees nothing at all
/// in it. `set r [expr {[id 9]}]` and `incr t [id 9]` therefore contributed no
/// evidence, and a lone visible `id 7` read as `id`'s complete caller set:
/// O100 folded `return $v` to `return 7` and tclsh 8.6.18's `7 9` became
/// `7 7` (#2118).
///
/// The surfaces are the same ones
/// [`crate::ir_helpers::evaluated_command_substitution_surfaces`] names for
/// the variable-effect walk — the two disagreeing about which commands a
/// statement runs is the underlying defect.
fn record_surface_call_sites(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    config: tcl_lexer::LexerConfig,
    stmt: &Statement,
) {
    let lifted = match stmt {
        Statement::AssignExpr {
            expr, expr_base, ..
        }
        | Statement::ExprEval {
            expr, expr_base, ..
        } => crate::word_subst::lifted_calls_in_expr(expr, *expr_base, config, &ctx.surface()),
        // The amount has no retained word, so a braced `{[f]}` — which runs
        // nothing — is read as a surface too. Over-reporting a caller only
        // retracts evidence; under-reporting one invents agreement.
        Statement::Incr {
            amount: Some(amount),
            ..
        } => crate::word_subst::lifted_calls_in_text(amount, None, config, &ctx.surface()),
        _ => return,
    };
    for mut lifted in lifted {
        if let (Some(nested), Some(parent)) = (&mut lifted.tokens, stmt.tokens()) {
            nested.inherit_nested_bindings(parent);
        }
        record_call_site_evidence(
            out,
            ctx,
            caller,
            &lifted.command,
            &lifted.args,
            lifted.tokens.as_ref(),
            0,
        );
    }
}

/// Record the call sites a block's terminator runs.
///
/// A `return` value and an `if`/`while` condition are terminators, not
/// statements, so the statement loop never reached them:
/// `proc a {} { return [id 9] }` contributed no evidence at all (#2118).
fn record_terminator_call_sites(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    config: tcl_lexer::LexerConfig,
    terminator: Option<&crate::cfg::Terminator>,
    source_tokens: Option<&crate::ir::CommandTokens>,
) {
    let parent = match terminator {
        Some(crate::cfg::Terminator::Return { tokens, .. }) => tokens.as_deref().or(source_tokens),
        _ => source_tokens,
    };
    let lifted = match terminator {
        Some(crate::cfg::Terminator::Return {
            value_word, expr, ..
        }) => {
            let mut lifted = crate::word_subst::lifted_calls_in_word(
                value_word.as_ref(),
                config,
                &ctx.surface(),
            );
            if let Some(expr) = expr {
                lifted.extend(crate::word_subst::lifted_calls_in_expr(
                    expr,
                    None,
                    config,
                    &ctx.surface(),
                ));
            }
            lifted
        }
        Some(crate::cfg::Terminator::Branch {
            condition,
            condition_base,
            ..
        }) => crate::word_subst::lifted_calls_in_expr(
            condition,
            *condition_base,
            config,
            &ctx.surface(),
        ),
        Some(crate::cfg::Terminator::Goto { .. } | crate::cfg::Terminator::Complete { .. })
        | None => return,
    };
    for mut lifted in lifted {
        if let (Some(nested), Some(parent)) = (&mut lifted.tokens, parent) {
            nested.inherit_nested_bindings(parent);
        }
        record_call_site_evidence(
            out,
            ctx,
            caller,
            &lifted.command,
            &lifted.args,
            lifted.tokens.as_ref(),
            0,
        );
    }
}
