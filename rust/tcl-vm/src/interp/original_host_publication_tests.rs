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

fn worker(_: &mut Vm, _: &[Value]) -> Completion<Value> {
    ok(Value::empty())
}

#[test]
fn host_cstring_publication_uses_global_bare_and_actual_qualified_holder_slots() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(engine));
        vm.declare_namespace("::outer");
        let outer = vm.namespace_token_for_written("::outer").unwrap();
        vm.push_ns_token(NameBytes::from(b"outer".as_slice()), outer);
        let key = vm
            .register_command("bare", Command::Builtin(worker))
            .into_key()
            .unwrap();
        let slot = vm.command_slot(&key).unwrap();
        assert_eq!(slot.namespace, ROOT_NS);
        assert_eq!(slot.simple.as_bytes(), b"bare");
        let key = vm
            .register_command("child::qualified\0ignored", Command::Builtin(worker))
            .into_key()
            .unwrap();
        let slot = vm.command_slot(&key).unwrap();
        let namespace = vm
            .namespace_descendant_token(outer, &[NameBytes::from(b"child".as_slice())])
            .unwrap();
        assert_eq!(slot.namespace, namespace);
        assert_eq!(slot.simple.as_bytes(), b"qualified");
        let key = vm
            .register_command("::rooted", Command::Builtin(worker))
            .into_key()
            .unwrap();
        let slot = vm.command_slot(&key).unwrap();
        assert_eq!(slot.namespace, ROOT_NS);
        assert_eq!(slot.simple.as_bytes(), b"rooted");
        vm.pop_ns();
    }
}
