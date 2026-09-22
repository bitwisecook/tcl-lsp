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

/// Where a `catch` region begins and ends.
#[derive(Debug, Clone)]
pub struct CatchRegionInfo {
    /// `catch_end_N` block name — the continuation both paths reach.
    pub catch_end: String,
    /// `catch`'s result variable, if the source named one.
    pub result_var: Option<String>,
    /// `catch`'s options-dict variable, if the source named one.
    pub options_var: Option<String>,
}

/// Detect `catch` regions in the CFG.
///
/// The builder emits `catch_body_N → catch_end_N` ([`crate::cfg_builder`]'s
/// `lower_catch`). Unlike try/finally the end block is *not* consumed: it is
/// the continuation, carrying the result/options variable defs and whatever
/// follows the `catch`. Only the scaffolding is spliced in at the body block.
#[must_use]
pub fn detect_catch_regions(
    cfg: &CfgFunction,
    block_order: &[String],
) -> HashMap<String, CatchRegionInfo> {
    let mut result: HashMap<String, CatchRegionInfo> = HashMap::new();

    for bname in block_order {
        if !bname.starts_with("catch_body_") {
            continue;
        }
        let Some(catch_end) = follow_until_prefix(cfg, bname, "catch_end_") else {
            continue;
        };
        // `lower_catch` parks the result/options variables on a defs-only
        // `catch` call in the end block, so SSA sees them defined however the
        // body ended. Codegen stores them itself, from C's stack order.
        let (result_var, options_var) = catch_result_vars(cfg, &catch_end);
        result.insert(
            bname.clone(),
            CatchRegionInfo {
                catch_end,
                result_var,
                options_var,
            },
        );
    }

    result
}

/// The result and options variables recorded on a `catch_end` block's
/// defs-only statement, in that order.
///
/// Returns `(None, None)` when the source named neither.
fn catch_result_vars(cfg: &CfgFunction, catch_end: &str) -> (Option<String>, Option<String>) {
    let Some(blk) = cfg.block_by_name(catch_end) else {
        return (None, None);
    };
    for stmt in &blk.statements {
        if let crate::ir::Statement::Call {
            command,
            args,
            defs,
            ..
        } = stmt
            && command == "catch"
            && args.is_empty()
        {
            return (defs.first().cloned(), defs.get(1).cloned());
        }
    }
    (None, None)
}

/// Whether `stmt` is the defs-only marker `lower_catch` leaves on a
/// `catch_end` block. It exists for SSA, not for emission — the stores it
/// stands for are emitted with the catch scaffolding.
#[must_use]
pub fn is_catch_defs_marker(stmt: &crate::ir::Statement) -> bool {
    matches!(
        stmt,
        crate::ir::Statement::Call { command, args, .. }
            if command == "catch" && args.is_empty()
    )
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
        let blk = cfg.block_by_name(&current)?;
        let Some(Terminator::Goto { target, .. }) = &blk.terminator else {
            return None;
        };
        let target = cfg.block_name(*target);
        if target.starts_with(prefix) {
            return Some(target.to_owned());
        }
        target.clone_into(&mut current);
    }
}

/// Return the direct Goto target of `name`, if any.
fn direct_goto(cfg: &CfgFunction, name: &str) -> Option<String> {
    let blk = cfg.block_by_name(name)?;
    match &blk.terminator {
        Some(Terminator::Goto { target, .. }) => Some(cfg.block_name(*target).to_owned()),
        _ => None,
    }
}
