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

//! A method that replaces itself while it runs, compared with tclsh.

use crate::prelude::*;

#[test]
fn original_method_preparation_keeps_entered_record_after_table_withdrawal_and_fresh_same_name() {
    // tclsh 8.6.18 and 9.0.4: `ORIGINAL REPLACEMENT`.
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.1"));
    let completion = crate::fixture::eval(
        &mut vm,
        "oo::class create P {method m {} {oo::define P deletemethod m; oo::define P method m {} {return REPLACEMENT}; oo::define P variable legal; return ORIGINAL}}; P create p; list [p m] [p m]",
    );
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.to_str().as_ref(), "ORIGINAL REPLACEMENT");
}
