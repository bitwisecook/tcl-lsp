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

//! CFG linearisation and loop body detection.
//!
//! Pure functions on a `CfgFunction` — no emission state.

#![allow(dead_code, clippy::cast_possible_truncation, clippy::doc_markdown)]

use std::collections::{HashMap, HashSet};

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;

// Block-name prefix constants

/// Block-name prefixes for if/switch join blocks.
///
/// Incoming edges to these blocks carry a value on TOS (the arm's
/// result), which the emitter pops before continuing.
pub const VALUE_JOIN_PREFIXES: &[&str] = &["if_end_", "switch_end_"];

/// Block-name prefixes for loop exit blocks.
///
/// The loop command's result is always the empty string.
pub const LOOP_END_PREFIXES: &[&str] = &["while_end_", "for_end_", "foreach_end_"];

/// Block-name prefixes for loop header (condition test) blocks.
pub const LOOP_HEADER_PREFIXES: &[&str] = &["for_header_", "while_header_"];

/// Block-name prefixes for loop body blocks.
pub const LOOP_BODY_PREFIXES: &[&str] = &["while_body_", "for_body_"];

/// Return `true` if `name` starts with any of `prefixes`.
#[must_use]
pub fn starts_with_any(name: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|p| name.starts_with(p))
}

// Constant branch folding

/// Evaluate a branch condition at compile time.
///
/// Returns `Some(true)`/`Some(false)` for constant conditions, `None`
/// if the value is unknown at compile time.
#[must_use]
pub fn fold_const_branch(cond: &ExprNode) -> Option<bool> {
    // Only textual literals can be folded. Var/Command/Raw carry runtime
    // values and structured nodes (Binary/Unary/Ternary/Call) are handled
    // by the caller's own folding path — both collapse to `None` here.
    let (ExprNode::Literal { text, .. } | ExprNode::String { text, .. }) = cond else {
        return None;
    };

    let trimmed = text
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| text.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
        .unwrap_or(text);
    if let Ok(i) = trimmed.parse::<i64>() {
        return Some(i != 0);
    }
    if let Ok(f) = trimmed.parse::<f64>() {
        return Some(f != 0.0);
    }
    match trimmed.to_lowercase().as_str() {
        "true" | "yes" | "on" => Some(true),
        "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

// Linearisation

/// RPO traversal from entry, with dead-branch elimination.
///
/// Returns a block order suitable for emission: the entry block comes
/// first, each block's successors (if not already visited) come in a
/// layout-friendly order, and unreachable blocks (dead branches from
/// constant-folded conditions) are omitted.
#[must_use]
pub fn linearise(cfg: &CfgFunction) -> Vec<String> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut order: Vec<String> = Vec::new();
    let entry = cfg.block_name(cfg.entry).to_owned();
    dfs(cfg, &entry, &mut visited, &mut order);
    order.reverse();
    reorder_bottom_tested(cfg, order)
}

/// Depth-first traversal populating `order` in post-order.
fn dfs(cfg: &CfgFunction, name: &str, visited: &mut HashSet<String>, order: &mut Vec<String>) {
    if visited.contains(name) {
        return;
    }
    let Some(blk) = cfg.block_by_name(name) else {
        return;
    };
    visited.insert(name.to_owned());
    if let Some(term) = &blk.terminator {
        match term {
            Terminator::Goto { target, .. } => {
                let target = cfg.block_name(*target).to_owned();
                dfs(cfg, &target, visited, order);
            }
            Terminator::Branch {
                condition,
                true_target,
                false_target,
                ..
            } => {
                let true_target = cfg.block_name(*true_target).to_owned();
                let false_target = cfg.block_name(*false_target).to_owned();
                match fold_const_branch(condition) {
                    Some(true) => {
                        // For loop headers, still visit the exit block
                        // so that break jumps have a valid target.
                        if starts_with_any(name, LOOP_HEADER_PREFIXES) {
                            dfs(cfg, &false_target, visited, order);
                        }
                        dfs(cfg, &true_target, visited, order);
                    }
                    Some(false) => {
                        dfs(cfg, &false_target, visited, order);
                    }
                    None => {
                        // Visit false-target first so that in the
                        // reversed post-order the true-target (then-body)
                        // appears immediately after the condition block.
                        dfs(cfg, &false_target, visited, order);
                        dfs(cfg, &true_target, visited, order);
                    }
                }
            }
            Terminator::Return { .. } => {}
        }
    }
    order.push(name.to_owned());
}

// Loop body collection and reordering

/// Collect blocks reachable from `start` that are part of a loop back
/// to `header`.
///
/// Blocks that are the loop's `exit_block` (or beyond) are excluded so
/// that `break` jumps don't pull exit blocks into the body.
pub(crate) fn collect_loop_body(
    cfg: &CfgFunction,
    start: &str,
    header: &str,
    result: &mut HashSet<String>,
    exit_block: Option<&str>,
) {
    if start == header || result.contains(start) {
        return;
    }
    if exit_block == Some(start) {
        return;
    }
    let Some(blk) = cfg.block_by_name(start) else {
        return;
    };
    result.insert(start.to_owned());
    if let Some(term) = &blk.terminator {
        match term {
            Terminator::Goto { target, .. } => {
                let target = cfg.block_name(*target).to_owned();
                collect_loop_body(cfg, &target, header, result, exit_block);
            }
            Terminator::Branch {
                true_target,
                false_target,
                ..
            } => {
                let true_target = cfg.block_name(*true_target).to_owned();
                let false_target = cfg.block_name(*false_target).to_owned();
                collect_loop_body(cfg, &true_target, header, result, exit_block);
                collect_loop_body(cfg, &false_target, header, result, exit_block);
            }
            Terminator::Return { .. } => {}
        }
    }
}

/// Move loop body/step blocks before their header (condition test).
///
/// Produces a bottom-tested loop pattern: the conditional jump becomes
/// the back-edge, and the unconditional back-edge jump is eliminated
/// via fallthrough.
#[must_use]
pub fn reorder_bottom_tested(cfg: &CfgFunction, order: Vec<String>) -> Vec<String> {
    let pos: HashMap<String, usize> = order
        .iter()
        .enumerate()
        .map(|(i, n)| (n.clone(), i))
        .collect();

    // Find back-edges: blocks with a Goto to an earlier block in RPO.
    let mut back_edges: Vec<(String, String)> = Vec::new();
    for name in &order {
        let Some(blk) = cfg.block_by_name(name) else {
            continue;
        };
        if let Some(Terminator::Goto { target, .. }) = &blk.terminator {
            let target = cfg.block_name(*target);
            if pos.get(target).is_some_and(|tp| *tp < pos[name]) {
                back_edges.push((name.clone(), target.to_owned()));
            }
        }
    }

    if back_edges.is_empty() {
        return order;
    }

    // Process innermost loops first (later in RPO).
    back_edges.sort_by_key(|(src, _)| std::cmp::Reverse(pos[src]));

    let mut result = order;
    for (_back_src, header) in back_edges {
        let Some(header_blk) = cfg.block_by_name(&header) else {
            continue;
        };
        let Some(Terminator::Branch {
            true_target,
            false_target,
            ..
        }) = &header_blk.terminator
        else {
            continue;
        };

        // foreach uses a top-test pattern — skip.
        if header.starts_with("foreach_header_") {
            continue;
        }

        let body_start = cfg.block_name(*true_target).to_owned();
        let exit_block = cfg.block_name(*false_target).to_owned();
        let mut loop_blocks: HashSet<String> = HashSet::new();
        collect_loop_body(
            cfg,
            &body_start,
            &header,
            &mut loop_blocks,
            Some(&exit_block),
        );

        if loop_blocks.is_empty() {
            continue;
        }

        // Extract loop blocks preserving their relative order.
        let loop_ordered: Vec<String> = result
            .iter()
            .filter(|b| loop_blocks.contains(*b))
            .cloned()
            .collect();
        result.retain(|b| !loop_blocks.contains(b));
        // Insert them just before header.
        if let Some(h_pos) = result.iter().position(|b| b == &header) {
            for (i, b) in loop_ordered.into_iter().enumerate() {
                result.insert(h_pos + i, b);
            }
        }
    }

    result
}

// Loop context (continue / break targets)

/// Map each loop-body block to its `(continue_target, break_target)`.
///
/// For `for` loops, continue jumps to the step block; for `while` and
/// `foreach` loops, continue jumps to the header. Break always jumps
/// to the end block.
///
/// The continue target is `None` for a `for` loop's *step* block: a
/// `continue` in the `next` script propagates out of the loop rather than
/// re-running the step (C's `Tcl_ForObjCmd` gives the step its own exception
/// range with `continue -1`; a self-jump here would infinite-loop). The step
/// keeps its break target, since `break` in the step exits the loop cleanly.
#[must_use]
pub fn build_loop_context(cfg: &CfgFunction) -> HashMap<String, (Option<String>, String)> {
    /// One loop's resolved targets and the set of blocks in its body.
    struct LoopInfo {
        is_for: bool,
        cont_target: String,
        end_block: String,
        body: HashSet<String>,
    }

    let all_loop_headers: &[&str] = &["for_header_", "while_header_", "foreach_header_"];
    let mut loops: Vec<LoopInfo> = Vec::new();

    for (id, blk) in &cfg.blocks {
        let bname = cfg.block_name(*id);
        if !starts_with_any(bname, all_loop_headers) {
            continue;
        }
        let Some(Terminator::Branch {
            true_target,
            false_target,
            ..
        }) = &blk.terminator
        else {
            continue;
        };
        let body_start = cfg.block_name(*true_target).to_owned();
        let end_block = cfg.block_name(*false_target).to_owned();

        // Determine continue target.
        let cont_target = if bname.starts_with("for_header_") {
            // Find the step block: it has a Goto back to the header.
            let mut found: Option<String> = None;
            for (bn_id, bl) in &cfg.blocks {
                let bn = cfg.block_name(*bn_id);
                if !bn.starts_with("for_step_") {
                    continue;
                }
                if let Some(Terminator::Goto { target, .. }) = &bl.terminator
                    && *target == *id
                {
                    found = Some(bn.to_owned());
                    break;
                }
            }
            let Some(ct) = found else { continue };
            ct
        } else {
            bname.to_owned()
        };

        // Collect body blocks (excluding exit). For nested loops this set also
        // contains the inner loop's blocks (they are reachable), so the final
        // assignment is resolved innermost-first below.
        let mut body: HashSet<String> = HashSet::new();
        collect_loop_body(cfg, &body_start, bname, &mut body, Some(&end_block));
        loops.push(LoopInfo {
            is_for: bname.starts_with("for_header_"),
            cont_target,
            end_block,
            body,
        });
    }

    // A `break`/`continue` targets the *innermost* enclosing loop. A more-nested
    // loop has a strictly smaller body, so assign in ascending body size and keep
    // the first (innermost) writer — otherwise an outer loop, whose body subsumes
    // the inner one's blocks, would overwrite the inner continue target (HashMap
    // order made this nondeterministic) and an inner `continue` would jump to the
    // outer header, looping forever.
    loops.sort_by_key(|l| l.body.len());
    let mut ctx: HashMap<String, (Option<String>, String)> = HashMap::new();
    for l in &loops {
        for bb in &l.body {
            // The for-step block: a `continue` there propagates out (no jump
            // target), not a jump to the step itself.
            let cont = if l.is_for && *bb == l.cont_target {
                None
            } else {
                Some(l.cont_target.clone())
            };
            ctx.entry(bb.clone()).or_insert((cont, l.end_block.clone()));
        }
    }

    ctx
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{Block, Function as CfgFunction, Terminator};
    use crate::expr_ast::{BinOp, ExprNode};

    fn lit(text: &str) -> ExprNode {
        ExprNode::Literal {
            text: text.into(),
            start: 0,
            end: text.len() as u32,
        }
    }

    #[test]
    fn fold_const_branch_int() {
        assert_eq!(fold_const_branch(&lit("1")), Some(true));
        assert_eq!(fold_const_branch(&lit("0")), Some(false));
        assert_eq!(fold_const_branch(&lit("42")), Some(true));
    }

    #[test]
    fn fold_const_branch_bool_words() {
        assert_eq!(fold_const_branch(&lit("true")), Some(true));
        assert_eq!(fold_const_branch(&lit("false")), Some(false));
        assert_eq!(fold_const_branch(&lit("yes")), Some(true));
        assert_eq!(fold_const_branch(&lit("no")), Some(false));
    }

    #[test]
    fn fold_const_branch_var_none() {
        let v = ExprNode::Var {
            text: "$x".into(),
            name: "x".into(),
            start: 0,
            end: 2,
        };
        assert_eq!(fold_const_branch(&v), None);
    }

    #[test]
    fn fold_const_branch_binary_none() {
        let b = ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(lit("1")),
            right: Box::new(lit("2")),
        };
        assert_eq!(fold_const_branch(&b), None);
    }

    /// A CFG whose interner maps `names` (in order, entry first) to ids and
    /// inserts a block for each.
    fn cfg_with_blocks(names: &[&str]) -> CfgFunction {
        let mut cfg = CfgFunction::new("::top", names[0]);
        for name in &names[1..] {
            let id = cfg.intern_block(*name);
            cfg.blocks.insert(id, Block::new(*name));
        }
        cfg
    }

    fn bid(cfg: &CfgFunction, name: &str) -> crate::cfg::BlockId {
        cfg.block_id(name).expect("interned")
    }

    #[test]
    fn linearise_single_block() {
        let mut cfg = CfgFunction::new("::top", "entry_0");
        let entry = cfg.entry;
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        assert_eq!(linearise(&cfg), vec!["entry_0"]);
    }

    #[test]
    fn linearise_diamond() {
        // entry → branch → {then, else} → join → return
        let mut cfg = cfg_with_blocks(&["entry_0", "then_1", "else_1", "join_1"]);
        let entry = cfg.entry;
        let (then, els, join) = (
            bid(&cfg, "then_1"),
            bid(&cfg, "else_1"),
            bid(&cfg, "join_1"),
        );

        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Branch {
            condition: ExprNode::Var {
                text: "$x".into(),
                name: "x".into(),
                start: 0,
                end: 2,
            },
            true_target: then,
            false_target: els,
            span: None,
            condition_base: None,
        });
        cfg.blocks.get_mut(&then).unwrap().terminator = Some(Terminator::Goto {
            target: join,
            span: None,
        });
        cfg.blocks.get_mut(&els).unwrap().terminator = Some(Terminator::Goto {
            target: join,
            span: None,
        });
        cfg.blocks.get_mut(&join).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let order = linearise(&cfg);
        assert_eq!(order[0], "entry_0");
        assert_eq!(order[order.len() - 1], "join_1");
        assert!(order.contains(&"then_1".to_owned()));
        assert!(order.contains(&"else_1".to_owned()));
    }

    #[test]
    fn linearise_dead_branch_eliminated() {
        // Constant true condition — else branch is unreachable.
        let mut cfg = cfg_with_blocks(&["entry_0", "then_1", "else_1", "join_1"]);
        let entry = cfg.entry;
        let (then, els, join) = (
            bid(&cfg, "then_1"),
            bid(&cfg, "else_1"),
            bid(&cfg, "join_1"),
        );

        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Branch {
            condition: lit("1"),
            true_target: then,
            false_target: els,
            span: None,
            condition_base: None,
        });
        cfg.blocks.get_mut(&then).unwrap().terminator = Some(Terminator::Goto {
            target: join,
            span: None,
        });
        cfg.blocks.get_mut(&els).unwrap().terminator = Some(Terminator::Goto {
            target: join,
            span: None,
        });
        cfg.blocks.get_mut(&join).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let order = linearise(&cfg);
        assert!(!order.contains(&"else_1".to_owned()));
        assert!(order.contains(&"then_1".to_owned()));
    }

    #[test]
    fn starts_with_any_matches() {
        assert!(starts_with_any("if_end_3", VALUE_JOIN_PREFIXES));
        assert!(starts_with_any("switch_end_7", VALUE_JOIN_PREFIXES));
        assert!(!starts_with_any("entry_0", VALUE_JOIN_PREFIXES));
    }
}
