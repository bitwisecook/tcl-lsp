//! Try/finally CFG pattern detection.
//!
//! The CFG builder emits try/finally as a chain of blocks:
//!
//! ```text
//! try_body_N → try_end_N → try_finally_N → try_after_finally_N
//! ```
//!
//! This module walks the chain and records where to splice in the
//! inline `beginCatch4`/`endCatch` bytecodes. The emission itself
//! lives in `crate::codegen::control_flow::CodegenCtx::emit_try_finally_inline`.

#![allow(dead_code)]

use std::collections::HashMap;

use crate::cfg::{Function as CfgFunction, Terminator};

/// Links identified by a try/finally block chain.
#[derive(Debug, Clone)]
pub struct TryFinallyInfo {
    /// `try_end_N` block name.
    pub try_end: String,
    /// `try_finally_N` block name.
    pub try_finally: String,
    /// `try_after_finally_N` block name.
    pub try_after: String,
}

/// Detect try/finally patterns in the CFG.
///
/// Returns a map from the `try_body_N` block name to the chain info.
/// The `try_end`, `try_finally`, and `try_after_finally` blocks should
/// be added to the emitter's skip set — they are consumed as a unit.
#[must_use]
pub fn detect_try_finally(
    cfg: &CfgFunction,
    block_order: &[String],
) -> HashMap<String, TryFinallyInfo> {
    let mut result: HashMap<String, TryFinallyInfo> = HashMap::new();

    for bname in block_order {
        if !bname.starts_with("try_body_") {
            continue;
        }
        // Follow Goto chain from try_body to find try_end.
        let try_end = follow_until_prefix(cfg, bname, "try_end_");
        let Some(te) = try_end else { continue };
        // Follow from try_end to find try_finally (direct Goto).
        let Some(tf) = direct_goto(cfg, &te) else {
            continue;
        };
        if !tf.starts_with("try_finally_") {
            continue;
        }
        // Follow from try_finally to find try_after_finally.
        let Some(ta) = follow_until_prefix(cfg, &tf, "try_after_finally_") else {
            continue;
        };
        result.insert(
            bname.clone(),
            TryFinallyInfo {
                try_end: te,
                try_finally: tf,
                try_after: ta,
            },
        );
    }

    result
}

/// Follow a chain of `Goto` terminators until reaching a block whose
/// name starts with `prefix`. Returns that block's name, or `None` if
/// the chain ends without finding one.
fn follow_until_prefix(cfg: &CfgFunction, start: &str, prefix: &str) -> Option<String> {
    let mut current = start.to_owned();
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    loop {
        if !visited.insert(current.clone()) {
            return None;
        }
        let blk = cfg.blocks.get(&current)?;
        let Some(Terminator::Goto { target, .. }) = &blk.terminator else {
            return None;
        };
        if target.starts_with(prefix) {
            return Some(target.clone());
        }
        current = target.clone();
    }
}

/// Return the direct Goto target of `name`, if any.
fn direct_goto(cfg: &CfgFunction, name: &str) -> Option<String> {
    let blk = cfg.blocks.get(name)?;
    match &blk.terminator {
        Some(Terminator::Goto { target, .. }) => Some(target.clone()),
        _ => None,
    }
}
