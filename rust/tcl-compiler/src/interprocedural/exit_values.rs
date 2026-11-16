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

//! The value a pure procedure returns under its own seedless lattice, read at every exit of its flow graph.

use super::SeedlessUnits;
use super::return_summaries::statement_may_return;
use super::transfer::ModuleProcedures;
use crate::ir::ProcedureId;
use std::collections::{HashMap, HashSet};
use tcl_registry::value_transfer::ExactValue;

/// The summaries' second stage: each pure procedure's own lattice, run with
/// its parameters unknown — no call-site seed, so a value exact only under
/// the literal every caller passes never enters a summary — under the
/// whole-module trust a rewrite folds under, and read at every way it
/// returns ([`exit_value`]). `None` for a procedure whose exits prove no
/// one value. It follows the purity fixpoint, which decides whose returns
/// may fold at all, and reads no summary: the driver takes a call to a
/// procedure of the module for a command it cannot see, so one stage is the
/// fixed point, and a return that passes through a recursive call is
/// computed.
pub(super) fn seedless_returns(
    ir_module: &crate::ir::Module,
    units: SeedlessUnits<'_>,
    pure: &HashMap<ProcedureId, bool>,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> HashMap<ProcedureId, Option<ExactValue>> {
    let module = ModuleProcedures::of_unit(units.unit, registry);
    units
        .procedures
        .iter()
        .filter(|(id, fu)| {
            ir_module.procedures.contains_key(id)
                && pure.get(id).copied().unwrap_or(false)
                && !fu.complexity_guarded
        })
        .map(|(qname, fu)| {
            let source_metadata_input = fu
                .invocation_metadata_context_for_module(registry, ir_module)
                .and_then(
                    crate::registry_invocation::InvocationMetadataContext::source_analysis_input,
                );
            if source_metadata_input.is_none() && !fu.cfg.metadata_context.is_standalone() {
                return (*qname, None);
            }
            let dialect = source_metadata_input
                .map(crate::analyser::ResolvedAnalysisInput::unit_profile)
                .or_else(|| {
                    fu.cfg
                        .metadata_context
                        .is_standalone()
                        .then_some(dialect)
                        .flatten()
                });
            let policy = crate::tcl_expr_eval::FoldPolicy::for_profile(
                dialect.and_then(crate::tcl_expr_eval::leading_zero_is_octal),
                dialect,
            );
            let reading = ExitReading {
                policy,
                grammar: fu.source_lexer_config().grammar_over(
                    dialect.map_or_else(tcl_dialect::LexerGrammar::default, |p| p.grammar),
                ),
                module: None,
                folds: crate::sccp::BuiltinFoldInputs {
                    registry,
                    source_metadata_input,
                    mutations: units.mutations,
                    dialect,
                    defining_class: None,
                    registry_engine: false,
                    trust: crate::sccp::FoldTrust::WholeModule,
                    proven_pure_parameters: false,
                },
            };
            let result = crate::sccp::sccp_in_module(&crate::sccp::SolveInputs {
                cfg: &fu.cfg,
                ssa: &fu.ssa,
                param_constants: None,
                policy,
                extra_escaping: &HashSet::new(),
                trace: crate::sccp::TraceInputs {
                    registry,
                    source_metadata_input,
                    traced_variables: &ir_module.traced_variables,
                    has_dynamic_variable_trace: ir_module.has_dynamic_variable_trace,
                    deferred_writes: &ir_module.deferred_writes,
                    analysis_context: None,
                    existence: None,
                },
                folds: Some(reading.folds),
                module: crate::sccp::ModuleRun {
                    analysis_body: Some(crate::sccp::AnalysisBody::Procedure(*qname)),
                    reads_exits: true,
                    ..crate::sccp::ModuleRun::reading(Some(&module))
                },
            });
            (*qname, exit_value(ExitBody::of(fu), &result, reading))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The value a procedure returns, read under a lattice

/// What an exit's value is read under: the expression route's value
/// semantics, the grammar the return's word and an expression's variables
/// are read with, and the rewrite's registry, mutation facts and
/// whole-module trust.
#[derive(Clone, Copy)]
pub(crate) struct ExitReading<'a> {
    /// The value semantics.
    pub(crate) policy: crate::tcl_expr_eval::FoldPolicy,
    /// The document's grammar.
    pub(crate) grammar: tcl_dialect::LexerGrammar,
    /// The registry, mutation facts and trust stance an expression is
    /// evaluated under.
    pub(crate) folds: crate::sccp::BuiltinFoldInputs<'a>,
    /// The module's procedures and the function the exits are read in,
    /// where a re-run reads them: a command a return's expression runs that
    /// names a procedure of the module is re-run for its result.
    pub(crate) module: Option<(&'a ModuleProcedures<'a>, &'a ProcedureId)>,
}

/// The body an exit reading reads: a flow graph and its SSA.
#[derive(Clone, Copy)]
pub(crate) struct ExitBody<'b> {
    /// The flow graph.
    pub(crate) cfg: &'b crate::cfg::Function,
    /// Its SSA.
    pub(crate) ssa: &'b crate::ssa::SsaFunction,
}

impl<'b> ExitBody<'b> {
    /// A unit's body.
    pub(crate) const fn of(unit: &'b crate::compilation_unit::FunctionUnit) -> Self {
        Self {
            cfg: &unit.cfg,
            ssa: &unit.ssa,
        }
    }
}

/// The one value every way `fu` returns gives under `result`, or `None`.
///
/// Every reachable exit must give the same value: an explicit `return`,
/// **or** a reachable fall-through to the function's implicit exit (a block
/// with no terminator: Tcl's "the result of the last command executed" rule
/// for a proc that runs off the end of its body without a `return` on that
/// path). Ignoring the fall-through would let a proc with `if {…} { return K
/// }` plus a trailing statement fold to `K` when the fall-through path is
/// also reachable and gives something else — a miscompile, not just a missed
/// optimisation (confirmed against tclsh 9.0.4: a proc whose `if` condition
/// is not foldable leaves both paths executable). A void return, an exit
/// that does not fold, two exits that disagree, or a statement of an
/// executable block that may itself run a `return` ([`statement_may_return`],
/// #2393) give `None`.
///
/// The one reading O103's argument-sensitive re-run and the summary's
/// seedless run share. A literal word is read through the exact value
/// ingress — a braced word its content, a bare or quoted one its escapes
/// decoded, nothing trimmed — so `return " 5"` is the two-character string.
///
/// The run is one made to read how the procedure completes
/// ([`crate::sccp::ModuleRun::reads_exits`]). A statement it proved raises
/// where no handler takes the throw, or a call whose completion it could
/// not decide, means some call may not reach an exit at all, so no exit's
/// value is the call's: `None`.
pub(crate) fn exit_value(
    fu: ExitBody<'_>,
    result: &crate::sccp::SccpResult,
    reading: ExitReading<'_>,
) -> Option<ExactValue> {
    use crate::cfg::Terminator;
    if !result.completion.is_decided() {
        return None;
    }
    if fu.cfg.blocks.iter().any(|(bn, block)| {
        result.executable_blocks.contains(bn)
            && block
                .statements
                .iter()
                .any(|stmt| statement_may_return(stmt, reading.folds.registry, 0))
    }) {
        return None;
    }
    let preds = fu.cfg.predecessors();
    let mut found: Option<ExactValue> = None;
    for (bn, block) in &fu.cfg.blocks {
        if !result.executable_blocks.contains(bn) {
            continue;
        }
        let value = match &block.terminator {
            Some(Terminator::Return {
                value,
                braced,
                expr,
                ..
            }) => return_value(
                fu,
                *bn,
                (value.as_deref(), *braced, expr.as_ref()),
                result,
                reading,
            )?,
            None => fallthrough_value(fu, *bn, result, &preds, reading)?,
            Some(_) => continue, // Goto / Branch — not an exit point
        };
        match &found {
            None => found = Some(value),
            Some(prev) if prev.bytes == value.bytes => {}
            Some(_) => return None, // reachable exits disagree
        }
    }
    found
}

/// The value a `return`'s word gives at block `bn`: a literal through the
/// exact ingress, `$name` as the version there holds it, an `expr` on the
/// shared expression route; a bare `return` (no word) gives none.
fn return_value(
    fu: ExitBody<'_>,
    bn: crate::cfg::BlockId,
    (value, braced, expr): (Option<&str>, bool, Option<&crate::expr_ast::ExprNode>),
    result: &crate::sccp::SccpResult,
    reading: ExitReading<'_>,
) -> Option<ExactValue> {
    let word = value?;
    if let Some(node) = expr {
        return expr_value(fu, bn, node, result, reading);
    }
    let config = tcl_lexer::LexerConfig::from_grammar(reading.grammar);
    if let Some(text) = crate::value_transfer::recorded_word_value(word, braced, &config) {
        return Some(ExactValue::from_literal(&text));
    }
    var_value(
        fu,
        bn,
        crate::value_shapes::whole_word_scalar_var_name(word, config)?,
        result,
    )
}

/// The value Tcl's implicit-return rule leaves when control falls through
/// block `bn` — the function's synthesised exit sink (a reachable block
/// with no terminator).
///
/// Trusts ONLY the narrow, unambiguous shape: `bn` has exactly one
/// executable predecessor, and that predecessor's OWN last statement is a
/// recognised value-producing tail (see [`tail_value`]). Deliberately does
/// NOT walk through an empty predecessor to whatever precedes *it*: an empty
/// block reached via a control-flow edge is not "no Tcl command ran here" —
/// it is frequently the empty **body** of a real command (`if {$c} {}`, or
/// the implicit `""` an `if` with no `else` produces when the condition is
/// false), whose own result is the empty string, not whatever ran before the
/// branch. Block shape alone can't soundly distinguish that from a genuine
/// structural join, so any empty predecessor — or more than one live
/// predecessor at all — bails to `None` rather than risk inheriting a stale
/// prior value. (A more permissive, recursive version shipped briefly and
/// mis-folded `proc f {c} { set x 1; if {$c} {} }`'s `[f 0]` to `1` instead
/// of the correct `""` — confirmed against tclsh 9.0.4 — by walking straight
/// through the empty `if`-body block back to the preceding `set x 1`.)
fn fallthrough_value(
    fu: ExitBody<'_>,
    bn: crate::cfg::BlockId,
    result: &crate::sccp::SccpResult,
    preds: &HashMap<crate::cfg::BlockId, HashSet<crate::cfg::BlockId>>,
    reading: ExitReading<'_>,
) -> Option<ExactValue> {
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
    tail_value(fu, *pred, last, result, reading)
}

/// The value Tcl's "result of the last executed command" rule leaves when
/// `stmt` is the last statement of a block that falls through to the
/// function's implicit exit — a trailing `set` / `incr` implicitly returns
/// exactly like `return $name` would (Tcl's `set` and `incr` both return the
/// value they just assigned), a trailing call whose resolved plan is a cell
/// update (`append`, `lappend`) returns the cell's new value the same way,
/// and a trailing bare `expr` implicitly returns exactly like `return [expr
/// {…}]` would. `None` for any other statement shape (a bare command call
/// whose own result this analysis doesn't track, …) — the caller simply
/// won't fold that path, never mis-folds it.
fn tail_value(
    fu: ExitBody<'_>,
    bn: crate::cfg::BlockId,
    stmt: &crate::ir::Statement,
    result: &crate::sccp::SccpResult,
    reading: ExitReading<'_>,
) -> Option<ExactValue> {
    use crate::ir::Statement;
    match stmt {
        Statement::ExprEval { expr, .. } => expr_value(fu, bn, expr, result, reading),
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::Incr { name, .. } => var_value(fu, bn, name, result),
        // A cell update's result is the value it wrote: the registry's
        // declared plan names the target, and the lattice at the block's
        // exit holds what it wrote.
        Statement::Call {
            command,
            canonical_command,
            args,
            ..
        } => {
            let head = canonical_command.as_deref().unwrap_or(command);
            let (_, target) =
                crate::value_transfer::resolved_cell_update(reading.folds.registry, head, args)?;
            var_value(fu, bn, args.get(target.0)?, result)
        }
        _ => None,
    }
}

/// The value `name` holds at block `bn`'s *exit* version — immediately
/// after `bn`'s own statements have run. This MUST use the exit version
/// precisely: a loop-carried var (`return $total` after a `foreach`) is a
/// phi whose exit value is Overdefined, even though an earlier `set total 0`
/// left a stale Const(0) under another version. Reading the precise version
/// is what makes the reading bail on `sum_list` / `fibonacci` instead of
/// mis-folding to the pre-loop value. The value is the one the version holds
/// at `bn` ([`crate::sccp::SccpResult::value_at`]), so the state an
/// enumerated loop leaves, in force past it, is read; a value a route
/// constructed rather than read from the source is never one
/// ([`crate::sccp::SccpResult::materialises`]).
fn var_value(
    fu: ExitBody<'_>,
    bn: crate::cfg::BlockId,
    name: &str,
    result: &crate::sccp::SccpResult,
) -> Option<ExactValue> {
    let sym = fu.ssa.var_symbol(name)?;
    let ver = fu
        .ssa
        .blocks
        .get(&bn)
        .and_then(|b| b.exit_versions.get(&sym).copied())
        .unwrap_or(0);
    match result.value_at(bn, (sym, ver)) {
        Some(crate::analyses::LatticeValue::Const(c)) if result.materialises((sym, ver)) => {
            Some(crate::value_transfer::const_to_exact(c))
        }
        _ => None,
    }
}

/// `expr` evaluated under the lattice for block `bn`'s exit environment.
/// Built FLOW-SENSITIVELY: each variable the expression references is bound
/// at *this block's exit version* (the precise state reaching this point),
/// and only when that version is a lattice constant at `bn`
/// ([`crate::sccp::SccpResult::value_at`]). A variable absent from
/// `exit_versions` (a never-reassigned parameter) falls back to version 0,
/// where a seeded parameter's constant lives.
///
/// The flow-INsensitive alternative ("every Const lattice entry, preferring
/// the newest version, then overlay exit versions") miscompiled: for `set x
/// 0; foreach v {…} { set x $v }; return [expr {$x + 1}]`, `x`'s exit
/// version is a non-Const loop phi, so the overlay didn't override, and the
/// stale pre-loop `(x,1)=Const(0)` leaked in — folding to `1` where tclsh
/// returns `3`. Reading the exit version (Overdefined here) leaves `x`
/// unbound so the route declines, as [`var_value`] does.
///
/// The expression runs on the shared expression route
/// ([`crate::value_transfer::evaluate_expression_detached`]) under the
/// rewrite's whole-module trust, so a return fold proves what the lattice
/// proves: no rebound math function, no function the target lacks, no value
/// past the target's integer tower.
fn expr_value(
    fu: ExitBody<'_>,
    bn: crate::cfg::BlockId,
    expr: &crate::expr_ast::ExprNode,
    result: &crate::sccp::SccpResult,
    reading: ExitReading<'_>,
) -> Option<ExactValue> {
    let mut constants: HashMap<String, ExactValue> = HashMap::new();
    if let Some(ssa_block) = fu.ssa.blocks.get(&bn) {
        for name in crate::var_refs::vars_in_expr(expr, reading.grammar) {
            let Some(sym) = fu.ssa.var_symbol(&name) else {
                continue;
            };
            let ver = ssa_block.exit_versions.get(&sym).copied().unwrap_or(0);
            if let Some(crate::analyses::LatticeValue::Const(c)) = result.value_at(bn, (sym, ver)) {
                constants.insert(
                    fu.ssa.var_name(sym).to_owned(),
                    crate::value_transfer::const_to_exact(c),
                );
            }
        }
    }
    crate::value_transfer::evaluate_expression_in_module(
        expr,
        &constants,
        (reading.folds, reading.policy),
        reading.module,
    )
}
