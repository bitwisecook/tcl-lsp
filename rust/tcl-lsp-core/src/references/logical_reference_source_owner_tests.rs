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
use tcl_compiler::analyser::Analyser;

#[test]
fn logical_name_references_keep_source_headers_without_reporting_maps() {
    let source = "proc p {} {}; p; interp alias {} shortcut {} p; shortcut";
    let mut analysis = Analyser::new().analyse(source, "tcl");
    let target = analysis.all_procs["::p"].clone();
    let original = logical_proc_reference_spans(source, &analysis, &target);
    assert_eq!(original.len(), 2);
    assert!(
        original
            .iter()
            .any(|span| &source[span.as_range()] == "shortcut")
    );
    analysis.all_procs.clear();
    for invocation in &mut analysis.command_invocations {
        invocation.name = "unrelated-report-label".to_owned();
        invocation.resolved_qualified_name = Some("::wrong".to_owned());
    }
    assert_eq!(
        logical_proc_reference_spans(source, &analysis, &target),
        original
    );
    assert_eq!(
        logical_proc_reference_spans(&format!("{source} "), &analysis, &target),
        [] as [tcl_lexer::Span; 0]
    );
}

#[test]
fn logical_class_references_keep_actual_constructor_and_relation_operands() {
    let source = "oo::class create C {}; oo::class create D {superclass C}; C new";
    let mut analysis = Analyser::new().analyse(source, "tcl");
    let original = logical_class_reference_spans(source, &analysis, &analysis.all_classes["::C"]);
    assert_eq!(original.len(), 2);
    assert!(original.iter().all(|span| &source[span.as_range()] == "C"));
    for invocation in &mut analysis.command_invocations {
        invocation.name = "wrong".to_owned();
        invocation.resolved_qualified_name = None;
    }
    assert_eq!(
        logical_class_reference_spans(source, &analysis, &analysis.all_classes["::C"]),
        original
    );
    let mut changed = analysis.clone();
    changed.body_lexer_config.as_mut().unwrap().strict_quoting =
        !analysis.body_lexer_config.unwrap().strict_quoting;
    assert_eq!(
        logical_class_reference_spans(source, &changed, &changed.all_classes["::C"]),
        [] as [tcl_lexer::Span; 0]
    );
}

#[test]
fn logical_reference_candidates_do_not_replace_missing_input_or_native_owners() {
    let source = "proc p {} {}; p";
    let mut analysis = Analyser::new().analyse(source, "tcl");
    let target = analysis.all_procs["::p"].clone();
    assert_eq!(
        logical_proc_reference_spans(source, &analysis, &target).len(),
        1
    );
    analysis.resolved_input = None;
    assert_eq!(
        logical_proc_reference_spans(source, &analysis, &target),
        [] as [tcl_lexer::Span; 0]
    );
    let native = Analyser::new().analyse(source, "tcl8.6");
    assert_eq!(
        logical_proc_reference_spans(source, &native, &native.all_procs["::p"]),
        [] as [tcl_lexer::Span; 0]
    );
}
