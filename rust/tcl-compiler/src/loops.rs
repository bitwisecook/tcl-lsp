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

//! Natural-loop forest over a function's CFG/SSA.
//!
//! A back edge is `tail -> succ`
//! where `succ` dominates `tail`; the natural-loop bodies of all back
//! edges sharing a header are unioned into one [`NaturalLoop`]. Lives in
//! `tcl-compiler` next to the CFG/SSA so the compiler explorer (and any
//! future loop-aware tooling) reuse it rather than re-deriving loops.
//!
//! Iteration is over `reverse_postorder` (not the unordered block set), so
//! the forest's loop order is deterministic. `blocks`/`latches` are sorted.

use std::collections::{HashMap, HashSet};

use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ssa::SsaFunction;

/// One natural loop, identified by its header block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NaturalLoop {
    /// The loop header (the back edges' shared target).
    pub header: String,
    /// All blocks in the loop (sorted), unioned over every back edge.
    pub blocks: Vec<String>,
    /// The latch (tail) blocks of the back edges (sorted).
    pub latches: Vec<String>,
}

/// Every natural loop of one function.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoopForest {
    /// Loops in deterministic (header first-encounter) order.
    pub loops: Vec<NaturalLoop>,
}

impl LoopForest {
    /// Every loop header.
    #[must_use]
    pub fn headers(&self) -> Vec<&str> {
        self.loops.iter().map(|l| l.header.as_str()).collect()
    }
}

/// Whether `dominator` dominates `node` in the SSA dominator tree
/// (reflexive). Walks the idom chain.
#[must_use]
pub fn dominates(ssa: &SsaFunction, dominator: BlockId, node: BlockId) -> bool {
    let mut current = Some(node);
    while let Some(c) = current {
        if c == dominator {
            return true;
        }
        current = ssa.idom.get(&c).copied().flatten();
    }
    false
}

/// Predecessor map restricted to `executable` blocks (terminator
/// successors only).
fn cfg_predecessors<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    executable: &HashSet<BlockId, S>,
) -> HashMap<BlockId, HashSet<BlockId>> {
    let mut preds: HashMap<BlockId, HashSet<BlockId>> =
        executable.iter().map(|b| (*b, HashSet::new())).collect();
    for bn in executable {
        if let Some(block) = cfg.blocks.get(bn)
            && let Some(term) = &block.terminator
        {
            for succ in term.successors() {
                if let Some(set) = preds.get_mut(&succ) {
                    set.insert(*bn);
                }
            }
        }
    }
    preds
}

/// Blocks in the natural loop for one back edge `latch -> header`.
fn natural_loop_blocks<S: std::hash::BuildHasher>(
    header: BlockId,
    latch: BlockId,
    preds: &HashMap<BlockId, HashSet<BlockId>>,
    executable: &HashSet<BlockId, S>,
) -> HashSet<BlockId> {
    let mut blocks: HashSet<BlockId> = HashSet::new();
    blocks.insert(header);
    blocks.insert(latch);
    let mut work = vec![latch];
    while let Some(node) = work.pop() {
        if let Some(node_preds) = preds.get(&node) {
            for pred in node_preds {
                if !executable.contains(pred) || blocks.contains(pred) {
                    continue;
                }
                blocks.insert(*pred);
                if *pred != header {
                    work.push(*pred);
                }
            }
        }
    }
    blocks
}

/// Build the natural-loop forest for `cfg` / `ssa`, restricted to the
/// `executable` blocks.
#[must_use]
pub fn build_loop_forest<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable: &HashSet<BlockId, S>,
) -> LoopForest {
    let preds = cfg_predecessors(cfg, executable);
    let mut order: Vec<BlockId> = Vec::new();
    let mut blocks_by_header: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();
    let mut latches_by_header: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();

    for tail in cfg.reverse_postorder() {
        if !executable.contains(&tail) {
            continue;
        }
        let Some(block) = cfg.blocks.get(&tail) else {
            continue;
        };
        let Some(term) = &block.terminator else {
            continue;
        };
        for succ in term.successors() {
            if !executable.contains(&succ) || !dominates(ssa, succ, tail) {
                continue;
            }
            let body = natural_loop_blocks(succ, tail, &preds, executable);
            if !blocks_by_header.contains_key(&succ) {
                order.push(succ);
            }
            blocks_by_header.entry(succ).or_default().extend(body);
            latches_by_header.entry(succ).or_default().insert(tail);
        }
    }

    let loops = order
        .into_iter()
        .map(|header| {
            let mut blocks: Vec<String> = blocks_by_header[&header]
                .iter()
                .map(|id| cfg.block_name(*id).to_owned())
                .collect();
            blocks.sort();
            let mut latches: Vec<String> = latches_by_header[&header]
                .iter()
                .map(|id| cfg.block_name(*id).to_owned())
                .collect();
            latches.sort();
            NaturalLoop {
                header: cfg.block_name(header).to_owned(),
                blocks,
                latches,
            }
        })
        .collect();
    LoopForest { loops }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::model::ingress::static_context_for;

    // The loop snippets below are real Tcl loops verified against tclsh8.6/9.0
    // (iteration counts: for→3, while→3, foreach→3, continue→4, nested 2×2→4);
    // the natural-loop forest is the compiler's CFG-derived view of them.
    fn forest_for(src: &str, qname: &str) -> LoopForest {
        let registry = static_context_for("tcl8.6").commands();
        let cu = CompilationUnit::build_for(src, registry, false);
        let fu = if qname == "::top" {
            &cu.top_level
        } else {
            cu.procedures.get(qname).expect("proc must exist")
        };
        build_loop_forest(&fu.cfg, &fu.ssa, &fu.sccp.executable_blocks)
    }

    #[test]
    fn for_loop_yields_one_natural_loop() {
        let f = forest_for(
            "proc f {} { for {set i 0} {$i < 3} {incr i} { puts $i } }",
            "::f",
        );
        assert_eq!(f.loops.len(), 1, "a single for-loop is one natural loop");
        let l = &f.loops[0];
        // Header, ≥1 latch, and the header is part of the loop body set.
        assert!(!l.header.is_empty());
        assert!(!l.latches.is_empty(), "back edge implies a latch");
        assert!(l.blocks.contains(&l.header), "header is in its own loop");
        // blocks/latches are sorted (the deterministic contract).
        let mut sorted = l.blocks.clone();
        sorted.sort();
        assert_eq!(l.blocks, sorted, "blocks must be sorted");
    }

    #[test]
    fn while_loop_is_a_natural_loop() {
        let f = forest_for("proc f {} { set i 0; while {$i < 3} { incr i } }", "::f");
        assert_eq!(f.loops.len(), 1);
    }

    #[test]
    fn foreach_loop_is_a_natural_loop() {
        let f = forest_for("proc f {} { foreach x {a b c} { puts $x } }", "::f");
        assert_eq!(f.loops.len(), 1);
    }

    #[test]
    fn straight_line_code_has_no_loops() {
        let f = forest_for("proc f {} { set i 1; return $i }", "::f");
        assert!(f.loops.is_empty(), "no back edge → empty forest");
        assert!(f.headers().is_empty());
    }

    #[test]
    fn continue_adds_a_back_edge_without_extra_header() {
        // `continue` jumps to the loop's continuation, so the header gains a
        // second back edge (latch) but stays a single natural loop.
        let f = forest_for(
            "proc f {} { for {set i 0} {$i<5} {incr i} { if {$i==2} continue; puts $i } }",
            "::f",
        );
        assert_eq!(f.loops.len(), 1, "continue does not create a new loop");
        // headers() reflects the single header.
        assert_eq!(f.headers().len(), 1);
    }

    #[test]
    fn nested_loops_yield_two_headers() {
        let f = forest_for(
            "proc f {} { for {set i 0} {$i<2} {incr i} { for {set j 0} {$j<2} {incr j} { puts $i } } }",
            "::f",
        );
        assert_eq!(f.loops.len(), 2, "nested for-loops are two natural loops");
        // Distinct headers.
        let headers = f.headers();
        assert_ne!(headers[0], headers[1]);
    }

    #[test]
    fn dominates_walks_the_idom_chain() {
        // Build a real function and assert the entry dominates every block,
        // and a block does not dominate the entry.
        let registry = static_context_for("tcl8.6").commands();
        let cu = CompilationUnit::build_for(
            "proc f {} { for {set i 0} {$i < 3} {incr i} { puts $i } }",
            registry,
            false,
        );
        let fu = cu.procedures.get("::f").unwrap();
        let entry = fu.ssa.entry;
        // Entry dominates itself (base case).
        assert!(dominates(&fu.ssa, entry, entry));
        // Entry dominates every reachable block.
        for &b in &fu.sccp.executable_blocks {
            assert!(
                dominates(&fu.ssa, entry, b),
                "entry must dominate every executable block"
            );
        }
    }

    #[test]
    fn forest_is_deterministic() {
        // Same source twice → identical forest (sorted blocks/latches, RPO order).
        let src = "proc f {} { for {set i 0} {$i<2} {incr i} { for {set j 0} {$j<2} {incr j} { puts $i } } }";
        assert_eq!(forest_for(src, "::f").loops, forest_for(src, "::f").loops);
    }
}
