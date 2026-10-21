// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Explicit, current source-bound package declaration authoring.
// This file is a split-out part of its parent module: it, and the test
// modules nested under it, share the parent's whole import set.
#[allow(clippy::wildcard_imports)]
use super::*;
impl Backend {
    pub(super) async fn insert_package_require_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(request) = document_transform::TransformRequest::parse(args.first()) else {
            return Ok(None);
        };
        let Some(package) = args.get(1).and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let uri = request.uri().clone();
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(None);
        };
        let Some(source) = self.capture_transform_source(&request, &doc).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, Arc::clone(&doc.text), doc.dialect.clone())
            .await?;
        let Some(worker) = document_transform::TransformWorkerSource::for_document(
            &doc.text,
            &analysis,
            Some(&source),
        ) else {
            return Ok(None);
        };
        let Some(proposal) =
            core_code_actions::package_require_source(&doc.text, &analysis, package)
        else {
            return Ok(None);
        };
        self.finish_document_transform(Some(source), vec![worker], serde_json::json!({ "source": proposal.source, "alreadyRequired": proposal.already_required })).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn open(backend: &Backend, uri: &Uri, source: &str, version: i32) {
        let admission = backend.document_path_admission(uri).await.unwrap();
        backend
            .documents
            .lock("package_authoring_fixture")
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

    async fn validate(backend: &Backend, result: &serde_json::Value) -> bool {
        let mut receipt = result["editReceipt"].clone();
        receipt["replacement"] = result["source"].clone();
        backend
            .validate_document_transform_command(&[receipt])
            .await
            .unwrap()
            == Some(serde_json::Value::Bool(true))
    }

    #[tokio::test]
    async fn package_authoring_ledger_keeps_actual_result_issuer_and_source_revision() {
        let first = crate::tests::test_backend();
        let second = crate::tests::test_backend();
        let uri = Uri::from_str("file:///package-authoring-current.tcl").unwrap();
        let source = "#! interpreter 😀\nputs café\n";
        open(&first, &uri, source, 4).await;
        open(&second, &uri, source, 4).await;
        let arguments = [
            serde_json::json!({"uri":uri.as_str(),"version":4,"source":source}),
            serde_json::json!("café"),
        ];
        let one = first
            .insert_package_require_command(&arguments)
            .await
            .unwrap()
            .unwrap();
        let two = second
            .insert_package_require_command(&arguments)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            one["source"],
            "#! interpreter 😀\npackage require café\nputs café\n"
        );
        assert!(validate(&first, &one).await);
        assert!(validate(&second, &two).await);
        assert!(!validate(&first, &two).await);
        assert!(!validate(&second, &one).await);
        let mut forged = one.clone();
        forged["source"] = serde_json::json!("package require different\n");
        assert!(!validate(&first, &forged).await);
        open(&first, &uri, source, 4).await;
        assert!(!validate(&first, &one).await);
    }

    #[tokio::test]
    async fn package_authoring_requires_snapshot_and_preserves_unicode_value() {
        let backend = crate::tests::test_backend();
        let uri = Uri::from_str("file:///package-authoring-refusal.tcl").unwrap();
        let source = "package require {a b}\nputs done\n";
        open(&backend, &uri, source, 8).await;
        let snapshot = serde_json::json!({"uri":uri.as_str(),"version":8,"source":source});
        let duplicate = backend
            .insert_package_require_command(&[snapshot.clone(), serde_json::json!("a b")])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(duplicate["source"], source);
        assert_eq!(duplicate["alreadyRequired"], true);
        assert!(validate(&backend, &duplicate).await);
        let unicode = backend
            .insert_package_require_command(&[snapshot, serde_json::json!("café")])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            unicode["source"],
            "package require {a b}\npackage require café\nputs done\n"
        );
        assert!(validate(&backend, &unicode).await);
        assert!(
            backend
                .insert_package_require_command(&[
                    serde_json::json!(uri.as_str()),
                    serde_json::json!("owned")
                ])
                .await
                .unwrap()
                .is_none()
        );
        let stale = serde_json::json!({"uri":uri.as_str(),"version":7,"source":source});
        assert!(
            backend
                .insert_package_require_command(&[stale, serde_json::json!("owned")])
                .await
                .is_err()
        );
    }
}
