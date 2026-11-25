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
    BranchFactKind, BranchFold, BranchSelection, ConstantBranch, SelectionRecord,
    TemplatePlanRecord, branch_has_pending_operands, transfer_terminator,
};
use super::TerminatorInputs;
use super::refinements::{Narrowing, restore_values};
use crate::analyses::LatticeValue;
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::ssa::{SsaFunction, SsaSourceView, ValueKey};
use crate::value_transfer::LatticeDriver;
use std::collections::{HashMap, HashSet};
use tcl_registry::value_transfer::Existence;

/// Defer using the optimiser's compatibility-wide SSA read view.
pub(in super::super) fn branch_deferrable(
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue>,
    ssa: &SsaFunction,
    grammar: tcl_dialect::LexerGrammar,
) -> bool {
    branch_has_pending_operands(
        ssa_block,
        condition,
        values,
        SsaSourceView::unpositioned(ssa),
        grammar,
    )
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
    // Analysis edges include a loop leaving an exhausted list. Like normal
    // successors, they are unavailable when the block certainly raises.
    let analysis = exit
        .completes
        .then_some(&cfg.analysis_edges)
        .into_iter()
        .flatten();
    // Entry exceptions open separately when the body can throw before a store.
    let exceptions = cfg
        .exception_edges
        .iter()
        .filter(|edge| edge.0 == bn && !entry_edges.contains(edge));
    transfer_terminator(
        cfg,
        bn,
        (executable_blocks, executable_edges),
        exit.completes,
        analysis.chain(exceptions),
        |condition, span, base| {
            let ssa_block = ssa.blocks.get(&bn)?;
            driver.explaining(span);
            let decision = evaluate_branch_under(
                ssa_block,
                condition,
                values,
                ssa,
                BranchFold { driver },
                cfg.source_tokens_at(bn, usize::MAX).zip(base),
            );
            driver.explaining(None);
            Some(BranchSelection::of(decision, finalizing, || {
                branch_deferrable(ssa_block, condition, values, ssa, grammar)
            }))
        },
    )
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
