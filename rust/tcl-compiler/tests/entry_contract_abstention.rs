// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::dispatch_proof::DispatchEntryAssumption;
use tcl_registry::dialects::DialectSet;
use tcl_registry::registry_for_dialect;

const DIALECT: &str = "tcl8.6";

fn assert_production_entry_contract(unit: &CompilationUnit, phase: &str) {
    assert_eq!(
        unit.top_level.semantic_facts.dispatch_entry_assumption(),
        DispatchEntryAssumption::PristineRegistryWorld,
        "{phase}: top-level entry contract changed",
    );

    // Keep each retained body family populated: an empty iterator could make
    // the abstention checks below vacuous if lowering stopped retaining one.
    assert_eq!(
        unit.procedures.len(),
        1,
        "{phase}: expected one procedure body",
    );
    assert_eq!(
        unit.methods.len(),
        1,
        "{phase}: expected one TclOO method body"
    );
    assert_eq!(
        unit.body_units.len(),
        2,
        "{phase}: expected apply and namespace-eval body units",
    );
    assert_eq!(
        unit.body_units
            .keys()
            .filter(|name| name.contains("apply#"))
            .count(),
        1,
        "{phase}: expected one apply lambda body unit",
    );
    assert_eq!(
        unit.body_units
            .keys()
            .filter(|name| name.contains("namespace-eval#"))
            .count(),
        1,
        "{phase}: expected one namespace-eval body unit",
    );

    for (name, function) in &unit.procedures {
        assert_eq!(
            function.semantic_facts.dispatch_entry_assumption(),
            DispatchEntryAssumption::UnknownWorld,
            "{phase}: procedure {name} received an unproved entry world",
        );
    }
    for (name, function) in &unit.methods {
        assert_eq!(
            function.semantic_facts.dispatch_entry_assumption(),
            DispatchEntryAssumption::UnknownWorld,
            "{phase}: TclOO method {name} received an unproved entry world",
        );
    }
    for (name, function) in &unit.body_units {
        assert_eq!(
            function.semantic_facts.dispatch_entry_assumption(),
            DispatchEntryAssumption::UnknownWorld,
            "{phase}: deferred body {name} received an unproved entry world",
        );
    }
}

#[test]
fn production_driver_makes_every_deferred_body_kind_abstain() {
    let source = r"
        proc p {} { llength {x y}; llength {x y} }
        oo::class create C {
            method m {} { llength {x y}; llength {x y} }
        }
        namespace eval n { llength {x y}; llength {x y} }
        apply {{} { llength {x y}; llength {x y} }}
    ";
    let registry = registry_for_dialect(DIALECT);
    let unit = CompilationUnit::build_for_dialect(source, registry, false, DIALECT);
    assert_production_entry_contract(&unit, "build_for_dialect");

    // Explorer's explicit deep path rebuilds every retained sidecar and must
    // preserve the same fresh-top-level versus deferred-body boundary.
    let deep = unit.with_deep_semantic_analysis(registry, DialectSet::TCL86);
    assert_production_entry_contract(&deep, "with_deep_semantic_analysis");
}
