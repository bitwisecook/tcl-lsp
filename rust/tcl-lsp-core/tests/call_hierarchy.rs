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

//! Tests for the LSP call-hierarchy provider
//! (single-document surface). Verifies `prepare` resolves the proc at the
//! cursor, `incoming_calls` finds callers, and `outgoing_calls` finds callees.
//!
//! C-Tcl proof: the call graph is real Tcl — every snippet is a runnable
//! script whose `proc` bodies invoke each other (e.g. `proc main {} { greet }`
//! calls `greet`, and the top level calls `greet`). The cross-document tests
//! (`extra_documents=`) exercise a workspace-index surface absent from this
//! single-document Rust API and are noted as out-of-surface.

use tcl_compiler::analyser::Analyser;
use tcl_lsp_core::call_hierarchy::{CallHierarchyItem, incoming_calls, outgoing_calls, prepare};

fn items(source: &str, line: u32, character: u32) -> Vec<CallHierarchyItem> {
    let mut a = Analyser::new();
    let analysis = a.analyse(source, "tcl8.6");
    prepare(source, line, character, &analysis)
}

fn incoming_names(source: &str, item: &CallHierarchyItem) -> Vec<String> {
    let mut a = Analyser::new();
    let analysis = a.analyse(source, "tcl8.6");
    incoming_calls(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        item,
        &analysis,
    )
    .into_iter()
    .map(|c| c.from.name)
    .collect()
}

fn outgoing_names(source: &str, item: &CallHierarchyItem) -> Vec<String> {
    let mut a = Analyser::new();
    let analysis = a.analyse(source, "tcl8.6");
    outgoing_calls(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        item,
        &analysis,
    )
    .into_iter()
    .map(|c| c.to.name)
    .collect()
}

const GREET: &str = "proc greet {name} { puts \"Hello $name\" }\ngreet World\n";

#[test]
fn prepare_on_proc_definition() {
    let it = items(GREET, 0, 6); // on `greet` in the definition
    assert_eq!(it.len(), 1);
    assert!(it[0].name.contains("greet"));
}

#[test]
fn prepare_on_proc_call() {
    let it = items(GREET, 1, 0); // on the `greet World` call
    assert_eq!(it.len(), 1);
    assert!(it[0].name.contains("greet"));
}

#[test]
fn prepare_on_builtin_is_empty() {
    // A builtin (`puts`) is not a call-hierarchy proc.
    assert!(items("puts hello", 0, 1).is_empty());
}

#[test]
fn prepare_on_empty_file_is_empty() {
    assert!(items("", 0, 0).is_empty());
}

#[test]
fn incoming_calls_find_callers() {
    // tclsh: both `main` and the top level call `greet`.
    let src = "proc greet {} { return }\nproc main {} { greet }\ngreet\n";
    let it = items(src, 0, 6);
    assert_eq!(it.len(), 1);
    let names = incoming_names(src, &it[0]);
    assert!(
        names
            .iter()
            .any(|n| n.contains("main") || n.contains("top")),
        "expected a `main` / top-level caller; got {names:?}"
    );
}

#[test]
fn outgoing_calls_find_callees() {
    // `main` calls `greet` and `puts`.
    let src = "proc greet {} { return }\nproc main {} { greet\n puts hi }\n";
    let main_item = items(src, 1, 6); // on `main`
    assert_eq!(main_item.len(), 1);
    let callees = outgoing_names(src, &main_item[0]);
    assert!(
        callees.iter().any(|n| n.contains("greet")),
        "main should call greet; got {callees:?}"
    );
}

#[test]
fn recursive_proc_calls_itself() {
    // tclsh: `fact` calls itself — outgoing includes greet-of-self.
    let src =
        "proc fact {n} { if {$n <= 1} { return 1 }\n return [expr {$n * [fact [expr {$n-1}]]}] }\n";
    let it = items(src, 0, 6);
    assert_eq!(it.len(), 1);
    let callees = outgoing_names(src, &it[0]);
    assert!(
        callees.iter().any(|n| n.contains("fact")),
        "self-call expected: {callees:?}"
    );
}
