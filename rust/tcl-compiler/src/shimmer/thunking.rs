//! Thunking (loop-oscillation) detection (S102).
//!
//! A "thunk" occurs when a variable inside a loop alternates between two
//! intrep types on successive iterations, forcing a type conversion on
//! every pass through the loop.
//!
//! Detection strategy:
//!
//! 1. Find **loop headers** — blocks that are:
//!    - inside a cycle (`loop_body_blocks`), **and**
//!    - have at least one predecessor from *outside* the loop (entry edge), **and**
//!    - have at least one predecessor from *inside* the loop (back edge).
//!
//! 2. For each phi node at a loop header whose `TypeLattice` is
//!    `Shimmered`, the variable merges two different types at every
//!    iteration.  That is the thunking pattern.
//!
//! Diagnostic code:
//! - **S102**: variable oscillates between two intrep types across loop
//!   iterations.

#![allow(clippy::implicit_hasher)]

use std::collections::{HashMap, HashSet};

use crate::cfg::Function as CfgFunction;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, ValueKey};
use crate::types::{TypeKind, TypeLattice};

use super::graph::{build_successors, loop_body_blocks};
use super::span::{def_range_map, phi_span};
use super::{type_name, ThunkingWarning};

/// Find thunking warnings for a function.
///
/// Returns one [`ThunkingWarning`] per phi node at a loop header whose
/// type lattice is `Shimmered`.
#[must_use]
pub fn find_thunking_warnings(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<String>,
) -> Vec<ThunkingWarning> {
    let loop_blocks = loop_body_blocks(cfg);
    let succs = build_successors(cfg);
    let def_map = def_range_map(ssa);
    let mut out = Vec::new();

    // A loop header is a block that is on a cycle and has both an
    // entry edge (predecessor outside the loop) and a back edge
    // (predecessor inside the loop, other than self-loops).
    let loop_headers: HashSet<String> = cfg
        .blocks
        .keys()
        .filter(|bn| {
            if !loop_blocks.contains(*bn) {
                return false;
            }
            let has_entry = succs.iter().any(|(pred, targets)| {
                targets.iter().any(|t| t == *bn) && !loop_blocks.contains(pred)
            });
            let has_back = succs.iter().any(|(pred, targets)| {
                targets.iter().any(|t| t == *bn) && loop_blocks.contains(pred) && pred != *bn
            });
            has_entry && has_back
        })
        .cloned()
        .collect();

    for block_name in cfg_order(cfg) {
        if !executable_blocks.contains(&block_name) {
            continue;
        }
        if !loop_headers.contains(&block_name) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&block_name) else {
            continue;
        };

        for phi in &ssa_block.phis {
            let key = (phi.name.clone(), phi.version);
            let Some(lattice) = types.get(&key) else {
                continue;
            };
            if lattice.kind != TypeKind::Shimmered {
                continue;
            }
            let Some(type_a) = lattice.from_type else {
                continue;
            };
            let Some(type_b) = lattice.tcl_type else {
                continue;
            };

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

            out.push(ThunkingWarning {
                span,
                variable: phi.name.clone(),
                type_a,
                type_b,
                code: "S102".to_owned(),
                message: format!(
                    "S102: '{var}' oscillates between {a} and {b} across \
                     loop iterations (thunking)",
                    var = phi.name,
                    a = type_name(type_a),
                    b = type_name(type_b),
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

    /// A loop variable that is always Int — no thunking.
    #[test]
    fn no_thunking_for_uniform_int_loop_variable() {
        let cu = CompilationUnit::build_for(
            "for {set i 0} {$i < 10} {incr i} { set x 1 }",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(w.is_empty(), "unexpected thunking warnings: {w:?}");
    }

    /// Empty source: no thunking warnings and no panic.
    #[test]
    fn thunking_empty_source() {
        let cu = CompilationUnit::build_for("", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(w.is_empty());
    }

    /// A while loop with no variables — no thunking.
    #[test]
    fn no_thunking_for_no_loop_variables() {
        let cu = CompilationUnit::build_for("while {1} { puts \"hello\" }", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(w.is_empty(), "unexpected thunking: {w:?}");
    }

    /// A loop variable initialised as Int before the loop and reassigned as
    /// String inside produces an S102 thunking warning: the loop-header phi
    /// merges Int (from the entry edge) and String (from the back edge).
    #[test]
    fn thunking_detected_for_int_string_oscillation() {
        // Use a non-constant while condition so SCCP does not eliminate
        // the loop body, keeping both the entry and back edges executable.
        let cu = CompilationUnit::build_for(
            "set x 0\nwhile {[gets stdin] ne \"\"} {\n    set x \"hello\"\n}",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = find_thunking_warnings(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        let has_thunking = w.iter().any(|tw| tw.variable == "x" && tw.code == "S102");
        assert!(
            has_thunking,
            "expected S102 thunking for 'x' oscillating Int/String, got: {w:?}"
        );
    }
}
