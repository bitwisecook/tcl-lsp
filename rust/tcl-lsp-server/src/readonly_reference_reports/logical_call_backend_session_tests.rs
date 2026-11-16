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
use crate::DocumentState;
use tower_lsp_server::LanguageServer;

async fn prepare(
    backend: &Backend,
    uri: &Uri,
    source: &str,
) -> tower_lsp_server::ls_types::CallHierarchyItem {
    backend
        .documents
        .lock("logical call Backend session test")
        .await
        .insert(
            uri.clone(),
            DocumentState::new(source.to_owned(), "tcl".to_owned()),
        );
    backend
        .db_set_source(uri, source, "tcl".to_owned())
        .await
        .unwrap();
    let analysis = backend
        .analysis_for(uri, Arc::from(source), "tcl".to_owned())
        .await
        .unwrap();
    backend
        .workspace_index
        .write()
        .await
        .replace_document(uri.as_str(), &analysis);
    let params = serde_json::from_value(
        serde_json::json!({"textDocument":{"uri":uri},"position":{"line":0,"character":5}}),
    )
    .unwrap();
    let mut items = backend
        .prepare_call_hierarchy(params)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(items.len(), 1);
    items.remove(0)
}

#[tokio::test]
async fn fresh_backend_rejects_prior_session_call_on_both_real_endpoints() {
    let first = crate::tests::test_backend();
    let second = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/logical-call-same-session-source.tcl"
        .parse()
        .unwrap();
    let source = "proc caller {} { helper }\nproc helper {} {}\ncaller\n";
    let old = prepare(&first, &uri, source).await;
    let current = prepare(&second, &uri, source).await;
    let old_wire = &old.data.as_ref().unwrap()["logicalCallHierarchy"];
    let current_wire = &current.data.as_ref().unwrap()["logicalCallHierarchy"];
    assert_eq!(old_wire["token"], current_wire["token"]);
    assert_eq!(old_wire["schema"], 3);
    assert_ne!(old_wire["session"], current_wire["session"]);
    // Each backend has a genuine fresh source/publication row and actual
    // incoming/outgoing positives before replaying the old wire on B.
    for (backend, item) in [(&first, &old), (&second, &current)] {
        let params = serde_json::from_value(serde_json::json!({"item":item})).unwrap();
        assert_eq!(
            Box::pin(backend.incoming_calls(params))
                .await
                .unwrap()
                .unwrap()
                .len(),
            1
        );
        let params = serde_json::from_value(serde_json::json!({"item":item})).unwrap();
        assert_eq!(
            Box::pin(backend.outgoing_calls(params))
                .await
                .unwrap()
                .unwrap()
                .len(),
            1
        );
    }
    let params = serde_json::from_value(serde_json::json!({"item":old})).unwrap();
    assert!(
        Box::pin(second.incoming_calls(params))
            .await
            .unwrap()
            .is_none()
    );
    let params = serde_json::from_value(serde_json::json!({"item":old})).unwrap();
    assert!(
        Box::pin(second.outgoing_calls(params))
            .await
            .unwrap()
            .is_none()
    );
}
