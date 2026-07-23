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

//! End-to-end coverage for the issue #945 resolution-model follow-up:
//! flow-sensitive constant dispatch with writable provenance (faults 1–2),
//! multi-seed source views (fault 3), `TclOO` export visibility + dispatch
//! entry + per-object binding identity (faults 4–6), the interpreter
//! domain model (faults 7–8), and probe references (fault 9).
//!
//! Rename outputs are **executed** under a real tclsh where one is
//! available (`TCL_LSP_TCLSH`, else `PATH` probes) — the issue's
//! validation bar is behaviour, not edit ranges.

use serde_json::Value;

use crate::common::helpers::{hover_text, locations, rename_edits, start_lines};
use crate::common::{Lsp, unique_uri};

/// Find a tclsh to execute transformed rename output under; `None` skips
/// the execution leg (mirrors `command_resolution_conformance`).
fn find_tclsh() -> Option<std::path::PathBuf> {
    if let Ok(explicit) = std::env::var("TCL_LSP_TCLSH") {
        let p = std::path::PathBuf::from(explicit);
        if p.exists() {
            return Some(p);
        }
    }
    for name in ["tclsh9.0", "tclsh8.6", "tclsh"] {
        if let Ok(out) = std::process::Command::new(name).arg("--version").output()
            && (out.status.success() || !out.stderr.is_empty() || !out.stdout.is_empty())
        {
            return Some(std::path::PathBuf::from(name));
        }
    }
    None
}

/// Apply an LSP edit array (single-line edits) to `src`.
fn apply_lsp_edits(src: &str, edits: &[Value]) -> String {
    let mut lines: Vec<String> = src.lines().map(str::to_owned).collect();
    let mut sorted: Vec<&Value> = edits.iter().collect();
    let pos = |e: &Value, key: &str| -> (usize, usize) {
        let p = &e["range"][key];
        (
            usize::try_from(p["line"].as_u64().unwrap()).unwrap(),
            usize::try_from(p["character"].as_u64().unwrap()).unwrap(),
        )
    };
    sorted.sort_by_key(|e| std::cmp::Reverse(pos(e, "start")));
    for e in sorted {
        let (sl, sc) = pos(e, "start");
        let (el, ec) = pos(e, "end");
        assert_eq!(sl, el, "single-line edits expected in these fixtures");
        lines[sl].replace_range(sc..ec, e["newText"].as_str().unwrap());
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Run `script` under the discovered tclsh, returning trimmed stdout;
/// panics on a non-zero exit.  `None` when no tclsh is installed.
fn run_tclsh(script: &str) -> Option<String> {
    use std::io::Write;
    let tclsh = find_tclsh()?;
    let mut child = std::process::Command::new(&tclsh)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn tclsh");
    child
        .stdin
        .as_mut()
        .expect("tclsh stdin")
        .write_all(script.as_bytes())
        .expect("write script");
    let out = child.wait_with_output().expect("tclsh run");
    assert!(
        out.status.success(),
        "script must execute cleanly under {}:\n{script}\nstderr: {}",
        tclsh.display(),
        String::from_utf8_lossy(&out.stderr),
    );
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// -- faults 1–2: flow-sensitive constant dispatch with provenance --------

#[test]
fn const_dispatch_rename_rewrites_the_defining_literal_and_executes_945() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc target {} { return hi }\nset cmd target\n$cmd\ntarget\n";
    lsp.open_ready(&uri, src);
    let result = lsp.rename(&uri, 0, 6, "renamed");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    let renamed_src = apply_lsp_edits(src, &for_uri);
    assert!(
        renamed_src.contains("set cmd renamed"),
        "the defining literal follows the rename:\n{renamed_src}"
    );
    assert!(
        renamed_src.contains("$cmd"),
        "the `$cmd` head is never rewritten:\n{renamed_src}"
    );
    // The issue's validation bar: the transformed output must EXECUTE —
    // the old edit set left `set cmd target` stale and died with
    // `invalid command name "target"` under tclsh 9.0.4.
    let script = format!("{renamed_src}puts [$cmd]\n");
    if let Some(out) = run_tclsh(&script) {
        assert_eq!(out, "hi", "the dispatch still reaches the renamed proc");
    } else {
        eprintln!("skipping tclsh execution leg: no tclsh (set TCL_LSP_TCLSH)");
    }
}

// Issue #1009 — the constant-`$cmd` dispatch settlement resolved through a
// proc/class/alias/rename target renamed or deleted away with no later
// re-establishment, the same root cause #973/#1006/#1007 fixed for the
// bareword-call paths. Confirmed against tclsh 8.6.14 that a deleted
// proc's dispatch fails "invalid command name" — the LSP must not still
// treat the dead name as a live reference.

#[test]
fn const_dispatch_draws_no_reference_to_a_deleted_proc_1009() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc target {} { return hi }\nrename target {}\nset cmd target\n$cmd\n";
    lsp.open_ready(&uri, src);
    let refs = start_lines(&lsp.references(&uri, 0, 6, true));
    assert!(
        !refs.contains(&2) && !refs.contains(&3),
        "a proc deleted with no re-establishment must draw no reference from \
         the dead $cmd dispatch (`set cmd target` / `$cmd`): {refs:?}"
    );
}

#[test]
fn const_dispatch_still_references_a_proc_reestablished_after_deletion_1009() {
    // FP guard: a fresh `proc target` after the deletion re-establishes the
    // name — the dispatch must still resolve and reference it normally.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc target {} { return hi }\nrename target {}\nproc target {} { return bye }\nset cmd target\n$cmd\n";
    lsp.open_ready(&uri, src);
    let refs = start_lines(&lsp.references(&uri, 2, 6, true));
    assert!(
        refs.contains(&3),
        "the re-established proc must still be referenced by the dispatch: {refs:?}"
    );
}

// Issue #1009, Codex PR #1014 review follow-up — two confirmed false
// positives found in review after the original #1009/#1006/#973 fixes
// landed:
//
// 1. `scope.rs`'s `finalise_invocation_resolutions` picked between a local
//    and a global candidate using a *file-end-only* deletion check, so a
//    namespaced local call textually before a later unconditional
//    deletion wrongly lost to the global candidate.
// 2. `fact_live_for_call` treated *any* call inside a proc/class body as
//    automatically after every top-level deletion, drawing a spurious
//    W123 even when the enclosing definition's own top-level invocation
//    demonstrably ran before that deletion.
//
// Both confirmed against tclsh 8.6.14 (see the unit tests alongside
// `finalise_invocation_resolutions` and `fact_live_for_call` for the exact
// repros run under a real interpreter).

// These two use `references` rather than `definition`: go-to-definition's
// own call-site resolver (`tcl-lsp-core::definition::resolve_called_proc`)
// is a namespace-visibility check with no deletion tracking of its own, so
// it always prefers a namespace-visible local proc regardless of a later
// `rename` — it does not exercise `finalise_invocation_resolutions`'s fix.
// `references` does: it matches a call site against a definition via
// `resolved_qualified_name` (`tcl-lsp-core::references`), the exact field
// this fix corrects.

#[test]
fn local_call_before_later_deletion_is_a_reference_to_the_local_definition_codex_1009() {
    // TP: `foo::caller`'s own top-level invocation (line 5) runs before
    // `rename foo::bar {}` (line 6), so the `bar` call inside its body
    // must be a reference to the *local* `::foo::bar` (line 2), not the
    // global `bar` (line 0).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc caller {} { return [bar] }\n}\nfoo::caller\nrename foo::bar {}\n";
    lsp.open_ready(&uri, src);
    let local_refs = start_lines(&lsp.references(&uri, 2, 9, false));
    assert!(
        local_refs.contains(&3),
        "the call before the deletion must reference the local definition: {local_refs:?}"
    );
    let global_refs = start_lines(&lsp.references(&uri, 0, 5, false));
    assert!(
        !global_refs.contains(&3),
        "the call must not also reference the global definition: {global_refs:?}"
    );
}

#[test]
fn local_call_after_deletion_is_a_reference_to_the_global_definition_issue_973() {
    // FN guard / regression: `foo::caller` is only ever invoked (line 6,
    // after `rename foo::bar {}` on line 3) — the local `bar` is genuinely
    // gone by the time the call executes, so it must be a reference to
    // the global `bar` (line 0), not the local one (line 2). Guards
    // against #973's original fix regressing.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    rename foo::bar {}\n    proc caller {} { return [bar] }\n}\nfoo::caller\n";
    lsp.open_ready(&uri, src);
    // Driven by `resolved_qualified_name` resolving to the global `::bar`:
    // before this fix it stayed `::foo::bar`, and `invocation_references_named`
    // would not have matched this query at all (`call_ns` ("foo") differs
    // from the global definition's own namespace ("")).
    let global_refs = start_lines(&lsp.references(&uri, 0, 5, false));
    assert!(
        global_refs.contains(&4),
        "a call genuinely after the deletion must reference the global definition: {global_refs:?}"
    );
}

#[test]
fn body_call_before_later_deletion_draws_no_w123_codex_1009() {
    // FP guard (the confirmed regression): `caller`'s own top-level
    // invocation runs before `rename helper {}`, so the `helper` call
    // inside its body must draw no W123 — confirmed against tclsh 8.6.14
    // (the script runs to completion).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {} {}\nproc caller {} { helper }\ncaller\nrename helper {}\n";
    lsp.open_ready(&uri, src);
    let diags = lsp.await_diagnostics(&uri);
    assert!(
        !diags
            .iter()
            .any(|d| d.get("code").and_then(Value::as_str) == Some("W123")),
        "a body call before a later deletion must not draw W123: {diags:?}"
    );
}

#[test]
fn body_call_deleted_before_definition_still_draws_w123_issue_973() {
    // TP regression: `helper` is deleted before `caller` is even defined,
    // with no re-establishment — the body call must still draw W123
    // (confirmed against tclsh 8.6.14: `invalid command name "helper"`).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {} {}\nrename helper {}\nproc caller {} { helper }\ncaller\n";
    lsp.open_ready(&uri, src);
    let diags = lsp.await_diagnostics(&uri);
    assert!(
        diags
            .iter()
            .any(|d| d.get("code").and_then(Value::as_str) == Some("W123")),
        "helper was deleted before caller was even defined: {diags:?}"
    );
}

#[test]
fn branch_joined_dispatch_definition_offers_both_targets_945() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc foo {} {return one}\nproc bar {} {return two}\nset cmd foo\nif {$runtime} {\n    set cmd bar\n}\n$cmd\n";
    lsp.open_ready(&uri, src);
    // References from each proc's declaration include the `$cmd` join site
    // (line 6) and that proc's own contributing literal.
    let foo_refs = start_lines(&lsp.references(&uri, 0, 6, true));
    assert!(
        foo_refs.contains(&2) && foo_refs.contains(&6),
        "foo: literal (2) + join dispatch (6) expected: {foo_refs:?}"
    );
    let bar_refs = start_lines(&lsp.references(&uri, 1, 6, true));
    assert!(
        bar_refs.contains(&4) && bar_refs.contains(&6),
        "bar: literal (4) + join dispatch (6) expected: {bar_refs:?}"
    );
    // Renaming `bar` rewrites only bar's contributing literal; foo's stays,
    // and the renamed script still dispatches correctly on BOTH paths.
    let result = lsp.rename(&uri, 1, 6, "baz");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    let renamed_src = apply_lsp_edits(src, &for_uri);
    assert!(
        renamed_src.contains("set cmd foo") && renamed_src.contains("set cmd baz"),
        "only bar's literal follows its rename:\n{renamed_src}"
    );
    for (runtime, want) in [("0", "one"), ("1", "two")] {
        let script = format!("set runtime {runtime}\n{renamed_src}puts [$cmd]\n");
        if let Some(out) = run_tclsh(&script) {
            assert_eq!(out, want, "runtime={runtime} dispatch after rename");
        }
    }
}

// -- fault 3: multi-seed source views ------------------------------------

#[test]
fn multi_seeded_source_declaration_unions_every_view_945() {
    let mut lsp = Lsp::tcl();
    let b = unique_uri("tcl");
    let b_name = b.rsplit('/').next().unwrap().to_owned();
    lsp.open_ready(&b, "proc helper {} {namespace current}\n");
    let a = unique_uri("tcl");
    let a_src = format!(
        "namespace eval ::x {{ source {b_name} }}\nnamespace eval ::y {{ source {b_name} }}\n::x::helper\n::y::helper\n"
    );
    lsp.open_ready(&a, &a_src);
    // Declaration-side references from b.tcl reach BOTH runtime identities'
    // callers (tclsh 9.0.4: both ::x::helper and ::y::helper exist).
    let refs = locations(&lsp.references(&b, 0, 6, false));
    let a_lines: std::collections::BTreeSet<i64> = refs
        .iter()
        .filter(|l| l.uri == a)
        .filter_map(|l| {
            l.range
                .get("start")
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
        })
        .collect();
    assert!(
        a_lines.contains(&2) && a_lines.contains(&3),
        "both seeded views' callers must surface: {a_lines:?}"
    );
}

// -- faults 4–6: TclOO visibility, dispatch entry, binding identity ------

#[test]
fn unexported_method_is_not_externally_resolvable_945() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Vault {\n    method _secret {} { return hidden }\n    method probe {} { return [my _secret] }\n}\nset v [Vault new]\n$v _secret\n";
    lsp.open_ready(&uri, src);
    // tclsh 9.0.4: `$v _secret` → `unknown method "_secret"` — external
    // navigation resolves nothing.
    let external = lsp.definition(&uri, 5, 5);
    assert!(
        start_lines(&external).is_empty(),
        "an externally unexported method is not callable: {external:?}"
    );
    // `my _secret` (internal) resolves the declaration.
    let internal: Vec<i64> = start_lines(&lsp.definition(&uri, 2, 34))
        .into_iter()
        .collect();
    assert_eq!(
        internal,
        vec![1],
        "`my _secret` reaches the unexported declaration: {internal:?}"
    );
}

#[test]
fn method_definition_selects_the_dispatch_entry_945() {
    let mut lsp = Lsp::tcl();
    let animal = unique_uri("tcl");
    lsp.open_ready(
        &animal,
        "oo::class create Animal {\n    method speak {} { return animal }\n}\n",
    );
    let dog = unique_uri("tcl");
    lsp.open_ready(
        &dog,
        "oo::class create Dog {\n    superclass Animal\n    method speak {} { return dog }\n}\n",
    );
    let main = unique_uri("tcl");
    lsp.open_ready(&main, "set d [Dog new]\n$d speak\n");
    // tclsh 9.0.4: `info object call` = Dog then Animal — definition
    // returns the runtime entry (Dog::speak) only, never the family.
    let defs = locations(&lsp.definition(&main, 1, 4));
    assert_eq!(defs.len(), 1, "one dispatch entry only: {defs:?}");
    assert_eq!(defs[0].uri, dog, "{defs:?}");
}

#[test]
fn per_object_methods_resolve_by_binding_identity_945() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create C {}\nproc a {} {\n    set o [C new]\n    oo::objdefine $o {method m {} {return a}}\n    $o m\n}\nproc b {} {\n    set o [C new]\n    oo::objdefine $o {method m {} {return b}}\n    $o m\n}\n";
    lsp.open_ready(&uri, src);
    // tclsh 9.0.4: a=a b=b — b's dispatch resolves b's own override
    // (line 8), never a's (line 3).
    let b_def: Vec<i64> = start_lines(&lsp.definition(&uri, 9, 7))
        .into_iter()
        .collect();
    assert_eq!(b_def, vec![8], "b's own override: {b_def:?}");
    let a_def: Vec<i64> = start_lines(&lsp.definition(&uri, 4, 7))
        .into_iter()
        .collect();
    assert_eq!(a_def, vec![3], "a's own override: {a_def:?}");
}

// -- faults 7–8: the interpreter domain ----------------------------------

#[test]
fn safe_interp_hides_unsafe_commands_945() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // tclsh 9.0.4: `interp create -safe s` hides source/exec/…; calling
    // them raises `invalid command name` (W129 here), and no source edge
    // may be built from the call.  A never-created interp target warns
    // (W140).
    let src =
        "interp create -safe s\ninterp eval s { source b.tcl }\ninterp eval ghost { puts hi }\n";
    lsp.open_ready(&uri, src);
    let diags = lsp.await_diagnostics(&uri);
    let codes: Vec<String> = diags
        .iter()
        .filter_map(|d| d.get("code").and_then(Value::as_str).map(str::to_owned))
        .collect();
    assert!(
        codes.iter().any(|c| c == "W129"),
        "hidden `source` in the safe interp warns: {codes:?}"
    );
    assert!(
        codes.iter().any(|c| c == "W140"),
        "the never-created `ghost` target warns: {codes:?}"
    );
}

// -- fault 9: probe references -------------------------------------------

#[test]
fn command_probe_navigates_without_asserting_existence_945() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc greet {} {}\nnamespace which -command greet\nnamespace which -command no_such_command_xyz\n";
    lsp.open_ready(&uri, src);
    // The existing probe navigates (definition + references + rename)…
    let defs: Vec<i64> = start_lines(&lsp.definition(&uri, 1, 27))
        .into_iter()
        .collect();
    assert_eq!(defs, vec![0], "the probe site navigates: {defs:?}");
    let refs = start_lines(&lsp.references(&uri, 0, 6, true));
    assert!(refs.contains(&1), "the probe site is a reference: {refs:?}");
    // …while the missing-target probe draws no W123.
    let diags = lsp.await_diagnostics(&uri);
    assert!(
        !diags
            .iter()
            .any(|d| d.get("code").and_then(Value::as_str) == Some("W123")),
        "a probe of an absent command asserts nothing: {diags:?}"
    );
}

// -- issue #923 idx 94: eval/uplevel argument-position indirect dispatch ----
//
// A bare `$var` body of an `eval`/`uplevel` call (as opposed to `$var`
// sitting at a command's own *head* position, fault 1's shape above)
// dynamically evaluates $var's value as a script at runtime — the same
// flow-sensitive constant-dispatch settlement this file already covers,
// just reached through a different registration site
// (`dispatch_one_body_argument`'s new `TokenType::Var` branch).

#[test]
fn eval_of_a_list_computed_var_rewrites_the_defining_literal_and_executes_923_idx94() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // The finding's own minimal repro: `set cmdD [list greetD World]; eval
    // $cmdD` — real tclsh9.0/8.6-verified to print "D World".
    let src = "proc greetD {n} {puts \"D $n\"}\nset cmdD [list greetD World]\neval $cmdD\n";
    lsp.open_ready(&uri, src);
    let result = lsp.rename(&uri, 0, 6, "greetRenamed");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    let renamed_src = apply_lsp_edits(src, &for_uri);
    assert!(
        renamed_src.contains("[list greetRenamed World]"),
        "the `greetD` word inside the `list` call follows the rename:\n{renamed_src}"
    );
    assert!(
        renamed_src.contains("eval $cmdD"),
        "the `$cmdD` dispatch site itself is never rewritten:\n{renamed_src}"
    );
    // The issue's validation bar: the transformed output must EXECUTE — the
    // old edit set left `[list greetD World]` stale (only the declaration
    // was rewritten) and died with `invalid command name "greetD"`.
    if let Some(out) = run_tclsh(&renamed_src) {
        assert_eq!(
            out, "D World",
            "eval $cmdD still dispatches to the renamed proc"
        );
    } else {
        eprintln!("skipping tclsh execution leg: no tclsh (set TCL_LSP_TCLSH)");
    }
}

#[test]
fn eval_of_a_dynamic_unresolvable_var_body_produces_no_edits_923_idx94() {
    // TN — regression guard: a genuinely dynamic eval body (piped through
    // `gets`, no provable constant origin) must not surface a false
    // "safe" rename that only rewrites the unrelated declaration while
    // silently leaving an indirect dispatch nobody warned about.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc target {} { return hi }\nset cmd [gets stdin]\neval $cmd\n";
    lsp.open_ready(&uri, src);
    let result = lsp.rename(&uri, 0, 6, "renamed");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    assert_eq!(
        for_uri.len(),
        1,
        "only the declaration should rewrite — no evidence the dynamic \
         eval body ever reaches `target`: {for_uri:?}"
    );
}

// -- issue #923 idx 121: TclOO instance-class inference through a
// `$var`-headed constructor -------------------------------------------
//
// `record_instance_creation` / `class_from_constructor_subst` only
// recognised a literal class-name bareword at the `new`/`create` call
// site.  Real corpus (tcllib's `httpd/httpd.tcl:1970-1994`) instead flows
// the class name through a single, unconditional `set` one line earlier
// (`set class ::Derived; set obj [$class create NAME]`) — the analyser
// never bound `obj`'s class, so hover / go-to-definition / rename on a
// later `$obj method` call silently found nothing, exactly like the
// `{*}$cmd` / `eval $cmd` dispatch gaps this file's fault 1 / idx 94
// sections already cover, just for TclOO instance construction instead of
// plain command dispatch.

#[test]
fn hover_and_definition_resolve_a_method_through_a_var_headed_constructor_923_idx121() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // tclsh9.0/8.6-verified: `set class ::Dog; set obj [$class create rex];
    // $obj bark` calls `::Dog::bark` either way.
    let src = "oo::class create Dog {\n    method bark {} { return \"woof\" }\n}\nset class Dog\nset obj [$class create rex]\n$obj bark\n";
    lsp.open_ready(&uri, src);

    // Hover on the `$obj bark` call site (line 5) resolves through the
    // indirect class the same way it already does for a literal `Dog
    // create rex` — pre-fix this returned nothing at all.
    let hover = lsp.hover(&uri, 5, 5);
    let text = hover_text(&hover);
    assert!(
        text.contains("bark"),
        "hover on the indirectly-typed instance's method call must resolve: {text:?}"
    );

    // Go-to-definition from the same call site lands on the `bark` method
    // declaration (line 1).
    let defs: Vec<i64> = start_lines(&lsp.definition(&uri, 5, 5))
        .into_iter()
        .collect();
    assert_eq!(
        defs,
        vec![1],
        "go-to-definition must reach the method declaration: {defs:?}"
    );

    // References from the declaration reach the indirect call site too.
    let refs = start_lines(&lsp.references(&uri, 1, 11, true));
    assert!(
        refs.contains(&5),
        "the indirect `$obj bark` call site must be a reference: {refs:?}"
    );
}

#[test]
fn rename_rewrites_a_method_reached_through_a_var_headed_constructor_923_idx121() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Dog {\n    method bark {} { return \"woof\" }\n}\nset class Dog\nset obj [$class create rex]\nputs [$obj bark]\n";
    lsp.open_ready(&uri, src);

    let result = lsp.rename(&uri, 1, 11, "speak");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    let renamed_src = apply_lsp_edits(src, &for_uri);
    assert!(
        renamed_src.contains("method speak"),
        "the declaration must rewrite:\n{renamed_src}"
    );
    assert!(
        renamed_src.contains("[$obj speak]"),
        "the indirect call site must rewrite too:\n{renamed_src}"
    );
    if let Some(out) = run_tclsh(&renamed_src) {
        assert_eq!(
            out, "woof",
            "the renamed method must still execute through the indirect dispatch"
        );
    } else {
        eprintln!("skipping tclsh execution leg: no tclsh (set TCL_LSP_TCLSH)");
    }
}

#[test]
fn hover_abstains_on_a_branch_ambiguous_class_var_923_idx121() {
    // TN — a class variable whose reaching definitions genuinely disagree
    // (one branch arm `Dog`, the other `Cat`) is unprovable at the
    // constructor call: binding *either* class would be a guess, so hover
    // must find nothing rather than resolve to an arbitrary one.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Dog {\n    method bark {} {}\n}\noo::class create Cat {\n    method meow {} {}\n}\nif {$flag} { set class Dog } else { set class Cat }\nset obj [$class create x]\n$obj bark\n";
    lsp.open_ready(&uri, src);
    let hover = lsp.hover(&uri, 8, 5);
    let text = hover_text(&hover);
    assert!(
        text.is_empty(),
        "an ambiguous class var must not resolve to either class: {text:?}"
    );
}
