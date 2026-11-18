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

//! Procedure summaries and argument-sensitive re-runs: computed returns, shared summaries, raises, default and element arguments.

use super::*;

/// O103's argument-independent fold replaces a call by the procedure's
/// constant return only where the call is one the argument-sensitive re-run
/// could make: every word after the head literal, and as many as the
/// parameters accept. A word that substitutes runs before the call and may
/// write or raise, and a count the parameters do not accept raises, and the
/// constant says neither: `[p [incr n]]` had become `foo` and lost the
/// `incr`, and `[p $undefined]` and `[p]` had become `foo` where tclsh raises
/// (#2389). A call with a literal word still folds, and a bare call draws the
/// hint only where it is such a call. Each program prints what tclsh 8.4 to
/// 9.1 print, before and after `tcl opt`.
#[test]
fn the_summary_folds_only_a_call_the_rerun_could_make() {
    let kept = [
        (
            "proc p {a} {return foo}\nset n 0\nputs [p [incr n]]\nputs $n\n",
            "foo\n1\n",
            "[p [incr n]]",
        ),
        (
            "proc p {a} {return foo}\nproc q {} {puts [p $undefined]}\nputs [catch q]\n",
            "1\n",
            "[p $undefined]",
        ),
        (
            "proc p {a} {return foo}\nproc q {} {puts [p]}\nputs [catch q]\n",
            "1\n",
            "[p]",
        ),
    ];
    for (source, printed, call) in kept {
        for dialect in DIALECTS {
            let (rewritten, _) = optimised(source, dialect);
            assert!(
                rewritten.contains(call),
                "{dialect}: {call} stays\n{rewritten}"
            );
        }
        prints_under_every_release(source, printed);
    }
    let folded = "proc p {a} {return foo}\nputs [p x]\n";
    let bare = "proc p {a} {return foo}\np x\np [incr n]\n";
    for dialect in DIALECTS {
        let (rewritten, _) = optimised(folded, dialect);
        assert!(rewritten.contains("puts foo"), "{dialect}\n{rewritten}");
        let hints: Vec<&str> = rewrites_of(bare, dialect)
            .iter()
            .filter(|rewrite| rewrite.code == DiagCode::O103)
            .map(|rewrite| &bare[rewrite.span.start() as usize..rewrite.span.end() as usize])
            .collect();
        assert_eq!(hints, ["p x"], "{dialect}");
    }
    prints_under_every_release(folded, "foo\n");
}

/// A `return` run inside a statement the flow graph keeps whole leaves the
/// procedure from that statement, where no exit block stands for it: an
/// opaque `switch`'s arm. O103's re-run read the other exits alone and folded
/// `[p]`, `[p abc]` and `[p q]` to `2`, where tclsh 8.4 to 9.1 print `1`
/// (#2393). No call folds to `2`, and each program prints what tclsh prints,
/// before and after `tcl opt`.
#[test]
fn a_return_inside_a_statement_kept_whole_stops_the_fold() {
    let programs = [
        (
            "proc p {} {switch -glob -- [clock seconds] {* {return 1}}; return 2}\nputs [p]\n",
            "1\n",
        ),
        (
            "proc p {x} {switch -glob -- $x {* {return 1}}; return 2}\nputs [p abc]\nputs [p q]\n",
            "1\n1\n",
        ),
    ];
    for (source, printed) in programs {
        for dialect in DIALECTS {
            let folded_to_two: Vec<Optimisation> = rewrites_of(source, dialect)
                .into_iter()
                .filter(|rewrite| rewrite.code == DiagCode::O103 && rewrite.replacement == "2")
                .collect();
            assert!(
                folded_to_two.is_empty(),
                "{dialect}: {folded_to_two:?}\n{source}"
            );
        }
        prints_under_every_release(source, printed);
    }
}

/// A loop over a qualified variable is a statement the flow graph keeps
/// whole too, as a call holding its body word, and a `return` that body runs
/// leaves the procedure from it. The reading asked only the statement
/// variants a `return` could sit in, so both O103 paths folded `[p]`, `[q 5]`
/// and `[r]` to `2`, `2` and `4`, where tclsh prints 1, 5 and 3 (#2393's
/// rule missed the call). No call folds, and each program prints what tclsh
/// 8.4 to 9.1 print, before and after `tcl opt`, the `lmap` one from 8.6.
#[test]
fn a_return_inside_a_call_holding_a_body_stops_the_fold() {
    let programs = [
        (
            "proc p {} {foreach ::x {1} {return 1}; return 2}\nputs [p]\n",
            "1\n",
            "8.4",
        ),
        (
            "proc q {a} {foreach ::y [list $a] {return $a}; return 2}\nputs [q 5]\n",
            "5\n",
            "8.4",
        ),
        (
            "proc r {} {lmap ::z {1} {return 3}; return 4}\nputs [r]\n",
            "3\n",
            "8.6",
        ),
    ];
    for (source, printed, first) in programs {
        for dialect in DIALECTS {
            let folds: Vec<Optimisation> = rewrites_of(source, dialect)
                .into_iter()
                .filter(|rewrite| rewrite.code == DiagCode::O103)
                .collect();
            assert!(folds.is_empty(), "{dialect}: {folds:?}\n{source}");
        }
        prints_under_releases_from(source, printed, first);
    }
}

/// The argument-independent O103 folds a procedure
/// whose return is a computed constant. `p`'s return is no literal, but its
/// seedless lattice — the procedure run with its parameters unknown — holds
/// `x` at `foo` where it returns, so the summary says `p` returns `foo` for
/// every caller. The summary path is asked before the re-run and answers on
/// its own: without `p`'s unit, which the re-run needs, `[p]` still folds to
/// `foo`, and the bare call `p` draws the summary path's hint. A return that
/// reads a parameter stays `UsesParam` with no constant; and `r`'s value,
/// exact only under the literal its one caller passes, which the unit's
/// seeded lattice holds, never enters the summary (R7). tclsh 8.4 to 9.1
/// print what each program prints, before and after `tcl opt`.
#[test]
fn o103_summary_path_folds_a_computed_return() {
    let source = "proc p {} {set x [string range foobar 0 2]; return $x}\nputs [p]\n";
    let bare = "proc p {} {set x [string range foobar 0 2]; return $x}\np\n";
    let uses = "proc q {a} {return [expr {$a + 1}]}\nputs [q 5]\n";
    let seeded = "proc r {a} {set x [string range $a 0 2]; return $x}\nputs [r foobar]\n";
    for dialect in DIALECTS {
        let summary = summary_of(source, dialect, "::p");
        assert!(summary.can_fold_static_calls, "{dialect}: {summary:?}");
        assert_eq!(
            summary.constant_return,
            Some(ConstantReturn::Str("foo".to_owned())),
            "{dialect}"
        );

        let registry = static_context_for(dialect).commands();
        let profile = resolve_environment(dialect).analyser_profile();
        let mut unit = summarised_unit(source, dialect);
        let p_id = fixture_id(&unit.ir_module.procedures, "::p");
        unit.procedures.remove(&p_id);
        let folds = optimise_unit(&unit, registry, Some(profile));
        assert!(
            folds.iter().any(|fold| fold.code == DiagCode::O103
                && !fold.hint_only
                && fold.replacement == "foo"),
            "{dialect}: {folds:?}"
        );
        assert!(
            rewrites_of(bare, dialect)
                .iter()
                .any(|fold| fold.code == DiagCode::O103
                    && fold.hint_only
                    && fold.replacement == "foo"),
            "{dialect}"
        );
        let (rewritten, _) = optimised(source, dialect);
        assert!(rewritten.contains("puts foo"), "{dialect}\n{rewritten}");

        let summary = summary_of(uses, dialect, "::q");
        assert!(!summary.returns_constant, "{dialect}: {summary:?}");
        assert_eq!(summary.return_depends_on_params, ["a"], "{dialect}");

        assert_eq!(
            last_value(seeded, dialect, "::r", "x"),
            text("foo"),
            "{dialect}"
        );
        let summary = summary_of(seeded, dialect, "::r");
        assert!(
            !summary.returns_constant && summary.constant_return.is_none(),
            "{dialect}: {summary:?}"
        );
    }
    prints_under_every_release(source, "foo\n");
    prints_under_every_release(uses, "6\n");
    prints_under_every_release(seeded, "foo\n");
}

/// What a summary says a procedure returns is the value it returns, byte
/// for byte, and O103 spells it so (#2388). A literal word is read through
/// the exact value ingress and kept as written — `1.0`, `1.00`, `1e3`,
/// `007`, `true` and ` 5` had become `1`, `1`, `1000`, `7`, `1` and `5`; a
/// braced `$a` is the text `$a`, no passthrough of `a`, and a braced `$x` no
/// read of `x`; and an `expr` literal operand is its own value only as a
/// canonical decimal integer, the expression route deciding `0x10`, `010`,
/// `true` and `1e3`, which the summary had taken as written. The re-run reads
/// a return word the same way: `return " 5"` had folded `[p 1]` to `5`. Each
/// program prints what tclsh 8.4 to 9.1 print, before and after `tcl opt`;
/// for the braced `$x`, O100 still rewrites the body's `return {$x}` (#2391),
/// and `[p]`, the one call, folds to the text.
#[test]
fn the_summary_returns_the_value_exactly() {
    let literals = [
        ("1.0", Some(ConstantReturn::Float(1.0)), "1.0"),
        ("1.00", Some(ConstantReturn::Str("1.00".to_owned())), "1.00"),
        ("1e3", Some(ConstantReturn::Str("1e3".to_owned())), "1e3"),
        ("007", Some(ConstantReturn::Str("007".to_owned())), "007"),
        ("true", Some(ConstantReturn::Bool(true)), "true"),
        ("{$a}", Some(ConstantReturn::Str("$a".to_owned())), "$a"),
    ];
    for (word, constant, printed) in literals {
        let source = format!("proc p {{a}} {{return {word}}}\nputs [p 1]\n");
        for dialect in DIALECTS {
            let summary = summary_of(&source, dialect, "::p");
            assert_eq!(summary.constant_return, constant, "{dialect}: {word}");
            assert_eq!(summary.return_passthrough_param, None, "{dialect}: {word}");
        }
        prints_under_every_release(&source, &format!("{printed}\n"));
    }
    let padded = "proc p {} {return \" 5\"}\nset y [p]\nputs <$y>\n";
    let arg_sensitive =
        "proc p {a} {if {$a} {return \" 5\"} else {return [expr {$a}]}}\nset y [p 1]\nputs <$y>\n";
    for source in [padded, arg_sensitive] {
        for dialect in DIALECTS {
            assert!(
                rewrites_of(source, dialect)
                    .iter()
                    .any(|fold| fold.code == DiagCode::O103 && fold.replacement == "{ 5}"),
                "{dialect}: {source}"
            );
        }
        prints_under_every_release(source, "< 5>\n");
    }
    let braced_read = "proc p {} {set x 5; return {$x}}\nputs [p]\n";
    for dialect in DIALECTS {
        assert!(
            rewrites_of(braced_read, dialect)
                .iter()
                .any(|fold| fold.code == DiagCode::O103 && fold.replacement == "{$x}"),
            "{dialect}"
        );
    }
    prints_under_every_release(braced_read, "$x\n");
    for (operand, printed) in [("0x10", "16"), ("true", "true"), ("1e3", "1000.0")] {
        let source = format!("proc p {{}} {{return [expr {{{operand}}}]}}\nputs [p]\n");
        prints_under_every_release(&source, &format!("{printed}\n"));
    }
    let octal = "proc p {} {return [expr {010}]}\nputs [p]\n";
    for (series, tclsh) in releases_on_path() {
        let printed = if series < "9.0" { "8\n" } else { "10\n" };
        let (rewritten, _) = optimised(octal, &dialect_of(series));
        for program in [octal, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, printed.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// A rewrite spells a value with whitespace about an integer in braces: the
/// word renderer had taken ` 5` for the integer 5 and spelled it bare, so
/// `puts $x` became `puts  5`, `return $y` became `return  7` and `set z [q]`
/// became `set z  7`, each printing the number without its space (#2392).
/// tclsh 8.4 to 9.1 print what the program prints, before and after `tcl
/// opt`.
#[test]
fn a_padded_value_is_braced_where_a_rewrite_spells_it() {
    let source = "set x \" 5\"\nputs <$x>\nputs $x\nproc q {} {set y \" 7\"; return $y}\n\
                  puts \"<[q]>\"\nset z [q]\nputs <$z>\n";
    for dialect in DIALECTS {
        let (rewritten, _) = optimised(source, dialect);
        for bare in ["puts  5", "return  7", "set z  7"] {
            assert!(!rewritten.contains(bare), "{dialect}: {bare}\n{rewritten}");
        }
    }
    prints_under_every_release(source, "< 5>\n 5\n< 7>\n< 7>\n");
}

/// Where the seedless run is made its answer is the summary's, and a run
/// that proves no one value leaves the summary no constant whatever the
/// return words say: `p`'s loop ends only by raising, so its `return 5` is
/// never reached. The words had made the summary `const(5)` and `tcl opt`
/// rewrote `[p]` to `5`, so the program printed 5 where tclsh 8.4 to 9.1
/// print `divide by zero` — the case of #2390 where the lattice proves no
/// exit. The program prints what tclsh prints, before and after `tcl opt`.
#[test]
fn a_run_that_proves_no_value_leaves_no_constant() {
    let source =
        "proc p {} {while {1} {set x [expr {1/0}]}; return 5}\ncatch {set y [p]} m\nputs $m\n";
    for dialect in DIALECTS {
        let summary = summary_of(source, dialect, "::p");
        assert!(
            !summary.returns_constant && summary.constant_return.is_none(),
            "{dialect}: {summary:?}"
        );
        assert!(
            !rewrites_of(source, dialect)
                .iter()
                .any(|fold| fold.code == DiagCode::O103),
            "{dialect}"
        );
    }
    prints_under_every_release(source, "divide by zero\n");
}

/// A callee's `Name` effect is a value in its caller, on both O103 paths:
/// `bump n` binds `n` to what a re-run of `bump` under `n`'s 1 leaves, 2,
/// so `p`'s seedless run returns 2 — `p` pure, since the call writes only
/// `p`'s own `n` — and the summary path folds `[p]`; `q`'s return depends on
/// `x`, so `[q 5]` folds to 6 on the argument-sensitive re-run, which
/// re-runs `bump` seeded from `q`'s `n`; and `rec`'s re-run reaches `rec 0`
/// through its own re-runs, so `[rec 4]` folds to 10. tclsh 8.4 to 9.1
/// print 2, 6 and 10, before and after `tcl opt`.
#[test]
fn bump_decides_through_both_o103_paths() {
    let source = "proc bump {name {by 1}} {upvar 1 $name v; incr v $by}\n\
                  proc p {} {set n 1; bump n; return $n}\n\
                  proc q {x} {set n $x; bump n; return $n}\n\
                  proc rec {n} {if {$n <= 0} {return 0}; expr {$n + [rec [expr {$n - 1}]]}}\n\
                  puts [p]\nputs [q 5]\nputs [rec 4]\n";
    for dialect in DIALECTS {
        assert_eq!(
            last_value(source, dialect, "::p", "n"),
            LatticeValue::Const(ConstValue::Int(2)),
            "{dialect}"
        );
        let p = summary_of(source, dialect, "::p");
        assert!(p.pure && p.can_fold_static_calls, "{dialect}: {p:?}");
        assert_eq!(p.constant_return, Some(ConstantReturn::Int(2)), "{dialect}");
        let q = summary_of(source, dialect, "::q");
        assert!(q.pure && q.constant_return.is_none(), "{dialect}: {q:?}");
        let (rewritten, _) = optimised(source, dialect);
        for folded in ["puts 2\n", "puts 6\n", "puts 10\n"] {
            assert!(rewritten.contains(folded), "{dialect}: {folded}{rewritten}");
        }
    }
    prints_under_every_release(source, "2\n6\n10\n");
}

/// A command a statement's words run that calls a procedure naming a place
/// is applied as a call statement is: the statement's word effects, paired
/// with it, take what a re-run of the callee leaves in the place, so `n` is
/// 2 after `set r [bump n]`. The unit's lattice takes no call's result, so
/// `s`'s `expr {[bump m] + $m}` is no constant there and `s` has no constant
/// return; the argument-sensitive re-run O103 makes of `s` takes the result
/// and reads `$m` after the write, so `[s]` folds to 6. tclsh 8.4 to 9.1
/// print 2 and 6, before and after `tcl opt`.
#[test]
fn an_embedded_call_applies_its_summary() {
    let source = "proc bump {name {by 1}} {upvar 1 $name v; incr v $by}\n\
                  proc p {} {set n 1; set r [bump n]; return $n}\n\
                  proc s {} {set m 2; set t [expr {[bump m] + $m}]; return $t}\n\
                  puts [p]\nputs [s]\n";
    for dialect in DIALECTS {
        assert_eq!(
            last_value(source, dialect, "::p", "n"),
            LatticeValue::Const(ConstValue::Int(2)),
            "{dialect}"
        );
        let s = summary_of(source, dialect, "::s");
        assert!(s.pure && s.constant_return.is_none(), "{dialect}: {s:?}");
        let (rewritten, _) = optimised(source, dialect);
        for folded in ["puts 2\n", "puts 6\n"] {
            assert!(rewritten.contains(folded), "{dialect}: {folded}{rewritten}");
        }
    }
    prints_under_every_release(source, "2\n6\n");
}

/// The top level's value for the last version of `var` in `source` under
/// `dialect`.
fn top_level_value(source: &str, dialect: &str, var: &str) -> LatticeValue {
    let unit = unit_of(source, dialect);
    // The supplied-metadata SSA keys a source name by its resolved cell, so
    // the name is matched against the interned display names.
    let ssa = &unit.top_level.ssa;
    let symbol = ssa
        .variable_symbols()
        .find(|symbol| ssa.var_name(*symbol) == var)
        .unwrap_or_else(|| panic!("no `{var}` in {source}"));
    unit.top_level
        .sccp
        .values
        .iter()
        .filter(|((sym, _), _)| *sym == symbol)
        .max_by_key(|((_, version), _)| *version)
        .map(|(_, value)| value.clone())
        .expect("a definition")
}

/// `series`' tclsh stops `source` with an error, and its optimised form
/// with the same one: nothing printed, and a non-zero exit.
fn errors_alike_under(source: &str, series: &str) {
    for (found, tclsh) in releases_on_path() {
        if found != series {
            continue;
        }
        let (rewritten, _) = optimised(source, &dialect_of(series));
        for program in [source, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((false, String::new())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

const BUMP: &str = "proc bump {name {by 1}} {upvar 1 $name v; incr v $by}\n";

/// The seven summary witnesses (`value-transfers.md` § *Proc-level transfer
/// summaries*), each a caller of a procedure whose summary it applies, under
/// every dialect and release: `bump n; bump n 2` leaves `n` 4 and `twice t`,
/// two `bump` updates through `twice`'s own alias, leaves `t` 3, each proved
/// and read as a constant; `reset m` unbinds `m`, so `[info exists m]`
/// decides false; `g` binds `::counter` and states no value, so its read
/// stays; `[rec 4]` folds to 10 on the argument-sensitive path; `ctr; ctr`
/// leaves `::hits` 2 from 8.5, where 8.4's `incr` raises on the absent
/// variable; and `bump absent` binds `absent` to 1 from 8.5, folded only
/// under a release that creates the cell, and raises under 8.4. tclsh 8.4
/// to 9.1 print the same before and after the optimiser, or stop with the
/// same error.
#[test]
fn the_seven_summary_witnesses() {
    let bump_twice = format!("{BUMP}set n 1\nbump n\nbump n 2\nputs $n\n");
    let reset = "proc reset {name} {upvar 1 $name v; unset v}\nset m 1\nreset m\n\
                 if {[info exists m]} {puts bound} else {puts unbound}\n";
    let twice = format!(
        "{BUMP}proc twice {{name}} {{upvar 1 $name w; bump w; bump w}}\nset t 1\ntwice t\nputs $t\n"
    );
    let global = "proc g {} {set ::counter 5}\ng\nputs $::counter\n";
    let rec = "proc rec {n} {if {$n <= 0} {return 0}; expr {$n + [rec [expr {$n - 1}]]}}\n\
               puts [rec 4]\n";
    let hits = "proc ctr {} {incr ::hits}\nctr\nctr\nputs $::hits\n";
    let absent = format!("{BUMP}bump absent\nputs $absent\n");
    for dialect in DIALECTS {
        assert_eq!(
            top_level_value(&bump_twice, dialect, "n"),
            LatticeValue::Const(ConstValue::Int(4)),
            "{dialect}"
        );
        assert_eq!(
            top_level_value(&twice, dialect, "t"),
            LatticeValue::Const(ConstValue::Int(3)),
            "{dialect}"
        );
        assert!(
            unit_of(reset, dialect)
                .top_level
                .sccp
                .constant_branches
                .iter()
                .any(|branch| branch.condition == "[info exists m]" && !branch.value),
            "{dialect}"
        );
        for (source, folded) in [
            (bump_twice.as_str(), "puts 4\n"),
            (twice.as_str(), "puts 3\n"),
            (rec, "puts 10\n"),
        ] {
            assert!(
                optimised(source, dialect).0.contains(folded),
                "{dialect}: {source}"
            );
        }
        assert!(
            !optimised(reset, dialect).0.contains("puts bound"),
            "{dialect}"
        );
        for (source, read) in [(global, "puts $::counter\n"), (hits, "puts $::hits\n")] {
            assert!(
                optimised(source, dialect).0.contains(read),
                "{dialect}: {source}"
            );
        }
        // Of the releases `DIALECTS` holds tcl8.4, tcl8.6 and tcl9.0; beside
        // them f5-irules, on its 8.4 base, and the lenient tcl, which answers
        // only where every release agrees, keep the read as tcl8.4 does, and
        // tcl8.5 and tcl9.1, not in the list, fold as tcl8.6 does.
        let creates_absent = matches!(dialect, "tcl8.6" | "tcl9.0");
        assert_eq!(
            optimised(&absent, dialect).0.contains("puts 1\n"),
            creates_absent,
            "{dialect}"
        );
    }
    prints_under_every_release(&bump_twice, "4\n");
    prints_under_every_release(reset, "unbound\n");
    prints_under_every_release(&twice, "3\n");
    prints_under_every_release(global, "5\n");
    prints_under_every_release(rec, "10\n");
    prints_under_releases_from(hits, "2\n", "8.5");
    errors_alike_under(hits, "8.4");
    prints_under_releases_from(&absent, "1\n", "8.5");
    errors_alike_under(&absent, "8.4");
}

/// One summary serves both callers (`value-transfers-examples.md`): each
/// call applies it with a re-run under its own place's value, so the lattice
/// proves `n` 2 and `other` 11, and `bump`'s body is left as written. tclsh
/// 8.4 to 9.1 print `2 11` before and after the optimiser.
#[test]
fn two_callers_share_one_summary() {
    let source = "proc bump {name} {upvar 1 $name v; incr v}\nset n 1\nbump n\n\
                  set other 10\nbump other\nputs \"$n $other\"\n";
    for dialect in DIALECTS {
        assert_eq!(
            top_level_value(source, dialect, "n"),
            LatticeValue::Const(ConstValue::Int(2)),
            "{dialect}"
        );
        assert_eq!(
            top_level_value(source, dialect, "other"),
            LatticeValue::Const(ConstValue::Int(11)),
            "{dialect}"
        );
        let (rewritten, _) = optimised(source, dialect);
        assert!(
            rewritten.contains("{upvar 1 $name v; incr v}"),
            "{dialect}: {rewritten}"
        );
    }
    prints_under_every_release(source, "2 11\n");
}

/// #2134's program: `[bump m]` nested in a word is a second call site, so no
/// rewrite specialises `bump` to `bump n`'s place and the body keeps `upvar 1
/// $name v`; the summary applies at both calls, the nested one through its
/// statement's word effects, so `n` is 2 and `m` 11. tclsh 8.4 to 9.1 print
/// `2 11 11` before and after the optimiser, where the specialised body
/// printed `3 10 3`.
#[test]
fn a_nested_caller_keeps_the_callee_whole() {
    let source = "proc bump {name} {\n  upvar 1 $name v\n  incr v\n}\nset n 1\nset m 10\n\
                  bump n\nset z [bump m]\nputs \"$n $m $z\"\n";
    for dialect in DIALECTS {
        assert_eq!(
            top_level_value(source, dialect, "n"),
            LatticeValue::Const(ConstValue::Int(2)),
            "{dialect}"
        );
        assert_eq!(
            top_level_value(source, dialect, "m"),
            LatticeValue::Const(ConstValue::Int(11)),
            "{dialect}"
        );
        let (rewritten, _) = optimised(source, dialect);
        assert!(
            rewritten.contains("upvar 1 $name v\n"),
            "{dialect}: {rewritten}"
        );
    }
    prints_under_every_release(source, "2 11 11\n");
}

/// Every release on `PATH` prints `printed` for `source` and then stops it
/// with an error, and does the same for its optimised form under that
/// release's dialect.
fn prints_then_errors(source: &str, printed: &str) {
    for (series, tclsh) in releases_on_path() {
        let (rewritten, _) = optimised(source, &dialect_of(series));
        for program in [source, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((false, printed.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// A call that cannot complete normally is a raise where a run reads how its
/// caller completes, so no exit past it is read: a callee whose re-run under
/// the call's seeds raises — `incr` of `foo`, a division by zero, 8.4's
/// `incr` of an absent place — and a call whose word count the callee's
/// parameters reject leave `[c]` as written on both O103 paths, as an
/// embedded call does. A call whose completion the re-run cannot decide, its
/// seed unknown, leaves the seedless return unknown, and the
/// argument-sensitive re-run decides each call: `[c 1]` folds to 0 and `[c
/// foo]` stays. tclsh 8.4 to 9.1 raise in each, before and after `tcl opt`,
/// and 8.5 onwards create the absent place, so there `[c]` folds to 0.
#[test]
fn a_call_that_cannot_complete_is_a_raise() {
    let raising = [
        format!("{BUMP}proc c {{}} {{set n foo; bump n; return 0}}\nputs [c]\n"),
        format!("{BUMP}proc c {{name}} {{bump name; return 0}}\nputs [c foo]\n"),
        format!("{BUMP}proc c {{}} {{set n foo; set r [bump n]; return 0}}\nputs [c]\n"),
        "proc half {name} {upvar 1 $name v; set v [expr {$v / 0}]}\n\
         proc c {} {set n 1; half n; return 0}\nputs [c]\n"
            .to_owned(),
        format!("{BUMP}proc c {{}} {{set n 1; bump n 1 extra; return 0}}\nputs [c]\n"),
    ];
    let seeded =
        format!("{BUMP}proc c {{x}} {{set n $x; bump n; return 0}}\nputs [c 1]\nputs [c foo]\n");
    let absent = format!("{BUMP}proc c {{}} {{bump n; return 0}}\nputs [c]\n");
    for dialect in ["tcl8.4", "tcl8.6"] {
        for source in &raising {
            let (rewritten, _) = optimised(source, dialect);
            assert!(
                rewritten.contains("puts [c"),
                "{dialect}: {source}{rewritten}"
            );
        }
        let (rewritten, _) = optimised(&seeded, dialect);
        assert!(
            rewritten.contains("puts 0\nputs [c foo]\n"),
            "{dialect}: {rewritten}"
        );
        assert!(
            summary_of(&seeded, dialect, "::c")
                .constant_return
                .is_none(),
            "{dialect}"
        );
        assert_eq!(
            optimised(&absent, dialect).0.contains("puts 0\n"),
            dialect != "tcl8.4",
            "{dialect}"
        );
        // A call its callee's parameters reject states nothing of its place.
        let wrong_count = unit_of(&raising[4], dialect);
        assert!(
            !matches!(
                last_existence(&wrong_count, "::c", "n"),
                Some(Existence::Bound(_))
            ),
            "{dialect}"
        );
    }
    for source in &raising {
        prints_then_errors(source, "");
    }
    prints_then_errors(&seeded, "0\n");
    errors_alike_under(&absent, "8.4");
    prints_under_releases_from(&absent, "0\n", "8.5");
}

/// A statement the solver proves raises ends its path in a run that reads
/// how the procedure completes, so a `return` past it is no exit of the
/// call: `incr` of `foo`, an `expr` statement or an assignment dividing by
/// zero, `error`, and a call to a procedure with a word count it rejects
/// each leave the call unfolded on both O103 paths. tclsh 8.4 to 9.1 raise
/// in each, before and after `tcl opt`.
#[test]
fn a_statement_that_certainly_raises_ends_the_exit_reading() {
    let sources = [
        "proc c {} {set n foo; incr n; return 0}\nputs [c]\n",
        "proc p {} {expr {1/0}; return foo}\nputs [p]\n",
        "proc p {a} {expr {1/0}; return [expr {$a + 1}]}\nputs [p 5]\n",
        "proc p {} {error boom; return 5}\nputs [p]\n",
        "proc p {} {set x [expr {1/0}]; return 5}\nputs [p]\n",
        "proc f {a} {return $a}\nproc p {} {f 1 2; return 0}\nputs [p]\n",
    ];
    for dialect in ["tcl8.4", "tcl8.6"] {
        for source in sources {
            assert!(
                !rewrites_of(source, dialect)
                    .iter()
                    .any(|fold| fold.code == DiagCode::O103),
                "{dialect}: {source}"
            );
        }
    }
    for source in sources {
        prints_then_errors(source, "");
    }
}

const BUMPD: &str = "proc bumpd {{name n}} {upvar 1 $name v; incr v}\n";

/// A call that omits a `Name` parameter names the place its default spells,
/// as Tcl binds it: the call defines that place and its summary applies
/// there, so a read after it, and the next call's re-run, see what it left.
/// At the top level `set n 1; bumpd n; bumpd` and `set n 1; bumpd; bumpd n`
/// leave `n` 3, read as `puts 3`; in a procedure the lattice proves `n` 3
/// and 2 after the same calls, and the store `bumpd` reads is kept, which
/// O126 deleted, so 8.4's `incr` raised, and is read: neither W220 nor W211
/// calls it unread. tclsh 8.4 to 9.1 print the same before and after
/// `tcl opt`.
#[test]
fn an_omitted_name_argument_names_its_default() {
    let after = format!("{BUMPD}set n 1\nbumpd n\nbumpd\nputs $n\n");
    let before = format!("{BUMPD}set n 1\nbumpd\nbumpd n\nputs $n\n");
    let both = format!(
        "{BUMPD}set n 1\nbumpd\nputs $n\nbumpd n\nputs $n\n\
         proc p {{}} {{set n 1; bumpd; return $n}}\nputs [p]\n\
         proc q {{}} {{set n 1; bumpd n; bumpd; return $n}}\nputs [q]\n\
         proc r {{}} {{set n 1; bumpd; return 0}}\nputs [r]\n"
    );
    for dialect in ["tcl8.4", "tcl8.6"] {
        for source in [&after, &before] {
            assert_eq!(
                top_level_value(source, dialect, "n"),
                LatticeValue::Const(ConstValue::Int(3)),
                "{dialect}: {source}"
            );
            assert!(
                optimised(source, dialect).0.contains("puts 3\n"),
                "{dialect}: {source}"
            );
        }
        assert_eq!(
            last_value(&both, dialect, "::p", "n"),
            LatticeValue::Const(ConstValue::Int(2)),
            "{dialect}"
        );
        assert_eq!(
            last_value(&both, dialect, "::q", "n"),
            LatticeValue::Const(ConstValue::Int(3)),
            "{dialect}"
        );
        let (rewritten, _) = optimised(&both, dialect);
        assert!(
            rewritten.contains("bumpd\nputs 2\nbumpd n\nputs 3\n")
                && rewritten.contains("{set n 1; bumpd; return 0}"),
            "{dialect}: {rewritten}"
        );
        for code in [DiagCode::W211, DiagCode::W220] {
            assert!(!reports(&both, dialect, code), "{dialect}: {code:?}");
        }
    }
    prints_under_every_release(&after, "3\n");
    prints_under_every_release(&before, "3\n");
    prints_under_every_release(&both, "2\n3\n2\n3\n0\n");
}

/// A `Name` argument that names an element is a barrier: the summary states
/// what the callee does to a whole place, and an element's index is dropped
/// when its name is normalised, so taking `a(k)` for `a` stated the array
/// bound as a scalar and folded `[array exists a]` to 0. tclsh 8.4 to 9.1
/// print `arr` and 1, before and after `tcl opt`.
#[test]
fn an_element_argument_is_a_barrier() {
    let source = format!(
        "{BUMP}proc q {{}} {{set a(k) 1; bump a(k); if {{[array exists a]}} {{return arr}} else {{return scalar}}}}\n\
         puts [q]\nset b(k) 1\nbump b(k)\nputs [array exists b]\n"
    );
    for dialect in DIALECTS {
        let (rewritten, _) = optimised(&source, dialect);
        assert!(
            rewritten.contains("if {[array exists a]}") && rewritten.contains("[array exists b]"),
            "{dialect}: {rewritten}"
        );
    }
    prints_under_every_release(&source, "arr\n1\n");
}

/// The `reset` of the read-before-set witnesses: it unsets the place its
/// argument names.
const RESET: &str = "proc reset {name} {upvar 1 $name v; unset v}\n";

/// Whether the analyser reports W210 for `name` in `source` under `dialect`.
fn reads_before_set(source: &str, dialect: &str, name: &str) -> bool {
    let named = format!("'{name}'");
    tcl_compiler::analyser::Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagCode::W210 && diagnostic.message.contains(&named))
}

/// A call to a procedure of the module is the assignment its summary states
/// for the place a `Name` argument names: after `reset m`, which unsets it,
/// a read of `m` is read before it is set, in a procedure and at the top
/// level, by `puts` and by `return`, and after the `info exists` test that
/// decides it gone; after `maybe k $c`, which may leave `k` unset, a read of
/// `k` nothing set before the call is too — the analyser has no may form of
/// W210, so it is the plain finding — and after `bump q`, which binds `q`,
/// the read draws nothing. tclsh 8.4 to 9.1 stop with `can't read` in each
/// that reports, before and after `tcl opt`.
#[test]
fn a_read_after_a_call_reads_the_place_as_the_call_leaves_it() {
    let reporting = [
        (
            format!("{RESET}proc p {{}} {{set m 1; reset m; puts $m}}\np\n"),
            "m",
            "",
        ),
        (format!("{RESET}set m 1\nreset m\nputs $m\n"), "m", ""),
        (
            format!("{RESET}proc p {{}} {{set m 1; reset m; return $m}}\nputs [p]\n"),
            "m",
            "",
        ),
        (
            "proc maybe {name c} {upvar 1 $name v; if {$c} {set v 9}}\n\
             proc p {c} {maybe k $c; puts $k}\np 0\n"
                .to_owned(),
            "k",
            "",
        ),
        (
            format!(
                "{RESET}proc p {{}} {{set m 1; reset m; \
                 if {{[info exists m]}} {{puts $m}} else {{puts gone}}; puts $m}}\np\n"
            ),
            "m",
            "gone\n",
        ),
    ];
    for (source, name, printed) in &reporting {
        for dialect in DIALECTS {
            assert!(
                reads_before_set(source, dialect, name),
                "{dialect}: {source}"
            );
        }
        prints_then_errors(source, printed);
    }
    let bound = format!("{BUMP}proc p {{}} {{bump q; puts $q}}\np\n");
    for dialect in DIALECTS {
        assert!(!reports(&bound, dialect, DiagCode::W210), "{dialect}");
    }
}

/// A call the analyser cannot re-run takes its callee's summary: `drop`'s
/// recursion goes 40 calls deep, past the 32 a re-run reaches, so the call
/// at that depth applies `drop`'s step, an unbind, and each run above it
/// reads the place from there; `z` and `y` are unbound after `drop z 40`
/// and `drop y 40`, so both `[info exists …]` tests fold to the `gone`
/// arm. The recursion is no tail call, which O122 would turn into a loop.
/// tclsh 8.4 to 9.1 print `gone` twice, before and after `tcl opt`.
#[test]
fn a_call_past_the_rerun_depth_takes_the_summarys_step() {
    let source = "proc drop {name n} {upvar 1 $name v; \
                  if {$n > 0} {drop v [expr {$n - 1}]; set n 0} else {unset v}}\n\
                  set z 1\ndrop z 40\nif {[info exists z]} {puts kept} else {puts gone}\n\
                  proc p {} {set y 1; drop y 40; \
                  if {[info exists y]} {return kept} else {return gone}}\nputs [p]\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let unit = unit_of(source, dialect);
        assert_eq!(last_existence(&unit, "::p", "y"), UNBOUND, "{dialect}");
        let (rewritten, _) = optimised(source, dialect);
        assert!(
            rewritten.contains("drop z 40\nputs gone\n")
                && rewritten.contains("drop y 40; return gone}"),
            "{dialect}: {rewritten}"
        );
    }
    prints_under_every_release(source, "gone\ngone\n");
}

/// `split` of the empty string is the empty list under every release, and
/// `tcl opt` folds it through the `split` route: the registry's literal
/// folder made it `{}` (#2418), so `set x [split ""]` became `set x {{}}`
/// and `puts [llength $x]` printed 1 where tclsh prints 0.
#[test]
fn split_of_the_empty_string_is_the_empty_list() {
    let source = "proc p {} {\n    set x [split \"\"]\n    puts [llength $x]\n    puts \"<[split {}]>\"\n}\np\n";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            !rewritten.contains("{{}}") && !rewritten.contains("puts 1"),
            "{dialect}: `split \"\"` is the empty list\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "0\n<>\n");
}

/// A substitution whose route raises is labelled by its completion, as the
/// `lset` view labels one, and not as a store: 8.4 reads no sums, so
/// `string range abcdefghijkl 0 1+1` raises `bad index` there and is no value.
#[test]
fn a_raising_substitution_is_labelled_by_its_completion() {
    let source = "proc p {} {\n    set r [string range abcdefghijkl 0 1+1]\n    return $r\n}\n";
    let unit = summarised_unit(source, "tcl8.4");
    assert_eq!(
        answers_for(&unit, "::p", "string"),
        ["evaluated: error after 0 stores"]
    );
    let unit = summarised_unit(source, "tcl8.6");
    assert_eq!(answers_for(&unit, "::p", "string"), ["evaluated"]);
}

/// Code that never runs widens nothing: an `if` arm a decided condition
/// rules out, a loop body false on entry, and a call to a closed user
/// procedure, direct or in a `[...]`, leave `n` a constant. Before, each made every
/// variable of the function external, and not even the condition that kills
/// the arm could be decided. A callee that
/// writes the caller's `n` through `upvar` or `uplevel` still leaves it
/// unknown. tclsh 8.4 to 9.1 print the same for each program before and
/// after the optimiser.
#[test]
fn code_that_never_runs_keeps_the_function_constant() {
    let zero = LatticeValue::Const(ConstValue::Int(0));
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        for source in [
            "set n 0\nif {$n} {puts a} else {puts b}\nputs $n\n",
            "set n 0\nwhile {$n} {puts a}\nputs $n\n",
            "proc p {} {return 1}\np\nset n 0\nif {$n} {puts a}\nputs $n\n",
        ] {
            assert_eq!(
                top_level_value(source, dialect, "n"),
                zero,
                "{dialect}: {source}"
            );
        }
        for writer in [
            "proc p {} {upvar 1 n v; set v 5}\n",
            "proc p {} {uplevel 1 {set n 5}}\n",
        ] {
            for call in ["p", "puts [p]", "set r [p]"] {
                let source = format!("{writer}set n 0\n{call}\nif {{$n}} {{puts five}}\n");
                assert_ne!(
                    top_level_value(&source, dialect, "n"),
                    zero,
                    "{dialect}: {source}"
                );
            }
        }
        assert_eq!(
            top_level_value(
                "proc p {} {return 1}\nset n 0\nputs [p]\nif {$n} {puts a}\nputs $n\n",
                dialect,
                "n"
            ),
            zero,
            "{dialect}"
        );
    }
    prints_under_every_release(
        "set n 0\nif {$n} {puts a} else {puts b}\nputs $n\n",
        "b\n0\n",
    );
    prints_under_every_release(
        "proc p {} {return 1}\np\nset n 0\nif {$n} {puts a}\nputs $n\n",
        "0\n",
    );
    prints_under_every_release(
        "proc p {} {upvar 1 n v; set v 5}\nset n 0\np\nif {$n} {puts five}\n",
        "five\n",
    );
    prints_under_every_release(
        "proc p {} {upvar 1 n v; set v 5; return 1}\nset n 0\nputs [p]\nputs $n\n",
        "1\n5\n",
    );
}

/// A store a callback reads is no dead store where it is registered: `after
/// 0 {puts $x}` run by `update` prints 5, so `set x 5` stays (#2308). A
/// callee that writes its caller's frame through `upvar 1 $name`, called
/// with `::g`, writes the global, so the top level's later read of `g` is
/// not the stale 1 (#2410). tclsh 8.4 to 9.1 print the same before and after
/// the optimiser.
#[test]
fn a_callback_read_and_a_qualified_upvar_write_keep_their_stores() {
    prints_under_every_release(
        "set x 5\nafter 0 {puts $x}\nupdate\nset x 6\nputs $x\n",
        "5\n6\n",
    );
    prints_under_every_release(
        "proc bump {name {by 1}} {upvar 1 $name v; incr v $by}\nset g 1\n\
         proc c {} {bump ::g; return $::g}\nputs [c]\nputs $g\n",
        "2\n2\n",
    );
    prints_under_every_release(
        "proc bump {name {by 1}} {upvar 1 $name v; incr v $by}\n\
         proc twice {name} {upvar 1 $name w; bump w; bump w}\nset g 1\n\
         proc c {} {twice ::g}\nc\nputs $g\n",
        "3\n",
    );
    // The alias forms: the callee writes the caller's `x`, which the caller
    // made the global `g` (#2410).
    prints_under_every_release(
        "proc bump {name {by 1}} {upvar 1 $name v; incr v $by}\nset g 1\n\
         proc c {} {upvar #0 g x; bump x}\nc\nputs $g\n",
        "2\n",
    );
    prints_under_every_release(
        "proc bump {name {by 1}} {upvar 1 $name v; incr v $by}\nset g 1\n\
         proc c {} {global g; bump g}\nc\nputs $g\n",
        "2\n",
    );
}

#[test]
fn zz_probe_c_bump() {
    for source in [
        "proc incr {args} {puts \"shadowed\"; return 0}\nproc bump {name} {upvar 1 $name v; set v [expr {$v + 1}]}\nproc c {} {set x 0; bump x; return $x}\nputs [c]\n",
        "proc bump {name} {upvar 1 $name v; set v [expr {$v + 1}]}\nproc c {} {set x 0; bump x; return $x}\nputs [c]\n",
    ] {
        let (rewritten, opts) = optimised(source, "tcl8.6");
        eprintln!("ZZOPT {rewritten:?} {:?}", opts.iter().map(|o| format!("{o:?}").chars().take(200).collect::<String>()).collect::<Vec<_>>());
    }
}
