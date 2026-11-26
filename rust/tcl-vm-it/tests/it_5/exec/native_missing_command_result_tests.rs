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

//! A command missing from the release reports an invalid command name.

use tcl_vm::{Code, Value};

#[test]
fn a_missing_lmap_reports_an_invalid_command_name() {
    // Native proof: naming.each-loop.original-lmap-collection
    // docs/design/analysis/name-resolution-proofs/each-loop-original-lmap-collection.md
    // Original native_each_loop case4 records the result String window after
    // this absent command; no loop body is entered on these two providers.
    for engine in ["tcl8.4", "tcl8.5"] {
        let profile = tcl_vm::test_support::profile(engine);
        let mut vm = crate::fixture::interpreter(profile);
        let strings = tcl_registry::InvocationDialect::of_static_profile(profile)
            .native_string_protocol()
            .unwrap();
        let arguments = [
            Value::native_list_constructor(
                vec![Value::new_native_string_bytes(b"v".as_slice())],
                strings,
            ),
            Value::native_list_constructor(Vec::new(), strings),
            Value::new_native_string_bytes(b"{".as_slice()),
        ];
        let completion = vm.try_invoke_command("lmap", &arguments).unwrap();
        assert_eq!(completion.code, Code::Error, "{engine}");
        assert_eq!(
            vm.native_name_operand_bytes(&completion.result)
                .unwrap()
                .as_ref(),
            b"invalid command name \"lmap\"",
            "{engine}"
        );
    }
}
