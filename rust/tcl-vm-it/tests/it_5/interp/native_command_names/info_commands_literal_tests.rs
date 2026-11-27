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

//! Compiled and generic `info commands` results, against the tclsh and jimsh captures.

use crate::prelude::*;

include!("../../../../../tcl-registry/tests/data/native_info_commands_literal_original/cases.rs");
const WINDOWS: &str = include_str!(
    "../../../../../tcl-registry/tests/data/native_info_commands_literal_original/windows.tsv"
);

fn vm_for(engine: &str) -> Vm {
    let profile = tcl_vm::test_support::profile(engine);
    // jimsh's namespace-aware inventory runs in its scripted `nshelper.tcl`.
    if engine == "jim" {
        crate::fixture::distribution(profile)
    } else {
        crate::fixture::interpreter(profile)
    }
}

#[test]
fn compiled_and_generic_info_commands_retain_one_hundred_eighty_six_original_header_windows() {
    // naming.compiler.original-info-commands-literal-resolution
    // docs/design/analysis/name-resolution-proofs/compiler-original-info-commands-literal-resolution.md
    let mut compared = 0;
    for row in WINDOWS.lines().skip(1) {
        let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
        let case = ORIGINAL_INFO_COMMANDS_CASES[fields[1].parse::<usize>().unwrap()];
        let context = format!("{}: {}", fields[0], case.0);
        let mut vm = vm_for(fields[0]);
        let prelude = vm.try_eval_source_bytes(case.1).unwrap();
        assert_eq!(prelude.code, Code::Ok, "{context}: prelude");
        let definition = [case.3, case.4, case.2].map(Value::new_native_string_bytes);
        let defined = crate::fixture::call(&mut vm, "proc", &definition);
        assert_eq!(defined.code, Code::Ok, "{context}: definition");
        let arguments: Vec<_> = case
            .5
            .into_iter()
            .map(Value::new_native_string_bytes)
            .collect();
        let name = std::str::from_utf8(case.3).unwrap();
        let completion = crate::fixture::call(&mut vm, name, &arguments);
        assert_eq!(
            completion.code.as_int().to_string(),
            fields[2],
            "{context}: completion"
        );
        // jimsh's whole-table inventory lists `alarm`, from its signal
        // extension, which the VM does not install.
        let whole_table = fields[0] == "jim" && matches!(case.0, "glob-star" | "escaped-star");
        if !whole_table {
            assert_eq!(
                crate::fixture::bytes(&vm, &completion.result),
                crate::fixture::unhex(fields[7]),
                "{context}: result bytes"
            );
        }
        compared += 1;
    }
    assert_eq!(compared, 186);
}

#[test]
fn original_jim_import_retains_helper_publication_before_inventory_root_prefix() {
    // naming.compiler.original-info-commands-literal-resolution
    // docs/design/analysis/name-resolution-proofs/compiler-original-info-commands-literal-resolution.md
    // Original _nshelper.c publishes ${current}::tail; namespace info adds the
    // separately selected global report prefix.
    let mut vm = vm_for("jim");
    let case = ORIGINAL_INFO_COMMANDS_CASES[4];
    assert_eq!(case.0, "imported-command");
    assert_eq!(vm.try_eval_source_bytes(case.1).unwrap().code, Code::Ok);
    let completion = vm.try_eval_source_bytes(case.2).unwrap();
    assert_eq!(completion.code, Code::Ok);
    let members = completion.result.as_list().unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(crate::fixture::bytes(&vm, &members[0]), b"::I::target");
}
