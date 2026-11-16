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
fn snapshot_reads_owned_cells_before_rendering_native_name_bytes() {
    let mut vm = Vm::new();
    let globals = vm.debug_snapshot(6, "before").variables;
    vm.push_call_frame(Some("inspection".into()), Vec::new());
    let array = VarBinding {
        owner: VarTableOwner::Frame(vm.current_level()),
        name: NameBytes::from(b"a".as_slice()),
    };
    vm.bind_new_var(&array, Local::Array(VarTable::new()));
    for (name, value) in [
        (b"a(k)".as_slice(), 11),
        (b"::local", 12),
        (b"", 13),
        (b"opaque\xff", 14),
        (b"opaque\\u00ff", 15),
    ] {
        vm.set_local_bytes(name, Value::int(value));
    }
    let snapshot = vm.debug_snapshot(7, "next");
    let mut variables: Vec<_> = snapshot
        .variables
        .iter()
        .map(|variable| (variable.name.as_str(), variable.value.as_str()))
        .collect();
    variables.sort_unstable();
    assert_eq!(
        variables,
        vec![
            ("", "13"),
            ("::local", "12"),
            ("a", ""),
            ("a(k)", "11"),
            ("opaque\\u00ff", "14"),
            ("opaque\\u00ff", "15"),
        ]
    );
    assert_eq!(snapshot.level, 1);
    assert_eq!(snapshot.stack[0].name, "inspection");
    vm.pop_call_frame();
    assert_eq!(vm.debug_snapshot(8, "after").variables, globals);
}
