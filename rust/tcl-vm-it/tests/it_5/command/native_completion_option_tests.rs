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

//! Live error metadata placement in return options, compared with tclsh.

use crate::prelude::*;

// tclsh 8.5.19, 8.6.18, 9.0.4 and 9.1.0 output for each script.
const LIVE_METADATA_CASES: [(&str, [&str; 4]); 10] = [
    (
        "array set A {k 1}; list [catch {lappend A(nk)} m o] $m $o",
        [
            "0 {} {-code 0 -level 0}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
        ],
    ),
    (
        "list [catch {incr absent} m o] $m $o",
        [
            "0 1 {-code 0 -level 0}",
            "0 1 {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 1 {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 1 {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
        ],
    ),
    (
        "list [catch {lappend absent} m o] $m $o",
        [
            "0 {} {-code 0 -level 0}",
            "0 {} {-code 0 -level 0}",
            "0 {} {-code 0 -level 0}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
        ],
    ),
    (
        "list [catch {return -level 0 -errorcode X ok} m o] $m $o",
        [
            "0 ok {-errorcode X -code 0 -level 0}",
            "0 ok {-errorcode X -code 0 -level 0}",
            "0 ok {-errorcode X -code 0 -level 0}",
            "0 ok {-errorcode X -code 0 -level 0}",
        ],
    ),
    (
        "proc p {} {array set A {k 1}; list [catch {lappend A(nk)} m o] $m $o}; p",
        [
            "0 {} {-code 0 -level 0}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
        ],
    ),
    (
        "proc p {} {list [catch {incr absent} m o] $m $o}; p",
        [
            "0 1 {-code 0 -level 0}",
            "0 1 {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 1 {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 1 {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
        ],
    ),
    (
        "proc tr args {error boom}; set t 1; trace add variable t read tr; list [catch {lappend t} m o] $m $o",
        [
            "0 {} {-code 0 -level 0}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME}}",
            "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
        ],
    ),
    (
        "proc tr args {error boom}; set t 1; trace add variable t read tr; list [catch {lappend t x} m o] $m $o",
        [
            "0 x {-code 0 -level 0 -errorcode NONE -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
            "0 x {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
            "0 x {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
            "0 x {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
        ],
    ),
    (
        "proc tr args {error boom}; set t 1; trace add variable t read tr; set c lappend; list [catch {$c t x} m o] $m $o",
        [
            "0 x {-code 0 -level 0 -errorcode NONE -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
            "0 x {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
            "0 x {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
            "0 x {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr t {} read\"\n    (read trace on \"t\")} -errorline 1}",
        ],
    ),
    (
        "proc q {} {list [catch {return -level 0 -errorcode X ok} m o] $m $o}; q",
        [
            "0 ok {-errorcode X -code 0 -level 0}",
            "0 ok {-errorcode X -code 0 -level 0}",
            "0 ok {-errorcode X -code 0 -level 0}",
            "0 ok {-errorcode X -code 0 -level 0}",
        ],
    ),
];

/// Live error metadata on a successful completion follows `-code` and
/// `-level`, as `Tcl_GetReturnOptions` appends `iPtr->errorCode` and
/// `errorInfo` after them, while a carried `return -errorcode` stays first.
/// A read trace's failure keeps its code through the command's return.
#[test]
fn successful_completions_place_live_error_metadata_after_control_keys() {
    let mut mismatches = Vec::new();
    for (source, expected) in LIVE_METADATA_CASES {
        for (release, expected) in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]
            .into_iter()
            .zip(expected)
        {
            let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(release));
            let completion = crate::fixture::eval(&mut vm, source);
            let actual = completion.result.to_str();
            if completion.code != Code::Ok || actual.as_ref() != expected {
                mismatches.push(format!(
                    "{release}: {source}\n  got  {actual}\n  want {expected}"
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// `array get` swallows a failed element read trace; its live metadata
/// follows the control keys (tclsh 8.6.18, 9.0.4 and 9.1.0).
#[test]
fn array_get_trace_failure_places_live_metadata_after_control_keys() {
    let source = "proc tr args {error boom}; array set A {x 1}; trace add variable A read tr; list [catch {array get A} m o] $m $o";
    let expected = "0 {} {-code 0 -level 0 -errorcode {TCL READ VARNAME} -errorinfo {boom\n    while executing\n\"error boom\"\n    (procedure \"tr\" line 1)\n    invoked from within\n\"tr A x read\"\n    (read trace on \"A(x)\")} -errorline 1}";
    for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(release));
        let completion = crate::fixture::eval(&mut vm, source);
        assert_eq!(completion.code, Code::Ok, "{release}");
        assert_eq!(completion.result.to_str().as_ref(), expected, "{release}");
    }
}
