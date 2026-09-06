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

//! Gather every Tcl-local name mentioned by an SSA function.
//!
//! Seeds [`super::state::CfgState::known_names`] before the
//! per-block walk runs.

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
            names.insert(ssa.var_name(phi.name).to_owned());
        }
        for stmt in &block.statements {
            for &sym in stmt.defs.keys() {
                names.insert(ssa.var_name(sym).to_owned());
            }
            for &sym in stmt.uses.keys() {
                names.insert(ssa.var_name(sym).to_owned());
            }
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg_builder::{
        build_cfg_function_with_upvars_and_config, prepare_cfg_context_with_registry,
    };
    use crate::lowering::lower_to_ir;
    use crate::ssa::build_ssa;
    use tcl_registry::CommandRegistry;

    fn ssa_of(src: &str) -> SsaFunction {
        let registry = CommandRegistry::build_default();
        let m = lower_to_ir(src, &registry);
        let context = prepare_cfg_context_with_registry(&m, &registry);
        let cfg = build_cfg_function_with_upvars_and_config(
            "::top",
            &m.top_level,
            true,
            &registry,
            m.plain_command_dispatch,
            context,
            tcl_lexer::LexerConfig::default(),
        );
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
