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

//! Which variables get φ nodes: diamonds, loops and single definitions.

use super::*;

// Phi placement tests

#[test]
fn phi_vars_diamond_with_defs() {
    // x defined in both then and else → phi needed at end
    let mut func = diamond_cfg();
    let entry = func.entry;
    let (then, els, end) = (
        id_of(&func, "then"),
        id_of(&func, "else"),
        id_of(&func, "end"),
    );
    func.blocks
        .get_mut(&then)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: Span::new(10, 20),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        });
    func.blocks
        .get_mut(&els)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: Span::new(30, 40),
            name: "x".into(),
            name_braced: false,
            value: "2".into(),
            value_span: None,
        });

    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);
    let df = compute_dominance_frontier(&func, &idom);
    let phi = compute_phi_vars(
        &func,
        &df,
        &CommandRegistry::build_default(),
        &ArrayElems::default(),
    );

    let x = crate::var_resolve::canonical_literal_variable_key(
        "x",
        &ResolveContext::for_function(&func.name),
        &CommandRegistry::build_default(),
    )
    .unwrap();
    assert!(phi[&end].contains(&x), "x should need a phi at 'end'");
    assert!(
        !phi[&entry].contains(&x),
        "x should not need a phi at entry"
    );
}

#[test]
fn phi_vars_single_def_no_phi() {
    // x defined only in entry → no phi needed anywhere
    let mut func = diamond_cfg();
    let entry = func.entry;
    func.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        });

    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);
    let df = compute_dominance_frontier(&func, &idom);
    let phi = compute_phi_vars(
        &func,
        &df,
        &CommandRegistry::build_default(),
        &ArrayElems::default(),
    );

    let x = crate::var_resolve::canonical_literal_variable_key(
        "x",
        &ResolveContext::for_function(&func.name),
        &CommandRegistry::build_default(),
    )
    .unwrap();
    for vars in phi.values() {
        assert!(!vars.contains(&x), "x should not need a phi anywhere");
    }
}

#[test]
fn phi_vars_loop_def() {
    // i defined in entry and body → phi at header
    let mut func = loop_cfg();
    let entry = func.entry;
    let (header, body) = (id_of(&func, "header"), id_of(&func, "body"));
    func.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: Span::new(0, 10),
            name: "i".into(),
            name_braced: false,
            value: "0".into(),
            value_span: None,
        });
    func.blocks
        .get_mut(&body)
        .unwrap()
        .statements
        .push(Statement::Incr {
            span: Span::new(30, 40),
            name: "i".into(),
            name_braced: false,
            amount: None,
            amount_braced: false,
            safe_on_uninit: false,
        });

    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);
    let df = compute_dominance_frontier(&func, &idom);
    let phi = compute_phi_vars(
        &func,
        &df,
        &CommandRegistry::build_default(),
        &ArrayElems::default(),
    );

    let i = crate::var_resolve::canonical_literal_variable_key(
        "i",
        &ResolveContext::for_function(&func.name),
        &CommandRegistry::build_default(),
    )
    .unwrap();
    assert!(phi[&header].contains(&i), "i should need a phi at 'header'");
}

#[test]
fn phi_vars_no_defs_no_phis() {
    let func = diamond_cfg();
    let dom = compute_dominators(&func);
    let idom = compute_idom(&func, &dom);
    let df = compute_dominance_frontier(&func, &idom);
    let phi = compute_phi_vars(
        &func,
        &df,
        &CommandRegistry::build_default(),
        &ArrayElems::default(),
    );

    for vars in phi.values() {
        assert_eq!(vars.len(), 0, "no defs → no phis");
    }
}
