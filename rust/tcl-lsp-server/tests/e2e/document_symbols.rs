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

//! Document symbols for Tcl and `TclOO`, end-to-end against the packaged server.
//! Symbol kinds come back as raw LSP integer codes (`SymbolKind`); the named
//! constants here mirror that enum.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::Value;

// LSP SymbolKind integer codes.
const NAMESPACE: i64 = 3;
const CLASS: i64 = 5;
const METHOD: i64 = 6;
const PROPERTY: i64 = 7;
const CONSTRUCTOR: i64 = 9;
const FUNCTION: i64 = 12;
const VARIABLE: i64 = 13;
const CONSTANT: i64 = 14;
const OPERATOR: i64 = 25;
const EVENT: i64 = 24;

/// The top-level document symbols for `uri` as a list (`or []`).
fn top(lsp: &mut Lsp, uri: &str) -> Vec<Value> {
    let result = lsp.document_symbols(uri);
    result.as_array().cloned().unwrap_or_default()
}

/// A symbol's `name`.
fn name(sym: &Value) -> &str {
    sym.get("name").and_then(Value::as_str).unwrap_or("")
}

/// A symbol's `kind` as an integer.
fn kind(sym: &Value) -> i64 {
    sym.get("kind").and_then(Value::as_i64).unwrap_or(0)
}

/// A symbol's `detail` (empty if absent/null).
fn detail(sym: &Value) -> &str {
    sym.get("detail").and_then(Value::as_str).unwrap_or("")
}

/// A symbol's `children` as a list.
fn children(sym: &Value) -> Vec<Value> {
    sym.get("children")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

// -- TestDocumentSymbols -------------------------------------------------

#[test]
fn single_proc() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {name} {\n    puts \"Hello $name\"\n}\n");
    let syms = top(&mut lsp, &uri);
    assert_eq!(syms.len(), 1);
    assert_eq!(name(&syms[0]), "greet");
    assert_eq!(kind(&syms[0]), FUNCTION);
    assert_eq!(detail(&syms[0]), "(name)");
}

#[test]
fn proc_with_defaults() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {name {greeting Hello}} {\n    puts \"$greeting\"\n}\n",
    );
    assert_eq!(detail(&top(&mut lsp, &uri)[0]), "(name {greeting Hello})");
}

#[test]
fn proc_no_params() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc nop {} { return }\n");
    assert_eq!(detail(&top(&mut lsp, &uri)[0]), "()");
}

#[test]
fn multiple_procs() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc foo {} { return 1 }\nproc bar {} { return 2 }\n");
    let names = symbol_names(&lsp.document_symbols(&uri));
    assert!(names.contains("foo"), "missing foo in {names:?}");
    assert!(names.contains("bar"), "missing bar in {names:?}");
}

#[test]
fn namespace_with_proc() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval myns {\n    proc helper {} {\n        return 1\n    }\n}\n";
    lsp.open_ready(&uri, src);
    let syms = top(&mut lsp, &uri);
    assert_eq!(syms.len(), 1);
    let ns = &syms[0];
    assert_eq!(name(ns), "myns");
    assert_eq!(kind(ns), NAMESPACE);
    let child_names: Vec<String> = children(ns).iter().map(|c| name(c).to_owned()).collect();
    assert_eq!(child_names, vec!["helper".to_owned()]);
    assert_eq!(kind(&children(ns)[0]), FUNCTION);
}

#[test]
fn global_variable() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set myvar 42\n");
    let var_syms: Vec<Value> = top(&mut lsp, &uri)
        .into_iter()
        .filter(|s| kind(s) == VARIABLE)
        .collect();
    assert!(var_syms.iter().any(|s| name(s) == "myvar"));
}

#[test]
fn append_and_lappend_targets_are_variables() {
    // `append` / `lappend` create their target variable, so it surfaces as a
    // VARIABLE symbol just like `set`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "lappend safe 1\nappend note hi\n");
    let names: std::collections::BTreeSet<String> = top(&mut lsp, &uri)
        .into_iter()
        .filter(|s| kind(s) == VARIABLE)
        .map(|s| name(&s).to_owned())
        .collect();
    assert!(names.contains("safe"), "{names:?}");
    assert!(names.contains("note"), "{names:?}");
}

#[test]
fn empty_file() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "");
    assert_eq!(top(&mut lsp, &uri), Vec::<Value>::new());
}

#[test]
fn nested_namespace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval outer {\n    namespace eval inner {\n        proc deep {} { return }\n    }\n}\n";
    lsp.open_ready(&uri, src);
    let syms = top(&mut lsp, &uri);
    assert_eq!(syms.len(), 1);
    let outer = &syms[0];
    assert_eq!(name(outer), "outer");
    let inner: Vec<Value> = children(outer)
        .into_iter()
        .filter(|c| name(c) == "inner")
        .collect();
    assert_eq!(inner.len(), 1);
    assert_eq!(name(&children(&inner[0])[0]), "deep");
}

#[test]
fn proc_symbol_range_contains_selection() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {name} {\n    puts \"Hello $name\"\n}\n");
    let syms = top(&mut lsp, &uri);
    let sym = &syms[0];
    let outer = sym.get("range").unwrap();
    let inner = sym.get("selectionRange").unwrap();
    let start_line = |r: &Value| r["start"]["line"].as_i64().unwrap();
    let end_line = |r: &Value| r["end"]["line"].as_i64().unwrap();
    assert!(start_line(outer) <= start_line(inner));
    assert!(end_line(outer) >= end_line(inner));
}

// -- TestTclOOSymbols ----------------------------------------------------

#[test]
fn class_symbol_emitted() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create Dog {\n    method bark {} { return \"woof\" }\n}\n",
    );
    let syms = top(&mut lsp, &uri);
    assert_eq!(syms.len(), 1);
    assert_eq!(kind(&syms[0]), CLASS);
    assert_eq!(name(&syms[0]), "Dog");
}

#[test]
fn methods_nested_under_class() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Dog {\n    method bark {} { return \"woof\" }\n    method fetch {item} { return $item }\n}\n";
    lsp.open_ready(&uri, src);
    let syms = top(&mut lsp, &uri);
    let cls = &syms[0];
    let method_names: Vec<String> = children(cls).iter().map(|c| name(c).to_owned()).collect();
    assert!(method_names.contains(&"bark".to_owned()));
    assert!(method_names.contains(&"fetch".to_owned()));
    assert!(children(cls).iter().all(|c| kind(c) == METHOD));
}

#[test]
fn constructor_symbol() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create Dog {\n    constructor {name} { set n $name }\n}\n",
    );
    let syms = top(&mut lsp, &uri);
    let ctor = &children(&syms[0])[0];
    assert_eq!(name(ctor), "constructor");
    assert_eq!(kind(ctor), CONSTRUCTOR);
    assert!(detail(ctor).contains("(name)"), "detail: {}", detail(ctor));
}

#[test]
fn property_symbol() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::configurable create Point {\n    property x y\n}\n",
    );
    let syms = top(&mut lsp, &uri);
    let cls = &syms[0];
    let props: std::collections::BTreeSet<String> = children(cls)
        .into_iter()
        .filter(|c| kind(c) == PROPERTY)
        .map(|c| name(&c).to_owned())
        .collect();
    assert!(props.contains("x"), "{props:?}");
    assert!(props.contains("y"), "{props:?}");
}

#[test]
fn class_detail_shows_superclass() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "oo::class create Dog {\n    superclass Animal\n}\n");
    let syms = top(&mut lsp, &uri);
    assert!(
        detail(&syms[0]).contains(": Animal"),
        "detail: {}",
        detail(&syms[0])
    );
}

#[test]
fn class_detail_shows_metaclass() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::abstract create Shape {\n    method area {} {}\n}\n",
    );
    let syms = top(&mut lsp, &uri);
    assert!(
        detail(&syms[0]).contains("oo::abstract"),
        "detail: {}",
        detail(&syms[0])
    );
}

#[test]
fn classmethod_detail() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create Counter {\n    classmethod count {} { return 0 }\n}\n",
    );
    let syms = top(&mut lsp, &uri);
    let cls = &syms[0];
    let cm: Vec<Value> = children(cls)
        .into_iter()
        .filter(|c| name(c) == "count")
        .collect();
    assert_eq!(cm.len(), 1);
    assert!(
        detail(&cm[0]).contains("classmethod"),
        "detail: {}",
        detail(&cm[0])
    );
}

#[test]
fn self_block_form_members_appear_in_the_outline() {
    // Issue #1081 — TP, end to end. `self { method … }` declares exactly what
    // `self method …` does; only the prefix spelling used to reach the outline.
    // Oracle (tclsh 9.0.4 / 8.6.16, identical):
    //   oo::class create ::C { self { method make {n} {…} } ; method tick {} {…} }
    //   ::C make 7               -> made-7
    //   info object methods ::C  -> make     (class-object side)
    //   info class methods ::C   -> tick     (instance side)
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create Counter {\n",
            "    self {\n",
            "        method make {n} { return $n }\n",
            "        method reset {} { return 0 }\n",
            "    }\n",
            "    method tick {} { return 1 }\n",
            "}\n",
        ),
    );
    let syms = top(&mut lsp, &uri);
    let cls = &syms[0];
    assert_eq!(kind(cls), CLASS);
    let kids = children(cls);
    let make: Vec<&Value> = kids.iter().filter(|c| name(c) == "make").collect();
    assert_eq!(make.len(), 1, "expected `make` in the outline: {kids:?}");
    assert_eq!(kind(make[0]), METHOD);
    assert!(
        detail(make[0]).contains("classmethod"),
        "a `self`-scoped member carries the classmethod detail, got {:?}",
        detail(make[0]),
    );
    assert!(
        kids.iter().any(|c| name(c) == "reset"),
        "every member of the block lists, not just the first: {kids:?}",
    );
    // The instance method alongside it keeps its own (non-classmethod) detail.
    let tick: Vec<&Value> = kids.iter().filter(|c| name(c) == "tick").collect();
    assert_eq!(tick.len(), 1);
    assert!(!detail(tick[0]).contains("classmethod"));
}

#[test]
fn self_block_deleted_member_is_absent_from_the_outline() {
    // Issue #1095 review — TN, end to end. A member the block deletes must not
    // reach the outline. Oracle (tclsh 9.0.4 / 8.6.16, identical):
    //   oo::class create ::C1 {
    //       self { method gone {} {…} ; method kept {} {…} ; deletemethod gone }
    //   }
    //   info object methods ::C1  ->  kept
    //   ::C1 gone                 ->  unknown method "gone"
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create Counter {\n",
            "    self {\n",
            "        method gone {} { return 1 }\n",
            "        method kept {} { return 2 }\n",
            "        deletemethod gone\n",
            "    }\n",
            "}\n",
        ),
    );
    let syms = top(&mut lsp, &uri);
    let kids = children(&syms[0]);
    let names: Vec<&str> = kids.iter().map(name).collect();
    assert_eq!(
        names,
        ["kept"],
        "stale deleted member in outline: {names:?}"
    );
}

#[test]
fn unwrapped_deleted_member_is_absent_from_the_outline() {
    // Issue #1101 — TP, end to end and the user-visible symptom. An
    // *unwrapped* `deletemethod` (no `self` / `private` wrapper, straight in
    // an `oo::define` body) really removes the instance method, so a retained
    // outline entry navigates to a name the interpreter does not have. Oracle
    // (tclsh 9.0.4 / 8.6.14, identical):
    //   oo::class create ::I4 { method gone {} {…} ; method kept {} {…} }
    //   oo::define ::I4 { deletemethod gone }
    //   info class methods ::I4  ->  kept
    //   [::I4 new] gone          ->  unknown method "gone"
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create Counter {\n",
            "    method gone {} { return 1 }\n",
            "    method kept {} { return 2 }\n",
            "}\n",
            "oo::define Counter {\n",
            "    deletemethod gone\n",
            "}\n",
        ),
    );
    let syms = top(&mut lsp, &uri);
    let kids = children(&syms[0]);
    let names: Vec<&str> = kids.iter().map(name).collect();
    assert_eq!(
        names,
        ["kept"],
        "stale deleted member in outline: {names:?}"
    );
}

#[test]
fn unwrapped_delete_does_not_reach_the_class_side_of_the_outline() {
    // Issue #1101 — TN, end to end. The unwrapped word is instance-scoped, so
    // a class-object-side member of the same name keeps its outline entry.
    // (Real Tcl makes the cross-side spelling a hard definition-aborting
    // error — `method cm does not exist` — so nothing is lost by keeping it.)
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create Counter {\n",
            "    self {\n",
            "        method cm {} { return 1 }\n",
            "    }\n",
            "}\n",
            "oo::define Counter {\n",
            "    deletemethod cm\n",
            "}\n",
        ),
    );
    let syms = top(&mut lsp, &uri);
    let kids = children(&syms[0]);
    assert!(
        kids.iter().any(|c| name(c) == "cm"),
        "class-side member wrongly retracted: {kids:?}",
    );
}

#[test]
fn self_introspection_inside_a_method_body_adds_no_symbol() {
    // Issue #1081 — TN, end to end. `self class` / `self object` in a method
    // body are introspection calls, not definer members: the outline must show
    // the method and nothing else.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create Counter {\n",
            "    method whoami {} {\n",
            "        set c [self class]\n",
            "        return [self object]\n",
            "    }\n",
            "}\n",
        ),
    );
    let syms = top(&mut lsp, &uri);
    let kids = children(&syms[0]);
    let names: Vec<&str> = kids.iter().map(name).collect();
    assert_eq!(names, ["whoami"], "unexpected outline members: {names:?}");
}

// -- tcltest test cases (issue #790) -------------------------------------

#[test]
fn tcltest_imported_test_name_is_a_symbol() {
    // After `namespace import ::tcltest::*`, a bare `test` names a test case
    // that appears in the outline with the test's description as detail.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "package require tcltest\n\
         namespace import ::tcltest::*\n\
         test widget-behaviour-1.1 {the widget behaves} -body { set x 1 } -result 1\n",
    );
    let syms = top(&mut lsp, &uri);
    let sym = syms
        .iter()
        .find(|s| name(s) == "widget-behaviour-1.1")
        .unwrap_or_else(|| panic!("test name not found in {syms:?}"));
    assert_eq!(kind(sym), FUNCTION);
    assert_eq!(detail(sym), "the widget behaves");
}

#[test]
fn tcltest_qualified_test_name_is_a_symbol() {
    // The fully-qualified `tcltest::test` form resolves without an import.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "package require tcltest\n\
         tcltest::test qualified-2.1 {desc} -body { expr 1 } -result 1\n",
    );
    let names = symbol_names(&lsp.document_symbols(&uri));
    assert!(names.contains("qualified-2.1"), "got {names:?}");
}

#[test]
fn tcltest_test_case_is_a_workspace_symbol() {
    // The same test case is navigable via `workspace/symbol`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "package require tcltest\n\
         namespace import ::tcltest::*\n\
         test find-this-case {desc} -body { set x 1 } -result 1\n",
    );
    let result = lsp.workspace_symbols("find-this-case");
    let syms = result.as_array().cloned().unwrap_or_default();
    assert!(
        syms.iter().any(|s| name(s) == "find-this-case"),
        "workspace symbol not found: {syms:?}"
    );
}

#[test]
fn tcltest_constraint_and_match_mode_are_symbols() {
    // `testConstraint` (setter) and `customMatch` each name a definition with
    // its own outline kind: Constant (14) for a constraint, Operator (25) for a
    // custom match mode.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "package require tcltest\n\
         namespace import ::tcltest::*\n\
         testConstraint needsNet 1\n\
         customMatch approx ::approxEq\n",
    );
    let syms = top(&mut lsp, &uri);
    let constraint = syms
        .iter()
        .find(|s| name(s) == "needsNet")
        .unwrap_or_else(|| panic!("constraint not found in {syms:?}"));
    assert_eq!(kind(constraint), CONSTANT);
    let matcher = syms
        .iter()
        .find(|s| name(s) == "approx")
        .unwrap_or_else(|| panic!("match mode not found in {syms:?}"));
    assert_eq!(kind(matcher), OPERATOR);
}

// -- TestSymbolNamesNonEmpty ---------------------------------------------

#[test]
fn all_symbols_have_non_empty_names() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval a { proc b {} { return } }\noo::class create C { method m {} {} }\n";
    lsp.open_ready(&uri, src);
    let flat = flatten_symbols(&lsp.document_symbols(&uri));
    let names: Vec<String> = flat.iter().map(|s| name(s).to_owned()).collect();
    assert!(!names.is_empty());
    assert!(names.iter().all(|n| !n.is_empty()), "{names:?}");
}

// Issue #934: a proc named `:` (legal Tcl — a lone colon is an ordinary name
// character) must surface with its real name.  The 2.1.9 regression collapsed
// the name to the empty string, which VS Code rejects with "name must not be
// falsy", killing the whole outline.
#[test]
fn colon_named_proc_has_a_non_empty_symbol_name() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc : args {\n    return \"hello\"\n}\nproc a:b {} {}\n",
    );
    let syms = top(&mut lsp, &uri);
    assert_eq!(syms.len(), 2, "{syms:?}");
    let names: Vec<String> = syms.iter().map(|s| name(s).to_string()).collect();
    assert!(names.contains(&":".to_string()), "{names:?}");
    assert!(names.contains(&"a:b".to_string()), "{names:?}");
    assert!(
        names.iter().all(|n| !n.is_empty()),
        "no falsy symbol names: {names:?}"
    );
}

// -- TestIrulesEventHandlers ---------------------------------------------
// An iRule's structure is its `when` blocks.  They carried no outline symbol
// at all, so the outline, breadcrumbs and Cmd+Shift+O listed only whatever
// variables the handlers happened to set.

#[test]
fn irule_event_handlers_are_outline_symbols() {
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    lsp.open_ready_lang(
        &uri,
        "when HTTP_REQUEST {\n\
        \x20   set host [HTTP::host]\n\
         }\n\
         when HTTP_RESPONSE priority 500 {\n\
        \x20   HTTP::header insert X-Host $host\n\
         }\n",
        "tcl-irule",
    );
    let syms = top(&mut lsp, &uri);
    let handlers: Vec<String> = syms
        .iter()
        .filter(|s| kind(s) == EVENT)
        .map(|s| name(s).to_owned())
        .collect();
    assert_eq!(handlers, vec!["HTTP_REQUEST", "HTTP_RESPONSE"], "{syms:?}");
}

#[test]
fn variables_set_in_a_handler_nest_under_it() {
    // A `when` body is structural, not a scope, so its `set`s land in the
    // same scope as the handler — they must still nest under it in the tree.
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    lsp.open_ready_lang(
        &uri,
        "when HTTP_REQUEST {\n\x20   set host [HTTP::host]\n}\n",
        "tcl-irule",
    );
    let syms = top(&mut lsp, &uri);
    assert_eq!(syms.len(), 1, "{syms:?}");
    assert_eq!(name(&syms[0]), "HTTP_REQUEST");
    let kids = children(&syms[0]);
    let inner: Vec<&str> = kids.iter().map(name).collect();
    assert_eq!(inner, vec!["host"], "{syms:?}");
}

#[test]
fn event_handler_is_a_workspace_symbol() {
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    lsp.open_ready_lang(
        &uri,
        "when CLIENTSSL_HANDSHAKE {\n\x20   log local0. ok\n}\n",
        "tcl-irule",
    );
    let result = lsp.workspace_symbols("CLIENTSSL_HANDSHAKE");
    let syms = result.as_array().cloned().unwrap_or_default();
    assert!(
        syms.iter().any(|s| name(s) == "CLIENTSSL_HANDSHAKE"),
        "workspace symbol not found: {syms:?}"
    );
}

/// Regression (#1179): the **first** `workspace/symbol` of a session must not
/// answer out of a still-empty index.
///
/// `initialized` pulls the client config and registers file watchers — two
/// client round-trips — before it starts the folder scan, so a query issued
/// straight after the handshake used to find nothing, while the identical
/// query a moment later found everything.  The VS Code suite reproduced this
/// as a flaky first test; this is the same race without an editor.
#[test]
fn workspace_symbol_waits_out_the_startup_scan() {
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-ws-symbol-scan-{}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&root).expect("mk workspace root");
    std::fs::write(root.join("scanned.tcl"), "proc scan_race_helper {} {}\n")
        .expect("write fixture");

    // No config settle, no scan wait, no `didOpen` — the query races the scan.
    let mut lsp = Lsp::at_workspace_root(&root);
    let result = lsp.workspace_symbols("scan_race_helper");
    let syms = result.as_array().cloned().unwrap_or_default();
    assert!(
        syms.iter().any(|s| name(s) == "scan_race_helper"),
        "the on-disk fixture's proc must be found on the first query: {syms:?}"
    );

    std::fs::remove_dir_all(&root).ok();
}

/// Regression (#1179): a document the editor has **just opened** must stay
/// searchable.
///
/// `didOpen` drops the document's index entry (its on-disk records stop being
/// authoritative once a live buffer exists) and the debounced diagnostics
/// publish is what puts it back, so for that window every proc in the
/// just-opened file vanished from the picker.  That is precisely what the VS
/// Code suite hit: `fib` lives only in the file it had opened, so it found
/// nothing, while `factorial` — which a second, unopened fixture also defines
/// — kept working.
#[test]
fn workspace_symbol_finds_a_just_opened_documents_proc() {
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-ws-symbol-open-{}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&root).expect("mk workspace root");
    let path = root.join("procs.tcl");
    let src = "proc open_race_fib {n} { return $n }\n";
    std::fs::write(&path, src).expect("write fixture");

    let mut lsp = Lsp::at_workspace_root(&root);
    let uri = format!("file://{}", path.to_string_lossy());
    // `open_document`, not `open_ready`: do not wait for the publish that
    // re-adds the index entry.
    lsp.open_document(&uri, src);
    let result = lsp.workspace_symbols("open_race_fib");
    let syms = result.as_array().cloned().unwrap_or_default();
    assert!(
        syms.iter().any(|s| name(s) == "open_race_fib"),
        "the open document's proc must be found while its index entry is pending: {syms:?}"
    );

    std::fs::remove_dir_all(&root).ok();
}
