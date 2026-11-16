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

//! References across documents, namespaces paths and imports.

use super::*;

#[tokio::test]
async fn original_workspace_reference_misses_never_reparse_reporting_names() {
    // Implementation contract: naming.server.original-reference-and-lens-source-selection
    // docs/design/analysis/name-resolution-proofs/server-original-reference-and-lens-source-selection.md
    let backend = test_backend();
    let provider = Uri::from_str("file:///original-reference-real.tcl").unwrap();
    let consumer = Uri::from_str("file:///original-reference-missing.tcl").unwrap();
    register(&backend, &provider, "proc real {} {}\nreal").await;
    register(&backend, &consumer, "missing").await;
    let document = backend.read_document(&consumer).await.unwrap().unwrap();
    let mut analysis = (*backend
        .analysis_for(&consumer, document.text.clone(), document.dialect.clone())
        .await
        .unwrap())
    .clone();
    let provider_doc = backend.read_document(&provider).await.unwrap().unwrap();
    let provider_analysis = backend
        .analysis_for(&provider, provider_doc.text, provider_doc.dialect)
        .await
        .unwrap();
    let mut counterfactual = provider_analysis.all_procs.values().next().unwrap().clone();
    counterfactual.name = "missing".to_owned();
    counterfactual.qualified_name = "::missing".to_owned();
    analysis
        .all_procs
        .insert("::missing".to_owned(), counterfactual);
    let position = Position {
        line: 0,
        character: 2,
    };
    assert_eq!(
        Box::pin(backend.reference_locations(&consumer, &document, &analysis, position, true))
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Location; 0]
    );
    assert_eq!(
        backend
            .resolve_workspace_symbols(&consumer, &document.text, &analysis, position)
            .await
            .unwrap(),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        backend
            .cross_document_definition(&consumer, &document.text, position, &analysis)
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Location; 0]
    );
    assert!(
        backend
            .original_declaration_identity_at(
                &provider,
                "proc real {} {}; real",
                &provider_analysis,
                position
            )
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn cross_document_references_finds_sibling_call_sites() {
    let backend = test_backend();
    let lib = Uri::from_str("file:///lib.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
    let lib_src = "proc helper {} {}\nhelper\n";
    register(&backend, &lib, lib_src).await;
    register(&backend, &consumer, "helper\nhelper\n").await;
    // From lib.tcl's `helper` declaration, cross-doc refs
    // are the two calls in consumer.tcl.
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(lib_src, "tcl8.6").clone()
    };
    let cross = backend
        .cross_document_references(&lib, lib_src, &analysis, Position::new(0, 5), false)
        .await
        .unwrap();
    assert_eq!(cross.len(), 2, "{cross:?}");
    assert!(cross.iter().all(|l| l.uri == consumer));
}

/// The pure-consumer shape, reduced to a literal `source`: a file that
/// `source`s a sibling and then calls its procs bare. `a.tcl` sources
/// `b.tcl` (which declares `helper`) and calls `helper` twice itself;
/// `c.tcl` calls it once more. `a.tcl` has no local declaration of
/// `helper` to anchor `cross_document_references`'s exclusion of the
/// current document on, so without the workspace-resolved fallback both
/// of `a.tcl`'s own calls — including the one under the cursor — are
/// dropped entirely.
#[tokio::test]
async fn workspace_resolved_references_reaches_the_current_documents_own_calls() {
    let backend = test_backend();
    let a = Uri::from_str("file:///a.tcl").unwrap();
    let b = Uri::from_str("file:///b.tcl").unwrap();
    let c = Uri::from_str("file:///c.tcl").unwrap();
    let b_src = "proc helper {} {}\n";
    register(&backend, &b, b_src).await;
    let a_src = "source b.tcl\nhelper\nhelper\n";
    register(&backend, &a, a_src).await;
    register(&backend, &c, "helper\n").await;
    let analysis = {
        let mut an = Analyser::new();
        an.analyse(a_src, "tcl8.6").clone()
    };
    // Cursor on a.tcl's own first `helper` call (line 1).
    let refs = backend
        .workspace_resolved_references(&a, a_src, &analysis, Position::new(1, 3), true)
        .await
        .unwrap();
    let a_hits = refs.iter().filter(|l| l.uri == a).count();
    assert_eq!(
        a_hits, 2,
        "both of a.tcl's own calls must be reached, not just the sourced-in decl: {refs:?}"
    );
    assert!(
        refs.iter().any(|l| l.uri == b),
        "b.tcl's declaration must still be reached: {refs:?}"
    );
    assert!(
        refs.iter().any(|l| l.uri == c),
        "c.tcl's call must still be reached: {refs:?}"
    );
}

/// End-to-end analogue of the test above, through the real
/// `textDocument/references` handler — proving the handler picks the
/// workspace-resolved fallback automatically (no caller-side branching
/// needed) whenever the single-document pass finds nothing local to
/// anchor on.
#[tokio::test]
async fn references_handler_reaches_own_calls_in_a_pure_consumer_document() {
    let backend = test_backend();
    let a = Uri::from_str("file:///a2.tcl").unwrap();
    let b = Uri::from_str("file:///b2.tcl").unwrap();
    register(&backend, &b, "proc helper {} {}\n").await;
    let a_src = "source b2.tcl\nhelper\nhelper\n";
    register(&backend, &a, a_src).await;
    let params = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: a.clone() },
            position: Position::new(1, 3), // on a.tcl's own first `helper` call
        },
        context: ReferenceContext {
            include_declaration: true,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let refs = Box::pin(backend.references(params))
        .await
        .expect("ok")
        .expect("some references");
    let a_hits: Vec<_> = refs.iter().filter(|l| l.uri == a).collect();
    assert_eq!(
        a_hits.len(),
        2,
        "both of a.tcl's own calls must be reached: {refs:?}"
    );
    assert!(
        refs.iter().any(|l| l.uri == b),
        "b.tcl's declaration must still be reached: {refs:?}"
    );
}

/// A proc declared twice, verbatim, in the same document (plain Tcl's own
/// "last redefinition wins" semantics, tclsh 9.0 / 8.6-verified) — the
/// shape a real library writes when it declares one proc at two separate
/// line ranges.  Identifying "the symbol at cursor" only
/// via `all_procs.values().find(|p| covers(p.name_span))`, and
/// `all_procs` (keyed by qualified name) retains only the *winning*
/// declaration's span on a duplicate insert — so a cursor on the
/// *shadowed* (non-winning) declaration's own name token could never
/// match, silently dropping every cross-file caller from
/// find-references, and — far worse — from rename: applying that
/// incomplete `WorkspaceEdit` leaves a cross-file caller still bound
/// to the old name, which a second, dead definition (the very shadowed
/// declaration the rename was issued from) is still lying around to
/// silently satisfy, changing the program's behaviour with no error
/// surfaced anywhere.
#[tokio::test]
pub(in super::super) async fn cross_document_references_reach_caller_from_shadowed_duplicate_decl()
{
    let backend = test_backend();
    let lib = Uri::from_str("file:///lib31_refs.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer31_refs.tcl").unwrap();
    let lib_src = "proc List2array {lst} { return ONE }\nproc List2array {lst} { return TWO }\n";
    register(&backend, &lib, lib_src).await;
    register(&backend, &consumer, "List2array x\n").await;
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(lib_src, "tcl8.6").clone()
    };
    // Cursor on the SHADOWED (first, non-winning) declaration's own
    // name token (line 0, col 6 — `proc List2array`).
    let cross = backend
        .cross_document_references(&lib, lib_src, &analysis, Position::new(0, 6), false)
        .await
        .unwrap();
    assert_eq!(
        cross.len(),
        1,
        "cross-file call site in consumer.tcl must not be silently \
             dropped when querying from the shadowed declaration: {cross:?}"
    );
    assert!(cross.iter().all(|l| l.uri == consumer));
}

/// The *nested, cross-namespace* self-redefinition: a
/// `proc ticklecharts::activate` whose body declares an unqualified
/// `proc activate`.
///
/// Distinct from the shape above (two verbatim-identical top-level
/// declarations in one namespace) in the half that matters: the inner
/// declaration is written *unqualified inside a qualified proc's body*,
/// so reaching the cross-file caller depends on namespace resolution, not
/// only on keeping every physical declaration's span.
///
/// Oracle, byte-identical on tclsh 9.0.4 and 8.6.16: running `main.tcl`
/// prints `activating for the first time` then `already activated`, and
/// probing the command table afterwards reports `::ticklecharts::activate`
/// present with no global `::activate` at all — the nested `proc`
/// redefines the *same* command in the enclosing namespace.
#[tokio::test]
async fn cross_document_references_reach_caller_from_nested_self_redefinition() {
    let backend = test_backend();
    let lib = Uri::from_str("file:///lib45_refs.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer45_refs.tcl").unwrap();
    let lib_src = "namespace eval ticklecharts {}\n\
                       proc ticklecharts::activate {bool} {\n\
                       \x20if {$bool} {\n\
                       \x20 proc activate {bool} {puts \"already activated\"}\n\
                       \x20 puts \"activating for the first time\"\n\
                       \x20}\n\
                       }\n";
    register(&backend, &lib, lib_src).await;
    register(
        &backend,
        &consumer,
        "ticklecharts::activate 1\nticklecharts::activate 1\n",
    )
    .await;
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(lib_src, "tcl8.6").clone()
    };
    // Both physical declarations must answer the same cross-file set:
    // the OUTER header (line 1, on `activate` of `ticklecharts::activate`)
    // and the NESTED one (line 3, on the bare `activate`).
    for (line, character, which) in [(1u32, 20u32, "outer"), (3, 7, "nested")] {
        let cross = backend
            .cross_document_references(
                &lib,
                lib_src,
                &analysis,
                Position::new(line, character),
                false,
            )
            .await
            .unwrap();
        assert_eq!(
            cross.len(),
            2,
            "both cross-file callers must be reached from the {which} \
                 declaration: {cross:?}"
        );
        assert!(cross.iter().all(|l| l.uri == consumer), "{cross:?}");
    }
}

#[tokio::test]
async fn cross_document_references_resolve_namespace_path_collision() {
    // References on `::mymod::helper`'s
    // declaration must include the bare `helper` call in app.tcl (reached
    // via `namespace path`), even though `::other` defines the same simple
    // name and the call's file-local guess settles to `::app::helper`.
    let (backend, mymod, _other, app) = register_namespace_path_workspace().await;
    let mymod_src = "namespace eval ::mymod { proc helper {} {} }\n";
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(mymod_src, "tcl8.6").clone()
    };
    // Cursor on `helper` in `::mymod`'s declaration (col 30).
    let refs = backend
        .cross_document_references(&mymod, mymod_src, &analysis, Position::new(0, 32), false)
        .await
        .unwrap();
    assert_eq!(refs.len(), 1, "{refs:?}");
    assert_eq!(refs[0].uri, app);
}

#[tokio::test]
async fn cross_document_references_do_not_cross_link_colliding_namespace() {
    // References on the *unrelated* `::other::helper` must be empty: the
    // app.tcl call resolves to `::mymod::helper` via the namespace path, so
    // it is never a reference of `::other::helper`.
    let (backend, _mymod, other, _app) = register_namespace_path_workspace().await;
    let other_src = "namespace eval ::other { proc helper {} {} }\n";
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(other_src, "tcl8.6").clone()
    };
    let refs = backend
        .cross_document_references(&other, other_src, &analysis, Position::new(0, 32), false)
        .await
        .unwrap();
    assert!(refs.is_empty(), "{refs:?}");
}

/// Build a two-file `namespace import` workspace: `::mymod` exports
/// `helper`, `::app` imports it and calls a bare `helper`.
async fn register_namespace_import_workspace() -> (Backend, Uri, Uri) {
    let backend = test_backend();
    let mymod = Uri::from_str("file:///mymod.tcl").unwrap();
    let app = Uri::from_str("file:///app.tcl").unwrap();
    register(
        &backend,
        &mymod,
        "namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n",
    )
    .await;
    register(
            &backend,
            &app,
            "namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\n",
        )
        .await;
    (backend, mymod, app)
}

#[tokio::test]
async fn cross_document_references_follow_namespace_import() {
    // References on `::mymod::helper` must include app.tcl's imported bare
    // call (reached through the import link) and the `namespace import`
    // pattern token that names it.
    let (backend, mymod, app) = register_namespace_import_workspace().await;
    let mymod_src = "namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n";
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(mymod_src, "tcl8.6").clone()
    };
    // Cursor on `helper` in `::mymod`'s declaration (col 32).
    let refs = backend
        .cross_document_references(&mymod, mymod_src, &analysis, Position::new(0, 32), false)
        .await
        .unwrap();
    assert_eq!(refs.len(), 2, "imported call + import pattern: {refs:?}");
    assert!(refs.iter().all(|l| l.uri == app));
}

#[tokio::test]
async fn cross_document_references_from_imported_call_reach_source() {
    // A cursor on the imported bare `helper` call resolves *through* the
    // import to `::mymod::helper`, so its references gather with the source
    // declaration's — go-to-references works from the consumer side too.
    let (backend, mymod, app) = register_namespace_import_workspace().await;
    let app_src = "namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\n";
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(app_src, "tcl8.6").clone()
    };
    // Cursor on the imported `helper` call (line 2, col 20), with the
    // declaration included.
    let refs = backend
        .cross_document_references(&app, app_src, &analysis, Position::new(2, 20), true)
        .await
        .unwrap();
    assert!(
        refs.iter().any(|l| l.uri == mymod),
        "imported call should reach the source declaration: {refs:?}",
    );
}

#[tokio::test]
async fn references_handler_merges_local_and_cross_document() {
    let backend = test_backend();
    let lib = Uri::from_str("file:///lib.tcl").unwrap();
    let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
    // lib.tcl: declaration + one local call.
    register(&backend, &lib, "proc helper {} {}\nhelper\n").await;
    // consumer.tcl: one cross-doc call.
    register(&backend, &consumer, "helper\n").await;
    let params = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: lib.clone() },
            position: Position::new(0, 5),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: ReferenceContext {
            include_declaration: true,
        },
    };
    let result = Box::pin(backend.references(params))
        .await
        .expect("ok")
        .expect("some");
    // Declaration (lib:0) + local call (lib:1) + cross-doc
    // call (consumer:0) = 3 locations across 2 files.
    assert_eq!(result.len(), 3, "{result:?}");
    assert!(result.iter().any(|l| l.uri == consumer));
    assert_eq!(result.iter().filter(|l| l.uri == lib).count(), 2);
}

#[tokio::test]
async fn references_handler_bigip_conf_finds_path_occurrences() {
    // A BIG-IP `.conf` document (dialect `f5-bigip`) routes references
    // through the TMSH-path textual search, not the Tcl analyser.
    let backend = test_backend();
    let uri = Uri::from_str("file:///bigip.conf").unwrap();
    let src = "ltm pool /Common/p {\n}\nltm virtual /Common/v {\n    pool /Common/p\n}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new(src.to_owned(), "f5-bigip".to_owned()),
    );
    let params = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            // On `/Common/p` in the virtual's `pool /Common/p` line.
            position: Position::new(3, 9),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: ReferenceContext {
            include_declaration: true,
        },
    };
    let result = Box::pin(backend.references(params))
        .await
        .expect("ok")
        .expect("some");
    // Pool declaration (line 0) + the virtual's `pool /Common/p` (line 3).
    assert_eq!(result.len(), 2, "{result:?}");
    assert!(result.iter().all(|l| l.uri == uri));
    let lines: Vec<u32> = result.iter().map(|l| l.range.start.line).collect();
    assert!(lines.contains(&0) && lines.contains(&3), "{lines:?}");
}
