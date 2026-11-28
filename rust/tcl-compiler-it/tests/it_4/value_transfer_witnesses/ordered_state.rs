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

//! Ordered evaluation state: nested writes in a host statement, substitutions landing writes beside their result, and the prefix rule.

use super::*;

/// A write a statement makes that none of its words read overwrites the store
/// before it, as it did: the call that carries the write reads the place only
/// where a word does. `puts [set x 2]` and a `gets` that fills a loop's
/// variable leave the stores before them dead, so O109 and O126 still remove
/// them, and each program prints the same afterwards.
#[test]
fn a_store_a_nested_write_overwrites_unread_is_still_dead() {
    crate::install_surfaces();
    let overwritten = "proc p {} {\n set x 1\n puts [set x 2]\n return $x\n}\nputs [p]\n";
    let loop_variable = "proc p {fd} {\n set line {}\n set n 0\n \
                         while {[gets $fd line] >= 0} {incr n}\n return $n\n}\n\
                         set f [open /dev/null r]\nputs [p $f]\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        assert!(removes_store(overwritten, dialect, "set x 1"), "{dialect}");
        assert!(
            removes_store(loop_variable, dialect, "set line {}"),
            "{dialect}"
        );
    }
    prints_under_every_release(overwritten, "2\n2\n");
    prints_under_every_release(loop_variable, "0\n");
}

/// The programs of [`a_nested_write_in_a_host_prints_what_tclsh_prints`], each with what
/// tclsh 8.4 to 9.1 print.
const NESTED_WRITE_PROGRAMS: &[(&str, &str)] = &[
    // a loop carries the write round
    (
        "set x 0\nfor {set i 0} {$i < 3} {incr i} { set r [expr {[incr x] * 2}] }\nputs \"$r $x\"\n",
        "6 3\n",
    ),
    (
        "set x 0\nset i 0\nwhile {$i < 4} { expr {[incr x 2] + 0}; incr i }\nputs $x\n",
        "8\n",
    ),
    (
        "proc p {n} {set x 0; for {set i 0} {$i < $n} {incr i} {set r [expr {$x + [incr x]}]}; return \"$r $x\"}\nputs [p 3]\nputs [p 1]\n",
        "5 3\n1 1\n",
    ),
    // a branch the engine takes or skips
    (
        "proc p {c} { set x 1; if {$c} { set r [expr {[incr x] + 1}] } else { set r 0 }; return \"$r $x\" }\nputs [p 1]\nputs [p 0]\n",
        "3 2\n0 1\n",
    ),
    (
        "proc p {c} { set x 1; set r [expr {$c ? [incr x] : 0}]; return \"$r $x\" }\nputs [p 1]\nputs [p 0]\n",
        "2 2\n0 1\n",
    ),
    (
        "proc p {c} { set x 1; set r [expr {$c && [incr x]}]; return \"$r $x\" }\nputs [p 1]\nputs [p 0]\n",
        "1 2\n0 1\n",
    ),
    (
        "proc p {c} { set x 1; set r [expr {$c || [incr x]}]; return \"$r $x\" }\nputs [p 1]\nputs [p 0]\n",
        "1 1\n1 2\n",
    ),
    (
        "set x 1\nset r [expr {1 ? [incr x] : [incr x 10]}]\nputs \"$r $x\"\n",
        "2 2\n",
    ),
    (
        "set x 1\nset r [expr {0 || [incr x]}]\nputs \"$r $x\"\n",
        "1 2\n",
    ),
    (
        "set x 1\nset r [expr {1 || [incr x]}]\nputs \"$r $x\"\n",
        "1 1\n",
    ),
    // an input the engine takes on one member and not on another
    (
        "proc p {n} {set x 1; if {$n} {set c 0} else {set c 1}; set r [expr {$c ? [incr x] : 7}]; return \"$r $x\"}\nputs [p 0]\nputs [p 1]\n",
        "2 2\n7 1\n",
    ),
    // an element of an array
    (
        "set a(1) 5\nset r [expr {$a(1) + [incr a(1)]}]\nputs \"$r $a(1)\"\n",
        "11 6\n",
    ),
    (
        "set a(k) 1\nset r [expr {[incr a(k)] + [incr a(k)]}]\nputs \"$r $a(k)\"\n",
        "5 3\n",
    ),
    (
        "array set a {1 5 2 6}\nset r [expr {$a(1) + [set a(2) 9] + $a(2)}]\nputs \"$r $a(1) $a(2)\"\n",
        "23 5 9\n",
    ),
    // a place the frame does not own is left alone
    (
        "set ::g 5\nset r [expr {$::g + [incr ::g]}]\nputs \"$r $::g\"\n",
        "11 6\n",
    ),
    (
        "proc p {} {global g; set g 1; set r [expr {$g + [incr g]}]; return \"$r $g\"}\nputs [p]\n",
        "3 2\n",
    ),
    (
        "proc p {} {upvar 1 v w; set w 1; set r [expr {$w + [incr w]}]; return \"$r $w\"}\nset v 0\nputs [p]\nputs $v\n",
        "3 2\n2\n",
    ),
    // a write a word's own command makes
    (
        "set s a\nset r [expr {[string length [append s bc]] + [string length $s]}]\nputs \"$r $s\"\n",
        "6 abc\n",
    ),
    (
        "set s a\nset r [expr {[string length $s] + [string length [append s bc]]}]\nputs \"$r $s\"\n",
        "4 abc\n",
    ),
    (
        "set l {}\nset r [expr {[llength [lappend l a]] + [llength $l]}]\nputs \"$r $l\"\n",
        "2 a\n",
    ),
    (
        "set x 5\nset r [expr {$x * [set x 2] + $x}]\nputs \"$r $x\"\n",
        "12 2\n",
    ),
    (
        "set s a\nset r [expr {[string length [append s [append s b]]]}]\nputs \"$r $s\"\n",
        "4 abab\n",
    ),
    (
        "set x 1\nset r [expr {$x} + [incr x]]\nputs \"$r $x\"\n",
        "4 2\n",
    ),
    (
        "unset -nocomplain u\nset r [expr {[string length [append u [set u 3]]]}]\nputs \"$r $u\"\n",
        "2 33\n",
    ),
    // substitutions inside substitutions
    (
        "set x 1\nset r [expr {[expr {$x + [incr x]}] + [incr x]}]\nputs \"$r $x\"\n",
        "6 3\n",
    ),
    (
        "set x 1\nset r [expr {[set y [incr x]] + $y + $x}]\nputs \"$r $x $y\"\n",
        "6 2 2\n",
    ),
    (
        "set x 1\nset r [expr {[incr x] * [incr x] * [incr x]}]\nputs \"$r $x\"\n",
        "24 4\n",
    ),
    // the statements a substitution sits in
    ("set x 1\nexpr {[incr x] + [incr x]}\nputs $x\n", "3\n"),
    (
        "set x 1\nif {[incr x] == 2} { puts yes } else { puts no }\nputs $x\n",
        "yes\n2\n",
    ),
    ("set x 1\nwhile {[incr x] < 5} { }\nputs $x\n", "5\n"),
    (
        "proc p {} {set x 1; return [expr {[incr x] + $x}]}\nputs [p]\n",
        "4\n",
    ),
    (
        "proc p {} {set x 1; set y [expr {[incr x] + $x}]; return \"$y $x\"}\nputs [p]\n",
        "4 2\n",
    ),
    ("set x 1\nputs [expr {$x + [incr x]}]\nputs $x\n", "3\n2\n"),
    (
        "set x 1\nset y [list [expr {[incr x] + 1}] $x]\nputs $y\n",
        "3 2\n",
    ),
    ("set x 1\nputs \"[expr {[incr x] + 1}] $x\"\n", "3 2\n"),
    (
        "set x 1\nset r [expr \"$x + [incr x] + $x\"]\nputs \"$r $x\"\n",
        "5 2\n",
    ),
    (
        "set x 1\nset r [expr {\"$x [incr x]\" eq {1 2}}]\nputs \"$r $x\"\n",
        "1 2\n",
    ),
    // a quoted or unbraced operand
    (
        "set x 1\nset r [expr $x + [incr x]]\nputs \"$r $x\"\n",
        "3 2\n",
    ),
    (
        "set x 1\nset r [expr $x \"+\" [incr x]]\nputs \"$r $x\"\n",
        "3 2\n",
    ),
    (
        "set x 1\nset r [expr {[incr x] + [info exists x]}]\nputs \"$r $x\"\n",
        "3 2\n",
    ),
    (
        "unset -nocomplain x\nset r [expr {[info exists x] + [set x 3]}]\nputs \"$r $x\"\n",
        "3 3\n",
    ),
    (
        "unset -nocomplain x\nset r [expr {[set x 3] + [info exists x]}]\nputs \"$r $x\"\n",
        "4 3\n",
    ),
    // existence
    (
        "set x 1\ncatch {set r [expr {[incr x] + [error mid]}]}\nputs $x\n",
        "2\n",
    ),
    (
        "set x 1\ncatch {set r [expr {1 / 0 + [incr x]}]}\nputs $x\n",
        "1\n",
    ),
    (
        "set x 1\ncatch {set r [expr {[incr x] + 1 / 0}]}\nputs $x\n",
        "2\n",
    ),
    // a value that is not an integer
    (
        "set x 1.5\nset r [expr {$x + [set x 2.5] + $x}]\nputs \"$r $x\"\n",
        "6.5 2.5\n",
    ),
];

/// A write a substitution inside an expression makes is a definition of the
/// statement that holds the expression, made before the statement takes the
/// result: each program prints what tclsh 8.4 to 9.1 print, before and after
/// the optimiser forwards the values the shared lattice now knows. They cover
/// a loop that carries the write round, a branch or a short-circuit that
/// skips it, an element of an array, a place the frame does not own, a write a
/// word's own command makes, a substitution inside a substitution, each kind
/// of statement that holds an expression, an operand that is quoted or
/// unbraced, a place the write creates, and a value that is not an integer.
#[test]
fn a_nested_write_in_a_host_prints_what_tclsh_prints() {
    crate::install_surfaces();
    for &(source, expected) in NESTED_WRITE_PROGRAMS {
        prints_under_every_release(source, expected);
    }
}

/// A command substitution that is the whole value of a statement is run in
/// order with its own writes whichever registry-owned route the command
/// declares: the statement's call holds what the command stored and the
/// statement what it returned, so `set a [incr n]` leaves `n` 2 as well as
/// `a`. Each program prints what tclsh 8.5 to 9.1 print before and after the
/// optimiser, and the values the lattice holds are the ones printed.
#[test]
fn a_command_substitution_lands_its_writes_beside_its_result() {
    crate::install_surfaces();
    type Program = (
        &'static str,
        &'static str,
        &'static [(&'static str, &'static str)],
    );
    let programs: [Program; 7] = [
        (
            "proc p {} {set n 1; set a [incr n]; return \"$a $n\"}\nputs [p]\n",
            "2 2\n",
            &[("a", "2"), ("n", "2")],
        ),
        (
            "proc p {} {set a [set b 5]; return \"$a $b\"}\nputs [p]\n",
            "5 5\n",
            &[("a", "5"), ("b", "5")],
        ),
        (
            "proc p {} {set s abc; set a [append s def]; return \"$a|$s\"}\nputs [p]\n",
            "abcdef|abcdef\n",
            &[("a", "abcdef"), ("s", "abcdef")],
        ),
        (
            "proc p {} {set l {}; set a [lappend l x y]; return \"$a|$l\"}\nputs [p]\n",
            "x y|x y\n",
            &[("a", "x y"), ("l", "x y")],
        ),
        (
            "proc p {} {set a [scan {7 8} {%d %d} x y]; return \"$a $x $y\"}\nputs [p]\n",
            "2 7 8\n",
            &[("a", "2"), ("x", "7"), ("y", "8")],
        ),
        (
            "proc p {} {set a [lassign {1 2 3} u v]; return \"$a|$u|$v\"}\nputs [p]\n",
            "3|1|2\n",
            &[("a", "3"), ("u", "1"), ("v", "2")],
        ),
        (
            "proc p {} {set d {}; set a [dict set d k v]; return \"$a|$d\"}\nputs [p]\n",
            "k v|k v\n",
            &[("a", "k v"), ("d", "k v")],
        ),
    ];
    for (source, expected, last) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            for &(var, text) in last {
                assert_eq!(
                    lattice_text(last_value(source, dialect, "::p", var)).as_deref(),
                    Some(text),
                    "{dialect}: {var} after\n{source}"
                );
            }
        }
        prints_under_releases_from(source, expected, "8.5");
    }
}

/// The interface page's ordered-state programs, each over `set x 1`: the
/// expression, the value it has and what `x` holds after it. The seventh
/// program stops at an error in the middle of its expression and is read
/// apart.
const ORDERED_STATE: [(&str, &str, &str); 6] = [
    ("{$x + [incr x] + $x}", "5", "2"),
    ("{0 && [incr x]}", "0", "1"),
    ("{$x + [set x 10] + $x}", "21", "10"),
    ("{[incr x] + [incr x]}", "5", "3"),
    ("{$x ? [incr x] : [incr x 10]}", "2", "2"),
    ("\"$x + [incr x]\"", "3", "2"),
];

/// The seven programs of the interface page's ordered-state paragraph reach
/// every consumer: the shared lattice holds the expression's value in `r#1`
/// and the last write the expression made in `x#2`, in a procedure and at the
/// top level and under every dialect; the optimiser forwards both into the
/// reads after it; and each optimised program prints what tclsh 8.4 to 9.1
/// print. Where the expression is a `puts` argument the statement is not
/// evaluated, no nested write is folded and `puts $x` still reads `x`, and the
/// program prints the same. The seventh program, an error between a write and
/// the end of its expression inside a `catch`, leaves `x` at 2: in a procedure
/// the lattice holds 2 in `x#2`, at the top level, where the `catch` is one
/// call, `x` is unknown, and nothing forwards the earlier value to the read.
#[test]
fn the_seven_ordered_state_witnesses() {
    crate::install_surfaces();
    for (expression, value, after) in ORDERED_STATE {
        let int = |text: &str| Some(LatticeValue::Const(ConstValue::int(text.parse().unwrap())));
        let top = format!("set x 1\nset r [expr {expression}]\nputs $r\nputs $x\n");
        let in_proc = format!(
            "proc p {{}} {{\n    set x 1\n    set r [expr {expression}]\n    puts $r\n    puts $x\n}}\np\n"
        );
        let shown = format!("{value}\n{after}\n");
        for dialect in DIALECTS {
            let unit = unit_of(&in_proc, dialect);
            assert_eq!(
                value_at(&unit, "::p", "r", 1),
                int(value),
                "{dialect}: {expression}"
            );
            assert_eq!(
                value_at(&unit, "::p", "x", 2),
                int(after),
                "{dialect}: {expression}"
            );
            let unit = unit_of(&top, dialect);
            assert_eq!(
                top_value_at(&unit, "r", 1),
                int(value),
                "{dialect}: {expression}"
            );
            assert_eq!(
                top_value_at(&unit, "x", 2),
                int(after),
                "{dialect}: {expression}"
            );
            for source in [&top, &in_proc] {
                let (rewritten, rewrites) = optimised(source, dialect);
                assert!(
                    rewrites.iter().any(|o| o.code == DiagCode::O100)
                        && rewritten.contains(&format!("puts {value}"))
                        && rewritten.contains(&format!("puts {after}"))
                        && !rewritten.contains("puts $"),
                    "{dialect}: {expression}\n{rewritten}"
                );
            }
        }
        prints_under_every_release(&top, &shown);
        prints_under_every_release(&in_proc, &shown);

        let argument = format!("set x 1\nputs [expr {expression}]\nputs $x\n");
        for dialect in DIALECTS.iter().filter(|_| value != "0") {
            let (rewritten, _) = optimised(&argument, dialect);
            assert!(
                rewritten.contains("puts $x") && rewritten.contains("expr"),
                "{dialect}: {expression}\n{rewritten}"
            );
        }
        prints_under_every_release(&argument, &shown);
    }

    let caught = [
        "set x 1\ncatch {expr {[incr x] + [error mid]}} msg\nputs $msg\nputs $x\n",
        "proc p {} {\n    set x 1\n    catch {expr {[incr x] + [error mid]}} msg\n    puts $msg\n    puts $x\n}\np\n",
    ];
    for dialect in DIALECTS {
        // A `catch` at the top level is one call whose body writes are
        // may-definitions, so `x` is unknown after it; in a procedure its
        // body is blocks of its own, the error leaves from the statement
        // that raised it, and the handler is thrown to with the write the
        // expression had made.
        assert_eq!(
            top_value_at(&unit_of(caught[0], dialect), "x", 2),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
        assert_eq!(
            value_at(&unit_of(caught[1], dialect), "::p", "x", 2),
            Some(LatticeValue::Const(ConstValue::int(2))),
            "{dialect}"
        );
        for source in caught {
            let (rewritten, _) = optimised(source, dialect);
            assert!(
                rewritten.contains("set x 1") && rewritten.contains("puts $x"),
                "{dialect}: nothing forwards the earlier x\n{rewritten}"
            );
        }
    }
    for source in caught {
        prints_under_every_release(source, "mid\n2\n");
    }
    // The completion code, 1, is read from the options.
    let coded = "set x 1\ncatch {expr {[incr x] + [error mid]}} msg opts\nputs $msg\n\
                 puts [dict get $opts -code]\nputs $x\n";
    assert!(optimised(coded, "tcl8.6").0.contains("puts $x"));
    prints_under_releases_from(coded, "mid\n1\n2\n", "8.5");
}

/// A nested write to a place the ordered state cannot own — a qualified
/// global, a `global` or an `upvar` alias — declines the expression whatever
/// the write's own answer: the statement is not folded, its nested write stays,
/// and the optimised program prints what tclsh 8.4 to 9.1 print. Where the
/// write also reads a place nothing proves (`incr ::g`) the read is what
/// declines, and a branch the expression never reaches owns nothing, so
/// `0 && [set ::g 10]` is 0. Each program calls `p` at the top level, which
/// is no iRule, so they run under the core Tcl dialects.
#[test]
fn a_nested_write_outside_the_state_declines() {
    crate::install_surfaces();
    let programs = [
        (
            "proc p {} {\n set x 1\n set r [expr {$x + [set ::g 10]}]\n puts \"$r $::g\"\n}\np\n",
            "11 10\n",
            "declined: stateful-nested",
        ),
        (
            "proc p {} {\n global g\n set x 1\n set r [expr {$x + [set g 10]}]\n puts \"$r $g\"\n}\np\n",
            "11 10\n",
            "declined: stateful-nested",
        ),
        (
            "proc p {} {\n upvar 1 v w\n set x 1\n set r [expr {$x + [set w 10]}]\n puts \"$r $w\"\n}\nset v 0\np\n",
            "11 10\n",
            "declined: stateful-nested",
        ),
        (
            "set ::g 5\nproc p {} {\n set x 1\n set r [expr {$x + [incr ::g]}]\n puts \"$r $::g\"\n}\np\n",
            "7 6\n",
            "declined: not-exact",
        ),
    ];
    for (source, printed, answer) in programs {
        for dialect in TCL_CORE_DIALECTS {
            let unit = unit_of(source, dialect);
            assert_eq!(
                answers_for(&unit, "::p", "expr"),
                [answer],
                "{dialect}:\n{source}"
            );
            assert_eq!(
                value_at(&unit, "::p", "r", 1),
                Some(LatticeValue::Overdefined),
                "{dialect}:\n{source}"
            );
            let (rewritten, _) = optimised(source, dialect);
            assert!(
                rewritten.contains("expr {$x + [") && rewritten.contains("puts \"$r "),
                "{dialect}: the statement is kept\n{rewritten}"
            );
        }
        prints_under_every_release(source, printed);
    }

    let unreached = "proc p {} {\n set r [expr {0 && [set ::g 10]}]\n puts $r\n}\np\n";
    for dialect in DIALECTS {
        assert_eq!(
            value_at(&unit_of(unreached, dialect), "::p", "r", 1),
            Some(LatticeValue::Const(ConstValue::int(0))),
            "{dialect}"
        );
    }
    prints_under_every_release(unreached, "0\n");
}

/// What a prefix-rule program leaves that the analysis proves.
pub(super) enum PrefixFact {
    /// The place's last version holds this text.
    Text(&'static str, &'static str),
    /// The place's last version holds this number, whichever way a route
    /// writes it.
    Number(&'static str, i64),
    /// The place's last version is unbound.
    Unbound(&'static str),
}

/// One of the interface page's nine programs for the prefix rule
/// (§ *`catch`, `try`, and completion*): the statements before the `catch`,
/// the body it runs — an error after some of its stores — what is printed
/// after it, what every release prints, the first release with the command,
/// and what the analysis proves of a body its blocks hold. A body that is one
/// straight-line block is lowered into blocks in a procedure; the rest, and
/// every body at the top level, stay one call.
struct PrefixProgram {
    pub(super) name: &'static str,
    before: &'static str,
    pub(super) body: &'static str,
    after: &'static str,
    pub(super) printed: &'static str,
    pub(super) first: &'static str,
    flattened: &'static [PrefixFact],
}

const PREFIX_PROGRAMS: [PrefixProgram; 10] = [
    PrefixProgram {
        name: "lassign stops at the array",
        before: "set a old\narray set b {k keep}",
        body: "lassign {new second} a b",
        after: "puts \"$a $b(k)\"",
        printed: "new keep\n",
        first: "8.5",
        flattened: &[
            PrefixFact::Text("a", "new"),
            PrefixFact::Text("b(k)", "keep"),
        ],
    },
    PrefixProgram {
        name: "lassign leaves the places after the array",
        before: "array set b {k keep}\nset a old\nset c old",
        body: "lassign {x y z} a b c",
        after: "puts \"$a $c\"",
        printed: "x old\n",
        first: "8.5",
        flattened: &[PrefixFact::Text("a", "x"), PrefixFact::Text("c", "old")],
    },
    PrefixProgram {
        name: "foreach binds in order",
        before: "array set b {k keep}\nset a old",
        body: "foreach {a b} {new second} {set inside 1}",
        after: "puts \"$a [info exists inside]\"",
        printed: "new 0\n",
        first: "8.4",
        flattened: &[],
    },
    PrefixProgram {
        name: "scan writes the place before the array",
        before: "array set b {k keep}\nset a old",
        body: "scan {1 2} {%d %d} a b",
        after: "puts $a",
        printed: "1\n",
        first: "8.4",
        flattened: &[PrefixFact::Number("a", 1)],
    },
    PrefixProgram {
        name: "scan goes on past the array",
        before: "array set a {k keep}\nset b old",
        body: "scan {1 2} {%d %d} a b",
        after: "puts \"$a(k) $b\"",
        printed: "keep 2\n",
        first: "8.4",
        flattened: &[PrefixFact::Number("b", 2)],
    },
    PrefixProgram {
        name: "regexp stops at the array",
        before: "array set b {k keep}\nset a old",
        body: "regexp {(x)(y)} xy a b",
        after: "puts $a",
        printed: "xy\n",
        first: "8.4",
        flattened: &[PrefixFact::Text("a", "xy")],
    },
    PrefixProgram {
        name: "unset stops at the absent name",
        before: "set p 1\nset q 2",
        body: "unset p nosuch q",
        after: "puts \"[info exists p] $q\"",
        printed: "0 2\n",
        first: "8.4",
        flattened: &[PrefixFact::Unbound("p"), PrefixFact::Number("q", 2)],
    },
    PrefixProgram {
        name: "an error in a word is before the command",
        before: "set x 1",
        body: "append x 2 [error boom]",
        after: "puts $x",
        printed: "1\n",
        first: "8.4",
        flattened: &[PrefixFact::Number("x", 1)],
    },
    PrefixProgram {
        name: "an error in the value is before the store",
        before: "",
        body: "set r [expr {1 + [error mid]}]",
        after: "puts [info exists r]",
        printed: "0\n",
        first: "8.4",
        flattened: &[PrefixFact::Unbound("r")],
    },
    PrefixProgram {
        name: "an error in an expression keeps its writes",
        before: "set x 1",
        body: "expr {[incr x] + [error mid]}",
        after: "puts $x",
        printed: "2\n",
        first: "8.4",
        flattened: &[PrefixFact::Number("x", 2)],
    },
];

impl PrefixProgram {
    /// The program as a script: the `catch` at the top level is one call.
    fn at_the_top_level(&self) -> String {
        format!(
            "{}\ncatch {{{}}} m\n{}\n",
            self.before, self.body, self.after
        )
    }

    /// The program in a procedure, where a one-block body is lowered.
    pub(super) fn in_a_procedure(&self) -> String {
        format!(
            "proc p {{}} {{\n{}\ncatch {{{}}} m\n{}\n}}\np\n",
            self.before, self.body, self.after
        )
    }

    /// The program with `try … on error` for its `catch` (8.6 on).
    fn with_a_handler(&self) -> String {
        format!(
            "proc p {{}} {{\n{}\ntry {{{}}} on error {{m}} {{}}\n{}\n}}\np\n",
            self.before, self.body, self.after
        )
    }

    /// The program with a store after its body in the `try` (8.6 on): the
    /// body's statements are a block each, the handler is thrown to from the
    /// state the failing command leaves, and the store after it never runs.
    fn with_a_store_after_in_the_try(&self) -> String {
        format!(
            "proc p {{}} {{\n{}\ntry {{{}; set w 0}} on error {{m}} {{}}\n{}\n}}\np\n",
            self.before, self.body, self.after
        )
    }

    /// What the analysis of `function` claims of `fact`: that it holds, a
    /// different value or binding, or nothing.
    pub(super) fn claim(
        fact: &PrefixFact,
        function: &tcl_compiler::compilation_unit::FunctionUnit,
    ) -> Claim {
        let last = |name: &str| {
            let symbol = function.ssa.var_symbol(name).expect("the variable");
            function
                .sccp
                .values
                .iter()
                .filter(|((sym, _), _)| *sym == symbol)
                .max_by_key(|((_, version), _)| *version)
                .map(|(_, value)| value.clone())
        };
        let value = |name: &str, texts: &[String]| match last(name) {
            Some(LatticeValue::Const(ConstValue::String(held)))
                if texts.iter().any(|text| text == held.as_str()) =>
            {
                Claim::Holds
            }
            Some(LatticeValue::Const(ConstValue::Number(NumericValue::Int(held))))
                if texts.contains(&held.to_string()) =>
            {
                Claim::Holds
            }
            Some(LatticeValue::Const(_)) => Claim::Contradicts,
            _ => Claim::Silent,
        };
        match fact {
            PrefixFact::Text(name, text) => value(name, &[(*text).to_owned()]),
            PrefixFact::Number(name, number) => value(name, &[number.to_string()]),
            PrefixFact::Unbound(name) => {
                let symbol = function.ssa.var_symbol(name).expect("the variable");
                let held = function
                    .sccp
                    .existence
                    .iter()
                    .filter(|((sym, _), _)| *sym == symbol)
                    .max_by_key(|((_, version), _)| *version)
                    .map(|(_, fact)| *fact);
                match held {
                    Some(Existence::Unbound) => Claim::Holds,
                    Some(Existence::Bound(_)) => Claim::Contradicts,
                    _ => Claim::Silent,
                }
            }
        }
    }
}

/// What an analysis says of a fact tclsh bears out.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Claim {
    /// It proves the fact.
    Holds,
    /// It proves something else, which tclsh does not do.
    Contradicts,
    /// It proves nothing of the place.
    Silent,
}

/// The prefix rule holds in the default build — a `catch` at the top level, or
/// one whose body is not a straight line, is one call whose body writes are
/// may-definitions, and a `catch` in a procedure with a one-block body is
/// lowered into blocks — and in the faithful-exceptions build, which gives a
/// `try` handler blocks and exception edges: an error after `k` stores leaves
/// those `k` and nothing else, so the handler is thrown to with them, and so
/// it is where a store after the body in the `try` splits it into blocks: the
/// state before the failing command reaches the handler only where that
/// command can fail before it stores. Each
/// program prints what tclsh 8.4 to 9.1 print, before and after `tcl opt`, in
/// every shape. Where a body's blocks are the analysis's own, what it proves
/// is exact: the place written before the error holds the value it was given
/// and the places after it hold what they did. A body that is one call
/// proves no such thing, and the places it may write hold no value.
#[test]
fn the_prefix_rule_holds_in_both_builds() {
    crate::install_surfaces();
    for program in &PREFIX_PROGRAMS {
        let (top, in_proc, handled, split) = (
            program.at_the_top_level(),
            program.in_a_procedure(),
            program.with_a_handler(),
            program.with_a_store_after_in_the_try(),
        );
        prints_under_releases_from(&top, program.printed, program.first);
        prints_under_releases_from(&in_proc, program.printed, program.first);
        let handler_first = if program.first < "8.6" {
            "8.6"
        } else {
            program.first
        };
        prints_under_releases_from(&handled, program.printed, handler_first);
        prints_under_releases_from(&split, program.printed, handler_first);

        for dialect in ["tcl8.6", "tcl9.0"] {
            for fact in program.flattened {
                // The body's blocks are the analysis's own: it proves the fact.
                for source in [&in_proc, &handled, &split] {
                    let unit = unit_of(source, dialect);
                    let function = unit
                        .procedures
                        .get(&fixture_id(&unit.procedures, "::p"))
                        .expect("the procedure");
                    assert_eq!(
                        PrefixProgram::claim(fact, function),
                        Claim::Holds,
                        "{}: {dialect}\n{source}",
                        program.name
                    );
                }
                // One call whose body writes are may-definitions proves
                // nothing of a place it may write, and never the value the
                // place held before it.
                let unit = unit_of(&top, dialect);
                assert_ne!(
                    PrefixProgram::claim(fact, &unit.top_level),
                    Claim::Contradicts,
                    "{}: {dialect}\n{top}",
                    program.name
                );
            }
        }
    }
}
