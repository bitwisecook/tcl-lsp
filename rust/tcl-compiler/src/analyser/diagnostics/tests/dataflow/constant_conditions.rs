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

//! I230 constant branches and I231 switch arms, and the calls, callbacks and catch bodies across which no value is kept.

use super::*;

/// I231 on an opaque `switch` reports each arm whose body no member of the
/// proven subject runs, at its pattern, from the unit's `Selected` branch
/// facts: the arm a pattern selects, the arms a `-` body passes through to a
/// running body and the final `default` are not reported; a subject the
/// lattice does not prove, code that cannot run and a form a release does
/// not have report nothing. Each selection is tclsh 8.6's.
#[test]
fn an_opaque_switch_reports_the_arms_it_never_selects() {
    let reported = |source: &str, dialect: &str| -> Vec<u32> {
        let mut starts: Vec<u32> = Analyser::new()
            .analyse(source, dialect)
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::I231)
            .map(|d| d.span.start())
            .collect();
        starts.sort_unstable();
        starts
    };
    let at = |source: &str, words: &[&str]| -> Vec<u32> {
        words
            .iter()
            .map(|word| u32::try_from(source.find(word).expect("an arm")).expect("an offset"))
            .collect()
    };
    let arms = "-glob -- $s {\n a* {puts A}\n b* {puts B}\n c* {puts C}\n default {puts D}\n }";
    let with = |subject: &str| format!("proc p {{}} {{\n set s {subject}\n switch {arms}\n}}\n");
    let selects_a = with("abc");
    assert_eq!(
        reported(&selects_a, "tcl8.6"),
        at(&selects_a, &["b*", "c*"])
    );
    let selects_default = with("zzz");
    assert_eq!(
        reported(&selects_default, "tcl8.6"),
        at(&selects_default, &["a*", "b*", "c*"])
    );

    // `a1 -` passes its body on to `a2`'s: the group runs when either matches.
    let group = |subject: &str| {
        format!(
            "proc p {{}} {{\n set s {subject}\n switch -glob -- $s {{\n a1 - a2 {{puts S}}\n b {{puts B}}\n default {{puts D}}\n }}\n}}\n"
        )
    };
    let runs = group("a2");
    assert_eq!(reported(&runs, "tcl8.6"), at(&runs, &["b {"]));
    let skips = group("zzz");
    assert_eq!(reported(&skips, "tcl8.6"), at(&skips, &["a1", "a2", "b {"]));

    // A finite subject: an arm any member reaches is not reported.
    let finite = "proc p {c} {\n if {$c} {set t a} else {set t b}\n switch -glob -- $t {\n a {puts A}\n b {puts B}\n c {puts C}\n default {puts D}\n }\n}\n";
    assert_eq!(reported(finite, "tcl8.6"), at(finite, &["c {"]));

    // `case` reads its own contract, and is gone from 9.0.
    let case = "proc p {} {\n case abc in a* {puts A} b* {puts B} default {puts D}\n}\n";
    assert_eq!(reported(case, "tcl8.6"), at(case, &["b*"]));
    assert_eq!(reported(case, "tcl9.0"), [] as [u32; 0]);

    // Nothing is proven about a parameter, and dead code is not analysed.
    let unknown = "proc p {s} {\n switch -glob -- $s {a* {puts A} b* {puts B}}\n}\n";
    assert_eq!(reported(unknown, "tcl8.6"), [] as [u32; 0]);
    let dead = "proc p {} {\n return\n switch -glob -- abc {a* {puts A} b* {puts B}}\n}\n";
    assert_eq!(reported(dead, "tcl8.6"), [] as [u32; 0]);

    // A quoted `-` body of the separate-words form reads two ways on 9.1b0,
    // and `abc` selects the arm that has one: no selection there, so nothing
    // is reported; 9.0 reads it by value and falls through into `b*`'s body.
    let quoted = "proc p {} {\n set s abc\n switch -glob -- $s a* \"-\" b* {puts B} c* {puts C} default {puts D}\n}\n";
    assert_eq!(reported(quoted, "tcl9.0"), at(quoted, &["c*"]));
    assert_eq!(reported(quoted, "tcl9.1"), [] as [u32; 0]);
}

/// I231 on an opaque case list names the command the statement is spelled
/// with: a `case` statement's arm is a `Case` arm, a `switch` statement's a
/// `Switch` arm, on the whole-file walk and the per-item walk alike.
#[test]
fn i231_names_the_command_of_the_case_list() {
    let switch = "proc p {} {\n set s abc\n switch -glob -- $s {a* {puts A} b* {puts B} default {puts D}}\n}\n";
    let case = "proc p {} {\n case abc in a* {puts A} b* {puts B} default {puts D}\n}\n";
    assert_eq!(
        i231_messages(switch, "tcl8.6"),
        ["Switch arm 'b*' is never selected; this arm is unreachable"]
    );
    for dialect in ["tcl8.4", "tcl8.6", "f5-irules"] {
        assert_eq!(
            i231_messages(case, dialect),
            ["Case arm 'b*' is never selected; this arm is unreachable"],
            "{dialect}"
        );
    }
    // A statement spelled with its namespace reads as the command it names.
    let qualified = "proc p {} {\n ::case abc in a* {puts A} b* {puts B} default {puts D}\n ::switch -glob -- abc {a* {puts A} b* {puts B}}\n}\n";
    assert_eq!(
        i231_messages(qualified, "tcl8.6"),
        [
            "Case arm 'b*' is never selected; this arm is unreachable",
            "Switch arm 'b*' is never selected; this arm is unreachable"
        ]
    );
}

/// The I231 messages `source` draws under `dialect`, which the whole-file walk
/// and the per-item walk must draw alike.
fn i231_messages(source: &str, dialect: &str) -> Vec<String> {
    let collect = |result: crate::analyser::types::AnalysisResult| -> Vec<String> {
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::I231)
            .map(|d| d.message.clone())
            .collect()
    };
    let whole = collect(Analyser::new().analyse(source, dialect));
    let per_item = collect(Analyser::new().analyse_per_item(source, dialect));
    assert_eq!(whole, per_item, "{dialect}: the two walks agree\n{source}");
    whole
}

/// #2375: a decided `if` inside a switch arm or at the switch's join is an
/// `if`: it is never I231, whatever block the lowering placed it in. tclsh
/// 8.4 to 9.1 print `A` for each program.
#[test]
fn a_decided_if_beside_a_switch_is_never_a_switch_arm() {
    for source in [
        "set x a\nswitch -- $x { a {puts A} b {puts B} }\nif {$x eq \"z\"} {puts never}",
        "set x a\nswitch -- $x { a {puts A; if {$x eq \"z\"} {puts never}} b {puts B} }",
    ] {
        let body = in_proc(source);
        let messages = i231_messages(&body, "tcl8.6");
        assert!(
            messages.iter().all(|m| !m.contains("$x eq")),
            "{body}: {messages:?}"
        );
        let result = Analyser::new().analyse(&body, "tcl8.6");
        let i230: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::I230)
            .map(|d| d.message.as_str())
            .collect();
        assert_eq!(
            i230,
            ["Condition '$x eq \"z\"' is always false; the consequent branch is unreachable"],
            "{body}"
        );
    }
}

/// I231 reads the words of a flattened `switch` by their values: each program
/// selects its `hit` arm whichever way its subject and pattern are spelled, so
/// the one claim is that the default after it is unreachable. The dispatch
/// chain compared the subject's spelling — `a\nb` is four characters there —
/// to the decoded pattern and reported the `hit` arm unreachable. The opaque
/// `-glob` form reads the same words and reports no arm. Each program prints
/// `hit` under tclsh 8.4 to 9.1.
#[test]
fn i231_reads_a_switch_words_by_their_values() {
    let programs = [
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
    ];
    for source in programs {
        for program in [source.to_owned(), in_proc(source)] {
            for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
                let reported = i231_messages(&program, dialect);
                assert!(
                    matches!(reported.as_slice(), [one] if one.contains("is always true here")),
                    "{dialect}: {program}\n{reported:?}"
                );
            }
        }
    }
    // A variable subject is read by the lattice, and is flattened from 8.5.
    let variable = "set s \"a\\nb\"\nswitch $s {a\\nb {puts hit} default {puts miss}}";
    for program in [variable.to_owned(), in_proc(variable)] {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let reported = i231_messages(&program, dialect);
            assert!(
                matches!(reported.as_slice(), [one] if one.contains("is always true here")),
                "{dialect}: {program}\n{reported:?}"
            );
        }
    }
    let glob = r#"switch -glob -- a\nb {"a\nb" {puts hit} default {puts miss}}"#;
    for source in [variable, glob] {
        for program in [source.to_owned(), in_proc(source)] {
            for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
                let reported = i231_messages(&program, dialect);
                assert!(
                    reported.iter().all(|one| !one.contains("always false")),
                    "{dialect}: {program}\n{reported:?}"
                );
            }
        }
    }
    assert_eq!(
        i231_messages(glob, "tcl8.6"),
        [] as [std::string::String; 0]
    );
}

/// Before 8.5 `switch` reads every leading word that starts with `-` as an
/// option, however many words follow, so the subject below is one: tclsh 8.4
/// rejects the program with `bad option`, where 8.5 to 9.1 select the `-glob`
/// arm and print `G`. A release that may be 8.4 — `tcl8.4`, the iRules base, a
/// profile that names no release — reports nothing of the statement; from 8.5
/// the arm is reported as the one selected. `--` ends the run, so the claim is
/// made under every release, and a subject that does not start with `-` keeps
/// the verdict of the statement's own record, which names the arm never
/// selected.
#[test]
fn i231_never_reports_a_switch_over_a_subject_a_release_may_read_as_an_option() {
    let bare = "set x -glob\nswitch $x {-glob {puts G} default {puts D}}\n";
    let ended = "set x -glob\nswitch -- $x {-glob {puts G} default {puts D}}\n";
    let escaped = "switch \\x2dglob {-glob {puts G} default {puts D}}\n";
    let plain = "set x a\nswitch $x {a {puts A} b {puts B} default {puts D}}\n";
    for wrap in [str::to_owned, in_proc] {
        for dialect in ["tcl8.4", "f5-irules", "tk"] {
            let reported = i231_messages(&wrap(bare), dialect);
            assert!(reported.is_empty(), "{dialect}: {reported:?}");
        }
        for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
            let reported = i231_messages(&wrap(bare), dialect);
            assert!(
                matches!(reported.as_slice(), [one] if one.contains("is always true here")),
                "{dialect}: {reported:?}"
            );
        }
        for dialect in ["tcl8.4", "tcl8.6", "tk"] {
            assert_eq!(
                i231_messages(&wrap(ended), dialect).len(),
                1,
                "{dialect}: `--` ends the options"
            );
        }
        assert_eq!(
            i231_messages(&wrap(escaped), "tcl8.4"),
            [] as [std::string::String; 0]
        );
        assert_eq!(i231_messages(&wrap(escaped), "tcl8.6").len(), 1);
        for dialect in ["tcl8.4", "tk"] {
            let reported = i231_messages(&wrap(plain), dialect);
            assert!(
                matches!(reported.as_slice(), [one] if one.contains("'b' is never selected")),
                "{dialect}: {reported:?}"
            );
        }
        // With pattern and body words the subject is inside the scan on every
        // release: tclsh rejects `words`, and the arm a plain value never
        // selects is reported from its record alone.
        let words = "set x -glob\nswitch $x a {puts A} default {puts D}\n";
        let words_plain = "set x a\nswitch $x a {puts A} b {puts B} default {puts D}\n";
        for dialect in ["tcl8.4", "f5-irules", "tk", "tcl8.5", "tcl8.6", "tcl9.0"] {
            let reported = i231_messages(&wrap(words), dialect);
            assert!(reported.is_empty(), "{dialect}: {reported:?}");
            let reported = i231_messages(&wrap(words_plain), dialect);
            assert!(
                matches!(reported.as_slice(), [one] if one.contains("'b' is never selected")),
                "{dialect}: {reported:?}"
            );
        }
    }
}

#[test]
fn analyse_i230_constant_if_branch() {
    // ``proc foo {} { if {1} { puts hi } }`` — the ``if 1``
    // condition is constant, the false branch is unreachable.
    // I230 should fire.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {} { if {1} { puts hi } }", "tcl8.6");
    let i230s: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::I230)
        .collect();
    assert!(
        !i230s.is_empty(),
        "I230 expected for constant ``if 1``; got {:?}",
        r.diagnostics,
    );
    assert!(i230s[0].message.contains("always true"));
}

#[test]
fn analyse_no_i230_for_dynamic_condition() {
    // ``proc foo {x} { if {$x > 0} {} }`` — ``$x > 0`` is
    // not constant; no I230.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {x} { if {$x > 0} { puts hi } }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::I230),
        "I230 must not fire on dynamic condition; got {:?}",
        r.diagnostics,
    );
}

/// The I230 messages `src` draws on the whole-file path and on the per-item
/// path, in emission order.
fn i230_on_both_paths(src: &str) -> (Vec<String>, Vec<String>) {
    let messages = |per_item: bool| -> Vec<String> {
        let mut a = Analyser::new();
        let result = if per_item {
            a.analyse_per_item(src, "tcl8.6")
        } else {
            a.analyse(src, "tcl8.6")
        };
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::I230)
            .map(|d| d.message.clone())
            .collect()
    };
    (messages(false), messages(true))
}

/// No I230 for any of `sources`, on either path.
fn assert_no_i230(sources: &[&str]) {
    for src in sources {
        let (whole, per_item) = i230_on_both_paths(src);
        assert!(whole.is_empty(), "whole file: {src}\n{whole:?}");
        assert_eq!(per_item, whole, "per item: {src}");
    }
}

/// A `switch` the flow graph keeps as one statement writes what its arms
/// write, so the condition after it is no constant: tclsh 8.4 to 9.1 print `b`
/// for `set go 1; switch -glob -- abc { a* { set go 0 } }; if {$go} {puts a}
/// else {puts b}`, not the `a` the earlier value gave. A name no arm writes
/// stays decided.
#[test]
fn i230_never_reports_a_condition_over_a_name_a_switch_arm_may_write() {
    assert_no_i230(&[
        "set go 1\nswitch -glob -- [gets stdin] { q* { set go 0 } }\nif {$go} {puts a} else {puts b}\n",
        "set go 1\nswitch -nocase -- [gets stdin] { q { set go 0 } }\nif {$go} {puts a} else {puts b}\n",
        "set go 1\nswitch -regexp -- [gets stdin] { {^q} { set go 0 } }\nif {$go} {puts a} else {puts b}\n",
        "set go 1\nswitch -glob -- [gets stdin] { x - q* { set go 0 } }\nif {$go} {puts a} else {puts b}\n",
        "set go 1\ncase [gets stdin] in { q* { set go 0 } }\nif {$go} {puts a} else {puts b}\n",
        "proc p {} {\n set go 1\n switch -glob -- [gets stdin] { q* { set go 0 } }\n if {$go} {puts a} else {puts b}\n}\n",
        "proc p {s} {\n set go 1\n switch -glob -- $s { q* { incr go -1 } default { set other 1 } }\n if {$go} {puts a} else {puts b}\n}\n",
        // A command an arm runs that may write any name, and a callee that
        // writes the caller's name through `upvar`.
        "set go 1\nswitch -glob -- [gets stdin] { q* { namespace eval :: {set go 0} } }\nif {$go} {puts a} else {puts b}\n",
        "proc p {s} {\n set go 1\n switch -glob -- $s { q* { dict with d { set go 0 } } }\n if {$go} {puts a} else {puts b}\n}\n",
        "proc zero {v} {upvar 1 $v x; set x 0}\nproc p {s} {\n set go 1\n switch -glob -- $s { q* { zero go } }\n if {$go} {puts a} else {puts b}\n}\n",
        "proc zero {v} {upvar 1 $v x; set x 0}\nset go 1\nswitch -glob -- [gets stdin] { q* { zero go } }\nif {$go} {puts a} else {puts b}\n",
    ]);
    let (whole, per_item) = i230_on_both_paths(
        "proc hit {} {global hits; incr hits}\nproc p {s} {\n set n 5\n switch -glob -- $s { q* { hit } }\n if {$n} {puts a} else {puts b}\n}\n",
    );
    assert_eq!(
        whole.len(),
        1,
        "a global writer leaves a local alone: {whole:?}"
    );
    assert_eq!(per_item, whole);
    let (whole, per_item) = i230_on_both_paths(
        "set go 1\nswitch -glob -- [gets stdin] { q* { set other 0 } }\nif {$go} {puts a} else {puts b}\n",
    );
    assert_eq!(whole.len(), 1, "{whole:?}");
    assert!(whole[0].contains("always true"), "{whole:?}");
    assert_eq!(per_item, whole);
}

/// A callback script the module stores writes the name after the registering
/// code has run: tclsh prints `b` for the variable-trace program, whose write
/// happens inside `set x 1`.
#[test]
fn i230_never_reports_a_condition_over_a_name_a_callback_writes() {
    assert_no_i230(&[
        "set go 1\ntrace add variable x write { set ::go 0 ;# }\nset x 1\nif {$go} {puts a} else {puts b}\n",
        "set go 1\nafter idle {set ::go 0}\nupdate\nif {$go} {puts a} else {puts b}\n",
        "set done 0\nafter 100 { set done 1 }\nvwait done\nif {$done} {puts a} else {puts b}\n",
        "proc tick {} { set ::go 0 }\nset go 1\nafter 100 tick\nupdate\nif {$go} {puts a} else {puts b}\n",
    ]);
}

/// A plain top-level name is the global name a command the module cannot see
/// may write, as it may `::g`, and a procedure's local is in the reach of such
/// a command too: an autoloaded or unknown-handled callee runs `upvar 1` into
/// the frame that called it, on every release (tclsh 8.4 to 9.1 return
/// `changed` from `proc p {} {set x 5; missing; if {$x == 5} {return stale}
/// {return changed}}` where `auto_index(missing)` defines `missing` as `upvar
/// 1 x x; set x 6`). A procedure the module defines that writes no global
/// changes nothing.
#[test]
fn i230_never_reports_a_condition_across_a_call_the_module_cannot_see() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    assert_no_i230(&[
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nfoo\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset ::g 5\nfoo\nif {$::g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nputs [foo]\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nwhile {[foo]} { puts x }\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nswitch -glob -- [gets stdin] { q* { foo } }\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nproc p {} {\n set g 5\n foo\n if {$g} {puts a} else {puts b}\n}\n",
    ]);
    let src = "proc foo {} { puts hi }\nset g 5\nfoo\nif {$g} {puts a} else {puts b}\n";
    let (whole, per_item) = i230_on_both_paths(src);
    assert_eq!(whole.len(), 1, "{src}: {whole:?}");
    assert!(whole[0].contains("always true"), "{src}: {whole:?}");
    assert_eq!(per_item, whole, "{src}");
}

/// A sourced file runs in the frame of the call, so it may write the name the
/// condition reads: a top-level name, and a procedure's local too.
#[test]
fn i230_never_reports_a_condition_across_a_sourced_file() {
    assert_no_i230(&[
        "set g 5\nsource other.tcl\nif {$g} {puts a} else {puts b}\n",
        "proc p {} {\n set g 5\n source other.tcl\n if {$g} {puts a} else {puts b}\n}\n",
        "proc p {s} {\n set g 5\n switch -glob -- $s { q* { source other.tcl } }\n if {$g} {puts a} else {puts b}\n}\n",
    ]);
}

/// A name only an arm of a `switch` the flow graph keeps as one statement
/// sets may be unset after it, so the read draws W210 — as it does after a
/// plain `if` — whatever the option: tclsh raises `can't read "x"` when no
/// arm matches. A name set before the switch draws none.
#[test]
fn a_read_after_an_opaque_switch_only_an_arm_sets_draws_w210() {
    for arm in [
        "-glob -- $s { a* { set x 1 } }",
        "-nocase -- $s { a { set x 1 } }",
        "-regexp -- $s { {^a} { set x 1 } }",
        "-glob -- $s { a* - b* { set x 1 } }",
    ] {
        let src = format!("proc p {{s}} {{\n switch {arm}\n puts $x\n}}\n");
        let found = lifecycle_findings(&src);
        assert_eq!(found.len(), 1, "{src}: {found:?}");
        assert_eq!(found[0].0, DiagCode::W210, "{src}");
    }
    let found = lifecycle_findings(
        "proc p {s} {\n set x 0\n switch -glob -- $s { a* { set x 1 } }\n puts $x\n}\n",
    );
    assert!(found.is_empty(), "{found:?}");
    // The name only a callee an arm calls writes (through `upvar`) is unset
    // when no arm runs, too.
    let src = "proc zero {v} {upvar 1 $v x; set x 0}\nproc p {s} {\n switch -glob -- $s { a* { zero y } }\n puts $y\n}\n";
    let found = lifecycle_findings(src);
    assert_eq!(found.len(), 1, "{src}: {found:?}");
    assert_eq!(found[0].0, DiagCode::W210, "{src}");
}

/// A call to a command the module cannot see inside the body of a `catch`, or
/// through a computed head, may write a plain top-level name as it may `::g`,
/// and a procedure's local through `upvar 1`, and what the body writes on some
/// path is a name the statement may leave as it was: tclsh prints `b` where
/// the `foo` of a sourced file sets `g` to 0, and `5` where the body writes
/// nothing.
#[test]
fn i230_never_reports_a_condition_across_a_catch_body_or_a_computed_head() {
    assert_no_i230(&[
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\ncatch {foo}\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\ncatch {foo} msg\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\ncatch {if {1} {foo}}\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\ncatch {puts [foo]}\nif {$g} {puts a} else {puts b}\n",
        "set g 5\ncatch {source other.tcl}\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nif {[catch {foo}]} {puts bad}\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nset rc [catch {foo} msg]\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nwhile {[catch {if {1} {foo}}]} {break}\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset ::g 5\ncatch {foo}\nif {$::g} {puts a} else {puts b}\n",
        "set cmd [gets stdin]\nset g 5\n$cmd\nif {$g} {puts a} else {puts b}\n",
        "set cmd [gets stdin]\nset g 5\nputs [$cmd]\nif {$g} {puts a} else {puts b}\n",
        "set cmd [gets stdin]\nset g 5\nwhile {[$cmd]} {puts x}\nif {$g} {puts a} else {puts b}\n",
        "set g 5\ncatch { if {[gets stdin] eq {q}} { set g 0 } }\nif {$g} {puts a} else {puts b}\n",
        "set go 1\ncatch { namespace eval :: {set go 0} }\nif {$go} {puts a} else {puts b}\n",
        "proc zero {v} {upvar 1 $v x; set x 0}\nproc p {} {\n set go 1\n catch { if {1} { zero go } }\n if {$go} {puts a} else {puts b}\n}\n",
        "proc p {c} {\n catch { if {$c} { set x 1 } }\n if {[info exists x]} {puts yes} else {puts no}\n}\n",
        "set ::auto_index(foo) {source lib.tcl}\nproc p {} {\n set g 5\n catch { if {1} {foo} }\n if {$g} {puts a} else {puts b}\n}\n",
        "proc p {cmd} {\n set g 5\n $cmd\n if {$g} {puts a} else {puts b}\n}\n",
        "set ::auto_index(foo) {source lib.tcl}\nproc p {} {\n set g 5\n if {[catch {foo}]} {puts bad}\n if {$g} {puts a} else {puts b}\n}\n",
    ]);
    for src in [
        "proc foo {} { puts hi }\nset g 5\ncatch {foo}\nif {$g} {puts a} else {puts b}\n",
        "set g 5\ncatch { set other 1 }\nif {$g} {puts a} else {puts b}\n",
    ] {
        let (whole, per_item) = i230_on_both_paths(src);
        assert_eq!(whole.len(), 1, "{src}: {whole:?}");
        assert!(whole[0].contains("always true"), "{src}: {whole:?}");
        assert_eq!(per_item, whole, "{src}");
    }
}

/// A braced `expr` in value position names its nested commands through the
/// command the CFG retains for the assignment, so a call the module knows
/// writes no computed name and the function's other values stand (#2454). An
/// unseen nested command still may write any name.
#[test]
fn i230_keeps_values_across_a_braced_expr_with_a_known_nested_call() {
    for body in [
        "set n [expr {[string length $s] * 2}]",
        "set n [expr {[string length abc]}]",
        "set n [expr {[llength $s] + 1}]",
    ] {
        let src = format!(
            "proc p {{}} {{\n set s abc\n {body}\n puts $n\n if {{$s eq \"abc\"}} {{puts yes}} else {{puts no}}\n}}\n"
        );
        let (whole, per_item) = i230_on_both_paths(&src);
        assert_eq!(whole.len(), 1, "{src}: {whole:?}");
        assert!(whole[0].contains("always true"), "{src}: {whole:?}");
        assert_eq!(per_item, whole, "{src}");
    }
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    assert_no_i230(&[
        "set ::auto_index(foo) {source lib.tcl}\nproc p {} {\n set g 5\n set n [expr {[foo]}]\n puts $n\n if {$g} {puts a} else {puts b}\n}\n",
    ]);
}

/// A nested call reading an untraced scalar folds in value position: tclsh
/// 8.4-9.1 print `yes` for each (#2454).
#[test]
fn i230_folds_a_nested_call_that_reads_a_variable() {
    for (value, expected) in [
        ("[string length $s]", "3"),
        ("[expr {[string length $s] * 2}]", "6"),
    ] {
        for src in [
            format!(
                "set s abc\nset n {value}\nif {{$n == {expected}}} {{puts yes}} else {{puts no}}\n"
            ),
            format!(
                "proc p {{}} {{\n set s abc\n set n {value}\n if {{$n == {expected}}} {{puts yes}} else {{puts no}}\n}}\n"
            ),
        ] {
            let (whole, per_item) = i230_on_both_paths(&src);
            assert_eq!(whole.len(), 1, "{src}: {whole:?}");
            assert!(whole[0].contains("always true"), "{src}: {whole:?}");
            assert_eq!(per_item, whole, "{src}");
        }
    }
    assert_no_i230(&[
        "set s abc\ntrace add variable s read {apply {args {}}}\nset n [string length $s]\nif {$n == 3} {puts yes} else {puts no}\n",
    ]);
}

/// A command the module cannot see runs inside a body that is no body of the
/// frame the substitution is written in — a lambda's, a `namespace eval` or
/// `uplevel` body, the text a `subst` substitutes, an expression word of a
/// body — and may write a plain top-level name as it may `::g`: tclsh prints
/// `b` where the `foo` of a sourced file sets `g` to 0. A body with nothing
/// unseen in it leaves the condition decided.
#[test]
fn i230_never_reports_a_condition_across_unseen_code_in_a_body_a_substitution_runs() {
    assert_no_i230(&[
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nset x [apply {{} {foo}}]\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nset x [namespace eval ns {foo}]\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nset x [uplevel #0 {foo}]\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nset x [subst {[foo]}]\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nset x [catch {apply {{} {foo}}}]\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nset x [catch {if {[foo]} {puts a}}]\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nputs [apply {{} {foo}}]\nif {$g} {puts a} else {puts b}\n",
        "set ::auto_index(foo) {source lib.tcl}\nset g 5\nif {[apply {{} {foo}}]} {puts x}\nif {$g} {puts a} else {puts b}\n",
        "set lambda [gets stdin]\nset g 5\nset x [apply $lambda]\nif {$g} {puts a} else {puts b}\n",
    ]);
    for src in [
        "set g 5\nset x [apply {{} {set y 1}}]\nif {$g} {puts a} else {puts b}\n",
        "set g 5\nset x [namespace eval ns {set y 1}]\nif {$g} {puts a} else {puts b}\n",
        "set g 5\nset x [uplevel #0 {set y 1}]\nif {$g} {puts a} else {puts b}\n",
        "set g 5\nset x [subst {[set y 1]}]\nif {$g} {puts a} else {puts b}\n",
        "set g 5\nset x [catch {apply {{} {set y 1}}}]\nif {$g} {puts a} else {puts b}\n",
    ] {
        let (whole, per_item) = i230_on_both_paths(src);
        assert_eq!(whole.len(), 1, "{src}: {whole:?}");
        assert!(whole[0].contains("always true"), "{src}: {whole:?}");
        assert_eq!(per_item, whole, "{src}");
    }
}

/// A name only the body of an opaque `catch` sets may be unset after it, so
/// the read draws W210 — as it does after an inlined `catch`, whose exception
/// edge leaves from before the body — and a name set before it draws none.
#[test]
fn a_read_after_an_opaque_catch_only_its_body_sets_draws_w210() {
    let src = "proc p {c} {\n catch { if {$c} { set x 1 } }\n puts $x\n}\n";
    let found = lifecycle_findings(src);
    assert_eq!(found.len(), 1, "{src}: {found:?}");
    assert_eq!(found[0].0, DiagCode::W210, "{src}");
    let found =
        lifecycle_findings("proc p {c} {\n set x 0\n catch { if {$c} { set x 1 } }\n puts $x\n}\n");
    assert!(found.is_empty(), "{found:?}");
    // The result variable the call assigns however the body ends is defined.
    let found =
        lifecycle_findings("proc p {c} {\n catch { if {$c} { set x 1 } } msg\n puts $msg\n}\n");
    assert!(found.is_empty(), "{found:?}");
}

/// #2323: a loop inside a command substitution binds its variables and runs
/// its body in this frame, so neither its loop variable nor a name its body
/// writes keeps the value it held before. tclsh 8.5 to 9.1 print `changed`
/// for each program.
#[test]
fn i230_never_reports_a_condition_over_a_name_a_substituted_loop_writes() {
    assert_no_i230(&[
        "proc q {} {set a old; set r [foreach a {x} {}]; if {$a eq \"old\"} {puts same} else {puts changed}}\nq\n",
        "proc q {} {set x 1; set r [foreach v 1 {incr x}]; if {$x == 1} {puts same} else {puts changed}}\nq\n",
        "proc q {} {set x 1; set r [lmap v {1 2} {incr x}]; if {$x == 1} {puts same} else {puts changed}}\nq\n",
        "proc q {} {set x 1; set r [while {$x < 3} {incr x}]; if {$x == 1} {puts same} else {puts changed}}\nq\n",
    ]);
}
