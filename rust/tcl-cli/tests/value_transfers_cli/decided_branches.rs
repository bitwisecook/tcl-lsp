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

//! Branches and loops the solver decides: the `switch` selection of program (4), dead arms, loop headers, nested equalities and the loop witnesses.

use super::*;

/// Program (4): a `switch` over the string two appends build, whose `baz`
/// arm no run reaches.
const PROGRAM_FOUR_BUILD: &str = "set acc \"\"; append acc foo; append acc bar\n";

/// Program (4)'s arms: `default` is the one that runs.
const PROGRAM_FOUR_ARMS: &str = "{\n    baz     { puts never }\n    default { puts always }\n}\n";

/// The shared-body arms: `baz` passes its body on with a bare `-`, the one
/// spelling every release reads alike, to `qux`'s, which never runs either.
const PROGRAM_FOUR_SHARED: &str =
    "{\n    baz     -\n    qux     { puts never }\n    default { puts always }\n}\n";

/// Program (4) yields I231 on the dead arm for every form of `switch`,
/// through the shipped binary: `tcl diag` reports the arm — `baz`'s pattern
/// at line 3, and `qux`'s at line 4 in the shared-body form — and `tcl opt
/// --profile full` folds the `switch` to `puts always`, which prints
/// `always` under every tclsh release that has the form, as the original
/// does. `-nocase` is 8.5's: a profile that may be 8.4 reports no arm and
/// keeps the statement, and tclsh 8.4 rejects the option.
#[test]
fn program_four_reaches_diag_and_opt_in_every_form() {
    let forms = [
        (
            "exact",
            format!("switch -- $acc {PROGRAM_FOUR_ARMS}"),
            "8.4",
        ),
        (
            "-glob",
            format!("switch -glob -- $acc {PROGRAM_FOUR_ARMS}"),
            "8.4",
        ),
        (
            "-regexp",
            format!("switch -regexp -- $acc {PROGRAM_FOUR_ARMS}"),
            "8.4",
        ),
        (
            "-nocase",
            format!("switch -nocase -- $acc {PROGRAM_FOUR_ARMS}"),
            "8.5",
        ),
        (
            "shared body",
            format!("switch -- $acc {PROGRAM_FOUR_SHARED}"),
            "8.4",
        ),
    ];
    for (form, switch, first) in forms {
        let source = format!("{PROGRAM_FOUR_BUILD}{switch}");
        let dead: &[u64] = if form == "shared body" { &[3, 4] } else { &[3] };
        let found = diagnostics_at(&source, "tcl8.6", &[]);
        for &line in dead {
            assert!(
                found.contains(&(line, 5, "I231".to_owned())),
                "{form}: I231 on line {line}: {found:?}\n{source}"
            );
        }
        let optimised = run_tcl(&[
            "opt",
            "--source",
            &source,
            "--profile",
            "full",
            "--dialect",
            "tcl8.6",
        ]);
        let statements: String = optimised
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            statements.contains("puts always")
                && !statements.contains("puts never")
                && !statements.contains("switch")
                && optimised.contains("O112"),
            "{form}:\n{optimised}"
        );
        for (series, tclsh) in tclshs_from(first) {
            for program in [source.as_str(), optimised.as_str()] {
                assert_eq!(
                    run_tclsh(&tclsh, program),
                    Some((true, "always\n".to_owned())),
                    "{form} under tclsh{series}:\n{program}"
                );
            }
        }
    }

    let nocase = format!("{PROGRAM_FOUR_BUILD}switch -nocase -- $acc {PROGRAM_FOUR_ARMS}");
    for dialect in ["tcl8.4", "tk"] {
        assert!(
            !diagnostics_at(&nocase, dialect, &[])
                .iter()
                .any(|(_, _, code)| code == "I231"),
            "{dialect}: no selection is made"
        );
        let kept = run_tcl(&[
            "opt",
            "--source",
            &nocase,
            "--profile",
            "full",
            "--dialect",
            dialect,
        ]);
        assert!(kept.contains("switch -nocase"), "{dialect}:\n{kept}");
    }
    if let Some((_, tclsh)) = tclshs_from("8.4")
        .into_iter()
        .find(|(series, _)| *series == "8.4")
    {
        assert_eq!(
            run_tclsh(&tclsh, &nocase).map(|(ran, _)| ran),
            Some(false),
            "tclsh8.4 rejects -nocase"
        );
    }
}

/// `tcl explore --show sccp --text` prints the selection the solver makes
/// at an opaque `switch`: over program (4)'s `-glob` form the selected arm
/// and the arm whose body runs are both `default`, with a line naming
/// `baz`'s arm as never selected; a subject that matches names the arm; a
/// subject of two members names one per member; an arm that passes its body
/// on with `-` selects one arm and runs another's; and nothing prints where
/// no selection is made — a parameter subject, and a quoted or braced `-`
/// body under 9.1, which reads two ways there.
#[test]
fn explore_sccp_prints_the_selection() {
    let explore = |source: &str, dialect: &str| {
        run_tcl(&[
            "explore",
            "--source",
            source,
            "--show",
            "sccp",
            "--text",
            "--no-colour",
            "--dialect",
            dialect,
        ])
    };
    let glob = explore(
        &format!("{PROGRAM_FOUR_BUILD}switch -glob -- $acc {PROGRAM_FOUR_ARMS}"),
        "tcl8.6",
    );
    assert!(glob.contains("selection: default"), "{glob}");
    assert!(glob.contains("bodies: default"), "{glob}");
    assert!(glob.contains("arm never selected: baz"), "{glob}");

    let matched = explore(
        "set s foobar\nswitch -glob -- $s {\n    foo*    { puts a }\n    default { puts d }\n}\n",
        "tcl8.6",
    );
    assert!(matched.contains("selection: arm 0"), "{matched}");
    assert!(matched.contains("bodies: arm 0"), "{matched}");

    let finite = explore(
        "proc p {c} {\n    if {$c} { set s foo } else { set s bar }\n    \
         switch -glob -- $s {\n        foo     { puts a }\n        bar     { puts b }\n        \
         default { puts d }\n    }\n}\n",
        "tcl8.6",
    );
    assert!(finite.contains("selection: arm 1, arm 0"), "{finite}");

    let passed_on = explore(
        "set s abc\nswitch -glob -- $s {\n    a*      -\n    b*      { puts ab }\n    \
         default { puts d }\n}\n",
        "tcl9.1",
    );
    assert!(passed_on.contains("selection: arm 0"), "{passed_on}");
    assert!(passed_on.contains("bodies: arm 1"), "{passed_on}");

    let parameter = explore(
        "proc p {x} {\n    switch -glob -- $x {\n        a*      { puts a }\n        \
         default { puts d }\n    }\n}\n",
        "tcl8.6",
    );
    assert!(!parameter.contains("selection:"), "{parameter}");

    let delimited = "set s abc\nswitch -glob -- $s a* {-} b* { puts ab } default { puts d }\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let read = explore(delimited, dialect);
        assert!(read.contains("selection: arm 0"), "{dialect}: {read}");
    }
    for dialect in ["tcl9.1", "tk"] {
        let declined = explore(delimited, dialect);
        assert!(!declined.contains("selection:"), "{dialect}: {declined}");
    }
}

/// The two applied-reachability consumers through `tcl diag`: a loop whose
/// header the solver decides is W240 or W241 where W242 hinted at its
/// counter, and an iRule's `HTTP::header` after a `HTTP::respond` in an arm
/// that never runs draws no IRULE1201, where one after a live respond does.
#[test]
fn diag_reads_the_decided_loop_header_and_the_dead_respond() {
    let loops = "set n 0\nwhile {$n} {puts \"never runs\"}\nset go 1\nwhile {$go} {puts x}\n\
                 proc p {n} {\n    set i 0\n    while {$i < $n} {puts $i}\n}\n";
    let found = diagnostics_at(loops, "tcl8.6", &["W242"]);
    for expected in [(2, 7, "W240"), (4, 7, "W241"), (7, 11, "W242")] {
        assert!(
            found.contains(&(expected.0, expected.1, expected.2.to_owned())),
            "{expected:?} in {found:?}"
        );
    }
    assert_eq!(
        found.iter().filter(|(_, _, code)| code == "W242").count(),
        1,
        "the decided headers draw no W242: {found:?}"
    );

    let dead = "when HTTP_REQUEST {\n    if {0} { HTTP::respond 200 }\n    \
                HTTP::header insert X-Custom val\n}\n";
    let live = "when HTTP_REQUEST {\n    if {[HTTP::method] eq \"GET\"} { HTTP::respond 200 }\n    \
                HTTP::header insert X-Custom val\n}\n";
    let irules = |source: &str| {
        diagnostics_at(source, "f5-irules", &[])
            .into_iter()
            .filter(|(_, _, code)| code == "IRULE1201")
            .count()
    };
    assert_eq!(irules(dead), 0);
    assert_eq!(irules(live), 1);
}

/// The nested equality program: inside `if {$x eq "a"}`'s arm `x` is `a`, so
/// `if {$x eq "b"}` there never holds. The procedure's argument is a global
/// no call site pins, so the outer test is the only evidence.
const NESTED_EQUALITY: &str = "proc p {x} {\n    if {$x eq \"a\"} {\n        if {$x eq \"b\"} {puts never} else {puts inner}\n    }\n}\nset ::v a\np $::v\nset ::v b\np $::v\n";

/// The nested equality decides through the shipped binary under every
/// release: `tcl diag` reports I230 on the inner condition (line 3), `tcl
/// opt --profile full` drops `puts never`, and both programs print `inner`
/// under every tclsh release on the oracle's path.
#[test]
fn the_nested_equality_decides_through_diag_and_opt() {
    for series in RELEASES {
        let dialect = format!("tcl{series}");
        let found = diagnostics_at(NESTED_EQUALITY, &dialect, &[]);
        assert!(
            found
                .iter()
                .any(|(line, _, code)| *line == 3 && code == "I230"),
            "{dialect}: {found:?}"
        );
        let optimised = statements_of(&opt_under(NESTED_EQUALITY, series));
        assert!(!optimised.contains("puts never"), "{dialect}:\n{optimised}");
    }
    for (series, tclsh) in tclshs_from("8.4") {
        let optimised = statements_of(&opt_under(NESTED_EQUALITY, series));
        for program in [NESTED_EQUALITY, optimised.as_str()] {
            assert_eq!(
                run_tclsh(&tclsh, program),
                Some((true, "inner\n".to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// The eleven loop programs of the interface page (§ *Bounded-loop
/// enumeration*) through the shipped binary: each loop runs to its exit over
/// exact state, so `tcl diag` reports I230 on the branch after it (line 2) and
/// `tcl opt --profile full` keeps only the arm that runs, under every release
/// the analysis names; the original and the optimised program print the same
/// under every tclsh release on the oracle's path.
#[test]
fn the_eleven_loop_witnesses() {
    let witnesses = [
        ("for {set i 0} {$i < 5} {incr i} {}", "$i == 5", true),
        (
            "set t 0; for {set i 0} {$i < 4} {incr i} {incr t $i}",
            "$t == 6",
            true,
        ),
        (
            "for {set i 0} {$i < 10} {incr i} {if {$i == 3} break}",
            "$i == 3",
            true,
        ),
        (
            "set t 0; for {set i 0} {$i < 4} {incr i} {if {$i == 1} continue; incr t}",
            "$t == 3",
            true,
        ),
        ("for {set i 0} {$i < 2.5} {incr i} {}", "$i == 3", true),
        ("set i 0; for {} {$i < 3} {} {incr i 2}", "$i == 4", true),
        (
            "set n 0; for {set i 0} {$i < 3} {incr i} {set i [expr {$i + 1}]; incr n}",
            "$i == 4 && $n == 2",
            true,
        ),
        (
            "catch {for {set i 0} {$i < 5} {incr i} {if {$i == 2} {error x}}}",
            "$i == 2",
            true,
        ),
        ("foreach x {} {}", "[info exists x]", false),
        ("foreach {a b} {1 2 3} {}", "$a == 3 && $b eq \"\"", true),
        (
            "set n 0; foreach x {1 2 3} {if {$x == 2} break; incr n}",
            "$n == 1 && $x == 2",
            true,
        ),
    ];
    let tclshs = tclshs_from("8.4");
    for (program, condition, holds) in witnesses {
        let source = format!("{program}\nif {{{condition}}} {{puts yes}} else {{puts no}}\n");
        let (printed, dropped) = if holds { ("yes", "no") } else { ("no", "yes") };
        for series in RELEASES {
            let dialect = format!("tcl{series}");
            let found = diagnostics_at(&source, &dialect, &[]);
            assert!(
                found
                    .iter()
                    .any(|(line, _, code)| *line == 2 && code == "I230"),
                "{dialect}: {source}{found:?}"
            );
            let optimised = statements_of(&opt_under(&source, series));
            assert!(
                !optimised.contains(&format!("puts {dropped}")),
                "{dialect}: {source}{optimised}"
            );
            for (_, tclsh) in tclshs.iter().filter(|(found, _)| *found == series) {
                for text in [source.as_str(), optimised.as_str()] {
                    assert_eq!(
                        run_tclsh(tclsh, text),
                        Some((true, format!("{printed}\n"))),
                        "tclsh{series}:\n{text}"
                    );
                }
            }
        }
    }
}
