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

//! O102: forwarding a single reaching literal definition to its operand use sites.

use super::super::{Optimisation, PassContext};
use super::{is_value_safe_bare_word, sccp_value_literal};
use crate::compilation_unit::FunctionUnit;
use crate::ir::Statement;
use crate::tcl_expr_eval::FoldPolicy;
use tcl_core_types::DiagCode;
use tcl_syntax::naming::whole_scalar_variable_reference_name;

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
            trace.writes_any_variable_in(&fu.cfg, &|block| {
                fu.sccp.executable_blocks.contains(block)
            }),
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
pub(super) fn cell_is_linked_outside_frame(fu: &FunctionUnit, symbol: crate::ssa::Symbol) -> bool {
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
pub(super) fn run_load_forwarding(
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
    pub(super) code: DiagCode,
    pub(super) symbol: crate::ssa::Symbol,
    pub(super) version: crate::ssa::Version,
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
