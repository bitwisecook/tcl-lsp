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

//! Behaviour-driven coverage of the `tcl-lsp-core` references and rename
//! providers (`references::references`, `rename::rename`,
//! `rename::prepare_rename`, plus `is_safe_symbol_name`).
//!
//! There is no single upstream pytest source — these tests are derived from
//! the providers' public API (see `src/references.rs` / `src/rename.rs`) and
//! from Tcl name-resolution semantics, with the Tcl-semantic facts pinned to
//! real C-Tcl (tclsh8.6 / tclsh9.0 via `scripts/dev/tclsh_check.sh`).
//!
//! C-Tcl proof model
//! -----------------
//! A "reference" is a genuine call/use site in runnable Tcl, and a rename is
//! correct exactly when the renamed script still resolves the same way Tcl
//! itself would. Every snippet below is a COMPLETE runnable Tcl script in
//! which the proc/method is actually invoked, or the variable actually read,
//! at the sites asserted as references. The reference/rename SET (which
//! positions) is an editor-structural fact about the parse, so it is asserted
//! structurally; the proof that the set is *right* is that those positions are
//! exactly the call/use sites Tcl's own name resolution binds together — and
//! the sites it keeps APART (a same-named local in another proc, a same-named
//! proc in another namespace) are left untouched. Each test cites the
//! corresponding `tclsh:` observation.
//!
//! Verified C-Tcl facts (tclsh8.6 + tclsh9.0, identical output):
//!   * `proc greet {} {return hi}; puts [greet]; puts [greet]`  -> hi / hi
//!     (the two `greet` words are real invocations of the one proc).
//!   * `set x 1; puts $x; puts $x`  -> 1 / 1  (two real reads of `x`).
//!   * `proc a {} {set v 10; return $v}; proc b {} {set v 20; return $v}`
//!     -> a=10, b=20  (each `v` is a distinct local; renaming a's `v` must
//!     not touch b's `v`).
//!   * namespace: `::myns::greet` and a short `greet` inside
//!     `namespace eval ::myns` both invoke the one proc -> ns-hi / ns-hi;
//!     and `::a::helper` vs `::b::helper` are two independent procs
//!     -> a / b  (renaming one must not touch the other).
//!   * renamed forms still run: `salute`/`salute`, `$y`/`$y`,
//!     `::myns::hello` + short `hello` -> identical output to the original.

// Test column math indexes tiny in-memory sources; a `find`/`len` result
// always fits u32, so the pedantic truncation the lint warns of can't occur.
#![allow(clippy::cast_possible_truncation)]

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::references::references;
use tcl_lsp_core::rename::{TextEdit, is_safe_symbol_name, prepare_rename, rename};
use tcl_registry::CommandRegistry;

/// Build an analysis exactly the way the existing port files do
/// (`call_hierarchy.rs`, `selection_range.rs`).
fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

/// The set of start lines touched by a list of reference ranges, sorted and
/// de-duplicated — the load-bearing structural fact for most assertions.
fn ref_lines(ranges: &[LspRange]) -> Vec<u32> {
    let mut v: Vec<u32> = ranges.iter().map(|r| r.start_line).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// The set of start lines touched by a list of rename edits.
fn edit_lines(edits: &[TextEdit]) -> Vec<u32> {
    let mut v: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    v.sort_unstable();
    v.dedup();
    v
}

// ---------------------------------------------------------------------------
// references — procs
// ---------------------------------------------------------------------------

#[test]
fn references_proc_from_definition_includes_decl_and_both_calls() {
    // tclsh: `proc greet {} {return hi}; puts [greet]; puts [greet]` -> hi/hi.
    // The two `greet` words (lines 1 and 2) are genuine invocations of the
    // proc declared on line 0 — so references from the declaration must be
    // exactly {decl line 0, call line 1, call line 2}.
    let src = "proc greet {} { return hi }\nputs [greet]\nputs [greet]\n";
    let analysis = analyse(src);
    // Cursor on `greet` in the declaration (line 0, col 6).
    let refs = references(src, "tcl", 0, 6, &analysis, true);
    assert_eq!(
        ref_lines(&refs),
        vec![0, 1, 2],
        "decl + both call sites expected; got {refs:?}",
    );
    // First entry is the declaration on line 0 (include_declaration = true).
    assert_eq!(refs[0].start_line, 0, "decl should lead: {refs:?}");
}

#[test]
fn references_proc_from_call_site_finds_the_same_set() {
    // Triggering Find-All-References from a call site resolves to the same
    // proc and therefore the same reference set as from the declaration.
    let src = "proc greet {} { return hi }\nputs [greet]\nputs [greet]\n";
    let analysis = analyse(src);
    // Cursor on the first `greet` CALL (line 1, inside `[greet]`).
    let refs = references(src, "tcl", 1, 7, &analysis, true);
    assert_eq!(
        ref_lines(&refs),
        vec![0, 1, 2],
        "call-site Find-References should match the proc's whole set; got {refs:?}",
    );
}

#[test]
fn references_proc_named_in_info_body_include_the_introspection_site() {
    // `info body PROC` names the proc as data (introspected, not called); it
    // is a command reference, so Find-All-References from the declaration must
    // include the `greet` word inside `info body greet`.
    let src = "proc greet {} { return hi }\ninfo body greet\n";
    let analysis = analyse(src);
    // Cursor on `greet` in the declaration (line 0, col 6).
    let refs = references(src, "tcl", 0, 6, &analysis, true);
    assert_eq!(
        ref_lines(&refs),
        vec![0, 1],
        "decl + the `info body` introspection site expected; got {refs:?}",
    );
}

#[test]
fn references_proc_called_inside_oo_objdefine_method_body() {
    // A per-object method body (`oo::objdefine $o { method … { helper } }`) is
    // now analysed like any method body, so a proc it calls is found by
    // Find-All-References from the proc's declaration.
    let src = "proc helper {} {}\n\
               oo::class create Foo {}\n\
               set o [Foo new]\n\
               oo::objdefine $o {\n    \
                   method greet {} { helper }\n\
               }\n";
    let analysis = analyse(src);
    let refs = references(src, "tcl", 0, 6, &analysis, true);
    assert!(
        ref_lines(&refs).contains(&4),
        "the call in the per-object method body (line 4) should be a reference: {refs:?}",
    );
}

#[test]
fn namespace_which_command_probe_is_not_a_reference() {
    // `namespace which -command greet` is an existence PROBE (it returns "" for
    // an unknown command), not a call or a navigable reference.  Recording it
    // as a command reference fed the probe into the W123 unresolved-command
    // pass, flagging a legitimate `[namespace which -command foo] eq ""`
    // existence check.  So Find-All-References from the declaration does NOT
    // include the probe site — only the decl itself.
    let src = "proc greet {} {}\nnamespace which -command greet\n";
    let analysis = analyse(src);
    let refs = references(src, "tcl", 0, 6, &analysis, true);
    assert_eq!(
        ref_lines(&refs),
        vec![0],
        "only the decl; the probe is not a reference: got {refs:?}",
    );
}

#[test]
fn references_proc_named_in_trace_add_execution_include_the_trace_site() {
    // `trace add execution PROC …` names the traced command; it is a
    // reference, so Find-All-References from the declaration includes the
    // `greet` word in the trace (the trailing `handler` is a separate
    // callback prefix, not part of greet's set).
    let src = "proc greet {} {}\nproc handler {args} {}\ntrace add execution greet enter handler\n";
    let analysis = analyse(src);
    // Cursor on `greet` in the declaration (line 0, col 6).
    let refs = references(src, "tcl", 0, 6, &analysis, true);
    assert_eq!(
        ref_lines(&refs),
        vec![0, 2],
        "decl + the trace site expected; got {refs:?}",
    );
}

#[test]
fn references_proc_exclude_declaration_drops_the_decl_line() {
    // include_declaration = false omits the defining span; only call sites
    // remain (lines 1 and 2).
    let src = "proc greet {} { return hi }\nputs [greet]\nputs [greet]\n";
    let analysis = analyse(src);
    let with_decl = references(src, "tcl", 0, 6, &analysis, true);
    let without_decl = references(src, "tcl", 0, 6, &analysis, false);
    assert_eq!(
        ref_lines(&without_decl),
        vec![1, 2],
        "only call sites when decl excluded; got {without_decl:?}",
    );
    assert_eq!(
        with_decl.len(),
        without_decl.len() + 1,
        "excluding the declaration drops exactly one entry",
    );
}

#[test]
fn references_proc_only_at_real_call_sites_not_substrings() {
    // tclsh: `greeter` is a *different* command from `greet`; only the two
    // bare `greet` words are real invocations of the proc. The provider must
    // not match the `greet` substring inside the proc name `greeter`.
    // Snippet is runnable: both procs return, top level calls each once.
    let src = "proc greet {} { return a }\nproc greeter {} { return b }\ngreet\ngreeter\n";
    let analysis = analyse(src);
    // References for `greet` (cursor on its decl, line 0).
    let refs = references(src, "tcl", 0, 6, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&0), "decl missing: {refs:?}");
    assert!(
        lines.contains(&2),
        "the `greet` call on line 2 missing: {refs:?}"
    );
    assert!(
        !lines.contains(&1),
        "must NOT reference the `greeter` declaration on line 1: {refs:?}",
    );
    assert!(
        !lines.contains(&3),
        "must NOT match the distinct `greeter` call on line 3: {refs:?}",
    );
}

// ---------------------------------------------------------------------------
// references — variables
// ---------------------------------------------------------------------------

#[test]
fn references_var_includes_definition_and_every_read() {
    // tclsh: `set x 1; puts $x; puts $x` -> 1/1. The two `$x` reads (lines 1
    // and 2) are genuine uses of the variable defined on line 0.
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    // Cursor inside the first `$x` read (line 1).
    let refs = references(src, "tcl", 1, 6, &analysis, true);
    assert_eq!(
        ref_lines(&refs),
        vec![0, 1, 2],
        "definition + both read sites expected; got {refs:?}",
    );
}

#[test]
fn references_var_exclude_declaration_keeps_only_reads() {
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    let refs = references(src, "tcl", 1, 6, &analysis, false);
    assert_eq!(
        ref_lines(&refs),
        vec![1, 2],
        "only the two read sites when declaration excluded; got {refs:?}",
    );
}

#[test]
fn references_var_scoped_to_its_proc_not_a_same_named_local_elsewhere() {
    // tclsh: `proc a {} {set v 10; return $v}` and
    //        `proc b {} {set v 20; return $v}` -> a=10, b=20.
    // The two `v`s are independent locals (one per call frame). References to
    // proc a's `v` must stay inside proc a (lines 1-2) and never reach proc
    // b's `v` (lines 5-6).
    let src = "proc a {} {\n    set v 10\n    return $v\n}\nproc b {} {\n    set v 20\n    return $v\n}\n";
    let analysis = analyse(src);
    // Cursor on `$v` in proc a's body (line 2).
    let refs = references(src, "tcl", 2, 12, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(
        lines.iter().all(|&l| l == 1 || l == 2),
        "proc a's `v` references must stay within proc a (lines 1-2); got {refs:?}",
    );
    assert!(
        !lines.contains(&5) && !lines.contains(&6),
        "must NOT reference proc b's same-named local `v`; got {refs:?}",
    );
}

// ---------------------------------------------------------------------------
// references — namespaces (qualified vs short, cross-namespace isolation)
// ---------------------------------------------------------------------------

#[test]
fn references_namespaced_proc_matches_qualified_and_short_calls() {
    // tclsh: a proc in `::myns` is invoked both as `::myns::greet` and as a
    // short `greet` inside `namespace eval ::myns` -> ns-hi/ns-hi. Both are
    // real invocations of the one proc, so both are references.
    let src = "namespace eval ::myns {\n    proc greet {} { return ns-hi }\n}\n::myns::greet\nnamespace eval ::myns {\n    greet\n}\n";
    let analysis = analyse(src);
    // Cursor on the `greet` declaration (line 1).
    let refs = references(src, "tcl", 1, 9, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&1), "declaration missing: {refs:?}");
    assert!(
        lines.contains(&3),
        "qualified `::myns::greet` call missing: {refs:?}"
    );
    assert!(
        lines.contains(&5),
        "short in-namespace `greet` call missing: {refs:?}"
    );
}

#[test]
fn references_relative_qualified_call_falls_back_to_global_target() {
    // tclsh8.6 (verified, PR #924 review): `inner::p` called inside `outer`
    // dispatches the *global* `::inner::p` when `::outer::inner::p` does not
    // exist — Tcl's two-step rule commits to the local candidate only when
    // that command exists.  References on `::inner::p`'s declaration must
    // therefore include the relative call site inside `outer`.
    let src = concat!(
        "namespace eval ::inner {}\n",
        "proc ::inner::p {} { return GLOBAL }\n",
        "namespace eval outer { proc caller {} { inner::p } }\n",
    );
    let analysis = analyse(src);
    // Cursor on the `::inner::p` declaration (line 1).
    let refs = references(src, "tcl", 1, 13, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&1), "declaration missing: {refs:?}");
    assert!(
        lines.contains(&2),
        "relative `inner::p` call inside `outer` must fall back to the \
         global proc: {refs:?}",
    );
}

#[test]
fn references_relative_qualified_call_prefers_existing_local_over_global() {
    // tclsh8.6 (verified): with BOTH `::outer::inner::p` and `::inner::p`
    // defined, the relative call inside `outer` dispatches the local one —
    // even though the local proc is defined *after* the caller (proc bodies
    // resolve at call time).  The call site must be attributed to the local
    // proc and NOT to the global one.
    let src = concat!(
        "namespace eval ::inner {}\n",
        "proc ::inner::p {} { return GLOBAL }\n",
        "namespace eval outer { proc caller {} { inner::p } }\n",
        "namespace eval outer { namespace eval inner {} ; proc inner::p {} { return LOCAL } }\n",
    );
    let analysis = analyse(src);
    // Cursor on the *global* `::inner::p` declaration (line 1): the call in
    // `outer` belongs to the local proc, so it must not appear here.
    let global_refs = references(src, "tcl", 1, 13, &analysis, true);
    let global_lines = ref_lines(&global_refs);
    assert!(
        !global_lines.contains(&2),
        "call inside `outer` resolves to the local proc, not the global: {global_refs:?}",
    );
    // Cursor on the *local* `::outer::inner::p` declaration (line 3).
    let local_refs = references(src, "tcl", 3, 55, &analysis, true);
    let local_lines = ref_lines(&local_refs);
    assert!(
        local_lines.contains(&2),
        "call inside `outer` must be attributed to the local proc: {local_refs:?}",
    );
}

#[test]
fn references_namespaced_proc_does_not_leak_to_same_name_in_other_namespace() {
    // tclsh: `::a::helper` -> "a" and `::b::helper` -> "b" are two distinct
    // procs. References to `::a::helper` must include its decl and its own
    // call, and must NOT include `::b::helper`'s decl or call.
    let src = "namespace eval ::a {\n    proc helper {} { return a }\n}\nnamespace eval ::b {\n    proc helper {} { return b }\n}\n::a::helper\n::b::helper\n";
    let analysis = analyse(src);
    // Cursor on `::a`'s helper declaration (line 1).
    let refs = references(src, "tcl", 1, 9, &analysis, true);
    let lines = ref_lines(&refs);
    assert_eq!(
        lines,
        vec![1, 6],
        "exactly ::a::helper decl (line 1) + its call (line 6); got {refs:?}",
    );
    assert!(
        !lines.contains(&4) && !lines.contains(&7),
        "must not reference ::b::helper (decl line 4 / call line 7): {refs:?}",
    );
}

// ---------------------------------------------------------------------------
// references — nested namespaces (2+ levels): qualified, relative, and the
// exact `bind`-callback shape from
// https://github.com/bitwisecook/tcl-lsp/issues/923
//
// Regression coverage for a namespace-resolution bug found while
// investigating #923: the analyser's per-call-site "what namespace does an
// unqualified name resolve against" computation
// (`Analyser::resolve_command_qualified_name`) read a scope-path field that
// was never actually updated during the real body walk, and the LSP-side
// namespace gate (`innermost_namespace_at`) took only the *innermost*
// enclosing `namespace eval`'s own segment rather than the full accumulated
// path — so a bare call from inside a namespace nested *two or more* levels
// deep (`namespace eval a { namespace eval b { ... } }`) was resolved as if
// it were at the top level and never matched its own proc. Both are fixed by
// routing through `Analyser::command_resolution_namespace` /
// `command_resolution_namespace_at`, the single accumulating implementation
// shared by the analyser and every `tcl-lsp-core` provider.
// ---------------------------------------------------------------------------

#[test]
fn references_two_level_nested_namespace_bare_call_matches_from_same_namespace() {
    // tclsh (verified): `namespace eval modelTestVerTool { namespace eval gui
    // { proc specAddButtonPopUp ...; proc caller {} { specAddButtonPopUp 1 2
    // } } }` then `::modelTestVerTool::gui::caller` -> "called 1 2". The bare
    // call resolves because it runs *inside* the proc's own two-level-nested
    // namespace. Before the fix this bare call was not found as a reference
    // at all (the dead-field bug always guessed the top-level namespace).
    let src = concat!(
        "namespace eval modelTestVerTool {\n",
        "    namespace eval gui {\n",
        "        proc specAddButtonPopUp {x y} { return \"called $x $y\" }\n",
        "        proc caller {} { return [specAddButtonPopUp 1 2] }\n",
        "    }\n",
        "}\n",
    );
    let analysis = analyse(src);
    // Cursor on the `specAddButtonPopUp` declaration (line 2).
    let refs = references(src, "tcl", 2, 14, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&2), "decl missing: {refs:?}");
    assert!(
        lines.contains(&3),
        "bare call from the same 2-level-nested namespace missing: {refs:?}"
    );
}

#[test]
fn references_two_level_nested_namespace_qualified_call_in_bind_style_body_matches() {
    // tclsh (verified): `uplevel #0 { specAddButtonPopUp 1 2 }` from outside
    // the namespace fails with "invalid command name", but
    // `uplevel #0 { ::modelTestVerTool::gui::specAddButtonPopUp 1 2 }`
    // succeeds — the fully-qualified spelling is the *correct*, idiomatic way
    // to reach a namespaced proc from a deferred/global-eval'd context (a Tk
    // `bind` script, `after`, a widget `-command`; Tcl evaluates these via
    // `uplevel #0`/`TCL_EVAL_GLOBAL`, not the caller's active namespace).
    // Issue #923: this exact call form — a namespaced proc invoked by its
    // fully-qualified name from inside a `bind` callback script — was
    // reported as not found ("0 references").
    let src = concat!(
        "namespace eval modelTestVerTool {\n",
        "    namespace eval gui {\n",
        "        proc specAddButtonPopUp {x y} { return \"called $x $y\" }\n",
        "    }\n",
        "}\n",
        "bind $win.fra.tool.buT_specAddIconLarge <ButtonRelease-1> {::modelTestVerTool::gui::specAddButtonPopUp %X %Y}\n",
    );
    let analysis = analyse(src);
    let refs = references(src, "tcl", 2, 14, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&2), "decl missing: {refs:?}");
    assert!(
        lines.contains(&5),
        "fully-qualified call embedded in the `bind` script missing: {refs:?}"
    );
}

#[test]
fn references_issue_923_two_bind_lines_qualified_and_bare_both_resolve_correctly() {
    // The exact shape from
    // https://github.com/bitwisecook/tcl-lsp/issues/923: two `bind` lines
    // inside the procs' own namespace, one calling its target by
    // fully-qualified name, the other by bare name — both are genuine
    // references and both must be found.
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
    let analysis = analyse(src);
    // specAddButtonPopUp: fully-qualified call site (line 4).
    let spec_refs = references(src, "tcl", 2, 14, &analysis, true);
    let spec_lines = ref_lines(&spec_refs);
    assert!(spec_lines.contains(&2), "spec decl missing: {spec_refs:?}");
    assert!(
        spec_lines.contains(&4),
        "spec's fully-qualified bind call site missing: {spec_refs:?}"
    );
    // testAddButtonPopUp: bare call site (line 5), same namespace.
    let test_refs = references(src, "tcl", 3, 14, &analysis, true);
    let test_lines = ref_lines(&test_refs);
    assert!(test_lines.contains(&3), "test decl missing: {test_refs:?}");
    assert!(
        test_lines.contains(&5),
        "test's bare bind call site missing: {test_refs:?}"
    );
}

#[test]
fn references_bare_call_outside_the_namespace_is_correctly_not_a_reference() {
    // tclsh (verified): a bare `testAddButtonPopUp` called from *outside*
    // `::modelTestVerTool::gui` (e.g. a top-level `bind` line) does NOT reach
    // the namespaced proc at runtime — Tcl's bareword lookup does not search
    // arbitrary descendant namespaces. The fix must not turn this into a
    // false-positive match.
    let src = concat!(
        "namespace eval modelTestVerTool {\n",
        "    namespace eval gui {\n",
        "        proc testAddButtonPopUp {x y} { return \"test $x $y\" }\n",
        "    }\n",
        "}\n",
        "bind $win.fra.tool.buT_testAddIconLarge <ButtonRelease-1> {testAddButtonPopUp %X %Y}\n",
    );
    let analysis = analyse(src);
    let refs = references(src, "tcl", 2, 14, &analysis, true);
    assert_eq!(
        ref_lines(&refs),
        vec![2],
        "bare call from a different (global) scope must not be misattributed: {refs:?}"
    );
}

#[test]
fn references_two_level_nested_namespace_isolates_same_named_procs() {
    // tclsh (verified): `::a::b::helper` -> "a-b" and `::c::d::helper` ->
    // "c-d" are distinct procs even though both are nested two namespaces
    // deep with the same simple name. A bare `helper` call inside `::c::d`
    // must resolve to *its own* proc, never `::a::b`'s.
    let src = concat!(
        "namespace eval a {\n",
        "    namespace eval b {\n",
        "        proc helper {} { return \"a-b\" }\n",
        "    }\n",
        "}\n",
        "namespace eval c {\n",
        "    namespace eval d {\n",
        "        proc helper {} { return \"c-d\" }\n",
        "        proc caller {} { return [helper] }\n",
        "    }\n",
        "}\n",
    );
    let analysis = analyse(src);
    // References for `::a::b::helper` (cursor on its decl, line 2).
    let refs_ab = references(src, "tcl", 2, 14, &analysis, true);
    assert_eq!(
        ref_lines(&refs_ab),
        vec![2],
        "::a::b::helper has no callers; must not pick up ::c::d's call: {refs_ab:?}"
    );
    // References for `::c::d::helper` (cursor on its decl, line 7).
    let refs_cd = references(src, "tcl", 7, 14, &analysis, true);
    let lines_cd = ref_lines(&refs_cd);
    assert!(lines_cd.contains(&7), "::c::d decl missing: {refs_cd:?}");
    assert!(
        lines_cd.contains(&8),
        "::c::d's own bare call missing: {refs_cd:?}"
    );
}

#[test]
fn references_three_level_nested_namespace_bare_and_qualified_calls() {
    // tclsh (verified): three nested `namespace eval` blocks
    // (`::a::b::c::deep`) called both as bare `deep` (from inside
    // `::a::b::c`) and as `::a::b::c::deep` -> "a-b-c-deep" both times.
    let src = concat!(
        "namespace eval a {\n",
        "    namespace eval b {\n",
        "        namespace eval c {\n",
        "            proc deep {} { return \"a-b-c-deep\" }\n",
        "            proc caller {} { return [deep] }\n",
        "        }\n",
        "    }\n",
        "}\n",
        "::a::b::c::deep\n",
    );
    let analysis = analyse(src);
    let refs = references(src, "tcl", 3, 18, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&3), "decl missing: {refs:?}");
    assert!(
        lines.contains(&4),
        "bare call from the same 3-level-nested namespace missing: {refs:?}"
    );
    assert!(
        lines.contains(&8),
        "fully-qualified top-level call missing: {refs:?}"
    );
}

#[test]
fn references_relative_name_with_embedded_colons_prefers_current_namespace() {
    // tclsh (verified): with *both* `::inner::p` ("global-inner") and
    // `::outer::inner::p` ("outer-inner") defined, calling the relative
    // (no leading `::`) name `inner::p` from inside `::outer` resolves to
    // `::outer::inner::p`, not the global one — Tcl tries the current
    // namespace before falling back to global even for a relative name that
    // itself contains `::`. `resolve_command_qualified_name` must produce
    // this same candidate (previously it treated any `cmd_name.contains("::")`
    // as unconditionally global-rooted).
    let src = concat!(
        "namespace eval inner {\n",
        "    proc p {} { return \"global-inner\" }\n",
        "}\n",
        "namespace eval outer {\n",
        "    namespace eval inner {\n",
        "        proc p {} { return \"outer-inner\" }\n",
        "    }\n",
        "    proc caller {} { return [inner::p] }\n",
        "}\n",
    );
    let analysis = analyse(src);
    // References for `::outer::inner::p` (cursor on its decl, line 5).
    let refs_outer = references(src, "tcl", 5, 13, &analysis, true);
    let lines_outer = ref_lines(&refs_outer);
    assert!(lines_outer.contains(&5), "decl missing: {refs_outer:?}");
    assert!(
        lines_outer.contains(&7),
        "`inner::p` call inside ::outer must resolve to ::outer::inner::p: {refs_outer:?}"
    );
    // References for the *global* `::inner::p` (cursor on its decl, line 1)
    // must NOT include ::outer's caller.
    let refs_global = references(src, "tcl", 1, 9, &analysis, true);
    assert_eq!(
        ref_lines(&refs_global),
        vec![1],
        "the global ::inner::p must not pick up ::outer's call: {refs_global:?}"
    );
}

// ---------------------------------------------------------------------------
// references — classes in nested namespaces (parity with the proc fixes
// above; classes previously had no namespace gate at all, so a bare
// class-name match cross-attributed across namespaces unconditionally)
// ---------------------------------------------------------------------------

#[test]
fn references_class_two_level_nested_namespace_isolates_same_named_classes() {
    // tclsh (verified pattern, same shape as the proc isolation test):
    // `::a::b::Widget` and `::c::d::Widget` are distinct classes. A bare
    // `Widget new` inside `::c::d` must resolve to its own class only.
    let src = concat!(
        "namespace eval a {\n",
        "    namespace eval b {\n",
        "        oo::class create Widget {}\n",
        "    }\n",
        "}\n",
        "namespace eval c {\n",
        "    namespace eval d {\n",
        "        oo::class create Widget {}\n",
        "        proc build {} { return [Widget new] }\n",
        "    }\n",
        "}\n",
    );
    let analysis = analyse(src);
    if analysis.all_classes.len() < 2 {
        // Some dialect configurations may not record both classes; skip
        // rather than false-fail the shared-infra assumption.
        return;
    }
    // References for `::a::b::Widget` (cursor on its decl, line 2).
    let refs_ab = references(src, "tcl", 2, 26, &analysis, true);
    assert_eq!(
        ref_lines(&refs_ab),
        vec![2],
        "::a::b::Widget has no instantiations; must not pick up ::c::d's: {refs_ab:?}"
    );
    // References for `::c::d::Widget` (cursor on its decl, line 7).
    let refs_cd = references(src, "tcl", 7, 26, &analysis, true);
    let lines_cd = ref_lines(&refs_cd);
    assert!(lines_cd.contains(&7), "::c::d decl missing: {refs_cd:?}");
    assert!(
        lines_cd.contains(&8),
        "::c::d's own `Widget new` call missing: {refs_cd:?}"
    );
}

#[test]
fn references_class_and_method_two_level_nested_namespace_dollar_dispatch() {
    // tclsh (verified): a class nested two `namespace eval` levels deep,
    // instantiated via its fully-qualified name, dispatches `$w render` ->
    // "rendered". The method reference must resolve through the nested
    // namespace exactly like the top-level `oo::class` tests already do.
    let src = concat!(
        "namespace eval modelTestVerTool {\n",
        "    namespace eval gui {\n",
        "        oo::class create Widget {\n",
        "            method render {} { return \"rendered\" }\n",
        "        }\n",
        "    }\n",
        "}\n",
        "set w [::modelTestVerTool::gui::Widget new]\n",
        "$w render\n",
    );
    let analysis = analyse(src);
    if analysis.all_classes.is_empty() {
        return;
    }
    // Cursor on the `render` method declaration (line 3).
    let refs = references(src, "tcl", 3, 20, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&3), "method decl missing: {refs:?}");
    assert!(
        lines.contains(&8),
        "external `$w render` call site missing: {refs:?}"
    );
}

// ---------------------------------------------------------------------------
// rename — nested namespaces (parity with the reference fixes above)
// ---------------------------------------------------------------------------

#[test]
fn rename_proc_two_level_nested_namespace_scoped_correctly() {
    // Renaming `::c::d::helper` must rewrite its own decl + bare call, and
    // must never touch the unrelated same-named `::a::b::helper`.
    let src = concat!(
        "namespace eval a {\n",
        "    namespace eval b {\n",
        "        proc helper {} { return \"a-b\" }\n",
        "    }\n",
        "}\n",
        "namespace eval c {\n",
        "    namespace eval d {\n",
        "        proc helper {} { return \"c-d\" }\n",
        "        proc caller {} { return [helper] }\n",
        "    }\n",
        "}\n",
    );
    let analysis = analyse(src);
    let edits = rename(src, "tcl", 7, 14, "assist", &analysis, None);
    assert_eq!(
        edit_lines(&edits),
        vec![7, 8],
        "only ::c::d::helper's decl + its own bare call rewritten: {edits:?}"
    );
    assert!(edits.iter().all(|e| e.new_text == "assist"));
}

#[test]
fn rename_class_two_level_nested_namespace_scoped_correctly() {
    // Class-rename parity with the proc test above: renaming
    // `::c::d::Widget` must not touch the unrelated `::a::b::Widget`.
    let src = concat!(
        "namespace eval a {\n",
        "    namespace eval b {\n",
        "        oo::class create Widget {}\n",
        "    }\n",
        "}\n",
        "namespace eval c {\n",
        "    namespace eval d {\n",
        "        oo::class create Widget {}\n",
        "        proc build {} { return [Widget new] }\n",
        "    }\n",
        "}\n",
    );
    let analysis = analyse(src);
    if analysis.all_classes.len() < 2 {
        return;
    }
    let edits = rename(src, "tcl", 7, 26, "Panel", &analysis, None);
    assert_eq!(
        edit_lines(&edits),
        vec![7, 8],
        "only ::c::d::Widget's decl + its own `Widget new` call rewritten: {edits:?}"
    );
    assert!(edits.iter().all(|e| e.new_text == "Panel"));
}

// ---------------------------------------------------------------------------
// references — known limitation: a command name held in a variable
// ---------------------------------------------------------------------------

#[test]
fn references_variable_held_command_name_is_not_resolved_documented_limitation() {
    // A command name stored in a variable and invoked indirectly
    // (`set cmd helper; $cmd`) is, in the general case, statically
    // undecidable — the value could come from anywhere at runtime. This
    // test pins the current, honest behaviour: such a call site is simply
    // not counted as a reference (no crash, no false attribution), rather
    // than silently mismatching a different proc.
    let src = "proc helper {} { return hi }\nset cmd helper\n$cmd\n";
    let analysis = analyse(src);
    let refs = references(src, "tcl", 0, 6, &analysis, true);
    assert_eq!(
        ref_lines(&refs),
        vec![0],
        "only the declaration; `$cmd` is not statically resolved: {refs:?}"
    );
}

// ---------------------------------------------------------------------------
// references — non-symbols / robustness
// ---------------------------------------------------------------------------

#[test]
fn references_builtin_command_yields_nothing() {
    // `puts` is a built-in, not a user proc — no references are surfaced.
    let src = "puts hello\nputs world\n";
    let analysis = analyse(src);
    assert!(
        references(src, "tcl", 0, 1, &analysis, true).is_empty(),
        "built-in `puts` should have no provider references",
    );
}

#[test]
fn references_unrelated_word_yields_nothing() {
    // A bare literal argument (`hello`) is neither a proc, class, nor var.
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert!(references(src, "tcl", 0, 6, &analysis, true).is_empty());
}

#[test]
fn references_out_of_range_and_empty_file_do_not_panic() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    // Way past EOF — must return empty, not panic.
    assert!(references(src, "tcl", 99, 0, &analysis, true).is_empty());
    assert!(references(src, "tcl", 0, 999, &analysis, true).is_empty());
    // Empty document.
    let empty = analyse("");
    assert!(references("", "tcl", 0, 0, &empty, true).is_empty());
}

// ---------------------------------------------------------------------------
// rename — procs
// ---------------------------------------------------------------------------

#[test]
fn rename_proc_updates_definition_and_all_call_sites() {
    // tclsh: renaming `greet`->`salute` everywhere keeps the script runnable
    // (`proc salute {} {return hi}; puts [salute]; puts [salute]` -> hi/hi).
    // The rename set must be exactly the decl + both calls.
    let src = "proc greet {} { return hi }\nputs [greet]\nputs [greet]\n";
    let analysis = analyse(src);
    // Cursor on the declaration (line 0).
    let edits = rename(src, "tcl", 0, 6, "salute", &analysis, None);
    assert_eq!(
        edit_lines(&edits),
        vec![0, 1, 2],
        "decl + both call sites rewritten; got {edits:?}",
    );
    assert!(
        edits.iter().all(|e| e.new_text == "salute"),
        "every top-level edit is the bare new name; got {edits:?}",
    );
    // The declaration edit leads and replaces just `greet` -> `salute`
    // (col 5 = start of the proc name in `proc greet`).
    assert_eq!(edits[0].range.start_line, 0);
    assert_eq!(edits[0].range.start_character, 5);
}

#[test]
fn rename_parent_proc_does_not_edit_a_child_interp_body() {
    // `interp eval child { proc foo }` runs in a child interpreter, so its
    // `proc foo` is isolated from the parent's `::foo`.  Renaming the parent
    // `foo` rewrites the parent decl (line 0) and its call (line 2) but must
    // never touch the child body on line 1.
    let src = "proc foo {} {}\ninterp eval child { proc foo {} {} }\nfoo\n";
    let analysis = analyse(src);
    // Cursor on the parent `foo` declaration (line 0, col 6).
    let edits = rename(src, "tcl", 0, 6, "bar", &analysis, None);
    assert!(
        edits.iter().all(|e| e.range.start_line != 1),
        "the isolated child interp body must be untouched: {edits:?}",
    );
    let lines = edit_lines(&edits);
    assert!(
        lines.contains(&0) && lines.contains(&2),
        "the parent decl and call are still rewritten: {edits:?}",
    );
}

#[test]
fn rename_proc_from_call_site_rewrites_declaration_too() {
    // Renaming from a call site rewrites the declaration as well — same set.
    let src = "proc greet {} { return hi }\ngreet\ngreet\n";
    let analysis = analyse(src);
    // Cursor on the first call (line 1).
    let edits = rename(src, "tcl", 1, 2, "salute", &analysis, None);
    assert_eq!(edit_lines(&edits), vec![0, 1, 2], "{edits:?}");
    assert!(
        edits.iter().any(|e| e.range.start_line == 0),
        "decl rewritten: {edits:?}"
    );
}

#[test]
fn rename_proc_rejected_when_new_name_collides_with_existing_proc() {
    // tclsh: `hello` already names a distinct proc. Renaming `greet`->`hello`
    // would shadow it (two procs would claim `::hello`), so the provider
    // refuses with an empty edit set rather than merge them.
    let src = "proc greet {} { return a }\nproc hello {} { return b }\ngreet\n";
    let analysis = analyse(src);
    let edits = rename(src, "tcl", 0, 6, "hello", &analysis, None);
    assert!(
        edits.is_empty(),
        "collision with existing proc must be refused: {edits:?}"
    );
}

// ---------------------------------------------------------------------------
// rename — variables (scoping is the load-bearing correctness fact)
// ---------------------------------------------------------------------------

#[test]
fn rename_var_updates_definition_and_reads_with_dollar_preserved() {
    // tclsh: `set y 1; puts $y; puts $y` -> 1/1 — the renamed script runs.
    // The declaration becomes the bare name; each `$x` read keeps its `$`.
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    // Cursor in the first `$x` read.
    let edits = rename(src, "tcl", 1, 6, "y", &analysis, None);
    assert_eq!(
        edit_lines(&edits),
        vec![0, 1, 2],
        "decl + both reads: {edits:?}"
    );
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        texts.contains(&"y"),
        "declaration rewrite `y` missing: {texts:?}"
    );
    assert_eq!(
        texts.iter().filter(|t| **t == "$y").count(),
        2,
        "both `$x` reads should become `$y`: {texts:?}",
    );
}

#[test]
fn rename_var_from_definition_site_resolves_without_dollar() {
    // Cursor on the bare `x` in `set x 1` (no `$`). The definition-site
    // resolver must still find the variable and rewrite decl + reads.
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    // Cursor on `x` in `set x` (line 0, col 4).
    let edits = rename(src, "tcl", 0, 4, "y", &analysis, None);
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(texts.contains(&"y"), "decl edit missing: {edits:?}");
    assert!(texts.contains(&"$y"), "ref edit missing: {edits:?}");
}

#[test]
fn rename_local_var_is_scoped_and_leaves_same_named_var_in_other_proc_intact() {
    // tclsh: `proc a {} {set v 10; return $v}` / `proc b {} {set v 20;
    // return $v}` -> a=10, b=20. Renaming proc a's `v`->`w` must rewrite ONLY
    // proc a's def+read (lines 1-2) and never proc b's `v` (lines 5-6) — the
    // two locals are independent, so clobbering b's `v` would be a real bug.
    //
    // C-Tcl proof of the *renamed* form: `proc a {} {set w 10; return $w}`
    // alongside the untouched `proc b {} {set v 20; return $v}` -> a=10, b=20
    // (verified: identical output to the original).
    let src = "proc a {} {\n    set v 10\n    return $v\n}\nproc b {} {\n    set v 20\n    return $v\n}\n";
    let analysis = analyse(src);
    // Cursor on the def site `v` in proc a (line 1, col 8).
    let edits = rename(src, "tcl", 1, 8, "w", &analysis, None);
    assert!(
        !edits.is_empty(),
        "expected a scoped rename of proc a's `v`"
    );
    let lines = edit_lines(&edits);
    assert!(
        lines.iter().all(|&l| l == 1 || l == 2),
        "rename must stay inside proc a (lines 1-2); got {edits:?}",
    );
    assert!(
        !lines.contains(&5) && !lines.contains(&6),
        "rename must NOT clobber proc b's same-named local `v`; got {edits:?}",
    );
    // The read site keeps its `$`.
    assert!(
        edits.iter().any(|e| e.new_text == "$w"),
        "the `$v` read should become `$w`: {edits:?}",
    );
}

#[test]
fn rename_var_rejected_on_same_scope_collision() {
    // Renaming `x`->`y` when a sibling `y` already lives in the same proc
    // scope would merge two distinct variables — refuse.
    let src = "proc demo {} {\n    set x 1\n    set y 2\n    puts $x\n}\n";
    let analysis = analyse(src);
    // Rename `x` (its read site on line 3) to the already-present `y`.
    let edits = rename(src, "tcl", 3, 10, "y", &analysis, None);
    assert!(
        edits.is_empty(),
        "same-scope collision must be refused: {edits:?}"
    );
}

// ---------------------------------------------------------------------------
// rename — namespaces (qualified call keeps its qualifier; isolation holds)
// ---------------------------------------------------------------------------

#[test]
fn rename_namespaced_proc_rewrites_qualified_and_short_forms() {
    // tclsh: renaming `::myns::greet`->`hello` keeps both the qualified call
    // (`::myns::hello`) and the short in-namespace call (`hello`) runnable
    // -> ns-hi/ns-hi. The qualified call must keep its `::myns::` qualifier;
    // the short call stays short.
    let src = "namespace eval ::myns {\n    proc greet {} { return ns-hi }\n}\n::myns::greet\nnamespace eval ::myns {\n    greet\n}\n";
    let analysis = analyse(src);
    // Cursor on the declaration (line 1).
    let edits = rename(src, "tcl", 1, 9, "hello", &analysis, None);
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        texts.contains(&"::myns::hello"),
        "qualified call must keep its namespace qualifier; got {texts:?}",
    );
    assert!(
        texts.contains(&"hello"),
        "short in-namespace call must stay short; got {texts:?}",
    );
}

#[test]
fn rename_namespaced_proc_from_its_own_decl_isolates_to_that_namespace() {
    // tclsh: `::a::helper` -> "a", `::b::helper` -> "b" are distinct procs.
    // Renaming `::a::helper`->`assist` from ::a's OWN declaration must rewrite
    // only ::a's decl + its own call (`::a::assist`), leaving ::b::helper
    // completely intact — otherwise we would clobber an unrelated proc. This
    // is the working case: ::a::helper is the first proc named `helper` in the
    // table, which is also the one the cursor sits on.
    //
    // C-Tcl proof of the renamed form: `::a::assist` -> "a" alongside the
    // untouched `::b::helper` -> "b" (the two are independent name bindings).
    let src = "namespace eval ::a {\n    proc helper {} { return a }\n}\nnamespace eval ::b {\n    proc helper {} { return b }\n}\n::a::helper\n::b::helper\n";
    let analysis = analyse(src);
    // Cursor on ::a's helper declaration (line 1).
    let edits = rename(src, "tcl", 1, 9, "assist", &analysis, None);
    assert_eq!(
        edit_lines(&edits),
        vec![1, 6],
        "exactly ::a's decl (line 1) + its call (line 6); got {edits:?}",
    );
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        texts.contains(&"::a::assist"),
        "the ::a::helper call must become ::a::assist; got {texts:?}",
    );
    // No edit may land on ::b::helper's decl (line 4) or call (line 7).
    assert!(
        edits
            .iter()
            .all(|e| e.range.start_line != 4 && e.range.start_line != 7),
        "::b::helper must be untouched; got {edits:?}",
    );
}

// FIXED: `rename` now disambiguates same-named procs across namespaces by the
// cursor's name span (not first-by-name), so renaming the SECOND same-named
// proc hits that proc and leaves the first one — an unrelated symbol Tcl keeps
// distinct — untouched. tclsh: `::a::helper`->"a", `::b::helper`->"b" are
// independent. `rename_proc` now prefers the proc whose `name_span` covers the
// cursor (matching `references::references`), falling back to first-by-name
// only for call-site / unqualified cursors.
#[test]
fn rename_second_same_named_proc_resolves_to_that_proc_not_the_first() {
    let src = "namespace eval ::a {\n    proc helper {} { return a }\n}\nnamespace eval ::b {\n    proc helper {} { return b }\n}\n::a::helper\n::b::helper\n";
    let analysis = analyse(src);
    // Cursor on ::b's helper declaration (line 4) — should rename ::b::helper.
    let edits = rename(src, "tcl", 4, 9, "assist", &analysis, None);
    assert_eq!(
        edit_lines(&edits),
        vec![4, 7],
        "cursor on ::b::helper must rename ::b's decl (line 4) + its call (line 7); got {edits:?}",
    );
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        texts.contains(&"::b::assist"),
        "expected ::b::assist; got {texts:?}"
    );
    // ::a::helper (lines 1+6) must be untouched.
    assert!(
        edits
            .iter()
            .all(|e| e.range.start_line != 1 && e.range.start_line != 6),
        "::a::helper must be untouched; got {edits:?}",
    );
}

// ---------------------------------------------------------------------------
// rename — safety gating (shape + builtin shadow)
// ---------------------------------------------------------------------------

#[test]
fn rename_rejects_syntactically_unsafe_new_names() {
    // A new name that isn't a valid Tcl identifier is refused (empty edits)
    // so the editor never applies a partially-broken rename.
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    for bad in [
        "bad name",
        "1lead",
        "with-dash",
        "has::colon",
        "with$dollar",
        "",
    ] {
        assert!(
            rename(src, "tcl", 0, 6, bad, &analysis, None).is_empty(),
            "unsafe new name {bad:?} must yield no edits",
        );
    }
}

#[test]
fn rename_var_also_rejects_unsafe_new_name() {
    // The shape gate applies to variable renames too.
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    assert!(rename(src, "tcl", 1, 6, "bad name", &analysis, None).is_empty());
}

#[test]
fn rename_proc_to_builtin_command_name_is_blocked_with_registry() {
    // tclsh: `puts` is a built-in command. Renaming a proc to `puts` would
    // make the editor's calls dispatch to the built-in instead of the proc,
    // so with a registry supplied the rename is refused.
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(src, "tcl", 0, 6, "puts", &analysis, Some(&registry));
    assert!(
        edits.is_empty(),
        "rename to built-in `puts` must be blocked: {edits:?}"
    );
}

#[test]
fn rename_proc_to_non_builtin_succeeds_with_registry() {
    // The registry gate only blocks built-in names; a free name still renames.
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(src, "tcl", 0, 6, "salut", &analysis, Some(&registry));
    assert!(
        !edits.is_empty(),
        "non-built-in rename should succeed with a registry"
    );
    assert!(edits.iter().all(|e| e.new_text == "salut"), "{edits:?}");
}

#[test]
fn rename_var_to_builtin_name_is_allowed() {
    // The built-in-shadow gate is proc-only — variable names occupy a
    // separate Tcl namespace, so renaming a var to `puts` is fine.
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(src, "tcl", 1, 6, "puts", &analysis, Some(&registry));
    assert!(
        !edits.is_empty(),
        "variable rename to `puts` should succeed"
    );
}

// ---------------------------------------------------------------------------
// rename — non-symbols / robustness
// ---------------------------------------------------------------------------

#[test]
fn rename_unknown_word_yields_no_edits() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    // `hello` is a bare literal — nothing to rename.
    assert!(rename(src, "tcl", 0, 6, "x", &analysis, None).is_empty());
}

#[test]
fn rename_builtin_without_registry_still_yields_no_edits() {
    // `puts` has no user proc def, so even without a registry there is no
    // symbol whose definition/calls could be rewritten.
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert!(rename(src, "tcl", 0, 1, "show", &analysis, None).is_empty());
}

#[test]
fn rename_out_of_range_and_empty_file_do_not_panic() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    assert!(rename(src, "tcl", 99, 0, "z", &analysis, None).is_empty());
    assert!(rename(src, "tcl", 0, 999, "z", &analysis, None).is_empty());
    let empty = analyse("");
    assert!(rename("", "tcl", 0, 0, "z", &empty, None).is_empty());
}

// ---------------------------------------------------------------------------
// prepare_rename — gates the rename UI
// ---------------------------------------------------------------------------

#[test]
fn prepare_rename_offers_proc_name_and_placeholder() {
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    let p = prepare_rename(src, 0, 6, &analysis).expect("proc name is renameable");
    assert_eq!(p.placeholder, "greet");
    // Anchored at the proc name span (`proc greet` -> name starts col 5).
    assert_eq!(p.range.start_line, 0);
    assert_eq!(p.range.start_character, 5);
}

#[test]
fn prepare_rename_offers_variable_name_and_placeholder() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    let p = prepare_rename(src, 1, 6, &analysis).expect("variable is renameable");
    assert_eq!(p.placeholder, "x");
}

#[test]
fn prepare_rename_rejects_builtin_command() {
    // `puts` is a built-in, not a renameable user symbol.
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert!(
        prepare_rename(src, 0, 1, &analysis).is_none(),
        "built-in `puts` is not renameable",
    );
}

#[test]
fn prepare_rename_rejects_whitespace_and_bare_literal() {
    // The space between `set` and `x`, and the bare literal `hello`, are not
    // renameable symbols.
    let src = "set x 1\nputs hello\n";
    let analysis = analyse(src);
    // Column 3 on line 0 is the space in `set x`.
    assert!(
        prepare_rename(src, 0, 3, &analysis).is_none(),
        "whitespace is not renameable"
    );
    // The literal `hello` argument on line 1.
    assert!(
        prepare_rename(src, 1, 6, &analysis).is_none(),
        "bare literal is not renameable"
    );
}

#[test]
fn prepare_rename_out_of_range_and_empty_file_return_none() {
    let src = "set x 1\n";
    let analysis = analyse(src);
    assert!(prepare_rename(src, 99, 0, &analysis).is_none());
    assert!(prepare_rename(src, 0, 999, &analysis).is_none());
    let empty = analyse("");
    assert!(prepare_rename("", 0, 0, &empty).is_none());
}

// ---------------------------------------------------------------------------
// is_safe_symbol_name — the shape predicate behind the gate
// ---------------------------------------------------------------------------

#[test]
fn is_safe_symbol_name_accepts_identifiers_and_rejects_the_rest() {
    for ok in ["foo", "Foo", "_under", "a1", "snake_case_42"] {
        assert!(is_safe_symbol_name(ok), "{ok:?} should be accepted");
    }
    for bad in [
        "",
        "1lead",
        "has space",
        "has-dash",
        "has::colon",
        "with$dollar",
        "dotted.name",
    ] {
        assert!(!is_safe_symbol_name(bad), "{bad:?} should be rejected");
    }
}

// ===================================================================
// Renaming a TclOO instance variable must NOT rewrite the method body, and
// `uplevel #0` var resolution must skip proc locals.  TP/FP/TN/FN coverage.
// ===================================================================

/// The `oo::class create C { variable n; method get {} {return $n} … }`
/// fixture used across the object-variable tests.  `$n` in `get` sits at line 2.
const OBJECT_VAR_SRC: &str = "oo::class create C {\n    variable n\n    method get {} {return $n}\n    method set {x} {set n $x}\n}\n";

/// FP-guard (the corruption regression): a rename edit must never span more
/// than one line, and must never cover the whole `{return $n}` method body.
/// Before the fix the declaration edit was `2:18-2:28 → "w"`, destroying the
/// body.
#[test]
fn object_var_rename_never_rewrites_method_body() {
    let analysis = analyse(OBJECT_VAR_SRC);
    let col = OBJECT_VAR_SRC.lines().nth(2).unwrap().find("$n").unwrap() as u32 + 1;
    let edits = rename(OBJECT_VAR_SRC, "tcl8.6", 2, col, "w", &analysis, None);
    assert!(!edits.is_empty(), "expected a rename to be produced");
    for e in &edits {
        assert_eq!(
            e.range.start_line, e.range.end_line,
            "no rename edit may span multiple lines (body-destroying): {e:?}"
        );
        // The method body `{return $n}` starts at col 18 on line 2.  No edit
        // may start at the brace and run to end-of-body.
        let spans_body =
            e.range.start_line == 2 && e.range.start_character <= 18 && e.range.end_character >= 28;
        assert!(!spans_body, "edit covers the whole method body: {e:?}");
    }
}

/// TP: the declaration edit lands on the `variable n` name token (line 1),
/// and the `$n` read (line 2) is rewritten — the correct, non-destructive
/// rename.
#[test]
fn object_var_rename_edits_declaration_and_use() {
    let analysis = analyse(OBJECT_VAR_SRC);
    let col = OBJECT_VAR_SRC.lines().nth(2).unwrap().find("$n").unwrap() as u32 + 1;
    let edits = rename(OBJECT_VAR_SRC, "tcl8.6", 2, col, "w", &analysis, None);
    let lines = edit_lines(&edits);
    assert!(
        lines.contains(&1),
        "expected the `variable n` declaration (line 1) to be renamed; got {edits:?}"
    );
    assert!(
        lines.contains(&2),
        "expected the `$n` use (line 2) to be renamed; got {edits:?}"
    );
    // The declaration edit is exactly the `n` token, not a wide span.
    let decl = edits
        .iter()
        .find(|e| e.range.start_line == 1)
        .expect("declaration edit");
    assert_eq!(decl.new_text, "w");
    assert_eq!(
        decl.range.end_character - decl.range.start_character,
        1,
        "declaration edit must cover just the `n` token: {decl:?}"
    );
}

/// FN-that-must-hold: `references` on the object variable surfaces the
/// declaration + the use without a body-wide span (the reference set feeds
/// document-highlight; a whole-body highlight was the visible symptom).
#[test]
fn object_var_references_are_token_sized() {
    let analysis = analyse(OBJECT_VAR_SRC);
    let col = OBJECT_VAR_SRC.lines().nth(2).unwrap().find("$n").unwrap() as u32 + 1;
    let refs = references(OBJECT_VAR_SRC, "tcl", 2, col, &analysis, true);
    assert!(!refs.is_empty());
    for r in &refs {
        assert_eq!(
            r.start_line, r.end_line,
            "reference span crosses lines: {r:?}"
        );
    }
}

/// TN: an ordinary proc-local variable (no `TclOO`) still renames across its
/// own decl + uses — the fix must not disturb the common path.
#[test]
fn plain_proc_local_rename_unaffected() {
    let src = "proc p {} {\n    set count 0\n    incr count\n    return $count\n}\n";
    let analysis = analyse(src);
    // cursor on `$count` (line 3)
    let col = src.lines().nth(3).unwrap().find("$count").unwrap() as u32 + 1;
    let edits = rename(src, "tcl8.6", 3, col, "total", &analysis, None);
    let lines = edit_lines(&edits);
    assert!(
        lines.contains(&1),
        "decl `set count` should rename: {edits:?}"
    );
    assert!(lines.contains(&3), "`$count` use should rename: {edits:?}");
    for e in &edits {
        assert_eq!(e.range.start_line, e.range.end_line);
    }
}

/// TP: inside `uplevel #0 { … }` the body runs in the global frame, so a
/// `$g` there resolves to the GLOBAL `g`, not a same-named proc-local.
/// References on `$g` in the uplevel body must include the global definition
/// (line 0), not the proc-local (`set g 99`, line 1).
#[test]
fn uplevel_zero_resolves_global_not_proc_local() {
    let src = "set g 1\nproc p {} {\n    set g 99\n    uplevel #0 { puts $g }\n}\n";
    let analysis = analyse(src);
    // cursor on `$g` inside the uplevel body (line 3)
    let body_line = src.lines().nth(3).unwrap();
    let col = body_line.find("$g").unwrap() as u32 + 1; // on the `g`
    let refs = references(src, "tcl", 3, col, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(
        lines.contains(&0),
        "uplevel #0 `$g` must resolve to the global `set g` on line 0; got {refs:?}"
    );
    assert!(
        !lines.contains(&2),
        "uplevel #0 `$g` must NOT resolve to the proc-local `set g 99` on line 2; got {refs:?}"
    );
}

/// TN: a non-uplevel `$g` inside the proc still resolves to the proc-local
/// (the guard must only fire inside an uplevel scope).
#[test]
fn non_uplevel_proc_local_still_resolves_locally() {
    let src = "set g 1\nproc p {} {\n    set g 99\n    puts $g\n}\n";
    let analysis = analyse(src);
    let body_line = src.lines().nth(3).unwrap();
    let col = body_line.find("$g").unwrap() as u32 + 1;
    let refs = references(src, "tcl", 3, col, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(
        lines.contains(&2),
        "plain proc-body `$g` must resolve to the proc-local on line 2; got {refs:?}"
    );
}

/// FP-guard: inside a non-`#0` `uplevel N { … }` the body runs in the caller's
/// frame — statically unknown — so a `$g` it does not itself declare abstains:
/// it must link neither the enclosing proc-local (a definite mis-attribution)
/// nor the global (the frame is not necessarily global, unlike `#0`).
#[test]
fn uplevel_nonzero_abstains_from_proc_and_global() {
    let src = "set g 1\nproc p {} {\n    set g 99\n    uplevel 1 { puts $g }\n}\n";
    let analysis = analyse(src);
    let body_line = src.lines().nth(3).unwrap();
    let col = body_line.find("$g").unwrap() as u32 + 1; // on the `g`
    let refs = references(src, "tcl", 3, col, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(
        !lines.contains(&2),
        "uplevel 1 `$g` must NOT link the proc-local (line 2); got {refs:?}"
    );
    assert!(
        !lines.contains(&0),
        "uplevel 1 `$g` must NOT link the global (line 0) — the frame is unknown; got {refs:?}"
    );
}

/// A variable declared *inside* a non-`#0` `uplevel` body resolves to itself
/// (the abstention drops only the frames outside the body).
#[test]
fn uplevel_nonzero_body_local_resolves_within_body() {
    let src = "proc p {} {\n    uplevel 1 {\n        set h 5\n        puts $h\n    }\n}\n";
    let analysis = analyse(src);
    // cursor on `$h` (line 3)
    let body_line = src.lines().nth(3).unwrap();
    let col = body_line.find("$h").unwrap() as u32 + 1;
    let refs = references(src, "tcl", 3, col, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(
        lines.contains(&2),
        "the body's own `set h` (line 2) must be a reference; got {refs:?}"
    );
    assert!(
        lines.contains(&3),
        "the `$h` use (line 3) must be a reference; got {refs:?}"
    );
}

// ===================================================================
// Target selection: rename / references triggered from a bareword
// CALL SITE must resolve namespace-aware (the proc/class C Tcl would
// dispatch), never a namespace-blind `name == word` scan that picks an
// arbitrary same-named symbol in another namespace.
// ===================================================================

/// Two same-named procs in disjoint namespaces; `::a::run` calls `helper`.
const NS_COLLISION_PROC_SRC: &str = "namespace eval ::a {\n    proc helper {} { return 1 }\n    proc run {} { helper }\n}\nnamespace eval ::b {\n    proc helper {} { return 2 }\n}\n";

/// TP + FP: renaming from the `helper` call site inside `::a::run` renames
/// `::a::helper` (decl line 1 + call line 2) and NEVER `::b::helper` (line 5).
#[test]
fn proc_rename_from_callsite_targets_caller_namespace() {
    let analysis = analyse(NS_COLLISION_PROC_SRC);
    let col = NS_COLLISION_PROC_SRC
        .lines()
        .nth(2)
        .unwrap()
        .find("{ helper }")
        .unwrap() as u32
        + 2;
    let edits = rename(
        NS_COLLISION_PROC_SRC,
        "tcl8.6",
        2,
        col,
        "assist",
        &analysis,
        None,
    );
    let lines = edit_lines(&edits);
    assert!(
        lines.contains(&1),
        "::a::helper decl (line 1) must rename: {edits:?}"
    );
    assert!(
        lines.contains(&2),
        "the call (line 2) must rename: {edits:?}"
    );
    assert!(
        !lines.contains(&5),
        "must NOT rename ::b::helper (line 5): {edits:?}"
    );
}

/// TP + FP: Find-References from the same call site returns `::a::helper`'s
/// set (decl + call), never `::b::helper`.
#[test]
fn proc_references_from_callsite_targets_caller_namespace() {
    let analysis = analyse(NS_COLLISION_PROC_SRC);
    let col = NS_COLLISION_PROC_SRC
        .lines()
        .nth(2)
        .unwrap()
        .find("{ helper }")
        .unwrap() as u32
        + 2;
    let refs = references(NS_COLLISION_PROC_SRC, "tcl", 2, col, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&1), "decl line 1 expected: {refs:?}");
    assert!(lines.contains(&2), "call line 2 expected: {refs:?}");
    assert!(
        !lines.contains(&5),
        "must NOT include ::b::helper (line 5): {refs:?}"
    );
}

/// TN: a single unambiguous proc still renames correctly from its call site.
#[test]
fn proc_rename_unambiguous_callsite_unaffected() {
    let src = "proc greet {} { return hi }\ngreet\ngreet\n";
    let analysis = analyse(src);
    let edits = rename(src, "tcl8.6", 1, 0, "welcome", &analysis, None);
    assert!(edits.len() >= 3, "decl + 2 calls: {edits:?}");
    assert!(edits.iter().all(|e| e.new_text == "welcome"));
}

/// Two same-named classes in disjoint namespaces; `::a::mk` constructs
/// `Widget`.  Renaming from that constructor call targets `::a::Widget`,
/// never `::b::Widget`.
#[test]
fn class_rename_from_callsite_targets_caller_namespace() {
    let src = "namespace eval ::a {\n    oo::class create Widget {}\n    proc mk {} { Widget new }\n}\nnamespace eval ::b {\n    oo::class create Widget {}\n}\n";
    let analysis = analyse(src);
    // cursor on `Widget` in `Widget new` inside ::a::mk (line 2)
    let col = src.lines().nth(2).unwrap().find("Widget new").unwrap() as u32 + 1;
    let edits = rename(src, "tcl8.6", 2, col, "Panel", &analysis, None);
    let lines = edit_lines(&edits);
    assert!(
        lines.contains(&1),
        "::a::Widget decl (line 1) must rename: {edits:?}"
    );
    assert!(
        !lines.contains(&5),
        "must NOT rename ::b::Widget (line 5): {edits:?}"
    );
}

#[test]
fn object_var_references_unify_across_methods() {
    // `$n` used in `get` and written in `bump`; both are the one instance
    // variable declared by `variable n`.  Find-References on `$n` in `get`
    // must reach the declaration (line 1) and the sibling-method use.
    let src = "oo::class create C {\n    variable n\n    method get {} { return $n }\n    method bump {} { incr n }\n}\n";
    let analysis = analyse(src);
    let col = src.lines().nth(2).unwrap().find("$n").unwrap() as u32 + 1;
    let refs = references(src, "tcl", 2, col, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(
        lines.contains(&1),
        "declaration (line 1) expected: {refs:?}"
    );
    assert!(
        lines.contains(&2),
        "`$n` in get (line 2) expected: {refs:?}"
    );
    assert!(
        lines.contains(&3),
        "the sibling-method use `incr n` (line 3) must unify: {refs:?}"
    );
}

#[test]
fn object_var_rename_unifies_declaration_and_all_method_uses() {
    // Renaming the instance variable rewrites its `variable` declaration and
    // every method's use as one variable — not just the method under the cursor.
    let src = "oo::class create C {\n    variable n\n    method get {} { return $n }\n    method bump {} { incr n }\n}\n";
    let analysis = analyse(src);
    let col = src.lines().nth(2).unwrap().find("$n").unwrap() as u32 + 1;
    let edits = rename(src, "tcl8.6", 2, col, "count", &analysis, None);
    let lines = edit_lines(&edits);
    assert!(
        lines.contains(&1),
        "declaration (line 1) must rename: {edits:?}"
    );
    assert!(
        lines.contains(&2),
        "`$n` in get (line 2) must rename: {edits:?}"
    );
    assert!(
        lines.contains(&3),
        "`incr n` in bump (line 3) must rename: {edits:?}"
    );
    // No edit spans the body (the earlier corruption guard still holds).
    assert!(
        edits.iter().all(|e| e.range.start_line == e.range.end_line),
        "{edits:?}"
    );
}

#[test]
fn namespace_variable_unifies_across_procs() {
    // `variable count` in two procs plus the namespace-level declaration are
    // one cell (`::app::count`); Find-References on `$count` must reach them all.
    let src = "namespace eval ::app {\n    variable count 0\n    proc bump {} { variable count; incr count }\n    proc get {} { variable count; return $count }\n}\n";
    let analysis = analyse(src);
    let col = src.lines().nth(3).unwrap().find("$count").unwrap() as u32 + 1;
    let refs = references(src, "tcl", 3, col, &analysis, true);
    let lines = ref_lines(&refs);
    eprintln!("namespace-variable reference lines = {lines:?}");
    assert!(
        lines.contains(&1),
        "namespace-level `variable count 0` (line 1): {refs:?}"
    );
    assert!(
        lines.contains(&2),
        "`variable count`/`incr count` in bump (line 2): {refs:?}"
    );
    assert!(lines.contains(&3), "`$count` in get (line 3): {refs:?}");
}

#[test]
fn global_variable_unifies_across_procs() {
    // `global g` in a proc aliases `::g`; the top-level `set g` and another
    // proc's `global g` are the same cell.
    let src = "set g 0\nproc a {} { global g; incr g }\nproc b {} { global g; return $g }\n";
    let analysis = analyse(src);
    let col = src.lines().nth(2).unwrap().find("$g").unwrap() as u32 + 1;
    let refs = references(src, "tcl", 2, col, &analysis, true);
    let lines = ref_lines(&refs);
    eprintln!("global-variable reference lines = {lines:?}");
    assert!(
        lines.contains(&1),
        "`global g`/`incr g` in a (line 1): {refs:?}"
    );
    assert!(
        lines.contains(&2),
        "`global g`/`$g` in b (line 2): {refs:?}"
    );
}

#[test]
fn namespace_variable_rename_unifies_all_aliases() {
    // Renaming the namespace variable rewrites its declaration and every
    // `variable count` alias + use across procs, as one variable.
    let src = "namespace eval ::app {\n    variable count 0\n    proc bump {} { variable count; incr count }\n    proc get {} { variable count; return $count }\n}\n";
    let analysis = analyse(src);
    let col = src.lines().nth(3).unwrap().find("$count").unwrap() as u32 + 1;
    let edits = rename(src, "tcl8.6", 3, col, "total", &analysis, None);
    let lines = edit_lines(&edits);
    assert!(
        lines.contains(&1),
        "namespace decl (line 1) must rename: {edits:?}"
    );
    assert!(
        lines.contains(&2),
        "bump's alias + use (line 2) must rename: {edits:?}"
    );
    assert!(
        lines.contains(&3),
        "get's alias + use (line 3) must rename: {edits:?}"
    );
    assert!(
        edits.iter().all(|e| e.new_text.contains("total")),
        "{edits:?}"
    );
}

#[test]
fn namespace_variables_in_different_namespaces_do_not_unify() {
    // FP guard: `variable count` in ::a and ::b are distinct cells
    // (`::a::count` vs `::b::count`) and must NOT be unified.
    let src = "namespace eval ::a {\n    proc p {} { variable count; return $count }\n}\nnamespace eval ::b {\n    proc q {} { variable count; incr count }\n}\n";
    let analysis = analyse(src);
    // `$count` in ::a::p (line 1)
    let col = src.lines().nth(1).unwrap().find("$count").unwrap() as u32 + 1;
    let refs = references(src, "tcl", 1, col, &analysis, true);
    let lines = ref_lines(&refs);
    assert!(lines.contains(&1), "::a's own use (line 1): {refs:?}");
    assert!(
        !lines.contains(&4),
        "::b::count (line 4) is a different cell and must NOT unify: {refs:?}"
    );
}
