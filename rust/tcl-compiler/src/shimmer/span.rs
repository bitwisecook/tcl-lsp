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

//! Source-span resolution for SSA values.
//!
//! Type-lattice results are keyed by `(name, version)` SSA values.
//! To emit diagnostics we need to recover the source span where each
//! value was defined.
//!
//! - [`def_range_map`] — build a `ValueKey → Span` index from SSA
//!   statement defs.
//! - [`phi_span`] — recover the best available span for a synthetic
//!   phi node that has no direct source location.

use std::collections::HashMap;

use tcl_lexer::Span;

use crate::ssa::{Phi, SsaFunction, ValueKey};

/// Build a `ValueKey → Span` map from the SSA statements.
///
/// Each `SsaStatement` knows which `(name, version)` it defines via
/// its `defs` map.  We extract the span from the underlying IR
/// statement and record it.  Phi nodes are handled separately by
/// [`phi_span`].
#[must_use]
pub fn def_range_map(ssa: &SsaFunction) -> HashMap<ValueKey, Span> {
    let mut out: HashMap<ValueKey, Span> = HashMap::new();
    for block in ssa.blocks.values() {
        for ss in &block.statements {
            let sp = ss.statement.span();
            for (&sym, &ver) in &ss.defs {
                out.insert((sym, ver), sp);
            }
        }
    }
    out
}

/// Return the best available `Span` for a phi node.
///
/// Phi nodes are synthetic — they don't correspond to a single source
/// statement.  We approximate their location in this priority order:
///
/// 1. Any incoming version that appears in `def_map`.
/// 2. The first statement of any block in the SSA function.
/// 3. A zero span `{ start: 0, end: 0 }`.
#[must_use]
pub(crate) fn phi_span(phi: &Phi, ssa: &SsaFunction, def_map: &HashMap<ValueKey, Span>) -> Span {
    // `phi.incoming` is a `HashMap`, so a "first match in iteration order" pick
    // would make the warning's anchor vary run-to-run (and between the offset-0
    // memo build and the whole-module build — the latent nondeterminism the
    // `compiler_check_corpus` guard catches).  Choose deterministically: the
    // earliest (smallest) incoming def span.
    let earliest = phi
        .incoming
        .values()
        .filter_map(|&ver| def_map.get(&(phi.name, ver)).copied())
        .min_by_key(|sp| (sp.start(), sp.end()));
    if let Some(sp) = earliest {
        return sp;
    }

    // Fallback: the earliest first-statement span across blocks (`ssa.blocks` is
    // also a `HashMap`, so likewise pick deterministically).
    ssa.blocks
        .values()
        .filter_map(|block| block.statements.first().map(|ss| ss.statement.span()))
        .min_by_key(|sp| (sp.start(), sp.end()))
        .unwrap_or_else(|| Span::new(0, 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::BlockId;
    use crate::ir::Statement;
    use crate::ssa::{Phi, SsaBlock, SsaFunction, SsaStatement};
    use std::collections::HashMap;
    use tcl_lexer::Span;

    /// Single-block ("entry") SSA shell with one `set` def. The interner has
    /// just `entry`, so its [`BlockId`] is `BlockId(0)`.
    fn make_ssa_with_def(name: &str, ver: u32, span: Span) -> SsaFunction {
        let entry = BlockId(0);
        let mut ssa = SsaFunction::trivial("::top", entry, vec!["entry".to_owned()]);
        let sym = ssa.intern_var(name);
        let stmt = Statement::AssignConst {
            span,
            name: name.to_owned(),
            value: "1".to_owned(),
            name_braced: false,
        };
        let ss = SsaStatement {
            statement: stmt,
            uses: HashMap::new(),
            defs: {
                let mut d = HashMap::new();
                d.insert(sym, ver);
                d
            },
            may_defs: std::collections::HashSet::new(),
        };
        let block = SsaBlock {
            name: "entry".to_owned(),
            phis: Vec::new(),
            statements: vec![ss],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        };
        ssa.blocks.insert(entry, block);
        ssa
    }

    #[test]
    fn def_range_map_finds_statement_span() {
        let ssa = make_ssa_with_def("x", 1, Span::new(10, 20));
        let x = ssa.var_symbol("x").unwrap();
        let map = def_range_map(&ssa);
        assert_eq!(map.get(&(x, 1)), Some(&Span::new(10, 20)));
    }

    #[test]
    fn def_range_map_misses_unknown_key() {
        let mut ssa = make_ssa_with_def("x", 1, Span::new(10, 20));
        let x = ssa.var_symbol("x").unwrap();
        let y = ssa.intern_var("y");
        let map = def_range_map(&ssa);
        assert!(!map.contains_key(&(y, 1)));
        assert!(!map.contains_key(&(x, 2)));
    }

    #[test]
    fn phi_span_uses_incoming_version() {
        let ssa = make_ssa_with_def("x", 1, Span::new(10, 20));
        let x = ssa.var_symbol("x").unwrap();
        let def_map = def_range_map(&ssa);
        let phi = Phi {
            name: x,
            version: 2,
            incoming: {
                let mut m = HashMap::new();
                m.insert(BlockId(0), 1u32);
                m
            },
        };
        assert_eq!(phi_span(&phi, &ssa, &def_map), Span::new(10, 20));
    }

    #[test]
    fn phi_span_falls_back_to_first_statement() {
        let mut ssa = make_ssa_with_def("x", 1, Span::new(5, 15));
        let z = ssa.intern_var("z");
        let empty_map: HashMap<ValueKey, Span> = HashMap::new();
        // phi for an unrelated variable — no incoming matches def_map.
        let phi = Phi {
            name: z,
            version: 1,
            incoming: HashMap::new(),
        };
        // Should find the first statement's span as fallback.
        let sp = phi_span(&phi, &ssa, &empty_map);
        assert_eq!(sp, Span::new(5, 15));
    }

    #[test]
    fn phi_span_returns_zero_when_empty_ssa() {
        let mut empty_ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".to_owned()]);
        let x = empty_ssa.intern_var("x");
        let phi = Phi {
            name: x,
            version: 1,
            incoming: HashMap::new(),
        };
        let sp = phi_span(&phi, &empty_ssa, &HashMap::new());
        assert_eq!(sp, Span::new(0, 0));
    }
}
