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

use super::{
    DefinitionBodyGrammar, ITCL_GRAMMAR, SNIT_GRAMMAR, SNIT_WIDGET_GRAMMAR, TCLOO_GRAMMAR,
};

#[test]
fn receiver_variables_require_the_explicit_role_and_implicit_binding() {
    const CUSTOM: DefinitionBodyGrammar = DefinitionBodyGrammar {
        implicit_vars: &["receiver", "options"],
        implicit_receiver_vars: &["receiver", "hidden"],
        ..ITCL_GRAMMAR
    };
    for grammar in [&SNIT_GRAMMAR, &SNIT_WIDGET_GRAMMAR] {
        assert!(grammar.is_implicit_receiver_variable("self"));
        for name in ["selfns", "type", "options", "win", "hull", "this"] {
            assert!(!grammar.is_implicit_receiver_variable(name));
        }
    }
    assert!(ITCL_GRAMMAR.is_implicit_receiver_variable("this"));
    assert!(!ITCL_GRAMMAR.is_implicit_receiver_variable("self"));
    assert!(!TCLOO_GRAMMAR.is_implicit_receiver_variable("self"));
    assert!(CUSTOM.is_implicit_receiver_variable("receiver"));
    for name in ["options", "hidden", "this", "self"] {
        assert!(!CUSTOM.is_implicit_receiver_variable(name));
    }
}
