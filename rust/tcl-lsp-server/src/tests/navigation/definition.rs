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

//! Go-to-definition: the command-head gate and cross-document fallbacks.

use super::*;

#[tokio::test]
async fn original_variable_queries_keep_opaque_slots_across_documents() {
    // Implementation contract: naming.server.original-variable-symbol-query
    // docs/design/analysis/name-resolution-proofs/server-original-variable-symbol-query.md
    let backend = test_backend();
    let first = Uri::from_str("file:///opaque-variable-first.tcl").unwrap();
    let second = Uri::from_str("file:///opaque-variable-second.tcl").unwrap();
    let source = r"set ::N::v\uD800 1
set ::N::v\uD801 2
info exists ::N::v\uD800";
    register(&backend, &first, source).await;
    register(&backend, &second, source).await;
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let position = Position::new(2, 14);
    let occurrence = core_definition::original_variable_occurrence_at(
        source,
        &analysis,
        position.line,
        position.character,
    )
    .expect("actual original variable operand");
    let symbol = occurrence.symbol().clone();
    for reference in &mut analysis.qualified_var_refs {
        reference.qualified_name.clear();
    }
    backend
        .workspace_index
        .write()
        .await
        .add_document(first.as_str(), &analysis);
    backend
        .workspace_index
        .write()
        .await
        .add_document(second.as_str(), &analysis);
    let declarations = backend
        .cross_document_variable_definition(&first, source, position, &analysis)
        .await
        .unwrap();
    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].uri, second);
    assert_eq!(declarations[0].range.start.line, 0);
    let references = backend
        .cross_document_variable_references(source, &analysis, position, false)
        .await
        .unwrap();
    assert_eq!(references.len(), 2);
    assert!(
        references
            .iter()
            .all(|location| location.range.start.line == 2)
    );
    let hover = backend
        .cross_document_variable_hover(&first, source, position, &analysis)
        .await
        .unwrap()
        .expect("typed variable geometry renders through its retained policy");
    assert!(hover.value.contains("1 declaration(s) and 1 reference(s)"));
    assert_eq!(
        backend
            .workspace_index
            .read()
            .await
            .original_variable_occurrences(&symbol, "")
            .count(),
        4,
        "opaque sibling keys cannot join through a reporting string"
    );
    let stale = source.replace("exists", "unset ");
    assert!(
        core_definition::original_variable_occurrence_at(
            &stale,
            &analysis,
            position.line,
            position.character,
        )
        .is_none(),
        "changed source cannot issue an original selection"
    );
    assert!(
        backend
            .cross_document_variable_definition(&first, &stale, position, &analysis)
            .await
            .unwrap()
            .is_empty(),
        "a retained original operand cannot fall back to its reporting spelling"
    );
}

#[test]
fn command_head_gate_distinguishes_head_from_argument() {
    let src = "proc greet {x} {}\ngreet arg\n";
    let analysis = Analyser::new().analyse(src, "tcl8.6").clone();
    let index = tcl_lexer::LineIndex::new_lsp(src);
    // `greet` at line 1 is a command head (the call site).
    assert!(position_is_command_head(
        src,
        Position {
            line: 1,
            character: 2
        },
        &analysis,
        &index,
    ));
    // `arg` at line 1 is an argument word, not a command head, so
    // the cross-document fallback must not fire on it.
    assert!(!position_is_command_head(
        src,
        Position {
            line: 1,
            character: 7
        },
        &analysis,
        &index,
    ));
}

#[test]
fn command_head_gate_uses_whole_original_words_and_exact_utf16_positions() {
    let src = "\"greet\" arg\ngr\\u0065et arg\n😀greet arg\nset value [greet arg]\nset data {greet arg}\n";
    let analysis = Analyser::new().analyse(src, "tcl8.6");
    let index = tcl_lexer::LineIndex::new_lsp(src);
    for pos in [
        Position::new(0, 0),
        Position::new(0, 3),
        Position::new(0, 6),
        Position::new(1, 4),
        Position::new(2, 0),
        Position::new(2, 2),
        Position::new(3, 12),
    ] {
        assert!(
            position_is_command_head(src, pos, &analysis, &index),
            "{pos:?}"
        );
    }
    for pos in [
        Position::new(0, 9),
        Position::new(2, 1),
        Position::new(2, 99),
        Position::new(3, 18),
        Position::new(4, 12),
        Position::new(99, 0),
    ] {
        assert!(
            !position_is_command_head(src, pos, &analysis, &index),
            "{pos:?}"
        );
    }
    let multiline = "\"greet\nmore\" arg\n";
    let analysis = Analyser::new().analyse(multiline, "tcl8.6");
    let index = tcl_lexer::LineIndex::new_lsp(multiline);
    assert!(!position_is_command_head(
        multiline,
        Position::new(0, 99),
        &analysis,
        &index
    ));
}

#[test]
fn command_head_gate_withdraws_stale_inputs_and_nonhead_report_rows() {
    let src = "greet arg\n";
    let analysis = Analyser::new().analyse(src, "tcl8.6");
    let index = tcl_lexer::LineIndex::new_lsp(src);
    let pos = Position::new(0, 2);
    assert!(position_is_command_head(src, pos, &analysis, &index));
    assert!(!position_is_command_head(
        "other arg\n",
        pos,
        &analysis,
        &index
    ));
    let foreign_index = tcl_lexer::LineIndex::new_lsp("one\ntwo\n");
    assert!(!position_is_command_head(
        src,
        pos,
        &analysis,
        &foreign_index
    ));
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(!position_is_command_head(src, pos, &missing, &index));
    let mut grammar = analysis.clone();
    grammar.body_lexer_config = None;
    assert!(!position_is_command_head(src, pos, &grammar, &index));
    let mut consumed = analysis.clone();
    for invocation in &mut consumed.command_invocations {
        invocation.lookup =
            tcl_compiler::signature_scan::types::SignatureCommandLookup::DeferredReference;
    }
    assert!(!position_is_command_head(src, pos, &consumed, &index));
    let mut argument_report = analysis;
    for invocation in &mut argument_report.command_invocations {
        invocation.range = tcl_lexer::Span::new(6, 9);
    }
    assert!(!position_is_command_head(
        src,
        Position::new(0, 7),
        &argument_report,
        &index
    ));
}

#[tokio::test]
async fn cross_document_definition_resolves_sibling_proc() {
    let backend = test_backend();
    let lib_uri = Uri::from_str("file:///lib.tcl").unwrap();
    let main_uri = Uri::from_str("file:///main.tcl").unwrap();
    let lib_src = "proc shared_helper {} {}\n";
    let main_src = "shared_helper\n";
    // Register both documents.
    {
        let mut docs = backend.documents.lock("test").await;
        docs.insert(
            lib_uri.clone(),
            DocumentState::new(lib_src.to_owned(), "tcl8.6".to_owned()),
        );
        docs.insert(
            main_uri.clone(),
            DocumentState::new(main_src.to_owned(), "tcl8.6".to_owned()),
        );
    }
    // Index the library document's proc.
    {
        let mut a = Analyser::new();
        let lib_analysis = a.analyse(lib_src, "tcl8.6").clone();
        backend
            .workspace_index
            .write()
            .await
            .add_document(lib_uri.as_str(), &lib_analysis);
    }
    // From main.tcl, jumping on `shared_helper` resolves to
    // lib.tcl's declaration.
    let locs = Box::pin(backend.compute_definition(&main_uri, Position::new(0, 0)))
        .await
        .expect("definition ok");
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].uri, lib_uri);
    // `proc shared_helper` — name token starts at column 5.
    assert_eq!(locs[0].range.start.line, 0);
    assert_eq!(locs[0].range.start.character, 5);
}

#[tokio::test]
async fn cross_document_definition_resolves_wildcard_imported_proc() {
    // TP, cross-document: `lib.tcl` defines and exports `bar`;
    // `main.tcl` wildcard-imports `::Lib::*` and calls `bar` bare.
    // `main.tcl`'s own in-document resolver can't see `::Lib::bar`
    // (a different file's proc), so this exercises the cross-document
    // fallback in `resolve_workspace_symbols`
    // (`WorkspaceIndex::resolve_wildcard_import`). Both documents must
    // be indexed: the `namespace import` itself is recorded in
    // `main.tcl`'s own analysis.
    let backend = test_backend();
    let lib_uri = Uri::from_str("file:///wc_lib.tcl").unwrap();
    let main_uri = Uri::from_str("file:///wc_main.tcl").unwrap();
    let lib_src = "namespace eval ::Lib {\n    proc bar {} {}\n    namespace export bar\n}\n";
    let main_src = "namespace import ::Lib::*\nbar\n";
    {
        let mut docs = backend.documents.lock("test").await;
        docs.insert(
            lib_uri.clone(),
            DocumentState::new(lib_src.to_owned(), "tcl8.6".to_owned()),
        );
        docs.insert(
            main_uri.clone(),
            DocumentState::new(main_src.to_owned(), "tcl8.6".to_owned()),
        );
    }
    {
        let mut index = backend.workspace_index.write().await;
        let mut a = Analyser::new();
        let lib_analysis = a.analyse(lib_src, "tcl8.6").clone();
        index.add_document(lib_uri.as_str(), &lib_analysis);
        let mut a2 = Analyser::new();
        let main_analysis = a2.analyse(main_src, "tcl8.6").clone();
        index.add_document(main_uri.as_str(), &main_analysis);
    }
    // Cursor on the bareword `bar` call (line 1, col 0).
    let locs = Box::pin(backend.compute_definition(&main_uri, Position::new(1, 0)))
        .await
        .expect("definition ok");
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].uri, lib_uri);
    // `proc bar` — name token starts at column 9 on line 1 of lib.tcl.
    assert_eq!(locs[0].range.start.line, 1);
    assert_eq!(locs[0].range.start.character, 9);
}

#[tokio::test]
async fn cross_document_wildcard_import_does_not_resolve_unexported_sibling() {
    // FP guard (CRITICAL, cross-document, server layer): `lib.tcl`'s
    // `other` is never exported, so `main.tcl`'s wildcard import must
    // not resolve a bare `other` call to it — matches real tclsh's own
    // `invalid command name` error there (tclsh9.0/8.6-verified).
    let backend = test_backend();
    let lib_uri = Uri::from_str("file:///wc_lib2.tcl").unwrap();
    let main_uri = Uri::from_str("file:///wc_main2.tcl").unwrap();
    let lib_src = "namespace eval ::Lib {\n    proc bar {} {}\n    proc other {} {}\n    namespace export bar\n}\n";
    let main_src = "namespace import ::Lib::*\nother\n";
    {
        let mut docs = backend.documents.lock("test").await;
        docs.insert(
            lib_uri.clone(),
            DocumentState::new(lib_src.to_owned(), "tcl8.6".to_owned()),
        );
        docs.insert(
            main_uri.clone(),
            DocumentState::new(main_src.to_owned(), "tcl8.6".to_owned()),
        );
    }
    {
        let mut index = backend.workspace_index.write().await;
        let mut a = Analyser::new();
        let lib_analysis = a.analyse(lib_src, "tcl8.6").clone();
        index.add_document(lib_uri.as_str(), &lib_analysis);
        let mut a2 = Analyser::new();
        let main_analysis = a2.analyse(main_src, "tcl8.6").clone();
        index.add_document(main_uri.as_str(), &main_analysis);
    }
    let locs = Box::pin(backend.compute_definition(&main_uri, Position::new(1, 0)))
        .await
        .expect("definition ok");
    assert!(
        locs.is_empty(),
        "an unexported sibling must stay unresolved through the wildcard import: {locs:?}"
    );
}

#[tokio::test]
async fn cross_document_definition_prefers_class_creation_over_define_stub() {
    // `::C` is created in a.tcl and extended by a cross-file `oo::define` in
    // b.tcl.  Go-to-definition on a `C` call jumps to the real creation site
    // (a.tcl), not the extension stub (b.tcl).
    let backend = test_backend();
    let a = Uri::from_str("file:///a.tcl").unwrap();
    let b = Uri::from_str("file:///b.tcl").unwrap();
    register(&backend, &a, "oo::class create C {}\n").await;
    register(&backend, &b, "oo::define C {\n    method foo {} {}\n}\n").await;
    let main_src = "C new\n";
    let main = Uri::from_str("file:///main.tcl").unwrap();
    let analysis = {
        let mut an = Analyser::new();
        an.analyse(main_src, "tcl8.6").clone()
    };
    // Cursor on the `C` call head.
    let defs = backend
        .cross_document_definition(&main, main_src, Position::new(0, 0), &analysis)
        .await
        .expect("ok");
    assert_eq!(defs.len(), 1, "{defs:?}");
    assert_eq!(defs[0].uri, a, "{defs:?}");
}

#[tokio::test]
async fn cross_document_definition_skipped_when_local_match_exists() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///main.tcl").unwrap();
    let src = "proc greet {} {}\ngreet\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new(src.to_owned(), "tcl8.6".to_owned()),
    );
    // Cursor on the `greet` call (line 1) resolves locally;
    // the result points at the same document, line 0.
    let locs = Box::pin(backend.compute_definition(&uri, Position::new(1, 0)))
        .await
        .expect("definition ok");
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].uri, uri);
    assert_eq!(locs[0].range.start.line, 0);
}

#[tokio::test]
async fn cross_document_definition_follows_namespace_path_not_collision() {
    // Go-to-definition on the bare `helper` call in app.tcl jumps to
    // `::mymod::helper` (reached via `namespace path`), never the same-named
    // `::other::helper` — the previous simple-name lookup surfaced both.
    let (backend, mymod, _other, app) = register_namespace_path_workspace().await;
    let app_src =
        "namespace eval ::app {\n    namespace path ::mymod\n    proc run {} { helper }\n}\n";
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(app_src, "tcl8.6").clone()
    };
    // Cursor on the `helper` call in `run`'s body (line 2, col 20).
    let defs = backend
        .cross_document_definition(&app, app_src, Position::new(2, 20), &analysis)
        .await
        .expect("ok");
    assert_eq!(defs.len(), 1, "{defs:?}");
    assert_eq!(defs[0].uri, mymod);
}
