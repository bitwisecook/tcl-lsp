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

//! Native port of `tests/lsp_e2e/test_navigation_e2e.py`.
//!
//! Type-definition and declaration navigation, end-to-end.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};
use serde_json::Value;

// -- TestTypeDefinition --------------------------------------------------

#[test]
fn set_with_class_new() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Point {\n    variable x y\n    constructor {a b} { set x $a; set y $b }\n}\nset p [Point new 1 2]\nputs $p\n";
    lsp.open_ready(&uri, src);
    let locs = locations(&lsp.type_definition(&uri, 5, 6));
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range["start"]["line"].as_i64(), Some(0));
}

#[test]
fn plain_set_returns_empty() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\nputs $x\n");
    assert!(locations(&lsp.type_definition(&uri, 1, 6)).is_empty());
}

#[test]
fn my_call_returns_enclosing_class() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Animal {\n    method speak {} { return \"...\" }\n    method greet {} { my speak }\n}\n";
    lsp.open_ready(&uri, src);
    let locs = locations(&lsp.type_definition(&uri, 2, 26));
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range["start"]["line"].as_i64(), Some(0));
}

// -- TestDeclaration -----------------------------------------------------

#[test]
fn global_var_in_proc_returns_declaration_not_set() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    global foo\n    set foo 42\n    return $foo\n}\n";
    lsp.open_ready(&uri, src);
    let locs = locations(&lsp.declaration(&uri, 3, 12));
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range["start"]["line"].as_i64(), Some(1));
}

#[test]
fn falls_back_to_definition_for_proc() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {name} { puts \"Hello $name\" }\ngreet World\n",
    );
    let locs = locations(&lsp.declaration(&uri, 1, 1));
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range["start"]["line"].as_i64(), Some(0));
}

#[test]
fn scope_isolation_between_procs() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc alpha {} {\n    global shared\n    set shared 1\n}\nproc beta {} {\n    global shared\n    return $shared\n}\n";
    lsp.open_ready(&uri, src);
    let lines = start_lines(&lsp.declaration(&uri, 6, 12));
    assert!(lines.contains(&5));
    assert!(!lines.contains(&1));
}

// M7: a proc-name literal held as a dispatch-table value (consumed by a
// `$table(...)` dispatch) is a real reference — go-to-definition from the
// literal jumps to the proc, and the `$cmd` const-dispatch head resolves too.
#[test]
fn dispatch_table_literal_resolves_to_the_proc_m7() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc do_add {a b} { return [expr {$a + $b}] }\narray set ops {add do_add}\nset k add\n$ops($k) 1 2\n";
    lsp.open_ready(&uri, src);
    // Cursor inside the `do_add` table value literal (line 1, col 20).
    let lines = start_lines(&lsp.definition(&uri, 1, 20));
    assert!(
        lines.contains(&0) && lines.len() == 1,
        "table literal must resolve to the proc: {lines:?}"
    );
    // References from the declaration include the table literal's line.
    let refs = start_lines(&lsp.references(&uri, 0, 6, true));
    assert!(
        refs.contains(&1),
        "table literal is a reference site: {refs:?}"
    );
}

// M9: `source` evaluates the file in the caller's namespace — a bare
// `proc helper` in a file sourced inside `namespace eval ::x` is really
// `::x::helper`, so a correctly-qualified cross-file call resolves to it and
// the sourced declaration's references reach the qualified callers.
#[test]
fn sourced_file_resolves_under_the_source_site_namespace_m9() {
    let mut lsp = Lsp::tcl();
    let b = unique_uri("tcl");
    let b_name = b.rsplit('/').next().unwrap().to_owned();
    lsp.open_ready(&b, "proc helper {} {}\n");
    let a = unique_uri("tcl");
    let a_src = format!("namespace eval ::x {{ source {b_name} }}\n::x::helper\n");
    lsp.open_ready(&a, &a_src);

    // Go-to-definition on the `::x::helper` call jumps into the sourced file.
    let defs = crate::common::helpers::locations(&lsp.definition(&a, 1, 4));
    assert!(
        defs.iter().any(|l| l.uri == b
            && l.range
                .get("start")
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
                == Some(0)),
        "::x::helper must resolve into the sourced file: {defs:?}"
    );

    // References from the sourced declaration reach the qualified caller.
    let refs = crate::common::helpers::locations(&lsp.references(&b, 0, 6, false));
    assert!(
        refs.iter().any(|l| l.uri == a
            && l.range
                .get("start")
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
                == Some(1)),
        "the qualified call is a reference of the sourced declaration: {refs:?}"
    );
}
