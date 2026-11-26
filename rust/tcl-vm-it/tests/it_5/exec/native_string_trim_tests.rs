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

//! Paired original trim object-vector and compiled result ownership windows.
use tcl_runtime_api::Code;
use tcl_vm::value::Value;
include!("../../../../tcl-registry/tests/data/native_string_trim_compilation/cases.rs");

#[test]
fn original_string_trim_matches_all_216_native_completions() {
    let table = include_str!(
        "../../../../tcl-registry/tests/data/native_string_trim_compilation/windows.tsv"
    );
    let mut compared = 0;
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = crate::fixture::interpreter(
            tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
        );
        for row in table.lines().skip(1) {
            let f: Vec<_> = row.split('|').collect();
            if f[0] != engine {
                continue;
            }
            let mode = f[1];
            let member = f[2];
            let case = f[3].parse::<usize>().unwrap();
            let source = format!(
                "proc p {{s c}} {{string {member} $s{}}}",
                if case == 5 { " $c" } else { "" }
            );
            vm.try_eval_source(&source).unwrap();
            let subject = if engine == "jim" {
                Value::new_native_string_bytes(TRIM_INPUTS[case])
            } else {
                Value::from_native_byte_array(
                    std::rc::Rc::from(TRIM_INPUTS[case]),
                    tcl_registry::InvocationDialect::of_static_profile(vm.dialect_profile()),
                )
                .unwrap()
            };
            let characters = Value::new_native_string_bytes(b"-".as_slice());
            let head = if mode == "1" { "p" } else { "string" };
            let arguments = if mode == "1" {
                vec![subject.clone(), characters.clone()]
            } else {
                let mut args = vec![
                    Value::new_native_string_bytes(member.as_bytes()),
                    subject.clone(),
                ];
                if case == 5 {
                    args.push(characters.clone());
                }
                args
            };
            // The argv vector is a lifetime transport of these originals. Drop its
            // Rust handles before observing final native result/subject ownership.
            let completion = vm
                .try_invoke_command(head, &arguments)
                .unwrap_or_else(|refusal| panic!("{engine}/{mode}/{member}/{case}: {refusal:?}"));
            drop(arguments);
            assert_eq!(
                completion.code,
                Code::from_int(f[4].parse().unwrap()),
                "{row}"
            );
            assert_eq!(trim_hex(&completion.result.string_bytes()), f[9], "{row}");
            compared += 1;
        }
    }
    assert_eq!(compared, 216);
}
