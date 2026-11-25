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

use crate::interprocedural::test_body_identity::fixture_id_if_present;

use super::*;
use crate::compilation_unit::UnitBuildOptions;
use std::sync::Arc;

fn supplied(
    text: &str,
    custom_definer: bool,
) -> (CompilationUnit, Arc<tcl_registry::model::ContextRegistry>) {
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let context = if custom_definer {
        let mut store =
            CommandRegistry::build_default().project_for_profile(environment.analyser_profile());
        let selected = store.get("when").unwrap().clone();
        store.insert(tcl_registry::CommandSpec {
            name: "on",
            ..selected
        });
        Arc::new(
            environment
                .default_context_registry()
                .with_command_store(Arc::new(store)),
        )
    } else {
        environment.default_context_registry()
    };
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        base_line: 23,
        ..tcl_lexer::LexerConfig::from_grammar(environment.grammar())
    };
    let input = crate::analyser::ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        Arc::clone(&context),
        config,
    );
    let unit = CompilationUnit::build_with_analysis_input(
        text,
        UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(environment.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    (unit, context)
}

fn events(source: &IrulesSourceQuery<'_>) -> Option<Vec<String>> {
    let mut names = Vec::new();
    for function in source.unit.functions() {
        if let tcl_core_types::Knowledge::Proven(event) = source.event(function)? {
            names.push(event.body.event().to_owned());
        }
    }
    names.sort();
    Some(names)
}

fn event_warnings(unit: &CompilationUnit, registry: &CommandRegistry) -> Vec<IrulesCheckWarning> {
    let surface = Some(SurfaceQuery::any_release(Family::F5Irules));
    let mut warnings = Vec::new();
    if let Some(advice) = find_conditional_collection_source_pairing_advice(unit, registry, surface)
    {
        if let Some(flow) = advice.conditional_source_flow() {
            warnings.extend(flow.findings.iter().map(|finding| finding.warning.clone()));
        }
        warnings.extend(
            advice
                .pairings
                .iter()
                .map(|pairing| pairing.warning.clone()),
        );
    }
    warnings.extend(find_hoistable_set_warnings(unit, surface));
    warnings.extend(find_generic_static_name_warnings(unit, surface, None));
    warnings
}

#[test]
fn source_flow_requires_actual_selected_commands_and_original_children() {
    let surface = Some(SurfaceQuery::any_release(Family::F5Irules));
    for text in [
        "proc drop args {}; when HTTP_REQUEST {drop}",
        "proc HTTP::respond args {}; when HTTP_REQUEST {HTTP::respond 200; HTTP::header insert X yes}",
        "when HTTP_REQUEST {set value {prefix[drop]}; set response {[HTTP::respond 200]}; HTTP::header insert X yes}",
    ] {
        let (unit, _) = supplied(text, false);
        assert!(
            find_unguarded_drop_warnings(&unit, surface).is_empty(),
            "{text}"
        );
        assert!(find_http_flow_warnings(&unit, surface).is_empty(), "{text}");
    }
    for text in [
        "when HTTP_REQUEST {::drop}",
        r#"when HTTP_REQUEST {set value "prefix[drop]"}"#,
        "when HTTP_REQUEST {drop; event disable $target}",
        "when HTTP_REQUEST {drop; event disable all extra}",
    ] {
        let (unit, _) = supplied(text, false);
        assert!(
            find_unguarded_drop_warnings(&unit, surface)
                .iter()
                .any(|warning| warning.code == DiagCode::Irule5002),
            "{text}"
        );
    }
    for text in [
        r#"when HTTP_REQUEST {set value "prefix[HTTP::respond 200]"; HTTP::header insert X yes}"#,
        "when HTTP_REQUEST {set value $a([HTTP::respond 200]); HTTP::header insert X yes}",
    ] {
        let (unit, _) = supplied(text, false);
        assert!(
            find_http_flow_warnings(&unit, surface)
                .iter()
                .any(|warning| warning.code == DiagCode::Irule1201),
            "{text}"
        );
    }
}

#[test]
fn source_flow_evaluates_original_header_and_return_arguments_before_dispatch() {
    let surface = Some(SurfaceQuery::any_release(Family::F5Irules));
    for tail in [
        "return [HTTP::header value X]",
        "if [HTTP::header exists X] {}",
        "while [HTTP::header exists X] {break}",
        "foreach name [HTTP::header names] {}",
        "switch [HTTP::header value X] {yes {}}",
    ] {
        let text = format!("when HTTP_REQUEST {{HTTP::respond 200; {tail}}}");
        let (unit, _) = supplied(&text, false);
        let warnings = find_http_flow_warnings(&unit, surface);
        assert_eq!(
            warnings
                .iter()
                .filter(|warning| warning.code == DiagCode::Irule1201)
                .count(),
            1,
            "{text}: {warnings:?}"
        );
    }

    let (unit, _) = supplied(
        "when HTTP_REQUEST {catch {return [HTTP::respond 200]}; HTTP::header value X}",
        false,
    );
    assert!(
        find_http_flow_warnings(&unit, surface)
            .iter()
            .any(|warning| warning.code == DiagCode::Irule1201)
    );

    for tail in [
        "return {[HTTP::header value X]}",
        "if 0 {HTTP::header value X}",
        "return; HTTP::header value X",
    ] {
        let text = format!("when HTTP_REQUEST {{HTTP::respond 200; {tail}}}");
        let (unit, _) = supplied(&text, false);
        assert!(find_http_flow_warnings(&unit, surface).is_empty(), "{text}");
    }
}

#[test]
fn ordinary_procedure_with_an_event_shaped_name_has_no_event_semantics() {
    // naming.variable.registry-event-frame-identity
    // docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
    // Source classification only; this does not execute an iRule or Tcl proc.
    let text = "namespace eval ::when {}; proc ::when::HTTP_REQUEST {} {\n\
            HTTP::respond 200; HTTP::header insert X-Test yes; drop;\n\
            set static::debug 1; set svc foo; HTTP::payload\n}";
    let (unit, context) = supplied(text, false);
    assert!(fixture_id_if_present(&unit.procedures, "::when::HTTP_REQUEST").is_some());
    let source = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    assert_eq!(events(&source), Some(Vec::new()));
    assert_eq!(event_mutation_counts(&source), Some(HashMap::new()));
    assert_eq!(
        collect_flow_state(&source).unwrap().1,
        [] as [std::string::String; 0]
    );
    assert_eq!(
        event_warnings(&unit, context.commands()),
        [] as [crate::irules_checks::IrulesCheckWarning; 0]
    );
    assert!(event_anchor_span(&unit, context.commands(), "HTTP_REQUEST").is_none());
}

#[test]
fn selected_custom_event_definer_and_report_names_use_the_original_descriptor() {
    // naming.variable.registry-event-frame-identity
    // docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
    // Event-table policy and source descriptor only, without entered event/TMM facts.
    let text = "on ASM_RESPONSE_VIOLATION {HTTP::respond 200; HTTP::header insert X-Test yes}\n\
            on HTTP_REQUEST {}\nwhen HTTP_REQUEST {}";
    let (unit, context) = supplied(text, true);
    let source = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    let expected = vec![
        "ASM_RESPONSE_VIOLATION".to_owned(),
        "HTTP_REQUEST".to_owned(),
        "HTTP_REQUEST".to_owned(),
    ];
    assert_eq!(events(&source), Some(expected.clone()));
    let baseline = event_warnings(&unit, context.commands());
    assert!(
        baseline
            .iter()
            .any(|warning| warning.code == DiagCode::Irule1201)
    );
    let anchor = event_anchor_span(&unit, context.commands(), "HTTP_REQUEST").unwrap();
    assert_eq!(
        &text[anchor.start() as usize..anchor.end() as usize],
        "HTTP_REQUEST"
    );
    let mut labelled = unit.clone();
    for (index, function) in labelled.procedures.values_mut().enumerate() {
        function.name = format!("::ordinary::report{index}");
    }
    let source = IrulesSourceQuery::capture(&labelled, context.commands()).unwrap();
    assert_eq!(events(&source), Some(expected));
    assert_eq!(event_warnings(&labelled, context.commands()), baseline);
    assert_eq!(
        event_anchor_span(&labelled, context.commands(), "HTTP_REQUEST"),
        Some(anchor)
    );
    for text in [
        "rename on moved; moved HTTP_REQUEST {}",
        "interp alias {} fire {} on; fire HTTP_REQUEST {}",
    ] {
        let (selected, context) = supplied(text, true);
        let source = IrulesSourceQuery::capture(&selected, context.commands()).unwrap();
        assert_eq!(events(&source), Some(vec!["HTTP_REQUEST".to_owned()]));
    }
    let function = source.unit.procedures.values().next().unwrap().clone();
    assert!(
        source.event(&function).is_none(),
        "a copied FU is not the actual CU-owned function"
    );
}

#[test]
fn event_projection_withdraws_changed_or_missing_complete_owners() {
    // naming.variable.registry-event-frame-identity
    // docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
    let (unit, context) = supplied(
        "when HTTP_REQUEST {drop; HTTP::payload; set static::debug 1}",
        false,
    );
    let original = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    assert_eq!(events(&original), Some(vec!["HTTP_REQUEST".to_owned()]));
    assert_ne!(
        event_warnings(&unit, context.commands()),
        [] as [crate::irules_checks::IrulesCheckWarning; 0]
    );
    for axis in 0..10 {
        let mut changed = unit.clone();
        match axis {
            0 => changed.source.push_str("foreign"),
            1 => changed.ir_module.source_metadata_input = None,
            2 => changed.ir_module.retained_source_bindings = None,
            3 => changed.ir_module.lexer_config.leading_bom = tcl_lexer::LeadingBom::Content,
            4 => {
                changed.ir_module.lexer_config.strict_quoting =
                    !changed.ir_module.lexer_config.strict_quoting;
            }
            5 => changed
                .procedures
                .values_mut()
                .for_each(|function| function.source_metadata_input = None),
            6 => changed.procedures.values_mut().for_each(|function| {
                function.source_config.strict_quoting = !function.source_config.strict_quoting;
            }),
            7 => changed
                .procedures
                .values_mut()
                .for_each(|function| function.irules_event_body = None),
            8 => changed.ir_module.irules_event_bodies.clear(),
            _ => changed
                .ir_module
                .procedures
                .values_mut()
                .for_each(|procedure| procedure.body.statements.clear()),
        }
        let source = IrulesSourceQuery::capture(&changed, context.commands());
        assert!(
            source
                .as_ref()
                .is_none_or(|source| events(source).is_none()),
            "axis {axis}"
        );
        assert!(
            source
                .as_ref()
                .is_none_or(|source| collect_flow_state(source).is_none()),
            "pairing axis {axis}"
        );
        assert!(
            source
                .as_ref()
                .is_none_or(|source| event_mutation_counts(source).is_none()),
            "mutations axis {axis}"
        );
        assert!(
            event_warnings(&changed, context.commands()).is_empty(),
            "warnings axis {axis}"
        );
        assert!(
            event_anchor_span(&changed, context.commands(), "HTTP_REQUEST").is_none(),
            "anchor axis {axis}"
        );
    }
    let foreign = tcl_registry::model::resolve_known_environment("tcl9.1")
        .unwrap()
        .default_context_registry();
    assert!(IrulesSourceQuery::capture(&unit, foreign.commands()).is_none());
    let (other, _) = supplied("when CLIENT_ACCEPTED {drop}", false);
    let mut donated = unit.clone();
    donated
        .procedures
        .values_mut()
        .next()
        .unwrap()
        .irules_event_body = other
        .procedures
        .values()
        .next()
        .unwrap()
        .irules_event_body
        .clone();
    let source = IrulesSourceQuery::capture(&donated, context.commands()).unwrap();
    assert!(events(&source).is_none());
    assert_eq!(
        event_warnings(&donated, context.commands()),
        [] as [crate::irules_checks::IrulesCheckWarning; 0]
    );
}

#[test]
fn explicitly_standalone_source_retains_events_without_report_name_assistance() {
    // naming.variable.registry-event-frame-identity
    // docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
    let (original, context) = supplied("when HTTP_REQUEST {drop}", false);
    let mut unit = CompilationUnit::build_for_with_config(
        &original.source,
        context.commands(),
        false,
        original.ir_module.lexer_config,
    );
    for function in unit.procedures.values_mut() {
        function.name = "::plain::report".to_owned();
    }
    let source = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    assert!(source.standalone);
    assert_eq!(events(&source), Some(vec!["HTTP_REQUEST".to_owned()]));
    assert!(
        find_unguarded_drop_warnings(&unit, Some(SurfaceQuery::any_release(Family::F5Irules)))
            .iter()
            .any(|warning| warning.code == DiagCode::Irule5002)
    );
    unit.procedures
        .values_mut()
        .next()
        .unwrap()
        .irules_event_body = None;
    let source = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
    assert!(events(&source).is_none());
    assert_eq!(
        find_unguarded_drop_warnings(&unit, Some(SurfaceQuery::any_release(Family::F5Irules))),
        [] as [crate::irules_checks::IrulesCheckWarning; 0]
    );
}
