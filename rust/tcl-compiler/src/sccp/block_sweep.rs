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

//! Reachable blocks, entry seeds and phi joins shared by SCCP consumers.

use super::statement_transfer::held_value;
use super::{join, set_value};
use crate::analyses::LatticeValue;
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::ssa::{SsaBlock, SsaFunction, Symbol, ValueKey};
use rustc_hash::FxHashSet;
use std::collections::{HashMap, HashSet};

pub(super) struct BlockSweepState {
    pub(super) values: HashMap<ValueKey, LatticeValue>,
    pub(super) executable_blocks: HashSet<BlockId>,
    pub(super) executable_edges: HashSet<(BlockId, BlockId)>,
}

pub(super) struct ExecutableBlock<'a> {
    pub(super) id: BlockId,
    pub(super) ssa: &'a SsaBlock,
    pub(super) incoming: &'a [BlockId],
    pub(super) finalizing: bool,
}

impl BlockSweepState {
    pub(super) fn new(cfg: &CfgFunction, values: HashMap<ValueKey, LatticeValue>) -> Self {
        Self {
            values,
            executable_blocks: cfg
                .blocks
                .contains_key(&cfg.entry)
                .then_some(cfg.entry)
                .into_iter()
                .collect(),
            executable_edges: HashSet::new(),
        }
    }

    /// Sweep to the optimistic fixpoint, then reopen any branches still
    /// waiting on unknown values and settle again. Newly executable blocks
    /// are eligible immediately when they occur later in this traversal.
    pub(super) fn settle(
        &mut self,
        (cfg, ssa, predecessors): (
            &CfgFunction,
            &SsaFunction,
            &HashMap<BlockId, HashSet<BlockId>>,
        ),
        order: &[BlockId],
        mut begin_pass: impl FnMut(),
        mut transfer: impl FnMut(ExecutableBlock<'_>, &mut Self) -> bool,
    ) {
        for finalizing in [false, true] {
            loop {
                begin_pass();
                let mut changed = false;
                for &id in order {
                    if !self.executable_blocks.contains(&id) {
                        continue;
                    }
                    let Some(block) = ssa.blocks.get(&id) else {
                        continue;
                    };
                    let incoming =
                        incoming_executable_predecessors(id, predecessors, &self.executable_edges);
                    if id != cfg.entry {
                        changed |= sccp_process_phis(&mut self.values, block, &incoming);
                    }
                    changed |= transfer(
                        ExecutableBlock {
                            id,
                            ssa: block,
                            incoming: &incoming,
                            finalizing,
                        },
                        self,
                    );
                }
                if !changed {
                    break;
                }
            }
        }
    }
}

fn incoming_executable_predecessors(
    block: BlockId,
    predecessors: &HashMap<BlockId, HashSet<BlockId>>,
    executable_edges: &HashSet<(BlockId, BlockId)>,
) -> Vec<BlockId> {
    predecessors
        .get(&block)
        .into_iter()
        .flatten()
        .copied()
        .filter(|predecessor| executable_edges.contains(&(*predecessor, block)))
        .collect()
}

/// The lattice's starting values: the interprocedural parameter seed. The
/// seed keys on the parameter *name* (a stable, cache-safe identity); each
/// resolves to this build's interned symbol. A parameter never read in the
/// body isn't interned, and its seed slot would never be consulted, so
/// dropping it is behaviour-neutral.
pub(super) fn seeded_values(
    param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
    symbol_of: impl Fn(&str) -> Option<Symbol>,
) -> HashMap<ValueKey, LatticeValue> {
    let mut values = HashMap::new();
    for ((name, version), value) in param_constants.into_iter().flatten() {
        if let Some(sym) = symbol_of(name) {
            values.insert((sym, *version), value.clone());
        }
    }
    values
}

/// Seed live-in roots to `Overdefined`: a value *used* but never *defined*
/// anywhere in this function (a proc parameter, a global / namespace read, an
/// upvar target, or an undefined-variable read) holds a runtime-unknown value,
/// so it is `Overdefined`, not the `Unknown` join-identity (which would
/// silently vanish from any phi it feeds — folding `join(const, $runtime)` to
/// the constant).
pub(super) fn seed_live_in_roots<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &mut HashMap<ValueKey, LatticeValue, S>,
    grammar: tcl_dialect::LexerGrammar,
    branch_symbol: impl Fn(BlockId, &str) -> Option<Symbol>,
) {
    let mut defined_keys: FxHashSet<ValueKey> = FxHashSet::default();
    let mut used_keys: FxHashSet<ValueKey> = FxHashSet::default();
    for ssa_block in ssa.blocks.values() {
        for phi in &ssa_block.phis {
            defined_keys.insert((phi.name, phi.version));
            for inc in phi.incoming.values() {
                // Record every phi feed, including the version-0 (entry /
                // live-in) incoming: it is never a def, so it drops through
                // to the `used_keys.difference(&defined_keys)` seeding below
                // and is pinned `Overdefined`. This is what lets the phi
                // join at [`sccp_process_phis`] see the caller's value
                // instead of silently dropping it and folding to the
                // defined-arm constant.
                used_keys.insert((phi.name, *inc));
            }
        }
        for s in &ssa_block.statements {
            for (&var, ver) in &s.defs {
                defined_keys.insert((var, *ver));
            }
            for (&var, ver) in &s.uses {
                used_keys.insert((var, *ver));
            }
        }
    }
    // Branch-condition reads, resolved at each block's exit versions.
    for (bn, block) in &cfg.blocks {
        if let Some(Terminator::Branch { condition, .. }) = &block.terminator
            && let Some(sb) = ssa.blocks.get(bn)
        {
            for var in crate::var_refs::vars_in_expr(condition, grammar) {
                let Some(sym) = branch_symbol(*bn, &var) else {
                    continue;
                };
                let ver = sb.exit_versions.get(&sym).copied().unwrap_or(0);
                used_keys.insert((sym, ver));
            }
        }
    }
    for key in used_keys.difference(&defined_keys) {
        values.entry(*key).or_insert(LatticeValue::Overdefined);
    }
}

/// Join phi values from edge-executable predecessors for one block. Returns
/// `true` if any lattice value changed.
pub(super) fn sccp_process_phis(
    values: &mut HashMap<ValueKey, LatticeValue>,
    ssa_block: &crate::ssa::SsaBlock,
    incoming_exec: &[BlockId],
) -> bool {
    if incoming_exec.is_empty() {
        return false;
    }
    let mut changed = false;
    for phi in &ssa_block.phis {
        let mut phi_val = LatticeValue::Unknown;
        for pred in incoming_exec {
            let incoming_ver = phi.incoming.get(pred).copied().unwrap_or(0);
            // An unseeded entry is Overdefined, while an unreached incoming
            // definition stays Unknown. Both reads use the shared lattice rule.
            let candidate = held_value(values, (phi.name, incoming_ver));
            phi_val = join(&phi_val, &candidate);
        }
        if set_value(values, (phi.name, phi.version), &phi_val) {
            changed = true;
        }
    }
    changed
}
