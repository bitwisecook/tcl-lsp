// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::dispatch_proof::{
    CompleteCallerSet, CompleteExposureSet, CompleteOrderedLoadGraph, FreshSealedInterpreter,
    LoadUnitKind, OrderedLoadUnit, RegistryDispatchBaseline, SealedLoadGraphEntry,
};
use tcl_compiler::gvn::{find_loop_invariants_for_cu, find_redundancies_for_cu};
use tcl_dialect::DialectProfile;
use tcl_registry::registry_for_dialect;

const DIALECT: &str = "tcl8.6";

const WORKSPACE_GRAPH: &[OrderedLoadUnit] = &[
    OrderedLoadUnit::new(LoadUnitKind::Workspace, "workspace:/main.tcl"),
    OrderedLoadUnit::new(LoadUnitKind::Workspace, "workspace:/lib.tcl"),
];
const SOURCED_GRAPH: &[OrderedLoadUnit] = &[
    OrderedLoadUnit::new(LoadUnitKind::Workspace, "workspace:/main.tcl"),
    OrderedLoadUnit::new(LoadUnitKind::Sourced, "workspace:/sourced.tcl"),
];
const PACKAGE_GRAPH: &[OrderedLoadUnit] = &[
    OrderedLoadUnit::new(LoadUnitKind::Workspace, "workspace:/main.tcl"),
    OrderedLoadUnit::new(LoadUnitKind::Package, "package:foo@1.0"),
];

fn sealed(source: &str, graph: &'static [OrderedLoadUnit]) -> CompilationUnit {
    let registry = registry_for_dialect(DIALECT);
    let profile = DialectProfile::find(DIALECT).expect("catalogued Tcl 8.6 profile");
    CompilationUnit::build_with_sealed_procedure_entry(
        source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_dialect(DIALECT),
            dialect: DIALECT,
            external_call_sites: None,
        },
        SealedLoadGraphEntry::new(
            profile,
            FreshSealedInterpreter,
            CompleteOrderedLoadGraph::new(graph),
            CompleteCallerSet,
            CompleteExposureSet,
            RegistryDispatchBaseline,
        ),
    )
}

fn o105_count(unit: &CompilationUnit) -> usize {
    find_redundancies_for_cu(unit, registry_for_dialect(DIALECT), Some(DIALECT)).len()
}

#[test]
fn hosted_procedure_world_remains_unknown() {
    let source = "proc f {} { llength {x y}; llength {x y} }";
    let unit =
        CompilationUnit::build_for_dialect(source, registry_for_dialect(DIALECT), false, DIALECT);
    assert_eq!(o105_count(&unit), 0);
}

#[test]
fn complete_workspace_entry_recovers_procedure_redundancy() {
    let source = "proc f {} { llength {x y}; llength {x y} }";
    let unit = sealed(source, WORKSPACE_GRAPH);
    assert_eq!(o105_count(&unit), 1);
}

#[test]
fn complete_sourced_file_entry_recovers_procedure_redundancy() {
    let source = "proc f {} { llength {x y}; llength {x y} }";
    let unit = sealed(source, SOURCED_GRAPH);
    assert_eq!(o105_count(&unit), 1);
}

#[test]
fn complete_package_entry_recovers_procedure_redundancy() {
    let source = "proc f {} { llength {x y}; llength {x y} }";
    let unit = sealed(source, PACKAGE_GRAPH);
    assert_eq!(o105_count(&unit), 1);
}

#[test]
fn exact_profile_mismatch_fails_closed() {
    let source = "proc f {} { llength {x y}; llength {x y} }";
    let registry = registry_for_dialect(DIALECT);
    let wrong = DialectProfile::find("tcl9.0").expect("catalogued Tcl 9.0 profile");
    let unit = CompilationUnit::build_with_sealed_procedure_entry(
        source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_dialect(DIALECT),
            dialect: DIALECT,
            external_call_sites: None,
        },
        SealedLoadGraphEntry::new(
            wrong,
            FreshSealedInterpreter,
            CompleteOrderedLoadGraph::new(WORKSPACE_GRAPH),
            CompleteCallerSet,
            CompleteExposureSet,
            RegistryDispatchBaseline,
        ),
    );
    assert_eq!(o105_count(&unit), 0);
}

#[test]
fn dynamic_dispatch_and_observers_still_fail_closed() {
    for body in [
        "set cmd [gets stdin]; $cmd x; llength {x y}; llength {x y}",
        "trace add execution llength enter cb; llength {x y}; llength {x y}",
        "rename llength old_llength; old_llength {x y}; old_llength {x y}",
        "interp alias {} n {} llength; n {x y}; n {x y}",
        "namespace unknown handler; llength {x y}; llength {x y}",
    ] {
        let source = format!("proc cb args {{}}\nproc handler args {{}}\nproc f {{}} {{ {body} }}");
        let unit = sealed(&source, WORKSPACE_GRAPH);
        assert_eq!(o105_count(&unit), 0, "body unexpectedly proved: {body}");
    }
}

#[test]
fn structured_loop_remains_conservative_under_the_sealed_entry_contract() {
    let source = "proc f {} { set k 42; for {set i 0} {$i < 10} {incr i} { set s [format %04d [expr {$k + 1}]] }; return $s }";
    let unit = sealed(source, WORKSPACE_GRAPH);
    assert!(
        find_loop_invariants_for_cu(&unit, registry_for_dialect(DIALECT), Some(DIALECT)).is_empty()
    );
}
