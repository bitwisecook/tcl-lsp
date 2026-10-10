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
    /// Original invocation proof used to select its compiler output protocol.
    pub tokens: Option<crate::ir::CommandTokens>,
}

/// Detect `catch` regions in the CFG.
///
/// The builder emits `catch_body_N → catch_end_N` ([`crate::cfg_builder`]'s
/// `lower_catch`). The explicit continuation also retains a body ending in
/// a caught error, which has no normal Goto edge. Unlike try/finally the end block is *not* consumed: it is
/// the continuation, carrying the result/options variable defs and whatever
/// follows the `catch`. Only the scaffolding is spliced in at the body block.
#[must_use]
pub fn detect_catch_regions(
    cfg: &CfgFunction,
    block_order: &[String],
    registry: &tcl_registry::CommandRegistry,
) -> HashMap<String, CatchRegionInfo> {
    let mut result: HashMap<String, CatchRegionInfo> = HashMap::new();

    for bname in block_order {
        if !bname.starts_with("catch_body_") {
            continue;
        }
        let Some(catch_end) = cfg
            .block_id(bname)
            .and_then(|entry| cfg.command_boundary_continuations.get(&entry))
            .map(|end| cfg.block_name(*end).to_owned())
            .or_else(|| follow_until_prefix(cfg, bname, "catch_end_"))
        else {
            continue;
        };
        // `lower_catch` parks the result/options variables on a defs-only
        // `catch` call in the end block, so SSA sees them defined however the
        // body ended. Codegen stores them itself, from C's stack order.
        let Some(info) = catch_region_outputs(cfg, &catch_end, registry) else {
            continue;
        };
        result.insert(bname.clone(), info);
    }

    result
}

/// Select original output argument positions independently of projected SSA defs.
fn catch_region_outputs(
    cfg: &CfgFunction,
    catch_end: &str,
    registry: &tcl_registry::CommandRegistry,
) -> Option<CatchRegionInfo> {
    let block = cfg.block_by_name(catch_end)?;
    let tokens = block.statements.iter().find_map(|statement| {
        is_catch_defs_marker(statement)
            .then(|| statement.tokens())
            .flatten()
    })?;
    let context = cfg.metadata_context.metadata_context(registry)?;
    let normal = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        registry, context, tokens,
    )?;
    normal.variable_output_arguments()?;
    let result = if normal.argument_count() > 1 {
        Some(normal.argument_literal(1)?)
    } else {
        None
    };
    let options = if normal.argument_count() > 2 {
        Some(normal.argument_literal(2)?)
    } else {
        None
    };
    let mut original = tokens.clone();
    original.synthetic = None;
    Some(CatchRegionInfo {
        catch_end: catch_end.to_owned(),
        result_var: result,
        options_var: options,
        tokens: Some(original),
    })
}

/// Whether this boundary stores captured catch outputs without invoking again.
#[must_use]
pub fn is_catch_defs_marker(statement: &crate::ir::Statement) -> bool {
    statement.tokens().is_some_and(|tokens| {
        tokens.synthetic == Some(crate::ir::SyntheticMarker::CapturedCatchOutputs)
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_outputs_retain_argument_positions_when_a_definition_is_withdrawn() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "proc p {} {catch {set x 1} result options; list $result $options}",
            registry,
            false,
        );
        let mut cfg = unit.function("::p").unwrap().cfg.clone();
        let marker = cfg
            .blocks
            .values_mut()
            .flat_map(|block| &mut block.statements)
            .find(|statement| is_catch_defs_marker(statement))
            .expect("captured output boundary");
        assert!(marker.source_edit_span().is_none());
        let tokens = marker.tokens().unwrap();
        assert!(!tokens.evaluates_words());
        assert_eq!(tokens.argv_texts.len(), 4);
        if let crate::ir::Statement::Call { defs, .. } = marker {
            *defs = vec!["options".to_owned()];
        }
        let order: Vec<_> = cfg
            .blocks
            .keys()
            .map(|&block| cfg.block_name(block).to_owned())
            .collect();
        let regions = detect_catch_regions(&cfg, &order, registry);
        assert_eq!(regions.len(), 1);
        let region = regions.values().next().unwrap();
        assert_eq!(region.result_var.as_deref(), Some("result"));
        assert_eq!(region.options_var.as_deref(), Some("options"));
        assert!(
            region
                .tokens
                .as_ref()
                .is_some_and(|tokens| tokens.synthetic.is_none())
        );
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        for owner in [
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable,
            crate::registry_invocation::OwnedInvocationMetadataContext::Supplied(foreign),
        ] {
            let mut withheld = cfg.clone();
            withheld.metadata_context = owner;
            assert!(detect_catch_regions(&withheld, &order, registry).is_empty());
        }
    }
}
