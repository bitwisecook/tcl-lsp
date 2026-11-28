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

//! Errors and non-local completions: throws between writes, `try` handlers and `finally`, and level-zero returns.

use super::*;

/// A command of a `catch` body may fail wherever it stands, and the handler is
/// thrown to with what the body has stored by then: after `set x 1; catch
/// {set x 2; foo; set x 1}` the handler may see `x` at 2 — `foo` raises
/// between the stores — so the lattice holds no constant for it, the store of
/// 2 is not dead, and the program prints what tclsh prints. The optimiser
/// had deleted `set x 2` and `set a 2`, which print `1` and `a=3` where tclsh
/// 8.4 to 9.1 print `2` and `a=2`, and the handler's state was joined from the
/// state before the body and the state at its end alone.
#[test]
fn a_throw_between_two_writes_is_a_state_the_handler_sees() {
    crate::install_surfaces();
    let programs = [
        (
            "proc foo {} {error x}\nproc p {} {\n    set x 1\n    catch {set x 2; foo; set x 1}\n    puts $x\n}\np\n",
            "2\n",
            "set x 2",
        ),
        (
            "proc foo {} {error x}\nproc p {} {\n    set a 1\n    catch {set a 2; foo; set a 3}\n    puts \"a=$a\"\n}\np\n",
            "a=2\n",
            "set a 2",
        ),
    ];
    for (source, printed, store) in programs {
        let name = if source.contains("set a 1") { "a" } else { "x" };
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, _) = optimised(source, dialect);
            assert!(
                rewritten.contains(store),
                "{dialect}: {store} stays\n{rewritten}"
            );
            let held = last_value(source, dialect, "::p", name);
            assert!(
                !matches!(
                    held,
                    LatticeValue::Const(ConstValue::Number(NumericValue::Int(1 | 3)))
                ),
                "{dialect}: the handler may see 2, not a constant 1 or 3: {held:?}"
            );
        }
        prints_under_releases_from(source, printed, "8.4");
    }
}

/// A statement after one that certainly raises never runs: after `array set b
/// {k v}`, the body `lassign {x y} a b; set z 1` stops at the array, so `z`
/// is never set where the handler runs, in a `catch` lowered into blocks and
/// in the body of a `try`, which stays one block.
#[test]
fn a_statement_after_a_certain_error_never_runs() {
    crate::install_surfaces();
    let in_catch = "proc p {} {\n    array set b {k v}\n    catch {lassign {x y} a b; set z 1} m\n    puts [info exists z]\n}\np\n";
    let in_try = "proc p {} {\n    array set b {k v}\n    try {lassign {x y} a b; set z 1} on error {} {}\n    puts [info exists z]\n}\np\n";
    for (source, first) in [(in_catch, "8.5"), (in_try, "8.6")] {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let unit = unit_of(source, dialect);
            assert_eq!(
                last_existence(&unit, "::p", "z"),
                UNBOUND,
                "{dialect}: z is never set\n{source}"
            );
        }
        prints_under_releases_from(source, "0\n", first);
    }
}

/// An error outside any `catch` or `try` is the procedure's own, with no
/// handler to be thrown to, and nothing that follows it is claimed: after
/// `lassign {x y} a b` over an array `b` a `catch` the procedure goes on to
/// run is still reached, as it was before the prefix rule.
#[test]
fn a_certain_error_outside_a_handler_claims_nothing() {
    crate::install_surfaces();
    let source = "proc p {} {\n    array set b {k v}\n    lassign {x y} a b\n    catch {set z 1}\n    return $z\n}\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let unit = unit_of(source, dialect);
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("the procedure");
        let reached = function
            .sccp
            .executable_blocks
            .iter()
            .any(|block| function.ssa.block_name(*block).starts_with("catch_body"));
        assert!(reached, "{dialect}: the body of the catch is reached");
    }
}

/// One program of [`a_level_zero_return_completes_where_it_stands`].
struct LevelZeroReturn {
    pub(super) source: &'static str,
    /// Statements a level-0 `return` leaves to run, which `tcl opt` keeps.
    kept: &'static [&'static str],
    /// Statements after a level-0 jump or error, which never run and O107
    /// removes.
    gone: &'static [&'static str],
    pub(super) printed: &'static str,
    /// The first release that runs it.
    pub(super) first: &'static str,
}

/// The programs of [`a_level_zero_return_completes_where_it_stands`].
const LEVEL_ZERO_RETURNS: [LevelZeroReturn; 5] = [
    LevelZeroReturn {
        source: "proc p {} {return -level 0 -code ok x; puts \"after in p\"; return done}\nputs [p]\nreturn -level 0 -code ok top\nputs \"after at top\"\n",
        kept: &["puts \"after in p\"", "puts \"after at top\""],
        gone: &[],
        printed: "after in p\ndone\nafter at top\n",
        first: "8.5",
    },
    LevelZeroReturn {
        source: "try {return -level 0 -code 0 x} on ok {v} {puts \"ok $v\"}\ntry {return -level 0 -code 2 x} on return {v} {puts \"ret $v\"}\n",
        kept: &["puts \"ok $v\"", "puts \"ret $v\""],
        gone: &[],
        printed: "ok x\nret x\n",
        first: "8.6",
    },
    LevelZeroReturn {
        source: "proc p {} {\n    set n 0\n    foreach i {1 2 3 4} {\n        if {$i == 2} {return -level 0 -code continue}\n        if {$i == 4} {return -level 0 -code break}\n        incr n\n    }\n    return $n\n}\nputs [p]\n",
        kept: &["incr n"],
        gone: &[],
        printed: "2\n",
        first: "8.5",
    },
    LevelZeroReturn {
        source: "proc e {} {\n    return -level 0 -code error -errorcode {A B} boom\n    set z 1\n    puts never\n}\nputs [catch e m]\nputs $m\nputs $::errorCode\n",
        kept: &[],
        gone: &["puts never"],
        printed: "1\nboom\nA B\n",
        first: "8.5",
    },
    LevelZeroReturn {
        source: "proc p {} {\n    foreach i {1 2 3} {\n        return -level 0 -code break\n        set xb 1\n    }\n    foreach i {1 2} {\n        return -level 0 -code continue\n        set yc 1\n    }\n    return [list [info exists xb] [info exists yc]]\n}\nputs [p]\nproc q {} {\n    catch {return -level 0 -code error e; set xe 1}\n    return [info exists xe]\n}\nputs [q]\n",
        kept: &["return [list [info exists xb] [info exists yc]]"],
        gone: &["set xb 1", "set yc 1"],
        printed: "0 0\n0\n",
        first: "8.5",
    },
];

/// A `return` at level 0 completes where it stands, with its code (#2357):
/// `ok` runs on to the next statement, `error` raises, and `break` or
/// `continue` leaves the loop around it. One decoding of `return`'s options,
/// the registry's, answers the lowering, the CFG and the solver alike, and
/// read a level-0 `return` as the procedure's exit no longer: O107 deleted
/// the statements after `return -level 0 -code ok x` in a procedure and at
/// the top level, and the `try` handlers its code selects. In a procedure, at
/// the top level, in a loop and under `try`, each program prints what tclsh
/// prints before and after `tcl opt`, from the first release with `-level`,
/// and what follows a level-0 jump or error in its block never runs, which
/// O107 removes. Under 8.4, which reads three options, `-level` is rejected,
/// and `catch` reports it.
#[test]
fn a_level_zero_return_completes_where_it_stands() {
    crate::install_surfaces();
    for program in &LEVEL_ZERO_RETURNS {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, rewrites) = optimised(program.source, dialect);
            for statement in program.kept {
                assert!(
                    rewritten.contains(statement),
                    "{dialect}: `{statement}` runs after a level-0 return\n{rewritten}\n{rewrites:#?}"
                );
            }
            for statement in program.gone {
                assert!(
                    !rewritten.contains(statement),
                    "{dialect}: `{statement}` never runs after a level-0 jump or error\n{rewritten}\n{rewrites:#?}"
                );
            }
        }
        prints_under_releases_from(program.source, program.printed, program.first);
    }
    let rejected = "proc p {} {catch {return -level 0 -code ok x} m; return $m}\nputs [p]\n";
    for (series, tclsh) in releases_on_path() {
        let printed = if series == "8.4" {
            "bad option \"-level\": must be -code, -errorcode, or -errorinfo\n"
        } else {
            "x\n"
        };
        let (rewritten, _) = optimised(rejected, &dialect_of(series));
        for program in [rejected, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, printed.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// A `try` body that cannot fall through is thrown to from the point it
/// raises at, and its first command may fail before it stores anything, with
/// the state the body entered with: after `try {set x [expr {1 / $d}]; error
/// boom} on error {} {}` the handler sees `x` unbound where the division
/// raised, so `info exists x` decides nothing, the program prints `no` for a
/// zero divisor and `yes` for any other, and it does so before and after `tcl
/// opt`.
#[test]
fn a_body_that_ends_in_an_error_may_have_failed_at_its_start() {
    crate::install_surfaces();
    let source = "proc p {d} {\n    try {set x [expr {1 / $d}]; error boom} on error {} {}\n    if {[info exists x]} {return yes}\n    return no\n}\nputs [p 0]\nputs [p 1]\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let (rewritten, _) = optimised(source, dialect);
        assert!(
            rewritten.contains("return no"),
            "{dialect}: x may be unbound where the handler runs\n{rewritten}"
        );
    }
    prints_under_releases_from(source, "no\nyes\n", "8.6");
}

/// The ways a `try` body or one of its handlers leaves, each with a `finally`
/// clause that sets the global `g`: what a procedure runs and why the clause
/// is reached.
const FINALLY_PATHS: [(&str, &str); 13] = [
    ("an error", "try {error boom} finally {set g 1}"),
    ("a throw", "try {throw {A B} boom} finally {set g 1}"),
    ("a return", "try {return early} finally {set g 1}"),
    (
        "a break out of a loop",
        "while 1 { try {break} finally {set g 1} }",
    ),
    (
        "an error in a loop",
        "foreach i {1 2} { try {error boom} finally {set g 1} }",
    ),
    (
        "every branch of an if leaving",
        "try {if {[info exists ::c]} {return ok} else {error boom}} finally {set g 1}",
    ),
    (
        "every arm of a switch leaving",
        "try {switch [info exists ::c] {1 {return ok} default {error boom}}} finally {set g 1}",
    ),
    ("a tailcall", "try {tailcall list} finally {set g 1}"),
    (
        "a handler's return",
        "try {error boom} on error {} {return handled} finally {set g 1}",
    ),
    (
        "a handler's error",
        "try {error boom} on error {} {error again} finally {set g 1}",
    ),
    (
        "a handler's break",
        "foreach i {1 2} { try {error boom} on error {} {break} finally {set g 1} }",
    ),
    (
        "an exit that rejects its status",
        "try {exit abc} finally {set g 1}",
    ),
    (
        "an error only a trap might take",
        "try {error boom} trap {NOT MATCHING} {} {exit 0} finally {set g 1}",
    ),
];

/// A `finally` clause runs however the `try` body or a handler leaves, so its
/// body is live on every path (#2142). Each statement of [`FINALLY_PATHS`] sets
/// the global `g` in its clause, and `catch {p}; puts $g` prints `1` under
/// tclsh 8.6 to 9.1 before and after the optimiser, which had emptied the
/// clause and printed `0`; so does a `return $x` whose substitution only a
/// handler that exits may take, and a handler that exits once its variable is
/// bound, where a write trace refuses the binding. In a procedure, `set f 0;
/// try {error boom} finally {set f 1}; return $f` reads no unset `f`: the body
/// always raises, so the `return` never runs and both stores are dead — W220
/// is a true positive on each — and once a `catch` lets the `return` run, the
/// clause's store is the one it reads. The issue's two programs that must keep
/// firing still do: a handler that may not run binds nothing (W210), and a
/// handler's store the clause overwrites is dead (W220).
#[test]
fn try_finally_runs_on_every_path() {
    crate::install_surfaces();
    let mut sources: Vec<(String, &str)> = FINALLY_PATHS
        .iter()
        .map(|(why, statement)| {
            (
                format!(
                    "set g 0\nproc p {{}} {{\n    global g\n    {statement}\n}}\ncatch {{p}}\nputs $g\n"
                ),
                *why,
            )
        })
        .collect();
    sources.push((
        "set g 0\nproc p {x} {\n    global g\n    try {return $x} on error {} {exit 0} finally {set g 1}\n}\np 5\nputs $g\n".to_owned(),
        "a return a handler takes only the substitution's error of",
    ));
    sources.push((
        "set g 0\nproc tr args {error TRACE}\nproc p {} {\n    global g\n    trace add variable msg write tr\n    try {error boom} on error msg {exit 0} finally {set g 1}\n}\ncatch p\nputs $g\n".to_owned(),
        "a handler whose binding a trace refuses before its exit",
    ));
    for (source, why) in &sources {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, _) = optimised(source, dialect);
            assert!(
                rewritten.contains("set g 1"),
                "{why}: {dialect}: the clause runs on this path\n{rewritten}"
            );
        }
        prints_under_releases_from(source, "1\n", "8.6");
    }

    let issue =
        "proc p {} {\n    set f 0\n    try {error boom} finally {set f 1}\n    return $f\n}\n";
    let caught = "proc p {} {\n    set f 0\n    catch {try {error boom} finally {set f 1}}\n    return $f\n}\nputs [p]\n";
    let unbound =
        "proc q {c} {\n    try {if {$c} {error boom}} on error {} {set g 1}\n    return $g\n}\n";
    let overwritten = "proc p {} {\n    try {error boom} on error {} {set f 2} finally {set f 1}\n    return $f\n}\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert!(
            !reports(issue, dialect, DiagCode::W210),
            "{dialect}: the clause binds `f` before any read\n{issue}"
        );
        assert!(
            reports(issue, dialect, DiagCode::W220),
            "{dialect}: the `return` never runs, so both stores are dead\n{issue}"
        );
        assert!(
            !reports(caught, dialect, DiagCode::W210),
            "{dialect}\n{caught}"
        );
        let (rewritten, _) = optimised(caught, dialect);
        assert!(
            rewritten.contains("set f 1"),
            "{dialect}: the clause's store is read\n{rewritten}"
        );
        assert!(
            reports(unbound, dialect, DiagCode::W210),
            "{dialect}: a handler that may not run binds nothing\n{unbound}"
        );
        assert!(
            reports(overwritten, dialect, DiagCode::W220),
            "{dialect}: the clause overwrites the handler's store\n{overwritten}"
        );
    }
    prints_under_releases_from(caught, "1\n", "8.6");
}

/// A `try`'s handlers are read once, by the registry's handler chain, and the
/// control-flow graph wires the edges it names. The first match runs, so a
/// handler an earlier unconditional one pre-empts runs nothing — a `-` handler
/// pre-empts like any other, a `trap` never does; a `-` handler runs its
/// owner's script, which a match reaches with the group's names bound; and a
/// `break` the first matching handler selects goes to that handler, not to its
/// loop. Each program prints what tclsh 8.6 to 9.1 print, before and after the
/// optimiser, and reads no unset name (no W210).
#[test]
fn try_handlers_run_as_the_registry_chain_reads_them() {
    crate::install_surfaces();
    let programs = [
        (
            "proc p {} {\n    try {error boom} on error {} {set x 1} on error {} {return} finally {puts $x}\n}\np\n",
            "1\n",
        ),
        (
            "proc p {} {\n    set x 1\n    try {error boom} on error {} - on ok {} {} on error {} {unset x; return} finally {puts $x}\n}\np\n",
            "1\n",
        ),
        (
            "proc p {} {\n    set x 0\n    try {error boom} on error {} - on ok {} {set x 1} finally {}\n    return $x\n}\nputs [p]\n",
            "1\n",
        ),
        (
            "proc p {} {\n    try {error boom} on error {} - on ok {} {set x 1} finally {puts $x}\n}\np\n",
            "1\n",
        ),
        (
            "proc q {} {\n    try {error boom} on error {m} - on ok {} {return \"caught $m\"}\n    return after\n}\nputs [q]\n",
            "caught boom\n",
        ),
        (
            "proc s {} {\n    try {error boom} trap {X} {} {return T} on error {} {return E}\n}\nputs [s]\n",
            "E\n",
        ),
        (
            "proc p {} {\n    while 1 {\n        try {break} on break {} {set x 1} finally {}\n        break\n    }\n    puts $x\n}\np\n",
            "1\n",
        ),
        (
            "proc p {} {\n    while 1 {\n        try {break} on break {} {set x 1}\n        break\n    }\n    puts $x\n}\np\n",
            "1\n",
        ),
    ];
    for (source, printed) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(
                !reports(source, dialect, DiagCode::W210),
                "{dialect}: every path into the read binds it\n{source}"
            );
        }
        prints_under_releases_from(source, printed, "8.6");
    }
}

/// A handler's selector is read with the target's numerals: `on 010` takes
/// code 8 up to 8.6 and code 10 from 9.0. A body that completes with code 8
/// reaches the handler under 8.6, so its store stays and `catch p; puts $g`
/// prints `1`; from 9.0 nothing takes the code, the handler never runs, its
/// store is dead and the program prints `0`. Each release prints the same
/// before and after the optimiser under its own dialect.
#[test]
fn a_handler_selector_reads_the_targets_numerals() {
    crate::install_surfaces();
    let source = "set g 0\nproc p {} {\n    global g\n    try {return -level 0 -code 8 boom} on 010 {} {set g 1} finally {}\n}\ncatch p\nputs $g\n";
    let (under_8_6, _) = optimised(source, "tcl8.6");
    assert!(
        under_8_6.contains("set g 1"),
        "8.6 reads `010` as 8: the handler runs\n{under_8_6}"
    );
    let (under_9_0, _) = optimised(source, "tcl9.0");
    assert!(
        !under_9_0.contains("set g 1"),
        "9.0 reads `010` as 10: the handler never runs\n{under_9_0}"
    );
    for (series, tclsh) in releases_on_path() {
        if series < "8.6" {
            continue;
        }
        let printed = if series == "8.6" { "1\n" } else { "0\n" };
        let (rewritten, _) = optimised(source, &dialect_of(series));
        for program in [source, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, printed.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// A handler that never runs — one an earlier handler pre-empts, a `-`
/// handler whose script is its owner's, one the body's completion is known
/// to miss, and one whose only way in is a normal completion the body never
/// makes — leaves the `try` that does run. The binding of a handler's
/// variables carries the span of the whole `try`, and the optimiser had
/// deleted it with the dead handler: the `try` vanished, with the handler
/// that runs and the `break` of its body. Each program prints what tclsh 8.6
/// to 9.1 print before and after the optimiser, and the `try` stays.
#[test]
fn a_handler_that_never_runs_keeps_its_try() {
    crate::install_surfaces();
    let programs = [
        (
            "proc p {} {\n    try {error boom} on error {} {puts first} on 1 {m} {puts second}\n    puts after\n}\np\n",
            "first\nafter\n",
            "try {error boom}",
        ),
        (
            "proc p {} {\n    try {error boom} on error {m} - on ok {} {puts \"caught $m\"}\n    puts after\n}\np\n",
            "caught boom\nafter\n",
            "try {error boom}",
        ),
        (
            "proc p {} {\n    foreach i {1 2} {\n        puts $i\n        try {break} on error {m} {puts caught}\n        puts after\n    }\n    puts done\n}\np\n",
            "1\ndone\n",
            "try {break}",
        ),
        (
            "proc p {} {\n    try {\n        try {error a} finally {}\n        puts unreached\n    } on ok {m} {puts ok} on error {} {puts caught}\n    puts after\n}\np\n",
            "caught\nafter\n",
            "try {error a}",
        ),
    ];
    for (source, printed, kept) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, _) = optimised(source, dialect);
            assert!(
                rewritten.contains(kept),
                "{dialect}: the `try` stays\n{rewritten}"
            );
        }
        prints_under_releases_from(source, printed, "8.6");
    }
}

/// What the `finally` clause resumes once it is done. A `try` that never
/// completes normally does not fall through its clause: after `try {break} on
/// error {} {} finally {}` in a loop, `set x 1` never runs and the read after
/// the loop is of an unset `x` (W210, a true positive: tclsh fails there), and
/// after `try {return early} on error {} {} finally {}` the code that follows
/// is dead (O107). A clause that itself transfers control keeps its transfer:
/// its `break` overrides a pending `return`, so the code after the loop runs.
/// A `return` an inner `try … finally` intercepts reaches the outer clause
/// through the inner one, so the name the inner clause binds is bound; an
/// inner handler that catches an error runs before the outer clause; a `break`
/// leaves through both clauses in order; and a `return` that passes an inner
/// clause resumes past the code after the inner `try`, where the outer clause
/// may read an unset name (W210, a true positive). Each program prints what
/// tclsh 8.6 to 9.1 print before and after the optimiser.
#[test]
fn a_finally_resumes_what_it_interrupted() {
    crate::install_surfaces();
    let never_completes = "proc p {} {\n    while 1 {\n        try {break} on error {} {} finally {}\n        set x 1\n    }\n    puts $x\n}\n";
    let returns = "proc p {} {\n    try {return early} on error {} {} finally {}\n    set x 1\n    return $x\n}\nputs [p]\n";
    let resumes_past = "proc p {c} {\n    try { try {if {$c} {return}} finally {}; set x 1 } finally {puts $x}\n}\np 0\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert!(
            reports(never_completes, dialect, DiagCode::W210),
            "{dialect}: `set x 1` never runs\n{never_completes}"
        );
        assert!(
            rewrites_of(returns, dialect)
                .iter()
                .any(|rewrite| rewrite.code == DiagCode::O107),
            "{dialect}: the code after the `try` is dead\n{returns}"
        );
        assert!(
            reports(resumes_past, dialect, DiagCode::W210),
            "{dialect}: the outer clause may read `x` unset\n{resumes_past}"
        );
        assert!(
            !rewrites_of(resumes_past, dialect)
                .iter()
                .any(|rewrite| rewrite.code == DiagCode::O102),
            "{dialect}: `x` has no single reaching definition\n{resumes_past}"
        );
    }
    prints_under_releases_from(returns, "early\n", "8.6");
    prints_under_releases_from(resumes_past, "1\n", "8.6");

    let programs = [
        (
            "proc p {} {\n    while 1 { try {return early} finally {break} }\n    set x 1\n    return \"after $x\"\n}\nputs [p]\n",
            "after 1\n",
        ),
        (
            "proc p {} {\n    try { try {return ok} finally {set x 1} } finally {puts $x}\n}\np\n",
            "1\n",
        ),
        (
            "proc p {} {\n    try {try {error boom} on error {} {set x 1; return} finally {set y 1}} finally {puts $x; puts $y}\n}\np\n",
            "1\n1\n",
        ),
        (
            "proc p {} {\n    set x 0\n    while 1 {\n        try { try {unset x; break} finally {set y 1} } finally {set x 5}\n    }\n    return $x\n}\nputs [p]\n",
            "5\n",
        ),
        (
            "proc p {} {\n    set x 0\n    while {$x < 3} {\n        try {unset x; continue} finally {set x 5}\n    }\n    return $x\n}\nputs [p]\n",
            "5\n",
        ),
    ];
    for (source, printed) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(
                !reports(source, dialect, DiagCode::W210),
                "{dialect}: every path into the read binds it\n{source}"
            );
        }
        prints_under_releases_from(source, printed, "8.6");
    }
}

/// A process exit runs no `finally`: in `try {exit 7} finally {set g 1}` the
/// clause is never entered, so its store is dead code (O107), where a
/// `return` or an error would have run it.
#[test]
fn an_exit_runs_no_finally() {
    crate::install_surfaces();
    for body in ["exit", "exit 7"] {
        let source =
            format!("proc p {{}} {{\n    global g\n    try {{{body}}} finally {{set g 1}}\n}}\n");
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(
                rewrites_of(&source, dialect)
                    .iter()
                    .any(|rewrite| rewrite.code == DiagCode::O107),
                "{dialect}: the clause is never entered\n{source}"
            );
        }
    }
}

/// A body that leaves before its last statement is thrown to from where it
/// leaves, not from the dead code after it: the handler of `try {return r3;
/// set v 8} on return {} {puts t8}` runs, where its only edge had come from
/// the dead `set v 8` and the optimiser deleted the handler's body and the code
/// after the `try`; a dead `error` after the `return` is no throw point that
/// takes the `return`'s place; and a `break` caught by `on break` binds the
/// handler's store on each pass. Each program prints what tclsh 8.6 to 9.1
/// print, before and after the optimiser, and reads no unset name.
#[test]
fn a_handler_is_thrown_to_from_where_the_body_leaves() {
    crate::install_surfaces();
    let programs = [
        (
            "proc p {} {\n    try {return r3; set v 8} on return {} {puts t8}\n    return done\n}\nputs [p]\n",
            "t8\ndone\n",
        ),
        (
            "proc p {} {\n    set x 1\n    try {set x 5; return r3; set y 1} on return {} {puts \"h $x\"}\n    puts $x\n}\np\n",
            "h 5\n5\n",
        ),
        (
            "proc p {} {\n    try {return r3; error dead} on return {} {puts t8}\n    return done\n}\nputs [p]\n",
            "t8\ndone\n",
        ),
        (
            "proc p {} {\n    foreach i {1 2} {\n        try {break; set y 1} on break {} {set x $i}\n    }\n    puts $x\n}\np\n",
            "2\n",
        ),
    ];
    for (source, printed) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(
                !reports(source, dialect, DiagCode::W210),
                "{dialect}: every path into the read binds it\n{source}"
            );
        }
        prints_under_releases_from(source, printed, "8.6");
    }
}

/// The ways a `try` body, or a script it holds, fails between two stores, each
/// with what tclsh 8.6 to 9.1 print and the store a failure leaves, which is
/// no dead store. `foo` raises with no argument and with `2`, and `p 1` sets
/// `x` to 1 before the statement, so each program prints the value stored
/// before the failure where the store after it would have given another.
const BETWEEN_TWO_STORES: [(&str, &str, &str, &str); 16] = [
    (
        "a handler",
        "try {set x 2; foo; set x 3} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a finally clause no handler stands in for",
        "try {set x 2; foo; set x 3} finally {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a finally clause, where the first command fails before it stores",
        "try {set x [foo]; set x 3} finally {puts $x}",
        "1\n",
        "set x 1",
    ),
    (
        "a handler and the clause after it",
        "try {set x 2; foo; set x 3} on error {} {puts \"h $x\"} finally {puts \"f $x\"}",
        "h 2\nf 2\n",
        "set x 2",
    ),
    (
        "a body that cannot fall through",
        "try {set x 2; foo; set x 3; error boom} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "an arm of an if",
        "try {if {$c} {set x 2; foo; set x 3}} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "an arm, with the clause",
        "try {if {$c} {set x 2; foo; set x 3}} finally {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a loop body",
        "try {foreach i {1 2} {set x $i; foo $i; set x 9}} on error {} {puts $x}",
        "2\n",
        "set x $i",
    ),
    (
        "a for loop",
        "try {for {set i 0} {$i < 3} {incr i} {set x $i; foo $i}} on error {} {puts $x}",
        "2\n",
        "set x $i",
    ),
    (
        "an arm of a switch",
        "try {switch $c {1 {set x 2; foo; set x 3} default {set x 4}}} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a nested try no handler of which takes the error",
        "try {try {set x 2; foo; set x 3} on break {} {}} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a nested try with no handler",
        "try {try {set x 2; foo; set x 3}} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a handler of a nested try",
        "try {try {error a} on error {} {set x 2; foo; set x 3}} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "the clause of a nested try",
        "try {try {error a} finally {set x 2; foo; set x 3}} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a trap",
        "try {set x 2; foo; set x 3} trap {} {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a handler a `-` handler hands its match to",
        "try {set x 2; foo; set x 3} on error {m} - on ok {} {puts $x}",
        "2\n",
        "set x 2",
    ),
];

/// A command of a `try` body may fail wherever it stands, at any depth, and a
/// handler or the `finally` clause is thrown to with what the body has stored
/// by then. Each statement of [`BETWEEN_TWO_STORES`] stores 2, calls `foo`,
/// which raises, and stores another value: the handler or clause prints 2
/// under tclsh 8.6 to 9.1, before and after the optimiser, and `set x 2` is
/// no dead store. The optimiser had deleted it and forwarded the store after
/// the failure: `try {set x 2; foo; set x 3} finally {puts $x}` became `puts
/// 3`, and `set x 1; try {set x 2; foo; set x 1} on error {} {}; puts $x`
/// printed `1`, since the handler and the clause were thrown to from the
/// state before the body and at its end alone.
#[test]
fn a_try_body_is_thrown_to_from_between_its_stores() {
    crate::install_surfaces();
    for (why, statement, printed, kept) in BETWEEN_TWO_STORES {
        let source = format!(
            "proc foo {{args}} {{if {{$args eq {{}} || [lindex $args 0] == 2}} {{error x}}}}\nproc p {{c}} {{\n    set x 1\n    {statement}\n}}\ncatch {{p 1}}\n"
        );
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, _) = optimised(&source, dialect);
            assert!(
                rewritten.contains(kept),
                "{why}: {dialect}: `{kept}`, the store a failure leaves, stays\n{rewritten}"
            );
        }
        prints_under_releases_from(&source, printed, "8.6");
    }
    let issue = "proc foo {} {error x}\nproc p {} {\n    set x 1\n    try {set x 2; foo; set x 1} on error {} {}\n    puts $x\n}\np\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let held = last_value(issue, dialect, "::p", "x");
        assert!(
            !matches!(
                held,
                LatticeValue::Const(ConstValue::Number(NumericValue::Int(1)))
            ),
            "{dialect}: the handler may see 2, not a constant 1: {held:?}"
        );
    }
    prints_under_releases_from(issue, "2\n", "8.6");
}

/// The blocks a `try` body is split into keep what the handlers and the clause
/// were proved of it as one block. A statement that completes with its code
/// from the state before it stays with the statement before it, and the first
/// statements of the body read as one run: `set z 0; error boom` and `set a 1;
/// set b 2; error boom` are an error an `on error` handler takes whole, so the
/// clause around a nested `try` sees the name that handler binds (no W210).
/// The clause is thrown to the state before a statement only where that
/// failure escapes the handlers: a literal assignment raises an error alone,
/// which an `on error` handler takes, and which an `on break` handler does not
/// see at all, so a `break` it takes is still forwarded the value stored
/// before it. And a statement that leaves after its own
/// stores is thrown from the state it leaves, where the next command raises
/// only after a store: `upfoo` writes `x` and raises before `lassign` runs, so
/// the handler sees `a` still `old`, and where a `catch` follows it, which
/// runs its own script first, so that the `on break` handler sees what a
/// `return -code break` left. Each program prints what tclsh 8.6 to 9.1
/// print, before and after the optimiser.
#[test]
fn a_split_try_body_keeps_what_its_handlers_were_proved() {
    crate::install_surfaces();
    let programs = [
        (
            "proc p {} {\n    try { try {set z 0; error boom} on error {} {set x 1; return} finally {} } finally {puts $x}\n}\np\n",
            "1\n",
        ),
        (
            "proc p {} {\n    try { try {set a 1; set b 2; error boom} on error {} {set x 1; return} finally {} } finally {puts $x}\n}\np\n",
            "1\n",
        ),
        (
            "proc p {} {\n    try { try {set {[} 0; error boom} on error {} {set x 1; return} finally {} } finally {puts $x}\n}\np\n",
            "1\n",
        ),
        (
            "proc p {} {\n    try {set z 0; set y 1} on error {} {set y 2} finally {puts $y}\n}\np\n",
            "1\n",
        ),
    ];
    for (source, printed) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(
                !reports(source, dialect, DiagCode::W210),
                "{dialect}: every path into the read binds it\n{source}"
            );
        }
        prints_under_releases_from(source, printed, "8.6");
    }
    // A literal assignment raises an error if it fails at all, which an `on
    // break` handler does not take: the handler sees `y` at 2 alone, and the
    // optimiser forwards it, as it did before the body was split.
    let breaks = "proc p {} {\n    foreach i {1} {\n        try {set y 1; set y 2; break} on break {} {puts $y}\n    }\n}\np\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let (rewritten, _) = optimised(breaks, dialect);
        assert!(
            rewritten.contains("puts 2"),
            "{dialect}: the handler sees `y` at 2\n{rewritten}"
        );
    }
    prints_under_releases_from(breaks, "2\n", "8.6");
    let after_stores = "proc upfoo {} {upvar x x; set x 5; error boom}\nproc p {} {\n    array set b {k v}\n    set a old\n    try {upfoo; lassign {new second} a b} on error {} {puts $a}\n}\np\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let (rewritten, _) = optimised(after_stores, dialect);
        assert!(
            rewritten.contains("puts $a"),
            "{dialect}: `a` is old or new where the handler runs\n{rewritten}"
        );
    }
    prints_under_releases_from(after_stores, "old\n", "8.6");

    // `upfoo` writes `x` and breaks; the `catch` after it runs its own script
    // first, so only the statement's own stores make its block a point, and
    // the `on break` handler sees `x` at 5, not the 1 before and after it.
    let breaks_after_stores = "proc upfoo {} {upvar x x; set x 5; return -code break}\nproc p {} {\n    set x 1\n    try {upfoo; catch {set y 1}; set x 1} on break {} {puts $x}\n}\np\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let (rewritten, _) = optimised(breaks_after_stores, dialect);
        assert!(
            rewritten.contains("puts $x"),
            "{dialect}: `x` is 1 or 5 where the handler runs\n{rewritten}"
        );
        let held = last_value(breaks_after_stores, dialect, "::p", "x");
        assert!(
            !matches!(
                held,
                LatticeValue::Const(ConstValue::Number(NumericValue::Int(1)))
            ),
            "{dialect}: the handler may see 5, not a constant 1: {held:?}"
        );
    }
    prints_under_releases_from(breaks_after_stores, "5\n", "8.6");
}
