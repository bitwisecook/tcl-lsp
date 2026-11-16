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
fn source_consumed_names_and_callbacks_join_actual_headers_without_becoming_edits() {
    let source = "proc p {} {}; info body p; after 0 p; p";
    let mut analysis = Analyser::new().analyse(source, "tcl");
    let target = analysis.all_procs["::p"].clone();
    let spans = logical_proc_reference_spans(source, &analysis, &target);
    assert_eq!(spans.len(), 3);
    for offset in [
        source.find("body p").unwrap() + 5,
        source.find("0 p").unwrap() + 2,
    ] {
        let header =
            logical_proc_at_cursor(source, &analysis, u32::try_from(offset).unwrap()).unwrap();
        assert_eq!(header.metadata().name_span, target.name_span);
    }
    assert!(spans.iter().all(|span| &source[span.as_range()] == "p"));
    analysis.command_invocations.clear();
    analysis.all_procs.clear();
    assert_eq!(
        logical_proc_reference_spans(source, &analysis, &target),
        spans
    );
    assert_eq!(
        logical_proc_reference_spans(&format!("{source} "), &analysis, &target),
        [] as [tcl_lexer::Span; 0]
    );
}

#[test]
fn consumed_alias_and_callback_receipts_keep_registration_horizon_and_known_barriers() {
    let source = "proc p {} {}; interp alias {} shortcut {} p; info body shortcut; after 0 shortcut; rename shortcut {}; info body shortcut; after 0 shortcut";
    let analysis = Analyser::new().analyse(source, "tcl");
    let references =
        tcl_compiler::registry_invocation::source_structure::source_logical_name_references(
            source, &analysis,
        )
        .unwrap();
    let before = u32::try_from(source.find("info body shortcut").unwrap()).unwrap();
    let after = u32::try_from(source.rfind("info body shortcut").unwrap()).unwrap();
    assert!(
        references
            .iter()
            .any(|reference| reference.operand().span().start() > before
                && reference.operand().span().start() < after
                && reference.declaration(&analysis).is_some())
    );
    assert!(
        !references
            .iter()
            .any(|reference| reference.operand().span().start() > after
                && reference.declaration(&analysis).is_some())
    );
    let callbacks =
        tcl_compiler::registry_invocation::source_structure::source_logical_callback_candidates(
            source, &analysis,
        )
        .unwrap();
    let before_callback = callbacks
        .iter()
        .find(|callback| callback.registration().site().offset < after)
        .unwrap();
    assert!(before_callback.procedure_declaration(&analysis).is_some());
    let after_callback = callbacks
        .iter()
        .find(|callback| callback.registration().site().offset > after)
        .unwrap();
    assert!(after_callback.procedure_declaration(&analysis).is_none());
}
#[test]
fn logical_class_consumed_operands_and_callbacks_retain_actual_alias_history() {
    let source = "oo::class create C {}; interp alias {} shortcut {} C; info body shortcut; after 0 shortcut; C new";
    let analysis = Analyser::new().analyse(source, "tcl");
    let class = analysis
        .all_classes
        .values()
        .find(|class| class.name == "C")
        .unwrap();
    let spans = logical_class_reference_spans(source, &analysis, class);
    assert_eq!(spans.len(), 3);
    assert_eq!(
        spans
            .iter()
            .filter(|span| &source[span.as_range()] == "shortcut")
            .count(),
        2
    );
    let references =
        tcl_compiler::registry_invocation::source_structure::source_logical_name_references(
            source, &analysis,
        )
        .unwrap();
    let reference = references
        .iter()
        .find(|reference| &source[reference.operand().span().as_range()] == "shortcut")
        .unwrap();
    assert!(std::ptr::eq(
        reference.class_declaration(&analysis).unwrap(),
        class
    ));
    assert!(reference.declaration(&analysis).is_none());
    assert_ne!(reference.lineage(), []);
}

#[test]
fn logical_probe_patterns_and_foreign_inputs_do_not_manufacture_selected_references() {
    let source = "proc p {} {}; info commands p; info commands p*; info body p";
    let analysis = Analyser::new().analyse(source, "tcl");
    let references =
        tcl_compiler::registry_invocation::source_structure::source_logical_name_references(
            source, &analysis,
        )
        .unwrap();
    assert!(references.iter().any(|reference| reference.role()
        == tcl_registry::ArgRole::CommandNameProbe
        && reference.declaration(&analysis).is_some()));
    assert!(
        !references
            .iter()
            .any(|reference| &source[reference.operand().span().as_range()] == "p*")
    );
    let foreign = Analyser::new().analyse(&format!("{source} "), "tcl");
    assert!(
        references
            .iter()
            .all(|reference| reference.declaration(&foreign).is_none())
    );
    let different_configuration = Analyser::new().analyse(source, "tcl8.6");
    assert!(
        references
            .iter()
            .all(|reference| reference.declaration(&different_configuration).is_none())
    );
}
