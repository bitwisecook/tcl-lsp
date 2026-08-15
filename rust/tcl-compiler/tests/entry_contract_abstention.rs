// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::dispatch_proof::DispatchEntryAssumption;
use tcl_registry::registry_for_dialect;

const DIALECT: &str = "tcl8.6";

#[test]
fn every_deferred_body_kind_abstains_without_a_runtime_owned_session() {
    let source = r"
        proc p {} { llength {x y}; llength {x y} }
        oo::class create C {
            method m {} { llength {x y}; llength {x y} }
        }
        namespace eval n { llength {x y}; llength {x y} }
        apply {{} { llength {x y}; llength {x y} }}
    ";
    let unit =
        CompilationUnit::build_for_dialect(source, registry_for_dialect(DIALECT), false, DIALECT);

    assert_eq!(
        unit.top_level.semantic_facts.dispatch_entry_assumption(),
        DispatchEntryAssumption::PristineRegistryWorld
    );
    assert!(
        !unit.procedures.is_empty(),
        "procedure fixture was not lowered"
    );
    assert!(!unit.methods.is_empty(), "method fixture was not lowered");
    assert!(
        unit.body_units.len() >= 2,
        "apply and namespace-eval fixtures were not both lowered"
    );
    for function in unit
        .procedures
        .values()
        .chain(unit.methods.values())
        .chain(unit.body_units.values())
    {
        assert_eq!(
            function.semantic_facts.dispatch_entry_assumption(),
            DispatchEntryAssumption::UnknownWorld,
            "{} received an unproved entry world",
            function.name
        );
    }
}
