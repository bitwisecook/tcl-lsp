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

//! `find_partial_redundancies`, the availability transfer and the dominance queries it uses.

use super::*;

// Partial-redundancy detection.

#[test]
fn find_partial_redundancies_if_diamond() {
    // Diamond:
    //   entry → tt (computes llength $x) → join
    //         → ff → join (computes llength $x again)
    // `llength $x` is may-available at join (came via tt), but
    // not must-available (ff didn't compute it). On the join
    // occurrence, report O106 — consider hoisting before the
    // branch.
    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    let tt = cfg.intern_block("tt");
    let ff = cfg.intern_block("ff");
    let join = cfg.intern_block("join");
    cfg.blocks.insert(tt, Block::new("tt"));
    cfg.blocks.insert(ff, Block::new("ff"));
    cfg.blocks.insert(join, Block::new("join"));
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Branch {
        condition: crate::expr_ast::ExprNode::Var {
            text: "$c".into(),
            name: "c".into(),
            start: 0,
            end: 2,
        },
        true_target: tt,
        false_target: ff,
        span: None,
        condition_base: None,
    });
    cfg.blocks
        .get_mut(&tt)
        .unwrap()
        .statements
        .push(llength_call_at(100, 110));
    cfg.blocks.get_mut(&tt).unwrap().terminator = Some(Terminator::Goto {
        target: join,
        span: None,
    });
    cfg.blocks.get_mut(&ff).unwrap().terminator = Some(Terminator::Goto {
        target: join,
        span: None,
    });
    cfg.blocks
        .get_mut(&join)
        .unwrap()
        .statements
        .push(llength_call_at(200, 210));
    cfg.blocks.get_mut(&join).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.idom.insert(entry, None);
    ssa.idom.insert(tt, Some(entry));
    ssa.idom.insert(ff, Some(entry));
    ssa.idom.insert(join, Some(entry));

    let entry_b = empty_ssa_block("entry");
    let mut tt_b = empty_ssa_block("tt");
    tt_b.statements
        .push(ssa_stmt_for(&mut ssa, llength_call_at(100, 110), Some(1)));
    let ff_b = empty_ssa_block("ff");
    let mut join_b = empty_ssa_block("join");
    join_b
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call_at(200, 210), Some(1)));
    ssa.blocks.insert(entry, entry_b);
    ssa.blocks.insert(tt, tt_b);
    ssa.blocks.insert(ff, ff_b);
    ssa.blocks.insert(join, join_b);

    let results = find_partial_redundancies(&registry, &cfg, &ssa, &all_blocks(&cfg), None);
    assert_eq!(results.len(), 1);
    // Canonical: partial redundancy (GVN-PRE) shares O105 with
    // full redundancy — the loop-invariant code O106 was wrong
    // here.
    assert_eq!(results[0].code, DiagCode::O105);
    assert_eq!(results[0].span.start(), 200);
    assert_eq!(results[0].first_span.start(), 100);
}

#[test]
fn find_partial_redundancies_none_on_linear_chain() {
    let registry = CommandRegistry::build_default();
    // Single-block function with one llength — no partial
    // redundancy possible.
    let mut cfg = Function::new("::top", "entry");
    cfg.blocks
        .get_mut(&cfg.entry)
        .unwrap()
        .statements
        .push(llength_call());
    cfg.blocks.get_mut(&cfg.entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.idom.insert(cfg.entry, None);
    let mut entry_b = empty_ssa_block("entry");
    entry_b
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa.blocks.insert(cfg.entry, entry_b);

    assert_eq!(
        find_partial_redundancies(&registry, &cfg, &ssa, &all_blocks(&cfg), None).len(),
        0
    );
}

#[test]
fn transfer_occurrence_keys_kill_clears_state() {
    let key: ExprKey = vec!["call".into(), "llength".into(), "$x@1".into()];
    let occ = ExprOccurrence {
        key: key.clone(),
        span: Span::new(0, 0),
        expression_text: String::new(),
        block: String::new(),
        statement_index: 0,
        variable_uses: Vec::new(),
    };
    let events = vec![OccurrenceEvent::Occur(occ), OccurrenceEvent::Kill];
    let out = transfer_occurrence_keys(&events, FxHashSet::default());
    assert!(!out.contains(&key));
}

#[test]
fn dominates_trivial_and_chain() {
    // Intern the chain through a CFG so the SSA shares its BlockIds.
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    let a = cfg.intern_block("a");
    let b = cfg.intern_block("b");
    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.idom.insert(entry, None);
    ssa.idom.insert(a, Some(entry));
    ssa.idom.insert(b, Some(a));
    let dom = ssa.dominator_intervals();
    assert!(dom.dominates(entry, b));
    assert!(dom.dominates(a, b));
    assert!(!dom.dominates(b, a));
    assert!(dom.dominates(b, b));
}

#[test]
fn dominator_intervals_answer_siblings_and_unreachable() {
    // The interval index must give the same answers an idom chain walk
    // does — including "no", which is the case an interval scheme can get
    // wrong if the numbering is off by one.
    //
    //   entry → a → { b, c },  d unreachable (no idom entry)
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    let a = cfg.intern_block("a");
    let b = cfg.intern_block("b");
    let c = cfg.intern_block("c");
    let d = cfg.intern_block("d");
    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.idom.insert(entry, None);
    ssa.idom.insert(a, Some(entry));
    ssa.idom.insert(b, Some(a));
    ssa.idom.insert(c, Some(a));
    let dom = ssa.dominator_intervals();
    for node in [a, b, c] {
        assert!(dom.dominates(entry, node), "entry must dominate {node:?}");
        assert!(dom.dominates(node, node), "{node:?} dominates itself");
    }
    assert!(dom.dominates(a, b) && dom.dominates(a, c));
    assert!(!dom.dominates(b, c) && !dom.dominates(c, b));
    assert!(!dom.dominates(b, a) && !dom.dominates(c, entry));
    // A block outside the dominator forest dominates only itself.
    assert!(dom.dominates(d, d));
    assert!(!dom.dominates(entry, d) && !dom.dominates(d, entry));
}

#[test]
fn expr_occurrence_carries_variable_uses() {
    let occ = ExprOccurrence {
        key: vec!["call".into(), "llength".into(), "$x@1".into()],
        span: Span::new(0, 5),
        expression_text: "llength $x".into(),
        block: "entry".into(),
        statement_index: 0,
        variable_uses: vec!["x".into()],
    };
    assert_eq!(occ.variable_uses, vec!["x".to_string()]);
}
