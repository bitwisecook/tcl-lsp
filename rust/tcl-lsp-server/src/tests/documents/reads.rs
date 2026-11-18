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

//! Snapshot reads and the readiness each provider waits for.

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_document_hands_out_analysis_text_and_keeps_the_raw_bytes() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///oldmac-read.tcl").unwrap();
    let src = "set a 1\rset b 2\r\nset c 3\n";
    register(&backend, &uri, src).await;
    let doc = backend
        .read_document(&uri)
        .await
        .unwrap()
        .expect("open document");
    // The provider-facing text has the lone `\r` rewritten…
    assert_eq!(&*doc.text, "set a 1\nset b 2\r\nset c 3\n");
    // …the formatter / W118 view keeps the client's exact bytes…
    assert_eq!(doc.raw(), src);
    // …the rewrite never changes the byte length, so spans are shared…
    assert_eq!(doc.text.len(), doc.raw().len());
    // …and the stored shadow buffer is untouched.
    assert_eq!(
        &*backend
            .documents
            .lock("test")
            .await
            .get(&uri)
            .expect("doc")
            .text,
        src,
    );
    // An LF document is handed out unchanged (and does not allocate).
    let plain = Uri::from_str("file:///plain.tcl").unwrap();
    register(&backend, &plain, "set a 1\n").await;
    let doc = backend
        .read_document(&plain)
        .await
        .unwrap()
        .expect("open document");
    assert_eq!(&*doc.text, "set a 1\n");
    assert!(doc.raw_text.is_none());
}

// Document-snapshot sharing and revision currency.
//
// `read_document` hands every in-flight request its own `DocumentState`.
// Those snapshots must (a) share one allocation of the document text and
// line index rather than copying both per request, and (b) stay pinned to
// the revision they were taken at, so a request that overlaps an edit
// finishes against coherent text *and* index instead of a mixture.

/// A synthetic document whose text and line index are both far larger than
/// a pointer, so "the snapshot shared them" is a meaningful claim rather
/// than a coincidence of small allocations. Deliberately not corpus-sized:
/// `register` analyses it, and the property under test is pointer
/// identity, which does not get truer with more lines.
fn large_document_source() -> String {
    use std::fmt::Write as _;
    (0..300).fold(String::new(), |mut acc, i| {
        let _ = writeln!(acc, "proc p{i} {{a b}} {{ return [expr {{$a + $b}}] }}");
        acc
    })
}

/// **TP** — concurrent reads of one open document share its storage.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_snapshots_share_one_text_and_index_allocation() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///shared.tcl").unwrap();
    let source = large_document_source();
    register(&backend, &uri, &source).await;

    // 32 overlapping readers, the concurrency the issue asks for.
    let mut tasks = Vec::new();
    for _ in 0..32 {
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        tasks.push(crate::rt::spawn(async move {
            backend
                .read_document(&uri)
                .await
                .unwrap()
                .expect("open document")
        }));
    }
    let mut snapshots = Vec::new();
    for t in tasks {
        snapshots.push(t.await.expect("reader task panicked"));
    }

    let owner = {
        let docs = backend.documents.lock("test").await;
        docs.get(&uri).expect("open document").clone()
    };
    for (i, snap) in snapshots.iter().enumerate() {
        assert!(
            Arc::ptr_eq(&owner.text, &snap.text),
            "snapshot {i} copied the document text instead of sharing it"
        );
        assert!(
            owner.line_index.shares_storage_with(&snap.line_index),
            "snapshot {i} copied the line index instead of sharing it"
        );
    }
    // One allocation, held by: the store, `owner`, and the 32 snapshots.
    assert_eq!(Arc::strong_count(&owner.text), 34);
    // …and every snapshot really does describe the whole document, so the
    // sharing assertions above are not passing on an empty buffer.
    assert_eq!(&*snapshots[0].text, source.as_str());
}

/// **FP** — editing while an older snapshot is alive must not mix
/// revisions: the in-flight reader keeps text *and* index from the
/// revision it was given, and they stay consistent with each other.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_edit_never_mutates_a_snapshot_an_in_flight_request_holds() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///revisions.tcl").unwrap();
    register(&backend, &uri, "set alpha 1\nset beta 2\n").await;

    let before = backend
        .read_document(&uri)
        .await
        .unwrap()
        .expect("open document");
    let before_revision = before.revision;

    backend
        .did_change(DidChangeTextDocumentParams {
            text_document: tower_lsp_server::ls_types::VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![tower_lsp_server::ls_types::TextDocumentContentChangeEvent {
                range: Some(Range {
                    start: pos(0, 0),
                    end: pos(0, 0),
                }),
                range_length: None,
                text: "# a new leading comment line\n".to_owned(),
            }],
        })
        .await;

    // The old snapshot is untouched…
    assert_eq!(&*before.text, "set alpha 1\nset beta 2\n");
    assert_eq!(before.revision, before_revision);
    // …and its index still describes *its own* text: line 1 starts at the
    // `set beta` of the pre-edit buffer. A snapshot that had picked up the
    // post-edit index would resolve line 1 to the wrong byte offset, which
    // is precisely the corruption the one-snapshot invariant rules out.
    let line1 = before.line_index.line_start(1) as usize;
    assert_eq!(&before.text[line1..], "set beta 2\n");

    // The live document did advance, on both fields together.
    let after = backend
        .read_document(&uri)
        .await
        .unwrap()
        .expect("open document");
    assert!(after.text.starts_with("# a new leading comment line\n"));
    assert!(after.revision > before_revision);
    let after_line1 = after.line_index.line_start(1) as usize;
    assert_eq!(&after.text[after_line1..], "set alpha 1\nset beta 2\n");
    assert!(!Arc::ptr_eq(&before.text, &after.text));
}

/// **TN** — reading a closed on-disk file yields a snapshot but must not
/// install it in the open-document store, so a one-off cross-file read
/// cannot retain every file it ever touched.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reading_a_closed_on_disk_file_does_not_retain_it() {
    let dir = crate::tests::ScratchDir::at(std::env::temp_dir().join(format!(
        "tcl-lsp-1184-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    )));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("ondisk.tcl");
    std::fs::write(&path, "proc onDisk {} { return 1 }\n").expect("write fixture");

    let backend = test_backend();
    let uri = canonical_file_uri(&path).expect("fixture path has a representable file URI");

    assert!(backend.documents.lock("test").await.is_empty());
    let snap = backend
        .read_document(&uri)
        .await
        .unwrap()
        .expect("disk fallback");
    assert_eq!(&*snap.text, "proc onDisk {} { return 1 }\n");
    assert!(
        backend.documents.lock("test").await.is_empty(),
        "a disk read must not become a permanent open-document entry"
    );
    // Reading again is a fresh read, not a retained snapshot.
    let again = backend
        .read_document(&uri)
        .await
        .unwrap()
        .expect("disk fallback");
    assert!(!Arc::ptr_eq(&snap.text, &again.text));

    std::fs::remove_dir_all(&dir).ok();
}

/// A second publication boundary: a cold large file's Salsa source can be
/// current while `didOpen` keeps publication short of `Indexed` as it builds
/// its workspace-index seed.  Making semantic tokens wait for that unrelated
/// whole-file analysis bypasses their bounded enriched/coarse fast path and
/// turns a first paint into a minutes-long response. They may proceed at
/// Salsa readiness; providers that
/// consume cross-document facts must still wait for indexed readiness.
#[tokio::test]
async fn semantic_tokens_do_not_wait_for_did_open_index_seed_1854() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///semantic-paint-before-index.tcl").unwrap();
    register(&backend, &uri, "proc ready {} {}\n").await;
    {
        let mut docs = backend.documents.lock("test").await;
        let doc = docs.get_mut(&uri).expect("the document is open");
        doc.publication = DocumentPublication::Salsa;
    }

    let token_doc = crate::rt::timeout(std::time::Duration::from_millis(100), async {
        backend
            .read_document_for_semantic_tokens(&uri)
            .await
            .unwrap()
    })
    .await
    .expect("semantic tokens only wait for the current Salsa source")
    .expect("the live token document exists");
    assert_eq!(token_doc.text.as_ref(), "proc ready {} {}\n");

    assert!(
        crate::rt::timeout(
            std::time::Duration::from_millis(25),
            backend.read_document(&uri),
        )
        .await
        .is_err(),
        "workspace-aware providers must wait for the live index seed",
    );

    backend
        .documents
        .lock("test")
        .await
        .get_mut(&uri)
        .expect("the document remains open")
        .publication = DocumentPublication::Indexed;
    backend.live_publication_advanced.notify_waiters();
    crate::rt::timeout(std::time::Duration::from_millis(100), async {
        backend.read_document(&uri).await.unwrap()
    })
    .await
    .expect("indexed readiness releases ordinary providers")
    .expect("the live indexed document exists");
}

async fn public_navigation_locations(
    backend: &Backend,
    uri: &Uri,
    references: bool,
) -> Vec<Location> {
    let mut params = serde_json::json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 2 },
    });
    if references {
        params["context"] = serde_json::json!({ "includeDeclaration": true });
        backend
            .references(serde_json::from_value(params).unwrap())
            .await
            .expect("the public references request succeeds")
            .expect("the call has references")
    } else {
        let definition = backend
            .goto_definition(serde_json::from_value(params).unwrap())
            .await
            .expect("the public definition request succeeds")
            .expect("the call has a definition");
        let GotoDefinitionResponse::Array(locations) = definition else {
            panic!("the public definition handler returns locations");
        };
        locations
    }
}

#[tokio::test]
async fn direct_diagnostic_waits_for_live_index_before_capture() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///direct-diagnostic-before-index.tcl").unwrap();
    let source = "proc ready {} {return 42}\nready\n";
    register(&backend, &uri, source).await;
    backend
        .documents
        .lock("test")
        .await
        .get_mut(&uri)
        .unwrap()
        .publication = DocumentPublication::Salsa;
    backend
        .workspace_index
        .write()
        .await
        .remove_document(uri.as_str());
    let mut request = Box::pin(backend.diagnostic(doc_diag_params(&uri, None)));
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(25), &mut request)
            .await
            .is_err(),
        "the same direct request must join its live index publication",
    );
    assert!(backend.rehoming_gate.try_lock().is_ok());
    assert!(backend.original_source_view_build_gate.try_lock().is_ok());
    crate::rt::timeout(
        std::time::Duration::from_secs(5),
        register(&backend, &uri, source),
    )
    .await
    .expect("the waiting request must leave publication unblocked");
    backend.live_publication_advanced.notify_waiters();
    let report = crate::rt::timeout(std::time::Duration::from_secs(5), request)
        .await
        .expect("index publication releases the same direct request")
        .expect("the current provider computes its actual report");
    let report = serde_json::to_value(report).unwrap();
    assert_eq!(report["kind"], "full");
    assert!(report["items"].is_array());
    let id = report["resultId"].as_str().unwrap();
    assert_ne!(id, "");
    let current = backend
        .diagnostic(doc_diag_params(&uri, Some(id)))
        .await
        .unwrap();
    let current = serde_json::to_value(current).unwrap();
    assert_eq!(current["kind"], "unchanged");
    assert_eq!(current["resultId"], id);
}

#[tokio::test]
async fn public_navigation_waits_for_live_index_before_source_view_routing() {
    for references in [false, true] {
        let backend = test_backend();
        let uri = Uri::from_str("file:///navigation-before-index.tcl").unwrap();
        let source = "proc ready {} {}\nready\n";
        register(&backend, &uri, source).await;
        backend
            .documents
            .lock("test")
            .await
            .get_mut(&uri)
            .expect("the document is open")
            .publication = DocumentPublication::Salsa;
        backend
            .workspace_index
            .write()
            .await
            .remove_document(uri.as_str());

        let mut navigation = Box::pin(public_navigation_locations(&backend, &uri, references));
        assert!(
            crate::rt::timeout(std::time::Duration::from_millis(25), &mut navigation)
                .await
                .is_err(),
            "public navigation must wait for its live index seed",
        );
        assert!(
            backend.original_source_view_build_gate.try_lock().is_ok(),
            "the pending request must leave the source-view gate available",
        );
        assert!(
            backend.rehoming_gate.try_lock().is_ok(),
            "the pending request must leave the publication gate available",
        );

        register(&backend, &uri, source).await;
        backend.live_publication_advanced.notify_waiters();
        let mut locations = crate::rt::timeout(std::time::Duration::from_secs(5), navigation)
            .await
            .expect("index publication releases the same public request");
        locations
            .sort_by_key(|location| (location.range.start.line, location.range.start.character));
        let declaration = Location::new(
            uri.clone(),
            Range::new(Position::new(0, 5), Position::new(0, 10)),
        );
        let expected = if references {
            vec![
                declaration,
                Location::new(
                    uri.clone(),
                    Range::new(Position::new(1, 0), Position::new(1, 5)),
                ),
            ]
        } else {
            vec![declaration]
        };
        assert_eq!(locations, expected);
    }
}

async fn assert_local_commands_use_salsa_readiness(backend: &Backend, uri: &Uri) {
    let uri_arg = serde_json::Value::String(uri.to_string());
    crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.minify_document_command(std::slice::from_ref(&uri_arg)),
    )
    .await
    .expect("minify must not wait for the workspace-index seed")
    .expect("minify must succeed")
    .expect("minify must return a result");
    crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.tk_preview_command(&[serde_json::json!({ "uri": uri.as_str() })]),
    )
    .await
    .expect("Tk preview must not wait for the workspace-index seed")
    .expect("Tk preview must succeed")
    .expect("Tk preview must return a result");
    crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.optimise_document_command(std::slice::from_ref(&uri_arg)),
    )
    .await
    .expect("optimise must not wait for the workspace-index seed")
    .expect("optimise must succeed")
    .expect("optimise must return a result");
    crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.fix_all_safe_issues_command(std::slice::from_ref(&uri_arg)),
    )
    .await
    .expect("fix-all must not wait for the workspace-index seed")
    .expect("fix-all must succeed")
    .expect("fix-all must return a result");
}

/// Document-local providers read the live Salsa
/// source, not the independently seeded workspace index. A cold file can
/// remain at `Salsa` while its whole-file index analysis runs; formatting,
/// folding, outlines, linked editing, and selection ranges remain prompt.
#[tokio::test]
async fn local_providers_do_not_wait_for_did_open_index_seed_1854() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///format-before-index-1854.tcl").unwrap();
    let source = "proc ready {} { ready }\n";
    register(&backend, &uri, source).await;
    {
        let mut docs = backend.documents.lock("test").await;
        docs.get_mut(&uri)
            .expect("the document is open")
            .publication = DocumentPublication::Salsa;
    }

    let local = crate::rt::timeout(std::time::Duration::from_secs(5), async {
        backend.read_local_document(&uri).await.unwrap()
    })
    .await
    .expect("a local provider only waits for the matching Salsa source")
    .expect("the live local document exists");
    assert_eq!(local.text.as_ref(), source);

    let params: DocumentFormattingParams = serde_json::from_value(serde_json::json!({
        "textDocument": { "uri": uri.as_str() },
        "options": { "tabSize": 4, "insertSpaces": true }
    }))
    .expect("valid formatting request");
    crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.formatting(params),
    )
    .await
    .expect("formatting must not wait for the workspace-index seed")
    .expect("formatting must succeed");

    let folding: FoldingRangeParams = serde_json::from_value(serde_json::json!({
        "textDocument": { "uri": uri.as_str() }
    }))
    .expect("valid folding request");
    crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.folding_range(folding),
    )
    .await
    .expect("folding must not wait for the workspace-index seed")
    .expect("folding must succeed");

    let symbols: DocumentSymbolParams = serde_json::from_value(serde_json::json!({
        "textDocument": { "uri": uri.as_str() }
    }))
    .expect("valid document-symbol request");
    crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.document_symbol(symbols),
    )
    .await
    .expect("document symbols must not wait for the workspace-index seed")
    .expect("document symbols must succeed");

    let linked: LinkedEditingRangeParams = serde_json::from_value(serde_json::json!({
        "textDocument": { "uri": uri.as_str() },
        "position": { "line": 0, "character": 5 }
    }))
    .expect("valid linked-editing request");
    crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.linked_editing_range(linked),
    )
    .await
    .expect("linked editing must not wait for the workspace-index seed")
    .expect("linked editing must succeed");

    let selection: SelectionRangeParams = serde_json::from_value(serde_json::json!({
        "textDocument": { "uri": uri.as_str() },
        "positions": [{ "line": 0, "character": 5 }]
    }))
    .expect("valid selection-range request");
    crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.selection_range(selection),
    )
    .await
    .expect("selection ranges must not wait for the workspace-index seed")
    .expect("selection ranges must succeed");

    assert_local_commands_use_salsa_readiness(&backend, &uri).await;

    assert!(
        crate::rt::timeout(
            std::time::Duration::from_millis(25),
            backend.read_document(&uri),
        )
        .await
        .is_err(),
        "the test must hold the independent workspace-index boundary pending",
    );
}

#[tokio::test]
async fn repeated_analysis_normalisation_preserves_original_source_receipts() {
    let mut backend = test_backend();
    let store = Arc::new(vfs::MemoryStore::new());
    let uri = Uri::from_str("file:///normalisation-idempotence.tcl").unwrap();
    let source = b"# note\rproc current {} {}\r";
    store.upsert_uri(
        uri.as_str(),
        "/normalisation-idempotence.tcl",
        source.to_vec(),
    );
    backend.state_mut_for_test().store = store;
    let once = backend
        .closed_workspace_symbol_source(&uri)
        .await
        .unwrap()
        .unwrap();
    let twice = once.clone().normalised_for_analysis();
    assert_eq!(once.text.as_ref(), "# note\nproc current {} {}\n");
    assert_eq!(twice.text, once.text);
    assert_eq!(twice.raw(), std::str::from_utf8(source).unwrap());
    assert_eq!(twice.raw(), once.raw());
    assert_eq!(twice.decode_report, once.decode_report);
    assert!(twice.decode_report.is_some());
    assert!(Arc::ptr_eq(
        twice.saved_source_bytes.as_ref().unwrap(),
        once.saved_source_bytes.as_ref().unwrap()
    ));
    assert_eq!(twice.saved_source_bytes.as_deref(), Some(source.as_slice()));
    assert!(Arc::ptr_eq(
        twice.path_admission.as_ref().unwrap(),
        once.path_admission.as_ref().unwrap()
    ));
}
