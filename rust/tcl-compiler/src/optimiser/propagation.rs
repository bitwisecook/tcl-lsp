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

//! Constant / copy propagation optimiser pass.
//!
//! Entry points:
//!
//! - **`optimise_constant_var_refs`** (`O100`) — replace a
//!   single-token `$var` call argument with its SCCP-proved
//!   literal, when the value is safe to inline as a bare word.
//! - **`optimise_static_proc_calls`** (`O103`) — fold calls to
//!   pure procs whose return is a proven constant
//!   (`can_fold_static_calls`). Fires
//!   applicable rewrites when the call appears as a `[proc …]`
//!   command substitution inside another call's argv (the argv
//!   span is the rewrite target); the bare statement form stays
//!   hint-only because the call result is discarded and folding
//!   `::answer` to `42` would leave an invalid command name.
//! - **`optimise_return_terminator`** (`O100`) — rewrite
//!   `return $v` as `return K` when `v` is SCCP-constant.
//!   (`O104` is reserved by the canonical optimisation-codes
//!   table for the string-build chain fold, so this path uses
//!   `O100`.)
//!
//! - **`optimise_string_interpolation_var_refs`** (`O100`) —
//!   inline SCCP-proved constants into `"…$x…"` double-quoted
//!   string arguments of calls (only when the interpolation is
//!   safe: the string contains no other substitutions, the
//!   constant value contains no Tcl metacharacters).
//! - **`optimise_load_forwarding`** (`O102`) — forward the
//!   single-reaching literal value of a variable at a use site,
//!   even when SCCP didn't fold it (e.g., when another path
//!   through the CFG makes the lattice Overdefined, but this
//!   particular use is dominated by one literal def). Fires
//!   applicable rewrites with argv-level spans when the use is
//!   a bare `$var` / `${var}` word in a `Statement::Call`; falls
//!   back to hint-only on statements without `CommandTokens` or
//!   on uses inside interpolated strings (the latter covered by
//!   the O100 string-interpolation path).
//!
//! Constant propagation into the condition sub-expressions of
//! `if` / `while` / `for` and the bodies of standalone `expr`
//! commands is handled elsewhere:
//! `branch_folding::propagate_into_branches` (for branch
//! conditions) and [`super::expr_simplify::run`] (for standalone
//! `expr` commands).

use crate::analyses::{ConstValue, LatticeValue};
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{CommandTokens, Script, Statement};
use crate::naming::normalise_var_name;
use crate::tcl_expr_eval::FoldPolicy;
use tcl_core_types::DiagCode;
use tcl_registry::CommandRegistry;

use super::helpers::expr_simplify::{NumericCtx, operand_types, try_unwrap_expr_in_expr};
use super::helpers::literals::{is_safe_word, is_static_var_word};
use super::helpers::spans::{full_quoted_string_span, full_word_span};
use super::{Optimisation, PassContext};

/// Run the propagation pass across every function.
///
/// Emits one `O100` per single-token `$var` argument that SCCP
/// proved to be a safe literal value. "Safe" means the value
/// parses as a decimal integer or as a bare identifier-shaped
/// word; string literals with Tcl metacharacters (`$`, `[`, `\`,
/// …) are skipped because substituting them as a bare word
/// would change the command's interpretation.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    run_oo_method_folds(ctx, cu);
    // The top-level body's extra escaping set (names some other procedure
    // declares `global`) — shared by the top-level constants projection
    // below and by `run_load_forwarding`.
    let top_level_extra_escaping =
        crate::var_observability::scan_module_global_names(&cu.ir_module);
    let no_extra_escaping = std::collections::HashSet::new();
    run_function(
        ctx,
        cu,
        &cu.top_level,
        &cu.ir_module.top_level,
        "::",
        &top_level_extra_escaping,
    );
    for (qname, fu) in &cu.procedures {
        let Some(proc) = cu.ir_module.procedures.get(qname) else {
            continue;
        };
        // The call-site namespace for O103 chain resolution is the proc's
        // own namespace (`::ns::foo` resolves a bare `bar` against `::ns`).
        let namespace = super::helpers::naming::namespace_from_qualified(qname);
        run_function(ctx, cu, fu, &proc.body, &namespace, &no_extra_escaping);
    }
    // Load-forwarding runs a separate per-function pass on top
    // of the SCCP-based substitutions. It consults the def-use
    // chains directly and fires independently of the SCCP
    // lattice — a variable whose *sole* reaching def is a
    // literal Assign is forwarded even when other paths make
    // the lattice Overdefined. Being independent of the SCCP lattice, it
    // must apply the *same* externally-mutable guard SCCP applies itself
    // (`global`/`variable`/`upvar`/`trace` aliasing, plus — for the
    // top-level body specifically — any name some other procedure declares
    // via `global`; see `scan_module_global_names`, and, for every function,
    // a name traced by some *other* proc, via the registry-driven whole-
    // module `Module::traced_variables`/`has_dynamic_variable_trace` facts),
    // or it would forward a stale literal past a call that reassigns or
    // traces the "sole" def.
    if let Some(registry) = ctx.registry {
        let trace = crate::sccp::TraceInputs {
            registry,
            traced_variables: &cu.ir_module.traced_variables,
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
        };
        run_load_forwarding(ctx, &cu.top_level, &top_level_extra_escaping, trace);
        for fu in cu.procedures.values() {
            run_load_forwarding(ctx, fu, &no_extra_escaping, trace);
        }
    }
    // O127 store-to-load forwarding for *computed* single-use
    // assignments (`set x [cmd]` inlined to its sole use site, then the
    // store deleted).  Skips the top-level body.
    for fu in cu.procedures.values() {
        run_store_to_load_forwarding(ctx, fu);
    }
}

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
    dialect: Option<&str>,
    traced: &std::collections::BTreeSet<String>,
    has_dynamic_trace: bool,
) -> bool {
    use super::helpers::expr_simplify::expr_has_command_subst;
    use crate::gvn::is_pure_command_with_traces;

    match stmt {
        Statement::Barrier { .. } | Statement::UpFrame { .. } => true,
        Statement::AssignValue { value, .. } => value.contains('['),
        Statement::AssignExpr { expr, .. } => expr_has_command_subst(expr),
        Statement::Call { command, args, .. } => !is_pure_command_with_traces(
            registry,
            command,
            args,
            dialect,
            traced,
            has_dynamic_trace,
        ),
        _ => false,
    }
}

/// Forward a single reaching literal definition to each of its
/// Operand use sites. Emits `O102` ("Forward literal load") with
/// the literal text as replacement.
///
/// When the consuming statement is a `Statement::Call` with
/// [`CommandTokens`] populated, fire one applicable `O102` per
/// argv entry whose text is `$var` / `${var}` matching the
/// defined variable — the argv span is the precise rewrite
/// target. Fall back to a hint-only diagnostic covering the whole
/// consuming statement when no `CommandTokens` are present or the
/// use is on a non-Call statement (where we still don't have
/// per-operand spans).
///
/// Safety gates layer two independent alias/trace facts plus one
/// intervening-effect scan, each catching cases the other misses. A plain
/// proc-local variable that is never `global`/`variable`/`upvar`/`trace`
/// declared anywhere in the module cannot be touched by an intervening
/// call to *any* other proc — Tcl's frame-based scoping gives a callee no
/// way to reach a caller's private local without one of those four
/// mechanisms — so, unlike an earlier revision of this pass, an ordinary
/// intervening call does *not* gate the forward on its own (see
/// `o102_still_forwards_top_level_global_no_proc_touches` and
/// `o102_still_forwards_genuinely_local_proc_variable`, which lock this
/// precision in):
///
/// - `escaping` (this function's own [`analyse_var_observability`] plus
///   `extra_escaping` — the top-level's
///   [`crate::var_observability::scan_module_global_names`] result, or an
///   empty set for an ordinary procedure — plus `trace.traced_variables`,
///   the registry-driven whole-module [`crate::ir::Module::traced_variables`]
///   fact) via [`crate::sccp::is_externally_mutable`] — the same guard SCCP
///   applies to its own lattice, so O102 (independent of the SCCP lattice)
///   stays consistent with it.
/// - [`has_intervening_barrier`] — a `Statement::Barrier`/`UpFrame`
///   (a literal-body `uplevel`/`interp eval`, which *can* reach into an
///   arbitrary frame) between def and use, checked both same-block
///   (precisely) and cross-block (conservatively).
fn run_load_forwarding(
    ctx: &mut PassContext<'_>,
    fu: &crate::compilation_unit::FunctionUnit,
    extra_escaping: &std::collections::HashSet<String>,
    trace: crate::sccp::TraceInputs<'_>,
) {
    use crate::def_use::{DefKind, UseKind};
    use crate::ir::Statement;

    // Independent of the SCCP lattice (see the doc comment on the call
    // site in `run`), this pass must apply the same externally-mutable
    // guard SCCP applies to its own lattice — a `global`/`variable`/
    // `upvar`/`trace`-aliased name's "sole reaching def" is not actually
    // sole: some other call frame can reassign it between the def and a
    // later use.
    let mut escaping = crate::var_observability::analyse_var_observability(&fu.cfg, trace.registry)
        .escaping_var_names();
    escaping.extend(extra_escaping.iter().cloned());
    escaping.extend(trace.traced_variables.iter().cloned());

    for chain in fu.def_use.chains.values() {
        if chain.definition.kind != DefKind::Statement {
            continue;
        }
        let var_name = chain.key.0.as_str();
        if crate::sccp::is_externally_mutable(var_name, &escaping, trace.has_dynamic_variable_trace)
        {
            continue;
        }
        // A synthetic may-def (the element fan of a dynamic-key write, or a
        // base refresh) is not a real reaching definition — forwarding its
        // statement's value would forward the *other* element's literal.
        if fu.ssa.is_synthetic_def(
            &chain.definition.block,
            chain.definition.statement_index,
            var_name,
        ) {
            continue;
        }
        // Find the defining statement — must be an AssignConst
        // with a literal value to forward. AssignValue without
        // substitutions could also work, but we're conservative
        // and restrict to the same shapes.
        let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
            continue;
        };
        let Some(block) = fu.cfg.block_by_name(&chain.definition.block) else {
            continue;
        };
        let Some(def_stmt) = block.statements.get(idx) else {
            continue;
        };
        let literal = match def_stmt {
            Statement::AssignConst { value, .. } => value.clone(),
            Statement::AssignValue { value, .. }
                if !value.contains(['$', '[', '\\', '"']) && !value.is_empty() =>
            {
                value.clone()
            }
            _ => continue,
        };
        if !is_value_safe_bare_word(&literal) {
            continue;
        }
        let message =
            format!("Forward literal load of '{var_name}' from its single reaching definition");
        for use_site in &chain.uses {
            if use_site.kind != UseKind::Operand {
                continue;
            }
            let Ok(use_idx) = usize::try_from(use_site.statement_index) else {
                continue;
            };
            let Some(use_block) = fu.cfg.block_by_name(&use_site.block) else {
                continue;
            };
            let Some(use_stmt) = use_block.statements.get(use_idx) else {
                continue;
            };
            // A fanned may-def's prior-version read is a synthetic use —
            // there is no `$var` in that statement to rewrite.
            if fu
                .ssa
                .blocks
                .values()
                .find(|b| b.name == use_site.block)
                .and_then(|b| b.statements.get(use_idx))
                .is_some_and(|st| {
                    fu.ssa
                        .var_symbol(var_name)
                        .is_some_and(|sym| st.may_defs.contains(&sym))
                })
            {
                continue;
            }
            if has_intervening_barrier(fu, &chain.definition.block, idx, &use_site.block, use_idx) {
                continue;
            }
            report_load_forward(ctx, fu, use_stmt, var_name, &message, &literal);
        }
    }
}

/// True when an opaque effect — a `Statement::Barrier` or `Statement::UpFrame`
/// (the CFG shape for a literal-body `uplevel`/`interp eval`) — could run
/// between `def_block[def_idx]` and `use_block[use_idx]`, making a "sole
/// reaching definition" forward unsound: `uplevel 1 {…}` / `uplevel #0 {…}`
/// evaluates its body in a *different* frame (the caller's, or the absolute
/// global one) and can reassign any name visible there, exactly like an
/// opaque call. Independent of [`crate::sccp::is_externally_mutable`] (which
/// only catches `global`/`variable`/`upvar`/`trace`-*declared* aliasing): a
/// plain proc-local variable with no alias/trace at all can still be mutated
/// by a literal `uplevel #0 {…}` body a few lines later.
///
/// Same-block case: checked precisely (only the statements strictly between
/// the two indices). Cross-block case: conservatively checks the remainder of
/// `def_block`, the prefix of `use_block`, and — since enumerating every path
/// between two arbitrary blocks is more machinery than this cheap forwarding
/// pass warrants — every *other* block in the function; a barrier/upframe
/// anywhere else in the body blocks the forward even if it turns out to sit on
/// an unrelated branch.
fn has_intervening_barrier(
    fu: &crate::compilation_unit::FunctionUnit,
    def_block: &str,
    def_idx: usize,
    use_block: &str,
    use_idx: usize,
) -> bool {
    fn is_barrier(stmt: &Statement) -> bool {
        matches!(stmt, Statement::Barrier { .. } | Statement::UpFrame { .. })
    }

    if def_block == use_block {
        let Some(block) = fu.cfg.block_by_name(def_block) else {
            return true; // can't verify safety — conservatively block the forward
        };
        let (lo, hi) = if def_idx <= use_idx {
            (def_idx, use_idx)
        } else {
            (use_idx, def_idx)
        };
        return block.statements[lo.saturating_add(1)..hi.min(block.statements.len())]
            .iter()
            .any(is_barrier);
    }
    let Some(def_blk) = fu.cfg.block_by_name(def_block) else {
        return true;
    };
    if def_blk.statements[(def_idx + 1).min(def_blk.statements.len())..]
        .iter()
        .any(is_barrier)
    {
        return true;
    }
    let Some(use_blk) = fu.cfg.block_by_name(use_block) else {
        return true;
    };
    if use_blk.statements[..use_idx.min(use_blk.statements.len())]
        .iter()
        .any(is_barrier)
    {
        return true;
    }
    fu.cfg
        .blocks
        .values()
        .any(|b| b.name != def_block && b.name != use_block && b.statements.iter().any(is_barrier))
}

/// Emit the `O102` rewrite(s) for one use site: a precise, applicable
/// per-argv rewrite for each bare `$var` / `${var}` word in a
/// `Statement::Call`'s tracked tokens, or a hint-only fallback covering
/// the whole consuming statement when no such word is found (the use is
/// on a non-`Call` statement, `CommandTokens` weren't captured, or the
/// read is nested inside a larger construct — e.g. an interpolated
/// string, which the O100 string-interpolation path handles instead).
fn report_load_forward(
    ctx: &mut PassContext<'_>,
    fu: &crate::compilation_unit::FunctionUnit,
    use_stmt: &Statement,
    var_name: &str,
    message: &str,
    literal: &str,
) {
    let mut emitted_applicable = false;
    if let Statement::Call {
        tokens: Some(tokens),
        ..
    } = use_stmt
    {
        for (i, argv_span) in tokens.argv.iter().enumerate() {
            let Some(text) = tokens.argv_texts.get(i) else {
                continue;
            };
            // A braced word (`{$x}`) is a literal — Tcl performs no
            // substitution inside braces — so `$x` there must not be
            // forwarded. `argv_texts` has the braces stripped, so the
            // word kind is the only signal.
            if tokens.argv_kinds.get(i) == Some(&tcl_lexer::TokenType::Str) {
                continue;
            }
            if !simple_var_ref_matches(text, var_name) {
                continue;
            }
            ctx.report(Optimisation::new(
                DiagCode::O102,
                message.to_owned(),
                full_word_span(ctx.source, fu.abs_span(*argv_span)),
                literal.to_owned(),
            ));
            emitted_applicable = true;
        }
    }
    if emitted_applicable {
        return;
    }
    let mut opt = Optimisation::new(
        DiagCode::O102,
        message.to_owned(),
        fu.abs_span(use_stmt.span()),
        literal.to_owned(),
    );
    opt.hint_only = true;
    ctx.report(opt);
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
fn run_store_to_load_forwarding(ctx: &mut PassContext<'_>, fu: &FunctionUnit) {
    use crate::memory_ssa::compute_aliases;
    use std::collections::BTreeSet;
    use tcl_dialect::DialectProfile;

    let Some(registry) = ctx.registry else {
        return;
    };
    // Aliasing facts depend on the selected registry profile. Without an
    // explicit profile this optimisation cannot prove its alias safety gate,
    // so it abstains rather than interpreting every dialect at once.
    let Some(dialect) = ctx
        .dialect
        .map(|name| DialectProfile::by_name(name).availability_mask)
    else {
        return;
    };

    // Names whose writes may be visible through an alias (upvar /
    // global / variable). Prefer the on-demand memory-SSA annotation;
    // fall back to a direct alias computation.
    let aliased: BTreeSet<String> = match &fu.memory_ssa {
        Some(m) => m.aliased_names(),
        None => compute_aliases(&fu.ssa, registry, dialect)
            .iter()
            .flat_map(crate::memory_ssa::AliasSet::names)
            .collect(),
    };

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
        dialect,
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
    ctx: &'a PassContext<'a>,
    fu: &'a FunctionUnit,
    registry: &'a tcl_registry::CommandRegistry,
    dialect: tcl_registry::dialects::DialectSet,
    aliased: &'a std::collections::BTreeSet<String>,
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
    use std::collections::BTreeSet;

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
    let (var_span, has_earlier_effect) = locate_use_var(tokens, chain.key.0.as_str())?;
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

    let def_key = chain.key.clone();
    // Skip SCCP constants (O100 owns those). The def-use chain keys on the
    // variable name; resolve it to the SSA symbol to index the SCCP lattice.
    if let Some(sym) = fu.ssa.var_symbol(&def_key.0)
        && matches!(
            fu.sccp.values.get(&(sym, def_key.1)),
            Some(LatticeValue::Const(_))
        )
    {
        return None;
    }
    // Skip statements another pass already rewrote / consumed. The
    // def-use chain keys on the variable name; resolve it to the SSA symbol
    // to test the `(Symbol, Version)`-keyed branch-use set.
    let def_key_sym = fu.ssa.var_symbol(&def_key.0).map(|s| (s, def_key.1));
    if ctx
        .propagated_expr_stmts
        .contains(&(def.block.clone(), def_idx))
        || ctx
            .propagated_expr_stmts
            .contains(&(use_site.block.clone(), use_idx))
        || def_key_sym.is_some_and(|k| ctx.propagated_branch_uses.contains(&k))
    {
        return None;
    }

    let def_name = chain.key.0.as_str();
    if ctx.cross_event_vars.contains(def_name) || env.aliased.contains(def_name) {
        return None;
    }
    // Names the def expression reads — used for alias + version safety.
    let def_read_names: BTreeSet<String> = ssa_block.statements[def_idx]
        .uses
        .keys()
        .map(|&s| fu.ssa.var_name(s).to_owned())
        .collect();
    if def_read_names.iter().any(|n| env.aliased.contains(n)) {
        return None;
    }

    // Intervening statements must not change the value the inlined
    // expression would recompute.
    if !intervening_is_safe(env, block, ssa_block, def_idx, use_idx, &def_read_names) {
        return None;
    }

    build_forward_edits(
        ctx.source,
        def_stmt,
        def_name,
        var_span,
        env.rewritten,
        fu.base_offset,
    )
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
                && crate::memory_ssa::statement_has_wildcard_aliasing(
                    &statement.statement,
                    env.registry,
                    env.dialect,
                )
        })
    })
}

/// Construct the grouped O127 `(inline, delete)` edits for a resolved
/// forwarding candidate, or `None` when the def span is degenerate or
/// would collide with an earlier pass's rewrite.
fn build_forward_edits(
    source: &str,
    def_stmt: &Statement,
    def_name: &str,
    var_span: tcl_lexer::Span,
    rewritten: &[(u32, u32)],
    base_offset: i64,
) -> Option<(Optimisation, Optimisation)> {
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
    let def_span = shift(def_stmt.span());
    let var_span = shift(var_span);
    let (ds, de) = (def_span.start(), def_span.end());
    if de as usize > source.len() || ds >= de {
        return None;
    }
    let target = full_word_span(source, var_span);
    // Don't collide with an earlier pass's rewrite (def line or use word).
    if ranges_overlap(rewritten, ds, de) || ranges_overlap(rewritten, target.start(), target.end())
    {
        return None;
    }
    let stmt_text = &source[ds as usize..de as usize];
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
    use super::helpers::expr_simplify::expr_has_command_subst;
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
    def_read_names: &std::collections::BTreeSet<String>,
) -> bool {
    for idx in (def_idx + 1)..use_idx {
        let Some(stmt) = block.statements.get(idx) else {
            break;
        };
        if statement_may_have_untracked_effects(
            stmt,
            env.registry,
            env.ctx.dialect,
            env.traced,
            env.has_dynamic_trace,
        ) {
            return false;
        }
        // A redefinition of any read name invalidates the forward.
        if let Some(sb) = ssa_block.statements.get(idx)
            && def_read_names.iter().any(|n| {
                env.fu
                    .ssa
                    .var_symbol(n)
                    .is_some_and(|s| sb.defs.contains_key(&s))
            })
        {
            return false;
        }
    }
    true
}

/// Find the `$var` word in a use-site command's tokens and report
/// whether a command substitution appears before it.
///
/// Returns `(var_word_span, has_earlier_command_subst)`. The match
/// requires a single-token `$var` / `${var}` word (a `$var` embedded
/// in a larger word is not a clean inline target).
fn locate_use_var(tokens: &CommandTokens, var_name: &str) -> Option<(tcl_lexer::Span, bool)> {
    use tcl_lexer::TokenType;
    let mut has_earlier_effect = false;
    for (i, span) in tokens.argv.iter().enumerate() {
        let text = tokens.argv_texts.get(i)?;
        let kind = tokens.argv_kinds.get(i).copied();
        if kind == Some(TokenType::Var) && simple_var_ref_matches(text, var_name) {
            // Multi-token words (e.g. `$x$y`) aren't a clean target.
            if tokens.single_token_word.get(i).copied().unwrap_or(true) {
                return Some((*span, has_earlier_effect));
            }
            continue;
        }
        if kind == Some(TokenType::Cmd) {
            has_earlier_effect = true;
        }
    }
    None
}

/// Extend a statement span to cover the whole source line — the
/// leading indentation back to the previous newline and the trailing
/// newline — so deleting the store removes its line cleanly.
fn line_delete_span(source: &str, span: tcl_lexer::Span) -> tcl_lexer::Span {
    let bytes = source.as_bytes();
    let mut start = span.start() as usize;
    // Walk back over leading spaces / tabs on the line.
    while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
        start -= 1;
    }
    let mut end = span.end() as usize;
    // Consume a single trailing newline (and a preceding CR).
    if end < bytes.len() && bytes[end] == b'\n' {
        end += 1;
    } else if end + 1 < bytes.len() && bytes[end] == b'\r' && bytes[end + 1] == b'\n' {
        end += 2;
    }
    tcl_lexer::Span::new(
        u32::try_from(start).unwrap_or(span.start()),
        u32::try_from(end).unwrap_or(span.end()),
    )
}

/// True when `[start, end)` overlaps any of the recorded ranges.
fn ranges_overlap(ranges: &[(u32, u32)], start: u32, end: u32) -> bool {
    ranges.iter().any(|&(s, e)| start < e && s < end)
}

/// True when `text` is the bare word `$name` / `${name}` and the
/// parsed name equals `var_name` (including namespace-qualified
/// comparison via [`normalise_var_name`]).
fn simple_var_ref_matches(text: &str, var_name: &str) -> bool {
    let Some(name) = simple_var_ref(text) else {
        return false;
    };
    if name == var_name {
        return true;
    }
    // Compare normalised names so `$::ns::x` matches chain key
    // `::ns::x` (and vice versa), and `$x(0)` is rejected by
    // simple_var_ref already.
    normalise_var_name(&format!("${name}")) == normalise_var_name(&format!("${var_name}"))
}

/// The statically-proven facts about the `TclOO` method frame a body runs in
/// (issue #1080).
///
/// Built once per method body by [`oo_frame_for`], which is where every
/// abstention lives; by the time an `OoFrame` exists, each field is a value
/// real `tclsh` would produce for every reachable invocation of that body.
#[derive(Debug, Clone)]
struct OoFrame {
    /// The fully-qualified class that defines this implementation — the value
    /// of [`tcl_registry::OoContextFact::DefiningClass`].
    defining_class: String,
}

/// `method`'s provable frame facts, or `None` to abstain.
///
/// Folding direction is abstain-toward-no-fold: a wrong fold is a correctness
/// bug, a missed fold only a lost optimisation. Three gates, each pinned to
/// the oracle transcript on [`tcl_registry::OoContextFact::DefiningClass`]:
///
/// * **Class-object implementations.** `self class` *raises* ("method not
///   defined by a class") inside an `oo::objdefine` instance method and inside
///   a `self method`, and inside a `classmethod` it answers the internal
///   `::oo::ObjN:: oo ::delegate` class (tclsh 9.0.4) — either way there is
///   no statically-knowable value to fold to. `oo::objdefine` never reaches here at all (the
///   lowering's OO extraction only recognises the `oo::class`-family and
///   `oo::define` definers), so the live gate is [`MethodKind::ClassMethod`].
/// * **A class the source doesn't name.** The lowering already declines a
///   dynamic class word, leaving an empty name; nothing to answer with.
/// * **A renamed class command.** `rename ::R ::R2` makes `self class` answer
///   `::R2` from bodies written under `::R` — the same rename-captures-identity
///   rule `indirection.rs` applies. [`trusts_proc_binding`] is the
///   whole-module, flow-insensitive query for "this name was never moved onto
///   or vacated", which is exactly the property needed: a rename buried in a
///   proc body still fires before some later method call.
///
/// [`trusts_proc_binding`]: crate::command_binding::ModuleCommandMutations::trusts_proc_binding
fn oo_frame_for(
    method: &crate::ir::MethodDef,
    mutations: &crate::command_binding::ModuleCommandMutations,
) -> Option<OoFrame> {
    use crate::ir::MethodKind;
    if method.kind == MethodKind::ClassMethod {
        return None;
    }
    if method.class_name.is_empty() {
        return None;
    }
    if !mutations.trusts_proc_binding(&method.class_name) {
        return None;
    }
    Some(OoFrame {
        defining_class: method.class_name.clone(),
    })
}

/// The method-local constants a method body may propagate, or an empty map
/// when it may propagate none (issue #1097).
///
/// This applies `elimination.rs`'s escaping model to the propagation
/// lattice.  `elimination.rs` already knows that a `TclOO` instance variable
/// escapes the method frame — it feeds
/// [`crate::ir::MethodDef::instance_vars`] through the same channel iRules
/// cross-event state uses, so a state-mutating `set ivar …` is never deleted
/// as a dead store.  The propagation lattice had no such model, which is why
/// this walk used to carry an unconditionally empty constants map.
///
/// SCCP is therefore **re-run** for the method with those names in its
/// escaping set, rather than the shared [`FunctionUnit::sccp`] being rebuilt
/// that way: forcing them `Overdefined` also poisons everything *derived*
/// from them (`set a $ivar ; set b $a`), which no filter on a projected map
/// could do — but it is a deliberately propagation-only view.  The unit's own
/// lattice stays as built, because other consumers read facts an instance
/// variable legitimately carries (the object-collection element typing of
/// issue #797 harvests `dict set pins $k [Pin new]` out of exactly such a
/// name).
///
/// What survives the projection is provably method-local: a name the class
/// never declares as state, never aliased by `variable` / `my variable` /
/// `upvar` / `global` inside the body (the per-function
/// [`crate::var_observability`] scan SCCP runs covers those), and not under a
/// trace.  A `my …` / `next` / `[self …]` dispatch is then no longer a barrier
/// for it: object state is the only thing such a dispatch can reach, and every
/// name that names object state has already left the map.
fn oo_method_constants(
    ctx: &PassContext<'_>,
    cu: &CompilationUnit,
    qname: &str,
    frame: &OoFrame,
    allow_locals: bool,
) -> std::collections::HashMap<String, String> {
    if !allow_locals {
        return std::collections::HashMap::new();
    }
    let (Some(fu), Some(registry)) = (cu.methods.get(qname), ctx.registry) else {
        return std::collections::HashMap::new();
    };
    if fu.complexity_guarded {
        return std::collections::HashMap::new();
    }
    // The re-run also carries the registry builtin-fold context (issue
    // #1134): `set base [self class]; set ns [namespace qualifiers $base]`
    // folds to fixpoint *inside* the lattice, so `base` and `ns` both
    // project as method-local constants rather than the chain stopping
    // after the first O129 suggestion. The frame's defining class is
    // proven by [`oo_frame_for`], so the `[self class]` frame fact is
    // sound here and only here.
    //
    // The escaping set is the unit's own `method_facts` carrier — the same
    // struct the existence fold and the W-family diagnostics read — never a
    // second lookup of `MethodDef::instance_vars` (issue #1174).
    let Some(facts) = fu.method_facts.as_deref() else {
        return std::collections::HashMap::new();
    };
    let sccp = crate::sccp::sccp_with_builtin_folds(
        &fu.cfg,
        &fu.ssa,
        None,
        FoldPolicy::from_registry(registry),
        &facts.instance_vars,
        crate::sccp::TraceInputs {
            registry,
            traced_variables: &cu.ir_module.traced_variables,
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
        },
        Some(crate::sccp::BuiltinFoldInputs {
            registry,
            mutations: &ctx.command_mutations,
            dialect: ctx.dialect,
            defining_class: Some(&frame.defining_class),
        }),
    );
    sccp_constants_from(&sccp, &fu.ssa)
}

/// Fold the command substitutions in every `TclOO` method body that the
/// enclosing method frame makes constant — `[self class]`, anything built
/// purely out of it (`[namespace qualifiers [self class]]` folds in one step,
/// since the builtin fold resolves nested substitutions first), and — since
/// issue #1097 — anything built out of a *provably method-local* variable as
/// well.
///
/// Still deliberately narrower than [`run_function`]: no O103 static-proc-call
/// fold and no `namespace`-relative chain resolution runs here.  What issue
/// #1097 switched on is the constants map, which used to be unconditionally
/// empty because the propagation lattice had no model of which names are
/// object state; [`oo_method_constants`] is that model.
fn run_oo_method_folds(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    if ctx.registry.is_none() {
        return;
    }
    // The PER-METHOD dispatch barrier (issue #1164): a method is barred
    // only when its dispatches can actually reach a caller-frame-reaching
    // (or unreadable) method — see `super::method_barrier`. The registry is
    // present (checked above), so unwrap-by-default to an all-barred
    // stance is unreachable; a bare context without one folds nothing
    // anyway.
    let barrier = ctx
        .registry
        .map(|registry| super::method_barrier::compute(&cu.ir_module, registry));
    // Sorted so the emitted rewrite order is deterministic across runs
    // (`ir_module.methods` is a `HashMap`).
    let mut qnames: Vec<&String> = cu.ir_module.methods.keys().collect();
    qnames.sort();
    for qname in qnames {
        let Some(method) = cu.ir_module.methods.get(qname) else {
            continue;
        };
        let Some(frame) = oo_frame_for(method, &ctx.command_mutations) else {
            continue;
        };
        let allow_locals = barrier.as_ref().is_some_and(|b| b.allows_locals(qname));
        let constants = oo_method_constants(ctx, cu, qname, &frame, allow_locals);
        walk_oo_script(ctx, &method.body, &frame, &constants, 0);
    }
}

/// `depth` is the nesting level of `script` — see
/// [`super::MAX_OPTIMISER_WALK_DEPTH`].
fn walk_oo_script(
    ctx: &mut PassContext<'_>,
    script: &Script,
    frame: &OoFrame,
    constants: &std::collections::HashMap<String, String>,
    depth: u32,
) {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        walk_oo_statement(ctx, stmt, frame, constants, depth);
    }
}

/// Visit one statement's command words for the frame-constant fold, then
/// recurse into any nested body it carries.
fn walk_oo_statement(
    ctx: &mut PassContext<'_>,
    stmt: &Statement,
    frame: &OoFrame,
    constants: &std::collections::HashMap<String, String>,
    depth: u32,
) {
    match stmt {
        Statement::Call {
            tokens: Some(t), ..
        }
        | Statement::AssignValue {
            tokens: Some(t), ..
        } => {
            // O100 / O129-in-interpolation over the method-local constants
            // (empty unless issue #1097's escaping model proved some name
            // method-local), then the frame-constant cmd-sub folds.
            visit_call_tokens(ctx, t, constants);
            visit_oo_frame_folds(ctx, t, frame, constants);
        }
        // `return [self class]` is the single most common shape of all, and a
        // `Return` carries no `CommandTokens` — only the whole statement span
        // and the raw value text. Rewrite the statement the way the sibling
        // `return`-terminator folds (O101 / O115) already do.
        Statement::Return {
            span,
            value: Some(raw),
            expr: None,
            ..
        } => try_oo_frame_return_fold(ctx, *span, raw, frame, constants),
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_oo_script(ctx, &c.body, frame, constants, depth + 1);
            }
            if let Some(b) = else_body {
                walk_oo_script(ctx, b, frame, constants, depth + 1);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_oo_script(ctx, init, frame, constants, depth + 1);
            walk_oo_script(ctx, next, frame, constants, depth + 1);
            walk_oo_script(ctx, body, frame, constants, depth + 1);
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Block { body, .. } => walk_oo_script(ctx, body, frame, constants, depth + 1),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_oo_script(ctx, body, frame, constants, depth + 1);
            for h in handlers {
                walk_oo_script(ctx, &h.body, frame, constants, depth + 1);
            }
            if let Some(fb) = finally_body {
                walk_oo_script(ctx, fb, frame, constants, depth + 1);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(body) = &a.body {
                    walk_oo_script(ctx, body, frame, constants, depth + 1);
                }
            }
            if let Some(b) = default_body {
                walk_oo_script(ctx, b, frame, constants, depth + 1);
            }
        }
        _ => {}
    }
}

/// Report an `O129` for each single-token `[cmd …]` word of this command that
/// the method frame folds to a constant.
fn visit_oo_frame_folds(
    ctx: &mut PassContext<'_>,
    tokens: &CommandTokens,
    frame: &OoFrame,
    constants: &std::collections::HashMap<String, String>,
) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let mut rewrites: Vec<(tcl_lexer::Span, String)> = Vec::new();
    for (i, argv_span) in tokens.argv.iter().enumerate() {
        if !tokens.single_token_word.get(i).copied().unwrap_or(false) {
            continue;
        }
        let Some(text) = tokens.argv_texts.get(i) else {
            continue;
        };
        let Some(inner) = text.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
            continue;
        };
        if let Some(folded) = try_o129_fold(
            registry,
            &ctx.command_mutations,
            constants,
            inner,
            ctx.dialect,
            Some(frame),
        ) {
            rewrites.push((*argv_span, folded));
        }
    }
    for (span, folded) in rewrites {
        ctx.report(Optimisation::new(
            DiagCode::O129,
            "Fold constant builtin command substitution",
            full_word_span(ctx.source, span),
            folded,
        ));
    }
}

/// `return [self class]` → `return ::TheClass`.
///
/// The whole statement is the rewrite target (a `Return` keeps no per-word
/// tokens), so the replacement re-spells the `return` keyword — the same shape
/// [`try_fold_return_terminator`]'s O101 / O115 rewrites emit.
fn try_oo_frame_return_fold(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    raw: &str,
    frame: &OoFrame,
    constants: &std::collections::HashMap<String, String>,
) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let Some(inner) = raw
        .trim()
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
    else {
        return;
    };
    let Some(folded) = try_o129_fold(
        registry,
        &ctx.command_mutations,
        constants,
        inner,
        ctx.dialect,
        Some(frame),
    ) else {
        return;
    };
    ctx.report(Optimisation::new(
        DiagCode::O129,
        "Fold constant builtin command substitution",
        span,
        format!("return {folded}"),
    ));
}

fn run_function(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    fu: &FunctionUnit,
    script: &Script,
    namespace: &str,
    extra_escaping: &std::collections::HashSet<String>,
) {
    // Project the per-function SCCP lattice into a name → literal
    // map that survives only when every tracked version of the
    // variable collapses to the same single constant value. No
    // additional trace/alias filtering is needed here: `sccp()` itself
    // forces a traced or frame-aliased name's own lattice entry (and
    // anything derived from it) to `Overdefined`, so a projected literal
    // is already trace/alias-safe by construction — `run_load_forwarding`
    // (below) is the one exception that still needs its own check, since it
    // runs an independent def-use-chain scan that never consults `fu.sccp`.
    let constants = constants_with_builtin_folds(ctx, cu, fu, extra_escaping);
    let numeric = operand_types(fu);
    walk_script(ctx, cu, script, &constants, Some(&numeric), namespace, 0);
}

/// The function's projected constants map, widened with the registry
/// builtin-fold lattice (issue #1134).
///
/// The baseline is the shared [`FunctionUnit::sccp`] projection
/// ([`sccp_constants_for`]) — untouched, so every existing single-hop fold
/// stays byte-identical and the per-unit lattice memo (which cannot carry
/// the whole-module command-mutation fact in its key) is not disturbed.
/// When the body contains a command-substitution assignment, SCCP is
/// re-run with [`crate::sccp::BuiltinFoldInputs`] so chains like `set a
/// [string length abcdef]; set b [expr {$a + 1}]` close inside the
/// lattice; the re-run's projection is overlaid **additively**
/// (`entry().or_insert`) — it can only add names, never change or drop a
/// baseline constant. The re-run passes no interprocedural seeds (they
/// are not stored on the unit); a fold needing a seeded parameter is
/// simply missed, never wrong.
///
/// Cost containment: the re-run is skipped entirely (returning the
/// baseline) when the registry is absent, the unit is complexity-guarded,
/// or no `AssignValue` RHS is a whole-word `[…]` substitution — so a body
/// with nothing to fold pays one cheap scan and nothing else.
fn constants_with_builtin_folds(
    ctx: &PassContext<'_>,
    cu: &CompilationUnit,
    fu: &FunctionUnit,
    extra_escaping: &std::collections::HashSet<String>,
) -> std::collections::HashMap<String, String> {
    let mut constants = sccp_constants_for(fu);
    let Some(registry) = ctx.registry else {
        return constants;
    };
    if fu.complexity_guarded || !has_cmd_subst_assignment(fu) {
        return constants;
    }
    let rerun = crate::sccp::sccp_with_builtin_folds(
        &fu.cfg,
        &fu.ssa,
        None,
        FoldPolicy::from_registry(registry),
        extra_escaping,
        crate::sccp::TraceInputs {
            registry,
            traced_variables: &cu.ir_module.traced_variables,
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
        },
        Some(crate::sccp::BuiltinFoldInputs {
            registry,
            mutations: &ctx.command_mutations,
            dialect: ctx.dialect,
            // No method frame here — `[self class]`-style frame facts fold
            // only in `run_oo_method_folds`' proven method re-runs.
            defining_class: None,
        }),
    );
    for (name, text) in sccp_constants_from(&rerun, &fu.ssa) {
        constants.entry(name).or_insert(text);
    }
    constants
}

/// Whether any statement in the unit assigns a whole-word `[cmd …]`
/// command substitution — the only shape the registry builtin-fold
/// lattice can improve, and therefore the cost gate for re-running SCCP
/// in [`constants_with_builtin_folds`].
fn has_cmd_subst_assignment(fu: &FunctionUnit) -> bool {
    fu.cfg.blocks.values().any(|block| {
        block.statements.iter().any(|stmt| {
            matches!(stmt, Statement::AssignValue { value, .. } if {
                let trimmed = value.trim();
                trimmed.starts_with('[') && trimmed.ends_with(']')
            })
        })
    })
}

/// `depth` is the nesting level of `script` — see
/// [`super::MAX_OPTIMISER_WALK_DEPTH`].
fn walk_script(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    script: &Script,
    constants: &std::collections::HashMap<String, String>,
    numeric: NumericCtx<'_>,
    namespace: &str,
    depth: u32,
) {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        walk_statement(ctx, cu, stmt, constants, numeric, namespace, depth);
    }
}

fn walk_statement(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    stmt: &Statement,
    constants: &std::collections::HashMap<String, String>,
    numeric: NumericCtx<'_>,
    namespace: &str,
    depth: u32,
) {
    match stmt {
        Statement::Call {
            span,
            command,
            args,
            tokens,
            ..
        } => {
            if let Some(t) = tokens {
                visit_call_tokens(ctx, t, constants);
                visit_call_cmd_subst_folds(ctx, cu, t, constants, namespace);
            }
            try_fold_static_proc_call(ctx, cu, *span, command, args, namespace);
        }
        // `set TARGET [cmd-sub]` lowers to `AssignValue` carrying the
        // full command's tokens (`["set", TARGET, "[cmd-sub]"]`). Walk
        // its value words for the value-position cmd-sub folds — O115
        // redundant-nested-expr collapse and the O103 pure-proc
        // constant-return fold — so a `set` target gets the same folds a
        // command-argument position already gets. Only the cmd-sub fold
        // path is wired (not `visit_call_tokens`): a bare `set y $c` RHS
        // is handled by SCCP / load-forwarding, and folding it here would
        // change behaviour beyond the documented gap.
        //
        // Note: the *canonical* `set x [expr {[expr {E}]}]` does not reach
        // this arm — `extract_single_expr_arg` strips the outer `expr` at
        // lowering time, producing an `AssignExpr` whose `ExprCommand`
        // holds the inner `[expr {E}]`. So O115 fires here only for the
        // rarer AssignValue forms that retain a nested-expr cmd-sub word;
        // the O103 pure-proc fold is the common reachable case.
        Statement::AssignValue {
            tokens: Some(t), ..
        } => {
            visit_call_cmd_subst_folds(ctx, cu, t, constants, namespace);
        }
        Statement::Return {
            span,
            value,
            expr,
            braced,
            ..
        } => {
            try_fold_return_terminator(
                ctx,
                *span,
                value.as_deref(),
                expr.as_ref(),
                *braced,
                constants,
                &cu.ir_module.procedures,
            );
        }
        Statement::AssignExpr {
            span, name, expr, ..
        } => {
            try_substitute_assign_expr(
                ctx,
                *span,
                name,
                expr,
                constants,
                numeric,
                &cu.ir_module.procedures,
            );
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_script(ctx, cu, &c.body, constants, numeric, namespace, depth + 1);
            }
            if let Some(b) = else_body {
                walk_script(ctx, cu, b, constants, numeric, namespace, depth + 1);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(ctx, cu, init, constants, numeric, namespace, depth + 1);
            walk_script(ctx, cu, next, constants, numeric, namespace, depth + 1);
            walk_script(ctx, cu, body, constants, numeric, namespace, depth + 1);
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. } => {
            walk_script(ctx, cu, body, constants, numeric, namespace, depth + 1);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(ctx, cu, body, constants, numeric, namespace, depth + 1);
            for h in handlers {
                walk_script(ctx, cu, &h.body, constants, numeric, namespace, depth + 1);
            }
            if let Some(fb) = finally_body {
                walk_script(ctx, cu, fb, constants, numeric, namespace, depth + 1);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    walk_script(ctx, cu, b, constants, numeric, namespace, depth + 1);
                }
            }
            if let Some(b) = default_body {
                walk_script(ctx, cu, b, constants, numeric, namespace, depth + 1);
            }
        }
        _ => {}
    }
}

/// Evaluate a pure procedure with its parameters bound to the call's
/// constant arguments, returning the constant return value when every
/// reachable `return` agrees on it.
///
/// Seeds the parameters as version-0 lattice constants, re-runs SCCP over
/// the callee's CFG, then resolves a single constant from all reachable
/// `return` terminators. Used for the argument-sensitive O103 fold
/// (`[::math::add 2 4]` → `6`) that the summary's argument-independent
/// `constant_return` cannot express.
///
/// This re-runs SCCP fresh (not `callee.sccp`), so it must feed the same
/// registry + whole-module trace facts the original build did — otherwise
/// this specific re-run path would be independently trace-blind (the same
/// silent-miscompile class `run_function`/`run_load_forwarding` guard
/// against). `ctx.registry` / `ctx.ir_module` are `None` only in a bare
/// hand-built `PassContext` (some pass-level unit tests); default to an
/// empty/false fact then, matching `run_function`'s own
/// `ctx.ir_module`-absent fallback.
fn evaluate_proc_with_constants(
    ctx: &PassContext<'_>,
    callee: &FunctionUnit,
    params: &[String],
    args: &[ConstValue],
    policy: FoldPolicy,
) -> Option<ConstValue> {
    let seed = seed_params_from_args(params, args)?;
    let registry: &CommandRegistry = ctx
        .registry
        .unwrap_or_else(|| tcl_registry::cache::default_registry());
    let empty_traced = std::collections::BTreeSet::new();
    let (traced_variables, has_dynamic_variable_trace) = match ctx.ir_module {
        Some(m) => (&m.traced_variables, m.has_dynamic_variable_trace),
        None => (&empty_traced, false),
    };
    let result = crate::sccp::sccp(
        &callee.cfg,
        &callee.ssa,
        Some(&seed),
        policy,
        crate::sccp::TraceInputs {
            registry,
            traced_variables,
            has_dynamic_variable_trace,
        },
    );
    resolve_return_constant(callee, &result, policy)
}

/// Bind each of `params` to its constant call argument for the
/// interprocedural SCCP seed. The seed keys on the parameter *name* (a
/// stable, cache-safe identity); `sccp` resolves each to the callee
/// build's interned symbol.
///
/// A trailing `args` parameter is variadic: Tcl collects every argument
/// beyond the fixed ones into a single list value bound to `args`, so this
/// seeds it as one canonical list-quoted [`ConstValue::String`] rather than
/// requiring (and silently mis-seeding on) an exact `params.len() ==
/// args.len()` — the earlier exact-length gate happened to be sound only
/// because [`parse_static_call_args`] never supplied more than one trailing
/// literal, an unstated coincidence rather than a modelled invariant.
/// `None` when the call doesn't supply enough arguments for the fixed
/// (non-`args`) parameters.
fn seed_params_from_args(
    params: &[String],
    args: &[ConstValue],
) -> Option<std::collections::HashMap<(String, crate::ssa::Version), LatticeValue>> {
    let is_variadic = params.last().is_some_and(|p| p == "args");
    let fixed = if is_variadic {
        params.len() - 1
    } else {
        params.len()
    };
    if args.len() < fixed || (!is_variadic && args.len() != fixed) {
        return None;
    }
    let mut seed: std::collections::HashMap<(String, crate::ssa::Version), LatticeValue> =
        std::collections::HashMap::new();
    for (p, a) in params.iter().take(fixed).zip(args.iter()) {
        seed.insert((p.clone(), 0), LatticeValue::Const(a.clone()));
    }
    if is_variadic {
        let tail: Vec<String> = args[fixed..].iter().map(const_value_text).collect();
        let list_text = tcl_syntax::list::join_list(tail);
        seed.insert(
            (params[fixed].clone(), 0),
            LatticeValue::Const(ConstValue::String(list_text)),
        );
    }
    Some(seed)
}

/// Render a [`ConstValue`] back to its plain text form — the inverse of
/// [`crate::sccp::parse_literal_value`], used to re-serialise a call
/// argument's already-typed constant into one element of the canonical
/// list text [`seed_params_from_args`] builds for a variadic `args`
/// parameter.
fn const_value_text(cv: &ConstValue) -> String {
    match cv {
        ConstValue::Int(i) => i.to_string(),
        ConstValue::Float(f) => f.to_string(),
        ConstValue::Bool(b) => i64::from(*b).to_string(),
        ConstValue::String(s) => s.clone(),
    }
}

/// Resolve the constant return value of `fu` under a computed SCCP `result`.
///
/// Every reachable *exit* must fold to the **same** constant — an explicit
/// `return` terminator, **or** a reachable fall-through to the function's
/// implicit exit (a block with no terminator: Tcl's "the result of the last
/// command executed" rule for a proc that runs off the end of its body
/// without a `return` on that path). Ignoring the fall-through case here
/// used to let a proc with `if {…} { return K }` plus a trailing
/// unconditional statement fold to `K` even when the fall-through path was
/// *also* reachable and produced a different value — a miscompile, not
/// just a missed optimisation (confirmed against tclsh 9.0.4: a proc whose
/// `if` condition itself isn't foldable, e.g. it depends on another call's
/// result, leaves both the `return` and the fall-through paths executable
/// under SCCP). A void return, an unfoldable return/tail, or disagreeing
/// exits — return-vs-return **or** return-vs-fall-through — yield `None`.
fn resolve_return_constant(
    fu: &FunctionUnit,
    result: &crate::sccp::SccpResult,
    policy: FoldPolicy,
) -> Option<ConstValue> {
    use crate::cfg::Terminator;
    let preds = fu.cfg.predecessors();
    let mut found: Option<ConstValue> = None;
    for (bn, block) in &fu.cfg.blocks {
        if !result.executable_blocks.contains(bn) {
            continue;
        }
        let folded = match &block.terminator {
            Some(Terminator::Return { value, expr, .. }) => {
                fold_return_under_lattice(fu, *bn, value.as_deref(), expr.as_ref(), result, policy)?
            }
            None => resolve_fallthrough_value(fu, *bn, result, &preds, policy)?,
            Some(_) => continue, // Goto / Branch — not an exit point
        };
        match &found {
            None => found = Some(folded),
            Some(prev) if *prev == folded => {}
            Some(_) => return None, // reachable exits disagree
        }
    }
    found
}

/// Resolve the value Tcl's implicit-return rule leaves behind when control
/// falls through block `bn` — the function's synthesised exit sink (a
/// reachable block with no terminator).
///
/// Trusts ONLY the narrow, unambiguous shape: `bn` has exactly one
/// executable predecessor, and that predecessor's OWN last statement is a
/// recognised value-producing tail (see [`fold_tail_statement_under_lattice`]).
/// Deliberately does NOT walk through an empty predecessor to whatever
/// precedes *it*: an empty block reached via a control-flow edge is not
/// "no Tcl command ran here" — it is frequently the empty **body** of a
/// real command (`if {$c} {}`, or the implicit `""` an `if` with no
/// `else` produces when the condition is false), whose own result is the
/// empty string, not whatever ran before the branch. Block shape alone
/// can't soundly distinguish that from a genuine structural join, so any
/// empty predecessor — or more than one live predecessor at all — bails
/// to `None` rather than risk inheriting a stale prior value. (A more
/// permissive, recursive version of this function shipped briefly and
/// mis-folded `proc f {c} { set x 1; if {$c} {} }`'s `[f 0]` to `1`
/// instead of the correct `""` — confirmed against tclsh 9.0.4 — by
/// walking straight through the empty `if`-body block back to the
/// preceding `set x 1`.)
fn resolve_fallthrough_value(
    fu: &FunctionUnit,
    bn: crate::cfg::BlockId,
    result: &crate::sccp::SccpResult,
    preds: &std::collections::HashMap<
        crate::cfg::BlockId,
        std::collections::HashSet<crate::cfg::BlockId>,
    >,
    policy: FoldPolicy,
) -> Option<ConstValue> {
    let mut executable_preds = preds
        .get(&bn)
        .into_iter()
        .flatten()
        .filter(|p| result.executable_blocks.contains(p));
    let pred = executable_preds.next()?;
    if executable_preds.next().is_some() {
        return None; // more than one live predecessor — ambiguous, bail
    }
    let block = fu.cfg.blocks.get(pred)?;
    let last = block.statements.last()?;
    fold_tail_statement_under_lattice(fu, *pred, last, result, policy)
}

/// Resolve the value Tcl's "result of the last executed command" rule
/// leaves behind when `stmt` is the last statement of a block that falls
/// through to the function's implicit exit — a trailing `set` / `incr`
/// implicitly returns exactly like `return $name` would (Tcl's `set` and
/// `incr` both return the value they just assigned), and a trailing bare
/// `expr` implicitly returns exactly like `return [expr {…}]` would.
/// `None` for any other statement shape (a bare command call whose own
/// result this analysis doesn't track, …) — the caller simply won't fold
/// that path, never mis-folds it.
fn fold_tail_statement_under_lattice(
    fu: &FunctionUnit,
    bn: crate::cfg::BlockId,
    stmt: &Statement,
    result: &crate::sccp::SccpResult,
    policy: FoldPolicy,
) -> Option<ConstValue> {
    match stmt {
        Statement::ExprEval { expr, .. } => fold_expr_under_lattice(fu, bn, expr, result, policy),
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::Incr { name, .. } => fold_var_ref_under_lattice(fu, bn, name, result),
        _ => None,
    }
}

/// Fold one `return` value/expr under the SCCP lattice for block `bn`.
/// Handles `return [expr {…}]` (evaluated under the block-exit env),
/// `return $var` (a lattice constant), and a bare literal return.
fn fold_return_under_lattice(
    fu: &FunctionUnit,
    bn: crate::cfg::BlockId,
    value: Option<&str>,
    expr: Option<&crate::expr_ast::ExprNode>,
    result: &crate::sccp::SccpResult,
    policy: FoldPolicy,
) -> Option<ConstValue> {
    let value = value?.trim();

    // Path 1 — bare literal return (no substitution metacharacters).
    if !value.contains(['$', '[', ']', '\\', '"', '{', '}']) {
        return Some(crate::sccp::parse_literal_value(value));
    }

    // Path 2 — a simple `$var` return.
    if let Some(name) = simple_var_ref(value) {
        return fold_var_ref_under_lattice(fu, bn, name, result);
    }

    // Path 3 — `return [expr {…}]`.
    fold_expr_under_lattice(fu, bn, expr?, result, policy)
}

/// Resolve a simple `$name` variable reference to its SCCP-proved constant
/// at block `bn`'s *exit* version — the value the variable holds
/// immediately after `bn`'s own statements have run. This MUST use the
/// exit version precisely: a loop-carried var (`return $total` after a
/// `foreach`) is a phi whose exit value is Overdefined, even though an
/// earlier `set total 0` left a stale Const(0) under another version.
/// Reading the precise version is what makes us bail on `sum_list` /
/// `fibonacci` instead of mis-folding to the pre-loop value.
///
/// Shared by [`fold_return_under_lattice`]'s Path 2 (`return $var`) and
/// [`fold_tail_statement_under_lattice`]'s fall-through case (a trailing
/// `set`/`incr` implicitly returns the value it just assigned — Tcl's
/// "result of the last command" rule makes it behave exactly like
/// `return $name`).
fn fold_var_ref_under_lattice(
    fu: &FunctionUnit,
    bn: crate::cfg::BlockId,
    name: &str,
    result: &crate::sccp::SccpResult,
) -> Option<ConstValue> {
    let sym = fu.ssa.var_symbol(name)?;
    let ver = fu
        .ssa
        .blocks
        .get(&bn)
        .and_then(|b| b.exit_versions.get(&sym).copied())
        .unwrap_or(0);
    match result.values.get(&(sym, ver)) {
        Some(LatticeValue::Const(c)) => Some(c.clone()),
        _ => None,
    }
}

/// Evaluate `expr` under the SCCP lattice for block `bn`'s exit
/// environment. Built FLOW-SENSITIVELY: bind each variable the expr
/// references at *this block's exit version* (the precise state reaching
/// this point), and only when that version is a lattice constant. A
/// variable absent from `exit_versions` (a never-reassigned parameter)
/// falls back to version 0, where interproc-seeded param constants live.
///
/// The flow-INsensitive alternative ("every Const lattice entry,
/// preferring the newest version, then overlay exit versions") miscompiled:
/// for `set x 0; foreach v {…} { set x $v }; return [expr {$x + 1}]`, `x`'s
/// exit version is a non-Const loop phi, so the overlay didn't override,
/// and the stale pre-loop `(x,1)=Const(0)` leaked in — folding to `1`
/// where tclsh returns `3`. Reading the exit version (Overdefined here)
/// leaves `x` unbound so `eval_tcl_expr` bails, matching
/// [`fold_var_ref_under_lattice`]'s `sum_list`/`fibonacci` precision.
///
/// Shared by [`fold_return_under_lattice`]'s Path 3 (`return [expr {…}]`)
/// and [`fold_tail_statement_under_lattice`]'s fall-through case (a
/// trailing bare `expr` implicitly returns its value).
fn fold_expr_under_lattice(
    fu: &FunctionUnit,
    bn: crate::cfg::BlockId,
    expr: &crate::expr_ast::ExprNode,
    result: &crate::sccp::SccpResult,
    policy: FoldPolicy,
) -> Option<ConstValue> {
    use crate::tcl_expr_eval::{Env, eval_tcl_expr_with_policy};

    let mut env: Env = Env::new();
    if let Some(ssa_block) = fu.ssa.blocks.get(&bn) {
        for name in crate::var_refs::vars_in_expr(expr) {
            let Some(sym) = fu.ssa.var_symbol(&name) else {
                continue;
            };
            let ver = ssa_block.exit_versions.get(&sym).copied().unwrap_or(0);
            if let Some(LatticeValue::Const(c)) = result.values.get(&(sym, ver)) {
                env.insert(fu.ssa.var_name(sym).to_owned(), const_to_env_value(c));
            }
        }
    }
    let v = eval_tcl_expr_with_policy(expr, &env, policy)?;
    Some(crate::sccp::tcl_value_to_const(v))
}

/// Convert a [`ConstValue`] to the expr-folder's [`EnvValue`].
fn const_to_env_value(c: &ConstValue) -> crate::tcl_expr_eval::EnvValue {
    use crate::tcl_expr_eval::EnvValue;
    match c {
        ConstValue::Int(i) => EnvValue::Int(*i),
        ConstValue::Float(f) => EnvValue::Float(*f),
        ConstValue::Bool(b) => EnvValue::Int(i64::from(*b)),
        ConstValue::String(s) => EnvValue::Str(s.clone()),
    }
}

/// Parse the static (constant) argument words of a `[proc arg…]` command
/// substitution body `inner`, given the number of leading head words to skip
/// (`1` for a direct call, `2` for `call proc …`). Each argument must be a
/// bare literal, a braced literal, a `$var` resolvable to a whole-function
/// constant via `constants`, or a nested command substitution that itself
/// folds to a literal; any other quoted / substituting word makes the whole
/// call non-static (returns `None`).
///
/// Shares [`literal_words`]'s proper Tcl-aware tokeniser rather than a naive
/// `split_whitespace` — the two used to duplicate this exact tokenising
/// logic, with this one *more conservatively* (and incorrectly, for a
/// braced multi-word argument) rejecting words `literal_words` already
/// folds soundly for the O129 builtin cmd-sub path.
fn parse_static_call_args(
    ctx: &PassContext<'_>,
    inner: &str,
    skip_words: usize,
    constants: &std::collections::HashMap<String, String>,
) -> Option<Vec<ConstValue>> {
    let registry = ctx.registry?;
    let words = literal_words(
        inner,
        constants,
        registry,
        &ctx.command_mutations,
        ctx.dialect,
        None,
    )?;
    Some(
        words
            .into_iter()
            .skip(skip_words)
            .map(|w| crate::sccp::parse_literal_value(&w))
            .collect(),
    )
}

/// Resolve a call's head word to a procedure qname that has an
/// interprocedural summary, following Tcl's real bareword command
/// resolution via [`crate::naming::bareword_resolution_candidates`] —
/// current namespace first, then global, exactly two levels, never every
/// enclosing ancestor namespace (Tcl's own command lookup does not walk
/// intermediate namespaces absent an explicit `namespace path`, which this
/// analysis does not model — a body using it is more conservatively left
/// unresolved here rather than risk folding to the wrong proc).
///
/// Shared with the analyser's identical same-file resolution chase
/// (`Analyser::resolve_indirect_call_target`) so the two can't diverge on
/// the same rule — a relative dotted word (`inner::p`) previously resolved
/// straight to `::inner::p` here, rooted at global, when real Tcl (and the
/// analyser side) tries the *current* namespace first
/// (`{namespace}::inner::p`); two procs of that shape in different
/// namespaces could fold a call to the wrong one's constant return.
///
/// Returns the first candidate that has a summary, else `None`.
fn resolve_proc_qname(
    command: &str,
    namespace: &str,
    ia: &crate::interprocedural::InterproceduralAnalysis,
) -> Option<String> {
    if command.is_empty() {
        return None;
    }
    crate::naming::resolve_command_with::<&str, _>(namespace, &[], command, |qname| {
        ia.procedures.contains_key(qname)
    })
}

/// O103: if `command` resolves to a proc with `can_fold_static_calls`
/// and a `constant_return`, emit a rewrite replacing the call
/// with the literal return value.
fn try_fold_static_proc_call(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    span: tcl_lexer::Span,
    command: &str,
    _args: &[String],
    namespace: &str,
) {
    use crate::interprocedural::ConstantReturn;

    let Some(ia) = cu.interproc.as_ref() else {
        return;
    };
    let Some(qname) = resolve_proc_qname(command, namespace, ia) else {
        return;
    };
    let Some(summary) = ia.procedures.get(&qname) else {
        return;
    };
    // A redefined proc has an ambiguous body — never fold its calls
    // (the flow-sensitive rename gate, mirroring the cmd-subst form and
    // `redefined_procedures` check).
    if cu.ir_module.redefined_procedures.contains(&qname) {
        return;
    }
    // Nor a proc whose bare name is later `rename`d over or `interp
    // alias`ed elsewhere in the module — folding this call site to the
    // *originally-declared* proc's constant return would miscompile a
    // script where `command` no longer denotes that proc by the time this
    // call runs (e.g. `proc a {…} …; proc b {…} …; rename b a` moves `b`'s
    // body onto the name `a`; a later `[a …]` call must not fold to `a`'s
    // original body).
    if !ctx.command_mutations.trusts_proc_binding(&qname) {
        return;
    }
    if !summary.can_fold_static_calls {
        return;
    }
    let Some(cr) = &summary.constant_return else {
        return;
    };
    let replacement = match cr {
        ConstantReturn::Int(i) => i.to_string(),
        ConstantReturn::Float(f) => f.to_string(),
        ConstantReturn::Bool(true) => "1".to_owned(),
        ConstantReturn::Bool(false) => "0".to_owned(),
        ConstantReturn::Str(s) => {
            if is_value_safe_bare_word(s) {
                s.clone()
            } else {
                return;
            }
        }
    };
    // The span here covers the whole Statement::Call — applying
    // the literal replacement would turn `::answer` into a
    // command named `42`, which is invalid Tcl. Targeting
    // `[procName …]` command substitutions with their token span
    // would avoid this. Until the Rust side tracks CMD-subst
    // spans at the call argument level, emit as a hint so
    // editors surface the fold without proposing an applicable
    // quick-fix.
    let mut opt = Optimisation::new(
        DiagCode::O103,
        format!(
            "Fold pure-proc call to '{}' to its constant return",
            summary.qualified_name
        ),
        span,
        replacement,
    );
    opt.hint_only = true;
    ctx.report(opt);
}

/// O100: rewrite `return $v` to `return K` when the SCCP
/// environment proves `v` is a constant. Works on the `value`
/// text since `Statement::Return::expr` is populated only when
/// the original source was `return [expr …]`.
///
/// Uses `O100` (the canonical constant-propagation code) rather
/// than the `O104` that earlier commits emitted — `O104` is
/// reserved by `docs/generated/optimisation_codes.md` for the
/// pattern-recognition string-build chain fold.
fn try_fold_return_terminator(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    value: Option<&str>,
    expr: Option<&crate::expr_ast::ExprNode>,
    _braced: bool,
    constants: &std::collections::HashMap<String, String>,
    procedures: &std::collections::HashMap<String, crate::ir::Procedure>,
) {
    use crate::naming::normalise_var_name;

    // O115: `return [expr {[expr {E}]}]` → `return [expr {E}]`. Checked
    // before the `expr.is_some()` early-return below because the return
    // value of a cmd-sub also populates `expr`, yet the redundant-nested-
    // expr collapse operates on the raw value text. Also requires `expr`
    // to be untouched anywhere in the module (mirrors the O129
    // builtin-fold trust check) — both the outer and inner `[expr {…}]`
    // are genuine command substitutions, and a shadowed `expr` no longer
    // has builtin semantics.
    if ctx.command_mutations.trusts("expr")
        && let Some(collapsed) = value.and_then(|raw| o115_redundant_nested_expr(raw.trim()))
    {
        ctx.report(Optimisation::new(
            DiagCode::O115,
            "Remove redundant nested expr",
            span,
            format!("return {collapsed}"),
        ));
        return;
    }

    // O101: a constant `[expr {…}]` return value folds to its value
    // (`return [expr {1 + 2}]` → `return 3`). Same trust requirement as
    // the O115 check above — a shadowed `expr` no longer has builtin
    // semantics and must not be folded as if it did.
    if let Some(inner) = value
        .map(str::trim)
        .and_then(|t| t.strip_prefix('[').and_then(|s| s.strip_suffix(']')))
        && ctx.command_mutations.trusts("expr")
    {
        let mut parts = inner.splitn(2, char::is_whitespace);
        if parts.next() == Some("expr") {
            let body = parts.next().unwrap_or("").trim();
            let body = body
                .strip_prefix('{')
                .and_then(|b| b.strip_suffix('}'))
                .unwrap_or(body);
            let body_node = crate::expr_parser::parse_expr(body, ctx.dialect);
            if !super::helpers::expr_simplify::expr_uses_shadowed_mathfunc(&body_node, procedures)
                && let Some(folded) =
                    super::helpers::expr_simplify::try_fold_expr(body, ctx.dialect)
                && !folded.contains(['$', '['])
            {
                ctx.report(Optimisation::new(
                    DiagCode::O101,
                    "Fold constant expression",
                    span,
                    format!("return {}", render_propagation_word(&folded)),
                ));
                return;
            }
        }
    }

    // Only fold `return $v` — numeric/bare literals and complex
    // values are left to richer passes.
    if expr.is_some() {
        return;
    }
    let Some(raw) = value else {
        return;
    };
    let v = raw.trim();
    let name = if let Some(n) = v.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        n.to_owned()
    } else if let Some(n) = v.strip_prefix('$') {
        n.to_owned()
    } else {
        return;
    };
    let normalised = normalise_var_name(&format!("${name}")).to_owned();
    let Some(resolved) = constants.get(&name).or_else(|| constants.get(&normalised)) else {
        return;
    };
    // Render the constant as a single self-contained word
    // rather than bailing on metacharacters (`return {Hello World}`).
    let word = render_propagation_word(resolved);
    ctx.report(Optimisation::new(
        DiagCode::O100,
        "Fold return of constant variable",
        span,
        format!("return {word}"),
    ));
}

/// O100 (`AssignExpr` form): substitute SCCP-proved constants
/// into ``set name [expr { … }]`` expressions.
///
/// Builds on ``substitute_expr_constants`` — the same helper the
/// branch-condition pass uses. Emits an O100 rewrite targeting
/// the whole ``set`` statement with the substituted expression
/// re-wrapped in ``[expr { … }]``. The rewrite span is extended
/// via ``full_rewrite_span`` so nested substitutions don't leave
/// orphan delimiters.
fn try_substitute_assign_expr(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    name: &str,
    expr: &crate::expr_ast::ExprNode,
    constants: &std::collections::HashMap<String, String>,
    numeric: NumericCtx<'_>,
    procedures: &std::collections::HashMap<String, crate::ir::Procedure>,
) {
    use super::helpers::expr_simplify::{
        expr_has_command_subst, expr_uses_shadowed_mathfunc, instcombine_expr_typed,
        substitute_expr_constants,
    };
    use super::helpers::spans::full_rewrite_span;
    use crate::expr_parser::parse_expr;
    use crate::tcl_expr_eval::{
        Env, eval_tcl_expr_with_octal_and_dialect, format_tcl_value, leading_zero_is_octal,
    };

    if matches!(expr, crate::expr_ast::ExprNode::Raw { .. }) {
        return;
    }
    if expr_has_command_subst(expr) {
        return;
    }
    // `expr` renamed/aliased anywhere in the module means the source's
    // `[expr {…}]` no longer has builtin semantics — do not propagate a
    // rewrite computed as if it did.
    if !ctx.command_mutations.trusts("expr") {
        return;
    }
    // A math-function call in the expression shadowed by a user-defined
    // `::tcl::mathfunc::<name>` proc means folding it would use builtin
    // semantics that no longer apply. Substitution doesn't change which
    // function names are called, so checking the pre-substitution AST is
    // sufficient.
    if expr_uses_shadowed_mathfunc(expr, procedures) {
        return;
    }
    let expr_text = crate::expr_ast::render_expr(expr);
    let result = substitute_expr_constants(&expr_text, constants, ctx.dialect);
    if !result.changed {
        return;
    }
    // After substitution, try to fold the result to a constant
    // — the cascade where O100 enables O101. When
    // the substituted expression is fully constant we can emit
    // the unwrapped ``set name VALUE`` form directly. Otherwise
    // keep the expression wrapper around the substituted text.
    let parsed = parse_expr(&result.text, ctx.dialect);
    let env = Env::new();
    let octal = ctx.dialect.and_then(leading_zero_is_octal);
    if let Some(val) = eval_tcl_expr_with_octal_and_dialect(&parsed, &env, octal, ctx.dialect) {
        let folded = format_tcl_value(&val);
        let needs_quoting = folded.is_empty()
            || folded.contains([
                ' ', '\t', '\n', '\r', '$', '[', ']', '{', '}', '"', '\\', '\0', ';',
            ]);
        if !needs_quoting {
            ctx.report(Optimisation::new(
                DiagCode::O100,
                "Propagate constant and fold",
                full_rewrite_span(ctx.source, span),
                format!("set {name} {folded}"),
            ));
            return;
        }
    }
    // Not fully constant; try one pass of instcombine on the
    // substituted expression to pick up identity simplifications
    // (e.g. ``$a + 0`` after ``$a`` folds to ``3``).
    let (simplified, _changed) = instcombine_expr_typed(&result.text, false, numeric);
    let final_text = if simplified.trim().is_empty() {
        result.text.clone()
    } else {
        simplified
    };
    ctx.report(Optimisation::new(
        DiagCode::O100,
        "Propagate constant into expr argument",
        full_rewrite_span(ctx.source, span),
        format!("set {name} [expr {{{final_text}}}]"),
    ));
}

/// O115 (value position): collapse a redundant double-`expr` command
/// substitution `[expr {[expr {E}]}]` to `[expr {E}]`, returning the
/// inner cmd-sub when *word* has that shape.
///
/// Sound because both forms evaluate `E` as an expression. The gate is
/// the double-unwrap: the outer `[expr {…}]`'s body must itself be an
/// `[expr {…}]` cmd-sub, so the collapsed form stays a valid value-
/// position word. A plain `[expr {$x + 1}]` (whose unwrap `$x + 1`
/// would be invalid as a bare value) — or an `[expr {[other]}]` whose
/// inner is not itself `expr` (not value-equivalent for non-numeric
/// results) — yields `None`.
fn o115_redundant_nested_expr(word: &str) -> Option<String> {
    let expr_arg = try_unwrap_expr_in_expr(word)?;
    try_unwrap_expr_in_expr(&expr_arg)?;
    Some(expr_arg)
}

/// O103 (CMD-subst form): walk each argv word looking for a
/// command substitution `[cmd …]` whose head resolves to a proc
/// with `can_fold_static_calls` and a proven `constant_return`.
/// Emits an applicable rewrite with the argv span and the literal
/// return value as replacement.
///
/// This is the "word-level" companion to the bare-call form in
/// [`try_fold_static_proc_call`], which stays hint-only because
/// folding `::answer` as a statement would turn the discarded
/// call into a bare `42` (invalid as a command name).
/// Fold a constant `[expr {…}]` command substitution in an argument position,
/// returning the rewritten word (`[expr {1 + 2}]` → `3`). A *braced* body folds
/// under the known `constants` (value substitution); a quoted / bare `expr "…"`
/// folds only as a literal (textual substitution is conservative). `None` when
/// the inner command is not `expr` or does not fold to a substitution-free
/// constant. Extracted from [`visit_call_cmd_subst_folds`].
fn try_o101_expr_arg_fold(
    ctx: &PassContext<'_>,
    inner: &str,
    constants: &std::collections::HashMap<String, String>,
    procedures: &std::collections::HashMap<String, crate::ir::Procedure>,
) -> Option<String> {
    let mut parts = inner.splitn(2, char::is_whitespace);
    if parts.next() != Some("expr") {
        return None;
    }
    // `expr` renamed/aliased anywhere in the module — a shadowed `expr` no
    // longer has builtin semantics, so this text no longer means what it
    // looks like. Mirrors the O129 builtin-fold gate (`try_o129_fold`).
    if !ctx.command_mutations.trusts("expr") {
        return None;
    }
    let raw_body = parts.next().unwrap_or("").trim();
    let body = raw_body
        .strip_prefix('{')
        .and_then(|b| b.strip_suffix('}'))
        .unwrap_or(raw_body);
    // A math-function call shadowed by a user-defined
    // `::tcl::mathfunc::<name>` proc anywhere in the module means folding
    // it would use builtin semantics that no longer apply.
    if super::helpers::expr_simplify::expr_uses_shadowed_mathfunc(
        &crate::expr_parser::parse_expr(body, ctx.dialect),
        procedures,
    ) {
        return None;
    }
    let folded =
        if let Some(braced_body) = raw_body.strip_prefix('{').and_then(|b| b.strip_suffix('}')) {
            super::helpers::expr_simplify::try_fold_expr_with_constants(
                braced_body,
                constants,
                true,
                ctx.dialect,
            )
        } else {
            super::helpers::expr_simplify::try_fold_expr(raw_body, ctx.dialect)
        };
    folded.filter(|f| !f.contains(['$', '[']))
}

fn visit_call_cmd_subst_folds(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    tokens: &CommandTokens,
    constants: &std::collections::HashMap<String, String>,
    namespace: &str,
) {
    for (i, argv_span) in tokens.argv.iter().enumerate() {
        let single = tokens.single_token_word.get(i).copied().unwrap_or(false);
        if !single {
            continue;
        }
        let Some(text) = tokens.argv_texts.get(i) else {
            continue;
        };
        let Some(inner) = text.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
            continue;
        };
        // O115: collapse a redundant double-`expr` cmd-sub in this
        // argument value position (needs no interproc summary). Requires
        // `expr` untouched anywhere in the module — both layers are
        // genuine command substitutions.
        if ctx.command_mutations.trusts("expr")
            && let Some(collapsed) = o115_redundant_nested_expr(text)
        {
            ctx.report(Optimisation::new(
                DiagCode::O115,
                "Remove redundant nested expr",
                full_word_span(ctx.source, *argv_span),
                collapsed,
            ));
            continue;
        }
        // O101: fold a constant `[expr {…}]` cmd-sub in this argument
        // position (`return [expr {1 + 2}]` → `return 3`). The general
        // `AssignExpr` / `ExprEval` expr folds don't reach a cmd-sub
        // embedded in a `Call` argument, so handle it here.
        if let Some(folded) =
            try_o101_expr_arg_fold(ctx, inner, constants, &cu.ir_module.procedures)
        {
            ctx.report(Optimisation::new(
                DiagCode::O101,
                "Fold constant expression",
                full_word_span(ctx.source, *argv_span),
                folded,
            ));
            continue;
        }
        // O129: fold a pure-builtin cmd-sub with constant (literal) args
        // through the registry `const_fold` callback (no interproc
        // needed). Checked before the O103 interproc bail so it fires
        // even when no interprocedural summary is available.
        if let Some(reg) = ctx.registry
            && let Some(folded) = try_o129_fold(
                reg,
                &ctx.command_mutations,
                constants,
                inner,
                ctx.dialect,
                None,
            )
        {
            // `list` / `lindex` keep their historical diagnostic codes
            // (O116 / O118) for editor granularity; everything else reports
            // the general O129.
            let (code, message) = match inner.split_whitespace().next() {
                Some("list") => (DiagCode::O116, "Fold constant list command"),
                Some("lindex") => (DiagCode::O118, "Fold constant lindex command"),
                _ => (DiagCode::O129, "Fold constant builtin command substitution"),
            };
            ctx.report(Optimisation::new(
                code,
                message,
                full_word_span(ctx.source, *argv_span),
                folded,
            ));
            continue;
        }
        // O103 (below) folds a pure-proc cmd-sub to its constant return.
        if let Some((qualified_name, replacement)) =
            try_o103_proc_fold(ctx, cu, inner, namespace, constants)
        {
            ctx.report(Optimisation::new(
                DiagCode::O103,
                format!("Fold pure-proc call to '{qualified_name}' to its constant return"),
                full_word_span(ctx.source, *argv_span),
                replacement,
            ));
        }
    }
}

/// Fold a pure-proc command substitution to its constant return (O103),
/// returning `(qualified_name, replacement_word)`. Uses the interprocedural
/// summary's argument-independent constant return when present, else re-runs
/// the pure callee under the call's constant arguments. `None` when the head
/// is not a foldable internal proc. Extracted from [`visit_call_cmd_subst_folds`].
fn try_o103_proc_fold(
    ctx: &PassContext<'_>,
    cu: &CompilationUnit,
    inner: &str,
    namespace: &str,
    constants: &std::collections::HashMap<String, String>,
) -> Option<(String, String)> {
    use crate::interprocedural::ConstantReturn;

    let ia = cu.interproc.as_ref()?;
    let head = parse_cmd_subst_head(inner)?;
    let qname = resolve_proc_qname(head, namespace, ia)?;
    let summary = ia.procedures.get(&qname)?;
    // A redefined proc has an ambiguous body — never fold its calls.
    if cu.ir_module.redefined_procedures.contains(&qname) {
        return None;
    }
    // Nor a proc whose bare name is later `rename`d over or `interp
    // alias`ed elsewhere in the module — see the sibling gate in
    // `try_fold_static_proc_call` for the miscompile this prevents.
    if !ctx.command_mutations.trusts_proc_binding(&qname) {
        return None;
    }
    let render_const = |cv: &ConstValue| match cv {
        ConstValue::Int(i) => i.to_string(),
        ConstValue::Float(f) => f.to_string(),
        ConstValue::Bool(b) => i64::from(*b).to_string(),
        ConstValue::String(s) => render_propagation_word(s),
    };
    let replacement = if summary.can_fold_static_calls
        && let Some(cr) = &summary.constant_return
    {
        // Argument-independent constant return from the summary.
        match cr {
            ConstantReturn::Int(i) => i.to_string(),
            ConstantReturn::Float(f) => f.to_string(),
            ConstantReturn::Bool(true) => "1".to_owned(),
            ConstantReturn::Bool(false) => "0".to_owned(),
            // A multi-word string return folds too, list-quoted as a single
            // word via the canonical quoter (the cmd-sub is one argument word)
            // — `set msg {a b}; return $msg` in the callee does not block it.
            ConstantReturn::Str(s) => render_propagation_word(s),
        }
    } else if summary.pure
        && let Some(callee) = cu.procedures.get(&qname)
        && let Some(args) = parse_static_call_args(ctx, inner, 1, constants)
        && let Some(cv) = evaluate_proc_with_constants(
            ctx,
            callee,
            &summary.params,
            &args,
            crate::tcl_expr_eval::FoldPolicy::for_dialect(
                ctx.dialect
                    .and_then(crate::tcl_expr_eval::leading_zero_is_octal),
                ctx.dialect,
            ),
        )
    {
        // Argument-sensitive: re-run SCCP on the pure callee with the call's
        // constant arguments bound and fold the constant return.
        render_const(&cv)
    } else {
        return None;
    };
    Some((summary.qualified_name.clone(), replacement))
}

/// Parse the head word out of a CMD-subst interior. Returns
/// `None` when the head word is empty or contains metacharacters
/// that would change the parsed command name under substitution.
fn parse_cmd_subst_head(inner: &str) -> Option<&str> {
    let trimmed = inner.trim_start();
    let end = trimmed
        .find(|c: char| c.is_whitespace())
        .unwrap_or(trimmed.len());
    let head = &trimmed[..end];
    if head.is_empty() {
        return None;
    }
    if head.contains(['$', '[', '\\', '"', '{', '(', '}', ']']) {
        return None;
    }
    Some(head)
}

/// O129: fold a pure-builtin command substitution
/// `[cmd args…]` (or `[cmd sub args…]`) whose arguments are constant
/// literals, by consulting the registry `const_fold` callback for the
/// resolved command (or subcommand). Returns the rendered replacement
/// word, or `None` when the head isn't a const-foldable builtin, an
/// argument isn't a clean literal, or the fold itself declines.
///
/// The result is rendered through [`render_propagation_word`] so a
/// multi-word fold result (`string toupper {a b}` → `A B`) is emitted
/// as a single brace-quoted word.
///
/// Gated by `mutations.trusts(head)` — if the command was
/// renamed / redefined anywhere in the module, it is no longer its
/// original builtin and must not be folded with the builtin's semantics.
fn try_o129_fold(
    registry: &tcl_registry::CommandRegistry,
    mutations: &crate::command_binding::ModuleCommandMutations,
    constants: &std::collections::HashMap<String, String>,
    inner: &str,
    dialect: Option<&str>,
    oo: Option<&OoFrame>,
) -> Option<String> {
    let folded = fold_builtin_cmd_subst_raw(registry, mutations, constants, inner, dialect, oo)?;
    Some(render_propagation_word(&folded))
}

/// The shared core of the O129 fold, now delegated to the module-wide
/// engine [`crate::const_subst::ConstSubstCtx`] (issues #1132 / #1134): the
/// cmd-sub head resolves to its spec (or subcommand), all args must be
/// clean literals, and the registry fold runs via
/// [`tcl_registry::CommandSpec::run_const_fold`], returning the **raw**
/// result (no single-word quoting).  The `dialect` is forwarded to the
/// registry, which owns all the Tcl-version interpretation (a versioned fold
/// like `string is` / `format` / `scan` reads it; an invariant fold ignores
/// it).  [`try_o129_fold`] wraps this with [`render_propagation_word`] for
/// free-standing argument positions; the embedded-interpolation path splices
/// the raw result directly into the surrounding string.
///
/// The `constants` map is whole-function (a var is present only if every
/// tracked version agrees), so substituting an entry is sound without any
/// same-block reaching-version gating.
fn fold_builtin_cmd_subst_raw(
    registry: &tcl_registry::CommandRegistry,
    mutations: &crate::command_binding::ModuleCommandMutations,
    constants: &std::collections::HashMap<String, String>,
    inner: &str,
    dialect: Option<&str>,
    oo: Option<&OoFrame>,
) -> Option<String> {
    let trusts = |name: &str| mutations.trusts(name);
    let lookup = |name: &str| constants.get(name).cloned();
    crate::const_subst::ConstSubstCtx {
        registry,
        dialect,
        defining_class: oo.map(|f| f.defining_class.as_str()),
        trusts: &trusts,
        lookup_var: &lookup,
    }
    .fold_cmd_subst(inner)
}

/// Re-lex a command-substitution interior into its literal words for the
/// O129 const-fold — see
/// [`crate::const_subst::ConstSubstCtx::literal_words`] for the exact
/// contract (this is the same engine, parameterised with the optimiser's
/// constants map and whole-module trust oracle).
fn literal_words(
    inner: &str,
    constants: &std::collections::HashMap<String, String>,
    registry: &tcl_registry::CommandRegistry,
    mutations: &crate::command_binding::ModuleCommandMutations,
    dialect: Option<&str>,
    oo: Option<&OoFrame>,
) -> Option<Vec<String>> {
    let trusts = |name: &str| mutations.trusts(name);
    let lookup = |name: &str| constants.get(name).cloned();
    crate::const_subst::ConstSubstCtx {
        registry,
        dialect,
        defining_class: oo.map(|f| f.defining_class.as_str()),
        trusts: &trusts,
        lookup_var: &lookup,
    }
    .literal_words(inner)
}

fn visit_call_tokens(
    ctx: &mut PassContext<'_>,
    tokens: &CommandTokens,
    constants: &std::collections::HashMap<String, String>,
) {
    for (i, span) in tokens.argv.iter().enumerate() {
        let single = tokens.single_token_word.get(i).copied().unwrap_or(false);
        let Some(text) = tokens.argv_texts.get(i) else {
            continue;
        };
        // A braced `{…}` word (kind `Str`) is a literal / script body: Tcl
        // performs NO substitution inside braces, so neither the simple-`$var`
        // fold nor the `"…"` / `[cmd]` interpolation folds may touch it (else
        // `puts {$x}` is wrongly rewritten to `puts 42`, or a proc body like
        // `{ set d [dict create a 1] }` is spliced into a quoted string).
        if tokens.argv_kinds.get(i).copied() == Some(tcl_lexer::TokenType::Str) {
            continue;
        }
        if single {
            visit_simple_var_word(ctx, *span, text, constants);
        }
        // `"..."` interpolation substitution — works on both
        // single-token (quoted strings) and composite (mixed
        // text + var) words.
        visit_string_interpolation(ctx, *span, text, constants);
        // Fold a pure-builtin `[cmd …]` substitution
        // embedded *inside* an interpolation string.
        visit_string_interpolation_cmd_subs(ctx, *span, text, constants);
    }
}

/// True when `inside` is exactly one bracket-balanced `[…]` covering the
/// whole word (a free-standing command substitution), as opposed to a
/// sub embedded in surrounding interpolation text.
fn is_whole_word_cmd_subst(inside: &str) -> bool {
    let b = inside.as_bytes();
    if b.first() != Some(&b'[') {
        return false;
    }
    let mut depth = 0u32;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return i == b.len() - 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

/// Fold a pure-builtin command substitution
/// embedded *inside* an interpolation string, splicing the raw result
/// into the surrounding `"…"` (O129): `puts "v=[string length abc]"` →
/// `puts "v=5"`.
///
/// Each `[cmd …]` whose head resolves to a const-foldable builtin with
/// clean literal args is folded via [`fold_builtin_cmd_subst_raw`] and
/// the raw result spliced in.  Soundness guards: the result must not
/// reintroduce a substitution into the `"…"` context — a result
/// carrying `$`, `[`, `]`, `\`, or `"` is left unfolded.  `$var`
/// substitutions and any non-foldable `[cmd]` are kept verbatim; at
/// least one successful fold is required to emit.
fn visit_string_interpolation_cmd_subs(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    text: &str,
    constants: &std::collections::HashMap<String, String>,
) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let inside = text
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(text);
    if !inside.contains('[') {
        return;
    }
    // A whole-word `[…]` cmd-sub is already folded by
    // `visit_call_cmd_subst_folds`; this path only handles a sub *embedded* in
    // surrounding interpolation text (else we'd emit a duplicate O129).
    if is_whole_word_cmd_subst(inside) {
        return;
    }
    // Byte scan is UTF-8-safe: the structural bytes we match (`[`, `]`,
    // `\`, `"`) are all < 0x80, so they never occur inside a multi-byte
    // sequence.  Text runs are flushed as `&str` slices (char-boundary
    // safe) only when a fold actually happens; unfolded subs stay in the
    // trailing un-flushed region.
    let bytes = inside.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut last = 0; // start of the not-yet-flushed text
    let mut i = 0;
    let mut folded_any = false;
    while i < n {
        match bytes[i] {
            b'\\' => i += 2, // skip an escaped pair (so `\[` is not a sub)
            b'[' => {
                let start = i;
                let mut depth = 0u32;
                let mut j = i;
                let mut close = None;
                while j < n {
                    match bytes[j] {
                        b'\\' => j += 1,
                        b'[' => depth += 1,
                        b']' => {
                            depth -= 1;
                            if depth == 0 {
                                close = Some(j);
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                let Some(close) = close else {
                    break; // unbalanced — leave the rest in `[last..]`
                };
                let cmd = &inside[start + 1..close];
                if let Some(result) = fold_builtin_cmd_subst_raw(
                    registry,
                    &ctx.command_mutations,
                    constants,
                    cmd,
                    ctx.dialect,
                    None,
                ) {
                    // Reject a result that would re-introduce a
                    // substitution into the `"…"` context.
                    if !result
                        .bytes()
                        .any(|b| matches!(b, b'$' | b'[' | b']' | b'\\' | b'"'))
                    {
                        out.push_str(&inside[last..start]);
                        out.push_str(&result);
                        last = close + 1;
                        folded_any = true;
                    }
                }
                i = close + 1;
            }
            _ => i += 1,
        }
    }
    if !folded_any {
        return;
    }
    out.push_str(&inside[last..]);
    let rewrite_span = full_quoted_string_span(ctx.source, span);
    ctx.report(Optimisation::new(
        DiagCode::O129,
        "Fold constant builtin command substitution in interpolation",
        rewrite_span,
        format!("\"{out}\""),
    ));
}

fn visit_simple_var_word(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    text: &str,
    constants: &std::collections::HashMap<String, String>,
) {
    let Some(varname) = simple_var_ref(text) else {
        return;
    };
    let normalised = normalise_var_name(&format!("${varname}")).to_owned();
    let Some(value) = constants
        .get(&normalised)
        .or_else(|| constants.get(varname))
    else {
        return;
    };
    // Re-render a metacharacter-bearing constant as a
    // single self-contained word instead of bailing.
    let word = render_propagation_word(value);
    ctx.report(Optimisation::new(
        DiagCode::O100,
        "Propagate constant into command argument",
        span,
        word,
    ));
}

/// Inline SCCP-proved constants into a `"…$x…"` string arg.
///
/// Rewrites only when:
/// 1. The argument begins with `"` and ends with `"` (a
///    double-quoted word).
/// 2. Every `$name` / `${name}` reference inside resolves to a
///    constant whose string form is safe for `"…"`
///    interpolation — no `$`, `[`, `\`, or `"`.
///
/// Produces a single `O100` diagnostic spanning the whole arg
/// with the fully-substituted string text.
fn visit_string_interpolation(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    text: &str,
    constants: &std::collections::HashMap<String, String>,
) {
    // The lexer's argv_text strips outer `"…"` delimiters, so
    // we accept either form. Skip strings whose content is a
    // single bare `$var` — that path is already covered by
    // `visit_simple_var_word` with a more appropriate span.
    let inside = text
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(text);
    if !inside.contains('$') {
        return;
    }
    if simple_var_ref(inside).is_some() {
        return;
    }
    // A free-standing `[cmd …]` word is a command substitution, not a
    // `"…"` interpolation: wrapping it in quotes (below) and extending the
    // span to a non-existent close-quote both corrupt the output (e.g.
    // `puts [expr {$a + $b}]` → `puts "[expr {3 + 4}]"]`). Whole-word subs
    // are folded by `visit_call_cmd_subst_folds`; leave them alone here.
    if is_whole_word_cmd_subst(inside) {
        return;
    }
    let Some(rewritten) = substitute_dollar_refs(inside, constants) else {
        return;
    };
    if rewritten == inside {
        return;
    }
    // The argv span for a composite `"$a $b $c"` word holds only
    // the opening-quote token (e.g. `"$` at 5..7). Extend to the
    // matching close quote so the rewrite target covers the full
    // string — otherwise we leave trailing `$b $c"` garbage.
    let rewrite_span = full_quoted_string_span(ctx.source, span);
    ctx.report(Optimisation::new(
        DiagCode::O100,
        "Inline constant into string interpolation",
        rewrite_span,
        format!("\"{rewritten}\""),
    ));
}

/// Scan `text` for `$name` / `${name}` references and replace
/// each with the corresponding literal from `constants`, rejecting
/// any value that would re-introduce new substitutions. Returns
/// `None` when any `$` is seen whose name is not a known
/// constant (a partial substitution would be worse than no
/// substitution).
fn substitute_dollar_refs(
    text: &str,
    constants: &std::collections::HashMap<String, String>,
) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    // Command-substitution nesting depth. A `$var` inside a `"…[cmd $var]…"`
    // command substitution is a *command argument*, not literal string text:
    // its value is re-parsed into words, so a multi-word value (e.g. the list
    // `{a b c}`) would split one argument into several — a miscompile. Track
    // the depth so those occurrences are held to the single-word bar.
    let mut cmd_depth = 0u32;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            // Copy a two-char backslash-escape verbatim.
            if i + 1 < bytes.len() {
                out.push(bytes[i] as char);
                out.push(bytes[i + 1] as char);
                i += 2;
            } else {
                out.push('\\');
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'[' {
            cmd_depth += 1;
            out.push('[');
            i += 1;
            continue;
        }
        if bytes[i] == b']' {
            cmd_depth = cmd_depth.saturating_sub(1);
            out.push(']');
            i += 1;
            continue;
        }
        if bytes[i] != b'$' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        // `$` — parse the var name.
        i += 1;
        if i >= bytes.len() {
            return None;
        }
        let (name, new_i) = if bytes[i] == b'{' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end] != b'}' {
                end += 1;
            }
            if end >= bytes.len() {
                return None;
            }
            (
                std::str::from_utf8(&bytes[start..end]).ok()?.to_owned(),
                end + 1,
            )
        } else {
            let start = i;
            let mut end = start;
            while end < bytes.len() {
                let b = bytes[end];
                if b.is_ascii_alphanumeric() || b == b'_' {
                    end += 1;
                } else if b == b':' && end + 1 < bytes.len() && bytes[end + 1] == b':' {
                    end += 2;
                } else {
                    break;
                }
            }
            if start == end {
                return None;
            }
            (
                std::str::from_utf8(&bytes[start..end]).ok()?.to_owned(),
                end,
            )
        };
        let value = constants.get(&name)?;
        if value.contains(['$', '[', '\\', '"']) {
            return None;
        }
        // Inside a `[…]` command substitution the value becomes a command
        // word, so it must be a single self-contained word — a value with
        // whitespace (a list literal like `tran 1n 100n uic`) would split
        // into multiple arguments. Bail rather than propagate, leaving the
        // `$var` in place. Plain string-text
        // occurrences (`cmd_depth == 0`) inline the value verbatim as before.
        if cmd_depth > 0 && !is_value_safe_bare_word(value) {
            return None;
        }
        out.push_str(value);
        i = new_i;
    }
    Some(out)
}

/// Return the variable name inside a `$var` or `${var}` word, or
/// `None` if the text is not a simple var reference.
fn simple_var_ref(text: &str) -> Option<&str> {
    if let Some(rest) = text.strip_prefix("${") {
        return rest.strip_suffix('}').filter(|n| is_static_var_word(n));
    }
    text.strip_prefix('$').filter(|n| is_static_var_word(n))
}

/// Allow a literal to be inlined as a bare word if it is
/// decimal-integer-shaped or matches the safe-word grammar.
/// Rejects anything containing Tcl metacharacters that would
/// introduce new substitutions.
fn is_value_safe_bare_word(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if value.trim().parse::<i64>().is_ok() {
        return true;
    }
    is_safe_word(value)
}

/// Render a constant `value` as a single,
/// self-contained Tcl word for O100 propagation into a command-argument
/// or `return` value position.
///
/// A safe bare word (integer or `[A-Za-z0-9_./:+-]`-only identifier) is
/// emitted verbatim. Anything
/// else (whitespace, `$` / `[` / `;` / quotes, unbalanced braces, a
/// trailing backslash, …) is re-rendered through the canonical
/// `TclConvertElement`-style quoter [`crate::codegen::helpers::tcl_list_element`]
/// (bare / brace-quoted / backslash-escaped, verified against Tcl's
/// `list`), so `set msg {Hello World}; return $msg` collapses to
/// `return {Hello World}` instead of bailing. The quoter never fails —
/// it always yields a word that re-evaluates to the literal `value` —
/// so this is total.
fn render_propagation_word(value: &str) -> String {
    if is_value_safe_bare_word(value) {
        value.to_owned()
    } else {
        crate::codegen::helpers::tcl_list_element(value)
    }
}

pub(super) fn sccp_constants_for(fu: &FunctionUnit) -> std::collections::HashMap<String, String> {
    sccp_constants_from(&fu.sccp, &fu.ssa)
}

/// [`sccp_constants_for`]'s projection, over an explicitly-supplied lattice.
///
/// Split out so a caller that re-runs SCCP under a *wider* escaping set can
/// project the result the same way — see [`oo_method_constants`], which does
/// exactly that for a `TclOO` method body without disturbing the shared
/// [`FunctionUnit`] lattice other consumers read.
fn sccp_constants_from(
    sccp: &crate::sccp::SccpResult,
    ssa: &crate::ssa::SsaFunction,
) -> std::collections::HashMap<String, String> {
    use super::helpers::literals::format_constant;

    let mut per_var: std::collections::HashMap<crate::ssa::Symbol, Vec<&ConstValue>> =
        std::collections::HashMap::new();
    let mut dirty: std::collections::HashSet<crate::ssa::Symbol> = std::collections::HashSet::new();
    for ((sym, _ver), lv) in &sccp.values {
        if dirty.contains(sym) {
            continue;
        }
        if let LatticeValue::Const(cv) = lv {
            per_var.entry(*sym).or_default().push(cv);
        } else {
            dirty.insert(*sym);
            per_var.remove(sym);
        }
    }
    let mut out = std::collections::HashMap::new();
    for (sym, cvs) in per_var {
        let first = cvs[0];
        if !cvs.iter().all(|cv| *cv == first) {
            continue;
        }
        if let Some(text) = format_constant(first) {
            out.insert(ssa.var_name(sym).to_owned(), text);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::CommandRegistry;

    use crate::interprocedural::InterproceduralAnalysis;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn run_pass(source: &str) -> Vec<Optimisation> {
        let reg = registry();
        let cu = CompilationUnit::build_for(source, &reg, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        // Mirror `optimise_unit_raw`'s production wiring — `run_load_forwarding`
        // (O102) needs `ctx.registry` for its intervening-call purity scan, and
        // the trace/alias safety gates need `ctx.ir_module`. Without these two
        // fields set, a hand-built `PassContext` silently loses O102 coverage
        // (and O127's pre-existing trace-purity gate), which is exactly the bug
        // this test helper would otherwise mask.
        ctx.registry = Some(&reg);
        ctx.ir_module = Some(&cu.ir_module);
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    #[test]
    fn resolve_proc_qname_walks_namespace_chain() {
        // Two procs in `::ns`; a bare `inner` call from `::ns` resolves to
        // `::ns::inner`, not the (absent) root `::inner`.
        let module = crate::lowering::lower_to_ir(
            "namespace eval ::ns {\n proc inner {} { return 1 }\n proc outer {} { inner }\n}",
            &registry(),
        );
        let ia = crate::interprocedural::build_interprocedural_analysis(
            &module,
            &registry(),
            None,
            crate::interprocedural::ObjectTypeMap::none(),
            crate::head_identity::HeadIdentityMap::none(),
        );
        assert!(
            ia.procedures.contains_key("::ns::inner"),
            "expected ::ns::inner in IA, got {:?}",
            ia.procedures.keys().collect::<Vec<_>>(),
        );
        assert_eq!(
            resolve_proc_qname("inner", "::ns", &ia).as_deref(),
            Some("::ns::inner"),
            "bare call should resolve against the enclosing namespace",
        );
        // An absolute spelling is taken as-is.
        assert_eq!(
            resolve_proc_qname("::ns::inner", "::other", &ia).as_deref(),
            Some("::ns::inner"),
        );
        // A bare call with no matching namespace candidate fails to resolve.
        assert_eq!(resolve_proc_qname("missing", "::ns", &ia), None);
        // From the root namespace a bare `inner` does *not* reach `::ns::inner`.
        assert_eq!(resolve_proc_qname("inner", "::", &ia), None);
    }

    #[test]
    fn resolve_proc_qname_does_not_walk_ancestor_namespaces() {
        // Real Tcl bareword resolution is exactly two levels — current
        // namespace, then global — absent an explicit `namespace path`.
        // `::a::b::c::caller`'s body calling bare `foo` must NOT resolve to
        // a `::a::foo` defined in a *grandparent* namespace: real tclsh
        // raises "invalid command name" there, it does not silently walk up
        // to find `::a::foo`.
        let module = crate::lowering::lower_to_ir(
            "namespace eval ::a {\n proc foo {} { return 1 }\n namespace eval b {\n  namespace eval c {\n   proc caller {} { foo }\n  }\n }\n}",
            &registry(),
        );
        let ia = crate::interprocedural::build_interprocedural_analysis(
            &module,
            &registry(),
            None,
            crate::interprocedural::ObjectTypeMap::none(),
            crate::head_identity::HeadIdentityMap::none(),
        );
        assert!(
            ia.procedures.contains_key("::a::foo"),
            "expected ::a::foo in IA, got {:?}",
            ia.procedures.keys().collect::<Vec<_>>(),
        );
        assert_eq!(
            resolve_proc_qname("foo", "::a::b::c", &ia),
            None,
            "a grandparent-namespace proc must not resolve for a bare call",
        );
        // Control: the *direct* enclosing namespace still resolves.
        let module2 = crate::lowering::lower_to_ir(
            "namespace eval ::a::b::c {\n proc foo {} { return 1 }\n proc caller {} { foo }\n}",
            &registry(),
        );
        let ia2 = crate::interprocedural::build_interprocedural_analysis(
            &module2,
            &registry(),
            None,
            crate::interprocedural::ObjectTypeMap::none(),
            crate::head_identity::HeadIdentityMap::none(),
        );
        assert_eq!(
            resolve_proc_qname("foo", "::a::b::c", &ia2).as_deref(),
            Some("::a::b::c::foo"),
        );
    }

    #[test]
    fn resolve_proc_qname_relative_dotted_word_prefers_current_namespace() {
        // A relative *dotted* word (`ns2::inner`, containing `::` but not
        // starting with it) must resolve against the current namespace
        // first, not be rooted straight at global — the same rule
        // `bareword_resolution_candidates` documents (confirmed against
        // tclsh 9.0.4). Two procs of this shape exist here, one nested
        // under `::ns` and one at the root; a bare `ns2::inner` call from
        // inside `::ns` must reach the nearer one.
        let module = crate::lowering::lower_to_ir(
            "proc ns2::inner {} { return 1 }\n\
             namespace eval ::ns {\n proc ns2::inner {} { return 2 }\n proc caller {} { ns2::inner }\n}",
            &registry(),
        );
        let ia = crate::interprocedural::build_interprocedural_analysis(
            &module,
            &registry(),
            None,
            crate::interprocedural::ObjectTypeMap::none(),
            crate::head_identity::HeadIdentityMap::none(),
        );
        assert!(ia.procedures.contains_key("::ns2::inner"));
        assert!(ia.procedures.contains_key("::ns::ns2::inner"));
        assert_eq!(
            resolve_proc_qname("ns2::inner", "::ns", &ia).as_deref(),
            Some("::ns::ns2::inner"),
            "a relative dotted call must prefer the current-namespace proc \
             over the root one",
        );
        // Falls back to the root proc when there is no current-namespace
        // candidate.
        assert_eq!(
            resolve_proc_qname("ns2::inner", "::other", &ia).as_deref(),
            Some("::ns2::inner"),
        );
    }

    /// Run the whole optimiser (raw, unfiltered) so the registry is
    /// threaded — needed for the O127 store-to-load-forwarding pass,
    /// which bails without a registry.
    fn o127(source: &str) -> Vec<Optimisation> {
        crate::optimiser::optimise_raw(source, &registry(), Some("tcl8.6"))
            .into_iter()
            .filter(|o| o.code == DiagCode::O127)
            .collect()
    }

    #[test]
    fn o127_forwards_single_use_computed_assignment() {
        // `set x [llength $y]` is computed and used once → inline at the
        // use site + delete the store, both O127 in one group.
        let src = "proc p {y} {\n    set x [llength $y]\n    puts $x\n}\n";
        let opts = o127(src);
        assert_eq!(opts.len(), 2, "{opts:?}");
        // Both edits share a group id (all-or-nothing).
        assert_eq!(opts[0].group, opts[1].group);
        assert!(opts[0].group.is_some());
        // One edit inlines `[set x [llength $y]]`; the other deletes.
        assert!(
            opts.iter()
                .any(|o| o.replacement.contains("[set x [llength $y]]"))
        );
        assert!(opts.iter().any(|o| o.replacement.is_empty()));
    }

    #[test]
    fn o127_skips_intervening_side_effect() {
        // The intervening `puts` is impure → forwarding past it is
        // unsafe.
        let src = "proc p {y} {\n    set x [llength $y]\n    puts hi\n    puts $x\n}\n";
        assert!(o127(src).is_empty(), "{:?}", o127(src));
    }

    #[test]
    fn o127_skips_intervening_upframe() {
        // An intervening `uplevel` lowers to a `Statement::UpFrame`,
        // whose opaque body can reach into / out of this frame and
        // mutate any name the forwarded `[llength $y]` reads.  It must
        // suppress the forward exactly like a barrier — otherwise the
        // re-evaluated inline could compute a different value.
        let src =
            "proc p {y} {\n    set x [llength $y]\n    uplevel 1 {incr ::n}\n    puts $x\n}\n";
        assert!(o127(src).is_empty(), "{:?}", o127(src));
    }

    #[test]
    fn o127_skips_command_substitution_before_endpoint_use() {
        // Tcl substitutes words left-to-right. Moving the `llength` past the
        // preceding substitution would make it observe the emptied `y`.
        let src = "proc p {y} {\n    set x [llength $y]\n    puts [set y {}] $x\n}\n";
        assert!(o127(src).is_empty(), "{:?}", o127(src));
    }

    #[test]
    fn o127_skips_multiple_uses() {
        // Two uses of `$x` → not single-use, no forwarding.
        let src = "proc p {y} {\n    set x [llength $y]\n    puts $x\n    puts $x\n}\n";
        assert!(o127(src).is_empty(), "{:?}", o127(src));
    }

    #[test]
    fn o127_forwards_past_extra_terminator_use() {
        // `$x` has one operand use (`baz $x`) plus a terminator use
        // (`return $x`). Def-use records the terminator use, but it must not
        // block the forward — the forward still fires into the operand use
        // while preserving the assignment for the `return`. The inlined
        // `[set x …]` keeps `x` defined for the trailing `return`.
        let src = "proc p {y} {\n    set x [llength $y]\n    baz $x\n    return $x\n}\n";
        let opts = o127(src);
        assert_eq!(opts.len(), 2, "{opts:?}");
        assert!(
            opts.iter()
                .any(|o| o.replacement.contains("[set x [llength $y]]")),
            "{opts:?}"
        );
    }

    #[test]
    fn o127_skips_literal_assignment() {
        // A literal `set x 5` is the O102 path, not O127.
        let src = "proc p {} {\n    set x 5\n    puts $x\n}\n";
        assert!(o127(src).is_empty(), "{:?}", o127(src));
    }

    // internal helpers

    #[test]
    fn simple_var_ref_parses_bare_and_braced() {
        assert_eq!(simple_var_ref("$foo"), Some("foo"));
        assert_eq!(simple_var_ref("${bar}"), Some("bar"));
        assert!(simple_var_ref("$foo(idx)").is_none());
        assert!(simple_var_ref("plain").is_none());
    }

    #[test]
    fn safe_bare_word_accepts_int_and_safe_words() {
        assert!(is_value_safe_bare_word("42"));
        assert!(is_value_safe_bare_word("-7"));
        assert!(is_value_safe_bare_word("foo_bar"));
        assert!(!is_value_safe_bare_word(""));
        assert!(!is_value_safe_bare_word("has space"));
        assert!(!is_value_safe_bare_word("$dollar"));
    }

    // end-to-end tests

    #[test]
    fn constant_int_propagates_into_call_arg() {
        let opts = run_pass("set x 42\nputs $x");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O100 && o.replacement == "42"),
            "expected O100 propagating 42, got {opts:?}",
        );
    }

    #[test]
    fn unsafe_string_constant_is_not_propagated() {
        // Tcl metacharacters in the value → must not inline as
        // a bare word.
        let opts = run_pass("set x \"$other\"\nputs $x");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O100),
            "unsafe string should not be propagated, got {opts:?}",
        );
    }

    #[test]
    fn o100_multi_word_constant_renders_via_quoter() {
        // A constant containing whitespace or
        // metacharacters is re-rendered as a single self-contained word
        // via the canonical quoter instead of bailing.
        // `return` value position → `return {Hello World}`.
        let ret = run_pass("proc ::f {} { set msg {Hello World}\nreturn $msg }");
        assert!(
            ret.iter()
                .any(|o| o.code == DiagCode::O100 && o.replacement == "return {Hello World}"),
            "expected O100 `return {{Hello World}}`, got {ret:?}",
        );
        // Command-argument position → `puts {Hello World}`.
        let arg = run_pass("proc ::f {} { set msg {Hello World}\nputs $msg }");
        assert!(
            arg.iter()
                .any(|o| o.code == DiagCode::O100 && o.replacement == "{Hello World}"),
            "expected O100 `{{Hello World}}` arg, got {arg:?}",
        );
        // A `$`/`[`-bearing literal constant is brace-quoted (the value
        // is literal — braces suppress the would-be substitution).
        let meta = run_pass("proc ::f {} { set m {a$b}\nputs $m }");
        assert!(
            meta.iter()
                .any(|o| o.code == DiagCode::O100 && o.replacement == "{a$b}"),
            "expected O100 `{{a$b}}` arg, got {meta:?}",
        );
    }

    #[test]
    fn non_const_lattice_skipped() {
        // Two different writes to x → ConstSet or Overdefined,
        // not a single Const.
        let opts = run_pass("set x 1\nif {$cond} { set x 2 }\nputs $x");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O100),
            "non-const lattice should be skipped, got {opts:?}",
        );
    }

    #[test]
    fn braced_var_reference_also_propagated() {
        let opts = run_pass("set x 7\nputs ${x}");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O100 && o.replacement == "7"),
            "expected O100 for braced var ref, got {opts:?}",
        );
    }

    #[test]
    fn return_terminator_folds_constant_variable() {
        let opts = run_pass("proc ::f {} { set x 42; return $x }");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O100 && o.replacement.contains("42")),
            "expected O100 folding return $x to return 42, got {opts:?}",
        );
    }

    #[test]
    fn static_proc_call_folds_to_constant_return() {
        // ::answer returns 42 unconditionally and is pure — a
        // call to ::answer can be folded.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc ::answer {} { return 42 }\n::answer",
            &registry,
            false,
        )
        .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations.iter().any(|o| o.code == DiagCode::O103),
            "expected O103 static-proc fold, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn return_expr_does_not_fold_stale_pre_loop_constant() {
        // `x` is overwritten by the loop, so its exit version at the return is
        // a non-Const phi (Overdefined). `::f` is pure but has no
        // argument-independent constant return, so `[::f]` reaches the
        // argument-sensitive fold's `fold_return_under_lattice` Path 3. That
        // fold must NOT leak the pre-loop `set x 0`: tclsh returns 3 (x ends
        // at 2), so folding `[::f]` to 1 would be a miscompile.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc ::f {} { set x 0; foreach v {1 2} { set x $v }; return [expr {$x + 1}] }\nputs [::f]",
            &registry,
            false,
        )
        .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        ctx.registry = Some(&registry);
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations.iter().all(|o| o.code != DiagCode::O103),
            "loop-overwritten var must not fold from a stale pre-loop constant, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn return_expr_folds_argument_sensitive_constant() {
        // Control: the argument-sensitive Path 3 still folds when the return
        // expr genuinely depends only on the constant call arguments — the
        // documented `[::add 2 4]` → 6 case (params bound at version 0).
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc ::add {a b} { return [expr {$a + $b}] }\nputs [::add 2 4]",
            &registry,
            false,
        )
        .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        ctx.registry = Some(&registry);
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations
                .iter()
                .any(|o| o.code == DiagCode::O103 && o.replacement == "6"),
            "expected O103 folding [::add 2 4] to 6, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn string_interpolation_const_is_inlined() {
        // `set count 42` → Const(42) in SCCP → interpolation into
        // `"count is $count"` substitutes to `"count is 42"`.
        let opts = run_pass("set count 42\nputs \"count is $count\"");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O100 && o.replacement.contains("42")),
            "expected O100 inlining count into interpolation, got {opts:?}",
        );
    }

    #[test]
    fn braced_expr_cmd_sub_arg_folds_under_constants() {
        // `puts [expr {$a + $b}]` with a, b proven constant folds the braced
        // expr in the call-argument position to its value.
        let opts = run_pass("set a 3\nset b 4\nputs [expr {$a + $b}]");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O101 && o.replacement == "7"),
            "expected O101 folding [expr {{$a + $b}}] to 7, got {opts:?}",
        );
    }

    #[test]
    fn shadowed_mathfunc_cmd_sub_arg_not_folded() {
        // FP guard: `proc ::tcl::mathfunc::abs` shadows the builtin
        // everywhere in the module — the cmd-sub-argument O101 fold path
        // (try_o101_expr_arg_fold) must not fold `abs(-5)` using builtin
        // semantics.
        let opts = run_pass("proc ::tcl::mathfunc::abs {x} { return 999 }\nputs [expr {abs(-5)}]");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O101),
            "must not fold a shadowed math function in a cmd-sub argument: {opts:?}",
        );
    }

    #[test]
    fn shadowed_mathfunc_return_value_not_folded() {
        // FP guard: `return [expr {…}]` folding must also respect the
        // math-function shadow gate.
        use tcl_registry::CommandRegistry;
        let reg = CommandRegistry::build_default();
        let source =
            "proc ::tcl::mathfunc::abs {x} { return 999 }\nproc f {} { return [expr {abs(-5)}] }";
        let cu = CompilationUnit::build_for(source, &reg, false).with_interprocedural(&reg, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations.iter().all(|o| o.code != DiagCode::O101),
            "must not fold a shadowed math function in a return value: {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn quoted_expr_cmd_sub_arg_not_folded_under_constants() {
        // `puts [expr "$a + $b"]` is the quoted form — it uses textual
        // substitution, so we conservatively never fold it.
        let opts = run_pass("set a 3\nset b 4\nputs [expr \"$a + $b\"]");
        assert!(
            opts.iter()
                .all(|o| !(o.code == DiagCode::O101 && o.replacement == "7")),
            "quoted expr must not fold under constants, got {opts:?}",
        );
    }

    /// Like [`run_pass`] but populates `ctx.command_mutations` from the
    /// whole module, the way the real pipeline does — needed to exercise
    /// the `expr`-redefinition trust gate.
    fn run_pass_with_mutations(source: &str) -> Vec<Optimisation> {
        let reg = registry();
        let cu = CompilationUnit::build_for(source, &reg, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.command_mutations =
            crate::command_binding::scan_module_command_mutations(&cu.ir_module, &reg);
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    #[test]
    fn renamed_expr_cmd_sub_arg_not_folded() {
        // FP guard: once `expr` is renamed anywhere in the module,
        // `puts [expr {$a + $b}]` no longer calls the builtin evaluator.
        let opts = run_pass_with_mutations(
            "rename expr real_expr\nset a 3\nset b 4\nputs [expr {$a + $b}]",
        );
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O101),
            "must not fold a cmd-sub through a renamed expr: {opts:?}",
        );
    }

    #[test]
    fn renamed_expr_assign_expr_substitution_not_folded() {
        // FP guard: `set x [expr {$a + $b}]` (the AssignExpr form,
        // try_substitute_assign_expr) must also respect the trust gate.
        let opts = run_pass_with_mutations("rename expr real_expr\nset a 3\nset x [expr {$a + 1}]");
        assert!(
            opts.iter()
                .all(|o| !(o.code == DiagCode::O100 || o.code == DiagCode::O101)),
            "must not propagate/fold set x [expr {{…}}] through a renamed expr: {opts:?}",
        );
    }

    #[test]
    fn ordinary_expr_cmd_sub_arg_still_folds_under_mutation_scan() {
        // TN/control: an unrelated module-wide mutation scan (no rename of
        // `expr` itself) must not block the fold.
        let opts = run_pass_with_mutations("set a 3\nset b 4\nputs [expr {$a + $b}]");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O101 && o.replacement == "7"),
            "expected O101 fold under an unrelated mutation scan, got {opts:?}",
        );
    }

    #[test]
    fn renamed_expr_return_value_not_folded() {
        // FP guard: `return [expr {…}]` folding (try_fold_return_terminator)
        // must also respect the trust gate.
        use tcl_registry::CommandRegistry;
        let reg = CommandRegistry::build_default();
        let source = "rename expr real_expr\nproc f {} { return [expr {1 + 2}] }";
        let cu = CompilationUnit::build_for(source, &reg, false).with_interprocedural(&reg, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        ctx.command_mutations =
            crate::command_binding::scan_module_command_mutations(&cu.ir_module, &reg);
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations.iter().all(|o| o.code != DiagCode::O101),
            "must not fold a renamed expr in a return value: {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn string_interpolation_unknown_var_skipped() {
        // `$name` is not in the constants map → must not fire.
        let opts = run_pass("puts \"hello $name\"");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O100),
            "unknown var should not be inlined, got {opts:?}",
        );
    }

    #[test]
    fn substitute_dollar_refs_handles_braced_and_missing_vars() {
        let mut c = std::collections::HashMap::new();
        c.insert("x".into(), "42".into());
        assert_eq!(
            substitute_dollar_refs("a${x}b", &c).as_deref(),
            Some("a42b"),
        );
        // Missing var → None (cannot partially substitute).
        assert!(substitute_dollar_refs("$unknown", &c).is_none());
    }

    #[test]
    fn substitute_dollar_refs_guards_multiword_in_cmd_sub() {
        let mut c = std::collections::HashMap::new();
        c.insert("lst".into(), "a b c".into());
        c.insert("n".into(), "5".into());
        // A multi-word value in plain string text inlines verbatim (it stays
        // part of the one string word).
        assert_eq!(
            substitute_dollar_refs("x=$lst", &c).as_deref(),
            Some("x=a b c"),
        );
        // The SAME value inside a `[…]` command substitution would split one
        // argument into several — bail, keeping `$lst`.
        assert!(substitute_dollar_refs("r: [lsearch $lst x]", &c).is_none());
        // A single-word value inside a cmd-sub is still safe to inline.
        assert_eq!(
            substitute_dollar_refs("r: [expr $n + 1]", &c).as_deref(),
            Some("r: [expr 5 + 1]"),
        );
    }

    #[test]
    fn whole_word_cmd_sub_not_wrapped_as_string_interpolation() {
        // Regression: `puts [expr {$a + $b}]` is a free-standing command
        // substitution, not a `"…"` string. The interpolation path must not
        // wrap it in quotes / mis-span it (which produced the corrupt
        // `puts "[expr {3 + 4}]"]`). No O100 string-interpolation rewrite may
        // fire on a whole-word `[…]`.
        let opts = run_pass("set a 3\nset b 4\nputs [expr {$a + $b}]");
        assert!(
            opts.iter()
                .all(|o| o.replacement != "\"[expr {3 + 4}]\"" && !o.replacement.contains("\"]")),
            "whole-word cmd-sub must not be string-interpolated, got {opts:?}",
        );
    }

    #[test]
    fn list_constant_not_inlined_into_string_cmd_sub() {
        // Regression: `[lsearch -exact $tokens uic]` must NOT become
        // `[lsearch -exact tran 1n 100n uic uic]` — that splits the list into
        // separate args and errors at runtime (`bad option "tran"`).
        let opts =
            run_pass("set tokens {tran 1n 100n uic}\nputs \"r: [lsearch -exact $tokens uic]\"");
        assert!(
            opts.iter()
                .all(|o| !o.replacement.contains("tran 1n 100n uic uic")),
            "list literal must not be word-split into the cmd-sub, got {opts:?}",
        );
    }

    #[test]
    fn load_forwarding_fires_o102_for_single_reaching_def() {
        // `set n 7; puts $n` — single reaching def is literal 7
        // → emit O102 on the puts use site.
        let opts = run_pass("set n 7\nputs $n");
        assert!(
            opts.iter().any(|o| o.code == DiagCode::O102),
            "expected O102 load-forwarding, got {opts:?}",
        );
    }

    // Regression: a top-level `global` name reassigned by a proc call must
    // never be folded as if its "sole reaching def" were stable — confirmed
    // against tclsh 8.6/9.0 as a real miscompile before this guard existed
    // (`set tcl_precision 4; proc helper {} {global tcl_precision; set
    // tcl_precision 17}; helper; puts $tcl_precision` prints `17`, not the
    // `4` the optimiser used to propose).  See
    // `crate::var_observability::scan_module_global_names`.
    #[test]
    fn o102_does_not_forward_top_level_global_reassigned_by_callee() {
        let src = "set g 4\nproc helper {} { global g\nset g 17 }\nhelper\nputs $g";
        let opts = run_pass(src);
        assert!(
            opts.iter()
                .all(|o| !(matches!(o.code, DiagCode::O100 | DiagCode::O102)
                    && o.replacement == "4")),
            "must not fold the stale pre-call literal 4, got {opts:?}",
        );
    }

    // TN control: an ordinary proc that itself `global`-declares the name it
    // reassigns is *already* protected via SCCP/O100's per-function escaping
    // set — O102's independent def-use-chain path must apply the identical
    // guard rather than bypassing it (this fired incorrectly before O102
    // consulted `var_observability` at all).
    #[test]
    fn o102_does_not_forward_proc_local_global_reassigned_by_callee() {
        let src = "proc p {} {\n    global g\n    set g 4\n    helper\n    puts $g\n}\nproc helper {} { global g\nset g 17 }\n";
        let opts = run_pass(src);
        assert!(
            opts.iter()
                .all(|o| !(matches!(o.code, DiagCode::O100 | DiagCode::O102)
                    && o.replacement == "4")),
            "must not fold the stale pre-call literal 4, got {opts:?}",
        );
    }

    // Precision control: a top-level name that *no* procedure ever
    // `global`-declares must still fold — the whole-module scan must not
    // over-widen to every top-level variable.
    //
    // Note this now folds via O100, not O102: the intervening call to
    // `other` is a command O102 cannot prove pure (it has no interprocedural
    // purity or upvar-reach information — a callee can dynamically alias
    // any name in the caller's frame via `upvar` with no textual
    // `global`/`variable` declaration anywhere), so O102's own independent
    // same-block scan conservatively withholds. SCCP (O100) already proves
    // `safe_const` a genuine whole-function constant via its own escaping
    // set (which correctly found no alias/trace anywhere for it), so the
    // fold still happens end to end.
    #[test]
    fn o102_still_forwards_top_level_global_no_proc_touches() {
        let src = "set safe_const 42\nproc other {} { puts unrelated }\nother\nputs $safe_const";
        let opts = run_pass(src);
        assert!(
            opts.iter().any(|o| matches!(o.code, DiagCode::O100 | DiagCode::O102)
                && o.replacement == "42"),
            "expected an untouched top-level constant to still fold, got {opts:?}",
        );
    }

    // TN control: a plain proc-local variable (never `global`/`variable`
    // declared) is genuinely private to that call frame — O102 must still
    // forward it.
    #[test]
    fn o102_still_forwards_genuinely_local_proc_variable() {
        let src = "proc p {} {\n    set n 7\n    puts $n\n}\n";
        let opts = run_pass(src);
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O102 && o.replacement == "7"),
            "expected O102 to still fold a genuinely private proc-local var, got {opts:?}",
        );
    }

    // Regression: a literal-body `interp eval {}` (targeting the *current*
    // interpreter — Tcl's documented meaning of the empty-string path) can
    // reassign a variable in the calling scope exactly like an opaque call,
    // even though it carries no `global`/`variable`/`upvar`/`trace`
    // declaration for it. Confirmed against tclsh 8.6/9.0: `set n 5; interp
    // eval {} {set n 99}; puts $n` prints `99`, not the stale `5`.
    #[test]
    fn o102_does_not_forward_past_interp_eval_barrier() {
        let src = "set n 5\ninterp eval {} { set n 99 }\nputs $n\n";
        let opts = run_pass(src);
        assert!(
            opts.iter()
                .all(|o| !(o.code == DiagCode::O102 && o.replacement == "5")),
            "must not forward the stale literal 5 past `interp eval {{}}`, got {opts:?}",
        );
    }

    // Regression: a literal-body `uplevel #0 {…}` evaluates in the absolute
    // global frame — at top level that coincides with the calling scope, so
    // it can reassign a variable there too, with no alias declaration of its
    // own. Confirmed against tclsh 8.6/9.0: `set n 5; uplevel #0 {set n
    // 99}; puts $n` prints `99`.
    #[test]
    fn o102_does_not_forward_past_uplevel_hash0_upframe() {
        let src = "set n 5\nuplevel #0 { set n 99 }\nputs $n\n";
        let opts = run_pass(src);
        assert!(
            opts.iter()
                .all(|o| !(o.code == DiagCode::O102 && o.replacement == "5")),
            "must not forward the stale literal 5 past `uplevel #0`, got {opts:?}",
        );
    }

    #[test]
    fn o102_operand_span_fires_applicable_on_call_argv() {
        // `set n 7; puts $n` — the O102 emitted on the puts use
        // must target the `$n` argv span (not the whole Call) and
        // be applicable (not hint-only). The argv span lands on
        // the last 2 chars of the source: `$n`.
        let source = "set n 7\nputs $n";
        let opts = run_pass(source);
        let o102s: Vec<_> = opts.iter().filter(|o| o.code == DiagCode::O102).collect();
        assert!(
            !o102s.is_empty(),
            "expected at least one O102, got {opts:?}"
        );
        let found = o102s.iter().any(|o| {
            let start = o.span.start() as usize;
            let end = o.span.end() as usize;
            !o.hint_only
                && o.replacement == "7"
                && end <= source.len()
                && &source[start..end] == "$n"
        });
        assert!(
            found,
            "expected applicable O102 with argv span covering `$n`, got {o102s:?}"
        );
    }

    #[test]
    fn o102_operand_span_matches_braced_var_word() {
        // ${n} forms must also be recognised at the argv-text
        // level and produce an applicable rewrite.
        let source = "set n 7\nputs ${n}";
        let opts = run_pass(source);
        let o102s: Vec<_> = opts.iter().filter(|o| o.code == DiagCode::O102).collect();
        assert!(
            o102s.iter().any(|o| !o.hint_only
                && o.replacement == "7"
                && &source[o.span.start() as usize..o.span.end() as usize] == "${n}"),
            "expected applicable O102 over ${{n}}, got {o102s:?}",
        );
    }

    #[test]
    fn o102_falls_back_to_hint_only_when_var_is_inside_interpolation() {
        // `$n` appears only inside a double-quoted composite word
        // — no argv text matches the bare `$n` form, so the
        // rewrite stays hint-only (the O100 string-interpolation
        // path handles the actual inlining).
        let source = "set n 7\nputs \"n=$n\"";
        let opts = run_pass(source);
        let o102s: Vec<_> = opts.iter().filter(|o| o.code == DiagCode::O102).collect();
        assert!(!o102s.is_empty(), "expected O102 hint, got {opts:?}");
        assert!(
            o102s.iter().all(|o| o.hint_only),
            "expected all O102s to be hint-only for interpolated use, got {o102s:?}",
        );
    }

    #[test]
    fn o103_cmd_subst_fires_applicable_rewrite() {
        // `[::answer]` inside an argv → applicable O103 with the
        // argv span covering `[::answer]` and the constant return
        // as replacement.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let source = "proc ::answer {} { return 42 }\nputs [::answer]";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        run(&mut ctx, &cu);
        let o103s: Vec<_> = ctx
            .optimisations
            .iter()
            .filter(|o| o.code == DiagCode::O103)
            .collect();
        assert!(
            o103s.iter().any(|o| !o.hint_only
                && o.replacement == "42"
                && &source[o.span.start() as usize..o.span.end() as usize] == "[::answer]"),
            "expected applicable O103 spanning `[::answer]`, got {o103s:?}",
        );
    }

    #[test]
    fn o103_folds_multi_word_string_return_list_quoted() {
        // A pure proc returning a multi-word
        // string folds in a cmd-sub position, list-quoted as one word
        // (a bare `a b c` is not a safe single word).
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let source = "proc ::greet {} { return \"a b c\" }\nputs [::greet]";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        run(&mut ctx, &cu);
        let o103s: Vec<_> = ctx
            .optimisations
            .iter()
            .filter(|o| o.code == DiagCode::O103)
            .collect();
        assert!(
            o103s
                .iter()
                .any(|o| !o.hint_only && o.replacement == "{a b c}"),
            "expected applicable O103 folding [::greet] to {{a b c}}, got {o103s:?}",
        );
    }

    #[test]
    fn o103_cmd_subst_fires_on_set_value_target() {
        // The value-position cmd-sub folds must also
        // fire on a `set TARGET [cmd-sub]` target. `set` lowers to
        // `AssignValue`, which `walk_statement` did not visit for cmd-sub
        // folds, so `set y [::answer]` missed the O103 fold that the same
        // `[::answer]` gets in a command-argument position (`puts
        // [::answer]`). With the AssignValue arm wired, the set value
        // folds identically.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let source = "proc ::answer {} { return 42 }\nproc ::f {} { set y [::answer]\nputs $y }";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        run(&mut ctx, &cu);
        let o103s: Vec<_> = ctx
            .optimisations
            .iter()
            .filter(|o| o.code == DiagCode::O103)
            .collect();
        assert!(
            o103s.iter().any(|o| !o.hint_only
                && o.replacement == "42"
                && &source[o.span.start() as usize..o.span.end() as usize] == "[::answer]"),
            "expected applicable O103 spanning the `[::answer]` set value, got {o103s:?}",
        );
    }

    #[test]
    fn o103_bare_call_stays_hint_only() {
        // Top-level `::answer` — the call result is discarded, so
        // folding it as a statement would leave an invalid `42`
        // command. Diagnostic must stay hint-only.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc ::answer {} { return 42 }\n::answer",
            &registry,
            false,
        )
        .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        run(&mut ctx, &cu);
        let o103s: Vec<_> = ctx
            .optimisations
            .iter()
            .filter(|o| o.code == DiagCode::O103)
            .collect();
        assert!(!o103s.is_empty(), "expected at least one O103");
        assert!(
            o103s.iter().all(|o| o.hint_only),
            "bare-call form must stay hint-only, got {o103s:?}",
        );
    }

    #[test]
    fn o103_folds_arg_sensitive_passthrough_cmd_subst() {
        // `::not_const {x} { return $x }` has no argument-*independent*
        // constant return, but with the call's constant arg bound it folds:
        // `[::not_const 1]` → `1` (the passthrough folds). The applicable
        // rewrite fires only for the CMD-subst form; the bare-call form
        // stays hint-only.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc ::not_const {x} { return $x }\nputs [::not_const 1]",
            &registry,
            false,
        )
        .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        ctx.registry = Some(&registry);
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations
                .iter()
                .any(|o| o.code == DiagCode::O103 && o.replacement == "1" && !o.hint_only),
            "expected applicable O103 folding [::not_const 1] to 1, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn o103_folds_arg_sensitive_passthrough_with_braced_multiword_literal() {
        // Precision: `parse_static_call_args` now shares `literal_words`'s
        // proper Tcl-aware tokeniser instead of a naive `split_whitespace`,
        // so a braced multi-word call argument (`{a b}` — one clean literal
        // argument in real Tcl, previously misread as two whitespace-split
        // words and conservatively rejected) folds too.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc ::passthrough {x} { return $x }\nputs [::passthrough {a b}]",
            &registry,
            false,
        )
        .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        ctx.registry = Some(&registry);
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations
                .iter()
                .any(|o| o.code == DiagCode::O103 && o.replacement == "{a b}" && !o.hint_only),
            "expected applicable O103 folding [::passthrough {{a b}}] to {{a b}}, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn o103_folds_variadic_args_proc_returning_args_directly() {
        // TP: `args` is seeded as a canonical Tcl list (see
        // `seed_params_from_args`), not skipped outright — `proc ::foo {a
        // args} { return $args }` called as `[::foo 1 2]` binds `a=1`,
        // `args={2}` (a one-element list, which renders bare as `2` with
        // no braces needed), so `return $args` folds to `2`. Confirmed
        // against tclsh 8.6: `puts [::foo 1 2]` prints `2`.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc ::foo {a args} { return $args }\nputs [::foo 1 2]",
            &registry,
            false,
        )
        .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        ctx.registry = Some(&registry);
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations
                .iter()
                .any(|o| o.code == DiagCode::O103 && o.replacement == "2" && !o.hint_only),
            "expected applicable O103 folding [::foo 1 2] to 2 (args seeded as a list), got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn o103_does_not_fold_recursive_or_loop_proc() {
        // A proc whose return SCCP cannot determine under the bound args must
        // not fold — a loop-carried `return $total` is a phi (Overdefined at
        // exit), even though a pre-loop `set total 0` leaves a stale Const(0).
        // Mis-folding it would be a miscompile.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let src = "proc ::sum {ns} {\n  set total 0\n  foreach n $ns { set total [expr {$total + $n}] }\n  return $total\n}\nputs [::sum {1 2 3}]\n";
        let cu =
            CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        ctx.registry = Some(&registry);
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations
                .iter()
                .all(|o| o.code != DiagCode::O103 || o.hint_only),
            "loop-carried return must not fold, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn o103_folds_implicit_return_proc_cmd_subst() {
        // TP: the KCS O103 doc's own canonical example — a proc with NO
        // explicit `return`, whose result is Tcl's "value of the last
        // command executed" (a bare `expr {…}`) — must fold. Before the
        // fall-through fix, `resolve_return_constant` only looked at
        // `Terminator::Return` blocks, so a pure proc that relies entirely
        // on implicit return never folded at all (`[double 21]` stayed
        // unrewritten even though tclsh evaluates it to `42`).
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let src = "proc double {n} { expr {$n * 2} }\nset x [double 21]\n";
        let cu =
            CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        ctx.registry = Some(&registry);
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations
                .iter()
                .any(|o| o.code == DiagCode::O103 && o.replacement == "42" && !o.hint_only),
            "expected applicable O103 folding [double 21] to 42 (implicit return), got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn o103_folds_implicit_return_zero_arg_proc() {
        // TP: a zero-parameter implicit-return proc (`set` as the trailing
        // statement — Tcl's `set` returns the value it just assigned, so
        // this is exactly like `return $pi`) also folds via the CMD-subst
        // path.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let src = "proc pi {} { set p 3 }\nputs [pi]\n";
        let cu =
            CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        ctx.registry = Some(&registry);
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations
                .iter()
                .any(|o| o.code == DiagCode::O103 && o.replacement == "3" && !o.hint_only),
            "expected applicable O103 folding [pi] to 3 (implicit `set` return), got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn o103_does_not_fold_when_return_and_fallthrough_disagree() {
        // FN / miscompile-guard: `helper`'s result is opaque to intraprocedural
        // SCCP (it's a call, not folded interprocedurally), so `g`'s `if`
        // condition can't be resolved even after `n` is seeded to 5 — both the
        // `return 1` branch AND the fall-through `expr {99}` stay executable.
        // tclsh 9.0.4 ground truth: `g 5` calls `helper 5` (== 6), `6 > 100` is
        // false, so `g 5` falls through and returns `99` — NOT `1`. Before the
        // fall-through fix, `resolve_return_constant` silently ignored the
        // reachable non-`Return` exit and confidently (wrongly) folded to the
        // `return 1` branch's value — a real miscompile, not just an
        // over-eager fold.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let src = "proc helper {n} { expr {$n + 1} }\nproc g {n} {\n    if {[helper $n] > 100} {\n        return 1\n    }\n    expr {99}\n}\nputs [g 5]\n";
        let cu =
            CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations
                .iter()
                .all(|o| o.code != DiagCode::O103 || o.hint_only),
            "disagreeing return/fall-through exits must not produce an applicable fold, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn o103_does_not_fold_through_empty_if_body() {
        // FP guard / miscompile-guard (reported in code review): a trailing
        // `if {$c} {}` with an empty body (and no `else`) is itself a real
        // Tcl command whose result is `""` — whether the condition is true
        // (the empty body's own result) or false (no branch ran, no
        // `else`). tclsh 9.0.4 ground truth: `f 0` and `f 1` both return
        // `""`, NOT `1` from the preceding `set x 1`. An earlier, more
        // permissive fall-through resolver walked straight through the
        // empty `if`-body block back to `set x 1` and wrongly folded `[f
        // 0]` to `1` — treating "0 statements" as "no command ran here"
        // rather than "this command's own result is empty". The fold must
        // decline entirely rather than inherit a stale prior value.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let src = "proc f {c} { set x 1\nif {$c} {} }\nputs [f 0]\n";
        let cu =
            CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations
                .iter()
                .all(|o| o.code != DiagCode::O103 || o.hint_only),
            "empty if-body fall-through must not produce an applicable fold, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn o103_folds_variadic_args_proc_ignoring_trailing_args() {
        // TP: a proc with a trailing `args` parameter folds on its fixed
        // parameter when the call supplies MORE arguments than the proc has
        // fixed params — the extra args collect into `args`, unused by this
        // body. Before the `args` fix, `evaluate_proc_with_constants`
        // required `params.len() == args.len()` exactly, so a call
        // supplying more than one trailing argument (`[f 5 x y z]` for
        // `proc f {a args}`) never even reached the fold attempt.
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let src = "proc f {a args} { expr {$a * 2} }\nputs [f 5 x y z]\n";
        let cu =
            CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        ctx.registry = Some(&registry);
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations
                .iter()
                .any(|o| o.code == DiagCode::O103 && o.replacement == "10" && !o.hint_only),
            "expected applicable O103 folding [f 5 x y z] to 10, got {:?}",
            ctx.optimisations,
        );
    }

    /// Run the propagation pass with `ctx.command_mutations` populated —
    /// the piece `optimise_raw` (used by the O129 trust-gate test) never
    /// wires (it doesn't set `cu.interproc` either, so it can't drive
    /// O103 at all) and a bare `PassContext::new` + `run` leaves at its
    /// all-trusting `Default`. Mirrors the whole-module scan every real
    /// production entry point (`optimise_unit_raw`, `find_dead_stores`,
    /// `optimise_by_pass`) performs before running passes.
    fn run_pass_with_command_mutations(source: &str) -> Vec<Optimisation> {
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let mut ctx = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        ctx.registry = Some(&registry);
        ctx.command_mutations =
            crate::command_binding::scan_module_command_mutations(&cu.ir_module, &registry);
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    #[test]
    fn o103_does_not_fold_call_to_proc_renamed_over() {
        // FP guard / miscompile-guard: `rename triple double` moves the
        // `triple` proc's body onto the name `double`, so a later `[double
        // 21]` call actually runs `triple`'s body (21*3 == 63), NOT the
        // original `double` proc's body (21*2 == 42) that `resolve_proc_qname`
        // finds in the interprocedural summary. Folding to `42` here would be
        // a miscompile (tclsh 9.0.4 confirmed: `double 21` after this rename
        // returns `63`).
        let src = "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple double\nputs [double 21]\n";
        let opts = run_pass_with_command_mutations(src);
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O103 || o.hint_only),
            "call to a proc name later `rename`d over must not fold, got {opts:?}",
        );
    }

    #[test]
    fn o103_does_not_fold_call_to_proc_shadowed_by_interp_alias() {
        // FP guard: `interp alias {} answer {} other` shadows the `answer`
        // command with an alias to `other` — a later `[answer]` call runs
        // `other`'s body (99), not the original `answer` proc's body (42).
        let src = "proc answer {} { return 42 }\nproc other {} { return 99 }\ninterp alias {} answer {} other\nputs [answer]\n";
        let opts = run_pass_with_command_mutations(src);
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O103 || o.hint_only),
            "call to a proc shadowed by interp alias must not fold, got {opts:?}",
        );
    }

    #[test]
    fn o103_still_folds_unrelated_proc_despite_unrelated_rename_elsewhere() {
        // TN control: `trusts_proc_binding` must be per-name, not a blanket
        // "any rename anywhere disables all O103 folds" — an unrelated
        // rename of a *different* proc must not block folding calls to
        // procs it never touched.
        let src = "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple somethingElse\nputs [double 21]\n";
        let opts = run_pass_with_command_mutations(src);
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O103 && o.replacement == "42" && !o.hint_only),
            "unrelated proc must still fold despite an unrelated rename, got {opts:?}",
        );
    }

    #[test]
    fn o103_does_not_fold_call_to_proc_renamed_over_via_namespace_relative_rename() {
        // FP guard / miscompile-guard (reported in code review): `rename
        // triple double` inside `proc ::ns::doit` resolves both bare names
        // relative to `::ns` (the proc's own namespace) — it renames
        // `::ns::triple` onto `::ns::double`, NOT the (nonexistent)
        // top-level `::triple`/`::double`. tclsh 9.0.4 ground truth: after
        // `::ns::doit` runs, `::ns::double 21` executes `triple`'s body
        // (21*3 == 63), not `double`'s original body (21*2 == 42). An
        // earlier version of `collect_proc_rebindings` always rooted a bare
        // rename argument at the GLOBAL namespace (`::double`/`::triple`),
        // so it never marked `::ns::double`/`::ns::triple` as rebound and
        // O103 could fold `[::ns::double 21]` to the wrong `42`.
        let src = "namespace eval ::ns {\n    proc double {n} { expr {$n * 2} }\n    proc triple {n} { expr {$n * 3} }\n}\nproc ::ns::doit {} { rename triple double }\n::ns::doit\nputs [::ns::double 21]\n";
        let opts = run_pass_with_command_mutations(src);
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O103 || o.hint_only),
            "call to a proc renamed over via a namespace-relative rename must not fold, got {opts:?}",
        );
    }

    #[test]
    fn parse_cmd_subst_head_rejects_metachars() {
        assert_eq!(parse_cmd_subst_head("::answer"), Some("::answer"));
        assert_eq!(parse_cmd_subst_head("::answer 1 2"), Some("::answer"));
        assert_eq!(parse_cmd_subst_head("  ::answer"), Some("::answer"));
        assert_eq!(parse_cmd_subst_head(""), None);
        // Leading `$` / `[` / `{` / `"` → would re-substitute.
        assert_eq!(parse_cmd_subst_head("$cmd"), None);
        assert_eq!(parse_cmd_subst_head("[cmd]"), None);
    }

    #[test]
    fn o115_collapses_redundant_nested_expr_in_value_positions() {
        // A redundant double-`expr` cmd-sub
        // `[expr {[expr {E}]}]` collapses to `[expr {E}]` in command-arg
        // and return value positions (O115 otherwise fires only on a
        // standalone `expr` statement).
        let collapsed = |src: &str| -> Vec<String> {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .into_iter()
                .filter(|o| o.code == DiagCode::O115)
                .map(|o| o.replacement)
                .collect()
        };
        assert_eq!(
            collapsed("proc ::f {x} { puts [expr {[expr {$x * 2}]}] }"),
            vec!["[expr {$x * 2}]".to_string()],
            "command-arg position should collapse",
        );
        assert_eq!(
            collapsed("proc ::f {x} { return [expr {[expr {$x * 2}]}] }"),
            vec!["return [expr {$x * 2}]".to_string()],
            "return position should collapse",
        );
    }

    #[test]
    fn o115_value_position_is_sound() {
        // A single `[expr {$x + 1}]` must NOT collapse (its unwrap
        // `$x + 1` would be invalid as a bare value), and an
        // `[expr {[other]}]` whose inner is not itself `expr` is not
        // value-equivalent for non-numeric results.
        let has_o115 = |src: &str| -> bool {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .iter()
                .any(|o| o.code == DiagCode::O115)
        };
        assert!(!has_o115("proc ::f {x} { puts [expr {$x + 1}] }"));
        assert!(!has_o115("proc ::f {x} { return [expr {$x + 1}] }"));
        assert!(!has_o115("proc ::f {x} { puts [expr {[someproc]}] }"));
    }

    #[test]
    fn o129_folds_pure_builtin_cmd_subst() {
        // A pure-builtin cmd-sub with constant
        // (literal) args folds via the registry `const_fold` callback
        // (O129). `optimise_raw` sets `ctx.registry`, which the O129
        // path requires.
        let fold = |src: &str| -> Vec<String> {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .into_iter()
                .filter(|o| o.code == DiagCode::O129)
                .map(|o| o.replacement)
                .collect()
        };
        assert_eq!(fold("puts [string toupper foo]"), vec!["FOO".to_string()]);
        assert_eq!(fold("puts [string tolower BAR]"), vec!["bar".to_string()]);
        assert_eq!(fold("puts [string reverse abc]"), vec!["cba".to_string()]);
        // The headline O129 example.
        assert_eq!(fold("puts [string length abcde]"), vec!["5".to_string()]);
        // The cat / repeat / trim folds.
        assert_eq!(
            fold("puts [string cat foo bar]"),
            vec!["foobar".to_string()]
        );
        assert_eq!(
            fold("puts [string repeat ab 3]"),
            vec!["ababab".to_string()]
        );
        assert_eq!(fold("puts [string trim {  hi  }]"), vec!["hi".to_string()]);
        assert_eq!(
            fold("puts [string range abcde 1 3]"),
            vec!["bcd".to_string()]
        );
        assert_eq!(fold("puts [string index abc end]"), vec!["c".to_string()]);
        // List + dict folds.
        assert_eq!(fold("puts [llength {a b c}]"), vec!["3".to_string()]);
        // The cmd-sub replacement is one word, so a spaced result is
        // brace-quoted by `render_propagation_word`.
        assert_eq!(fold("puts [concat a b c]"), vec!["{a b c}".to_string()]);
        assert_eq!(fold("puts [join {a b c} -]"), vec!["a-b-c".to_string()]);
        // `lindex` / `list` keep their historical codes (O118 / O116) for
        // editor granularity, so filter by those rather than O129.
        let fold_code = |src: &str, code: &str| -> Vec<String> {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .into_iter()
                .filter(|o| o.code.as_str() == code)
                .map(|o| o.replacement)
                .collect()
        };
        assert_eq!(
            fold_code("puts [lindex {a b c} 1]", "O118"),
            vec!["b".to_string()]
        );
        assert_eq!(fold_code("set x [list]", "O116"), vec!["{}".to_string()]);
        assert_eq!(fold("puts [dict get {a 1 b 2} b]"), vec!["2".to_string()]);
        assert_eq!(
            fold("puts [lreverse {a b c}]"),
            vec!["{c b a}".to_string()],
            "list result is brace-quoted as one word",
        );
        assert_eq!(fold("puts [string map {a b} aaa]"), vec!["bbb".to_string()]);
        assert_eq!(fold("puts [subst hello]"), vec!["hello".to_string()]);
        // `subst` with a substitution must NOT fold (no upstream resolution).
        assert!(fold("puts [subst {$x}]").is_empty());
        // `string is` Tcl-faithful classes.
        assert_eq!(fold("puts [string is alpha abc]"), vec!["1".to_string()]);
        assert_eq!(fold("puts [string is lower abc1]"), vec!["0".to_string()]);
        assert_eq!(fold("puts [string is boolean yes]"), vec!["1".to_string()]);
        // `list` is a Tcl 8.5+ class (absent from 8.4's 18-class table) --
        // like `wideinteger` below, this optimiser run uses `dialect=None`,
        // so the version-aware folder bails rather than risk folding a call
        // that would raise "bad class" under an 8.4 profile.
        assert!(fold("puts [string is list {a b c}]").is_empty());
        // `format` %s / %d / %% subset.
        assert_eq!(fold("puts [format %d 42]"), vec!["42".to_string()]);
        assert_eq!(fold("puts [format {v=%s} hi]"), vec!["v=hi".to_string()]);
        // `format` flag / width / precision folds for
        // the decimal-integer + string conversions (dialect-invariant).
        assert_eq!(fold("puts [format %05d 7]"), vec!["00007".to_string()]);
        assert_eq!(fold("puts [format %.3d 5]"), vec!["005".to_string()]);
        // `%#d` stays unfolded (`0d5` on Tcl 9, `5` on 8.6 — divergent).
        assert!(fold("puts [format %#d 5]").is_empty());
        // `string is integer` / `double` fold over their
        // dialect-invariant subsets.
        assert_eq!(fold("puts [string is integer 42]"), vec!["1".to_string()]);
        assert_eq!(fold("puts [string is double 1.5]"), vec!["1".to_string()]);
        assert_eq!(fold("puts [string is double abc]"), vec!["0".to_string()]);
        // This optimiser run uses `dialect=None`, so the version-aware folder
        // bails on `wideinteger` (it raises on some supported versions, hence is
        // unsafe to fold without a known dialect).
        assert!(fold("puts [string is wideinteger 42]").is_empty());
        // A braced literal with a space → result rendered as one word.
        assert_eq!(
            fold("puts [string toupper {a b}]"),
            vec!["{A B}".to_string()],
        );
        // A `$var` arg is not a constant literal → no fold.
        assert!(fold("puts [string toupper $x]").is_empty());
        // The range form (`string toupper s first last`) does not fold.
        assert!(fold("puts [string toupper foo 0 0]").is_empty());
        // A non-builtin head (a user proc) is not an O129 candidate.
        assert!(fold("proc ::p {} { return 1 }\nputs [::p]").is_empty());
    }

    #[test]
    fn o129_concat_trailing_backslash_space_word_is_not_folded() {
        // `Tcl_ConcatObj`'s trailing-whitespace trim in the VM, when it lands
        // on a backslash, re-exposes one byte (`concat a {b\ } c` -> `a b\  c`,
        // two spaces; the exact util-4.3 shape `concat a {b\\   } c` ->
        // `a b\\  c`).  The registry `fold_concat` uses a simple
        // `" ".join(a.strip() …)` model and so does NOT replicate that
        // re-expose rule — it is unsound *in isolation* for a
        // trailing-backslash-whitespace word.  Soundness is upheld by the
        // optimiser's input gate: `literal_words` bails on ANY backslash-bearing
        // word (it does not decode escapes), so `fold_concat` never receives one.
        // Drop the `literal_words` backslash bail and this test
        // fails — the cmd-sub would fold to a wrong literal.
        let fold = |src: &str| -> Vec<String> {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .into_iter()
                .filter(|o| o.code == DiagCode::O129)
                .map(|o| o.replacement)
                .collect()
        };
        // Sanity: a backslash-free concat still folds (the path is live).
        assert_eq!(fold("puts [concat a b c]"), vec!["{a b c}".to_string()]);
        // A braced word carrying a backslash escape is left unfolded.
        assert!(
            fold("puts [concat a {b\\ } c]").is_empty(),
            "trailing backslash-space word must not fold (re-expose rule unmodelled)",
        );
        assert!(fold("puts [concat {b\\ }]").is_empty());
        // The exact util-4.3 shape (double backslash, trailing spaces).
        assert!(fold("puts [concat a {b\\\\   } c]").is_empty());
    }

    #[test]
    fn o129_resolves_constant_var_args_b2() {
        // A constant `$var` arg in a builtin
        // cmd-sub is resolved (whole-function-constant → sound) before
        // folding; a multi-word value stays a single argument.
        let fold = |src: &str| -> Vec<String> {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .into_iter()
                .filter(|o| o.code == DiagCode::O129)
                .map(|o| o.replacement)
                .collect()
        };
        assert_eq!(
            fold("set s abcde\nputs [string length $s]"),
            vec!["5".to_string()],
        );
        // A multi-word constant value is kept as one arg.
        assert_eq!(
            fold("set s {a b}\nputs [llength $s]"),
            vec!["2".to_string()],
        );
        // Combined: a resolved var inside an interpolation cmd-sub.
        assert_eq!(
            fold("set s abc\nputs \"len=[string length $s]\""),
            vec!["\"len=3\"".to_string()],
        );
        // A non-constant var does not resolve → no fold.
        assert!(fold("puts [string length $undefined]").is_empty());
    }

    #[test]
    fn o129_folds_embedded_cmd_subst_in_interpolation() {
        // A pure-builtin cmd-sub embedded inside
        // an interpolation string folds, splicing the raw result.
        let fold = |src: &str| -> Vec<String> {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .into_iter()
                .filter(|o| o.code == DiagCode::O129)
                .map(|o| o.replacement)
                .collect()
        };
        assert_eq!(
            fold("puts \"v=[string length abc]\""),
            vec!["\"v=3\"".to_string()],
        );
        assert_eq!(
            fold("puts \"n=[llength {a b c}] items\""),
            vec!["\"n=3 items\"".to_string()],
        );
        // A non-foldable embedded sub leaves the string unfolded.
        assert!(fold("puts \"x=[someproc]\"").is_empty());
        // Soundness guard: a result that would re-introduce a `$`
        // substitution into the `\"…\"` is not spliced.
        assert!(fold("puts \"v=[string cat {$} x]\"").is_empty());
    }

    #[test]
    fn o129_trust_gate_suppresses_fold_for_rebound_builtin() {
        // The builtin-fold trust gate. When the
        // module renames/redefines `string` anywhere, the whole-module
        // mutation scan distrusts it and O129 must not fold a `[string
        // …]` cmd-sub with the original builtin semantics.
        let fold = |src: &str| -> Vec<String> {
            crate::optimiser::optimise_raw(src, &registry(), None)
                .into_iter()
                .filter(|o| o.code == DiagCode::O129)
                .map(|o| o.replacement)
                .collect()
        };
        // Baseline: untouched `string` folds.
        assert_eq!(fold("puts [string toupper foo]"), vec!["FOO".to_string()]);
        // Rebound in a proc body → distrusted everywhere → no fold (the
        // scan over-approximates: any builtin a body may rebind is
        // untrusted, regardless of whether the proc is ever called).
        assert!(
            fold("proc clobber {} { rename string {} }\nputs [string toupper foo]").is_empty(),
            "rebound-in-proc-body builtin must not fold",
        );
        // Top-level rename before the call → also distrusted (the scan is
        // whole-module / flow-insensitive).
        assert!(
            fold("rename string {}\nputs [string toupper foo]").is_empty(),
            "renamed-away builtin must not fold",
        );
    }

    #[test]
    fn simple_var_ref_matches_recognises_forms() {
        assert!(simple_var_ref_matches("$x", "x"));
        assert!(simple_var_ref_matches("${x}", "x"));
        assert!(!simple_var_ref_matches("$y", "x"));
        assert!(!simple_var_ref_matches("plain", "x"));
        // Array subscript → not a simple ref.
        assert!(!simple_var_ref_matches("$x(0)", "x"));
    }

    #[test]
    fn run_passes_dispatches_propagation() {
        let cu = CompilationUnit::build_for("set x 9\nputs $x", &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::Propagation]);
        assert!(
            ctx.optimisations.iter().any(|o| o.code == DiagCode::O100),
            "expected O100 via run_passes, got {:?}",
            ctx.optimisations,
        );
    }
}
