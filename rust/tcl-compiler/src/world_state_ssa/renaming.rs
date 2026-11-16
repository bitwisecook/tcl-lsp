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

//! The renaming half of the planner: versions per symbol, joins at block entry and the definitions materialised on each edge.

use super::{EdgeCompletion, ExecutableWorldStateSsa, Planner, WorldStateSsaDecline, block_id};
use crate::effect_ssa::WorldStateIntentKind;
use crate::state_ssa::adapters::{WorldRegion, WorldRegionOverlapPolicy};
use crate::state_ssa::{
    StateClobber, StateDef, StateOp, StateOverlap, StatePhi, StateSite, StateSsa, StateSsaError,
    StateUse, StateVersion,
};
use std::collections::{BTreeMap, BTreeSet};

impl Planner<'_> {
    pub(super) fn rename(self) -> Result<ExecutableWorldStateSsa, WorldStateSsaDecline> {
        let count = self.function.blocks.len();
        let slots = self.locations.len();
        let initial = vec![Symbol::Initial; slots];
        let mut entry = vec![initial.clone(); count];
        let mut edge_exit: Vec<Vec<Vec<Symbol>>> = self
            .successors
            .iter()
            .map(|successors| vec![initial.clone(); successors.len()])
            .collect();
        let entry_index = self.function.entry.index();

        loop {
            let mut changed = false;
            for block in self.reachable.iter().copied() {
                let next_entry = self.join_entry(block, &edge_exit, slots, block == entry_index)?;
                let next_edges = self.apply_block_edges(block, next_entry.clone());
                if entry[block] != next_entry || edge_exit[block] != next_edges {
                    entry[block] = next_entry;
                    edge_exit[block] = next_edges;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        let mut phi_symbols = BTreeSet::new();
        for (block, values) in entry.iter().enumerate() {
            if self.reachable.contains(&block) {
                phi_symbols.extend(values.iter().copied().filter_map(Symbol::as_phi));
            }
        }
        let mut versions = BTreeMap::new();
        let mut next = 1_u32;
        for symbol in phi_symbols {
            versions.insert(Symbol::Phi(symbol.0, symbol.1), take_version(&mut next)?);
        }
        for (token, intent) in self.tokens.iter().enumerate() {
            if intent.kind != WorldStateIntentKind::Use {
                versions.insert(Symbol::Token(token), take_version(&mut next)?);
            }
        }

        let mut operations = Vec::new();
        for block in self.reachable.iter().copied() {
            for (slot, symbol) in entry[block].iter().copied().enumerate() {
                let Symbol::Phi(phi_block, phi_slot) = symbol else {
                    continue;
                };
                if phi_block != block || phi_slot != slot {
                    continue;
                }
                let mut incoming = BTreeMap::new();
                for predecessor in self.predecessors[block]
                    .iter()
                    .filter(|predecessor| self.reachable.contains(predecessor))
                {
                    let edge = self.edge_index(*predecessor, block)?;
                    incoming.insert(
                        block_id(*predecessor)?,
                        version_of(edge_exit[*predecessor][edge][slot], &versions)?,
                    );
                }
                operations.push(StateOp::Phi(StatePhi {
                    location: self.locations[slot].clone(),
                    version: version_of(Symbol::Phi(block, slot), &versions)?,
                    incoming,
                    includes_initial: block == entry_index,
                    site: StateSite::phi(
                        block_id(block)?,
                        u32::try_from(slot).map_err(|_| {
                            WorldStateSsaDecline::StateSiteOverflow {
                                block: self.function.blocks[block].id,
                                instruction: slot,
                            }
                        })?,
                    ),
                }));
            }
            let mut current = entry[block].clone();
            for token in &self.plans[block].base {
                self.materialise_token(*token, &mut current, &versions, &mut operations)?;
            }
            // Transition records have their definition site at execution, but
            // only edge data flow makes them visible. This preserves both the
            // Tcl sequencing and the registry's exact completion contract.
            for transition in &self.plans[block].transitions {
                for token in transition.edge_tokens.values() {
                    self.materialise_definition(*token, &versions, &mut operations)?;
                }
            }
        }
        let state = StateSsa::new(operations).map_err(WorldStateSsaDecline::InvalidStateSsa)?;
        Ok(ExecutableWorldStateSsa {
            state,
            locations: self.locations,
        })
    }

    fn join_entry(
        &self,
        block: usize,
        edge_exit: &[Vec<Vec<Symbol>>],
        slots: usize,
        includes_initial: bool,
    ) -> Result<Vec<Symbol>, WorldStateSsaDecline> {
        let mut entry = Vec::with_capacity(slots);
        for (slot, _) in self.locations.iter().enumerate() {
            let mut values = BTreeSet::new();
            if includes_initial {
                values.insert(Symbol::Initial);
            }
            for predecessor in self.predecessors[block]
                .iter()
                .filter(|predecessor| self.reachable.contains(predecessor))
            {
                let edge = self.edge_index(*predecessor, block)?;
                values.insert(edge_exit[*predecessor][edge][slot]);
            }
            let value = match values.len() {
                0 => return Err(WorldStateSsaDecline::MissingStatePredecessor { block, slot }),
                1 => *values
                    .iter()
                    .next()
                    .ok_or(WorldStateSsaDecline::MissingStatePredecessor { block, slot })?,
                _ => Symbol::Phi(block, slot),
            };
            entry.push(value);
        }
        Ok(entry)
    }

    pub(super) fn apply_block_edges(
        &self,
        block: usize,
        mut state: Vec<Symbol>,
    ) -> Vec<Vec<Symbol>> {
        for token in &self.plans[block].base {
            self.apply_token(*token, &mut state);
        }
        self.successors[block]
            .iter()
            .copied()
            .map(|successor| {
                let mut edge = state.clone();
                for item in &self.plans[block].transitions {
                    if let Some(token) = item.edge_tokens.get(&successor) {
                        self.apply_token(*token, &mut edge);
                    }
                }
                edge
            })
            .collect()
    }

    fn edge_index(
        &self,
        predecessor: usize,
        successor: usize,
    ) -> Result<usize, WorldStateSsaDecline> {
        self.successors[predecessor]
            .iter()
            .position(|candidate| *candidate == successor)
            .ok_or(WorldStateSsaDecline::MissingCfgEdge {
                predecessor,
                successor,
            })
    }

    pub(super) fn edge_mode(
        &self,
        block: usize,
        successor: usize,
    ) -> Result<EdgeCompletion, WorldStateSsaDecline> {
        let edge = self.edge_index(block, successor)?;
        self.edge_completion[block]
            .get(edge)
            .copied()
            .ok_or(WorldStateSsaDecline::MissingCfgEdge {
                predecessor: block,
                successor,
            })
    }

    fn apply_token(&self, token: usize, state: &mut [Symbol]) {
        let token_info = &self.tokens[token];
        if token_info.kind == WorldStateIntentKind::Use {
            return;
        }
        let policy = WorldRegionOverlapPolicy::conservative();
        let written = &self.locations[token_info.location];
        for (slot, read) in self.locations.iter().enumerate() {
            if policy.may_overlap(written, read) {
                state[slot] = Symbol::Token(token);
            }
        }
    }

    fn materialise_token(
        &self,
        token: usize,
        state: &mut [Symbol],
        versions: &BTreeMap<Symbol, StateVersion>,
        operations: &mut Vec<StateOp<WorldRegion>>,
    ) -> Result<(), WorldStateSsaDecline> {
        let token_info = &self.tokens[token];
        if token_info.kind == WorldStateIntentKind::Use {
            operations.push(StateOp::Use(StateUse {
                location: self.locations[token_info.location].clone(),
                reaching_version: version_of(state[token_info.location], versions)?,
                site: token_info.site.clone(),
            }));
        } else {
            self.materialise_definition(token, versions, operations)?;
            self.apply_token(token, state);
        }
        Ok(())
    }

    fn materialise_definition(
        &self,
        token: usize,
        versions: &BTreeMap<Symbol, StateVersion>,
        operations: &mut Vec<StateOp<WorldRegion>>,
    ) -> Result<(), WorldStateSsaDecline> {
        let token_info = &self.tokens[token];
        let location = self.locations[token_info.location].clone();
        let version = version_of(Symbol::Token(token), versions)?;
        match token_info.kind {
            WorldStateIntentKind::Use => {}
            WorldStateIntentKind::Def => operations.push(StateOp::Def(StateDef {
                location,
                version,
                site: token_info.site.clone(),
            })),
            WorldStateIntentKind::Clobber => operations.push(StateOp::Clobber(StateClobber {
                location,
                version,
                site: token_info.site.clone(),
            })),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum Symbol {
    Initial,
    Token(usize),
    Phi(usize, usize),
}

impl Symbol {
    fn as_phi(self) -> Option<(usize, usize)> {
        match self {
            Self::Phi(block, slot) => Some((block, slot)),
            Self::Initial | Self::Token(_) => None,
        }
    }
}

fn take_version(next: &mut u32) -> Result<StateVersion, WorldStateSsaDecline> {
    let version = StateVersion::new(*next);
    *next = next
        .checked_add(1)
        .ok_or(WorldStateSsaDecline::InvalidStateSsa(
            StateSsaError::VersionExhausted,
        ))?;
    Ok(version)
}

fn version_of(
    symbol: Symbol,
    versions: &BTreeMap<Symbol, StateVersion>,
) -> Result<StateVersion, WorldStateSsaDecline> {
    match symbol {
        Symbol::Initial => Ok(StateVersion::INITIAL),
        _ => versions
            .get(&symbol)
            .copied()
            .ok_or(WorldStateSsaDecline::MissingStateVersion),
    }
}
