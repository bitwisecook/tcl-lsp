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

//! Decided branches: flattened conditions, `switch` and `case` arms under each release, and decided loop headers.

use super::*;

/// A whole-variable `switch` subject resolves from the lattice (§ `switch`
/// of the interface page), so program (4)'s flattened form decides per arm: I231
/// on the dead arm's pattern and O107 on its body, beside O112 on the whole
/// statement, which subsumes O107's rewrite when the findings are applied
/// together — so O107 is read from the passes' raw findings and the
/// applied program keeps neither the arm nor the `switch`, printing
/// `always` under 8.4 to 9.1. A `${…}` subject whose name carries a
/// backslash stays `Raw` and decides nothing — no I231 and no O107 — and
/// tclsh prints `hit` before and after the optimiser.
#[test]
fn the_flattened_form_yields_o107() {
    crate::install_surfaces();
    let source = "set acc \"\"; append acc foo; append acc bar\nswitch -- $acc {\n    baz     { puts never }\n    default { puts always }\n}\n";
    let pattern = u32::try_from(source.find("baz").expect("the dead arm")).expect("an offset");
    for dialect in DIALECTS {
        if f5_refuses_top_level(source, dialect) {
            continue;
        }
        let diagnostics = tcl_compiler::analyser::Analyser::new()
            .analyse(source, dialect)
            .diagnostics;
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == DiagCode::I231 && d.span.start() == pattern),
            "{dialect}: I231 on the dead arm's pattern: {diagnostics:#?}"
        );
        let registry = static_context_for(dialect).commands();
        let raw = optimise_raw(source, registry, Some(dialect));
        for code in [DiagCode::O107, DiagCode::O112] {
            assert!(
                raw.iter().any(|o| o.code == code),
                "{dialect}: {code:?} is a finding:\n{raw:#?}"
            );
        }
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            !rewritten.contains("puts never") && !rewritten.contains("switch"),
            "{dialect}: the applied program keeps neither the arm nor the switch:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "always\n");

    let negative = "set {a\\b} baz\nswitch -- ${a\\b} {\n    baz     { puts hit }\n    default { puts miss }\n}\n";
    for dialect in DIALECTS {
        assert!(!reports(negative, dialect, DiagCode::I231), "{dialect}");
        let rewrites = rewrites_of(negative, dialect);
        assert!(
            !rewrites.iter().any(|o| o.code == DiagCode::O107),
            "{dialect}: {rewrites:#?}"
        );
    }
    prints_under_every_release(negative, "hit\n");
}

/// Program (4) — `switch` over the string two appends build — yields O112
/// and I231 on the dead arm in every form. The exact form decides through its
/// dispatch chain, which also drops the arm's body (O107); the `-glob`,
/// `-regexp`, `-nocase` and fall-through forms decide through the selection
/// record the solver makes at the statement, where the subject has the version
/// the statement reads, and report the arm as a selection fact: no block is
/// dropped, O107 does not fire and no O100 hints at a branch. `-nocase` is
/// 8.5's, so a profile that may be 8.4 leaves it alone. The optimised program
/// prints `always` under tclsh 8.4 to 9.1 (`-nocase` from 8.5).
#[test]
fn program_four_yields_o112_and_i231_for_every_form() {
    crate::install_surfaces();
    let build = "set acc \"\"; append acc foo; append acc bar\n";
    let arms = "{\n    baz     { puts never }\n    default { puts always }\n}\n";
    // `baz -` shares `qux`'s body; the `-` is spelled bare, the one spelling
    // every release reads alike.
    let shared = "{\n    baz     -\n    qux     { puts never }\n    default { puts always }\n}\n";
    let forms = [
        (format!("{build}switch -- $acc {arms}"), "8.4", true),
        (format!("{build}switch -glob -- $acc {arms}"), "8.4", false),
        (
            format!("{build}switch -regexp -- $acc {arms}"),
            "8.4",
            false,
        ),
        (
            format!("{build}switch -nocase -- $acc {arms}"),
            "8.5",
            false,
        ),
        (format!("{build}switch -- $acc {shared}"), "8.4", false),
    ];
    for (source, first, flattened) in forms {
        let dead = |word: &str| u32::try_from(source.find(word).expect("an arm")).expect("offset");
        let mut dead_arms = vec![dead("baz")];
        if source.contains("qux") {
            dead_arms.push(dead("qux"));
        }
        for dialect in DIALECTS {
            if f5_refuses_top_level(&source, dialect) {
                continue;
            }
            let has_the_form = first == "8.4" || !matches!(dialect, "tcl8.4" | "f5-irules" | "tcl");
            let diagnostics = tcl_compiler::analyser::Analyser::new()
                .analyse(&source, dialect)
                .diagnostics;
            let registry = static_context_for(dialect).commands();
            let raw = optimise_raw(&source, registry, Some(dialect));
            if !has_the_form {
                assert!(
                    !raw.iter().any(|o| o.code == DiagCode::O112)
                        && !diagnostics.iter().any(|d| d.code == DiagCode::I231),
                    "{dialect}: no selection is made\n{source}"
                );
                continue;
            }
            for &arm in &dead_arms {
                assert!(
                    diagnostics
                        .iter()
                        .any(|d| d.code == DiagCode::I231 && d.span.start() == arm),
                    "{dialect}: I231 on the dead arm at {arm}\n{source}\n{diagnostics:#?}"
                );
            }
            let (rewritten, rewrites) = optimised(&source, dialect);
            assert!(
                rewrites.iter().any(|o| o.code == DiagCode::O112)
                    && !rewritten.contains("puts never")
                    && !rewritten.contains("switch"),
                "{dialect}: O112 leaves `puts always`\n{source}\n{rewritten}\n{rewrites:#?}"
            );
            assert_eq!(
                raw.iter().any(|o| o.code == DiagCode::O107),
                flattened,
                "{dialect}: O107 on the dead body only where the CFG has the arm's block\n{source}\n{raw:#?}"
            );
            if !flattened {
                let unit = unit_of(&source, dialect);
                let top = &unit.top_level;
                assert_eq!(
                    top.sccp.executable_blocks.len(),
                    top.cfg.blocks.len(),
                    "{dialect}: no block is dropped\n{source}"
                );
                let profile = resolve_environment(dialect).analyser_profile();
                let checks =
                    tcl_compiler::compiler_checks::run_all_checks(&unit, registry, Some(profile));
                assert!(
                    checks.iter().all(|check| check.code != DiagCode::O100),
                    "{dialect}: a selection fact hints at no branch\n{source}\n{checks:#?}"
                );
            }
        }
        prints_under_releases_from(&source, "always\n", first);
    }
}

/// A selection fact names an arm no branch leads to, so it folds no
/// condition: the statement's block also ends in the `if`'s branch, and a
/// fact keyed by that block had rewritten `if {$x}` to `if {0}`. The procedure
/// is called with both a true and a false argument, so `x` is no constant.
/// tclsh 8.4 to 9.1 print `A`, `X` and `A` for the program, before and after
/// the optimiser.
#[test]
fn a_selection_fact_folds_no_condition_beside_it() {
    crate::install_surfaces();
    let source = "proc p {x} {\n    set s abc\n    switch -glob -- $s {a* {puts A} b* {puts B}}\n    if {$x} {puts X}\n}\np 1\np 0\n";
    for dialect in DIALECTS {
        let registry = static_context_for(dialect).commands();
        let raw = optimise_raw(source, registry, Some(dialect));
        assert!(
            !raw.iter().any(|o| o.code == DiagCode::O101),
            "{dialect}: no condition folds\n{raw:#?}"
        );
        let (rewritten, _) = optimised(source, dialect);
        assert!(
            rewritten.contains("if {$x}") && !rewritten.contains("switch"),
            "{dialect}: the switch folds, the condition stays\n{rewritten}"
        );
    }
    prints_under_every_release(source, "A\nX\nA\n");
}

/// A loop whose header the solver decides is reported from that fact: W240
/// where the header is false at entry, W241 where it is true at every test
/// and nothing leaves the loop, and either replaces W242's hint that a
/// counter is never modified. Each W240 program prints only `done` under
/// tclsh 8.4 to 9.1, before and after the optimiser: the body never runs.
/// The infinite loops are not run. A header nothing decides keeps W242, and
/// one with an exit the flow graph or the body's text finds draws none.
#[test]
fn a_decided_loop_header_gives_w240_or_w241() {
    crate::install_surfaces();
    let codes = |source: &str, dialect: &str| -> Vec<String> {
        let mut found: Vec<String> = tcl_compiler::analyser::Analyser::new()
            .analyse(source, dialect)
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .filter(|code| matches!(code.as_str(), "W240" | "W241" | "W242"))
            .collect();
        found.sort();
        found
    };
    let never = [
        "set n 0\nwhile {$n} {puts never}\nputs done\n",
        "proc p {} {set n 0; while {$n} {puts never}; return done}\nputs [p]\n",
        "for {set i 0} {$i < 0} {incr i} {puts never}\nputs done\n",
        "set n 0\nwhile 0 {puts never}\nputs done\n",
    ];
    for source in never {
        for dialect in DIALECTS {
            if f5_refuses_top_level(source, dialect) {
                continue;
            }
            assert_eq!(codes(source, dialect), ["W240"], "{dialect}\n{source}");
        }
        prints_under_every_release(source, "done\n");
    }
    let infinite = [
        "set go 1\nwhile {$go} {puts x}\n",
        "proc p {} {set go 1; while {$go} {puts x}}\n",
        "for {set i 0} {$i < 10} {} {puts hi}\n",
        "while 1 {puts x}\n",
    ];
    for source in infinite {
        for dialect in DIALECTS {
            if f5_refuses_top_level(source, dialect) {
                continue;
            }
            assert_eq!(codes(source, dialect), ["W241"], "{dialect}\n{source}");
        }
    }
    let undecided = "proc p {n} {\n    while {$n} {puts x}\n}\n";
    let left =
        "proc p {} {\n    set go 1\n    while {$go} {if {[gets stdin] eq \"q\"} {break}}\n}\n";
    for dialect in DIALECTS {
        assert_eq!(codes(undecided, dialect), ["W242"], "{dialect}");
        assert!(codes(left, dialect).is_empty(), "{dialect}");
    }
}

/// `case` lowers as an opaque glob selection over its own contract: `a*` and
/// a literal pattern each select the first arm, in the one-word form and in
/// the separate-words form with `in` alike — the selection record says so
/// under 8.4, 8.6 and the iRules profile — no I231 claims the live arm is
/// dead, and the optimiser leaves `puts yes`, which tclsh 8.4 to 8.6 print
/// before and after it. From 9.0 there is no `case`: nothing lowers it, so
/// nothing is recorded or folded.
#[test]
fn case_selects_its_glob_arm() {
    crate::install_surfaces();
    for clauses in [
        "in a* {puts yes} default {puts no}",
        "in abc {puts yes} default {puts no}",
        "{a* {puts yes} default {puts no}}",
    ] {
        let source = format!("proc p {{}} {{\n    case abc {clauses}\n}}\np\n");
        for dialect in ["tcl8.4", "tcl8.6", "f5-irules"] {
            let unit = unit_of(&source, dialect);
            let records = &unit
                .procedures
                .get(&fixture_id(&unit.procedures, "::p"))
                .expect("the procedure")
                .sccp
                .selections;
            assert_eq!(records.len(), 1, "{dialect}: {source}");
            assert_eq!(records[0].fact.selected, [Some(0)], "{dialect}: {source}");
            assert!(
                !reports(&source, dialect, DiagCode::I231),
                "{dialect}: {source}"
            );
            let (rewritten, rewrites) = optimised(&source, dialect);
            assert!(
                rewritten.contains("puts yes") && !rewritten.contains("puts no"),
                "{dialect}: {source}\n{rewritten}\n{rewrites:#?}"
            );
        }
        let unit = unit_of(&source, "tcl9.0");
        let function = unit
            .procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("the procedure");
        assert!(function.sccp.selections.is_empty(), "tcl9.0: {source}");
        assert!(
            !function
                .ssa
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .any(|statement| matches!(statement.statement, Statement::Switch { .. })),
            "tcl9.0: {source}"
        );
        for (series, tclsh) in releases_on_path() {
            if !series.starts_with("8.") {
                continue;
            }
            let (rewritten, _) = optimised(&source, &dialect_of(series));
            for program in [source.as_str(), rewritten.as_str()] {
                assert_eq!(
                    run_script(&tclsh, program),
                    Some((true, "yes\n".to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}

/// Each program selects its `hit` arm of a `switch` whose subject and pattern
/// are the same characters spelled two ways: a bare or quoted word is its
/// escapes decoded, a braced word its content, and an element of a braced arm
/// list either. The flattened dispatch compared the subject's spelling — `a\nb`
/// is four characters there — to the decoded pattern, so the analyser called
/// the `hit` arm unreachable (I231) and the optimiser rewrote the program to
/// its `miss` default, where tclsh 8.4 to 9.1 print `hit`.
const SAME_CHARACTERS: [&str; 18] = [
    r#"switch -- a\nb {"a\nb" {puts hit} default {puts miss}}"#,
    r#"switch -exact -- "a\tb" {a\tb {puts hit} default {puts miss}}"#,
    r#"switch a\nb {"a\nb" {puts hit} default {puts miss}}"#,
    r#"switch "a\nb" {a\nb {puts hit} default {puts miss}}"#,
    r#"switch "a\tb" {a\tb {puts hit} default {puts miss}}"#,
    r#"switch a\tb {"a\tb" {puts hit} default {puts miss}}"#,
    r#"switch "a\\b" {{a\b} {puts hit} default {puts miss}}"#,
    r#"switch {a\b} {"a\\b" {puts hit} default {puts miss}}"#,
    "switch \"a\\nb\" {{a\nb} {puts hit} default {puts miss}}",
    "switch {a\nb} {\"a\\nb\" {puts hit} default {puts miss}}",
    "switch {a\\\nb} {{a b} {puts hit} default {puts miss}}",
    r#"switch "a\tb" a\tb {puts hit} default {puts miss}"#,
    r#"switch a\tb "a\tb" {puts hit} default {puts miss}"#,
    r"switch a\$b {a\$b {puts hit} default {puts miss}}",
    r"switch a\[b {a\[b {puts hit} default {puts miss}}",
    "set s \"a\\nb\"\nswitch $s {a\\nb {puts hit} default {puts miss}}",
    r#"switch -glob -- a\nb {"a\nb" {puts hit} default {puts miss}}"#,
    r"switch -glob -- a\$b {a\$b {puts hit} default {puts miss}}",
];

/// A `switch` compares the values of its words however they are spelled: every
/// program of [`SAME_CHARACTERS`] prints `hit` under every release, before and
/// after the optimiser, which keeps the matching arm and nothing else, and no
/// O107 rewrite removes the arm that runs.
#[test]
fn a_switch_compares_the_values_of_its_words_however_they_are_spelled() {
    crate::install_surfaces();
    for source in SAME_CHARACTERS {
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            let (rewritten, _) = optimised(source, dialect);
            assert!(
                rewritten.trim().ends_with("puts hit")
                    && !rewritten.contains("switch")
                    && !rewritten.contains("miss"),
                "{dialect}: {source}\n{rewritten}"
            );
            for dead in rewrites_of(source, dialect)
                .iter()
                .filter(|rewrite| rewrite.code == DiagCode::O107)
            {
                let removed = &source[dead.span.start() as usize..dead.span.end() as usize];
                assert!(
                    !removed.contains("hit"),
                    "{dialect}: O107 removes the arm that runs: {source}\n{removed}"
                );
            }
        }
        prints_under_every_release(source, "hit\n");
    }
}

/// Before 8.5 `switch` reads every leading word that starts with `-` as an
/// option, however many words follow, so a subject holding `-glob` is one:
/// tclsh 8.4 rejects the program with `bad option`, where 8.5 to 9.1 select
/// the `-glob` arm and print `G`. A release that may be 8.4 — `tcl8.4`, a
/// profile that names none — keeps the statement as it is, and from 8.5 the
/// optimiser keeps the arm. `--` ends the run, so the subject is one under
/// every release and the statement folds under all of them; a subject that
/// does not start with `-` is selected through the statement's own record
/// where the flattened chain is not built; and a literal subject is read by
/// its decoded value, so `\x2dglob` is an option as `-glob` is.
#[test]
fn a_subject_a_release_may_read_as_an_option_is_not_folded_before_8_5() {
    crate::install_surfaces();
    let bare = "set x -glob\nswitch $x {-glob {puts G} default {puts D}}\n";
    let escaped = "switch \\x2dglob {-glob {puts G} default {puts D}}\n";
    for source in [bare, escaped] {
        for (series, tclsh) in releases_on_path() {
            let before_85 = series == "8.4";
            let expected = if before_85 {
                (false, String::new())
            } else {
                (true, "G\n".to_owned())
            };
            let (rewritten, _) = optimised(source, &dialect_of(series));
            for program in [source, rewritten.as_str()] {
                assert_eq!(
                    run_script(&tclsh, program),
                    Some(expected.clone()),
                    "tclsh{series}:\n{program}"
                );
            }
            assert_eq!(
                rewritten.contains("switch"),
                before_85,
                "{series}: {rewritten}"
            );
        }
        let (kept, _) = optimised(source, "tk");
        assert!(kept.contains("switch"), "a profile with no release: {kept}");
    }
    let ended = "set x -glob\nswitch -- $x {-glob {puts G} default {puts D}}\n";
    for dialect in ["tcl8.4", "tcl8.6", "tk"] {
        let (rewritten, _) = optimised(ended, dialect);
        assert!(
            rewritten.contains("puts G") && !rewritten.contains("switch"),
            "{dialect}: {rewritten}"
        );
    }
    prints_under_every_release(ended, "G\n");
    let plain = "set x a\nswitch $x {a {puts A} default {puts D}}\n";
    for dialect in ["tcl8.4", "tk", "tcl9.0"] {
        let (rewritten, _) = optimised(plain, dialect);
        assert!(
            rewritten.contains("puts A") && !rewritten.contains("switch"),
            "{dialect}: {rewritten}"
        );
    }
    prints_under_every_release(plain, "A\n");
}

/// With pattern and body words the subject is inside the option scan on every
/// release — 8.5 to 9.1 stop the scan with two words left, and a pattern and
/// its body are two words — so a variable holding `-glob` is an option there
/// too: every tclsh rejects the program below with `extra switch pattern with
/// no body`, where the default arm would print `D`. The statement is kept
/// under every release's profile and under one that names none.
#[test]
fn a_subject_inside_the_scan_of_the_arms_as_words_is_not_folded_on_any_release() {
    crate::install_surfaces();
    let source = "set x -glob\nswitch $x a {puts A} default {puts D}\n";
    for (series, tclsh) in releases_on_path() {
        let (rewritten, _) = optimised(source, &dialect_of(series));
        for program in [source, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((false, String::new())),
                "tclsh{series}:\n{program}"
            );
        }
        assert!(rewritten.contains("switch"), "{series}: {rewritten}");
    }
    let (kept, _) = optimised(source, "tk");
    assert!(kept.contains("switch"), "a profile with no release: {kept}");
}

/// A braced arm list is a list: a bare or quoted element's escapes collapse
/// under the release's grammar, and before 8.6 a `\x` takes every hex digit
/// that follows and keeps the last two, so `a\x41b` is not `aAb` there. tclsh
/// prints `miss` from 8.4 and 8.5 and `hit` from 8.6 for each program, before
/// and after the optimiser, which folded the arm to `hit` under all of them.
#[test]
fn a_braced_arm_list_decodes_its_elements_under_the_releases_escapes() {
    crate::install_surfaces();
    for (series, tclsh) in releases_on_path() {
        let expected = if series < "8.6" { "miss\n" } else { "hit\n" };
        for source in [
            r#"switch -- aAb {"a\x41b" {puts hit} default {puts miss}}"#,
            r"switch -- aAb {a\x41b {puts hit} default {puts miss}}",
            r#"switch -glob -- aAb {"a\x41b" {puts hit} default {puts miss}}"#,
            r"switch -glob -- aAb {a\x41b {puts hit} default {puts miss}}",
        ] {
            let dialect = dialect_of(series);
            let (rewritten, _) = optimised(source, &dialect);
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
