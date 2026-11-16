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
fn positioned_procedure_reference_navigates_without_reported_maps() {
    let source = "proc p {} {return BODY}\np";
    let mut result = crate::analyser::Analyser::new().analyse(source, "tcl9.0");
    let definition = result
        .command_invocations
        .iter()
        .find(|invocation| {
            invocation.range.start() == u32::try_from(source.rfind('p').unwrap()).unwrap()
        })
        .unwrap()
        .resolved_definition
        .clone()
        .unwrap();
    let expected = result
        .original_procedure_declarations()
        .next()
        .unwrap()
        .metadata()
        .name_span;
    result.all_procs.clear();
    result.superseded_procs.clear();
    assert_eq!(
        result
            .proc_for_definition(&definition, source)
            .unwrap()
            .name_span,
        expected
    );
    assert!(
        result
            .proc_for_definition(&definition, "proc p {} {return OTHER}\np")
            .is_none()
    );
}
