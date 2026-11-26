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

mod tests {
    use tcl_core_types::Code;
    use tcl_syntax::value::ValueOps;
    use tcl_vm::{Value, Vm};

    fn actual_jim() -> Vm {
        crate::fixture::interpreter(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        )
    }

    fn defined(vm: &mut Vm, name: &[u8]) -> bool {
        let listed = vm
            .try_invoke_command(
                "info",
                &[
                    Value::string("commands"),
                    Value::from_native_string_bytes(name),
                ],
            )
            .unwrap();
        assert_eq!(listed.code, Code::Ok);
        !vm.native_string_bytes(&listed.result).unwrap().is_empty()
    }

    fn rename(vm: &mut Vm, from: &[u8], to: &[u8]) {
        let renamed = vm
            .try_invoke_command(
                "rename",
                &[
                    Value::from_native_string_bytes(from),
                    Value::from_native_string_bytes(to),
                ],
            )
            .unwrap();
        assert_eq!(renamed.code, Code::Ok);
    }

    #[test]
    fn opaque_jim_rename_moves_the_same_declaration_and_original_namespace_role() {
        let mut vm = actual_jim();
        let defined_proc = vm
            .try_invoke_command(
                "proc",
                &[
                    Value::from_native_string_bytes(b"old\xff".as_slice()),
                    Value::list(Vec::new()),
                    Value::from_native_string_bytes(b"return BODY".as_slice()),
                ],
            )
            .unwrap();
        assert_eq!(defined_proc.code, Code::Ok);
        rename(&mut vm, b"old\xff", b"n\xff::p\0z");
        assert!(!defined(&mut vm, b"old\xff"));
        assert!(defined(&mut vm, b"n\xff::p\0z"));
        rename(&mut vm, b"n\xff::p\0z", b"again\xff");
        assert!(!defined(&mut vm, b"n\xff::p\0z"));
        assert!(defined(&mut vm, b"again\xff"));
        rename(&mut vm, b"again\xff", b"");
        assert!(!defined(&mut vm, b"again\xff"));
    }
}
