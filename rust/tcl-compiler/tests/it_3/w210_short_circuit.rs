// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original completed command values govern expression short-circuit order.

use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_core_types::DiagCode;

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../data/w210-short-circuit.json")).unwrap()
}

fn retained_final_normal_result(source: &str, dialect: &str) -> Option<String> {
    let mut analyser = Analyser::new();
    let _analysis = analyser.analyse(source, dialect);
    let input = analyser.resolved_analysis_input();
    let settings = analyser.source_compilation_settings();
    let registry = input.borrowed_context_registry().commands();
    let profile = input.unit_profile();
    let options = UnitBuildOptions {
        registry,
        defer_top_level: false,
        config: input.lexer_config(),
        dialect: Some(profile),
        external_call_sites: None,
        declared_commands: None,
    };
    let unit = CompilationUnit::build_with_analysis_input_and_source_settings(
        source, options, None, &input, &settings,
    );
    assert!(unit.ir_module.source_entry.native_entry.is_none());
    assert_eq!(
        unit.ir_module
            .source_entry
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            .map(tcl_syntax::naming::NamePolicyProtocol::authority),
        Some(tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation)
    );
    let body = &unit.ir_module.top_level;
    let statement = body.statements.last()?;
    let tokens = body.retained_source_tokens_for_statement(statement)?;
    let binding = tokens.source_binding.as_ref()?;
    assert!(binding.admitted_inline_invocation().is_none());
    assert!(binding.admitted_named_invocation().is_none());
    binding
        .original_normal_result(tokens)
        .map(|value| value.text().to_owned())
}

#[test]
fn skipped_reads_stay_quiet_and_later_genuine_reads_keep_their_span() {
    let fixtures = fixture();
    let mut failures = Vec::new();
    for case in fixtures["diagnostics"].as_array().unwrap() {
        let source = case["source"].as_str().unwrap();
        let variable = case["variable"].as_str().unwrap();
        let expected = case["expected_w210"].as_bool().unwrap();
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse(source, dialect);
            let findings: Vec<_> = analysis
                .diagnostics
                .iter()
                .filter(|finding| finding.code == DiagCode::W210)
                .collect();
            println!(
                "SHORT_CIRCUIT {}",
                serde_json::json!({
                    "case": case["id"], "dialect": dialect,
                    "findings": findings.iter().map(|finding| serde_json::json!({
                        "start": finding.span.start(), "end": finding.span.end(),
                        "message": finding.message,
                    })).collect::<Vec<_>>(),
                })
            );
            let mut correct = findings.len() == usize::from(expected);
            if expected && findings.len() == 1 {
                let read = format!("${variable}");
                correct &= source.get(findings[0].span.as_range()) == Some(read.as_str());
                if case["expected_read"] == "last" {
                    correct &= usize::try_from(findings[0].span.start()).unwrap()
                        == source.rfind(&read).unwrap();
                }
            }
            if !correct {
                failures.push(format!(
                    "{} under {dialect}: {:?}",
                    case["id"], analysis.diagnostics
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn source_values_follow_normal_alternatives_without_compiler_admission() {
    let fixtures = fixture();
    let mut failures = Vec::new();
    for case in fixtures["mutations"].as_array().unwrap() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = retained_final_normal_result(case["source"].as_str().unwrap(), dialect);
            println!(
                "SHORT_CIRCUIT {}",
                serde_json::json!({
                    "case": case["id"], "dialect": dialect, "retained_normal_result": result,
                })
            );
            if result.as_deref() != case["expected_retained_normal_result"].as_str() {
                failures.push(format!(
                    "{} under {dialect}: {result:?}, expected {}",
                    case["id"], case["expected_retained_normal_result"]
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "Source values must match their original Normal alternatives:\n{}",
        failures.join("\n")
    );
}
