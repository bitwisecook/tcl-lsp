// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

async fn open(backend: &Backend, uri: &Uri, source: &str, version: i32) {
    let admission = backend.document_path_admission(uri).await.unwrap();
    backend
        .documents
        .lock("source_context_fixture")
        .await
        .insert(
            uri.clone(),
            DocumentState::with_version(source.to_owned(), "tcl8.6".into(), version)
                .with_path_admission(admission),
        );
    backend
        .db_set_source(uri, source, "tcl8.6".into())
        .await
        .unwrap();
}

#[tokio::test]
async fn source_context_retains_actual_sync_version_and_distinct_raw_analysis_images() {
    let backend = crate::tests::test_backend();
    let uri = Uri::from_str("file:///actual-sync-context.tcl").unwrap();
    let raw = "puts café\rputs next\n";
    open(&backend, &uri, raw, 17).await;
    let context = backend
        .capture_document_source_context_command(&[
            serde_json::json!(uri.as_str()),
            serde_json::json!(17),
            serde_json::json!(raw),
        ])
        .await
        .unwrap()
        .unwrap();
    assert!(
        backend
            .capture_document_source_context_command(&[
                serde_json::json!(uri.as_str()),
                serde_json::json!(16),
                serde_json::json!(raw),
            ])
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(context["version"], 17);
    assert_eq!(context["source"], raw);
    assert_eq!(context["analysisSource"], "puts café\nputs next\n");
    assert!(
        backend
            .current_document_source_context(Some(&context))
            .await
            .unwrap()
    );
    let mut attempted_edit = context.clone();
    attempted_edit["replacement"] = serde_json::json!("puts changed");
    assert_eq!(
        backend
            .validate_document_transform_command(&[attempted_edit])
            .await
            .unwrap(),
        Some(serde_json::Value::Bool(false))
    );
    let foreign = crate::tests::test_backend();
    open(&foreign, &uri, raw, 17).await;
    assert!(
        !foreign
            .current_document_source_context(Some(&context))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn source_context_refuses_configuration_change_reopen_and_unpublished_input() {
    let backend = crate::tests::test_backend();
    let uri = Uri::from_str("file:///actual-sync-context-currentness.tcl").unwrap();
    let raw = "puts value\n";
    open(&backend, &uri, raw, 4).await;
    let context = backend
        .capture_document_source_context_command(&[
            serde_json::json!(uri.as_str()),
            serde_json::json!(4),
            serde_json::json!(raw),
        ])
        .await
        .unwrap()
        .unwrap();
    {
        use salsa::Setter as _;
        let configuration = backend.resolved_db_config(&uri).await;
        configuration
            .set_targets(&mut *backend.db.lock().await)
            .to(vec![("tcl".into(), "8.4-9.0".into())]);
    }
    assert!(
        !backend
            .current_document_source_context(Some(&context))
            .await
            .unwrap()
    );
    open(&backend, &uri, raw, 5).await;
    assert!(
        !backend
            .current_document_source_context(Some(&context))
            .await
            .unwrap()
    );
    assert!(
        backend
            .capture_document_source_context_command(&[
                serde_json::json!(uri.as_str()),
                serde_json::json!(5),
                serde_json::json!("different input"),
            ])
            .await
            .unwrap()
            .is_none()
    );
}
