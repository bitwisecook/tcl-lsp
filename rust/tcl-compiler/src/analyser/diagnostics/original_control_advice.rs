// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Observable source advice under the genuinely selected control descriptor.

use crate::analyser::{Analyser, DiagnosticSubject, RegistrySourceDiagnosticKind};
use tcl_core_types::DiagCode;

#[test]
fn original_control_shape_keeps_unknown_payload_and_selected_alias() {
    // naming.diagnostic.original-control-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-control-advice.md
    for source in [
        "if {$unknown}",
        "rename if branch; branch {$unknown}",
        "interp alias {} branch {} if; branch {$unknown}",
    ] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::E004)
            .unwrap();
        assert!(
            matches!(&diagnostic.subject, Some(DiagnosticSubject::RegistrySource(subject)) if subject.kind() == RegistrySourceDiagnosticKind::ClauseShape)
        );
    }
}

#[test]
fn original_control_shape_refuses_replaced_heads_and_unknown_keywords() {
    // naming.diagnostic.original-control-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-control-advice.md
    for source in ["proc if args {}; if {$unknown}", "if 1 {puts ok} $keyword"] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::E004)
        );
    }
}

#[test]
fn original_control_capture_keeps_alias_body_and_whole_edit_extent() {
    // naming.diagnostic.original-control-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-control-advice.md
    let source = "interp alias {} capture {} catch; capture {\nerror oops\n}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W302)
        .unwrap();
    assert!(
        matches!(&diagnostic.subject, Some(DiagnosticSubject::RegistrySource(subject)) if subject.kind() == RegistrySourceDiagnosticKind::ErrorCapture)
    );
    assert!(!diagnostic.fixes.is_empty());
    for fix in &diagnostic.fixes {
        assert_eq!(fix.span.start() as usize, source.len());
        assert_eq!(fix.span.start(), fix.span.end());
    }
}

#[test]
fn original_control_capture_does_not_donate_teardown_to_namespace_tail() {
    // naming.diagnostic.original-control-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-control-advice.md
    let source = "namespace eval application {proc close args {error actual}}; catch {application::close value}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W302)
    );
    let replaced = Analyser::new().analyse("proc catch args {}; catch {error actual}", "tcl8.6");
    assert!(
        !replaced
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W302)
    );
}

#[test]
fn original_control_context_uses_count_without_guessing_operand_values() {
    // naming.diagnostic.original-control-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-control-advice.md
    let result = Analyser::new().analyse("when CLIENT_ACCEPTED {return $unknown}", "f5-irules");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W142)
        .unwrap();
    assert!(
        matches!(&diagnostic.subject, Some(DiagnosticSubject::RegistrySource(subject)) if subject.kind() == RegistrySourceDiagnosticKind::ContextGate)
    );
    let procedure = Analyser::new().analyse("proc f {} {return $unknown}", "f5-irules");
    assert!(
        !procedure
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W142)
    );
}

#[test]
fn original_control_captured_prefix_does_not_mint_a_written_clause_edit() {
    // naming.diagnostic.original-control-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-control-advice.md
    let result = Analyser::new().analyse("interp alias {} branch {} if 1; branch", "tcl8.6");
    // The missing-body subject addresses the captured condition. It has no
    // call-site word and therefore cannot supply this word-anchored diagnostic.
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::E004)
    );
}

#[test]
fn original_control_clause_edit_keeps_the_last_compound_word_complete() {
    // naming.diagnostic.original-control-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-control-advice.md
    let source = "if 1 {puts ok} else body trailing[command]";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|row| row.code == DiagCode::E004)
        .unwrap();
    let [fix] = diagnostic.fixes.as_slice() else {
        panic!("one complete proposal")
    };
    assert_eq!(&source[fix.span.as_range()], "body trailing[command]");
    assert_eq!(fix.new_text, "{body trailing[command]}");
    let mut fixed = source.to_owned();
    fixed.replace_range(fix.span.as_range(), &fix.new_text);
    assert_eq!(fixed, "if 1 {puts ok} else {body trailing[command]}");
    assert!(
        !Analyser::new()
            .analyse(&fixed, "tcl8.6")
            .diagnostics
            .iter()
            .any(|row| row.code == DiagCode::E004)
    );
}

#[test]
fn original_control_clause_edit_uses_the_grammar_keyword_not_a_condition_name() {
    // naming.diagnostic.original-control-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-control-advice.md
    let source = "if 1 {puts ok} elseif elseif";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|row| row.code == DiagCode::E004)
        .unwrap();
    let [fix] = diagnostic.fixes.as_slice() else {
        panic!("one grammar-owned proposal")
    };
    assert_eq!(&source[fix.span.as_range()], "elseif elseif");
    assert!(fix.new_text.is_empty());
    let mut fixed = source.to_owned();
    fixed.replace_range(fix.span.as_range(), &fix.new_text);
    assert_eq!(fixed, "if 1 {puts ok} ");
    assert!(
        !Analyser::new()
            .analyse(&fixed, "tcl8.6")
            .diagnostics
            .iter()
            .any(|row| row.code == DiagCode::E004)
    );
}
