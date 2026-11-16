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
use std::cell::RefCell;
struct Observe(Rc<RefCell<Vec<Vec<usize>>>>);
impl crate::command::NativeCommand for Observe {
    fn invoke(&self, vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
        let words = vm.frame_argv(vm.current_level()).expect("active OO frame");
        self.0
            .borrow_mut()
            .push(words.iter().map(Value::native_object_identity).collect());
        ok(Value::empty())
    }
}
fn instance(version: tcl_dialect::TclVersion, setup: &str) -> (Vm, Rc<RefCell<Vec<Vec<usize>>>>) {
    let profile = crate::environment::profile_for_dialect(version.dialect_profile_name());
    let mut vm = Vm::with_native_core(
        Box::new(std::io::sink()),
        Rc::new(crate::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("selected actual C constructor");
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
    ));
    let observed = Rc::new(RefCell::new(Vec::new()));
    vm.register_command_in_slot(
        tcl_runtime_api::CommandSlot {
            namespace: tcl_core_types::ROOT_NS,
            simple: NameBytes::from(b"observe_original".as_slice()),
        },
        Command::Native(Rc::new(Observe(Rc::clone(&observed)))),
    );
    let result = vm.eval_source(setup).unwrap();
    assert_eq!(result.code, Code::Ok, "{:?}", result.result.string_bytes());
    (vm, observed)
}
#[test]
fn constructor_and_method_frames_borrow_original_dispatch_children() {
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        for constructor in [false, true] {
            let setup = if constructor {
                "oo::class create C {constructor {x} {observe_original; error FAILURE}}"
            } else {
                "oo::class create C {method m {x} {observe_original; error FAILURE}}; C create o"
            };
            let (mut vm, observed) = instance(version, setup);
            let head =
                Value::new_native_string_bytes(if constructor { &b"C"[..] } else { &b"o"[..] });
            let argument = Value::native_list_constructor(
                vec![Value::string("ORIGINAL")],
                tcl_syntax::native_string::NativeStringProtocol::C(version),
            );
            let arguments = if constructor {
                vec![Value::string("create"), Value::string("obj"), argument]
            } else {
                vec![Value::string("m"), argument]
            };
            let expected: Vec<_> = std::iter::once(&head)
                .chain(&arguments)
                .map(Value::native_object_identity)
                .collect();
            let result = vm.invoke_host_original_object_vector(&head, &arguments);
            assert_eq!(result.code, Code::Error);
            assert_eq!(
                observed.borrow().as_slice(),
                std::slice::from_ref(&expected)
            );
            let stack = vm.error_stack_value();
            let flat = stack.as_list().unwrap();
            let calls: Vec<_> = flat
                .as_chunks::<2>()
                .0
                .iter()
                .filter(|pair| pair[0].string_bytes().as_ref() == b"CALL")
                .map(|pair| {
                    pair[1]
                        .as_list()
                        .unwrap()
                        .iter()
                        .map(Value::native_object_identity)
                        .collect::<Vec<_>>()
                })
                .collect();
            assert_eq!(calls, vec![expected]);
        }
    }
}
#[test]
fn next_nextto_and_destructor_frames_use_native_invocation_vectors() {
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        for next in ["next $x", "nextto Base $x"] {
            let setup = format!(
                "oo::class create Base {{method m {{x}} {{observe_original; error FAILURE}}}}; oo::class create Derived {{superclass Base; method m {{x}} {{{next}}}}}; Derived create o"
            );
            let (mut vm, observed) = instance(version, &setup);
            let argument = Value::native_list_constructor(
                vec![Value::string("ORIGINAL")],
                tcl_syntax::native_string::NativeStringProtocol::C(version),
            );
            let identity = argument.native_object_identity();
            let result = vm.invoke_host_original_object_vector(
                &Value::string("o"),
                &[Value::string("m"), argument],
            );
            assert_eq!(result.code, Code::Error);
            let observed = observed.borrow();
            assert_eq!(observed.len(), 1);
            assert_eq!(
                observed[0].len(),
                if next.starts_with("nextto") { 3 } else { 2 }
            );
            assert_eq!(observed[0].last(), Some(&identity));
        }
        let (mut vm, observed) = instance(
            version,
            "oo::class create C {destructor {observe_original; error FAILURE}}; C create o",
        );
        let result =
            vm.invoke_host_original_object_vector(&Value::string("o"), &[Value::string("destroy")]);
        assert_eq!(result.code, Code::Error);
        assert_eq!(observed.borrow().as_slice(), &[Vec::<usize>::new()]);
        let stack = vm.error_stack_value();
        assert!(
            stack
                .as_list()
                .unwrap()
                .as_chunks::<2>()
                .0
                .iter()
                .all(|pair| pair[0].string_bytes().as_ref() != b"CALL")
        );
    }
}
