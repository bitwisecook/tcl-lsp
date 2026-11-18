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
fn variable_index_uses_typed_source_declarations_and_literal_alias_targets() {
    let source =
        "namespace eval ::N {set {$literal} 1; proc p {} {variable {$literal}; puts ${$literal}}}";
    let mut analysis = Analyser::new().analyse(source, "tcl");
    let index = WorkspaceIndex::from_documents([("file:///actual.tcl", &analysis)]);
    let expected = index
        .variable_definitions_qualified("::N::$literal", "")
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    assert_ne!(
        expected,
        [] as [crate::workspace_index::WorkspaceVariable; 0]
    );
    assert_eq!(
        index.documents_aliasing_variable("::N::$literal"),
        ["file:///actual.tcl"]
    );
    assert_eq!(
        index.documents_aliasing_variable("N::$literal"),
        index.documents_aliasing_variable("::N::$literal")
    );
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
    let changed = WorkspaceIndex::from_documents([("file:///actual.tcl", &analysis)]);
    assert_eq!(
        changed
            .variable_definitions_qualified("::N::$literal", "")
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        changed.documents_aliasing_variable("::N::$literal"),
        ["file:///actual.tcl"]
    );
    assert_eq!(
        changed.variable_definitions_qualified("::REPORT_ONLY::REPORT_ONLY", ""),
        [] as [&crate::workspace_index::WorkspaceVariable; 0]
    );
    assert_eq!(
        changed.documents_aliasing_variable("::COUNTERFEIT"),
        [] as [std::string::String; 0]
    );
}

#[test]
fn unavailable_alias_source_coverage_and_missing_inputs_are_not_absence() {
    let source = "proc p {ns name} {namespace upvar $ns $name local; puts $local}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let index = WorkspaceIndex::from_documents([("file:///unknown.tcl", &analysis)]);
    assert_eq!(
        index.documents_with_ambiguous_alias_of("::N::value"),
        ["file:///unknown.tcl"]
    );
    assert_eq!(
        index.documents_aliasing_variable("::$ns::$name"),
        [] as [std::string::String; 0]
    );
    let mut removed = index.clone();
    removed.remove_document("file:///unknown.tcl");
    assert_eq!(
        removed.documents_with_ambiguous_alias_of("::N::value"),
        [] as [std::string::String; 0]
    );
    // tclsh 8.4.20/8.6.18: `set value` in a namespace writes an existing
    // global ::value; `variable` makes it ::N::value on every release.
    let source = "namespace eval ::N {variable value 1; puts $value}";
    let mut analysis = Analyser::new().analyse(source, "tcl");
    let symbol = analysis
        .original_variable_symbols
        .iter()
        .find(|occurrence| {
            occurrence.symbol().checked_c_namespace_key().as_deref() == Some("::N::value")
        })
        .expect("actual original namespace variable")
        .symbol()
        .clone();
    let foreign_source = "namespace eval ::Other {variable forged 1}";
    let foreign = Analyser::new().analyse(foreign_source, "tcl");
    let foreign_occurrence = foreign
        .original_variable_symbols
        .iter()
        .find(|occurrence| {
            occurrence.symbol().checked_c_namespace_key().as_deref() == Some("::Other::forged")
        })
        .expect("genuine foreign source occurrence")
        .clone();
    let foreign_symbol = foreign_occurrence.symbol().clone();
    analysis.original_variable_symbols.push(foreign_occurrence);
    let current = WorkspaceIndex::from_documents([("file:///current.tcl", &analysis)]);
    assert_eq!(
        current.variable_definitions_qualified("::Other::forged", ""),
        [] as [&crate::workspace_index::WorkspaceVariable; 0]
    );
    assert!(
        current
            .original_variable_occurrences(&foreign_symbol, "")
            .next()
            .is_none()
    );
    assert!(
        crate::variable_symbol::namespace_symbol_for_compatibility_key(
            source,
            &analysis,
            "::Other::forged"
        )
        .is_none()
    );
    analysis.resolved_input = None;
    let missing = WorkspaceIndex::from_documents([("file:///missing.tcl", &analysis)]);
    assert!(
        missing
            .original_variable_occurrences(&symbol, "")
            .next()
            .is_none()
    );
    assert_eq!(
        missing.variable_definitions_qualified("::N::value", ""),
        [] as [&crate::workspace_index::WorkspaceVariable; 0]
    );
    assert_eq!(
        missing.documents_with_ambiguous_alias_of("::N::value"),
        ["file:///missing.tcl"]
    );
}
