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

//! S100 to S102 shimmer warnings: true positives, no false positives and loop paths.

use super::*;

/// Shimmer-warning codes (S100/S101 + S102 thunking).
fn shimmer_codes(src: &str) -> Vec<String> {
    let registry = static_context_for(TCL).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let mut v: Vec<String> = Vec::new();
    for w in find_shimmer_warnings_for_cu(&cu, registry) {
        v.push(w.code.as_str().to_owned());
    }
    for w in find_thunking_warnings_for_cu(&cu, registry) {
        v.push(w.code.as_str().to_owned());
    }
    v.sort();
    v
}

/// Count shimmer warnings carrying `code` (S100/S101/S102).
fn shimmer_count(src: &str, code: &str) -> usize {
    shimmer_codes(src).iter().filter(|c| *c == code).count()
}

// Shimmer detection — S100 / S101 / S102

#[test]
fn shimmer_no_false_positives() {
    crate::install_surfaces();
    // Clean integer code: no shimmer.
    assert_eq!(shimmer_codes("set x 0\nincr x"), Vec::<String>::new());
    // List operations stay list-typed: no shimmer.
    assert_eq!(
        shimmer_codes("set x [list a b c]\nset n [llength $x]"),
        Vec::<String>::new()
    );
    // Integer arithmetic: no shimmer.
    assert_eq!(
        shimmer_codes("set x 42\nset y [expr {$x + 1}]"),
        Vec::<String>::new()
    );
    // `string length` accepts any type — int→strlen does not shimmer.
    assert_eq!(
        shimmer_count("set x 42\nset y [string length $x]", "S100"),
        0
    );
    // boolean → int is a numeric sub-type, not a shimmer.
    assert_eq!(
        shimmer_count("set x true\nset y [expr {$x + 1}]", "S100"),
        0
    );
}

/// The variables `catch` writes hold the script's result and its options
/// dictionary, not the integer completion code `catch` itself returns, so a
/// list or dictionary command over one shimmers nothing — in the top-level
/// script and in a procedure whose body the graph keeps as one statement, with
/// the result variable alone or beside the options variable.
#[test]
fn a_catch_result_variable_holds_no_integer_to_shimmer() {
    crate::install_surfaces();
    for source in [
        "catch {foo} msg\nputs [lindex $msg 0]\n",
        "catch {foo} msg opts\nputs [dict get $opts -code]\nputs [lindex $msg 0]\n",
        "proc p {} {\n catch {if {1} {foo}} msg\n puts [lindex $msg 0]\n}\n",
        "proc p {} {\n catch {set x [foo]} msg\n puts [dict get $msg x]\n}\n",
        "proc p {} {\n catch {foo} msg\n puts [dict get $msg x]\n}\n",
    ] {
        assert_eq!(shimmer_codes(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn shimmer_string_in_numeric_context_s100() {
    crate::install_surfaces();
    // A STRING-typed var used in a numeric expr shimmers (S100 outside a loop).
    assert!(
        shimmer_count(
            "set x [string trim \" hello \"]\nset y [expr {$x + 1}]",
            "S100"
        ) >= 1
    );
    // incr on a STRING var shimmers.
    assert!(shimmer_count("set x [string trim \" hello \"]\nincr x", "S100") >= 1);
}

#[test]
fn shimmer_loop_paths_no_crash() {
    crate::install_surfaces();
    // Shimmer inside a loop (S101 territory) — type-analysis-dependent; assert no
    // panic and a well-formed result.
    let _ = shimmer_codes(
        "set x [list a b c]\nfor {set i 0} {$i < 3} {incr i} {\n    set n [llength $x]\n    set y [expr {$x + 1}]\n}\n",
    );
    // Consistent integer loop: no S102 thunking.
    assert_eq!(
        shimmer_count(
            "set x 0\nfor {set i 0} {$i < 10} {incr i} {\n    incr x\n}\n",
            "S102"
        ),
        0
    );
    // Oscillating-type loop body — may or may not detect S102 depending on
    // inference depth; assert no panic.
    let _ = shimmer_codes(
        "set x 0\nfor {set i 0} {$i < 10} {incr i} {\n    set x [string length [string repeat a $x]]\n}\n",
    );
}
