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

//! Behaviour tests moved from `tcl-vm`'s `embed::procedure_definition_tests`.

use crate::prelude::*;

fn vm() -> Vm {
    let mut vm = tcl_vm::test_support::native_vm_with_output(Box::new(std::io::sink()));
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::default(),
    ));
    vm
}

#[test]
fn embedded_procedure_captures_its_allocated_namespace_before_compilation() {
    for name in ["root_body", "::spectcl::unit::body"] {
        let mut vm = vm();
        vm.try_define_procedure(name, &["words"], "return [llength $words]")
            .expect("actual definition namespace owns compilation");
        let result = vm
            .try_invoke_command(
                name,
                &[Value::list(vec![Value::string("a"), Value::string("b")])],
            )
            .expect("original body executes");
        assert_eq!(result.code, tcl_runtime_api::Code::Ok);
        assert_eq!(result.result.string_bytes().as_ref(), b"2");
        let qualified = crate::fixture::call(
            &mut vm,
            "namespace",
            &[
                Value::string("which"),
                Value::string("-command"),
                Value::string(name),
            ],
        );
        let expected = if name.starts_with("::") {
            name.to_owned()
        } else {
            format!("::{name}")
        };
        assert_eq!(qualified.result.to_str().as_ref(), expected);
    }
}

#[test]
fn embedded_procedure_recreated_namespace_gets_its_current_token() {
    let mut vm = vm();
    vm.try_define_procedure("::N::body", &[], "return OLD")
        .unwrap();
    assert_eq!(
        vm.try_eval_source("namespace delete ::N").unwrap().code,
        tcl_runtime_api::Code::Ok
    );
    vm.try_define_procedure("::N::body", &[], "return NEW")
        .unwrap();
    assert_eq!(
        crate::fixture::eval(&mut vm, "namespace exists ::N")
            .result
            .to_str()
            .as_ref(),
        "1"
    );
    let result = vm.try_invoke_command("::N::body", &[]).unwrap();
    assert_eq!(result.code, tcl_runtime_api::Code::Ok);
    assert_eq!(result.result.string_bytes().as_ref(), b"NEW");
}

#[test]
fn folded_eval_uses_final_caller_locals_and_propagates_child_control() {
    for (body, expected) in [
        (
            "eval {set later CHILD}; set later PARENT; return $later",
            "PARENT",
        ),
        (
            "set count 0; while {1} {incr count; eval {break}; incr count}; return $count",
            "1",
        ),
        ("eval {set literal [list CHILD]}; return $literal", "CHILD"),
    ] {
        let mut vm = vm();
        vm.try_define_procedure("::eval_boundary::p", &[], body)
            .unwrap();
        let completion = vm.try_invoke_command("::eval_boundary::p", &[]).unwrap();
        assert_eq!(completion.code, tcl_runtime_api::Code::Ok, "{body}");
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            expected.as_bytes(),
            "{body}"
        );
    }
}

#[test]
fn invalid_embedded_formals_do_not_create_the_definition_namespace() {
    let mut vm = vm();
    assert!(
        vm.try_define_procedure("::invalid_header::body", &["{x y z}"], "return OK")
            .is_err()
    );
    assert_eq!(
        crate::fixture::eval(
            &mut vm,
            "list [namespace exists ::invalid_header] [info commands ::invalid_header::body]"
        )
        .result
        .to_str()
        .as_ref(),
        "0 {}"
    );
}

#[test]
fn guest_proc_keeps_its_missing_namespace_header_failure() {
    let mut vm = vm();
    let result = vm
        .try_invoke_command(
            "proc",
            &[
                Value::string("::missing_header::body"),
                Value::empty(),
                Value::string("return OK"),
            ],
        )
        .unwrap();
    assert_eq!(result.code, tcl_runtime_api::Code::Error);
    assert_eq!(
        crate::fixture::eval(
            &mut vm,
            "list [namespace exists ::missing_header] [info commands ::missing_header::body]"
        )
        .result
        .to_str()
        .as_ref(),
        "0 {}"
    );
}
