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

//! The existence rung against `tclsh`: the absent-cell release table, bound and maybe-bound places, and stores an existence read keeps.

use super::*;

/// What `tclsh` reports of `places` after `body` runs in a procedure:
/// `None` when the body raises, else each place's `info exists` and
/// `array exists`.
fn oracle_existence(tclsh: &str, body: &str, places: &[&str]) -> Option<Vec<(bool, bool)>> {
    let mut probes = String::new();
    for place in places {
        probes.push_str(" [info exists ");
        probes.push_str(place);
        probes.push_str("] [array exists ");
        probes.push_str(place);
        probes.push(']');
    }
    let script = format!(
        "proc p {{}} {{\n{body}\nreturn [list{probes}]\n}}\n\
         if {{[catch p answer]}} {{puts -nonewline error}} else {{puts -nonewline $answer}}\n"
    );
    let (ok, output) = run_script(tclsh, &script)?;
    assert!(ok, "{tclsh} ran the probe for {body:?}");
    if output == "error" {
        return None;
    }
    let bits: Vec<bool> = output.split_whitespace().map(|bit| bit == "1").collect();
    Some(bits.chunks(2).map(|pair| (pair[0], pair[1])).collect())
}

/// Whether `fact` agrees with what `tclsh` reports of a place: a proven
/// binding exists, as an array exactly when it is one, and a proven
/// absence does not; a fact that proves neither claims nothing.
fn existence_agrees(fact: Option<Existence>, (exists, array): (bool, bool)) -> bool {
    match fact {
        Some(Existence::Unbound) => !exists && !array,
        Some(Existence::Bound(BindingKind::Scalar)) => exists && !array,
        Some(Existence::Bound(BindingKind::Array)) => exists && array,
        Some(Existence::Bound(BindingKind::Either)) => exists,
        Some(Existence::MayBound | Existence::Pending) | None => true,
    }
}

/// The release table's lines every release reads alike: each place's fact
/// after the line, under every dialect, and against every `tclsh` on
/// `PATH` wherever the line completes.
fn the_lines_every_release_reads_alike(releases: &[(&'static str, String)]) {
    type Row = (&'static str, &'static [(&'static str, Option<Existence>)]);
    let alike: [Row; 9] = [
        ("append s foo", &[("s", SCALAR)]),
        ("lappend l foo", &[("l", SCALAR)]),
        ("regexp {(x)(y)} zz a b", &[("a", UNBOUND), ("b", UNBOUND)]),
        (
            "scan {12 nope} {%d %d} a b",
            &[("a", SCALAR), ("b", UNBOUND)],
        ),
        ("unset nosuch", &[("nosuch", UNBOUND)]),
        ("unset -nocomplain nosuch", &[("nosuch", UNBOUND)]),
        (
            "set arr(k) 1; unset arr(k)",
            &[("arr", ARRAY), ("arr(k)", UNBOUND)],
        ),
        ("set x 1; unset x; set x 2", &[("x", SCALAR)]),
        ("foreach x {} {}", &[("x", UNBOUND)]),
    ];
    for (body, places) in alike {
        let source = format!("proc p {{}} {{{body}}}\n");
        for dialect in RELEASE_DIALECTS {
            let unit = unit_of(&source, dialect);
            for &(place, expected) in places {
                assert_eq!(
                    last_existence(&unit, "::p", place),
                    expected,
                    "{dialect}: `{place}` after `{body}`"
                );
            }
        }
        let names: Vec<&str> = places.iter().map(|(place, _)| *place).collect();
        for (series, tclsh) in releases {
            let Some(answers) = oracle_existence(tclsh, body, &names) else {
                continue;
            };
            let unit = unit_of(&source, &dialect_of(series));
            for (&(place, _), answer) in places.iter().zip(answers) {
                assert!(
                    existence_agrees(last_existence(&unit, "::p", place), answer),
                    "tclsh{series}: `{place}` after `{body}` is {answer:?}"
                );
            }
        }
    }
    for dialect in RELEASE_DIALECTS {
        assert_eq!(
            last_value("proc p {} {append s foo}\n", dialect, "::p", "s"),
            text("foo"),
            "{dialect}: `append` creates its cell in every release"
        );
        assert_eq!(
            last_value("proc p {} {lappend l foo}\n", dialect, "::p", "l"),
            text("foo"),
            "{dialect}: `lappend` creates its cell in every release"
        );
    }
}

/// The release table's `incr` lines: the cell is created from 8.5, with
/// the amount as its value; under 8.4, where `tclsh8.4` raises, the route
/// answers the error after no store, and under the spanning profile, which
/// cannot say which release runs, the value declines as an unbound place.
fn the_increment_split(releases: &[(&'static str, String)]) {
    let increments: [(&str, &str, i64); 3] = [
        ("incr fresh", "fresh", 1),
        ("incr fresh 2", "fresh", 2),
        ("incr arr(k)", "arr(k)", 1),
    ];
    for (body, place, amount) in increments {
        let source = format!("proc p {{}} {{{body}}}\n");
        for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let unit = unit_of(&source, dialect);
            assert_eq!(
                last_value(&source, dialect, "::p", place),
                LatticeValue::Const(ConstValue::Int(amount)),
                "{dialect}: `{body}` creates its cell"
            );
            assert_eq!(last_existence(&unit, "::p", place), SCALAR, "{dialect}");
            if place == "arr(k)" {
                assert_eq!(last_existence(&unit, "::p", "arr"), ARRAY, "{dialect}");
            }
        }
        for (dialect, answer) in [
            ("tcl8.4", "evaluated: error after 0 stores"),
            ("tcl", "declined: unbound-place"),
        ] {
            let unit = unit_of(&source, dialect);
            assert_eq!(
                last_value(&source, dialect, "::p", place),
                LatticeValue::Overdefined,
                "{dialect}: `{body}` raises under 8.4, so its value declines"
            );
            let answers = answers_for(&unit, "::p", "incr");
            assert!(
                answers.iter().any(|found| found == answer),
                "{dialect}: `{body}` answers {answer}: {answers:?}"
            );
        }
        for (series, tclsh) in releases {
            let answer = oracle_existence(tclsh, body, &[place]);
            if *series == "8.4" {
                assert_eq!(answer, None, "tclsh8.4 raises on `{body}`");
            } else {
                assert_eq!(answer, Some(vec![(true, false)]), "tclsh{series}: `{body}`");
            }
        }
    }
}

/// The page's release table for an absent cell
/// (`docs/design/compiler/value-transfers.md` § *Existence*), every line but
/// the `unset p nosuch q` prefix line, whose stores an error leaves behind
/// are not modelled: each place's
/// existence after the line, and the value a cell update leaves, under
/// each release's dialect and the `tcl` profile that spans them all. An
/// `incr` of an absent place binds from 8.5 and declines under 8.4 and
/// under the spanning profile; `append` and `lappend` bind in every
/// release; a no-match and an exhausted `scan` leave their places as they
/// were; an unbind leaves the place unbound, and an unset element its array
/// bound. Every fact is checked against `tclsh` wherever the line completes.
#[test]
fn the_absent_cell_release_table() {
    let releases = releases_on_path();
    the_lines_every_release_reads_alike(&releases);
    the_increment_split(&releases);
}

/// O109 keeps a store an existence read observes (#2132): while the
/// read stands, no pass deletes the store behind it — the item's two
/// programs keep `set x 1` and `incr n` behind `[info exists …]`, and an
/// existence read in a bare statement, a `catch` body, a value word, a
/// `return`, an `expr` word and `array exists` keeps its store, as does an
/// unbind the store feeds: a nested `[unset x]` in an argument, a value
/// word, a condition or a `return` (it raises on an absent `x`), and an
/// `array unset` of a scalar, which leaves the scalar bound. An unbind
/// statement is never removed. Every program the multipass optimiser
/// rewrites, folding a decided read and then the store it no longer
/// needs, prints what the original does under every release on `PATH` —
/// the `incr` program from 8.5, where the original creates `n`; 8.4
/// raises there.
#[test]
fn o109_keeps_a_store_an_existence_read_observes() {
    let set_first = "proc p {} {set x 1; if {[info exists x]} {puts yes}}\np\n";
    let incr_first = "proc p {} {incr n; if {[info exists n]} {puts yes}}\np\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
        assert!(!removes_store(set_first, dialect, "set x 1"), "{dialect}");
    }
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert!(!removes_store(incr_first, dialect, "incr n"), "{dialect}");
    }
    prints_under_every_release(set_first, "yes\n");
    for (series, tclsh) in releases_on_path()
        .into_iter()
        .filter(|(series, _)| *series != "8.4")
    {
        let (rewritten, _) = optimised(incr_first, &dialect_of(series));
        for program in [incr_first, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, "yes\n".to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
    let positions = "proc p1 {} {set x 1; info exists x}\n\
         proc p2 {} {set x 1; catch {info exists x} r; return $r}\n\
         proc p3 {} {set x 1; lappend l [info exists x]; return $l}\n\
         proc p4 {} {set x 1; return [info exists x]}\n\
         proc p5 {} {set x(a) 1; array exists x}\n\
         proc p6 {} {set x 1; set y [expr {[info exists x] ? \"yes\" : \"no\"}]; return $y}\n\
         proc p7 {} {set x 1; unset x; info exists x}\n\
         proc p8 {c} {set x 1; if {$c} {unset x}; info exists x}\n\
         proc p9 {} {set x 1; lappend l [unset x]; info exists x}\n\
         proc p10 {} {set x 1; set y [unset x]; info exists x}\n\
         proc p11 {} {set x 1; if {[unset x] eq \"\"} {info exists x}}\n\
         proc p12 {} {set x 1; return [unset x]}\n\
         proc p13 {} {set x 1; array unset x; return $x}\n\
         proc p14 {} {set x 1; lappend l [array unset x]; info exists x}\n\
         puts [list [p1] [p2] [p3] [p4] [p5] [p6] [p7] [p8 0] [p8 1] \
         [p9] [p10] [p11] [p12] [p13] [p14]]\n";
    let unbinds = positions.matches("unset x").count();
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        for line in positions.lines().filter(|line| line.starts_with("proc ")) {
            let store = if line.contains("set x(a) 1") {
                "set x(a) 1"
            } else {
                "set x 1"
            };
            let one = format!("{line}\n");
            assert!(!removes_store(&one, dialect, store), "{dialect}: {one}");
        }
        let (rewritten, _) = optimised(positions, dialect);
        assert_eq!(
            rewritten.matches("unset x").count(),
            unbinds,
            "{dialect}: an unbind is never removed:\n{rewritten}"
        );
    }
    prints_under_every_release(positions, "1 1 1 1 1 yes 0 1 0 0 0 0 {} 1 1\n");
}

/// The entry rule for a parameter and a never-assigned local (the
/// Existence row): a parameter enters `Bound(Scalar)`, so `[info exists
/// a]` decides true, and a local nothing ever assigns enters `Unbound`, so
/// `[info exists b]` decides false — both inside the fixed point, so
/// neither dead arm survives the optimiser.
#[test]
fn a_parameter_binds_and_a_never_assigned_local_stays_unbound() {
    let source = "proc p {a} {\n    if {[info exists a]} {puts yes} else {puts no}\n    \
                  if {[info exists b]} {puts wrong} else {puts right}\n}\np 1\n";
    for dialect in TCL_CORE_DIALECTS {
        let unit = unit_of(source, dialect);
        assert_eq!(
            last_existence(&unit, "::p", "a"),
            SCALAR,
            "{dialect}: a parameter binds at entry"
        );
        assert_eq!(
            last_existence(&unit, "::p", "b"),
            UNBOUND,
            "{dialect}: a never-assigned local is unbound at entry"
        );
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            !rewritten.contains("puts no") && !rewritten.contains("puts wrong"),
            "{dialect}: both guards decide inside the fixed point:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "yes\nright\n");
}

/// `unset -nocomplain` never raises, whether the place was bound or
/// always absent — its completion domain has no error, unlike a plain
/// `unset` of an absent name (the release table's own `unset -nocomplain
/// nosuch` line, read here through the whole program rather than the
/// existence probe alone): a second, redundant `-nocomplain` and one over
/// a name never set both leave silently.
#[test]
fn unset_nocomplain_never_raises_bound_or_not() {
    let source = "proc p {} {\n    set x 1\n    unset -nocomplain x\n    \
                  unset -nocomplain x\n    unset -nocomplain never\n    \
                  puts done\n    puts [info exists x]\n}\np\n";
    for dialect in RELEASE_DIALECTS {
        let unit = unit_of(source, dialect);
        assert_eq!(
            last_existence(&unit, "::p", "x"),
            UNBOUND,
            "{dialect}: a repeated -nocomplain leaves the place unbound"
        );
    }
    prints_under_every_release(source, "done\n0\n");
}

/// A scope-alias local enters `MayBound`, never provably `Unbound`,
/// however little the visible source writes the linked global: the alias
/// tracks a cell other, unanalysed code may already have set, so the
/// guard on it never decides and both arms of `[info exists g]` survive
/// the optimiser — even though this program's own `::g` is never set
/// anywhere, so tclsh always takes the "no" arm.
#[test]
fn a_scope_alias_enters_maybound() {
    let source =
        "proc p {} {\n    global g\n    if {[info exists g]} {puts yes} else {puts no}\n}\np\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        assert_eq!(
            last_existence(&unit, "::p", "g"),
            Some(Existence::MayBound),
            "{dialect}: a scope alias is MayBound, not provably absent"
        );
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("puts yes") && rewritten.contains("puts no"),
            "{dialect}: a MayBound guard never folds away either arm:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "no\n");
}

/// A cross-event iRules variable enters `MayBound` (a superset of
/// `ConnectionScope::cross_event_defs`): `y` is bound in `CLIENT_ACCEPTED`
/// and read at `HTTP_REQUEST`'s own entry, so the guard there never
/// decides either — no oracle here, since `when` is not a command a plain
/// `tclsh` runs.
#[test]
fn a_cross_event_variable_enters_maybound() {
    let source = "when CLIENT_ACCEPTED {\n    set y 1\n}\n\
                  when HTTP_REQUEST {\n    if {[info exists y]} {puts yes} else {puts no}\n}\n";
    let unit = unit_of(source, "f5-irules");
    assert_eq!(
        last_existence(&unit, "::when::HTTP_REQUEST", "y"),
        Some(Existence::MayBound),
        "a cross-event variable is MayBound at another handler's entry"
    );
    let (rewritten, rewrites) = optimised(source, "f5-irules");
    assert!(
        rewritten.contains("puts yes") && rewritten.contains("puts no"),
        "a MayBound guard never folds away either arm:\n{rewritten}\n{rewrites:#?}"
    );
}

/// O130 folds a chain starting at an absent cell (the O104 / O130 row):
/// the release rule creates `l` in every release, so `lappend l a;
/// lappend l b` folds through the value at the last write exactly as a
/// `set`-anchored chain does. The single O130 rewrite is read here
/// (`rewrites_of`, matching `chain_fold.rs`'s own unit-test shape) since
/// the multipass optimiser goes on to propagate `l`'s now-known value
/// into `return $l` and drop the fold as unused in its turn — a further,
/// sound reduction the exit line's own program does not name.
#[test]
fn o130_folds_a_chain_from_an_absent_cell() {
    let source = "proc p {} {\n    lappend l a\n    lappend l b\n    return $l\n}\nputs [p]\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
        let rewrites = rewrites_of(source, dialect);
        let fold = rewrites
            .iter()
            .find(|o| o.code == DiagCode::O130 && o.replacement.starts_with("set"))
            .unwrap_or_else(|| panic!("{dialect}: expected an O130 fold: {rewrites:#?}"));
        assert_eq!(fold.replacement, "set l {a b}", "{dialect}");
    }
    prints_under_every_release(source, "a b\n");
}

/// A failing dead write is retained (O108's totality proof, the Rewrites
/// row's "failing dead write retained"): `lappend l c` over the malformed
/// list `"a {b"` raises `unmatched open brace in list` in every release,
/// so deleting it — `l` is never read afterwards — would turn a raising
/// program into a silent one.
#[test]
fn a_failing_dead_lappend_is_retained() {
    let source = "proc p {} {\n    set l \"a {b\"\n    lappend l c\n}\np\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
        assert!(!removes_store(source, dialect, "lappend l c"), "{dialect}");
    }
    for (series, tclsh) in releases_on_path() {
        assert_eq!(
            run_script(&tclsh, source).map(|(ok, _)| ok),
            Some(false),
            "tclsh{series}: a malformed list raises on `lappend`"
        );
    }
}

/// A failing dead `incr` is retained under 8.4 (O108's totality proof,
/// permission 3): `incr n` on the never-bound, never-read `n` raises
/// `can't read "n": no such variable` under 8.4, so removing it would turn
/// a raising program into a silent one there; a profile whose every
/// release creates the cell (8.5 onwards) still removes it.
#[test]
fn a_failing_dead_incr_is_retained_under_84() {
    let source = "proc p {} {\n    incr n\n    return\n}\np\n";
    for dialect in ["tcl8.4", "tcl", "f5-irules"] {
        assert!(!removes_store(source, dialect, "incr n"), "{dialect}");
    }
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert!(removes_store(source, dialect, "incr n"), "{dialect}");
    }
    for (series, tclsh) in releases_on_path() {
        let ok = run_script(&tclsh, source).map(|(ok, _)| ok);
        if series == "8.4" {
            assert_eq!(
                ok,
                Some(false),
                "tclsh8.4: `incr n` raises on an absent place"
            );
        } else {
            assert_eq!(ok, Some(true), "tclsh{series}: `incr n` creates the cell");
        }
    }
}

/// An externally mutable place is never refined: a call the
/// module cannot see — here a computed head —
/// sets or unsets a global between the guard and the inner query, with no
/// barrier in between, so the guard's refinement decides nothing about the
/// inner `info exists`. tclsh 8.4 to 9.1 print `yes yes gone`; a refinement
/// would fold the inner conditions to `no no still`, with three false I230s.
/// The source and its optimised form are both run on every release.
#[test]
fn an_unseen_call_ends_no_refinement_because_none_is_made() {
    let source = "\
proc init {} { set ::x 1 }
proc cleanup {} { unset ::x }
set handlers {init cleanup}
proc p {} {
    if {![info exists ::x]} {
        [lindex $::handlers 0]
        if {[info exists ::x]} { puts yes } else { puts no }
    }
}
proc q {} {
    global x
    if {![info exists x]} {
        [lindex $::handlers 0]
        if {[info exists x]} { puts yes } else { puts no }
    }
}
proc r {} {
    set ::x 1
    if {[info exists ::x]} {
        [lindex $::handlers 1]
        if {[info exists ::x]} { puts still } else { puts gone }
    }
}
p; unset ::x; q; r
";
    for dialect in DIALECTS {
        assert!(
            !reports(source, dialect, DiagCode::I230),
            "{dialect}: no inner query is decided"
        );
        // The arms tclsh takes are never folded away. With the command
        // world closed over a `global` link the walk resolves the computed
        // head to `init` or `cleanup` and runs it, so the arms tclsh never
        // takes may go; what the rewrite prints is checked below.
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("puts yes") && rewritten.contains("puts gone"),
            "{dialect}: a taken arm is folded away:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "yes\nyes\ngone\n");
    prints_under_every_release(source, "yes\nyes\ngone\n");
}

/// A call nothing can resolve ends every refinement of the caller's frame:
/// a head computed from a value the analysis cannot know (`clock seconds`
/// is mapped out of `cleanup`, so `tclsh` always runs `cleanup`), and a
/// name a `rename` with such a source binds. The procedures are defined
/// before that `rename`, which might otherwise have renamed `proc` itself;
/// it may still have renamed `set`, so there `x` need not be tracked at
/// all, which claims nothing. The handler unsets the
/// caller's `x`, so `x` is not proved bound after the call, no condition
/// is decided and both arms stay. tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and
/// 9.1.0 print `gone` for both programs.
#[test]
fn an_unresolvable_head_ends_every_refinement() {
    let computed = "\
proc cleanup {} { uplevel 1 {unset x} }
proc p {} {
    set x 1
    [string map [list [clock seconds] {}] cleanup]
    if {[info exists x]} { puts still } else { puts gone }
}
p
";
    let renamed = "\
proc cleanup {} { uplevel 1 {unset x} }
proc p {} {
    set x 1
    step
    if {[info exists x]} { puts still } else { puts gone }
}
rename [string map [list [clock seconds] {}] cleanup] step
p
";
    for source in [computed, renamed] {
        for dialect in RELEASE_DIALECTS {
            let unit = unit_of(source, dialect);
            let function = &unit.procedures[&fixture_id(&unit.procedures, "::p")];
            let fact = function
                .ssa
                .var_symbol("x")
                .and_then(|_| last_existence(&unit, "::p", "x"));
            assert!(
                existence_agrees(fact, (false, false)),
                "{dialect}: `x` is proved bound after the call:\n{source}"
            );
            assert!(
                condition_claims(source, dialect).is_empty(),
                "{dialect}: a condition is decided:\n{source}"
            );
            let (rewritten, rewrites) = optimised(source, dialect);
            assert!(
                rewritten.contains("puts still") && rewritten.contains("puts gone"),
                "{dialect}: an arm is folded away:\n{rewritten}\n{rewrites:#?}"
            );
        }
        prints_under_every_release(source, "gone\n");
    }
}

/// The absent-start chain anchor reads the fact at the statement: after a
/// non-lowered `switch` whose arm may bind
/// `l`, the per-version fact of `l`'s version 0 is still `Unbound`, but
/// the fact at `lappend l a` is `MayBound` — the arm's clobber — so no
/// chain anchors there. tclsh 8.4 to 9.1 print `z a b` and `a b`, before
/// and after the optimiser.
#[test]
fn an_absent_start_anchor_reads_the_fact_at_the_statement() {
    let source = "proc p {c} { switch -glob -- $c { a* { set l z } }; lappend l a; lappend l b; puts $l }\np abc\np q\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
        let rewrites = rewrites_of(source, dialect);
        assert!(
            !rewrites.iter().any(|o| o.code == DiagCode::O130),
            "{dialect}: {rewrites:#?}"
        );
    }
    prints_under_every_release(source, "z a b\na b\n");
}

/// A failing dead `incr` on a may-bound place is retained under 8.4:
/// `incr n` after `if {$c} {set n 1}` raises `can't
/// read "n"` under 8.4 when `c` is false, so removing it would silence a
/// raising program there; a profile whose every release creates the cell
/// still removes it. tclsh 8.4 prints `1` (the call raised) and 8.5 to 9.1
/// print `0`.
#[test]
fn a_failing_dead_incr_on_a_maybound_place_is_retained_under_84() {
    let source =
        "proc p {c} {\n    if {$c} {set n 1}\n    incr n\n    return\n}\nputs [catch {p 0} msg]\n";
    for dialect in ["tcl8.4", "tcl", "f5-irules"] {
        assert!(!removes_store(source, dialect, "incr n"), "{dialect}");
    }
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert!(removes_store(source, dialect, "incr n"), "{dialect}");
    }
    for (series, tclsh) in releases_on_path() {
        let expected = if series == "8.4" { "1\n" } else { "0\n" };
        let (rewritten, _) = optimised(source, &dialect_of(series));
        for program in [source, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, expected.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// A script body nested in a substitution clobbers what it may unset
/// (#2231's consequence): `[catch {unset x}]` in a
/// condition leaves `x` may-bound, so the later `[info exists x]` decides
/// nothing — a fold would give `1` with an I230. tclsh 8.4 to 9.1 print
/// `no`, before and after the optimiser.
#[test]
fn a_substituted_body_clobbers_what_it_unsets() {
    let source = "set x 1\nif {[catch {unset x}]} { puts err }\nif {[info exists x]} {puts yes} else {puts no}\n";
    for dialect in DIALECTS {
        assert!(!reports(source, dialect, DiagCode::I230), "{dialect}");
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("puts no"),
            "{dialect}: the else arm stays:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "no\n");
}
