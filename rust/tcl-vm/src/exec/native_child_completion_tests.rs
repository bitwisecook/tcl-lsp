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

//! Completion handoff through entered script and control continuations.

use tcl_runtime_api::Code;

fn unhex(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn entered_child_completions_match_forty_eight_real_engine_controls() {
    let mut compared = 0;
    for (engine, table) in [
        (
            "tcl8.4",
            include_str!("../../tests/data/native_child_completion/8.4.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_child_completion/8.5.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_child_completion/8.6.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_child_completion/9.0.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_child_completion/9.1.tsv"),
        ),
        (
            "jim",
            include_str!("../../tests/data/native_child_completion/jim.tsv"),
        ),
    ] {
        for row in table.lines() {
            let fields = row.split('\t').collect::<Vec<_>>();
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
            if engine == "jim" {
                // jimsh supplies binary as an extension with a tailcalling Tcl wrapper.
                crate::cmd_binary::register(&mut vm);
            }
            let source = String::from_utf8(unhex(fields[1])).unwrap();
            let completion = vm
                .try_eval_source(&source)
                .unwrap_or_else(|error| panic!("{engine}/{}: {error:?}", fields[0]));
            assert_eq!(
                completion.code.as_int().to_string(),
                fields[2],
                "{engine}/{}",
                fields[0]
            );
            assert_eq!(completion.code, Code::Ok, "{engine}/{}", fields[0]);
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                unhex(fields[3]),
                "{engine}/{}",
                fields[0]
            );
            assert!(vm.refused_completion().is_none(), "{engine}/{}", fields[0]);
            compared += 1;
        }
    }
    assert_eq!(compared, 48);
}

#[test]
fn synchronous_tailcall_validates_parent_before_array_index_completion() {
    use std::{cell::Cell, rc::Rc};

    struct Target {
        replace_compiler: bool,
        entered: Rc<Cell<usize>>,
    }
    impl crate::NativeCommand for Target {
        fn invoke(
            &self,
            vm: &mut crate::Vm,
            _: &[crate::Value],
        ) -> crate::Completion<crate::Value> {
            self.entered.set(self.entered.get() + 1);
            if self.replace_compiler {
                let profile =
                    tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
                vm.set_compiler(Box::new(
                    tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
                ));
            }
            crate::interp::ok(crate::Value::string("key"))
        }
    }
    struct Observe(Rc<Cell<usize>>);
    impl crate::NativeCommand for Observe {
        fn invoke(&self, _: &mut crate::Vm, _: &[crate::Value]) -> crate::Completion<crate::Value> {
            self.0.set(self.0.get() + 1);
            crate::interp::ok(crate::Value::empty())
        }
    }

    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
    for (body, value) in [
        ("subst {$a([tailcaller])}", "9"),
        ("expr {$a([tailcaller]) + 1}", "10"),
    ] {
        for replace_compiler in [false, true] {
            let mut vm = crate::native_fixture::interpreter(profile);
            let entered = Rc::new(Cell::new(0));
            let reads = Rc::new(Cell::new(0));
            vm.register_native_command(
                "target",
                Rc::new(Target {
                    replace_compiler,
                    entered: entered.clone(),
                }),
            );
            vm.register_native_command("observe_array_read", Rc::new(Observe(reads.clone())));
            let setup = vm
                .try_eval_source(&format!(
                    "set a(key) 9; trace add variable a read observe_array_read; \
                     proc tailcaller {{}} {{tailcall target}}; \
                     proc parent {{}} {{global a; {body}}}"
                ))
                .expect("original declarations compile");
            assert_eq!(setup.code, Code::Ok, "{body}");
            let completion = vm
                .try_eval_source("parent")
                .expect("original caller compiles");
            assert_eq!(entered.get(), 1, "{body}");
            if replace_compiler {
                assert_eq!(completion.code, Code::Error, "{body}");
                assert_eq!(
                    completion.result.to_str().as_ref(),
                    "cannot continue bytecode after compile service changed",
                    "{body}"
                );
                assert_eq!(
                    reads.get(),
                    0,
                    "stale {body} must not read the original array"
                );
            } else {
                assert_eq!(completion.code, Code::Ok, "{body}");
                assert_eq!(completion.result.to_str().as_ref(), value, "{body}");
                assert_eq!(
                    reads.get(),
                    1,
                    "live {body} must read the original array once"
                );
            }
        }
    }
}

#[test]
fn original_coroutine_execution_matches_seventy_eight_engine_controls() {
    let mut compared = 0;
    for (engine, table) in [
        (
            "tcl8.4",
            include_str!(
                "../../../tcl-registry/tests/data/native_coroutine_compilation/runtime-8.4.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../tcl-registry/tests/data/native_coroutine_compilation/runtime-8.5.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_coroutine_compilation/runtime-8.6.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_coroutine_compilation/runtime-9.0.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_coroutine_compilation/runtime-9.1.tsv"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../tcl-registry/tests/data/native_coroutine_compilation/runtime-jim.tsv"
            ),
        ),
    ] {
        for row in table.lines() {
            let fields = row.split('\t').collect::<Vec<_>>();
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
            let source = String::from_utf8(unhex(fields[1])).unwrap();
            let completion = vm
                .try_eval_source(&source)
                .unwrap_or_else(|error| panic!("{engine}/{}: {error:?}", fields[0]));
            assert_eq!(
                completion.code.as_int().to_string(),
                fields[2],
                "{engine}/{}",
                fields[0]
            );
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                unhex(fields[3]),
                "{engine}/{}",
                fields[0]
            );
            assert!(vm.refused_completion().is_none(), "{engine}/{}", fields[0]);
            compared += 1;
        }
    }
    assert_eq!(compared, 78);
}

#[test]
fn coroutine_creation_yield_and_resume_preserve_original_list_headers() {
    use crate::Value;
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        assert_eq!(
            vm.try_eval_source(
                "proc generator value {set resumed [yield $value]; return $resumed}"
            )
            .unwrap()
            .code,
            Code::Ok
        );
        let original = Value::list(vec![Value::string("A"), Value::string("B")]);
        let yielded = vm
            .try_invoke_command(
                "coroutine",
                &[
                    Value::string("c"),
                    Value::string("generator"),
                    original.clone(),
                ],
            )
            .unwrap();
        assert_eq!(yielded.code, Code::Ok, "{engine}");
        assert_eq!(
            yielded.result.native_object_identity(),
            original.native_object_identity(),
            "{engine}: creation argv/yield"
        );
        let resumed = Value::list(vec![Value::string("C"), Value::string("D")]);
        let result = vm
            .try_invoke_command("c", std::slice::from_ref(&resumed))
            .unwrap();
        assert_eq!(result.code, Code::Ok, "{engine}");
        assert_eq!(
            result.result.native_object_identity(),
            resumed.native_object_identity(),
            "{engine}: resume/return"
        );
    }
}
