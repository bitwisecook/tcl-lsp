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

#[test]
fn jim_wrapper_body_keeps_a_distinct_opaque_future_world() {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    let source = "class Point {x 0}; Point method m {arg} {missing_jim_method $arg $self}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "jim");
    let bindings = analysis
        .retained_command_realm()
        .unwrap()
        .source_bindings_ref();
    let offset = u32::try_from(source.find("missing_jim_method").unwrap()).unwrap();
    assert!(
        bindings
            .invocation_at_source("", offset)
            .proved_execution_target()
            .is_none()
    );
    let futures = bindings.future_body_inventories();
    assert_eq!(futures.len(), 1);
    assert_eq!(
        futures[0].registration().offset,
        u32::try_from(source.find("Point method").unwrap()).unwrap()
    );
    assert_eq!(
        futures[0].source().origin.source_image().bytes(),
        source.as_bytes()
    );
    assert!(futures[0].frame().namespace_identity().is_none());
    assert!(matches!(
        futures[0].frame().layout(),
        tcl_names::variable::VariableExecutionFrame::Procedure { namespace, .. } if namespace == "::"
    ));
    let future = futures[0].invocation_at(offset).unwrap();
    assert_eq!(
        future.selected_slot_diagnostic_presence(),
        tcl_compiler::command_binding::SourceCommandSlotPresence::unavailable()
    );
}

#[test]
fn jim_wrapper_namespace_follows_both_published_procedure_keys() {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    let source = "class n::C {}; n::C method {m::k} {} {missing_jim_method}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "jim");
    let bindings = analysis
        .retained_command_realm()
        .unwrap()
        .source_bindings_ref();
    let future = bindings.future_body_inventories().first().unwrap();
    assert!(future.frame().namespace_identity().is_none());
    assert!(matches!(
        future.frame().layout(),
        tcl_names::variable::VariableExecutionFrame::Procedure { namespace, .. } if namespace == "::n::n::C m"
    ));
    assert_eq!(future.source().try_text().unwrap(), "missing_jim_method");
    assert!(matches!(
        future.frame().layout(),
        tcl_names::variable::VariableExecutionFrame::Procedure { .. }
    ));
}

#[test]
fn jim_wrapper_does_not_borrow_computed_or_recooked_body_geometry() {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    for source in [
        "set body {missing_jim_method}; class C {}; C method m {} $body",
        "class C {}; C method m {} \"missing_jim_method\\n\"",
        "oo::class create C {method m {} {missing_jim_method}}",
    ] {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "jim");
        assert!(
            analysis
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref()
                .future_body_inventories()
                .is_empty(),
            "{source}"
        );
    }
}
