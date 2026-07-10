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

//! Per-block handlers for loop-related CFG patterns.
//!
//! foreach opcode compilation, loop-end result handling, for-init /
//! while startCommand wrapping. Each handler corresponds to one
//! branch of the block-name prefix check in the emitter.

#![allow(dead_code, clippy::implicit_hasher, clippy::doc_markdown)]

use std::collections::{HashMap, HashSet};

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::ir::Statement;

/// Metadata about a foreach loop detected in the CFG.
#[derive(Debug, Clone)]
pub struct ForeachInfo {
    /// Body block name.
    pub body: String,
    /// End block name.
    pub end: String,
    /// List arguments to the foreach/lmap command.
    pub list_args: Vec<String>,
    /// Loop-variable groups (one per iterator), reconstructed from the
    /// header `Call`'s `defs` + `foreach_groups`. Carried onto the
    /// `FOREACH_START` instruction so the VM can bind them (C Tcl `ForeachInfo`).
    pub var_groups: Vec<Vec<String>>,
    /// This is an `lmap` (a collecting loop): the codegen strips the body's
    /// trailing `POP` and emits `LMAP_COLLECT` so each iteration's result is
    /// gathered VM-side, and `FOREACH_END` yields `list(accum)`.
    pub collect: bool,
}

/// Metadata about a complex foreach: one whose body block terminates with a
/// Branch (the body's first/only control structure is an `if`/loop whose
/// condition becomes the body terminator). The body may carry statements — e.g.
/// the `<cond>` placeholder of an inline command substitution in the condition.
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
    for (id, blk) in &cfg.blocks {
        let bn = cfg.block_name(*id);
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
            if let Statement::Call {
                command,
                args,
                defs,
                foreach_groups,
                ..
            } = stmt
                && (command == "foreach" || command == "lmap")
            {
                // Split the flattened `defs` into per-iterator groups.
                let sizes = foreach_groups.clone().unwrap_or_else(|| vec![defs.len()]);
                let mut var_groups = Vec::with_capacity(sizes.len());
                let mut i = 0;
                for sz in sizes {
                    let end = (i + sz).min(defs.len());
                    var_groups.push(defs[i..end].to_vec());
                    i = end;
                }
                info.insert(
                    bn.to_owned(),
                    ForeachInfo {
                        body: cfg.block_name(*true_target).to_owned(),
                        end: cfg.block_name(*false_target).to_owned(),
                        list_args: args.clone(),
                        var_groups,
                        collect: command == "lmap",
                    },
                );
                break;
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
        let Some(body_blk) = cfg.block_by_name(&info.body) else {
            continue;
        };
        // A *simple* foreach is a single straight-line body block that loops
        // directly back to the header (`Goto header`); its `FOREACH_STEP`/
        // `FOREACH_END` are emitted inline after the body. Anything else is
        // complex — the step/end move to the foreach_end block and body
        // back-edges route to the step label. This covers both a branching body
        // (an `if`, terminator `Branch`) *and* a body that flows through further
        // blocks before looping (a nested `for`/`while`/`foreach`, whose tail
        // `Goto`s the header from a *different* block — keying only on `Branch`
        // here mis-classified that as simple and emitted the outer step/end
        // before the inner loop ran, an infinite loop).
        let is_simple = matches!(
            &body_blk.terminator,
            Some(Terminator::Goto { target, .. }) if cfg.block_name(*target) == header
        );
        if is_simple {
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
