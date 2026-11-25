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

//! The optimistic fixpoint over the blocks in reverse post-order: seeds, φ joins and the per-block step.

use super::super::{incoming_executable_predecessors, join, set_value, settle_reachability};
use super::branches::sccp_process_terminator;
use super::refinements::{narrow_values, restore_values};
use super::statements::{clobber_at_unseen_call, sccp_process_statements};
use super::{BlockExit, CatchBodies, StatementInputs, SweepContext, SweepState, TerminatorInputs};
use crate::analyses::LatticeValue;
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::value_transfer::LatticeDriver;
use rustc_hash::FxHashSet;
use std::collections::{HashMap, HashSet};

impl SweepContext<'_> {
    /// Settle the optimiser's block transfers over the shared reachability sweep.
    pub(super) fn settle(&self, order: &[BlockId], state: &mut SweepState) {
        settle_reachability(|finalizing| {
            // Every sweep re-evaluates each statement. Only the settled
            // sweep's route entries belong in the reported tally.
            self.driver.reset_tally_for_sweep();
            self.driver.reset_undecided_for_sweep();
            self.driver.open_iteration();
            let mut changed = false;
            for bn in order {
                if state.executable_blocks.contains(bn) {
                    changed |= self.block(*bn, state, finalizing);
                }
            }
            changed
        });
    }

    /// One executable block of a sweep: its φs, its statements, the
    /// existence rung through them, and its terminator's edges; whether
    /// anything moved.
    pub(in super::super) fn block(
        &self,
        bn: BlockId,
        state: &mut SweepState,
        finalizing: bool,
    ) -> bool {
        let Some(ssa_block) = self.ssa.blocks.get(&bn) else {
            return false;
        };
        let mut changed = false;
        let incoming_exec =
            incoming_executable_predecessors(bn, self.preds, &state.executable_edges);

        // Phi nodes (not at entry, only when some predecessor is
        // executable).
        if bn != self.cfg.entry {
            changed |= sccp_process_phis(&mut state.values, ssa_block, &incoming_exec);
            record_phi_folded_types(&state.values, ssa_block, &incoming_exec, self.driver);
        }

        // The refinements every executable edge into the block carries
        // narrow the versions they name for its statements and its
        // terminator; the φs above read each version's own value.
        let narrowed = self.narrow(bn, state, &mut changed);

        // The existence rung enters the block with the join of its
        // executable edges, which is each φ's fact.
        let mut at = state.existence.as_mut().map(|run| {
            run.enter(
                self.cfg,
                (bn, ssa_block),
                &incoming_exec,
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
    pub(super) fn narrow(
        &self,
        bn: BlockId,
        state: &mut SweepState,
        changed: &mut bool,
    ) -> Vec<(ValueKey, Option<LatticeValue>)> {
        let Some(flow) = state.refined.as_mut() else {
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

/// The lattice's starting values: the interprocedural parameter seed. The
/// seed keys on the parameter *name* (a stable, cache-safe identity); each
/// resolves to this build's interned symbol. A parameter never read in the
/// body isn't interned, and its seed slot would never be consulted, so
/// dropping it is behaviour-neutral.
pub(in super::super) fn seeded_values(
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
pub(in super::super) fn seed_live_in_roots<S: std::hash::BuildHasher>(
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
/// `true` if any lattice value changed. Extracted from [`sccp`].
pub(in super::super) fn sccp_process_phis(
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
            // A version-0 incoming is the entry / live-in root (a proc
            // parameter, global, or other caller-supplied value). Its
            // runtime value is unknown at compile time, so it joins in as
            // `Overdefined` — never skipped. Skipping it would let a phi
            // that merges a live-in with a defined-arm constant fold to
            // that constant, miscompiling any `if {$param} { set x k }`
            // followed by a test on `x`. This mirrors the interval pass,
            // which joins `TOP` for `inc == 0` (intervals.rs:570-574).
            // `seed_live_in_roots` pins `(name, 0)` `Overdefined`; the
            // explicit default keeps the join correct even if a feed was
            // never seeded.
            let key: ValueKey = (phi.name, incoming_ver);
            let candidate = values.get(&key).cloned().unwrap_or(if incoming_ver == 0 {
                LatticeValue::Overdefined
            } else {
                LatticeValue::Unknown
            });
            phi_val = join(&phi_val, &candidate);
        }
        if set_value(values, (phi.name, phi.version), &phi_val) {
            changed = true;
        }
    }
    changed
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
