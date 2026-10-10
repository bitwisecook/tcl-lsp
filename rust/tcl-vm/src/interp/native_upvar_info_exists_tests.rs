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

//! Original ordinary upvar/existence execution and reached name-header windows.
use crate::{Value, Vm};
use std::cell::RefCell;
include!("../../../tcl-registry/tests/data/native_upvar_info_exists/cases.rs");
const TABLES: &[(&str, &str)] = &[
    (
        "tcl8.4",
        include_str!("../../../tcl-registry/tests/data/native_upvar_info_exists/8.4.20.txt"),
    ),
    (
        "tcl8.5",
        include_str!("../../../tcl-registry/tests/data/native_upvar_info_exists/8.5.19.txt"),
    ),
    (
        "tcl8.6",
        include_str!("../../../tcl-registry/tests/data/native_upvar_info_exists/8.6.18.txt"),
    ),
    (
        "tcl9.0",
        include_str!("../../../tcl-registry/tests/data/native_upvar_info_exists/9.0.4.txt"),
    ),
    (
        "tcl9.1",
        include_str!("../../../tcl-registry/tests/data/native_upvar_info_exists/9.1.0.txt"),
    ),
];
include!("../../../tcl-registry/tests/data/native_upvar_info_exists/quiet/cases.rs");
const QUIET_TABLES: &[(&str, &str)] = &[
    (
        "tcl8.4",
        include_str!("../../../tcl-registry/tests/data/native_upvar_info_exists/quiet/8.4.20.txt"),
    ),
    (
        "tcl8.5",
        include_str!("../../../tcl-registry/tests/data/native_upvar_info_exists/quiet/8.5.19.txt"),
    ),
    (
        "tcl8.6",
        include_str!("../../../tcl-registry/tests/data/native_upvar_info_exists/quiet/8.6.18.txt"),
    ),
    (
        "tcl9.0",
        include_str!("../../../tcl-registry/tests/data/native_upvar_info_exists/quiet/9.0.4.txt"),
    ),
    (
        "tcl9.1",
        include_str!("../../../tcl-registry/tests/data/native_upvar_info_exists/quiet/9.1.0.txt"),
    ),
];
const SETUP: &str = "catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}";
fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

thread_local! {
    static MARKS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}
fn mark(_vm: &mut Vm, arguments: &[Value]) -> tcl_runtime_api::Completion<Value> {
    let value = &arguments[0];
    let prefix = format!(
        "{}|{}|{}|",
        value.native_object_type_name(),
        usize::from(value.resident_string_bytes().is_some()),
        value.native_object_reference_count()
    );
    let mut bytes = prefix;
    for byte in value.string_bytes().iter() {
        use std::fmt::Write;
        write!(bytes, "{byte:02x}").unwrap();
    }
    MARKS.with(|marks| marks.borrow_mut().push(bytes));
    crate::interp::ok(Value::empty())
}
fn original_variable_instructions(vm: &mut Vm) -> String {
    let key = vm
        .resolve_command_bytes_checked(tcl_core_types::ROOT_NS, b"p", true)
        .unwrap()
        .expect("same original published procedure");
    let Some(crate::command::Command::Proc(procedure)) = vm.commands.get(&key) else {
        panic!("same original procedure binding");
    };
    let Some(cache) = procedure.body_src.native_bytecode_cache() else {
        return String::new();
    };
    let mut rows = String::new();
    for instruction in &cache.unit.asm.instructions {
        let name = instruction.op.mnemonic();
        if name == "upvar" || name.starts_with("exist") {
            rows.push_str(name);
            if let Some(tcl_bytecode::Operand::Imm(slot)) = instruction.operands.first() {
                use std::fmt::Write;
                write!(rows, ":{slot}").unwrap();
            }
            rows.push(',');
        }
    }
    rows
}
fn compare_native_tables(tables: &[(&str, &str)], cases: &[&str]) -> (usize, usize) {
    let mut results = 0;
    let mut headers = 0;
    for &(engine, table) in tables {
        let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
        let mut vm = crate::native_fixture::interpreter(profile);
        vm.register("mark", mark);
        for row in table.lines().filter(|row| row.starts_with("R|")) {
            let fields: Vec<_> = row.split('|').collect();
            let case = fields[1].parse::<usize>().unwrap();
            assert_eq!(
                vm.try_eval_source(SETUP).unwrap().code,
                tcl_runtime_api::Code::Ok
            );
            vm.try_eval_source(&format!(
                "proc p {{name idx}} {{set x LOCAL;set a(k) LOCALARRAY;{}}}",
                cases[case]
            ))
            .unwrap();
            MARKS.with(|marks| marks.borrow_mut().clear());
            let head = Value::new_native_string_bytes(b"p".as_slice());
            let arguments = [
                Value::new_native_string_bytes(b"x".as_slice()),
                Value::new_native_string_bytes(b"k".as_slice()),
            ];
            let completion = vm.invoke_host_original_object_vector(&head, &arguments);
            assert!(
                vm.execution_refusal.is_none(),
                "{engine}/{case}: {:?}",
                vm.execution_refusal
            );
            assert_eq!(
                completion.code.as_int().to_string(),
                fields[2],
                "{engine}/{case}"
            );
            let value = &completion.result;
            assert_eq!(
                value.native_object_type_name(),
                fields[3],
                "{engine}/{case} primary"
            );
            assert_eq!(
                usize::from(value.resident_string_bytes().is_some()).to_string(),
                fields[4],
                "{engine}/{case} resident"
            );
            assert_eq!(
                value.native_object_reference_count().to_string(),
                fields[5],
                "{engine}/{case} references"
            );
            assert_eq!(
                value.string_bytes().as_ref(),
                unhex(fields[6]),
                "{engine}/{case} result"
            );
            assert_eq!(
                original_variable_instructions(&mut vm),
                fields[7],
                "{engine}/{case} original variable instructions"
            );
            let expected = table
                .lines()
                .filter(|row| row.starts_with(&format!("H|{case}|")))
                .map(|row| row.split('|').skip(3).collect::<Vec<_>>().join("|"))
                .collect::<Vec<_>>();
            MARKS.with(|marks| {
                assert_eq!(
                    *marks.borrow(),
                    expected,
                    "{engine}/{case} reached originals"
                )
            });
            results += 1;
            headers += expected.len();
        }
    }
    (results, headers)
}

// Native proof: naming.variable.original-upvar-and-exists-completion-and-name-windows
// docs/design/analysis/name-resolution-proofs/variable.original-upvar-and-exists-completion-and-name-windows.md
#[test]
fn original_upvar_and_exists_preserve_140_native_completions_and_20_name_headers() {
    assert_eq!(compare_native_tables(TABLES, CASES), (140, 20));
}
// Native proof: naming.variable.exists-read-trace-quiet-result-purpose
// docs/design/analysis/name-resolution-proofs/variable.exists-read-trace-quiet-result-purpose.md
#[test]
fn original_exists_quiet_trace_semantics_preserve_15_native_result_windows() {
    assert_eq!(compare_native_tables(QUIET_TABLES, QUIET_CASES), (15, 0));
}
