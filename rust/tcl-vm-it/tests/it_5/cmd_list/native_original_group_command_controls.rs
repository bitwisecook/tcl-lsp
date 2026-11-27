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

//! Grouped `lsort` fields against the tclsh CLI capture.

use crate::prelude::*;

include!(
    "../../../../../runtime/rust/tests/data/native_original_list_group_cli656/software_controls.rs"
);

#[test]
fn original_cli_group_fields_match_native656_source_capture() {
    // naming.list.original-cli-group-boundary-results
    // docs/design/analysis/name-resolution-proofs/list-original-cli-group-boundary-results.md
    // Same fixed Tcl capture prefix with separately initialised argv; compare
    // exact script/code/result/errorCode/errorInfo/presence and callback count.
    // Jim651 case12 is a different input and remains unavailable.
    let probe = include_str!(
        "../../../../../runtime/rust/tests/data/native_original_list_group_cli656/probe.tcl"
    );
    let capture = probe.split("puts \"case\\t").next().unwrap();
    assert_eq!(CLI_ROWS.len(), 66);
    for row in CLI_ROWS {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(row.dialect));
        let startup = format!("set argv {{{}}}", row.case);
        assert_eq!(vm.eval_source(&startup).unwrap().code, Code::Ok);
        let label = format!("{}:{}:{}", row.dialect, row.case, row.stream_sha256);
        assert_eq!(vm.eval_source(capture).unwrap().code, Code::Ok, "{label}");
        for (name, expected) in CLI_VARIABLES.into_iter().zip(row.fields) {
            let original = vm
                .try_invoke_command("set", &[Value::string(name)])
                .unwrap()
                .result;
            assert_eq!(
                vm.native_string_bytes(&original).unwrap().as_ref(),
                crate::fixture::unhex(expected),
                "{label}:{name}"
            );
        }
    }
}
