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

//! The life of an import alias across documents: `namespace forget`, source deletion and rename, and where the call sits relative to the install.

use super::*;

// The import edge's own lifecycle, cross-document.
//
// The workspace twin of `definition.rs`'s in-document block. Same oracle
// rows (tclsh 9.0.4 + 8.6.14, byte-identical), same shared decision
// function (`namespace_import::alias_live_at`), so the two tiers cannot
// drift.

#[test]
fn cross_file_forget_after_the_import_stops_resolving() {
    // TN — the source lives in another file, so only this tier can
    // answer; the forget and the call are in one document, so they are
    // ordered.
    let src = "namespace eval ::app {\n    namespace import ::mymod::*\n}\nnamespace eval ::app {\n    namespace forget ::mymod::helper\n}\nhelper\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_at("file:///app.tcl", call),
    );
    assert!(
        resolved.is_none(),
        "a call after the forget must not resolve through the alias: {resolved:?}"
    );
}

#[test]
fn cross_file_call_before_the_forget_still_resolves() {
    // TP — the same document, a call written before the forget.
    let src = "namespace eval ::app {\n    namespace import ::mymod::*\n}\nhelper\nnamespace eval ::app {\n    namespace forget ::mymod::helper\n}\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_at("file:///app.tcl", call),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

#[test]
fn a_forget_in_another_file_revokes_nothing() {
    // TN-for-abstention — no static load order between two files, so a
    // foreign forget is passed unordered and cannot silently drop a real
    // alias. Same direction as an unordered `namespace export -clear`.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse("namespace eval ::app {\n    namespace import ::mymod::*\n}\n");
    let teardown = analyse("namespace eval ::app {\n    namespace forget ::mymod::helper\n}\n");
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
fn cross_file_source_deletion_kills_the_alias() {
    // TN — `rename ::mymod::helper {}` destroys the command object, and
    // the alias holds the object. Ordered because the deletion and the
    // call share a document.
    let src = "namespace eval ::app {\n    namespace import ::mymod::*\n}\nrename ::mymod::helper {}\nhelper\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_at("file:///app.tcl", call),
    );
    assert!(resolved.is_none(), "{resolved:?}");
}

#[test]
fn cross_file_source_rename_leaves_the_alias_alive() {
    // TP — the asymmetry again: `rename ::mymod::helper ::mymod::h2`
    // moves the origin and keeps `::app::helper` working.
    let src = "namespace eval ::app {\n    namespace import ::mymod::*\n}\nrename ::mymod::helper ::mymod::h2\nhelper\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_at("file:///app.tcl", call),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

#[test]
fn an_unforced_import_onto_an_existing_workspace_command_installs_nothing() {
    // TN — `::app` already defines `helper`, so the non-`-force` import
    // errors and installs nothing; the call reaches the local definition,
    // not `::mymod::helper` (oracle: `namespace origin ::app::helper` →
    // `::app::helper`).
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc helper {} {}\n    namespace import ::mymod::*\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///app.tcl"),
    );
    assert!(resolved.is_none(), "{resolved:?}");
}

#[test]
fn a_forced_import_onto_an_existing_workspace_command_installs() {
    // TP — the same program with `-force` replaces the local command, so
    // the call reaches `::mymod::helper` (oracle: `namespace origin
    // ::app::helper` → `::mymod::helper`).
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc helper {} {}\n    namespace import -force ::mymod::*\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_from("file:///app.tcl"),
    );
    assert_eq!(resolved.as_deref(), Some("::mymod::helper"));
}

#[test]
fn an_exact_import_onto_an_existing_workspace_command_installs_no_link() {
    // TN — the same conflict rule on the exact-import link path
    // (`live_command_links`), so definition / references / the existence
    // oracle all agree the import bound nothing.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc helper {} {}\n    namespace import ::mymod::helper\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::app::helper",
        "a conflicting exact import installs no link"
    );
}

#[test]
fn a_forced_exact_import_still_installs_its_link() {
    // FN guard for the rule above.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    proc helper {} {}\n    namespace import -force ::mymod::helper\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    assert_eq!(
        index.resolve_command_target("::app::helper"),
        "::mymod::helper"
    );
}

#[test]
fn a_cross_file_forget_cannot_revoke_on_unrelated_offsets() {
    // FP guard (CRITICAL) — the import lives in a short `imports.tcl`
    // (small byte offset); the forget lives in a long `caller.tcl` (large
    // byte offset) and names a namespace the caller never imported into
    // itself. Nothing orders the two files, so the forget must revoke
    // nothing — comparing the raw offsets would make the caller's larger
    // number "later" and drop a live alias.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let imports = analyse("namespace eval ::app { namespace import ::mymod::* }\n");
    // Padding so the forget's offset is numerically far past the import's.
    let pad = "# ".to_string() + &"x".repeat(400) + "\n";
    let caller_src =
        format!("{pad}namespace eval ::app {{\n    namespace forget ::mymod::helper\n}}\nhelper\n");
    let caller = analyse(&caller_src);
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///imports.tcl", &imports),
        ("file:///caller.tcl", &caller),
    ]);
    let call = u32::try_from(app_src_offset(&caller_src, "\nhelper\n") + 1).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_at("file:///caller.tcl", call),
    );
    assert_eq!(
        resolved.as_deref(),
        Some("::mymod::helper"),
        "a forget in another document has no order against the import and \
             must not revoke it: {resolved:?}"
    );
}

#[test]
fn a_same_file_forget_after_the_import_still_revokes() {
    // TN, the other direction of the same-document rule — when the import, the forget
    // and the call really do share a document the offsets mean something
    // and the ordering is unchanged.
    let src = "rename auto_import savedPrelude; namespace eval ::app {\n    namespace import ::mymod::*\n}\nnamespace eval ::app {\n    namespace forget ::mymod::helper\n}\nhelper\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let call = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        call_at("file:///app.tcl", call),
    );
    assert!(resolved.is_none(), "{resolved:?}");
}

// Call-site ordering of the install.
//
// Oracle, byte-identical on tclsh 8.6.14 and 9.0.4:
//
//   namespace eval ::src { proc p {} {return P}; namespace export p }
//   namespace eval ::dst { p }                  ;# invalid command name "p"
//   namespace eval ::dst { namespace import ::src::* }
//   namespace eval ::dst { p }                  ;# P
//
// …and the body-scope half, same transcript run:
//
//   namespace eval ::app { proc run {} { helper } }
//   namespace eval ::app { namespace import ::src::* }
//   ::app::run                                  ;# HELP
//
//   namespace eval ::app2 { proc run2 {} {
//       catch {helper} e ; namespace import ::src::* ; return "$e [helper]" } }
//   ::app2::run2   ;# {invalid command name "helper"} HELP

#[test]
fn a_top_level_call_before_its_own_import_does_not_resolve() {
    // TN. Both call sites are top level, so the offsets are in genuine
    // execution order and the earlier one reaches nothing.
    let src = "rename auto_import savedPrelude; helper\nnamespace eval ::app {\n    namespace import ::mymod::*\n}\nhelper\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let candidates = ["::app::helper".to_string(), "::helper".to_string()];
    let before = u32::try_from(app_src_offset(src, "helper\n")).expect("tiny source");
    assert!(
        index
            .resolve_wildcard_import("helper", &candidates, call_at("file:///app.tcl", before))
            .is_none(),
        "a call written before its own import must not resolve through it",
    );
    // TP — the same call after the import still does.
    let after = u32::try_from(app_src_offset(src, "\nhelper\n") + 1).expect("tiny source");
    assert_eq!(
        index
            .resolve_wildcard_import("helper", &candidates, call_at("file:///app.tcl", after))
            .as_deref(),
        Some("::mymod::helper"),
    );
}

#[test]
fn a_body_local_call_resolves_through_an_import_written_later_in_its_file() {
    // TP (CRITICAL) — the reason the install gate is `in_effect_within`
    // and not `at < call`: the whole file loads, imports included, before
    // any body runs, so this shape (procs first, `namespace import` at the
    // bottom) still resolves. `tcllib`'s `modules/uev/uevent.tcl` is
    // exactly it; a plain-offset gate broke all five of its call sites.
    let src = "namespace eval ::app {\n    proc run {} { helper }\n}\nnamespace eval ::app {\n    namespace import ::mymod::*\n}\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let call = u32::try_from(app_src_offset(src, "helper }")).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        CallSite {
            uri: "file:///app.tcl",
            at: call,
            enclosing_body: app.innermost_definition_body_span(call),
        },
    );
    assert_eq!(
        resolved.as_deref(),
        Some("::mymod::helper"),
        "a body-local call observes its own file's later top-level import",
    );
}

#[test]
fn a_body_local_call_before_an_import_in_that_same_body_does_not_resolve() {
    // TN — the FN guard for the leniency above: an import written in the
    // *same* body after the call is an ordinary later statement of the
    // running script, exactly as the export snapshot already treats one.
    let src =
        "namespace eval ::app {\n    proc run {} { helper ; namespace import ::mymod::* }\n}\n";
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(src);
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    let call = u32::try_from(app_src_offset(src, "helper ;")).expect("tiny source");
    let resolved = index.resolve_wildcard_import(
        "helper",
        &["::app::helper".to_string(), "::helper".to_string()],
        CallSite {
            uri: "file:///app.tcl",
            at: call,
            enclosing_body: app.innermost_definition_body_span(call),
        },
    );
    assert!(resolved.is_none(), "{resolved:?}");
}

#[test]
fn a_call_before_an_import_in_another_file_still_resolves() {
    // TP — cross-file installs have no static load order, so the gate
    // cannot fire on a foreign offset (the same abstention every other
    // cross-file event takes here). `at` is deliberately tiny.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let imports = analyse("namespace eval ::app { namespace import ::mymod::* }\n");
    let caller = analyse("helper\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///imports.tcl", &imports),
        ("file:///caller.tcl", &caller),
    ]);
    assert_eq!(
        index
            .resolve_wildcard_import(
                "helper",
                &["::app::helper".to_string(), "::helper".to_string()],
                call_at("file:///caller.tcl", 0),
            )
            .as_deref(),
        Some("::mymod::helper"),
    );
}

#[test]
fn the_body_span_column_matches_the_per_offset_lookup() {
    // `enclosing_body_spans` is a stack sweep standing in for one
    // `innermost_definition_body_span` per row (which is
    // O(procs × invocations)); nesting, shared starts and calls between
    // sibling bodies are where the two could part company.
    let a = analyse(
        "proc outer {} {\n  proc inner {} { set x 1 }\n  set y 2\n}\nset z 3\noo::class create C { method m {} { set w 4 } }\nset q 5\n",
    );
    let offsets: Vec<u32> = a
        .command_invocations
        .iter()
        .map(|i| i.range.start())
        .collect();
    assert!(offsets.len() > 5, "the fixture must exercise the sweep");
    let swept = WorkspaceIndex::enclosing_body_spans(&a, &offsets);
    let naive: Vec<Option<Span>> = offsets
        .iter()
        .map(|&o| a.innermost_definition_body_span(o))
        .collect();
    assert_eq!(swept, naive);
    // …and the column really is populated on the rows the resolver reads.
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    assert!(
        index.invocations().any(|i| i.enclosing_body.is_some()),
        "body-local invocations must carry their span",
    );
}
