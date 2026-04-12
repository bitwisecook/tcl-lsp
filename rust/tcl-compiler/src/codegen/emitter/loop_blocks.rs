//! Per-block handlers for loop-related CFG patterns.
//!
//! foreach opcode compilation, loop-end result handling, for-init /
//! while startCommand wrapping. Each handler corresponds to one
//! `if bname.startswith(...)` branch in Python's `generate()`.

#![allow(
    dead_code,
    unused_imports,
    clippy::implicit_hasher,
    clippy::doc_markdown
)]

use std::collections::{HashMap, HashSet};

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::ir::Statement;

use super::super::{CodegenCtx, Op, Operand};

/// Metadata about a foreach loop detected in the CFG.
#[derive(Debug, Clone)]
pub struct ForeachInfo {
    /// Body block name.
    pub body: String,
    /// End block name.
    pub end: String,
    /// List arguments to the foreach/lmap command.
    pub list_args: Vec<String>,
}

/// Metadata about a complex foreach: one whose body is empty and
/// terminates with a Branch (the first statement is an `if` whose
/// condition becomes the body terminator).
///
/// The emitter must (a) emit `FOREACH_STEP`/`FOREACH_END` at the
/// foreach_end block rather than the body, (b) route continue/break
/// through synthetic step/break labels, and (c) suppress back-edge
/// gotos from body blocks to the foreach header.
#[derive(Debug, Clone)]
pub struct ComplexForeach {
    /// foreach_end_N block name.
    pub end: String,
    /// Label placed before `FOREACH_STEP` (continue target).
    pub step_label: String,
    /// Label placed between `FOREACH_STEP` and `FOREACH_END`
    /// (break target — past the step opcode).
    pub break_label: String,
    /// Body blocks reachable from the foreach body (excluding end).
    pub body_blocks: HashSet<String>,
}

/// Detect foreach loops by scanning `foreach_header_*` blocks.
#[must_use]
pub fn detect_foreach(cfg: &CfgFunction) -> HashMap<String, ForeachInfo> {
    let mut info: HashMap<String, ForeachInfo> = HashMap::new();
    for (bn, blk) in &cfg.blocks {
        if !bn.starts_with("foreach_header_") {
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
        for stmt in &blk.statements {
            if let Statement::Call { command, args, .. } = stmt {
                if command == "foreach" || command == "lmap" {
                    info.insert(
                        bn.clone(),
                        ForeachInfo {
                            body: true_target.clone(),
                            end: false_target.clone(),
                            list_args: args.clone(),
                        },
                    );
                    break;
                }
            }
        }
    }
    info
}

/// Detect complex foreach loops from the foreach info map.
///
/// A complex foreach has an empty body block whose terminator is a
/// `Branch` (not a `Goto`). Returns a map keyed by the foreach header
/// block name.
#[must_use]
pub fn detect_complex_foreach(
    cfg: &CfgFunction,
    foreach_info: &HashMap<String, ForeachInfo>,
    label_counter: &mut u32,
) -> HashMap<String, ComplexForeach> {
    let mut result: HashMap<String, ComplexForeach> = HashMap::new();
    for (header, info) in foreach_info {
        let Some(body_blk) = cfg.blocks.get(&info.body) else {
            continue;
        };
        if !body_blk.statements.is_empty() {
            continue;
        }
        if !matches!(body_blk.terminator, Some(Terminator::Branch { .. })) {
            continue;
        }
        let step_label = format!("foreach_continue_{}", *label_counter);
        *label_counter += 1;
        let break_label = format!("foreach_break_{}", *label_counter);
        *label_counter += 1;

        // Collect body blocks (reachable from body, excluding end).
        let mut body_blocks: HashSet<String> = HashSet::new();
        super::ordering::collect_loop_body(
            cfg,
            &info.body,
            header,
            &mut body_blocks,
            Some(&info.end),
        );

        result.insert(
            header.clone(),
            ComplexForeach {
                end: info.end.clone(),
                step_label,
                break_label,
                body_blocks,
            },
        );
    }
    result
}
