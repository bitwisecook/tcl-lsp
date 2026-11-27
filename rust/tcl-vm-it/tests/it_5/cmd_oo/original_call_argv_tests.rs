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

//! The words a `TclOO` constructor, method, `next` and destructor frame report, and `info errorstack`.

use crate::prelude::*;
use std::cell::RefCell;
use tcl_runtime_api::Introspect;

type Frames = Rc<RefCell<Vec<Vec<Vec<u8>>>>>;

struct Observe(Frames);
impl tcl_vm::NativeCommand for Observe {
    fn invoke(&self, vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
        let level = Introspect::level(vm);
        let words = vm.level_argv(level).unwrap().unwrap_or_else(Value::empty);
        self.0.borrow_mut().push(words_of(vm, &words));
        Completion::new(Code::Ok, Value::empty(), Value::empty())
    }
}
fn words_of(vm: &Vm, list: &Value) -> Vec<Vec<u8>> {
    list.as_list()
        .unwrap()
        .iter()
        .map(|word| crate::fixture::bytes(vm, word))
        .collect()
}
fn instance(version: tcl_dialect::TclVersion, setup: &str) -> (Vm, Frames) {
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(
        version.dialect_profile_name(),
    ));
    let observed = Rc::new(RefCell::new(Vec::new()));
    vm.register_native_command("observe_original", Rc::new(Observe(Rc::clone(&observed))));
    let result = crate::fixture::eval(&mut vm, setup);
    assert_eq!(result.code, Code::Ok, "{:?}", result.result.string_bytes());
    (vm, observed)
}
fn calls(vm: &mut Vm) -> Vec<Vec<Vec<u8>>> {
    let stack = crate::fixture::call(vm, "info", &[Value::string("errorstack")]).result;
    stack
        .as_list()
        .unwrap()
        .as_chunks::<2>()
        .0
        .iter()
        .filter(|pair| pair[0].string_bytes().as_ref() == b"CALL")
        .map(|pair| words_of(vm, &pair[1]))
        .collect()
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
            let head = if constructor { "C" } else { "o" };
            let argument = Value::native_list_constructor(
                vec![Value::string("ORIGINAL")],
                tcl_syntax::native_string::NativeStringProtocol::C(version),
            );
            let arguments = if constructor {
                vec![Value::string("create"), Value::string("obj"), argument]
            } else {
                vec![Value::string("m"), argument]
            };
            let expected: Vec<_> = std::iter::once(head.as_bytes().to_vec())
                .chain(
                    arguments
                        .iter()
                        .map(|word| crate::fixture::bytes(&vm, word)),
                )
                .collect();
            let result = crate::fixture::call(&mut vm, head, &arguments);
            assert_eq!(result.code, Code::Error);
            assert_eq!(
                observed.borrow().as_slice(),
                std::slice::from_ref(&expected)
            );
            assert_eq!(calls(&mut vm), vec![expected]);
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
            let result = crate::fixture::call(&mut vm, "o", &[Value::string("m"), argument]);
            assert_eq!(result.code, Code::Error);
            let observed = observed.borrow();
            assert_eq!(observed.len(), 1);
            assert_eq!(
                observed[0].len(),
                if next.starts_with("nextto") { 3 } else { 2 }
            );
            assert_eq!(observed[0].last().unwrap(), b"ORIGINAL");
        }
        let (mut vm, observed) = instance(
            version,
            "oo::class create C {destructor {observe_original; error FAILURE}}; C create o",
        );
        let result = crate::fixture::call(&mut vm, "o", &[Value::string("destroy")]);
        assert_eq!(result.code, Code::Error);
        assert_eq!(observed.borrow().as_slice(), &[Vec::<Vec<u8>>::new()]);
        assert_eq!(calls(&mut vm), Vec::<Vec<Vec<u8>>>::new());
    }
}
