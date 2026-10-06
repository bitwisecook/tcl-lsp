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
use crate::value::Value;
use tcl_runtime_api::Code;
include!("../../../tcl-registry/tests/data/native_string_trim_compilation/cases.rs");

#[test]
fn original_string_trim_matches_all_216_native_header_windows() {
    let table =
        include_str!("../../../tcl-registry/tests/data/native_string_trim_compilation/windows.tsv");
    let mut compared = 0;
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = crate::native_fixture::interpreter(
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
                    vm.actual_native_invocation_dialect(),
                )
                .unwrap()
            };
            let characters = Value::new_native_string_bytes(b"-".as_slice());
            let head = Value::new_native_string_bytes(if mode == "1" {
                b"p".as_slice()
            } else {
                b"string"
            });
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
            let completion = vm.invoke_host_original_object_vector(&head, &arguments);
            drop(arguments);
            assert!(
                vm.execution_refusal.is_none(),
                "{engine}/{mode}/{member}/{case}: {:?}",
                vm.execution_refusal
            );
            assert_eq!(
                completion.code,
                Code::from_int(f[4].parse().unwrap()),
                "{row}"
            );
            assert_eq!(
                trim_header(
                    &completion.result.native_object_snapshot(),
                    completion.result.native_object_reference_count()
                ),
                f[5],
                "{row}"
            );
            assert_eq!(
                usize::from(completion.result.is_same_object(&subject)).to_string(),
                f[6],
                "{row}"
            );
            assert_eq!(
                trim_header(
                    &subject.native_object_snapshot(),
                    subject.native_object_reference_count()
                ),
                f[7],
                "{row}"
            );
            assert_eq!(trim_hex(&completion.result.string_bytes()), f[9], "{row}");
            compared += 1;
        }
    }
    assert_eq!(compared, 216);
}
