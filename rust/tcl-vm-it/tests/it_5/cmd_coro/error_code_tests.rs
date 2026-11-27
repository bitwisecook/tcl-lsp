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

//! Behaviour tests moved from `tcl-vm`'s `cmd_coro::error_code_tests`.

use crate::prelude::*;

#[test]
fn coroutine_errors_keep_the_interpreter_error_code() {
    // tclsh 8.6.18, 9.0.4 and 9.1.0 print exactly these lines.
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
        let mut vm = crate::fixture::interpreter(profile);
        for (script, expected) in [
            (
                "proc p {} {yield}; proc q {} {yieldto foo}; \
                 list [catch p m] $m $::errorCode [catch q m] $m $::errorCode",
                "1 {yield can only be called in a coroutine} {TCL COROUTINE ILLEGAL_YIELD} \
                 1 {yieldto can only be called in a coroutine} {TCL COROUTINE ILLEGAL_YIELD}",
            ),
            (
                "list [catch {coroutine c apply {{} {error x {} {A B}}}} m] $m $::errorCode",
                "1 x {A B}",
            ),
        ] {
            let completion = vm.try_eval_source(script).unwrap();
            assert_eq!(completion.code, Code::Ok, "{engine}");
            assert_eq!(
                vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                expected.as_bytes(),
                "{engine}"
            );
        }
    }
}
