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

//! The expression route: rebound math functions, finite and correlated inputs, numeral grammar per release and what O101 may rewrite.

use super::*;

/// A BPF-Tcl expression never takes the Tcl engine's answer: BPF-Tcl's
/// signed division truncates towards zero (`-7 / 2` is `-3`) where Tcl
/// floors (`-4` under tclsh 8.4 to 9.1), so the route under the BPF
/// language evaluates nothing and the BPF frontend's own arithmetic stays
/// the only answer. No shipped command declares the BPF language yet; the
/// synthetic `bpfexpr` stands for the frontend's expression arguments.
#[test]
fn a_bpf_expression_never_takes_the_tcl_answer() {
    use tcl_registry::spec::CommandSpec;
    use tcl_registry::value_transfer::SemanticsDeclaration;
    use tcl_registry::value_transfer::builtins::BPF_EXPR;
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "bpfexpr",
        semantics: SemanticsDeclaration::Declared(&BPF_EXPR),
        ..CommandSpec::DEFAULT
    });
    let source = "proc p {} {set r [bpfexpr {-7 / 2}]; set t [expr {-7 / 2}]; return $r$t}\n";
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    assert_eq!(
        value_at(&unit, "::p", "r", 1),
        Some(LatticeValue::Overdefined)
    );
    assert_eq!(
        answers_for(&unit, "::p", "bpfexpr"),
        ["declined: unsupported"]
    );
    assert_eq!(
        value_at(&unit, "::p", "t", 1),
        Some(LatticeValue::Const(ConstValue::int(-4)))
    );
    prints_under_every_release("puts [expr {-7 / 2}]\n", "-4\n");
}

/// A math function the module rebinds declines. From 8.5 `abs(…)`
/// dispatches to the command `::tcl::mathfunc::abs`, so the module's `proc`
/// of that name is what runs: tclsh 8.5 to 9.1 print 99. Under 8.4 the
/// grammar dispatches internally and has no such command, so the `proc`
/// cannot even be created and the program prints 2. A namespace-local
/// override is the same rebinding: from 8.5 `expr` resolves
/// `tcl::mathfunc::abs` relative to the namespace it runs in first, so
/// `abs(…)` inside `::ns` calls `::ns::tcl::mathfunc::abs` (tclsh 8.5 to
/// 9.1: 99; 8.4: 2).
#[test]
fn abs_rebinding_declines() {
    let rebound = "catch {rename ::tcl::mathfunc::abs ::tcl::mathfunc::saved_abs}\n\
                   catch {proc ::tcl::mathfunc::abs {x} {return 99}}\n\
                   proc p {} {set r [expr {abs(-2)}]; return $r}\nputs [p]\n";
    let local = "namespace eval ns {\n\
                 namespace eval tcl::mathfunc { proc abs {x} {return 99} }\n\
                 proc p {} {set r [expr {abs(-2)}]; return $r}\n\
                 }\nputs [ns::p]\n";
    let plain = "proc p {} {set r [expr {abs(-2)}]; return $r}\nputs [p]\n";
    let two = Some(LatticeValue::Const(ConstValue::int(2)));
    for (dialect, want) in [
        ("tcl8.4", two.clone()),
        ("tcl8.6", Some(LatticeValue::Overdefined)),
        ("tcl9.0", Some(LatticeValue::Overdefined)),
    ] {
        let unit = unit_of(rebound, dialect);
        assert_eq!(value_at(&unit, "::p", "r", 1), want, "{dialect}");
        assert_eq!(
            value_at(&unit_of(local, dialect), "::ns::p", "r", 1),
            want,
            "{dialect}: the namespace-local override"
        );
        assert_eq!(
            value_at(&unit_of(plain, dialect), "::p", "r", 1),
            two,
            "{dialect}"
        );
    }
    assert_eq!(
        answers_for(&unit_of(rebound, "tcl9.0"), "::p", "expr"),
        ["declined: rebinding-suspected"]
    );
    assert_eq!(
        answers_for(&unit_of(local, "tcl9.0"), "::ns::p", "expr"),
        ["declined: rebinding-suspected"]
    );
    for (series, tclsh) in releases_on_path() {
        let expected = if series == "8.4" { "2\n" } else { "99\n" };
        for source in [rebound, local] {
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
}

/// A nested command the module rebinds declines the whole expression: with
/// the module's own `proc llength`, `[llength {a b}]` is 99, and tclsh 8.4
/// to 9.1 print 198 for `expr {[llength {a b}] * 2}`; the builtin fold would
/// have answered 4.
#[test]
fn a_rebound_nested_head_declines_the_expression() {
    let source = "proc llength {l} {return 99}\n\
                  proc p {} {set r [expr {[llength {a b}] * 2}]; return $r}\nputs [p]\n";
    for dialect in TCL_CORE_DIALECTS {
        let unit = unit_of(source, dialect);
        assert_eq!(
            value_at(&unit, "::p", "r", 1),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
        assert_eq!(
            answers_for(&unit, "::p", "expr"),
            ["declined: rebinding-suspected"],
            "{dialect}"
        );
    }
    let pure = "proc p {} {set r [expr {[llength {a b}] * 2}]; return $r}\nputs [p]\n";
    for dialect in TCL_CORE_DIALECTS {
        assert_eq!(
            value_at(&unit_of(pure, dialect), "::p", "r", 1),
            Some(LatticeValue::Const(ConstValue::int(4))),
            "{dialect}"
        );
    }
    prints_under_every_release(source, "198\n");
    prints_under_every_release(pure, "4\n");
}

/// A condition over one finite input decides when every member agrees:
/// `foreach a {1 2}` makes `a` the set `{1, 2}`, so `$a > 0` is true for
/// each and the branch folds, while `$a > 1` differs between the members
/// and stays open (tclsh 8.4 to 9.1 print `pos pos` and `small big`).
#[test]
fn a_finite_condition_decides_when_every_member_agrees() {
    let decided = |source: &str, dialect: &str| -> Vec<(String, bool)> {
        let unit = unit_of(source, dialect);
        unit.procedures
            .get(&fixture_id(&unit.ir_module.procedures, "::p"))
            .expect("the procedure")
            .sccp
            .constant_branches
            .iter()
            .map(|branch| (branch.condition.clone(), branch.value))
            .collect()
    };
    let agree = "proc p {} {foreach a {1 2} {if {$a > 0} {puts pos} else {puts neg}}}\np\n";
    let split = "proc p {} {foreach a {1 2} {if {$a > 1} {puts big} else {puts small}}}\np\n";
    for dialect in TCL_CORE_DIALECTS {
        assert!(
            decided(agree, dialect).contains(&("$a > 0".to_owned(), true)),
            "{dialect}: {:?}",
            decided(agree, dialect)
        );
        assert!(
            !decided(split, dialect)
                .iter()
                .any(|(condition, _)| condition == "$a > 1"),
            "{dialect}: {:?}",
            decided(split, dialect)
        );
    }
    prints_under_every_release(agree, "pos\npos\n");
    prints_under_every_release(split, "small\nbig\n");
}

/// A folded call keeps a double's Tcl spelling: `proc p {} {return [expr
/// {1.0 * 3}]}; puts [p]` prints `3.0` under tclsh 8.4 to 9.1, and O103 had
/// rewritten the call to `puts 3` with Rust's float rendering.
#[test]
fn a_folded_call_keeps_a_doubles_spelling() {
    for source in [
        "proc p {} {return [expr {1.0 * 3}]}\nputs [p]\n",
        "proc p {} {set r [expr {1.0 * 3}]; return $r}\nputs [p]\n",
    ] {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, _) = optimised(source, dialect);
            assert!(!rewritten.contains("puts 3\n"), "{dialect}:\n{rewritten}");
        }
        prints_under_every_release(source, "3.0\n");
    }
}

/// `format` runs the shared core on its registry-owned route, reading its
/// operands from the lattice and its literal words under the document's
/// escape grammar: tclsh 8.4 and 8.5 read `"\x4142"` as the one byte `B`
/// (`\x` takes every hex digit before 8.6) and 8.6 to 9.1 as `A42`; `%03d`
/// of the constant 5 is `005` and `%5.2f` of 3.14159 is ` 3.14` everywhere.
#[test]
fn format_folds_through_the_shared_core() {
    let source = "proc p {} {set n 5; set r [format %03d $n]; set s [format %s \"\\x4142\"]; \
                  set t [format %5.2f 3.14159]; return \"$r|$s|$t\"}\nputs [p]\n";
    for (dialect, bytes) in [("tcl8.4", "B"), ("tcl8.6", "A42"), ("tcl9.0", "A42")] {
        let unit = unit_of(source, dialect);
        assert_eq!(
            value_at(&unit, "::p", "r", 1),
            Some(text("005")),
            "{dialect}"
        );
        assert_eq!(
            value_at(&unit, "::p", "s", 1),
            Some(text(bytes)),
            "{dialect}"
        );
        assert_eq!(
            value_at(&unit, "::p", "t", 1),
            Some(text(" 3.14")),
            "{dialect}"
        );
    }
    for (series, tclsh) in releases_on_path() {
        let expected = if matches!(series, "8.4" | "8.5") {
            "005|B| 3.14\n"
        } else {
            "005|A42| 3.14\n"
        };
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

/// The route tally counts every family's entries once — the fixed point
/// re-evaluates each statement several times before it settles, so only
/// the settled sweep counts — with a nested route entry counted beside its
/// host's: `set r [expr {"x"}]; set n [expr {[string length abcdef] *
/// 2}]` is two fused expressions, one nesting a direct-routed `string
/// length`, so `direct 1 · expression 2 · implementation 0`; `incr` alone
/// is a direct entry with no expression; a decided literal condition is an
/// expression entry, recorded once for reachability and once for the
/// collected branch, so `if {1} {…}` is two expression entries and no
/// direct one. No shipped command declares an `EvaluatorCapability`, so
/// `implementation` stays 0 throughout; a pack's declared implementation
/// counts there (`a_declared_implementation_folds_through_the_driver`).
/// An entry is counted only
/// once the route is entered: with the module's own `proc expr`, `set r
/// [expr {1 + 1}]` declines at the trust check and never reaches the
/// engine, so it counts nothing.
#[test]
fn route_entries_are_counted_per_family() {
    let tally_of = |source: &str, dialect: &str| {
        let unit = unit_of(source, dialect);
        unit.procedures
            .get(&fixture_id(&unit.ir_module.procedures, "::p"))
            .expect("the procedure")
            .sccp
            .route_tally
    };
    let nested_expr = "proc p {} {set r [expr {\"x\"}]; \
                        set n [expr {[string length abcdef] * 2}]}";
    let direct_only = "proc p {} {set x 1; incr x; return $x}";
    let expression_only = "proc p {} {if {1} {puts a} else {puts b}}";
    let rebound_expr = "proc expr {args} {return 99}\nproc p {} {set r [expr {1 + 1}]}";
    for dialect in DIALECTS {
        let tally = tally_of(nested_expr, dialect);
        assert_eq!(tally.direct, 1, "{dialect}: the nested `string length`");
        assert_eq!(tally.expression, 2, "{dialect}: both fused `expr`s");
        assert_eq!(tally.implementation, 0, "{dialect}");

        let tally = tally_of(direct_only, dialect);
        assert_eq!(tally.direct, 1, "{dialect}: the typed `incr`");
        assert_eq!(tally.expression, 0, "{dialect}");
        assert_eq!(tally.implementation, 0, "{dialect}");

        let tally = tally_of(expression_only, dialect);
        assert_eq!(tally.direct, 0, "{dialect}");
        assert_eq!(
            tally.expression, 2,
            "{dialect}: the condition, reachability and the collected branch"
        );
        assert_eq!(tally.implementation, 0, "{dialect}");

        let tally = tally_of(rebound_expr, dialect);
        assert_eq!(
            (tally.direct, tally.expression, tally.implementation),
            (0, 0, 0),
            "{dialect}: a rebound `expr` enters no route"
        );
    }
}

/// The interface contract's `expr` acceptance list: multi-argument forms,
/// braced versus quoted arguments, short-circuit operators and ternaries,
/// strings that look like code, nested pure substitutions, errors,
/// bignums, and target release ambiguity.
/// Each case is `proc p {} {<prelude>; set r [<expr call>]}`, oracle
/// values checked against `tclsh8.4` to `tclsh9.1` directly.
/// `expression_witnesses_match_every_release_on_path` re-runs the same
/// programs against the real interpreters. `2**64` and `1 << 70` are
/// beyond-wide from 8.5: `tclsh8.4` raises for the first (`**` is not an
/// 8.4 operator) and wraps to 0 for the second, so both decline under
/// `tcl8.4` (`WrongRepresentation`); `"010" + 0` reads the leading zero as
/// octal up to 8.6 and as decimal from 9.0. The procedure is not called at an
/// iRules top level, so the rows run under the core Tcl dialects. The lenient profile
/// declares no release, so all three release-dependent cases decline there.
#[test]
fn expr_acceptance_list() {
    let cases: [(&str, &str, &str, [Option<&str>; 4]); 10] = [
        ("multi-argument form", "", "expr 1 + 2", [Some("3"); 4]),
        (
            "a quoted argument substituted as text",
            "set a {1 + 1}",
            "expr \"$a * 2\"",
            [Some("3"); 4],
        ),
        (
            "short-circuit &&",
            "",
            "expr {0 && [error never]}",
            [Some("0"); 4],
        ),
        (
            "a ternary",
            "",
            "expr {1 ? \"yes\" : \"no\"}",
            [Some("yes"); 4],
        ),
        (
            "a value that looks like code is never re-substituted",
            "set a {[exit]}",
            "expr {$a eq {[exit]}}",
            [Some("1"); 4],
        ),
        (
            "a nested pure substitution",
            "",
            "expr {[string length abcdef] * 2}",
            [Some("12"); 4],
        ),
        ("an error", "", "expr {1/0}", [None; 4]),
        (
            "a bignum",
            "",
            "expr {2**64}",
            [
                None,
                Some("18446744073709551616"),
                Some("18446744073709551616"),
                None,
            ],
        ),
        (
            "a shift beyond wide",
            "",
            "expr {1 << 70}",
            [
                None,
                Some("1180591620717411303424"),
                Some("1180591620717411303424"),
                None,
            ],
        ),
        (
            "a leading-zero numeral, release-dependent",
            "",
            "expr {\"010\" + 0}",
            [Some("8"), Some("8"), Some("10"), None],
        ),
    ];
    for (description, prelude, expr_call, expected) in cases {
        let source = format!("proc p {{}} {{{prelude}\nset r [{expr_call}]\n}}");
        for (dialect, want) in TCL_CORE_DIALECTS.into_iter().zip(expected) {
            let unit = unit_of(&source, dialect);
            let folded = value_at(&unit, "::p", "r", 1).and_then(lattice_text);
            assert_eq!(
                folded.as_deref(),
                want,
                "{dialect}: {description} ({expr_call})"
            );
        }
    }
}

/// One distinct finite SSA value stays correlated with itself: `foreach a
/// {1 2} {set r [expr {$a * $a}]}` gives the in-loop `r` the set `{1, 4}`,
/// never `{1, 2, 4}` — the cartesian product a wrongly-independent pairing
/// would produce (§ *The correlated finite-set limit*).
#[test]
fn the_square_of_one_finite_input_stays_correlated() {
    let source = "proc p {} {foreach a {1 2} {set r [expr {$a * $a}]}}";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        let value = value_at(&unit, "::p", "r", 1).expect("r's in-loop definition");
        let LatticeValue::ConstSet(mut members) = value else {
            panic!("{dialect}: {value:?} is not a finite set");
        };
        members.sort_by_key(|c| match c {
            ConstValue::Number(NumericValue::Int(i)) => *i,
            other => panic!("{dialect}: {other:?}"),
        });
        assert_eq!(
            members,
            vec![ConstValue::int(1), ConstValue::int(4)],
            "{dialect}"
        );
    }
}

/// The mirror pairs of the interface page
/// (`docs/design/compiler/value-transfers.md` § *The correlated
/// finite-set limit*): `a` and `b` are the loop's two binders, so pairing
/// them by position or taking their cartesian product would both be
/// unsound. The loop header answers each binder of the two-binder source
/// with the elements it takes (`a` is `{1 2}`, `b` is `{10 20}`), so each
/// quotient sees the page's two distinct `Finite` identities and declines
/// `CorrelatedSets`, and the version each loop leaves `x` and `y` at holds
/// no value of its own. Only ordered enumeration answers what the loops
/// leave — `x` is 10 and `y` is 5, each loop's last quotient — which the
/// solver states on each loop's exit, so each post-loop branch decides
/// false: tclsh 8.4 to 9.1 print `other` twice.
#[test]
fn the_mirror_pairs_decline_as_correlated() {
    let source = "proc p {} {\n\
                   set x 0\n\
                   foreach {a b} {1 10 2 20} { set x [expr {$b / $a}] }\n\
                   if {$x == 20} { puts twenty } else { puts other }\n\
                   set y 0\n\
                   foreach {a b} {1 20 2 10} { set y [expr {$b / $a}] }\n\
                   if {$y == 25} { puts twentyfive } else { puts other }\n\
                  }\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("the procedure");
        for var in ["x", "y"] {
            let symbol = function.ssa.var_symbol(var).expect(var);
            let last = function
                .sccp
                .values
                .iter()
                .filter(|((sym, _), _)| *sym == symbol)
                .max_by_key(|((_, version), _)| *version)
                .map(|(_, value)| value.clone())
                .expect("a definition");
            assert_eq!(last, LatticeValue::Overdefined, "{dialect}: {var}");
        }
        for condition in ["$x == 20", "$y == 25"] {
            let decided = function
                .sccp
                .constant_branches
                .iter()
                .find(|branch| branch.condition == condition)
                .unwrap_or_else(|| {
                    panic!(
                        "{dialect}: {condition} undecided: {:?}",
                        function.sccp.constant_branches
                    )
                });
            assert!(!decided.value, "{dialect}: {condition}");
        }
        // The two-binder source is lowered: `a` and `b` are two
        // distinct finite inputs, so each quotient declines as correlated.
        let answers = answers_for(&unit, "::p", "expr");
        assert_eq!(
            answers,
            ["declined: correlated-sets", "declined: correlated-sets"],
            "{dialect}"
        );
    }
    prints_under_every_release(&format!("{source}p\n"), "other\nother\n");
}

/// A condition over a leading-zero digit string decides nothing under 8.x:
/// the grammar reads `08` as an invalid octal, so it reaches the condition
/// as text and `if` raises `expected boolean value but got "08"` (tclsh 8.5
/// and 8.6; 8.4, 9.0 and 9.1 print `yes`). It was decided true (I230) and
/// rewritten to `if {1}`. Under 9.0 `08` is the number 8 and the branch is
/// decided.
#[test]
fn an_invalid_octal_condition_decides_nothing() {
    for (literal, quoted) in [("08", false), ("-08", false), ("08", true)] {
        let condition = if quoted {
            format!("\"{literal}\"")
        } else {
            "$x".to_owned()
        };
        let source = format!(
            "proc p {{}} {{set x {literal}; if {{{condition}}} {{puts yes}} else {{puts no}}}}\np\n"
        );
        for (dialect, decided) in [("tcl8.6", false), ("tcl9.0", true)] {
            let unit = unit_of(&source, dialect);
            let function = unit
                .procedures
                .get(&fixture_id(&unit.procedures, "::p"))
                .expect("the procedure");
            assert_eq!(
                !function.sccp.constant_branches.is_empty(),
                decided,
                "{dialect}: {source}: {:?}",
                function.sccp.constant_branches
            );
        }
        for (series, tclsh) in releases_on_path() {
            let (rewritten, rewrites) = optimised(&source, &dialect_of(series));
            assert_eq!(
                run_script(&tclsh, &rewritten),
                run_script(&tclsh, &source),
                "tclsh{series}:\n{rewritten}\n{rewrites:#?}"
            );
        }
    }
}

/// `expr_acceptance_list`'s programs, run against the real `tclsh` per
/// release found on `PATH`: wherever the compiler's fold answers a value
/// it is the same value `tclsh` prints, and wherever the underlying
/// program raises the fold has declined too. At least half the programs
/// must fold on any release, so the witness is not vacuous. The optimised
/// program must print what the original prints — or raise where it raises —
/// under the same release, so a rewrite never folds what the lattice
/// declines: the last four programs are O101's (`1 << 70` wraps to 0 under
/// tclsh 8.4; `1e308 * 10` raises `floating-point value too large to
/// represent` there; `min` is unknown before 8.5; `ABS` is no math function
/// in any release).
#[test]
fn expression_witnesses_match_every_release_on_path() {
    let cases: [(&str, &str); 13] = [
        ("", "expr 1 + 2"),
        ("set a {1 + 1}", "expr \"$a * 2\""),
        ("", "expr {0 && [error never]}"),
        ("", "expr {1 ? \"yes\" : \"no\"}"),
        ("set a {[exit]}", "expr {$a eq {[exit]}}"),
        ("", "expr {[string length abcdef] * 2}"),
        ("", "expr {1/0}"),
        ("", "expr {2**64}"),
        ("", "expr {1 << 70}"),
        ("", "expr {\"010\" + 0}"),
        ("", "expr {1e308 * 10}"),
        ("", "expr {min(1,2)}"),
        ("", "expr {ABS(-2)}"),
    ];
    for (series, tclsh) in releases_on_path() {
        let dialect = dialect_of(series);
        let mut answered = 0usize;
        for (prelude, expr_call) in cases {
            let source = format!("proc p {{}} {{{prelude}\nset r [{expr_call}]\n}}");
            let unit = unit_of(&source, &dialect);
            let folded = value_at(&unit, "::p", "r", 1).and_then(lattice_text);
            let oracle = run_script(&tclsh, &format!("{prelude}\nset r [{expr_call}]\nputs $r"));
            match (&folded, oracle) {
                (Some(value), Some((true, printed))) => {
                    assert_eq!(
                        value.as_str(),
                        printed.trim_end_matches('\n'),
                        "tclsh{series}: {expr_call}"
                    );
                    answered += 1;
                }
                (Some(value), other) => panic!(
                    "tclsh{series}: {expr_call} folded to {value} but tclsh answered {other:?}"
                ),
                // A decline under a release whose runtime tclsh happens
                // not to raise for (`1 << 70` wraps silently under 8.4)
                // is still a decline: nothing to check either way.
                (None, _) => {}
            }
            let program =
                format!("proc p {{}} {{{prelude}\nset r [{expr_call}]\nreturn $r\n}}\nputs [p]\n");
            let (rewritten, rewrites) = optimised(&program, &dialect);
            assert_eq!(
                run_script(&tclsh, &rewritten),
                run_script(&tclsh, &program),
                "tclsh{series}: {expr_call}\n{rewritten}\n{rewrites:#?}"
            );
        }
        assert!(
            answered * 2 > cases.len(),
            "tclsh{series}: the fold answered only {answered} of {} cases",
            cases.len()
        );
    }
}

/// O101 rewrites only what the shared expression route proves, so the
/// programs the old constant folder rewrote against the target stay put
/// (`tcl opt --profile full` applies every pass, as `optimised` does).
/// Oracle, tclsh 8.4: `1 << 70` is `0` (the wide shift wraps), `1e308 * 10`
/// raises `floating-point value too large to represent`, and `min(1,2)`
/// raises `unknown math function "min"`; 8.5 to 9.1 answer
/// `1180591620717411303424`, `Inf` and `1`. `ABS(-2)` raises on every
/// release (`unknown math function "ABS"` under 8.4, `invalid command name
/// "tcl::mathfunc::ABS"` from 8.5): a math function's name is
/// case-sensitive. An infinity in the middle is the same tower: `(1e308 *
/// 10) > 0` is 1 from 8.5 and raises at the product under 8.4. None of them
/// folds under iRules — 8.4, the release its engine runs, folds none — nor
/// under the version-less `tcl` profile, whose releases disagree on each.
#[test]
fn o101_rewrites_only_what_the_route_proves() {
    let cases: [(&str, [Option<&str>; 5]); 5] = [
        (
            "1 << 70",
            [
                None,
                Some("1180591620717411303424"),
                Some("1180591620717411303424"),
                None,
                None,
            ],
        ),
        ("1e308 * 10", [None, Some("Inf"), Some("Inf"), None, None]),
        ("(1e308 * 10) > 0", [None, Some("1"), Some("1"), None, None]),
        ("min(1,2)", [None, Some("1"), Some("1"), None, None]),
        ("ABS(-2)", [None; 5]),
    ];
    for (expression, answers) in cases {
        let program =
            format!("proc p {{}} {{set r [expr {{{expression}}}]; return $r}}\nputs [p]\n");
        for (dialect, answer) in DIALECTS.into_iter().zip(answers) {
            let (rewritten, rewrites) = optimised(&program, dialect);
            let kept = rewritten.contains(&format!("expr {{{expression}}}"));
            match answer {
                None => assert!(
                    kept,
                    "{dialect}: `expr {{{expression}}}` must stay:\n{rewritten}\n{rewrites:#?}"
                ),
                Some(value) => assert!(
                    !kept && rewritten.contains(value),
                    "{dialect}: `expr {{{expression}}}` folds to {value}:\n{rewritten}\n{rewrites:#?}"
                ),
            }
        }
    }
}

/// O112 decided a condition with the old constant folder, which answered
/// a beyond-wide integer or an infinity under 8.4 as 8.5 does and read no
/// binding. tclsh 8.4: `1 << 70` wraps to 0, so `if {(1 << 70) == 0}`
/// takes its first branch and `while {(1 << 70) == 0}` runs; `1e308 * 10`
/// raises `floating-point value too large to represent`. From 8.5 the
/// shift is 1180591620717411303424 and the product `Inf`. With `proc
/// ::tcl::mathfunc::abs {x} {return 99}`, tclsh 8.5 to 9.1 print `other`
/// and `loop`, where O112 kept `puts two` alone (8.4 has no wrapper
/// namespace, so the `proc` fails and the builtin prints `two`). Each
/// program prints the same optimised as it does unoptimised under every
/// release.
#[test]
fn a_structure_fold_stays_within_the_targets_tower() {
    let rebound = "catch {proc ::tcl::mathfunc::abs {x} {return 99}}\n\
                   proc p {} {\n\
                   if {abs(-2) == 2} {puts two} else {puts other}\n\
                   while {abs(-2) == 99} {puts loop; break}\n\
                   }\np\n";
    let programs = [
        (
            "proc p {} {if {(1 << 70) == 0} {puts zero} else {puts big}}\np\n",
            &["tcl8.4"][..],
            &["if {"][..],
        ),
        (
            "proc p {} {while {(1 << 70) == 0} {puts once; break}; puts done}\np\n",
            &["tcl8.4"][..],
            &["while {"][..],
        ),
        (
            "proc p {} {if {1e308 * 10 > 0} {puts inf} else {puts finite}}\np\n",
            &["tcl8.4"][..],
            &["if {"][..],
        ),
        (
            rebound,
            &["tcl8.6", "tcl9.0"][..],
            &["if {abs(-2) == 2}", "while {"][..],
        ),
    ];
    for (program, undecided_under, kept) in programs {
        for dialect in undecided_under {
            let (rewritten, rewrites) = optimised(program, dialect);
            assert!(
                kept.iter().all(|structure| rewritten.contains(structure)),
                "{dialect} decides nothing:\n{rewritten}\n{rewrites:#?}"
            );
        }
        for (series, tclsh) in releases_on_path() {
            let (rewritten, rewrites) = optimised(program, &dialect_of(series));
            assert_eq!(
                run_script(&tclsh, &rewritten),
                run_script(&tclsh, program),
                "tclsh{series}:\n{rewritten}\n{rewrites:#?}"
            );
        }
    }
}
