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

//! Running a loop, or an opaque `catch` body, to its exit over the exact state the settled run proves it starts from.

use super::super::evaluation::DefValues;
use super::super::{EdgeRefinement, EnumeratedLoop};
use super::CatchBodies;
use super::refinements::{Narrowing, restore_values};
use super::statements::held_value;
use crate::analyses::LatticeValue;
use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::value_transfer::LatticeDriver;
use std::collections::{HashMap, HashSet};
use tcl_registry::value_transfer::{
    BindingKind, DependencyEvidence, Existence, FactDomain, FactView,
};

/// Run each loop whose start state the settled run proves to its exit
/// ([`crate::static_loops::enumerate_loop`]), and state what it leaves as an
/// exact-value refinement on every executable edge into the block it leaves
/// to, for the version of each place it wrote that is live there and that
/// the loop defines — a φ there, or a definition in its blocks — never the
/// version live where it starts: the loops run, and the refinements their
/// exits publish. A name the loop writes that is dead after it has no φ
/// where it leaves, so the version live there is the one from before the
/// loop, which its state does not describe. A loop that leaves with a
/// completion its plan does not absorb publishes nothing, since what it
/// leaves holds on that path alone, and one that declines is not run.
pub(super) fn enumerate_loops(
    (cfg, ssa, preds): (
        &CfgFunction,
        &SsaFunction,
        &HashMap<BlockId, HashSet<BlockId>>,
    ),
    (values, narrowing): (&mut HashMap<ValueKey, LatticeValue>, Narrowing<'_>),
    (executable, edges): (&HashSet<BlockId>, &HashSet<(BlockId, BlockId)>),
    existence_exits: Option<&HashMap<BlockId, Vec<Existence>>>,
    (driver, external): (&LatticeDriver<'_>, &dyn Fn(&str) -> bool),
) -> (Vec<EnumeratedLoop>, Vec<EdgeRefinement>) {
    let mut loops: Vec<(BlockId, &crate::cfg::LoopNode)> = cfg
        .loop_nodes
        .iter()
        .map(|(&end, node)| (end, node))
        .filter(|(end, node)| executable.contains(end) && executable.contains(&node.start))
        .collect();
    loops.sort_by_key(|(end, _)| end.0);
    let mut records = Vec::new();
    let mut published = Vec::new();
    for (end, node) in loops {
        let narrowed = narrowing.narrow(values, node.start);
        let state = start_state(
            ssa,
            node.start,
            values,
            existence_exits.and_then(|exits| exits.get(&node.start)),
        );
        restore_values(values, narrowed);
        let Ok(run) = crate::static_loops::enumerate_loop(
            driver,
            &node.statement,
            state,
            (true, crate::static_loops::DEFAULT_MAX_STATIC_LOOP_ITERS),
            external,
        ) else {
            continue;
        };
        let mut left = Vec::new();
        if run.completion == tcl_registry::value_transfer::CompletionOutcome::Normal
            && let Some(entry) = ssa.blocks.get(&end).map(|block| &block.entry_versions)
        {
            let before = ssa
                .blocks
                .get(&node.start)
                .map(|block| &block.exit_versions);
            for (name, slot) in &run.state {
                let crate::static_loops::Slot::Value(value) = slot else {
                    continue;
                };
                let Some(symbol) = ssa.var_symbol(name) else {
                    continue;
                };
                let Some(&version) = entry.get(&symbol) else {
                    continue;
                };
                let live_before = before
                    .and_then(|versions| versions.get(&symbol))
                    .copied()
                    .unwrap_or(0);
                if version == live_before {
                    continue;
                }
                for &pred in preds.get(&end).into_iter().flatten() {
                    if edges.contains(&(pred, end)) {
                        published.push(EdgeRefinement {
                            edge: (pred, end),
                            key: (symbol, version),
                            domain: FactDomain::ExactValue,
                            fact: FactView::Exact(value.clone(), None),
                            evidence: DependencyEvidence::default(),
                            loop_exit: true,
                        });
                    }
                }
                left.push((
                    name.clone(),
                    LatticeValue::Const(crate::value_transfer::exact_to_const(value)),
                ));
            }
        }
        records.push(EnumeratedLoop {
            exit_block: end,
            span: node.span,
            iterations: run.iterations,
            exit: run.exit,
            published: left,
        });
    }
    (records, published)
}

/// The exact state a loop starts from: what the block `start` leaves in each
/// place — the value the settled lattice proves exact at its exit, under the
/// refinements in force there, a scalar of unknown value where the existence
/// rung proves it bound as one, and unbound where the rung proves it so. A
/// place the run proves none of is not held, and a read of it or a store to
/// it declines the enumeration.
fn start_state(
    ssa: &SsaFunction,
    start: BlockId,
    values: &HashMap<ValueKey, LatticeValue>,
    exits: Option<&Vec<Existence>>,
) -> crate::static_loops::LoopState {
    use crate::static_loops::Slot;
    let mut state = crate::static_loops::LoopState::default();
    let Some(block) = ssa.blocks.get(&start) else {
        return state;
    };
    for (slot, name) in ssa.var_names().iter().enumerate() {
        let Ok(index) = u32::try_from(slot) else {
            continue;
        };
        let symbol = Symbol(index);
        let version = block.exit_versions.get(&symbol).copied().unwrap_or(0);
        match values.get(&(symbol, version)) {
            Some(LatticeValue::Const(constant)) => state.set(
                name.clone(),
                Slot::Value(crate::value_transfer::const_to_exact(constant)),
            ),
            _ => {
                if let Some(slot) = exits
                    .and_then(|facts| facts.get(slot))
                    .copied()
                    .and_then(slot_of_existence)
                {
                    state.set(name.clone(), slot);
                }
            }
        }
    }
    state
}

/// What a loop enumeration holds of a place whose value the lattice does not
/// prove exact, from the existence rung's fact: a scalar of unknown value
/// where it is proven bound as one, unbound where it is proven so, and
/// nothing otherwise.
fn slot_of_existence(fact: Existence) -> Option<crate::static_loops::Slot> {
    use crate::static_loops::Slot;
    match fact {
        Existence::Bound(BindingKind::Scalar) => Some(Slot::Scalar),
        Existence::Unbound => Some(Slot::Unbound),
        _ => None,
    }
}

/// What the names an opaque `catch` body writes hold after it, when the
/// enumeration runs the body to its end over the exact state before it
/// ([`crate::static_loops::enumerate_script`]): the `catch` absorbs however
/// the body completes, so each name holds what the body left — a value, or
/// unbound — and one it never wrote holds what it held. The marker at
/// `index` states the body's writes, reading each name's version before it,
/// and the call it stands ahead of reads the rest of what the body reads;
/// `existence` is the rung's state before the marker. `None` for any other
/// statement and for a body the enumeration does not run.
pub(super) fn catch_body_answer(
    (ssa_block, index): (&crate::ssa::SsaBlock, usize),
    (values, ssa): (&HashMap<ValueKey, LatticeValue>, &SsaFunction),
    (driver, catches): (&LatticeDriver<'_>, CatchBodies<'_>),
    existence: Option<&[Existence]>,
) -> Option<DefValues> {
    use crate::static_loops::Slot;
    use crate::value_transfer::{DefAnswer, ExistenceStep};
    let marker = ssa_block.statements.get(index)?;
    if !crate::ssa::is_arm_writes_marker(&marker.statement) {
        return None;
    }
    let host = &ssa_block.statements[crate::ssa::arm_writes_host(&ssa_block.statements, index)?];
    let body = catches.bodies.get(&host.statement.span())?;
    let mut state = crate::static_loops::LoopState::default();
    for (&symbol, &version) in marker.uses.iter().chain(&host.uses) {
        let name = ssa.var_name(symbol);
        match values.get(&(symbol, version)) {
            Some(LatticeValue::Const(constant)) => state.set(
                name.to_owned(),
                Slot::Value(crate::value_transfer::const_to_exact(constant)),
            ),
            _ => {
                if let Some(slot) = existence
                    .and_then(|facts| facts.get(symbol.0 as usize))
                    .copied()
                    .and_then(slot_of_existence)
                {
                    state.set(name.to_owned(), slot);
                }
            }
        }
    }
    let run = crate::static_loops::enumerate_script(
        driver,
        body,
        state,
        crate::static_loops::DEFAULT_MAX_STATIC_LOOP_ITERS,
        catches.external,
    )
    .ok()?;
    // The marker states every name the body writes; a write it does not
    // state is one the solver has no definition for.
    let stated: HashSet<&str> = marker
        .defs
        .keys()
        .map(|&symbol| ssa.var_name(symbol))
        .collect();
    if run
        .written
        .iter()
        .any(|name| !stated.contains(name.as_str()))
    {
        return None;
    }
    let answers = marker
        .defs
        .iter()
        .map(|(&symbol, &version)| {
            let key = (symbol, version);
            let name = ssa.var_name(symbol);
            match (run.written.contains(name), run.state.get(name)) {
                (true, Some(Slot::Value(value))) => Some(DefAnswer {
                    stated: true,
                    existence: ExistenceStep::Set(Existence::Bound(BindingKind::Scalar)),
                    ..DefAnswer::untyped(
                        key,
                        LatticeValue::Const(crate::value_transfer::exact_to_const(value)),
                    )
                }),
                (true, Some(Slot::Unbound)) => Some(DefAnswer {
                    stated: true,
                    existence: ExistenceStep::Set(Existence::Unbound),
                    ..DefAnswer::untyped(key, LatticeValue::Overdefined)
                }),
                (true, _) => None,
                (false, _) => {
                    let prior = marker.uses.get(&symbol).copied().unwrap_or(0);
                    Some(DefAnswer {
                        preserved: true,
                        existence: ExistenceStep::PRESERVE,
                        ..DefAnswer::untyped(key, held_value(values, (symbol, prior)))
                    })
                }
            }
        })
        .collect::<Option<Vec<_>>>()?;
    Some(DefValues::PerDef(answers))
}
