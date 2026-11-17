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

//! Terminators: which outgoing edges are executable, and the constant branches and settled statement records the post-passes collect.

use super::super::evaluation::evaluate_branch_under;
use super::super::{
    BranchFactKind, BranchFold, ConstantBranch, SelectionRecord, TemplatePlanRecord,
};
use super::TerminatorInputs;
use super::refinements::{Narrowing, restore_values};
use crate::analyses::LatticeValue;
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::ssa::{SsaFunction, ValueKey};
use crate::value_transfer::LatticeDriver;
use std::collections::{HashMap, HashSet};
use tcl_registry::value_transfer::Existence;

/// Optimistic (Wegman–Zadeck) deferral test for a non-constant branch.
///
/// Returns `true` when the condition *may still fold* on a later sweep:
/// some operand defined in this function (SSA exit-version > 0) is still
/// `Unknown` (not yet computed) and none is `Overdefined`. Such a branch
/// opens neither arm until either the operand resolves to a constant or an
/// `Overdefined` operand proves the condition genuinely non-constant.
///
/// Operands read at version 0 (proc parameters, globals, and other
/// live-in roots) are excluded: [`seed_live_in_roots`] already seeds them
/// `Overdefined`, so they are never "not yet computed".
pub(in super::super) fn branch_deferrable(
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue>,
    ssa: &SsaFunction,
    grammar: tcl_dialect::LexerGrammar,
) -> bool {
    let mut any_operand = false;
    let mut any_unknown = false;
    for name in crate::var_refs::vars_in_expr(condition, grammar) {
        let Some(sym) = ssa.var_symbol(&name) else {
            continue;
        };
        let ver = ssa_block.exit_versions.get(&sym).copied().unwrap_or(0);
        if ver == 0 {
            continue;
        }
        any_operand = true;
        match values.get(&(sym, ver)) {
            Some(LatticeValue::Overdefined) => return false,
            Some(LatticeValue::Unknown) | None => any_unknown = true,
            _ => {}
        }
    }
    any_operand && any_unknown
}

/// Process a block's terminator: mark the matching outgoing edges
/// as executable.  Returns `true` when any new edge / block was
/// added.  Extracted from [`sccp`].
pub(in super::super) fn sccp_process_terminator(
    bn: BlockId,
    inputs: &TerminatorInputs<'_>,
    executable_blocks: &mut HashSet<BlockId>,
    executable_edges: &mut HashSet<(BlockId, BlockId)>,
    finalizing: bool,
) -> bool {
    let TerminatorInputs {
        cfg,
        ssa,
        values,
        grammar,
        driver,
        exit,
        entry_edges,
    } = *inputs;
    let mut changed = false;
    let Some(block) = cfg.blocks.get(&bn) else {
        return false;
    };
    let Some(term) = &block.terminator else {
        return false;
    };
    // A block that certainly raises never reaches its successors: only its
    // exception edges leave it.
    match term {
        _ if !exit.completes => {}
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
            span,
            condition_base,
            ..
        } => {
            let Some(ssa_block) = ssa.blocks.get(&bn) else {
                return changed;
            };
            driver.explaining(*span);
            let decision = evaluate_branch_under(
                ssa_block,
                condition,
                values,
                ssa,
                BranchFold { driver },
                cfg.source_tokens_at(bn, usize::MAX).zip(*condition_base),
            );
            driver.explaining(None);
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
        // Observed completion has no normal successor and performs no second
        // operand evaluation. Original block exception edges remain below.
        Terminator::Return { .. } | Terminator::Complete { .. } => {}
    }
    // An analysis edge leaves a block that completes normally for a target
    // its terminator does not name: the exit of a possible body region, which
    // a loop leaves when its list is exhausted and a wrapper reaches when its
    // handler declines the body. Without it, `puts $n` after `foreach v [incr
    // n] {}` was unreachable and O107 deleted it (tclsh 8.4.20 prints `2`).
    if exit.completes {
        for &(from, to) in &cfg.analysis_edges {
            if from != bn {
                continue;
            }
            if executable_edges.insert((bn, to)) {
                changed = true;
            }
            if cfg.blocks.contains_key(&to) && executable_blocks.insert(to) {
                changed = true;
            }
        }
    }
    // `try` exception edges sourced at `bn`: when `bn` is executable the
    // handler is reachable (a throw can occur in the body).
    for (from, to) in &cfg.exception_edges {
        // The edge from the block before a body opens with the body's first
        // command, which says whether the state before it can be thrown from.
        if *from != bn || entry_edges.contains(&(bn, *to)) {
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
/// condition evaluated to a constant lattice value, and the definitions a
/// condition the shared engine decided left untouched
/// ([`record_condition_preserves`]). Extracted from [`sccp`].
pub(in super::super) fn collect_constant_branches(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    (values, narrowing): (&mut HashMap<ValueKey, LatticeValue>, Narrowing<'_>),
    executable_blocks: &HashSet<BlockId>,
    order: &[BlockId],
    fold: BranchFold<'_>,
    existence_exits: Option<&HashMap<BlockId, Vec<Existence>>>,
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
            condition_base,
            ..
        }) = &block.terminator
        else {
            continue;
        };
        let Some(ssa_block) = ssa.blocks.get(bn) else {
            continue;
        };
        fold.driver.explaining(*term_span);
        // The condition reads the existence rung at the block's exit.
        if let Some(exit) = existence_exits.and_then(|exits| exits.get(bn)) {
            fold.driver.existence_enter(exit.clone());
        }
        let narrowed = narrowing.narrow(values, *bn);
        let decision = evaluate_branch_under(
            ssa_block,
            condition,
            values,
            ssa,
            fold,
            cfg.source_tokens_at(*bn, usize::MAX).zip(*condition_base),
        );
        restore_values(values, narrowed);
        fold.driver.existence_leave();
        fold.driver.explaining(None);
        if decision.is_some() {
            record_condition_preserves(cfg, ssa, *bn, fold.driver);
        }
        let cond_text = condition
            .substituted_word_source()
            .unwrap_or_else(|| crate::expr_ast::expr_text(condition));
        let (true_name, false_name) = (
            cfg.block_name(*true_target).to_owned(),
            cfg.block_name(*false_target).to_owned(),
        );
        match decision {
            Some(true) => constant_branches.push(ConstantBranch {
                block: cfg.block_name(*bn).to_owned(),
                span: *term_span,
                condition: cond_text,
                value: true,
                taken_target: true_name,
                not_taken_target: false_name,
                kind: BranchFactKind::Applied,
            }),
            Some(false) => constant_branches.push(ConstantBranch {
                block: cfg.block_name(*bn).to_owned(),
                span: *term_span,
                condition: cond_text,
                value: false,
                taken_target: false_name,
                not_taken_target: true_name,
                kind: BranchFactKind::Applied,
            }),
            None => {}
        }
    }
    constant_branches
}

/// The post-passes over the settled lattice, block by block under the
/// refinements in force there: each executable `subst` call's template
/// plan, each opaque case list's selection, and the `Selected` fact of each
/// arm no member of a subject reaches, each in source order.
pub(super) fn settled_statements(
    (driver, cfg, ssa): (&LatticeDriver<'_>, &CfgFunction, &SsaFunction),
    (values, narrowing): (&mut HashMap<ValueKey, LatticeValue>, Narrowing<'_>),
    executable_blocks: &HashSet<BlockId>,
) -> (
    Vec<TemplatePlanRecord>,
    Vec<SelectionRecord>,
    Vec<ConstantBranch>,
) {
    let (mut plans, mut selections, mut unreached) = (Vec::new(), Vec::new(), Vec::new());
    for (&block_id, block) in &ssa.blocks {
        if !executable_blocks.contains(&block_id) {
            continue;
        }
        let narrowed = narrowing.narrow(values, block_id);
        plans.extend(driver.template_plans_in(ssa, (block_id, block), values));
        let (records, arms) = driver.selection_facts_in(cfg, ssa, (block_id, block), values);
        selections.extend(records);
        unreached.extend(arms);
        restore_values(values, narrowed);
    }
    plans.sort_by_key(|record: &TemplatePlanRecord| (record.span.start(), record.span.end()));
    selections.sort_by_key(|record: &SelectionRecord| (record.span.start(), record.span.end()));
    unreached.sort_by_key(|fact: &ConstantBranch| fact.span.map(|span| (span.start(), span.end())));
    (plans, selections, unreached)
}

/// The `<cond>` statement before a branch carries what the condition's
/// command substitutions may write ([`crate::ir::SyntheticMarker::Condition`]).
/// When the shared engine decided the condition, every nested command it
/// ran answered without a store — any other declines `StatefulNested` — so
/// each definition the statement reads before it writes (a conditional
/// writer's target, a read-modify-write's) holds the version it read
/// ([`SccpResult::preserved`]). Only the block's last statement, the one
/// right before the branch whose condition it summarises, speaks for it.
pub(in super::super) fn record_condition_preserves(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    bn: BlockId,
    driver: &LatticeDriver<'_>,
) {
    let (Some(block), Some(ssa_block)) = (cfg.blocks.get(&bn), ssa.blocks.get(&bn)) else {
        return;
    };
    let Some(Terminator::Branch {
        span: Some(branch), ..
    }) = &block.terminator
    else {
        return;
    };
    let Some(last) = ssa_block.statements.last() else {
        return;
    };
    let Statement::Call {
        tokens: Some(tokens),
        span,
        ..
    } = &last.statement
    else {
        return;
    };
    let summarises_branch = tokens.synthetic == Some(crate::ir::SyntheticMarker::Condition)
        && span.start() <= branch.start()
        && branch.end() <= span.end();
    if !summarises_branch {
        return;
    }
    for (&var, &ver) in &last.defs {
        if let Some(&prior) = last.uses.get(&var) {
            driver.record_preserved((var, ver), Some(prior));
        }
    }
}
