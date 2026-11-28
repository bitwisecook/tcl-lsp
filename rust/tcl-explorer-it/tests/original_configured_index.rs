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

use tcl_explorer::{run_pipeline, serialise_result};

mod support;

#[test]
fn original_structural_completeness_keeps_module_input_and_unknown_null() {
    support::install_surfaces();
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Explorer source display; fixed C9 structural indexes remain separate.
    let mut result = run_pipeline("set x ${a{}", "tcl9.1");
    result.unit.ir_module.lexer_config.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
    assert_eq!(
        serialise_result(&result)["structuralIndex"]["scriptComplete"],
        true
    );
    result.unit.ir_module.lexer_config.braced_var = tcl_lexer::BracedVarStyle::Tcl9Nesting;
    assert_eq!(
        serialise_result(&result)["structuralIndex"]["scriptComplete"],
        false
    );
    result.source.push_str("foreign");
    assert!(serialise_result(&result)["structuralIndex"]["scriptComplete"].is_null());
    let mut jim = run_pipeline("set x 1", "jim");
    jim.unit.ir_module.lexer_config.braced_var = tcl_lexer::BracedVarStyle::Tcl9Nesting;
    assert!(serialise_result(&jim)["structuralIndex"]["scriptComplete"].is_null());
}
