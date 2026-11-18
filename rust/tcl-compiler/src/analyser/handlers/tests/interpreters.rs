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

//! `interp create` and the isolation of scripts evaluated in a child interpreter.

use super::*;

/// Whether a W123 is reported at the `nth` occurrence of `needle` in `src`.
fn w123_at(src: &str, needle: &str, nth: usize) -> bool {
    let offset = src.match_indices(needle).nth(nth).expect("the needle").0;
    Analyser::new()
        .analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .any(|d| d.code.to_string() == "W123" && d.span.start() as usize == offset)
}

#[test]
fn interp_create_value_words_strip_a_single_braced_path_issue_1025() {
    // TP — `{child}` is one Tcl word naming interpreter `child`, so the
    // value-flow path reaches the same interpreter as the literal name.
    // tclsh8.6.18/9.0.4: `interp eval child p` runs, a top-level `p`
    // raises invalid command name "p".
    let src = "set i [interp create {child}]\n$i eval { proc p {} {} }\n\
               interp eval child { p }\np\n";
    assert!(!w123_at(src, "p }", 0));
    assert!(w123_at(src, "p\n", 0));
}

#[test]
fn interp_create_value_words_keep_a_nested_path_as_one_word_issue_1025() {
    // TP — `{parent child}` is one word (a descent path), not two.
    // tclsh8.6.18/9.0.4: after `interp eval $i {proc p {} {}}`,
    // `interp eval {parent child} p` runs and a top-level `p` raises.
    let src = "interp create parent\nset i [interp create {parent child}]\n\
               interp eval $i { proc p {} {} }\ninterp eval {parent child} { p }\np\n";
    assert!(!w123_at(src, "p }", 0));
    assert!(w123_at(src, "p\n", 0));
}

#[test]
fn interp_create_value_words_unchanged_for_bare_and_flagged_forms_issue_1025() {
    // TN — the shapes that already worked must be byte-for-byte
    // unchanged by the switch to list parsing.
    assert_eq!(
        elements("[interp create -safe -- name]"),
        Some(vec![
            "interp".into(),
            "create".into(),
            "-safe".into(),
            "--".into(),
            "name".into()
        ])
    );
    assert_eq!(
        elements("[interp create]"),
        Some(vec!["interp".into(), "create".into()])
    );
    // Only one whole bracketed substitution is a call.
    assert_eq!(elements("interp create child"), None);
}

#[test]
fn interp_create_value_words_reject_an_unmatched_brace_issue_1025() {
    // A malformed (mid-edit) path: tclsh8.6.18/9.0.4 reject the script
    // (missing close-brace) and create no interpreter, so no auto-named
    // phantom may back a later `$i eval`: nothing it defines exists.
    for src in [
        "set i [interp create {child]\n$i eval { proc p {} {} }\n",
        "set i [interp create good {child]\n$i eval { proc p {} {} }\n",
    ] {
        let r = Analyser::new().analyse(src, "tcl8.6");
        assert!(
            !r.all_procs.keys().any(|k| k.ends_with("::p")),
            "{src}: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }
}

// `interp create` — the registry's `InterpreterTransition::Create`,
// whose option scan mirrors tclInterp.c (`create_reads_the_c_option_scan`
// in `tcl-registry`'s `interp.rs` pins every row against tclsh 8.4–9.1).

/// Analyse `interp create WORDS…` under Tcl 8.6 and report whether `path`
/// was recorded, and as safe.
fn created_by(words: &[&str], path: &str) -> Option<bool> {
    let a = analysed(&format!("interp create {}\n", words.join(" ")));
    let key = a.qualified_interp_key(path);
    a.interpreters.get(&key).map(|state| state.safe)
}

#[test]
fn interp_create_flags_are_still_scanned_after_the_path() {
    // C Tcl examines every word and treats a `-` word as a flag until
    // `--`, whether or not the path has been read. tclsh8.6/9.0:
    // `interp create x -safe` yields `interp issafe x` == 1. Stopping at
    // the path recorded `x` as an *unsafe* interpreter.
    assert_eq!(created_by(&["x", "-safe"], "x"), Some(true));
    assert_eq!(created_by(&["-safe", "y"], "y"), Some(true));
    // `-safe` matches by unique prefix.
    assert_eq!(created_by(&["-s", "x"], "x"), Some(true));
}

#[test]
fn interp_create_double_dash_ends_flag_parsing() {
    // tclsh8.6/9.0: `interp create -safe -- z` creates a safe `z`, and
    // `interp create -- -safe` creates an *unsafe* interpreter whose
    // path is the literal `-safe`.
    assert_eq!(created_by(&["-safe", "--", "z"], "z"), Some(true));
    assert_eq!(created_by(&["--", "-safe"], "-safe"), Some(false));
}

#[test]
fn interp_create_two_path_words_record_no_path() {
    // tclsh8.6/9.0: `interp create a b` is `wrong # args`, and `interp
    // create n -bogus` a bad option. No interpreter is created, so
    // recording either word would bind a name that does not exist.
    assert_eq!(created_by(&["a", "b"], "a"), None);
    assert_eq!(created_by(&["a", "b"], "b"), None);
    assert_eq!(created_by(&["n", "-bogus"], "n"), None);
    assert_eq!(
        created_by(&["--", "x", "-safe"], "x"),
        None,
        "after `--` a second `-` word is a path word, so this is the error shape too",
    );
}

#[test]
fn interp_create_bare_and_flag_only_forms_are_unchanged() {
    // TN control: a named child is recorded; an auto-named one has no
    // name this file can reference, so nothing is.
    assert_eq!(created_by(&["child"], "child"), Some(false));
    let a = analysed("interp create -safe\n");
    assert!(a.interpreters.is_empty());
    // A computed path makes interpreter existence unknowable.
    let a = analysed("set name [gets stdin]\ninterp create $name\n");
    assert!(a.interpreters.is_empty());
    assert!(a.dynamic_interp_ops);
}

/// `interp eval child { proc foo }` runs in a child interpreter, so `foo`
/// is isolated from the parent's `::foo` — the two stay distinct commands,
/// so a parent rename of `::foo` can never reach the child body.
#[test]
fn interp_eval_child_isolates_definitions_from_the_parent() {
    // Runnable Tcl (tclsh 9.0.4): the child is created before the
    // eval.  The child's `foo` homes under the synthetic
    // `@interp@child` domain — unrepresentable as a real namespace,
    // so a parent namespace literally named `child` can never
    // collide with the interpreter's definitions (the
    // child-global-vs-parent-namespace identity split).
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create child\nproc foo {} {}\ninterp eval child { proc foo {} {} }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(r.all_procs.contains_key("::foo"), "parent proc: {keys:?}");
    assert!(
        r.all_procs.contains_key("::@interp@child::foo"),
        "child proc isolated under the interpreter domain: {keys:?}",
    );
    assert!(
        !r.all_procs.contains_key("::child::foo"),
        "the child's global is not the parent namespace `::child`: {keys:?}",
    );
}

/// `sandbox eval { proc foo }` via the interpreter's own object command
/// (`interp create sandbox` binds `sandbox` itself as a callable
/// command) isolates definitions from the parent exactly like the
/// literal `interp eval sandbox { … }` form above — the far more common
/// real-world spelling (`docstrip_util.tcl`'s `$c eval {...}`, reduced to
/// a literal name here so it resolves statically).
#[test]
fn interp_handle_eval_isolates_definitions_from_the_parent() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create sandbox\nproc foo {} {}\nsandbox eval { proc foo {} {} }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(r.all_procs.contains_key("::foo"), "parent proc: {keys:?}");
    assert!(
        r.all_procs.contains_key("::@interp@sandbox::foo"),
        "child proc isolated under the interpreter domain: {keys:?}",
    );
}

#[test]
fn renamed_away_interp_handle_reused_as_a_proc_is_not_treated_as_interp_eval() {
    // TP: `self.interpreters` is only cleared by `interp delete`, so a plain
    // `rename sandbox {}` (deleting the interpreter's own object
    // command — confirmed against tclsh9.0: afterwards `info commands
    // sandbox` is empty and the child interpreter is only reachable
    // through whatever name, if any, it was renamed *to*) left a stale
    // `sandbox` entry. A later, wholly unrelated `proc sandbox {sub
    // body} {...}` reusing the freed name would then have any
    // `sandbox eval …` call misidentified as isolated interpreter-eval
    // (walking the literal second argument as a *script*) instead of an
    // ordinary two-arg proc call — confirmed against tclsh9.0 that the
    // real dispatch reaches the new proc, never the (now nameless,
    // orphaned) interpreter.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create sandbox\n\
             rename sandbox {}\n\
             proc sandbox {sub body} {}\n\
             sandbox eval { proc shouldNotBeIsolated {} {} }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        !r.all_procs
            .contains_key("::@interp@sandbox::shouldNotBeIsolated"),
        "the call must not be isolated into the (deleted) interpreter's \
             domain once `sandbox` has been renamed away and reused: {keys:?}",
    );
}

#[test]
fn renamed_interp_handle_still_isolates_under_its_new_name() {
    // TP — a rename that moves the interpreter handle (`NEW` non-empty)
    // keeps the child interpreter. tclsh8.6.18/9.0.4: after
    // `interp create sandbox; rename sandbox t; t eval {proc p {} {}}`,
    // `interp slaves` answers `sandbox` and `interp eval sandbox
    // {info procs p}` finds `p`: the command moved, the interpreter
    // keeps its path, so the body homes to the `sandbox` domain.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create sandbox\n\
             rename sandbox t\n\
             t eval { proc shouldBeIsolated {} {} }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs
            .contains_key("::@interp@sandbox::shouldBeIsolated"),
        "the renamed handle must still isolate into the interpreter it \
             names, whose path stays `sandbox`: {keys:?}",
    );
}

#[test]
fn interp_handle_eval_and_literal_interp_eval_agree_on_the_same_domain() {
    // The two spellings of the same operation must home under the
    // *same* synthetic domain — the analyser can't tell them apart at
    // run time (both dispatch on the identical interpreter object
    // command), so it must not tell them apart statically either.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create sandbox\ninterp eval sandbox { proc viaLiteral {} {} }\n\
             sandbox eval { proc viaHandle {} {} }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs.contains_key("::@interp@sandbox::viaLiteral"),
        "{keys:?}"
    );
    assert!(
        r.all_procs.contains_key("::@interp@sandbox::viaHandle"),
        "{keys:?}"
    );
}

#[test]
fn two_interp_handles_never_cross_contaminate_same_named_procs() {
    // TP: two
    // separate safe child interpreters, each independently defining
    // their own same-named `helper`, must never merge — a call inside
    // one script must resolve to *that* interpreter's helper, never
    // the other's.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create sandboxA\ninterp create sandboxB\n\
             interp eval sandboxA { proc helper {} { return A } }\n\
             sandboxB eval { proc helper {} { return B } }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs.contains_key("::@interp@sandboxA::helper"),
        "{keys:?}"
    );
    assert!(
        r.all_procs.contains_key("::@interp@sandboxB::helper"),
        "{keys:?}"
    );
    // Each interpreter's own `helper` is a genuinely distinct
    // ProcDef, not the same entry aliased twice.
    assert_ne!(
        r.all_procs["::@interp@sandboxA::helper"].body_span,
        r.all_procs["::@interp@sandboxB::helper"].body_span,
    );
}

#[test]
fn untracked_bareword_eval_is_not_isolated() {
    // TN / FN guard — a bareword shaped exactly like the tracked-handle
    // idiom (`NAME eval { script }`) but never created via `interp
    // create` must fall through to the generic dispatch untouched: no
    // isolated scope, no crash.
    let mut a = Analyser::new();
    let r = a.analyse("untracked eval { proc foo {} {} }\n", "tcl8.6");
    assert!(
        !r.all_procs.keys().any(|k| k.contains("@interp@")),
        "an untracked head must never open an interpreter domain: {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn ordinary_proc_with_eval_as_sole_argument_is_untouched() {
    // FP guard — an unrelated proc whose single argument merely happens
    // to be the literal text `eval` must resolve as an ordinary call,
    // never trip the arity/shape check into isolating anything.
    let mut a = Analyser::new();
    let r = a.analyse("proc foo {a} { return $a }\nfoo eval\n", "tcl8.6");
    assert!(
        !r.all_procs.keys().any(|k| k.contains("@interp@")),
        "a 1-arg call must never be treated as NAME eval SCRIPT: {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn interp_eval_into_uncreated_interp_warns_and_multiword_stays_isolated_945() {
    // `interp eval ghost { … }` with no `interp create ghost` anywhere
    // raises `could not find interpreter` at run time — W140.
    let mut a = Analyser::new();
    let r = a.analyse("interp eval ghost { proc foo {} {} }\n", "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W140),
        "eval into a never-created interpreter warns: {:?}",
        r.diagnostics
    );
    // A dynamic create makes existence unknowable — W140 abstains.
    let mut a2 = Analyser::new();
    let r2 = a2.analyse(
        "interp create $name\ninterp eval ghost { proc foo {} {} }\n",
        "tcl8.6",
    );
    assert!(
        !r2.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W140),
        "dynamic interp creation abstains: {:?}",
        r2.diagnostics
    );
    // Multi-word scripts concatenate at run time — the words must not
    // be analysed in the *parent* scope, nor
    // walked per-word (commands span word boundaries).
    let mut a3 = Analyser::new();
    let r3 = a3.analyse(
        "interp create c\ninterp eval c {proc leak {} {}} {puts hi}\n",
        "tcl8.6",
    );
    assert!(
        !r3.all_procs.contains_key("::leak"),
        "multi-word eval must not leak definitions into the parent: {:?}",
        r3.all_procs.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn child_created_in_a_namespace_proc_behind_a_glob_import_is_not_w140_2289() {
    let source = "package require tcltest\n\
             namespace eval vl {\n\
             namespace import ::tcltest::*\n\
             proc New {} {\n\
             catch {interp delete c}\n\
             interp create c\n\
             interp eval c [list set auto_path $::auto_path]\n\
             }\n\
             }\n\
             interp eval c {set x 1}\n";
    let r = Analyser::new().analyse(source, "tcl8.6");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W140),
        "{:?}",
        r.diagnostics
    );
}

#[test]
fn deleted_and_recreated_interp_is_a_fresh_domain_945() {
    // C: `interp delete s; interp create s` starts with an empty
    // command table — the first lifetime's definitions are gone.  The
    // epoch-stamped
    // domain keeps the two lifetimes apart: the first eval's `foo`
    // homes under `@interp@s`, the recreated interpreter's under
    // `@interp@s#1`, and they never merge.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create s\ninterp eval s { proc foo {} {} }\n\
             interp delete s\ninterp create s\n\
             interp eval s { proc bar {} {} }\n",
        "tcl8.6",
    );
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs.contains_key("::@interp@s::foo"),
        "first lifetime's proc: {keys:?}"
    );
    assert!(
        r.all_procs.contains_key("::@interp@s#1::bar"),
        "recreated interpreter is a fresh domain: {keys:?}"
    );
    assert!(
        !r.all_procs.contains_key("::@interp@s::bar"),
        "the recreated child never merges with its predecessor: {keys:?}"
    );
}

#[test]
fn nested_interp_ops_qualify_against_the_enclosing_child_945() {
    // `interp create t` inside `interp eval s {…}` creates `s t` —
    // paths are relative to the *current* interpreter, so a top-level
    // `interp eval {s t} {…}` reaches the grandchild without W140 and
    // its definitions home under the composed domain.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp create s\n\
             interp eval s { interp create t }\n\
             interp eval {s t} { proc deep {} {} }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W140),
        "the nested create makes `s t` exist: {:?}",
        r.diagnostics
    );
    assert!(
        r.all_procs.contains_key("::@interp@s t::deep"),
        "grandchild definitions home under the composed domain: {:?}",
        r.all_procs.keys().collect::<Vec<_>>()
    );
}

#[test]
fn cross_domain_interp_alias_links_child_calls_to_the_parent_target_945() {
    // `interp alias s helper {} ::parent_helper` makes `helper`
    // callable *inside the child* running the parent's proc — the
    // alias deliberately crosses interpreter domains while
    // definitions do not.  The child-side call
    // resolves through the alias link to the parent target.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc parent_helper {} {}\ninterp create s\n\
             interp alias s helper {} parent_helper\n\
             interp eval s { helper }\n",
        "tcl8.6",
    );
    let alias = r
        .command_aliases
        .get("::@interp@s::helper")
        .unwrap_or_else(|| {
            panic!(
                "child-domain alias recorded: {:?}",
                r.command_aliases.keys().collect::<Vec<_>>()
            )
        });
    assert_eq!(
        alias.target, "::parent_helper",
        "the alias target resolves in the parent domain",
    );
}

/// `interp eval {} { proc foo }` targets the *current* interpreter, so
/// `foo` is the parent's `::foo` (no isolation, no synthetic namespace).
#[test]
fn interp_eval_empty_path_is_the_current_interpreter() {
    let mut a = Analyser::new();
    let r = a.analyse("interp eval {} { proc foo {} {} }\n", "tcl8.6");
    let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
    assert!(
        r.all_procs.contains_key("::foo"),
        "current-interp proc: {keys:?}"
    );
    assert!(
        !keys.iter().any(|k| k.contains("::::")),
        "an empty path must not open a synthetic namespace: {keys:?}",
    );
}
