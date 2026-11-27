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

//! Jim `switch` options and callbacks against the original captures.

mod native_jim {
    mod tests {
        use crate::prelude::*;
        use std::cell::Cell;

        struct Callback<F>(F);

        impl<F> tcl_vm::NativeCommand for Callback<F>
        where
            F: Fn(&mut Vm, &[Value]) -> Completion<Value>,
        {
            fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
                (self.0)(vm, args)
            }
        }

        fn register_matcher(
            vm: &mut Vm,
            callback: impl Fn(&mut Vm, &[Value]) -> Completion<Value> + 'static,
        ) {
            vm.register_native_command("matcher", Rc::new(Callback(callback)));
        }

        fn ok(value: Value) -> Completion<Value> {
            Completion::new(Code::Ok, value, Value::empty())
        }

        fn install_case_matcher(vm: &mut Vm, index: usize, list: Value, calls: &Rc<Cell<usize>>) {
            let count = Rc::clone(calls);
            register_matcher(vm, move |vm, _| {
                count.set(count.get() + 1);
                if index == 30
                    && let Err(error) = list.native_character_count_with_protocol(
                        tcl_syntax::native_string::NativeStringProtocol::Jim084,
                        tcl_native_model::native_string_length::NativeStringLengthRepresentation::JimCachedString,
                    )
                {
                    return Completion::new(
                        Code::Error,
                        Value::string(error.to_string()),
                        Value::empty(),
                    );
                }
                if index == 31 && count.get() == 1 {
                    let count = Rc::clone(&count);
                    register_matcher(vm, move |_, _| {
                        count.set(count.get() + 1);
                        ok(Value::int(1))
                    });
                    return ok(Value::int(0));
                }
                match index {
                    27 => ok(Value::int(0)),
                    28 => ok(Value::string("NONNUMERIC")),
                    29 => Completion::new(
                        Code::Error,
                        Value::string("CALLBACK_ERROR"),
                        Value::empty(),
                    ),
                    _ => ok(Value::int(1)),
                }
            });
        }

        #[test]
        fn switch_matches_all_192_original_native_option_and_callback_results() {
            let cases =
                include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/cases.tsv");
            let mut compared = 0;
            for (profile, rows) in [
                (
                    "tcl8.4",
                    include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/8.4.20.tsv"),
                ),
                (
                    "tcl8.5",
                    include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/8.5.19.tsv"),
                ),
                (
                    "tcl8.6",
                    include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/8.6.18.tsv"),
                ),
                (
                    "tcl9.0",
                    include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/9.0.4.tsv"),
                ),
                (
                    "tcl9.1",
                    include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/9.1.0.tsv"),
                ),
                (
                    "jim",
                    include_str!("../../../tcl-cmd-core/tests/data/native_jim_switch/jim0.84.tsv"),
                ),
            ] {
                for (index, row) in rows
                    .lines()
                    .filter(|line| line.starts_with("RESULT\t"))
                    .enumerate()
                {
                    let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
                    assert_eq!(fields[1].parse::<usize>().unwrap(), index);
                    let case: Vec<_> =
                        tcl_test_support::fixture::tsv_fields(cases.lines().nth(index).unwrap())
                            .collect();
                    let mut vm =
                        crate::fixture::interpreter(tcl_vm::test_support::profile(profile));
                    let words: Vec<_> = case[1..]
                        .iter()
                        .map(|word| {
                            Value::from_string_bytes(crate::fixture::unhex(word).as_slice())
                        })
                        .collect();
                    let calls = Rc::new(Cell::new(0));
                    install_case_matcher(&mut vm, index, words.last().unwrap().clone(), &calls);
                    let head =
                        String::from_utf8(vm.native_string_bytes(&words[0]).unwrap().to_vec())
                            .unwrap();
                    let result = vm
                        .try_invoke_command(&head, &words[1..])
                        .unwrap_or_else(|refusal| panic!("{profile} {}: {refusal:?}", case[0]));
                    assert_eq!(
                        result.code.as_int(),
                        fields[3].parse::<i64>().unwrap(),
                        "{profile} {}",
                        case[0]
                    );
                    assert_eq!(
                        calls.get(),
                        fields[4].parse::<usize>().unwrap(),
                        "{profile} {}: callback count",
                        case[0]
                    );
                    assert_eq!(
                        vm.native_string_bytes(&result.result).unwrap().as_ref(),
                        crate::fixture::unhex(fields[5]).as_slice(),
                        "{profile} {}",
                        case[0]
                    );
                    compared += 1;
                }
            }
            assert_eq!(compared, 192);
        }
    }
}
