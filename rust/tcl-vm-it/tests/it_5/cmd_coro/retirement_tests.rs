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

//! Behaviour tests moved from `tcl-vm`'s `cmd_coro::retirement_tests`.

use crate::prelude::*;

#[test]
fn yieldto_retains_deleted_entered_namespace_when_its_spelling_is_recreated() {
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for relay in ["yieldto target", "$relay target"] {
            let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
            let mut vm = crate::fixture::interpreter(profile);
            let source = format!(
                "namespace eval N {{proc generator {{relay}} {{\
                 namespace delete ::N; namespace eval ::N {{proc target {{}} {{return NEW}}}}; \
                 {relay}}}}}; \
                 set status [catch {{coroutine c N::generator yieldto}} message]; \
                 list $status $message $::errorCode"
            );
            let completion = vm.try_eval_source(&source).unwrap();
            assert_eq!(completion.code, Code::Ok, "{engine}/{relay}");
            assert_eq!(
                vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                b"1 {yieldto called in deleted namespace} {TCL COROUTINE YIELDTO_IN_DELETED}",
                "{engine}/{relay}"
            );
        }
    }
}
