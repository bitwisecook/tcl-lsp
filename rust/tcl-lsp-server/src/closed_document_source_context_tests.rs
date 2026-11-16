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

use super::*;

#[tokio::test]
async fn closed_read_uses_detected_configured_and_document_override_dialect() {
    let store = Arc::new(vfs::MemoryStore::new());
    let uri = Uri::from_str("file:///workspace/closed.tcl").unwrap();
    store.upsert_uri(
        uri.as_str(),
        "/mounted/closed.tcl",
        b"# tcl-dialect: tcl8.4\rset x 1\r".to_vec(),
    );
    let mut backend = tests::test_backend();
    backend.state_mut_for_test().store = store.clone();
    *backend.session_dialect_override.lock().await = Some("tcl9.0".to_owned());
    *backend.folder_dialects.lock().await = vec![(
        Uri::from_str("file:///workspace/").unwrap(),
        "tcl8.5".to_owned(),
    )];
    let extension_uri = Uri::from_str("file:///workspace/plain.exp").unwrap();
    store.upsert_uri(
        extension_uri.as_str(),
        "/mounted/plain.data",
        b"set x 1\n".to_vec(),
    );
    assert_eq!(
        backend
            .read_document(&extension_uri)
            .await
            .unwrap()
            .unwrap()
            .dialect,
        "expect"
    );
    let detected = backend.read_document(&uri).await.unwrap().unwrap();
    assert_eq!(detected.dialect, "tcl8.4");
    assert_eq!(detected.raw(), "# tcl-dialect: tcl8.4\rset x 1\r");
    assert_eq!(detected.text.as_ref(), "# tcl-dialect: tcl8.4\nset x 1\n");
    store.upsert("/mounted/closed.tcl", b"set x 1\n".to_vec());
    assert_eq!(
        backend.read_document(&uri).await.unwrap().unwrap().dialect,
        "tcl8.5"
    );
    backend.folder_dialects.lock().await.clear();
    assert_eq!(
        backend.read_document(&uri).await.unwrap().unwrap().dialect,
        "tcl9.0"
    );
    backend
        .document_dialect_overrides
        .lock()
        .await
        .insert(uri.as_str().to_owned(), "tcl8.6".to_owned());
    store.upsert(
        "/mounted/closed.tcl",
        b"# tcl-dialect: tcl8.4\nset x 1\n".to_vec(),
    );
    assert_eq!(
        backend.read_document(&uri).await.unwrap().unwrap().dialect,
        "tcl8.6"
    );
}

#[tokio::test]
async fn closed_read_retains_actual_decoder_report_and_pre_normalised_text() {
    let store = Arc::new(vfs::MemoryStore::new());
    let uri = Uri::from_str("file:///decode.tcl").unwrap();
    let mut bytes = "# 😀\rputs \u{fffd}\rputs ".as_bytes().to_vec();
    bytes.extend_from_slice(b"\xff\r");
    let (decoded, report) = tcl_lsp_core::source_decode::decode_source(&bytes);
    store.upsert_uri(uri.as_str(), "/actual/decode.tcl", bytes.clone());
    let mut backend = tests::test_backend();
    backend.state_mut_for_test().store = store;
    for readiness in [DocumentReadiness::Indexed, DocumentReadiness::Salsa] {
        let document = backend
            .read_document_at(&uri, readiness)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(document.raw(), decoded);
        assert_eq!(
            document.text.as_ref(),
            tcl_lexer::normalise_lone_cr(&decoded)
        );
        assert_eq!(document.decode_report, Some(report));
        assert_eq!(report.error_count, 1);
        assert!(!report.is_faithful());
        assert!(
            report.first_error.unwrap().replacement_text_offset > decoded.find('\u{fffd}').unwrap()
        );
        let at = u32::try_from(document.text.find("puts").unwrap()).unwrap();
        let position = document.line_index.position_at_utf16(at, &document.text);
        assert_eq!((position.line, position.character.get()), (1, 0));
    }
    let document = backend
        .read_document_for_semantic_tokens(&uri)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(document.decode_report, Some(report));
    let symbols_document = backend
        .closed_workspace_symbol_source(&uri)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(symbols_document.raw(), decoded);
    assert_eq!(symbols_document.text, document.text);
    assert_eq!(symbols_document.decode_report, Some(report));
    assert_eq!(
        symbols_document.saved_source_bytes.as_deref(),
        Some(bytes.as_slice())
    );
    let non_text_uri = Uri::from_str("file:///utf16.tcl").unwrap();
    let store = Arc::new(vfs::MemoryStore::new());
    store.upsert_uri(
        non_text_uri.as_str(),
        "/actual/utf16.tcl",
        vec![0xff, 0xfe, b'x', 0],
    );
    backend.state_mut_for_test().store = store;
    let document = backend
        .read_local_document(&non_text_uri)
        .await
        .unwrap()
        .unwrap();
    assert!(tcl_lsp_core::source_decode::should_abstain(
        document.decode_report.as_ref()
    ));
}

#[tokio::test]
async fn closed_selected_source_and_dialect_reach_real_fresh_analysis_input() {
    let store = Arc::new(vfs::MemoryStore::new());
    let uri = Uri::from_str("file:///client/app.tcl").unwrap();
    let source = "# tcl-dialect: tcl8.4\nproc p {} {return 1}\np\n";
    store.upsert_uri(
        uri.as_str(),
        "/mounted/app.data",
        source.as_bytes().to_vec(),
    );
    store.upsert("/client/app.tcl", b"# tcl-dialect: tcl9.0\n".to_vec());
    let mut backend = tests::test_backend();
    backend.state_mut_for_test().store = store;
    let document = backend.read_document(&uri).await.unwrap().unwrap();
    assert_eq!(document.text.as_ref(), source);
    assert_eq!(document.dialect, "tcl8.4");
    assert_eq!(
        document.path_admission.as_ref().unwrap().path(),
        Some(Path::new("/mounted/app.data"))
    );
    let seed = backend
        .fresh_analysis_seed(&uri, Arc::clone(&document.text), document.dialect.clone())
        .await
        .unwrap();
    assert!(std::ptr::eq(
        seed.analysis.resolved_profile().unwrap(),
        tcl_lsp_core::profile_for_dialect("tcl8.4")
    ));
    assert!(tcl_compiler::source_graph::current_analysis(&document.text, &seed.analysis).is_some());
    assert!(
        tcl_compiler::source_graph::current_analysis("set different 1\n", &seed.analysis).is_none()
    );
    backend
        .document_dialect_overrides
        .lock()
        .await
        .insert(uri.as_str().to_owned(), "tcl9.0".to_owned());
    let changed = backend.read_document(&uri).await.unwrap().unwrap();
    let changed_seed = backend
        .fresh_analysis_seed(&uri, Arc::clone(&changed.text), changed.dialect)
        .await
        .unwrap();
    assert!(std::ptr::eq(
        changed_seed.analysis.resolved_profile().unwrap(),
        tcl_lsp_core::profile_for_dialect("tcl9.0")
    ));
    assert_ne!(
        seed.analysis.resolved_input,
        changed_seed.analysis.resolved_input
    );
}

#[tokio::test]
async fn closed_source_read_does_not_renew_retired_provider_mapping() {
    let store = Arc::new(vfs::MemoryStore::new());
    let uri = Uri::from_str("file:///client/same.tcl").unwrap();
    let source = b"# tcl-dialect: tcl8.4\nset x 1\n".to_vec();
    store.upsert_uri(uri.as_str(), "/mounted/same.tcl", source.clone());
    let mut backend = tests::test_backend();
    backend.state_mut_for_test().store = store.clone();
    let original = backend.read_document(&uri).await.unwrap().unwrap();
    assert!(store.remove_uri(uri.as_str()));
    store.upsert_uri(uri.as_str(), "/mounted/same.tcl", source);
    assert!(
        original
            .path_admission
            .as_ref()
            .unwrap()
            .check_for(&uri, store.as_ref())
            .is_err()
    );
    let current = backend.read_document(&uri).await.unwrap().unwrap();
    assert_eq!(original.text, current.text);
    assert_eq!(original.dialect, current.dialect);
    assert_ne!(original.path_admission, current.path_admission);
}

#[tokio::test]
async fn closed_context_does_not_replace_live_editor_source_or_language_choice() {
    let store = Arc::new(vfs::MemoryStore::new());
    let uri = Uri::from_str("file:///live.tcl").unwrap();
    store.upsert_uri(
        uri.as_str(),
        "/mounted/live.tcl",
        b"# tcl-dialect: tcl8.4\n".to_vec(),
    );
    let mut backend = tests::test_backend();
    backend.state_mut_for_test().store = store;
    let admission = backend.document_path_admission(&uri).await.unwrap();
    backend.documents.lock("closed_source_test").await.insert(
        uri.clone(),
        DocumentState::with_version("set live 1\r".to_owned(), "tcl9.0".to_owned(), 17)
            .with_path_admission(admission)
            .with_language_id("tcl9.0".to_owned()),
    );
    let document = backend.read_document(&uri).await.unwrap().unwrap();
    assert_eq!(document.raw(), "set live 1\r");
    assert_eq!(document.text.as_ref(), "set live 1\n");
    assert_eq!(document.dialect, "tcl9.0");
    assert_eq!(document.language_id, "tcl9.0");
    assert_eq!(document.version, Some(17));
    assert!(document.decode_report.is_none());
}
