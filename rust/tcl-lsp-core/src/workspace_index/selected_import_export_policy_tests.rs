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
fn jim_import_search_uses_actual_tail_without_a_c_export_requirement() {
    // Original C5/Jim probe: no export keeps C's target missing; Jim imports.
    let source = "namespace eval ::Source {proc p {} {return SOURCE}}; namespace eval ::Target {namespace import ::Source::*; p}";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
        let mut index = WorkspaceIndex::new();
        index.add_document("original", &analysis);
        let actual = index.resolve_wildcard_import(
            "p",
            &["::Target::p".to_owned(), "::p".to_owned()],
            CallSite {
                uri: "original",
                at: u32::try_from(source.len() - 1).unwrap(),
                enclosing_body: None,
            },
        );
        assert_eq!(
            actual.as_deref(),
            (dialect == "jimtcl").then_some("::Source::p"),
            "{dialect}"
        );
    }
    let source = "namespace eval ::Self {proc p {} {return SELF}; namespace import ::Self::*; p}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "jimtcl");
    let mut index = WorkspaceIndex::new();
    index.add_document("original", &analysis);
    assert_eq!(
        index.resolve_wildcard_import(
            "p",
            &["::Self::p".to_owned(), "::p".to_owned()],
            CallSite {
                uri: "original",
                at: u32::try_from(source.len() - 1).unwrap(),
                enclosing_body: None
            }
        ),
        None
    );
}
