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

//! A failed `oo::define` script keeps the method it defined before failing, compared with tclsh 9.0.4.

use crate::prelude::*;

#[test]
fn property_epochs_follow_mutations_before_definition_failure() {
    // tclsh 9.0.4: `1 LATE live {}`.
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.1"));
    let completion = crate::fixture::eval(
        &mut vm,
        "oo::configurable create C {property a}; C create o; list [catch {oo::define C {::oo::define::method live {} {return};error LATE}} r] $r [info class methods C] [o live]",
    );
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.to_str().as_ref(), "1 LATE live {}");
}
