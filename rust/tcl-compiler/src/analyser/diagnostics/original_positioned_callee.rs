// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Positioned body availability never comes from reporting-name populations.

use super::*;

fn unproved_call(command: &str) -> crate::ir::Statement {
    crate::ir::Statement::Barrier {
        span: tcl_lexer::Span::new(0, u32::try_from(command.len()).unwrap()),
        reason: "unproved source invocation".into(),
        command: command.into(),
        canonical_command: Some("puts".into()),
        args: vec!["local_name".into()],
        tokens: None,
    }
}

#[test]
fn positioned_body_availability_refuses_unproved_native_labels_and_future_definitions() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    let mut analyser = Analyser::new();
    let analysis = analyser.analyse("unknown local_name; proc unknown {} {}", "tcl8.6");
    analyser.result = analysis;
    let context = analyser.analysis_context();
    let registry = context.commands();
    let resolver = analyser.unit_command_resolver(registry);
    assert!(resolver.logical_definitions.is_none());
    assert!(!resolver.procedures.is_empty());
    for name in ["unknown", "puts", "C", "object", "alias"] {
        assert!(!resolver.resolves(&unproved_call(name)), "{name}");
    }
    analyser
        .result
        .created_instance_commands
        .insert("object".into());
    analyser
        .result
        .instance_classes
        .insert("object".into(), "::C".into());
    let resolver = analyser.unit_command_resolver(registry);
    assert!(!resolver.resolves(&unproved_call("object")));
    assert!(!resolver.resolves(&unproved_call("alias")));
}

#[test]
fn reporting_procedure_compatibility_requires_positive_logical_inputs() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    let mut analyser = Analyser::new();
    analyser.result = analyser.analyse("proc available {} {}", "tcl");
    let context = analyser.analysis_context();
    let registry = context.commands();
    let resolver = analyser.unit_command_resolver(registry);
    assert!(resolver.logical_definitions.is_some());
    assert!(resolver.resolves(&unproved_call("available")));
    assert!(!resolver.resolves(&unproved_call("missing")));
}

#[test]
fn reporting_procedure_compatibility_refuses_missing_input_and_uses_shared_qualification() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    let mut analyser = Analyser::new();
    analyser.result = analyser.analyse("namespace eval ns {proc available {} {}}", "tcl");
    let context = analyser.analysis_context();
    let resolver = analyser.unit_command_resolver(context.commands());
    assert!(resolver.resolves(&unproved_call("::ns::available")));
    assert!(!resolver.resolves(&unproved_call("available")));
    analyser.result.resolved_input = None;
    analyser.result.lexical_declaration_advice = true;
    let resolver = analyser.unit_command_resolver(context.commands());
    assert!(resolver.logical_definitions.is_none());
    assert!(!resolver.resolves(&unproved_call("::ns::available")));
}
