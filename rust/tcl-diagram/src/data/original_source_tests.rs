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

use super::*;
use std::collections::HashSet;
use std::sync::Arc;
use tcl_compiler::analyser::ResolvedAnalysisInput;

fn analysed(
    source: &str,
    dialect: &str,
) -> (
    AnalysisResult,
    CompilationUnit,
    Arc<tcl_registry::model::ContextRegistry>,
) {
    document_from_one_input(source, dialect).expect("actual fixture source input")
}

#[test]
fn original_diagrams_keep_opaque_declarations_and_body_allocations_after_labels_clear() {
    // Implementation contract: naming.consumer.original-structural-diagrams
    // docs/design/analysis/name-resolution-proofs/original-structural-diagrams.md
    let source = r"proc p\uD800 {} {return FIRST}; p\uD800; proc p\uD801 {} {return OTHER}; p\uD801; proc p\uD800 {} {return LAST}; p\uD800";
    let (mut analysis, mut unit, context) = analysed(source, "tcl8.6");
    analysis.all_procs.clear();
    analysis.global_scope.procs.clear();
    for invocation in &mut analysis.command_invocations {
        invocation.name = "counterfactual".to_owned();
    }
    for procedure in unit.ir_module.procedures.values_mut() {
        procedure.name = "counterfactual".to_owned();
        procedure.qualified_name = "counterfactual".to_owned();
    }
    let data = diagram_data_for_analysis(source, &analysis, &unit.ir_module, &context);
    let declarations = data["procedures"].as_array().unwrap();
    assert_eq!(declarations.len(), 3, "{data}");
    assert_eq!(declarations[0]["name"], declarations[2]["name"]);
    assert_ne!(declarations[0]["name"], declarations[1]["name"]);
    assert_eq!(
        declarations
            .iter()
            .map(|row| row["id"].as_str().unwrap())
            .collect::<HashSet<_>>()
            .len(),
        3
    );
    assert!(
        declarations.iter().any(|row| row["flow"].is_array()),
        "{data}"
    );
    let mut without_bodies = unit.ir_module.clone();
    without_bodies.procedure_implementation_bodies = Arc::from([]);
    without_bodies.original_declaration_body_units.clear();
    without_bodies.installed_procedure_body_units.clear();
    let data = diagram_data_for_analysis(source, &analysis, &without_bodies, &context);
    assert!(
        data["procedures"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["flow"].is_null()),
        "{data}"
    );
    assert!(
        diagram_data_for_analysis(
            &format!("#changed\n{source}"),
            &analysis,
            &unit.ir_module,
            &context
        )["procedures"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mut wrong_config = analysis.clone();
    wrong_config
        .body_lexer_config
        .as_mut()
        .unwrap()
        .strict_quoting = !analysis.body_lexer_config.unwrap().strict_quoting;
    assert!(
        diagram_data_for_analysis(source, &wrong_config, &unit.ir_module, &context)["procedures"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let foreign = tcl_registry::model::ingress::context_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let changed_registry = context.with_command_store(foreign.commands().clone());
    assert!(diagram_data_for_analysis(source, &analysis, &unit.ir_module, &changed_registry)
            ["procedures"].as_array().unwrap().is_empty());
}

#[test]
fn original_diagram_calls_use_retained_allocations_and_decline_missing_lookup() {
    // Implementation contract: naming.consumer.original-structural-diagrams
    // docs/design/analysis/name-resolution-proofs/original-structural-diagrams.md
    let source = "proc target {} {return OK}; target; proc other {} {return OTHER}; other";
    let (mut analysis, unit, availability) = analysed(source, "tcl8.6");
    let context = DiagramContext {
        source,
        analysis: &analysis,
        registry: availability.commands(),
        availability: &availability,
        module: &unit.ir_module,
    };
    let at = u32::try_from(source.find("; target").unwrap() + 2).unwrap();
    let tokens = unit
        .ir_module
        .top_level
        .statements
        .iter()
        .find_map(|statement| {
            let tokens = unit
                .ir_module
                .top_level
                .retained_source_tokens_for_statement(statement)?;
            (tokens.source_binding.as_ref()?.invocation_site()?.offset == at).then_some(tokens)
        })
        .expect("actual original call tokens");
    assert!(procedure_target_at(at, Some(tokens), &context).is_some());
    for invocation in &mut analysis.command_invocations {
        invocation.original_lookup = None;
    }
    let context = DiagramContext {
        source,
        analysis: &analysis,
        registry: availability.commands(),
        availability: &availability,
        module: &unit.ir_module,
    };
    assert!(procedure_target_at(at, Some(tokens), &context).is_none());
}

#[test]
fn diagram_source_calls_keep_real_body_owners_and_refuse_foreign_ids() {
    // naming.consumer.original-structural-diagrams
    // docs/design/analysis/name-resolution-proofs/original-structural-diagrams.md
    let source = "proc p {} {return FIRST}; p; proc p {} {return LAST}; p";
    let (analysis, unit, availability) = analysed(source, "tcl8.6");
    let registry = availability.commands();
    let calls = unit
        .ir_module
        .top_level
        .statements
        .iter()
        .filter_map(|statement| {
            let tokens = unit
                .ir_module
                .top_level
                .retained_source_tokens_for_statement(statement)?;
            let (id, _) = tcl_compiler::source_graph::procedure_body_for_original_call(
                source,
                &analysis,
                &unit.ir_module,
                tokens,
                registry,
            )?;
            Some((tokens.clone(), id))
        })
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 2);
    assert_ne!(calls[0].1, calls[1].1);
    for (tokens, id) in &calls {
        assert!(unit.ir_module.procedure_ids.owns(*id));
        let context = DiagramContext {
            module: &unit.ir_module,
            source,
            analysis: &analysis,
            registry,
            availability: &availability,
        };
        let at = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .unwrap()
            .offset;
        let target = procedure_target_at(at, Some(tokens), &context).expect("original source call");
        assert_eq!(target.id, *id);
        let node = walk_call("counterfactual", &[], Some(tokens), at, &context).unwrap();
        assert_eq!(node["bodyKey"], id.reporting_key());
    }
    let (tokens, id) = &calls[0];
    let (_, foreign_unit, _) = analysed("proc foreign {} {}", "tcl8.6");
    assert_eq!(foreign_unit.ir_module.procedures.len(), 1);
    let foreign = *foreign_unit.ir_module.procedures.keys().next().unwrap();
    let mut replaced = unit.ir_module.clone();
    replaced.procedure_ids = foreign_unit.ir_module.procedure_ids.clone();
    assert!(replaced.procedure_ids.owns(foreign));
    assert!(!replaced.procedure_ids.owns(*id));
    assert!(
        tcl_compiler::source_graph::procedure_body_for_original_call(
            source, &analysis, &replaced, tokens, registry
        )
        .is_none()
    );
    let mut ambiguous = unit.ir_module.clone();
    ambiguous.retain_procedure(ambiguous.procedures.get(id).unwrap().clone());
    assert!(
        tcl_compiler::source_graph::procedure_body_for_original_call(
            source, &analysis, &ambiguous, tokens, registry
        )
        .is_none()
    );
    let mut missing = unit.ir_module.clone();
    missing.source_metadata_input = None;
    assert!(
        tcl_compiler::source_graph::procedure_body_for_original_call(
            source, &analysis, &missing, tokens, registry
        )
        .is_none()
    );
    assert!(
        tcl_compiler::source_graph::procedure_body_for_original_call(
            "proc p {} {return CHANGED}; p",
            &analysis,
            &unit.ir_module,
            tokens,
            registry
        )
        .is_none()
    );
}

#[test]
fn original_diagram_metadata_keeps_actual_availability_and_source_currency() {
    // naming.consumer.original-structural-diagrams
    // docs/design/analysis/name-resolution-proofs/original-structural-diagrams.md
    // Conditional source metadata only; no native invocation or completion
    // is issued by the same-store availability controls.
    let source = "throw ERR payload";
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let available =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(available.commands())),
    );
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    for (availability, expected) in [(&available, Some("throw")), (&older, None)] {
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(availability), config);
        let unit = CompilationUnit::build_with_analysis_input(
            source,
            tcl_compiler::compilation_unit::UnitBuildOptions {
                registry: availability.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        let mut analyser = Analyser::new().with_resolved_input(input);
        analyser.set_owned_cu_override(unit.clone());
        let mut analysis = analyser.analyse(source, profile.name);
        let tokens = unit
            .ir_module
            .top_level
            .retained_source_tokens_for_statement(&unit.ir_module.top_level.statements[0])
            .unwrap();
        let selected = |analysis: &AnalysisResult, text: &str| {
            let context = DiagramContext {
                module: &unit.ir_module,
                registry: availability.commands(),
                source: text,
                analysis,
                availability,
            };
            registry_command(0, Some(tokens), &context)
        };
        assert_eq!(selected(&analysis, source).as_deref(), expected);
        assert!(selected(&analysis, "throw OTHER payload").is_none());
        let original = analysis.resolved_input.take();
        assert!(
            selected(&analysis, source).is_none(),
            "missing supplied input"
        );
        analysis.resolved_input = original;
        let original_config = analysis.body_lexer_config;
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert!(
            selected(&analysis, source).is_none(),
            "changed source grammar"
        );
        analysis.body_lexer_config = original_config;
        assert_eq!(selected(&analysis, source).as_deref(), expected);
    }
}

#[test]
fn hosted_diagram_procedures_require_the_current_body_card() {
    let source = "proc ordinary {value {fallback DEFAULT}} {return $value}";
    let (analysis, unit, availability) = analysed(source, "f5-irules");
    let data = diagram_data_for_analysis(source, &analysis, &unit.ir_module, &availability);
    let procedures = data["procedures"].as_array().unwrap();
    assert_eq!(procedures.len(), 1);
    assert_eq!(procedures[0]["name"], "ordinary");
    assert!(!procedures[0]["bodyId"].is_null());
    assert!(!procedures[0]["flow"].is_null());
    let mut missing = unit.ir_module.clone();
    missing.original_hosted_declaration_body_units.clear();
    let data = diagram_data_for_analysis(source, &analysis, &missing, &availability);
    assert_eq!(data["procedures"].as_array().unwrap().len(), 1);
    assert!(data["procedures"][0]["bodyId"].is_null());
    assert!(data["procedures"][0]["flow"].is_null());
    let (_, other, _) = analysed("proc foreign {} {return OTHER}", "f5-irules");
    let mut foreign = unit.ir_module.clone();
    foreign.original_hosted_declaration_body_units = other
        .ir_module
        .original_hosted_declaration_body_units
        .clone();
    let data = diagram_data_for_analysis(source, &analysis, &foreign, &availability);
    assert!(data["procedures"][0]["bodyId"].is_null());
    assert!(data["procedures"][0]["flow"].is_null());
}

#[test]
fn original_diagram_events_require_actual_event_descriptors_instead_of_when_labels() {
    // Implementation contract: naming.consumer.original-structural-diagrams
    // docs/design/analysis/name-resolution-proofs/original-structural-diagrams.md
    let source = "namespace eval when {proc HTTP_REQUEST {} {return ordinary}}";
    let (analysis, unit, context) = analysed(source, "tcl8.6");
    let data = diagram_data_for_analysis(source, &analysis, &unit.ir_module, &context);
    assert!(data["events"].as_array().unwrap().is_empty(), "{data}");
    assert_eq!(data["procedures"].as_array().unwrap().len(), 1, "{data}");
    let source = "when HTTP_REQUEST { set seen [clock seconds] }";
    let (analysis, mut unit, context) = analysed(source, "f5-irules");
    let data = diagram_data_for_analysis(source, &analysis, &unit.ir_module, &context);
    assert_eq!(data["events"].as_array().unwrap().len(), 1, "{data}");
    assert_eq!(data["events"][0]["name"], "HTTP_REQUEST");
    unit.ir_module.irules_event_bodies.clear();
    assert!(
        diagram_data_for_analysis(source, &analysis, &unit.ir_module, &context)["events"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
#[test]
fn original_diagram_actions_keep_source_schema_separate_from_display_labels() {
    // Implementation contract: naming.consumer.original-irules-source-context
    // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
    for source in [
        "when HTTP_REQUEST {pool /Common/first; HTTP::respond 503}",
        "when HTTP_REQUEST {pool /Common/first\nHTTP::respond 503}",
    ] {
        let (mut analysis, unit, availability) = analysed(source, "f5-irules");
        for invocation in &mut analysis.command_invocations {
            invocation.name = "counterfactual".to_owned();
        }
        analysis.all_procs.clear();
        analysis.global_scope.classes.clear();
        let at = u32::try_from(source.find("pool /Common/first").unwrap()).unwrap();
        let (metadata, segment) =
            tcl_compiler::registry_invocation::source_structure::selected_vendor_registry_words_at(
                source, &analysis, at,
            )
            .expect("the exact retained pool source vector owns its conditional card");
        assert_eq!(segment.span.start(), at);
        assert_eq!(metadata.shape().command(), "pool");
        assert!(
            metadata
                .shape()
                .possible_traits()
                .contains(tcl_registry::Traits::DIAGRAM_ACTION)
        );
        let data = diagram_data_for_analysis(source, &analysis, &unit.ir_module, &availability);
        let action = data.pointer("/events/0/flow/0").unwrap();
        assert_eq!(action["source_schema"]["command"], "pool");
        let start = usize::try_from(action["source_span"][0].as_u64().unwrap()).unwrap();
        let end = usize::try_from(action["source_span"][1].as_u64().unwrap()).unwrap();
        assert_eq!(&source[start..end], "pool /Common/first");
        assert_eq!(
            action["source_schema"]["applicability"],
            "conditional-source"
        );
        let stale = diagram_data_for_analysis(
            &format!("#changed\n{source}"),
            &analysis,
            &unit.ir_module,
            &availability,
        );
        assert_eq!(stale["unavailable"], "current-source");
        assert!(stale["events"].as_array().unwrap().is_empty());
        assert!(stale["procedures"].as_array().unwrap().is_empty());
    }
}
