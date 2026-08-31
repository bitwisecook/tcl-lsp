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

//! Version-matched `expr` vectors for the bytecode VM.
//!
//! These call the VM's expression boundary directly so the vectors cover both
//! `expr` command evaluation and bytecode `EXPR_STK` callers without requiring
//! a compiler service. The LSP and VS Code layers are intentionally
//! inapplicable: this is runtime execution behaviour, while their static
//! version diagnostics already consume the same registry profile data.

use tcl_dialect::TclVersion;
use tcl_vm::Vm;

#[test]
fn boolean_word_prefixes_are_literals_in_every_release() {
    for version in TclVersion::ALL {
        let mut vm = Vm::new();
        vm.set_runtime_version(version);

        for source in ["t", "tru", "y", "ye", "f", "n", "of"] {
            assert_eq!(
                vm.eval_expr(source)
                    .unwrap_or_else(|error| panic!("{version} expr {{{source}}}: {error:?}"))
                    .to_str()
                    .as_ref(),
                source,
                "{version} expr {{{source}}}"
            );
        }
        assert_eq!(
            vm.eval_expr("tru ? yes : no")
                .expect("unique prefix is true in a boolean context")
                .to_str()
                .as_ref(),
            "yes",
            "{version}"
        );
        assert_eq!(
            vm.eval_expr("!of")
                .expect("unique false prefix under unary not")
                .to_str()
                .as_ref(),
            "1",
            "{version}"
        );

        let ambiguous = vm
            .eval_expr("o")
            .expect_err("the shared on/off prefix remains ambiguous");
        assert_eq!(
            ambiguous.error_code.as_deref(),
            Some("TCL PARSE EXPR BAREWORD"),
            "{version}"
        );
    }
}

#[test]
fn tip461_and_tip521_follow_the_registry_selected_runtime_release() {
    let mut old = Vm::new();
    old.set_runtime_version(TclVersion::V8_6);

    // FN: Tcl 8.6 must not accept Tcl 9's string-ordering grammar.
    let operator = old
        .eval_expr("{a} lt {b}")
        .expect_err("Tcl 8.6 rejects the 9.0-only operator");
    assert_eq!(
        operator.error_code.as_deref(),
        Some("TCL PARSE EXPR BAREWORD")
    );
    assert_eq!(
        operator.message,
        "invalid bareword \"lt\"\nin expression \"{a} lt {b}\";\nshould be \"$lt\" or \"{lt}\" or \"lt(...)\" or ..."
    );

    // FN: Tcl 8.6 likewise lacks the 9.0 builtin while keeping the namespace
    // open for an eventual user-provided replacement.
    let function = old
        .eval_expr("isfinite(1.0)")
        .expect_err("Tcl 8.6 has no built-in isfinite");
    assert_eq!(
        function.error_code.as_deref(),
        Some("TCL LOOKUP COMMAND tcl::mathfunc::isfinite")
    );
    assert_eq!(
        function.message,
        "invalid command name \"tcl::mathfunc::isfinite\""
    );

    // TN: a pre-existing grammar feature must not be rejected by the new
    // release gate.
    assert_eq!(
        old.eval_expr("2 + 3")
            .expect("base expression")
            .to_str()
            .as_ref(),
        "5"
    );

    let mut modern = Vm::new();
    modern.set_runtime_version(TclVersion::V9_0);

    // TP: both features are accepted at their 9.0 floor.
    assert_eq!(
        modern
            .eval_expr("{a} lt {b}")
            .expect("9.0 string ordering")
            .to_str()
            .as_ref(),
        "1"
    );
    assert_eq!(
        modern
            .eval_expr("isfinite(1.0)")
            .expect("9.0 math function")
            .to_str()
            .as_ref(),
        "1"
    );

    // FP: the function's normal false result is still distinguishable from a
    // version rejection.
    assert_eq!(
        modern
            .eval_expr("isfinite(1.0 / 0.0)")
            .expect("9.0 classification")
            .to_str()
            .as_ref(),
        "0"
    );
}
