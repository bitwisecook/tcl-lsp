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

//! The structural facts the relooper needs about a lowered function's control-flow graph.

use std::collections::HashSet;
use tcl_compiler::native_lowering::ir::{NativeFunction, NativeTerminator};

/// The structural facts the relooper needs.
pub(super) struct Shape {
    pub(super) rpo_index: Vec<usize>,
    pub(super) successors: Vec<Vec<usize>>,
    /// Dominator-tree children that are merge nodes, sorted by RPO index
    /// descending, per node.
    pub(super) merge_children: Vec<Vec<usize>>,
    pub(super) loop_headers: HashSet<usize>,
    pub(super) merge_nodes: HashSet<usize>,
}

impl Shape {
    #[allow(clippy::too_many_lines)]
    pub(super) fn of(function: &NativeFunction) -> Self {
        let count = function.blocks.len();
        let successors: Vec<Vec<usize>> = function
            .blocks
            .iter()
            .map(|block| match &block.terminator {
                NativeTerminator::Goto(target) => vec![target.index()],
                NativeTerminator::Branch {
                    then_target,
                    else_target,
                    ..
                } => vec![then_target.index(), else_target.index()],
                NativeTerminator::CompletionSwitch { cases, default, .. } => {
                    let mut targets: Vec<usize> =
                        cases.iter().map(|(_, target)| target.index()).collect();
                    targets.push(default.index());
                    targets
                }
                NativeTerminator::Return(_) => Vec::new(),
            })
            .collect();
        // Reverse post-order from the entry.
        let entry = function.entry.index();
        let mut state = vec![0u8; count];
        let mut post = Vec::with_capacity(count);
        let mut stack: Vec<(usize, usize)> = vec![(entry, 0)];
        state[entry] = 1;
        while let Some((block, next)) = stack.last_mut() {
            let block = *block;
            if *next < successors[block].len() {
                let target = successors[block][*next];
                *next += 1;
                if state[target] == 0 {
                    state[target] = 1;
                    stack.push((target, 0));
                }
            } else {
                state[block] = 2;
                post.push(block);
                stack.pop();
            }
        }
        post.reverse();
        let order = post;
        let mut rpo_index = vec![usize::MAX; count];
        for (position, block) in order.iter().enumerate() {
            rpo_index[*block] = position;
        }
        let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); count];
        for (from, targets) in successors.iter().enumerate() {
            if rpo_index[from] == usize::MAX {
                continue;
            }
            let mut seen = HashSet::new();
            for target in targets {
                if seen.insert(*target) {
                    predecessors[*target].push(from);
                }
            }
        }
        // Loop headers and merge nodes.
        let mut loop_headers = HashSet::new();
        let mut merge_nodes = HashSet::new();
        for block in &order {
            let forward = predecessors[*block]
                .iter()
                .filter(|pred| rpo_index[**pred] < rpo_index[*block])
                .count();
            let backward = predecessors[*block].len() - forward;
            if backward > 0 {
                loop_headers.insert(*block);
            }
            if forward > 1 {
                merge_nodes.insert(*block);
            }
        }
        // Immediate dominators (Cooper–Harvey–Kennedy).
        let mut idom = vec![usize::MAX; count];
        idom[entry] = entry;
        let mut changed = true;
        while changed {
            changed = false;
            for block in order.iter().skip(1) {
                let mut new_idom = usize::MAX;
                for pred in &predecessors[*block] {
                    if idom[*pred] == usize::MAX {
                        continue;
                    }
                    new_idom = if new_idom == usize::MAX {
                        *pred
                    } else {
                        intersect(&idom, &rpo_index, *pred, new_idom)
                    };
                }
                if new_idom != usize::MAX && idom[*block] != new_idom {
                    idom[*block] = new_idom;
                    changed = true;
                }
            }
        }
        let mut merge_children: Vec<Vec<usize>> = vec![Vec::new(); count];
        for block in &order {
            if *block == entry || idom[*block] == usize::MAX {
                continue;
            }
            if merge_nodes.contains(block) {
                merge_children[idom[*block]].push(*block);
            }
        }
        for children in &mut merge_children {
            children.sort_by(|a, b| rpo_index[*b].cmp(&rpo_index[*a]));
        }
        Self {
            rpo_index,
            successors,
            merge_children,
            loop_headers,
            merge_nodes,
        }
    }
}

fn intersect(idom: &[usize], rpo: &[usize], mut a: usize, mut b: usize) -> usize {
    while a != b {
        while rpo[a] > rpo[b] {
            a = idom[a];
        }
        while rpo[b] > rpo[a] {
            b = idom[b];
        }
    }
    a
}
