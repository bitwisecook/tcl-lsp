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

//! Dominators, immediate dominators, the dominance frontier and the dominator tree.

use super::*;

// Dominator tests

#[test]
fn dominators_linear() {
    // entry → b1 → b2 → return
    let mut func = Function::new("::test", "entry");
    let entry = func.entry;
    let b1 = block(&mut func, "b1");
    let b2 = block(&mut func, "b2");
    func.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(b1));
    func.blocks.get_mut(&b1).unwrap().terminator = Some(make_goto(b2));
    func.blocks.get_mut(&b2).unwrap().terminator = Some(make_return());

    let dom = compute_dominators(&func);
    assert_eq!(dom[&entry], HashSet::from([entry]));
    assert_eq!(dom[&b1], HashSet::from([entry, b1]));
    assert_eq!(dom[&b2], HashSet::from([entry, b1, b2]));
}

#[test]
fn dominators_diamond() {
    let func = diamond_cfg();
    let entry = func.entry;
    let (then, els, end) = (
        id_of(&func, "then"),
        id_of(&func, "else"),
        id_of(&func, "end"),
    );
    let dom = compute_dominators(&func);

    // entry dominates everything
    for id in func.blocks.keys() {
        assert!(dom[id].contains(&entry));
    }
    // then and else are not dominated by each other
    assert!(!dom[&then].contains(&els));
    assert!(!dom[&els].contains(&then));
    // end is dominated by entry but not by then or else
    assert!(dom[&end].contains(&entry));
    assert!(!dom[&end].contains(&then));
    assert!(!dom[&end].contains(&els));
}

#[test]
fn idom_diamond() {
    let func = diamond_cfg();
    let entry = func.entry;
    let (then, els, end) = (
        id_of(&func, "then"),
        id_of(&func, "else"),
        id_of(&func, "end"),
    );
    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);

    assert_eq!(idom[&entry], None);
    assert_eq!(idom[&then], Some(entry));
    assert_eq!(idom[&els], Some(entry));
    assert_eq!(idom[&end], Some(entry));
}

#[test]
fn idom_loop() {
    let func = loop_cfg();
    let entry = func.entry;
    let (header, body, end) = (
        id_of(&func, "header"),
        id_of(&func, "body"),
        id_of(&func, "end"),
    );
    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);

    assert_eq!(idom[&entry], None);
    assert_eq!(idom[&header], Some(entry));
    assert_eq!(idom[&body], Some(header));
    assert_eq!(idom[&end], Some(header));
}

#[test]
fn compute_idom_fast_matches_reference() {
    // The production CHK path (`compute_idom_fast`) must produce
    // exactly the same immediate dominators as the set-based
    // reference (`compute_idom` over `compute_dominators`) across
    // linear / diamond / loop shapes.
    let mut linear = Function::new("::test", "entry");
    let entry = linear.entry;
    let b1 = block(&mut linear, "b1");
    let b2 = block(&mut linear, "b2");
    linear.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(b1));
    linear.blocks.get_mut(&b1).unwrap().terminator = Some(make_goto(b2));
    linear.blocks.get_mut(&b2).unwrap().terminator = Some(make_return());

    for func in [linear, diamond_cfg(), loop_cfg()] {
        let reference = compute_idom(&func, &compute_dominators(&func));
        let fast = compute_idom_fast(&func);
        assert_eq!(fast, reference, "CHK idom diverged for {:?}", func.entry);
    }
}

#[test]
fn compute_idom_fast_handles_long_chain_without_blowup() {
    // Generated dispatch procedures form long diamond chains; dominator
    // computation must remain bounded on this shape.
    let mut func = Function::new("::big", "b0");
    let n = 4000;
    for i in 0..n {
        let cur = func.intern_block(format!("b{i}"));
        func.blocks
            .entry(cur)
            .or_insert_with(|| Block::new(format!("b{i}")));
        let then = block(&mut func, &format!("t{i}"));
        let next = block(&mut func, &format!("b{}", i + 1));
        func.blocks.get_mut(&then).unwrap().terminator = Some(make_return());
        func.blocks.get_mut(&cur).unwrap().terminator = Some(make_branch("c", then, next));
    }
    let bn = id_of(&func, &format!("b{n}"));
    func.blocks.get_mut(&bn).unwrap().terminator = Some(make_return());
    let idom = compute_idom_fast(&func);
    // Each chain block's idom is the previous chain block.
    assert_eq!(idom[&id_of(&func, "b1")], Some(id_of(&func, "b0")));
    assert_eq!(idom[&bn], Some(id_of(&func, &format!("b{}", n - 1))));
    assert_eq!(idom[&id_of(&func, "b0")], None);
}

#[test]
fn dominance_frontier_diamond() {
    let func = diamond_cfg();
    let entry = func.entry;
    let (then, els, end) = (
        id_of(&func, "then"),
        id_of(&func, "else"),
        id_of(&func, "end"),
    );
    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);
    let df = compute_dominance_frontier(&func, &idom);

    // then and else have "end" in their dominance frontier
    assert!(df[&then].contains(&end));
    assert!(df[&els].contains(&end));
    // entry has no dominance frontier
    assert_eq!(df[&entry].len(), 0);
}

#[test]
fn dominance_frontier_loop() {
    let func = loop_cfg();
    let entry = func.entry;
    let (header, body) = (id_of(&func, "header"), id_of(&func, "body"));
    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);
    let df = compute_dominance_frontier(&func, &idom);

    // body has "header" in its dominance frontier (back edge)
    assert!(df[&body].contains(&header));
    // entry strictly dominates header, so header is NOT in entry's DF
    assert_eq!(
        df[&entry].len(),
        0,
        "entry's DF should be empty; got {:?}",
        df[&entry]
    );
}

#[test]
fn dom_tree_diamond() {
    let func = diamond_cfg();
    let entry = func.entry;
    let (then, els, end) = (
        id_of(&func, "then"),
        id_of(&func, "else"),
        id_of(&func, "end"),
    );
    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);
    let tree = build_dom_tree(&idom);

    // entry's children include then, else, end (all directly dominated)
    let entry_children = &tree[&entry];
    assert!(entry_children.contains(&els));
    assert!(entry_children.contains(&end));
    assert!(entry_children.contains(&then));
}
