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
fn a_jim_unit_is_built_as_jim_end_to_end() {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect("set x 1\n", &registry, false, "jim");
    assert_eq!(
        unit.ir_module.dialect.as_deref(),
        Some("jim"),
        "lowering hands codegen the document's own dialect name"
    );
    // (the unit does not expose its lexer config; the ingress test pins it)
    // And codegen, resolving that name, lands on Jim's grammar.
    let grammar = tcl_dialect::grammar_of_dialect_name(unit.ir_module.dialect.as_deref());
    assert_eq!(grammar.numbers, tcl_dialect::NumberSyntax::Jim080);
    assert_eq!(grammar.list_parse, tcl_dialect::ListParse::Lenient);
}

/// The alias spellings reach the same place.
#[test]
fn jim_aliases_build_the_same_unit() {
    let registry = CommandRegistry::build_default();
    for spelling in ["jimsh", "jimtcl"] {
        let unit = CompilationUnit::build_for_dialect("set x 1\n", &registry, false, spelling);
        assert_eq!(unit.ir_module.dialect.as_deref(), Some("jim"), "{spelling}");
    }
}
