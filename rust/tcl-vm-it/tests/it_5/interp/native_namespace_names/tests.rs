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

//! Qualified `info commands` and `info vars` patterns, per release.

use crate::prelude::*;

#[test]
fn qualified_inventory_keeps_existing_positive_command_and_variable_results() {
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(release));
        assert_eq!(
            crate::fixture::eval(
                &mut vm,
                "namespace eval selected {proc marker {} {}; variable item 1}"
            )
            .code,
            tcl_runtime_api::Code::Ok
        );
        let commands = crate::fixture::call(
            &mut vm,
            "info",
            &[Value::string("commands"), Value::string("::selected::*")],
        );
        assert_eq!(commands.code, tcl_runtime_api::Code::Ok, "{release}");
        assert!(
            commands
                .result
                .as_list()
                .unwrap()
                .iter()
                .any(|name| name.string_bytes().as_ref() == b"::selected::marker")
        );
        let variables = crate::fixture::call(
            &mut vm,
            "info",
            &[Value::string("vars"), Value::string("::selected::*")],
        );
        assert_eq!(variables.code, tcl_runtime_api::Code::Ok, "{release}");
        assert!(
            variables
                .result
                .as_list()
                .unwrap()
                .iter()
                .any(|name| name.string_bytes().as_ref() == b"::selected::item")
        );
    }
}
