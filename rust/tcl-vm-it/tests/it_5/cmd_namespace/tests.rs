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

//! `namespace eval` and `namespace inscope` frames and completions.

use crate::prelude::*;
use tcl_runtime_api::Introspect;

fn capture_namespace_frame(vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
    let level = Introspect::level(vm);
    let argv = vm.level_argv(level).unwrap().unwrap_or_else(Value::empty);
    Completion::new(Code::Ok, argv, Value::empty())
}

fn words(vm: &Vm, list: &Value) -> Vec<Vec<u8>> {
    list.native_object_list_elements(crate::fixture::strings(vm))
        .unwrap()
        .elements()
        .unwrap()
        .iter()
        .map(|word| crate::fixture::bytes(vm, word))
        .collect()
}

fn text(vm: &mut Vm, script: &str) -> Vec<u8> {
    let completion = crate::fixture::eval(vm, script);
    crate::fixture::bytes(vm, &completion.result)
}

fn invoke(vm: &mut Vm, command: Vec<Value>) -> Completion<Value> {
    let script = Value::native_list_constructor(command, crate::fixture::strings(vm));
    crate::fixture::call(vm, "eval", &[script])
}

#[test]
fn namespace_frames_retain_actual_public_and_worker_argument_objects() {
    for version in tcl_dialect::TclVersion::ALL {
        for member in ["eval", "inscope"] {
            let profile = tcl_vm::test_support::profile(version.dialect_profile_name());
            for worker in [false, true] {
                if worker
                    && matches!(
                        version,
                        tcl_dialect::TclVersion::V8_4 | tcl_dialect::TclVersion::V8_5
                    )
                {
                    continue;
                }
                let mut vm = crate::fixture::interpreter(profile);
                vm.register("captureNamespaceFrame", capture_namespace_frame);
                crate::fixture::eval(&mut vm, "namespace eval n {}");
                let head = Value::new_native_string_bytes(b"renamed namespace\xff".as_slice());
                let source = if worker {
                    format!("::tcl::namespace::{member}")
                } else {
                    "namespace".to_owned()
                };
                let renamed =
                    crate::fixture::call(&mut vm, "rename", &[Value::string(source), head.clone()]);
                assert_eq!(renamed.code, Code::Ok, "{version:?}/{member}/{worker}");
                let mut command = vec![head.clone()];
                if !worker {
                    command.push(Value::string(if member == "eval" { "ev" } else { "insc" }));
                }
                command.push(Value::string("n"));
                command.push(Value::list(vec![Value::string("::captureNamespaceFrame")]));
                let expected: Vec<_> = command
                    .iter()
                    .map(|word| crate::fixture::bytes(&vm, word))
                    .collect();
                let completion = invoke(&mut vm, command);
                assert_eq!(
                    completion.code,
                    Code::Ok,
                    "{version:?}/{member}/{worker}: {:?}",
                    completion.result.string_bytes()
                );
                let returned = words(&vm, &completion.result);
                if version == tcl_dialect::TclVersion::V8_4 && member == "inscope" {
                    assert_eq!(returned, Vec::<Vec<u8>>::new());
                } else {
                    assert_eq!(returned, expected, "{version:?}/{member}/{worker}");
                }
                let current = if worker {
                    crate::fixture::call(&mut vm, "namespace", &[Value::string("current")])
                } else {
                    invoke(&mut vm, vec![head, Value::string("current")])
                };
                assert_eq!(crate::fixture::bytes(&vm, &current.result), b"::");
            }
        }
    }
}

#[test]
fn jim_namespace_frames_retain_the_actual_eval_and_scripted_tailcall_vectors() {
    let mut vm = crate::fixture::distribution(tcl_vm::test_support::profile("jimtcl"));
    vm.register("captureNamespaceFrame", capture_namespace_frame);
    let body = Value::list(vec![Value::string("::captureNamespaceFrame")]);
    let completion = invoke(
        &mut vm,
        vec![
            Value::string("namespace"),
            Value::string("eval"),
            Value::string("n"),
            body.clone(),
        ],
    );
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(
        words(&vm, &completion.result),
        [
            b"namespace".as_slice(),
            b"eval",
            b"n",
            b"::captureNamespaceFrame"
        ]
    );
    let inscope = invoke(
        &mut vm,
        vec![
            Value::string("namespace"),
            Value::string("inscope"),
            Value::string("n"),
            body,
        ],
    );
    assert_eq!(inscope.code, Code::Ok);
    // The original distribution helper tailcalls namespace eval with its
    // actual args List as a fourth operand; its caller head is not retained.
    assert_eq!(
        words(&vm, &inscope.result),
        [
            b"namespace".as_slice(),
            b"eval",
            b"n",
            b"::captureNamespaceFrame",
            b""
        ]
    );
}

#[test]
fn native_namespace_bodies_propagate_return_and_inscope_requires_c_target() {
    // tclsh 8.6.18 and 9.0.4, and jimsh 0.84, give each result.
    for version in tcl_dialect::TclVersion::ALL {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(
            version.dialect_profile_name(),
        ));
        assert_eq!(
            text(
                &mut vm,
                "list [catch {namespace eval n {return x}} r] $r [namespace current] [catch {namespace inscope n {return x}} r] $r [catch {namespace inscope absent {return y}} r] [string match *absent* $r] [namespace exists absent]"
            ),
            b"2 x :: 2 x 1 1 0",
            "{version:?}"
        );
    }
    let mut vm = crate::fixture::distribution(tcl_vm::test_support::profile("jim"));
    assert_eq!(
        text(
            &mut vm,
            "list [catch {namespace eval n {return x}} r] $r [namespace current] [catch {namespace inscope n {return x}} r] $r [catch {namespace inscope absent {return y}} r] $r"
        ),
        b"2 x :: 2 x 2 y"
    );
}
