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
fn source_documentation_preserves_exact_class_and_method_declaration_cards() {
    let source = "source lib.tcl\noo::class create Widget {\n    method run {} {return 1}\n}\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let class = source_documentation_hover(source, 1, 18, &analysis).unwrap();
    assert!(class.value.contains("oo::class create ::Widget"));
    assert!(class.value.contains("run"));
    let method = source_documentation_hover(source, 2, 12, &analysis).unwrap();
    assert!(method.value.contains("run"));
    assert!(method.value.contains("availability depend on execution"));
    let mut unavailable = analysis.clone();
    unavailable.original_class_metadata.clear();
    assert!(source_documentation_hover(source, 1, 18, &unavailable).is_none());
}

#[test]
fn source_documentation_does_not_borrow_schema_roles_for_variable_substitution() {
    let source = "source lib.tcl\nset $missing value\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    assert!(source_documentation_hover(source, 1, 7, &analysis).is_none());
    analysis.original_variable_symbols.clear();
    analysis.original_variable_symbol_conflicts.clear();
    assert!(source_documentation_hover(source, 1, 7, &analysis).is_none());
    let source = "puts \"$missing\"";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    assert!(source_documentation_hover(source, 0, 9, &analysis).is_none());
}

#[test]
fn source_documentation_keeps_schema_and_root_special_cards_only() {
    let source = "source lib.tcl\nset value 1\nputs $value\nputs $::env(PATH)\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let set = source_documentation_hover(source, 1, 1, &analysis).unwrap();
    let puts = source_documentation_hover(source, 2, 1, &analysis).unwrap();
    let special = source_documentation_hover(source, 3, 9, &analysis).unwrap();
    assert!(set.value.contains("**`set`**"));
    assert!(puts.value.contains("**`puts`**"));
    assert!(puts.value.contains("command availability is unresolved"));
    assert!(special.value.contains("env"));
    assert!(
        special
            .value
            .contains("binding, availability and the current value are unresolved")
    );
    assert!(source_documentation_hover(source, 2, 8, &analysis).is_none());
    assert!(
        source_documentation_hover(&source.replace("value 1", "value 2"), 1, 1, &analysis)
            .is_none()
    );
}

#[test]
fn source_documentation_withholds_known_shadow_and_local_variable_metadata() {
    let source = "proc puts args {}\nputs 7\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    assert!(source_documentation_hover(source, 1, 1, &analysis).is_none());
    let source = "proc p {env} {puts $env(PATH)}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let cursor = u32::try_from(source.find("env(PATH)").unwrap() + 1).unwrap();
    assert!(source_documentation_hover(source, 0, cursor, &analysis).is_none());
    let mut unavailable = analysis.clone();
    unavailable.resolved_input = None;
    assert!(source_documentation_hover(source, 0, cursor, &unavailable).is_none());
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    let source = "set ::auto_index(opaque) {source lib.tcl}; opaque\nputs 7";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let hover = source_documentation_hover(source, 1, 1, &analysis).unwrap();
    assert!(hover.value.contains("Source documentation"));
    assert!(hover.value.contains("command availability is unresolved"));
}
#[test]
fn conditional_native_formal_hover_requires_real_source_formal_owner() {
    let source = "proc ::tk::SourceLibFile {file} {\n    namespace eval :: [list source [file join $::tk_library $file.tcl]]\n}\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let declaration = analysis.original_procedure_declarations().next().unwrap();
    assert!(
        analysis
            .original_procedure_formal_name_sources(
                declaration,
                analysis.resolved_registry().unwrap()
            )
            .is_none()
    );
    let mut missing_input = analysis.clone();
    missing_input.resolved_input = None;
    assert!(conditional_native_formal_hover(source, &missing_input, 26).is_none());
    let foreign = Analyser::new().analyse(source, "tcl8.6");
    let foreign_declaration = foreign.original_procedure_declarations().next().unwrap();
    assert!(
        analysis
            .conditional_native_procedure_formal_name_sources(
                foreign_declaration,
                analysis.resolved_registry().unwrap()
            )
            .is_none()
    );
    let other = Analyser::new().analyse(source, "tcl9.0");
    assert!(
        analysis
            .conditional_native_procedure_formal_name_sources(
                declaration,
                other.resolved_registry().unwrap()
            )
            .is_none()
    );
    let card =
        hover(source, 0, 26, &analysis, None).expect("conditional original parameter source");
    assert!(card.value.contains("Parameter") && card.value.contains("file"));
    assert!(
        card.value
            .contains("runtime binding and value are unresolved")
    );
    assert!(
        conditional_native_formal_hover(&format!("{source}# changed"), &analysis, 26).is_none()
    );
    let simple = "proc greetPerson {name {greeting hello}} {return $name}";
    let analysis = Analyser::new().analyse(simple, "tcl8.6");
    assert!(
        conditional_native_formal_hover(simple, &analysis, 19)
            .unwrap()
            .value
            .contains("name")
    );
    let logical = Analyser::new().analyse(simple, "tcl");
    assert!(conditional_native_formal_hover(simple, &logical, 19).is_none());
    let dynamic = "proc p $formals {return VALUE}";
    let analysis = Analyser::new().analyse(dynamic, "tcl8.6");
    assert!(conditional_native_formal_hover(dynamic, &analysis, 9).is_none());
    let overridden = "proc proc {args} {}; proc p {x} {return VALUE}";
    let analysis = Analyser::new().analyse(overridden, "tcl8.6");
    let position = u32::try_from(overridden.rfind("{x}").unwrap() + 1).unwrap();
    assert!(conditional_native_formal_hover(overridden, &analysis, position).is_none());
}
