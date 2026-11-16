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
fn original_tcl8_variable_closer_exposes_real_suffix_structure() {
    let source = "${a{b}[x]}";
    let tcl8 = crate::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::FirstClose,
        ..crate::LexerConfig::default()
    };
    let tcl9 = crate::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::Tcl9Nesting,
        ..tcl8
    };
    let old = BracketIndex::build_with_config(source, tcl8);
    let modern = BracketIndex::build_with_config(source, tcl9);
    assert_eq!(old.events().len(), 2);
    assert_eq!(modern.events().len(), 0);
    let bracket = u32::try_from(source.find('[').unwrap()).unwrap();
    assert!(!old.is_inert(bracket));
    assert!(modern.is_inert(bracket));
    assert_eq!(BracketIndex::build(source).events(), modern.events());
}
