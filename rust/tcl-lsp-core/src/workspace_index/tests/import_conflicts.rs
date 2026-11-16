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

//! Import conflicts between glob and exact imports, exact-link liveness and import chains.

use super::*;

// Glob and exact import conflicts are symmetric.
//
// Oracle, byte-identical on tclsh 8.6.14 and 9.0.4, both orders:
//
//   namespace eval ::A { proc p {} {return A}; namespace export p }
//   namespace eval ::B { proc p {} {return B}; namespace export p }
//   namespace eval ::dst  { namespace import ::A::* ; namespace import ::B::p }
//   namespace eval ::dst2 { namespace import ::A::p ; namespace import ::B::* }
//   → both second imports: can't import command "p": already exists
//   → namespace origin ::dst::p  = ::A::p ; ::dst::p  → A
//   → namespace origin ::dst2::p = ::A::p ; ::dst2::p → A

/// The `::A` / `::B` sources both exporting `p`, used by the conflict
/// tests below.
fn two_exporting_sources() -> (AnalysisResult, AnalysisResult) {
    (
        analyse("namespace eval ::A { proc p {} { return A }\n namespace export p }\n"),
        analyse("namespace eval ::B { proc p {} { return B }\n namespace export p }\n"),
    )
}

#[test]
fn a_glob_import_conflicts_with_a_later_exact_import_of_the_same_name() {
    // TN (the gap) — the exact link's conflict check only ever compared
    // other *exact* links, so the earlier glob import was invisible to it
    // and `::dst::p` resolved to `::B::p`, the one binding Tcl refuses.
    let (a, b) = two_exporting_sources();
    let dst = analyse(
        "namespace eval ::dst {\n    namespace import ::A::*\n    namespace import ::B::p\n}\np\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    assert_ne!(
        index.resolve_command_target("::dst::p"),
        "::B::p",
        "the exact import installs nothing over the live glob alias",
    );
    // …and the name the call really reaches is the glob import's source.
    assert_eq!(
        index
            .resolve_wildcard_import(
                "p",
                &["::dst::p".to_string(), "::p".to_string()],
                call_from("file:///dst.tcl"),
            )
            .as_deref(),
        Some("::A::p"),
    );
}

#[test]
fn an_exact_import_conflicts_with_a_later_glob_import_of_the_same_name() {
    // TN, the other direction of the same rule — the glob side asked only
    // about other glob imports.
    let (a, b) = two_exporting_sources();
    let dst = analyse(
        "namespace eval ::dst {\n    namespace import ::A::p\n    namespace import ::B::*\n}\np\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    let resolved = index.resolve_wildcard_import(
        "p",
        &["::dst::p".to_string(), "::p".to_string()],
        call_from("file:///dst.tcl"),
    );
    assert!(
        resolved.is_none_or(|r| r == "::A::p"),
        "the later glob import must not install over a live exact alias",
    );
    assert_eq!(index.resolve_command_target("::dst::p"), "::A::p");
}

#[test]
fn a_forced_exact_import_still_replaces_a_live_glob_alias() {
    // TP / FN guard — `-force` is exactly the case that *does* install
    // over whatever was there (oracle on `WorkspaceGlobImport::forced`),
    // so widening the conflict check must not swallow it.
    let (a, b) = two_exporting_sources();
    let dst = analyse(
        "namespace eval ::dst {\n    namespace import ::A::*\n    namespace import -force ::B::p\n}\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    assert_eq!(index.resolve_command_target("::dst::p"), "::B::p");
}

#[test]
fn a_body_local_import_conflicts_with_a_later_top_level_import() {
    // TN (CRITICAL) — the conflict check must order the two imports the
    // way they *run*, not the way they are written. Oracle, byte-identical
    // on tclsh 8.6.14 and 9.0.4:
    //
    //   namespace eval ::A { proc x {} {return A}; namespace export x }
    //   namespace eval ::B { proc x {} {return B}; namespace export x }
    //   namespace eval ::dst { proc p {} { namespace import ::B::x } }
    //   namespace eval ::dst { namespace import ::A::* }
    //   namespace origin ::dst::x  -> ::A::x   (after load)
    //   ::dst::p                   -> can't import command "x": already exists
    //   namespace origin ::dst::x  -> ::A::x   (unchanged)
    //
    // The whole file loads before any body runs, so the top-level `::A`
    // glob import owns the name by the time `p`'s body-local `::B` import
    // executes — and that one installs nothing. A plain offset compare
    // sees `::A` written *later* and lets `::B` install.
    let (a, b) = two_exporting_sources();
    let dst = analyse(
        "namespace eval ::dst {\n    proc runner {} { namespace import ::B::p }\n}\nnamespace eval ::dst {\n    namespace import ::A::*\n}\np\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    assert_ne!(
        index.resolve_command_target("::dst::p"),
        "::B::p",
        "the body-local import runs after the top-level one and installs nothing",
    );
    assert_eq!(
        index
            .resolve_wildcard_import(
                "p",
                &["::dst::p".to_string(), "::p".to_string()],
                call_from("file:///dst.tcl"),
            )
            .as_deref(),
        Some("::A::p"),
        "the top-level glob import is the edge the name really has",
    );
}

/// The later import of `p` from `::B` against an earlier `::A` alias whose
/// forget sits where its position is unknown, as `src` is laid out.
fn later_import_after_an_unplaced_forget(src: &str, forget: Option<&str>) -> Option<String> {
    let (a, b) = two_exporting_sources();
    let dst = analyse(src);
    let forget = analyse(forget.unwrap_or(""));
    let index = sourced_index([
        ("file:///p/a.tcl", &a),
        ("file:///p/b.tcl", &b),
        ("file:///p/forgetA.tcl", &forget),
        ("file:///p/dst.tcl", &dst),
    ]);
    let call = u32::try_from(app_src_offset(src, "\np\n") + 1).expect("tiny source");
    index.resolve_wildcard_import(
        "p",
        &["::dst::p".to_string(), "::p".to_string()],
        call_at("file:///p/dst.tcl", call),
    )
}

#[test]
fn a_sourced_forget_between_two_imports_does_not_make_the_later_one_conflict() {
    // tclsh 8.6 / 9.0: `namespace import ::A::*`, then `source forgetA.tcl`
    // (which forgets `::A::p`), then `namespace import ::B::*` leaves
    // `namespace origin ::dst::p` at `::B::p`. The forget is in another
    // document, so the earlier alias is not certainly live at the later
    // import and no conflict cancels the install Tcl performs.
    let src = "namespace eval ::dst { namespace import ::A::* }\nsource forgetA.tcl\nnamespace eval ::dst { namespace import ::B::* }\np\n";
    assert_eq!(
        later_import_after_an_unplaced_forget(
            src,
            Some("namespace eval ::dst { namespace forget ::A::p }\n")
        )
        .as_deref(),
        Some("::B::p"),
    );
}

#[test]
fn a_branch_forget_between_two_imports_does_not_make_the_later_one_conflict() {
    // tclsh 8.6 / 9.0 with the branch taken: `::B::p`. A forget inside an
    // `if` body is not straight-line sequence, so it keeps the earlier
    // alias possibly dead and the later install stands.
    let src = "namespace eval ::dst { namespace import ::A::* }\nif {1} { namespace eval ::dst { namespace forget ::A::p } }\nnamespace eval ::dst { namespace import ::B::* }\np\n";
    assert_eq!(
        later_import_after_an_unplaced_forget(src, None).as_deref(),
        Some("::B::p"),
    );
}

#[test]
fn a_clear_in_the_sourcing_document_stays_conditional_for_the_sourced_export() {
    // Cross-document order stays conditional even along a `source` edge:
    // the clear in `app.tcl` is not ordered after the pattern `lib.tcl`
    // exports, so it revokes nothing.
    let lib = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app_src = "source lib.tcl\nnamespace eval ::mymod { namespace export -clear }\n";
    let app = analyse(app_src);
    let declaration = lib
        .original_procedure_declarations()
        .next()
        .unwrap()
        .name()
        .clone();
    let pattern = lib.original_namespace_exports().next().unwrap().span();
    let tombstone = app.original_namespace_exports().next().unwrap().span();
    let index = sourced_index([("file:///p/lib.tcl", &lib), ("file:///p/app.tcl", &app)]);
    let order = index.run_order();
    let first = RunPoint {
        uri: "file:///p/lib.tcl",
        at: pattern.start(),
        enclosing_body: None,
    };
    let clear = RunPoint {
        uri: "file:///p/app.tcl",
        at: tombstone.start(),
        enclosing_body: None,
    };
    for (a, b) in [(first, clear), (clear, first)] {
        assert!(order.navigation_precedence(a, b).is_none_or(|relation| {
            relation.purpose() == crate::source_graph::SourceNavigationOrderPurpose::Conditional
                && relation.modeled_source_exclusion().is_none()
        }));
    }
    let query = RunPoint {
        uri: "file:///p/app.tcl",
        at: u32::try_from(app_src.len()).unwrap(),
        enclosing_body: None,
    };
    assert_eq!(
        crate::namespace_import::NamespaceExportOracle::exported_at_original(
            &*index.export_snapshot(),
            declaration.slot(),
            declaration.policy(),
            query
        ),
        ExportVerdict::Exported
    );
}

#[test]
fn a_top_level_import_is_not_conflicted_by_a_later_top_level_one() {
    // TN guard — at load level the offsets *are* execution order, so the
    // first import still wins and the second is the one that installs
    // nothing. Oracle (8.6.14 / 9.0.4): after `namespace import ::A::*`
    // then `namespace import ::B::p`, the second raises `can't import
    // command "p": already exists` and `namespace origin ::dst::p` stays
    // `::A::p`. Making the check body-aware must not reorder these.
    let (a, b) = two_exporting_sources();
    let dst = analyse(
        "namespace eval ::dst {\n    namespace import ::A::*\n}\nnamespace eval ::dst {\n    namespace import ::B::p\n}\np\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    assert_ne!(index.resolve_command_target("::dst::p"), "::B::p");
    assert_eq!(
        index
            .resolve_wildcard_import(
                "p",
                &["::dst::p".to_string(), "::p".to_string()],
                call_from("file:///dst.tcl"),
            )
            .as_deref(),
        Some("::A::p"),
    );
}

#[test]
fn a_later_import_in_the_same_body_has_not_run_yet() {
    // TN guard, the other half — inside one body the offsets are genuine
    // execution order again. Oracle (8.6.14 / 9.0.4): `proc q {} {
    // namespace import ::A::* ; namespace import ::B::p }` → the first
    // succeeds, the second raises `already exists`, origin stays `::A::p`.
    // So the *first* must not be conflicted away by the second.
    let (a, b) = two_exporting_sources();
    let dst = analyse(
        "namespace eval ::dst {\n    proc q {} { namespace import ::A::* ; namespace import ::B::p }\n}\np\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    assert_ne!(index.resolve_command_target("::dst::p"), "::B::p");
    assert_eq!(
        index
            .resolve_wildcard_import(
                "p",
                &["::dst::p".to_string(), "::p".to_string()],
                call_from("file:///dst.tcl"),
            )
            .as_deref(),
        Some("::A::p"),
    );
}

#[test]
fn a_glob_import_in_another_file_does_not_conflict_with_an_exact_one() {
    // FP guard — two imports in different documents have no static load
    // order, so conflicting on a guess would drop a link Tcl installed.
    // The abstention is the same one every other cross-file event takes.
    let (a, b) = two_exporting_sources();
    let globbed = analyse("namespace eval ::dst { namespace import ::A::* }\n");
    let exact = analyse("namespace eval ::dst { namespace import ::B::p }\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///glob.tcl", &globbed),
        ("file:///exact.tcl", &exact),
    ]);
    assert_eq!(index.resolve_command_target("::dst::p"), "::B::p");
}

#[test]
fn an_exact_import_link_dies_on_a_same_file_forget() {
    // TN (CRITICAL) — an exact `namespace import ::mymod::helper`
    // produces a fixed link rather than a per-call glob lookup, so the
    // lifecycle events must reach it: otherwise the link stays live after
    // the forget and cross-document definition / references keep
    // resolving `::app::helper` to `::mymod::helper`.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "rename auto_import savedPrelude; namespace eval ::app {\n    namespace import ::mymod::helper\n    namespace forget ::mymod::helper\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::app::helper",
        "a forgotten exact import installs no live link"
    );
    assert!(!index.workspace_command_exists("::app::helper"));
}

#[test]
fn an_exact_import_link_keeps_a_conditional_same_file_forget() {
    // The link reads the document after it completes normally. A forget
    // behind a possible early exit, or in a procedure body, has run only
    // on some paths (tclsh 8.6 / 9.0: `x` true, or `teardown` never
    // called, leaves `::app::helper` imported), so the link stays live.
    // The renamed `auto_import` keeps the imports themselves modelled.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    for source in [
        "rename auto_import savedPrelude; namespace eval ::app {\n    namespace import ::mymod::helper\n}\nif {$x} return\nnamespace eval ::app {\n    namespace forget ::mymod::helper\n}\n",
        "rename auto_import savedPrelude; namespace eval ::app {\n    namespace import ::mymod::helper\n}\nproc teardown {} {\n    namespace eval ::app {namespace forget ::mymod::helper}\n}\n",
    ] {
        let app = analyse(source);
        let index = WorkspaceIndex::from_documents([
            ("file:///mymod.tcl", &mymod),
            ("file:///app.tcl", &app),
        ]);
        assert_eq!(
            index.resolve_command_target("::app::helper"),
            "::mymod::helper",
            "{source}"
        );
    }
}

#[test]
fn an_exact_import_link_survives_a_forget_written_before_it() {
    // FN guard for the row above — the forget is ordered against the
    // *import*, and one written before it is undone by the import itself
    // (a re-import after a forget reinstalls — oracle).
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace forget ::mymod::helper\n    namespace import ::mymod::helper\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::mymod::helper"
    );
}

#[test]
fn a_destroyed_source_kills_a_body_local_exact_import_link() {
    // TN (CRITICAL): the destruction is not a timeline event — the command
    // object is gone workspace-wide and no load order brings it back — so
    // it must revoke however the import is written. A **body-local**
    // import gate is the case that separates the two encodings: the
    // load-order rule reads a removal written outside the import's own
    // body as having run before it, which is right for a `namespace
    // forget` (the import then undoes it) and backwards for a destruction
    // the import cannot undo. Oracle: `rename ::mymod::helper {}` makes
    // `::app::helper` an `invalid command name` and empties `info commands
    // ::app::*`.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc setup {} { namespace import ::mymod::helper }\n}\nrename ::mymod::helper {}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::app::helper",
        "destroying the source command revokes the link wherever the import sits"
    );
    assert!(!index.workspace_command_exists("::app::helper"));
    // …and the top-level spelling of the same import, for the control.
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::helper\n}\nrename ::mymod::helper {}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::app::helper"
    );
}

#[test]
fn a_body_local_exact_import_survives_a_top_level_forget() {
    // FN guard for the row above, and the reason the destruction cannot
    // simply be encoded as "a removal at `u32::MAX`": a `namespace forget`
    // at the file's load level runs *before* a body-local import, which
    // then reinstalls the alias. Oracle: `::app::setup` followed by
    // `namespace origin ::app::helper` answers `::mymod::helper`.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc setup {} { namespace import ::mymod::helper }\n}\nnamespace eval ::app { namespace forget ::mymod::helper }\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::mymod::helper",
        "a load-level forget runs before the body-local import that reinstalls it"
    );
}

#[test]
fn an_exact_import_link_ignores_a_forget_in_another_file() {
    // FP guard — the same-document ordering rule on the link tier: a forget with
    // no static order against the import revokes nothing.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse("namespace eval ::app {\n    namespace import ::mymod::helper\n}\n");
    let teardown = analyse("namespace eval ::app {\n    namespace forget ::mymod::helper\n}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///app.tcl", &app),
        ("file:///teardown.tcl", &teardown),
    ]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::mymod::helper"
    );
}

#[test]
fn an_exact_import_link_dies_when_the_source_command_is_destroyed() {
    // TN — destroying the source is not order-ambiguous the way a forget
    // is: the command *object* the alias holds is gone workspace-wide
    // (oracle: `rename ::mymod::helper {}` makes `::app::helper` an
    // `invalid command name` and empties `info commands ::app::*`), so the
    // link dies even though the deletion sits in another document.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse("namespace eval ::app {\n    namespace import ::mymod::helper\n}\n");
    let teardown = analyse("rename ::mymod::helper {}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///app.tcl", &app),
        ("file:///teardown.tcl", &teardown),
    ]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::app::helper",
        "destroying the source destroys the link"
    );
}

#[test]
fn an_exact_import_link_dies_when_the_imported_name_is_redefined() {
    // TN, cross-document — a `proc ::app::helper`
    // after the import recreates the name as an ordinary command
    // (oracle: rc 0, `namespace origin` → `::app::helper`).
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::helper\n}\nproc ::app::helper {} {}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::app::helper"
    );
}

#[test]
fn a_cross_file_live_alias_is_an_import_conflict_for_a_different_source() {
    // TN, cross-document — `::dst` imports `::A::*`
    // and then, further down the same file, `::B::*` without `-force`.
    // Oracle: the second import raises `can't import command "p": already
    // exists` and `namespace origin ::dst::p` stays `::A::p`.
    let a = analyse("namespace eval ::A { proc p {} { return AP }\n namespace export p }\n");
    let b = analyse("namespace eval ::B { proc p {} { return BP }\n namespace export p }\n");
    let dst = analyse(
        "namespace eval ::dst {\n    namespace import ::A::*\n}\nnamespace eval ::dst {\n    namespace import ::B::*\n}\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    let resolved = index.resolve_wildcard_import(
        "p",
        &["::dst::p".to_string(), "::p".to_string()],
        call_from("file:///dst.tcl"),
    );
    assert_eq!(
        resolved.as_deref(),
        Some("::A::p"),
        "the failed second import leaves the first alias in place: {resolved:?}"
    );
}

#[test]
fn a_cross_file_forced_import_replaces_a_live_alias() {
    // TP, the other half — with `-force` the second import wins (oracle:
    // `namespace origin ::dst::p` → `::B::p`).
    let a = analyse("namespace eval ::A { proc p {} { return AP }\n namespace export p }\n");
    let b = analyse("namespace eval ::B { proc p {} { return BP }\n namespace export p }\n");
    let dst = analyse(
        "namespace eval ::dst {\n    namespace import ::A::*\n}\nnamespace eval ::dst {\n    namespace import -force ::B::*\n}\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    let resolved = index.resolve_wildcard_import(
        "p",
        &["::dst::p".to_string(), "::p".to_string()],
        call_from("file:///dst.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::B::p"));
}

#[test]
fn a_cross_file_exact_link_conflicts_with_an_earlier_different_source() {
    // TN — the live-alias conflict on the exact-link tier.
    let a = analyse("namespace eval ::A { proc p {} {}\n namespace export p }\n");
    let b = analyse("namespace eval ::B { proc p {} {}\n namespace export p }\n");
    let dst = analyse(
        "namespace eval ::dst {\n    namespace import ::A::p\n    namespace import ::B::p\n}\n",
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///a.tcl", &a),
        ("file:///b.tcl", &b),
        ("file:///dst.tcl", &dst),
    ]);
    assert_eq!(
        index.resolve_command_target("::dst::p"),
        "::A::p",
        "the second exact import fails and the first alias stays"
    );
}

#[test]
fn a_cross_file_import_chain_follows_to_the_original_source() {
    // TP — `::A` imports `::B::*`, `::B` imported `::C::*`, each in its
    // own file. Oracle: `::A::p` runs `::C`'s body and `namespace origin
    // ::A::p` → `::C::p`. The middle hop is in no workspace proc table,
    // so a single-hop walk abstained.
    let c = analyse("namespace eval ::C { proc p {} { return CP }\n namespace export p }\n");
    let b = analyse("namespace eval ::B { namespace import ::C::*\n namespace export p }\n");
    let a = analyse("namespace eval ::A { namespace import ::B::* }\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///c.tcl", &c),
        ("file:///b.tcl", &b),
        ("file:///a.tcl", &a),
    ]);
    let resolved = index.resolve_wildcard_import(
        "p",
        &["::A::p".to_string(), "::p".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::C::p"), "{resolved:?}");
}

#[test]
fn a_cross_file_chain_hop_that_was_never_re_exported_abstains() {
    // FN guard — the middle hop keeps its own export gate.
    let c = analyse("namespace eval ::C { proc p {} { return CP }\n namespace export p }\n");
    let b = analyse("namespace eval ::B { namespace import ::C::* }\n");
    let a = analyse("namespace eval ::A { namespace import ::B::* }\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///c.tcl", &c),
        ("file:///b.tcl", &b),
        ("file:///a.tcl", &a),
    ]);
    let resolved = index.resolve_wildcard_import(
        "p",
        &["::A::p".to_string(), "::p".to_string()],
        call_from("file:///caller.tcl"),
    );
    assert!(resolved.is_none(), "{resolved:?}");
}

#[test]
fn a_mutually_importing_pair_terminates_cross_file() {
    // FP/hang guard — bounded by `MAX_COMMAND_NAME_HOPS`.
    let a = analyse("namespace eval ::A { namespace import ::B::*\n namespace export * }\n");
    let b = analyse("namespace eval ::B { namespace import ::A::*\n namespace export * }\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert!(
        index
            .resolve_wildcard_import(
                "p",
                &["::A::p".to_string()],
                call_from("file:///caller.tcl"),
            )
            .is_none()
    );
}
