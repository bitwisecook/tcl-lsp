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

//! Deciding a branch condition: the math bindings it is evaluated under and the static-loop summary at a loop exit.

use super::math_expressions::{
    FoldedExpressionContents, const_to_env_value, env_from_uses, fold_math_expression,
};
use super::{BranchFold, ExpressionEvaluationPoint, MathFoldContext, SolverPurpose};
use crate::analyses::{ConstValue, LatticeValue};
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::math_function_binding::ExpressionMathBindings;
use crate::ssa::{SsaFunction, SsaSourceView, ValueKey};
use crate::tcl_expr_eval::{FoldPolicy, TclValue};
use std::collections::HashMap;

pub(super) fn branch_decision(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    bn: BlockId,
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue>,
    fold: BranchFold<'_>,
) -> Option<bool> {
    // The flattened `switch` compares its subject as a `Raw` whole-variable
    // operand, which the evaluator reads only as that variable (#2375).
    let resolved = fold
        .reference_config
        .and_then(|config| crate::sccp::with_whole_variable_operands(condition, config));
    let condition = resolved.as_ref().unwrap_or(condition);
    loop_summary_decision(cfg, ssa, bn, condition, values, fold).or_else(|| {
        let math = branch_math_bindings(cfg, bn);
        let math = if fold
            .math_dependencies
            .is_some_and(|ledger| ledger.purpose == SolverPurpose::SemanticAnalysis)
        {
            materialised_branch_bindings(cfg, bn, math)
        } else {
            math
        };
        evaluate_branch_with_math(
            ssa_block,
            condition,
            values,
            BranchExpressionPolicy {
                policy: fold.policy,
                grammar: fold.grammar,
            },
            ssa,
            Some(MathFoldContext {
                point: Some(ExpressionEvaluationPoint::Branch { block: bn }),
                bindings: math,
                dependencies: fold.math_dependencies,
                incoming_reads: Some((SsaSourceView::at_terminator(ssa, bn), fold.registry)),
            }),
        )
    })
}

fn branch_math_bindings(cfg: &CfgFunction, block: BlockId) -> ExpressionMathBindings<'_> {
    let base = match cfg
        .blocks
        .get(&block)
        .and_then(|block| block.terminator.as_ref())
    {
        Some(Terminator::Branch { condition_base, .. }) => *condition_base,
        _ => None,
    };
    ExpressionMathBindings::for_origin(
        &cfg.implicit_math_invocations,
        cfg.terminator_sources
            .get(&block)
            .and_then(Option::as_deref),
        base,
    )
    .with_preparations(&cfg.expression_preparations)
}

/// A dynamic condition word is prepared from its actual evaluated bytes.
/// Only analysis may use that source object: its fresh tree does not prove
/// that discarding the original argv/read/conversion effects is safe.
pub(super) fn materialised_branch_bindings<'a>(
    cfg: &'a CfgFunction,
    block: BlockId,
    bindings: ExpressionMathBindings<'a>,
) -> ExpressionMathBindings<'a> {
    if bindings.preparation().is_some() {
        return bindings;
    }
    cfg.source_input_tokens_at(block, usize::MAX)
        .and_then(|tokens| tokens.argv.first())
        .and_then(|span| bindings.at_invocation(span.start()))
        .unwrap_or(bindings)
}

/// Convert an SCCP [`ConstValue`] to the static simulator's
/// [`crate::static_loops::StaticValue`].
fn const_to_static(c: &ConstValue) -> crate::static_loops::StaticValue {
    use crate::static_loops::StaticValue;
    match c {
        ConstValue::Int(i) => StaticValue::Int(*i),
        ConstValue::Float(f) => StaticValue::Float(*f),
        ConstValue::Bool(b) => StaticValue::Bool(*b),
        ConstValue::String(s) => StaticValue::Str(s.clone()),
    }
}

/// Fold `condition` via a static summary of the `for` loop whose exit block is
/// `bn`, or `None` when `bn` is not a loop exit or the loop cannot be
/// summarised. The simulation is seeded with the constants known at the
/// pre-loop block's exit under the retained CFG metadata and fold Registry.
fn loop_summary_decision(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    bn: BlockId,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue>,
    fold: BranchFold<'_>,
) -> Option<bool> {
    let policy = fold.policy;
    let node = cfg.loop_nodes.get(&bn)?;
    let start_ssa = ssa.blocks.get(&node.entry_block)?;
    let mut start_env = crate::static_loops::StaticEnv::new();
    let lookup = if ssa.point_contexts.is_some() {
        SsaSourceView::at_terminator(ssa, node.entry_block)
    } else {
        SsaSourceView::unpositioned(ssa)
    };
    if lookup.is_positioned() {
        for (name, reference) in lookup.reaching_bindings(fold.registry) {
            let Some(version) = reference.version else {
                continue;
            };
            if let Some(LatticeValue::Const(value)) = values.get(&(reference.symbol, version)) {
                start_env.insert(name.to_owned(), const_to_static(value));
            }
        }
    } else {
        for (name, symbol) in lookup.source_symbols() {
            let Some(version) = start_ssa.exit_versions.get(&symbol) else {
                continue;
            };
            if let Some(LatticeValue::Const(value)) = values.get(&(symbol, *version)) {
                start_env.insert(name.to_owned(), const_to_static(value));
            }
        }
    }
    let condition_base = match &node.statement {
        crate::ir::Statement::For { condition_base, .. } => *condition_base,
        _ => None,
    };
    let loop_math = ExpressionMathBindings::for_origin(
        &cfg.implicit_math_invocations,
        node.executed_source.as_deref(),
        condition_base,
    )
    .with_preparations(&cfg.expression_preparations);
    if fold
        .math_dependencies
        .is_some_and(|ledger| ledger.purpose == SolverPurpose::SemanticAnalysis)
    {
        return semantic_loop_summary_decision(
            cfg,
            bn,
            condition,
            &node.statement,
            &start_env,
            loop_math,
            fold,
        );
    }
    let summarised = crate::static_loops::summarise_for_statement_with_source_dependencies(
        &node.statement,
        &start_env,
        crate::static_loops::DEFAULT_MAX_STATIC_LOOP_ITERS,
        policy,
        Some(loop_math),
        crate::static_loops::StaticLoopSource::new(fold.registry, &cfg.metadata_context),
    )?;
    let v =
        crate::static_loops::evaluate_expr_with_constants(condition, &summarised.values, policy)?;
    if let Some(ledger) = fold.math_dependencies
        && !ledger.retain(
            &summarised.required_math_invocations,
            &summarised.required_expression_preparations,
        )
    {
        return None;
    }
    Some(v != 0)
}

fn semantic_loop_summary_decision(
    cfg: &CfgFunction,
    block: BlockId,
    condition: &ExprNode,
    statement: &Statement,
    initial: &crate::static_loops::StaticEnv,
    bindings: ExpressionMathBindings<'_>,
    fold: BranchFold<'_>,
) -> Option<bool> {
    let ledger = fold.math_dependencies?;
    let point = ExpressionEvaluationPoint::Branch { block };
    ledger.loops.borrow_mut().remove(&point);
    let analysis = crate::static_loops::analyse_for_statement_with_source_dependencies(
        statement,
        initial,
        crate::static_loops::DEFAULT_MAX_STATIC_LOOP_ITERS,
        fold.policy,
        Some(bindings),
        crate::static_loops::StaticLoopSource::new(fold.registry, &cfg.metadata_context),
    )?;
    let environment = analysis
        .values
        .iter()
        .map(|(name, value)| (name.clone(), value.to_env_value()))
        .collect();
    let FoldedExpressionContents::Numeric(value) = fold_math_expression(
        condition,
        &environment,
        fold.policy,
        Some(MathFoldContext {
            point: Some(point),
            bindings: branch_math_bindings(cfg, block),
            dependencies: Some(ledger),
            incoming_reads: None,
        }),
    )?
    else {
        return None;
    };
    if matches!(&value, TclValue::Float(value) if value.is_nan())
        || !ledger.retain(
            &analysis.required_math_invocations,
            &analysis.required_expression_preparations,
        )
    {
        return None;
    }
    ledger.loops.borrow_mut().insert(point, analysis);
    Some(value.is_truthy())
}

/// Evaluate a branch condition.
///
/// Returns `Some(true)` / `Some(false)` when the condition folds to
/// a constant under the current lattice; `None` otherwise.
#[must_use]
pub fn evaluate_branch<S: std::hash::BuildHasher>(
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue, S>,
    policy: FoldPolicy,
    ssa: &SsaFunction,
    grammar: tcl_dialect::LexerGrammar,
) -> Option<bool> {
    evaluate_branch_with_math(
        ssa_block,
        condition,
        values,
        BranchExpressionPolicy { policy, grammar },
        ssa,
        None,
    )
}

#[derive(Clone, Copy)]
struct BranchExpressionPolicy {
    pub(super) policy: FoldPolicy,
    pub(super) grammar: tcl_dialect::LexerGrammar,
}

fn evaluate_branch_with_math<S: std::hash::BuildHasher>(
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue, S>,
    fold: BranchExpressionPolicy,
    ssa: &SsaFunction,
    math: Option<MathFoldContext<'_>>,
) -> Option<bool> {
    let BranchExpressionPolicy { policy, grammar } = fold;
    let block = ssa.block_id(&ssa_block.name).unwrap_or(BlockId(u32::MAX));
    let lookup = if ssa.point_contexts.is_some() {
        SsaSourceView::at_terminator(ssa, block)
    } else {
        SsaSourceView::unpositioned(ssa)
    };
    let mut env = env_from_uses(&ssa_block.exit_versions, values, lookup);
    // A parameter read in a branch condition without a local redefinition
    // isn't in `exit_versions` (those carry defined-in-block versions), so
    // its caller-provided version-0 seed never reaches the fold. Bind it
    // here — but only when version 0 is still live (the param is not
    // redefined to another value before the branch).
    for name in crate::var_refs::vars_in_expr(condition, grammar) {
        if env.contains_key(&name) {
            continue;
        }
        let symbol = if lookup.is_positioned() {
            lookup
                .read_spelling(&name)
                .filter(|read| read.version == Some(0))
                .map(|read| read.symbol)
        } else {
            lookup.symbol(&name)
        };
        let Some(sym) = symbol else {
            continue;
        };
        let v0_live = ssa_block.exit_versions.get(&sym).copied().unwrap_or(0) == 0;
        if v0_live && let Some(LatticeValue::Const(c)) = values.get(&(sym, 0)) {
            env.insert(name, const_to_env_value(c));
        }
    }
    let FoldedExpressionContents::Numeric(v) = fold_math_expression(condition, &env, policy, math)?
    else {
        return None;
    };
    // A NaN condition is C's "floating point value is Not a Number" runtime
    // error, not a truth value — folding either way would delete a branch
    // that must raise. Decline.
    if matches!(&v, crate::tcl_expr_eval::TclValue::Float(f) if f.is_nan()) {
        return None;
    }
    Some(v.is_truthy())
}
