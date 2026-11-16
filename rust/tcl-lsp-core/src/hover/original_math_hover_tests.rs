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
fn original_math_hover_keeps_fixed_registration_without_a_fake_wrapper() {
    // Implementation contract: naming.core.original-math-function-hover
    // docs/design/analysis/name-resolution-proofs/core-original-math-function-hover.md
    let source = "expr {abs(-2)}\n";
    for dialect in ["tcl8.4", "jim"] {
        let mut analysis = Analyser::new().analyse(source, dialect);
        analysis.command_invocations.clear();
        let offset = u32::try_from(source.find("abs").unwrap()).unwrap();
        let std::ops::ControlFlow::Break(Some(selected)) =
            crate::math_function_symbol::select_at_offset(source, &analysis, offset)
        else {
            panic!("{dialect}: missing fixed function");
        };
        let hover = original_math_function_hover(&selected).unwrap();
        assert!(
            hover.value.contains("function table entry"),
            "{}",
            hover.value
        );
        assert!(!hover.value.contains("Dispatches to"));
        assert!(!hover.value.contains("Command-table lookup"));
        assert!(!hover.value.contains("tcl::mathfunc"));
    }
}

#[test]
fn original_math_hover_uses_the_actual_current_source_and_override() {
    // Implementation contract: naming.core.original-math-function-hover
    // docs/design/analysis/name-resolution-proofs/core-original-math-function-hover.md
    let source = "# Actual override\nproc ::tcl::mathfunc::abs {argument} {return OVERRIDE}\nexpr {abs(-2)}\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.all_procs.clear();
    analysis.command_invocations.clear();
    let offset = u32::try_from(source.rfind("abs(").unwrap()).unwrap();
    let std::ops::ControlFlow::Break(Some(selected)) =
        crate::math_function_symbol::select_at_offset(source, &analysis, offset)
    else {
        panic!("missing actual override");
    };
    let hover = original_math_function_hover(&selected).unwrap();
    assert!(hover.value.contains("argument"));
    assert!(hover.value.contains("Actual override"));
    assert!(!hover.value.contains("Command-table lookup"));
    assert!(matches!(
        crate::math_function_symbol::select_at_offset(
            &format!("# changed\n{source}"),
            &analysis,
            offset
        ),
        std::ops::ControlFlow::Break(None)
    ));
}
