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

use crate::prelude::*;

#[test]
fn bootstrap_seeds_the_release_tcloo_version_variables() {
    // tclsh8.6.18 prints `1.1.0 1.1.0 1.1.0`; tclsh9.0.4 and 9.1.0 print
    // `1.3 1.3.1 1.3.1`.
    for (dialect, expected) in [
        ("tcl8.6", "1.1.0 1.1.0 1.1.0"),
        ("tcl9.0", "1.3 1.3.1 1.3.1"),
        ("tcl9.1", "1.3 1.3.1 1.3.1"),
    ] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(dialect));
        let reached = vm
            .eval_source("list $::oo::version $::oo::patchlevel [package present TclOO]")
            .unwrap();
        assert_eq!(reached.code, Code::Ok, "{dialect}");
        assert_eq!(reached.result.to_str().as_ref(), expected, "{dialect}");
    }
}
