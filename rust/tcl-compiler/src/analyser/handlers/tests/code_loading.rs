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

//! `source` and `package require`, `ifneeded` and `prefer`.

use super::*;

// handle_source_command

#[test]
fn handle_source_command_resolves_a_same_file_constant_variable() {
    // The simplest possible case: a straight-line `set`, zero branches, zero
    // external input, immediately followed by `source $var` — the real
    // corpus's `set p "e.tcl"; source $p` shape, resolved through the
    // same constant-string lattice `rename`'s OLD/NEW words use, via the
    // shared `resolve_dynamic_word`.
    let mut a = Analyser::new();
    let r = a.analyse("namespace eval ::z { set p \"e.tcl\"; source $p }\n", "tcl");
    let target = r
        .source_targets
        .iter()
        .find(|s| s.raw_path == "e.tcl")
        .unwrap_or_else(|| panic!("resolved literal not found: {:?}", r.source_targets));
    assert!(target.is_literal, "{target:?}");
}

#[test]
fn handle_source_command_resolves_a_concatenated_constant_variable() {
    // A multi-token word (`${base}.tcl`, a Var fragment concatenated
    // with a literal) goes through `fold_interpolation_single` rather
    // than the single-token `resolve_const_word` fast path — covered
    // separately since it's a different code path inside
    // `resolve_dynamic_word`.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::z { set base \"helper\"; source ${base}.tcl }\n",
        "tcl",
    );
    let target = r
        .source_targets
        .iter()
        .find(|s| s.raw_path == "helper.tcl")
        .unwrap_or_else(|| panic!("resolved literal not found: {:?}", r.source_targets));
    assert!(target.is_literal, "{target:?}");
}

#[test]
fn handle_source_command_a_dynamic_variable_with_no_known_value_stays_dynamic() {
    // A proc parameter is never constant-tracked (the same limitation
    // `resolve_dynamic_word` already documents for `rename`) — `source
    // $p` here must stay conservatively dynamic rather than guess.
    let mut a = Analyser::new();
    let r = a.analyse("proc f {p} { source $p }\n", "tcl");
    assert_eq!(r.source_targets.len(), 1, "{:?}", r.source_targets);
    assert!(!r.source_targets[0].is_literal, "{:?}", r.source_targets[0]);
}

#[test]
fn handle_source_command_leaves_a_bracket_wrapped_variable_dynamic() {
    // A same-file constant wrapped inside a `[file join ...]` command
    // substitution — the real corpus's `source [file join $edir
    // extra.tcl]` shape — can't be folded by the same-word constant
    // lattice (`fold_interpolation_single` rejects any word containing
    // `[` outright, by design — a command substitution can have
    // arbitrary side effects), and `evaluate_auto_path_expr`'s
    // separate `[info script]`-anchored folder doesn't evaluate `$var`
    // at all.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval ::z { set dir \"lib\"; source [file join $dir extra.tcl] }\n",
        "tcl",
    );
    assert_eq!(r.source_targets.len(), 1, "{:?}", r.source_targets);
    assert!(!r.source_targets[0].is_literal, "{:?}", r.source_targets[0]);
}

/// `conditional_depth` is driven by
/// `Traits::BRANCH_SELECTED_BODY`, so exactly the branch-selected bodies
/// mark a `package require` conditional.
///
/// `if` and `try` bodies are branch-selected: at most one runs, chosen at
/// run time, so nothing inside dominates the code after the command. A
/// `while` body is skippable *and* repeatable — a different question,
/// owned by `control_flow_body_depth` — and does **not** mark the require
/// conditional. A top-level require is unconditional.
#[test]
fn package_require_conditionality_follows_the_branch_selected_body_trait() {
    let conditional_flags = |src: &str| -> Vec<bool> {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, "tcl8.6")
            .package_requires
            .iter()
            .map(|p| p.conditional)
            .collect()
    };
    assert_eq!(conditional_flags("package require Tcl 8.6\n"), vec![false]);
    assert_eq!(
        conditional_flags("if {$x} { package require Tcl 8.6 }\n"),
        vec![true],
    );
    assert_eq!(
        conditional_flags("while {$x} { package require Tcl 8.6 }\n"),
        vec![false],
        "a loop body is skippable-and-repeatable, a different question",
    );
    // `try`'s bodies take the generic body walk at their clause's timing:
    // the protected body is a guarded probe, so it is conditional like
    // `if`'s.
    assert_eq!(
        conditional_flags("try { package require Tcl 8.6 } on error {} {}\n"),
        vec![true],
    );
}

/// The generic body walk raises the branch-selected depth per clause
/// timing (`body_depths`), so a `package require` records the right
/// conditionality wherever in a `try` it sits.
///
/// Clause semantics are C Tcl's (Tcl 9.0.4 `try(n)`, `TclNRTryObjCmd` in
/// `generic/tclCmdMZ.c`): the main body may be cut short by an exception a
/// handler swallows and the `on`/`trap` handlers run only on a match — both
/// branch-selected — while `finally` always runs, so it is not.
#[test]
fn package_require_conditionality_per_try_clause_kind() {
    let conditional_flags = |src: &str| -> Vec<bool> {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, "tcl8.6")
            .package_requires
            .iter()
            .map(|p| p.conditional)
            .collect()
    };
    // TP — the guarded optional-dependency idiom.
    assert_eq!(
        conditional_flags("try { package require Foo } on error {} {}\n"),
        vec![true],
        "a require in the main try body is conditional",
    );
    // TP — an `on` handler body.
    assert_eq!(
        conditional_flags("try { set x 1 } on error {} { package require Foo }\n"),
        vec![true],
        "a require in an `on` handler is conditional",
    );
    // TP — a `trap` handler body.
    assert_eq!(
        conditional_flags("try { set x 1 } trap {POSIX ENOENT} {m o} { package require Foo }\n"),
        vec![true],
        "a require in a `trap` handler is conditional",
    );
    // TN — a `finally` body always runs, so it is not branch-selected.
    assert_eq!(
        conditional_flags("try { set x 1 } finally { package require Foo }\n"),
        vec![false],
        "a `finally` body always runs — the require is unconditional",
    );
    // TN — a top-level require, the control.
    assert_eq!(conditional_flags("package require Foo\n"), vec![false]);
    // FN guard — the depth is restored after the walk, so a require that
    // follows the whole `try` command is still unconditional.
    assert_eq!(
        conditional_flags(
            "try { package require A } on error {} { package require B } finally { package require C }\npackage require D\n"
        ),
        vec![true, true, false, false],
        "main body + handler conditional; finally and the following \
             top-level require unconditional",
    );
    // FP guard — nesting a `try` inside an `if` must not leave the depth
    // stuck: the require after both is still unconditional.
    assert_eq!(
        conditional_flags(
            "set x [gets stdin]\nif {$x} { try { package require A } finally { set y 1 } }\npackage require B\n"
        ),
        vec![true, false],
    );
}

// unknown_proc_info / package require

#[test]
fn analyse_records_package_require() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("package require Tcl 8.6", "tcl");
    assert_eq!(r.package_requires.len(), 1);
    let p = &r.package_requires[0];
    assert_eq!(p.name, "Tcl");
    assert_eq!(p.version.as_deref(), Some("8.6"));
    assert_eq!(p.requirements, ["8.6"]);
    assert!(!p.conditional);
    assert!(!p.control_flow);
    assert!(!p.exact, "no `-exact` word means the ranged requirement");
}

#[test]
fn analyse_records_every_package_requirement_alternative() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("package require Foo 1.0 2.0-2.5", "tcl8.6");
    let p = &r.package_requires[0];
    assert_eq!(p.version.as_deref(), Some("1.0"));
    assert_eq!(p.requirements, ["1.0", "2.0-2.5"]);
}

#[test]
fn analyse_marks_package_requires_in_control_flow() {
    let cases = [
        (
            "set enabled [gets stdin]\nwhile {$enabled} { package require WhileDep 1.0 }",
            "tcl8.6",
            "WhileDep",
        ),
        (
            "set items $argv\nforeach item $items { package require ForeachDep 1.0 }",
            "tcl8.6",
            "ForeachDep",
        ),
        (
            "set items $argv\nlmap item $items { package require LmapDep 1.0 }",
            "tcl8.6",
            "LmapDep",
        ),
        (
            "dict for {key value} {} { package require DictForDep 1.0 }",
            "tcl8.6",
            "DictForDep",
        ),
        (
            "dict map {key value} {} { package require DictMapDep 1.0 }",
            "tcl8.6",
            "DictMapDep",
        ),
        (
            "::tcl::dict::for {key value} {} { package require QualifiedDictForDep 1.0 }",
            "tcl8.6",
            "QualifiedDictForDep",
        ),
        (
            "array for {key value} values { package require ArrayForDep 1.0 }",
            "tcl9.0",
            "ArrayForDep",
        ),
        (
            "set kind [gets stdin]\nswitch $kind a { package require SwitchDep 1.0 }",
            "tcl8.6",
            "SwitchDep",
        ),
        (
            "timerate { package require TimerateDep 1.0 } 10 1",
            "tcl9.0",
            "TimerateDep",
        ),
    ];
    for (source, dialect, name) in cases {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(source, dialect);
        let p = r
            .package_requires
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("missing {name} from {source:?}"));
        assert!(
            p.control_flow,
            "{name} is inside a skippable or repeatable body: {source:?}",
        );
    }
}

#[test]
fn analyse_marks_only_the_repeatable_for_scripts_as_control_flow() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(
        "for {package require InitDep 1.0} {$enabled} \
             {package require NextDep 1.0} {package require BodyDep 1.0}\n\
             package require TailDep 1.0",
        "tcl8.6",
    );
    let flags: Vec<_> = r
        .package_requires
        .iter()
        .map(|p| (p.name.as_str(), p.control_flow))
        .collect();
    assert_eq!(
        flags,
        [
            ("InitDep", false),
            ("NextDep", true),
            ("BodyDep", true),
            ("TailDep", false),
        ],
    );
}

#[test]
fn analyse_records_package_require_exact_flag() {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("package require -exact Tcl 8.6", "tcl");
    let p = &r.package_requires[0];
    assert_eq!(p.name, "Tcl");
    assert_eq!(p.version.as_deref(), Some("8.6"));
    // TP: the flag reaches the record, so the resolver can
    // narrow `8.6` to the degenerate range `8.6-8.6`.
    assert!(p.exact);
    // A lone `-exact` is the option with no package after it — tclsh
    // 8.4–9.0: `wrong # args` (`PkgRequireCore` compares the word with
    // `strcmp(…, "-exact")` whatever follows) — so nothing is required.
    let r = a.analyse("package require -exact", "tcl");
    assert!(r.package_requires.is_empty(), "{:?}", r.package_requires);
    // FP guard — only the exact spelling is the flag: `-e` is not
    // `-exact` (tclsh: `package require -e Tcl` asks for a package named
    // `-e`). The registry's option scan reads a dash word naming no
    // option as a call Tcl rejects, so the layout is unproven and no
    // requirement — exact or not — is recorded for it.
    let r = a.analyse("package require -e 1.0", "tcl");
    assert!(
        !r.package_requires.iter().any(|p| p.exact),
        "{:?}",
        r.package_requires
    );
}

/// `package ifneeded NAME VER SCRIPT` registers a load script; the
/// two-word query form registers nothing.
#[test]
fn analyse_records_package_ifneeded_registrations_only() {
    let mut a = crate::analyser::Analyser::new();
    // TP — the setter form, the shape every `pkgIndex.tcl` writes.
    let r = a.analyse(
        "package ifneeded base64 2.5 [list source [file join $dir base64.tcl]]",
        "tcl",
    );
    assert_eq!(r.package_ifneededs.len(), 1);
    assert_eq!(r.package_ifneededs[0].name, "base64");
    assert_eq!(r.package_ifneededs[0].version, "2.5");
    // TP — a braced script body is equally a registration.
    let r = a.analyse("package ifneeded mylib 1.0 {source lib.tcl}", "tcl");
    assert_eq!(r.package_ifneededs.len(), 1);
    // TN — the two-word form is a *query*; it registers nothing, so a
    // document that merely asks what is registered must not be read as
    // making the package's loading dynamic.
    let r = a.analyse("package ifneeded base64 2.5", "tcl");
    assert_eq!(
        r.package_ifneededs,
        [] as [crate::analyser::types::PackageIfneeded; 0]
    );
}

/// `package prefer latest` is recorded; every other spelling of the
/// subcommand changes no state and is not.
#[test]
fn analyse_records_package_prefer_latest() {
    let mut a = crate::analyser::Analyser::new();
    // TP — the one transition that exists.
    let r = a.analyse("package prefer latest", "tcl");
    assert_eq!(r.package_prefer_latest.len(), 1);
    assert!(!r.package_prefer_latest[0].conditional);
    // TP — inside a guarded branch the record carries `conditional`, so a
    // consumer can refuse to flip the selection rule on it.
    let r = a.analyse("catch {package prefer latest}", "tcl");
    assert_eq!(r.package_prefer_latest.len(), 1);
    assert!(r.package_prefer_latest[0].conditional);
    // TN — `stable` is a no-op from the default and silently ineffective
    // afterwards; the bare form is a query; a dynamic mode word is not
    // guessed at.
    for src in [
        "package prefer stable",
        "package prefer",
        "package prefer $mode",
        "package prefer [mode]",
    ] {
        let r = a.analyse(src, "tcl");
        assert_eq!(r.package_prefer_latest.len(), 0, "{src}");
    }
}
