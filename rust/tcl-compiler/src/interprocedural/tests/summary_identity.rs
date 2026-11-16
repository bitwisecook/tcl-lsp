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

//! Summaries keyed by issued body IDs: authored report labels, declaration receivers and refusal of foreign or changed owners.

use super::*;

fn authored_report_unit() -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    CompilationUnit,
) {
    let environment = crate::environment_ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    // Explicit Standalone authoring is the dialect-blind build (see
    // `original_reference_unit`); a profiled build is a fresh document.
    let unit = CompilationUnit::build_for_with_config(
        "proc p {} {return ONE}; proc p {} {return TWO}",
        context.commands(),
        false,
        tcl_lexer::LexerConfig::from_grammar(environment.unit_profile().grammar),
    );
    (context, unit)
}

#[test]
fn authored_source_report_labels_keep_real_body_ids_and_current_source() {
    let environment = crate::environment_ingress::resolve_environment("tcl8.6");
    let (context, unit) = authored_report_unit();
    let registry = context.commands();
    let module = &unit.ir_module;
    assert_eq!(module.original_authored_declaration_body_units.len(), 2);
    let ids: Vec<_> = module
        .original_authored_declaration_body_units
        .keys()
        .copied()
        .collect();
    assert_ne!(ids[0], ids[1]);
    for &id in &ids {
        assert!(module.procedure_ids.owns(id));
        assert_eq!(
            module.original_procedure_report_label(id, registry),
            Some("::p")
        );
        let procedure = module.body_units.get(&id).unwrap();
        assert!(procedure.publication.is_none());
        assert!(procedure.body.executed_source.is_none());
        assert!(procedure.body.native_compilation_admission.is_none());
    }
    let identities = crate::realm::document_realm_bindings_for_module(module, registry).unwrap();
    let analysis = build_interprocedural_analysis(
        module,
        registry,
        Some(environment.unit_profile()),
        ObjectTypeMap::none(),
        &identities,
        None,
    );
    for &id in &ids {
        assert_eq!(
            analysis
                .procedures
                .get(&id)
                .expect("actual issued body summary")
                .qualified_name,
            "::p"
        );
    }
}

#[test]
fn authored_source_report_labels_refuse_foreign_and_changed_owners() {
    let environment = crate::environment_ingress::resolve_environment("tcl8.6");
    let (context, unit) = authored_report_unit();
    let registry = context.commands();
    let module = &unit.ir_module;
    let id = *module
        .original_authored_declaration_body_units
        .keys()
        .next()
        .unwrap();
    let mut presentation = module.clone();
    presentation.body_units.get_mut(&id).unwrap().qualified_name = "display changed".into();
    assert_eq!(
        presentation.original_procedure_report_label(id, registry),
        Some("::p")
    );
    let mut changed = module.clone();
    changed.source = tcl_lexer::SourceImage::document("proc p {} {return CHANGED}");
    assert!(
        changed
            .original_procedure_report_label(id, registry)
            .is_none()
    );
    let mut changed = module.clone();
    changed
        .body_units
        .get_mut(&id)
        .unwrap()
        .params
        .push("changed".into());
    assert!(
        changed
            .original_procedure_report_label(id, registry)
            .is_none()
    );
    let mut detached = module.clone();
    detached.retained_source_bindings = None;
    assert!(
        detached
            .original_procedure_report_label(id, registry)
            .is_none()
    );
    // The foreign owner is the same explicit Standalone authoring build
    // (`authored_report_unit`), retained under a different unit.
    let other = CompilationUnit::build_for_with_config(
        "proc p {} {return ONE}; proc p {} {return TWO}",
        registry,
        false,
        tcl_lexer::LexerConfig::from_grammar(environment.unit_profile().grammar),
    );
    let foreign_id = *other
        .ir_module
        .original_authored_declaration_body_units
        .keys()
        .next()
        .unwrap();
    assert!(
        module
            .original_procedure_report_label(foreign_id, registry)
            .is_none()
    );
    let foreign_registry =
        crate::environment_ingress::resolve_environment("tcl9.0").default_context_registry();
    assert!(
        module
            .original_procedure_report_label(id, foreign_registry.commands())
            .is_none()
    );
}

#[test]
fn original_declaration_receivers_keep_owned_ids_and_preceding_setup() {
    let environment = tcl_registry::model::ingress::resolve_environment("tk");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = environment.unit_profile();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let build = |source| {
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_resolved_analysis_input(input.clone());
        lowerer.lower(source);
        lowerer.finish_module(source)
    };
    let module = build("entry .before; proc danger {} {return OK}; entry .after");
    let (id, procedure) = module.original_source_procedure_bodies().next().unwrap();
    let id = *id;
    assert_eq!(
        procedure.body_kind,
        crate::ir::ProcedureBodyKind::OriginalDeclaration
    );
    assert!(procedure.publication.is_none());
    assert!(!module.procedures.contains_key(&id));
    assert!(module.source_entry.native_entry.is_none());
    let classes = global_instance_classes(&module, registry);
    assert!(classes[&id].contains_key(".before"));
    assert!(!classes[&id].contains_key(".after"));
    assert!(!module.procedures.contains_key(&id));

    let later = build("proc danger {} {return OK}; entry .later");
    let later_id = *later.original_source_procedure_bodies().next().unwrap().0;
    assert!(global_instance_classes(&later, registry)[&later_id].is_empty());
    assert!(!classes.contains_key(&later_id));
    let mut missing = module.clone();
    missing.retained_source_bindings = None;
    assert!(!global_instance_classes(&missing, registry).contains_key(&id));
    let mut changed = module.clone();
    changed.source = tcl_lexer::SourceImage::document("entry .changed; proc danger {} {return OK}");
    assert!(!global_instance_classes(&changed, registry).contains_key(&id));
    let mut foreign = module.clone();
    foreign.procedure_ids = later.procedure_ids.clone();
    assert!(!global_instance_classes(&foreign, registry).contains_key(&id));
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    assert!(!global_instance_classes(&module, foreign).contains_key(&id));
}

#[test]
fn original_declaration_summaries_keep_actual_cards_without_callable_publication() {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = environment.unit_profile();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let source = "proc p {} {return ORIGINAL}";
    // Unentered declaration cards need explicit unknown history; with no
    // entry a fresh interpreter defines `::p` (user decision 2026-10-19T14:20Z).
    let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
        .with_resolved_analysis_input(input)
        .with_explicit_unknown_entry();
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    let mut bodies = module.original_source_procedure_bodies();
    let (id, body) = bodies.next().expect("genuine original declaration card");
    assert!(bodies.next().is_none());
    let id = *id;
    assert_eq!(
        body.body_kind,
        crate::ir::ProcedureBodyKind::OriginalDeclaration
    );
    assert!(body.publication.is_none());
    assert!(!module.procedures.contains_key(&id));
    assert!(module.source_entry.native_entry.is_none());
    assert!(summary_procedure_bodies(&module, registry).contains_key(&id));
    let identities = crate::realm::document_realm_bindings_with_source_entry(
        source,
        module.lexer_config,
        registry,
        &module.source_entry,
    )
    .with_resolved_analysis_input(module.source_metadata_input.as_ref().unwrap().clone());
    let analysis = build_interprocedural_analysis(
        &module,
        registry,
        Some(profile),
        ObjectTypeMap::none(),
        &identities,
        None,
    );
    let summary = analysis
        .procedures
        .get(&id)
        .expect("conditional source summary");
    assert!(!summary.completes);
    assert!(summary.defined_at.is_none());
    assert_eq!(
        summary.defined_callees,
        [] as [tcl_runtime_api::ProcedureId; 0]
    );
    assert!(!module.procedures.contains_key(&id));

    let mut missing = module.clone();
    missing.retained_source_bindings = None;
    assert!(!summary_procedure_bodies(&missing, registry).contains_key(&id));
    let mut changed = module.clone();
    changed.source = tcl_lexer::SourceImage::document("proc p {} {return CHANGED}");
    assert!(!summary_procedure_bodies(&changed, registry).contains_key(&id));
    let mut changed = module.clone();
    changed
        .body_units
        .get_mut(&id)
        .unwrap()
        .body
        .statements
        .clear();
    assert!(!summary_procedure_bodies(&changed, registry).contains_key(&id));
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    assert!(!summary_procedure_bodies(&module, foreign).contains_key(&id));
}
