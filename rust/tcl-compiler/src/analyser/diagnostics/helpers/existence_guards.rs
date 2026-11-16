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

//! The cells a branch condition guards (`info exists` and its kin) and the names a condition's own substitutions write.

use crate::cfg::BlockId;

/// Exact cell guarded by a condition and its dominated successor region.
pub(in super::super) type ExistenceGuard = (crate::var_resolve::VariableCellKey, BlockId);

/// Collect `(cell, guard_block)` pairs for every
/// `[info exists X]` / `[array exists X]` branch condition in `fu`.
/// A read of `var` in any block dominated by `guard_block` is guarded
/// (X provably exists).  A positive query guards the true target; a
/// `![info exists X]` query guards the false target.
pub(in super::super) fn collect_existence_guards(
    fu: &crate::compilation_unit::FunctionUnit,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Vec<ExistenceGuard> {
    use crate::cfg::Terminator;
    let mut guards = Vec::new();
    let Some(metadata) = fu.invocation_metadata_context(registry) else {
        return guards;
    };
    if config.normalized() != fu.source_lexer_config().normalized() {
        return guards;
    }
    for (&condition_block, block) in &fu.cfg.blocks {
        if let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            condition_base,
            ..
        }) = &block.terminator
        {
            let selected = if fu.ssa.point_contexts.is_some() {
                condition_base.and_then(|base| {
                    let tokens = crate::ssa::SsaSourceView::at_statement(
                        &fu.ssa,
                        condition_block,
                        usize::MAX,
                    )
                    .source_tokens()?;
                    let (query, context) =
                        crate::existence_query::in_expr_for_diagnostics_at_with_metadata_context(
                            condition,
                            base,
                            tokens,
                            registry,
                            config,
                            Some(metadata),
                        )?;
                    let place = crate::var_resolve::resolve_literal_place(
                        &query.var, &context, false, registry,
                    );
                    Some((query, crate::var_resolve::canonical_place_key(&place)?))
                })
            } else {
                fu.cfg
                    .source_tokens_at(condition_block, usize::MAX)
                    .filter(|tokens| {
                        tokens.source_binding.as_ref().and_then(|binding| {
                            binding.original_source_advice_purpose(tokens, metadata, registry)
                        }) == Some(crate::command_binding::OriginalInvocationSourcePurpose::Logical)
                    })
                    .and_then(|_| {
                        crate::existence_query::in_expr_with_metadata_context(
                            condition,
                            registry,
                            config,
                            Some(metadata),
                        )
                    })
                    .and_then(|query| {
                        let symbol = fu
                            .ssa
                            .var_symbol_at_terminator(condition_block, &query.var)?;
                        Some((query, fu.ssa.cell_key(symbol)?.clone()))
                    })
            };
            if let Some((query, cell)) = selected {
                let target = if query.negated {
                    *false_target
                } else {
                    *true_target
                };
                guards.push((cell, target));
            }
        }
    }
    guards
}

/// An original read inside a condition can be guarded by an earlier operand,
/// before the CFG branch reaches either successor block.
pub(in super::super) fn original_condition_read_is_guarded(
    fu: &crate::compilation_unit::FunctionUnit,
    block: BlockId,
    span: tcl_lexer::Span,
    cell: &crate::var_resolve::VariableCellKey,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let Some(crate::cfg::Terminator::Branch {
        condition,
        condition_base: Some(base),
        ..
    }) = fu
        .cfg
        .blocks
        .get(&block)
        .and_then(|block| block.terminator.as_ref())
    else {
        return false;
    };
    let tokens =
        crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, usize::MAX).source_tokens();
    let reading = crate::tcl_expr_eval::ConditionReading {
        registry,
        config: fu.source_lexer_config(),
        policy: crate::tcl_expr_eval::FoldPolicy::from_registry(registry),
        trusted: &|_| false,
        existence: crate::existence_query::QueryReading::for_cfg(
            &fu.cfg,
            registry,
            fu.source_lexer_config(),
            tokens,
            Some(*base),
        )
        .for_diagnostics(),
    };
    crate::tcl_expr_eval::condition_read_guards(condition, *base, reading)
        .iter()
        .any(|(read, guarded)| fu.abs_span(*read) == span && guarded == cell)
}

/// Names a branch condition's own `[…]` substitutions write: the `<cond>`
/// kill-set the CFG records ahead of the branch. C Tcl evaluates the
/// condition, and so every write it makes, before either arm runs.
#[derive(Default)]
pub(in super::super) struct ConditionWrites {
    /// `(name, true arm)` for a condition that is one bare substitution
    /// (`if {[regexp … m]}`): its arm runs only on a true result, which the
    /// writing commands (`regexp`, `scan`, `gets`, `catch`) give only after
    /// assigning their output variables.
    truthy: Vec<(String, BlockId)>,
    /// `(name, condition block)` for every condition substitution's write.
    written: Vec<(String, BlockId)>,
}

impl ConditionWrites {
    /// Whether a possibly-missing read of `name` in `block` follows a
    /// condition that writes it: inside the true arm of a bare substitution
    /// condition, or after any condition write when the read is not a
    /// proved absence on every path (`may_be_defined`). A proved absence —
    /// a `regexp` that provably never matches — still reports.
    pub(in super::super) fn covers(
        &self,
        name: &str,
        ssa: &crate::ssa::SsaFunction,
        block: BlockId,
        may_be_defined: bool,
    ) -> bool {
        self.truthy
            .iter()
            .any(|(written, arm)| written == name && block_dominated_by(ssa, block, *arm))
            || may_be_defined
                && self.written.iter().any(|(written, condition)| {
                    written == name
                        && block != *condition
                        && block_dominated_by(ssa, block, *condition)
                })
    }
}

pub(in super::super) fn collect_condition_writes(
    fu: &crate::compilation_unit::FunctionUnit,
) -> ConditionWrites {
    let mut out = ConditionWrites::default();
    for (&block_id, block) in &fu.cfg.blocks {
        let Some(crate::cfg::Terminator::Branch {
            condition,
            true_target,
            ..
        }) = &block.terminator
        else {
            continue;
        };
        for statement in &block.statements {
            let crate::ir::Statement::Call {
                defs,
                tokens: Some(tokens),
                ..
            } = statement
            else {
                continue;
            };
            if tokens.synthetic != Some(crate::ir::SyntheticMarker::Condition) {
                continue;
            }
            for name in defs {
                if matches!(condition, crate::expr_ast::ExprNode::Command { .. }) {
                    out.truthy.push((name.clone(), *true_target));
                }
                out.written.push((name.clone(), block_id));
            }
        }
    }
    out
}

/// True when `block` is dominated by `dom` (walking the SSA immediate
/// dominator chain; a block dominates itself).
pub(in super::super) fn block_dominated_by(
    ssa: &crate::ssa::SsaFunction,
    block: BlockId,
    dom: BlockId,
) -> bool {
    let mut cur = block;
    loop {
        if cur == dom {
            return true;
        }
        match ssa.idom.get(&cur) {
            Some(Some(parent)) => cur = *parent,
            _ => return false,
        }
    }
}
