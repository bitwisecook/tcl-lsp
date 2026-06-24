//! Shared control-flow-graph edge-routing model for the compiler explorer.
//!
//! The web GUI (SVG)
//! and the CLI/TUI (ASCII box-drawing gutter) draw control-flow edges in
//! different media, but they must agree on *routing* — which edges exist
//! and which lane each one occupies — so a reader sees the same graph
//! everywhere. That model lives here, once, next to the CFG so the
//! serialiser, the CLI/TUI renderers, and the planned diagram extractor
//! all reuse it rather than each re-deriving lanes.
//!
//! Lane assignment is shortest span first → innermost lane, greedy
//! interval colouring over block ordinals, so every surface nests edges
//! identically. Only the *drawing* differs between surfaces; the model
//! does not.

use std::collections::HashMap;

use crate::cfg::{Function, Terminator};

/// Which kind of control-flow edge this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// Unconditional jump.
    Goto,
    /// The `true` arm of a conditional branch.
    True,
    /// The `false` arm of a conditional branch.
    False,
}

impl EdgeKind {
    /// The contract string the explorer JSON / renderers use.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Goto => "goto",
            Self::True => "true",
            Self::False => "false",
        }
    }
}

/// A routed control-flow edge between two blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfgEdge {
    /// Source block name.
    pub src: String,
    /// Destination block name.
    pub dst: String,
    /// Ordinal index of the source block (0-based, in the supplied order).
    pub src_pos: usize,
    /// Ordinal index of the destination block.
    pub dst_pos: usize,
    /// Edge kind.
    pub kind: EdgeKind,
    /// Routing lane (0 = innermost, nearest the blocks).
    pub lane: usize,
}

/// Greedy interval colouring: shortest span first → innermost lane.
///
/// `spans` is a list of `(from_pos, to_pos)` ordinal pairs. Returns one
/// lane index per span (in input order). Two spans share a lane only if
/// their closed ordinal intervals do not overlap (touching endpoints count
/// as overlap, so an edge into a block and an edge out of that same block
/// never share a lane).
#[must_use]
pub fn assign_lanes(spans: &[(usize, usize)]) -> Vec<usize> {
    // Shortest span first; stable so equal-length spans keep input order.
    let mut by_span: Vec<usize> = (0..spans.len()).collect();
    by_span.sort_by_key(|&i| spans[i].1.abs_diff(spans[i].0));

    let mut lanes: Vec<Vec<(usize, usize)>> = Vec::new();
    let mut result = vec![0usize; spans.len()];
    for i in by_span {
        let (a, b) = spans[i];
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let mut placed: Option<usize> = None;
        for (lane, intervals) in lanes.iter_mut().enumerate() {
            if intervals.iter().all(|&(x, y)| hi < x || lo > y) {
                intervals.push((lo, hi));
                placed = Some(lane);
                break;
            }
        }
        result[i] = placed.unwrap_or_else(|| {
            lanes.push(vec![(lo, hi)]);
            lanes.len() - 1
        });
    }
    result
}

/// Block names in their canonical display order.
///
/// Block names are minted as `{prefix}_{counter}` with a monotonically
/// increasing counter (`cfg_builder::new_block`), so the creation order —
/// the order the explorer renders in — is recovered by sorting on the
/// trailing numeric suffix. Rust's `Function.blocks` is a `HashMap`, so
/// callers that need this ordering (the CFG view, the edge
/// router) go through this rather than iterating the map.
#[must_use]
pub fn ordered_block_names(func: &Function) -> Vec<String> {
    let mut names: Vec<String> = func
        .blocks
        .keys()
        .map(|id| func.block_name(*id).to_owned())
        .collect();
    names.sort_by_key(|name| (suffix_ordinal(name), name.clone()));
    names
}

/// Parse the trailing `_<n>` counter from a block name; `u64::MAX` when
/// absent so unnumbered names sort last (and then lexicographically).
fn suffix_ordinal(name: &str) -> u64 {
    name.rsplit_once('_')
        .and_then(|(_, n)| n.parse::<u64>().ok())
        .unwrap_or(u64::MAX)
}

/// Extract control-flow edges from `func` with routing lanes assigned.
///
/// `order` is the block-name ordering the caller wants ordinals computed
/// against (Rust's `Function.blocks` is a `HashMap`, so the order is the
/// caller's concern — typically the CFG view's display order). Edges follow
/// terminator successors only (the same set the CFG view draws); `try`
/// exception edges are analysis-only and not shown here.
#[must_use]
pub fn build_cfg_edges(func: &Function, order: &[String]) -> Vec<CfgEdge> {
    let pos: HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(i, name)| (name.as_str(), i))
        .collect();

    let mut raw: Vec<(String, String, usize, usize, EdgeKind)> = Vec::new();
    for name in order {
        let Some(block) = func.block_by_name(name) else {
            continue;
        };
        let Some(term) = &block.terminator else {
            continue;
        };
        let src_pos = pos[name.as_str()];
        for target in term.successors() {
            let target_name = func.block_name(target);
            let Some(&dst_pos) = pos.get(target_name) else {
                continue;
            };
            let kind = match term {
                Terminator::Branch { true_target, .. } => {
                    if target == *true_target {
                        EdgeKind::True
                    } else {
                        EdgeKind::False
                    }
                }
                _ => EdgeKind::Goto,
            };
            raw.push((name.clone(), target_name.to_owned(), src_pos, dst_pos, kind));
        }
    }

    let spans: Vec<(usize, usize)> = raw.iter().map(|&(_, _, sp, dp, _)| (sp, dp)).collect();
    let lanes = assign_lanes(&spans);
    raw.into_iter()
        .zip(lanes)
        .map(|((src, dst, src_pos, dst_pos, kind), lane)| CfgEdge {
            src,
            dst,
            src_pos,
            dst_pos,
            kind,
            lane,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_spans_assign_no_lanes() {
        assert_eq!(assign_lanes(&[]), Vec::<usize>::new());
    }

    #[test]
    fn non_overlapping_spans_share_lane_zero() {
        // (0,1) and (2,3) do not overlap → both innermost.
        assert_eq!(assign_lanes(&[(0, 1), (2, 3)]), vec![0, 0]);
    }

    #[test]
    fn touching_endpoints_count_as_overlap() {
        // (0,2) and (2,4) share block 2 → must take different lanes.
        // Shortest-first is a tie (both length 2); stable order keeps
        // index 0 placed first on lane 0, index 1 onto lane 1.
        assert_eq!(assign_lanes(&[(0, 2), (2, 4)]), vec![0, 1]);
    }

    #[test]
    fn shortest_span_gets_innermost_lane() {
        // Long span (0,4) processed after short (1,2): the short one is
        // placed first (lane 0), the long overlapping one onto lane 1.
        assert_eq!(assign_lanes(&[(0, 4), (1, 2)]), vec![1, 0]);
    }

    #[test]
    fn reversed_span_is_normalised() {
        // (4,0) overlaps (1,2); same result as the forward orientation.
        assert_eq!(assign_lanes(&[(4, 0), (1, 2)]), vec![1, 0]);
    }
}
