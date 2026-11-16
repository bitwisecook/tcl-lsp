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
use crate::diagnostic_policy::{Policy, apply};
use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::optimiser::{
    Optimisation, finalise_optimisations, optimise_unit, optimise_unit_raw,
};

fn produced(source: &str, code: DiagCode) -> (AnalysisResult, Vec<Optimisation>) {
    let mut analyser = Analyser::new();
    let input = analyser.prepare_analysis_input("tcl8.6").unwrap();
    let registry = input.borrowed_context_registry().commands();
    let unit = CompilationUnit::build_standalone_with_analysis_input(
        source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(input.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        &input,
    )
    .unwrap()
    .with_interprocedural(registry, Some(input.unit_profile()));
    let raw = optimise_unit_raw(&unit, registry, Some(input.unit_profile()));
    assert!(
        raw.iter()
            .all(|member| member.original_rewrite_batch().is_none())
    );
    let raw_group: Vec<_> = raw
        .iter()
        .filter(|member| member.code == code)
        .cloned()
        .collect();
    assert_eq!(raw_group.len(), 2);
    assert!(
        finalise_optimisations(&raw_group[..1], &unit, registry, Some(input.unit_profile()))
            .iter()
            .all(|member| member.original_rewrite_batch().is_none())
    );
    let issued = optimise_unit(&unit, registry, Some(input.unit_profile()));
    let manual = Optimisation::new(
        DiagCode::O114,
        "manual",
        tcl_lexer::Span::new(0, 0),
        "incr x",
    );
    assert!(
        finalise_optimisations(&[manual], &unit, registry, Some(input.unit_profile()))
            .iter()
            .all(|member| member.original_rewrite_batch().is_none())
    );
    let mut current_analyser = Analyser::new().with_resolved_input(input.clone());
    current_analyser.set_cu_override(std::sync::Arc::new(unit));
    let analysis = current_analyser.analyse(source, "tcl8.6");
    (analysis, issued)
}
fn range() -> LspRange {
    LspRange {
        start_line: 0,
        start_character: 0,
        end_line: u32::MAX,
        end_character: u32::MAX,
    }
}

#[test]
fn actual_worker_group_keeps_edits_but_stale_and_incomplete_report_cannot_issue_them() {
    let source = "proc f {flag} {set x 1; if {$flag} {puts $x} else {puts no}}\n";
    let (analysis, issued) = produced(source, DiagCode::O125);
    let pair: Vec<_> = issued
        .into_iter()
        .filter(|member| member.code == DiagCode::O125)
        .collect();
    assert_eq!(pair.len(), 2);
    assert!(
        pair.iter()
            .all(|member| member.original_rewrite_batch().is_some())
    );
    let report = apply(
        pair.iter().cloned().map(Finding::from).collect(),
        &Policy::unrestricted(),
    );
    let current = DiagnosticEditSource::for_analysis(source, &analysis).unwrap();
    let actions = rewrite_actions(
        &report,
        source,
        range(),
        &current,
        &LineIndex::new_lsp(source),
    );
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].edits.len(), 2);
    // Equal bytes/input cannot donate Standalone permission into an
    // independently supplied current source interpretation.
    let supplied = Analyser::new()
        .with_resolved_input(analysis.resolved_input.clone().unwrap())
        .analyse(source, "tcl8.6");
    let supplied_current = DiagnosticEditSource::for_analysis(source, &supplied).unwrap();
    assert_eq!(
        rewrite_actions(
            &report,
            source,
            range(),
            &supplied_current,
            &LineIndex::new_lsp(source)
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
    let partial = apply(
        vec![Finding::from(pair[0].clone())],
        &Policy::unrestricted(),
    );
    assert_eq!(
        rewrite_actions(
            &partial,
            source,
            range(),
            &current,
            &LineIndex::new_lsp(source)
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
    let changed = source.replace("puts", "list");
    let changed_analysis = Analyser::new()
        .with_resolved_input(analysis.resolved_input.clone().unwrap())
        .analyse(&changed, "tcl8.6");
    let changed_current = DiagnosticEditSource::for_analysis(&changed, &changed_analysis).unwrap();
    assert_eq!(
        rewrite_actions(
            &report,
            &changed,
            range(),
            &changed_current,
            &LineIndex::new_lsp(&changed)
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
    let mut changed_member = pair[0].clone();
    changed_member.replacement.push_str("; injected");
    let changed_report = apply(
        vec![
            Finding::from(changed_member),
            Finding::from(pair[1].clone()),
        ],
        &Policy::unrestricted(),
    );
    assert_eq!(
        rewrite_actions(
            &changed_report,
            source,
            range(),
            &current,
            &LineIndex::new_lsp(source)
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
}

#[test]
fn o127_worker_group_keeps_edits_but_stale_and_incomplete_report_cannot_issue_them() {
    let source = "proc p {y} {\n    set x [llength $y]\n    puts $x\n}\n";
    let (analysis, issued) = produced(source, DiagCode::O127);
    let pair: Vec<_> = issued
        .into_iter()
        .filter(|member| member.code == DiagCode::O127)
        .collect();
    assert_eq!(pair.len(), 2);
    assert!(
        pair.iter()
            .all(|member| member.original_rewrite_batch().is_some())
    );
    let report = apply(
        pair.iter().cloned().map(Finding::from).collect(),
        &Policy::unrestricted(),
    );
    let current = DiagnosticEditSource::for_analysis(source, &analysis).unwrap();
    let actions = rewrite_actions(
        &report,
        source,
        range(),
        &current,
        &LineIndex::new_lsp(source),
    );
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].edits.len(), 2);
    // Equal bytes/input cannot donate Standalone permission into an
    // independently supplied current source interpretation.
    let supplied = Analyser::new()
        .with_resolved_input(analysis.resolved_input.clone().unwrap())
        .analyse(source, "tcl8.6");
    let supplied_current = DiagnosticEditSource::for_analysis(source, &supplied).unwrap();
    assert_eq!(
        rewrite_actions(
            &report,
            source,
            range(),
            &supplied_current,
            &LineIndex::new_lsp(source)
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
    let partial = apply(
        vec![Finding::from(pair[0].clone())],
        &Policy::unrestricted(),
    );
    assert_eq!(
        rewrite_actions(
            &partial,
            source,
            range(),
            &current,
            &LineIndex::new_lsp(source)
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
    let changed = source.replace("puts", "list");
    let changed_analysis = Analyser::new()
        .with_resolved_input(analysis.resolved_input.clone().unwrap())
        .analyse(&changed, "tcl8.6");
    let changed_current = DiagnosticEditSource::for_analysis(&changed, &changed_analysis).unwrap();
    assert_eq!(
        rewrite_actions(
            &report,
            &changed,
            range(),
            &changed_current,
            &LineIndex::new_lsp(&changed)
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
    let mut changed_member = pair[0].clone();
    changed_member.replacement.push_str("; injected");
    let changed_report = apply(
        vec![
            Finding::from(changed_member),
            Finding::from(pair[1].clone()),
        ],
        &Policy::unrestricted(),
    );
    assert_eq!(
        rewrite_actions(
            &changed_report,
            source,
            range(),
            &current,
            &LineIndex::new_lsp(source)
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
}
