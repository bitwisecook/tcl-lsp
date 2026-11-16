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

//! Loops: enumeration over proved places, the state a loop leaves, `break` and `continue`, counters and the iteration cap.

use super::*;

/// The loops a `catch` body holds, each in the statement form in a procedure:
/// the body of the loop, a command after the loop in the same script, what
/// tclsh 8.4 to 9.1 leave in the result variable, and the first release with
/// what the body uses. A loop absorbs its body's `break` and `continue`, so
/// the script goes on past the loop; every other completion is the loop's
/// own, so the script ends there with it.
const LOOP_BODIES: [(&str, &str, &str, &str); 12] = [
    (
        "foreach x {1 2 3} {set y $x; break}",
        "set z $y",
        "1",
        "8.4",
    ),
    (
        "foreach x {1 2 3} {set y $x; continue; set y no}",
        "set z $y",
        "3",
        "8.4",
    ),
    (
        "foreach x {1 2} {set y $x; error boom}",
        "set z done",
        "boom",
        "8.4",
    ),
    (
        "foreach x {1 2} {set y $x; return r}",
        "set z done",
        "r",
        "8.4",
    ),
    (
        "foreach x {1 2} {foreach y {a b} {break}; set z $x}",
        "set w $z",
        "2",
        "8.4",
    ),
    (
        "set z none; foreach x {} {set z ran}",
        "set z",
        "none",
        "8.4",
    ),
    ("foreach x {1 2} {set x}", "", "", "8.4"),
    (
        "foreach {a b} {1 2 3} {set z \"$a:$b\"}",
        "set z",
        "3:",
        "8.4",
    ),
    ("lmap x {1 2 3 4} {set x}", "", "1 2 3 4", "8.6"),
    ("lmap x {1 2 3} {set y $x; continue}", "", "", "8.6"),
    ("lmap x {1 2 3} {set y $x; break}", "", "", "8.6"),
    (
        "foreach x {1 2 3} {set y $x; break;}",
        "set z $y",
        "1",
        "8.4",
    ),
];

/// A loop in a `catch` body runs as its iteration plan says
/// (`IterationPlan::step`, `LoopResult::of`): `break` ends the loop and
/// `continue` the iteration, each absorbed, and an error, a `return`, a code of
/// the body's own and a `break` at level 1, which the loop sees as a
/// `return`, leave it. `foreach` yields the empty string and `lmap` the
/// results of the iterations that completed normally. The loop's variables
/// are stores of the script, taken where their kind is proved: an array
/// raises after the stores before it, and a variable of unproven kind, like a
/// list the analysis does not know, leaves the `catch` unevaluated. Each
/// program prints what tclsh prints before and after `tcl opt`.
#[test]
fn a_loop_absorbs_break_and_continue() {
    for (body, after, held, first) in LOOP_BODIES {
        let script = if after.is_empty() {
            body.to_owned()
        } else {
            format!("{body}; {after}")
        };
        let source = format!(
            "proc p {{}} {{\n    catch {{{script}}} m\n    return \"<$m>\"\n}}\nputs [p]\n"
        );
        let dialects: &[&str] = if first == "8.6" {
            &["tcl8.6", "tcl9.0"]
        } else {
            &DIALECTS
        };
        for dialect in dialects {
            assert_eq!(
                lattice_text(last_value(&source, dialect, "::p", "m")).as_deref(),
                Some(held),
                "{dialect}: {script}"
            );
        }
        prints_under_releases_from(&source, &format!("<{held}>\n"), first);
    }

    // The value form, the loop variable also set where the script starts, so
    // it is among what the statement writes: the code and the message.
    let value_forms = [
        ("foreach x {1 2} {break}", 0, "", "0 {}", "8.4"),
        ("foreach x {1 2} {error boom}", 1, "boom", "1 boom", "8.4"),
        (
            "foreach x {1 2} {return -code break b}",
            2,
            "b",
            "2 b",
            "8.4",
        ),
        (
            "foreach x {1 2} {return -level 0 -code 5 v}",
            5,
            "v",
            "5 v",
            "8.5",
        ),
    ];
    for (body, code, held, printed, first) in value_forms {
        let source = format!(
            "proc p {{}} {{\n    set c [catch {{set x 0; {body}}} m]\n    return [list $c $m]\n}}\nputs [p]\n"
        );
        for dialect in ["tcl8.6", "tcl9.0"] {
            let unit = unit_of(&source, dialect);
            assert_eq!(
                value_at(&unit, "::p", "c", 1),
                Some(LatticeValue::Const(ConstValue::Int(code))),
                "{dialect}: {body}"
            );
            assert_eq!(
                value_at(&unit, "::p", "m", 1)
                    .and_then(lattice_text)
                    .as_deref(),
                Some(held),
                "{dialect}: {body}"
            );
        }
        prints_under_releases_from(&source, &format!("{printed}\n"), first);
    }

    // A loop variable the script proved an array raises there, after the
    // variables bound before it: the code is 1 and `a` is `new`.
    let raises = "proc p {} {\n    set c [catch {set a old; set b(k) keep; foreach {a b} {new second} {set inside 1}} m]\n    return [list $c $a [info exists inside]]\n}\nputs [p]\n";
    for dialect in DIALECTS {
        let unit = unit_of(raises, dialect);
        assert_eq!(
            value_at(&unit, "::p", "c", 1),
            Some(LatticeValue::Const(ConstValue::Int(1))),
            "{dialect}"
        );
        assert_eq!(
            last_value(raises, dialect, "::p", "a"),
            text("new"),
            "{dialect}"
        );
    }
    prints_under_every_release(raises, "1 new 0\n");

    // What leaves the `catch` unevaluated, and the variable that raises: an
    // unknown list, a loop variable that may be an array, and one that is.
    let undecided = [
        "proc p {l} {\n    catch {foreach x $l {break}; set z done} m\n    return [string equal $m done]\n}\nputs [p {1 2}]\n",
        "proc p {n} {\n    if {$n} {array set x {a 1}}\n    catch {foreach x {1 2} {break}; set z done} m\n    return [string equal $m done]\n}\nputs [p 1]\nputs [p 0]\n",
        "proc p {} {\n    array set b {k keep}\n    set a old\n    catch {foreach {a b} {new second} {set inside 1}; set z done} m\n    return \"[string equal $m done] $a [info exists inside]\"\n}\nputs [p]\n",
    ];
    let printed = ["1\n", "0\n1\n", "0 new 0\n"];
    for (source, printed) in undecided.into_iter().zip(printed) {
        for dialect in DIALECTS {
            let held = last_value(source, dialect, "::p", "m");
            assert_ne!(held, text("done"), "{dialect}:\n{source}");
        }
        prints_under_every_release(source, printed);
    }
}

/// A loop that runs past the enumeration cap
/// (`DEFAULT_MAX_STATIC_LOOP_ITERS` passes) declines: the solver publishes
/// nothing on its exit, so the branch after it decides nothing and the
/// widened lattice stands. A loop that runs to the cap is enumerated, and
/// its branch decides. tclsh 8.4 to 9.1 print `yes` for both, before and
/// after `tcl opt`.
#[test]
fn the_iteration_cap_publishes_nothing() {
    let cap = tcl_compiler::static_loops::DEFAULT_MAX_STATIC_LOOP_ITERS;
    for (bound, enumerated) in [(cap, true), (cap + 1, false)] {
        let source = format!(
            "proc p {{}} {{\n    for {{set i 0}} {{$i < {bound}}} {{incr i}} {{}}\n    \
             if {{$i == {bound}}} {{puts yes}} else {{puts no}}\n}}\np\n"
        );
        let condition = format!("$i == {bound}");
        for dialect in DIALECTS {
            let unit = unit_of(&source, dialect);
            let function = unit
                .procedures
                .get(&fixture_id(&unit.procedures, "::p"))
                .expect("the procedure");
            let decided = function
                .sccp
                .constant_branches
                .iter()
                .find(|branch| branch.condition == condition)
                .map(|branch| branch.value);
            assert_eq!(decided, enumerated.then_some(true), "{dialect}: {bound}");
            assert_eq!(
                function.sccp.loop_enumerations.len(),
                usize::from(enumerated),
                "{dialect}: {bound}"
            );
        }
        prints_under_every_release(&source, "yes\n");
    }
}

/// The enumeration stores only to a place whose kind it proves — one it
/// holds a scalar of, or holds unbound — and names an element as the word
/// gives it: `foreach b` over an array `b` raises on its first binding, and
/// `${a(k)}` and `set a(k) …` raise on a scalar `a`, so a `catch` body that
/// does any of them leaves its counter where the error left it. Had the run
/// bound the array as a scalar, or read and written `a(k)` as `a`, `$n == 0`
/// would decide false and `i` would leave the `catch` at 3. A binder takes
/// the same rule as every other store, so `foreach a(k) {1 2} {}` is not
/// run, as `set a(k) …` is not. tclsh 8.4 to 9.1 print `zero`, `0` twice and
/// `two`, before and after `tcl opt`.
#[test]
fn an_enumeration_runs_only_over_places_it_proves() {
    let array_binder = "proc p {} {\n    array set b {k keep}\n    set n 0\n    \
                        catch {foreach b {1 2} {incr n}}\n    \
                        if {$n == 0} {puts zero} else {puts other}\n}\np\n";
    let element = |command: &str| {
        format!(
            "proc p {{}} {{\n    set a 5\n    set i 9\n    \
             catch {{for {{set i 0}} {{$i < 3}} {{incr i}} {{set y $a; {command}}}}}\n    \
             puts $i\n}}\np\n"
        )
    };
    let elements = [element("set x ${a(k)}"), element("set a(k) $i")];
    let element_binder = "proc p {} {\n    set a(k) 0\n    foreach a(k) {1 2} {}\n    \
                          if {$a(k) == 2} {puts two} else {puts other}\n}\np\n";
    for dialect in DIALECTS {
        let binder_unit = unit_of(element_binder, dialect);
        assert!(
            binder_unit
                .procedures
                .get(&fixture_id(&binder_unit.procedures, "::p"))
                .expect("the procedure")
                .sccp
                .loop_enumerations
                .is_empty(),
            "{dialect}"
        );
        let unit = unit_of(array_binder, dialect);
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("the procedure");
        assert!(
            function
                .sccp
                .constant_branches
                .iter()
                .all(|branch| branch.condition != "$n == 0"),
            "{dialect}: {:?}",
            function.sccp.constant_branches
        );
        for source in &elements {
            assert!(
                !matches!(
                    last_value(source, dialect, "::p", "i"),
                    LatticeValue::Const(_)
                ),
                "{dialect}\n{source}"
            );
        }
    }
    prints_under_every_release(array_binder, "zero\n");
    for source in &elements {
        prints_under_every_release(source, "0\n");
    }
    prints_under_every_release(element_binder, "two\n");
}

/// A name a loop rebinds that is dead after it — a temporary set before the
/// loop and in its body, read nowhere after — has no φ where the loop leaves,
/// so the version live there is the one from before the loop, which the
/// loop's state does not describe, and the solver states nothing of it. It
/// had stated the loop's last value of the name for that version, which the
/// settled run contradicted, and the contradiction dropped every loop's state
/// in the unit: the loop's own counter, and a later loop's that has nothing
/// to do with the name. A contradiction now drops only the loop whose state
/// it contradicts. The branch after each loop decides, for I230 as for `tcl
/// opt`. tclsh 8.4 to 9.1 print `yes`, `yes` and `three`, `yes`, and `yes`,
/// before and after `tcl opt`.
#[test]
fn a_name_dead_after_its_loop_keeps_every_loops_state() {
    let top = "set tmp 1\nset n 0\nforeach x {1 2} { set tmp $x; incr n }\n\
               if {$n == 2} {puts yes} else {puts no}\n";
    let later = format!(
        "{top}for {{set i 0}} {{$i < 3}} {{incr i}} {{}}\n\
         if {{$i == 3}} {{puts three}} else {{puts other}}\n"
    );
    let dead_while = "set tmp 1\nset n 0\nwhile {$n < 2} { set tmp $n; incr n }\n\
                      if {$n == 2} {puts yes} else {puts no}\n";
    let idiom = "proc p {} {\n    set found 0\n    set last \"\"\n    \
                 foreach x {a b c} { set last $x; if {$x eq \"b\"} { set found 1 } }\n    \
                 if {$found} { return yes }\n    return no\n}\nputs [p]\n";
    for dialect in DIALECTS {
        for (source, claims) in [
            (top, vec![true]),
            (later.as_str(), vec![true, true]),
            (dead_while, vec![true]),
            (idiom, vec![true]),
        ] {
            assert_eq!(
                condition_claims(source, dialect),
                claims,
                "{dialect}\n{source}"
            );
        }
        assert_eq!(
            unit_of(&later, dialect)
                .top_level
                .sccp
                .loop_enumerations
                .len(),
            2,
            "{dialect}"
        );
    }
    prints_under_every_release(top, "yes\n");
    prints_under_every_release(&later, "yes\nthree\n");
    prints_under_every_release(dead_while, "yes\n");
    prints_under_every_release(idiom, "yes\n");
}

/// The state a `foreach` or a `while` leaves decides the branch after it,
/// for I230 as for `tcl opt`: the counter the solver keeps widened inside
/// each loop is 3, then -2, on the loop's exit edges. The `foreach` leaves by
/// its latch, whose test the analysis cannot read, so the solver reaches the
/// block after it while the counter's own value is still a transient
/// constant the loop's state rules out; that value decides nothing, and the
/// arm the state rules out is never reached. tclsh 8.4 to 9.1 print `three`
/// and `minus`, before and after `tcl opt`.
#[test]
fn the_state_a_loop_leaves_decides_the_branch_after_it() {
    let source = "proc p {} {\n    set n 0\n    foreach x {a b c} { incr n }\n    \
                  if {$n == 3} { puts three } else { puts other }\n    set i 10\n    \
                  while {$i > 0} { incr i -3 }\n    \
                  if {$i == -2} { puts minus } else { puts other }\n}\np\n";
    for dialect in DIALECTS {
        assert_eq!(condition_claims(source, dialect), [true, true], "{dialect}");
        let unit = unit_of(source, dialect);
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("the procedure");
        assert_eq!(
            function
                .sccp
                .loop_enumerations
                .iter()
                .map(|record| record.iterations)
                .collect::<Vec<_>>(),
            [3, 4],
            "{dialect}"
        );
    }
    prints_under_every_release(source, "three\nminus\n");
}

/// A loop's own test is the branch that leaves the loop when false, wherever
/// its block sits: `while 1` decided true draws no I230, and an `if` in a
/// loop's body is an `if` like any other, so one decided true there draws
/// one. tclsh 8.4 to 9.1 print `3` and `done`, before and after `tcl opt`.
#[test]
fn a_loops_own_test_is_the_branch_that_leaves_it() {
    let source = "proc p {} {\n    set n 0\n    for {set i 0} {$i < 3} {incr i} {\n        \
                  set k 1\n        if {$k == 1} { incr n }\n    }\n    puts $n\n}\n\
                  proc q {} {\n    while 1 { return done }\n}\np\nputs [q]\n";
    for dialect in DIALECTS {
        assert_eq!(condition_claims(source, dialect), [true], "{dialect}");
    }
    prints_under_every_release(source, "3\ndone\n");
}

/// A store before an opaque `switch` arm or an opaque `catch` body that may
/// write an element of the array it names is read by the statement: where
/// no arm runs, or the body raises before the element write lands, the base
/// holds what it held, so `set a 5` stays live, W220 does not report it, and
/// O109 does not delete it — which made `puts $a` raise `can't read "a": no
/// such variable` where tclsh prints `5`. The read is quoted, and
/// read-before-set reads the base as the definition it was, so a base never
/// set before draws no W210. tclsh 8.4 to 9.1 print `5` twice, `5`, `1`, and
/// `1` and `0`, before and after `tcl opt`.
#[test]
fn a_may_written_element_reads_the_store_to_its_array() {
    let switch_arm = "proc p {x} {\n    set a 5\n    switch -glob -- $x { x* {set a(k) 1} }\n    \
                      puts $a\n}\np y\np z\n";
    let catch_body = "set a 5\ncatch {set a(k) 1}\nputs $a\n";
    let never_set_catch = "catch {set a(k) 1}\nputs [array size a]\n";
    let never_set_switch = "proc p {x} {\n    switch -glob -- $x { x* {set a(k) 1} }\n    \
                            puts [array size a]\n}\np x\np y\n";
    for dialect in DIALECTS {
        for source in [switch_arm, catch_body] {
            assert!(
                !reports(source, dialect, DiagCode::W220),
                "{dialect}\n{source}"
            );
        }
        for source in [never_set_catch, never_set_switch] {
            assert!(
                !reports(source, dialect, DiagCode::W210),
                "{dialect}\n{source}"
            );
        }
    }
    prints_under_every_release(switch_arm, "5\n5\n");
    prints_under_every_release(catch_body, "5\n");
    prints_under_every_release(never_set_catch, "1\n");
    prints_under_every_release(never_set_switch, "1\n0\n");
}

/// A loop's body leaves the loop, or writes its counter, through a script
/// word however it is written: `if` runs a bare or a quoted body as it runs a
/// braced one, so `if {$i < 0} break` is an exit and `if {$i < -5} "set i
/// 20"` a write of the counter, and none of these loops is provably infinite
/// (#2381) — the first was W241, "counter $i starts at 5, moves by -1 per
/// step, and compares < 10 (never reached)". tclsh 8.4 to 9.1 print `-1`,
/// `-1`, `-6` and `19`, before and after `tcl opt`.
#[test]
fn a_bare_or_quoted_body_word_leaves_the_loop() {
    let programs = [
        (
            "for {set i 5} {$i < 10} {incr i -1} {if {$i < 0} break}\nputs $i\n",
            "-1\n",
        ),
        (
            "set i 5\nwhile {$i < 10} {if {$i < 0} break; incr i -1}\nputs $i\n",
            "-1\n",
        ),
        (
            "for {set i 0} {$i < 10} {incr i -1} {if {$i < -5} \"break\"}\nputs $i\n",
            "-6\n",
        ),
        (
            "for {set i 0} {$i < 10} {incr i -1} {if {$i < -5} \"set i 20\"}\nputs $i\n",
            "19\n",
        ),
    ];
    for (source, printed) in programs {
        for dialect in DIALECTS {
            assert!(
                !reports(source, dialect, DiagCode::W241),
                "{dialect}\n{source}"
            );
        }
        prints_under_every_release(source, printed);
    }
}

/// W241's counter path reads what may write the counter from the unit that
/// holds the loop: every statement of the loop's blocks that defines it, and
/// every call there to code the module cannot see. A write a `[…]` word
/// makes, a binder, a procedure's `uplevel` and the `upvar` helper idiom each
/// define the counter, and a call to a procedure another file defines — here
/// one the program sources — is one the module cannot see, at the top level
/// and in a procedure; each `while` was W241, "while loop is provably
/// infinite: counter $i starts at 5, moves by -1 per step, and compares < 10
/// (never reached)", and so was the `for` with a literal start, before the
/// slice. tclsh 8.4 to 9.1 print each program's number, before and after
/// `tcl opt`.
#[test]
fn whatever_writes_the_counter_keeps_w241_silent() {
    let helper = std::env::temp_dir().join(format!(
        "value-transfer-witness-helper-{}.tcl",
        std::process::id()
    ));
    std::fs::write(&helper, "proc foo {name} {upvar 1 $name v; set v 100}\n")
        .expect("the helper file");
    let source_helper = format!("source {{{}}}\n", helper.display());
    let sourced = [
        format!("{source_helper}set i 5\nwhile {{$i < 10}} {{incr i -1; foo i}}\nputs $i\n"),
        format!(
            "{source_helper}proc s {{}} {{\n    set i 5\n    \
             while {{$i < 10}} {{incr i -1; foo i}}\n    return $i\n}}\nputs [s]\n"
        ),
    ];
    let mut programs: Vec<(&str, &str)> = vec![
        (
            "set i 5\nwhile {$i < 10} {incr i -1; set j [incr i 20]}\nputs $i\n",
            "24\n",
        ),
        (
            "set i 5\nwhile {$i < 10} {incr i -1; foreach i {100} {}}\nputs $i\n",
            "100\n",
        ),
        (
            "proc q {} {uplevel 1 {set i 100}}\nset i 5\n\
             while {$i < 10} {incr i -1; q}\nputs $i\n",
            "100\n",
        ),
        (
            "proc foo {name} {upvar 1 $name v; set v 100}\nset i 5\n\
             while {$i < 10} {incr i -1; foo i}\nputs $i\n",
            "100\n",
        ),
        (
            "for {set i 5} {$i < 10} {incr i -1} {if {$i < 0} {set j [incr i 20]}}\nputs $i\n",
            "18\n",
        ),
    ];
    programs.extend(sourced.iter().map(|source| (source.as_str(), "100\n")));
    for (source, printed) in programs {
        for dialect in DIALECTS {
            assert!(
                !reports(source, dialect, DiagCode::W241),
                "{dialect}\n{source}"
            );
        }
        prints_under_every_release(source, printed);
    }
    let _ = std::fs::remove_file(&helper);
}

/// A loop statement's literal word is its value as Tcl substitutes it: a bare
/// or quoted word's escapes are decoded, so `lappend r a\x41` appends `aA`,
/// `append r \x41` appends `A`, and an `incr` by `\x31` adds 1. The
/// enumeration had handed each word on as written, so it left `r` holding
/// `a\x41 a\x41` and `tcl opt` folded the test after the loop to its false
/// arm. Each loop now leaves what Tcl leaves and the test decides true, for
/// I230 as for `tcl opt`. tclsh 8.4 to 9.1 print `yes` for each, before and
/// after `tcl opt`.
#[test]
fn a_loop_word_is_its_value_with_its_escapes_decoded() {
    for source in [
        "set r {}\nforeach x {1 2} { lappend r a\\x41 }\n\
         if {$r eq \"aA aA\"} {puts yes} else {puts no}\n",
        "set r {}\nfor {set i 0} {$i < 2} {incr i} { append r \\x41 }\n\
         if {$r eq \"AA\"} {puts yes} else {puts no}\n",
        "set r {}\nforeach x {1 2} { lappend r \"a\\x41\" }\n\
         if {$r eq \"aA aA\"} {puts yes} else {puts no}\n",
        "for {set i 0} {$i < 2} {incr i \\x31} {}\n\
         if {$i == 2} {puts yes} else {puts no}\n",
    ] {
        for dialect in DIALECTS {
            assert_eq!(
                condition_claims(source, dialect),
                [true],
                "{dialect}\n{source}"
            );
        }
        prints_under_every_release(source, "yes\n");
    }
}

/// The eleven loop programs of the interface page (§ *Bounded-loop
/// enumeration*), one each at the top level: the solver runs each loop to
/// its exit over exact state — a `for` to its false condition, after a
/// `break`, past a `continue`, against a fractional bound, from an empty
/// start script and with its counter written in its body; a `for` the error
/// path of an opaque `catch` leaves; a `foreach` over no elements, over a
/// list too short for its last pass, and to a `break` — and the branch after
/// it decides on what the loop leaves, for the lattice, for I230 and for
/// `tcl opt`, which keeps only the arm that runs: `i` 5; `t` 6; `i` 3 after
/// `break`; `t` 3 with `continue`; `i` 3 for `$i < 2.5`; `i` 4 for the
/// empty start; `i` 4 and `n` 2; `i` 2 on the error path; `x` unbound after
/// `foreach x {} {}`; `a` 3 and `b` empty; `n` 1 and `x` 2. tclsh 8.4 to
/// 9.1 print each program's answer before and after `tcl opt`.
#[test]
fn the_eleven_loop_witnesses() {
    let witnesses = [
        ("for {set i 0} {$i < 5} {incr i} {}\n", "$i == 5", "five"),
        (
            "set t 0; for {set i 0} {$i < 4} {incr i} {incr t $i}\n",
            "$t == 6",
            "six",
        ),
        (
            "for {set i 0} {$i < 10} {incr i} {if {$i == 3} break}\n",
            "$i == 3",
            "three",
        ),
        (
            "set t 0; for {set i 0} {$i < 4} {incr i} {if {$i == 1} continue; incr t}\n",
            "$t == 3",
            "three",
        ),
        ("for {set i 0} {$i < 2.5} {incr i} {}\n", "$i == 3", "three"),
        (
            "set i 0; for {} {$i < 3} {} {incr i 2}\n",
            "$i == 4",
            "four",
        ),
        (
            "set n 0; for {set i 0} {$i < 3} {incr i} {set i [expr {$i + 1}]; incr n}\n",
            "$i == 4 && $n == 2",
            "yes",
        ),
        (
            "catch {for {set i 0} {$i < 5} {incr i} {if {$i == 2} {error x}}}\n",
            "$i == 2",
            "two",
        ),
        ("foreach x {} {}\n", "[info exists x]", ""),
        ("foreach {a b} {1 2 3} {}\n", "$a == 3 && $b eq \"\"", "yes"),
        (
            "set n 0; foreach x {1 2 3} {if {$x == 2} break; incr n}\n",
            "$n == 1 && $x == 2",
            "yes",
        ),
    ];
    for (program, condition, taken) in witnesses {
        // The ninth program's test is the one that holds false.
        let holds = !taken.is_empty();
        let (then, otherwise) = if holds {
            (taken, "other")
        } else {
            ("bound", "unbound")
        };
        let source =
            format!("{program}if {{{condition}}} {{puts {then}}} else {{puts {otherwise}}}\n");
        let printed = if holds { then } else { otherwise };
        let dropped = if holds { otherwise } else { then };
        for dialect in DIALECTS {
            let unit = unit_of(&source, dialect);
            assert!(
                unit.top_level
                    .sccp
                    .constant_branches
                    .iter()
                    .any(|branch| { branch.condition == condition && branch.value == holds }),
                "{dialect}: {source}{:?}",
                unit.top_level.sccp.constant_branches
            );
            assert_eq!(
                condition_claims(&source, dialect),
                [holds],
                "{dialect}: {source}"
            );
            let (rewritten, _) = optimised(&source, dialect);
            assert!(
                !rewritten.contains(&format!("puts {dropped}")),
                "{dialect}: {source}{rewritten}"
            );
        }
        prints_under_every_release(&source, &format!("{printed}\n"));
    }
}

/// The interface page's correlated pairs (§ *The correlated finite-set
/// limit*) as the page writes them, `incr x [expr {$b / $a}]`: the
/// enumeration runs the amount's `[expr …]` over its state, so each loop
/// leaves what ordered execution leaves — `x` 20 and `y` 25 — where pairing
/// the binders by position would answer 20 for both and their cartesian
/// product neither, and each branch after a loop decides true. Inside each
/// loop the lattice still proves no amount — the quotient pairs two distinct
/// finite inputs, which it declines as correlated — so each `incr` declines.
/// tclsh 8.4 to 9.1 print `twenty` and `twentyfive`, before and after `tcl
/// opt`.
#[test]
fn the_correlated_pairs_decide_by_enumeration() {
    let source = "proc p {} {\n\
                   set x 0\n\
                   foreach {a b} {1 10 2 20} { incr x [expr {$b / $a}] }\n\
                   if {$x == 20} { puts twenty } else { puts other }\n\
                   set y 0\n\
                   foreach {a b} {1 20 2 10} { incr y [expr {$b / $a}] }\n\
                   if {$y == 25} { puts twentyfive } else { puts other }\n\
                  }\n";
    for dialect in DIALECTS {
        assert_eq!(condition_claims(source, dialect), [true, true], "{dialect}");
        let unit = unit_of(source, dialect);
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("the procedure");
        let published: Vec<(String, LatticeValue)> = function
            .sccp
            .loop_enumerations
            .iter()
            .flat_map(|record| record.published.iter().cloned())
            .filter(|(name, _)| name == "x" || name == "y")
            .collect();
        assert_eq!(
            published,
            [
                ("x".to_owned(), LatticeValue::Const(ConstValue::Int(20))),
                ("y".to_owned(), LatticeValue::Const(ConstValue::Int(25))),
            ],
            "{dialect}"
        );
        assert_eq!(
            answers_for(&unit, "::p", "incr"),
            ["declined: not-exact", "declined: not-exact"],
            "{dialect}"
        );
    }
    prints_under_every_release(&format!("{source}p\n"), "twenty\ntwentyfive\n");
}

/// A loop condition's math function is the one the module binds: with `abs`
/// rebound by `proc ::tcl::mathfunc::abs`, `$i < abs(-3)` runs the loop to
/// 99 under 8.5 to 9.1, so the enumeration declines — the shared lattice
/// under the module's observed bindings, the rewrite under its whole-module
/// trust — rather than stop at 3, and the branch after the loop decides
/// nothing. tclsh 8.5 to 9.1 print `other` at the top level and in a
/// procedure, before and after `tcl opt`.
#[test]
fn a_loop_condition_reads_the_math_binding() {
    let top_level = "proc ::tcl::mathfunc::abs {x} {return 99}\n\
                     for {set i 0} {$i < abs(-3)} {incr i} {}\n\
                     if {$i == 3} {puts three} else {puts other}\n";
    let in_a_procedure = "proc p {} {\n    for {set i 0} {$i < abs(-3)} {incr i} {}\n    \
                          if {$i == 3} {puts three} else {puts other}\n}\n\
                          proc ::tcl::mathfunc::abs {x} {return 99}\np\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        for source in [top_level, in_a_procedure] {
            assert!(
                !reports(source, dialect, DiagCode::I230),
                "{dialect}\n{source}"
            );
        }
        let unit = unit_of(top_level, dialect);
        assert!(
            unit.top_level.sccp.loop_enumerations.is_empty(),
            "{dialect}"
        );
        let unit = unit_of(in_a_procedure, dialect);
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("the procedure");
        assert!(function.sccp.loop_enumerations.is_empty(), "{dialect}");
    }
    prints_under_releases_from(top_level, "other\n", "8.5");
    prints_under_releases_from(in_a_procedure, "other\n", "8.5");
}

/// The argument-sensitive re-run that O103 folds a call from runs the
/// callee's loop under the call's seeds: `[f 3]` runs `f`'s `for` three times
/// and returns `t` at 6, which the loop's exit state holds where the return
/// reads it, so the call folds to 6. tclsh 8.4 to 9.1 print `6`, before and
/// after `tcl opt`.
#[test]
fn the_argument_sensitive_rerun_runs_the_callees_loop() {
    let source = "proc f {n} {\n    set t 0\n    for {set i 0} {$i < $n} {incr i} {incr t 2}\n    \
                  return $t\n}\nset r [f 3]\nputs $r\n";
    for dialect in DIALECTS {
        let (rewritten, applied) = optimised(source, dialect);
        assert!(
            applied
                .iter()
                .any(|rewrite| rewrite.code == DiagCode::O103 && rewrite.replacement == "6"),
            "{dialect}: {applied:?}"
        );
        assert!(!rewritten.contains("[f 3]"), "{dialect}:\n{rewritten}");
    }
    prints_under_every_release(source, "6\n");
}

/// The state an enumerated loop leaves bounds its counter past the loop
/// header's widening: after `for {set i 0} {$i < 5} {incr i} {}`, `i` is
/// exactly 5, so `lindex` of a three-element list at `$i` is past the end and
/// W230 says so, at the top level and in a procedure. tclsh 8.4 to 9.1 print
/// an empty line, before and after `tcl opt`.
#[test]
fn an_enumerated_loop_bounds_its_counter_after_it() {
    let top_level = "set l {a b c}\nfor {set i 0} {$i < 5} {incr i} {}\nputs [lindex $l $i]\n";
    let in_a_procedure = "proc p {} {\n    set l {a b c}\n    for {set i 0} {$i < 5} {incr i} {}\n    \
                          puts [lindex $l $i]\n}\np\n";
    for dialect in DIALECTS {
        for source in [top_level, in_a_procedure] {
            assert!(
                reports(source, dialect, DiagCode::W230),
                "{dialect}\n{source}"
            );
        }
    }
    prints_under_every_release(top_level, "\n");
    prints_under_every_release(in_a_procedure, "\n");
}

/// A double the lattice computes is spelled as Tcl spells it: `[expr {1.0 *
/// 3}]` is `3.0` and `[expr {1e300 * 10.0}]` is `1e+301`, so `string length`
/// of each is 3 and 6. The lattice had spelled them as Rust does, `3` and a
/// 302-digit integer, and `tcl opt` folded the calls to `1` and `302`. tclsh
/// 8.4 to 9.1 print `3` and `6`, before and after `tcl opt`.
#[test]
fn a_computed_double_is_spelled_as_tcl_spells_it() {
    let source = "proc p {} {\n    set x [expr {1.0 * 3}]\n    set n [string length $x]\n    \
                  return $n\n}\nproc q {} {\n    set y [expr {1e300 * 10.0}]\n    \
                  set m [string length $y]\n    return $m\n}\nputs [p]\nputs [q]\n";
    for dialect in DIALECTS {
        assert_eq!(
            last_value(source, dialect, "::p", "n"),
            LatticeValue::Const(ConstValue::Int(3)),
            "{dialect}"
        );
        assert_eq!(
            last_value(source, dialect, "::q", "m"),
            LatticeValue::Const(ConstValue::Int(6)),
            "{dialect}"
        );
    }
    prints_under_every_release(source, "3\n6\n");
}
