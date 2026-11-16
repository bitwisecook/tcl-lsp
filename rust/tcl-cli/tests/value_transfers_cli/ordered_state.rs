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

//! The ordered evaluation state: what a statement stores before an error, what a `catch` completed with and the prefix rule, against `tclsh`.

use super::*;

/// The ordered evaluation state's exit evidence: `tcl explore --show sccp`
/// over the expression that reads `x`, increments it and reads it again
/// prints the expression's value, 5, as `r#1` and the last write the
/// expression made, 2, as `x#2`.
#[test]
fn explore_sccp_prints_the_ordered_state() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set x 1; set r [expr {$x + [incr x] + $x}]; return $x}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(text.contains("r#1 = const(5)"), "{text}");
    assert!(text.contains("x#2 = const(2)"), "{text}");
}

/// The completion paths' exit evidence: `tcl explore --show sccp` over a
/// procedure that keeps the code of a `catch` in a substitution prints the
/// code, 1, as `c#1` and the message the script raised, `boom`, as `m#1`, and
/// the route the command was run on, `catch-protected`, answering `evaluated`.
#[test]
fn explore_sccp_prints_what_a_catch_completed_with() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set c [catch {error boom} m]; return $c}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(text.contains("c#1 = const(1)"), "{text}");
    assert!(text.contains("m#1 = const('boom')"), "{text}");
    assert!(
        text.contains("route catch: direct catch-protected (registry)"),
        "{text}"
    );
    assert!(text.contains("· answer: evaluated"), "{text}");
}

/// `tcl opt` over programs whose `catch` stands in a substitution forwards
/// what the script completed with and keeps what it wrote: the code and the
/// message, the value a body's `incr` left, and the condition #2231 reads
/// after one, each printing under tclsh 8.4 to 9.1 what the original prints.
#[test]
fn opt_forwards_what_a_nested_catch_completed_with() {
    let programs = [
        (
            "proc p {} {\n    set c [catch {error boom} m]\n    puts \"$c|$m\"\n}\np\n",
            "1|boom\n",
            "puts \"1|boom\"",
        ),
        (
            "set x 1\nset c [catch {incr x} m]\nputs \"$c|$m\"\nputs $x\n",
            "0|2\n2\n",
            "puts \"0|2\"\nputs 2",
        ),
        (
            "set x 1\nset c [catch {incr x}]\nif {$x == 2} {puts two} else {puts other}\n",
            "two\n",
            "puts two",
        ),
    ];
    for (source, printed, forwarded) in programs {
        for (series, tclsh) in tclshs_from("8.4") {
            let optimised = opt_under(source, series);
            let statements = statements_of(&optimised);
            assert!(statements.contains(forwarded), "tcl{series}:\n{optimised}");
            for program in [source, optimised.as_str()] {
                assert_eq!(
                    run_tclsh(&tclsh, program),
                    Some((true, printed.to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}

/// A `catch` the flow graph lowers into a procedure's blocks — straight-line
/// statements — ends at a statement with no words of its own, and the solver
/// evaluates the `catch` as it does the one the graph keeps whole:
/// `tcl explore --show sccp` over `catch {set v 1} r` prints `r#1` as the 1 its
/// script returned, and over `set x 1; catch {incr x} m` prints `m#1` as 2 and
/// `x#2` as 2, where the state after the script would give 3.
#[test]
fn explore_sccp_prints_what_a_flattened_catch_returned() {
    let result = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {catch {set v 1} r; return $r}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(result.contains("r#1 = const(1)"), "{result}");
    assert!(
        result.contains("route catch: direct catch-protected (registry)"),
        "{result}"
    );
    assert!(result.contains("· answer: evaluated"), "{result}");

    let before = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set x 1; catch {incr x} m; return $m}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(before.contains("m#1 = const(2)"), "{before}");
    assert!(before.contains("x#2 = const(2)"), "{before}");
}

/// The value a flattened `catch` script ends with is the result variable's, so
/// the store it ends with is not one nothing reads: `tcl opt` keeps `set v $c`
/// where the result is a parameter's, and a global, and prints under tclsh 8.4
/// to 9.1 what the original prints.
#[test]
fn opt_keeps_the_store_a_flattened_catch_returns() {
    let programs = [
        (
            "proc p {c} {\n    catch {set v $c} m\n    puts \"<$m>\"\n}\np 7\n",
            "<7>\n",
            "set v ",
        ),
        (
            "set g 0\nproc p {c} {\n    global g\n    catch {set v $c} g\n}\np 7\nputs $g\n",
            "7\n",
            "set v ",
        ),
    ];
    for (source, printed, kept) in programs {
        for (series, tclsh) in tclshs_from("8.4") {
            let optimised = opt_under(source, series);
            assert!(
                statements_of(&optimised).contains(kept),
                "tcl{series}:\n{optimised}"
            );
            for program in [source, optimised.as_str()] {
                assert_eq!(
                    run_tclsh(&tclsh, program),
                    Some((true, printed.to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}

/// The interface page's seven ordered-state programs through `tcl opt`,
/// each printing under tclsh 8.4 to 9.1 what the original prints: the value
/// of the expression and what `x` holds after it, 5 and 2, 0 and 1, 21 and
/// 10, 5 and 3, 2 and 2, 3 and 2, and — for an error between a write and the
/// end of the expression inside a `catch` — the message and 2. Both reads
/// after the expression are the proved values, and the error path's is not.
#[test]
fn opt_prints_what_tclsh_prints_for_the_seven_ordered_state_programs() {
    let expressions = [
        ("{$x + [incr x] + $x}", "5", "2"),
        ("{0 && [incr x]}", "0", "1"),
        ("{$x + [set x 10] + $x}", "21", "10"),
        ("{[incr x] + [incr x]}", "5", "3"),
        ("{$x ? [incr x] : [incr x 10]}", "2", "2"),
        ("\"$x + [incr x]\"", "3", "2"),
    ];
    let mut programs: Vec<(String, String, bool)> = expressions
        .iter()
        .map(|(expression, value, after)| {
            (
                format!("set x 1\nset r [expr {expression}]\nputs $r\nputs $x\n"),
                format!("{value}\n{after}\n"),
                true,
            )
        })
        .collect();
    programs.push((
        "set x 1\ncatch {expr {[incr x] + [error mid]}} msg\nputs $msg\nputs $x\n".to_owned(),
        "mid\n2\n".to_owned(),
        false,
    ));
    for (source, printed, forwarded) in &programs {
        for (series, tclsh) in tclshs_from("8.4") {
            let optimised = opt_under(source, series);
            let statements = statements_of(&optimised);
            let (value, after) = printed.split_once('\n').expect("two lines");
            if *forwarded {
                assert!(
                    statements.contains(&format!("puts {value}\nputs {after}")),
                    "tcl{series}:\n{optimised}"
                );
            } else {
                assert!(
                    statements.contains("set x 1") && statements.contains("puts $x"),
                    "tcl{series}: no earlier x is forwarded\n{optimised}"
                );
            }
            for program in [source.as_str(), optimised.as_str()] {
                assert_eq!(
                    run_tclsh(&tclsh, program),
                    Some((true, printed.clone())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}

/// The absent-cell table's `unset` line through `tcl explore --show sccp`:
/// `unset p nosuch q` unbinds `p`, raises on the absent name and leaves `q`,
/// so the route answers an error after one store, and that store is the
/// unbinding of `p`.
#[test]
fn explore_sccp_prints_what_an_unset_stores_before_its_error() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set p 1; set q 2; unset p nosuch q}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(
        text.contains("route unset: direct variable-unset (registry)"),
        "{text}"
    );
    assert!(
        text.contains("· answer: evaluated: error after 1 store"),
        "{text}"
    );
    assert!(
        text.contains("· path error after 1 store: unbind p"),
        "{text}"
    );
}

/// The ordered-state witness whose expression raises after a write, in a
/// procedure, where the `catch` is lowered into blocks: `tcl explore --show
/// sccp` proves `x` 2 on the error path — the expression's route answers an
/// error after one store, the write of 2 — and so `x` after the `catch`.
#[test]
fn explore_sccp_prints_x_exact_on_the_error_path() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set x 1; catch {expr {[incr x] + [error mid]}} msg; return $x}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(text.contains("route expr: expression tcl.expr"), "{text}");
    assert!(
        text.contains("· answer: evaluated: error after 1 store"),
        "{text}"
    );
    assert!(
        text.contains("· path error after 1 store: write x = 2"),
        "{text}"
    );
    assert!(text.contains("x#3 = const(2)"), "{text}");
    assert!(text.contains("msg#1 = const('mid')"), "{text}");
}

/// The programs of the design page's prefix rule through the shipped binary,
/// each in a procedure: a command that raises part-way leaves the stores
/// before its failure and nothing after — save `scan`, which goes on past
/// the array — and `tcl opt` keeps what each program prints under every
/// release from the first that has its command.
#[test]
fn opt_prints_what_tclsh_prints_for_the_prefix_programs() {
    let programs: [(&str, &str, &str, &str, &str); 10] = [
        (
            "set a old\n    array set b {k keep}",
            "lassign {new second} a b",
            "puts \"$a $b(k)\"",
            "new keep\n",
            "8.5",
        ),
        (
            "array set b {k keep}\n    set a old\n    set c old",
            "lassign {x y z} a b c",
            "puts \"$a $c\"",
            "x old\n",
            "8.5",
        ),
        (
            "array set b {k keep}\n    set a old",
            "foreach {a b} {new second} {set inside 1}",
            "puts \"$a [info exists inside]\"",
            "new 0\n",
            "8.4",
        ),
        (
            "array set b {k keep}\n    set a old",
            "scan {1 2} {%d %d} a b",
            "puts $a",
            "1\n",
            "8.4",
        ),
        (
            "array set a {k keep}\n    set b old",
            "scan {1 2} {%d %d} a b",
            "puts \"$a(k) $b\"",
            "keep 2\n",
            "8.4",
        ),
        (
            "array set b {k keep}\n    set a old",
            "regexp {(x)(y)} xy a b",
            "puts $a",
            "xy\n",
            "8.4",
        ),
        (
            "set p 1\n    set q 2",
            "unset p nosuch q",
            "puts \"[info exists p] $q\"",
            "0 2\n",
            "8.4",
        ),
        (
            "set x 1",
            "append x 2 [error boom]",
            "puts $x",
            "1\n",
            "8.4",
        ),
        (
            "set w 0",
            "set r [expr {1 + [error mid]}]",
            "puts [info exists r]",
            "0\n",
            "8.4",
        ),
        (
            "set x 1",
            "expr {[incr x] + [error mid]}",
            "puts $x",
            "2\n",
            "8.4",
        ),
    ];
    for (before, body, after, printed, first) in programs {
        let source =
            format!("proc p {{}} {{\n    {before}\n    catch {{{body}}} m\n    {after}\n}}\np\n");
        for (series, tclsh) in tclshs_from(first) {
            let optimised = opt_under(&source, series);
            for program in [source.as_str(), optimised.as_str()] {
                assert_eq!(
                    run_tclsh(&tclsh, program),
                    Some((true, printed.to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}

/// #2141's programs through the shipped binary: a write nested in a braced
/// `expr` word is a write of the frame, so `tcl opt` forwards nothing past it
/// to the `puts $x` that follows a `puts` argument — `5` then `2` where it
/// printed `5` then `1` — and keeps the stores the expression reads, so the
/// optimised program neither raises `can't read "n"` nor answers 3 for
/// `[incr n] + [incr n]`; `tcl diag` calls neither store unused.
#[test]
fn opt_keeps_what_the_issue_2141_programs_read() {
    let programs = [
        (
            "set x 1\nputs [expr {$x + [incr x] + $x}]\nputs $x\n",
            "5\n2\n",
            "puts $x",
        ),
        (
            "set x 1\nputs [expr {$x + [set x 10] + $x}]\nputs $x\n",
            "21\n10\n",
            "puts $x",
        ),
        (
            "proc p {} {\n    set n 1\n    set r [expr {$n + [incr n]}]\n    return $r\n}\nputs [p]\n",
            "3\n",
            "set n 1",
        ),
        (
            "proc q {} {\n    set n 1\n    set r [expr {[incr n] + [incr n]}]\n    return $r\n}\nputs [q]\n",
            "5\n",
            "set n 1",
        ),
    ];
    for (source, printed, kept) in programs {
        for (series, tclsh) in tclshs_from("8.4") {
            let optimised = opt_under(source, series);
            assert!(
                statements_of(&optimised).contains(kept),
                "tcl{series}:\n{optimised}"
            );
            for program in [source, optimised.as_str()] {
                assert_eq!(
                    run_tclsh(&tclsh, program),
                    Some((true, printed.to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
        let found = diagnostics_at(source, "tcl8.6", &[]);
        assert!(
            !found.iter().any(|(_, _, code)| code == "W211"),
            "no store is unused: {found:?}\n{source}"
        );
    }
}
