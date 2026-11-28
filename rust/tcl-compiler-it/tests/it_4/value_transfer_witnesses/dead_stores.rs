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

//! Dead-store removal (O109): stores kept by a later read, a write that may not run or a raise, and stores a completing call leaves dead.

use super::*;

/// A read inside a braced `expr` is a use of the version it reads, and a read
/// beside a write the same statement's substitutions make is read before,
/// between and after it: the store feeding the word that runs first stays,
/// and no word is forwarded the earlier value. Each program prints what
/// tclsh 8.4 to 9.1 print, before and after the optimiser — where the
/// statement was a `puts`, a `set` of an expression or of a string, an `expr`,
/// a `return`, an `incr`, a condition, or a call to a procedure, the optimiser
/// had deleted `set x 1` and the program raised `can't read "x"`, or forwarded
/// `1` past the increment and printed `1 2 1`.
#[test]
fn a_braced_expr_read_keeps_its_store() {
    crate::install_surfaces();
    // The two programs the examples page gives.
    prints_under_every_release(
        "proc p {} {\n set n 1\n set r [expr {$n + [incr n]}]\n return $r\n}\nputs [p]\n",
        "3\n",
    );
    prints_under_every_release(
        "proc q {} {\n set n 1\n set r [expr {[incr n] + [incr n]}]\n return $r\n}\nputs [q]\n",
        "5\n",
    );
    for (body, expected) in [
        ("puts [expr {$x + [set x 10] + $x}]\n puts $x", "21\n10\n"),
        (
            "set r [expr {$x + [set x 10] + $x}]\n puts $r\n puts $x",
            "21\n10\n",
        ),
        ("expr {$x + [set x 10] + $x}\n puts $x", "10\n"),
        ("if {$x + [set x 10] > 3} {puts yes}\n puts $x", "yes\n10\n"),
        ("while {$x + [set x 10] < 3} {break}\n puts $x", "10\n"),
        (
            "set y 5\n incr y [expr {$x + [set x 10]}]\n puts \"$y $x\"",
            "16 10\n",
        ),
        (
            "set r \"$x [set x 10] $x\"\n puts \"$r|$x\"",
            "1 10 10|10\n",
        ),
        ("foo $x [incr x] $x\n puts $x", "1 2 2\n2\n"),
        (
            "set l {}\n lappend l $x [incr x] $x\n puts \"$l|$x\"",
            "1 2 2|2\n",
        ),
        (
            "set r [expr {[string length $x] + [set x 10]}]\n puts \"$r $x\"",
            "11 10\n",
        ),
        (
            "set r [expr {[expr {$x + 1}] + [set x 10]}]\n puts \"$r $x\"",
            "12 10\n",
        ),
    ] {
        let source = format!(
            "proc foo {{a b c}} {{puts \"$a $b $c\"}}\nproc p {{}} {{\n set x 1\n {body}\n}}\np\n"
        );
        prints_under_every_release(&source, expected);
    }
    prints_under_every_release(
        "proc p {} {\n set x 1\n return [expr {$x + [set x 10]}]\n}\nputs [p]\n",
        "11\n",
    );
    // At the top level the same statements print the same.
    prints_under_every_release(
        "set x 1\nputs [expr {$x + [set x 10] + $x}]\nputs $x\n",
        "21\n10\n",
    );
}

/// A store ahead of a write that may stop part-way is dead only where every
/// path through the body overwrites it. `lassign` assigns its targets in
/// order and raises at the first it cannot write, so a store into a target it
/// reaches before the one that may fail is overwritten on every path, and a
/// store into a target after it is not. O109 keeps `set b old` ahead of
/// `lassign {new second} a b` where `a` may be or is an array, which leaves
/// `b` old, keeps `set c old` ahead of `lassign {x y z} a b c` where `b` is an
/// array, in a `try` and in `[catch {…}]`, since the command that raised
/// preserved `c`, and deletes `set a old` ahead of `lassign {new second} a b`
/// in a `try` where
/// `b` is an array: the body's first command certainly raises after it stores
/// `a`, so the solver leaves the edge from the block before the body to the
/// handler closed, and no handler reads `old`. Every program prints what
/// tclsh 8.5 (8.6 for `try`) to 9.1 prints before and after `tcl opt`.
#[test]
fn o109_refuses_the_store_ahead_of_a_partial_lassign() {
    crate::install_surfaces();
    let kept = [
        (
            "proc p {c} {\n    if {$c} {array set a {k v}}\n    set b old\n    catch {lassign {new second} a b}\n    return $b\n}\nputs [p 0]\nputs [p 1]\n",
            "second\nold\n",
            "8.5",
        ),
        (
            "proc p {} {\n    array set a {k v}\n    set b old\n    try {lassign {new second} a b} on error {} {}\n    return $b\n}\nputs [p]\n",
            "old\n",
            "8.6",
        ),
        (
            "proc p {c} {\n    if {$c} {array set a {k v}}\n    set b old\n    try {lassign {new second} a b} on error {} {}\n    return $b\n}\nputs [p 0]\nputs [p 1]\n",
            "second\nold\n",
            "8.6",
        ),
        (
            "proc p {} {\n    array set b {k v}\n    set c old\n    try {lassign {x y z} a b c} on error {} {}\n    return [list $c [info exists a]]\n}\nputs [p]\n",
            "old 1\n",
            "8.6",
        ),
        (
            "proc p {} {\n    array set b {k v}\n    set c old\n    set r [catch {lassign {x y z} a b c} m]\n    if {[info exists c]} {lappend r $c}\n    return $r\n}\nputs [p]\n",
            "1 old\n",
            "8.5",
        ),
    ];
    for (source, printed, first) in kept {
        for dialect in ["tcl8.6", "tcl9.0"] {
            // The store stays while its variable is read: a read the
            // optimiser forwards `old` into reads no store.
            let (rewritten, rewrites) = optimised(source, dialect);
            let kept = ["b", "c"].iter().all(|name| {
                !source.contains(&format!("set {name} old"))
                    || rewritten.contains(&format!("set {name} old"))
                    || !rewritten.contains(&format!("${name}"))
            });
            assert!(
                kept,
                "{dialect}: the store `lassign` may never reach stays\n{rewritten}\n{rewrites:#?}"
            );
        }
        prints_under_releases_from(source, printed, first);
    }
    // Deleted: a store every path overwrites, and one a raise preserved where
    // nothing reads what it preserved.
    let deleted = [
        (
            "proc p {} {\n    set a old\n    array set b {k keep}\n    try {lassign {new second} a b} on error {m} {}\n    return $a\n}\nputs [p]\n",
            "set a old",
            "new\n",
        ),
        (
            "proc p {} {\n    array set b {k v}\n    set c old\n    try {lassign {x y z} a b c} on error {} {}\n    return done\n}\nputs [p]\n",
            "set c old",
            "done\n",
        ),
    ];
    for (source, store, printed) in deleted {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, rewrites) = optimised(source, dialect);
            assert!(
                !rewritten.contains(store)
                    && rewrites
                        .iter()
                        .any(|rewrite| matches!(rewrite.code, DiagCode::O109 | DiagCode::O126)),
                "{dialect}: `{store}` is dead\n{rewritten}\n{rewrites:#?}"
            );
        }
        prints_under_releases_from(source, printed, "8.6");
    }
}

/// A write that may not run leaves the store ahead of it to the reads after
/// it, in the three shapes a sweep of generated bodies found folding or
/// deleting that store. `scan` makes every store it can and raises after the
/// last, so with `a` an array `scan {1 2} {%d %d} a b` leaves `b` 2, which
/// the route proves in a `catch`, a `try` and `[catch {…}]`. A write in a
/// `catch` or `try` body inside a substitution happens only where the body
/// has not stopped first, so `set c old` stays ahead of `[catch {lassign {x
/// y z} a b c} m]` where `b` may be an array — as a value, in a condition,
/// and nested in a word or an expression of the body — and `set b old`
/// ahead of `[catch {set a x; set b [error mid]} m]`, whose word raises. And
/// a `try` body that ends in a raise may fail at its first command before
/// it stores, so `set b old` stays ahead of `try {lassign {x y} a b; error
/// boom} on error {} {}` where `a` may be an array. Every program prints
/// what tclsh prints before and after `tcl opt`, from the first release
/// that has its commands.
#[test]
fn a_store_ahead_of_a_write_that_may_not_run_stays() {
    crate::install_surfaces();
    let scans = [
        (
            "proc p {} {\n    array set a {k v}\n    set b old\n    catch {scan {1 2} {%d %d} a b} m\n    return $b\n}\nputs [p]\n",
            "8.4",
        ),
        (
            "proc p {} {\n    array set a {k v}\n    set b old\n    try {scan {1 2} {%d %d} a b} on error {m} {}\n    return $b\n}\nputs [p]\n",
            "8.6",
        ),
        (
            "proc p {} {\n    array set a {k v}\n    set b old\n    set r [catch {scan {1 2} {%d %d} a b} m]\n    return $b\n}\nputs [p]\n",
            "8.4",
        ),
    ];
    for (source, first) in scans {
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert_eq!(
                lattice_text(last_value(source, dialect, "::p", "b")),
                Some("2".to_owned()),
                "{dialect}: scan writes b after failing on a\n{source}"
            );
        }
        prints_under_releases_from(source, "2\n", first);
    }
    let kept = [
        (
            "proc p {n} {\n    if {$n} {array set b {k v}}\n    set c old\n    set r [catch {lassign {x y z} a b c} m]\n    if {[info exists c]} {lappend r $c}\n    return $r\n}\nputs [p 0]\nputs [p 1]\n",
            "0 z\n1 old\n",
            "8.5",
        ),
        (
            "proc p {n} {\n    if {$n} {array set b {k v}}\n    set c old\n    if {[catch {lassign {x y z} a b c} m]} {return [list raised $c]}\n    return [list ok $c]\n}\nputs [p 0]\nputs [p 1]\n",
            "ok z\nraised old\n",
            "8.5",
        ),
        (
            "proc p {n} {\n    if {$n} {array set b {k v}}\n    set c old\n    set r [catch {set q [lassign {x y z} a b c]} m]\n    return [list $r $c]\n}\nputs [p 0]\nputs [p 1]\n",
            "0 z\n1 old\n",
            "8.5",
        ),
        (
            "proc p {n} {\n    if {$n} {array set b {k v}}\n    set c old\n    set r [catch {expr {[lassign {x y z} a b c] eq {}}} m]\n    return [list $r $c]\n}\nputs [p 0]\nputs [p 1]\n",
            "0 z\n1 old\n",
            "8.5",
        ),
        (
            "proc p {} {\n    set b old\n    set r [catch {set a x; set b [error mid]} m]\n    return [list $r $a $b]\n}\nputs [p]\n",
            "1 x old\n",
            "8.4",
        ),
        (
            "proc p {n} {\n    if {$n} {array set a {k v}}\n    set b old\n    try {lassign {x y} a b; error boom} on error {m} {}\n    return $b\n}\nputs [p 0]\nputs [p 1]\n",
            "y\nold\n",
            "8.6",
        ),
    ];
    for (source, printed, first) in kept {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, rewrites) = optimised(source, dialect);
            let kept = ["b", "c"].iter().all(|name| {
                !source.contains(&format!("set {name} old"))
                    || rewritten.contains(&format!("set {name} old"))
                    || !rewritten.contains(&format!("${name}"))
            });
            assert!(
                kept,
                "{dialect}: the store a write may not reach stays\n{rewritten}\n{rewrites:#?}"
            );
        }
        prints_under_releases_from(source, printed, first);
    }
}

/// The programs of [`a_raising_store_is_never_dead`]: each source, the raising
/// store it keeps, what it prints and the first release that runs it.
const RAISING_STORES: [(&str, &str, &str, &str); 16] = [
    (
        "proc p {} {\n    set x 1\n    catch {set x [expr {1/0}]; set y 2} m\n    return [list $m [info exists y]]\n}\nputs [p]\n",
        "set x [expr {1/0}]",
        "{divide by zero} 0\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    catch {set x [lindex {a b} 1.5]; set y 2} m\n    return [info exists y]\n}\nputs [p]\n",
        "set x [lindex {a b} 1.5]",
        "0\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    catch {set x [lindex {a b} 1.5]; set y 2}\n    return [info exists y]\n}\nputs [p]\n",
        "set x [lindex {a b} 1.5]",
        "0\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x abc\n    catch {incr x; set y 2} m\n    return [list [info exists y] $x]\n}\nputs [p]\n",
        "incr x",
        "0 abc\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set a 1\n    catch {unset a b; set y 2} m\n    return [list [info exists y] [info exists a]]\n}\nputs [p]\n",
        "unset a b",
        "0 0\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set c [catch {foreach i {1 2 3} {set x [lindex {a b} 1.5]; set y 2}}]\n    return [list $c [info exists y]]\n}\nputs [p]\n",
        "set x [lindex {a b} 1.5]",
        "1 0\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    try {set x [lindex {a b} 1.5]; set y 2} on error {m} {return [info exists y]}\n    return none\n}\nputs [p]\n",
        "set x [lindex {a b} 1.5]",
        "0\n",
        "8.6",
    ),
    (
        "proc p {} {\n    set x 1\n    try {set x [expr {1/0}]; set y 2} finally {puts \"fin [info exists y]\"}\n}\ncatch {p} m\nputs $m\n",
        "set x [expr {1/0}]",
        "fin 0\ndivide by zero\n",
        "8.6",
    ),
    (
        "proc p {} {\n    set x [format %d abc]\n    return 1\n}\nputs [catch {p}]\n",
        "set x [format %d abc]",
        "1\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    try {set x [lindex {a b} 1.5]; set y 2} finally {puts \"fin [info exists y]\"}\n}\nputs [catch {p}]\n",
        "set x [lindex {a b} 1.5]",
        "fin 0\n1\n",
        "8.6",
    ),
    (
        "proc p {} {\n    set x 1\n    catch {set x [string range abc 1 x]; set y 2} m\n    return [info exists y]\n}\nputs [p]\n",
        "set x [string range abc 1 x]",
        "0\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set a old; set c old; array set b {k v}\n    catch {scan {1 2 3} {%d %d %d} a b c; set y 2} m\n    return [list [info exists y] $a $c]\n}\nputs [p]\n",
        "scan {1 2 3} {%d %d %d} a b c",
        "0 1 3\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set a old; set c old; array set b {k v}\n    catch {lassign {1 2 3} a b c; set y 2} m\n    return [list [info exists y] $a $c]\n}\nputs [p]\n",
        "lassign {1 2 3} a b c",
        "0 1 old\n",
        "8.5",
    ),
    (
        "proc q {x} {\n    catch {set x [expr {1/0}]; set y 2} m\n    puts $x\n    puts [list $m [info exists y]]\n}\nq 5\n",
        "set x [expr {1/0}]",
        "5\n{divide by zero} 0\n",
        "8.4",
    ),
    (
        "proc p {i} {\n    set a(k) 1\n    catch {set a($i) [expr {1/0}]; set y 2} m\n    return [list $m [info exists y]]\n}\nputs [p j]\n",
        "set a($i) [expr {1/0}]",
        "{divide by zero} 0\n",
        "8.4",
    ),
    (
        "proc add {a b} {\n    expr {$a + $b}\n}\nproc f {} {\n    set unused [add x 1]\n    puts done\n}\nputs [catch f]\n",
        "set unused [add x 1]",
        "1\n",
        "8.4",
    ),
];

/// A store whose value raises is never dead, and the solver's proof that it
/// raises is no proof that it evaluated cleanly. Inside a `catch` or `try`
/// body the raise is the store's effect: `set x [expr {1/0}]` ahead of `set
/// y 2` stops the body there, so `y` is never set, though nothing reads the
/// `x` it would have stored and the solver gives that definition the value
/// `x` held before it. The same holds for a command substitution that raises
/// on its own words (`[lindex {a b} 1.5]`, `[string range abc 1 x]`), an
/// `incr` of a value that is no integer, an `unset` of an absent name, a
/// `scan` or `lassign` that stops at an array, a value-position `catch` over
/// a loop, a store to a parameter whose constant argument is propagated past
/// it, and a store to an element whose key is not known, and a command
/// substitution that raises outside any body stops the procedure (#2346's
/// arm: `set x [format %d abc]` unread, and a call to a pure procedure that
/// raises, `set unused [add x 1]`). Every program prints what tclsh
/// prints before and after `tcl opt`, from the first release that has its
/// commands, and keeps its raising store.
#[test]
fn a_raising_store_is_never_dead() {
    crate::install_surfaces();
    for (source, store, printed, first) in RAISING_STORES {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, rewrites) = optimised(source, dialect);
            assert!(
                rewritten.contains(store),
                "{dialect}: `{store}` raises and stays\n{rewritten}\n{rewrites:#?}"
            );
        }
        prints_under_releases_from(source, printed, first);
    }
}

/// One program of [`a_call_that_cannot_raise_leaves_a_dead_store`].
struct CompletingCall {
    pub(super) source: &'static str,
    /// The unused store of the call.
    pub(super) store: &'static str,
    /// Whether `tcl opt` drops it: only where the call cannot raise.
    goes: bool,
    pub(super) printed: &'static str,
}

/// The programs of [`a_call_that_cannot_raise_leaves_a_dead_store`].
const COMPLETING_CALLS: [CompletingCall; 15] = [
    // The registry baseline: `string length` completes whatever it is given.
    CompletingCall {
        source: "proc lbl {x} {return [string length $x]}\nproc main {} {\n    set a [lbl abc]\n    return 1\n}\nputs [main]\n",
        store: "set a [lbl abc]",
        goes: true,
        printed: "1\n",
    },
    // The call's word reads a parameter, which is set.
    CompletingCall {
        source: "proc lbl {x} {set n [string length $x]; return $n}\nproc main {y} {\n    set a [lbl $y]\n    return 1\n}\nputs [main q]\n",
        store: "set a [lbl $y]",
        goes: true,
        printed: "1\n",
    },
    // A literal return, reached through a second procedure.
    CompletingCall {
        source: "proc one {} {return 1}\nproc two {} {return [one]}\nproc main {} {\n    set a [two]\n    return 1\n}\nputs [main]\n",
        store: "set a [two]",
        goes: true,
        printed: "1\n",
    },
    // `expr` raises for `x + 1`.
    CompletingCall {
        source: "proc add {a b} {expr {$a + $b}}\nproc main {} {\n    set a [add x 1]\n    return 1\n}\nputs [catch main]\n",
        store: "set a [add x 1]",
        goes: false,
        printed: "1\n",
    },
    // A word count the parameters reject.
    CompletingCall {
        source: "proc lbl {x} {return [string length $x]}\nproc main {} {\n    set a [lbl]\n    return 1\n}\nputs [catch main]\n",
        store: "set a [lbl]",
        goes: false,
        printed: "1\n",
    },
    // A recursion, which the nesting limit ends.
    CompletingCall {
        source: "proc rec {n} {return [string length [rec $n]]}\nproc main {} {\n    set a [rec 1]\n    return 1\n}\nputs [catch main]\n",
        store: "set a [rec 1]",
        goes: false,
        printed: "1\n",
    },
    // `lindex` raises on a list with an unmatched brace.
    CompletingCall {
        source: "proc first {x} {return [lindex $x 0]}\nproc main {} {\n    set a [first \\{]\n    return 1\n}\nputs [catch main]\n",
        store: "set a [first \\{]",
        goes: false,
        printed: "1\n",
    },
    // The call's word reads a variable nothing sets.
    CompletingCall {
        source: "proc lbl {x} {return [string length $x]}\nproc main {} {\n    set a [lbl $y]\n    return 1\n}\nputs [catch main]\n",
        store: "set a [lbl $y]",
        goes: false,
        printed: "1\n",
    },
    // `lbl` in `::ns` is `::ns::lbl`, which raises, not the global one.
    CompletingCall {
        source: "proc ::lbl {x} {return [string length $x]}\nnamespace eval ns {\n    proc lbl {x} {error boom}\n    proc main {} {\n        set a [lbl abc]\n        return 1\n    }\n}\nputs [catch ns::main]\n",
        store: "set a [lbl abc]",
        goes: false,
        printed: "1\n",
    },
    // `main` runs before `lbl`'s definition: `invalid command name`.
    CompletingCall {
        source: "proc main {} {\n    set a [lbl abc]\n    return 1\n}\nputs [catch main]\nproc lbl {x} {return [string length $x]}\n",
        store: "set a [lbl abc]",
        goes: false,
        printed: "1\n",
    },
    // `lbl` is defined only under a condition that does not hold.
    CompletingCall {
        source: "if {[info exists ::env(TCL_LSP_NEVER_SET)]} {\n    proc lbl {x} {return [string length $x]}\n}\nproc main {} {\n    set a [lbl abc]\n    return 1\n}\nputs [catch main]\n",
        store: "set a [lbl abc]",
        goes: false,
        printed: "1\n",
    },
    // A callback runs after the load, which defined `lbl`.
    CompletingCall {
        source: "proc main {} {\n    set a [lbl abc]\n    return 1\n}\nproc lbl {x} {return [string length $x]}\nafter 0 {puts [main]; set ::done 1}\nvwait ::done\n",
        store: "set a [lbl abc]",
        goes: true,
        printed: "1\n",
    },
    // `lbl`'s body is `extra characters after close-quote`.
    CompletingCall {
        source: "proc lbl {x} {return [string length \"a\"b]}\nproc main {} {\n    set a [lbl abc]\n    return 1\n}\nputs [catch main]\n",
        store: "set a [lbl abc]",
        goes: false,
        printed: "1\n",
    },
    // `lbl`'s body is `extra characters after close-brace`.
    CompletingCall {
        source: "proc lbl {x} {return [string length {a}b]}\nproc main {} {\n    set a [lbl abc]\n    return 1\n}\nputs [catch main]\n",
        store: "set a [lbl abc]",
        goes: false,
        printed: "1\n",
    },
    // Under 8.4 `{*}` is a braced word, so `{*}$x` is the close-brace error;
    // from 8.5 the expansion reaches `string length a b`.
    CompletingCall {
        source: "proc lbl {x} {return [string length {*}$x]}\nproc main {} {\n    set a [lbl {a b}]\n    return 1\n}\nputs [catch main]\n",
        store: "set a [lbl {a b}]",
        goes: false,
        printed: "1\n",
    },
];

/// The unused store of a call goes only where the call cannot raise:
/// purity says a call changes nothing, not that it completes, so the
/// call's procedure must complete whatever its arguments hold — a
/// straight-line body that reads only what it set and runs only commands
/// that complete so, `string length` among them — and be called with a word
/// count its parameters accept, its words reading only set variables. A body
/// whose `expr` or `lindex` may raise, a rejected word count, a recursion, an
/// unset read and a procedure the caller's namespace shadows each keep the
/// store, and so does a callee whose `proc` statement has not surely run
/// where the caller runs — defined after the load first runs the caller, or
/// under a condition — and a callee whose body holds a word the
/// release's parser rejects; a caller only a callback runs, after the
/// load, takes a callee defined anywhere at the top level. Every program
/// prints what tclsh prints before and after `tcl opt` under every release.
#[test]
fn a_call_that_cannot_raise_leaves_a_dead_store() {
    crate::install_surfaces();
    for call in &COMPLETING_CALLS {
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            let (rewritten, rewrites) = optimised(call.source, dialect);
            assert_eq!(
                !rewritten.contains(call.store),
                call.goes,
                "{dialect}: `{}` goes only where the call cannot raise\n{rewritten}\n{rewrites:#?}",
                call.store
            );
        }
        prints_under_releases_from(call.source, call.printed, "8.4");
    }
}
