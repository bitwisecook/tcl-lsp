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
use tower_lsp_server::ls_types::{
    PartialResultParams, ReferenceContext, TextDocumentIdentifier, WorkDoneProgressParams,
};

/// What the browser host does for a closed file: `vfs_upsert_bytes`.
fn upload(store: &vfs::MemoryStore, uri: &str, text: &str) {
    use vfs::SourceStore as _;
    let selected = store.document_uri_path(uri).expect(uri);
    let canonical = uri_norm::canonical_uri_string(uri);
    store
        .try_upsert_uri_aliases(
            &[uri, canonical.as_ref()],
            selected.path().to_path_buf(),
            text.as_bytes().to_vec(),
        )
        .expect(uri);
}

async fn open(backend: &Backend, uri: &Uri, src: &str, dialect: &str) {
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new(src.to_owned(), dialect.to_owned()),
    );
    backend
        .db_set_source(uri, src, dialect.to_owned())
        .await
        .expect("the fixture retains its source");
    let analysis = backend
        .analysis_for(uri, Arc::from(src), dialect.to_owned())
        .await
        .expect("the fixture analyses its source");
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &analysis);
}

fn backend_over(store: &Arc<vfs::MemoryStore>) -> Backend {
    let mut backend = tests::test_backend();
    backend.state_mut_for_test().store = store.clone();
    backend
}

#[tokio::test]
async fn cross_file_navigation_keeps_a_virtual_workspace_scheme() {
    for root in ["vscode-vfs://github/owner/repo", "vscode-test-web://mount"] {
        let lib = Uri::from_str(&format!("{root}/lib/crossFileDefLib.tcl")).unwrap();
        let caller = Uri::from_str(&format!("{root}/crossFileCaller.tcl")).unwrap();
        let store = Arc::new(vfs::MemoryStore::new());
        upload(
            &store,
            lib.as_str(),
            "proc libtest {a} {\n    return $a\n}\n",
        );
        let backend = backend_over(&store);
        *backend.workspace_folders.lock().await = vec![Uri::from_str(root).unwrap()];
        backend.scan_workspace_folders().await.unwrap();
        assert!(
            backend
                .workspace_index
                .read()
                .await
                .contains_document(lib.as_str()),
            "the scan indexes the closed file under its own URI: {root}"
        );
        open(
            &backend,
            &caller,
            "set x [libtest 1]\nlibtest 2\n",
            "tcl8.6",
        )
        .await;

        let defs = Box::pin(backend.compute_definition(&caller, Position::new(1, 2)))
            .await
            .expect("definition");
        assert_eq!(defs.len(), 1, "{root}: {defs:?}");
        assert_eq!(defs[0].uri, lib, "{root}: {defs:?}");
        assert_eq!(defs[0].range.start.line, 0, "{root}: {defs:?}");

        let refs = backend
            .references(ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: caller.clone(),
                    },
                    position: Position::new(1, 2),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: ReferenceContext {
                    include_declaration: true,
                },
            })
            .await
            .expect("references")
            .unwrap_or_default();
        assert!(
            refs.iter().any(|l| l.uri == lib),
            "{root}: the declaration keeps the scheme: {refs:?}"
        );
        assert_eq!(
            refs.iter().filter(|l| l.uri == caller).count(),
            2,
            "{root}: {refs:?}"
        );
        assert!(
            refs.iter().all(|l| l.uri.scheme().as_str() != "file"),
            "{root}: {refs:?}"
        );
    }
}

const ILX_EXTENSION: &str = concat!(
    "var f5 = require('f5-nodejs');\n",
    "var ilx = new f5.ILXServer();\n",
    "ilx.addMethod('my_js_function', function (req, res) {\n",
    "    res.reply('ok');\n",
    "});\n",
    "ilx.listen();\n",
);

#[tokio::test]
async fn the_ilx_relation_crosses_a_virtual_workspace_in_both_directions() {
    for root in ["vscode-vfs://github/owner/repo", "vscode-test-web://mount"] {
        let rule = Uri::from_str(&format!("{root}/ilx_ws/rules/rule1.tcl")).unwrap();
        let js = format!("{root}/ilx_ws/extensions/my_extension/index.js");
        let rule_src = "when HTTP_REQUEST {\n    set handle [ILX::init ilx_ws my_extension]\n    set reply [ILX::call $handle my_js_function [HTTP::uri]]\n}\n";
        let store = Arc::new(vfs::MemoryStore::new());
        upload(&store, &js, ILX_EXTENSION);
        upload(&store, rule.as_str(), rule_src);
        let backend = backend_over(&store);
        *backend.workspace_folders.lock().await = vec![Uri::from_str(root).unwrap()];
        open(&backend, &rule, rule_src, IRULES_DIALECT).await;

        let defs = Box::pin(backend.compute_definition(&rule, Position::new(2, 34)))
            .await
            .expect("definition");
        assert_eq!(defs.len(), 1, "{root}: {defs:?}");
        assert_eq!(defs[0].uri.as_str(), js, "{root}: {defs:?}");
        assert_eq!(defs[0].range.start, Position::new(2, 14), "{root}");

        let from_js = backend
            .ilx_references_command(&[serde_json::json!({
                "uri": js,
                "text": ILX_EXTENSION,
                "line": 2,
                "character": 20,
                "includeDeclaration": false,
            })])
            .await
            .expect("the JavaScript end answers");
        let found: Vec<Location> = serde_json::from_value(from_js).unwrap();
        assert_eq!(found.len(), 1, "{root}: {found:?}");
        assert_eq!(found[0].uri, rule, "{root}: {found:?}");
        assert_eq!(found[0].range.start.line, 2, "{root}: {found:?}");
    }
}
