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

//! `try` and `finally`: reachability of handlers and `finally` bodies on every way out, and the names and stores they see.

use super::*;

/// A `finally` clause runs on every completion path, so its body is never
/// unreachable — whatever the `try` body does.
///
/// `lower_try` wired `try_end` (and the `finally` hanging off it) only from a
/// body that falls through normally, or from a handler's throw edge. A body
/// that cannot fall through and no handler left the whole tail with no
/// predecessor at all, SCCP called it dead, and O107 emptied the clause.
/// Measured on tclsh 8.6.18 and 9.0.4, `catch {p}; puts $g` printed `1` and
/// the rewritten program printed `0` (#2142).
#[test]
fn a_finally_body_is_reachable_however_the_try_body_leaves() {
    crate::install_surfaces();
    for (why, body, wrapper) in [
        ("error", "error boom", ""),
        ("throw", "throw {A B} boom", ""),
        ("return", "return early", ""),
        ("break", "break", "while {1} "),
        ("error in a loop", "error boom", "foreach i {1 2} "),
        // Every branch leaves, so the body cannot fall through — yet it still
        // ends in a resting `if_end` block. Gating on "has no tail" missed it
        // (found in review).
        (
            "every branch of an if leaves",
            "if {[info exists ::c]} {return ok} else {error boom}",
            "",
        ),
        (
            "every arm of a switch leaves",
            "switch [info exists ::c] {1 {return ok} default {error boom}}",
            "",
        ),
        // `tailcall` leaves the frame, but only once the `finally` has run:
        // tclsh 8.6.18 and 9.0.4 print `FINALLY` for `try {tailcall t}
        // finally {puts FINALLY}` inside a proc.
        ("tailcall", "tailcall list", ""),
    ] {
        let stmt = format!("try {{{body}}} finally {{set g 1}}");
        // Only a loop wraps the statement: `{wrapper}{ … }` with an empty
        // wrapper is a braced command *name*, not a `try` at all.
        let line = if wrapper.is_empty() {
            stmt
        } else {
            format!("{wrapper}{{ {stmt} }}")
        };
        let src = format!(
            "set g 0\nproc p {{}} {{\n    global g\n    {line}\n}}\ncatch {{p}}\nputs $g\n"
        );
        let out = optimised(&src, TCL);
        assert!(
            out.contains("set g 1"),
            "{why}: `finally` runs on this path, so its store is live: {:?}\n{out}",
            opt_rewrites(&src, TCL)
        );
    }
}

/// A handler that itself leaves — `return`, `error`, or a `break` out of an
/// enclosing loop — reaches the `finally` too. The handler's own exit
/// blocks were never wired to it, so with any handler present a `finally`
/// reached only that way was dead: `try {error boom} on error {} {return
/// handled} finally {set g 1}` lost its store, and O109 then deleted the
/// caller's `set g 0`, so the rewritten program failed with `can't read "g"`
/// where tclsh 8.6.18 prints `1` (found in review).
#[test]
fn a_finally_body_is_reachable_however_a_handler_leaves() {
    crate::install_surfaces();
    for (why, handler, wrapper) in [
        ("return", "return handled", ""),
        ("error", "error again", ""),
        ("break", "break", "foreach i {1 2} "),
    ] {
        let stmt = format!("try {{error boom}} on error {{}} {{{handler}}} finally {{set g 1}}");
        let line = if wrapper.is_empty() {
            stmt
        } else {
            format!("{wrapper}{{ {stmt} }}")
        };
        let src = format!(
            "set g 0\nproc p {{}} {{\n    global g\n    {line}\n}}\ncatch {{p}}\nputs $g\n"
        );
        let out = optimised(&src, TCL);
        assert!(
            out.contains("set g 1"),
            "{why}: `finally` runs after the handler leaves: {:?}\n{out}",
            opt_rewrites(&src, TCL)
        );
    }
}

/// `exit` ends the interpreter without unwinding, so it reaches no `finally`:
/// `try {exit 7} finally {puts FINALLY}` exits with status 7 and prints
/// nothing on tclsh 8.6.18 and 9.0.4. Wiring it to the clause made the clause
/// executable in SCCP and SSA though it can never run (found in review).
#[test]
fn an_exit_reaches_no_finally() {
    crate::install_surfaces();
    for (why, body) in [("no argument", "exit"), ("a literal status", "exit 7")] {
        let src =
            format!("proc p {{}} {{\n    global g\n    try {{{body}}} finally {{set g 1}}\n}}\n");
        assert!(
            opt_fires(&src, TCL, "O107"),
            "{why}: a `finally` reached only through `exit` never runs: {:?}",
            opt_codes(&src, TCL)
        );
    }

    // Precision: anything that can stop the `exit` from running keeps the
    // clause live, because a `return` or an error does run it. Each of these
    // prints `1` on tclsh 8.6.18; review found each one emptied.
    for (why, body) in [
        (
            "an arm that returns instead",
            "switch -glob $x {a {exit 7} default {return ok}}",
        ),
        (
            "every arm may return before it exits",
            "switch -glob $x {a {if {$c} {return ok}; exit 7} default {if {$c} {return ok}; exit 8}}",
        ),
        // The subject substitution runs first and may throw.
        (
            "a switch whose subject may throw",
            "switch -glob $nosuch {a {exit 7} default {exit 8}}",
        ),
        ("an exit whose argument throws", "exit [error boom]"),
        ("an exit that rejects its literal", "exit abc"),
        // Release-aware: an invalid octal in 8.x (`TCL` is 8.6), status 9 in
        // 9.0 — the registry answers, not a digit check.
        ("an exit whose status is an invalid 8.x octal", "exit 09"),
    ] {
        let src = format!(
            "set g 0\nproc p {{x c}} {{\n    global g\n    try {{{body}}} finally {{set g 1}}\n}}\ncatch {{p a 1}}\nputs $g\n"
        );
        assert!(
            optimised(&src, TCL).contains("set g 1"),
            "{why}: the `finally` still runs: {:?}",
            opt_rewrites(&src, TCL)
        );
    }

    // An alias invokes more words than the call site shows: `bye` here runs
    // `exit abc`, which raises, so tclsh 8.6.18 and 9.0.4 print `1` (found in
    // review).
    for (why, src) in [
        (
            "an alias with a prefixed status",
            "set g 0\ninterp alias {} bye {} exit abc\nproc p {} {\n    global g\n    try {bye} finally {set g 1}\n}\ncatch p\nputs $g\n",
        ),
        // A handler catches only the substitution's error; the `return` still
        // runs the clause, so tclsh prints `1` (found in review).
        (
            "a handler that catches only some of the body's completions",
            "set g 0\nproc p {x} {\n    global g\n    try {return $x} on error {} {exit 0} finally {set g 1}\n}\np 5\nputs $g\n",
        ),
        // A statement before the `return` may raise first, and a `trap` may
        // not match: either way the `finally` still runs, so tclsh prints `1`
        // (found in review).
        (
            "a statement that may raise before an exact `return`",
            "set g 0\nproc p {} {\n    global g\n    try {set y $x; return ok} on error {} {} on return {} {exit 0} finally {set g 1}\n}\np\nputs $g\n",
        ),
        // An earlier block may raise before the `return` or `exit` in the
        // block after it: `$c` is unset, and tclsh prints `1` (found in
        // review). So may the outer body before a nested `try`.
        (
            "an earlier block that may raise before an exact `return`",
            "set g 0\nproc p {} {\n    global g\n    try {if {$c} {}; return ok} on error {} {} on return {} {exit 0} finally {set g 1}\n}\np\nputs $g\n",
        ),
        (
            "an earlier block that may raise before an `exit`",
            "set g 0\nproc p {} {\n    global g\n    try {if {$c} {}; exit 0} finally {set g 1}\n}\ncatch p\nputs $g\n",
        ),
        (
            "an outer statement that may raise before a nested `exit`",
            "set g 0\nproc p {} {\n    global g\n    try { set y $x; try {exit 0} finally {} } finally {set g 1}\n}\ncatch p\nputs $g\n",
        ),
        // `on 010` is octal code 8 in Tcl 8.x (`TCL` is 8.6), so the handler
        // catches the body and its store is live (found in review).
        (
            "a handler selector in the dialect's own numerals",
            "set g 0\nproc p {} {\n    global g\n    try {return -level 0 -code 8 boom} on 010 {} {set g 1} finally {}\n}\ncatch p\nputs $g\n",
        ),
        (
            "an error only a `trap` might catch",
            "set g 0\nproc p {} {\n    global g\n    try {error boom} trap {NOT MATCHING} {} {exit 0} finally {set g 1}\n}\ncatch p\nputs $g\n",
        ),
        // Binding `msg` is a write, and a write trace can reject it before
        // the `exit` runs; the error then runs the clause. tclsh 8.6.18 and
        // 9.0.4 print `1`, as they do when `msg` is an `upvar` to an array
        // (found in review).
        (
            "a handler whose variable binding may raise before its `exit`",
            "set g 0\nproc tr args {error TRACE}\nproc p {} {\n    global g\n    trace add variable msg write tr\n    try {error boom} on error msg {exit 0} finally {set g 1}\n}\ncatch p\nputs $g\n",
        ),
        (
            "a namespace alias spelled like its target",
            "set g 0\nnamespace eval foo {}\ninterp alias {} ::foo::exit {} ::exit abc\nproc ::foo::p {} {\n    global g\n    try {exit} finally {set g 1}\n}\ncatch foo::p\nputs $g\n",
        ),
    ] {
        assert!(
            optimised(src, TCL).contains("set g 1"),
            "{why}: the `finally` still runs: {:?}",
            opt_rewrites(src, TCL)
        );
    }
}

/// The definiteness half. A name bound before the `try` is still bound after
/// it, and the `finally` store that rebinds it is visible.
#[test]
fn a_try_finally_does_not_hide_the_names_bound_around_it() {
    crate::install_surfaces();
    for (why, src) in [
        // The `catch` is what makes the read live: without it the error
        // propagates, and the `return` after the `try` never runs.
        (
            "bound before the `try`, rebound by `finally`",
            "proc p {} {\n    set f 0\n    catch { try {error boom} finally {set f 1} }\n    return $f\n}\n",
        ),
        // An inner `finally` runs before the outer one on every path, so the
        // name it binds is bound when the outer clause reads it. Wiring the
        // inner body's `return` straight to the outer `finally` skipped the
        // inner clause (found in review).
        (
            "bound by an inner `finally` before the outer one reads it",
            "proc p {} {\n    try { try {return ok} finally {set x 1} } finally {puts $x}\n}\n",
        ),
        // An inner handler that catches the error runs, and the inner clause
        // after it, before the outer clause reads: tclsh 8.6.18 and 9.0.4
        // print `1` twice (found in review).
        (
            "bound by an inner handler and inner clause before the outer one",
            "proc p {} {\n    try {try {error boom} on error {} {set x 1; return} finally {set y 1}} finally {puts $x; puts $y}\n}\n",
        ),
        // A literal assignment before the `error` can only raise an error too,
        // so the inner handler catches the block whichever raises; tclsh
        // prints `1` (found in review).
        (
            "bound by an inner handler after a braced literal assignment",
            "proc p {} {\n    try { try {set {[} 0; error boom} on error {} {set x 1; return} finally {} } finally {puts $x}\n}\n",
        ),
        (
            "bound by an inner handler after a literal assignment",
            "proc p {} {\n    try { try {set z 0; error boom} on error {} {set x 1; return} finally {} } finally {puts $x}\n}\n",
        ),
        // Only the first matching handler runs; the second `on error` is dead,
        // and tclsh prints `1` (found in review).
        (
            "bound by the first of two handlers for the same code",
            "proc p {} {\n    try {error boom} on error {} {set x 1} on error {} {return} finally {puts $x}\n}\n",
        ),
        // The first `on error` selects the error though its body is `-`; the
        // last handler never runs, and tclsh prints `1` (found in review).
        (
            "not unbound by a handler a `-` handler pre-empts",
            "proc p {} {\n    set x 1\n    try {error boom} on error {} - on ok {} {} on error {} {unset x; return} finally {puts $x}\n}\n",
        ),
        // A `break`/`continue` runs the clause before it reaches the loop.
        // An edge into the `finally` alongside the jump still left a path
        // into the loop that skipped it, carrying the `unset` (found in
        // review); tclsh 8.6.18 and 9.0.4 print `5` for each of these.
        (
            "rebound by `finally` before a `continue` reaches the loop test",
            "proc p {} {\n    set x 0\n    while {$x < 3} {\n        try {unset x; continue} finally {set x 5}\n    }\n    return $x\n}\n",
        ),
        (
            "rebound by `finally` before a `break` leaves the loop",
            "proc p {} {\n    set x 0\n    while 1 {\n        try {unset x; break} finally {set x 5}\n    }\n    return $x\n}\n",
        ),
        (
            "rebound by `finally` after a handler's `continue`",
            "proc p {} {\n    set x 0\n    while {$x < 3} {\n        try {error boom} on error {} {unset x; continue} finally {set x 5}\n    }\n    return $x\n}\n",
        ),
        (
            "rebound by `finally` after a `continue` no handler catches",
            "proc p {} {\n    set x 0\n    while {$x < 3} {\n        try {unset x; continue} on error {} {} finally {set x 5}\n    }\n    return $x\n}\n",
        ),
        // A handler that selects the jump's code catches it, so the jump never
        // reaches the loop; tclsh 8.6.18 and 9.0.4 print `1` for both (found
        // in review).
        (
            "bound by an `on break` handler that catches the `break`",
            "proc p {} {\n    while 1 {\n        try {break} on break {} {set x 1} finally {}\n        break\n    }\n    puts $x\n}\n",
        ),
        (
            "bound by an `on break` handler, with no `finally`",
            "proc p {} {\n    while 1 {\n        try {break} on break {} {set x 1}\n        break\n    }\n    puts $x\n}\n",
        ),
        (
            "rebound by the outer of two nested clauses a `break` leaves",
            "proc p {} {\n    set x 0\n    while 1 {\n        try { try {unset x; break} finally {set y 1} } finally {set x 5}\n    }\n    return $x\n}\n",
        ),
    ] {
        assert!(
            !analyser_codes(src, TCL).iter().any(|c| c == "W210"),
            "{why}: {:?}",
            analyser_codes(src, TCL)
        );
    }
}

/// A `try` whose body and handlers can never complete normally does not fall
/// through its `finally` into the code after it: every way in is an exit that
/// resumes unwinding or a saved jump. Letting the clause fall through made
/// `set x 1` look reachable from the `break` (found in review).
#[test]
fn a_try_that_never_completes_does_not_fall_through_its_finally() {
    crate::install_surfaces();
    // tclsh 8.6.18 and 9.0.4 both fail these with `can't read "x"`.
    for (why, src) in [
        (
            "`break`",
            "proc p {} {\n    while 1 {\n        try {break} finally {}\n        set x 1\n    }\n    puts $x\n}\n",
        ),
        (
            "`break` through nested clauses",
            "proc p {} {\n    while 1 {\n        try { try {break} finally {} } finally {}\n        set x 1\n    }\n    puts $x\n}\n",
        ),
        // A handler whose code is not the jump's offers no way to complete:
        // `on error`, `on continue` and `on 4` cannot catch a `break`.
        (
            "`break` past an `on error` handler",
            "proc p {} {\n    while 1 {\n        try {break} on error {} {} finally {}\n        set x 1\n    }\n    puts $x\n}\n",
        ),
        (
            "`break` past an `on continue` handler",
            "proc p {} {\n    while 1 {\n        try {break} on continue {} {} finally {}\n        set x 1\n    }\n    puts $x\n}\n",
        ),
        (
            "`break` past an `on 4` handler",
            "proc p {} {\n    while 1 {\n        try {break} on 4 {} {} finally {}\n        set x 1\n    }\n    puts $x\n}\n",
        ),
    ] {
        assert!(
            analyser_codes(src, TCL).iter().any(|c| c == "W210"),
            "{why}: {:?}",
            analyser_codes(src, TCL)
        );
    }

    // `on error` cannot catch a `return`, so the code after the `try` is dead:
    // tclsh 8.6.18 and 9.0.4 return `early` (found in review).
    let returns = "proc p {} {\n    try {return early} on error {} {} finally {}\n    set x 1\n    return $x\n}\n";
    assert!(
        opt_fires(returns, TCL, "O107"),
        "`return` past an `on error` handler: {:?}",
        opt_codes(returns, TCL)
    );
}

/// A body that falls through into `try_ok` completes normally even when every
/// handler leaves abruptly, so the code after the `try` is live: tclsh
/// 8.6.18 and 9.0.4 return `2` (found in review).
#[test]
fn a_body_that_completes_through_try_ok_keeps_the_code_after_the_try() {
    crate::install_surfaces();
    let src = "proc p {} {\n    try {set x 1} on error {} {return early} finally {}\n    set y 2\n    return $y\n}\n";
    assert!(
        !opt_fires(src, TCL, "O107"),
        "the code after the `try` runs: {:?}",
        opt_codes(src, TCL)
    );
}

/// A `-` handler runs the body of the handler after it, whatever that
/// handler's own selector, so a completion the `-` handler matches reaches the
/// shared body. tclsh 8.6.18 and 9.0.4 return `1`, `1`, `1`, and `5` / `6`
/// (found in review).
#[test]
fn a_fallthrough_handler_reaches_the_body_it_shares() {
    crate::install_surfaces();
    for (why, src, kept) in [
        (
            "an error selected by `on error {} -`",
            "proc p {} {\n    set x 0\n    try {error boom} on error {} - on ok {} {set x 1} finally {}\n    return $x\n}\n",
            "set x 1",
        ),
        (
            "a return selected by `on return {} -`",
            "proc p {} {\n    set x 0\n    try {return early} on return {} - on error {} {set x 1} finally {}\n    return $x\n}\n",
            "set x 1",
        ),
        (
            "an error through a chain of two `-` handlers",
            "proc p {} {\n    set x 0\n    try {error boom} on error {} - trap {} {} - on ok {} {set x 1} finally {}\n    return $x\n}\n",
            "set x 1",
        ),
    ] {
        assert!(
            optimised(src, TCL).contains(kept),
            "{why}: the shared body runs: {}",
            optimised(src, TCL)
        );
    }

    // Precision: the match runs the shared body, never the `-` handler's own
    // empty block, so `x` is set before the clause reads it. tclsh prints
    // `1`; the empty block's edge on to `try_end` drew W210 (found in
    // review).
    let bound =
        "proc p {} {\n    try {error boom} on error {} - on ok {} {set x 1} finally {puts $x}\n}\n";
    assert!(
        !analyser_codes(bound, TCL).contains(&"W210".to_owned()),
        "`x` is bound on every path into the clause: {:?}",
        analyser_codes(bound, TCL)
    );

    // An `on ok` owner shared with `on error {} -` is not reached from the
    // tail alone: the error path carries `y` = 5 into it.
    let shared = "proc p {c} {\n    set y 0\n    try {set y 5; if {$c} {error boom}; set y 6} on error {} - on ok {} {return $y} finally {}\n    return none\n}\n";
    assert!(
        optimised(shared, TCL).contains("return $y"),
        "`y` is 5 or 6 in the shared body: {}",
        optimised(shared, TCL)
    );
}

/// A `finally` clause that itself transfers control keeps that transfer: its
/// `break` overrides the pending return or error, so the code after the loop
/// is live. tclsh 8.6.18 and 9.0.4 print `after 1` and `survived` (found in
/// review).
#[test]
fn a_finally_that_transfers_control_keeps_its_transfer() {
    crate::install_surfaces();
    for (why, src) in [
        (
            "`break` over a pending `return`",
            "proc p {} {\n    while 1 { try {return early} finally {break} }\n    set x 1\n    return \"after $x\"\n}\n",
        ),
        (
            "`break` over a pending error",
            "proc p {} {\n    while 1 { try {error boom} finally {break} }\n    set y survived\n    return $y\n}\n",
        ),
    ] {
        assert!(
            !opt_fires(src, TCL, "O107"),
            "{why}: the code after the loop runs: {:?}",
            opt_codes(src, TCL)
        );
    }
}

/// A `return` that passes an inner `finally` resumes past the statements
/// after the inner `try`, even when that `try` can also fall through: in
/// `try { try {if {$c} {return}} finally {}; set x 1 } finally {puts $x}` the
/// outer clause reads `x` unset on the `return` path, and tclsh 8.6.18 and
/// 9.0.4 fail there. Sending the `return` through the clause's fall-through
/// made `set x 1` look certain and O102 forwarded it (found in review).
#[test]
fn a_return_through_an_inner_finally_skips_the_code_after_it() {
    crate::install_surfaces();
    let src = "proc p {c} {\n    try { try {if {$c} {return}} finally {}; set x 1 } finally {puts $x}\n}\n";
    assert!(
        analyser_codes(src, TCL).iter().any(|c| c == "W210"),
        "the outer clause may read `x` unset: {:?}",
        analyser_codes(src, TCL)
    );
    assert!(
        !opt_fires(src, TCL, "O102"),
        "`x` has no single reaching definition at the outer clause: {:?}",
        opt_codes(src, TCL)
    );
}

/// A handler is reached from the explicit throws inside a nested construct,
/// with their block's stores live — including a `finally` that only ever
/// resumes unwinding. tclsh 8.6.18 and 9.0.4 return `1` for both.
#[test]
fn a_try_handler_sees_the_stores_before_a_nested_throw() {
    crate::install_surfaces();
    for (why, src) in [
        (
            "every arm of an `if` throws",
            "proc p {c} {\n    try { if {$c} {set x 1; error b} else {set x 2; error c} } on error {} {}\n    return $x\n}\n",
        ),
        (
            "an inner `finally` stores, then resumes the error",
            "proc p {} {\n    try { try {error boom} finally {set x 1} } on error {} {}\n    return $x\n}\n",
        ),
    ] {
        let codes = opt_codes(src, TCL);
        assert!(
            !codes.iter().any(|c| c == "O109")
                && !analyser_codes(src, TCL).iter().any(|c| c == "W220"),
            "{why}: the store is read after the handler: {codes:?} {:?}",
            analyser_codes(src, TCL)
        );
    }
}

/// Precision: the fix must not make a handler's variable look bound on a path
/// that never runs it, nor silence the dead store a `finally` really does
/// create.
#[test]
fn a_try_handler_still_binds_only_on_the_path_that_runs_it() {
    crate::install_surfaces();
    // tclsh 8.6.18 fails this with `can't read "g": no such variable` when the
    // body does not throw, so W210 is a true positive.
    let unbound =
        "proc q {c} {\n    try { if {$c} {error boom} } on error {} {set g 1}\n    return $g\n}\n";
    assert!(
        analyser_codes(unbound, TCL).iter().any(|c| c == "W210"),
        "a handler that may not run does not bind its names: {:?}",
        analyser_codes(unbound, TCL)
    );

    // `finally` overwrites the handler's store before any read, so the
    // handler's assignment really is dead.
    let overwritten = "proc p {} {\n    try {error boom} on error {} {set f 2} finally {set f 1}\n    return $f\n}\n";
    assert!(
        opt_fires(overwritten, TCL, "O109"),
        "`finally` runs after the handler, so `set f 2` is dead: {:?}",
        opt_codes(overwritten, TCL)
    );
}
