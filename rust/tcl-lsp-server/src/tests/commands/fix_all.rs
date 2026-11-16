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

//! Fix all safe issues.

use super::*;

/// `fixAllSafeIssues` applies a fix for a shown finding and for no other:
/// a `# noqa: W100` over the command, W100 turned off at a layer, and an
/// abstaining document each keep the brace fix from being applied in bulk.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fix_all_safe_issues_applies_only_shown_fixes() {
    let fixed = |out: Option<serde_json::Value>| {
        out.and_then(|v| v["source"].as_str().map(str::to_owned))
            .expect("a result")
    };
    let backend = test_backend();
    let plain = Uri::from_str("file:///fixall-plain.tcl").unwrap();
    register(&backend, &plain, "set n [expr abs(-2)]\n").await;
    let out = backend
        .fix_all_safe_issues_command(&[serde_json::json!(plain.as_str())])
        .await
        .expect("ok");
    assert_eq!(
        fixed(out),
        "set n [expr {abs(-2)}]\n",
        "the control is fixed"
    );

    let marked = Uri::from_str("file:///fixall-noqa.tcl").unwrap();
    let src = "# noqa: W100\nset n [expr abs(-2)]\n";
    register(&backend, &marked, src).await;
    let out = backend
        .fix_all_safe_issues_command(&[serde_json::json!(marked.as_str())])
        .await
        .expect("ok");
    assert_eq!(fixed(out), src, "a silenced W100 applies nothing");

    let disabled = test_backend();
    disabled
        .apply_global_config(&serde_json::json!({ "diagnostics": { "W100": false } }))
        .await
        .unwrap();
    let uri = Uri::from_str("file:///fixall-disabled.tcl").unwrap();
    register(&disabled, &uri, "set n [expr abs(-2)]\n").await;
    let out = disabled
        .fix_all_safe_issues_command(&[serde_json::json!(uri.as_str())])
        .await
        .expect("ok");
    assert_eq!(
        fixed(out),
        "set n [expr abs(-2)]\n",
        "a disabled W100 applies nothing"
    );

    // A UTF-16 byte-order mark ahead of UTF-8 text: the document
    // abstains, so no analyser finding shows and nothing is applied.
    let (text, report) =
        tcl_lsp_core::source_decode::decode_source(b"\xff\xfeset n [expr abs(-2)]\n");
    assert!(report.requires_abstention());
    let abstaining = Uri::from_str("file:///fixall-abstaining.tcl").unwrap();
    let mut doc = DocumentState::new(text.clone(), "tcl8.6".to_owned());
    doc.decode_report = Some(report);
    backend
        .documents
        .lock("test")
        .await
        .insert(abstaining.clone(), doc);
    let out = backend
        .fix_all_safe_issues_command(&[serde_json::json!(abstaining.as_str())])
        .await
        .expect("ok");
    assert_eq!(fixed(out), text, "an abstaining document applies nothing");
    let decoded = Uri::from_str("file:///fixall-decoded.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        decoded.clone(),
        DocumentState::new(text.clone(), "tcl8.6".to_owned()),
    );
    let out = backend
        .fix_all_safe_issues_command(&[serde_json::json!(decoded.as_str())])
        .await
        .expect("ok");
    assert!(
        fixed(out).contains("[expr {abs(-2)}]"),
        "the same text without the byte evidence is fixed"
    );
}

/// The bulk pass analyses under the document's own layers: a code
/// the session turns off and a folder turns back on is computed, shown and
/// fixed in that folder's documents, and in no other.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fix_all_safe_issues_analyses_under_the_documents_layers() {
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({ "diagnostics": { "IRULE2002": false } }))
        .await
        .unwrap();
    *backend.folder_configs.lock().await = vec![(
        Uri::from_str("file:///fixall-folder").unwrap(),
        FolderConfig {
            policy_layers: Some(PolicyLayers {
                project: serde_json::json!({ "diagnostics": { "IRULE2002": true } }),
                ..PolicyLayers::default()
            }),
            ..FolderConfig::default()
        },
    )];
    let src = "when HTTP_REQUEST {\n    log local0. [http_host]\n}\n";
    for (path, expected) in [
        (
            "file:///fixall-folder/a.irul",
            "when HTTP_REQUEST {\n    log local0. [HTTP::host]\n}\n",
        ),
        ("file:///elsewhere/b.irul", src),
    ] {
        let uri = Uri::from_str(path).unwrap();
        backend.documents.lock("test").await.insert(
            uri.clone(),
            DocumentState::new(src.to_owned(), "f5-irules".to_owned()),
        );
        let out = backend
            .fix_all_safe_issues_command(&[serde_json::json!(uri.as_str())])
            .await
            .expect("ok")
            .expect("a result");
        assert_eq!(out["source"].as_str(), Some(expected), "{path}: {out}");
    }
}
