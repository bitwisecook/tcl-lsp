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
use tcl_runtime_api::Code;
use tcl_syntax::value::ValueOps;
use tcl_vm::value::Value;
include!("../../../../tcl-registry/tests/data/native_info_commands/cases.rs");

fn define_names(vm: &mut tcl_vm::Vm) {
    vm.try_eval_source("namespace eval N {}").unwrap();
    for name in INFO_COMMAND_NAMES {
        let original = Value::new_native_string_bytes(name);
        let empty = Value::new_native_string_bytes(b"".as_slice());
        let outcome = vm
            .try_invoke_command("proc", &[original, empty.clone(), empty])
            .expect("no host refusal");
        assert_eq!(outcome.code, Code::Ok);
    }
}

#[test]
fn original_info_commands_matches_all_110_native_pattern_results() {
    let rows = include_str!("../../../../tcl-registry/tests/data/native_info_commands/windows.tsv");
    let mut count = 0;
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = crate::fixture::distribution(
            tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
        );
        define_names(&mut vm);
        let dialect = tcl_native_model::InvocationDialect::of_static_profile(vm.dialect_profile());
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
            let command = Value::new_native_string_bytes(b"commands".as_slice());
            let outcome = vm
                .try_invoke_command("info", &[command, original.clone()])
                .unwrap_or_else(|refusal| panic!("{row}: {refusal:?}"));
            assert_eq!(
                outcome.code,
                Code::from_int(fields[3].parse().unwrap()),
                "{row}"
            );
            let members = ValueOps::list_elements(&mut vm, &outcome.result).unwrap();
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
