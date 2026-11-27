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

//! `foreach` and `lmap` completions against the original captures.

use crate::prelude::*;
use std::cell::Cell;

const FIXTURES: [(&str, &str, &str); 6] = [
    (
        "tcl8.4",
        include_str!("../../../../tcl-cmd-core/tests/data/native_each_loop/original/tcl8.4.txt"),
        include_str!("../../../../tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.4.txt"),
    ),
    (
        "tcl8.5",
        include_str!("../../../../tcl-cmd-core/tests/data/native_each_loop/original/tcl8.5.txt"),
        include_str!("../../../../tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.5.txt"),
    ),
    (
        "tcl8.6",
        include_str!("../../../../tcl-cmd-core/tests/data/native_each_loop/original/tcl8.6.txt"),
        include_str!("../../../../tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.6.txt"),
    ),
    (
        "tcl9.0",
        include_str!("../../../../tcl-cmd-core/tests/data/native_each_loop/original/tcl9.0.txt"),
        include_str!("../../../../tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl9.0.txt"),
    ),
    (
        "tcl9.1",
        include_str!("../../../../tcl-cmd-core/tests/data/native_each_loop/original/tcl9.1.txt"),
        include_str!("../../../../tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl9.1.txt"),
    ),
    (
        "jim",
        include_str!("../../../../tcl-cmd-core/tests/data/native_each_loop/original/jim.txt"),
        include_str!("../../../../tcl-cmd-core/tests/data/native_each_loop/shimmer/jim.txt"),
    ),
];

/// The each-loop words of `case`, built as the capture's probe builds them.
fn inputs(vm: &mut Vm, case: usize) -> (Vec<Value>, [Value; 2]) {
    let string = |bytes: &[u8]| Value::from_string_bytes(bytes);
    let mut names = vec![string(b"v"), string(b"w")];
    names.truncate(if case == 2 { 2 } else { 1 });
    let mut members = vec![string(b"A"), string(b"B"), string(b"C")];
    if case == 8 {
        members[0] = Value::list(vec![string(b"N")]);
    }
    members.truncate(match case {
        2 => 3,
        5 => 2,
        3 | 4 => 0,
        _ => 1,
    });
    let body = string(if matches!(case, 3 | 4) {
        b"{"
    } else {
        b"observe"
    });
    let mut argv = vec![Value::list(names), Value::list(members), body];
    if matches!(case, 6 | 7) {
        let variables = argv.remove(0);
        let body = argv.pop().unwrap();
        let empty = Value::list(Vec::new());
        argv = if case == 6 {
            vec![
                variables,
                string(b"{"),
                empty,
                Value::list(Vec::new()),
                body,
            ]
        } else {
            vec![
                empty,
                string(b"{"),
                variables,
                Value::list(Vec::new()),
                body,
            ]
        };
    }
    if case == 9 {
        argv[0] = Value::list(Vec::new());
    }
    if case == 1 {
        for root in &argv[..2] {
            vm.native_string_bytes(root).unwrap();
        }
    }
    let roots = [argv[0].clone(), argv[1].clone()];
    (argv, roots)
}

/// The loop body: on its first call with `shimmer`, it takes the character
/// length of the value and variable words.
struct Observe {
    roots: [Value; 2],
    shimmer: bool,
    entered: Cell<bool>,
}

impl tcl_vm::NativeCommand for Observe {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        if self.shimmer && !self.entered.replace(true) {
            for root in [&self.roots[1], &self.roots[0]] {
                vm.native_char_len(root).unwrap();
            }
        }
        Completion::new(Code::Ok, Value::string("BODY"), Value::empty())
    }
}

#[test]
fn generic_each_loops_match_all_120_original_native_completions() {
    let mut completions = 0;
    for (engine, original, shimmer_fixture) in FIXTURES {
        for (shimmer, fixture) in [(false, original), (true, shimmer_fixture)] {
            for case in 0..10 {
                let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
                let (argv, roots) = inputs(&mut vm, case);
                vm.register_native_command(
                    "observe",
                    Rc::new(Observe {
                        roots,
                        shimmer,
                        entered: Cell::new(false),
                    }),
                );
                let command = if matches!(case, 4 | 5) {
                    "lmap"
                } else {
                    "foreach"
                };
                let completion = crate::fixture::call(&mut vm, command, &argv);
                let prefix = format!("R\t{case}\t");
                let row = fixture
                    .lines()
                    .find(|row| row.starts_with(&prefix))
                    .unwrap();
                let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
                assert_eq!(
                    completion.code.as_int(),
                    fields[2].parse::<i64>().unwrap(),
                    "{engine}/{case}/shimmer={shimmer}"
                );
                assert_eq!(
                    vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                    crate::fixture::unhex(fields[3]).as_slice(),
                    "{engine}/{case}/shimmer={shimmer}"
                );
                completions += 1;
            }
        }
    }
    assert_eq!(completions, 120);
}

#[test]
fn original_procedure_bytecode_reconciles_script_and_procedure_contexts() {
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl8.6"));
    let body = Value::string("set x");
    let declaration = [Value::string("p"), Value::string("x"), body.clone()];
    assert_eq!(
        crate::fixture::call(&mut vm, "proc", &declaration).code,
        Code::Ok
    );
    for (call, expected) in [("ONE", Code::Ok), ("", Code::Error), ("TWO", Code::Ok)] {
        let completion = if call.is_empty() {
            crate::fixture::call(&mut vm, "eval", std::slice::from_ref(&body))
        } else {
            crate::fixture::call(&mut vm, "p", &[Value::string(call)])
        };
        assert_eq!(completion.code, expected, "{call}");
        if !call.is_empty() {
            assert_eq!(
                vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                call.as_bytes()
            );
        }
    }
}
