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

async fn register_source(backend: &Backend, uri: &Uri, source: &str) {
    backend
        .documents
        .lock("native_definition_snapshot_fixture")
        .await
        .insert(
            uri.clone(),
            DocumentState::new(source.to_owned(), "tcl8.6".to_owned()),
        );
    backend
        .db_set_source(uri, source, "tcl8.6".to_owned())
        .await
        .unwrap();
    let analysis = backend
        .analysis_for(uri, Arc::from(source), "tcl8.6".to_owned())
        .await
        .unwrap();
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &analysis);
}

#[tokio::test]
async fn original_cross_document_declaration_keeps_caller_revision_after_remote_analysis() {
    let backend = crate::tests::test_backend();
    let provider: Uri = "file:///native-definition-provider.tcl".parse().unwrap();
    let caller: Uri = "file:///native-definition-caller.tcl".parse().unwrap();
    register_source(&backend, &provider, "proc retained_target {} {}\n").await;
    register_source(&backend, &caller, "retained_target\n").await;
    let epoch = backend.diag_inputs_epoch();
    let generation = backend.live_publication_uri_generation(&caller);
    let document = backend.read_document(&caller).await.unwrap().unwrap();
    let analysis = backend
        .analysis_for(&caller, document.text.clone(), document.dialect.clone())
        .await
        .unwrap();
    let locations = backend
        .original_definition_locations_for_snapshot(
            (caller.clone(), document.clone(), generation),
            &analysis,
            Position::new(0, 0),
            epoch,
        )
        .await
        .unwrap();
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].uri, provider);
    {
        let mut documents = backend
            .documents
            .lock("native_definition_snapshot_bump")
            .await;
        let current = documents.get_mut(&caller).unwrap();
        current.bump_analysis_generation();
        current.publication = DocumentPublication::Indexed;
        assert_eq!(current.text, document.text);
        assert_eq!(current.version, document.version);
    }
    assert_eq!(
        backend
            .original_definition_locations_for_snapshot(
                (caller, document, generation),
                &analysis,
                Position::new(0, 0),
                epoch,
            )
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Location; 0]
    );
}
