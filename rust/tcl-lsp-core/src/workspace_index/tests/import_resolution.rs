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

//! Wildcard and exact `namespace import` resolution across documents, the export gate at each import site and conditional import advice.

use super::*;

// Cross-document wildcard-import resolution:
// `resolve_wildcard_import` is the mechanism that DOES resolve a bare
// call through the glob import `glob_import_introduces_no_command_link`
// (above) proves records no fixed link for.

#[test]
fn resolve_wildcard_import_resolves_exported_proc_cross_document() {
    // TP — `::mymod` (a.k.a. `mymod.tcl`) exports `helper`; `app.tcl`
    // wildcard-imports it and calls it bare from inside `run`, whose
    // own namespace is `::app` (a proc runs in the namespace it was
    // defined in, regardless of call site).
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

#[test]
fn colon_run_glob_import_drives_cross_file_navigation_references_and_w123() {
    // Mutation guard: `rsplit_once("::")` sees the final two
    // colons in `::src:::im*`, while the Tcl owner sees source `::src`
    // and tail glob `im*`.  The same fixture proves the indexed target,
    // linked cross-file reference, and analyser W123 suppression.
    let src =
        analyse("namespace eval ::src {\n    proc image {} {}\n    namespace export image\n}\n");
    let app_src =
        "namespace eval ::app {\n    namespace import ::src:::im*\n    proc run {} { image }\n}\n";
    let app = analyse(app_src);
    assert!(
        app.diagnostics
            .iter()
            .all(|d| d.code != tcl_core_types::DiagCode::W123),
        "imported image must suppress W123: {:?}",
        app.diagnostics
    );
    let index =
        WorkspaceIndex::from_documents([("file:///src.tcl", &src), ("file:///app.tcl", &app)]);
    let call = index.linked_invocations_of("::src::image", "file:///src.tcl");
    assert!(
        call.iter()
            .any(|inv| inv.uri == "file:///app.tcl" && inv.name == "image"),
        "glob import must link the cross-file call: {call:?}"
    );
    let direct = index.invocations_of("::src::image", "file:///src.tcl");
    assert!(
        direct.is_empty(),
        "direct view must retain the local imported name"
    );
    assert_eq!(
        index.resolve_wildcard_import(
            "helper",
            &["::app::helper".to_owned(), "::helper".to_owned()],
            call_from("file:///caller.tcl"),
        ),
        None
    );
}

#[test]
fn non_suffix_glob_import_resolves_cross_file_reference() {
    let src =
        analyse("namespace eval ::src {\n    proc helper {} {}\n    namespace export helper\n}\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::src::he*r\n    proc run {} { helper }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///src.tcl", &src), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_owned(), "::helper".to_owned()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::src::helper"));
}

#[test]
fn resolve_wildcard_import_does_not_resolve_unexported_sibling_cross_document() {
    // FP guard (CRITICAL) — `::mymod` also declares `other`, but never
    // exports it; real Tcl's `namespace import ::mymod::*` never binds
    // it, so the resolver must abstain (matching the in-document
    // `wildcard_namespace_import_unexported_sibling_stays_unresolved`
    // guard in `definition.rs`).
    let mymod = analyse(
        "namespace eval ::mymod {\n    proc helper {} {}\n    proc other {} {}\n    namespace export helper\n}\n",
    );
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { other }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "other",
        &["::app::other".to_string(), "::other".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert!(resolved.is_none(), "{resolved:?}");
}

#[test]
fn resolve_wildcard_import_resolves_exported_class_cross_document() {
    // TP — the georgtree_tclopt corpus shape: `tclopt.tcl` exports a
    // TclOO class and `examples/*.tcl` wildcard-imports and instantiates
    // it bare.  The shared cross-document mechanism
    // (`workspace_command_exists` / `defined_command_names`) covers
    // classes exactly like procs; pinned here as class-side coverage.
    let mypkg = analyse(
        "namespace eval ::mypkg {\n    namespace export Widget\n    oo::class create Widget {\n        method run {} { return 42 }\n    }\n}\n",
    );
    let consumer = analyse("namespace import ::mypkg::*\nset w [Widget new]\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///mypkg.tcl", &mypkg),
        ("file:///consumer.tcl", &consumer),
    ]);
    let resolved = index.resolve_wildcard_import(
        "Widget",
        &["::Widget".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::mypkg::Widget"));
}

#[test]
fn resolve_wildcard_import_is_not_restricted_to_the_calling_document() {
    // TP — `namespace import` binds to the
    // *namespace*, not the file that wrote it: real Tcl reopening
    // `::app` in a later `namespace eval ::app { ... }` block sees
    // every import already recorded for `::app`, regardless of which
    // file recorded it. A common real-world shape is a shared
    // "imports.tcl" that does the import once, with sibling files
    // reopening the same namespace to call the bare name. Three
    // separate files: `mymod.tcl` exports `helper`; `imports.tcl` does
    // `namespace eval ::app { namespace import ::mymod::* }`;
    // `caller.tcl` does `namespace eval ::app { proc run {} { helper } }`
    // — the import statement and the call site are never in the same
    // document, so filtering candidate glob imports by `g.uri == uri`
    // (the calling document) would never admit this shape.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let imports = analyse("namespace eval ::app {\n    namespace import ::mymod::*\n}\n");
    let caller = analyse("namespace eval ::app {\n    proc run {} { helper }\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///imports.tcl", &imports),
        ("file:///caller.tcl", &caller),
    ]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

// Per-import-site export snapshots, cross-document tier.
// The workspace resolver applies the same shared decision function the
// same-document one does (`namespace_import::exported_at_import_site`),
// so the two tiers cannot disagree — but only events in the *import's own
// document* are ordered against it; another file's load order is not a
// static fact.

#[test]
fn resolve_wildcard_import_survives_a_later_export_clear_in_the_import_file() {
    // TP, direction A, cross-document — the import and the later
    // `namespace export -clear` are both in `app.tcl`, so they *are*
    // ordered: the `-clear` runs after the import and cannot revoke what
    // it bound (oracle tclsh 8.6.14/9.0.4). `::mymod`'s own export is in
    // the other file and unordered, which the shared function keeps.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export -clear\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

#[test]
fn resolve_wildcard_import_ignores_a_same_file_export_written_after_the_import() {
    // FP guard (CRITICAL), direction B, cross-document — the import and
    // the export are both in `app.tcl` and the export comes *after*, so
    // real Tcl never binds `::app::helper` (oracle: `invalid command
    // name`). `mymod.tcl` exports nothing, so there is no unordered
    // event to fall back on and the resolver must abstain.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let app = analyse(
        "rename auto_import savedPrelude; namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export helper\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert!(
        resolved.is_none(),
        "an export written after the import in the same file must not \
             apply retroactively: {resolved:?}"
    );
}

#[test]
fn resolve_wildcard_import_keeps_answering_when_the_export_is_in_another_file() {
    // TN-for-abstention — which of two files loads first is not a static
    // fact, so a `namespace export -clear` in a *third* document cannot
    // be ordered against this import and must not silently revoke it.
    // Navigation keeps answering; the residual is documented in
    // `namespace_import`'s module docs.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\n",
    );
    let teardown = analyse("namespace eval ::mymod { namespace export -clear }\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///app.tcl", &app),
        ("file:///teardown.tcl", &teardown),
    ]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

#[test]
fn wildcard_import_inside_a_body_sees_a_later_top_level_export() {
    // TP — a plain `at <= import_at` predicate is *weaker* than the
    // same-document tier's `indirection::in_effect` and rejects this real
    // alias. The import
    // sits in `::app::setup`'s body; the `namespace export` is written
    // further down the *same file* but at load level, so loading `app.tcl`
    // runs the export before `setup` can ever be called.
    //
    // Oracle (tclsh 8.6.14 / 9.0.4): with `::mymod::helper` defined
    // elsewhere, `namespace eval ::app {proc setup {} {namespace import
    // ::mymod::*}; proc run {} {helper}}` followed by `namespace eval
    // ::mymod {namespace export helper}`, then `::app::setup; ::app::run`
    // → `HELP`.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc setup {} { namespace import ::mymod::* }\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export helper\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(
        resolved.as_deref(),
        Some("::mymod::helper"),
        "an import inside a body observes every top-level statement of \
             its own file, wherever written",
    );
}

#[test]
fn wildcard_import_inside_a_body_still_refuses_an_export_in_that_same_body() {
    // FN guard for the leniency above: the
    // "whole file loads first" exception does **not** extend to a
    // statement of the *same* body — there the offsets are in genuine
    // execution order. Oracle: `proc setup {} {namespace import ::m::*;
    // namespace eval ::m {namespace export helper}}` then `::a::setup`
    // leaves `::a::helper` an `invalid command name`.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc setup {} { namespace import ::mymod::* ; namespace eval ::mymod { namespace export helper } }\n    proc run {} { helper }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert!(
        resolved.is_none(),
        "an export written after the import in the same body is a later \
             statement of the running script: {resolved:?}",
    );
}

// Global-rooted imports go through the gate too.
//
// Oracle, byte-identical on tclsh 8.6.14 and 9.0.4:
//
//   proc p {} { return GLOBAL }
//   namespace eval ::dst  { namespace import ::p }   ;# no error…
//   info commands ::dst::*                           ;# …and nothing bound
//   namespace export p
//   namespace eval ::dst2 { namespace import ::p }
//   info commands ::dst2::*                          ;# ::dst2::p

#[test]
fn a_global_rooted_exact_import_is_export_gated() {
    // TN — `::p` splits to an *empty* source namespace, which both tiers
    // read as "no source" and skipped, leaving the one import shape that
    // bypassed the gate entirely.
    let lib = analyse("proc p {} { return GLOBAL }\n");
    let dst = analyse("namespace eval ::dst { namespace import ::p }\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///dst.tcl", &dst)]);
    assert_eq!(
        index.resolve_command_target("::dst::p"),
        "::dst::p",
        "an unexported global command installs nothing",
    );
}

#[test]
fn a_global_rooted_exact_import_of_an_exported_name_still_binds() {
    // TP — the export at global level is recorded with `ns` = `::`, the
    // same spelling the gate now asks with, so the working case works.
    let lib = analyse("proc p {} { return GLOBAL }\nnamespace export p\n");
    let dst = analyse("namespace eval ::dst { namespace import ::p }\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///dst.tcl", &dst)]);
    assert_eq!(index.resolve_command_target("::dst::p"), "::p");
}

#[test]
fn a_global_rooted_glob_import_is_export_gated() {
    // TN — the glob spelling of the same shape, whose empty source
    // namespace makes it easy to drop rather than record.
    let lib = analyse("proc p {} { return GLOBAL }\n");
    let dst = analyse("namespace eval ::dst { namespace import ::* }\np\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///dst.tcl", &dst)]);
    let candidates = ["::dst::p".to_string(), "::p".to_string()];
    assert!(
        index
            .resolve_wildcard_import("p", &candidates, call_from("file:///dst.tcl"))
            .is_none(),
        "an unexported global command is not reachable through `::*`",
    );
    // TP — with the export, it is.
    let lib = analyse("proc p {} { return GLOBAL }\nnamespace export p\n");
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///dst.tcl", &dst)]);
    assert_eq!(
        index
            .resolve_wildcard_import("p", &candidates, call_from("file:///dst.tcl"))
            .as_deref(),
        Some("::p"),
    );
}

// Exact (non-glob) `namespace import` is export-gated too.
// Real Tcl silently installs nothing when the name is
// not exported at the import's own position (oracle: `namespace eval
// ::m {proc helper {} {}}; namespace eval ::a {namespace import
// ::m::helper}` leaves `info commands ::a::*` empty and raises no error).

#[test]
fn exact_import_of_an_unexported_name_installs_no_link() {
    // FP guard (CRITICAL) — `::mymod` never exports `helper`, so the
    // exact import binds nothing: no `::app::helper` exists, the bare
    // call is not a reference to the source, and the pattern token is not
    // a link span.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert!(
        !index.workspace_command_exists("::app::helper"),
        "an unexported exact import installs no command",
    );
    assert!(
        index
            .linked_invocations_of("::mymod::helper", "file:///mymod.tcl")
            .is_empty(),
        "the bare call reaches nothing, so it is no reference",
    );
    assert!(
        index
            .link_target_spans("::mymod::helper", "file:///mymod.tcl")
            .is_empty(),
        "a link that was never installed has no target span",
    );
}

#[test]
fn exact_import_ignores_an_export_written_after_it() {
    // FP guard, direction B for the exact form — the export lands after
    // the import in the same file, so real Tcl still binds nothing
    // (oracle: `info commands ::a5::*` empty).
    let mymod = analyse("namespace eval ::mymod { proc helper {} {} }\n");
    let app = analyse(
        "rename auto_import savedPrelude; namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export helper\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert!(!index.workspace_command_exists("::app::helper"));
}

#[test]
fn exact_import_survives_a_later_export_clear() {
    // TP, direction A for the exact form — the `-clear` runs after the
    // import, so the alias it already installed stays.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\nnamespace eval ::mymod {\n    namespace export -clear\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert!(index.workspace_command_exists("::app::helper"));
    let refs = index.linked_invocations_of("::mymod::helper", "file:///mymod.tcl");
    assert_eq!(refs.len(), 1, "{refs:?}");
}

#[test]
fn exact_import_from_an_unseen_namespace_keeps_its_link() {
    // Abstention guard (CRITICAL for W123 silence) — `::msgcat` lives in
    // an installed package, not in any indexed document, so the index
    // holds no export declaration for it and never will. Treating that
    // silence as "not exported" would revoke `::app::mc` and hand the
    // unknown-command pass a false positive on every bare `mc` call. The
    // gate only fires where the workspace can see the namespace's own
    // definitions or exports.
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::msgcat::mc\n    proc run {} { mc hello }\n}\n",
    );
    let index = WorkspaceIndex::from_documents([("file:///app.tcl", &app)]);
    assert!(
        index.workspace_command_exists("::app::mc"),
        "an import from a namespace the workspace cannot observe must not \
             be gated away",
    );
}

#[test]
fn alias_and_rename_links_are_never_export_gated() {
    // TN — only a `namespace import` link carries an export snapshot.
    // `interp alias` and `rename` introduce a name with no reference to
    // any export list, and must keep working unchanged.
    let lib = analyse("proc ::mymod::helper {} {}\n");
    let app = analyse(
        "interp alias {} ::app::a {} ::mymod::helper\nrename ::mymod::helper ::mymod::gone\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///lib.tcl", &lib), ("file:///app.tcl", &app)]);
    assert!(
        index.workspace_command_exists("::app::a"),
        "an interp alias is not gated by any namespace export",
    );
    assert!(
        index.workspace_command_exists("::mymod::gone"),
        "a rename is not gated by any namespace export",
    );
}

#[test]
fn conditional_namespace_horizon_keeps_binding_and_report_purposes_separate() {
    let (module, export, import) = import_order_modules();
    let app = analyse("source mod.tcl\nsource imp.tcl\nsource exp.tcl\n");
    let index = sourced_index([
        ("file:///p/mod.tcl", &module),
        ("file:///p/exp.tcl", &export),
        ("file:///p/imp.tcl", &import),
        ("file:///p/app.tcl", &app),
    ]);
    let candidates = ["::app::helper".to_owned(), "::helper".to_owned()];
    let advice = index
        .conditional_namespace_import_advice(
            "helper",
            &candidates,
            call_from("file:///p/caller.tcl"),
        )
        .unwrap();
    assert!(
        advice.compatibility_report().is_none(),
        "the source route cannot use a later export"
    );
    assert!(advice.has_unresolved_operation_debt());
    assert!(
        advice
            .debts()
            .contains(&ConditionalNamespaceDebt::UnknownAlternatives)
    );
    assert!(advice.matches_index(&index));
    assert_eq!(
        index
            .resolve_wildcard_import_for_binding(
                "helper",
                &candidates,
                call_from("file:///p/caller.tcl")
            )
            .as_deref(),
        Some("::mymod::helper"),
        "readonly horizon advice cannot suppress a binding candidate"
    );
}

#[test]
fn conditional_namespace_forget_preserves_unknowns_and_current_index_owner() {
    let (module, export, import) = import_order_modules();
    let app = analyse(
        "source mod.tcl\nsource exp.tcl\nsource imp.tcl\nnamespace eval ::app {namespace forget ::mymod::helper}\n",
    );
    let mut index = sourced_index([
        ("file:///p/mod.tcl", &module),
        ("file:///p/exp.tcl", &export),
        ("file:///p/imp.tcl", &import),
        ("file:///p/app.tcl", &app),
    ]);
    let candidates = ["::app::helper".to_owned(), "::helper".to_owned()];
    let advice = index
        .conditional_namespace_import_advice(
            "helper",
            &candidates,
            call_at("file:///p/app.tcl", u32::MAX),
        )
        .unwrap();
    assert!(advice.compatibility_report().is_none());
    assert!(
        advice.has_unresolved_operation_debt(),
        "empty report is never namespace absence"
    );
    assert_eq!(
        index
            .resolve_wildcard_import_for_binding(
                "helper",
                &candidates,
                call_at("file:///p/app.tcl", u32::MAX)
            )
            .as_deref(),
        Some("::mymod::helper")
    );
    let dynamic = analyse(
        "source mod.tcl\nsource exp.tcl\nsource imp.tcl\nnamespace eval ::app {namespace forget $unknown}\n",
    );
    index.replace_document("file:///p/app.tcl", &dynamic);
    assert!(!advice.matches_index(&index));
    let unknown = index
        .conditional_namespace_import_advice(
            "helper",
            &candidates,
            call_at("file:///p/app.tcl", u32::MAX),
        )
        .unwrap();
    assert_eq!(unknown.compatibility_report(), Some("::mymod::helper"));
    assert!(
        unknown
            .debts()
            .contains(&ConditionalNamespaceDebt::UnrepresentedLifecycle)
    );
}
