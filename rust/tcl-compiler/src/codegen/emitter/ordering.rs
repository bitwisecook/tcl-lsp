//! CFG linearisation and loop body detection.
//!
//! Pure functions on a `CfgFunction` — no emission state.

#![allow(dead_code, clippy::cast_possible_truncation, clippy::doc_markdown)]

use std::collections::{HashMap, HashSet};

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;

// ---------------------------------------------------------------------------
// Block-name prefix constants
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Constant branch folding
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Linearisation
// ---------------------------------------------------------------------------

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
    dfs(cfg, &cfg.entry, &mut visited, &mut order);
    order.reverse();
    reorder_bottom_tested(cfg, order)
}

/// Depth-first traversal populating `order` in post-order.
fn dfs(cfg: &CfgFunction, name: &str, visited: &mut HashSet<String>, order: &mut Vec<String>) {
    if visited.contains(name) || !cfg.blocks.contains_key(name) {
        return;
    }
    visited.insert(name.to_owned());
    let blk = &cfg.blocks[name];
    if let Some(term) = &blk.terminator {
        match term {
            Terminator::Goto { target, .. } => {
                dfs(cfg, target, visited, order);
            }
            Terminator::Branch {
                condition,
                true_target,
                false_target,
                ..
            } => {
                match fold_const_branch(condition) {
                    Some(true) => {
                        // For loop headers, still visit the exit block
                        // so that break jumps have a valid target.
                        if starts_with_any(name, LOOP_HEADER_PREFIXES) {
                            dfs(cfg, false_target, visited, order);
                        }
                        dfs(cfg, true_target, visited, order);
                    }
                    Some(false) => {
                        dfs(cfg, false_target, visited, order);
                    }
                    None => {
                        // Visit false-target first so that in the
                        // reversed post-order the true-target (then-body)
                        // appears immediately after the condition block.
                        dfs(cfg, false_target, visited, order);
                        dfs(cfg, true_target, visited, order);
                    }
                }
            }
            Terminator::Return { .. } => {}
        }
    }
    order.push(name.to_owned());
}

// ---------------------------------------------------------------------------
// Loop body collection and reordering
// ---------------------------------------------------------------------------

/// Collect blocks reachable from `start` that are part of a loop back
/// to `header`.
///
/// Blocks that are the loop's `exit_block` (or beyond) are excluded so
/// that `break` jumps don't pull exit blocks into the body.
#[allow(clippy::implicit_hasher)]
pub fn collect_loop_body(
    cfg: &CfgFunction,
    start: &str,
    header: &str,
    result: &mut HashSet<String>,
    exit_block: Option<&str>,
) {
    if start == header || result.contains(start) || !cfg.blocks.contains_key(start) {
        return;
    }
    if exit_block == Some(start) {
        return;
    }
    result.insert(start.to_owned());
    let blk = &cfg.blocks[start];
    if let Some(term) = &blk.terminator {
        match term {
            Terminator::Goto { target, .. } => {
                collect_loop_body(cfg, target, header, result, exit_block);
            }
            Terminator::Branch {
                true_target,
                false_target,
                ..
            } => {
                collect_loop_body(cfg, true_target, header, result, exit_block);
                collect_loop_body(cfg, false_target, header, result, exit_block);
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
        let Some(blk) = cfg.blocks.get(name) else {
            continue;
        };
        if let Some(Terminator::Goto { target, .. }) = &blk.terminator {
            if pos.get(target).is_some_and(|tp| *tp < pos[name]) {
                back_edges.push((name.clone(), target.clone()));
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
        let Some(header_blk) = cfg.blocks.get(&header) else {
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

        let body_start = true_target.clone();
        let exit_block = false_target.clone();
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

// ---------------------------------------------------------------------------
// Loop context (continue / break targets)
// ---------------------------------------------------------------------------

/// Map each loop-body block to its `(continue_target, break_target)`.
///
/// For `for` loops, continue jumps to the step block; for `while` and
/// `foreach` loops, continue jumps to the header. Break always jumps
/// to the end block.
#[must_use]
pub fn build_loop_context(cfg: &CfgFunction) -> HashMap<String, (String, String)> {
    let all_loop_headers: &[&str] = &["for_header_", "while_header_", "foreach_header_"];
    let mut ctx: HashMap<String, (String, String)> = HashMap::new();

    for (bname, blk) in &cfg.blocks {
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
        let body_start = true_target.clone();
        let end_block = false_target.clone();

        // Determine continue target.
        let cont_target = if bname.starts_with("for_header_") {
            // Find the step block: it has a Goto back to the header.
            let mut found: Option<String> = None;
            for (bn, bl) in &cfg.blocks {
                if !bn.starts_with("for_step_") {
                    continue;
                }
                if let Some(Terminator::Goto { target, .. }) = &bl.terminator {
                    if target == bname {
                        found = Some(bn.clone());
                        break;
                    }
                }
            }
            let Some(ct) = found else { continue };
            ct
        } else {
            bname.clone()
        };

        // Collect body blocks (excluding exit).
        let mut body_blocks: HashSet<String> = HashSet::new();
        collect_loop_body(cfg, &body_start, bname, &mut body_blocks, Some(&end_block));
        for bb in body_blocks {
            ctx.insert(bb, (cont_target.clone(), end_block.clone()));
        }
    }

    ctx
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

    #[test]
    fn linearise_single_block() {
        let mut cfg = CfgFunction::new("::top", "entry_0");
        cfg.blocks.get_mut("entry_0").unwrap().terminator = Some(Terminator::Return {
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
        let mut cfg = CfgFunction::new("::top", "entry_0");
        cfg.blocks.insert("then_1".into(), Block::new("then_1"));
        cfg.blocks.insert("else_1".into(), Block::new("else_1"));
        cfg.blocks.insert("join_1".into(), Block::new("join_1"));

        cfg.blocks.get_mut("entry_0").unwrap().terminator = Some(Terminator::Branch {
            condition: ExprNode::Var {
                text: "$x".into(),
                name: "x".into(),
                start: 0,
                end: 2,
            },
            true_target: "then_1".into(),
            false_target: "else_1".into(),
            span: None,
        });
        cfg.blocks.get_mut("then_1").unwrap().terminator = Some(Terminator::Goto {
            target: "join_1".into(),
            span: None,
        });
        cfg.blocks.get_mut("else_1").unwrap().terminator = Some(Terminator::Goto {
            target: "join_1".into(),
            span: None,
        });
        cfg.blocks.get_mut("join_1").unwrap().terminator = Some(Terminator::Return {
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
        let mut cfg = CfgFunction::new("::top", "entry_0");
        cfg.blocks.insert("then_1".into(), Block::new("then_1"));
        cfg.blocks.insert("else_1".into(), Block::new("else_1"));
        cfg.blocks.insert("join_1".into(), Block::new("join_1"));

        cfg.blocks.get_mut("entry_0").unwrap().terminator = Some(Terminator::Branch {
            condition: lit("1"),
            true_target: "then_1".into(),
            false_target: "else_1".into(),
            span: None,
        });
        cfg.blocks.get_mut("then_1").unwrap().terminator = Some(Terminator::Goto {
            target: "join_1".into(),
            span: None,
        });
        cfg.blocks.get_mut("else_1").unwrap().terminator = Some(Terminator::Goto {
            target: "join_1".into(),
            span: None,
        });
        cfg.blocks.get_mut("join_1").unwrap().terminator = Some(Terminator::Return {
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
