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

//! `array set` and `array for` against the original getter-order captures.

use crate::prelude::*;
#[cfg(debug_assertions)]
use std::cell::Cell;

const ORIGINALS: &[(&str, &str)] = &[
    (
        "tcl8.4",
        include_str!(
            "../../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/tcl8.4/execute.stdout"
        ),
    ),
    (
        "tcl8.5",
        include_str!(
            "../../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/tcl8.5/execute.stdout"
        ),
    ),
    (
        "tcl8.6",
        include_str!(
            "../../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/tcl8.6/execute.stdout"
        ),
    ),
    (
        "tcl9.0",
        include_str!(
            "../../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/tcl9.0/execute.stdout"
        ),
    ),
    (
        "tcl9.1",
        include_str!(
            "../../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/tcl9.1/execute.stdout"
        ),
    ),
    (
        "jim",
        include_str!(
            "../../../../../runtime/rust/tests/data/native_variable_introspection_getter_order_original/505b/providers/jim/execute.stdout"
        ),
    ),
];
fn native(version: &str) -> Vm {
    crate::fixture::interpreter(tcl_vm::test_support::profile(version))
}

fn bytes(value: &[u8]) -> Value {
    Value::from_string_bytes(value)
}

fn array_op(vm: &mut Vm, subcommand: &str, args: &[Value]) -> Completion<Value> {
    let mut words = vec![Value::string(subcommand)];
    words.extend_from_slice(args);
    vm.try_invoke_command("array", &words)
        .unwrap_or_else(|refusal| panic!("array {subcommand}: {refusal:?}"))
}

fn compare_completion(vm: &mut Vm, completion: &Completion<Value>, capture: &str, case: usize) {
    let row: Vec<_> = capture
        .lines()
        .find(|row| row.starts_with(&format!("ROW|{case}|worker|")))
        .unwrap()
        .split('|')
        .collect();
    assert_eq!(completion.code.as_int(), row[3].parse::<i64>().unwrap());
    assert_eq!(
        vm.native_string_bytes(&completion.result).unwrap().as_ref(),
        crate::fixture::unhex(row[4])
    );
}

#[test]
fn original_vm_array_set_matches_bounded_successful_public_rows() {
    for &(version, capture) in ORIGINALS {
        for case in [4, 6] {
            let mut vm = native(version);
            let key = bytes(b"k");
            let second = bytes(if case == 4 { b"VALUE" } else { b"m" });
            let members = if case == 4 {
                vec![key, second]
            } else {
                vec![key, bytes(b"FIRST"), second, bytes(b"SECOND")]
            };
            let completion = array_op(&mut vm, "set", &[bytes(b"::arr"), Value::list(members)]);
            compare_completion(&mut vm, &completion, capture, case);
        }
    }
}

#[test]
fn original_vm_jim_array_set_observes_original_root_before_cow() {
    for shared in [false, true] {
        let mut vm = native("jim");
        let script = if shared {
            "set arr {old OLD}; set external $arr; array set arr {new NEW}; list [array size arr] [dict size $external]"
        } else {
            "set arr {old OLD}; array set arr {new NEW}; list [array size arr] 1"
        };
        let completion = crate::fixture::eval(&mut vm, script);
        assert_eq!(completion.code, Code::Ok, "shared={shared}");
        assert_eq!(
            completion.result.to_str().as_ref(),
            "2 1",
            "shared={shared}"
        );
    }
}

#[test]
fn original_vm_array_for_keeps_the_actual_table_key_header() {
    for &(version, capture) in &ORIGINALS[3..5] {
        let mut vm = native(version);
        let name = bytes(b"::arr");
        assert_eq!(
            array_op(
                &mut vm,
                "set",
                &[
                    name.clone(),
                    Value::list(vec![bytes(b"k"), bytes(b"VALUE")])
                ]
            )
            .code,
            Code::Ok
        );
        let variables = Value::list(vec![bytes(b"::k"), bytes(b"::v")]);
        let completion = array_op(&mut vm, "for", &[variables, name, bytes(b"")]);
        compare_completion(&mut vm, &completion, capture, 7);
        assert_eq!(
            vm.get_var_bytes(b"k").unwrap().string_bytes().as_ref(),
            b"k"
        );
        assert_eq!(
            vm.get_var_bytes(b"v").unwrap().string_bytes().as_ref(),
            b"VALUE"
        );
    }
}

#[cfg(debug_assertions)]
struct ContextTrace {
    mode: u8,
    calls: Rc<Cell<usize>>,
}

#[cfg(debug_assertions)]
impl tcl_vm::NativeCommand for ContextTrace {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        vm.set_var_bytes(b"reached", bytes(b"REACHED")).unwrap();
        if self.mode == 1 {
            let _ = vm.refuse_host_command("first actual array observer".into());
        } else if self.mode == 2 {
            let original = vm.runtime_context().clone();
            let mut changed = original.clone();
            changed.packages = vec![("actual-array-context".into(), "1".into())];
            vm.pin_context(&changed).unwrap();
            vm.pin_context(&original).unwrap();
        }
        Completion::new(Code::Ok, bytes(b"LATER_GUEST"), Value::empty())
    }
}

#[cfg(debug_assertions)]
fn traced(mode: u8) -> (Vm, Rc<Cell<usize>>) {
    let mut vm = native("tcl9.1");
    assert_eq!(
        array_op(
            &mut vm,
            "set",
            &[
                bytes(b"arr"),
                Value::list(vec![bytes(b"old"), bytes(b"OLD")])
            ]
        )
        .code,
        Code::Ok
    );
    let calls = Rc::new(Cell::new(0));
    vm.register_native_command(
        "context_trace",
        Rc::new(ContextTrace {
            mode,
            calls: calls.clone(),
        }),
    );
    crate::fixture::eval(&mut vm, "trace add variable arr array context_trace");
    (vm, calls)
}

#[cfg(debug_assertions)]
#[test]
fn original_vm_array_observer_refusal_stops_list_conversion_and_releases_receiver() {
    let (mut vm, calls) = traced(1);
    let first = vm
        .try_invoke_command("array", &[bytes(b"set"), bytes(b"arr"), bytes(b"{")])
        .unwrap_err();
    assert!(matches!(
        first,
        tcl_runtime_api::NativeExecutionError::HostCommandRefusal(_)
    ));
    assert_eq!(calls.get(), 1);
    assert_eq!(
        vm.get_var_bytes(b"reached")
            .unwrap()
            .string_bytes()
            .as_ref(),
        b"REACHED"
    );
}

#[cfg(debug_assertions)]
#[test]
fn an_array_observer_that_pins_another_context_is_not_a_host_refusal() {
    let (mut vm, calls) = traced(2);
    let malformed = array_op(&mut vm, "set", &[bytes(b"arr"), bytes(b"{")]);
    assert_eq!(
        malformed.code,
        Code::Error,
        "the malformed list is the guest's error"
    );
    assert_eq!(calls.get(), 1);
    let later = array_op(
        &mut vm,
        "set",
        &[
            bytes(b"arr"),
            Value::list(vec![bytes(b"later"), bytes(b"LATER")]),
        ],
    );
    assert_eq!(later.code, Code::Ok);
    assert_eq!(calls.get(), 2);
}
