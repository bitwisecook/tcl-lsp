// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The original sinking-helper fixture qualifies source-body custody first.

use super::TaintSourceContext;
use crate::compilation_unit::CompilationUnit;

// Exact original interprocedural_taint::tainted_argument_into_sinking_helper_warns.
const SOURCE: &str = "proc do_eval {x} { eval $x }\nset data [read $fd]\ndo_eval $data\n";

fn fixture(registry: &tcl_registry::CommandRegistry) -> CompilationUnit {
    CompilationUnit::build_for(SOURCE, registry, false)
}

#[test]
fn sinking_helper_fixture_retains_its_actual_authored_body_and_current_cfg() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = fixture(registry);
    let module = &unit.ir_module;
    assert!(module.source_entry.metadata_context.is_standalone());
    assert!(module.source_metadata_input.is_none());
    assert!(module.source_entry.native_entry.is_none());
    assert!(module.procedures.is_empty());
    assert_eq!(module.original_authored_declaration_body_units.len(), 1);
    let (&id, card) = module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let procedure = unit.original_procedure_for_body(id).unwrap();
    assert!(card.matches_procedure(module, procedure));
    assert!(procedure.publication.is_none());
    assert!(procedure.body.native_compilation_admission.is_none());
    let function = unit.function_for_body(Some(id)).unwrap();
    assert!(crate::ssa::current_authored_analysis_function(
        &unit, function, registry
    ));
    assert!(function.has_original_semantic_value_owner());
    assert_eq!(
        unit.original_procedure_cfg_for_body(id),
        Some(&function.cfg)
    );
    assert!(function.ssa.has_current_source_cfg_topology(&function.cfg));
    let tokens = procedure
        .body
        .statements
        .iter()
        .find_map(|statement| {
            procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .filter(|tokens| tokens.synthetic.is_none())
        })
        .unwrap();
    let scope = tokens
        .source_binding
        .as_ref()
        .unwrap()
        .original_authored_source_body_scope_for_name_roles(tokens, registry)
        .unwrap();
    assert!(scope.matches_module(module));
    assert!(scope.original_words(tokens, registry).is_some());
    assert!(card.owns_original_source_scope(scope));
    let retained = module.retained_source_bindings.as_deref().unwrap();
    assert!(retained.positioned_for_module(module, registry).is_some());
    assert!(retained.owns_original_analysis_attachment(tokens));
    // The general top-level context remains closed for this distinct body purpose.
    let general = TaintSourceContext::for_unit_function(registry, &unit, function);
    assert!(general.metadata_context().is_none());
    assert!(!general.allows_nominal_metadata());
}

#[test]
fn sinking_helper_fixture_body_guard_refuses_foreign_stale_and_changed_cfg() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = fixture(registry);
    let (&id, _) = unit
        .ir_module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let foreign = fixture(registry);
    let foreign_id = *foreign
        .ir_module
        .original_authored_declaration_body_units
        .keys()
        .next()
        .unwrap();
    assert!(!crate::ssa::current_authored_analysis_function(
        &unit,
        foreign.function_for_body(Some(foreign_id)).unwrap(),
        registry,
    ));
    let mut stale = unit.clone();
    stale.source.push(' ');
    assert!(!crate::ssa::current_authored_analysis_function(
        &stale,
        stale.function_for_body(Some(id)).unwrap(),
        registry,
    ));
    let mut changed = unit.clone();
    changed.body_units.get_mut(&id).unwrap().cfg.blocks.clear();
    assert!(!crate::ssa::current_authored_analysis_function(
        &changed,
        changed.function_for_body(Some(id)).unwrap(),
        registry,
    ));
    let mut withdrawn = unit.clone();
    withdrawn.ir_module.source_entry.metadata_context =
        crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(None);
    assert!(!crate::ssa::current_authored_analysis_function(
        &withdrawn,
        withdrawn.function_for_body(Some(id)).unwrap(),
        registry,
    ));
    let other = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    assert!(!crate::ssa::current_authored_analysis_function(
        &unit,
        unit.function_for_body(Some(id)).unwrap(),
        other,
    ));
}
