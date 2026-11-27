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

//! A child's startup globals, against jimsh 0.84 and tclsh 8.4.20 and 9.0.4.

use crate::prelude::*;

#[test]
fn child_startup_copy_ignores_parent_local_shadows_and_c_has_no_copy_recipe() {
    // jimsh 0.84 prints `GLOBAL`; tclsh 8.4.20 and 9.0.4 print `0`.
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("jim"));
    let completion = crate::fixture::eval(
        &mut vm,
        "set argv GLOBAL; proc create_with_local {} {set argv LOCAL; interp}; [create_with_local] eval {set argv}",
    );
    assert_eq!(completion.code, Code::Ok, "{:?}", completion.result);
    assert_eq!(completion.result.string_bytes().as_ref(), b"GLOBAL");
    let mut c = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.0"));
    let completion = crate::fixture::eval(
        &mut c,
        "set argv parent; interp create child; child eval {info exists argv}",
    );
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.string_bytes().as_ref(), b"0");
}
