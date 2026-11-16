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
use crate::interprocedural::ConstantReturn;
use crate::ir::{CommandTokens, Script, Statement};
use crate::tcl_expr_eval::FoldPolicy;
use tcl_core_types::DiagCode;
use tcl_registry::CommandRegistry;

use super::helpers::expr_simplify::{NumericCtx, operand_types};
use super::helpers::literals::is_safe_word;
use super::helpers::spans::{
    line_delete_span, original_quoted_word_rewrite_span, original_statement_rewrite_span,
};
use super::{Optimisation, PassContext};
use tcl_syntax::naming::whole_scalar_variable_reference_name;

mod declared_body;

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
    //
    // `scan_module_global_names` resolves the `global` alias grammar through
    // the registry, so it answers only for the dialect `cu` was lowered
    // under — and a *missing* name is the unsound direction here: it tells
    // the top level a name does not escape, licensing a fold of a value some
    // other procedure reassigns through `global`. So this deliberately has no
    // default-dialect fallback: with no registry the top-level body is left
    // un-propagated rather than propagated against a guess.
    //
    // Every production entry point does set `ctx.registry` —
    // `super::manager::build_pass_context` is the single chokepoint every
    // `optimise*` / `find_dead_stores` entry builds through — so this `None`
    // arm is reachable only from a hand-built `PassContext` in a pass-level
    // unit test, and a test that wants top-level coverage wires the registry
    // the way this pass's own `run_pass` helper does.
    let top_level_extra_escaping = ctx.registry.map(|registry| {
        crate::var_observability::scan_module_global_names(&cu.ir_module, registry)
    });
    let no_extra_escaping = std::collections::HashSet::new();
    if let Some(top_level_extra_escaping) = &top_level_extra_escaping {
        run_function(
            ctx,
            cu,
            &cu.top_level,
            &cu.ir_module.top_level,
            "::",
            top_level_extra_escaping,
        );
    }
    for (qname, proc) in cu.ir_module.original_source_procedure_bodies() {
        let Some(fu) = cu.function_for_body(Some(*qname)) else {
            continue;
        };
        // The call-site namespace for O103 chain resolution is the proc's
        // own namespace (`::ns::foo` resolves a bare `bar` against `::ns`).
        let Some(namespace) = proc
            .body
            .namespace_context
            .as_deref()
            .and_then(crate::command_binding::SourceNamespaceKey::advisory_key)
        else {
            continue;
        };

        run_function(ctx, cu, fu, &proc.body, &namespace, &no_extra_escaping);
    }
    declared_body::run(ctx, cu);
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
    if let (Some(registry), Some(top_level_extra_escaping)) =
        (ctx.registry, &top_level_extra_escaping)
    {
        let trace = crate::sccp::TraceInputs {
            registry,
            source_metadata_input:
                crate::registry_invocation::InvocationMetadataContext::for_module(
                    registry,
                    &cu.ir_module,
                )
                .and_then(
                    crate::registry_invocation::InvocationMetadataContext::source_analysis_input,
                ),
            traced_variables: &cu.ir_module.traced_variables,
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
            deferred_writes: &cu.ir_module.deferred_writes,
            analysis_context: None,
            existence: None,
        };
        run_load_forwarding(
            ctx,
            &cu.ir_module,
            &cu.top_level,
            top_level_extra_escaping,
            trace,
        );
        for fu in cu.procedures.values() {
            run_load_forwarding(ctx, &cu.ir_module, fu, &no_extra_escaping, trace);
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
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    traced: &std::collections::BTreeSet<String>,
    has_dynamic_trace: bool,
) -> bool {
    use super::helpers::expr_simplify::expr_has_command_subst;
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

fn synthetic_forwarding_use(
    fu: &FunctionUnit,
    use_site: &crate::def_use::UseSite,
    index: usize,
    cell_name: &crate::var_resolve::VariableCellKey,
) -> bool {
    fu.ssa
        .blocks
        .values()
        .find(|block| block.name == use_site.block)
        .and_then(|block| block.statements.get(index))
        .is_some_and(|statement| {
            fu.ssa
                .cell_symbol(cell_name)
                .is_some_and(|symbol| statement.may_defs.contains(&symbol))
        })
}

/// Whether the defined cell is reachable from outside the frame: a `global`,
/// `variable` or `upvar` link, or a trace. Another frame or a trace handler
/// may then write it between the store and the load (tclsh 8.6.18 and 9.0.4:
/// `proc f {} {global x; set x 1; puts $x}` prints `traced` once
/// `trace add variable ::x write {apply {args {set ::x traced}}}` is set), so
/// its value is never forwarded. Legacy SSA without point receipts fails
/// closed on an unknown answer as well.
fn compatibility_def_is_external(
    fu: &FunctionUnit,
    block: crate::cfg::BlockId,
    index: usize,
    symbol: crate::ssa::Symbol,
    names: &std::collections::HashSet<String>,
    trace: crate::sccp::TraceInputs<'_>,
) -> bool {
    let external = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index)
        .externally_mutable_by(
            symbol,
            names,
            trace.has_dynamic_variable_trace,
            trace.registry,
        );
    if fu.ssa.point_contexts.is_none() {
        external != Some(false)
    } else {
        external == Some(true) || cell_is_linked_outside_frame(fu, symbol)
    }
}

/// Whether a cell a procedure body defines lives outside that body's own
/// activation: the walk resolved the name through a `global`, `variable` or
/// `upvar` link to a namespace table or another frame. The file's own frame
/// has no activation, so this asks nothing of it.
fn cell_is_linked_outside_frame(fu: &FunctionUnit, symbol: crate::ssa::Symbol) -> bool {
    fu.body_id.is_some()
        && fu.ssa.cell_key(symbol).is_some_and(|key| {
            key.authored_spelling().is_none() && key.activation_identity().is_none()
        })
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
/// Production rewrites require the exact retained reference to name the same
/// physical binding and represented SSA contents version as the definition.
/// Unrepresented callee writes, retargeting and observers withdraw that proof.
/// Whole-function alias and barrier scans remain compatibility gates only for
/// SSA fixtures without point contexts.
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
///   fact, and the names a callback script writes,
///   [`crate::ir::Module::deferred_writes`]) via
///   [`crate::sccp::is_externally_mutable`] — the same guard SCCP
///   applies to its own lattice, so O102 (independent of the SCCP lattice)
///   stays consistent with it.
/// - [`has_intervening_barrier`] — a `Statement::Barrier`/`UpFrame`
///   (a literal-body `uplevel`/`interp eval`, which *can* reach into an
///   arbitrary frame) between def and use, checked both same-block
///   (precisely) and cross-block (conservatively).
fn run_load_forwarding(
    ctx: &mut PassContext<'_>,
    module: &crate::ir::Module,
    fu: &crate::compilation_unit::FunctionUnit,
    extra_escaping: &std::collections::HashSet<String>,
    trace: crate::sccp::TraceInputs<'_>,
) {
    use crate::def_use::{DefKind, UseKind};

    if fu.cfg.has_opaque_native_accesses() {
        return;
    }

    // Compatibility SSA lacks the physical reference proof used below.
    if fu.ssa.point_contexts.is_none() && fu.dynamic_barrier_blocks_value_motion() {
        return;
    }

    let Some((trace, escaping)) = load_forwarding_escaping(module, fu, extra_escaping, trace)
    else {
        return;
    };

    for chain in fu.def_use.chains.values() {
        if chain.definition.kind != DefKind::Statement {
            continue;
        }
        let cell_name = &chain.key.0;
        let Some(symbol) = fu.def_use.symbol(&fu.ssa, *cell_name) else {
            continue;
        };
        let Some(cell_name) = fu.ssa.cell_key(symbol) else {
            continue;
        };
        let (Some(block_id), Ok(idx)) = (
            fu.cfg.block_id(&chain.definition.block),
            usize::try_from(chain.definition.statement_index),
        ) else {
            continue;
        };
        let var_name = fu.ssa.var_name(symbol);
        if compatibility_def_is_external(fu, block_id, idx, symbol, &escaping, trace) {
            continue;
        }
        // A synthetic may-def (the element fan of a dynamic-key write, or a
        // base refresh) is not a real reaching definition — forwarding its
        // statement's value would forward the *other* element's literal.
        if fu.ssa.is_synthetic_def(
            &chain.definition.block,
            chain.definition.statement_index,
            cell_name,
        ) {
            continue;
        }
        // Find the defining statement — must be an AssignConst
        // with a literal value to forward. AssignValue without
        // substitutions could also work, but we're conservative
        // and restrict to the same shapes.
        let Some(block) = fu.cfg.block_by_name(&chain.definition.block) else {
            continue;
        };
        let Some(def_stmt) = block.statements.get(idx) else {
            continue;
        };
        // Which code claims the rewrite follows from *why* the value is known,
        // and the two contracts are different: O102 forwards a variable whose
        // single reaching definition is written out as a literal, O100 inlines
        // a constant SCCP proved. `set a 1; incr a` is the second — the
        // defining statement computes its value — so it is folded, not
        // forwarded.
        let Some((code, literal)) =
            forwarding_literal(fu, def_stmt, cell_name, chain.key.1, ctx.fold_policy())
        else {
            continue;
        };
        if !is_value_safe_bare_word(&literal) {
            continue;
        }
        let message = load_forward_message(code, var_name);
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
            if synthetic_forwarding_use(fu, use_site, use_idx, cell_name) {
                continue;
            }
            if fu.ssa.point_contexts.is_none()
                && has_intervening_barrier(
                    fu,
                    &chain.definition.block,
                    idx,
                    &use_site.block,
                    use_idx,
                )
            {
                continue;
            }
            let Some(use_block_id) = fu.cfg.block_id(&use_site.block) else {
                continue;
            };
            report_load_forward(
                ctx,
                fu,
                (use_block_id, use_idx),
                use_stmt,
                LoadForwardReport {
                    code,
                    symbol,
                    version: chain.key.1,
                    message: &message,
                    literal: &literal,
                },
            );
        }
    }
}

fn load_forward_message(code: DiagCode, var_name: &str) -> String {
    if code == DiagCode::O102 {
        format!("Forward literal load of '{var_name}' from its single reaching definition")
    } else {
        format!("Inline the constant value of '{var_name}' proved at this read")
    }
}

fn load_forwarding_escaping<'a>(
    module: &'a crate::ir::Module,
    fu: &'a FunctionUnit,
    extra_escaping: &std::collections::HashSet<String>,
    trace: crate::sccp::TraceInputs<'a>,
) -> Option<(
    crate::sccp::TraceInputs<'a>,
    std::collections::HashSet<String>,
)> {
    // Independent of the SCCP lattice (see the doc comment on the call
    // site in `run`), this pass must apply the same externally-mutable
    // guard SCCP applies to its own lattice — a `global`/`variable`/
    // `upvar`/`trace`-aliased name's "sole reaching def" is not actually
    // sole: some other call frame can reassign it between the def and a
    // later use.
    let metadata = fu.invocation_metadata_context_for_module(trace.registry, module)?;
    let trace = crate::sccp::TraceInputs {
        source_metadata_input: metadata.source_analysis_input(),
        ..trace
    };
    let mut escaping = crate::var_observability::analyse_var_observability_with_metadata_context(
        &fu.cfg,
        trace.registry,
        Some(metadata),
    )
    .escaping_var_names();
    escaping.extend(extra_escaping.iter().cloned());
    trace.extend_module_escaping(&mut escaping);

    Some((trace, escaping))
}

fn forwarding_literal(
    fu: &crate::compilation_unit::FunctionUnit,
    def_stmt: &Statement,
    cell_name: &crate::var_resolve::VariableCellKey,
    version: crate::ssa::Version,
    policy: FoldPolicy,
) -> Option<(DiagCode, String)> {
    Some(match def_stmt {
        Statement::AssignConst { value, .. } => (DiagCode::O102, value.clone()),
        Statement::AssignValue { value, .. }
            if !value.contains(['$', '[', '\\', '"']) && !value.is_empty() =>
        {
            (DiagCode::O102, value.clone())
        }
        // The name-keyed projection `sccp_constants_for` the other O100
        // forms read cannot answer here: it drops any variable whose
        // versions hold different constants, which is every variable that
        // is ever reassigned — precisely this shape. A def-use consumer
        // already knows which version it is inlining, so it can index the
        // lattice by `(symbol, version)` and get an answer where the
        // name-keyed map has none. Every guard above this point still
        // applies.
        _ => (
            DiagCode::O100,
            sccp_value_literal(fu, cell_name, version, policy)?,
        ),
    })
}

/// True when an opaque effect — a `Statement::Barrier` or `Statement::UpFrame`
/// (the CFG shape for a literal-body `uplevel`/`interp eval`) — could run
/// between `def_block[def_idx]` and `use_block[use_idx]`, making a "sole
/// reaching definition" forward unsound: `uplevel 1 {…}` / `uplevel #0 {…}`
/// evaluates its body in a *different* frame (the caller's, or the absolute
/// global one) and can reassign any name visible there, exactly like an
/// opaque call. Independent of [`crate::ssa::SsaSourceView::externally_mutable_by`] (which
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
        matches!(
            stmt,
            Statement::Barrier { .. } | Statement::NativeCall { .. } | Statement::UpFrame { .. }
        )
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
#[derive(Clone, Copy)]
struct LoadForwardReport<'a> {
    code: DiagCode,
    symbol: crate::ssa::Symbol,
    version: crate::ssa::Version,
    message: &'a str,
    literal: &'a str,
}

fn report_load_forward(
    ctx: &mut PassContext<'_>,
    fu: &crate::compilation_unit::FunctionUnit,
    site: (crate::cfg::BlockId, usize),
    use_stmt: &Statement,
    report: LoadForwardReport<'_>,
) {
    if fu.cfg.statement_source_edit_span(site.0, site.1).is_none() {
        return;
    }
    let LoadForwardReport {
        code,
        symbol,
        version,
        message,
        literal,
    } = report;
    let mut emitted_applicable = false;
    if let Statement::Call {
        tokens: Some(tokens),
        ..
    } = use_stmt
    {
        for (i, argv_span) in tokens.argv.iter().enumerate() {
            let Some(word @ crate::ir::WordExpr::Variable { spelling, .. }) = tokens.words().get(i)
            else {
                continue;
            };
            // A braced word (`{$x}`) is a literal — Tcl performs no
            // substitution inside braces — so `$x` there must not be
            // forwarded. `argv_texts` has the braces stripped, so the
            // word kind is the only signal.
            if tokens.argv_kinds.get(i) == Some(&tcl_lexer::TokenType::Str) {
                continue;
            }
            let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, site.0, site.1);
            let Some(read) = view.read_word(word) else {
                continue;
            };
            if whole_scalar_variable_reference_name(spelling, fu.source_lexer_config()).is_none()
                || read.symbol != symbol
                || read.version != Some(version)
            {
                continue;
            }
            ctx.report(Optimisation::new(
                code,
                message.to_owned(),
                tcl_lexer::word_span_at(ctx.source, fu.abs_span(*argv_span)),
                literal.to_owned(),
            ));
            emitted_applicable = true;
        }
    }
    if emitted_applicable {
        return;
    }
    let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, site.0, site.1);
    if !view.source_tokens().is_some_and(|tokens| {
        tokens.variable_accesses.iter().any(|access| {
            view.read_reference(&access.source, &access.original_spelling)
                .is_some_and(|read| read.symbol == symbol && read.version == Some(version))
        })
    }) {
        return;
    }
    // No operand word was found to target, so the span is the whole consuming
    // statement — and `literal` is not a valid replacement for it. Record the
    // hint with no replacement rather than a payload that would corrupt the
    // statement if anything ever applied it: `1` recorded over the whole of
    // `incr a` reads as a one-click rewrite to a bare `1` in command position
    // wherever a surface shows the payload without the flag. The appliers
    // filter `hint_only`; this keeps the data model in agreement with them.
    let mut opt = Optimisation::new(
        code,
        message.to_owned(),
        fu.abs_span(use_stmt.span()),
        String::new(),
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
    ctx: &'a PassContext<'a>,
    fu: &'a FunctionUnit,
    registry: &'a tcl_registry::CommandRegistry,
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
fn locate_use_var(
    tokens: &CommandTokens,
    source: crate::ssa::SsaSourceView<'_>,
    symbol: crate::ssa::Symbol,
    version: crate::ssa::Version,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Option<(tcl_lexer::Span, bool)> {
    use crate::ir::WordExpr;
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

/// Original declaring-class report, separate from method display metadata.
#[derive(Debug, Clone)]
struct OoFrame {
    defining_class: String,
}

fn oo_frame_for(
    method: &crate::ir::MethodDef,
    mutations: &crate::command_binding::ModuleCommandMutations,
    world: &crate::command_binding::OriginalCompletedCommandWorld,
) -> Option<OoFrame> {
    // Relative `self` can be shadowed in a receiver-selected namespace.
    // The natural `[self class]` spelling deliberately remains unchanged
    // until the original command target is proved for that method frame.
    if mutations.has_runtime_selected_frames() || method.kind == crate::ir::MethodKind::ClassMethod
    {
        return None;
    }
    let (defining_class, original_operand) = world.defining_class_name_for_body(method)?;
    if !mutations.trusts_proc_binding(&original_operand) {
        return None;
    }
    Some(OoFrame { defining_class })
}

/// The method-local constants a method body may propagate, or an empty map
/// when it may propagate none.
///
/// This applies `elimination.rs`'s escaping model to the propagation
/// lattice.  `elimination.rs` already knows that a `TclOO` instance variable
/// escapes the method frame — it feeds
/// [`crate::ir::MethodDef::instance_vars`] through the same channel iRules
/// cross-event state uses, so a state-mutating `set ivar …` is never deleted
/// as a dead store.  The propagation lattice needs the same model, or this
/// walk can only carry an empty constants map.
///
/// SCCP is therefore **re-run** for the method with those names in its
/// escaping set, rather than the shared [`FunctionUnit::sccp`] being rebuilt
/// that way: forcing them `Overdefined` also poisons everything *derived*
/// from them (`set a $ivar ; set b $a`), which no filter on a projected map
/// could do — but it is a deliberately propagation-only view.  The unit's own
/// lattice stays as built, because other consumers read facts an instance
/// variable legitimately carries (object-collection element typing harvests
/// `dict set pins $k [Pin new]` out of exactly such a name).
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
    body_id: &crate::ir::ProcedureId,
    frame: &OoFrame,
    allow_locals: bool,
) -> std::collections::HashMap<String, String> {
    if !allow_locals {
        return std::collections::HashMap::new();
    }
    let (Some(fu), Some(registry)) = (cu.methods.get(body_id), ctx.registry) else {
        return std::collections::HashMap::new();
    };
    if fu.complexity_guarded {
        return std::collections::HashMap::new();
    }
    // This re-run bypasses the unit build's dynamic-name widening of the
    // SCCP escaping switch, so apply the same abstention here: a computed
    // variable name in the body makes no method-local constant trustworthy
    if fu.dynamic_barrier_blocks_value_motion() {
        return std::collections::HashMap::new();
    }
    // The re-run also carries the registry builtin-fold context:
    // `set base [self class]; set ns [namespace qualifiers $base]`
    // folds to fixpoint *inside* the lattice, so `base` and `ns` both
    // project as method-local constants rather than the chain stopping
    // after the first O129 suggestion. The frame's defining class is
    // proven by [`oo_frame_for`], so the `[self class]` frame fact is
    // sound here and only here.
    //
    // The escaping set is the unit's own `method_facts` carrier — the same
    // struct the existence fold and the W-family diagnostics read — never a
    // second lookup of `MethodDef::instance_vars`.
    let Some(facts) = fu.method_facts.as_deref() else {
        return std::collections::HashMap::new();
    };
    let sccp = crate::sccp::sccp_in_module(&crate::sccp::SolveInputs {
        cfg: &fu.cfg,
        ssa: &fu.ssa,
        param_constants: None,
        policy: FoldPolicy::from_registry(registry),
        extra_escaping: &facts.instance_vars,
        trace: crate::sccp::TraceInputs {
            registry,
            source_metadata_input: fu
                .invocation_metadata_context_for_module(registry, &cu.ir_module)
                .and_then(
                    crate::registry_invocation::InvocationMetadataContext::source_analysis_input,
                ),
            traced_variables: &cu.ir_module.traced_variables,
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
            deferred_writes: &cu.ir_module.deferred_writes,
            analysis_context: None,
            existence: None,
        },
        folds: Some(crate::sccp::BuiltinFoldInputs {
            registry,
            source_metadata_input: fu
                .invocation_metadata_context_for_module(registry, &cu.ir_module)
                .and_then(
                    crate::registry_invocation::InvocationMetadataContext::source_analysis_input,
                ),
            mutations: &ctx.command_mutations,
            dialect: ctx.dialect,
            defining_class: Some(&frame.defining_class),
            registry_engine: true,
            trust: crate::sccp::FoldTrust::WholeModule,
            proven_pure_parameters: false,
        }),
        module: crate::sccp::ModuleRun {
            analysis_body: Some(crate::sccp::AnalysisBody::Procedure(*body_id)),
            ..crate::sccp::ModuleRun::NONE
        },
    });
    sccp_constants_from(&sccp, &fu.ssa, ctx.fold_policy())
}

/// Fold the command substitutions in every `TclOO` method body that the
/// enclosing method frame makes constant — `[self class]`, anything built
/// purely out of it (`[namespace qualifiers [self class]]` folds in one step,
/// since the builtin fold resolves nested substitutions first), and anything
/// built out of a *provably method-local* variable as well.
///
/// Deliberately narrower than [`run_function`]: no O103 static-proc-call fold
/// and no `namespace`-relative chain resolution runs here.  The constants map
/// comes from [`oo_method_constants`], which models which names are object
/// state.
fn run_oo_method_folds(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let Some(bindings) = cu
        .ir_module
        .retained_source_bindings
        .as_deref()
        .filter(|bindings| bindings.matches_module(&cu.ir_module, registry))
    else {
        return;
    };
    let Some(world) = bindings
        .positioned_for_module(&cu.ir_module, registry)
        .and_then(crate::command_binding::SourceCommandBindings::original_completed_command_world)
    else {
        return;
    };
    // The PER-METHOD dispatch barrier: a method is barred
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
    let mut qnames: Vec<&crate::ir::ProcedureId> = cu.ir_module.methods.keys().collect();
    qnames.sort();
    for qname in qnames {
        let Some(method) = cu.ir_module.methods.get(qname) else {
            continue;
        };
        let Some(frame) = oo_frame_for(method, &ctx.command_mutations, &world) else {
            continue;
        };
        let allow_locals = barrier.as_ref().is_some_and(|b| b.allows_locals(*qname));
        let constants = oo_method_constants(ctx, cu, qname, &frame, allow_locals);
        walk_oo_script(ctx, cu, &method.body, &frame, &constants, 0);
    }
}

/// `depth` is the nesting level of `script` — see
/// [`super::MAX_OPTIMISER_WALK_DEPTH`].
fn walk_oo_script(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    script: &Script,
    frame: &OoFrame,
    constants: &std::collections::HashMap<String, String>,
    depth: u32,
) {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) || !script.is_authored_source() {
        return;
    }
    for stmt in &script.statements {
        walk_oo_statement(ctx, cu, script, stmt, frame, constants, depth);
    }
}

/// Visit one statement's command words for the frame-constant fold, then
/// recurse into any nested body it carries.
fn walk_oo_statement(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    script: &Script,
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
            // (empty unless the escaping model proved some name
            // method-local), then the frame-constant cmd-sub folds.
            visit_call_tokens(ctx, cu, t, constants);
            visit_oo_frame_folds(ctx, cu, t, frame, constants);
        }
        // `return [self class]` is the single most common shape of all, and a
        // Retain its actual Script-owned vector and result word when replacing
        // the value; the original head and unrelated argv stay untouched.
        Statement::Return {
            value: Some(raw),
            expr: None,
            ..
        } => try_oo_frame_return_fold(
            ctx,
            cu,
            stmt,
            raw,
            frame,
            script.retained_source_tokens_for_statement(stmt),
        ),
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_oo_script(ctx, cu, &c.body, frame, constants, depth + 1);
            }
            if let Some(b) = else_body {
                walk_oo_script(ctx, cu, b, frame, constants, depth + 1);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_oo_script(ctx, cu, init, frame, constants, depth + 1);
            walk_oo_script(ctx, cu, next, frame, constants, depth + 1);
            walk_oo_script(ctx, cu, body, frame, constants, depth + 1);
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Block { body, .. } => {
            walk_oo_script(ctx, cu, body, frame, constants, depth + 1);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_oo_script(ctx, cu, body, frame, constants, depth + 1);
            for h in handlers {
                walk_oo_script(ctx, cu, &h.body, frame, constants, depth + 1);
            }
            if let Some(fb) = finally_body {
                walk_oo_script(ctx, cu, fb, frame, constants, depth + 1);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(body) = &a.body {
                    walk_oo_script(ctx, cu, body, frame, constants, depth + 1);
                }
            }
            if let Some(b) = default_body {
                walk_oo_script(ctx, cu, b, frame, constants, depth + 1);
            }
        }
        _ => {}
    }
}

pub(crate) fn retained_substitution_calls(
    tokens: &CommandTokens,
    module: &crate::ir::Module,
    registry: &CommandRegistry,
) -> Option<Vec<crate::word_subst::LiftedCall>> {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    if module.source_metadata_input.is_none()
        && module.source_entry.metadata_context.is_standalone()
    {
        return crate::word_subst::checked_lifted_calls(
            tokens,
            tokens.native_lexer_config(module.lexer_config.nested()),
        );
    }
    if !module
        .retained_source_bindings
        .as_deref()?
        .matches_module(module, registry)
    {
        return None;
    }
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)?;
    crate::word_subst::checked_original_lifted_calls_with_metadata_context(
        tokens,
        module.lexer_config.nested(),
        registry,
        metadata,
    )
}

pub(crate) fn retained_call_at(
    calls: &[crate::word_subst::LiftedCall],
    start: u32,
) -> Option<&crate::word_subst::LiftedCall> {
    let mut matching = calls.iter().filter(|call| call.span.start() == start);
    let call = matching.next()?;
    matching.next().is_none().then_some(call)
}

/// A result whose literal spelling a script evaluation would read apart from
/// the result itself ([`crate::const_subst::folded_literal_reads_apart`])
/// abstains: `[list string range $s 0 end-2147483649]` evaluates directly,
/// its braced literal compiles. `dialect` names the release that reads it.
/// The declared script encoding of the unit's retained source input.
fn module_script_encoding(cu: &CompilationUnit) -> tcl_registry::value_transfer::ScriptEncoding {
    cu.ir_module
        .source_metadata_input
        .as_ref()
        .map_or_default(crate::analyser::ResolvedAnalysisInput::script_encoding)
}

fn fold_retained_builtin(
    registry: &CommandRegistry,
    call: &crate::word_subst::LiftedCall,
    calls: &[crate::word_subst::LiftedCall],
    defining_class: Option<&str>,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    (dialect, constants, script_encoding): (
        Option<&'static tcl_dialect::DialectProfile>,
        Option<&std::collections::HashMap<String, String>>,
        tcl_registry::value_transfer::ScriptEncoding,
    ),
) -> Option<String> {
    let metadata = metadata?;
    let release = dialect.map_or(tcl_dialect::TclVersion::V9_0, |profile| {
        profile.vm_runtime_version
    });
    let lookup = |name: &str| constants.and_then(|constants| constants.get(name).cloned());
    crate::const_subst::ConstSubstCtx {
        registry,
        resolution_namespace: "::",
        namespace_context: call
            .tokens
            .as_ref()
            .and_then(crate::registry_invocation::compiled_namespace_context),
        version: None,
        defining_class,
        trusts: &|_| false,
        lookup_var: &lookup,
        script_encoding,
    }
    .fold_retained_call_with_metadata_context(call, calls, metadata)
    .filter(|value| !crate::const_subst::folded_literal_reads_apart(value, release))
}

fn retained_expr_invocation(
    call: &crate::word_subst::LiftedCall,
    registry: &CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<crate::registry_invocation::ResolvedStatementInvocation> {
    let invocation = crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        registry,
        Some(metadata?),
        call.tokens.as_ref()?,
    )?;
    (invocation.facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Expr,
        ))
    .then_some(invocation)
}

fn retained_o115(
    ctx: &PassContext<'_>,
    module: &crate::ir::Module,
    _tokens: &CommandTokens,
    call: &crate::word_subst::LiftedCall,
    word: &str,
    registry: &CommandRegistry,
) -> Option<String> {
    let actual = crate::registry_invocation::retained_module_metadata_context(registry, module)?;
    let metadata = Some(actual.as_ref().into());
    let invocation = retained_expr_invocation(call, registry, metadata)?;
    if !ctx
        .command_mutations
        .trusts(invocation.facts.descriptor_key().label())
    {
        return None;
    }
    let tokens = call.tokens.as_ref()?;
    let (_, command) = tokens
        .source_binding
        .as_ref()?
        .nested_expression_normalisation_source(
            registry,
            tokens,
            &ctx.fold_policy().preparation_context()?,
            None,
        )?;
    // The call was selected from the exact original whole substitution. The
    // inner command source comes from its retained outer expression owner.
    (ctx.source.get(call.span.as_range()) == Some(word)).then_some(command)
}

/// Report an `O129` for each single-token `[cmd …]` word of this command that
/// the method frame folds to a constant.
fn visit_oo_frame_folds(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    tokens: &CommandTokens,
    frame: &OoFrame,
    _constants: &std::collections::HashMap<String, String>,
) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let actual =
        crate::registry_invocation::retained_module_metadata_context(registry, &cu.ir_module);
    let metadata = actual.as_deref().map(Into::into);
    let Some(calls) = retained_substitution_calls(tokens, &cu.ir_module, registry) else {
        return;
    };
    let mut rewrites: Vec<(tcl_lexer::Span, String)> = Vec::new();
    for word in tokens.words() {
        let crate::ir::WordExpr::CommandSubstitution { source, .. } = word else {
            continue;
        };
        let Some(call) = retained_call_at(&calls, source.span.start()) else {
            continue;
        };
        if let Some(value) = fold_retained_builtin(
            registry,
            call,
            &calls,
            Some(&frame.defining_class),
            metadata,
            (ctx.dialect, None, module_script_encoding(cu)),
        ) && let Some(word) = render_propagation_word(&value, ctx.lexer_config())
        {
            rewrites.push((call.span, word));
        }
    }
    for (span, folded) in rewrites {
        ctx.report(Optimisation::new(
            DiagCode::O129,
            "Fold constant builtin command substitution",
            tcl_lexer::word_span_at(ctx.source, span),
            folded,
        ));
    }
}

/// `return [self class]` → `return ::TheClass`.
///
/// The original whole statement is the rewrite target. Its retained result
/// word selects the replacement site, preserving the actual written head.
fn try_oo_frame_return_fold(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    statement: &Statement,
    raw: &str,
    frame: &OoFrame,
    tokens: Option<&CommandTokens>,
) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let Some(tokens) = tokens else {
        return;
    };
    let actual =
        crate::registry_invocation::retained_module_metadata_context(registry, &cu.ir_module);
    let metadata = actual.as_deref().map(Into::into);
    let Some(calls) = retained_substitution_calls(tokens, &cu.ir_module, registry) else {
        return;
    };
    let mut roots = calls.iter().filter(|call| {
        ctx.source
            .get(call.span.start() as usize..call.span.end() as usize)
            == Some(raw.trim())
    });
    let Some(call) = roots.next() else {
        return;
    };
    if roots.next().is_some() {
        return;
    }
    let Some(value) = fold_retained_builtin(
        registry,
        call,
        &calls,
        Some(&frame.defining_class),
        metadata,
        (ctx.dialect, None, module_script_encoding(cu)),
    ) else {
        return;
    };
    let Some((rewrite_span, replacement)) =
        original_return_value_replacement(ctx, statement, tokens, &value)
    else {
        return;
    };
    ctx.report(Optimisation::new(
        DiagCode::O129,
        "Fold constant builtin command substitution",
        rewrite_span,
        replacement,
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
    // A computed or opaque variable name can rewrite any local binding under
    // a spelling absent from SCCP's per-name lattice.  Do not let its raw
    // constants reach the text-level folds below: unlike
    // `run_load_forwarding`, this path would otherwise bypass the shared
    // dynamic-name guard and could emit (for example) `return 1` after a
    // depth-capped nested substitution may have run `set $name 2`.
    if fu.dynamic_barrier_blocks_value_motion() {
        // A local value-motion barrier withdraws lattice substitutions. Exact
        // original invocation receipts still independently authorise closed
        // constant calls that consume no such local facts.
        walk_script(
            ctx,
            cu,
            script,
            &std::collections::HashMap::new(),
            None,
            namespace,
            0,
        );
        return;
    }

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
/// builtin-fold lattice.
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
    let mut constants = sccp_constants_for(fu, ctx.fold_policy());
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
            source_metadata_input: fu
                .invocation_metadata_context_for_module(registry, &cu.ir_module)
                .and_then(
                    crate::registry_invocation::InvocationMetadataContext::source_analysis_input,
                ),
            traced_variables: &cu.ir_module.traced_variables,
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
            deferred_writes: &cu.ir_module.deferred_writes,
            analysis_context: None,
            existence: None,
        },
        Some(crate::sccp::BuiltinFoldInputs {
            registry,
            source_metadata_input: fu
                .invocation_metadata_context_for_module(registry, &cu.ir_module)
                .and_then(
                    crate::registry_invocation::InvocationMetadataContext::source_analysis_input,
                ),
            mutations: &ctx.command_mutations,
            dialect: ctx.dialect,
            // No method frame here — `[self class]`-style frame facts fold
            // only in `run_oo_method_folds`' proven method re-runs.
            defining_class: None,
            registry_engine: true,
            trust: crate::sccp::FoldTrust::WholeModule,
            proven_pure_parameters: false,
        }),
    );
    for (name, text) in sccp_constants_from(&rerun, &fu.ssa, ctx.fold_policy()) {
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
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) || !script.is_authored_source() {
        return;
    }
    for stmt in &script.statements {
        walk_statement(
            ctx,
            cu,
            StatementSource {
                statement: stmt,
                tokens: script.retained_source_tokens_for_statement(stmt),
            },
            constants,
            numeric,
            namespace,
            depth,
        );
    }
}

#[derive(Clone, Copy)]
struct StatementSource<'a> {
    statement: &'a Statement,
    tokens: Option<&'a CommandTokens>,
}

fn walk_statement(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    source: StatementSource<'_>,
    constants: &std::collections::HashMap<String, String>,
    numeric: NumericCtx<'_>,
    namespace: &str,
    depth: u32,
) {
    match source.statement {
        Statement::Call {
            span,
            command,
            tokens,
            ..
        } => {
            if let Some(t) = tokens {
                visit_call_tokens(ctx, cu, t, constants);
                visit_call_cmd_subst_folds(ctx, cu, t, constants, namespace, *span);
            }
            try_fold_static_proc_call(ctx, cu, *span, command, tokens.as_ref(), namespace);
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
            span,
            tokens: Some(t),
            ..
        } => {
            visit_call_cmd_subst_folds(ctx, cu, t, constants, namespace, *span);
        }
        statement @ Statement::Return { .. } => {
            try_fold_return_terminator(ctx, statement, constants, &cu.ir_module, source.tokens);
        }
        Statement::AssignExpr { span, expr, .. } => {
            try_substitute_assign_expr(ctx, source, *span, expr, constants, numeric);
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
/// the callee's CFG, then reads the one exact value every reachable exit
/// gives ([`crate::interprocedural::exit_value`], the reading the summary's
/// seedless run shares). Used for the argument-sensitive O103 fold
/// (`[::math::add 2 4]` → `6`) that the summary's argument-independent
/// `constant_return` cannot express.
///
/// This re-runs SCCP fresh (not `callee.sccp`), so it must feed the same
/// registry + whole-module trace facts the original build did — otherwise
/// this specific re-run path would be independently trace-blind (the same
/// silent-miscompile class `run_function`/`run_load_forwarding` guard
/// against). The callee and unit must retain the same supplied owner;
/// unavailable callee metadata cannot borrow the caller's grammar or folds.
fn evaluate_proc_with_constants(
    (ctx, cu): (&PassContext<'_>, &CompilationUnit),
    callee: &FunctionUnit,
    params: &[tcl_syntax::formal_params::FormalParameter],
    args: &[ConstValue],
    grammar: tcl_dialect::ParameterGrammar,
    policy: FoldPolicy,
) -> Option<tcl_registry::value_transfer::ExactValue> {
    let seed = seed_params_from_args(params, args, grammar, policy)?;
    let registry = ctx.registry?;
    let metadata = callee.invocation_metadata_context_for_module(registry, &cu.ir_module)?;
    let source_metadata_input = metadata.source_analysis_input()?;
    let folds = crate::sccp::BuiltinFoldInputs {
        source_metadata_input: Some(source_metadata_input),
        dialect: Some(source_metadata_input.unit_profile()),
        proven_pure_parameters: true,
        ..ctx.rewrite_folds()
    };
    // A call the callee makes to another procedure of the module is applied
    // as that procedure's transfer summary says, its result too: the
    // re-run's module is the unit's.
    let module = crate::interprocedural::ModuleProcedures::of_unit(cu, registry);
    let no_escaping = std::collections::HashSet::new();
    let result = crate::sccp::sccp_in_module(&crate::sccp::SolveInputs {
        cfg: &callee.cfg,
        ssa: &callee.ssa,
        param_constants: Some(&seed),
        policy,
        extra_escaping: &no_escaping,
        trace: crate::sccp::TraceInputs {
            registry,
            source_metadata_input: Some(source_metadata_input),
            traced_variables: &cu.ir_module.traced_variables,
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
            deferred_writes: &cu.ir_module.deferred_writes,
            analysis_context: None,
            existence: None,
        },
        // This re-run feeds an O103 *rewrite*, so it takes the whole-module
        // stance. Without any trust fact — what it used before — it folded
        // `[llength …]` with builtin semantics even where the module shadows
        // `llength`, handing O103 a value the rest of the pipeline disagrees
        // with (#2164).
        folds: Some(folds),
        module: crate::sccp::ModuleRun {
            analysis_body: callee.body_id.map(crate::sccp::AnalysisBody::Procedure),
            procedures: Some(&module),
            owned: None,
            level: crate::sccp::ModuleLevel::Results,
            reads_exits: true,
        },
    });
    crate::interprocedural::exit_value(
        crate::interprocedural::ExitBody::of(callee),
        &result,
        crate::interprocedural::ExitReading {
            policy,
            grammar: callee
                .source_lexer_config()
                .grammar_over(source_metadata_input.unit_profile().grammar),
            folds,
            module: Some((&module, &callee.body_id?)),
        },
    )
}

/// Seed ordinary incoming formal values using the shared native activation
/// plan. Defaults and rest names retain their selected Tcl/Jim semantics;
/// caller links cannot supply ordinary constant local slots.
fn seed_params_from_args(
    params: &[tcl_syntax::formal_params::FormalParameter],
    args: &[ConstValue],
    grammar: tcl_dialect::ParameterGrammar,
    policy: FoldPolicy,
) -> Option<std::collections::HashMap<(String, crate::ssa::Version), LatticeValue>> {
    use tcl_syntax::formal_params::FormalArgumentBinding;
    let plan =
        tcl_syntax::formal_params::bind_formal_arguments(params, args.len(), grammar).ok()?;
    let mut seed = std::collections::HashMap::new();
    for formal in plan {
        let (name, value) = match formal {
            FormalArgumentBinding::Value {
                parameter,
                argument,
            } => (
                params.get(parameter)?.name.clone(),
                args.get(argument)?.clone(),
            ),
            FormalArgumentBinding::Default { parameter } => {
                let formal = params.get(parameter)?;
                (
                    formal.name.clone(),
                    crate::sccp::parse_literal_value(formal.default.as_deref()?),
                )
            }
            FormalArgumentBinding::Rest {
                name, start, len, ..
            } => {
                let end = start.checked_add(len)?;
                let tail = args
                    .get(start..end)?
                    .iter()
                    .map(|value| const_value_text(value, policy))
                    .collect::<Option<Vec<_>>>()?;
                (name, ConstValue::String(tcl_syntax::list::join_list(tail)))
            }
            FormalArgumentBinding::CallerLink { .. } => return None,
        };
        seed.insert((name, 0), LatticeValue::Const(value));
    }
    Some(seed)
}

/// Render a [`ConstValue`] back to its plain text form — the inverse of
/// [`crate::sccp::parse_literal_value`], used to re-serialise a call
/// argument's already-typed constant into one element of the canonical
/// list text [`seed_params_from_args`] builds for a variadic `args`
/// parameter.
fn const_value_text(cv: &ConstValue, policy: FoldPolicy) -> Option<String> {
    super::helpers::literals::format_constant_with_policy(cv, policy)
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
/// the same rule: a relative dotted word (`inner::p`) must not resolve
/// straight to `::inner::p`, rooted at global, because real Tcl (and the
/// analyser side) tries the *current* namespace first
/// (`{namespace}::inner::p`); two procs of that shape in different
/// namespaces could fold a call to the wrong one's constant return.
///
/// Returns the first candidate that has a summary, else `None`.
#[cfg(test)]
fn resolve_proc_qname(
    command: &str,
    namespace: &str,
    ia: &crate::interprocedural::InterproceduralAnalysis,
) -> Option<String> {
    if command.is_empty() {
        return None;
    }
    crate::naming::resolve_command_with::<&str, _>(namespace, &[], command, |qname| {
        ia.procedures
            .values()
            .filter(|summary| summary.qualified_name == qname)
            .count()
            == 1
    })
}

/// O103: if `command` resolves to a proc with `can_fold_static_calls`
/// and a `constant_return`, and the call is one [`summary_answers`],
/// emit a rewrite replacing the call with the literal return value.
fn try_fold_static_proc_call(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    span: tcl_lexer::Span,
    command: &str,
    tokens: Option<&CommandTokens>,
    _namespace: &str,
) {
    let Some(ia) = cu.interproc.as_ref() else {
        return;
    };
    let Some(registry) = ctx.registry else {
        return;
    };
    let Some(calls) = tokens.and_then(|tokens| {
        crate::registry_invocation::original_source_procedure_calls_for_word_for_module(
            tokens,
            command,
            &cu.ir_module,
            registry,
        )
    }) else {
        return;
    };
    let [call] = calls.as_slice() else {
        return;
    };
    let Some(summary) = ia.procedures.get(&call.id()) else {
        return;
    };
    let qname = &call.procedure().qualified_name;
    // A redefined proc has an ambiguous body — never fold its calls
    // (the flow-sensitive rename gate, mirroring the cmd-subst form and
    // `redefined_procedures` check).
    if cu.ir_module.redefined_procedures.contains(qname) {
        return;
    }
    // Nor a proc whose bare name is later `rename`d over or `interp
    // alias`ed elsewhere in the module — folding this call site to the
    // *originally-declared* proc's constant return would miscompile a
    // script where `command` no longer denotes that proc by the time this
    // call runs (e.g. `proc a {…} …; proc b {…} …; rename b a` moves `b`'s
    // body onto the name `a`; a later `[a …]` call must not fold to `a`'s
    // original body).
    if !ctx.command_mutations.trusts_proc_binding(qname) {
        return;
    }
    if !summary.can_fold_static_calls {
        return;
    }
    let literal_words = call
        .effective_arguments()
        .words
        .iter()
        .all(|word| {
            matches!(
                word,
                crate::ir::WordExpr::Literal { .. } | crate::ir::WordExpr::BracedLiteral { .. }
            )
        })
        .then(|| call.argument_count())
        .flatten();
    if !summary_answers(&summary.params, literal_words) {
        return;
    }
    let Some(cr) = &summary.constant_return else {
        return;
    };
    let replacement = match cr {
        ConstantReturn::Int(i) => i.to_string(),
        ConstantReturn::Float(f) => {
            let Some(text) = crate::tcl_expr_eval::format_tcl_value_with_policy(
                &crate::tcl_expr_eval::TclValue::Float(*f),
                ctx.fold_policy(),
            ) else {
                return;
            };
            text
        }
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
    // would avoid this. Until CMD-subst spans are tracked at
    // the call argument level, emit as a hint so
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

/// Replace only the retained original result word of a simple lowered return.
/// This constructs source; the caller must independently approve the fold.
fn original_return_value_replacement(
    ctx: &PassContext<'_>,
    statement: &Statement,
    tokens: &CommandTokens,
    value: &str,
) -> Option<(tcl_lexer::Span, String)> {
    let replacement = render_propagation_word(value, ctx.lexer_config())?;
    original_return_word_replacement(ctx, statement, tokens, &replacement)
}

/// Splice one deliberately constructed complete word into the same original
/// return's retained result ordinal. This supplies source geometry only.
fn original_return_word_replacement(
    ctx: &PassContext<'_>,
    statement: &Statement,
    tokens: &CommandTokens,
    replacement: &str,
) -> Option<(tcl_lexer::Span, String)> {
    let image = tcl_lexer::SourceImage::document(replacement);
    let extent = tcl_lexer::Span::new(0, u32::try_from(replacement.len()).ok()?);
    let plan = tcl_lexer::native_script_words_in(image, extent, ctx.lexer_config()).ok()?;
    if plan.fatal_tail.is_some() {
        return None;
    }
    let [proposed] = plan.commands.as_slice() else {
        return None;
    };
    let [proposed_word] = proposed.words.as_slice() else {
        return None;
    };
    if proposed_word.span() != extent || proposed_word.group().expand {
        return None;
    }
    let Statement::Return {
        value_word: Some(value_word),
        ..
    } = statement
    else {
        return None;
    };
    let module = ctx.ir_module?;
    let registry = ctx.registry?;
    let config = ctx.lexer_config();
    let span = original_statement_rewrite_span(ctx, statement.span())?;
    let binding = tokens.source_binding.as_ref()?;
    binding.original_invocation_metadata_for_module(tokens, module, registry)?;
    let site = binding.invocation_site()?;
    if site.offset != span.start() || site.source.source_image() != &module.source {
        return None;
    }
    let command = site
        .source
        .original_lexical_command(tokens.words(), site.offset, config)?;
    let mut matches = tokens
        .words()
        .iter()
        .enumerate()
        .filter(|(_, word)| *word == value_word);
    let (written, _) = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    let word = command.native_words().get(written)?;
    let start = usize::try_from(word.span().start().checked_sub(span.start())?).ok()?;
    let end = usize::try_from(word.span().end().checked_sub(span.start())?).ok()?;
    let mut source = ctx
        .source
        .get(span.start() as usize..span.end() as usize)?
        .to_owned();
    source.get(start..end)?;
    source.replace_range(start..end, replacement);
    // Full proposed statement parsing detects any quote/continuation seam.
    let image = tcl_lexer::SourceImage::document(&source);
    let extent = tcl_lexer::Span::new(0, u32::try_from(source.len()).ok()?);
    let after = tcl_lexer::native_script_words_in(image, extent, config).ok()?;
    if after.fatal_tail.is_some() {
        return None;
    }
    let [after] = after.commands.as_slice() else {
        return None;
    };
    (after.words.len() == command.native_words().len()).then_some((span, source))
}

/// Fold an original return operand using its retained word kind and constants.
fn try_fold_return_terminator(
    ctx: &mut PassContext<'_>,
    statement: &Statement,
    constants: &std::collections::HashMap<String, String>,
    module: &crate::ir::Module,
    tokens: Option<&CommandTokens>,
) {
    let Statement::Return {
        span,
        value,
        value_word,
        expr,
        ..
    } = statement
    else {
        return;
    };
    let span = *span;
    let value = value.as_deref();
    let expr = expr.as_ref();

    // O115: `return [expr {[expr {E}]}]` → `return [expr {E}]`. Checked
    // before the `expr.is_some()` early-return below because the return
    // value of a cmd-sub also populates `expr`, yet the redundant-nested-
    // expr collapse operates on the raw value text. Also requires `expr`
    // to be untouched anywhere in the module (mirrors the O129
    // builtin-fold trust check) — both the outer and inner `[expr {…}]`
    // are genuine command substitutions, and a shadowed `expr` no longer
    // has builtin semantics.
    let collapse = tokens.and_then(|tokens| {
        let registry = ctx.registry?;
        let calls = retained_substitution_calls(tokens, module, registry)?;
        let raw = value?.trim();
        let mut matching = calls.iter().filter(|call| {
            ctx.source
                .get(call.span.start() as usize..call.span.end() as usize)
                == Some(raw)
        });
        let call = matching.next()?;
        if matching.next().is_some() {
            return None;
        }
        retained_o115(ctx, module, tokens, call, raw, registry)
    });
    if let Some(collapsed) = collapse
        && let Some((span, replacement)) = tokens
            .and_then(|tokens| original_return_word_replacement(ctx, statement, tokens, &collapsed))
    {
        ctx.report(Optimisation::new(
            DiagCode::O115,
            "Remove redundant nested expr",
            span,
            replacement,
        ));
        return;
    }

    if let Some(expression) = expr {
        let env = constants
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    crate::tcl_expr_eval::EnvValue::Str(value.clone()),
                )
            })
            .collect::<crate::tcl_expr_eval::Env>();
        if let Some(value) = ctx.eval_expression_at(expression, &env, span)
            && let Some(folded) =
                crate::tcl_expr_eval::format_tcl_value_with_policy(&value, ctx.fold_policy())
            && !folded.contains(['$', '['])
            && let Some((rewrite_span, replacement)) = tokens.and_then(|tokens| {
                original_return_value_replacement(ctx, statement, tokens, &folded)
            })
        {
            ctx.report(Optimisation::new(
                DiagCode::O101,
                "Fold constant expression",
                rewrite_span,
                replacement,
            ));
            return;
        }
    }

    // Only fold `return $v` — numeric/bare literals and complex
    // values are left to richer passes.
    if expr.is_some() {
        return;
    }
    let Some(crate::ir::WordExpr::Variable { spelling, .. }) = value_word.as_ref() else {
        return;
    };
    let Some(name) = whole_scalar_variable_reference_name(spelling, module.lexer_config) else {
        return;
    };
    let Some(resolved) = constants.get(name) else {
        return;
    };
    // Render the constant as a single self-contained word
    // rather than bailing on metacharacters (`return {Hello World}`).
    let Some((rewrite_span, replacement)) = tokens
        .and_then(|tokens| original_return_value_replacement(ctx, statement, tokens, resolved))
    else {
        return;
    };
    ctx.report(Optimisation::new(
        DiagCode::O100,
        "Fold return of constant variable",
        rewrite_span,
        replacement,
    ));
}

/// O100 (`AssignExpr` form): substitute SCCP-proved constants
/// into ``set name [expr { … }]`` expressions.
///
/// Builds on ``substitute_expr_constants`` — the same helper the
/// branch-condition pass uses. Emits an O100 rewrite targeting
/// the whole ``set`` statement with the substituted expression
/// re-wrapped in ``[expr { … }]``. The rewrite span is extended
/// through the original retained command vector so nested substitutions do not
/// leave orphan delimiters. The actual written target and head are retained.
fn try_substitute_assign_expr(
    ctx: &mut PassContext<'_>,
    source: StatementSource<'_>,
    span: tcl_lexer::Span,
    expr: &crate::expr_ast::ExprNode,
    constants: &std::collections::HashMap<String, String>,
    numeric: NumericCtx<'_>,
) {
    use super::helpers::expr_simplify::{
        expr_has_command_subst, instcombine_expr_typed, substitute_expr_constants_for_execution,
    };
    use crate::tcl_expr_eval::{Env, format_tcl_value_with_policy};

    let Some(rewrite_span) = original_statement_rewrite_span(ctx, span) else {
        return;
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
    let original = source.tokens.and_then(|tokens| {
        super::expr_simplify::OriginalExpressionStatementSource::capture_with_tokens(
            ctx,
            source.statement,
            tokens,
            expr,
        )
    });
    let expr_text = crate::expr_ast::render_expr(expr);
    let result = substitute_expr_constants_for_execution(
        &expr_text,
        constants,
        ctx.dialect,
        &crate::tcl_expr_eval::NativeOperandProofs::new(),
    );
    if !result.changed {
        return;
    }
    // After substitution, try to fold the result to a constant
    // — the cascade where O100 enables O101. When
    // the substituted expression is fully constant we can emit
    // the unwrapped ``set name VALUE`` form directly. Otherwise
    // keep the expression wrapper around the substituted text.
    let env = constants
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                crate::tcl_expr_eval::EnvValue::Str(value.clone()),
            )
        })
        .collect::<Env>();
    if let Some(val) = ctx.eval_expression_at(expr, &env, span)
        && let Some(folded) = format_tcl_value_with_policy(&val, ctx.fold_policy())
    {
        let needs_quoting = folded.is_empty()
            || folded.contains([
                ' ', '\t', '\n', '\r', '$', '[', ']', '{', '}', '"', '\\', '\0', ';',
            ]);
        if !needs_quoting
            && let Some(replacement) = original.as_ref().and_then(|source| source.folded(&folded))
        {
            ctx.report(Optimisation::new(
                DiagCode::O100,
                "Propagate constant and fold",
                rewrite_span,
                replacement,
            ));
            return;
        }
    }
    // Not fully constant; try one pass of instcombine on the
    // substituted expression to pick up identity simplifications
    // (e.g. ``$a + 0`` after ``$a`` folds to ``3``).
    let (simplified, _changed) = instcombine_expr_typed(&result.text, false, numeric, ctx.dialect);
    let final_text = if simplified.trim().is_empty() {
        result.text.clone()
    } else {
        simplified
    };
    if ctx
        .expression_rewrite_equivalence_at(expr, &final_text, &env, span)
        .is_err()
    {
        return;
    }
    let Some(replacement) = original
        .as_ref()
        .and_then(|source| source.simplified(&final_text))
    else {
        return;
    };
    ctx.report(Optimisation::new(
        DiagCode::O100,
        "Propagate constant into expr argument",
        rewrite_span,
        replacement,
    ));
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
    cu: &CompilationUnit,
    tokens: &CommandTokens,
    call: &crate::word_subst::LiftedCall,
    constants: &std::collections::HashMap<String, String>,
) -> Option<String> {
    let registry = ctx.registry?;

    let input = cu.ir_module.source_metadata_input.as_ref()?;

    let context = PassContext {
        source: ctx.source,
        dialect: ctx.dialect,
        registry: Some(registry),
        ir_module: Some(&cu.ir_module),
        ..PassContext::default()
    };
    let actual = context.retained_metadata_context()?;

    let mut parser = tcl_syntax::expr::parser::ExprParseContext::for_profile(input.unit_profile());
    parser.lexer_grammar = input.lexer_config().grammar_over(parser.lexer_grammar);
    let expression = crate::word_subst::lifted_source_expressions_with_metadata_context(
        Some(tokens),
        registry,
        Some(actual.as_ref().into()),
        input.lexer_config(),
        parser,
    )
    .into_iter()
    .find(|expression| expression.span == call.span)?;

    let env = constants
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                crate::tcl_expr_eval::EnvValue::Str(value.clone()),
            )
        })
        .collect();
    let value = context.eval_expression_at(&expression.expression, &env, call.span)?;

    crate::tcl_expr_eval::format_tcl_value_with_policy(&value, context.fold_policy())
        .and_then(|value| render_propagation_word(&value, input.lexer_config()))
}

fn visit_call_cmd_subst_folds(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    tokens: &CommandTokens,
    constants: &std::collections::HashMap<String, String>,
    _namespace: &str,
    statement_span: tcl_lexer::Span,
) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let actual =
        crate::registry_invocation::retained_module_metadata_context(registry, &cu.ir_module);
    let metadata = actual.as_deref().map(Into::into);
    let Some(calls) = retained_substitution_calls(tokens, &cu.ir_module, registry) else {
        return;
    };

    for word in tokens.words() {
        let crate::ir::WordExpr::CommandSubstitution { spelling, source } = word else {
            continue;
        };
        let Some(call) = retained_call_at(&calls, source.span.start()) else {
            continue;
        };
        if let Some(collapsed) = retained_o115(ctx, &cu.ir_module, tokens, call, spelling, registry)
        {
            let wrapper =
                crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
                    registry, metadata, tokens,
                )
                .filter(|invocation| {
                    invocation.facts.operation
                        == tcl_registry::SemanticOperationId::StructuredLowering(
                            tcl_registry::hooks::LoweringHookId::Return,
                        )
                        && tokens.words().len() == 2
                })
                .and_then(|_| {
                    let original = ctx.source.get(statement_span.as_range())?;
                    let start = call.span.start().checked_sub(statement_span.start())? as usize;
                    let end = call.span.end().checked_sub(statement_span.start())? as usize;
                    let mut replacement = original.to_owned();
                    replacement.replace_range(start..end, &collapsed);
                    Some((statement_span, replacement))
                });
            let (span, replacement) = wrapper.unwrap_or((call.span, collapsed));
            ctx.report(Optimisation::new(
                DiagCode::O115,
                "Remove redundant nested expr",
                span,
                replacement,
            ));
            continue;
        }
        let is_expr = retained_expr_invocation(call, registry, metadata).is_some();

        if is_expr && let Some(folded) = try_o101_expr_arg_fold(ctx, cu, tokens, call, constants) {
            ctx.report(Optimisation::new(
                DiagCode::O101,
                "Fold constant expression",
                call.span,
                folded,
            ));
            continue;
        }
        if let Some(value) = fold_retained_builtin(
            registry,
            call,
            &calls,
            None,
            metadata,
            (ctx.dialect, Some(constants), module_script_encoding(cu)),
        ) && let Some(word) = render_propagation_word(&value, ctx.lexer_config())
        {
            let code = call
                .tokens
                .as_ref()
                .and_then(|tokens| {
                    crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                        registry, metadata, tokens,
                    )
                })
                .map_or(DiagCode::O129, |invocation| {
                    // value-transfer-ok: irreducible — names the reported
                    // rewrite family of a fold already made; no value depends on it.
                    match invocation.facts.canonical_command.as_str() {
                        "list" => DiagCode::O116,
                        "lindex" => DiagCode::O118,
                        _ => DiagCode::O129,
                    }
                });
            ctx.report(Optimisation::new(
                code,
                "Fold constant builtin command substitution",
                call.span,
                word,
            ));
            continue;
        }
        if let Some((name, replacement)) = try_o103_proc_fold(ctx, cu, call, &calls) {
            ctx.report(Optimisation::new(
                DiagCode::O103,
                format!("Fold pure-proc call to '{name}' to its constant return"),
                call.span,
                replacement,
            ));
        }
    }
}

/// Fold a pure-proc command substitution to its constant return (O103),
/// returning `(qualified_name, replacement_word)`. Uses the interprocedural
/// summary's argument-independent constant return when present and the call
/// is one it answers ([`summary_answers`]), else re-runs the pure callee
/// under the call's constant arguments. `None` when the head
/// is not a foldable internal proc. Extracted from [`visit_call_cmd_subst_folds`].
fn try_o103_proc_fold(
    ctx: &PassContext<'_>,
    cu: &CompilationUnit,
    call: &crate::word_subst::LiftedCall,
    calls: &[crate::word_subst::LiftedCall],
) -> Option<(String, String)> {
    let ia = cu.interproc.as_ref()?;
    let tokens = call.tokens.as_ref()?;
    let binding = tokens.source_binding.as_ref()?;
    let scoped = binding.scoped_procedure_evaluation(tokens);
    let target = binding
        .proved_execution_target()
        .or_else(|| scoped.as_ref().map(|receipt| receipt.target))?;
    if target.kind != crate::command_binding::BindingKind::Proc || target.registry_backed {
        return None;
    }
    let qname = target.command.clone();
    let (declaration_id, declaration) = cu.ir_module.source_procedure_for_target(target)?;
    if !target.matches_authored_implementation_image(&cu.ir_module.source, declaration.span.start())
    {
        return None;
    }
    let summary = ia.procedures.get(&declaration_id)?;
    // A redefined proc has an ambiguous body — never fold its calls.
    if cu.ir_module.redefined_procedures.contains(&qname) {
        return None;
    }
    let grammar = binding
        .variable_context
        .invocation_dialect?
        .parameter_grammar()?;
    let parameters =
        tcl_syntax::formal_params::parse_formal_parameters_in(&declaration.params_raw, grammar)
            .ok()?;
    let registry = ctx.registry?;
    let actual =
        crate::registry_invocation::retained_module_metadata_context(registry, &cu.ir_module)?;
    let original = crate::const_subst::ConstSubstCtx {
        registry,
        resolution_namespace: "::",
        namespace_context: binding.lookup_namespace_key.to_compiled_context(),
        version: None,
        defining_class: None,
        trusts: &|_| false,
        lookup_var: &|_| None,
        script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
    }
    .retained_arguments_with_metadata_context(call, calls, actual.as_ref().into())?;
    let plan =
        tcl_syntax::formal_params::bind_formal_arguments(&parameters, original.len(), grammar)
            .ok()?;
    if plan.iter().any(|formal| {
        matches!(
            formal,
            tcl_syntax::formal_params::FormalArgumentBinding::CallerLink { .. }
        )
    }) {
        return None;
    }
    if summary.pure
        && let Some(scoped) = scoped
    {
        return Some((
            qname,
            render_propagation_word(scoped.result.text(), ctx.lexer_config())?,
        ));
    }
    if summary.pure
        && binding.original_invocation_completes_normally(tokens)
        && let Some(result) = binding.original_normal_result(tokens)
    {
        return Some((
            qname,
            render_propagation_word(result.text(), ctx.lexer_config())?,
        ));
    }
    let args: Vec<_> = original
        .iter()
        .map(|value| crate::sccp::parse_literal_value(value))
        .collect();
    let replacement = if summary.can_fold_static_calls
        && let Some(cr) = &summary.constant_return
        && tcl_syntax::formal_params::bind_formal_arguments(&parameters, args.len(), grammar)
            .is_ok()
    {
        render_proc_constant_return(ctx, cr)?
    } else if summary.pure
        && let Some(callee) = cu.procedures.get(&declaration_id)
        && let Some(value) = evaluate_proc_with_constants(
            (ctx, cu),
            callee,
            &parameters,
            &args,
            grammar,
            ctx.fold_policy(),
        )
    {
        // Argument-sensitive: re-run SCCP on the pure callee with the call's
        // constant arguments bound and fold the constant return.
        render_propagation_word(value.as_str().ok()?, ctx.lexer_config())?
    } else {
        return None;
    };
    Some((summary.qualified_name.clone(), replacement))
}

fn render_proc_constant_return(
    ctx: &PassContext<'_>,
    constant: &crate::interprocedural::ConstantReturn,
) -> Option<String> {
    Some(match constant {
        ConstantReturn::Int(value) => value.to_string(),
        ConstantReturn::Float(value) => crate::tcl_expr_eval::format_tcl_value_with_policy(
            &crate::tcl_expr_eval::TclValue::Float(*value),
            ctx.fold_policy(),
        )?,
        ConstantReturn::Bool(true) => "1".to_owned(),
        ConstantReturn::Bool(false) => "0".to_owned(),
        ConstantReturn::Str(value) => render_propagation_word(value, ctx.lexer_config())?,
    })
}
/// Whether a procedure's argument-independent constant return answers a
/// call whose words after the head are `literal_words` many, all literal
/// (`None` when one substitutes): only a call the argument-sensitive re-run
/// could make — every word literal, as many as the parameters accept. A
/// word that substitutes runs before the call and may raise or write, and a
/// count the parameters do not accept raises; the constant says neither
/// (#2389).
fn summary_answers(params: &[String], literal_words: Option<usize>) -> bool {
    literal_words
        .and_then(|count| u16::try_from(count).ok())
        .is_some_and(|count| crate::interprocedural::arity_from_names(params).accepts(count))
}

/// Parse the head word out of a CMD-subst interior. Returns
/// `None` when the head word is empty or contains metacharacters
/// that would change the parsed command name under substitution.
#[cfg(test)]
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

fn visit_call_tokens(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
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
        // A word the lexer reads as one `[…]` is a command substitution, not
        // interpolation text: `visit_call_cmd_subst_folds` folds it through
        // its route, whose decline is final, so neither rewriter below
        // may take it apart — a `]` inside a braced argument
        // (`[string match -nocase {a]€} {€a}]`) is no close of it.
        if single && tokens.argv_kinds.get(i).copied() == Some(tcl_lexer::TokenType::Cmd) {
            continue;
        }
        if single
            && let Some(crate::ir::WordExpr::Variable { spelling, .. }) = tokens.words().get(i)
        {
            visit_simple_var_word(ctx, *span, spelling, constants, cu.ir_module.lexer_config);
        }
        // `"..."` interpolation substitution — works on both
        // single-token (quoted strings) and composite (mixed
        // text + var) words.
        visit_string_interpolation(ctx, *span, text, constants, cu.ir_module.lexer_config);
        // Fold a pure-builtin `[cmd …]` substitution
        // embedded *inside* an interpolation string.
        visit_string_interpolation_cmd_subs(ctx, cu, tokens, *span, text, constants);
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
/// Each original substitution with a proved executable registry target and
/// closed original arguments is folded through the shared retained-call owner.  Soundness guards: the result must not
/// reintroduce a substitution into the `"…"` context — a result
/// carrying `$`, `[`, `]`, `\`, or `"` is left unfolded.  `$var`
/// substitutions and any non-foldable `[cmd]` are kept verbatim; at
/// least one successful fold is required to emit.
fn visit_string_interpolation_cmd_subs(
    ctx: &mut PassContext<'_>,
    cu: &CompilationUnit,
    tokens: &CommandTokens,
    span: tcl_lexer::Span,
    text: &str,
    constants: &std::collections::HashMap<String, String>,
) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let actual =
        crate::registry_invocation::retained_module_metadata_context(registry, &cu.ir_module);
    let metadata = actual.as_deref().map(Into::into);
    let Some(calls) = retained_substitution_calls(tokens, &cu.ir_module, registry) else {
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
    let Some(rewrite_span) = original_quoted_word_rewrite_span(ctx, span, inside) else {
        return;
    };
    let content_base = if ctx.source.as_bytes().get(rewrite_span.start() as usize) == Some(&b'"') {
        rewrite_span.start().checked_add(1)
    } else {
        Some(rewrite_span.start())
    };
    let Some(content_base) = content_base else {
        return;
    };
    let Some(content_end) = u32::try_from(inside.len())
        .ok()
        .and_then(|length| content_base.checked_add(length))
    else {
        return;
    };
    if ctx.source.get(content_base as usize..content_end as usize) != Some(inside) {
        return;
    }
    let contained: Vec<_> = calls
        .iter()
        .filter(|call| call.span.start() >= content_base && call.span.end() <= content_end)
        .collect();
    let mut roots: Vec<_> = contained
        .iter()
        .copied()
        .filter(|call| {
            !contained.iter().any(|outer| {
                outer.span.start() < call.span.start() && outer.span.end() >= call.span.end()
            })
        })
        .collect();
    roots.sort_by_key(|call| call.span.start());
    let mut out = String::with_capacity(inside.len());
    let mut last = 0;
    let mut folded_any = false;
    let lattice = (ctx.dialect, Some(constants), module_script_encoding(cu));
    for call in roots {
        let Some(result) = fold_retained_builtin(registry, call, &calls, None, metadata, lattice)
        else {
            continue;
        };
        if result
            .bytes()
            .any(|byte| matches!(byte, b'$' | b'[' | b']' | b'\\' | b'"'))
        {
            continue;
        }
        let start = (call.span.start() - content_base) as usize;
        let end = (call.span.end() - content_base) as usize;
        let Some(prefix) = inside.get(last..start) else {
            return;
        };
        let mut next = out.clone();
        next.push_str(prefix);
        if !interpolation_splice_preserves_escapes(
            &next,
            &result,
            &inside[end..],
            ctx.lexer_config(),
            ctx.dialect,
        ) {
            continue;
        }
        next.push_str(&result);
        out = next;
        last = end;
        folded_any = true;
    }
    if !folded_any {
        return;
    }
    out.push_str(&inside[last..]);
    ctx.report(Optimisation::new(
        DiagCode::O129,
        "Fold constant builtin command substitution in interpolation",
        rewrite_span,
        format!("\"{out}\""),
    ));
}

/// Substitution boundaries terminate variable-width escapes. No preceding
/// escape may consume bytes across a boundary removed by this splice.
fn interpolation_splice_preserves_escapes(
    prefix: &str,
    result: &str,
    suffix: &str,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&tcl_dialect::DialectProfile>,
) -> bool {
    let selected_escapes = [config.escapes];
    let escapes = if dialect
        .and_then(tcl_dialect::DialectProfile::runtime_version)
        .is_some()
    {
        selected_escapes.as_slice()
    } else {
        tcl_dialect::EscapeSyntax::ALL
    };
    let joined = format!("{prefix}{result}{suffix}");
    escapes.iter().all(|escapes| {
        prefix.match_indices('\\').all(|(start, _)| {
            tcl_syntax::backslash::escape_end_in(&joined, start, *escapes) <= prefix.len()
        })
    })
}

fn visit_simple_var_word(
    ctx: &mut PassContext<'_>,
    span: tcl_lexer::Span,
    text: &str,
    constants: &std::collections::HashMap<String, String>,
    config: tcl_lexer::LexerConfig,
) {
    let Some(name) = whole_scalar_variable_reference_name(text, config) else {
        return;
    };
    let Some(value) = constants.get(name) else {
        return;
    };
    // Re-render a metacharacter-bearing constant as a
    // single self-contained word instead of bailing.
    let Some(word) = render_propagation_word(value, config) else {
        return;
    };
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
    config: tcl_lexer::LexerConfig,
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
    if whole_scalar_variable_reference_name(inside, config).is_some() {
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
    let Some(rewritten) = substitute_dollar_refs(inside, constants, config.nested()) else {
        return;
    };
    if rewritten == inside {
        return;
    }
    // The argv span for a composite `"$a $b $c"` word holds only
    // the opening-quote token (e.g. `"$` at 5..7). Extend to the
    // matching close quote so the rewrite target covers the full
    // string — otherwise we leave trailing `$b $c"` garbage.
    let (rewrite_span, rewritten) =
        if let Some(rewrite_span) = original_quoted_word_rewrite_span(ctx, span, inside) {
            (rewrite_span, rewritten)
        } else {
            // The argv text spells every reference `${name}`; the word as
            // written is what the edit replaces.
            let Some((rewrite_span, written)) =
                super::helpers::spans::original_quoted_word_contents(ctx, span)
            else {
                return;
            };
            match substitute_dollar_refs(&written, constants, config.nested()) {
                Some(rewritten) if rewritten != written => (rewrite_span, rewritten),
                _ => return,
            }
        };
    ctx.report(Optimisation::new(
        DiagCode::O100,
        "Inline constant into string interpolation",
        rewrite_span,
        format!("\"{rewritten}\""),
    ));
}

/// Scan `text` — the inside of a quoted word — for `$name` / `${name}`
/// references and replace each with the corresponding literal from
/// `constants`, rejecting any value that would re-introduce new
/// substitutions. Returns `None` when any reference names something that is
/// not a known constant (a partial substitution would be worse than no
/// substitution), or when the word does not lex.
///
/// The references are the lexer's own `Var` tokens, so only what Tcl
/// substitutes is rewritten: a command substitution is lexed as the script it
/// is, and a braced word in it (`"b:[list {$x}]"`) is a `Str` token whose
/// `$x` stays (#2250), while a `{` inside a quoted word of that script
/// (`"q:[list "foo {$x}"]"`) is literal text and its `$x` is substituted.
/// Unchanged text is copied as `&str` slices, so a multi-byte character keeps
/// its bytes (#2251).
fn substitute_dollar_refs(
    text: &str,
    constants: &std::collections::HashMap<String, String>,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let word = format!("\"{text}\"");
    let mut refs = Vec::new();
    collect_var_refs(&word, config, 0, 0, &mut refs)?;
    refs.sort_unstable_by_key(|r| r.start);
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    for r in refs {
        // Offsets are into `word`, one past `text`'s.
        let (start, end) = (r.start - 1, r.end - 1);
        let value = constants.get(&r.name)?;
        if value.contains(['$', '[', '\\', '"']) {
            return None;
        }
        // Inside a `[…]` command substitution the value becomes a command
        // word, so it must be a single self-contained word — a value with
        // whitespace (a list literal like `tran 1n 100n uic`) would split
        // into multiple arguments. Plain string-text occurrences
        // (`cmd_depth == 0`) inline the value verbatim.
        if r.cmd_depth > 0 && !is_value_safe_bare_word(value) {
            return None;
        }
        out.push_str(text.get(copied..start)?);
        out.push_str(value);
        copied = end;
    }
    out.push_str(text.get(copied..)?);
    Some(out)
}

/// One variable substitution [`substitute_dollar_refs`] may rewrite: its byte
/// range in the lexed word, the variable it names, and how many command
/// substitutions enclose it.
struct VarRef {
    start: usize,
    end: usize,
    name: String,
    cmd_depth: u32,
}

/// Collect the `Var` tokens of `src` (a script or a quoted word, lexed under
/// `config`) at offset `base` into `out`, recursing into each command
/// substitution's script. A braced word is a `Str` token and holds none.
/// `None` when the text does not lex or holds a substitution this rewrite
/// cannot map (Jim's `$(…)`).
fn collect_var_refs(
    src: &str,
    config: tcl_lexer::LexerConfig,
    base: usize,
    cmd_depth: u32,
    out: &mut Vec<VarRef>,
) -> Option<()> {
    use tcl_lexer::TokenType;
    if crate::depth_guard::MAX_BRACKET_TEXT_DEPTH.exceeded(cmd_depth) {
        return None;
    }
    let sm = tcl_lexer::SourceMap::new(src);
    let tokens = tcl_lexer::Lexer::with_config(src, config)
        .tokenise_all()
        .ok()?;
    for tok in tokens {
        let (start, end) = (tok.span.start() as usize, tok.span.end() as usize);
        match tok.kind {
            TokenType::Var => {
                let name = sm.token_text(tok);
                // An element index that substitutes (`$arr($idx)`, `$a([k])`,
                // `$a(\x)`) names the element its value selects, not the one
                // its spelling does, so it is left as written.
                if tok.content_offset != 2
                    && tcl_syntax::naming::split_element_ref(name)
                        .is_some_and(|(_, index)| index.contains(['$', '[', '\\']))
                {
                    continue;
                }
                // A `${name}` span stops before its closing brace, but the
                // empty `${}` span already covers it.
                let close =
                    usize::from(tok.content_offset == 2 && src.get(start..end) != Some("${}"));
                out.push(VarRef {
                    start: base + start,
                    end: base + end + close,
                    name: name.to_owned(),
                    cmd_depth,
                });
            }
            TokenType::Cmd => {
                let inner = start + usize::from(tok.content_offset);
                collect_var_refs(
                    sm.token_text(tok),
                    config.nested(),
                    base + inner,
                    cmd_depth + 1,
                    out,
                )?;
            }
            TokenType::ExprSugar => return None,
            _ => {}
        }
    }
    Some(())
}

/// Allow a literal to be inlined as a bare word if it is
/// decimal-integer-shaped or matches the safe-word grammar.
/// Rejects anything containing Tcl metacharacters that would
/// introduce new substitutions.
fn is_value_safe_bare_word(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if value.parse::<i64>().is_ok() {
        return true;
    }
    is_safe_word(value)
}

/// Construct one literal source word under the caller's complete configuration.
/// The shared Syntax owner verifies its decoded value and complete word shape;
/// failure supplies no replacement. Evaluation and erasure proofs stay separate.
fn render_propagation_word(value: &str, config: tcl_lexer::LexerConfig) -> Option<String> {
    tcl_syntax::word_rules::lexical_unicode_source_word(value, config)
}

/// The SCCP-proved constant for one exact SSA value, rendered as source text.
///
/// The per-value counterpart of [`sccp_constants_for`]'s name-keyed map: that
/// projection answers "is this *name* always this constant", which is the
/// question a by-name source substitution has to ask, and it necessarily
/// abstains for a name whose versions hold different constants. A def-use
/// consumer already knows which version it is forwarding, so it can ask the
/// stronger question and get an answer where the map has none.
fn sccp_value_literal(
    fu: &FunctionUnit,
    var_name: &crate::var_resolve::VariableCellKey,
    version: crate::ssa::Version,
    policy: FoldPolicy,
) -> Option<String> {
    use super::helpers::literals::format_constant_with_policy;

    let sym = fu.ssa.cell_symbol(var_name)?;
    if !fu.sccp.materialises((sym, version)) {
        return None;
    }
    match fu.sccp.values.get(&(sym, version))? {
        LatticeValue::Const(cv) => format_constant_with_policy(cv, policy),
        _ => None,
    }
}

pub(super) fn sccp_constants_for(
    fu: &FunctionUnit,
    policy: FoldPolicy,
) -> std::collections::HashMap<String, String> {
    sccp_constants_from(&fu.sccp, &fu.ssa, policy)
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
    policy: FoldPolicy,
) -> std::collections::HashMap<String, String> {
    use super::helpers::literals::format_constant_with_policy;

    let mut per_var: std::collections::HashMap<crate::ssa::Symbol, Vec<&ConstValue>> =
        std::collections::HashMap::new();
    let mut dirty: std::collections::HashSet<crate::ssa::Symbol> = std::collections::HashSet::new();
    for (&(sym, ver), lv) in &sccp.values {
        if dirty.contains(&sym) {
            continue;
        }
        if let LatticeValue::Const(cv) = lv
            && sccp.materialises((sym, ver))
        {
            per_var.entry(sym).or_default().push(cv);
        } else {
            dirty.insert(sym);
            per_var.remove(&sym);
        }
    }
    let mut out = std::collections::HashMap::new();
    for (sym, cvs) in per_var {
        let first = cvs[0];
        if !cvs.iter().all(|cv| *cv == first) {
            continue;
        }
        let display = ssa.var_name(sym);
        if ssa.var_symbol(display) == Some(sym)
            && let Some(text) = format_constant_with_policy(first, policy)
        {
            out.insert(display.to_owned(), text);
        }
    }
    out
}

#[cfg(test)]
mod tests;
