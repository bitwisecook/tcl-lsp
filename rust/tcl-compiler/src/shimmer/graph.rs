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
//! - [`blocks_reaching`] — reverse-reachability query.
//! - [`build_successors`] — successor map from CFG terminators.

use std::collections::{HashMap, HashSet};

use crate::cfg::{Function as CfgFunction, Terminator};

/// Return the set of block names that are part of a loop body.
///
/// A block is "in a loop" if it lies on a cycle: there exists a
/// path from it back to itself via CFG successor edges.
#[must_use]
pub fn loop_body_blocks(cfg: &CfgFunction) -> HashSet<String> {
    let succs = build_successors(cfg);
    let mut loop_blocks: HashSet<String> = HashSet::new();
    for id in cfg.blocks.keys() {
        let start = cfg.block_name(*id);
        if loop_blocks.contains(start) {
            continue;
        }
        let mut visited: HashSet<String> = HashSet::new();
        let initial = succs.get(start).cloned().unwrap_or_default();
        let mut frontier = initial;
        let mut found = false;
        while let Some(bn) = frontier.pop() {
            if bn == *start {
                found = true;
                break;
            }
            if !visited.insert(bn.clone()) {
                continue;
            }
            if let Some(next) = succs.get(&bn) {
                frontier.extend(next.iter().cloned());
            }
        }
        if found {
            // A block is on the cycle iff it was visited on the
            // forward BFS AND it can reach `start` — otherwise
            // the BFS merely passed through it on a dead-end branch.
            let reaching = blocks_reaching(&succs, start);
            loop_blocks.insert(start.to_owned());
            for name in visited {
                if reaching.contains(&name) {
                    loop_blocks.insert(name);
                }
            }
        }
    }
    loop_blocks
}

/// Return the set of blocks that can reach `target` via `succs`.
#[must_use]
pub(crate) fn blocks_reaching(
    succs: &HashMap<String, Vec<String>>,
    target: &str,
) -> HashSet<String> {
    // Build reverse edges and BFS back from target.
    let mut preds: HashMap<String, Vec<String>> = HashMap::new();
    for (src, sx) in succs {
        for dst in sx {
            preds.entry(dst.clone()).or_default().push(src.clone());
        }
    }
    let mut visited: HashSet<String> = HashSet::new();
    let mut frontier: Vec<String> = vec![target.to_owned()];
    while let Some(bn) = frontier.pop() {
        if !visited.insert(bn.clone()) {
            continue;
        }
        if let Some(ps) = preds.get(&bn) {
            frontier.extend(ps.iter().cloned());
        }
    }
    visited
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

    #[test]
    fn blocks_reaching_walks_predecessors() {
        let mut succs: HashMap<String, Vec<String>> = HashMap::new();
        succs.insert("a".into(), vec!["b".into()]);
        succs.insert("b".into(), vec!["c".into()]);
        succs.insert("c".into(), Vec::new());
        let r = blocks_reaching(&succs, "c");
        assert!(r.contains("a"));
        assert!(r.contains("b"));
        assert!(r.contains("c"));
    }
}
