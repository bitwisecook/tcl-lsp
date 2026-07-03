//! Native port of `tests/lsp_e2e/test_document_symbols_e2e.py`.
//!
//! Document symbols, end-to-end against the packaged server. Full-parity port
//! of the Tcl/TclOO cases. Symbol kinds come back as raw LSP integer codes
//! (`SymbolKind`); the named constants here mirror that enum.

mod common;

use common::helpers::*;
use common::{Lsp, unique_uri};

use serde_json::Value;

// LSP SymbolKind integer codes.
const NAMESPACE: i64 = 3;
const CLASS: i64 = 5;
const METHOD: i64 = 6;
const PROPERTY: i64 = 7;
const CONSTRUCTOR: i64 = 9;
const FUNCTION: i64 = 12;
const VARIABLE: i64 = 13;

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
    lsp.open_ready(&uri, "oo::configurable create Point {\n    property x y\n}\n");
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
    lsp.open_ready(&uri, "oo::abstract create Shape {\n    method area {} {}\n}\n");
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
