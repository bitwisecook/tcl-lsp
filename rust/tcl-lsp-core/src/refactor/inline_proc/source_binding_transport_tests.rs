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
fn logical_inline_uses_actual_source_call_header_after_reporting_maps_are_removed() {
    let source = "proc p {x} {expr {$x + 1}}\np 2";
    let mut analysis = Analyser::new().analyse(source, "tcl");
    analysis.all_procs.clear();
    analysis.command_invocations.clear();
    let action = inline_proc(
        source,
        u32::try_from(source.rfind("p 2").unwrap()).unwrap(),
        &analysis,
        analysis.resolved_registry().unwrap(),
    )
    .unwrap();
    assert!(action.disabled.is_none(), "{:?}", action.disabled);
    assert!(action.try_apply(source).unwrap().ends_with("expr {2 + 1}"));
    assert!(
        inline_proc(
            &format!("{source} "),
            u32::try_from(source.rfind("p 2").unwrap()).unwrap(),
            &analysis,
            analysis.resolved_registry().unwrap()
        )
        .is_none()
    );
}

#[test]
fn logical_formals_defaults_and_actual_argv_do_not_use_report_parameter_fields() {
    let source = "proc p {x {y 3} args} {expr {$x + $y}}\np 2";
    let analysis = Analyser::new().analyse(source, "tcl");
    let walk = super::super::FrameWalk::new(source, &analysis).unwrap();
    let commands = walk.segment_document(source);
    let call = commands.last().unwrap();
    let mut report = analysis.all_procs["::p"].clone();
    report.params.clear();
    report.params_computed = true;
    let bindings = bind_arguments(source, call, &report, &walk).unwrap();
    assert_eq!(
        bindings
            .iter()
            .map(|binding| (binding.name.as_str(), binding.value.as_str()))
            .collect::<Vec<_>>(),
        vec![("x", "2"), ("y", "3")]
    );
    assert!(
        bindings
            .iter()
            .all(|binding| !binding.evaluation_is_observable)
    );
    assert!(bind_arguments(&format!("{source} "), call, &report, &walk).is_err());
    report.name_span = tcl_lexer::Span::new(0, 1);
    assert!(bind_arguments(source, call, &report, &walk).is_err());
}

#[test]
fn logical_binding_keeps_dynamic_values_and_disagreed_formal_grammar_unavailable() {
    for source in [
        "proc p {x} {puts $x}; p $value",
        "proc p {args x} {puts $x}; p 1 2",
        "proc p {x args} {puts $x}; p 2 [observable]",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let walk = super::super::FrameWalk::new(source, &analysis).unwrap();
        let commands = walk.segment_document(source);
        assert!(
            bind_arguments(
                source,
                commands.last().unwrap(),
                &analysis.all_procs["::p"],
                &walk
            )
            .is_err()
        );
    }
}
