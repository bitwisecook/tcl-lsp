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

//! CFG graph helpers for shimmer analysis.
//!
//! - [`loop_body_blocks`] — identify blocks that are part of a cycle.
//! - [`build_successors`] — successor map from CFG terminators.

use std::collections::{HashMap, HashSet};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::cfg::{BlockId, Function as CfgFunction, Terminator};

/// Return the set of block names that are part of a loop body.
///
/// A block is "in a loop" if it lies on a cycle: there exists a
/// path from it back to itself via CFG successor edges.
///
/// That set is exactly the union of every **non-trivial** strongly-connected
/// component plus every self-looping block, so one Tarjan pass answers it in
/// O(V+E).  The previous formulation ran a fresh forward BFS from every block
/// and a reverse-reachability BFS for each cycle block it found — O(V·(V+E)),
/// which on a wide loop-free CFG is pure waste (every walk crosses the whole
/// graph and finds nothing) and took ~69 s on a 2000-branch flat proc
/// (issue #1240).
#[must_use]
pub fn loop_body_blocks(cfg: &CfgFunction) -> HashSet<String> {
    cycle_block_ids(cfg)
        .into_iter()
        .map(|id| cfg.block_name(id).to_owned())
        .collect()
}

/// `block → successors`, keyed by [`BlockId`], over the same edge set
/// [`build_successors`] exposes by name: `Goto` and `Branch` targets that are
/// present in the CFG.  Keeping the graph walk on ids avoids the per-edge name
/// clone the name-keyed map costs.
fn successor_ids(cfg: &CfgFunction) -> FxHashMap<BlockId, Vec<BlockId>> {
    let mut out: FxHashMap<BlockId, Vec<BlockId>> = FxHashMap::default();
    for (id, block) in &cfg.blocks {
        let mut s: Vec<BlockId> = Vec::new();
        match &block.terminator {
            Some(Terminator::Goto { target, .. }) if cfg.blocks.contains_key(target) => {
                s.push(*target);
            }
            Some(Terminator::Branch {
                true_target,
                false_target,
                ..
            }) => {
                if cfg.blocks.contains_key(true_target) {
                    s.push(*true_target);
                }
                if cfg.blocks.contains_key(false_target) {
                    s.push(*false_target);
                }
            }
            _ => {}
        }
        out.insert(*id, s);
    }
    out
}

/// Bookkeeping for the iterative Tarjan walk in [`cycle_block_ids`].
#[derive(Default)]
struct TarjanState {
    /// Discovery index per visited node.
    index: FxHashMap<BlockId, u32>,
    /// Lowest discovery index reachable from the node's subtree.
    low: FxHashMap<BlockId, u32>,
    /// Nodes currently on the SCC stack.
    on_stack: FxHashSet<BlockId>,
    /// The SCC stack itself.
    stack: Vec<BlockId>,
    /// Next discovery index to hand out.
    next_index: u32,
    /// Blocks found to lie on a cycle.
    cycle: FxHashSet<BlockId>,
}

impl TarjanState {
    /// Start a node: give it a discovery index and push it on the SCC stack.
    fn discover(&mut self, node: BlockId) {
        self.index.insert(node, self.next_index);
        self.low.insert(node, self.next_index);
        self.next_index += 1;
        self.stack.push(node);
        self.on_stack.insert(node);
    }

    /// Relax `node`'s low-link against `candidate`.
    fn relax(&mut self, node: BlockId, candidate: u32) {
        let low = self.low.entry(node).or_insert(candidate);
        *low = (*low).min(candidate);
    }

    /// Close `node`: when it roots an SCC, pop the component and record its
    /// members as cycle blocks if the component is non-trivial (more than one
    /// member, or a single member with a self-edge).
    fn close(&mut self, node: BlockId, has_self_edge: bool) {
        if self.low.get(&node) != self.index.get(&node) {
            return;
        }
        let mut members: Vec<BlockId> = Vec::new();
        while let Some(popped) = self.stack.pop() {
            self.on_stack.remove(&popped);
            members.push(popped);
            if popped == node {
                break;
            }
        }
        if members.len() > 1 || has_self_edge {
            self.cycle.extend(members);
        }
    }
}

/// Every block on a cycle, in one O(V+E) Tarjan strongly-connected-components
/// pass.  Iterative (an explicit `(node, next edge)` stack) so a long CFG
/// chain cannot blow the native stack.
fn cycle_block_ids(cfg: &CfgFunction) -> FxHashSet<BlockId> {
    let succs = successor_ids(cfg);
    let mut state = TarjanState::default();
    for &root in cfg.blocks.keys() {
        if state.index.contains_key(&root) {
            continue;
        }
        state.discover(root);
        let mut call: Vec<(BlockId, usize)> = vec![(root, 0)];
        while let Some(&(node, edge)) = call.last() {
            let edges: &[BlockId] = succs.get(&node).map_or(&[], Vec::as_slice);
            if edge < edges.len() {
                call.last_mut().expect("frame just observed").1 += 1;
                let next = edges[edge];
                if let Some(&seen) = state.index.get(&next) {
                    if state.on_stack.contains(&next) {
                        state.relax(node, seen);
                    }
                } else {
                    state.discover(next);
                    call.push((next, 0));
                }
                continue;
            }
            call.pop();
            let has_self_edge = edges.contains(&node);
            state.close(node, has_self_edge);
            if let Some(&(parent, _)) = call.last() {
                let low = state.low.get(&node).copied().unwrap_or(u32::MAX);
                state.relax(parent, low);
            }
        }
    }
    state.cycle
}

/// Build a `block → successors` adjacency map from CFG terminators.
///
/// Nodes are keyed by block NAME (the data-flow graph the shimmer passes
/// consume identifies blocks by name); terminator [`BlockId`] targets are
/// resolved back to their names via [`CfgFunction::block_name`].
pub(super) fn build_successors(cfg: &CfgFunction) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for (id, block) in &cfg.blocks {
        let mut s: Vec<String> = Vec::new();
        match &block.terminator {
            Some(Terminator::Goto { target, .. }) if cfg.blocks.contains_key(target) => {
                s.push(cfg.block_name(*target).to_owned());
            }
            Some(Terminator::Branch {
                true_target,
                false_target,
                ..
            }) => {
                if cfg.blocks.contains_key(true_target) {
                    s.push(cfg.block_name(*true_target).to_owned());
                }
                if cfg.blocks.contains_key(false_target) {
                    s.push(cfg.block_name(*false_target).to_owned());
                }
            }
            _ => {}
        }
        out.insert(cfg.block_name(*id).to_owned(), s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{Block, BlockId, Function, Terminator};
    use crate::expr_ast::ExprNode;

    /// Intern `name`, insert an empty block for it, and return its id.
    fn block(f: &mut Function, name: &str) -> BlockId {
        let id = f.intern_block(name);
        f.blocks.insert(id, Block::new(name));
        id
    }

    fn branch(cond: ExprNode, tt: BlockId, ft: BlockId) -> Terminator {
        Terminator::Branch {
            condition: cond,
            true_target: tt,
            false_target: ft,
            span: None,
            condition_base: None,
        }
    }

    fn goto(target: BlockId) -> Terminator {
        Terminator::Goto { target, span: None }
    }

    #[test]
    fn loop_body_blocks_detects_simple_cycle() {
        // entry → body → entry (back edge), entry → exit
        let mut f = Function::new("::top", "entry");
        let entry = f.entry;
        let body = block(&mut f, "body");
        let exit = block(&mut f, "exit");
        f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(
            ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            body,
            exit,
        ));
        f.blocks.get_mut(&body).unwrap().terminator = Some(goto(entry));
        f.blocks.get_mut(&exit).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let lb = loop_body_blocks(&f);
        assert!(lb.contains("entry"));
        assert!(lb.contains("body"));
        assert!(!lb.contains("exit"));
    }

    #[test]
    fn loop_body_blocks_linear_chain_is_empty() {
        let mut f = Function::new("::top", "entry");
        let entry = f.entry;
        let next = block(&mut f, "next");
        f.blocks.get_mut(&entry).unwrap().terminator = Some(goto(next));
        f.blocks.get_mut(&next).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        assert!(loop_body_blocks(&f).is_empty());
    }

    /// A block that merely *leads into* a loop, or that the loop leads out
    /// to, is not itself on a cycle — the SCC boundary has to be exact, since
    /// the S100/S101 split keys on it.
    #[test]
    fn loop_body_blocks_excludes_preheader_and_exit() {
        // pre → head → body → head, head → exit
        let mut f = Function::new("::top", "pre");
        let pre = f.entry;
        let head = block(&mut f, "head");
        let body = block(&mut f, "body");
        let exit = block(&mut f, "exit");
        f.blocks.get_mut(&pre).unwrap().terminator = Some(goto(head));
        f.blocks.get_mut(&head).unwrap().terminator = Some(branch(
            ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            body,
            exit,
        ));
        f.blocks.get_mut(&body).unwrap().terminator = Some(goto(head));
        f.blocks.get_mut(&exit).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let lb = loop_body_blocks(&f);
        assert!(lb.contains("head"));
        assert!(lb.contains("body"));
        assert!(!lb.contains("pre"), "a preheader is not on the cycle");
        assert!(!lb.contains("exit"), "an exit block is not on the cycle");
    }

    /// A block branching to itself is a one-member cycle: a trivial SCC with a
    /// self-edge, which Tarjan must not discard along with the ordinary
    /// singletons.
    #[test]
    fn loop_body_blocks_detects_self_loop() {
        let mut f = Function::new("::top", "entry");
        let entry = f.entry;
        let exit = block(&mut f, "exit");
        f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(
            ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            entry,
            exit,
        ));
        f.blocks.get_mut(&exit).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let lb = loop_body_blocks(&f);
        assert!(lb.contains("entry"));
        assert!(!lb.contains("exit"));
    }

    /// Two independent loops joined by a straight-line block: both cycles are
    /// found, the joining block is not.
    #[test]
    fn loop_body_blocks_handles_two_disjoint_cycles() {
        let mut f = Function::new("::top", "a");
        let a = f.entry;
        let mid = block(&mut f, "mid");
        let c = block(&mut f, "c");
        let done = block(&mut f, "done");
        let cond = || ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        };
        f.blocks.get_mut(&a).unwrap().terminator = Some(branch(cond(), a, mid));
        f.blocks.get_mut(&mid).unwrap().terminator = Some(goto(c));
        f.blocks.get_mut(&c).unwrap().terminator = Some(branch(cond(), c, done));
        f.blocks.get_mut(&done).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let lb = loop_body_blocks(&f);
        assert!(lb.contains("a"));
        assert!(lb.contains("c"));
        assert!(!lb.contains("mid"));
        assert!(!lb.contains("done"));
    }

    /// A wide, loop-free CFG — the shape issue #1240 measured — must come back
    /// empty, and must do so without walking the graph once per block.
    #[test]
    fn loop_body_blocks_wide_acyclic_chain_is_empty() {
        let mut f = Function::new("::top", "b0");
        let mut prev = f.entry;
        for i in 1..200 {
            let next = block(&mut f, &format!("b{i}"));
            let side = block(&mut f, &format!("s{i}"));
            f.blocks.get_mut(&prev).unwrap().terminator = Some(branch(
                ExprNode::Literal {
                    text: "1".into(),
                    start: 0,
                    end: 1,
                },
                side,
                next,
            ));
            f.blocks.get_mut(&side).unwrap().terminator = Some(Terminator::Return {
                value: None,
                span: None,
                expr: None,
                braced: false,
            });
            prev = next;
        }
        f.blocks.get_mut(&prev).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        assert!(loop_body_blocks(&f).is_empty());
    }
}
