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

//! Phi-node shimmer detection (S101).
//!
//! When control flow merges two differently-typed versions of a variable,
//! the phi node that selects between them gets a `Shimmered` type-lattice
//! element.  At runtime Tcl will silently invalidate the intrep on the
//! first use after the merge — a hidden performance cost.
//!
//! Diagnostic code:
//! - **S101**: two paths that carry different intreps converge.
//!
//! Each warning includes the definition spans of the incoming versions as
//! "related" notes so the user can see both assignment sites.

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;

use tcl_registry::TclType;

use crate::cfg::{BlockId, Function as CfgFunction};
use crate::sccp::cfg_order;
use crate::ssa::{Phi, SsaFunction, Symbol, ValueKey, Version};
use crate::types::{TypeKind, TypeLattice};

use super::graph::loop_body_blocks;
use super::span::{def_range_map, phi_span};
use super::thunking::{destructure_foreach_blocks, empty_value_versions, per_loop_body_types};
use super::{ShimmerWarning, type_name};

/// Find phi-node shimmer warnings for a function.
///
/// Walks every phi node in every SCCP-executable block.  If the phi's
/// `TypeLattice` is `Shimmered`, the variable will require an intrep
/// conversion on the first use after the merge point.
/// Shared, read-only context threaded through the per-phi classifier.
struct PhiCtx<'a> {
    cfg: &'a CfgFunction,
    ssa: &'a SsaFunction,
    types: &'a HashMap<ValueKey, TypeLattice>,
    empty_by_name: &'a HashMap<Symbol, HashSet<Version>>,
    loop_blocks: &'a HashSet<String>,
    loop_body_types: &'a HashMap<Symbol, HashSet<TclType>>,
    def_map: &'a HashMap<ValueKey, tcl_lexer::Span>,
}

/// Decide whether one phi node is a genuine merge-point shimmer, returning
/// the warning when it is (the per-phi body of [`find_phi_shimmers`]).
fn classify_phi_shimmer(ctx: &PhiCtx<'_>, phi: &Phi, in_loop: bool) -> Option<ShimmerWarning> {
    let lattice = ctx.types.get(&(phi.name, phi.version))?;
    if lattice.kind != TypeKind::Shimmered {
        return None;
    }
    let from = lattice.from_type?;
    let to = lattice.tcl_type?;

    // Refine the SHIMMERED-phi verdict.
    // A loop-header phi is SHIMMERED on any entry-vs-body type change,
    // but the empty-accumulator promotion (`set r {}`; `lappend r …`)
    // and a branch merge whose only shimmer comes from an empty literal
    // are not real per-use intrep conversions.
    let empty_vers = ctx.empty_by_name.get(&phi.name);
    let mut entry_types: HashSet<TclType> = HashSet::new();
    let mut body_types: HashSet<TclType> = HashSet::new();
    let mut nonempty_known: HashSet<TclType> = HashSet::new();
    let mut has_shimmered_incoming = false;
    for (pred, &inc_ver) in &phi.incoming {
        if inc_ver == 0 {
            continue;
        }
        let Some(inc_type) = ctx.types.get(&(phi.name, inc_ver)) else {
            continue;
        };
        if inc_type.kind == TypeKind::Unknown {
            continue;
        }
        let is_empty = empty_vers.is_some_and(|s| s.contains(&inc_ver));
        if inc_type.kind == TypeKind::Known && !is_empty {
            if let Some(t) = inc_type.tcl_type {
                if ctx.loop_blocks.contains(ctx.cfg.block_name(*pred)) {
                    body_types.insert(t);
                } else {
                    entry_types.insert(t);
                }
                nonempty_known.insert(t);
            }
        } else if inc_type.kind == TypeKind::Shimmered {
            has_shimmered_incoming = true;
        }
    }
    if in_loop {
        let mut all_body = body_types;
        if let Some(pl) = ctx.loop_body_types.get(&phi.name) {
            all_body.extend(pl.iter().copied());
        }
        let oscillates =
            entry_types.intersection(&all_body).next().is_some() || all_body.len() >= 2;
        if !oscillates {
            return None;
        }
    } else if !has_shimmered_incoming && nonempty_known.len() <= 1 {
        return None;
    }

    let span = phi_span(phi, ctx.ssa, ctx.def_map);
    let related: Vec<_> = phi
        .incoming
        .iter()
        .filter_map(|(pred_block, &ver)| {
            ctx.def_map.get(&(phi.name, ver)).copied().map(|sp| {
                (
                    sp,
                    format!("version from '{}'", ctx.cfg.block_name(*pred_block)),
                )
            })
        })
        .collect();

    // A merge inside a loop body re-shimmers every iteration (S101);
    // an out-of-loop branch merge is a one-time conversion (S100).
    let code = if in_loop {
        DiagCode::S101
    } else {
        DiagCode::S100
    };
    let var = ctx.ssa.var_name(phi.name);
    Some(ShimmerWarning {
        span,
        variable: var.to_owned(),
        from_type: from,
        to_type: to,
        command: "<phi>".to_owned(),
        in_loop,
        code,
        message: format!(
            "{code}: '{var}' merges {from} and {to} at control-flow join",
            from = type_name(from),
            to = type_name(to),
        ),
        related,
    })
}

#[must_use]
pub(crate) fn find_phi_shimmers(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<BlockId>,
) -> Vec<ShimmerWarning> {
    let loop_blocks = loop_body_blocks(cfg);
    let def_map = def_range_map(ssa);
    let empty_by_name = empty_value_versions(ssa);
    let destructure = destructure_foreach_blocks(cfg);
    // The phi pass uses the *function-wide* loop-body type map (unlike the
    // per-loop S102 thunking pass), built from the whole loop-block set.
    let loop_body_types =
        per_loop_body_types("", &loop_blocks, &destructure, ssa, types, &empty_by_name);
    let ctx = PhiCtx {
        cfg,
        ssa,
        types,
        empty_by_name: &empty_by_name,
        loop_blocks: &loop_blocks,
        loop_body_types: &loop_body_types,
        def_map: &def_map,
    };
    let mut out = Vec::new();

    for block_id in cfg_order(cfg) {
        if !executable_blocks.contains(&block_id) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&block_id) else {
            continue;
        };
        let in_loop = loop_blocks.contains(cfg.block_name(block_id));

        for phi in &ssa_block.phis {
            if let Some(warning) = classify_phi_shimmer(&ctx, phi, in_loop) {
                out.push(warning);
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// Two branches both assign Int literals — phi has uniform type, no shimmer.
    #[test]
    fn no_phi_shimmer_for_uniform_int_type() {
        let cu = CompilationUnit::build_for(
            "if {1} { set x 1 } else { set x 2 }\nincr x",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let warnings = find_phi_shimmers(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        // Both 1 and 2 are Int — no Shimmered phi expected.
        for w in &warnings {
            if w.command == "<phi>" {
                assert!(
                    !(w.from_type == tcl_registry::TclType::Int
                        && w.to_type == tcl_registry::TclType::Int),
                    "spurious Int/Int phi shimmer: {w:?}"
                );
            }
        }
    }

    /// The empty-accumulator promotion (`set r {}`; `foreach … {lappend r …}`)
    /// is a one-time STRING→LIST stabilisation, not a per-use shimmer — no S101.
    #[test]
    fn no_phi_shimmer_for_empty_accumulator() {
        let cu = CompilationUnit::build_for(
            "proc f {} { set r {}\n foreach x {1 2 3} { lappend r $x }\n return [llength $r] }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let warnings = find_phi_shimmers(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(
            !warnings.iter().any(|w| w.variable == "r"),
            "accumulator must not phi-shimmer: {warnings:?}"
        );
    }

    /// API smoke-test: function runs on empty source without panicking.
    #[test]
    fn phi_shimmers_empty_source() {
        let cu = CompilationUnit::build_for("", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let _ = find_phi_shimmers(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
    }

    /// An if/else that assigns Int on one branch and String on the other
    /// produces an S100 warning at the phi merge point — the conversion is
    /// one-time (the merge is not inside a loop), so the code is S100, not
    /// the per-iteration S101.
    #[test]
    fn phi_shimmer_emitted_for_int_string_merge() {
        // Use [gets stdin] for the condition so SCCP cannot fold the branch;
        // both arms remain executable, and the phi merges Int and String.
        let cu = CompilationUnit::build_for(
            "set cond [gets stdin]\nif {$cond} { set x 1 } else { set x \"hello\" }\nputs $x",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let warnings = find_phi_shimmers(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        let merge = warnings.iter().find(|w| w.variable == "x");
        assert!(
            merge.is_some(),
            "expected a phi shimmer for Int/String merge of 'x', got: {warnings:?}"
        );
        let merge = merge.unwrap();
        assert_eq!(merge.code, DiagCode::S100, "out-of-loop merge must be S100");
        assert!(!merge.in_loop);
    }
}
