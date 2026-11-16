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

//! The route each statement's value transfer takes, as `tcl explore --show sccp` prints it and `tcl opt` forwards it.

use super::*;

/// The route tally, read from `tcl explore --show sccp`: two fused
/// `expr` statements are two expression entries, one nesting a
/// direct-routed `string length`, so `direct 1 · expression 2 ·
/// implementation 0`.
#[test]
fn explore_sccp_prints_the_route_tally() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set r [expr {\"x\"}]; set n [expr {[string length abcdef] * 2}]}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(text.contains("r#1 = const('x')"), "{text}");
    assert!(text.contains("n#1 = const(12)"), "{text}");
    assert!(
        text.contains("routes entered: direct 1 · expression 2 · implementation 0"),
        "{text}"
    );
}

/// `tcl explore --show sccp` prints the
/// folded type a route's representation evidence states, beside the
/// value, and says nothing where a route states none. `binary format` is
/// the interface page's program (2) — `h#1 = const('ABCDEF')` typed
/// `bytearray (constructed)` — read beside `format`'s `string
/// (constructed)` and `list`'s `list (constructed)` so "types" is
/// not one route's alone; a plain literal constructs nothing and prints
/// no type line at all.
#[test]
fn explore_sccp_prints_folded_types() {
    let binary = run_tcl(&[
        "explore",
        "--source",
        "set h [binary format H* 414243444546]",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(binary.contains("h#1 = const('ABCDEF')"), "{binary}");
    assert!(binary.contains("type: bytearray (constructed)"), "{binary}");

    let format = run_tcl(&[
        "explore",
        "--source",
        "set s [format %d 5]",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(format.contains("s#1 = const(5)"), "{format}");
    assert!(format.contains("type: string (constructed)"), "{format}");

    let list = run_tcl(&[
        "explore",
        "--source",
        "set l [list a b c]",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(list.contains("l#1 = const('a b c')"), "{list}");
    assert!(list.contains("type: list (constructed)"), "{list}");

    let literal = run_tcl(&[
        "explore",
        "--source",
        "set n 5",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(literal.contains("n#1 = const(5)"), "{literal}");
    assert!(!literal.contains("type:"), "{literal}");
}

/// The `tcl explore` witnesses — program (3)'s `incr` route
/// twice, a release-ambiguous decline, and `llength`'s direct route — each
/// hand-verified and pinned here. The decline was
/// `f5-irules`'s until ruling 8 gave iRules its declared 8.4 base: it now
/// folds `incr` over `010` to 9, as tclsh 8.4 does, and `tk`, a profile
/// that declares no release, is the one that declines.
#[test]
fn explore_sccp_prints_the_route_of_each_statement() {
    let program_three = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set n 1; incr n; incr n 2; return $n}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(program_three.contains("n#3 = const(4)"), "{program_three}");
    assert_eq!(
        program_three
            .matches("route incr: direct cell-increment (registry)")
            .count(),
        2,
        "{program_three}"
    );
    assert_eq!(
        program_three.matches("· answer: evaluated").count(),
        2,
        "{program_three}"
    );

    let leading_zero = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set z 010; incr z}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
        "--dialect",
        "tk",
    ]);
    assert!(
        leading_zero.contains("· answer: declined: release-ambiguous: numeral-grammar"),
        "{leading_zero}"
    );
    let irules = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set z 010; incr z}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
        "--dialect",
        "f5-irules",
    ]);
    assert!(irules.contains("z#2 = const(9)"), "{irules}");

    let llength = run_tcl(&[
        "explore",
        "--source",
        "set r [llength {a b}]",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(
        llength.contains("route llength: direct list-length (registry)"),
        "{llength}"
    );
}

/// An iRules pure function folds through its registry-owned route over the
/// shared core the simulator runs: `b64encode abc` in `RULE_INIT` is
/// `YWJj`, and the statement names the route; `lset` writes the list it
/// computes.
#[test]
fn explore_sccp_folds_the_irules_cores_and_the_list_cell_updates() {
    let irules = run_tcl(&[
        "explore",
        "--dialect",
        "f5-irules",
        "--source",
        "when RULE_INIT {set h [b64encode abc]}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(irules.contains("h#1 = const('YWJj')"), "{irules}");
    assert!(
        irules.contains("route b64encode: direct base64-encode (registry)"),
        "{irules}"
    );
    let lset = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set l {a b c}; lset l 1 B; return $l}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(lset.contains("l#2 = const('a B c')"), "{lset}");
    assert!(
        lset.contains("route lset: direct list-set (registry)"),
        "{lset}"
    );
}

/// `tcl explore --show sccp --text` lists what a statement stores on each
/// completion path: `lassign` over an array `c` stores its first target and
/// stops, and over a place whose kind is not proven it may stop at any of its
/// targets, which its existence transfer lists beside the normal path.
#[test]
fn explore_sccp_prints_what_each_completion_path_stores() {
    let array = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {array set c {k keep}; set a old; lassign {new second} a c}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(
        array.contains("· answer: evaluated: error after 1 store"),
        "{array}"
    );
    assert!(
        array.contains("· path error after 1 store: write a = new"),
        "{array}"
    );
    assert!(
        array.contains("· path normal: write c(k) = keep"),
        "a statement that stores on its normal path alone lists that one:\n{array}"
    );

    let unproven = run_tcl(&[
        "explore",
        "--source",
        "proc p {c} {set a old; if {$c} {set b 1}; lassign {x y} a b}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(
        unproven.contains("· path normal: write a = x; write b = y"),
        "{unproven}"
    );
    assert!(
        unproven.contains("· path error: bind a as scalar; may-bind b as scalar"),
        "{unproven}"
    );
}

/// The existence branch decides inside the
/// fixed point — `unset x` leaves `x` provably unbound, so `[info exists
/// x]` folds `False` as an ordinary `Applied` branch — and the `if`'s
/// true block (`puts yes`) is outside `executable blocks`.
#[test]
fn explore_sccp_prints_the_existence_branch_decided() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set x 1; unset x; if {[info exists x]} {puts yes}}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(
        text.contains("branch entry_1: False") && text.contains("condition: [info exists x]"),
        "the existence branch decides inside the fixed point:\n{text}"
    );
    assert!(
        text.contains("executable blocks: entry_1, exit_5, if_end_2, if_next_4"),
        "the true block is outside executable blocks:\n{text}"
    );
    assert!(
        !text.contains("if_then_3"),
        "the if's true block never appears:\n{text}"
    );
}

/// `tcl opt --profile full --dialect tcl8.6` forwards program
/// (3)'s constant return into its call site (O100 / O103).
#[test]
fn opt_forwards_program_three() {
    let out = run_tcl(&[
        "opt",
        "--source",
        "proc p {} {set n 1; incr n; incr n 2; return $n}\nputs [p]",
        "--profile",
        "full",
        "--dialect",
        "tcl8.6",
    ]);
    assert!(out.contains("puts 4"), "{out}");
}

/// #2214: a `::`-qualified global a nested `incr` writes is a
/// global write in its procedure's summary, so `set hits 0` and `puts
/// $hits` around `bump`'s `[incr ::hits]` are both kept — the program
/// prints `1` under `tclsh8.4` to `tclsh9.1`, as the original does.
#[test]
fn opt_keeps_a_global_a_nested_increment_writes() {
    let source = "set hits 0\nproc bump {} {set y [incr ::hits]; return $y}\nbump\nputs $hits\n";
    let out = run_tcl(&["opt", "--source", source, "--profile", "full"]);
    assert!(out.contains("set hits 0"), "{out}");
    assert!(out.contains("puts $hits"), "{out}");
}
