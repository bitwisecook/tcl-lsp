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

use super::variable_name_roles::proc_var_write_indices;
use super::*;
use tcl_compiler::analyser::Analyser;

#[test]
fn original_procedure_colours_map_formal_roles_to_their_actual_expansion_children() {
    // naming.source.original-procedure-argument-topology
    // docs/design/analysis/name-resolution-proofs/original-procedure-argument-topology.md
    for source in [
        "proc target {fixed variable rest} {upvar 1 $variable local; set local 1}; target {*}{FIXED destination} ordinary",
        "proc target {fixed variable rest} {upvar 1 $variable local; set local 1}; interp alias {} alias {} target FIXED; alias {*}{destination} ordinary",
    ] {
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let declaration = analysis.original_procedure_declarations().next().unwrap();
        assert_eq!(
            proc_var_write_indices(declaration.metadata()),
            [1],
            "independent original formal-role prerequisite"
        );
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        analysis.command_invocations.clear();
        let tokens = tests::decode_semantic(&full_with_cu_and_analysis(
            source,
            analysis.resolved_profile().unwrap(),
            analysis.resolved_registry().unwrap(),
            None,
            Some(&analysis),
        ));
        let index = LineIndex::new(source);
        let expected = index.position_at_utf16(
            u32::try_from(source.rfind("destination").unwrap()).unwrap(),
            source,
        );
        assert!(
            tokens
                .iter()
                .any(|&(line, column, _, kind, mods)| line == expected.line
                    && column == expected.character.get()
                    && kind == TokenKind::Variable as u32
                    && mods == MOD_DECLARATION)
        );
        let ordinary = index.position_at_utf16(
            u32::try_from(source.rfind("ordinary").unwrap()).unwrap(),
            source,
        );
        assert!(
            !tokens
                .iter()
                .any(|&(line, column, _, kind, mods)| line == ordinary.line
                    && column == ordinary.character.get()
                    && kind == TokenKind::Variable as u32
                    && mods == MOD_DECLARATION)
        );
    }
}
