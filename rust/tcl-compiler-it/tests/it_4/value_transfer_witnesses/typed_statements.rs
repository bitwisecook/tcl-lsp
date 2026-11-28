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

//! Typed statements: the three `incr` models, typed assignments, quoting of rewritten words and keyed dictionary updates.

use super::*;

/// The loop enumeration's answer for `x` after `statement` runs once as the
/// body of `for {set i 0} {$i < 1} {incr i} {…}`, seeded as the solver
/// seeds it — `x` from `init` and `n` from 3 through the literal ingress.
fn simulated_text(dialect: &str, init: &str, statement: &str) -> Option<String> {
    let registry = static_context_for(dialect).commands();
    let profile = resolve_environment(dialect).analyser_profile();
    let module = lower_to_ir_with_dialect(
        &format!("for {{set i 0}} {{$i < 1}} {{incr i}} {{{statement}}}\n"),
        registry,
        tcl_lexer::LexerConfig::default(),
        Some(profile),
    );
    let for_stmt = module
        .top_level
        .statements
        .iter()
        .find(|stmt| matches!(stmt, Statement::For { .. }))
        .expect("the loop");
    let mut seed = StaticEnv::new();
    seed.insert("x".to_owned(), seeded(init));
    seed.insert("n".to_owned(), seeded("3"));
    let env = summarise_for_statement(
        for_stmt,
        &seed,
        DEFAULT_MAX_STATIC_LOOP_ITERS,
        LoopSemantics {
            policy: FoldPolicy::from_registry(registry),
            registry,
        },
    )?;
    match env.get("x").expect("x after the loop") {
        StaticValue::Number(NumericValue::Int(i)) => Some(i.to_string()),
        StaticValue::String(s) => Some(s.as_str().to_owned()),
        other @ StaticValue::Number(_) => panic!("{dialect}: an increment gave {other:?}"),
    }
}

/// The interval domain's answer at `x`'s last definition in `::p`.
fn interval_at(unit: &CompilationUnit, dialect: &str) -> Interval {
    let function = unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("the procedure");
    let profile = resolve_environment(dialect).analyser_profile();
    let intervals = compute_intervals_with(
        &function.cfg,
        &function.ssa,
        &function.sccp.values,
        numbers_for_dialect(Some(profile)),
    );
    let x = function.ssa.var_symbol("x").expect("x");
    intervals
        .iter()
        .filter(|((symbol, _), _)| *symbol == x)
        .max_by_key(|((_, version), _)| *version)
        .map(|(_, interval)| *interval)
        .expect("an interval for x")
}

/// Whether `interval` holds the integer `text` spells (a bignum included).
fn interval_holds(interval: Interval, text: &str) -> bool {
    let value: i128 = text.parse().expect("an integer");
    interval.lo.is_none_or(|lo| i128::from(lo) <= value)
        && interval.hi.is_none_or(|hi| value <= i128::from(hi))
}

/// The lattice, the loop simulator and the interval domain answer one
/// increment alike: the lattice and the simulator give the same value or
/// both decline, and the interval never excludes the lattice's value.
/// Each value is the release's own: `incr x $n` is 8 everywhere; `incr x`
/// over `010` is 9 up to 8.6 and 11 from 9.0; over `9223372036854775807`
/// it is `9223372036854775808` from 8.5 and `-8` under 8.4, which no model
/// computes, so 8.4 declines. `f5-irules` answers as its 8.4 base does
/// (ruling 8), and the lenient profile, naming no single release, declines
/// both release-dependent cases.
#[test]
fn the_three_incr_models_agree() {
    crate::install_surfaces();
    let cases: [(&str, &str, [Option<&str>; 5]); 3] = [
        ("5", "incr x $n", [Some("8"); 5]),
        (
            "010",
            "incr x",
            [Some("9"), Some("9"), Some("11"), Some("9"), None],
        ),
        (
            "9223372036854775807",
            "incr x",
            [
                None,
                Some("9223372036854775808"),
                Some("9223372036854775808"),
                None,
                None,
            ],
        ),
    ];
    for (init, statement, expected) in cases {
        let source = format!("proc p {{}} {{set x {init}; set n 3; {statement}; return $x}}\n");
        for (dialect, want) in DIALECTS.into_iter().zip(expected) {
            let unit = unit_of(&source, dialect);
            let lattice =
                lattice_text(value_at(&unit, "::p", "x", 2).expect("the increment's definition"));
            let simulated = simulated_text(dialect, init, statement);
            assert_eq!(
                lattice.as_deref(),
                want,
                "{dialect}: lattice, {statement} over {init}"
            );
            assert_eq!(
                simulated, lattice,
                "{dialect}: simulator, {statement} over {init}"
            );
            if let Some(value) = &lattice {
                let interval = interval_at(&unit, dialect);
                assert!(
                    interval_holds(interval, value),
                    "{dialect}: {interval:?} excludes {value}"
                );
            }
        }
        for (series, tclsh) in releases_on_path() {
            let unit = unit_of(&source, &dialect_of(series));
            let Some(value) = lattice_text(value_at(&unit, "::p", "x", 2).expect("the definition"))
            else {
                continue;
            };
            let program = format!("set x {init}; set n 3; {statement}; puts $x");
            assert_eq!(
                run_script(&tclsh, &program),
                Some((true, format!("{value}\n"))),
                "tclsh{series}: {program}"
            );
        }
    }
}

/// `set result [incr n]` reads the store it increments, so neither dead
/// store rewrite (O109, O126) takes `set n 1` and the increment stays;
/// the optimised program prints 2 twice, as every release does (#2050).
#[test]
fn set_result_incr_keeps_its_increment() {
    crate::install_surfaces();
    let source = "proc p {} {set n 1; set result [incr n]; puts $result; puts $n}\np\n";
    let store = source.find("set n 1").expect("the store");
    for dialect in DIALECTS {
        let rewrites = rewrites_of(source, dialect);
        assert!(
            !rewrites.iter().any(|r| {
                matches!(r.code, DiagCode::O109 | DiagCode::O126)
                    && (r.span.start() as usize..r.span.end() as usize).contains(&store)
            }),
            "{dialect}: {rewrites:#?}"
        );
        let (rewritten, all) = optimised(source, dialect);
        assert!(
            rewritten.contains("set n 1") && rewritten.contains("[incr n]"),
            "{dialect}:\n{rewritten}\n{all:#?}"
        );
    }
    prints_under_every_release(source, "2\n2\n");
}

/// The string and list build chains fold through a `$var` piece the
/// lattice proves — `append s $p`, a leading space kept (#2052), and
/// `lappend l a $x` — and the folded value reaches the read: the optimised
/// program prints it as a constant, whichever of the chain fold (O104/O130)
/// or forwarding and dead-store removal takes the statements (the
/// optimiser's order, ruling of 18 November), and prints what tclsh 8.4 to
/// 9.1 print. The programs run at the top level, which is no iRule.
#[test]
fn o104_and_o130_fold_a_chain_through_a_lattice_operand() {
    crate::install_surfaces();
    let cases = [
        (
            "set s hello\nset p again\nappend s $p\nputs $s\n",
            "puts helloagain",
            "helloagain\n",
        ),
        (
            "set s hello\nset p { again}\nappend s $p\nputs $s\n",
            "puts {hello again}",
            "hello again\n",
        ),
        (
            "set l {}\nset x b\nlappend l a $x\nputs $l\n",
            "puts {a b}",
            "a b\n",
        ),
    ];
    for (source, read, output) in cases {
        for dialect in TCL_CORE_DIALECTS {
            let (rewritten, rewrites) = optimised(source, dialect);
            assert!(
                rewritten.contains(read),
                "{dialect}: {source}\n{rewritten}\n{rewrites:#?}"
            );
        }
        for (series, tclsh) in releases_on_path() {
            let (rewritten, _) = optimised(source, &dialect_of(series));
            assert_eq!(
                run_script(&tclsh, &rewritten),
                Some((true, output.to_owned())),
                "tclsh{series}:\n{rewritten}"
            );
        }
        prints_under_every_release(source, output);
    }
}

/// Deleting a store whose value was a quoted word takes the whole word:
/// no line of the optimised program holds only its closing quote (#2053).
#[test]
fn deleting_a_quoted_store_leaves_no_quote() {
    crate::install_surfaces();
    let source = "set s hello\nset p \"again\"\nappend s $p\nputs $s\n";
    for dialect in DIALECTS {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            !rewritten.lines().any(|line| line.trim() == "\""),
            "{dialect}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "helloagain\n");
}

/// `lassign` is no write under a profile whose release lacks it: under
/// 8.4 the call is an unknown command `catch` absorbs, so `a` is still
/// `old` at the `puts` — the optimiser forwards it, and the store that fed
/// it is then dead, where deleting the store while keeping the read made
/// 8.4 fail with `can't read "a"` — and 8.4 prints `old`. From 8.5
/// `lassign` assigns `new`, which no rewrite forwards past (#2144).
#[test]
fn lassign_is_not_a_write_under_a_profile_without_it() {
    crate::install_surfaces();
    let source = "set a old\ncatch {lassign {new second} a b}\nputs $a\n";
    let (rewritten, rewrites) = optimised(source, "tcl8.4");
    assert!(rewritten.contains("puts old"), "{rewritten}\n{rewrites:#?}");
    for dialect in ["tcl8.6", "tcl9.0"] {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("puts $a"),
            "{dialect}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    for (series, tclsh) in releases_on_path() {
        let (rewritten, _) = optimised(source, &dialect_of(series));
        let want = if series == "8.4" { "old\n" } else { "new\n" };
        for program in [source, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, want.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// A typed assignment reads its value word as Tcl substitutes it: a quoted
/// `\t` is a tab and a braced backslash-newline one space. Read by its
/// spelling, `set s "a\tb"` held four characters, so `string length $s`
/// folded to 4 and `[list $c]` quoted a backslash (tclsh 8.4 to 9.1 print
/// 3, 3, 2, `{x<TAB>y}` and 3).
#[test]
fn a_typed_assignment_reads_its_word_as_tcl_substitutes_it() {
    crate::install_surfaces();
    let cases: [(&str, &str, LatticeValue, &str); 5] = [
        (
            "proc p {} {set s \"a\\tb\"; puts [string length $s]}\np\n",
            "s",
            text("a\tb"),
            "3\n",
        ),
        (
            "proc p {} {set e \"p\\tq\"; set f [set e]; puts [string length $f]}\np\n",
            "f",
            text("p\tq"),
            "3\n",
        ),
        (
            "proc p {} {set b \"\\t\"; append b q; puts [string length $b]}\np\n",
            "b",
            text("\tq"),
            "2\n",
        ),
        (
            "proc p {} {set c \"x\\ty\"; set d [list $c]; puts $d}\np\n",
            "d",
            text("{x\ty}"),
            "{x\ty}\n",
        ),
        (
            "proc p {} {set s {a\\\nb}; puts [string length $s]}\np\n",
            "s",
            text("a b"),
            "3\n",
        ),
    ];
    for (source, var, value, output) in cases {
        for dialect in TCL_CORE_DIALECTS {
            assert_eq!(
                last_value(source, dialect, "::p", var),
                value,
                "{dialect}: {source}"
            );
        }
        prints_under_every_release(source, output);
    }
}

/// A braced `incr` amount is its own text: `incr x {$n}` raises `expected
/// integer but got "$n"` in every release, so `x#2` has no value and the
/// statement reads no `n`; the route proves the error and answers it as the
/// completion it is, after no store. Read as a substitution it folded to 4.
#[test]
fn a_braced_increment_amount_is_its_text() {
    crate::install_surfaces();
    let body = "proc p {} {set n 3; set x 1; incr x {$n}; return $x}\n";
    for dialect in DIALECTS {
        let unit = unit_of(body, dialect);
        assert_eq!(
            value_at(&unit, "::p", "x", 2),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
        assert_eq!(
            answers_for(&unit, "::p", "incr"),
            ["evaluated: error after 0 stores"],
            "{dialect}"
        );
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("authored fixture body");
        let n = function.ssa.var_symbol("n").expect("n");
        let increment = function
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .find(|stmt| matches!(stmt.statement, Statement::Incr { .. }))
            .expect("the increment");
        assert!(
            !increment.uses.contains_key(&n),
            "{dialect}: a braced amount reads nothing"
        );
    }
    let source = format!("{body}puts [catch p msg]; puts $msg\n");
    prints_under_every_release(&source, "1\nexpected integer but got \"$n\"\n");
}

/// A list's first element that starts with `#` is brace-quoted from 8.5 and
/// bare in 8.4 (`puts [list # a]` prints `# a` under tclsh 8.4 and `{#} a`
/// from 8.5), so each release folds its own rendering and a profile naming
/// no release declines.
#[test]
fn a_leading_hash_is_quoted_per_release() {
    crate::install_surfaces();
    let source = "proc p {} {set r [list # a]; set l {}; lappend l # b; puts $r; puts $l}\np\n";
    for (dialect, list, appended) in [
        ("tcl8.4", Some("# a"), Some("# b")),
        ("tcl8.6", Some("{#} a"), Some("{#} b")),
        ("tcl9.0", Some("{#} a"), Some("{#} b")),
        ("tcl", None, None),
    ] {
        let expect = |value: Option<&str>| value.map_or(LatticeValue::Overdefined, text);
        assert_eq!(
            last_value(source, dialect, "::p", "r"),
            expect(list),
            "{dialect}"
        );
        assert_eq!(
            last_value(source, dialect, "::p", "l"),
            expect(appended),
            "{dialect}"
        );
    }
    // The same rule reaches every consumer that renders a list for the
    // target: the `lappend` chain fold (O130) and the `args` list an
    // argument-sensitive procedure fold (O103) seeds.
    let variadic = "proc f {args} {return $args}\nputs [f # a]\nputs [f #x b]\n";
    for (series, tclsh) in releases_on_path() {
        let (expected, variadic_expected) = if series == "8.4" {
            ("# a\n# b\n", "# a\n#x b\n")
        } else {
            ("{#} a\n{#} b\n", "{#} a\n{#x} b\n")
        };
        for (program, expected) in [(source, expected), (variadic, variadic_expected)] {
            let (rewritten, _) = optimised(program, &dialect_of(series));
            for program in [program, rewritten.as_str()] {
                assert_eq!(
                    run_script(&tclsh, program),
                    Some((true, expected.to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}

/// A typed assignment is `set`'s lowering, so once the module shadows
/// `set` it proves nothing: with `proc set {name value} {return ZZZ}`,
/// `set s hello` never writes `s` and `append s world` leaves `world`
/// (tclsh 8.4 to 9.1), where folding the assignment rewrote the program to
/// print `helloworld`.
#[test]
fn a_typed_assignment_declines_once_set_is_rebound() {
    crate::install_surfaces();
    let source =
        "proc set {name value} {return ZZZ}\nproc p {} {set s hello; append s world; puts $s}\np\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let unit = unit_of(source, dialect);
        // The rebound `set` never writes `s`, so `append` starts from an
        // unbound `s`: its value is `world`, as tclsh prints, or unknown —
        // never the `helloworld` a typed assignment would have given.
        assert!(
            matches!(
                value_at(&unit, "::p", "s", 1),
                Some(LatticeValue::Overdefined)
            ) || value_at(&unit, "::p", "s", 1)
                == Some(LatticeValue::Const(ConstValue::String("world".into()))),
            "{dialect}: {:?}",
            value_at(&unit, "::p", "s", 1)
        );
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            !rewritten.contains("helloworld"),
            "{dialect}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "world\n");
}

/// O100 forwards a value holding a tab as a word that re-reads to the same
/// value: the tab stays a tab inside the braces (tclsh 8.4 to 9.1 print
/// `a<TAB>b` and `x<TAB>y`, before and after the rewrite).
#[test]
fn o100_forwards_a_tab_verbatim() {
    crate::install_surfaces();
    for (source, word, output) in [
        ("set r {a\tb}\nputs $r\n", "{a\tb}", "a\tb\n"),
        (
            "set b [string range \"x\\ty\" 0 2]\nputs $b\n",
            "{x\ty}",
            "x\ty\n",
        ),
    ] {
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            let rewrites = rewrites_of(source, dialect);
            assert!(
                rewrites
                    .iter()
                    .any(|r| r.code == DiagCode::O100 && r.replacement == word),
                "{dialect}: {rewrites:#?}"
            );
        }
        prints_under_every_release(source, output);
    }
}

/// A keyed update reads the dictionary it rewrites under every spelling
/// that reaches it: the qualified `::tcl::dict::` commands, a nested
/// `[dict set …]`, and an alias of `dict set`. The read had been recorded
/// only for the `dict` ensemble at statement level, so O109 deleted the
/// store feeding the others: `::tcl::dict::set d k v` printed `k v` where
/// tclsh 8.5 to 9.1 print `a 1 k v`. Tcl 8.4 has no `dict`, and every
/// program fails there the same way before and after.
#[test]
fn a_keyed_update_reads_its_dictionary_under_every_spelling() {
    crate::install_surfaces();
    let programs = [
        (
            "proc p {} {set d {a 1}; ::tcl::dict::set d k v; return $d}\nputs [p]\n",
            "a 1 k v\n",
        ),
        (
            "proc p {} {set d {a 1}; puts [dict set d k v]}\np\n",
            "a 1 k v\n",
        ),
        (
            "proc p {} {set d {a 1}; puts [::tcl::dict::set d k v]}\np\n",
            "a 1 k v\n",
        ),
        (
            "proc p {} {set d {a 1 b 2}; ::tcl::dict::unset d a; return $d}\nputs [p]\n",
            "b 2\n",
        ),
        (
            "proc p {} {set d {a 1}; ::tcl::dict::incr d a; return $d}\nputs [p]\n",
            "a 2\n",
        ),
        (
            "proc p {} {set d {a 1}; ::tcl::dict::append d a x; return $d}\nputs [p]\n",
            "a 1x\n",
        ),
        (
            "proc p {} {set d {a 1}; ::tcl::dict::lappend d a x; return $d}\nputs [p]\n",
            "a {1 x}\n",
        ),
        (
            "interp alias {} ds {} dict set\nproc p {} {set d {a 1}; ds d k v; return $d}\nputs [p]\n",
            "a 1 k v\n",
        ),
    ];
    for (source, expected) in programs {
        for (series, tclsh) in releases_on_path() {
            let (rewritten, _) = optimised(source, &dialect_of(series));
            if series == "8.4" {
                assert_eq!(
                    run_script(&tclsh, &rewritten),
                    run_script(&tclsh, source),
                    "tclsh8.4:\n{rewritten}"
                );
                continue;
            }
            for program in [source, rewritten.as_str()] {
                assert_eq!(
                    run_script(&tclsh, program),
                    Some((true, expected.to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}
