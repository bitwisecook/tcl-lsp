// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::dispatch_proof::DispatchEntryAssumption;
use tcl_compiler::executable_ir::ExecutableFunctionId;
use tcl_compiler::semantic_analysis::ExecutableAnalysisAvailability;
use tcl_registry::dialects::DialectSet;
use tcl_registry::model::ingress::static_context_for;

const DIALECT: &str = "tcl8.6";
const SOURCE: &str = r"
    proc p {} { llength {x y}; llength {x y} }
    oo::class create C {
        method m {} { llength {x y}; llength {x y} }
    }
    namespace eval n { llength {x y}; llength {x y} }
    apply {{} { llength {x y}; llength {x y} }}
";

fn fixture() -> (CompilationUnit, &'static tcl_registry::CommandRegistry) {
    let registry = static_context_for(DIALECT).commands();
    (
        CompilationUnit::build_for_dialect(SOURCE, registry, false, DIALECT),
        registry,
    )
}

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

/// Assert evidence that can only come from the explicit deep rebuild.
///
/// The ordinary path also retains an executable function for these bodies,
/// but deliberately records `WorldStateNotRequired` under `UnknownWorld`.
/// Checking only the entry assumption and `function().is_some()` would
/// therefore let a no-op deep call pass.  Match every executable invocation
/// back to the exact retained source script so a sidecar from another body,
/// or an unavailable/missing body, cannot satisfy the contract.
fn assert_deep_body_evidence(
    name: &str,
    function_unit: &tcl_compiler::compilation_unit::FunctionUnit,
    source: &tcl_compiler::ir::Script,
    phase: &str,
) {
    assert_eq!(function_unit.name, name, "{phase}: unit key/name mismatch");
    assert_eq!(
        source.statements.len(),
        2,
        "{phase}: fixture body {name} must retain both repeated llength statements",
    );

    let executable = function_unit.semantic_facts.executable();
    let function = executable
        .function()
        .unwrap_or_else(|| panic!("{phase}: {name} has no executable function after deep rebuild"));
    assert_eq!(
        function.id,
        ExecutableFunctionId::new(0),
        "{phase}: {name} has an unexpected executable function identity",
    );
    assert!(
        !function.blocks.is_empty(),
        "{phase}: {name} has no executable blocks"
    );
    for block in &function.blocks {
        assert_eq!(
            block.id.function(),
            function.id,
            "{phase}: {name} block escaped its executable function",
        );
    }

    let invocations: Vec<_> = executable.invocations().collect();
    assert_eq!(
        invocations.len(),
        source.statements.len(),
        "{phase}: {name} executable invocation population does not match its exact IR body",
    );
    for (index, invocation) in invocations.iter().enumerate() {
        assert_eq!(
            invocation.node.path(),
            &[u32::try_from(index).expect("fixture invocation index fits in a node path")],
            "{phase}: {name} invocation {index} came from a different source node",
        );
        assert_eq!(
            invocation.source.span,
            source.statements[index].span(),
            "{phase}: {name} invocation {index} came from a different source span",
        );
        assert_eq!(
            invocation.completion.function(),
            function.id,
            "{phase}: {name} completion escaped its executable function",
        );
        assert_eq!(
            invocation.argv.function(),
            function.id,
            "{phase}: {name} argv escaped its executable function",
        );
    }

    assert!(
        executable.world_state_ssa().is_some(),
        "{phase}: {name} deep rebuild did not materialise world-state SSA",
    );
}

fn assert_deep_body_families(unit: &CompilationUnit, phase: &str) {
    // The per-family loops below must not turn into vacuous success if a
    // deep rebuild drops one retained unit. Keep the same concrete fixture
    // population and apply/namespace type guards here, next to the evidence
    // they justify.
    assert_eq!(
        unit.procedures.len(),
        1,
        "{phase}: expected one retained procedure for deep evidence",
    );
    assert_eq!(
        unit.methods.len(),
        1,
        "{phase}: expected one retained method for deep evidence",
    );
    assert_eq!(
        unit.body_units.len(),
        2,
        "{phase}: expected apply and namespace-eval units for deep evidence",
    );
    assert_eq!(
        unit.body_units
            .keys()
            .filter(|name| name.contains("apply#"))
            .count(),
        1,
        "{phase}: expected one apply body for deep evidence",
    );
    assert_eq!(
        unit.body_units
            .keys()
            .filter(|name| name.contains("namespace-eval#"))
            .count(),
        1,
        "{phase}: expected one namespace-eval body for deep evidence",
    );

    for (name, function) in &unit.procedures {
        let source = &unit
            .ir_module
            .procedures
            .get(name)
            .unwrap_or_else(|| panic!("{phase}: procedure {name} missing from retained IR"))
            .body;
        assert_deep_body_evidence(name, function, source, phase);
    }
    for (name, function) in &unit.methods {
        let source = &unit
            .ir_module
            .methods
            .get(name)
            .unwrap_or_else(|| panic!("{phase}: method {name} missing from retained IR"))
            .body;
        assert_deep_body_evidence(name, function, source, phase);
    }
    for (name, function) in &unit.body_units {
        let source = &unit
            .ir_module
            .body_units
            .get(name)
            .unwrap_or_else(|| panic!("{phase}: body unit {name} missing from retained IR"))
            .body;
        assert_deep_body_evidence(name, function, source, phase);
    }
}

fn assert_panics(f: impl FnOnce()) {
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_err(),
        "mutated deep contract unexpectedly passed"
    );
}

#[test]
fn production_driver_makes_every_deferred_body_kind_abstain() {
    let (unit, registry) = fixture();
    assert_production_entry_contract(&unit, "build_for_dialect");

    // Explorer's explicit deep path rebuilds every retained sidecar and must
    // preserve the same fresh-top-level versus deferred-body boundary.
    let deep = unit.with_deep_semantic_analysis(registry, DialectSet::TCL86);
    assert_production_entry_contract(&deep, "with_deep_semantic_analysis");
    assert_deep_body_families(&deep, "with_deep_semantic_analysis");
}

#[test]
fn deep_contract_rejects_noop_and_unavailable_body_evidence() {
    let (unit, registry) = fixture();

    // Mutation: a no-op deep path leaves the interactive sidecars in place.
    // They have executable functions, but no world-state SSA, so they must
    // not satisfy the deep contract.
    assert_panics(|| assert_deep_body_families(&unit, "no-op deep"));

    // Mutation: removing one source IR entry forces the production deep API
    // to publish its typed unavailable sidecar; the retained map population
    // alone must not make this pass.
    let mut missing_ir = unit;
    missing_ir.ir_module.procedures.remove("::p");
    let rebuilt = missing_ir.with_deep_semantic_analysis(registry, DialectSet::TCL86);
    assert!(matches!(
        rebuilt
            .procedures
            .get("::p")
            .expect("retained procedure map population")
            .semantic_facts
            .executable(),
        ExecutableAnalysisAvailability::SourceUnavailable,
    ));
    assert_panics(|| assert_deep_body_families(&rebuilt, "missing procedure IR"));
}
