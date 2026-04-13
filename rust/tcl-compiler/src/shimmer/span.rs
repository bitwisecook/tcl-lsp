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

#![allow(clippy::implicit_hasher)]

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
            for (name, &ver) in &ss.defs {
                out.insert((name.clone(), ver), sp);
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
pub fn phi_span(phi: &Phi, ssa: &SsaFunction, def_map: &HashMap<ValueKey, Span>) -> Span {
    // Try each incoming version.
    for &ver in phi.incoming.values() {
        if let Some(&sp) = def_map.get(&(phi.name.clone(), ver)) {
            return sp;
        }
    }

    // Fallback: first statement in any block.
    for block in ssa.blocks.values() {
        if let Some(ss) = block.statements.first() {
            return ss.statement.span();
        }
    }

    Span::new(0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Statement;
    use crate::ssa::{Phi, SsaBlock, SsaFunction, SsaStatement};
    use std::collections::HashMap;
    use tcl_lexer::Span;

    fn make_ssa_with_def(name: &str, ver: u32, span: Span) -> SsaFunction {
        let mut blocks = HashMap::new();
        let stmt = Statement::AssignConst {
            span,
            name: name.to_owned(),
            value: "1".to_owned(),
        };
        let ss = SsaStatement {
            statement: stmt,
            uses: HashMap::new(),
            defs: {
                let mut d = HashMap::new();
                d.insert(name.to_owned(), ver);
                d
            },
        };
        let block = SsaBlock {
            name: "entry".to_owned(),
            phis: Vec::new(),
            statements: vec![ss],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        };
        blocks.insert("entry".to_owned(), block);
        SsaFunction {
            name: "::top".to_owned(),
            entry: "entry".to_owned(),
            blocks,
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        }
    }

    #[test]
    fn def_range_map_finds_statement_span() {
        let ssa = make_ssa_with_def("x", 1, Span::new(10, 20));
        let map = def_range_map(&ssa);
        assert_eq!(
            map.get(&("x".to_owned(), 1)),
            Some(&Span::new(10, 20))
        );
    }

    #[test]
    fn def_range_map_misses_unknown_key() {
        let ssa = make_ssa_with_def("x", 1, Span::new(10, 20));
        let map = def_range_map(&ssa);
        assert!(map.get(&("y".to_owned(), 1)).is_none());
        assert!(map.get(&("x".to_owned(), 2)).is_none());
    }

    #[test]
    fn phi_span_uses_incoming_version() {
        let ssa = make_ssa_with_def("x", 1, Span::new(10, 20));
        let def_map = def_range_map(&ssa);
        let phi = Phi {
            name: "x".to_owned(),
            version: 2,
            incoming: {
                let mut m = HashMap::new();
                m.insert("entry".to_owned(), 1u32);
                m
            },
        };
        assert_eq!(phi_span(&phi, &ssa, &def_map), Span::new(10, 20));
    }

    #[test]
    fn phi_span_falls_back_to_first_statement() {
        let ssa = make_ssa_with_def("x", 1, Span::new(5, 15));
        let empty_map: HashMap<ValueKey, Span> = HashMap::new();
        // phi for an unrelated variable — no incoming matches def_map.
        let phi = Phi {
            name: "z".to_owned(),
            version: 1,
            incoming: HashMap::new(),
        };
        // Should find the first statement's span as fallback.
        let sp = phi_span(&phi, &ssa, &empty_map);
        assert_eq!(sp, Span::new(5, 15));
    }

    #[test]
    fn phi_span_returns_zero_when_empty_ssa() {
        let empty_ssa = SsaFunction {
            name: "::top".to_owned(),
            entry: "entry".to_owned(),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        };
        let phi = Phi {
            name: "x".to_owned(),
            version: 1,
            incoming: HashMap::new(),
        };
        let sp = phi_span(&phi, &empty_ssa, &HashMap::new());
        assert_eq!(sp, Span::new(0, 0));
    }
}
