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

//! Procedure summaries: computed returns, name-argument writes and the summary witnesses.

use super::*;

/// Through the shipped binary, `tcl opt --profile
/// full` folds `[p]` to `foo` — `p`'s return computed, the constant read
/// from its seedless lattice — and `tcl explore --show interproc` prints the
/// summary that answers it, foldable with the return shape `const('foo')`.
/// The original and the optimised program print `foo` under each release's
/// tclsh.
#[test]
fn o103_summary_path_folds_a_computed_return() {
    let source = "proc p {} {set x [string range foobar 0 2]; return $x}\nputs [p]\n";
    let tclshs = tclshs_from("8.4");
    for series in RELEASES {
        let optimised = statements_of(&opt_under(source, series));
        assert!(optimised.contains("puts foo"), "tcl{series}: {optimised}");
        for (_, tclsh) in tclshs.iter().filter(|(found, _)| *found == series) {
            for text in [source, optimised.as_str()] {
                assert_eq!(
                    run_tclsh(tclsh, text),
                    Some((true, "foo\n".to_owned())),
                    "tclsh{series}:\n{text}"
                );
            }
        }
    }
    let summary = run_tcl(&[
        "explore",
        "--source",
        source,
        "--show",
        "interproc",
        "--text",
        "--no-colour",
    ]);
    assert!(summary.contains("foldable: yes"), "{summary}");
    assert!(summary.contains("return shape: const('foo')"), "{summary}");
}

/// `tcl explore --show sccp` prints what `bump n` leaves in `::p`'s `n` as
/// `const(2)`, the call applying `bump`'s summary.
#[test]
fn explore_sccp_prints_what_a_call_leaves_in_its_name_argument() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc bump {name} {upvar 1 $name v; incr v}; proc p {} {set n 1; bump n; return $n}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    let p = text.split("function ::p").nth(1).expect("::p's section");
    assert!(p.contains("n#2 = const(2)"), "{text}");
}

/// Through the shipped binary, `tcl explore --show interproc` prints, among
/// a caller's globals, the global it writes through
/// a qualified or aliased `Name` argument — `bump ::g`, `upvar #0 g x; bump
/// x`, `global g; bump g` — and says a caller whose argument names a place
/// the summary cannot name has none.
#[test]
fn explore_interproc_prints_the_global_a_name_argument_writes() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc bump {name {by 1}} {upvar 1 $name v; incr v $by}\nset g 1\n\
         proc c {} {bump ::g; return $::g}\nproc d {} {upvar #0 g x; bump x}\n\
         proc e {} {global g; bump g}\n\
         proc f {c} {if {$c} {upvar #0 g x} else {upvar #0 k x}; bump x}\nputs [c]\n",
        "--show",
        "interproc",
        "--text",
        "--no-colour",
    ]);
    let transfer = |caller: &str| {
        text.split(&format!("{caller} arity"))
            .nth(1)
            .and_then(|section| section.lines().find(|line| line.contains("transfer:")))
            .unwrap_or_default()
            .to_owned()
    };
    for caller in ["::c", "::d", "::e"] {
        assert!(
            transfer(caller)
                .ends_with("transfer: globals: ::g binds a scalar · completes: ok or error"),
            "{caller}: {text}"
        );
    }
    assert!(
        transfer("::f").ends_with("transfer: none — a call is a barrier"),
        "{text}"
    );
}

/// The `bump` of the summary witnesses.
const BUMP: &str = "proc bump {name {by 1}} {upvar 1 $name v; incr v $by}\n";

/// The seven summary witnesses through the shipped binary (the compiler's
/// `the_seven_summary_witnesses`): under each release's dialect `tcl opt
/// --profile full` leaves the line each call proves or keeps, and the original
/// and the optimised program print alike under that release's tclsh, or stop
/// alike, with nothing printed, where 8.4's `incr` raises on an absent
/// variable.
#[test]
fn the_seven_summary_witnesses() {
    // The program, what it prints, the first release that prints it, and the
    // line `tcl opt` leaves from that release and before it.
    let witnesses = [
        (
            format!("{BUMP}set n 1\nbump n\nbump n 2\nputs $n\n"),
            "4\n",
            "8.4",
            "puts 4",
            "puts 4",
        ),
        (
            "proc reset {name} {upvar 1 $name v; unset v}\nset m 1\nreset m\n\
             if {[info exists m]} {puts bound} else {puts unbound}\n"
                .to_owned(),
            "unbound\n",
            "8.4",
            "if {0} {} else {puts unbound}",
            "if {0} {} else {puts unbound}",
        ),
        (
            format!(
                "{BUMP}proc twice {{name}} {{upvar 1 $name w; bump w; bump w}}\n\
                 set t 1\ntwice t\nputs $t\n"
            ),
            "3\n",
            "8.4",
            "puts 3",
            "puts 3",
        ),
        (
            "proc g {} {set ::counter 5}\ng\nputs $::counter\n".to_owned(),
            "5\n",
            "8.4",
            "puts $::counter",
            "puts $::counter",
        ),
        (
            "proc rec {n} {if {$n <= 0} {return 0}; expr {$n + [rec [expr {$n - 1}]]}}\n\
             puts [rec 4]\n"
                .to_owned(),
            "10\n",
            "8.4",
            "puts 10",
            "puts 10",
        ),
        (
            "proc ctr {} {incr ::hits}\nctr\nctr\nputs $::hits\n".to_owned(),
            "2\n",
            "8.5",
            "puts $::hits",
            "puts $::hits",
        ),
        (
            format!("{BUMP}bump absent\nputs $absent\n"),
            "1\n",
            "8.5",
            "puts 1",
            "puts $absent",
        ),
    ];
    let tclshs = tclshs_from("8.4");
    for (source, printed, first, line, line_before) in &witnesses {
        for series in RELEASES {
            let optimised = statements_of(&opt_under(source, series));
            let expected = if series >= *first { line } else { line_before };
            assert!(
                optimised.lines().any(|found| found == *expected),
                "tcl{series}: {expected}\n{optimised}"
            );
            let output = if series >= *first {
                Some((true, (*printed).to_owned()))
            } else {
                Some((false, String::new()))
            };
            for (_, tclsh) in tclshs.iter().filter(|(found, _)| *found == series) {
                for text in [source.as_str(), optimised.as_str()] {
                    assert_eq!(run_tclsh(tclsh, text), output, "tclsh{series}:\n{text}");
                }
            }
        }
    }
}

/// One summary serves both callers, and #2134's nested caller keeps the
/// callee whole, through the shipped binary: `tcl opt --profile full` leaves
/// `bump`'s `upvar 1 $name v` as written, and the original and the optimised
/// program print `2 11` and `2 11 11` under each release's tclsh.
#[test]
fn two_callers_share_one_summary() {
    let programs = [
        (
            "proc bump {name} {upvar 1 $name v; incr v}\nset n 1\nbump n\n\
             set other 10\nbump other\nputs \"$n $other\"\n",
            "2 11\n",
        ),
        (
            "proc bump {name} {\n  upvar 1 $name v\n  incr v\n}\nset n 1\nset m 10\n\
             bump n\nset z [bump m]\nputs \"$n $m $z\"\n",
            "2 11 11\n",
        ),
    ];
    let tclshs = tclshs_from("8.4");
    for (source, printed) in programs {
        for series in RELEASES {
            let optimised = statements_of(&opt_under(source, series));
            assert!(
                optimised.contains("upvar 1 $name v"),
                "tcl{series}: {optimised}"
            );
            for (_, tclsh) in tclshs.iter().filter(|(found, _)| *found == series) {
                for text in [source, optimised.as_str()] {
                    assert_eq!(
                        run_tclsh(tclsh, text),
                        Some((true, printed.to_owned())),
                        "tclsh{series}:\n{text}"
                    );
                }
            }
        }
    }
}
