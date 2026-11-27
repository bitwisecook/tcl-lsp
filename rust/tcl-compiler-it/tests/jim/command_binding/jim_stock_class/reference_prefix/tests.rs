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
fn global_incoming_values_cannot_donate_reference_freshness_through_the_memo() {
    use tcl_compiler::command_binding::{
        SourceAnalysisOptions, SourceCommandBindings, SourceCommandSlotPresence,
    };
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    let source = "class C {x 0}; C method m {} {missing_after_incoming $self $x}";
    let result = tcl_compiler::analyser::Analyser::new().analyse(source, "jim");
    let input = result.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let offset = u32::try_from(source.find("missing_after_incoming").unwrap()).unwrap();
    let incoming = ["existing_reference".to_owned()];
    for names in [&[][..], incoming.as_slice(), &[][..]] {
        let bindings = SourceCommandBindings::analyse_in_frame_with_options(
            source,
            &tcl_compiler::var_resolve::VariableExecutionFrame::Global,
            input.lexer_config(),
            registry,
            SourceAnalysisOptions {
                metadata_context:
                    tcl_compiler::registry_invocation::InvocationMetadataInput::SuppliedSource(
                        Some(input),
                    ),
                invocation_dialect: Some(
                    tcl_compiler::environment_ingress::authoring_invocation_dialect(
                        registry,
                        Some(input.unit_profile()),
                        input.lexer_config(),
                    ),
                ),
                incoming_formals: names,
                native_compilation: tcl_compiler::environment_ingress::authoring_native_compilation(
                ),
                ..SourceAnalysisOptions::default()
            },
        );
        assert_eq!(
            bindings.diagnostic_slot_presence_at(offset),
            SourceCommandSlotPresence::unavailable()
        );
        assert!(
            bindings
                .invocation_at_source("", offset)
                .proved_execution_target()
                .is_none()
        );
    }
}
