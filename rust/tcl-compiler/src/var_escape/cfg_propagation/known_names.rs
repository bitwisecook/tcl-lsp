//! Gather every Tcl-local name mentioned by an SSA function (C33e2).
//!
//! Mirrors `_collect_known_names_from_cfg` from
//! `core/compiler/var_escape/_cfg_propagation.py`. Seeds
//! [`super::state::CfgState::known_names`] before the per-block
//! walk runs.

use std::collections::HashSet;

use crate::ssa::SsaFunction;

/// Collect every variable name that has at least one SSA
/// definition, use, or phi appearance across *ssa*'s blocks,
/// plus every name in *params*.
#[must_use]
pub fn collect_known_names_from_cfg<I: IntoIterator<Item = String>>(
    params: I,
    ssa: &SsaFunction,
) -> HashSet<String> {
    let mut names: HashSet<String> = params.into_iter().collect();
    for block in ssa.blocks.values() {
        for phi in &block.phis {
            names.insert(phi.name.clone());
        }
        for stmt in &block.statements {
            for n in stmt.defs.keys() {
                names.insert(n.clone());
            }
            for n in stmt.uses.keys() {
                names.insert(n.clone());
            }
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg_builder::build_cfg_function;
    use crate::lowering::lower_to_ir;
    use crate::ssa::build_ssa;
    use tcl_registry::CommandRegistry;

    fn ssa_of(src: &str) -> SsaFunction {
        let registry = CommandRegistry::build_default();
        let m = lower_to_ir(src, &registry);
        let cfg = build_cfg_function("::top", &m.top_level, true);
        build_ssa(&cfg, &registry)
    }

    #[test]
    fn collects_names_from_assigns() {
        let s = ssa_of("set foo 1\nset bar 2");
        let names = collect_known_names_from_cfg(std::iter::empty::<String>(), &s);
        assert!(names.contains("foo"));
        assert!(names.contains("bar"));
    }

    #[test]
    fn includes_seeded_params() {
        let s = ssa_of("");
        let names = collect_known_names_from_cfg(["a".to_string(), "b".to_string()], &s);
        assert!(names.contains("a"));
        assert!(names.contains("b"));
    }

    #[test]
    fn captures_branch_uses() {
        // ``if {$x > 0}`` uses ``x``; ``set y 1`` defines ``y``.
        let s = ssa_of("set x 1\nif {$x > 0} { set y 1 }");
        let names = collect_known_names_from_cfg(std::iter::empty::<String>(), &s);
        assert!(names.contains("x"));
        assert!(names.contains("y"));
    }

    #[test]
    fn includes_phi_names_after_merge() {
        // ``$x`` with two writers across the merge block produces a
        // phi at the join — its name should appear in known_names.
        let s = ssa_of("if {1} { set x 1 } else { set x 2 }\nputs $x");
        let names = collect_known_names_from_cfg(std::iter::empty::<String>(), &s);
        assert!(names.contains("x"));
    }

    #[test]
    fn empty_function_only_includes_params() {
        let s = ssa_of("");
        let names = collect_known_names_from_cfg(["only".to_string()], &s);
        assert_eq!(names.len(), 1);
        assert!(names.contains("only"));
    }
}
