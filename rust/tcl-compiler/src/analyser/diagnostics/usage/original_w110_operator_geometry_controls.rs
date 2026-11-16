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
fn actual_selected_whole_braced_expression_owns_every_operator_edit() {
    let source = "expr {  $a == \"literal == bytes\" && ($b != \"bar\") }";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W110)
        .unwrap();
    assert_eq!(source.get(diagnostic.span.as_range()), Some("=="));
    let [fix] = diagnostic.fixes.as_slice() else {
        panic!("original whole expression fix")
    };
    assert!(fix.new_text.contains("literal == bytes"));
    assert!(fix.new_text.contains(" ne "));
    assert_eq!(
        fix.safety,
        crate::irules_checks::FixSafety::SemanticsEquivalent
    );
    let unbraced = Analyser::new().analyse("expr $a == \"literal\"", "tcl8.6");
    assert!(
        !unbraced
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W110)
    );
}
