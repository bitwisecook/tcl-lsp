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

//! O127: forwarding a computed single-use assignment into its one use, and the checks on what runs between them.

use super::super::helpers::spans::{line_delete_span, original_statement_rewrite_span};
use super::super::{Optimisation, PassContext};
use super::load_forwarding::cell_is_linked_outside_frame;
use crate::analyses::LatticeValue;
use crate::compilation_unit::FunctionUnit;
use crate::ir::{CommandTokens, Statement};
use tcl_core_types::DiagCode;
use tcl_syntax::naming::whole_scalar_variable_reference_name;

/// Whether `stmt`, if it appears between a `O127` store-to-load
/// forwarding candidate's definition and use, must block the forward:
/// moving the definition's *computed* side-effecting value (a command
/// substitution) later, past `stmt`, could reorder its observable effects
/// relative to `stmt`'s own — a barrier (`eval`/`uplevel`/`interp eval`/…),
/// a frame-crossing `uplevel`/`upvar` body, a call the registry cannot
/// prove pure (including every call to a proc, since an unregistered name
/// classifies as an unknown write — see
/// `side_effects::fallback_unknown_write`), or an assignment whose value
/// itself runs a command substitution.
///
/// Used only by the `O127` store-to-load `intervening_is_safe` gate.
/// `O102`'s plain-variable load-forwarding ([`run_load_forwarding`]) does
/// *not* need this: it never moves a side-effecting computation, only a
/// literal value, and a plain proc-local variable cannot be rewritten by
/// an intervening call at all without an alias/trace/barrier — each of
/// which `run_load_forwarding` already checks directly (see its own doc
/// comment).
fn statement_may_have_untracked_effects(
    stmt: &Statement,
    registry: &tcl_registry::CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    traced: &std::collections::BTreeSet<String>,
    has_dynamic_trace: bool,
) -> bool {
    use super::super::helpers::expr_simplify::expr_has_command_subst;
    use crate::gvn::is_pure_tokens_with_metadata_context;

    match stmt {
        Statement::Barrier { .. } | Statement::NativeCall { .. } | Statement::UpFrame { .. } => {
            true
        }
        Statement::AssignValue { value, .. } => value.contains('['),
        Statement::AssignExpr { expr, .. } => expr_has_command_subst(expr),
        Statement::Call { tokens, .. } => !tokens.as_ref().is_some_and(|tokens| {
            is_pure_tokens_with_metadata_context(
                registry,
                metadata,
                tokens,
                traced,
                has_dynamic_trace,
            )
        }),
        _ => false,
    }
}

/// O127 — store-to-load forwarding for a *computed* single-use
/// assignment.
///
/// The O127 store-to-load-forwarding path.  When `set x [cmd]` (a command
/// substitution or a command-bearing expr) has exactly one
/// `Operand` use of `$x` later in the same block, and nothing
/// between the def and the use can change the value the expression
/// would recompute, emit a grouped rewrite: inline `[set x [cmd]]`
/// at the use site (`O127`) and delete the original store (`O127`,
/// empty replacement).
///
/// Safety gates: single operand use; same
/// executable block; use after def; the value is not SCCP-constant
/// (those are O100/O102); neither the defined name nor any name the
/// expression reads is memory-SSA aliased (upvar / global /
/// variable); no intervening barrier, side-effecting call, or
/// command-substitution assignment; no intervening redefinition of a
/// read name; and, intra-statement, no command substitution appears
/// before `$x` in the use word list unless the inlined expression
/// reads nothing.
pub(super) fn run_store_to_load_forwarding(ctx: &mut PassContext<'_>, fu: &FunctionUnit) {
    use crate::memory_ssa::compute_aliases_with_metadata_context;
    use std::collections::BTreeSet;

    // Like O102, O127 moves a value across statements. A dynamic or opaque
    // variable name can change either the stored name or a name read by the
    // expression under an unseen spelling, so this whole-function rewrite
    // must honour the same fail-closed barrier.
    if fu.dynamic_barrier_blocks_value_motion() {
        return;
    }

    let Some(registry) = ctx.registry else {
        return;
    };
    let Some(context) = fu.invocation_metadata_context(registry) else {
        return;
    };

    // Names whose writes may be visible through an alias (upvar /
    // global / variable). Prefer the on-demand memory-SSA annotation;
    // fall back to a direct alias computation.
    let mut aliased: std::collections::HashSet<String> = match &fu.memory_ssa {
        Some(m) => m.aliased_names().into_iter().collect(),
        None => compute_aliases_with_metadata_context(&fu.ssa, registry, Some(context))
            .iter()
            .flat_map(crate::memory_ssa::AliasSet::names)
            .collect(),
    };
    // A name the frame links outside itself (`global`, `variable`, `upvar`)
    // is also reachable by another frame or a trace on the linked cell, with
    // or without a second spelling in this function: tclsh 8.6.18 and 9.0.4
    // print `traced` for `proc f {} {global x; set x 1; puts $x}` once a write
    // trace on `::x` replaces the value.
    aliased.extend(
        crate::var_observability::analyse_var_observability_with_metadata_context(
            &fu.cfg,
            registry,
            Some(context),
        )
        .escaping_var_names(),
    );

    // Trace context: a traced command is never pure (its write set is
    // unknown), so the intervening-effect check must see it.
    let (traced, has_dynamic_trace) = match ctx.ir_module {
        Some(m) => (m.traced_commands.clone(), m.has_dynamic_trace),
        None => (BTreeSet::new(), false),
    };

    // Source ranges already targeted by earlier passes — O127 must not
    // overlap them. Snapshot before we start emitting.
    let rewritten: Vec<(u32, u32)> = ctx
        .optimisations
        .iter()
        .map(|o| (o.span.start(), o.span.end()))
        .collect();

    // Collect emissions first; group ids + ctx pushes happen after the
    // immutable walk over `fu`.
    let env = ForwardEnv {
        ctx,
        fu,
        registry,
        context: Some(context),
        aliased: &aliased,
        traced: &traced,
        has_dynamic_trace,
        rewritten: &rewritten,
    };
    let mut pending: Vec<(Optimisation, Optimisation)> = fu
        .def_use
        .chains
        .values()
        .filter_map(|chain| forward_candidate(&env, chain))
        .collect();
    // `def_use.chains` is a `HashMap`, so the candidate order — and hence the
    // monotonic group ids allocated below — would vary run-to-run (and between
    // the offset-0 memo build and the whole-module build).  Sort on the edit
    // spans so emission order, group numbering, and the final Vec are
    // byte-identical regardless of map iteration order.
    pending.sort_by_key(|(inline, delete)| {
        (
            inline.span.start(),
            inline.span.end(),
            delete.span.start(),
            delete.span.end(),
        )
    });

    // Emit, allocating a shared group id per inline/delete pair so the
    // two edits apply all-or-nothing.
    for (mut inline, mut delete) in pending {
        let group = ctx.alloc_group();
        inline.group = Some(group);
        delete.group = Some(group);
        ctx.report(inline);
        ctx.report(delete);
    }
}

/// Read-only context threaded into [`forward_candidate`] so the
/// per-chain evaluation stays a pure function over the borrowed
/// analysis state.
struct ForwardEnv<'a> {
    pub(super) ctx: &'a PassContext<'a>,
    fu: &'a FunctionUnit,
    pub(super) registry: &'a tcl_registry::CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    aliased: &'a std::collections::HashSet<String>,
    traced: &'a std::collections::BTreeSet<String>,
    has_dynamic_trace: bool,
    rewritten: &'a [(u32, u32)],
}

/// Evaluate one def-use chain as an O127 store-to-load-forwarding
/// candidate, returning the `(inline, delete)` edit pair when every
/// safety gate passes (see [`run_store_to_load_forwarding`] for the
/// gate list). Returns `None` for any chain that doesn't qualify.
fn forward_candidate(
    env: &ForwardEnv<'_>,
    chain: &crate::def_use::DefUseChain,
) -> Option<(Optimisation, Optimisation)> {
    use crate::def_use::{DefKind, UseKind};
    let (ctx, fu) = (env.ctx, env.fu);

    // Exactly one *non-terminator* use, and it must be a statement operand.
    // A `return $x` / branch-condition read is a `Terminator` use; def-use
    // records those in the chain. The forward preserves the assignment
    // (`[set x …]`), and the operand use precedes the block terminator, so
    // any terminator read still sees `x` — exclude terminator uses from the
    // single-use test. (A phi use still blocks it.)
    let mut non_terminator = chain.uses.iter().filter(|u| u.kind != UseKind::Terminator);
    let use_site = non_terminator.next()?;
    if non_terminator.next().is_some() || use_site.kind != UseKind::Operand {
        return None;
    }
    let def = &chain.definition;
    if def.kind != DefKind::Statement {
        return None;
    }
    // Same executable block; use strictly after def. `def.block` /
    // `use_site.block` are block names; resolve to the shared `BlockId`
    // for the `executable_blocks` / `cfg.blocks` / `ssa.blocks` lookups.
    let def_block = fu.cfg.block_id(&def.block)?;
    if def.block != use_site.block || !fu.sccp.executable_blocks.contains(&def_block) {
        return None;
    }
    let (Ok(def_idx), Ok(use_idx)) = (
        usize::try_from(def.statement_index),
        usize::try_from(use_site.statement_index),
    ) else {
        return None;
    };
    if use_idx <= def_idx {
        return None;
    }
    fu.cfg.statement_source_edit_span(def_block, def_idx)?;
    fu.cfg.statement_source_edit_span(def_block, use_idx)?;
    let block = fu.cfg.blocks.get(&def_block)?;
    let ssa_block = fu.ssa.blocks.get(&def_block)?;
    let (def_stmt, use_stmt) = (
        block.statements.get(def_idx)?,
        block.statements.get(use_idx)?,
    );

    // The use must be a `Call` with tokens so we can pinpoint the `$x`
    // word and check intra-statement evaluation order. Tcl evaluates words
    // left-to-right before invoking the outer command. A command substitution
    // before `$x` would therefore run before the moved assignment; reject it
    // even when the RHS has no scalar reads, because it can still establish an
    // alias or reorder observable effects.
    let Statement::Call {
        tokens: Some(tokens),
        ..
    } = use_stmt
    else {
        return None;
    };
    let symbol = fu.def_use.symbol(&fu.ssa, chain.key.0)?;
    let use_source = crate::ssa::SsaSourceView::at_statement(&fu.ssa, def_block, use_idx);
    let (var_span, has_earlier_effect) = locate_use_var(
        tokens,
        use_source,
        symbol,
        chain.key.1,
        env.registry,
        fu.source_lexer_config(),
    )?;
    if has_earlier_effect {
        return None;
    }

    // An opaque outer invocation at the endpoint runs only after `$x` has
    // been substituted, so it cannot retroactively alias that read. Wildcard
    // transitions at every other statement remain a conservative blocker.
    if has_non_endpoint_wildcard_aliasing(env, def_block, use_idx) {
        return None;
    }

    // The def must be a *computed* assignment — a command substitution
    // or a command-bearing expr. Literal / constant assignments are the
    // O102 / O100 path.
    if !is_computed_assignment(def_stmt) {
        return None;
    }

    if forwarding_value_consumed(env, chain, use_site, def_idx, use_idx) {
        return None;
    }

    let def_source = crate::ssa::SsaSourceView::at_statement(&fu.ssa, def_block, def_idx);
    let def_reads = &ssa_block.statements[def_idx].uses;
    if !forwarded_cells_are_private(env, def_source, symbol, def_reads)
        || cell_is_linked_outside_frame(fu, symbol)
    {
        return None;
    }
    let (Statement::AssignValue { name: def_name, .. }
    | Statement::AssignExpr { name: def_name, .. }) = def_stmt
    else {
        return None;
    };

    // Intervening statements must not change the value the inlined
    // expression would recompute.
    if !intervening_is_safe(env, block, ssa_block, def_idx, use_idx, def_reads) {
        return None;
    }

    build_forward_edits(
        ctx,
        def_stmt,
        def_name,
        var_span,
        env.rewritten,
        fu.base_offset,
    )
}

fn forwarding_value_consumed(
    env: &ForwardEnv<'_>,
    chain: &crate::def_use::DefUseChain,
    use_site: &crate::def_use::UseSite,
    def_idx: usize,
    use_idx: usize,
) -> bool {
    let fu = env.fu;
    let ctx = env.ctx;
    let def = &chain.definition;
    let def_key = chain.key;
    // SCCP constants belong to O100.
    if let Some(sym) = fu.def_use.symbol(&fu.ssa, def_key.0)
        && matches!(
            fu.sccp.values.get(&(sym, def_key.1)),
            Some(LatticeValue::Const(_))
        )
        && fu.sccp.materialises((sym, def_key.1))
    {
        return true;
    }
    // Skip statements another pass already rewrote or consumed.
    let def_key_sym = fu
        .def_use
        .symbol(&fu.ssa, def_key.0)
        .map(|s| (s, def_key.1));
    ctx.propagated_expr_stmts
        .contains(&(def.block.clone(), def_idx))
        || ctx
            .propagated_expr_stmts
            .contains(&(use_site.block.clone(), use_idx))
        || def_key_sym.is_some_and(|k| ctx.propagated_branch_uses.contains(&k))
}

/// Whether any statement other than the endpoint use has a wildcard
/// variable-cell transition obligation.
///
/// This remains deliberately flow-insensitive: a wildcard elsewhere in the
/// function suppresses O127. The sole exception is the outer endpoint
/// invocation, which Tcl runs after substituting the forwarded variable.
fn has_non_endpoint_wildcard_aliasing(
    env: &ForwardEnv<'_>,
    use_block: crate::cfg::BlockId,
    use_idx: usize,
) -> bool {
    env.fu.ssa.blocks.iter().any(|(&block_id, block)| {
        block.statements.iter().enumerate().any(|(idx, statement)| {
            (block_id != use_block || idx != use_idx)
                && crate::memory_ssa::statement_has_wildcard_aliasing_with_metadata_context(
                    &statement.statement,
                    env.registry,
                    env.context,
                )
        })
    })
}

/// Construct the grouped O127 `(inline, delete)` edits for a resolved
/// forwarding candidate, or `None` when the def span is degenerate or
/// would collide with an earlier pass's rewrite.
fn build_forward_edits(
    ctx: &PassContext<'_>,
    def_stmt: &Statement,
    def_name: &str,
    var_span: tcl_lexer::Span,
    rewritten: &[(u32, u32)],
    base_offset: i64,
) -> Option<(Optimisation, Optimisation)> {
    let source = ctx.source;
    // `def_stmt` / `var_span` come from the unit's `cfg`, so they are relative
    // to `base_offset`; absolutise before slicing `source` / comparing against
    // the (absolute) already-rewritten ranges.
    let shift = |sp: tcl_lexer::Span| -> tcl_lexer::Span {
        if base_offset == 0 {
            return sp;
        }
        let s = (i64::from(sp.start()) + base_offset).max(0);
        let e = (i64::from(sp.end()) + base_offset).max(0);
        // Clamped `>= 0`; an absolutised offset past `u32::MAX` is degenerate
        // and clamps to the max (spans are `u32` source offsets).
        tcl_lexer::Span::new(
            u32::try_from(s).unwrap_or(u32::MAX),
            u32::try_from(e).unwrap_or(u32::MAX),
        )
    };
    let def_span = shift(def_stmt.source_edit_span()?);
    let var_span = shift(var_span);
    let (ds, de) = (def_span.start(), def_span.end());
    if de as usize > source.len() || ds >= de {
        return None;
    }
    let target = tcl_lexer::word_span_at(source, var_span);
    // Don't collide with an earlier pass's rewrite (def line or use word).
    if ranges_overlap(rewritten, ds, de) || ranges_overlap(rewritten, target.start(), target.end())
    {
        return None;
    }
    // The def span follows the lexer's inner-end convention, so a value word
    // that is quoted, braced, or bracketed leaves its closer outside. Both the
    // replayed text and the deletion extent need the whole written statement,
    // or the closer is copied without its opener and dropped from its own
    // line.
    let def_span = original_statement_rewrite_span(ctx, def_span)?;
    let stmt_text = &source[def_span.start() as usize..def_span.end() as usize];
    let inline = Optimisation::new(
        DiagCode::O127,
        format!("Inline single-use variable `${def_name}`"),
        target,
        format!("[{stmt_text}]"),
    );
    let delete = Optimisation::new(
        DiagCode::O127,
        "Remove inlined assignment",
        line_delete_span(source, def_span),
        "",
    );
    Some((inline, delete))
}

/// True when `stmt` is a `set`-style assignment whose value is
/// *computed* — an `AssignValue` carrying a `[command substitution]`
/// or an `AssignExpr` whose expression contains one.  Literal
/// assignments are handled by the O100 / O102 paths instead.
fn is_computed_assignment(stmt: &Statement) -> bool {
    use super::super::helpers::expr_simplify::expr_has_command_subst;
    match stmt {
        Statement::AssignValue { value, .. } => value.contains('['),
        Statement::AssignExpr { expr, .. } => expr_has_command_subst(expr),
        _ => false,
    }
}

/// Check the statements strictly between `def_idx` and `use_idx` for
/// anything that could change the value the inlined expression would
/// recompute: a barrier, a side-effecting call, a
/// command-substitution assignment, or a redefinition of a name the
/// expression reads.  The trace / registry / dialect safety context is
/// carried by `env`.
fn intervening_is_safe(
    env: &ForwardEnv<'_>,
    block: &crate::cfg::Block,
    ssa_block: &crate::ssa::SsaBlock,
    def_idx: usize,
    use_idx: usize,
    def_reads: &std::collections::HashMap<crate::ssa::Symbol, crate::ssa::Version>,
) -> bool {
    for idx in (def_idx + 1)..use_idx {
        let Some(stmt) = block.statements.get(idx) else {
            break;
        };
        if statement_may_have_untracked_effects(
            stmt,
            env.registry,
            env.context,
            env.traced,
            env.has_dynamic_trace,
        ) {
            return false;
        }
        // A redefinition of an actual physical read invalidates the forward.
        if let Some(sb) = ssa_block.statements.get(idx)
            && def_reads.keys().any(|symbol| sb.defs.contains_key(symbol))
        {
            return false;
        }
    }
    true
}

/// Verify observer and alias exclusions against the definition's actual cells.
fn forwarded_cells_are_private(
    env: &ForwardEnv<'_>,
    source: crate::ssa::SsaSourceView<'_>,
    defined: crate::ssa::Symbol,
    reads: &std::collections::HashMap<crate::ssa::Symbol, crate::ssa::Version>,
) -> bool {
    source.externally_mutable_by(defined, &env.ctx.cross_event_vars, false, env.registry)
        == Some(false)
        && std::iter::once(defined)
            .chain(reads.keys().copied())
            .all(|symbol| {
                source.externally_mutable_by(symbol, env.aliased, false, env.registry)
                    == Some(false)
            })
}

/// Locate the original single-variable operand by its captured physical read.
/// Earlier opaque words or observed reads retain an evaluation-order obligation.
pub(super) fn locate_use_var(
    tokens: &CommandTokens,
    source: crate::ssa::SsaSourceView<'_>,
    symbol: crate::ssa::Symbol,
    version: crate::ssa::Version,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Option<(tcl_lexer::Span, bool)> {
    use tcl_source::words::WordExpr;
    let mut has_earlier_effect = false;
    for word in tokens.words() {
        match word {
            WordExpr::Variable {
                spelling,
                source: site,
            } => {
                if whole_scalar_variable_reference_name(spelling, config).is_some()
                    && source.read_word_produces_value(word, registry)
                    && source
                        .read_word(word)
                        .is_some_and(|read| read.symbol == symbol && read.version == Some(version))
                {
                    return Some((site.span, has_earlier_effect));
                }
                has_earlier_effect |= !source.read_word_produces_value(word, registry)
                    || source
                        .read_word(word)
                        .is_none_or(|read| read.version.is_none());
            }
            WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => {}
            _ => has_earlier_effect = true,
        }
    }
    None
}

/// True when `[start, end)` overlaps any of the recorded ranges.
fn ranges_overlap(ranges: &[(u32, u32)], start: u32, end: u32) -> bool {
    ranges.iter().any(|&(s, e)| start < e && s < end)
}
