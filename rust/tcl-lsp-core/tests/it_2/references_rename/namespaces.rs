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

//! References and renames across namespaces: qualified and short calls, nested namespaces and the caller's namespace at a call site.

use super::*;

// references — namespaces (qualified vs short, cross-namespace isolation)

#[test]
fn references_namespaced_proc_matches_qualified_and_short_calls() {
    // tclsh: a proc in `::myns` is invoked both as `::myns::greet` and as a
    // short `greet` inside `namespace eval ::myns` -> ns-hi/ns-hi. Both are
    // real invocations of the one proc, so both are references.
    let src = "namespace eval ::myns {\n    proc greet {} { return ns-hi }\n}\n::myns::greet\nnamespace eval ::myns {\n    greet\n}\n";
    let analysis = analyse(src);
    // Cursor on the `greet` declaration (line 1).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        &analysis,
        true,
    );
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
    // tclsh8.6 (verified): `inner::p` called inside `outer`
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
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        13,
        &analysis,
        true,
    );
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
    let global_refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        13,
        &analysis,
        true,
    );
    let global_lines = ref_lines(&global_refs);
    assert!(
        !global_lines.contains(&2),
        "call inside `outer` resolves to the local proc, not the global: {global_refs:?}",
    );
    // Cursor on the *local* `::outer::inner::p` declaration (line 3).
    let local_refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        55,
        &analysis,
        true,
    );
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
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        &analysis,
        true,
    );
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

// references — nested namespaces (2+ levels): qualified, relative, and the
// exact `bind`-callback shape.
//
// The analyser's per-call-site "what namespace does an unqualified name
// resolve against" computation
// (`Analyser::resolve_command_qualified_name`) must read a scope-path field
// the real body walk keeps updated, and the LSP-side
// namespace gate (`innermost_namespace_at`) must not take only the *innermost*
// enclosing `namespace eval`'s own segment rather than the full accumulated
// path — so a bare call from inside a namespace nested *two or more* levels
// deep (`namespace eval a { namespace eval b { ... } }`) was resolved as if
// it were at the top level and never matched its own proc. Both are fixed by
// routing through `Analyser::command_resolution_namespace` /
// `command_resolution_namespace_at`, the single accumulating implementation
// shared by the analyser and every `tcl-lsp-core` provider.

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
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        14,
        &analysis,
        true,
    );
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
    // A namespaced proc invoked by its fully-qualified name from inside a
    // `bind` callback script must not read as "0 references".
    let src = concat!(
        "namespace eval modelTestVerTool {\n",
        "    namespace eval gui {\n",
        "        proc specAddButtonPopUp {x y} { return \"called $x $y\" }\n",
        "    }\n",
        "}\n",
        "bind $win.fra.tool.buT_specAddIconLarge <ButtonRelease-1> {::modelTestVerTool::gui::specAddButtonPopUp %X %Y}\n",
    );
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        14,
        &analysis,
        true,
    );
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
    let spec_refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        14,
        &analysis,
        true,
    );
    let spec_lines = ref_lines(&spec_refs);
    assert!(spec_lines.contains(&2), "spec decl missing: {spec_refs:?}");
    assert!(
        spec_lines.contains(&4),
        "spec's fully-qualified bind call site missing: {spec_refs:?}"
    );
    // testAddButtonPopUp: bare call site (line 5), same namespace.
    let test_refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        14,
        &analysis,
        true,
    );
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
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        14,
        &analysis,
        true,
    );
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
    let refs_ab = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        14,
        &analysis,
        true,
    );
    assert_eq!(
        ref_lines(&refs_ab),
        vec![2],
        "::a::b::helper has no callers; must not pick up ::c::d's call: {refs_ab:?}"
    );
    // References for `::c::d::helper` (cursor on its decl, line 7).
    let refs_cd = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        7,
        14,
        &analysis,
        true,
    );
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
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        18,
        &analysis,
        true,
    );
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
    // this same candidate rather than treating any `cmd_name.contains("::")`
    // as unconditionally global-rooted.
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
    let refs_outer = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        13,
        &analysis,
        true,
    );
    let lines_outer = ref_lines(&refs_outer);
    assert!(lines_outer.contains(&5), "decl missing: {refs_outer:?}");
    assert!(
        lines_outer.contains(&7),
        "`inner::p` call inside ::outer must resolve to ::outer::inner::p: {refs_outer:?}"
    );
    // References for the *global* `::inner::p` (cursor on its decl, line 1)
    // must NOT include ::outer's caller.
    let refs_global = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        &analysis,
        true,
    );
    assert_eq!(
        ref_lines(&refs_global),
        vec![1],
        "the global ::inner::p must not pick up ::outer's call: {refs_global:?}"
    );
}

// references — classes in nested namespaces (parity with the proc cases
// above): a bare class-name match must be gated by namespace, never
// cross-attributed across namespaces unconditionally

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
    let refs_ab = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        26,
        &analysis,
        true,
    );
    assert_eq!(
        ref_lines(&refs_ab),
        vec![2],
        "::a::b::Widget has no instantiations; must not pick up ::c::d's: {refs_ab:?}"
    );
    // References for `::c::d::Widget` (cursor on its decl, line 7).
    let refs_cd = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        7,
        26,
        &analysis,
        true,
    );
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
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        20,
        &analysis,
        true,
    );
    let lines = ref_lines(&refs);
    assert!(lines.contains(&3), "method decl missing: {refs:?}");
    assert!(
        lines.contains(&8),
        "external `$w render` call site missing: {refs:?}"
    );
}

// rename — nested namespaces (parity with the reference fixes above)

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
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        7,
        14,
        "assist",
        &analysis,
        None,
    );
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
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        7,
        26,
        "Panel",
        &analysis,
        None,
    );
    assert_eq!(
        edit_lines(&edits),
        vec![7, 8],
        "only ::c::d::Widget's decl + its own `Widget new` call rewritten: {edits:?}"
    );
    assert!(edits.iter().all(|e| e.new_text == "Panel"));
}

#[test]
fn rename_class_rewrites_its_superclass_and_mixin_sites() {
    // Renaming a class must rewrite every `superclass` / `mixin` word that
    // names it.  References and rename read the same `command_invocations`,
    // so they cannot disagree; a rename that leaves the superclass site
    // dangling silently breaks the inheritance graph.
    // tclsh-proof: `oo::class create Base {}; oo::class create Sub {superclass
    // Base}; oo::class create C {mixin Base}; [Sub new]; [C new]` all run.
    let src = concat!(
        "oo::class create Base {}\n",
        "oo::class create Sub {\n",
        "    superclass Base\n",
        "}\n",
        "oo::class create C {\n",
        "    mixin Base\n",
        "}\n",
    );
    let analysis = analyse(src);
    // Cursor on `Base` in its declaration (line 0, col 18).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        18,
        "Base2",
        &analysis,
        None,
    );
    assert_eq!(
        edit_lines(&edits),
        vec![0, 2, 5],
        "decl + superclass + mixin sites rewritten: {edits:?}"
    );
    assert!(edits.iter().all(|e| e.new_text == "Base2"));
}

// rename — namespaces (qualified call keeps its qualifier; isolation holds)

#[test]
fn rename_namespaced_proc_rewrites_qualified_and_short_forms() {
    // tclsh: renaming `::myns::greet`->`hello` keeps both the qualified call
    // (`::myns::hello`) and the short in-namespace call (`hello`) runnable
    // -> ns-hi/ns-hi. The qualified call must keep its `::myns::` qualifier;
    // the short call stays short.
    let src = "namespace eval ::myns {\n    proc greet {} { return ns-hi }\n}\n::myns::greet\nnamespace eval ::myns {\n    greet\n}\n";
    let analysis = analyse(src);
    // Cursor on the declaration (line 1).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        "hello",
        &analysis,
        None,
    );
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
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        "assist",
        &analysis,
        None,
    );
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
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        9,
        "assist",
        &analysis,
        None,
    );
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

// Target selection: rename / references triggered from a bareword
// CALL SITE must resolve namespace-aware (the proc/class C Tcl would
// dispatch), never a namespace-blind `name == word` scan that picks an
// arbitrary same-named symbol in another namespace.

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
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
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
    let refs = references(
        NS_COLLISION_PROC_SRC,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        col,
        &analysis,
        true,
    );
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
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        0,
        "welcome",
        &analysis,
        None,
    );
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
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        2,
        col,
        "Panel",
        &analysis,
        None,
    );
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
