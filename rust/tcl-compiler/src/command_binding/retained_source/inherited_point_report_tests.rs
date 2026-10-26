// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::RetainedSourceModuleBindings;
use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
use crate::ir::{Module, ProcedureId};
use tcl_registry::CommandRegistry;

fn inherited_unit(parent_source: &str, source: &str) -> CompilationUnit {
    let parent = crate::analyser::Analyser::new().analyse(parent_source, "tcl8.6");
    let offset = u32::try_from(parent_source.find("source child.tcl").unwrap()).unwrap();
    let point = crate::registry_invocation::source_structure::original_project_source_load_at(
        parent_source,
        &parent,
        offset,
    )
    .unwrap()
    .source_point_entry(&parent)
    .unwrap();
    let input = point.input();
    let registry = input.borrowed_context_registry().commands();
    let entry = point.source_analysis_entry(registry, input).unwrap();
    CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(input.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        input,
    )
}

// Keep the original live fixtures: the installed-first lowerer must classify
// them independently of the retired-declaration custody controls below.
fn assert_original_live_installed_category(source: &str) {
    let unit = inherited_unit("source child.tcl", source);
    let module = &unit.ir_module;
    assert!(module.original_declaration_body_units.is_empty());
    assert!(
        !module.installed_procedure_body_units.is_empty(),
        "live fixture category: installed={} allocated={} named={}",
        module.installed_procedure_body_units.len(),
        module.original_declaration_body_units.len(),
        module.procedures.len()
    );
    assert!(module.source_entry.native_entry.is_none());
}

fn retired_original_source(live: &str) -> String {
    format!("{live}; rename p {{}}")
}

fn assert_original_mutating_parent_whole_point_refused() {
    let parent_source = "set prior FOREIGN; source child.tcl";
    let parent = crate::analyser::Analyser::new().analyse(parent_source, "tcl8.6");
    let offset = u32::try_from(parent_source.find("source child.tcl").unwrap()).unwrap();
    let point = crate::registry_invocation::source_structure::original_project_source_load_at(
        parent_source,
        &parent,
        offset,
    )
    .unwrap()
    .source_point_entry(&parent)
    .unwrap();
    let input = point.input();
    let registry = input.borrowed_context_registry().commands();
    assert!(
        point.source_analysis_entry(registry, input).is_none(),
        "the original mutating-parent fixture is not an eligible whole-point entry"
    );
}

fn first_allocated(module: &Module) -> ProcedureId {
    *module
        .original_declaration_body_units
        .keys()
        .next()
        .expect("genuine inherited declaration body")
}

fn accepted(
    retained: &RetainedSourceModuleBindings,
    module: &Module,
    registry: &CommandRegistry,
    id: ProcedureId,
) -> bool {
    retained.matches_inherited_point_report_body(
        module,
        registry,
        id,
        module.body_units.get(&id).unwrap(),
    )
}

#[test]
fn inherited_allocated_report_custody_keeps_generic_membership_closed() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for source in ["proc p {arg} {return $arg}; p 1", "proc p {} {}; p"] {
        assert_original_live_installed_category(source);
        let retired = retired_original_source(source);
        let unit = inherited_unit("source child.tcl", &retired);
        let module = &unit.ir_module;
        let retained = module.retained_source_bindings.as_ref().unwrap();
        let id = first_allocated(module);
        assert!(accepted(retained, module, registry, id));
        let allocation = module.original_declaration_body_units.get(&id).unwrap();
        assert!(
            !allocation.matches_source_declaration_image(&module.source, allocation.site.offset)
        );
        assert!(
            !module
                .original_source_procedure_bodies()
                .any(|(candidate, _)| *candidate == id)
        );
        assert!(module.procedures.is_empty());
        assert!(module.source_entry.native_entry.is_none());
    }
}

#[test]
fn inherited_report_custody_refuses_foreign_entry_store_and_detached_body() {
    let source = "proc p {arg} {return $arg}; p 1";
    assert_original_mutating_parent_whole_point_refused();
    let retired = retired_original_source(source);
    let unit = inherited_unit("source child.tcl", &retired);
    // Independently analyse the same eligible parent; no entry is cloned or retagged.
    let foreign = inherited_unit("source child.tcl", &retired);
    assert!(
        unit.ir_module.source_entry.source_point_entry
            != foreign.ir_module.source_entry.source_point_entry,
        "independently issued eligible parent points must differ"
    );
    let module = &unit.ir_module;
    let retained = module.retained_source_bindings.as_ref().unwrap();
    let id = first_allocated(module);
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    assert!(accepted(retained, module, registry, id));
    let mut changed = module.clone();
    changed.source_entry.source_point_entry = None;
    assert!(!accepted(retained, &changed, registry, id));
    let mut changed = module.clone();
    changed
        .source_entry
        .source_point_entry
        .clone_from(&foreign.ir_module.source_entry.source_point_entry);
    assert!(!accepted(retained, &changed, registry, id));
    let foreign_retained = foreign.ir_module.retained_source_bindings.as_ref().unwrap();
    let foreign_id = first_allocated(&foreign.ir_module);
    assert!(accepted(
        foreign_retained,
        &foreign.ir_module,
        registry,
        foreign_id
    ));
    assert!(!accepted(foreign_retained, module, registry, id));
    let body = module.body_units.get(&id).unwrap().clone();
    assert!(!retained.matches_inherited_point_report_body(module, registry, id, &body));
    let jim = tcl_registry::model::ingress::static_context_for("jim").commands();
    assert!(!accepted(retained, module, jim, id));
}

#[test]
fn inherited_report_custody_refuses_changed_header_source_namespace_and_allocation() {
    let live = "proc p {arg} {return $arg}; p 1";
    let retired = retired_original_source(live);
    let unit = inherited_unit("source child.tcl", &retired);
    let module = &unit.ir_module;
    let retained = module.retained_source_bindings.as_ref().unwrap();
    let id = first_allocated(module);
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    assert!(accepted(retained, module, registry, id));
    let mut changed = module.clone();
    changed
        .body_units
        .get_mut(&id)
        .unwrap()
        .params
        .push("foreign".to_owned());
    assert!(!accepted(retained, &changed, registry, id));
    let mut changed = module.clone();
    changed
        .body_units
        .get_mut(&id)
        .unwrap()
        .params_raw
        .push_str(" other");
    assert!(!accepted(retained, &changed, registry, id));
    let mut changed = module.clone();
    changed.body_units.get_mut(&id).unwrap().body_source = Some("return FOREIGN".to_owned());
    assert!(!accepted(retained, &changed, registry, id));
    let mut changed = module.clone();
    changed
        .body_units
        .get_mut(&id)
        .unwrap()
        .body
        .namespace_context = None;
    assert!(!accepted(retained, &changed, registry, id));
    let mut changed = module.clone();
    changed
        .original_declaration_body_units
        .get_mut(&id)
        .unwrap()
        .site
        .offset += 1;
    assert!(!accepted(retained, &changed, registry, id));
}
