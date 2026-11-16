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

//! Reachability and dominators of the executable CFG, for definition availability.

use super::{ExecutableBlockId, ExecutableFunction, ExecutableTerminator};
use std::collections::BTreeSet;

/// Reachability and dominators for the executable CFG.
///
/// Definition availability must use graph structure, not the incidental order
/// in the `blocks` vector: deterministic IDs are for reproducibility, not an
/// execution schedule.
///
/// Dominators are held as the dominator **tree** — one immediate dominator per
/// block, plus a DFS entry/exit numbering of that tree — rather than as a
/// per-block `BTreeSet` of every dominator. The set form is O(blocks²) in
/// memory and clones an O(blocks)-sized set per block per fixpoint round, which
/// is fine for the handful of blocks a single proc lowers to but not for the
/// unit's *top level*: a file of several hundred procedures flattens to
/// thousands of blocks there, and the set fixpoint then dominated whole-file
/// analysis time (tens of seconds on a ~5k-line file, growing quadratically).
/// The tree is built by the Cooper/Harvey/Kennedy iterative algorithm, which is
/// near-linear in practice, and [`Self::dominates`] becomes an O(1) interval
/// test. The answers are identical — only the representation changed.
pub(super) struct Dominance {
    reachable: BTreeSet<usize>,
    /// Dominator-tree DFS entry number and subtree-maximum entry number per
    /// block. `None` for a block unreachable from the entry: it dominates
    /// nothing and is dominated by nothing.
    intervals: Vec<Option<(u32, u32)>>,
}

impl Dominance {
    pub(super) fn compute(function: &ExecutableFunction) -> Self {
        let count = function.blocks.len();
        let mut successors = vec![Vec::new(); count];
        let mut predecessors = vec![Vec::new(); count];
        for (index, block) in function.blocks.iter().enumerate() {
            let Some(terminator) = block.terminator.as_ref() else {
                continue;
            };
            successors[index] = terminator_successors(terminator)
                .into_iter()
                .map(ExecutableBlockId::index)
                .collect();
            for successor in &successors[index] {
                predecessors[*successor].push(index);
            }
        }

        let entry = function.entry.index();
        // Reverse postorder is both the reachable set and the visit order the
        // dominator iteration below needs to converge in few rounds.
        let order = reverse_postorder(entry, &successors, count);
        let reachable: BTreeSet<usize> = order.iter().copied().collect();
        let idom = immediate_dominators(entry, &order, &predecessors, count);
        let intervals = dominator_tree_intervals(entry, &idom, &order, count);
        Self {
            reachable,
            intervals,
        }
    }

    pub(super) fn dominates(&self, definition: usize, use_block: usize) -> bool {
        if !self.reachable.contains(&use_block) {
            return definition == use_block;
        }
        // `definition` dominates `use_block` exactly when `use_block` lies in
        // `definition`'s dominator-tree subtree — a block's own interval
        // contains its entry number, so a block still dominates itself.
        let (Some((definition_in, definition_out)), Some((use_in, _))) = (
            self.intervals.get(definition).copied().flatten(),
            self.intervals.get(use_block).copied().flatten(),
        ) else {
            return false;
        };
        definition_in <= use_in && use_in <= definition_out
    }
}

/// Blocks reachable from `entry`, in reverse postorder (an iterative DFS, so a
/// deeply nested CFG cannot overflow the stack).
fn reverse_postorder(entry: usize, successors: &[Vec<usize>], count: usize) -> Vec<usize> {
    if entry >= count {
        return Vec::new();
    }
    let mut visited = vec![false; count];
    let mut postorder = Vec::new();
    visited[entry] = true;
    // Each frame is `(block, index of the next successor to walk)`.
    let mut stack = vec![(entry, 0_usize)];
    while let Some(frame) = stack.last_mut() {
        let block = frame.0;
        let next = frame.1;
        if let Some(successor) = successors[block].get(next).copied() {
            frame.1 += 1;
            if !visited[successor] {
                visited[successor] = true;
                stack.push((successor, 0));
            }
        } else {
            postorder.push(block);
            stack.pop();
        }
    }
    postorder.reverse();
    postorder
}

/// The immediate dominator of every block in `order`, by the Cooper/Harvey/
/// Kennedy iterative algorithm. `idom[entry] == Some(entry)`; a block that is
/// unreachable (absent from `order`) keeps `None`.
fn immediate_dominators(
    entry: usize,
    order: &[usize],
    predecessors: &[Vec<usize>],
    count: usize,
) -> Vec<Option<usize>> {
    let mut idom: Vec<Option<usize>> = vec![None; count];
    if entry >= count || order.is_empty() {
        return idom;
    }
    let mut rpo_number = vec![usize::MAX; count];
    for (position, block) in order.iter().enumerate() {
        rpo_number[*block] = position;
    }
    idom[entry] = Some(entry);
    let mut changed = true;
    while changed {
        changed = false;
        for block in order.iter().copied().filter(|block| *block != entry) {
            let mut candidate: Option<usize> = None;
            for predecessor in predecessors[block].iter().copied() {
                // A predecessor with no dominator yet is unreachable, or not
                // visited in this round; either way it contributes nothing.
                if idom[predecessor].is_none() {
                    continue;
                }
                candidate = Some(match candidate {
                    None => predecessor,
                    Some(current) => {
                        nearest_common_dominator(predecessor, current, &idom, &rpo_number)
                    }
                });
            }
            if candidate.is_some() && idom[block] != candidate {
                idom[block] = candidate;
                changed = true;
            }
        }
    }
    idom
}

/// Walk two blocks up the partially-built dominator tree to their nearest
/// common ancestor — the Cooper/Harvey/Kennedy `intersect`. Both walks strictly
/// decrease the reverse-postorder number and the entry is its own dominator, so
/// this always terminates.
fn nearest_common_dominator(
    mut a: usize,
    mut b: usize,
    idom: &[Option<usize>],
    rpo_number: &[usize],
) -> usize {
    while a != b {
        while rpo_number[a] > rpo_number[b] {
            match idom[a] {
                Some(parent) if parent != a => a = parent,
                // Reached the entry (or an unparented block): it is the only
                // common ancestor available.
                _ => return b,
            }
        }
        while rpo_number[b] > rpo_number[a] {
            match idom[b] {
                Some(parent) if parent != b => b = parent,
                _ => return a,
            }
        }
    }
    a
}

/// DFS entry number and subtree-maximum entry number for every block of the
/// dominator tree rooted at `entry`, so that "does `a` dominate `b`?" is the
/// interval test `in[a] <= in[b] <= out[a]`.
fn dominator_tree_intervals(
    entry: usize,
    idom: &[Option<usize>],
    order: &[usize],
    count: usize,
) -> Vec<Option<(u32, u32)>> {
    let mut intervals = vec![None; count];
    if entry >= count || order.is_empty() {
        return intervals;
    }
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); count];
    for block in order.iter().copied().filter(|block| *block != entry) {
        if let Some(parent) = idom[block] {
            children[parent].push(block);
        }
    }
    // Every non-entry node has exactly one parent, strictly earlier in reverse
    // postorder, so the tree is acyclic and this DFS visits each block once.
    let mut entry_number = vec![0_u32; count];
    let mut counter: u32 = 0;
    let mut stack = vec![(entry, 0_usize)];
    counter += 1;
    while let Some(frame) = stack.last_mut() {
        let block = frame.0;
        let next = frame.1;
        if let Some(child) = children[block].get(next).copied() {
            frame.1 += 1;
            entry_number[child] = counter;
            counter += 1;
            stack.push((child, 0));
        } else {
            intervals[block] = Some((entry_number[block], counter - 1));
            stack.pop();
        }
    }
    intervals
}

fn terminator_successors(terminator: &ExecutableTerminator) -> Vec<ExecutableBlockId> {
    match terminator {
        ExecutableTerminator::Goto(target) => vec![*target],
        ExecutableTerminator::RegionChoice { enter, skip } => vec![*enter, *skip],
        ExecutableTerminator::Branch {
            then_target,
            else_target,
            ..
        } => vec![*then_target, *else_target],
        ExecutableTerminator::CompletionSwitch { cases, default, .. } => cases
            .iter()
            .map(|case| case.target)
            .chain(std::iter::once(*default))
            .collect(),
        ExecutableTerminator::ReturnCompletion(_) => Vec::new(),
    }
}
