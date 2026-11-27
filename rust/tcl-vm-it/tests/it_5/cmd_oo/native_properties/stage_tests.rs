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

//! `configure` reads and writes of configurable properties, against the retained tclsh captures.

use crate::prelude::*;

#[test]
fn materialized_property_reports_preserve_native_snapshot_and_table_index_roles() {
    let cases = [
        (
            include_str!(
                "../../../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-configurable.tcl"
            ),
            "code 0 result {-a A -z Z} events {a z} properties {-a -b -z}",
        ),
        (
            include_str!(
                "../../../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-configurable.tcl"
            ),
            "code 0 result {} events {a b} properties {-a -b -z}",
        ),
    ];
    for (script, expected) in cases {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.1"));
        assert_eq!(crate::fixture::eval(&mut vm, script).code, Code::Ok);
        let output = crate::fixture::eval(
            &mut vm,
            "list code $code result $result events $events properties [info object properties o -all -readable]",
        );
        assert_eq!(output.code, Code::Ok);
        assert_eq!(output.result.to_str().as_ref(), expected);
    }
}
