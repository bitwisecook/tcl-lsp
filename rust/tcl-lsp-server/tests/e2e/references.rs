//! Native port of `tests/lsp_e2e/test_references_e2e.py`.
//!
//! Find-references, end-to-end against the packaged server. Full-parity port of
//! the request/response cases. The references result is a list of `Location`s;
//! `starts(&result)` gives the `(line, character)` start set and
//! `start_lines(&result)` the set of start lines.


use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

// -- TestProcReferences --------------------------------------------------

#[test]
fn find_proc_definition_and_calls() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc greet {name} { puts \"Hello $name\" }\ngreet World\ngreet Everyone\n";
    lsp.open_ready(&uri, src);
    let result = lsp.references(&uri, 0, 6, true);
    assert!(starts(&result).len() >= 2);
}

#[test]
fn exclude_declaration() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc greet {name} { puts \"Hello $name\" }\ngreet World\n";
    lsp.open_ready(&uri, src);
    let with_decl = starts(&lsp.references(&uri, 0, 6, true));
    let without = starts(&lsp.references(&uri, 0, 6, false));
    assert!(with_decl.len() >= without.len());
}

#[test]
fn find_indented_proc_call() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {} { return }\n    greet\n");
    assert!(start_lines(&lsp.references(&uri, 0, 6, true)).contains(&1));
}

#[test]
fn find_qualified_proc_call_sites() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval myns {\n    proc helper {} { return 1 }\n}\nmyns::helper\n::myns::helper\n";
    lsp.open_ready(&uri, src);
    let lines = start_lines(&lsp.references(&uri, 1, 10, true));
    assert!(lines.contains(&3), "{lines:?}");
    assert!(lines.contains(&4), "{lines:?}");
}

#[test]
fn find_proc_call_in_nested_braced_body() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc greet {} { return }\nif {1} {\n    greet\n}\n";
    lsp.open_ready(&uri, src);
    assert!(start_lines(&lsp.references(&uri, 0, 6, true)).contains(&2));
}

#[test]
fn qualified_calls_do_not_cross_namespace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval a {\n    proc helper {} { return 1 }\n    helper\n}\nnamespace eval b {\n    proc helper {} { return 2 }\n    helper\n}\na::helper\nb::helper\n";
    lsp.open_ready(&uri, src);
    let lines = start_lines(&lsp.references(&uri, 1, 10, true));
    assert!(lines.contains(&2), "{lines:?}");
    assert!(lines.contains(&8), "{lines:?}");
    assert!(!lines.contains(&6), "{lines:?}");
    assert!(!lines.contains(&9), "{lines:?}");
}

// -- TestVariableReferences ----------------------------------------------

#[test]
fn find_var_refs() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\nputs $x\n");
    let s = starts(&lsp.references(&uri, 1, 7, true));
    let expected: std::collections::BTreeSet<(i64, i64)> = [(0, 4), (1, 5)].into_iter().collect();
    assert_eq!(s, expected);
}

#[test]
fn multiple_var_refs() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\nset x 2\nputs $x\n");
    let s = starts(&lsp.references(&uri, 2, 6, true));
    let expected: std::collections::BTreeSet<(i64, i64)> =
        [(0, 4), (1, 4), (2, 5)].into_iter().collect();
    assert_eq!(s, expected);
}

#[test]
fn no_refs_for_unknown() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts hello\n");
    assert!(starts(&lsp.references(&uri, 0, 6, true)).is_empty());
}

#[test]
fn find_namespace_var_refs() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval myns {\n    variable nsVar 1\n    puts $nsVar\n}\n";
    lsp.open_ready(&uri, src);
    let lines = start_lines(&lsp.references(&uri, 2, 10, true));
    assert!(lines.contains(&1), "{lines:?}");
    assert!(lines.contains(&2), "{lines:?}");
}

#[test]
fn var_refs_respect_shadowing_global_target() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set x 1\nputs $x\nproc demo {} {\n    set x 2\n    puts $x\n}\ndemo\n";
    lsp.open_ready(&uri, src);
    let s = start_lines(&lsp.references(&uri, 1, 6, true));
    let expected: std::collections::BTreeSet<i64> = [0, 1].into_iter().collect();
    assert_eq!(s, expected);
}

#[test]
fn var_refs_respect_shadowing_local_target() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set x 1\nputs $x\nproc demo {} {\n    set x 2\n    puts $x\n}\ndemo\n";
    lsp.open_ready(&uri, src);
    let s = start_lines(&lsp.references(&uri, 4, 10, true));
    let expected: std::collections::BTreeSet<i64> = [3, 4].into_iter().collect();
    assert_eq!(s, expected);
}

// -- TestClassSuperclassMixinReferences ----------------------------------

#[test]
fn superclass_and_mixin_references() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Animal {\n    method speak {} {return noise}\n}\noo::class create Dog {\n    superclass Animal\n    method speak {} {return woof}\n}\noo::define Cat {\n    mixin Animal\n}\n";
    lsp.open_ready(&uri, src);
    let s = starts(&lsp.references(&uri, 0, 17, true));
    assert!(s.contains(&(0, 17)), "definition: {s:?}");
    assert!(s.contains(&(4, 15)), "superclass: {s:?}");
    assert!(s.contains(&(8, 10)), "mixin: {s:?}");
}
