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

use super::*;

const RELEASES: [&str; 3] = ["tcl8.6", "tcl9.0", "tcl9.1"];

/// The `(code, variable)` pairs of the variable-lifecycle diagnostics.
fn variable_diagnostics(src: &str, release: &str) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = analyser_diags(src, release)
        .into_iter()
        .filter(|(code, _, _)| matches!(code.as_str(), "W210" | "W211" | "W220"))
        .map(|(code, message, _)| {
            let variable = message.split('\'').nth(1).unwrap_or_default().to_owned();
            (code, variable)
        })
        .collect();
    found.sort();
    found
}

#[test]
fn a_class_variable_is_object_state_in_every_body_of_its_class() {
    crate::install_surfaces();
    // A class-level `variable` is linked in every method, constructor
    // and destructor of the class, and another body may have stored it:
    // reading it is no defect, storing it is no dead store, and a read
    // after it is still examined. Each row is the body set of one class
    // and the diagnostics the release's own interpreter confirms:
    // tclsh 8.6.18, 9.0.4 and 9.1.0 raise `can't read "other"` from
    // `Buggy`, `can't read "nope"` from the constructor and
    // `can't read "gone"` from the destructor, and nothing else.
    let table: [(&str, &[(&str, &str)]); 3] = [
        (
            concat!(
                "oo::class create Vector3d {\n",
                "    variable _x\n",
                "    constructor {x} { ::set _x $x }\n",
                "    method DotProduct {other} { ::return [::expr {$_x * $other}] }\n",
                "    method Buggy {type} { ::return [::expr {$_x * $other}] }\n",
                "}\n",
            ),
            &[("W210", "other")],
        ),
        (
            concat!(
                "oo::class create K {\n",
                "    variable _x\n",
                "    constructor {a} { ::set _x $a; ::set tmp $nope }\n",
                "    destructor { ::set q $gone; ::set _x }\n",
                "}\n",
            ),
            &[("W210", "gone"), ("W210", "nope")],
        ),
        // The same class once an object is created and the method runs:
        // the constructor's store reaches the method's read.
        (
            concat!(
                "oo::class create C {\n",
                "    variable _x\n",
                "    constructor {} { ::set _x 1 }\n",
                "    method m {} { ::return [::expr {$_x * $other}] }\n",
                "}\n",
                "C create c\n",
                "c m\n",
            ),
            &[("W210", "other")],
        ),
    ];
    for release in RELEASES {
        for (src, expected) in table {
            let expected: Vec<(String, String)> = expected
                .iter()
                .map(|(code, variable)| ((*code).to_owned(), (*variable).to_owned()))
                .collect();
            assert_eq!(
                variable_diagnostics(src, release),
                expected,
                "{release}: {src}"
            );
        }
    }
}
