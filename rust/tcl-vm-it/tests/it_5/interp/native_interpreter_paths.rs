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
    use tcl_runtime_api::{Code, Completion};
    use tcl_vm::{Value, Vm};

    fn call(vm: &mut Vm, subcommand: &[u8], arguments: &[Value]) -> Completion<Value> {
        let mut words = vec![Value::new_native_string_bytes(subcommand)];
        words.extend_from_slice(arguments);
        vm.try_invoke_command("interp", &words)
            .expect("no host refusal")
    }

    fn exists(vm: &mut Vm, path: &Value) -> bool {
        let answer = call(vm, b"exists", std::slice::from_ref(path));
        assert_eq!(answer.code, Code::Ok);
        answer.result.string_bytes().as_ref() == b"1"
    }

    #[test]
    fn original_interpreter_paths_keep_counted_objects_and_child_cstring_keys() {
        // Native proof: naming.interpreter.counted-list-path-cstring-child-keys
        // docs/design/analysis/name-resolution-proofs/interpreter-counted-list-path-cstring-child-keys.md
        // Native proof: naming.interpreter.singleton-create-original-spelling
        // docs/design/analysis/name-resolution-proofs/interpreter-singleton-create-original-spelling.md
        // Native proof: naming.interpreter.malformed-path-list-error
        // docs/design/analysis/name-resolution-proofs/interpreter-malformed-path-list-error.md
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
            for bytes in [
                b"child\0suffix".as_slice(),
                b"opaque\xff",
                b"surrogate\xed\xa0\x80",
                b"encoded\xc0\x80tail",
            ] {
                let path = Value::new_native_string_bytes(bytes);
                let created = call(&mut vm, b"create", std::slice::from_ref(&path));
                assert_eq!(created.code, Code::Ok, "{engine}: {created:?}");
                assert_eq!(created.result.string_bytes().as_ref(), bytes, "{engine}");
                assert!(exists(&mut vm, &path), "{engine}");
                let deleted = call(&mut vm, b"delete", std::slice::from_ref(&path));
                assert_eq!(deleted.code, Code::Ok);
                assert!(!exists(&mut vm, &path), "{engine}");
            }
            let original = Value::new_native_string_bytes(b"{with space}".as_slice());
            {
                let argument = original.clone();
                assert_eq!(
                    call(&mut vm, b"create", std::slice::from_ref(&argument)).code,
                    Code::Ok
                );
            }
            assert!(!exists(&mut vm, &original), "{engine}");
            let quoted = Value::new_native_string_bytes(b"{{with space}}".as_slice());
            assert!(exists(&mut vm, &quoted), "{engine}");
            let malformed = Value::new_native_string_bytes(b"{".as_slice());
            assert_eq!(call(&mut vm, b"create", &[malformed]).code, Code::Error);
        }
    }
}
