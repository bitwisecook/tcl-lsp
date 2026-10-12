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

//! Original command-table slots and native pattern-object enumeration.
use crate::value::Value;
use tcl_runtime_api::Code;
include!("../../../tcl-registry/tests/data/native_info_commands/cases.rs");

fn define_names(vm: &mut crate::Vm) {
    vm.try_eval_source("namespace eval N {}").unwrap();
    let head = Value::new_native_string_bytes(b"proc".as_slice());
    for name in INFO_COMMAND_NAMES {
        let original = Value::new_native_string_bytes(name);
        let empty = Value::new_native_string_bytes(b"".as_slice());
        let outcome =
            vm.invoke_host_original_object_vector(&head, &[original, empty.clone(), empty]);
        assert_eq!(outcome.code, Code::Ok);
        assert!(vm.execution_refusal.is_none(), "{:?}", vm.execution_refusal);
    }
}

#[test]
fn original_info_commands_matches_all_110_native_pattern_object_windows() {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    let rows = include_str!("../../../tcl-registry/tests/data/native_info_commands/windows.tsv");
    let mut count = 0;
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = crate::native_fixture::interpreter(
            tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
        );
        define_names(&mut vm);
        let dialect = vm.actual_native_invocation_dialect();
        let protocol = dialect.native_string_protocol().unwrap();
        for row in rows.lines().skip(1) {
            let fields: Vec<_> = row.split('|').collect();
            if fields[0] != engine {
                continue;
            }
            let byte_array = fields[1] == "1";
            let input = INFO_COMMAND_PATTERNS[fields[2].parse::<usize>().unwrap()];
            let original = if byte_array {
                Value::from_native_byte_array(std::rc::Rc::from(input), dialect).unwrap()
            } else {
                Value::new_native_string_bytes(input)
            };
            if byte_array {
                assert!(matches!(
                    original.native_object_snapshot().cache,
                    Cache::ByteArray { .. }
                ));
            }
            let head = Value::new_native_string_bytes(b"info".as_slice());
            let command = Value::new_native_string_bytes(b"commands".as_slice());
            let outcome =
                vm.invoke_host_original_object_vector(&head, &[command, original.clone()]);
            assert!(
                vm.execution_refusal.is_none(),
                "{row}: {:?}",
                vm.execution_refusal
            );
            assert_eq!(
                outcome.code,
                Code::from_int(fields[3].parse().unwrap()),
                "{row}"
            );
            if byte_array {
                assert!(
                    matches!(
                        original.native_object_snapshot().cache,
                        Cache::ByteArray { .. }
                    ),
                    "{row}"
                );
            }
            let members = vm
                .native_object_list_elements_in(&outcome.result, protocol)
                .unwrap();
            assert_eq!(members.len().to_string(), fields[4], "{row}");
            let mut names: Vec<_> = members
                .iter()
                .map(|name| info_command_hex(&name.native_string_bytes(protocol).unwrap()))
                .collect();
            names.sort();
            assert_eq!(names.join(","), fields[5], "{row}");
            count += 1;
        }
    }
    assert_eq!(count, 110);
}
