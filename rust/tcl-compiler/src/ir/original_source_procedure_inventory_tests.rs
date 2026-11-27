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

fn logical_missing_namespace_module() -> super::Module {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(context.commands(), config)
            .with_resolved_analysis_input(input),
        "proc ::uncreated::p {} {global x; set x VALUE}",
    )
}

fn allocated_module() -> (tcl_vm::Vm, super::Module) {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let (owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let mut lowerer = crate::lowering::Lowerer::with_config(
        registry,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );
    lowerer.set_source_analysis_options(crate::command_binding::SourceAnalysisOptions {
        native_entry: Some(&entry),
        invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
        ..Default::default()
    });
    let module = crate::lowering::lower_to_ir_with(
        lowerer,
        "proc p {} {global x; set x VALUE}; rename p {}; missing_command",
    );
    (owner, module)
}

#[test]
fn logical_missing_namespace_has_no_allocation_bearing_declaration() {
    let module = logical_missing_namespace_module();
    assert!(module.source_entry.native_entry.is_none());
    assert!(module.procedures.is_empty());
    assert!(module.original_declaration_body_units.is_empty());
    assert!(!module.original_logical_declaration_body_units.is_empty());
    assert!(module.body_units.values().all(|procedure| {
        procedure.publication.is_none() && procedure.body.executed_source.is_none()
    }));
}

#[test]
fn original_declaration_summary_inventory_requires_issued_body_and_allocation() {
    let (_owner, module) = allocated_module();
    assert!(module.source_entry.native_entry.is_some());
    let (id, allocation) = module
        .original_declaration_body_units
        .iter()
        .next()
        .expect("original declaration retained without callable publication");
    let id = *id;
    assert!(
        module
            .original_source_procedure_bodies()
            .any(|(body, _)| *body == id)
    );
    assert!(!module.procedures.iter().any(|(body, _)| *body == id));
    let mut stale = module.clone();
    stale
        .original_declaration_body_units
        .get_mut(&id)
        .unwrap()
        .site
        .offset = allocation.site.offset.saturating_add(1);
    assert!(
        !stale
            .original_source_procedure_bodies()
            .any(|(body, _)| *body == id)
    );
    let mut missing = module.clone();
    missing.body_units.remove(&id);
    assert!(
        !missing
            .original_source_procedure_bodies()
            .any(|(body, _)| *body == id)
    );
    let mut other_issuer = module;
    other_issuer.procedure_ids = super::ProcedureIdIssuer::default();
    assert!(
        !other_issuer
            .original_source_procedure_bodies()
            .any(|(body, _)| *body == id)
    );
}
