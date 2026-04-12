//! Per-block handlers for loop-related CFG patterns.
//!
//! foreach opcode compilation, loop-end result handling, for-init /
//! while startCommand wrapping. Each handler corresponds to one
//! `if bname.startswith(...)` branch in Python's `generate()`.

#![allow(dead_code, unused_imports)]

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

// TODO (follow-up chunk): handlers for foreach opcode emission,
// loop-end result push, while/for startCommand wrapping, etc.
// The MVP emitter in generate.rs does not need these — CFG-lowered
// loops currently emit generic invokeStk calls for foreach/while/for.
