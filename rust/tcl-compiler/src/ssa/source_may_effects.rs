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

//! The source-may calls and clobbers a built function records, and the source-variable snapshot taken over them.

use super::{
    Phi, SourceCfgBlockSnapshot, SourceMayClobber, SourceMaySsaCall, SsaFunction, Symbol, Version,
    is_unseen_call_marker,
};
use crate::cfg::BlockId;
use crate::ir::Statement;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct SourceVariableSsaSnapshot {
    identity: crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    definitions: Vec<(
        BlockId,
        usize,
        Version,
        crate::cfg_builder::global_write_info::OriginalSourceCallMayWrites,
    )>,
    graph: Vec<SourceVariableBlockSnapshot>,
    topology: HashMap<BlockId, SourceCfgBlockSnapshot>,
    pub(super) entry: BlockId,
    pub(super) idom: HashMap<BlockId, Option<BlockId>>,
    dominance_frontier: HashMap<BlockId, Vec<BlockId>>,
    dominator_tree: HashMap<BlockId, Vec<BlockId>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SourceVariableBlockSnapshot {
    pub(super) block: BlockId,
    pub(super) phis: Vec<Phi>,
    pub(super) entry: Option<Version>,
    exit: Option<Version>,
    pub(super) statements: Vec<SourceVariableStatementSnapshot>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SourceVariableReadFlags {
    pub(super) quoted: bool,
    name_only: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SourceVariableStatementSnapshot {
    pub(super) index: usize,
    pub(super) statement: Statement,
    pub(super) used: Option<Version>,
    pub(super) defined: Option<Version>,
    may: bool,
    pub(super) reads: SourceVariableReadFlags,
    destroyed: bool,
}

impl SourceVariableSsaSnapshot {
    pub(crate) fn same_owner(&self, other: &Self) -> bool {
        self.identity == other.identity
            && self.graph == other.graph
            && self.topology == other.topology
            && self.entry == other.entry
            && self.idom == other.idom
            && self.dominance_frontier == other.dominance_frontier
            && self.dominator_tree == other.dominator_tree
            && self.definitions.len() == other.definitions.len()
            && self
                .definitions
                .iter()
                .zip(&other.definitions)
                .all(|(left, right)| {
                    left.0 == right.0
                        && left.1 == right.1
                        && left.2 == right.2
                        && left.3.same_owner(&right.3)
                })
    }
}

impl SsaFunction {
    fn current_source_may_call(&self, block: BlockId, index: usize) -> Option<&SourceMaySsaCall> {
        let effect = self.source_may_effects.get(&(block, index))?;
        let statement = self.blocks.get(&block)?.statements.get(index)?;
        let original = effect.original.as_ref()?;
        let input = self
            .point_contexts
            .as_ref()?
            .source_metadata_owner()
            .source_analysis_input();
        if statement.statement != effect.statement
            || !original.matches_source_statement(&statement.statement, input)
            || effect.definitions.len() != original.writes().len()
            || effect.definitions.iter().zip(original.writes()).any(
                |(&(symbol, version), place)| {
                    statement.defs.get(&symbol) != Some(&version)
                        || !statement.may_defs.contains(&symbol)
                        || self.source_variable_symbol(symbol) != Some(place.symbol())
                },
            )
        {
            return None;
        }
        Some(effect)
    }

    /// Hidden source MAY effects cannot issue a call outcome or a location relation.
    pub(crate) fn source_may_requires_unknown_at(&self, block: BlockId, index: usize) -> bool {
        let logical = self
            .point_contexts
            .as_ref()
            .and_then(|points| points.source_metadata_owner().source_analysis_input())
            .is_some_and(crate::analyser::ResolvedAnalysisInput::has_logical_source_name_context);
        let applicable = self.blocks.get(&block).and_then(|body| body.statements.get(index)).is_some_and(|statement| {
            matches!(&statement.statement, Statement::Call { tokens, foreach_groups: None, .. } if tokens.as_ref().is_none_or(|tokens| tokens.synthetic.is_none()))
        });
        logical
            && applicable
            && self
                .current_source_may_call(block, index)
                .is_none_or(|effect| {
                    effect
                        .original
                        .as_ref()
                        .is_none_or(|original| !original.writes().is_empty())
                })
    }

    pub(super) fn seal_source_may_clobbers(&mut self) {
        if self.source_may_effects.is_empty() {
            return;
        }
        let locations = std::sync::Arc::new(
            self.cell_symbols()
                .map(|(symbol, key)| (symbol, key.clone()))
                .collect::<Vec<_>>(),
        );
        for (&(block, index), effect) in &mut self.source_may_effects {
            let Some(body) = self.blocks.get(&block) else {
                continue;
            };
            let Some((marker_index, marker)) = body
                .statements
                .iter()
                .enumerate()
                .skip(index + 1)
                .take_while(|(_, statement)| statement.statement.synthetic_marker().is_some())
                .find(|(_, statement)| is_unseen_call_marker(&statement.statement))
            else {
                continue;
            };
            let Some(versions) = self
                .value_clobbers
                .get(&block)
                .and_then(|markers| markers.get(&marker_index))
                .filter(|versions| !versions.is_empty())
            else {
                continue;
            };
            effect.clobber = Some(SourceMayClobber {
                marker_index,
                marker: marker.statement.clone(),
                versions: versions.clone(),
                locations: std::sync::Arc::clone(&locations),
            });
        }
    }

    /// Reuse only the marker and location clobbers emitted by this SSA producer.
    /// Missing correspondence supplies no closed effect coverage.
    pub(crate) fn source_may_clobber_is_current_at(&self, block: BlockId, index: usize) -> bool {
        let Some(effect) = self.source_may_effects.get(&(block, index)) else {
            return false;
        };
        let Some(held) = effect.clobber.as_ref() else {
            return false;
        };
        let Some(body) = self.blocks.get(&block) else {
            return false;
        };
        body.statements
            .get(index)
            .is_some_and(|statement| statement.statement == effect.statement)
            && body
                .statements
                .get(held.marker_index)
                .is_some_and(|statement| statement.statement == held.marker)
            && body
                .statements
                .get(index + 1..held.marker_index)
                .is_some_and(|statements| {
                    statements
                        .iter()
                        .all(|statement| statement.statement.synthetic_marker().is_some())
                })
            && self
                .value_clobbers
                .get(&block)
                .and_then(|markers| markers.get(&held.marker_index))
                == Some(&held.versions)
            && self
                .cell_symbols()
                .eq(held.locations.iter().map(|(symbol, key)| (*symbol, key)))
    }

    /// Correspondence to the graph held by this SSA owner. A current external
    /// FunctionUnit/CFG still requires its independent owner checks.
    pub(crate) fn source_variable_snapshot(
        &self,
        symbol: Symbol,
    ) -> Option<SourceVariableSsaSnapshot> {
        let identity = self.source_variable_symbol(symbol)?.clone();
        let mut definitions = Vec::new();
        for (&(block, index), effect) in &self.source_may_effects {
            for &(defined, version) in &effect.definitions {
                if defined == symbol {
                    let current = self.current_source_may_call(block, index)?;
                    definitions.push((block, index, version, current.original.as_ref()?.clone()));
                }
            }
        }
        if definitions.is_empty() {
            return None;
        }
        definitions.sort_by_key(|(block, index, version, _)| (*block, *index, *version));
        let mut graph = Vec::new();
        for (&block, body) in &self.blocks {
            let phis: Vec<_> = body
                .phis
                .iter()
                .filter(|phi| phi.name == symbol)
                .cloned()
                .collect();
            let statements: Vec<_> = body
                .statements
                .iter()
                .enumerate()
                .filter_map(|(index, statement)| {
                    let used = statement.uses.get(&symbol).copied();
                    let defined = statement.defs.get(&symbol).copied();
                    (used.is_some() || defined.is_some()).then(|| SourceVariableStatementSnapshot {
                        index,
                        statement: statement.statement.clone(),
                        used,
                        defined,
                        may: statement.may_defs.contains(&symbol),
                        reads: SourceVariableReadFlags {
                            quoted: statement.quoted_uses.contains(&symbol),
                            name_only: statement.name_only_uses.contains(&symbol),
                        },
                        destroyed: statement.destruction_defs.contains(&symbol),
                    })
                })
                .collect();
            let entry = body.entry_versions.get(&symbol).copied();
            let exit = body.exit_versions.get(&symbol).copied();
            if !phis.is_empty() || !statements.is_empty() || entry.is_some() || exit.is_some() {
                graph.push(SourceVariableBlockSnapshot {
                    block,
                    phis,
                    entry,
                    exit,
                    statements,
                });
            }
        }
        graph.sort_by_key(|block| block.block);
        Some(SourceVariableSsaSnapshot {
            identity,
            definitions,
            graph,
            topology: self.source_topology.clone(),
            entry: self.entry,
            idom: self.idom.clone(),
            dominance_frontier: self.dominance_frontier.clone(),
            dominator_tree: self.dominator_tree.clone(),
        })
    }
}
