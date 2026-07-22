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
