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

//! Optimiser-specific work at each reachable block of the shared SCCP sweep.

use super::super::block_sweep::{BlockSweepState, ExecutableBlock};
use super::branches::sccp_process_terminator;
use super::refinements::{narrow_values, restore_values};
use super::statements::{clobber_at_unseen_call, sccp_process_statements};
use super::{
    BlockExit, CatchBodies, StatementInputs, SweepContext, SweepLifecycle, TerminatorInputs,
};
use crate::analyses::LatticeValue;
use crate::cfg::BlockId;
use crate::ssa::ValueKey;
use crate::value_transfer::LatticeDriver;
use std::collections::{HashMap, HashSet};

impl SweepContext<'_> {
    /// Settle the optimiser's block transfers over the shared reachability sweep.
    pub(super) fn settle(
        &self,
        order: &[BlockId],
        state: &mut BlockSweepState,
        lifecycle: &mut SweepLifecycle,
    ) {
        state.settle(
            (self.cfg, self.ssa, self.preds),
            order,
            || {
                // Every sweep re-evaluates each statement. Only the settled
                // sweep's route entries belong in the reported tally.
                self.driver.reset_tally_for_sweep();
                self.driver.reset_undecided_for_sweep();
                self.driver.open_iteration();
            },
            |block, state| self.block(block, state, lifecycle),
        );
    }

    fn block(
        &self,
        ExecutableBlock {
            id: bn,
            ssa: ssa_block,
            incoming,
            finalizing,
        }: ExecutableBlock<'_>,
        state: &mut BlockSweepState,
        lifecycle: &mut SweepLifecycle,
    ) -> bool {
        let mut changed = false;
        if bn != self.cfg.entry {
            record_phi_folded_types(&state.values, ssa_block, incoming, self.driver);
        }

        // The refinements every executable edge into the block carries
        // narrow the versions they name for its statements and its
        // terminator; the φs above read each version's own value.
        let narrowed = self.narrow(bn, state, &mut lifecycle.refined, &mut changed);

        // The existence rung enters the block with the join of its
        // executable edges, which is each φ's fact.
        let mut at = lifecycle.existence.as_mut().map(|run| {
            run.enter(
                self.cfg,
                (bn, ssa_block),
                incoming,
                self.driver,
                &mut changed,
            )
        });

        // Statements.
        let throwing = self.regions.throwing.contains(&bn);
        self.driver.set_throwing(throwing);
        let run = sccp_process_statements(
            &mut state.values,
            ssa_block,
            self.ssa,
            StatementInputs {
                block: bn,
                escaping: self.escaping,
                has_dynamic_variable_trace: self.has_dynamic_variable_trace,
                clobbers: self.ssa.value_clobbers.get(&bn),
                catches: Some(CatchBodies {
                    bodies: &self.cfg.opaque_catch_bodies,
                    external: self.external,
                }),
            },
            self.driver,
            at.as_mut(),
        );
        self.driver.set_throwing(false);
        changed |= run.changed;
        if self
            .ssa
            .value_clobbers
            .get(&bn)
            .is_some_and(|markers| markers.contains_key(&usize::MAX))
        {
            changed |= clobber_at_unseen_call(
                &mut state.values,
                (
                    usize::MAX,
                    self.ssa.value_clobbers.get(&bn),
                    run.exit.completes,
                ),
                (self.driver, None),
            );
        }
        if let Some(at) = &at {
            at.before_terminator(self.driver);
        }

        // Terminator.
        let inputs = TerminatorInputs {
            cfg: self.cfg,
            ssa: self.ssa,
            values: &state.values,
            grammar: self.grammar,
            driver: self.driver,
            exit: run.exit,
            entry_edges: &self.regions.entry_edges,
        };
        changed |= sccp_process_terminator(
            bn,
            &inputs,
            &mut state.executable_blocks,
            &mut state.executable_edges,
            finalizing,
        );
        changed |= self.open_region_entries(
            bn,
            run.exit,
            (&mut state.executable_blocks, &mut state.executable_edges),
        );
        if let Some(at) = at {
            changed |= at.leave(self.driver, run.exit.entry_throws);
        }
        restore_values(&mut state.values, narrowed);
        changed
    }

    /// Enter `bn` in the refinement flow and narrow, in the lattice, each
    /// version a value-domain refinement in force there names; what each
    /// narrowed version held before, for [`restore_values`]. `changed` is
    /// set when the refinements the edges into `bn` carry moved.
    fn narrow(
        &self,
        bn: BlockId,
        state: &mut BlockSweepState,
        refinement_flow: &mut Option<super::RefinementFlow>,
        changed: &mut bool,
    ) -> Vec<(ValueKey, Option<LatticeValue>)> {
        let Some(flow) = refinement_flow.as_mut() else {
            return Vec::new();
        };
        let (moved, in_force) = flow.enter(self.cfg, self.preds, bn, &state.executable_edges);
        *changed |= moved;
        narrow_values(&mut state.values, self.refinements, in_force)
    }

    /// The region entries whose body starts at `bn`: the state before the
    /// body reaches the handler when the body's first command can fail
    /// before it stores anything, and otherwise never.
    fn open_region_entries(
        &self,
        bn: BlockId,
        exit: BlockExit,
        (blocks, edges): (&mut HashSet<BlockId>, &mut HashSet<(BlockId, BlockId)>),
    ) -> bool {
        let mut changed = false;
        if !exit.entry_throws {
            return changed;
        }
        for entry in self.regions.entries.get(&bn).into_iter().flatten() {
            changed |= edges.insert((entry.source, entry.handler));
            if self.cfg.blocks.contains_key(&entry.handler) {
                changed |= blocks.insert(entry.handler);
            }
        }
        changed
    }
}

/// Each φ's folded type: what every executable incoming value states alike
/// ([`crate::value_transfer::FoldedType::join_all`]). A live-in root states
/// nothing, so a φ over one states nothing; an incoming value the solver
/// has not reached is skipped, as the value join skips it.
fn record_phi_folded_types(
    values: &HashMap<ValueKey, LatticeValue>,
    ssa_block: &crate::ssa::SsaBlock,
    incoming_exec: &[BlockId],
    driver: &LatticeDriver<'_>,
) {
    if incoming_exec.is_empty() {
        return;
    }
    for phi in &ssa_block.phis {
        let members = incoming_exec.iter().filter_map(|pred| {
            let version = phi.incoming.get(pred).copied().unwrap_or(0);
            if version == 0 {
                return Some(None);
            }
            let key: ValueKey = (phi.name, version);
            match values.get(&key) {
                None | Some(LatticeValue::Unknown) => None,
                Some(_) => Some(driver.folded_of(key)),
            }
        });
        let folded = crate::value_transfer::FoldedType::join_all(members);
        driver.record_folded((phi.name, phi.version), folded);
    }
}
