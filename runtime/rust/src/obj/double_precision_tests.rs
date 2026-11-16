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

use crate::interp::{Code, Interp};
use tcl_dialect::TclVersion;

fn run(interp: &mut Interp, source: &str) -> String {
    assert_eq!(
        interp.eval_str(source.as_bytes()),
        Code::Ok,
        "{source}: {:?}",
        interp.result_bytes()
    );
    String::from_utf8(interp.result_bytes()).unwrap()
}

#[test]
fn native_double_precision_is_lazy_shared_and_preserves_cached_strings() {
    for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        assert_eq!(
                run(
                    &mut interp,
                    "set ::tcl_precision 12; set x 1.0; set d [expr {$x/3}]; set ::tcl_precision 4; list $d"
                ),
                "0.3333"
            );
        assert_eq!(
            run(&mut interp, "set ::tcl_precision 12; list $d"),
            "0.3333"
        );
        assert_eq!(
                run(
                    &mut interp,
                    "interp create child; child eval {set ::tcl_precision 8}; list $::tcl_precision [expr {$x/3}]"
                ),
                "8 0.33333333"
            );
        assert_eq!(
            run(
                &mut interp,
                "set d [expr {$x/7}]; child eval {set ::tcl_precision 12}; list $d"
            ),
            "0.142857142857"
        );
        assert_eq!(
            run(
                &mut interp,
                "unset ::tcl_precision; list [info exists ::tcl_precision] $::tcl_precision"
            ),
            "1 12"
        );
        assert_eq!(
                run(
                    &mut interp,
                    "proc precision_alias {} {upvar #0 ::tcl_precision p; set p 4}; precision_alias; list $::tcl_precision"
                ),
                "4"
            );
        assert_eq!(
            run(
                &mut interp,
                "catch {set ::tcl_precision 18} problem; list $problem $::tcl_precision"
            ),
            "{can't set \"::tcl_precision\": improper value for precision} 4"
        );
    }
}

#[test]
fn binary_scan_doubles_materialise_only_at_the_selected_precision() {
    for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        assert_eq!(
                run(
                    &mut interp,
                    "set ::tcl_precision 12; set x 1.0; set bytes [binary format d [expr {$x/3}]]; binary scan $bytes d d; set ::tcl_precision 4; list $d"
                ),
                "0.3333"
            );
        assert_eq!(
                run(
                    &mut interp,
                    "set ::tcl_precision 12; binary scan $bytes d* ds; set ::tcl_precision 8; list [lindex $ds 0]"
                ),
                "0.33333333"
            );
        assert_eq!(
                run(
                    &mut interp,
                    "set ::tcl_precision 4; set original [expr {$x/3}]; binary scan [binary format d $original] d copy; set ::tcl_precision 12; list $original $copy"
                ),
                "0.333333333333 0.333333333333"
            );
        assert_eq!(
                run(
                    &mut interp,
                    "set ::tcl_precision 4; set original [expr {$x/3}]; binary scan [binary format d* [list $original]] d* copies; set ::tcl_precision 12; list $original [lindex $copies 0]"
                ),
                "0.333333333333 0.333333333333"
            );
    }
}

#[test]
fn native_precision_array_elements_share_the_hidden_trace_state() {
    for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        assert_eq!(
                run(
                    &mut interp,
                    "set ::tcl_precision 12; unset ::tcl_precision; set ::tcl_precision(x) 4; set x 1.0; list [expr {$x/3}] $::tcl_precision(x)"
                ),
                "0.3333 4"
            );
        assert_eq!(
                run(
                    &mut interp,
                    "set ::tcl_precision(x) 12; upvar #0 ::tcl_precision(x) p; set p 4; list [expr {$x/3}] $p $::tcl_precision(x)"
                ),
                "0.333333333333 4 12"
            );
        assert_eq!(
                run(
                    &mut interp,
                    "proc adjust {n1 n2 op} {set ::tcl_precision(x) 8}; trace variable ::tcl_precision(x) w adjust; set ::tcl_precision(x) 4; list [expr {$x/3}] $::tcl_precision(x)"
                ),
                "0.3333 4"
            );
    }
}

#[test]
fn native_fixed_double_policies_ignore_ordinary_precision_variables() {
    for profile in ["tcl9.0", "tcl9.1", "jim"] {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect(profile));
        let expected = if profile == "jim" {
            "0.333333333333"
        } else {
            "0.3333333333333333"
        };
        assert_eq!(
            run(
                &mut interp,
                "set tcl_precision 4; set x 1.0; list [expr {$x/3}]"
            ),
            expected
        );
    }
}
