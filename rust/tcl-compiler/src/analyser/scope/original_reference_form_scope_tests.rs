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

#[test]
fn scope_reference_forms_keep_literal_closing_braces_and_nested_sigil_names() {
    let source = "set {a{b}} VALUE\nputs ${a{b}}";
    let analysis = Analyser::new().analyse(source, "tcl9.0");
    let at = u32::try_from(source.find("${a{b}}").unwrap()).unwrap();
    assert!(
        analysis.global_scope.variables["a{b}"]
            .references
            .iter()
            .any(|span| span.start() == at)
    );

    let source = "puts [list ${$n}]; expr {${$n} eq {VALUE}}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let config = analysis.body_lexer_config.unwrap();
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let mut reads = Vec::new();
    for command in tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config) {
        collect_script_command_reads(source, &analysis, &command, registry, config, &mut reads);
    }
    let sigils: Vec<_> = reads.iter().filter(|read| read.name == "$n").collect();
    assert_eq!(sigils.len(), 2);
    assert!(sigils.iter().all(|read| read.braced_literal));
}
