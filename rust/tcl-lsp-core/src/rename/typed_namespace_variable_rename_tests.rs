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
fn namespace_variable_source_edits_ignore_alias_reports_and_require_current_symbols() {
    let source = "namespace eval ::N {set value 1; puts $value}";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let expected = namespace_variable_rename_edits(source, &analysis, "::N::value", "changed");
    assert_eq!(expected.len(), 2);
    let mut scopes = vec![&mut analysis.global_scope];
    while let Some(scope) = scopes.pop() {
        scope.name = "::REPORT_ONLY".to_owned();
        for variable in scope.variables.values_mut() {
            variable.name = "REPORT_ONLY".to_owned();
            variable.link_target = Some("::COUNTERFEIT".to_owned());
        }
        scopes.extend(&mut scope.children);
    }
    analysis.qualified_var_refs.clear();
    assert_eq!(
        namespace_variable_rename_edits(source, &analysis, "::N::value", "changed"),
        expected
    );
    assert_eq!(
        namespace_variable_rename_edits(source, &analysis, "::COUNTERFEIT", "changed"),
        [] as [crate::rename::TextEdit; 0]
    );
    assert_eq!(
        namespace_variable_rename_edits(
            "namespace eval ::N {set value 2; puts $value}",
            &analysis,
            "::N::value",
            "changed"
        ),
        [] as [crate::rename::TextEdit; 0]
    );
    analysis.resolved_input = None;
    assert_eq!(
        namespace_variable_rename_edits(source, &analysis, "::N::value", "changed"),
        [] as [crate::rename::TextEdit; 0]
    );
}
