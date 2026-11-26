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

//! `catch`: nested and flattened bodies, result and options variables, the code table per release, and scripts that are not braced text.

use super::*;

/// A name the body of a `catch` writes on one path only keeps the value it
/// held before, so the store before the `catch` is read: tclsh prints `5` for
/// each program, where taking the body's write as the one every path makes
/// deleted `set g 5` and the read raised `can't read "g"`.
#[test]
fn a_store_a_catch_body_may_leave_untouched_is_not_dead() {
    let top = "set g 5\ncatch { if {[expr {[clock seconds] < 0}]} { set g 0 } }\nputs $g\n";
    let in_proc = "proc p {} {\n set g 5\n catch { if {[expr {[clock seconds] < 0}]} { set g 0 } }\n puts $g\n}\np\n";
    let with_result =
        "set g 5\ncatch { if {[expr {[clock seconds] < 0}]} { set g 0 } } msg\nputs $g\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        for source in [top, in_proc, with_result] {
            assert!(
                !removes_store(source, dialect, "set g 5"),
                "{dialect}: {source}"
            );
        }
    }
    for source in [top, in_proc, with_result] {
        prints_under_every_release(source, "5\n");
    }
}

/// A program, what tclsh prints for it, and the one condition it holds, if any:
/// its truth, and whether the analysis decides it.
type BodyProgram = (&'static str, &'static str, Option<(bool, bool)>);

/// Programs whose `catch` stands in a `[…]` substitution, with what tclsh
/// 8.4 to 9.1 print and the one condition each holds, if any: its truth, and
/// whether the analysis decides it. They are the three of #2231 (the second is
/// the statement form), one in a procedure, a result variable beside the
/// substitution, a `catch` in a `catch`, one reached through an alias of
/// `catch` (whose head the module rebinds, so nothing is decided), and a body
/// that sets a name, unsets one, substitutes a command, or writes through an
/// expression word.
const CATCH_BODY_PROGRAMS: &[BodyProgram] = &[
    (
        "set x 1\nset c [catch {incr x}]\nif {$x == 2} {puts two} else {puts \"not two: $x\"}\n",
        "two\n",
        Some((true, true)),
    ),
    ("set x 5\ncatch {incr x} m\nputs \"$x $m\"\n", "6 6\n", None),
    (
        "set x 1\nset c [catch {append x y}]\nputs $x\n",
        "1y\n",
        None,
    ),
    (
        "proc p {} {\n set x 1\n set c [catch {incr x}]\n if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        Some((true, true)),
    ),
    (
        "set x 5\nputs [catch {incr x} m]\nputs \"$x $m\"\n",
        "0\n6 6\n",
        None,
    ),
    (
        "set x 1\nset c [catch {catch {incr x}}]\nif {$x == 2} {puts two} else {puts other}\n",
        "two\n",
        Some((true, true)),
    ),
    (
        "set g 5\nset rc [catch {set g 0}]\nif {$g} {puts a} else {puts b}\n",
        "b\n",
        Some((false, true)),
    ),
    (
        "set x 1\nputs [catch {unset x}]\nputs [info exists x]\n",
        "0\n0\n",
        None,
    ),
    (
        "set x 1\nputs [catch {expr {[incr x] + [error mid]}}]\nputs $x\n",
        "1\n2\n",
        None,
    ),
    (
        "set x 5\nset c [catch {puts $x}]\nputs $c\n",
        "5\n0\n",
        None,
    ),
    ("set x 5\nset c [catch {incr x}]\nputs $c\n", "0\n", None),
    (
        "set x 1\nset c [catch {set y [incr x]}]\nif {$x == 2} {puts two} else {puts other}\n",
        "two\n",
        Some((true, true)),
    ),
    (
        "interp alias {} c {} catch\nset x 1\nset r [c {incr x}]\n\
         if {$x == 2} {puts two} else {puts other}\n",
        "two\n",
        Some((true, false)),
    ),
];

/// The body of a `catch`, or of a `try`, inside a `[…]` substitution runs once
/// in the frame the substitution is written in, whatever it completes with,
/// so what it writes and what it reads are the statement's own effect: the
/// writes are definitions after the statement and the reads are uses of the
/// definitions before it (#2231). Each program prints what it did before the
/// multipass optimiser rewrote it — where the lattice held the value before
/// the body, the stores the body reads were deleted — and the condition it
/// holds is claimed by no diagnostic but with the truth tclsh gives it; a
/// closed body under the builtin head decides it. A `try` body does the same
/// from 8.6.
#[test]
fn a_nested_catch_body_is_the_statements_effect() {
    for &(source, expected, condition) in CATCH_BODY_PROGRAMS {
        prints_under_every_release(source, expected);
        let truth = condition.map(|(truth, _)| truth);
        for dialect in DIALECTS {
            assert!(
                condition_claims(source, dialect)
                    .iter()
                    .all(|claimed| Some(*claimed) == truth),
                "{dialect}: a condition is decided as tclsh does not:\n{source}"
            );
            for store in ["set x 1", "set x 5"] {
                if source.contains(store) {
                    assert!(
                        !removes_store(source, dialect, store),
                        "{dialect}: {store} is read:\n{source}"
                    );
                }
            }
        }
        for dialect in ["tcl8.6", "tcl9.0"] {
            let decided = condition
                .filter(|(_, decided)| *decided)
                .map(|(truth, _)| truth);
            assert_eq!(
                condition_claims(source, dialect),
                decided.into_iter().collect::<Vec<_>>(),
                "{dialect}: the condition is decided exactly where the body is closed:\n{source}"
            );
        }
    }
    let in_try = "set x 1\nset r [try {incr x} on error {} {set y 0}]\n\
                  if {$x == 2} {puts two} else {puts other}\n";
    prints_under_releases_from(in_try, "two\n", "8.6");
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert!(
            condition_claims(in_try, dialect)
                .iter()
                .all(|claimed| *claimed),
            "{dialect}"
        );
    }
}

/// The scripts of the `catch` code table that read nothing the program has
/// not set, with what the result variable holds after `catch {script} m`
/// under tclsh 8.4 to 9.1.
const CATCH_RESULTS: &[(&str, &str)] = &[
    ("error boom", "boom"),
    ("return 5", "5"),
    ("break", ""),
    ("continue", ""),
    ("set v 1", "1"),
    ("expr {1/0}", "divide by zero"),
    ("set y 5; error $y", "5"),
    ("set y 5; set z $y; error $z", "5"),
    ("error a; error b", "a"),
    ("", ""),
    ("catch {error z}", "1"),
    ("set x 1; incr x", "2"),
    ("string length abc", "3"),
];

/// A closed `catch` — brace-quoted, every command with a route of its own and
/// every value exact — is evaluated to the code and the result its script
/// completes with, so the result variable holds what the script returned: the
/// last command's result, the message of the error it stopped at (the first
/// completion that is not a normal one ends the script, whatever follows), or
/// what `return` carried. The lattice holds it, the optimiser forwards it, and
/// tclsh 8.4 to 9.1 print the same before and after.
///
/// What the program does not prove stays unknown — an error whose message
/// reads a variable never set — and the variable is still bound.
#[test]
fn a_closed_catch_script_gives_its_result_variable_what_it_returned() {
    for &(script, expected) in CATCH_RESULTS {
        let source = format!("catch {{{script}}} m\nputs \"<$m>\"\n");
        for dialect in DIALECTS {
            let unit = unit_of(&source, dialect);
            assert_eq!(
                top_value_at(&unit, "m", 1).and_then(lattice_text),
                Some(expected.to_owned()),
                "{dialect}: {source}"
            );
            let (rewritten, _) = optimised(&source, dialect);
            assert!(
                rewritten.contains(&format!("puts \"<{expected}>\"")),
                "{dialect}:\n{rewritten}"
            );
        }
        prints_under_every_release(&source, &format!("<{expected}>\n"));
    }

    let unknown = "catch {error $nothing} m\nputs \"<$m>\"\n";
    for dialect in DIALECTS {
        assert_eq!(
            top_value_at(&unit_of(unknown, dialect), "m", 1),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
        let unbound_reads: Vec<String> = tcl_compiler::analyser::Analyser::new()
            .analyse(unknown, dialect)
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W210)
            .map(|diagnostic| diagnostic.message.clone())
            .collect();
        assert!(
            unbound_reads.iter().all(|message| !message.contains("'m'")),
            "{dialect}: the result variable is bound: {unbound_reads:?}"
        );
    }
    prints_under_every_release(unknown, "<can't read \"nothing\": no such variable>\n");

    // A script that is not brace-quoted text is not known: `"error $e"` runs
    // `error a b`, whose message is `a`, where its spelling would give `a b`.
    let quoted = "set e {a b}\ncatch \"error $e\" m\nputs \"<$m>\"\n";
    for dialect in DIALECTS {
        assert_eq!(
            top_value_at(&unit_of(quoted, dialect), "m", 1),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
    }
    prints_under_every_release(quoted, "<a>\n");
}

/// A word of a command inside a `catch` script that raises is the script's
/// error, which the `catch` absorbs: the statement itself does not raise, so
/// what follows it in a `try` body runs. tclsh 8.6 to 9.1 print `1`; taking
/// the word's error for the statement's ended the block and folded `after` to
/// the handler's `0`.
#[test]
fn an_error_a_catch_script_raises_is_not_the_statements_own() {
    // The script's `return` keeps the `catch` one opaque call, and without it
    // the script is lowered into the procedure's blocks.
    for script in ["set a [error boom]; return x", "set a [error boom]"] {
        let source = format!(
            "proc p {{}} {{\n    try {{\n        catch {{{script}}} m\n        \
             set after 1\n    }} on error {{}} {{\n        set after 0\n    }}\n    \
             return $after\n}}\nputs [p]\n"
        );
        prints_under_releases_from(&source, "1\n", "8.6");
        let unit = unit_of(&source, "tcl8.6");
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("the procedure");
        let executable: Vec<&str> = function
            .sccp
            .executable_blocks
            .iter()
            .filter_map(|id| function.cfg.blocks.get(id))
            .map(|block| block.name.as_str())
            .collect();
        assert!(
            executable.iter().any(|name| name.starts_with("try_ok")),
            "the body completes normally: {executable:?}"
        );
    }
}

/// A result the program does not prove is still bound, and the options
/// dictionary beside it is a dictionary whose text the analysis never has:
/// `catch {error $x} m o` evaluates to the code 1, binds `m` as a string and
/// `o` as a dictionary, each with a value that is not available, and draws
/// no W210 where they are read.
#[test]
fn an_unproven_result_and_the_options_dictionary_are_bound_and_typed() {
    use tcl_compiler::value_transfer::FoldedType;
    use tcl_registry::TclType;
    use tcl_registry::value_transfer::RepresentationEvidence;
    let source = "proc p {x} {\n    catch {error $x} m o\n    return [list $m $o]\n}\n";
    let stated = |ty| {
        Some(FoldedType {
            intrep: Some(ty),
            shape: None,
            representation: RepresentationEvidence::Unknown,
        })
    };
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
        let unit = unit_of(source, dialect);
        for var in ["m", "o"] {
            assert_eq!(
                value_at(&unit, "::p", var, 1),
                Some(LatticeValue::Overdefined),
                "{dialect}: {var}"
            );
        }
        assert_eq!(folded_at(&unit, "::p", "m", 1), stated(TclType::String));
        assert_eq!(folded_at(&unit, "::p", "o", 1), stated(TclType::Dict));
        assert_eq!(answers_for(&unit, "::p", "catch"), ["evaluated"]);
        assert!(!reports(source, dialect, DiagCode::W210), "{dialect}");
    }
}

/// The options dictionary of a `catch` starts `-code N -level L` for every
/// completion but an error (`error msg info code` lists `-errorinfo` first),
/// so that is all the route states of its text: tclsh 8.5 to 9.1 give each
/// row below that prefix. It is never an exact value, because a success is
/// not always `-code 0 -level 0` alone: from 8.6 `incr` and `lappend` of an
/// absent variable leave a stale `-errorcode {TCL READ VARNAME}` in the
/// options of a normal completion, and `-errorinfo`, `-errorline` and
/// `-errorstack` are the interpreter's own text.
#[test]
fn the_options_dictionary_starts_with_the_code_and_level_the_route_states() {
    let rows = [
        ("set v 1", "-code 0 -level 0"),
        ("return 5", "-code 0 -level 1"),
        ("break", "-code 3 -level 0"),
        ("continue", "-code 4 -level 0"),
        ("return -code 5 custom", "-code 5 -level 1"),
        ("incr absent", "-code 0 -level 0"),
        ("lappend absent x", "-code 0 -level 0"),
    ];
    let stale = "catch {incr absent} m o\nputs $o\n";
    let mut ran = 0;
    for (series, tclsh) in releases_on_path() {
        if series < "8.5" {
            continue;
        }
        ran += 1;
        for (script, prefix) in rows {
            let program = format!(
                "catch {{{script}}} m o\nputs [expr {{[string first {{{prefix}}} $o] == 0}}]\n"
            );
            assert_eq!(
                run_script(&tclsh, &program),
                Some((true, "1\n".to_owned())),
                "tclsh{series}: {program}"
            );
        }
        let expected = if series >= "8.6" {
            "-code 0 -level 0 -errorcode {TCL READ VARNAME}\n"
        } else {
            "-code 0 -level 0\n"
        };
        assert_eq!(
            run_script(&tclsh, stale),
            Some((true, expected.to_owned())),
            "tclsh{series}"
        );
    }
    assert!(ran > 0 || releases_on_path().is_empty());
    let source = "proc p {} {\n    catch {incr absent} m o\n    return $o\n}\n";
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
        assert_eq!(
            value_at(&unit_of(source, dialect), "::p", "o", 1),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
    }
}

/// Programs whose `catch`, or `try`, runs a script that is not brace-quoted
/// text, in a substitution, with what tclsh prints and the first release that
/// has the command: a quoted word that substitutes nothing, a script held in a
/// variable, a quoted word that substitutes, one with an escape, one that
/// writes what the script reads, a `catch` in a `catch`, a condition, and a
/// `try` body.
const UNBRACED_SCRIPT_PROGRAMS: &[(&str, &str, &str)] = &[
    (
        "set x 1\nset c [catch \"incr x\"]\nif {$x == 2} {puts two} else {puts other}\n",
        "two\n",
        "8.4",
    ),
    (
        "set x 1\nset c [catch \"incr x; set y 2\" m]\nif {$x == 2} {puts two} else {puts other}\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set c [catch \"incr x\"]\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.4",
    ),
    (
        "set x 1\nset s {incr x}\nset c [catch $s]\nif {$x == 2} {puts two} else {puts other}\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set s {incr x}\n    set c [catch $s]\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set v x\n    set c [catch \"incr $v\"]\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set c [catch \"incr x\\n\"]\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set c [catch \"catch {incr x}\"]\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set s {incr x}\n    if {[catch $s]} {puts bad}\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    if {[catch \"incr x\"]} {puts bad}\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set a 1\n    set b 1\n    set c [catch \"incr a; incr b\"]\n    \
         return \"$a$b\"\n}\nputs [p]\n",
        "22\n",
        "8.4",
    ),
    (
        "set x 5\nset s {puts $x}\nset c [catch $s]\nputs $c\n",
        "5\n0\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set r [try \"incr x\" on error {} {}]\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.6",
    ),
    (
        "proc p {} {\n    set x 1\n    set s {incr x}\n    set r [try $s on error {} {}]\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.6",
    ),
];

/// A script a substitution runs in this frame, once, whatever it completes
/// with, is the statement's effect wherever its text is known: a quoted word
/// that substitutes nothing is read as the brace-quoted one is, and a script
/// that is run-time data — held in a variable, substituted, escaped, expanded
/// — may write any name, so nothing after the statement is decided on a value
/// from before it, as after the statement form's barrier. Each program prints
/// what tclsh prints, before and after the optimiser; no diagnostic decides
/// its condition as tclsh does not, and the store a script reads stays.
#[test]
fn a_quoted_or_computed_script_is_the_statements_effect() {
    for &(source, expected, first) in UNBRACED_SCRIPT_PROGRAMS {
        prints_under_releases_from(source, expected, first);
        let dialects: &[&str] = if first == "8.4" {
            &DIALECTS
        } else {
            &["tcl8.6", "tcl9.0"]
        };
        for &dialect in dialects {
            assert!(
                condition_claims(source, dialect)
                    .iter()
                    .all(|claimed| *claimed),
                "{dialect}: a condition is decided as tclsh does not:\n{source}"
            );
        }
    }
    let reads = "set x 5\nset s {puts $x}\nset c [catch $s]\nputs $c\n";
    for dialect in DIALECTS {
        assert!(
            !removes_store(reads, dialect, "set x 5"),
            "{dialect}: the script reads x"
        );
    }
}

/// The rows of the `catch` code table that read nothing the program has not
/// set: the script, the code `catch` returns for it and what its result
/// variable holds, the same under tclsh 8.4 to 9.1. `return -code 5 custom`
/// completes with the code 2 where it stands in a `catch` — the code 5 is the
/// one a procedure's caller sees.
const CATCH_CODES: &[(&str, i64, &str)] = &[
    ("return 5", 2, "5"),
    ("break", 3, ""),
    ("continue", 4, ""),
    ("error boom", 1, "boom"),
    ("set v 1", 0, "1"),
    ("expr {1/0}", 1, "divide by zero"),
    ("return -code 5 custom", 2, "custom"),
    ("string length abc", 0, "3"),
];

/// The program one row of the code table runs: a procedure that keeps the
/// code of a `catch` in a substitution and the result beside it, and prints
/// both.
fn catch_program(script: &str) -> String {
    format!("proc p {{}} {{\n    set c [catch {{{script}}} m]\n    puts \"$c|$m\"\n}}\np\n")
}

/// The rows every release reads alike, under each release's dialect and the
/// `tcl` profile that spans them: the code, the result, the `evaluated`
/// answer and the optimiser's forwarding, and the lines tclsh 8.4 to 9.1
/// print before and after.
fn the_catch_rows_every_release_reads_alike() {
    for &(script, code, result) in CATCH_CODES {
        let source = catch_program(script);
        for dialect in RELEASE_DIALECTS {
            let unit = unit_of(&source, dialect);
            assert_eq!(
                value_at(&unit, "::p", "c", 1),
                Some(LatticeValue::Const(ConstValue::Int(code))),
                "{dialect}: the code of {script}"
            );
            assert_eq!(
                value_at(&unit, "::p", "m", 1).and_then(lattice_text),
                Some(result.to_owned()),
                "{dialect}: the result of {script}"
            );
            assert_eq!(
                answers_for(&unit, "::p", "catch"),
                ["evaluated"],
                "{dialect}: {script}"
            );
            let (rewritten, _) = optimised(&source, dialect);
            assert!(
                rewritten.contains(&format!("{code}|{result}")),
                "{dialect}: {script} is forwarded:\n{rewritten}"
            );
        }
        prints_under_every_release(&source, &format!("{code}|{result}\n"));
    }
}

/// `catch {incr absent}`: the error of 8.4 and of the iRules dialect on its
/// 8.4 base, the created cell from 8.5, and nothing under the spanning
/// profile, which cannot say which release runs.
fn the_catch_row_that_splits_on_an_absent_cell() {
    let absent = catch_program("incr absent");
    let message = "can't read \"absent\": no such variable";
    let created = Some((0, "1"));
    let raised = Some((1, message));
    for (dialect, expected) in [
        ("tcl8.4", raised),
        ("f5-irules", raised),
        ("tcl8.5", created),
        ("tcl8.6", created),
        ("tcl9.0", created),
        ("tcl9.1", created),
        ("tcl", None),
    ] {
        let unit = unit_of(&absent, dialect);
        let code_fact = value_at(&unit, "::p", "c", 1);
        let result_fact = value_at(&unit, "::p", "m", 1).and_then(lattice_text);
        if let Some((code, result)) = expected {
            assert_eq!(
                code_fact,
                Some(LatticeValue::Const(ConstValue::Int(code))),
                "{dialect}: the code"
            );
            assert_eq!(
                result_fact.as_deref(),
                Some(result),
                "{dialect}: the result"
            );
        } else {
            assert_eq!(code_fact, Some(LatticeValue::Overdefined), "{dialect}");
            assert_eq!(result_fact, None, "{dialect}");
        }
    }
    for (series, tclsh) in releases_on_path() {
        let expected = if series == "8.4" {
            format!("1|{message}\n")
        } else {
            "0|1\n".to_owned()
        };
        for text in [absent.clone(), optimised(&absent, &dialect_of(series)).0] {
            assert_eq!(
                run_script(&tclsh, &text),
                Some((true, expected.clone())),
                "tclsh{series}:\n{text}"
            );
        }
    }
}

/// A `catch` in a `[…]` substitution — where the code is the value of the
/// statement and the message or result a variable beside it — is evaluated to
/// the code its script completes with: the code table, forwarded by the
/// optimiser and printed by tclsh 8.4 to 9.1 before and after, with the
/// release split of `incr` on a cell that is absent as the one row that
/// differs. A procedure's own completion is a callee's summary, so `catch
/// {q}` over a `proc q {} {return -code 5 custom}` states nothing of the code
/// 5 tclsh prints.
#[test]
fn the_catch_code_table() {
    the_catch_rows_every_release_reads_alike();
    the_catch_row_that_splits_on_an_absent_cell();
    let callee = "proc q {} {return -code 5 custom}\n\
                  proc p {} {\n    set c [catch {q} m]\n    puts \"$c|$m\"\n}\np\n";
    for dialect in RELEASE_DIALECTS {
        assert_eq!(
            value_at(&unit_of(callee, dialect), "::p", "c", 1),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
    }
    prints_under_every_release(callee, "5|custom\n");
}

/// A script that is not brace-quoted text is not known: `"error $e"` runs
/// `error a b`, whose message is `a`, where its spelling would give `a b`. A
/// `catch` in a substitution has no barrier ahead of it, so the script word is
/// the only thing between the lattice and a message that was never raised:
/// the result and the code stay unknown under every dialect, and tclsh 8.4 to
/// 9.1 print `1|a`.
#[test]
fn a_catch_script_that_is_not_brace_quoted_text_is_not_the_script_it_runs() {
    let quoted = "proc p {} {\n    set e {a b}\n    set c [catch \"error $e\" m]\n    \
                  puts \"$c|$m\"\n}\np\n";
    for dialect in RELEASE_DIALECTS {
        let unit = unit_of(quoted, dialect);
        for var in ["c", "m"] {
            assert_eq!(
                value_at(&unit, "::p", var, 1),
                Some(LatticeValue::Overdefined),
                "{dialect}: {var}"
            );
        }
    }
    prints_under_every_release(quoted, "1|a\n");
}

/// The scripts of a procedure's `catch` that are straight-line statements —
/// which the flow graph lowers into blocks, ending at a statement that defines
/// the result variable — with what the result variable holds after `catch
/// {script} m` under tclsh 8.4 to 9.1.
const FLATTENED_CATCH_RESULTS: &[(&str, &str)] = &[
    ("set v 1", "1"),
    ("expr {1/0}", "divide by zero"),
    ("set y 5; set z $y", "5"),
    ("string length abc", "3"),
    ("set x 1; incr x", "2"),
    ("set v [string length abc]; set w [expr {$v * 2}]", "6"),
];

/// The number of flattened `catch` regions of `proc`.
fn flattened_catches(unit: &CompilationUnit, proc: &str) -> usize {
    unit.procedures
        .get(&fixture_id(&unit.procedures, proc))
        .expect("the procedure")
        .cfg
        .catch_ends
        .len()
}

/// A `catch` the flow graph flattens into the procedure's blocks has no words
/// where its region ends: the statement there only defines the result and
/// options variables. The graph keeps the `catch` as written beside it, and
/// the solver evaluates it as it does the statement the graph leaves whole,
/// so the result variable holds what the script returned — the last
/// command's result, or the message of the error it stopped at — under every
/// dialect. The lattice holds it, the optimiser forwards it, and tclsh 8.4 to
/// 9.1 print the same before and after.
#[test]
fn a_flattened_catch_gives_its_result_variable_what_its_script_returned() {
    for &(script, expected) in FLATTENED_CATCH_RESULTS {
        let source =
            format!("proc p {{}} {{\n    catch {{{script}}} m\n    puts \"<$m>\"\n}}\np\n");
        for dialect in DIALECTS {
            let unit = unit_of(&source, dialect);
            assert_eq!(flattened_catches(&unit, "::p"), 1, "{dialect}: {source}");
            assert_eq!(
                value_at(&unit, "::p", "m", 1).and_then(lattice_text),
                Some(expected.to_owned()),
                "{dialect}: {source}"
            );
            assert_eq!(
                answers_for(&unit, "::p", "catch"),
                ["evaluated"],
                "{dialect}: {source}"
            );
            let (rewritten, _) = optimised(&source, dialect);
            assert!(
                rewritten.contains(&format!("puts \"<{expected}>\"")),
                "{dialect}:\n{rewritten}"
            );
        }
        prints_under_every_release(&source, &format!("<{expected}>\n"));
    }
}

/// The script of a flattened `catch` runs over the state before its body, not
/// the state where its region ends, where its own writes have already been
/// made: over `x` = 1, `catch {incr x} m` leaves `m` 2 and `x` 2, where the
/// state after the body would give 3. A script that reads what it writes
/// finds it as it was, and a result variable the body also writes is stored
/// last. tclsh 8.4 to 9.1 print what the analysis holds, before and after the
/// optimiser.
#[test]
fn a_flattened_catch_runs_its_script_over_the_state_before_it() {
    let int = |n: i64| Some(LatticeValue::Const(ConstValue::Int(n)));
    let programs: [(&str, &str, i64); 4] = [
        (
            "proc p {} {\n    set x 1\n    catch {incr x} m\n    puts \"$m|$x\"\n}\np\n",
            "2|2\n",
            2,
        ),
        (
            "proc p {} {\n    set x 1\n    catch {incr x; incr x} m\n    puts \"$m|$x\"\n}\np\n",
            "3|3\n",
            3,
        ),
        (
            "proc p {} {\n    set x 1\n    catch {set x [expr {$x + 10}]; set x} m\n    \
             puts \"$m|$x\"\n}\np\n",
            "11|11\n",
            11,
        ),
        (
            "proc p {} {\n    catch {set m 5} m\n    puts $m\n}\np\n",
            "5\n",
            5,
        ),
    ];
    for (source, printed, result) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let unit = unit_of(source, dialect);
            assert_eq!(flattened_catches(&unit, "::p"), 1, "{dialect}: {source}");
            assert_eq!(
                Some(last_value(source, dialect, "::p", "m")),
                int(result),
                "{dialect}: m in\n{source}"
            );
        }
        prints_under_every_release(source, printed);
    }
}

/// What a flattened `catch` cannot prove stays unknown, and the variables it
/// defines stay bound: a script that raises from a word it substitutes, a
/// parameter the procedure was called with, and a result variable a write
/// trace watches. Each program prints under tclsh 8.4 to 9.1 what the
/// original prints, before and after the optimiser.
#[test]
fn a_flattened_catch_the_solver_cannot_prove_stays_unknown() {
    let programs = [
        (
            "proc p {} {\n    catch {puts $undefined} m\n    puts \"<$m>\"\n}\np\n",
            "<can't read \"undefined\": no such variable>\n",
            "8.4",
        ),
        (
            "proc p {x} {\n    catch {incr x} m\n    return \"$m|$x\"\n}\nputs [p 5]\nputs [p abc]\n",
            "6|6\nexpected integer but got \"abc\"|abc\n",
            "8.4",
        ),
        (
            "proc tr {args} {puts T}\nproc p {} {\n    trace add variable m write tr\n    \
             catch {set v 1} m\n    puts $m\n}\np\n",
            "T\n1\n",
            "8.5",
        ),
    ];
    for (source, printed, first) in programs {
        let dialects: &[&str] = if first == "8.4" {
            &DIALECTS
        } else {
            &["tcl8.6", "tcl9.0"]
        };
        for &dialect in dialects {
            let unit = unit_of(source, dialect);
            assert_eq!(
                value_at(&unit, "::p", "m", 1),
                Some(LatticeValue::Overdefined),
                "{dialect}: {source}"
            );
        }
        prints_under_releases_from(source, printed, first);
    }
}

/// `catch` stores the value of its script's last command in the result
/// variable, so a flattened body's last store is observed where the region
/// ends: deleting it as a store nothing reads changes what the result variable
/// holds. Each program prints under tclsh 8.4 to 9.1 what the original prints,
/// before and after the optimiser, with the store still there — where the
/// result is not one the lattice proves, a parameter, a global, a place a
/// `catch` in a branch or an `upvar` alias names — and so does the program
/// that stores to the same name earlier, which the pass leaves alone.
#[test]
fn a_store_the_flattened_catch_returns_is_not_dead() {
    // The statement stays whatever a word of it is folded to, so each program
    // names the store by what leads it.
    let programs = [
        (
            "proc p {c} {\n    catch {set v $c} m\n    puts \"<$m>\"\n}\np 7\n",
            "<7>\n",
            "set v ",
            "8.4",
        ),
        (
            "proc p {c} {\n    catch {set v $c; set w $v} m\n    puts \"<$m>\"\n}\np 7\n",
            "<7>\n",
            "set w ",
            "8.4",
        ),
        (
            "proc p {c} {\n    catch {set v [string length $c]} m o\n    puts \"<$m>\"\n}\np 7\n",
            "<1>\n",
            "set v ",
            "8.5",
        ),
        (
            "set g 0\nproc p {c} {\n    global g\n    catch {set v $c} g\n}\np 7\nputs $g\n",
            "7\n",
            "set v ",
            "8.4",
        ),
        (
            "proc p {c} {\n    if {$c} {catch {set v 1} m} else {set m other}\n    return $m\n}\n\
             puts \"[p 1] [p 0]\"\n",
            "1 other\n",
            "set v ",
            "8.4",
        ),
        (
            "set g 0\nproc p {} {\n    upvar 1 g m\n    catch {set v 1} m\n}\np\nputs $g\n",
            "1\n",
            "set v ",
            "8.4",
        ),
    ];
    for (source, printed, store, first) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(
                !removes_store(source, dialect, store),
                "{dialect}: {store} is the result:\n{source}"
            );
            assert!(
                optimised(source, dialect).0.contains(store),
                "{dialect}: {store} stays:\n{source}"
            );
        }
        prints_under_releases_from(source, printed, first);
    }
}

/// A store in a `catch` body is taken as run only where its place is proved
/// to take it. Tcl raises on a scalar store into an array and on an element
/// store into a scalar, and `catch` observes the error, so where the analysis
/// does not prove a variable's kind the script's completion is not exact and
/// the `catch` is left unevaluated. The kind is read from before the body in
/// every form: at the statement in the value form, before the marker that
/// states what the body may write in the statement form, and at the exit of
/// the block before a flattened body. The route had taken `set y 1` over a `y`
/// that may be an array as run, so `p 1` printed `zero`, and at the top level
/// the marker had made the array `b` either kind, so the program printed
/// `empty`, where tclsh prints `c=1` and `error`. Where the kind is proved, the
/// `catch` is still evaluated in each form, and the body's own earlier store
/// decides the kind a later one needs.
#[test]
fn a_catch_body_store_is_run_only_where_its_place_is_proved_to_take_it() {
    let value_form = "proc p {n} {\n    if {$n} {array set y {a 1}}\n    set c [catch {set y 1}]\n    if {$c == 0} {return zero}\n    return \"c=$c\"\n}\nputs [p 1]\nputs [p 0]\n";
    let flattened = "proc p {n} {\n    if {$n} {array set y {a 1}}\n    catch {set y 1} m\n    if {$m eq \"1\"} {return one}\n    return other\n}\nputs [p 1]\nputs [p 0]\n";
    let top_level = "set a old\narray set b {k keep}\ncatch {lassign {new second} a b} m2\nif {$m2 eq \"\"} {puts empty} else {puts error}\nputs $a\n";
    for dialect in DIALECTS {
        assert_ne!(
            last_value(value_form, dialect, "::p", "c"),
            LatticeValue::Const(ConstValue::Int(0)),
            "{dialect}: `y` may be an array"
        );
        let held = last_value(flattened, dialect, "::p", "m");
        assert!(
            !matches!(held, LatticeValue::Const(_)),
            "{dialect}: `y` may be an array: {held:?}"
        );
        assert_ne!(
            top_value_at(&unit_of(top_level, dialect), "m2", 1),
            Some(text("")),
            "{dialect}: `b` is an array"
        );
    }
    prints_under_every_release(value_form, "c=1\nzero\n");
    prints_under_every_release(flattened, "other\none\n");
    prints_under_releases_from(top_level, "error\nnew\n", "8.5");

    // The value form: the code each body completes with, where the kinds
    // its stores need are proved (`Some`) or not (`None`), and what tclsh
    // prints for it. An element store needs an array or nothing, which an
    // earlier element store of the body proves; a scalar store after one
    // raises.
    let value_forms = [
        ("set y 1", Some(0), "0"),
        ("set y(a) 1; set y(b) 2", Some(0), "0"),
        ("set y(a) 1; set y 2", None, "1"),
    ];
    for (body, code, printed) in value_forms {
        let source = format!(
            "proc p {{}} {{\n    set c [catch {{{body}}} m]\n    return $c\n}}\nputs [p]\n"
        );
        for dialect in DIALECTS {
            let held = value_at(&unit_of(&source, dialect), "::p", "c", 1);
            match code {
                Some(code) => assert_eq!(
                    held,
                    Some(LatticeValue::Const(ConstValue::Int(code))),
                    "{dialect}: {body}"
                ),
                None => assert!(
                    !matches!(held, Some(LatticeValue::Const(_))),
                    "{dialect}: {body}: {held:?}"
                ),
            }
        }
        prints_under_every_release(&source, &format!("{printed}\n"));
    }
    let into_an_array = "proc p {} {\n    array set y {k v}\n    set c [catch {set y(a) 1} m]\n    return $c\n}\nputs [p]\n";
    let flattened_proved = "proc p {} {\n    catch {set v 1} r\n    return $r\n}\nputs [p]\n";
    let top_proved = "set w 0\ncatch {set v 1; set w 2} r\nif {[info exists v]} {puts yes} else {puts no}\nputs \"$r $w\"\n";
    for dialect in DIALECTS {
        assert_eq!(
            value_at(&unit_of(into_an_array, dialect), "::p", "c", 1),
            Some(LatticeValue::Const(ConstValue::Int(0))),
            "{dialect}"
        );
        assert_eq!(
            last_value(flattened_proved, dialect, "::p", "r"),
            LatticeValue::Const(ConstValue::Int(1)),
            "{dialect}"
        );
        assert_eq!(
            top_value_at(&unit_of(top_proved, dialect), "r", 1),
            Some(LatticeValue::Const(ConstValue::Int(2))),
            "{dialect}"
        );
    }
    prints_under_every_release(into_an_array, "0\n");
    prints_under_every_release(flattened_proved, "1\n");
    prints_under_every_release(top_proved, "yes\n2 2\n");

    // The body's own earlier store decides the kind a later one needs: a
    // scalar then an element store raises, and an element then a scalar one.
    let scalar_then_element =
        "proc p {} {\n    catch {set y 1; set y(a) 2} m\n    return \"$m $y\"\n}\nputs [p]\n";
    let element_then_scalar = "catch {set y(a) 1; set y 2} m\nputs \"$m [array exists y]\"\n";
    for dialect in DIALECTS {
        let held = last_value(scalar_then_element, dialect, "::p", "m");
        assert!(
            !matches!(held, LatticeValue::Const(_)),
            "{dialect}: `y` is a scalar: {held:?}"
        );
        let held = top_value_at(&unit_of(element_then_scalar, dialect), "m", 1);
        assert!(
            !matches!(held, Some(LatticeValue::Const(_))),
            "{dialect}: `y` is an array: {held:?}"
        );
    }
    prints_under_every_release(
        scalar_then_element,
        "can't set \"y(a)\": variable isn't array 1\n",
    );
    prints_under_every_release(
        element_then_scalar,
        "can't set \"y\": variable is array 1\n",
    );
}

/// The script an opaque `catch` keeps inside itself reads a name it writes
/// as its own, by the rule an opaque `switch`'s arms are read by: its reads
/// less its writes are what it reads of the frame. A body that sets a name
/// and then reads it — a `set`, a `foreach` binder, a `for` counter in a
/// procedure — draws no W210 under every dialect, where the marker ahead of
/// the call had made each read one of a name that may be unset; tclsh 8.4
/// to 9.1 print what the bodies print, before and after `tcl opt`. A name the
/// body only may set is still undefined after the call on the path where it
/// did not: `puts $x` past `catch {if {$c} {set x 1}}` draws W210, and tclsh
/// raises on `p 0`.
#[test]
fn an_opaque_catch_reads_its_own_writes_as_its_own() {
    for (source, printed) in [
        ("catch {set i 0; puts $i}\n", "0\n"),
        ("catch {foreach x {1 2} {puts $x}}\n", "1\n2\n"),
        (
            "proc p {} {catch {for {set i 0} {$i < 3} {incr i} {puts $i}}}\np\n",
            "0\n1\n2\n",
        ),
    ] {
        for dialect in TCL_CORE_DIALECTS {
            assert!(
                !reports(source, dialect, DiagCode::W210),
                "{dialect}: {source}"
            );
        }
        prints_under_every_release(source, printed);
    }
    let may_set = "proc p {c} {catch {if {$c} {set x 1}}; puts $x}\np 1\np 0\n";
    for dialect in TCL_CORE_DIALECTS {
        assert!(reports(may_set, dialect, DiagCode::W210), "{dialect}");
    }
    for (series, tclsh) in releases_on_path() {
        let (rewritten, _) = optimised(may_set, &dialect_of(series));
        for program in [may_set, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((false, "1\n".to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}
