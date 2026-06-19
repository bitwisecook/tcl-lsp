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

use tcl_registry::TclType;

use crate::cfg::Function as CfgFunction;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, ValueKey};
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
#[must_use]
#[allow(clippy::too_many_lines)]
pub(crate) fn find_phi_shimmers(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<String>,
) -> Vec<ShimmerWarning> {
    let loop_blocks = loop_body_blocks(cfg);
    let def_map = def_range_map(ssa);
    let empty_by_name = empty_value_versions(ssa);
    let destructure = destructure_foreach_blocks(cfg);
    // Python's phi pass uses the *function-wide* loop-body type map (unlike the
    // per-loop S102 thunking pass); reproduce that with the whole loop-block set.
    let loop_body_types =
        per_loop_body_types("", &loop_blocks, &destructure, ssa, types, &empty_by_name);
    let mut out = Vec::new();

    for block_name in cfg_order(cfg) {
        if !executable_blocks.contains(&block_name) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&block_name) else {
            continue;
        };
        let in_loop = loop_blocks.contains(&block_name);

        for phi in &ssa_block.phis {
            let key = (phi.name.clone(), phi.version);
            let Some(lattice) = types.get(&key) else {
                continue;
            };
            if lattice.kind != TypeKind::Shimmered {
                continue;
            }
            let Some(from) = lattice.from_type else {
                continue;
            };
            let Some(to) = lattice.tcl_type else {
                continue;
            };

            // Refine the SHIMMERED-phi verdict (mirrors `_find_phi_shimmers`).
            // A loop-header phi is SHIMMERED on any entry-vs-body type change,
            // but the empty-accumulator promotion (`set r {}`; `lappend r …`)
            // and a branch merge whose only shimmer comes from an empty literal
            // are not real per-use intrep conversions.
            let empty_vers = empty_by_name.get(&phi.name);
            let mut entry_types: HashSet<TclType> = HashSet::new();
            let mut body_types: HashSet<TclType> = HashSet::new();
            let mut nonempty_known: HashSet<TclType> = HashSet::new();
            let mut has_shimmered_incoming = false;
            for (pred, &inc_ver) in &phi.incoming {
                if inc_ver == 0 {
                    continue;
                }
                let Some(inc_type) = types.get(&(phi.name.clone(), inc_ver)) else {
                    continue;
                };
                if inc_type.kind == TypeKind::Unknown {
                    continue;
                }
                let is_empty = empty_vers.is_some_and(|s| s.contains(&inc_ver));
                if inc_type.kind == TypeKind::Known && !is_empty {
                    if let Some(t) = inc_type.tcl_type {
                        if loop_blocks.contains(pred) {
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
                if let Some(pl) = loop_body_types.get(&phi.name) {
                    all_body.extend(pl.iter().copied());
                }
                let oscillates =
                    entry_types.intersection(&all_body).next().is_some() || all_body.len() >= 2;
                if !oscillates {
                    continue;
                }
            } else if !has_shimmered_incoming && nonempty_known.len() <= 1 {
                continue;
            }

            let span = phi_span(phi, ssa, &def_map);
            let related: Vec<_> = phi
                .incoming
                .iter()
                .filter_map(|(pred_block, &ver)| {
                    def_map
                        .get(&(phi.name.clone(), ver))
                        .copied()
                        .map(|sp| (sp, format!("version from '{pred_block}'")))
                })
                .collect();

            out.push(ShimmerWarning {
                span,
                variable: phi.name.clone(),
                from_type: from,
                to_type: to,
                command: "<phi>".to_owned(),
                in_loop,
                code: "S101".to_owned(),
                message: format!(
                    "S101: '{var}' merges {from} and {to} at control-flow join",
                    var = phi.name,
                    from = type_name(from),
                    to = type_name(to),
                ),
                related,
            });
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
    /// produces an S101 warning at the phi merge point.
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
        let has_shimmer = warnings
            .iter()
            .any(|w| w.variable == "x" && w.code == "S101");
        assert!(
            has_shimmer,
            "expected S101 phi shimmer for Int/String merge of 'x', got: {warnings:?}"
        );
    }
}
