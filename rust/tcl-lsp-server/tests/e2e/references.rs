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

//! Native port of `tests/lsp_e2e/test_references_e2e.py`.
//!
//! Find-references, end-to-end against the packaged server. Full-parity port of
//! the request/response cases. The references result is a list of `Location`s;
//! `starts(&result)` gives the `(line, character)` start set and
//! `start_lines(&result)` the set of start lines.

// Test column math indexes tiny in-memory sources; a `find`/`len` result
// always fits u32, so the pedantic truncation the lint warns of can't occur.
#![allow(clippy::cast_possible_truncation)]

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

/// idx=9 (differential-audit main wave, high severity): a cursor placed
/// directly on a variable's own bareword declaration/write token (a proc
/// parameter, or a `catch script name` result-var reusing an existing
/// variable) previously returned zero references, even though the same
/// query from any `$name` read of the same variable resolved the full set.
#[test]
fn find_references_from_proc_param_bareword_declaration() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc greet {name} { return $name }\ngreet hi\n";
    lsp.open_ready(&uri, src);
    // Cursor on `name` inside the parameter list, col 12-16.
    let lines = start_lines(&lsp.references(&uri, 0, 13, true));
    assert!(lines.contains(&0), "decl missing: {lines:?}");
}

#[test]
fn find_references_from_catch_resultvar_bareword_include_the_original_declaration() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc resolveSwitch {name def} {\n    catch {foo} name\n    return $name\n}\n";
    lsp.open_ready(&uri, src);
    // Cursor on the catch result-var `name`, line 1 col 16-20.
    let lines = start_lines(&lsp.references(&uri, 1, 17, true));
    assert!(lines.contains(&0), "original decl missing: {lines:?}");
    assert!(
        lines.contains(&2),
        "the later $name read missing: {lines:?}"
    );
}

#[test]
fn find_qualified_proc_call_sites() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src =
        "namespace eval myns {\n    proc helper {} { return 1 }\n}\nmyns::helper\n::myns::helper\n";
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

/// idx=61 (differential-audit main wave, critical severity): `if {$cond}
/// mymod::foo` — an unbraced (bareword) if-body — is a legitimate,
/// statically-known zero-arg call, exactly like a braced `{ mymod::foo }`
/// body. `dispatch_body_arguments` previously only ever recursed a *braced*
/// body into `command_invocations`, so this call site was invisible to
/// `references` while `definition`/`hover` still resolved it fine (they
/// walk independently off the cursor token).
#[test]
fn find_unbraced_if_body_bareword_call_site() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc foo {} { return 1 }\nif {1} foo\n");
    let lines = start_lines(&lsp.references(&uri, 0, 6, true));
    assert!(lines.contains(&0), "decl missing: {lines:?}");
    assert!(
        lines.contains(&1),
        "unbraced if-body call site missing: {lines:?}"
    );
}

/// Same root cause, the finding's other confirmed shape: `uplevel 1
/// mymod::qux` (unbraced). `handle_uplevel_command` only special-cases a
/// braced body and otherwise falls through to the same generic
/// `ArgRole::Body` dispatch this fix covers.
#[test]
fn find_unbraced_uplevel_body_bareword_call_site() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc qux {} { return 1 }\nproc caller {} { uplevel 1 qux }\n",
    );
    let lines = start_lines(&lsp.references(&uri, 0, 6, true));
    assert!(lines.contains(&0), "decl missing: {lines:?}");
    assert!(
        lines.contains(&1),
        "unbraced uplevel-body call site missing: {lines:?}"
    );
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

// -- Issue #923: fully-qualified / relative names in namespaces nested
// two or more levels deep. Regression coverage over the full JSON-RPC
// protocol (not just the tcl-lsp-core unit-test harness) for a bug where a
// bareword call from inside a namespace nested 2+ levels deep was resolved
// as if it were at the top level and never matched its own proc.

#[test]
fn find_qualified_proc_call_in_two_level_nested_namespace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval modelTestVerTool {\n    namespace eval gui {\n        proc specAddButtonPopUp {x y} { return \"$x $y\" }\n    }\n}\n::modelTestVerTool::gui::specAddButtonPopUp 1 2\n";
    lsp.open_ready(&uri, src);
    let lines = start_lines(&lsp.references(&uri, 2, 14, true));
    assert!(lines.contains(&2), "decl missing: {lines:?}");
    assert!(
        lines.contains(&5),
        "fully-qualified call missing: {lines:?}"
    );
}

#[test]
fn find_bare_proc_call_from_inside_two_level_nested_namespace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval modelTestVerTool {\n    namespace eval gui {\n        proc specAddButtonPopUp {x y} { return \"$x $y\" }\n        proc caller {} { return [specAddButtonPopUp 1 2] }\n    }\n}\n";
    lsp.open_ready(&uri, src);
    let lines = start_lines(&lsp.references(&uri, 2, 14, true));
    assert!(lines.contains(&2), "decl missing: {lines:?}");
    assert!(
        lines.contains(&3),
        "bare call from the same 2-level-nested namespace missing: {lines:?}"
    );
}

#[test]
fn issue_923_bind_callback_qualified_and_bare_both_resolve() {
    // The exact shape from
    // https://github.com/bitwisecook/tcl-lsp/issues/923: a proc called from
    // a Tk `bind` callback script by its fully-qualified name (reported as
    // "0 references"), alongside a sibling called by bare name from the
    // same namespace.
    let src = concat!(
        "namespace eval modelTestVerTool {\n",
        "    namespace eval gui {\n",
        "        proc specAddButtonPopUp {x y} { return \"spec $x $y\" }\n",
        "        proc testAddButtonPopUp {x y} { return \"test $x $y\" }\n",
        "        bind $win.fra.tool.buT_specAddIconLarge <ButtonRelease-1> {::modelTestVerTool::gui::specAddButtonPopUp %X %Y}\n",
        "        bind $win.fra.tool.buT_testAddIconLarge <ButtonRelease-1> {testAddButtonPopUp %X %Y}\n",
        "    }\n",
        "}\n",
    );
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, src);
    let spec_lines = start_lines(&lsp.references(&uri, 2, 14, true));
    assert!(spec_lines.contains(&2), "spec decl missing: {spec_lines:?}");
    assert!(
        spec_lines.contains(&4),
        "spec's fully-qualified bind call site missing: {spec_lines:?}"
    );
    let test_lines = start_lines(&lsp.references(&uri, 3, 14, true));
    assert!(test_lines.contains(&3), "test decl missing: {test_lines:?}");
    assert!(
        test_lines.contains(&5),
        "test's bare bind call site missing: {test_lines:?}"
    );
}

#[test]
fn qualified_calls_do_not_cross_two_level_nested_namespace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval a {\n    namespace eval b {\n        proc helper {} { return 1 }\n    }\n}\nnamespace eval c {\n    namespace eval d {\n        proc helper {} { return 2 }\n        proc caller {} { return [helper] }\n    }\n}\n";
    lsp.open_ready(&uri, src);
    // ::a::b::helper has no callers.
    let lines_ab = start_lines(&lsp.references(&uri, 2, 14, true));
    assert_eq!(
        lines_ab,
        [2].into_iter().collect(),
        "::a::b::helper must not pick up ::c::d's call: {lines_ab:?}"
    );
    // ::c::d::helper is called once, by its own sibling.
    let lines_cd = start_lines(&lsp.references(&uri, 7, 14, true));
    assert!(lines_cd.contains(&7), "{lines_cd:?}");
    assert!(lines_cd.contains(&8), "{lines_cd:?}");
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

/// End-to-end (real server): a `TclOO` instance variable's uses unify across
/// every method body — Find-References on `$n` in one method reaches its
/// declaration and the sibling method's use.
#[test]
fn references_object_variable_unify_across_methods() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // line 1: variable n; line 2: `$n` in get; line 3: `incr n` in bump
    let src = "oo::class create C {\n    variable n\n    method get {} { return $n }\n    method bump {} { incr n }\n}\n";
    lsp.open_ready(&uri, src);
    let col = src.lines().nth(2).unwrap().find("$n").unwrap() as u32 + 1;
    let lines = start_lines(&lsp.references(&uri, 2, col, true));
    assert!(lines.contains(&1), "declaration (line 1): {lines:?}");
    assert!(lines.contains(&2), "`$n` in get (line 2): {lines:?}");
    assert!(
        lines.contains(&3),
        "`incr n` in bump (line 3) must unify: {lines:?}"
    );
}

/// idx 32 (differential-audit main audit wave, high severity): a `TclOO`
/// class body with TWO separate `variable` statements (the real corpus
/// shape — `georgtree_tclopt`'s `::tclopt::Mpfit` declares `variable funct
/// m ftol ...` then, separately, `variable Pars`). The analyser's
/// per-statement handler assigned `class_def.variables = sub_args.to_vec()`
/// instead of accumulating, so the second statement silently discarded
/// every name the first one declared — only the names in the LAST
/// `variable` statement stayed resolvable, even though tclsh9.0 proves
/// both statements' names are simultaneously live instance variables.
#[test]
fn references_reach_instance_variable_declared_in_a_non_last_variable_statement() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // line 1: variable funct (the FIRST, previously-discarded statement);
    // line 2: variable Pars (the LAST, previously-surviving statement);
    // line 3: `$funct`/`$Pars` both used in `run`.
    let src = "oo::class create Mpfit {\n    variable funct\n    variable Pars\n    method run {} { return [list $Pars $funct] }\n}\n";
    lsp.open_ready(&uri, src);
    let col = src.lines().nth(3).unwrap().find("$funct").unwrap() as u32 + 1;
    let lines = start_lines(&lsp.references(&uri, 3, col, true));
    assert!(
        lines.contains(&1),
        "funct's own declaration (line 1) must resolve: {lines:?}"
    );
    assert!(
        lines.contains(&3),
        "the `$funct` use itself must unify: {lines:?}"
    );
}

/// End-to-end (real server): a namespace variable's `variable` aliases across
/// procs and its namespace-level declaration unify into one reference set.
#[test]
fn references_namespace_variable_unify_across_procs() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval ::app {\n    variable count 0\n    proc bump {} { variable count; incr count }\n    proc get {} { variable count; return $count }\n}\n";
    lsp.open_ready(&uri, src);
    let col = src.lines().nth(3).unwrap().find("$count").unwrap() as u32 + 1;
    let lines = start_lines(&lsp.references(&uri, 3, col, true));
    assert!(
        lines.contains(&1),
        "namespace-level decl (line 1): {lines:?}"
    );
    assert!(lines.contains(&2), "bump alias+use (line 2): {lines:?}");
    assert!(lines.contains(&3), "get alias+use (line 3): {lines:?}");
}

/// End-to-end (real server): an unrelated ordinary `proc pf` sharing an
/// `expr` math-function's bare tail name must not leak into either symbol's
/// references — the two live in entirely separate command tables in real
/// Tcl. Pins the same fix `definition.rs`'s
/// `no_definition_for_mathfunc_call_despite_unrelated_same_named_proc`
/// covers, at the find-references layer.
#[test]
fn references_do_not_cross_between_unrelated_proc_and_mathfunc_override() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc pf {y} { return bogus }\n\
               namespace eval ::nsa::tcl::mathfunc {}\n\
               proc ::nsa::tcl::mathfunc::pf {x} { return 20 }\n\
               namespace eval ::nsa {\n    proc caller {} { return [expr {pf(1)}] }\n}\n";
    lsp.open_ready(&uri, src);

    // References on the mathfunc override's declaration (line 2) must reach
    // the expr call site (line 4), never the unrelated proc (line 0).
    let override_col = src.lines().nth(2).unwrap().rfind("pf").unwrap() as u32;
    let override_lines = start_lines(&lsp.references(&uri, 2, override_col, true));
    assert!(
        override_lines.contains(&4),
        "override references must reach the expr call site: {override_lines:?}"
    );
    assert!(
        !override_lines.contains(&0),
        "override references must not include the unrelated proc: {override_lines:?}"
    );

    // References on the unrelated proc's declaration (line 0) must not
    // include the expr call site, which never dispatches to it.
    let unrelated_col = src.lines().next().unwrap().find("pf").unwrap() as u32;
    let unrelated_lines = start_lines(&lsp.references(&uri, 0, unrelated_col, true));
    assert!(
        !unrelated_lines.contains(&4),
        "unrelated proc's references must not include the expr call site: {unrelated_lines:?}"
    );
}

/// idx 68 (differential-audit main audit wave, high severity, pix corpus):
/// reduces the real `isEqual`/`tolComp` shape from nico-robert/pix's
/// `test/data_b64.test` — a proc aliases a top-level cell via `global`, and a
/// caller overrides it via a plain `set ::name` before invoking the proc.
/// tclsh proves `tolComp` (via `global`) and `::tolComp` (the caller's
/// `set`) are the identical storage cell; previously, Find-References from
/// either side reached only its own half, so Rename (which shares the same
/// helper) would silently decouple the two, leaving the caller's override
/// unreachable and `isEqual` falling back to its hardcoded default.
#[test]
fn references_unify_a_procs_global_alias_with_the_callers_canonical_set() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src =
        "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
    lsp.open_ready(&uri, src);
    let col = src.lines().nth(2).unwrap().find("$tolComp").unwrap() as u32 + 1;
    let lines = start_lines(&lsp.references(&uri, 2, col, true));
    assert!(
        lines.contains(&1),
        "the `global tolComp` decl (line 1): {lines:?}"
    );
    assert!(
        lines.contains(&2),
        "the in-proc $tolComp read (line 2): {lines:?}"
    );
    assert!(
        lines.contains(&4),
        "the caller's canonical `set ::tolComp` (line 4) must now be reached: {lines:?}"
    );
}

/// idx 71 (differential-audit main audit wave, high severity, pix corpus):
/// reduces nico-robert/pix's `test_context.test` (`source [file join [file
/// dirname [info script]] data_b64.test]`, then calls `isEqual` bare) to a
/// literal-`source` control shape — the finding's own control repro proved
/// the identical bug reproduces with a trivial `source lib.tcl`, no
/// `[info script]` at all. `main.tcl` sources `lib.tcl` (which declares
/// `helper`) and calls `helper` itself; `main.tcl` has no local declaration
/// of `helper` to anchor `cross_document_references`'s exclusion of the
/// current document on, so the call under the cursor — in the very
/// document the query was issued from — was previously dropped entirely,
/// returning only `lib.tcl`'s declaration.
#[test]
fn references_reach_the_current_documents_own_call_when_it_has_no_local_declaration() {
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(&lib_uri, "proc helper {} {}\n");
    let main_uri = unique_uri("tcl");
    let main_src = "source lib.tcl\nhelper\n";
    lsp.open_ready(&main_uri, main_src);
    // Cursor on main.tcl's own `helper` call (line 1).
    let lines = start_lines(&lsp.references(&main_uri, 1, 0, true));
    assert!(
        lines.contains(&1),
        "main.tcl's own call site must be reached, not just lib.tcl's decl: {lines:?}"
    );
    assert!(lines.contains(&0), "lib.tcl's decl (line 0): {lines:?}");
    assert_eq!(lines.len(), 2, "{lines:?}");
}

/// idx 21 (differential-audit main audit wave): `interp alias {} sayHi {}
/// greet` makes every `[sayHi]` a real call site of `greet` — tclsh 9.0.4
/// and 8.6.16 both execute `greet`'s body twice for the two calls below.
/// Find-references never consulted the alias table, so a user asking whether
/// `greet` was safe to delete was told it had no callers.
#[test]
fn references_include_call_sites_spelled_through_an_alias() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {} { return hi }\ninterp alias {} sayHi {} greet\nputs [sayHi]\nputs [sayHi]\n",
    );
    // Line 0: cursor on the `greet` declaration.
    let lines = start_lines(&lsp.references(&uri, 0, 6, true));
    assert!(lines.contains(&0), "decl missing: {lines:?}");
    assert!(
        lines.contains(&2) && lines.contains(&3),
        "both alias call sites must be reported: {lines:?}"
    );
}

/// The reverse direction of the same fact: a query from the alias's own call
/// site resolves through to the target and answers the identical unified set.
#[test]
fn references_from_an_alias_call_site_offer_the_targets_sites() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {} { return hi }\ninterp alias {} sayHi {} greet\nputs [sayHi]\nputs [sayHi]\n",
    );
    // Line 2: cursor on the `sayHi` call.
    let lines = start_lines(&lsp.references(&uri, 2, 6, true));
    assert!(lines.contains(&0), "the target's decl missing: {lines:?}");
    assert!(
        lines.contains(&2) && lines.contains(&3),
        "both alias call sites must be reported: {lines:?}"
    );
}

/// idx 92 (differential-audit main audit wave): the `[namespace code [list
/// ProcName]]` callback wrapper Tk's own `library/fontchooser.tcl` uses ten
/// times. tclsh 9.0.4 and 8.6.16 both report the installed trace as
/// `{write {::namespace inscope ::demo Tracer}}` and really dispatch
/// `::demo::Tracer` on a write, so the wrapped word is a genuine call site.
#[test]
fn references_reach_a_namespace_code_list_wrapped_callback() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval ::demo {\n    variable S\n    proc Tracer {a b op} { }\n    proc Setup {} {\n        trace add variable S(size) write [namespace code [list Tracer]]\n    }\n}\n",
    );
    // Line 2: cursor on the `Tracer` declaration.
    let lines = start_lines(&lsp.references(&uri, 2, 10, true));
    assert!(lines.contains(&2), "decl missing: {lines:?}");
    assert!(
        lines.contains(&4),
        "the `[namespace code [list Tracer]]` callback site is missing: {lines:?}"
    );
}

/// idx 63 (differential-audit main audit wave, high severity): a `my
/// methodName` call written inside a `switch` arm body is a genuine,
/// statically-known call site (tclsh9.0/8.6-verified) — the real corpus
/// shape (`ticklecharts::chart`'s `Add` dispatcher: `switch ... {
/// barSeries { my AddBarSeries {*}$args } ... }`). `scan_my_method_region`'s
/// `[...]`-substitution recursion never reached a switch arm's braced body
/// (it isn't a command substitution), so this was invisible to
/// find-references even though go-to-definition (an independent
/// cursor-token walk) already resolved it.
#[test]
fn references_reach_a_my_dispatch_call_inside_a_switch_arm() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create widget {\n    method bar {} { return \"bar-value\" }\n    method dispatch {args} {\n        switch -exact -- [lindex $args 0] {\n            bar { my bar {*}[lrange $args 1 end] }\n        }\n    }\n}\n",
    );
    // Line 1: `    method bar {} { return "bar-value" }` — cursor on the
    // `bar` declaration (column 11).
    let lines = start_lines(&lsp.references(&uri, 1, 11, true));
    assert!(lines.contains(&1), "decl missing: {lines:?}");
    assert!(
        lines.contains(&4),
        "the my bar call site inside the switch arm is missing: {lines:?}"
    );
}

/// Issue #1108: a registry `VarRead`-role name word is a use site.  A variable
/// is read by more than `$name` — tclsh 9.0.4 / 8.6.16 both print `1` for
/// `proc f {} {set m 1; puts [set m]; puts [info exists m]}; f`, so both bare
/// `m` words really do read the cell.  Find References reported neither.
#[test]
fn references_reach_a_var_read_role_name_word() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc f {} {\n    set m 1\n    puts [set m]\n    puts [info exists m]\n}\n",
    );
    // Cursor on the `m` of `set m 1`.
    let lines = start_lines(&lsp.references(&uri, 1, 8, true));
    assert!(lines.contains(&1), "decl missing: {lines:?}");
    assert!(lines.contains(&2), "`[set m]` read missing: {lines:?}");
    assert!(
        lines.contains(&3),
        "`[info exists m]` read missing: {lines:?}"
    );
    // And the same set answers from a cursor on either read word.
    for (line, col) in [(2u32, 14u32), (3, 22)] {
        let from_read = start_lines(&lsp.references(&uri, line, col, true));
        assert!(
            from_read.contains(&1) && from_read.contains(&2) && from_read.contains(&3),
            "the cursor on the read word at ({line},{col}) must answer the whole \
             set: {from_read:?}"
        );
    }
}

/// Issue #1138 idx 102: `::tk::SourceLibFile`'s `$file` read lives inside a
/// `[list …]`-built `namespace eval` body.  The `[…]` is evaluated in the
/// proc's own frame before `namespace eval` enters `::`, so the read is an
/// ordinary use of the parameter — but the namespace scope had claimed those
/// bytes, and find-references answered nothing.
#[test]
fn references_reach_a_parameter_read_inside_a_list_built_namespace_body() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc ::tk::SourceLibFile {file} {\n    namespace eval :: [list source [file join $::tk_library $file.tcl]]\n}\n",
    );
    // Cursor on the `file` of `$file` (line 1, columns 61-64).
    let lines = start_lines(&lsp.references(&uri, 1, 62, true));
    assert!(
        lines.contains(&0),
        "the parameter decl is missing: {lines:?}"
    );
    assert!(
        lines.contains(&1),
        "the `$file` read inside the built body is missing: {lines:?}"
    );
}
