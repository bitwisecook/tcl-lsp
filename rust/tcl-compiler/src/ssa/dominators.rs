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

//! Dominators, immediate dominators, the dominance frontier, the dominator tree and its interval index.

use super::SsaFunction;
use crate::cfg;
use crate::cfg::BlockId;
use rustc_hash::{FxBuildHasher, FxHashMap};
use std::collections::{HashMap, HashSet};

/// Dominator-tree DFS interval numbering: answers `dominates(a, b)` in O(1).
///
/// The straightforward answer walks `b`'s immediate-dominator chain looking
/// for `a`, which is O(depth) with a hash lookup per hop — and on a flat
/// N-branch dispatch chain the idom chain *is* the whole function, so a
/// per-block-pair loop over it is O(V²).  A pre-order DFS of
/// the dominator tree instead assigns every block a half-open `[enter, exit)`
/// interval that contains exactly its dominator-tree subtree, and `a`
/// dominates `b` iff `b`'s interval nests inside `a`'s.
///
/// Blocks outside the dominator forest (unreachable, hence absent from
/// `idom`) have no interval and dominate nothing but themselves — the same
/// answer the chain walk gives, since it runs out of parents.
#[derive(Debug, Clone, Default)]
pub struct DominatorIntervals {
    /// `(enter, exit)` per block, indexed by [`BlockId`]`.0`.  `None` for a
    /// block the DFS never reached.
    pub(super) intervals: Vec<Option<(u32, u32)>>,
}

impl DominatorIntervals {
    /// Number `ssa`'s dominator forest.
    ///
    /// Children are derived from `idom` rather than read off
    /// [`SsaFunction::dominator_tree`], so the index is correct for any
    /// function whose `idom` is populated (including hand-built test
    /// fixtures that never ran the tree builder).
    ///
    /// The DFS starts at the entry block and then picks up any other root
    /// (a block whose `idom` is `None`), so a forest with more than one root
    /// is numbered in full rather than silently losing a component.
    #[must_use]
    pub(super) fn build(ssa: &SsaFunction) -> Self {
        let slots = ssa
            .idom
            .iter()
            .flat_map(|(id, parent)| [Some(*id), *parent])
            .flatten()
            .map(|b| b.0 as usize + 1)
            .chain(std::iter::once(ssa.block_names.len()))
            .max()
            .unwrap_or(0);
        let mut intervals = vec![None; slots];
        if ssa.idom.is_empty() {
            return Self { intervals };
        }
        let mut children: Vec<Vec<BlockId>> = vec![Vec::new(); slots];
        let mut roots: Vec<BlockId> = Vec::new();
        for (id, parent) in &ssa.idom {
            match parent {
                Some(p) => children[p.0 as usize].push(*id),
                None if *id != ssa.entry => roots.push(*id),
                None => {}
            }
        }
        for kids in &mut children {
            kids.sort_unstable();
        }
        roots.sort_unstable();
        if ssa.idom.contains_key(&ssa.entry) {
            roots.insert(0, ssa.entry);
        }

        let mut counter: u32 = 0;
        // Explicit stack of `(block, next child index)` — an iterative
        // pre-order DFS, so a deep dominator chain cannot blow the stack.
        let mut stack: Vec<(BlockId, usize)> = Vec::new();
        for root in roots {
            if intervals[root.0 as usize].is_some() {
                continue;
            }
            intervals[root.0 as usize] = Some((counter, counter));
            counter += 1;
            stack.push((root, 0));
            while let Some((node, child_idx)) = stack.pop() {
                let kids = &children[node.0 as usize];
                if child_idx < kids.len() {
                    stack.push((node, child_idx + 1));
                    let child = kids[child_idx];
                    if intervals[child.0 as usize].is_none() {
                        intervals[child.0 as usize] = Some((counter, counter));
                        counter += 1;
                        stack.push((child, 0));
                    }
                } else if let Some((_, exit)) = intervals[node.0 as usize].as_mut() {
                    *exit = counter;
                }
            }
        }
        Self { intervals }
    }

    /// True when `ancestor` dominates `node` (a block dominates itself).
    #[must_use]
    pub fn dominates(&self, ancestor: BlockId, node: BlockId) -> bool {
        if ancestor == node {
            return true;
        }
        let (Some(Some((a_enter, a_exit))), Some(Some((n_enter, n_exit)))) = (
            self.intervals.get(ancestor.0 as usize),
            self.intervals.get(node.0 as usize),
        ) else {
            return false;
        };
        *a_enter <= *n_enter && *n_exit <= *a_exit
    }
}

// Dominator algorithms

/// Compute the dominator sets for all blocks in a CFG function.
///
/// Uses the iterative dataflow algorithm. Returns a map from block
/// name to the set of blocks that dominate it.
///
/// The fixpoint visits blocks in **reverse postorder** so that each
/// block is processed after the predecessors it depends on, which is
/// what makes the iteration converge in a small constant number of
/// passes for a reducible CFG instead of one pass per block.  Driving
/// the fixpoint off the reachable-block *set* (arbitrary hash order)
/// instead made convergence O(blocks) passes — pathological on a proc
/// body that lowers to a long chain of branches (e.g. a 700-way
/// `if {$x == N} {...}` dispatch), where it turned an O(N²) job into
/// O(N³) and stalled the analyser.
#[must_use]
pub fn compute_dominators(func: &cfg::Function) -> HashMap<BlockId, HashSet<BlockId>> {
    let reachable = func.reachable_blocks();
    let mut dom: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();

    for id in func.blocks.keys() {
        if !reachable.contains(id) || *id == func.entry {
            dom.insert(*id, HashSet::from([*id]));
        } else {
            dom.insert(*id, reachable.clone());
        }
    }

    // Reverse postorder over blocks reachable from the entry — this is
    // exactly the reachable set, ordered so predecessors precede the
    // blocks that depend on them.
    let rpo = func.reverse_postorder();
    let preds = func.predecessors();
    let mut changed = true;
    while changed {
        changed = false;
        for id in &rpo {
            if *id == func.entry {
                continue;
            }
            let bn_preds: Vec<BlockId> = preds.get(id).map_or_default(|p| {
                p.iter()
                    .copied()
                    .filter(|p| reachable.contains(p))
                    .collect()
            });

            let new_dom = if bn_preds.is_empty() {
                HashSet::from([*id])
            } else {
                let mut inter = dom[&bn_preds[0]].clone();
                for p in &bn_preds[1..] {
                    inter = inter.intersection(&dom[p]).copied().collect();
                }
                inter.insert(*id);
                inter
            };

            if new_dom != dom[id] {
                dom.insert(*id, new_dom);
                changed = true;
            }
        }
    }
    dom
}

/// Compute immediate dominators directly via the Cooper-Harvey-Kennedy
/// "A Simple, Fast Dominance Algorithm".
///
/// Returns the same map shape as [`compute_idom`] (entry and
/// unreachable blocks map to `None`, every other reachable block to
/// `Some(parent)`), but **without** materialising the full dominator
/// *sets*: it works on reverse-postorder block indices and a single
/// `idom` pointer per block, so it is O(N·D) time and O(N) memory
/// rather than the O(N²) memory / O(N³) worst-case time of the
/// set-based [`compute_dominators`] + [`compute_idom`] pair.  This is
/// what keeps `build_ssa` bounded on pathologically large functions
/// (a single multi-thousand-branch generated proc would otherwise
/// exhaust memory building the dominator sets).
#[must_use]
pub(crate) fn compute_idom_fast(func: &cfg::Function) -> HashMap<BlockId, Option<BlockId>> {
    const UNDEF: usize = usize::MAX;

    // Shared iterative RPO — see `cfg::Function::reverse_postorder`.
    let rpo = func.reverse_postorder();
    let mut out: HashMap<BlockId, Option<BlockId>> = HashMap::new();
    for id in func.blocks.keys() {
        out.insert(*id, None);
    }
    if rpo.is_empty() {
        return out;
    }
    // Map block id → reverse-postorder index (entry == 0).
    let mut rpo_index: FxHashMap<BlockId, usize> =
        FxHashMap::with_capacity_and_hasher(rpo.len(), FxBuildHasher);
    for (i, n) in rpo.iter().enumerate() {
        rpo_index.insert(*n, i);
    }
    let preds = func.predecessors();

    let mut idom: Vec<usize> = vec![UNDEF; rpo.len()];
    idom[0] = 0; // entry is its own dominator (sentinel for the walk).

    // Walk up the idom tree from both nodes until they meet, using
    // RPO indices (a dominator always has a strictly smaller index).
    let intersect = |mut a: usize, mut b: usize, idom: &[usize]| -> usize {
        while a != b {
            while a > b {
                a = idom[a];
            }
            while b > a {
                b = idom[b];
            }
        }
        a
    };

    let mut changed = true;
    while changed {
        changed = false;
        // Skip the entry (index 0); process the rest in RPO order so
        // each block sees already-processed predecessors.
        for i in 1..rpo.len() {
            let mut new_idom = UNDEF;
            if let Some(ps) = preds.get(&rpo[i]) {
                for p in ps {
                    let Some(&pi) = rpo_index.get(p) else {
                        continue; // unreachable predecessor
                    };
                    if idom[pi] == UNDEF {
                        continue; // not processed yet this pass
                    }
                    new_idom = if new_idom == UNDEF {
                        pi
                    } else {
                        intersect(pi, new_idom, &idom)
                    };
                }
            }
            if new_idom != UNDEF && idom[i] != new_idom {
                idom[i] = new_idom;
                changed = true;
            }
        }
    }

    for (i, id) in rpo.iter().enumerate() {
        if i == 0 || idom[i] == UNDEF {
            out.insert(*id, None);
        } else {
            out.insert(*id, Some(rpo[idom[i]]));
        }
    }
    out
}

/// Compute immediate dominators from dominator sets.
///
/// The immediate dominator of a block is the closest strict dominator
/// (the one with the largest dominator set).
///
/// Retained as the reference set-based implementation that
/// [`compute_idom_fast`] (the production path) is cross-validated
/// against; see the `compute_idom_fast_matches_reference` test. Only
/// the fast path is used in production, so this is compiled for tests.
#[cfg(test)]
#[must_use]
pub(crate) fn compute_idom(
    func: &cfg::Function,
    dom: &HashMap<BlockId, HashSet<BlockId>>,
) -> HashMap<BlockId, Option<BlockId>> {
    let reachable = func.reachable_blocks();
    let mut idom: HashMap<BlockId, Option<BlockId>> = HashMap::new();

    for id in func.blocks.keys() {
        idom.insert(*id, None);
    }

    for id in &reachable {
        if *id == func.entry {
            continue;
        }
        let strict: HashSet<BlockId> = dom[id].iter().copied().filter(|d| d != id).collect();
        if strict.is_empty() {
            continue;
        }
        // The idom is the strict dominator with the largest dom set.
        let best = *strict.iter().max_by_key(|d| dom[*d].len()).unwrap();
        idom.insert(*id, Some(best));
    }
    idom
}

/// Compute the dominance frontier for each block.
///
/// A block `b` is in the dominance frontier of block `a` if `a`
/// dominates a predecessor of `b` but does not strictly dominate `b`.
#[must_use]
pub(crate) fn compute_dominance_frontier(
    func: &cfg::Function,
    idom: &HashMap<BlockId, Option<BlockId>>,
) -> HashMap<BlockId, HashSet<BlockId>> {
    let reachable = func.reachable_blocks();
    let preds = func.predecessors();
    let mut df: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();

    for id in func.blocks.keys() {
        df.insert(*id, HashSet::new());
    }

    for id in &reachable {
        let bn_preds: Vec<BlockId> = preds.get(id).map_or_default(|p| {
            p.iter()
                .copied()
                .filter(|p| reachable.contains(p))
                .collect()
        });

        if bn_preds.len() < 2 {
            continue;
        }

        for p in &bn_preds {
            let mut runner = Some(*p);
            while let Some(r) = runner {
                if idom.get(id).and_then(|i| i.as_ref()) == Some(&r) {
                    break;
                }
                df.entry(r).or_default().insert(*id);
                runner = idom.get(&r).copied().flatten();
            }
        }
    }
    df
}

/// Build the dominator tree from immediate dominators.
///
/// Returns a map from each block to its children in the dominator tree.
/// Children are sorted by [`BlockId`] (block-creation order) for
/// deterministic traversal.
#[must_use]
pub(crate) fn build_dom_tree(
    idom: &HashMap<BlockId, Option<BlockId>>,
) -> HashMap<BlockId, Vec<BlockId>> {
    let mut tree: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for id in idom.keys() {
        tree.entry(*id).or_default();
    }
    for (id, parent) in idom {
        if let Some(p) = parent {
            tree.entry(*p).or_default().push(*id);
        }
    }
    for children in tree.values_mut() {
        children.sort_unstable();
    }
    tree
}

pub(super) fn sorted_dominance_frontier(
    frontier: HashMap<BlockId, HashSet<BlockId>>,
) -> HashMap<BlockId, Vec<BlockId>> {
    frontier
        .into_iter()
        .map(|(k, v)| {
            let mut sorted: Vec<BlockId> = v.into_iter().collect();
            sorted.sort_unstable();
            (k, sorted)
        })
        .collect()
}
