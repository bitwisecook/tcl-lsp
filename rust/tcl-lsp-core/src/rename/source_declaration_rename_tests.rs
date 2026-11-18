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
fn logical_variable_rename_uses_original_edit_coverage_and_refuses_alias_only_anchors() {
    let source = "set actual 1; puts $actual";
    let analysis = Analyser::new().analyse(source, "tcl");
    let conditional =
        conditional_variable_rename_with_diagnosis(source, 0, 4, "changed", &analysis).unwrap();
    assert_eq!(conditional.edits.len(), 2);
    assert_eq!(conditional.generated_source, "set changed 1; puts $changed");
    let source = "proc p {i} {upvar ::n::a($i) data; puts $data}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let offset = u32::try_from(source.find(" data;").unwrap() + 1).unwrap();
    let index = LineIndex::new(source);
    let position = index.position_at_utf16(offset, source);
    assert_eq!(
        rename_var(
            source,
            position.line,
            position.character.get(),
            "changed",
            &analysis,
            &index,
            "data"
        ),
        [] as [crate::rename::TextEdit; 0]
    );
    assert!(
        conditional_variable_rename_with_diagnosis(
            source,
            position.line,
            position.character.get(),
            "changed",
            &analysis
        )
        .is_err()
    );
}
