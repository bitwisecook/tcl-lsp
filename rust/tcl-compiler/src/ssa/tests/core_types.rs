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

//! The SSA data structures: φ nodes, statements, blocks, functions, array-root refresh symbols and the interner.

use super::*;

// Data structure tests

#[test]
fn phi_construction() {
    let phi = Phi {
        name: Symbol(0),
        version: 3,
        incoming: HashMap::from([(BlockId(1), 1), (BlockId(2), 2)]),
    };
    assert_eq!(phi.name, Symbol(0));
    assert_eq!(phi.version, 3);
    assert_eq!(phi.incoming.len(), 2);
}

#[test]
fn ssa_statement_construction() {
    let stmt = SsaStatement {
        statement: Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        },
        uses: HashMap::new(),
        defs: HashMap::from([(Symbol(0), 1)]),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    };
    assert_eq!(stmt.defs[&Symbol(0)], 1);
    assert_eq!(stmt.uses.len(), 0);
}

#[test]
fn ssa_block_construction() {
    let block = SsaBlock {
        name: "entry".into(),
        phis: vec![],
        statements: vec![],
        entry_versions: HashMap::new(),
        exit_versions: HashMap::from([(Symbol(0), 1)]),
    };
    assert_eq!(block.name, "entry");
    assert_eq!(block.phis, [] as [crate::ssa::Phi; 0]);
}

#[test]
fn array_relationships_use_physical_kinds_instead_of_key_text() {
    let mut elements = ArrayElems::default();
    elements.note(tcl_names::place::scalar(
        "literal(parenthesis)",
        "::",
        false,
    ));
    assert!(elements.elements.is_empty());
    let element = tcl_names::place::array_elem(
        "a",
        tcl_names::place::Index::literal("key"),
        "::N(parenthesis)",
        false,
    );
    let element_key = crate::var_resolve::canonical_binding_value_key(&element).unwrap();
    let mut root = element.clone();
    root.kind = tcl_names::place::PlaceKind::ArrayWhole;
    root.index = None;
    let root_key = crate::var_resolve::canonical_binding_value_key(&root).unwrap();
    elements.note(element);
    assert_eq!(elements.root_for_element(&element_key), Some(&root_key));
    assert_eq!(
        expand_defs(std::slice::from_ref(&element_key), &elements),
        vec![element_key, root_key]
    );
}

#[test]
fn array_refresh_preserves_parentheses_in_its_namespace_identity() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "namespace eval {::N(parenthesis)} {proc p {} {variable a; set a(key) OLD; set a(key) NEW; return $a(key)}}",
        registry,
        false,
    );
    let function = unit.function("::N(parenthesis)::p").unwrap();
    let root = function
        .ssa
        .cell_symbol("::N(parenthesis)::a")
        .expect("qualified physical root");
    assert!(function.ssa.is_array_root_refresh_symbol(root));
    assert_eq!(function.ssa.cell_symbol("::N"), None);
}

#[test]
fn scalar_recreation_does_not_inherit_array_refresh_classification() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc f {} {set a(key) OLD; unset a; set a 5; return $a}";
    let unit = crate::compilation_unit::CompilationUnit::build_for(source, registry, false);
    let function = unit.function("::f").unwrap();
    let mut refreshes = HashSet::new();
    let mut scalar_stores = HashSet::new();
    for statement in function
        .ssa
        .blocks
        .values()
        .flat_map(|block| &block.statements)
    {
        for (&symbol, &version) in &statement.defs {
            if function.ssa.is_array_root_refresh_version(symbol, version) {
                assert!(!function.ssa.is_array_root_refresh_symbol(symbol));
                if source
                    .get(statement.statement.span().start() as usize..)
                    .is_some_and(|written| written.starts_with("set a(key) OLD"))
                {
                    refreshes.insert(statement.statement.span().start());
                }
            }
            if source
                .get(statement.statement.span().start() as usize..)
                .is_some_and(|written| written.starts_with("set a 5"))
            {
                scalar_stores.insert(statement.statement.span().start());
                assert!(!function.ssa.is_array_root_refresh_version(symbol, version));
            }
        }
    }
    // The element store refreshes its aggregate root, and recreation
    // writes a scalar. Destruction may carry a separate invalidation.
    assert_eq!(refreshes.len(), 1);
    assert_eq!(scalar_stores.len(), 1);
}

#[test]
fn synthetic_array_root_refresh_is_not_dictionary_value_contents() {
    for profile in ["tcl8.6", "jim"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {} {set a(key) OLD; set a(key) NEW; return $a(key)}",
            registry,
            false,
        );
        let function = unit.function("::f").unwrap();
        let refreshes: Vec<_> = function
            .ssa
            .cell_names()
            .iter()
            .enumerate()
            .filter_map(|(index, name)| {
                let symbol = Symbol(u32::try_from(index).unwrap());
                function
                    .ssa
                    .is_array_root_refresh_symbol(symbol)
                    .then_some(name)
            })
            .collect();
        if profile == "tcl8.6" {
            assert_eq!(refreshes.len(), 1, "{refreshes:?}");
        } else {
            assert!(refreshes.is_empty(), "{refreshes:?}");
        }
    }
}

#[test]
fn ssa_function_construction() {
    let func = SsaFunction {
        name: "::test".into(),
        entry: BlockId(0),
        blocks: HashMap::new(),
        value_clobbers: HashMap::new(),
        idom: HashMap::new(),
        dominance_frontier: HashMap::new(),
        dominator_tree: HashMap::new(),
        block_names: vec!["entry".into()],
        var_names: Vec::new(),
        cell_names: Vec::new(),
        cell_keys: Vec::new(),
        cell_indices: Vec::new(),
        source_variables: FxHashMap::default(),
        source_may_effects: HashMap::new(),
        original_script_points: HashMap::new(),
        source_topology: HashMap::new(),
        var_to_symbol: rustc_hash::FxHashMap::default(),
        cell_to_symbol: rustc_hash::FxHashMap::default(),
        point_symbols: HashMap::new(),
        point_contexts: None,
        read_references: HashMap::new(),
        source_read_references: HashMap::new(),
        array_root_refresh_symbols: HashSet::new(),
        array_root_refresh_versions: HashSet::new(),
        unseen_call_versions: HashSet::new(),
    };
    assert_eq!(func.name, "::test");
}

#[test]
fn var_interner_assigns_first_seen_order_symbols() {
    let mut func = SsaFunction::trivial("::test", BlockId(0), vec!["entry".into()]);
    let x = func.intern_var("x");
    let y = func.intern_var("y");
    assert_eq!(x, Symbol(0));
    assert_eq!(y, Symbol(1));
    // Re-interning a known name returns the existing symbol.
    assert_eq!(func.intern_var("x"), x);
    assert_eq!(func.var_symbol("y"), Some(y));
    assert_eq!(func.var_symbol("missing"), None);
    assert_eq!(func.var_name(x), "x");
    assert_eq!(func.var_names(), ["x".to_string(), "y".to_string()]);
}
