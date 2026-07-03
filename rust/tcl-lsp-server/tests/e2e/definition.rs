//! Native port of `tests/lsp_e2e/test_definition_e2e.py`.
//!
//! Go-to-definition, end-to-end against the packaged server. Ported from the
//! `test_definition.py` cases plus the VS Code `definition.test.ts` scenario
//! (navigate from a call site to the proc).


use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::Value;

/// The start line of a location's `range`.
fn start_line(loc: &Loc) -> i64 {
    loc.range
        .get("start")
        .and_then(|s| s.get("line"))
        .and_then(Value::as_i64)
        .unwrap_or(-1)
}

// -- TestProcDefinition --------------------------------------------------

#[test]
fn jump_to_proc() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {name} { puts \"Hello $name\" }\ngreet World\n");
    let result = lsp.definition(&uri, 1, 2);
    let locs = locations(&result);
    assert!(!locs.is_empty());
    assert_eq!(locs[0].uri, uri);
    assert_eq!(start_line(&locs[0]), 0);
}

#[test]
fn no_definition_for_builtin() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts hello\n");
    let result = lsp.definition(&uri, 0, 2);
    assert!(locations(&result).is_empty());
}

#[test]
fn proc_in_namespace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval myns {\n    proc helper {} { return 1 }\n}\nmyns::helper\n";
    lsp.open_ready(&uri, src);
    let result = lsp.definition(&uri, 3, 7);
    let locs = locations(&result);
    assert!(!locs.is_empty());
    assert_eq!(start_line(&locs[0]), 1);
}

#[test]
fn recursive_call_navigates_to_definition() {
    // Mirrors editors/vscode/src/test/definition.test.ts.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc fib {n} {\n    if {$n < 2} { return $n }\n    return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]\n}\nputs \"fib(10) = [fib 10]\"\n";
    lsp.open_ready(&uri, src);
    // cursor on the [fib 10] call on the last line
    let lines: Vec<&str> = src.split('\n').collect();
    let line = lines
        .iter()
        .position(|l| *l == "puts \"fib(10) = [fib 10]\"")
        .expect("target line present") as u32;
    // Python: 'puts "fib(10) = ['.index("[") + 1 — '[' is the last char, so
    // its index is len-1 and col is len.
    let col = "puts \"fib(10) = [".len() as u32;
    let result = lsp.definition(&uri, line, col);
    let locs = locations(&result);
    assert!(!locs.is_empty());
    assert_eq!(start_line(&locs[0]), 0);
}

// -- TestVariableDefinition ----------------------------------------------

#[test]
fn jump_to_var_definition() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\nputs $x\n");
    let result = lsp.definition(&uri, 1, 7);
    let locs = locations(&result);
    assert!(!locs.is_empty());
    assert_eq!(start_line(&locs[0]), 0);
}

#[test]
fn var_in_proc() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc foo {} {\n    set local 42\n    puts $local\n}\n";
    lsp.open_ready(&uri, src);
    let result = lsp.definition(&uri, 2, 11);
    let locs = locations(&result);
    assert!(!locs.is_empty());
    assert_eq!(start_line(&locs[0]), 1);
}

#[test]
fn no_definition_for_unknown_var() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts $unknown\n");
    let result = lsp.definition(&uri, 0, 8);
    assert!(locations(&result).is_empty());
}

#[test]
fn namespace_var_definition() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval myns {\n    variable nsVar 1\n    puts $nsVar\n}\n";
    lsp.open_ready(&uri, src);
    let result = lsp.definition(&uri, 2, 10);
    let locs = locations(&result);
    assert!(!locs.is_empty());
    assert_eq!(start_line(&locs[0]), 1);
}
