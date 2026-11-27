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

//! Jim's rejected `namespace` selector message, through a replaced `lsort`.

use crate::prelude::*;

#[test]
fn rejected_selector_uses_the_actual_mutable_sort_result_and_preserves_sort_error_text() {
    const ORIGINAL: &str = include_str!(
        "../../../../../../runtime/rust/tests/data/native_jim_namespace_subcommand/table-20261015.tsv"
    );
    for (label, setup, head, body) in [
        (
            "MISSING_REPLACED_SORT",
            "rename namespace renamed",
            "renamed",
            "return {which canonical}",
        ),
        (
            "MISSING_FAILED_SORT",
            "",
            "namespace",
            "return -code error SORTFAIL",
        ),
    ] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("jimtcl"));
        let script = format!("proc lsort {{args}} {{{body}}}; {setup}");
        assert_eq!(crate::fixture::eval(&mut vm, &script).code, Code::Ok);
        for suffix in ["COLD", "REUSE"] {
            let phase = format!("{label}_{suffix}");
            let (code, result) = crate::fixture::run(&mut vm, head, &["unavailable"]);
            let expected = ORIGINAL
                .lines()
                .find_map(|line| line.strip_prefix(&format!("RESULT|{phase}|{}|", code.as_int())))
                .unwrap_or_else(|| panic!("{phase}: code {code:?}"));
            assert_eq!(result, crate::fixture::unhex(expected), "{phase}");
        }
    }
}
