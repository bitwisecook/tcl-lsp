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

//! One block of the value solver: its statements' stores and clobbers, its terminator's executable edges and the constant branches.

use super::super::{BuiltinFoldInputs, ConstantBranch, join, set_value};
use super::assignment_folds::parse_literal_value;
use super::branch_decisions::branch_decision;
use super::definition_values::evaluate_def_with_math;
use super::math_expressions::{env_from_uses, fold_math_expression};
use super::value_solver::branch_deferrable;
use super::{
    BranchFold, ExpressionEvaluationPoint, MathDependencies, MathFoldContext, SolverPurpose,
    StatementInputs, StoreInputs, TerminatorInputs,
};
use crate::analyses::LatticeValue;
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::ir::Statement;
use crate::math_function_binding::ExpressionMathBindings;
use crate::ssa::{SsaFunction, SsaSourceView, SsaStatement, ValueKey};
use std::collections::{HashMap, HashSet};
use tcl_registry::CommandRegistry;

fn observed_store_keys(
    ssa: &SsaFunction,
    block: BlockId,
    index: usize,
    statement: &Statement,
    registry: &CommandRegistry,
) -> HashSet<crate::var_resolve::VariableCellKey> {
    let Some(points) = &ssa.point_contexts else {
        return HashSet::new();
    };
    crate::place_bridge::def_places_at(statement, block, index, points, registry)
        .into_iter()
        .filter(|place| place.observed)
        .filter_map(|place| crate::var_resolve::canonical_binding_value_key(&place))
        .collect()
}

fn statement_math_context<'a>(
    cfg: Option<&'a CfgFunction>,
    block: BlockId,
    index: usize,
    statement: &Statement,
    dependencies: Option<&'a MathDependencies>,
) -> Option<MathFoldContext<'a>> {
    let cfg = cfg?;
    let base = match statement {
        Statement::AssignExpr { expr_base, .. } | Statement::ExprEval { expr_base, .. } => {
            *expr_base
        }
        _ => None,
    };
    Some(MathFoldContext {
        point: Some(ExpressionEvaluationPoint::Statement { block, index }),
        dependencies,
        incoming_reads: None,
        bindings: ExpressionMathBindings::for_origin(
            &cfg.implicit_math_invocations,
            cfg.statement_sources
                .get(&(block, index))
                .and_then(Option::as_deref),
            base,
        )
        .with_preparations(&cfg.expression_preparations),
    })
}

/// Widen only fresh scalar keys; original stores and read versions remain intact.
fn widen_registry_clobbers(
    values: &mut HashMap<ValueKey, LatticeValue>,
    clobbers: Option<&crate::ssa::BlockValueClobbers>,
    index: usize,
    folds: Option<BuiltinFoldInputs<'_>>,
) -> bool {
    let mut changed = false;
    for (&var, &(prior, fresh)) in clobbers
        .and_then(|markers| markers.get(&index))
        .into_iter()
        .flatten()
    {
        let value = if folds.is_some_and(|f| f.proven_pure_parameters)
            && matches!(values.get(&(var, 0)), Some(LatticeValue::Const(_)))
        {
            values
                .get(&(var, prior))
                .cloned()
                .unwrap_or(LatticeValue::Unknown)
        } else {
            LatticeValue::Overdefined
        };
        changed |= set_value(values, (var, fresh), &value);
    }
    changed
}

fn widen_unrepresented_statement(
    values: &mut HashMap<ValueKey, LatticeValue>,
    stmt_ssa: &SsaStatement,
    positioned: bool,
) -> bool {
    let mut changed = false;
    let keys: Vec<ValueKey> = if positioned {
        // Exact reads after an unrepresented store have no contents
        // version. Earlier SSA versions remain immutable evidence.
        Vec::new()
    } else {
        values.keys().copied().collect()
    };
    for k in keys {
        if k.1 != 0 && set_value(values, k, &LatticeValue::Overdefined) {
            changed = true;
        }
    }
    // A barrier also *defines* variables of its own (e.g. `dict for {x
    // y} …` defines `x`/`y`). Those defs are opaque — the barrier can
    // set them to anything — so set each to `Overdefined`. Without this
    // the def key is never inserted (the widen loop above only touches
    // keys already present), so it stays `Unknown` and vanishes from a
    // downstream phi join, letting a phi that merges a barrier-def with
    // a constant fold to that constant and miscompile a following test.
    for (&var, ver) in &stmt_ssa.defs {
        if set_value(values, (var, *ver), &LatticeValue::Overdefined) {
            changed = true;
        }
    }
    changed
}

pub(super) fn sccp_process_statements(
    values: &mut HashMap<ValueKey, LatticeValue>,
    ssa_block: &crate::ssa::SsaBlock,
    ssa: &SsaFunction,
    inputs: StatementInputs<'_>,
) -> bool {
    let StatementInputs { store, clobbers } = inputs;
    let StoreInputs {
        reference_config,
        math_dependencies,
        cfg,
        escaping,
        policy,
        has_dynamic_variable_trace,
        opaque_native_accesses,
        folds,
        registry,
    } = *store;
    let mut changed = false;
    let block = ssa.block_id(&ssa_block.name).unwrap_or(BlockId(u32::MAX));
    for (index, stmt_ssa) in ssa_block.statements.iter().enumerate() {
        // Statement defs become live after its inputs and barrier effect.
        let registry_barrier = stmt_ssa.statement.synthetic_marker()
            == Some(crate::ir::SyntheticMarker::RegistryBarrier);
        if registry_barrier {
            changed |= widen_registry_clobbers(values, clobbers, index, folds);
            continue;
        }
        let lookup = if ssa.point_contexts.is_some() {
            SsaSourceView::at_statement(ssa, block, index)
        } else {
            SsaSourceView::unpositioned(ssa)
        };
        if matches!(
            stmt_ssa.statement,
            Statement::Barrier { .. } | Statement::NativeCall { .. } | Statement::UpFrame { .. }
        ) {
            // Executable barriers widen tracked values except version-0
            // parameter seeds. Registry boundaries use fresh versions above,
            // preserving every earlier proof. Ordinary barrier seeds hold the caller's
            // literal and are immutable across the barrier (a barrier
            // that mutates the var produces a fresh version), so a
            // callee `dict with $param` still sees the interproc
            // literal.
            //
            // `UpFrame` (the CFG shape for a literal-body `uplevel`)
            // shares this treatment: `uplevel 1 {…}` / `uplevel #0 {…}`
            // evaluates its body in a DIFFERENT frame — the caller's, or
            // the absolute global one — so it can reassign any name
            // visible there, exactly like an opaque barrier. Reproduced
            // against tclsh 8.6/9.0: `set n 5; uplevel #0 {set n 99};
            // puts [expr {$n + 1}]` prints `100`. The value before
            // `uplevel` cannot justify folding that later expression.
            if stmt_ssa.statement.is_opaque_script_body() {
                changed |= widen_registry_clobbers(values, clobbers, index, None);
            }
            changed |=
                widen_unrepresented_statement(values, stmt_ssa, ssa.point_contexts.is_some());
            continue;
        }
        let observed = observed_store_keys(ssa, block, index, &stmt_ssa.statement, registry);
        let math =
            statement_math_context(cfg, block, index, &stmt_ssa.statement, math_dependencies)
                .map(|context| context.with_incoming_reads(lookup, registry));
        if math_dependencies.is_some_and(|ledger| ledger.purpose == SolverPurpose::SemanticAnalysis)
            && let Statement::ExprEval { expr, .. } = &stmt_ssa.statement
        {
            let environment = env_from_uses(&stmt_ssa.uses, values, lookup);
            let _ = fold_math_expression(expr, &environment, policy, math);
        }
        for (&var, ver) in &stmt_ssa.defs {
            let evaluated = || {
                evaluate_def_with_math(
                    stmt_ssa,
                    &*values,
                    lookup,
                    policy,
                    folds,
                    math,
                    reference_config,
                )
            };
            let val = if ssa.cell_key(var).is_none_or(|cell| observed.contains(cell))
                || lookup.externally_mutable_given(
                    var,
                    escaping,
                    has_dynamic_variable_trace,
                    opaque_native_accesses,
                    registry,
                ) != Some(false)
                || ssa.is_array_root_refresh_version(var, *ver)
                || stmt_ssa.destruction_defs.contains(&var)
            {
                LatticeValue::Overdefined
            } else if math_dependencies
                .is_some_and(|ledger| ledger.purpose == SolverPurpose::SemanticAnalysis)
                && let Some(value) = lookup.conditional_logical_scalar_store_contents(var, registry)
            {
                LatticeValue::Const(parse_literal_value(&value))
            } else if stmt_ssa.may_defs.contains(&var) {
                // A synthetic array-element may-def: the write may or may
                // not have hit this element, so its value is the JOIN of
                // the prior version (recorded as a use) and the written
                // value. The base refresh of an element write carries no
                // prior use — the base holds no value of its own.
                match stmt_ssa.uses.get(&var) {
                    Some(prev_ver) => {
                        let prev = values
                            .get(&(var, *prev_ver))
                            .cloned()
                            .unwrap_or(LatticeValue::Overdefined);
                        join(&prev, &evaluated())
                    }
                    None => LatticeValue::Overdefined,
                }
            } else {
                evaluated()
            };
            if set_value(values, (var, *ver), &val) {
                changed = true;
            }
        }
    }
    changed
}

/// Process a block's terminator: mark the matching outgoing edges
/// as executable.  Returns `true` when any new edge / block was
/// added.  Used by [`execution_value_facts`].
pub(super) fn sccp_process_terminator(
    bn: BlockId,
    inputs: &TerminatorInputs<'_>,
    executable_blocks: &mut HashSet<BlockId>,
    executable_edges: &mut HashSet<(BlockId, BlockId)>,
    finalizing: bool,
) -> bool {
    let TerminatorInputs {
        math_dependencies,
        cfg,
        ssa,
        values,
        policy,
        grammar,
        registry,
        reference_config,
    } = *inputs;
    let mut changed = false;
    let Some(block) = cfg.blocks.get(&bn) else {
        return false;
    };
    let Some(term) = &block.terminator else {
        return false;
    };
    match term {
        Terminator::Goto { target, .. } => {
            let edge = (bn, *target);
            if !executable_edges.contains(&edge) {
                executable_edges.insert(edge);
                changed = true;
            }
            if cfg.blocks.contains_key(target) && executable_blocks.insert(*target) {
                changed = true;
            }
        }
        Terminator::Branch {
            condition,
            true_target,
            false_target,
            ..
        } => {
            let Some(ssa_block) = ssa.blocks.get(&bn) else {
                return changed;
            };
            let decision = branch_decision(
                cfg,
                ssa,
                bn,
                ssa_block,
                condition,
                values,
                BranchFold {
                    math_dependencies,
                    registry,
                    policy,
                    grammar,
                    reference_config,
                },
            );
            let targets: Vec<BlockId> = match decision {
                Some(true) => vec![*true_target],
                Some(false) => vec![*false_target],
                // Optimistic (Wegman–Zadeck): while the condition may still
                // fold on a later sweep (a not-yet-computed operand, no
                // `Overdefined` one), open neither arm and let the fixpoint
                // retry — this is what detects loop-carried constant
                // conditions instead of pessimistically opening both arms
                // forever. The finalising pass forces both arms for any
                // branch still stuck this way.
                None if !finalizing
                    && branch_deferrable(ssa_block, condition, values, ssa, grammar) =>
                {
                    Vec::new()
                }
                None => vec![*true_target, *false_target],
            };
            for tgt in targets {
                let edge = (bn, tgt);
                if !executable_edges.contains(&edge) {
                    executable_edges.insert(edge);
                    changed = true;
                }
                if cfg.blocks.contains_key(&tgt) && executable_blocks.insert(tgt) {
                    changed = true;
                }
            }
        }
        Terminator::Return { .. } | Terminator::Complete { .. } => {}
    }
    // `try` exception edges sourced at `bn`: when `bn` is executable the
    // handler is reachable (a throw can occur in the body).
    for (from, to) in cfg.exception_edges.iter().chain(&cfg.analysis_edges) {
        if *from != bn {
            continue;
        }
        let edge = (bn, *to);
        if executable_edges.insert(edge) {
            changed = true;
        }
        if cfg.blocks.contains_key(to) && executable_blocks.insert(*to) {
            changed = true;
        }
    }
    changed
}

/// Post-fixpoint sweep that records every reachable branch whose
/// condition evaluated to a constant lattice value.  Used by
/// [`execution_value_facts`].
pub(super) fn collect_constant_branches(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &HashMap<ValueKey, LatticeValue>,
    executable_blocks: &HashSet<BlockId>,
    order: &[BlockId],
    fold: BranchFold<'_>,
) -> Vec<ConstantBranch> {
    let mut constant_branches: Vec<ConstantBranch> = Vec::new();
    for bn in order {
        if !executable_blocks.contains(bn) {
            continue;
        }
        let Some(block) = cfg.blocks.get(bn) else {
            continue;
        };
        let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            span: term_span,
            ..
        }) = &block.terminator
        else {
            continue;
        };
        let Some(ssa_block) = ssa.blocks.get(bn) else {
            continue;
        };
        let decision = branch_decision(cfg, ssa, *bn, ssa_block, condition, values, fold);
        let cond_text = crate::expr_ast::expr_text(condition);
        let (true_name, false_name) = (
            cfg.block_name(*true_target).to_owned(),
            cfg.block_name(*false_target).to_owned(),
        );
        match decision {
            Some(true) => constant_branches.push(ConstantBranch {
                kind: super::super::BranchFactKind::Applied,
                block: cfg.block_name(*bn).to_owned(),
                span: *term_span,
                condition: cond_text,
                value: true,
                taken_target: true_name,
                not_taken_target: false_name,
            }),
            Some(false) => constant_branches.push(ConstantBranch {
                kind: super::super::BranchFactKind::Applied,
                block: cfg.block_name(*bn).to_owned(),
                span: *term_span,
                condition: cond_text,
                value: false,
                taken_target: false_name,
                not_taken_target: true_name,
            }),
            None => {}
        }
    }
    constant_branches
}
