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

//! Native port of `tests/lsp_e2e/test_rename_e2e.py`.
//!
//! Rename, end-to-end against the packaged server. Full-parity port of the
//! request/response cases. An invalid or unsafe rename comes back as a `null`
//! `WorkspaceEdit` on the live wire (the `on_rename` handler returns `None`), so
//! the safety cases assert the result carries no edits. `prepareRename` is
//! registered server-side and returns a `{range, placeholder}` (or `null` to
//! reject).

// Test column math indexes tiny in-memory sources; a `find`/`len` result
// always fits u32, so the pedantic truncation the lint warns of can't occur.
#![allow(clippy::cast_possible_truncation)]

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::Value;

/// The set of `newText` values for edits applied to `uri`.
fn texts(
    lsp: &mut Lsp,
    uri: &str,
    line: u32,
    ch: u32,
    new_name: &str,
) -> std::collections::BTreeSet<String> {
    let result = lsp.rename(uri, line, ch, new_name);
    let edits = rename_edits(&result);
    edits
        .get(uri)
        .into_iter()
        .flatten()
        .map(|e| {
            e.get("newText")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        })
        .collect()
}

// -- TestPrepareRename ---------------------------------------------------

#[test]
fn prepare_rename_proc_name() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {name} { puts \"Hello $name\" }\ngreet World\n",
    );
    let result = lsp.prepare_rename(&uri, 0, 6);
    assert!(!result.is_null());
    assert_eq!(result["placeholder"], "greet");
}

#[test]
fn prepare_rename_variable() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\nputs $x\n");
    let result = lsp.prepare_rename(&uri, 1, 7);
    assert!(!result.is_null());
    assert_eq!(result["placeholder"], "x");
}

#[test]
fn prepare_rename_variable_from_definition_site() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\nputs $x\n");
    let result = lsp.prepare_rename(&uri, 0, 4);
    assert!(!result.is_null());
    assert_eq!(result["placeholder"], "x");
}

#[test]
fn prepare_rename_builtin_rejected() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts hello\n");
    assert!(lsp.prepare_rename(&uri, 0, 1).is_null());
}

#[test]
fn prepare_rename_unknown_rejected() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "something_unknown\n");
    assert!(lsp.prepare_rename(&uri, 0, 5).is_null());
}

// -- TestRenameProc ------------------------------------------------------

#[test]
fn rename_definition_and_calls() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc greet {name} { puts \"Hello $name\" }\ngreet World\ngreet Everyone\n";
    lsp.open_ready(&uri, src);
    let result = lsp.rename(&uri, 0, 6, "welcome");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    assert!(for_uri.len() >= 3);
    assert!(for_uri.iter().all(|e| e["newText"] == "welcome"));
}

/// idx=9 (differential-audit main wave, high severity): a cursor placed
/// directly on a proc parameter's own bareword declaration (not a
/// `$`-prefixed read) previously produced zero rename edits — an LSP
/// silently no-oping a rename request is worse than an explicit failure,
/// since the user has no signal anything went wrong. Both the parameter's
/// declaration and its `$name` read must be rewritten.
#[test]
fn rename_from_proc_param_bareword_declaration_rewrites_every_use_e2e() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc greet {name} { puts \"Hello $name\" }\n";
    lsp.open_ready(&uri, src);
    // Cursor on `name` inside the parameter list, col 12-16.
    let result = lsp.rename(&uri, 0, 13, "label");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    assert_eq!(for_uri.len(), 2, "{for_uri:?}");
    let texts: std::collections::BTreeSet<&str> = for_uri
        .iter()
        .map(|e| e["newText"].as_str().unwrap_or(""))
        .collect();
    assert!(texts.contains("label"), "{texts:?}");
    assert!(texts.contains("$label"), "{texts:?}");
}

#[test]
fn rename_namespaced_proc_preserves_qualifier() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval utils {\n    proc helper {x} { return $x }\n}\nutils::helper 42\n";
    lsp.open_ready(&uri, src);
    let t = texts(&mut lsp, &uri, 1, 10, "assist");
    assert!(t.contains("assist"), "{t:?}");
    assert!(t.contains("utils::assist"), "{t:?}");
}

#[test]
fn rename_proc_in_two_level_nested_namespace_does_not_leak_across_namespaces() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval a {\n    namespace eval b {\n        proc helper {} { return 1 }\n    }\n}\nnamespace eval c {\n    namespace eval d {\n        proc helper {} { return 2 }\n        proc caller {} { return [helper] }\n    }\n}\n";
    lsp.open_ready(&uri, src);
    let result = lsp.rename(&uri, 7, 14, "assist");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    // Only ::c::d::helper's decl + its own bare call rewritten (2 edits),
    // never ::a::b::helper (RUST_ISSUE_035-style leak across namespaces).
    assert_eq!(for_uri.len(), 2, "{for_uri:?}");
    assert!(for_uri.iter().all(|e| e["newText"] == "assist"));
}

#[test]
fn rename_from_call_site() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {name} { puts \"Hello $name\" }\ngreet World\n",
    );
    let result = lsp.rename(&uri, 1, 0, "welcome");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    assert!(for_uri.len() >= 2);
}

/// idx=61 (differential-audit main wave, critical severity): before this
/// fix, an unbraced `if`-body call site (`if {1} foo`) was invisible to
/// `command_invocations`, so a rename built on that list silently skipped
/// it — the LSP presented the rename as complete while leaving this call
/// site referring to the old (now nonexistent) name, breaking the program
/// at runtime. Both the declaration and the unbraced call site must be
/// rewritten together.
#[test]
fn rename_rewrites_unbraced_if_body_bareword_call_site() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc foo {} { return 1 }\nif {1} foo\n");
    let result = lsp.rename(&uri, 0, 6, "bar");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    assert_eq!(
        for_uri.len(),
        2,
        "decl + unbraced if-body call site must both be rewritten: {for_uri:?}"
    );
    assert!(for_uri.iter().all(|e| e["newText"] == "bar"));
}

// -- TestRenameVariable --------------------------------------------------

#[test]
fn rename_var() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\nputs $x\n");
    let t = texts(&mut lsp, &uri, 1, 7, "newvar");
    assert!(t.contains("newvar"), "{t:?}");
    assert!(t.contains("$newvar"), "{t:?}");
}

#[test]
fn rename_var_from_definition_site() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\nputs $x\n");
    let t = texts(&mut lsp, &uri, 0, 4, "newvar");
    assert!(t.contains("newvar"), "{t:?}");
    assert!(t.contains("$newvar"), "{t:?}");
}

#[test]
fn rename_var_preserves_braced_form() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\nputs ${x}\n");
    let t = texts(&mut lsp, &uri, 0, 4, "newvar");
    assert!(t.contains("newvar"), "{t:?}");
    assert!(t.contains("${newvar}"), "{t:?}");
}

/// Find the one `{range, newText}` edit whose replacement text is exactly
/// `expected_text`, applied to `source` — the strongest check available at
/// this layer, since it proves the fix through real JSON-RPC (de)serialization,
/// not just the core crate's own in-process `TextEdit`s.
fn apply_named_edit(edits: &[Value], source: &str, expected_text: &str) -> String {
    let (rng, new_text) = edits
        .iter()
        .find_map(|e| {
            let text = e.get("newText")?.as_str()?;
            (text == expected_text).then(|| (e.get("range").cloned().unwrap(), text.to_owned()))
        })
        .unwrap_or_else(|| panic!("expected a {expected_text:?} edit among {edits:?}"));
    apply_edit(source, &rng, &new_text)
}

#[test]
fn rename_var_applying_the_braced_reference_edit_does_not_duplicate_the_closing_brace() {
    // Issue #923 idx 95, applied end-to-end against the packaged server
    // over real JSON-RPC. `rename_var_preserves_braced_form` right above
    // only asserts `newText` in isolation — this actually applies
    // `(range, newText)` back onto the source, which is exactly what
    // shipping this bug uncaught required nobody doing.  The `Var`
    // token's own lexer span for a non-degenerate `${name}` form stops
    // one byte short of the closing `}`, so using it verbatim as the
    // edit range left the source's original `}` behind, corrupting
    // `${x}` into `${y}}`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set x 1\nputs ${x}\n";
    lsp.open_ready(&uri, src);
    let result = lsp.rename(&uri, 1, 7, "y");
    let edits = rename_edits(&result);
    let uri_edits = edits.get(&uri).cloned().unwrap_or_default();
    assert_eq!(
        apply_named_edit(&uri_edits, src, "${y}"),
        "set x 1\nputs ${y}\n"
    );
}

#[test]
fn rename_var_applying_the_dir_view_idiom_reference_edit_does_not_corrupt_the_source() {
    // The real `tk/library/tk.tcl:594-596` idiom this finding traces
    // through (`$w ${dir}view scroll ...`, a subcommand synthesized by
    // concatenating `$dir` with literal `view`): applying the LSP's own
    // rename edit for the `${dir}view` reference previously produced
    // `$w ${direction}}view ...` — tclsh8.6/9.0 both fail to even parse
    // the enclosing proc ("extra characters after close-brace") once
    // that edit is applied, since the stray extra `}` shifts Tcl's own
    // brace-counting scan for where the proc body ends.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc ::tk::MouseWheel {w dir amount {factor -120.0} {units units}} {\n    $w ${dir}view scroll [expr {$amount/$factor}] $units\n}\n";
    lsp.open_ready(&uri, src);
    // Cursor on the `d` of `dir` inside `${dir}view` (line 1, col 9).
    let result = lsp.rename(&uri, 1, 9, "direction");
    let edits = rename_edits(&result);
    let uri_edits = edits.get(&uri).cloned().unwrap_or_default();
    let applied = apply_named_edit(&uri_edits, src, "${direction}");
    assert!(
        applied.contains("${direction}view"),
        "expected a single, correctly-closed brace: {applied}"
    );
    assert!(
        !applied.contains("${direction}}view"),
        "must not duplicate the closing brace: {applied}"
    );
}

#[test]
fn rename_qualified_var_preserves_namespace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set myns::count 0\nputs $myns::count\n");
    let t = texts(&mut lsp, &uri, 0, 10, "total");
    assert!(t.contains("myns::total"), "{t:?}");
    assert!(t.contains("$myns::total"), "{t:?}");
}

#[test]
fn rename_qualified_var_braced_form() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set myns::count 0\nputs ${myns::count}\n");
    let t = texts(&mut lsp, &uri, 0, 10, "total");
    assert!(t.contains("myns::total"), "{t:?}");
    assert!(t.contains("${myns::total}"), "{t:?}");
}

#[test]
fn rename_preserves_array_index() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set arr 1\nputs $arr\nputs $arr(a)\nputs $arr($i)\nputs ${arr(b)}\n";
    lsp.open_ready(&uri, src);
    let t = texts(&mut lsp, &uri, 0, 4, "new");
    for expected in ["new", "$new", "$new(a)", "$new($i)", "${new(b)}"] {
        assert!(t.contains(expected), "missing {expected:?} in {t:?}");
    }
}

#[test]
fn rename_respects_scope() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set x 1\nproc demo {} {\n    set x 2\n    puts $x\n}\n";
    lsp.open_ready(&uri, src);
    let result = lsp.rename(&uri, 3, 10, "y");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    let lines: std::collections::BTreeSet<i64> = for_uri
        .iter()
        .filter_map(|e| e["range"]["start"]["line"].as_i64())
        .collect();
    assert!(!lines.contains(&0), "{lines:?}");
}

// -- TestRenameSafety ----------------------------------------------------

#[test]
fn rejects_invalid_new_symbol_name() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {name} { puts \"Hello $name\" }\ngreet World\n",
    );
    let result = lsp.rename(&uri, 0, 6, "bad-name");
    assert!(rename_edits(&result).is_empty());
}

#[test]
fn rejects_proc_collision() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc greet {name} { puts \"Hello $name\" }\nproc hello {name} { puts \"Hi $name\" }\ngreet World\n";
    lsp.open_ready(&uri, src);
    let result = lsp.rename(&uri, 0, 6, "hello");
    assert!(rename_edits(&result).is_empty());
}

#[test]
fn rejects_proc_rename_to_builtin() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {name} { puts \"Hello $name\" }\ngreet World\n",
    );
    let result = lsp.rename(&uri, 0, 6, "puts");
    assert!(rename_edits(&result).is_empty());
}

#[test]
fn rejects_var_collision_in_same_scope() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc demo {} {\n    set x 1\n    set y 2\n    puts $x\n}\n";
    lsp.open_ready(&uri, src);
    let result = lsp.rename(&uri, 3, 10, "y");
    assert!(rename_edits(&result).is_empty());
}

// -- TclOO instance-variable rename must not corrupt the body --

/// End-to-end (real server / incremental path): renaming a `TclOO`
/// instance variable rewrites its `variable` declaration and its `$var`
/// uses — and NEVER the whole method body.  Before the fix the declaration
/// edit spanned `{return $n}`, replacing the body with the new name.
#[test]
fn rename_tcloo_instance_variable_does_not_rewrite_body() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // line 0: oo::class create C {
    // line 1:     variable n
    // line 2:     method get {} {return $n}
    // line 3:     method set {x} {set n $x}
    // line 4: }
    let src = "oo::class create C {\n    variable n\n    method get {} {return $n}\n    method set {x} {set n $x}\n}\n";
    lsp.open_ready(&uri, src);
    // cursor on `$n` in `get` (line 2). `    method get {} {return $n}`
    let col = src.lines().nth(2).unwrap().find("$n").unwrap() as u32 + 1;
    let result = lsp.rename(&uri, 2, col, "w");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    assert!(!for_uri.is_empty(), "expected a rename: {result:?}");
    for e in &for_uri {
        let sl = e["range"]["start"]["line"].as_u64().unwrap();
        let el = e["range"]["end"]["line"].as_u64().unwrap();
        assert_eq!(sl, el, "no edit may span lines (body-destroying): {e:?}");
        // No edit may cover the method body `{return $n}` (line 2, cols 18..28).
        let sc = e["range"]["start"]["character"].as_u64().unwrap();
        let ec = e["range"]["end"]["character"].as_u64().unwrap();
        let covers_body = sl == 2 && sc <= 18 && ec >= 28;
        assert!(!covers_body, "edit covers the whole method body: {e:?}");
    }
    // The `variable n` declaration (line 1) is renamed.
    assert!(
        for_uri
            .iter()
            .any(|e| e["range"]["start"]["line"] == 1 && e["newText"] == "w"),
        "expected the `variable n` declaration to be renamed: {for_uri:?}"
    );
}

/// End-to-end: go-to-definition on `$g` inside `uplevel #0 { … }`
/// resolves to the GLOBAL `set g` (line 0), not the same-named proc-local.
#[test]
fn definition_uplevel_zero_var_resolves_global() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // line 0: set g 1
    // line 2:     set g 99   (proc-local)
    // line 3:     uplevel #0 { puts $g }
    let src = "set g 1\nproc p {} {\n    set g 99\n    uplevel #0 { puts $g }\n}\n";
    lsp.open_ready(&uri, src);
    let col = src.lines().nth(3).unwrap().find("$g").unwrap() as u32 + 1;
    let result = lsp.definition(&uri, 3, col);
    // definition returns a Location or [Location]; extract the target line(s).
    let locs = match &result {
        Value::Array(a) => a.clone(),
        Value::Null => vec![],
        other => vec![other.clone()],
    };
    let lines: Vec<u64> = locs
        .iter()
        .filter_map(|l| l["range"]["start"]["line"].as_u64())
        .collect();
    assert!(
        lines.contains(&0),
        "uplevel #0 `$g` must go to the global `set g` (line 0); got {result:?}"
    );
    assert!(
        !lines.contains(&2),
        "must NOT go to the proc-local `set g 99` (line 2); got {result:?}"
    );
}

// -- rename from a call site targets the caller's namespace --

/// End-to-end: renaming from the `helper` call site inside `::a::run` renames
/// `::a::helper` and its call, never the same-named `::b::helper`.
#[test]
fn rename_from_callsite_does_not_touch_same_named_proc_in_other_namespace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // line 1: ::a::helper decl; line 2: the `helper` call; line 5: ::b::helper decl
    let src = "namespace eval ::a {\n    proc helper {} { return 1 }\n    proc run {} { helper }\n}\nnamespace eval ::b {\n    proc helper {} { return 2 }\n}\n";
    lsp.open_ready(&uri, src);
    let col = src.lines().nth(2).unwrap().find("{ helper }").unwrap() as u32 + 2;
    let result = lsp.rename(&uri, 2, col, "assist");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    let lines: Vec<u64> = for_uri
        .iter()
        .filter_map(|e| e["range"]["start"]["line"].as_u64())
        .collect();
    assert!(
        lines.contains(&1),
        "::a::helper decl (line 1) must rename: {for_uri:?}"
    );
    assert!(
        lines.contains(&2),
        "the call (line 2) must rename: {for_uri:?}"
    );
    assert!(
        !lines.contains(&5),
        "must NOT rename ::b::helper (line 5): {for_uri:?}"
    );
}

/// End-to-end: renaming an `expr` math-function override renames both its
/// declaration and the call site's bare tail token (`pf(1)` ->
/// `pfRenamed(1)`), and never touches an unrelated same-named ordinary proc.
#[test]
fn rename_mathfunc_override_updates_call_site_and_skips_unrelated_proc() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // line 0: unrelated proc pf; line 2: the override decl; line 4: the expr call
    let src = "proc pf {y} { return bogus }\n\
               namespace eval ::nsa::tcl::mathfunc {}\n\
               proc ::nsa::tcl::mathfunc::pf {x} { return 20 }\n\
               namespace eval ::nsa {\n    proc caller {} { return [expr {pf(1)}] }\n}\n";
    lsp.open_ready(&uri, src);
    let col = src.lines().nth(2).unwrap().rfind("pf").unwrap() as u32;
    let result = lsp.rename(&uri, 2, col, "pfRenamed");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    let lines: Vec<u64> = for_uri
        .iter()
        .filter_map(|e| e["range"]["start"]["line"].as_u64())
        .collect();
    assert!(
        lines.contains(&2),
        "the override decl (line 2) must rename: {for_uri:?}"
    );
    assert!(
        lines.contains(&4),
        "the expr call site (line 4) must rename: {for_uri:?}"
    );
    assert!(
        !lines.contains(&0),
        "must NOT rename the unrelated proc pf (line 0): {for_uri:?}"
    );
    // The declaration's own name token is written in source as the full
    // qualified name (`proc ::nsa::tcl::mathfunc::pf {...}`), so its edit
    // rewrites the whole qualified string; the `expr` call site only ever
    // has the bare tail written (`pf(1)`), so its edit rewrites just that.
    let new_texts: std::collections::BTreeSet<&str> = for_uri
        .iter()
        .filter_map(|e| e["newText"].as_str())
        .collect();
    assert_eq!(
        new_texts,
        std::collections::BTreeSet::from(["::nsa::tcl::mathfunc::pfRenamed", "pfRenamed"]),
        "decl rewrites the qualified name, the call site rewrites only the tail: {for_uri:?}"
    );
}

/// idx 31 (differential-audit main audit wave, high severity): a proc
/// declared twice, verbatim, in the same document (plain Tcl's own "last
/// redefinition wins" semantics, tclsh9.0/8.6-verified — the real corpus
/// shape is `georgtree_tclopt`'s `tclopt.tcl` declaring
/// `::tclopt::List2array` at two separate line ranges). Before this fix,
/// `resolve_workspace_symbols` identified "the symbol at cursor" only via
/// a scan of `all_procs` (keyed by qualified name, so a duplicate insert
/// retains only the *winning* declaration's span) — a rename issued from
/// the *shadowed* (non-winning) declaration's own name token silently
/// dropped every cross-file caller from the edit set. Applying that
/// incomplete edit is worse than a no-op: the shadowed declaration being
/// renamed is a real, dead definition still lying around under the old
/// name, so the un-rewritten caller silently starts running it instead —
/// proven end-to-end in the finding's own repro (a program's real output
/// changed with no error surfaced anywhere).
#[test]
fn rename_from_shadowed_duplicate_proc_decl_reaches_cross_document_caller() {
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(
        &lib_uri,
        "proc List2array {lst} { return ONE }\nproc List2array {lst} { return TWO }\n",
    );
    let consumer_uri = unique_uri("tcl");
    lsp.open_ready(&consumer_uri, "List2array x\n");
    // Cursor on the SHADOWED (first, non-winning) declaration's own name
    // token (line 0, col 6 — `proc List2array`).
    let result = lsp.rename(&lib_uri, 0, 6, "ListToArray");
    let edits = rename_edits(&result);
    let consumer_edits = edits.get(&consumer_uri).cloned().unwrap_or_default();
    assert_eq!(
        consumer_edits.len(),
        1,
        "the cross-file caller must be rewritten, or it stays bound to \
         the old name while the dead shadowed definition (also renamed \
         away) silently resurrects for it: {edits:?}"
    );
    assert_eq!(consumer_edits[0]["newText"], "ListToArray");
}

/// idx 39 (differential-audit main audit wave, high severity): `rename OLD
/// NEW`'s own `OLD` word was omitted from the reference set find-references
/// and rename both build from — go-to-definition/hover on that exact token
/// resolved it correctly (an independent cursor-token walk), but rename
/// silently left it unrewritten. The real corpus shape is a tcltest
/// `-setup`/`-body`/`-cleanup` idiom (`proc gaussfunc {...} {...}` /
/// `rename gaussfunc ""`) — applying the LSP's own incomplete rename
/// `WorkspaceEdit` to that shape crashes a previously-passing test at runtime
/// ("can't delete ...: command doesn't exist") with no diagnostic warning
/// anywhere.
#[test]
fn rename_rewrites_the_renames_own_old_word_too() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc helperFunc {x} { return [expr {$x * 2}] }\nhelperFunc 21\nrename helperFunc \"\"\n",
    );
    let result = lsp.rename(&uri, 0, 6, "newName");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    let lines: Vec<i64> = for_uri
        .iter()
        .filter_map(|e| e["range"]["start"]["line"].as_i64())
        .collect();
    assert_eq!(for_uri.len(), 3, "{for_uri:?}");
    assert!(lines.contains(&0), "decl missing: {for_uri:?}");
    assert!(lines.contains(&1), "call site missing: {for_uri:?}");
    assert!(
        lines.contains(&2),
        "the rename statement's own OLD word must be rewritten too: {for_uri:?}"
    );
    assert!(for_uri.iter().all(|e| e["newText"] == "newName"));
}

/// FP guard for issue #923 idx 21: find-references now attributes an alias's
/// call sites to the target proc, but **rename must not rewrite them**. A
/// `[sayHi]` call names the *alias*, which keeps its own spelling when the
/// target is renamed — tclsh 9.0.4/8.6.16 confirm `interp alias {} sayHi {}
/// greet` followed by `rename greet hi2` leaves `sayHi` bound (it re-resolves
/// `greet`, which is now gone, and fails) — so rewriting the call site would
/// change a different command's name.
#[test]
fn rename_does_not_rewrite_call_sites_that_go_through_an_alias() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {} { return hi }\ninterp alias {} sayHi {} greet\nputs [sayHi]\n",
    );
    // Line 0: cursor on the `greet` declaration.
    let result = lsp.rename(&uri, 0, 6, "welcome");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    let lines: Vec<i64> = for_uri
        .iter()
        .filter_map(|e| e["range"]["start"]["line"].as_i64())
        .collect();
    assert!(lines.contains(&0), "decl missing: {for_uri:?}");
    assert!(
        lines.contains(&1),
        "the alias's own TARGET word names `greet` and must be rewritten: {for_uri:?}"
    );
    assert!(
        !lines.contains(&2),
        "the `[sayHi]` call names the alias, not `greet`, and must be left alone: {for_uri:?}"
    );
}

/// idx 45 (differential-audit main audit wave): rename issued from the
/// declaration a later same-named `proc` displaced must still reach every
/// call site — the displaced header declares the same command, so a partial
/// edit would leave callers bound to a name that no longer exists.
#[test]
fn rename_from_a_superseded_declaration_rewrites_every_site() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc p {} { return first }\np\nproc p {a} { return $a }\np x\n",
    );
    // Line 0: cursor on the *first* (later displaced) `p` declaration.
    let result = lsp.rename(&uri, 0, 5, "q");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    let lines: Vec<i64> = for_uri
        .iter()
        .filter_map(|e| e["range"]["start"]["line"].as_i64())
        .collect();
    for expected in [1_i64, 2, 3] {
        assert!(
            lines.contains(&expected),
            "line {expected} must be rewritten: {for_uri:?}"
        );
    }
}

/// idx 56 (differential-audit main audit wave, high severity): a proc
/// installed directly into `::oo::Helpers` (the documented "`TclOO` Tricks"
/// idiom — real corpus usage: nico-robert/ticklecharts installs `classvar`/
/// `callback` this way) is bare-callable from every method body in the
/// program via `TclOO`'s own fixed runtime namespace path. Renaming it
/// previously produced a `WorkspaceEdit` that rewrote only the declaration,
/// leaving every bare call site pointed at the now-nonexistent old name —
/// applying that edit verbatim crashes the very next invocation with
/// "invalid command name" at runtime, while the tool reported it as a
/// complete, safe rename.
#[test]
fn rename_rewrites_bare_calls_to_a_proc_installed_into_oo_helpers() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc ::oo::Helpers::classvar {name} {\n    set ns [uplevel 1 {my getONSClass}]\n    tailcall namespace upvar $ns $name $name\n}\noo::class create Counter {\n    variable _label\n    constructor {label} { set _label $label }\n    method getONSClass {} { return [self class] }\n    method bump {} {\n        classvar hits\n        incr hits\n        return \"$_label:$hits\"\n    }\n}\n",
    );
    // Line 0: `proc ::oo::Helpers::classvar {name} {` — cursor on the
    // `classvar` word (column 20).
    let result = lsp.rename(&uri, 0, 20, "renamedClassvar");
    let edits = rename_edits(&result);
    let for_uri = edits.get(&uri).cloned().unwrap_or_default();
    let lines: Vec<i64> = for_uri
        .iter()
        .filter_map(|e| e["range"]["start"]["line"].as_i64())
        .collect();
    assert!(lines.contains(&0), "decl missing: {for_uri:?}");
    assert!(
        lines.contains(&9),
        "the bare classvar call site inside the method body must be rewritten too: {for_uri:?}"
    );
    let replacements: Vec<&str> = for_uri
        .iter()
        .filter_map(|e| e["newText"].as_str())
        .collect();
    assert!(
        replacements.contains(&"::oo::Helpers::renamedClassvar"),
        "expected the qualified replacement at decl; got {replacements:?}"
    );
    assert!(
        replacements.contains(&"renamedClassvar"),
        "expected the short replacement at the bare call site; got {replacements:?}"
    );
}
