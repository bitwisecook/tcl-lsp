// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::OriginalSourceCommandAvailability;
use crate::analyser::{
    Analyser, AnalysisReportComparison, AuthenticatedAnalysisReport, DiagnosticSubject,
};
use std::sync::Arc;

fn reports() -> (AuthenticatedAnalysisReport, AuthenticatedAnalysisReport) {
    let parent_source = "source child.tcl";
    let parent = Analyser::new().analyse(parent_source, "tcl8.6");
    let point = crate::registry_invocation::source_structure::original_project_source_load_at(
        parent_source,
        &parent,
        0,
    )
    .unwrap()
    .source_point_entry(&parent)
    .unwrap();
    let source = include_str!("../../../../../samples/for_screenshots/20-folding.tcl");
    let left = Analyser::new().source_point_report_for_cache_test(source, "tcl8.6", &point);
    let right = Analyser::new().source_point_report_for_cache_test(source, "tcl8.6", &point);
    (left, right)
}

fn subject(report: &AuthenticatedAnalysisReport) -> OriginalSourceCommandAvailability {
    report
        .analysis()
        .diagnostics
        .iter()
        .find_map(|diagnostic| {
            if diagnostic.code.to_string() != "W002" {
                return None;
            }
            match diagnostic.subject.as_ref()? {
                DiagnosticSubject::CommandAvailability(receipt) => Some(receipt.as_ref().clone()),
                _ => None,
            }
        })
        .expect("original W002 CommandAvailability subject")
}

#[test]
fn availability_report_joins_actual_owners_without_changing_runtime_eq() {
    let (left, right) = reports();
    let a = subject(&left);
    let b = subject(&right);
    assert!(a.matches_analysis(left.analysis()));
    assert!(b.matches_analysis(right.analysis()));
    assert_ne!(a, b, "ordinary subject equality remains strict");
    let output = left.availability_output_pair_for_test(&right).unwrap();
    assert_eq!(a.authenticated_report_eq(&b, &output), Some(true));
    assert_eq!(left.compare(&right), AnalysisReportComparison::Equal);
}

#[test]
fn availability_report_refuses_foreign_missing_and_corrupt_subjects() {
    let (left, right) = reports();
    let (foreign, _) = reports();
    let a = subject(&left);
    let b = subject(&right);
    let output = left.availability_output_pair_for_test(&right).unwrap();
    assert_eq!(a.authenticated_report_eq(&b, &output), Some(true));
    assert!(
        subject(&foreign)
            .authenticated_report_eq(&b, &output)
            .is_none()
    );
    let mut missing = a.clone();
    let mut invocation = (*missing.invocation).clone();
    invocation.source_binding = None;
    missing.invocation = Arc::new(invocation);
    assert!(missing.authenticated_report_eq(&b, &output).is_none());
    let mut changed = a.clone();
    let mut invocation = (*changed.invocation).clone();
    invocation.argv_texts[0].push('x');
    changed.invocation = Arc::new(invocation);
    assert!(changed.authenticated_report_eq(&b, &output).is_none());
    let mut empty = a;
    empty.original = Arc::from(Vec::new());
    assert!(empty.authenticated_report_eq(&b, &output).is_none());
}

#[test]
fn availability_report_preserves_every_other_binding_facet() {
    let (left, right) = reports();
    let a = subject(&left);
    let b = subject(&right);
    let output = left.availability_output_pair_for_test(&right).unwrap();
    let a = a.invocation.source_binding.as_ref().unwrap();
    let b = b.invocation.source_binding.as_ref().unwrap();
    assert!(a.authenticated_availability_report_eq(b, &output).is_some());
    a.assert_argument_syntax_report_refusals_for_test(b, &output);
    let mut changed = b.clone();
    changed.unknown = !changed.unknown;
    assert!(
        a.authenticated_availability_report_eq(&changed, &output)
            .is_none()
    );
    let mut changed = b.clone();
    changed.lookup_namespace.push('x');
    assert!(
        a.authenticated_availability_report_eq(&changed, &output)
            .is_none()
    );
    assert!(
        a.authenticated_availability_report_eq(
            &crate::command_binding::SourceInvocationBinding::default(),
            &output
        )
        .is_none()
    );
    let different = Analyser::new().analyse_for_report("set value VALUE", "tcl8.4");
    assert!(left.availability_output_pair_for_test(&different).is_none());
}
