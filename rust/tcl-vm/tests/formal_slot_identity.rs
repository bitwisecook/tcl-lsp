// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original formal declarations retain separate C compiled cells and Jim named cells.

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_vm::{Code, Value, Vm};

fn vm(dialect: &str) -> Vm {
    let mut vm = Vm::new();
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
    );
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    vm
}

#[test]
fn duplicate_formals_keep_native_binding_and_enumeration_identity() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = vm(dialect);
        let definition = vm.try_invoke_command("proc", &[
            Value::string("p"), Value::list(vec![Value::string("x"), Value::string("x")]),
            Value::string("set extra Z; if {0} {set absent 1}; list $x [set x] [info locals] [info vars]"),
        ]).unwrap();
        assert_eq!(definition.code, Code::Ok, "{dialect}");
        let result = vm
            .try_invoke_command("p", &[Value::string("ONE"), Value::string("TWO")])
            .unwrap();
        assert_eq!(result.code, Code::Ok, "{dialect}: {:?}", result.result);
        let fields = result.result.as_list().unwrap();
        let expected = if dialect == "jim" { "TWO" } else { "ONE" };
        assert_eq!(
            fields[0].to_str().as_ref(),
            expected,
            "{dialect} compiled read"
        );
        assert_eq!(
            fields[1].to_str().as_ref(),
            expected,
            "{dialect} dynamic read"
        );
        for field in &fields[2..] {
            let names = field.as_list().unwrap();
            let names: Vec<_> = names
                .iter()
                .map(|name| name.resident_string_bytes().unwrap().to_vec())
                .collect();
            let expected = if dialect == "jim" {
                vec![b"extra".to_vec(), b"x".to_vec()]
            } else {
                vec![b"x".to_vec(), b"x".to_vec(), b"extra".to_vec()]
            };
            assert_eq!(names, expected, "{dialect} original slot enumeration");
        }
    }
}

#[test]
fn counted_formals_separate_dynamic_cells_from_native_compiler_comparison() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for identical in [false, true] {
            let mut vm = vm(dialect);
            let first = b"k\0a";
            let second = if identical {
                first.as_slice()
            } else {
                b"k\0b".as_slice()
            };
            vm.set_var("::A", Value::from_native_string_bytes(first.to_vec()))
                .unwrap();
            vm.set_var("::B", Value::from_native_string_bytes(second.to_vec()))
                .unwrap();
            let mut body = b"list ${".to_vec();
            body.extend_from_slice(first);
            body.extend_from_slice(b"} ${");
            body.extend_from_slice(second);
            body.extend_from_slice(b"} [set $::A] [set $::B]");
            let definition = vm
                .try_invoke_command(
                    "proc",
                    &[
                        Value::string("p"),
                        Value::list(vec![
                            Value::from_native_string_bytes(first.to_vec()),
                            Value::from_native_string_bytes(second.to_vec()),
                        ]),
                        Value::from_native_string_bytes(body),
                    ],
                )
                .unwrap();
            assert_eq!(definition.code, Code::Ok, "{dialect} duplicate={identical}");
            let result = vm
                .try_invoke_command("p", &[Value::string("ONE"), Value::string("TWO")])
                .unwrap();
            if matches!(dialect, "tcl8.4" | "tcl8.5") {
                // These original parameter Lists are stringified through TclSplitList's CString input.
                assert_eq!(result.code, Code::Error, "{dialect}");
                continue;
            }
            assert_eq!(
                result.code,
                Code::Ok,
                "{dialect} duplicate={identical}: {:?}",
                result.result
            );
            let fields = result.result.as_list().unwrap();
            let actual: Vec<_> = fields
                .iter()
                .map(|field| field.to_str().to_string())
                .collect();
            let expected = if dialect == "jim" {
                if identical {
                    ["TWO", "TWO", "TWO", "TWO"]
                } else {
                    ["ONE", "TWO", "ONE", "TWO"]
                }
            } else if identical {
                ["ONE", "ONE", "ONE", "ONE"]
            } else {
                ["ONE", "ONE", "ONE", "TWO"]
            };
            assert_eq!(actual, expected, "{dialect} duplicate={identical}");
        }
    }
}
