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

//! The two-file `namespace import -force` shadow: the document tier and the workspace tier must agree.

use super::*;

// Both tiers on the two-file `namespace import -force` shadow.
//
// `MAIN` below is one document, byte-for-byte identical in every test of
// this group. What changes is the *rest of the program*, and with it the
// correct answer — which is exactly why the single-document tier needed a
// whole-program export oracle. Oracle transcript (tclsh 8.6.14 and 9.0.4,
// byte-identical), running `MAIN` from a loader that does or does not also
// source the `namespace export helper`:
//
//   with the export sourced first: call -> SRC    origin -> ::src::helper
//   with nothing exporting it:     call -> LOCAL  origin -> ::app::helper

/// The pinned two-file shape's main document.
const MAIN: &str = "namespace eval src {\n    proc helper {} { return SRC }\n    proc other {} { return O }\n    namespace export other\n}\nnamespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";

/// What each tier says about the `helper` call at the end of [`MAIN`],
/// given the rest of the program.
///
/// Returns `(in-document target, cross-document target)`, where a tier's
/// "target" is the qualified name the call reaches: the local definition
/// when the `-force` import bound nothing, the import's source when it
/// did, and `None` when the tier cannot say. The two are computed
/// independently — the single-document resolver over `MAIN`'s own analysis
/// plus the index's export oracle, and the workspace's own
/// [`WorkspaceIndex::resolve_wildcard_import`] — so agreeing is a real
/// result and not a tautology.
fn both_tiers_on_main(others: &[(&str, &AnalysisResult)]) -> (Option<String>, Option<String>) {
    let main = analyse(MAIN);
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///main.tcl", &main);
    for (uri, analysis) in others {
        index.add_document(uri, analysis);
    }
    let call = u32::try_from(MAIN.rfind("    helper\n").expect("call present") + 4)
        .expect("tiny test source");
    let candidates =
        tcl_syntax::naming::command_resolution_candidates("::app", &[] as &[String], "helper");

    let exports = index.export_snapshot();
    let in_document = crate::definition::resolve_called_proc(
        &main,
        MAIN,
        "::app",
        "helper",
        call,
        crate::definition::CallResolution::document_only().in_program(
            crate::definition::ProgramExports {
                uri: "file:///main.tcl",
                oracle: exports.as_ref(),
            },
        ),
    )
    .map(|p| p.qualified_name.clone());

    // The cross-document tier answers the import question; when no import
    // is live the call reaches whatever the workspace defines under the
    // first candidate, which is the local proc.
    let cross_document = index
        .resolve_wildcard_import(
            "helper",
            &candidates,
            CallSite {
                uri: "file:///main.tcl",
                at: call,
                enclosing_body: main.innermost_definition_body_span(call),
            },
        )
        .or_else(|| {
            candidates
                .iter()
                .find(|c| index.defines_command(c))
                .cloned()
        });
    (in_document, cross_document)
}

#[test]
fn both_tiers_shadow_when_another_file_holds_the_covering_export() {
    // TP (CRITICAL) — the export that decides it lives in `exports.tcl`.
    // Oracle: SRC / `::src::helper`.
    let exports = analyse("namespace eval src {\n    namespace export helper\n}\n");
    let (in_document, cross_document) = both_tiers_on_main(&[("file:///exports.tcl", &exports)]);
    assert_eq!(in_document.as_deref(), Some("::src::helper"));
    assert_eq!(
        in_document, cross_document,
        "the two tiers must reach the same command",
    );
}

#[test]
fn both_tiers_keep_the_local_when_the_whole_program_exports_nothing() {
    // TN, byte-identical `MAIN` — nothing anywhere exports `helper`, so
    // the `-force` import binds only `other` and the local definition
    // survives. Oracle: LOCAL / `::app::helper`.
    let unrelated = analyse("namespace eval other {\n    proc q {} {}\n}\n");
    let (in_document, cross_document) =
        both_tiers_on_main(&[("file:///unrelated.tcl", &unrelated)]);
    assert_eq!(in_document.as_deref(), Some("::app::helper"));
    assert_eq!(
        in_document, cross_document,
        "the two tiers must reach the same command",
    );
}

#[test]
fn both_tiers_shadow_when_the_source_namespace_is_wholly_in_another_file() {
    // TP — the third oracle shape: `::src`'s procs *and* its export are
    // elsewhere, so this document sees only the `-force` import and the
    // local proc it deletes. Oracle: SRC / `::src::helper`.
    let main = analyse(WHOLLY_FOREIGN);
    let lib = analyse(
        "namespace eval src {\n    proc helper {} { return SRC }\n    namespace export helper\n}\n",
    );
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///main.tcl", &main);
    index.add_document("file:///lib.tcl", &lib);
    let call = u32::try_from(WHOLLY_FOREIGN.rfind("    helper\n").expect("call present") + 4)
        .expect("tiny test source");
    let candidates =
        tcl_syntax::naming::command_resolution_candidates("::app", &[] as &[String], "helper");
    let exports = index.export_snapshot();
    // In-document: the source is in no local table, so the resolver cannot
    // *name* the target — but it must refuse to answer with the local
    // definition the import deleted, which is the abstention the shadow
    // gate exists for.
    let ctx = crate::definition::CallResolution::document_only().in_program(
        crate::definition::ProgramExports {
            uri: "file:///main.tcl",
            oracle: exports.as_ref(),
        },
    );
    assert!(crate::definition::forced_import_shadows_call(
        &main,
        ctx,
        "helper",
        &candidates,
        call,
    ));
    assert!(
        crate::definition::resolve_called_proc(&main, WHOLLY_FOREIGN, "::app", "helper", call, ctx)
            .is_none(),
        "the deleted local definition must not be the answer",
    );
    // …and the cross-document tier supplies the name.
    assert_eq!(
        index.resolve_wildcard_import(
            "helper",
            &candidates,
            CallSite {
                uri: "file:///main.tcl",
                at: call,
                enclosing_body: main.innermost_definition_body_span(call),
            },
        ),
        Some("::src::helper".to_owned()),
    );
}

/// The `-force` shape whose source namespace is wholly in another file.
const WHOLLY_FOREIGN: &str = "namespace eval app {\n    proc helper {} { return LOCAL }\n}\nnamespace eval app {\n    namespace import -force ::src::*\n}\nnamespace eval app {\n    helper\n}\n";

#[test]
fn the_settled_call_moves_to_the_import_source_when_the_shadow_is_live() {
    // The same fact on the *settle* path find-references reads: a
    // candidate naming the command a `-force` import deleted must not
    // settle the call, or find-references files it under a definition
    // go-to-definition no longer answers with.
    let exports = analyse("namespace eval src {\n    namespace export helper\n}\n");
    let main = analyse(MAIN);
    let index = WorkspaceIndex::from_documents([
        ("file:///main.tcl", &main),
        ("file:///exports.tcl", &exports),
    ]);
    assert_eq!(
        index
            .linked_invocations_of("::src::helper", "file:///exports.tcl")
            .len(),
        1,
        "the shadowed call is a reference to the import source",
    );
    assert!(
        index
            .linked_invocations_of("::app::helper", "file:///exports.tcl")
            .is_empty(),
        "…and not to the command the import deleted",
    );
}

#[test]
fn the_settled_call_stays_local_when_the_program_exports_nothing() {
    // TN, byte-identical `MAIN` — with nothing exporting `helper` the
    // import binds nothing and the call still belongs to the local proc.
    let unrelated = analyse("namespace eval other {\n    proc q {} {}\n}\n");
    let main = analyse(MAIN);
    let index = WorkspaceIndex::from_documents([
        ("file:///main.tcl", &main),
        ("file:///unrelated.tcl", &unrelated),
    ]);
    assert_eq!(
        index
            .linked_invocations_of("::app::helper", "file:///unrelated.tcl")
            .len(),
        1,
        "the surviving local definition owns the call",
    );
    assert!(
        index
            .linked_invocations_of("::src::helper", "file:///unrelated.tcl")
            .is_empty(),
        "…and an import that bound nothing creates no reference",
    );
}

#[test]
fn the_export_snapshot_answers_unknown_for_a_namespace_no_file_declares() {
    // The abstention, stated directly on the oracle: `::msgcat` is an
    // installed package, in no indexed document. Silence about its exports
    // is not evidence that it exports nothing — the same rule
    // `live_command_links` already applies to exact imports.
    use crate::namespace_import::NamespaceExportOracle as _;
    let app = analyse("namespace eval app {\n    namespace import -force ::msgcat::*\n}\n");
    let index = WorkspaceIndex::from_documents([("file:///app.tcl", &app)]);
    let snapshot = index.export_snapshot();
    let site = RunPoint {
        uri: "file:///app.tcl",
        at: 0,
        enclosing_body: None,
    };
    assert_eq!(
        snapshot.exported_at("::msgcat", "mc", site),
        ExportVerdict::Unknown,
    );
    // …while a namespace the workspace *can* see gives a real negative.
    assert_eq!(
        snapshot.exported_at("::app", "mc", site),
        ExportVerdict::NotExported,
    );
}
