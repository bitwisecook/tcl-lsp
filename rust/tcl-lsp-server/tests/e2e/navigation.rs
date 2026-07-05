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
