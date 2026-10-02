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

//! Go-to-definition, end-to-end against the packaged server. Covers the plain
//! Tcl and `TclOO` cases plus the VS Code `definition.test.ts` scenario (navigate
//! from a call site to the proc).

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
    // Go-to-definition on a fully-qualified call to a proc
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
    // `[` is the last character of the prefix, so the prefix length is the
    // column just past it — the first character of the `fib` call inside the
    // command substitution.
    let col = u32::try_from("puts \"fib(10) = [".len()).unwrap();
    let result = lsp.definition(&uri, line, col);
    let locs = locations(&result);
    assert!(!locs.is_empty());
    assert_eq!(start_line(&locs[0]), 0);
}

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

// Wildcard namespace import bareword resolution.
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

// Per-import-site export snapshots, cross-document.
//
// `namespace import` binds the names exported *when it runs*. A later
// `namespace export -clear` does not revoke the alias, and a later
// `namespace export` does not create one. Both oracle-pinned against
// tclsh 8.6.14 and 9.0.4; see `tcl_lsp_core::namespace_import` for the
// transcripts. Ordering only exists within one document — which of two
// files loads first is not a static fact — so both cases put the ordered
// pair in the same file.

#[test]
fn wildcard_import_survives_a_later_export_clear_cross_document() {
    // TP, direction A: `main.tcl` imports `::Lib::*` and then, further down
    // the same file, clears `::Lib`'s exports. Real Tcl keeps the alias
    // (`::dst::bar` still runs); before the per-import-site snapshot the
    // resolver read `::Lib`'s final export state and stopped resolving the
    // call entirely.
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(
        &lib_uri,
        "namespace eval Lib {\n    proc bar {} { return 1 }\n    namespace export bar\n}\n",
    );
    let main_uri = unique_uri("tcl");
    lsp.open_ready(
        &main_uri,
        "namespace import ::Lib::*\nbar\nnamespace eval Lib {\n    namespace export -clear\n}\n",
    );
    let result = lsp.definition(&main_uri, 1, 0);
    let locs = locations(&result);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].uri, lib_uri, "must still jump into lib.tcl");
    assert_eq!(start_line(&locs[0]), 1, "proc bar is declared on line 1");
}

#[test]
fn wildcard_import_ignores_an_export_written_after_it_cross_document() {
    // FP guard (CRITICAL), direction B: `lib.tcl` exports nothing;
    // `main.tcl` imports `::Lib::*` and only *afterwards* exports `bar`.
    // Real Tcl never binds the alias (`invalid command name`), so the bare
    // call must not resolve to `::Lib::bar`.
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(
        &lib_uri,
        "namespace eval Lib {\n    proc bar {} { return 1 }\n}\n",
    );
    let main_uri = unique_uri("tcl");
    lsp.open_ready(
        &main_uri,
        "namespace import ::Lib::*\nbar\nnamespace eval Lib {\n    namespace export bar\n}\n",
    );
    let result = lsp.definition(&main_uri, 1, 0);
    assert!(
        locations(&result).is_empty(),
        "an export written after the import must not resolve the call \
         retroactively: {result:?}"
    );
}

// The import edge's lifecycle, cross-document.
//
// The edge is a per-import-site *snapshot*; these pin it as a link
// with a lifetime. Rows oracle-confirmed byte-identically on tclsh 9.0.4 and
// 8.6.14 (transcripts in `tcl_lsp_core::namespace_import`). Ordering exists
// only within one document, so every ordered pair sits in the same file.

#[test]
fn a_forgotten_wildcard_import_stops_resolving_cross_document() {
    // TN: `main.tcl` imports `::Lib::*`, forgets
    // it, and only then calls `bar`. Oracle: `namespace forget ::Lib::bar`
    // empties `info commands` of the alias and the later bare call raises
    // `invalid command name "bar"`.
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(
        &lib_uri,
        "namespace eval Lib {\n    proc bar {} { return 1 }\n    namespace export bar\n}\n",
    );
    let main_uri = unique_uri("tcl");
    lsp.open_ready(
        &main_uri,
        "namespace import ::Lib::*\nnamespace forget ::Lib::bar\nbar\n",
    );
    let result = lsp.definition(&main_uri, 2, 0);
    assert!(
        locations(&result).is_empty(),
        "a call after `namespace forget` must not resolve through the alias: {result:?}"
    );
}

#[test]
fn a_call_before_the_forget_still_resolves_cross_document() {
    // TP — the forget is order-gated, not file-wide: the same file, with the
    // call written between the import and the forget, still jumps into
    // `lib.tcl`.
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(
        &lib_uri,
        "namespace eval Lib {\n    proc bar {} { return 1 }\n    namespace export bar\n}\n",
    );
    let main_uri = unique_uri("tcl");
    lsp.open_ready(
        &main_uri,
        "namespace import ::Lib::*\nbar\nnamespace forget ::Lib::bar\n",
    );
    let result = lsp.definition(&main_uri, 1, 0);
    let locs = locations(&result);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].uri, lib_uri);
    assert_eq!(start_line(&locs[0]), 1, "proc bar is declared on line 1");
}

#[test]
fn a_forced_import_shadows_the_local_command_cross_document() {
    // TP: `main.tcl` defines its own `bar`, then
    // `namespace import -force ::Lib::*`. Oracle: the local command is
    // replaced, the later bare call runs `::Lib::bar`, and `namespace origin
    // ::bar` answers `::Lib::bar` — so go-to-definition must land in
    // `lib.tcl`, not on the local `proc bar`.
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(
        &lib_uri,
        "namespace eval Lib {\n    proc bar {} { return 1 }\n    namespace export bar\n}\n",
    );
    let main_uri = unique_uri("tcl");
    lsp.open_ready(
        &main_uri,
        "proc bar {} { return 0 }\nnamespace import -force ::Lib::*\nbar\n",
    );
    let result = lsp.definition(&main_uri, 2, 0);
    let locs = locations(&result);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].uri, lib_uri,
        "a `-force` import replaces the local command: {locs:?}"
    );
}

#[test]
fn an_unforced_conflicting_import_leaves_the_local_command_cross_document() {
    // FP guard for the row above — without `-force` the same program raises
    // `can't import command "bar": already exists` and installs nothing, so
    // the call still reaches the *local* definition (line 0), never
    // `lib.tcl`.
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(
        &lib_uri,
        "namespace eval Lib {\n    proc bar {} { return 1 }\n    namespace export bar\n}\n",
    );
    let main_uri = unique_uri("tcl");
    lsp.open_ready(
        &main_uri,
        "proc bar {} { return 0 }\nnamespace import ::Lib::*\nbar\n",
    );
    let result = lsp.definition(&main_uri, 2, 0);
    let locs = locations(&result);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].uri, main_uri,
        "a failed import leaves the local definition in place: {locs:?}"
    );
    assert_eq!(start_line(&locs[0]), 0);
}

#[test]
fn a_wildcard_import_chain_follows_to_the_original_source_cross_document() {
    // TP: three files — `::C` defines and exports
    // `p`, `::B` imports `::C::*` and re-exports, `main.tcl` imports
    // `::B::*` and calls `p` bare. Oracle: the call runs `::C`'s body and
    // `namespace origin` answers `::C::p`, so definition must jump to
    // `c.tcl`. The middle hop is in no proc table, so a naive lookup would
    // abstain entirely.
    let mut lsp = Lsp::tcl();
    let c_uri = unique_uri("tcl");
    lsp.open_ready(
        &c_uri,
        "namespace eval C {\n    proc p {} { return CP }\n    namespace export p\n}\n",
    );
    let b_uri = unique_uri("tcl");
    lsp.open_ready(
        &b_uri,
        "namespace eval B {\n    namespace import ::C::*\n    namespace export p\n}\n",
    );
    let main_uri = unique_uri("tcl");
    lsp.open_ready(&main_uri, "namespace import ::B::*\np\n");
    let result = lsp.definition(&main_uri, 1, 0);
    let locs = locations(&result);
    assert_eq!(locs.len(), 1, "the chain must resolve: {locs:?}");
    assert_eq!(locs[0].uri, c_uri, "must follow ::B through to ::C");
    assert_eq!(start_line(&locs[0]), 1, "proc p is declared on line 1");
}

#[test]
fn deleting_the_source_command_kills_the_import_cross_document() {
    // TN: the alias holds the command *object*, so
    // `rename ::Lib::bar {}` makes the later bare call an `invalid command
    // name`. A plain rename would not — that row is pinned as a unit test in
    // `tcl_lsp_core::definition`.
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(
        &lib_uri,
        "namespace eval Lib {\n    proc bar {} { return 1 }\n    namespace export bar\n}\n",
    );
    let main_uri = unique_uri("tcl");
    lsp.open_ready(
        &main_uri,
        "namespace import ::Lib::*\nrename ::Lib::bar {}\nbar\n",
    );
    let result = lsp.definition(&main_uri, 2, 0);
    assert!(
        locations(&result).is_empty(),
        "destroying the source command destroys the alias: {result:?}"
    );
}

/// A class *instantiation* call (`GSA new`, the `georgtree_tclopt`
/// `arbitaryTest.tcl` idiom — `GSA new -funct fRastrigin ...`) must
/// resolve when reached only through a cross-document wildcard
/// `namespace import NS::*`, the same as a plain proc call.
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

/// A class created via `oo::class create` with no body, then extended by
/// every one of its methods through a *separate*, later `oo::define
/// ClassName { ... }` block — the real corpus shape (`ticklecharts::chart`:
/// `oo::class create` at one line, every method — including the `my
/// AddBarSeries`-style internal dispatch calls in its switch arms — added
/// via a later, separate `oo::define` block). tclsh 9.0/8.6 both prove `my
/// Helper` genuinely dispatches to `Helper` here, so `ClassDef::body_span`
/// must cover every block recorded for the class, not just the first, or
/// a cursor inside the separate `oo::define` block's own text fails the
/// "which class am I lexically inside" containment check `my`-dispatch
/// resolution depends on.
#[test]
fn my_dispatch_resolves_when_class_extended_via_separate_oo_define() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    method Helper {} { return hi }\n    method Caller {} { my Helper }\n}\n",
    );
    // Line 5: `    method Caller {} { my Helper }` — cursor on the `Helper`
    // word inside `my Helper` (column 26).
    let result = lsp.definition(&uri, 5, 26);
    let locs = locations(&result);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        start_line(&locs[0]),
        4,
        "must resolve to the Helper method declaration on line 4"
    );
}

/// The real, unmodified `docs/pixdoc.tcl` shape — a parallel/lock-step
/// multi-list `foreach dirName {...} name {...} {...}` followed, ~300
/// lines later, by a wholly unrelated `foreach name {...}` reusing the
/// same bare name. `handle_foreach_command` must bind every varList, not
/// just the first, or the first loop's own `name` is never a tracked
/// variable and go-to-definition on any `$name` use inside the first
/// loop's body silently resolves to the coincidentally same-named second
/// loop instead, ~300 lines away in the wrong part of the file.
#[test]
fn multi_list_foreach_name_resolves_to_its_own_loop_not_a_later_unrelated_one() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "foreach dirName {src src {src core}} name {alpha beta gamma} {\n    puts \"$dirName $name\"\n    if {$name eq \"pixutils\"} { puts skip }\n}\nforeach name {examples color changes} {\n    puts $name.ruff\n}\n",
    );
    // Line 1: `    puts "$dirName $name"` — cursor on the first loop's own
    // `$name` read (column 21).
    let result = lsp.definition(&uri, 1, 21);
    let locs = locations(&result);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        start_line(&locs[0]),
        0,
        "must resolve to the first loop's own `name` clause (line 0), not the unrelated second loop (line 4): {locs:?}"
    );
}

/// The real `tk/library/systray.tcl` (and `print.tcl`, `fileicon.tcl`,
/// `accessibility.tcl`) idiom splices `systray` into the pre-existing,
/// registry-builtin `tk` ensemble at runtime via `namespace ensemble
/// configure tk -map [dict merge [namespace ensemble configure tk -map]
/// {systray ::tk::systray}]` — a `CONFIGURE`, not `CREATE`, statement,
/// which the analyser must not treat as invisible. tclsh 9.0/8.6 both
/// prove `tk systray create` really calls `::tk::systray`; falling
/// through to `fallback_proc_by_simple_name` would wrongly resolve to a
/// same-tail-name decoy proc in an unrelated namespace.
#[test]
fn tk_ensemble_configure_splice_resolves_to_the_real_target_not_a_decoy() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval ::decoy {\n    proc systray {args} { return \"DECOY\" }\n}\nproc ::tk::systray {args} { return \"real systray: $args\" }\nnamespace ensemble configure tk -map [dict merge [namespace ensemble configure tk -map] {systray ::tk::systray}]\ntk systray create -image book\n",
    );
    // Line 5: `tk systray create -image book` — cursor on "systray".
    let result = lsp.definition(&uri, 5, 5);
    let locs = locations(&result);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        start_line(&locs[0]),
        3,
        "must resolve to the real `proc ::tk::systray` (line 3), not the same-tail-name decoy inside ::decoy (line 1): {locs:?}"
    );
}

/// The real `tk/library/accessibility.tcl` idiom renames each classic
/// widget command away and reinstalls a wrapper proc under the same
/// original name, once per element of a literal `foreach` list —
/// `foreach wtype {button entry ...} { rename ::$wtype ::tk::accessible::
/// orig_$wtype ; proc ::$wtype {args} {...} }`. tclsh 9.0/8.6 both prove
/// `button` is the *new* wrapper afterwards; the old body survives only as
/// `::tk::accessible::orig_button`. The dynamic `$wtype` name must be
/// constant-folded the same as `rename`'s own operands, or both `rename`
/// and `proc` register under garbled literal text and go-to-definition on
/// a `button` call site falls through to the stale, pre-rename `proc
/// button` declaration.
#[test]
fn foreach_rename_reinstall_idiom_resolves_to_the_wrapper_not_the_stale_original() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc button {args} {return orig_button}\nproc entry {args} {return orig_entry}\nnamespace eval ::tk::accessible {\n    foreach wtype {button entry} {\n        rename ::$wtype ::tk::accessible::orig_$wtype\n        proc ::$wtype {args} {return wrapped}\n    }\n}\nset r1 [button .b1]\nset r2 [entry .e1]\n",
    );
    // Line 8: `set r1 [button .b1]` — cursor on "button".
    let result = lsp.definition(&uri, 8, 9);
    let locs = locations(&result);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        start_line(&locs[0]),
        5,
        "must resolve to the wrapper's own `proc ::$wtype` declaration (line 5) inside the loop, not the stale original `proc button` (line 0): {locs:?}"
    );
}

/// Go-to-definition follows a `rename`.
/// tclsh 9.0.4 and 8.6.16 both prove `hello` runs `greet`'s body after
/// `rename greet hello` (and that `greet` is then `invalid command name`),
/// so the call site's declaration is the original `proc greet` header.
#[test]
fn definition_follows_a_rename_to_the_original_proc_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {} { return hi }\nrename greet hello\nhello\n",
    );
    // Line 2: `hello` — cursor on the renamed-to call.
    let locs = locations(&lsp.definition(&uri, 2, 1));
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        start_line(&locs[0]),
        0,
        "the renamed-to call resolves to `proc greet` (line 0): {locs:?}"
    );
}

/// The order-gated other direction of the same fact: a call written *before*
/// the rename is `invalid command name "hello"` on both tclsh 9.0.4 and
/// 8.6.16, so there is nothing to navigate to and the provider must abstain
/// rather than resolve backwards through a mutation that has not run.
#[test]
fn definition_declines_a_rename_written_after_the_call_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {} { return hi }\nhello\nrename greet hello\n",
    );
    let locs = locations(&lsp.definition(&uri, 1, 1));
    assert!(
        locs.is_empty(),
        "a rename that has not run yet must not resolve the call: {locs:?}"
    );
}

/// `interp alias {} NAME {}
/// TARGET` silently *replaces* an existing command of that name — the real
/// `tk/library/accessibility.tcl` `interp alias {} ::ttk::spinbox {}
/// ::tk::spinbox` trick. tclsh 9.0.4 and 8.6.16 both print `classic
/// tk::spinbox:` for a later `::ttk::spinbox` call, so the original proc is
/// dead code under that name and the call's definition is the alias target.
#[test]
fn definition_prefers_an_alias_over_the_proc_it_replaced_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval ::ttk {}\nnamespace eval ::tk {}\nproc ::ttk::spinbox {w args} { }\nproc ::tk::spinbox {w args} { }\ninterp alias {} ::ttk::spinbox {} ::tk::spinbox\n::ttk::spinbox .sb\n",
    );
    // Line 5: `::ttk::spinbox .sb` — cursor on the call's head.
    let locs = locations(&lsp.definition(&uri, 5, 2));
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        start_line(&locs[0]),
        3,
        "must resolve to the alias target `proc ::tk::spinbox` (line 3), not the replaced `::ttk::spinbox` (line 2): {locs:?}"
    );
}

/// A proc redefined later in the
/// same document is two definitions sharing one name. tclsh 9.0.4 and 8.6.16
/// both prove the call *between* them runs the first body, so it must
/// resolve to the first header even though the map keeps only the second.
#[test]
fn definition_between_two_declarations_reaches_the_first_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc p {} { return first }\np\nproc p {a} { return $a }\np x\n",
    );
    let first = locations(&lsp.definition(&uri, 1, 0));
    assert_eq!(first.len(), 1, "{first:?}");
    assert_eq!(
        start_line(&first[0]),
        0,
        "the in-between call reaches the first definition: {first:?}"
    );
    let second = locations(&lsp.definition(&uri, 3, 0));
    assert_eq!(second.len(), 1, "{second:?}");
    assert_eq!(
        start_line(&second[0]),
        2,
        "the trailing call reaches the redefinition: {second:?}"
    );
}

/// `tcl::OptProc` needs its own `AnalyserHookId` — without one,
/// go-to-definition from a real call site falls through to nothing (the
/// stub proc's stale, unresolved `ProcDef` is never overwritten).
#[test]
fn opt_proc_call_site_resolves_to_its_declaration_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "package require opt\n::tcl::OptProc greet {child -use -display} { return $child }\ngreet foo\n",
    );
    // Line 2: `greet foo` — cursor on "greet".
    let result = lsp.definition(&uri, 2, 0);
    let locs = locations(&result);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        start_line(&locs[0]),
        1,
        "must resolve to the tcl::OptProc declaration (line 1): {locs:?}"
    );
}

// The `source` graph orders what the file boundary did not.
//
// Sourcing a file inlines its whole body at the `source` statement's
// position, so the DFS of the `source` forest *is* the run order and an
// import in one file is genuinely ordered against an export in another.
// Oracle, byte-identical on tclsh 8.6.14 and 9.0.4, with
//
//   # mod.tcl: namespace eval Lib { proc bar {} {return 1} }
//   # exp.tcl: namespace eval Lib { namespace export bar }
//   # imp.tcl: namespace import ::Lib::*
//
//   # app.tcl: source mod.tcl; source exp.tcl; source imp.tcl
//   bar   -> 1
//   # app.tcl: source mod.tcl; source imp.tcl; source exp.tcl
//   bar   -> invalid command name "bar"

/// The four documents the source-order end-to-end tests share, under one
/// directory so the relative `source` literals resolve against each other.
fn source_order_uris(tag: &str) -> (String, String, String, String) {
    let dir = format!("file:///e2e/order_{}_{tag}", std::process::id());
    (
        format!("{dir}/mod.tcl"),
        format!("{dir}/exp.tcl"),
        format!("{dir}/imp.tcl"),
        format!("{dir}/app.tcl"),
    )
}

#[test]
fn an_export_sourced_before_the_import_resolves_end_to_end() {
    // TP: the export's file is sourced first, so the import really did see
    // it and the bare call jumps into mod.tcl.
    let mut lsp = Lsp::tcl();
    let (mod_uri, exp_uri, imp_uri, app_uri) = source_order_uris("tp");
    lsp.open_ready(
        &mod_uri,
        "namespace eval Lib {\n    proc bar {} { return 1 }\n}\n",
    );
    lsp.open_ready(
        &exp_uri,
        "namespace eval Lib {\n    namespace export bar\n}\n",
    );
    lsp.open_ready(&imp_uri, "namespace import ::Lib::*\n");
    lsp.open_ready(
        &app_uri,
        "source mod.tcl\nsource exp.tcl\nsource imp.tcl\nbar\n",
    );
    let locs = locations(&lsp.definition(&app_uri, 3, 0));
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].uri, mod_uri, "must jump into mod.tcl");
    assert_eq!(start_line(&locs[0]), 1, "proc bar is declared on line 1");
}

#[test]
fn an_export_sourced_after_the_import_does_not_resolve_end_to_end() {
    // FP guard (CRITICAL): identical documents, only app.tcl's two `source`
    // lines swapped. The import now runs before `::Lib` has exported
    // anything, so real Tcl installs no alias and the bare call is an
    // `invalid command name` — go-to-definition must find nothing.
    let mut lsp = Lsp::tcl();
    let (mod_uri, exp_uri, imp_uri, app_uri) = source_order_uris("fp");
    lsp.open_ready(
        &mod_uri,
        "namespace eval Lib {\n    proc bar {} { return 1 }\n}\n",
    );
    lsp.open_ready(
        &exp_uri,
        "namespace eval Lib {\n    namespace export bar\n}\n",
    );
    lsp.open_ready(&imp_uri, "namespace import ::Lib::*\n");
    lsp.open_ready(
        &app_uri,
        "source mod.tcl\nsource imp.tcl\nsource exp.tcl\nbar\n",
    );
    let result = lsp.definition(&app_uri, 3, 0);
    assert!(
        locations(&result).is_empty(),
        "an export sourced after the import cannot apply retroactively: {result:?}"
    );
}

#[test]
fn find_references_agrees_with_the_source_order() {
    // The two tiers must agree on the LSP surfaces, not only in the
    // resolver: with the export sourced first, `proc bar`'s references
    // include the bare call in app.tcl; with it sourced last, they do not.
    let mut lsp = Lsp::tcl();
    let (mod_uri, exp_uri, imp_uri, app_uri) = source_order_uris("refs");
    lsp.open_ready(
        &mod_uri,
        "namespace eval Lib {\n    proc bar {} { return 1 }\n}\n",
    );
    lsp.open_ready(
        &exp_uri,
        "namespace eval Lib {\n    namespace export bar\n}\n",
    );
    lsp.open_ready(&imp_uri, "namespace import ::Lib::*\n");
    lsp.open_ready(
        &app_uri,
        "source mod.tcl\nsource exp.tcl\nsource imp.tcl\nbar\n",
    );
    let refs = locations(&lsp.references(&mod_uri, 1, 9, false));
    assert!(
        refs.iter().any(|l| l.uri == app_uri),
        "the bare call reached through the sourced import is a reference: {refs:?}"
    );

    let mut lsp = Lsp::tcl();
    let (mod_uri, exp_uri, imp_uri, app_uri) = source_order_uris("refs_fp");
    lsp.open_ready(
        &mod_uri,
        "namespace eval Lib {\n    proc bar {} { return 1 }\n}\n",
    );
    lsp.open_ready(
        &exp_uri,
        "namespace eval Lib {\n    namespace export bar\n}\n",
    );
    lsp.open_ready(&imp_uri, "namespace import ::Lib::*\n");
    lsp.open_ready(
        &app_uri,
        "source mod.tcl\nsource imp.tcl\nsource exp.tcl\nbar\n",
    );
    let refs = locations(&lsp.references(&mod_uri, 1, 9, false));
    assert!(
        !refs.iter().any(|l| l.uri == app_uri),
        "a call the import never bound is not a reference: {refs:?}"
    );
}

#[test]
fn a_computed_source_path_keeps_the_pre_graph_abstention_end_to_end() {
    // TN for the deliberate abstention: `source $dir/exp.tcl` names no
    // document statically, so no edge is built and the export goes back to
    // being unrankable — the same as when no `source` graph exists at all.
    // Same document text as the FP guard above otherwise.
    let mut lsp = Lsp::tcl();
    let (mod_uri, exp_uri, imp_uri, app_uri) = source_order_uris("computed");
    lsp.open_ready(
        &mod_uri,
        "namespace eval Lib {\n    proc bar {} { return 1 }\n}\n",
    );
    lsp.open_ready(
        &exp_uri,
        "namespace eval Lib {\n    namespace export bar\n}\n",
    );
    lsp.open_ready(&imp_uri, "namespace import ::Lib::*\n");
    lsp.open_ready(
        &app_uri,
        "set dir /nowhere\nsource mod.tcl\nsource imp.tcl\nsource $dir/exp.tcl\nbar\n",
    );
    let locs = locations(&lsp.definition(&app_uri, 4, 0));
    assert_eq!(
        locs.len(),
        1,
        "an unprovable `source` path must not be given an order: {locs:?}"
    );
    assert_eq!(locs[0].uri, mod_uri);
}

// The in-document `-force` shadow needs whole-program
// export knowledge.
//
// `PARTLY_OBSERVABLE_MAIN` below is one document, byte-for-byte identical in
// both tests of this pair. `::src` is *partly* observable in it: its procs and
// one of its exports (`other`) are right there, while whether it also exports
// `helper` is decided in another file. Oracle (tclsh 8.6.14 and 9.0.4,
// byte-identical), loading this document with and without a sibling file
// holding `namespace eval ::src {namespace export helper}`:
//
//   with it:     call -> SRC     origin -> ::src::helper
//   without it:  call -> LOCAL   origin -> ::app::helper
//
// No rule reading only this document can separate those, which is why the
// single-document resolver now takes a whole-program export oracle.
//
//  line 0  namespace eval src {
//  line 1      proc helper {a b} { return SRC }
//  line 2      proc other {} { return O }
//  line 3      namespace export other
//  line 4  }
//  line 5  namespace eval app {
//  line 6      proc helper {} { return LOCAL }
//  line 7  }
//  line 8  namespace eval app {
//  line 9      namespace import -force ::src::*
//  line 10 }
//  line 11 namespace eval app {
//  line 12     helper
//  line 13 }
const PARTLY_OBSERVABLE_MAIN: &str = "namespace eval src {\n    proc helper {a b} { return SRC }\n    proc other {} { return O }\n    namespace export other\n}\nnamespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";

#[test]
fn a_forced_import_shadows_when_another_file_exports_the_name_end_to_end() {
    // TP (CRITICAL) — with `::src`'s `namespace export helper` in a sibling
    // file the `-force` import really did delete `::app::helper`, so the call
    // reaches `::src::helper` (line 1), not the local one (line 6).
    let mut lsp = Lsp::tcl();
    let exports_uri = unique_uri("tcl");
    lsp.open_ready(
        &exports_uri,
        "namespace eval src {\n    namespace export helper\n}\n",
    );
    let main_uri = unique_uri("tcl");
    lsp.open_ready(&main_uri, PARTLY_OBSERVABLE_MAIN);
    let locs = locations(&lsp.definition(&main_uri, 12, 4));
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].uri, main_uri);
    assert_eq!(
        start_line(&locs[0]),
        1,
        "the `-force` import replaced the local `helper`: {locs:?}"
    );
    // Hover reads the same resolution, so it must name the same proc — the
    // parameter lists differ precisely so the two are distinguishable.
    let hover = hover_text(&lsp.hover(&main_uri, 12, 4));
    assert!(
        hover.contains('a') && hover.contains('b'),
        "hover must describe `::src::helper {{a b}}`: {hover:?}"
    );
}

#[test]
fn a_forced_import_of_an_unexported_name_keeps_the_local_end_to_end() {
    // TN, byte-identical `main.tcl` — nothing in the program exports
    // `helper`, so the `-force` import binds only `other` and the local
    // definition (line 6) survives. The pinned true negative: this is the
    // case the document-only rule got right and must keep getting right.
    let mut lsp = Lsp::tcl();
    let unrelated_uri = unique_uri("tcl");
    lsp.open_ready(
        &unrelated_uri,
        "namespace eval other {\n    proc q {} { return Q }\n}\n",
    );
    let main_uri = unique_uri("tcl");
    lsp.open_ready(&main_uri, PARTLY_OBSERVABLE_MAIN);
    let locs = locations(&lsp.definition(&main_uri, 12, 4));
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].uri, main_uri);
    assert_eq!(
        start_line(&locs[0]),
        6,
        "an import that binds nothing leaves the local definition: {locs:?}"
    );
    let hover = hover_text(&lsp.hover(&main_uri, 12, 4));
    assert!(
        !hover.contains("a b"),
        "hover must describe the local `helper {{}}`: {hover:?}"
    );
}

#[test]
fn a_forced_import_from_a_wholly_foreign_namespace_shadows_end_to_end() {
    // TP — the third oracle shape: `::src`'s proc *and* its export are in
    // another file, so this document holds only the `-force` import and the
    // local proc it deletes. Oracle: SRC / `::src::helper`. Go-to-definition
    // must leave this file rather than answer the deleted local definition.
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(
        &lib_uri,
        "namespace eval src {\n    proc helper {} { return SRC }\n    namespace export helper\n}\n",
    );
    let main_uri = unique_uri("tcl");
    lsp.open_ready(
        &main_uri,
        "namespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n",
    );
    let locs = locations(&lsp.definition(&main_uri, 7, 4));
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].uri, lib_uri,
        "the deleted local definition must not be the answer: {locs:?}"
    );
    assert_eq!(start_line(&locs[0]), 1);
}

// Every provider the export oracle was threaded through,
// end to end over the packaged server.
//
// `SHADOW_MAIN` is byte-identical in both directions; only the sibling
// document differs, so these pin the server's *wiring* (does each provider
// actually receive `export_snapshot()`?) rather than the core resolver, which
// `tcl-lsp-core/tests/force_import_shadow_consumers.rs` covers.
//
// C-Tcl proof (tclsh 8.6.14 and tclsh 9.0.4 agreeing): with the sibling's
// `namespace export helper` present, `helper 1 2` prints `SRC/1/2`; without
// it, `LOCAL`.  The call is at global scope because that is the one importing
// namespace whose call sites the inlay-hint segmenter reaches.

//  line 0  namespace eval src {
//  line 1      proc helper {alpha beta} { puts "SRC/$alpha/$beta" }
//  line 2      proc other {} { puts O }
//  line 3      namespace export other
//  line 4  }
//  line 5  proc helper {args} { puts LOCAL }
//  line 6  namespace import -force ::src::*
//  line 7  helper 1 2
const SHADOW_MAIN: &str = "namespace eval src {\n    proc helper {alpha beta} { puts \"SRC/$alpha/$beta\" }\n    proc other {} { puts O }\n    namespace export other\n}\nproc helper {args} { puts LOCAL }\nnamespace import -force ::src::*\nhelper 1 2\n";

const SHADOW_SIBLING: &str = "namespace eval src {\n    namespace export helper\n}\n";
const INERT_SIBLING: &str = "namespace eval other {\n    proc q {} { puts Q }\n}\n";

/// Open `SHADOW_MAIN` alongside `sibling` and return the ready server plus the
/// main document's URI.
fn shadow_workspace(sibling: &str) -> (Lsp, String) {
    shadow_workspace_on(Lsp::tcl(), sibling)
}

/// The same two-document workspace on a caller-supplied server — inlay hints
/// are gated off by default, so that test needs `Lsp::inlay()`.
fn shadow_workspace_on(mut lsp: Lsp, sibling: &str) -> (Lsp, String) {
    lsp.open_ready(&unique_uri("tcl"), sibling);
    let main_uri = unique_uri("tcl");
    lsp.open_ready(&main_uri, SHADOW_MAIN);
    (lsp, main_uri)
}

#[test]
fn the_server_hands_definition_and_hover_the_same_shadow_answer() {
    // TP — the sibling's export means `-force` really deleted `::helper`, so
    // the call reaches `::src::helper` on line 1, not the local on line 5.
    let (mut lsp, uri) = shadow_workspace(SHADOW_SIBLING);
    let locs = locations(&lsp.definition(&uri, 7, 0));
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(start_line(&locs[0]), 1, "the import replaced ::helper");
    let hover = hover_text(&lsp.hover(&uri, 7, 0));
    assert!(
        hover.contains("alpha") && hover.contains("beta"),
        "hover must describe `::src::helper {{alpha beta}}`: {hover:?}"
    );

    // TN — byte-identical document, inert sibling: the local survives.
    let (mut lsp, uri) = shadow_workspace(INERT_SIBLING);
    let locs = locations(&lsp.definition(&uri, 7, 0));
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(start_line(&locs[0]), 5, "the import bound nothing");
    let hover = hover_text(&lsp.hover(&uri, 7, 0));
    assert!(
        !hover.contains("alpha"),
        "hover must describe the local `helper {{args}}`: {hover:?}"
    );
}

#[test]
fn the_server_hands_signature_help_the_shadow_answer() {
    let (mut lsp, uri) = shadow_workspace(SHADOW_SIBLING);
    let sig = serde_json::to_string(&lsp.signature_help(&uri, 7, 8)).unwrap_or_default();
    assert!(
        sig.contains("alpha") && sig.contains("beta"),
        "a live -force shadow must render `::src::helper`'s signature: {sig}"
    );

    let (mut lsp, uri) = shadow_workspace(INERT_SIBLING);
    let sig = serde_json::to_string(&lsp.signature_help(&uri, 7, 8)).unwrap_or_default();
    assert!(
        !sig.contains("alpha") && sig.contains("args"),
        "with no covering export the local signature is correct: {sig}"
    );
}

#[test]
fn the_server_hands_inlay_hints_the_shadow_answer() {
    let (mut lsp, uri) = shadow_workspace_on(Lsp::inlay(), SHADOW_SIBLING);
    let hints = serde_json::to_string(&lsp.inlay_hints(&uri, (7, 0), (8, 0))).unwrap_or_default();
    assert!(
        hints.contains("alpha") && hints.contains("beta"),
        "a live -force shadow must label the imported proc's params: {hints}"
    );

    let (mut lsp, uri) = shadow_workspace_on(Lsp::inlay(), INERT_SIBLING);
    let hints = serde_json::to_string(&lsp.inlay_hints(&uri, (7, 0), (8, 0))).unwrap_or_default();
    assert!(
        !hints.contains("alpha"),
        "with no covering export the variadic local proc is reached: {hints}"
    );
}

#[test]
fn the_server_hands_call_hierarchy_the_shadow_answer() {
    // The two candidates differ in arity, which the item `detail` carries, so
    // this pins *which* definition the hierarchy is rooted at.
    let (mut lsp, uri) = shadow_workspace(SHADOW_SIBLING);
    let items = serde_json::to_string(&lsp.prepare_call_hierarchy(&uri, 7, 0)).unwrap_or_default();
    assert!(
        items.contains("2 params"),
        "a live -force shadow roots the hierarchy at `::src::helper`: {items}"
    );

    let (mut lsp, uri) = shadow_workspace(INERT_SIBLING);
    let items = serde_json::to_string(&lsp.prepare_call_hierarchy(&uri, 7, 0)).unwrap_or_default();
    assert!(
        items.contains("1 params"),
        "with no covering export it roots at the local `helper {{args}}`: {items}"
    );
}

#[test]
fn the_server_hands_code_actions_the_shadow_answer() {
    // Inlining the wrong body is a behaviour change, not a refactor: the two
    // bodies print different text, so the offered edit names the winner.
    let range = serde_json::json!({
        "start": { "line": 7, "character": 0 },
        "end":   { "line": 7, "character": 0 },
    });
    let (mut lsp, uri) = shadow_workspace(SHADOW_SIBLING);
    let actions =
        serde_json::to_string(&lsp.code_actions(&uri, range.clone(), serde_json::json!([])))
            .unwrap_or_default();
    assert!(
        actions.contains("SRC/") && !actions.contains("LOCAL"),
        "a live -force shadow must offer the imported body for inlining: {actions}"
    );

    let (mut lsp, uri) = shadow_workspace(INERT_SIBLING);
    let actions = serde_json::to_string(&lsp.code_actions(&uri, range, serde_json::json!([])))
        .unwrap_or_default();
    assert!(
        actions.contains("LOCAL") && !actions.contains("SRC/"),
        "with no covering export the local body is offered: {actions}"
    );
}

/// `ticklecharts`' `etypes.tcl`
/// shape: a `namespace ensemble create -command ::new -subcommands {…}` whose
/// implementing procs are installed by a `foreach` loop through a substituted
/// name (`proc ticklecharts::${ptype} …`).
///
/// tclsh 9.0.4 and 8.6.16 agree: `new elist {1 2 3}` prints
/// `elist {1 2 3}` and `new edict {k v}` prints `edict {k v}`, so every
/// literal loop element really does become a callable command.
///
/// The analyser's `all_procs` table is pinned by a unit test; this checks
/// that the LSP-facing consumers (go-to-definition, diagnostics, the
/// outline) also read it correctly for a loop-generated name.
#[test]
fn foreach_generated_ensemble_subcommands_navigate_and_outline_923_idx43() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = concat!(
        "namespace eval ticklecharts {\n", // 0
        "    namespace ensemble create -command ::new -subcommands {elist elist.n edict}\n", // 1
        "}\n",                             // 2
        "foreach ptype {elist elist.n} {\n", // 3
        "    proc ticklecharts::${ptype} {args} [string map [list %P% $ptype] {\n", // 4
        "        return \"%P% $args\"\n",  // 5
        "    }]\n",                        // 6
        "}\n",                             // 7
        "proc ticklecharts::edict {args} { return \"edict $args\" }\n", // 8
    );
    lsp.open_ready(&uri, src);

    // 1. Go-to-definition on `elist` inside the `-subcommands` list (line 1,
    //    column 60) lands on the one physical `proc` statement that backs it.
    let def = locations(&lsp.definition(&uri, 1, 60));
    assert_eq!(
        def.iter().map(start_line).collect::<Vec<_>>(),
        vec![4],
        "`elist` in -subcommands must reach the foreach-installed proc: {def:?}",
    );

    // 2. No false "unknown command" for a name only the loop creates.
    let diags = lsp.pull_diagnostics(&uri);
    assert!(
        !diags.iter().any(|d| {
            d.get("code").and_then(Value::as_str) == Some("W123")
                && d.get("message")
                    .and_then(Value::as_str)
                    .is_some_and(|m| m.contains("elist"))
        }),
        "a loop-installed proc must not draw W123: {diags:?}",
    );

    // 3. The outline names the real per-literal procs, not the unsubstituted
    //    `${ptype}` placeholder.
    let symbols = serde_json::to_string(&lsp.document_symbols(&uri)).expect("symbols json");
    for want in ["elist", "elist.n"] {
        assert!(
            symbols.contains(&format!("\"{want}\"")),
            "the outline must list `{want}`: {symbols}",
        );
    }
    assert!(
        !symbols.contains("${ptype}"),
        "the outline must not show the unsubstituted placeholder: {symbols}",
    );
}
