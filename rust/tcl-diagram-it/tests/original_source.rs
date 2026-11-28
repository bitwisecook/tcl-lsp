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

use std::collections::HashSet;
use std::sync::Arc;
use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_diagram::diagram_data_for_analysis;

fn document_from_one_input(
    source: &str,
    dialect: &str,
) -> Option<(
    AnalysisResult,
    CompilationUnit,
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
)> {
    let input = Analyser::new().prepare_analysis_input(dialect).ok()?;
    let availability = input.context_registry();
    let options = tcl_compiler::compilation_unit::UnitBuildOptions {
        registry: availability.commands(),
        defer_top_level: false,
        config: input.lexer_config(),
        dialect: Some(input.unit_profile()),
        external_call_sites: None,
        declared_commands: None,
    };
    let cu = CompilationUnit::build_with_analysis_input(source, options, None, &input);
    let mut analyser = Analyser::new().with_resolved_input(input);
    analyser.set_owned_cu_override(cu.clone());
    let analysis = analyser.analyse(source, dialect);
    Some((analysis, cu, availability))
}

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

fn install_surfaces() {
    tcl_registry::install_shipped().unwrap();
}

#[test]
fn original_diagrams_keep_opaque_declarations_and_body_allocations_after_labels_clear() {
    install_surfaces();
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
    assert_eq!(
        diagram_data_for_analysis(
            &format!("#changed\n{source}"),
            &analysis,
            &unit.ir_module,
            &context
        )["procedures"]
            .as_array()
            .unwrap()
            .as_slice(),
        [] as [serde_json::Value; 0]
    );
    let mut wrong_config = analysis.clone();
    wrong_config
        .body_lexer_config
        .as_mut()
        .unwrap()
        .strict_quoting = !analysis.body_lexer_config.unwrap().strict_quoting;
    assert_eq!(
        diagram_data_for_analysis(source, &wrong_config, &unit.ir_module, &context)["procedures"]
            .as_array()
            .unwrap()
            .as_slice(),
        [] as [serde_json::Value; 0]
    );
    let foreign = tcl_registry::model::ingress::context_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let changed_registry = context.with_command_store(foreign.commands().clone());
    assert_eq!(diagram_data_for_analysis(source, &analysis, &unit.ir_module, &changed_registry)
            ["procedures"].as_array().unwrap().as_slice(), [] as [serde_json::Value; 0]);
}

#[test]
fn original_diagram_metadata_keeps_actual_availability_and_source_currency() {
    install_surfaces();
    // naming.consumer.original-structural-diagrams
    // docs/design/analysis/name-resolution-proofs/original-structural-diagrams.md
    // Conditional source metadata only; no native invocation or completion
    // is issued by the same-store availability controls.
    let source = "proc p {} {throw ERR payload}";
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
        let selected = |analysis: &AnalysisResult, text: &str| {
            let data = diagram_data_for_analysis(text, analysis, &unit.ir_module, availability);
            data["procedures"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|procedure| procedure["flow"].as_array().into_iter().flatten())
                .any(|node| node["completion"] == "error")
        };
        assert_eq!(selected(&analysis, source), expected.is_some());
        assert!(!selected(&analysis, "proc p {} {throw OTHER payload}"));
        let original = analysis.resolved_input.take();
        assert!(!selected(&analysis, source), "missing supplied input");
        analysis.resolved_input = original;
        let original_config = analysis.body_lexer_config;
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert!(!selected(&analysis, source), "changed source grammar");
        analysis.body_lexer_config = original_config;
        assert_eq!(selected(&analysis, source), expected.is_some());
    }
}

#[test]
fn hosted_diagram_procedures_require_the_current_body_card() {
    install_surfaces();
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
    install_surfaces();
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
    assert_eq!(
        diagram_data_for_analysis(source, &analysis, &unit.ir_module, &context)["events"]
            .as_array()
            .unwrap()
            .as_slice(),
        [] as [serde_json::Value; 0]
    );
}

#[test]
fn original_diagram_actions_keep_source_schema_separate_from_display_labels() {
    install_surfaces();
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
        assert_eq!(
            stale["events"].as_array().unwrap().as_slice(),
            [] as [serde_json::Value; 0]
        );
        assert_eq!(
            stale["procedures"].as_array().unwrap().as_slice(),
            [] as [serde_json::Value; 0]
        );
    }
}

#[test]
fn original_diagram_calls_decline_missing_lookup() {
    install_surfaces();
    let source = "proc target {} {return OK}; proc caller {} {target}";
    let (mut analysis, unit, context) = analysed(source, "tcl8.6");
    let project = |analysis: &AnalysisResult| {
        diagram_data_for_analysis(source, analysis, &unit.ir_module, &context)
    };
    let data = project(&analysis);
    let caller = data["procedures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "caller")
        .unwrap();
    assert!(
        caller["flow"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["kind"] == "proc_call")
    );
    for invocation in &mut analysis.command_invocations {
        invocation.original_lookup = None;
    }
    let data = project(&analysis);
    let caller = data["procedures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "caller")
        .unwrap();
    assert!(
        !caller["flow"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["kind"] == "proc_call")
    );
}

#[test]
fn diagram_calls_reject_foreign_procedure_owners() {
    install_surfaces();
    let source = "proc target {} {return OK}; proc caller {} {target}";
    let (analysis, unit, context) = analysed(source, "tcl8.6");
    let data = diagram_data_for_analysis(source, &analysis, &unit.ir_module, &context);
    let procedures = data["procedures"].as_array().unwrap();
    let target = procedures.iter().find(|p| p["name"] == "target").unwrap();
    let caller = procedures.iter().find(|p| p["name"] == "caller").unwrap();
    assert!(
        caller["flow"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["kind"] == "proc_call" && node["bodyKey"] == target["bodyKey"])
    );
    let (_, foreign, _) = analysed("proc foreign {} {}", "tcl8.6");
    let mut replaced = unit.ir_module.clone();
    replaced.procedure_ids = foreign.ir_module.procedure_ids.clone();
    let data = diagram_data_for_analysis(source, &analysis, &replaced, &context);
    assert!(
        !data["procedures"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|p| p["flow"].as_array().into_iter().flatten())
            .any(|node| node["kind"] == "proc_call")
    );
}
