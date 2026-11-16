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

//! Wildcard and exact `namespace import` resolution and the export gate at each import site.

use super::*;

// wildcard namespace import bareword resolution:
// `namespace import NS::*` makes every command `NS` has `namespace
// export`ed callable bare wherever the import is in scope — but real
// Tcl only imports *exported* names (`Tcl_Export`), so an unexported
// sibling in `NS` stays unreachable through the import.

#[test]
fn wildcard_namespace_import_resolves_exported_proc() {
    // TP — single command in `Foo`, exported, reached only through the
    // wildcard import (tclsh9.0/8.6-verified: `bar` prints 1).
    let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nbar\n";
    let analysis = analyse(src);
    // Cursor on the bareword `bar` call (line 5, col 0).
    let locs = definition(src, 5, 0, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "must resolve to Foo::bar's declaration"
    );
    assert_eq!(locs[0].start_character, 9);
}

#[test]
fn wildcard_namespace_import_resolves_exported_proc_alongside_unexported_sibling() {
    // TP — the finding's own headline shape: `Foo` has both an exported
    // `bar` and an unrelated, unexported `other`. Sibling count must not
    // enter into it: there is no "ambiguity threshold" in real Tcl.
    let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    proc other {} { return 2 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nbar\n";
    let analysis = analyse(src);
    // Cursor on the bareword `bar` call (line 6, col 0).
    let locs = definition(src, 6, 0, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "must resolve to Foo::bar's declaration"
    );
    assert_eq!(locs[0].start_character, 9);
}

#[test]
fn wildcard_namespace_import_unexported_sibling_stays_unresolved() {
    // FP guard (CRITICAL) — `other` lives in `Foo` but is never
    // exported, so real Tcl's `namespace import ::Foo::*` never binds
    // it: a bare `other` call errors "invalid command name" at runtime
    // (Tcl manual, `namespace` — "namespace import" only imports
    // exported names). The wildcard-import resolver must not
    // manufacture a resolution the interpreter itself would reject.
    //
    // Exercises `proc_visible_via_wildcard_import` directly rather than
    // the full `definition()` provider: `resolve_called_proc`'s own
    // pre-existing, unrelated lenient fallback
    // (`fallback_proc_by_simple_name`, "any proc with this simple name,
    // anywhere") would otherwise also resolve a bare `other` call
    // regardless of import/export state — a longstanding, documented
    // leniency for procs whose defining namespace isn't statically
    // visible, not the gate this test is proving.
    let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    proc other {} { return 2 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nother\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "::", "other", u32::MAX);
    assert!(
        hit.is_none(),
        "an unexported sibling must stay unresolved through the wildcard import: {hit:?}"
    );
}

#[test]
fn wildcard_namespace_import_resolves_exported_class() {
    // TP — the class analogue of
    // `wildcard_namespace_import_resolves_exported_proc`:
    // `resolve_class_target_at` is the fixed function directly (the
    // top-level `definition()` provider's own bareword-class match is
    // namespace-blind already and would not exercise this fix).
    let src = "namespace eval Foo {\n    oo::class create Widget {}\n    namespace export Widget\n}\nnamespace import ::Foo::*\nWidget new\n";
    let analysis = analyse(src);
    let line_index = LineIndex::new(src);
    let cursor_off = byte_offset_at(&line_index, src, 5, 0);
    let hit = resolve_class_target_at(&analysis, src, DOC, cursor_off, "Widget");
    assert!(
        hit.is_some(),
        "must resolve Widget through the wildcard import"
    );
    let (qname, _) = hit.unwrap();
    assert_eq!(qname, "::Foo::Widget");
}

#[test]
fn wildcard_namespace_import_unexported_sibling_class_stays_unresolved() {
    // FP guard — the class analogue: `Other` lives in `Foo` but is
    // never exported, so it must stay unresolved through the wildcard
    // import exactly like the proc case.
    let src = "namespace eval Foo {\n    oo::class create Widget {}\n    oo::class create Other {}\n    namespace export Widget\n}\nnamespace import ::Foo::*\nOther new\n";
    let analysis = analyse(src);
    let line_index = LineIndex::new(src);
    let cursor_off = byte_offset_at(&line_index, src, 6, 0);
    let hit = resolve_class_target_at(&analysis, src, DOC, cursor_off, "Other");
    assert!(
        hit.is_none(),
        "an unexported sibling class must stay unresolved: {hit:?}"
    );
}

#[test]
fn wildcard_namespace_import_wins_over_an_unrelated_decoy_with_the_same_simple_name() {
    // TP — a bareword call reached via a wildcard import must consult
    // `namespace_imports`: falling straight to
    // `fallback_proc_by_simple_name`'s lenient "any proc with this simple
    // name, prefer lexicographically smallest qualified name" tie-break
    // silently resolves (and hovers) to an entirely unrelated,
    // never-imported decoy
    // (`::decoyns::helper`, "d" < "r") instead of the real,
    // oracle-proven target (`::realns::helper`, tclsh9.0/8.6-verified:
    // prints `REAL`). The wildcard-import resolution is tried *before*
    // that fallback, so it must win here.
    let src = "namespace eval realns {\n    namespace export helper\n    proc helper {} { return REAL }\n}\nnamespace eval decoyns {\n    proc helper {} { return DECOY }\n}\nnamespace eval userns {\n    namespace import ::realns::*\n    proc run {} {\n        return [helper]\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on the bareword `helper` call inside `userns::run` (line
    // 10, col 16 — `        return [helper]`).
    let locs = definition(src, 10, 16, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 2,
        "must resolve to realns::helper (the exported, imported target), \
             not the decoy: {locs:?}"
    );
}

// Per-import-site export snapshots. `namespace import`
// binds the names exported *when it runs*; the export list keeps
// changing afterwards and none of it reaches back. Every case below is
// oracle-pinned against tclsh 8.6.14 and 9.0.4.
//
// These drive `proc_visible_via_wildcard_import` rather than the full
// `definition()` provider for the same reason
// `wildcard_namespace_import_unexported_sibling_stays_unresolved` does:
// `resolve_called_proc`'s unrelated lenient `fallback_proc_by_simple_name`
// would answer either way and would prove nothing about this gate.

#[test]
fn import_site_snapshot_survives_a_later_export_clear() {
    // TP, direction A — `namespace export -clear` after the
    // import does NOT revoke the alias the import already installed.
    // Oracle: `namespace eval ::src {proc p {} {return P}; namespace
    // export p}; namespace eval ::dst {namespace import ::src::*};
    // namespace eval ::src {namespace export -clear}; ::dst::p` → `P`,
    // and `info commands ::dst::*` still lists `::dst::p`.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export -clear\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a `-clear` written after the import must not revoke it: {hit:?}"
    );
}

#[test]
fn import_site_snapshot_ignores_a_later_export() {
    // FP guard (CRITICAL), direction B — exporting a name
    // *after* an import does not add it retroactively. Oracle:
    // `namespace eval ::src {proc p {} {return P}}; namespace eval ::dst
    // {namespace import ::src::*}; namespace eval ::src {namespace export
    // p}; ::dst::p` → `invalid command name "::dst::p"`.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export p\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_none(),
        "an export written after the import must not apply retroactively: {hit:?}"
    );
}

#[test]
fn a_global_rooted_import_is_export_gated_in_document_too() {
    // TN / TP pair — `namespace import ::p` reads as an *empty* source
    // namespace, which is easy to skip and so bypass the gate. Oracle
    // (8.6.14 /
    // 9.0.4): without `namespace export p` at global level the import is a
    // silent no-op and `info commands ::dst::*` stays empty; with it,
    // `::dst::p` binds.
    let ungated =
        "proc p {} { return GLOBAL }\nnamespace eval dst {\n    namespace import ::p\n}\n";
    let analysis = analyse(ungated);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_none(),
        "an unexported global command installs nothing",
    );
    let gated = "proc p {} { return GLOBAL }\nnamespace export p\nnamespace eval dst {\n    namespace import ::p\n}\n";
    let analysis = analyse(gated);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX)
            .is_some_and(|p| p.qualified_name == "::p"),
        "with the export it binds",
    );
}

// The same gate, as the **provider** answers it.  Everything above drives
// `proc_visible_via_wildcard_import` directly; these pin that
// `resolve_called_proc`'s lenient tail match does not overrule it.

#[test]
fn single_file_definition_refuses_direction_b_through_the_fallback() {
    // FP guard (CRITICAL) — the whole Direction-B fixture in one
    // document. Oracle (8.6.14 / 9.0.4): the bare call is `invalid command
    // name "p"` and `info commands ::dst::*` is empty, so go-to-definition
    // must not answer `::src::p` through `fallback_proc_by_simple_name`.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export p\n}\nnamespace eval dst {\n    p\n}\n";
    let analysis = analyse(src);
    let locs = definition(src, 10, 4, &analysis);
    assert!(
        locs.is_empty(),
        "the lenient tail match must not overrule the import gate: {locs:?}"
    );
}

#[test]
fn single_file_definition_still_answers_when_the_import_is_live() {
    // TP — the same fixture with the export written *before* the import
    // must still resolve, or the tightening has simply broken the feature.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    p\n}\n";
    let analysis = analyse(src);
    let locs = definition(src, 8, 4, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn the_tail_match_still_answers_where_no_import_names_that_namespace() {
    // Nothing imports `::other` and `::dst` has no path to it: tclsh
    // 8.4-9.1 and jimsh answer `invalid command name "helper"`, so a
    // same-file tail match must not answer.
    let src = "namespace eval other {\n    proc helper {} { return H }\n}\nnamespace eval dst {\n    helper\n}\n";
    let analysis = analyse(src);
    let locs = definition(src, 4, 4, &analysis);
    assert!(
        locs.is_empty(),
        "an unrelated same-file tail match must not answer: {locs:?}"
    );
}

#[test]
fn the_tail_match_still_answers_when_a_gated_import_names_another_namespace() {
    // A dead import of `::src` says nothing about `::other`, and nothing
    // else makes `::other::helper2` visible from `::dst`: tclsh 8.4-9.1
    // answer `invalid command name "helper2"`.
    let src = "namespace eval src {\n    proc helper {} { return S }\n}\nnamespace eval other {\n    proc helper2 {} { return O }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    helper2\n}\n";
    let analysis = analyse(src);
    let locs = definition(src, 10, 4, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn the_tail_match_refuses_a_forgotten_import_too() {
    // TN — the same tightening covers every way the route can be dead,
    // not just the export snapshot: after `namespace forget ::src::p` the
    // bare call is `invalid command name` (oracle on `alias_live_at`), so
    // the fallback must not answer either.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    namespace forget ::src::p\n}\nnamespace eval dst {\n    p\n}\n";
    let analysis = analyse(src);
    let locs = definition(src, 11, 4, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn the_tail_match_refuses_a_call_written_before_its_import() {
    // TN — the call-site ordering reaches the provider through the same
    // door: the call above the import resolves
    // nowhere, the one below it resolves. The early call is caught, as
    // tclsh 8.4-9.1 otherwise stop at `invalid command name "p"` before
    // the import runs (with the catch they print `P` for the later call).
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    catch {p}\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval dst {\n    p\n}\n";
    let analysis = analyse(src);
    assert!(
        definition(src, 5, 11, &analysis).is_empty(),
        "the call before the import must not resolve",
    );
    assert_eq!(
        definition(src, 11, 4, &analysis).len(),
        1,
        "the call after it must",
    );
}

#[test]
fn a_second_import_after_a_later_export_does_pick_the_name_up() {
    // TP — each import takes its own snapshot, so the *second* import,
    // written after the export, binds `q` even though the first could
    // not. Oracle: `namespace eval ::src {proc p …; proc q …; namespace
    // export p}; namespace eval ::dst {namespace import ::src::*};
    // namespace eval ::src {namespace export -clear p q}; namespace eval
    // ::dst {namespace import ::src::*}` → both `::dst::p` and
    // `::dst::q` run.
    let src = "namespace eval src {\n    proc p {} { return P }\n    proc q {} { return Q }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\nnamespace eval src {\n    namespace export -clear p q\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    for name in ["p", "q"] {
        let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", name, u32::MAX);
        assert!(
            hit.is_some(),
            "the second import must see `{name}`: {hit:?}"
        );
    }
}

#[test]
fn export_clear_before_the_import_does_revoke() {
    // TN — the ordering gate is real in both directions: a `-clear`
    // written *before* the import leaves nothing for it to bind.
    // Oracle: after `namespace export -clear`, `namespace export`
    // returns the empty list, and the later import binds no command.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc p {} { return P }\n    namespace export p\n    namespace export -clear\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_none(),
        "a `-clear` before the import leaves nothing exported: {hit:?}"
    );
}

#[test]
fn export_clear_with_patterns_on_the_same_call_keeps_only_the_new_ones() {
    // TP + TN — `namespace export a b; namespace export -clear p` leaves
    // exactly `p` exported (oracle: `namespace export` → `p`), so an
    // import after it binds `p` and not `a`.
    let src = "rename auto_import savedPrelude; namespace eval src {\n    proc a {} { return A }\n    proc p {} { return P }\n    namespace export a\n    namespace export -clear p\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_some(),
        "the `-clear` call's own pattern survives it"
    );
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "a", u32::MAX).is_none(),
        "the pattern cleared by the same call must not survive"
    );
}

#[test]
fn separate_export_calls_are_additive() {
    // TP — `namespace export a` then `namespace export b` exports both
    // (oracle: `namespace export` → `a b`); `export` appends, it does
    // not replace. A snapshot model that kept only the newest call's
    // patterns would lose `a`.
    let src = "namespace eval src {\n    proc a {} { return A }\n    proc b {} { return B }\n    namespace export a\n    namespace export b\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    for name in ["a", "b"] {
        assert!(
            proc_visible_via_wildcard_import(&analysis, DOC, "dst", name, u32::MAX).is_some(),
            "`{name}` must stay exported — `namespace export` is additive"
        );
    }
}

#[test]
fn export_glob_patterns_use_tcl_glob_semantics() {
    // TP + TN — export patterns are glob *patterns*: oracle `namespace
    // export get*` with `getX`/`setX` in the namespace imports only
    // `getX` (`::dst::setX` → `invalid command name`).
    let src = "namespace eval src {\n    proc getX {} { return GX }\n    proc setX {} { return SX }\n    namespace export get*\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    assert!(proc_visible_via_wildcard_import(&analysis, DOC, "dst", "getX", u32::MAX).is_some());
    assert!(proc_visible_via_wildcard_import(&analysis, DOC, "dst", "setX", u32::MAX).is_none());
}

#[test]
fn an_export_written_before_the_proc_still_exports_it() {
    // TP — an export pattern is a *name pattern*, not a reference to a
    // command: oracle `namespace eval ::src {namespace export p; proc p
    // {} {return P}}` then importing `::src::*` yields a working
    // `::dst::p`. Nothing in the snapshot may require the proc to have
    // been defined by the time the export ran.
    let src = "namespace eval src {\n    namespace export p\n    proc p {} { return P }\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    assert!(
        proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX).is_some(),
        "`namespace export` names a pattern, not an existing command"
    );
}

#[test]
fn an_import_inside_a_body_sees_a_later_top_level_export() {
    // TP — the same-document half of the ordering rule the workspace tier
    // must match (pinned on both sides so they cannot drift): the import
    // runs only when `setup` is called, by which
    // time the whole file — including the `namespace export` written below
    // it — has loaded. Oracle (tclsh 8.6.14 / 9.0.4): `::app::setup;
    // ::app::run` → `HELP`.
    //
    // `indirection::in_effect` states exactly that; a plain offset compare
    // does not.
    let src = "namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    proc setup {} { namespace import ::src::* }\n}\nnamespace eval src {\n    namespace export p\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::p"),
        "a body-local import observes a top-level export written later in \
             the same file: {hit:?}"
    );
}

#[test]
fn an_export_in_the_importing_body_itself_stays_ordered() {
    // FN guard for the leniency above: the "whole file loads first"
    // exception does not extend to a statement of the *same* body, where
    // the offsets are in genuine execution order. Oracle: `proc setup {}
    // {namespace import ::src::*; namespace eval ::src {namespace export
    // p}}` then `::dst::setup` leaves `::dst::p` an invalid command name.
    let src = "namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    proc setup {} { namespace import ::src::* ; namespace eval ::src { namespace export p } }\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "p", u32::MAX);
    assert!(
        hit.is_none(),
        "an export written after the import in the same body has not run \
             yet: {hit:?}"
    );
}

#[test]
fn a_command_named_like_the_export_flag_is_exportable() {
    // TP — the user-visible consequence of consuming only one `-clear`:
    // `namespace export -clear -clear`
    // exports a command genuinely *named* `-clear`, and a wildcard import
    // then binds it. Oracle (tclsh 8.6.14 / 9.0.4): `namespace export`
    // returns `-clear`, and `info commands ::dst::*` lists `::dst::-clear`.
    // Consuming both words as flags would drop the export entirely.
    let src = "namespace eval src {\n    proc -clear {} { return DC }\n    namespace export -clear -clear\n}\nnamespace eval dst {\n    namespace import ::src::*\n}\n";
    let analysis = analyse(src);
    let hit = proc_visible_via_wildcard_import(&analysis, DOC, "dst", "-clear", u32::MAX);
    assert!(
        hit.is_some_and(|p| p.qualified_name == "::src::-clear"),
        "the second `-clear` is an export pattern, not a second flag: {hit:?}"
    );
}

#[test]
fn exact_namespace_import_resolves_source_proc_in_document() {
    // FP guard / bonus TP — a plain (non-glob) `namespace import
    // ::Foo::bar` must keep working through the same in-document path
    // (the wildcard resolver's glob match degrades to exact equality
    // for a literal pattern), matching the already-passing
    // cross-document `namespace_import_call_site_references_the_source_command`
    // (`tcl-lsp-core::workspace_index`) — this exercises the same-file
    // case that resolver never covers.
    let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    namespace export bar\n}\nnamespace import ::Foo::bar\nbar\n";
    let analysis = analyse(src);
    let locs = definition(src, 5, 0, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "must resolve to Foo::bar's declaration"
    );
}
