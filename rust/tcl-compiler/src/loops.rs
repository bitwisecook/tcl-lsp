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

use crate::cfg::Function as CfgFunction;
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
/// (reflexive). Walks the idom chain — identical to Python's fallback.
#[must_use]
pub fn dominates(ssa: &SsaFunction, dominator: &str, node: &str) -> bool {
    let mut current = Some(node.to_owned());
    while let Some(c) = current {
        if c == dominator {
            return true;
        }
        current = ssa.idom.get(&c).and_then(Clone::clone);
    }
    false
}

/// Predecessor map restricted to `executable` blocks (terminator
/// successors only). Mirrors `cfg_predecessors`.
fn cfg_predecessors(
    cfg: &CfgFunction,
    executable: &HashSet<String>,
) -> HashMap<String, HashSet<String>> {
    let mut preds: HashMap<String, HashSet<String>> = executable
        .iter()
        .map(|b| (b.clone(), HashSet::new()))
        .collect();
    for bn in executable {
        if let Some(block) = cfg.blocks.get(bn)
            && let Some(term) = &block.terminator
        {
            for succ in term.successors() {
                if let Some(set) = preds.get_mut(succ) {
                    set.insert(bn.clone());
                }
            }
        }
    }
    preds
}

/// Blocks in the natural loop for one back edge `latch -> header`.
fn natural_loop_blocks(
    header: &str,
    latch: &str,
    preds: &HashMap<String, HashSet<String>>,
    executable: &HashSet<String>,
) -> HashSet<String> {
    let mut blocks: HashSet<String> = HashSet::new();
    blocks.insert(header.to_owned());
    blocks.insert(latch.to_owned());
    let mut work = vec![latch.to_owned()];
    while let Some(node) = work.pop() {
        if let Some(node_preds) = preds.get(&node) {
            for pred in node_preds {
                if !executable.contains(pred) || blocks.contains(pred) {
                    continue;
                }
                blocks.insert(pred.clone());
                if pred != header {
                    work.push(pred.clone());
                }
            }
        }
    }
    blocks
}

/// Build the natural-loop forest for `cfg` / `ssa`, restricted to the
/// `executable` blocks. Mirrors `build_loop_forest`.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn build_loop_forest(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable: &HashSet<String>,
) -> LoopForest {
    let preds = cfg_predecessors(cfg, executable);
    let mut order: Vec<String> = Vec::new();
    let mut blocks_by_header: HashMap<String, HashSet<String>> = HashMap::new();
    let mut latches_by_header: HashMap<String, HashSet<String>> = HashMap::new();

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
            if !executable.contains(succ) || !dominates(ssa, succ, &tail) {
                continue;
            }
            let body = natural_loop_blocks(succ, &tail, &preds, executable);
            if !blocks_by_header.contains_key(succ) {
                order.push(succ.to_owned());
            }
            blocks_by_header
                .entry(succ.to_owned())
                .or_default()
                .extend(body);
            latches_by_header
                .entry(succ.to_owned())
                .or_default()
                .insert(tail.clone());
        }
    }

    let loops = order
        .into_iter()
        .map(|header| {
            let mut blocks: Vec<String> = blocks_by_header[&header].iter().cloned().collect();
            blocks.sort();
            let mut latches: Vec<String> = latches_by_header[&header].iter().cloned().collect();
            latches.sort();
            NaturalLoop {
                header,
                blocks,
                latches,
            }
        })
        .collect();
    LoopForest { loops }
}
