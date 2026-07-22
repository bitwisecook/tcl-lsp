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
    lsp.open_ready(
        &uri,
        "proc greet {name} { puts \"Hello $name\" }\ngreet World\n",
    );
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
fn proc_in_two_level_nested_namespace_via_qualified_call() {
    // Issue #923: go-to-definition on a fully-qualified call to a proc
    // nested two `namespace eval` levels deep must land on its own decl.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval modelTestVerTool {\n    namespace eval gui {\n        proc specAddButtonPopUp {x y} { return \"$x $y\" }\n    }\n}\n::modelTestVerTool::gui::specAddButtonPopUp 1 2\n";
    lsp.open_ready(&uri, src);
    let result = lsp.definition(&uri, 5, 30);
    let locs = locations(&result);
    assert!(!locs.is_empty(), "{result:?}");
    assert_eq!(start_line(&locs[0]), 2);
}

#[test]
fn proc_definition_disambiguates_same_named_procs_in_two_level_nested_namespaces() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval a {\n    namespace eval b {\n        proc helper {} { return 1 }\n    }\n}\nnamespace eval c {\n    namespace eval d {\n        proc helper {} { return 2 }\n    }\n}\n";
    lsp.open_ready(&uri, src);
    let locs_ab = locations(&lsp.definition(&uri, 2, 14));
    assert!(!locs_ab.is_empty(), "{locs_ab:?}");
    assert_eq!(start_line(&locs_ab[0]), 2, "must resolve to ::a::b::helper");
    let locs_cd = locations(&lsp.definition(&uri, 7, 14));
    assert!(!locs_cd.is_empty(), "{locs_cd:?}");
    assert_eq!(start_line(&locs_cd[0]), 7, "must resolve to ::c::d::helper");
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
    let line = u32::try_from(
        lines
            .iter()
            .position(|l| *l == "puts \"fib(10) = [fib 10]\"")
            .expect("target line present"),
    )
    .unwrap();
    // Python: 'puts "fib(10) = ['.index("[") + 1 — '[' is the last char, so
    // its index is len-1 and col is len.
    let col = u32::try_from("puts \"fib(10) = [".len()).unwrap();
    let result = lsp.definition(&uri, line, col);
    let locs = locations(&result);
    assert!(!locs.is_empty());
    assert_eq!(start_line(&locs[0]), 0);
}

// -- TestNamespaceResolution -----------------------------------------------
// C Tcl resolves an unqualified command in the current namespace first, then
// the global namespace (`Tcl_FindCommand`, `tclNamesp.c`) — never a sibling
// namespace picked by proc-table iteration order.

#[test]
fn unqualified_call_resolves_in_callers_namespace_first() {
    // Two namespaces each define `helper`; the unqualified call inside ::b
    // must land on ::b::helper, never ::a::helper.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval a {\n    proc helper {} { return 1 }\n}\nnamespace eval b {\n    proc helper {} { return 2 }\n    helper\n}\n";
    lsp.open_ready(&uri, src);
    // Cursor on the bare `helper` call (line 5).
    let result = lsp.definition(&uri, 5, 6);
    let locs = locations(&result);
    assert!(!locs.is_empty());
    assert_eq!(start_line(&locs[0]), 4, "must resolve to ::b::helper");
}

#[test]
fn global_call_with_single_namespaced_proc_uses_fallback() {
    // Only ::a::helper exists: a global-scope call has no namespace-visible
    // candidate, and `helper` names no builtin, so the lenient fallback
    // resolves the call to the only same-named proc.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval a {\n    proc helper {} { return 1 }\n}\nhelper\n";
    lsp.open_ready(&uri, src);
    let result = lsp.definition(&uri, 3, 2);
    let locs = locations(&result);
    assert!(!locs.is_empty());
    assert_eq!(start_line(&locs[0]), 1, "fallback must reach ::a::helper");
}

#[test]
fn global_call_fallback_is_deterministic_across_repeats() {
    // No ::helper exists, so the lenient fallback fires — and must pick the
    // lexicographically smallest qualified name (::a::helper) on every
    // repeat, never a proc-table-iteration-order hijack (::z::helper is
    // deliberately defined first).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval z {\n    proc helper {} { return 26 }\n}\nnamespace eval a {\n    proc helper {} { return 1 }\n}\nhelper\n";
    lsp.open_ready(&uri, src);
    for attempt in 0..4 {
        let result = lsp.definition(&uri, 6, 2);
        let locs = locations(&result);
        assert!(!locs.is_empty(), "attempt {attempt}: no definition");
        assert_eq!(
            start_line(&locs[0]),
            4,
            "attempt {attempt}: fallback must pick ::a::helper deterministically"
        );
    }
}

// -- TestMathFunctionDefinition -------------------------------------------
// `expr` math-function calls (`sin(...)`) dispatch through the fixed
// `::tcl::mathfunc` sub-namespace, never the calling namespace — a generic
// one-hop resolver that (mis)treated the qualified dispatch name as
// `{callingNamespace}::{name}` could mis-jump to an unrelated same-named
// top-level proc. These pin the fix at the go-to-definition layer, not just
// the underlying resolved-name data.

#[test]
fn no_definition_for_mathfunc_call_despite_unrelated_same_named_proc() {
    // `proc sin` is an ordinary, unrelated top-level command — it has zero
    // effect on `expr {sin(...)}`, which only ever dispatches through
    // `::tcl::mathfunc::sin` (a real Tcl builtin with no source location).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc sin {x} { return bogus }\nset y [expr {sin(1.0)}]\n";
    lsp.open_ready(&uri, src);
    let result = lsp.definition(&uri, 1, 14);
    assert!(
        locations(&result).is_empty(),
        "must not jump to the unrelated proc sin: {result:?}"
    );
}

#[test]
fn mathfunc_call_jumps_to_namespace_local_override() {
    // TIP 232: a namespace-local `proc ::nsa::tcl::mathfunc::pf` shadows the
    // global `::tcl::mathfunc::pf` for a call made from inside `::nsa` (real
    // Tcl behaviour — see the VM's
    // `namespace_local_mathfunc_shadows_global_in_expr`). Go-to-definition
    // on the call must land on the local override, not report nothing.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval ::nsa::tcl::mathfunc {}\n\
               proc ::nsa::tcl::mathfunc::pf {x} { return 20 }\n\
               namespace eval ::nsa {\n    proc caller {} { return [expr {pf(1)}] }\n}\n";
    lsp.open_ready(&uri, src);
    let result = lsp.definition(&uri, 3, 36);
    let locs = locations(&result);
    assert!(!locs.is_empty(), "{result:?}");
    assert_eq!(
        start_line(&locs[0]),
        1,
        "must resolve to the local override"
    );
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

// -- wildcard namespace import bareword resolution (issue #923 idx 18) --
//
// `namespace import NS::*` makes every command `NS` has `namespace
// export`ed callable bare wherever the import is in scope, including
// across files — but real Tcl only imports *exported* names, so an
// unexported sibling in `NS` stays unreachable through the import.

#[test]
fn wildcard_namespace_import_resolves_cross_document_proc() {
    // TP (headline case, cross-document): `lib.tcl` defines and exports
    // `bar`; `main.tcl` wildcard-imports `::Lib::*` and calls `bar` bare.
    // Both files are opened in the same session, exercising the real
    // `textDocument/didOpen` → workspace-index → cross-document
    // go-to-definition path a user's editor would hit.
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(
        &lib_uri,
        "namespace eval Lib {\n    proc bar {} { return 1 }\n    namespace export bar\n}\n",
    );
    let main_uri = unique_uri("tcl");
    lsp.open_ready(&main_uri, "namespace import ::Lib::*\nbar\n");
    let result = lsp.definition(&main_uri, 1, 0);
    let locs = locations(&result);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].uri, lib_uri, "must jump into lib.tcl");
    assert_eq!(start_line(&locs[0]), 1, "proc bar is declared on line 1");
}

#[test]
fn wildcard_namespace_import_does_not_resolve_unexported_sibling_cross_document() {
    // FP guard (CRITICAL, cross-document): `lib.tcl`'s `other` is never
    // exported, so `main.tcl`'s wildcard import must not resolve a bare
    // `other` call to it — matches real tclsh's own `invalid command name`
    // error there (tclsh9.0/8.6-verified; Tcl manual, `namespace` —
    // `namespace import` only imports exported names).
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(
        &lib_uri,
        "namespace eval Lib {\n    proc bar {} { return 1 }\n    proc other {} { return 2 }\n    namespace export bar\n}\n",
    );
    let main_uri = unique_uri("tcl");
    lsp.open_ready(&main_uri, "namespace import ::Lib::*\nother\n");
    let result = lsp.definition(&main_uri, 1, 0);
    assert!(
        locations(&result).is_empty(),
        "an unexported sibling must stay unresolved through the wildcard import: {result:?}"
    );
}

/// idx 33 (differential-audit main audit wave, high severity): a class
/// *instantiation* call (`GSA new`, the real corpus's
/// `georgtree_tclopt`'s `arbitaryTest.tcl` idiom — `GSA new -funct
/// fRastrigin ...`) reached only through a cross-document wildcard
/// `namespace import NS::*`. Same root cause as idx 18 — the finding's
/// own root-cause citation is `WorkspaceIndex::index_command_links`'s
/// glob-pattern skip, the exact mechanism idx 18 fixed — found
/// independently before idx 18 landed. Verified fixed via this reliable
/// `Lsp::tcl()` e2e harness after an *unreliable* CLI-script (`lsp_client.py`)
/// verification pass initially reported this as still broken, even for
/// same-document, non-wildcard, fully-qualified calls that the
/// `tcl-lsp-core::definition::definition()` unit-level test harness (and
/// this e2e test) both proved resolve correctly — the CLI script's result
/// was misleading here (a tooling artifact, not a real regression), so
/// treat any future CLI-only "class instantiation doesn't resolve" report
/// with suspicion until cross-checked against the real server via this
/// harness or the Rust unit-test level.
#[test]
fn class_instantiation_resolves_cross_document_via_wildcard_import() {
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(
        &lib_uri,
        "namespace eval ::optlib {\n    namespace export GSA\n    oo::class create GSA {\n        method run {} { return 1 }\n    }\n}\n",
    );
    let main_uri = unique_uri("tcl");
    // line 1: `set optimizer [GSA new]` — `GSA` starts at column 15.
    lsp.open_ready(
        &main_uri,
        "namespace import ::optlib::*\nset optimizer [GSA new]\n",
    );
    let result = lsp.definition(&main_uri, 1, 15);
    let locs = locations(&result);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].uri, lib_uri, "must jump into the library file");
    assert_eq!(start_line(&locs[0]), 2, "oo::class create GSA is on line 2");
}
