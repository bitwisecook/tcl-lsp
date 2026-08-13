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

//! Call hierarchy, implementation, and linked editing — end-to-end.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::{Value, json};

/// The list of items returned by `prepareCallHierarchy` (null -> empty).
fn items(result: &Value) -> Vec<Value> {
    match result {
        Value::Array(a) => a.clone(),
        Value::Null => Vec::new(),
        other => vec![other.clone()],
    }
}

/// The array of `CallHierarchy{Incoming,Outgoing}Calls` (null -> empty).
fn calls(result: &Value) -> Vec<Value> {
    match result {
        Value::Array(a) => a.clone(),
        _ => Vec::new(),
    }
}

fn name_of(item: &Value) -> String {
    item.get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

// -- TestCallHierarchy ---------------------------------------------------

#[test]
fn prepare_returns_proc_item() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc helper {} { return 1 }\nhelper\n");
    let its = items(&lsp.prepare_call_hierarchy(&uri, 0, 6));
    assert!(!its.is_empty());
    assert_eq!(name_of(&its[0]), "helper");
    assert_eq!(its[0].get("kind").and_then(Value::as_i64), Some(12)); // Function
}

#[test]
fn incoming_calls() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {} { return 1 }\nproc caller {} { helper }\ncaller\n";
    lsp.open_ready(&uri, src);
    let item = items(&lsp.prepare_call_hierarchy(&uri, 0, 6))[0].clone();
    let incoming = calls(&lsp.incoming_calls(item));
    let callers: std::collections::BTreeSet<String> = incoming
        .iter()
        .map(|c| name_of(c.get("from").unwrap_or(&Value::Null)))
        .collect();
    assert!(callers.contains("caller"), "{callers:?}");
}

#[test]
fn outgoing_calls() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {} { return 1 }\nproc caller {} { helper }\ncaller\n";
    lsp.open_ready(&uri, src);
    let item = items(&lsp.prepare_call_hierarchy(&uri, 1, 6))[0].clone();
    let outgoing = calls(&lsp.outgoing_calls(item));
    let callees: std::collections::BTreeSet<String> = outgoing
        .iter()
        .map(|c| name_of(c.get("to").unwrap_or(&Value::Null)))
        .collect();
    assert!(callees.contains("helper"), "{callees:?}");
}

/// Regression: intra-class `TclOO` method call-hierarchy edges must match
/// through `my <method>` dispatch, never a bare head — a method is not a
/// bare-callable command (`greet` alone errors "invalid command name" at
/// runtime; only `my greet` dispatches). The previous bare-head matcher
/// found nothing for this shape and would have (wrongly) matched a bare
/// `greet` call instead.
#[test]
fn method_incoming_and_outgoing_calls_match_my_dispatch() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create C {\n    method greet {} { return hi }\n    method twice {} { my greet ; my greet }\n}\n";
    lsp.open_ready(&uri, src);
    // `greet`'s incoming calls: `twice` dispatches via `my greet` twice.
    let greet_item = items(&lsp.prepare_call_hierarchy(&uri, 1, 11))[0].clone();
    let incoming = calls(&lsp.incoming_calls(greet_item));
    let incoming_from: std::collections::BTreeSet<String> = incoming
        .iter()
        .map(|c| name_of(c.get("from").unwrap_or(&Value::Null)))
        .collect();
    assert!(incoming_from.contains("::C::twice"), "{incoming_from:?}");
    // `twice`'s outgoing calls: dispatches to `greet`.
    let twice_item = items(&lsp.prepare_call_hierarchy(&uri, 2, 11))[0].clone();
    let outgoing = calls(&lsp.outgoing_calls(twice_item));
    let outgoing_to: std::collections::BTreeSet<String> = outgoing
        .iter()
        .map(|c| name_of(c.get("to").unwrap_or(&Value::Null)))
        .collect();
    assert!(outgoing_to.contains("::C::greet"), "{outgoing_to:?}");
}

/// FN→TP (issue #957's general form): a `my method` dispatch nested inside
/// `if` control flow is an outgoing/incoming call edge too — call hierarchy
/// shares the same control-flow-recursing matcher Find-References / the
/// code lens use.
#[test]
fn method_outgoing_calls_nested_in_control_flow() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create C {\n    method greet {} { return hi }\n    method twice {} {\n        if {1} {\n            my greet\n        }\n    }\n}\n";
    lsp.open_ready(&uri, src);
    let twice_item = items(&lsp.prepare_call_hierarchy(&uri, 2, 11))[0].clone();
    let outgoing = calls(&lsp.outgoing_calls(twice_item));
    let callees: std::collections::BTreeSet<String> = outgoing
        .iter()
        .map(|c| name_of(c.get("to").unwrap_or(&Value::Null)))
        .collect();
    assert!(callees.contains("::C::greet"), "{callees:?}");
}

/// FN→TP (issue #995): a `classmethod` dispatches on the class's own
/// command (`Factory make`), so neither its callers nor its callees have
/// its name as a head word.  Incoming on `make` must find the sibling
/// classmethod *and* the top-level statement; outgoing on `build` must find
/// `make`.  tclsh9.0-verified — `Factory build` and the bare `Factory make`
/// both enter `make` (8.6's `TclOO` has no `classmethod` keyword at all).
#[test]
fn classmethod_incoming_and_outgoing_calls_match_bare_class_dispatch() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Factory {\n    classmethod make {} { return 1 }\n    classmethod build {} { Factory make }\n}\nFactory make\n";
    lsp.open_ready(&uri, src);
    // `make`'s incoming calls: `build`'s body, plus the top-level statement.
    let make_item = items(&lsp.prepare_call_hierarchy(&uri, 1, 16))[0].clone();
    let incoming = calls(&lsp.incoming_calls(make_item));
    let incoming_from: std::collections::BTreeSet<String> = incoming
        .iter()
        .map(|c| name_of(c.get("from").unwrap_or(&Value::Null)))
        .collect();
    assert!(
        incoming_from.contains("::Factory::build"),
        "the sibling classmethod's `Factory make` is a caller: {incoming_from:?}"
    );
    assert!(
        incoming_from.contains("<top-level>"),
        "the top-level `Factory make` is a caller: {incoming_from:?}"
    );
    // `build`'s outgoing calls: dispatches to `make`.
    let build_item = items(&lsp.prepare_call_hierarchy(&uri, 2, 16))[0].clone();
    let outgoing = calls(&lsp.outgoing_calls(build_item));
    let outgoing_to: std::collections::BTreeSet<String> = outgoing
        .iter()
        .map(|c| name_of(c.get("to").unwrap_or(&Value::Null)))
        .collect();
    assert!(outgoing_to.contains("::Factory::make"), "{outgoing_to:?}");
}

/// FP guard (issue #995): `Factory make` where `Factory` is an ordinary
/// proc calls *that proc* with the literal argument `make`
/// (tclsh8.6/9.0-verified), so it is no edge at all to an unrelated class's
/// same-named classmethod.
#[test]
fn bare_dispatch_on_a_same_named_proc_is_not_a_classmethod_edge() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Widget {\n    classmethod make {} { return 1 }\n}\nproc Factory {args} { return $args }\nFactory make\n";
    lsp.open_ready(&uri, src);
    let make_item = items(&lsp.prepare_call_hierarchy(&uri, 1, 16))[0].clone();
    let incoming = calls(&lsp.incoming_calls(make_item));
    assert!(
        incoming.is_empty(),
        "a proc call spelled `Factory make` must not edge to Widget's classmethod: {incoming:?}"
    );
}

// -- TestImplementation --------------------------------------------------

#[test]
fn method_implementations_across_subclasses() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Animal {\n    method speak {} {}\n}\noo::class create Dog {\n    superclass Animal\n    method speak {} {}\n}\n";
    lsp.open_ready(&uri, src);
    // Cursor on Animal's `speak` — implementations span both classes.
    let lines = start_lines(&lsp.implementation(&uri, 1, 11));
    assert!(lines.contains(&1), "{lines:?}");
    assert!(lines.contains(&5), "{lines:?}");
}

// -- TestLinkedEditingRange ----------------------------------------------

#[test]
fn recursive_self_calls_linked_with_declaration() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc fib {n} {\n    if {$n < 2} { return $n }\n    return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]\n}\n";
    lsp.open_ready(&uri, src);
    let result = lsp.linked_editing_range(&uri, 0, 6);
    assert!(!result.is_null());
    let ranges = result
        .get("ranges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(ranges.len() >= 2, "{ranges:?}");
    assert!(
        ranges
            .iter()
            .any(|r| r["start"]["line"].as_i64() == Some(0))
    );
    assert!(
        ranges
            .iter()
            .any(|r| r["start"]["line"].as_i64() == Some(2))
    );
    let word_pattern = result
        .get("wordPattern")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        word_pattern.contains("[A-Za-z"),
        "wordPattern: {word_pattern:?}"
    );
}

#[test]
fn non_recursive_proc_returns_none() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {} { return hi }\ngreet\n");
    let result = lsp.linked_editing_range(&uri, 0, 6);
    let empty = result.is_null()
        || result
            .get("ranges")
            .and_then(Value::as_array)
            .is_none_or(std::vec::Vec::is_empty);
    assert!(empty, "{result:?}");
}

// Keep `json` referenced so the mandated import never warns.
#[allow(dead_code)]
const _JSON_USED: fn() -> Value = || json!(null);
