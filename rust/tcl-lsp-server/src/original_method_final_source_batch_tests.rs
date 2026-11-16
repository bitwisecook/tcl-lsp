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
async fn actual_native_member_candidate_cannot_renew_a_dependency_revision() {
    let backend = tests::test_backend();
    let uri: Uri = "file:///native-member-final-source.tcl".parse().unwrap();
    let source = "oo::class create C {method member {} {return ok}}";
    backend
        .documents
        .lock("native member fixture")
        .await
        .insert(
            uri.clone(),
            DocumentState::new(source.to_owned(), "tcl8.6".to_owned()),
        );
    backend
        .db_set_source(&uri, source, "tcl8.6".to_owned())
        .await
        .unwrap();
    let analysis = backend
        .analysis_for(&uri, Arc::from(source), "tcl8.6".to_owned())
        .await
        .unwrap();
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &analysis);
    let document = backend.read_document(&uri).await.unwrap().unwrap();
    let generation = backend.live_publication_uri_generation(&uri);
    let caller = (uri.clone(), document, generation);
    let position = Position::new(0, u32::try_from(source.find("member").unwrap()).unwrap());
    let epoch = backend.diag_inputs_epoch();
    let first = Box::pin(backend.original_method_locations_for_snapshot(
        caller.clone(),
        &analysis,
        position,
        epoch,
    ))
    .await
    .unwrap();
    assert!(matches!(first, std::ops::ControlFlow::Break(ref locations) if locations.len() == 1));
    let std::ops::ControlFlow::Break(Some(target)) = backend
        .original_method_source_target(&caller, &analysis, position)
        .await
        .unwrap()
    else {
        panic!("actual original Native declaration source candidate")
    };
    backend
        .documents
        .lock("same source dependency revision")
        .await
        .get_mut(&uri)
        .unwrap()
        .bump_analysis_generation();
    let stale = backend
        .resolve_current_declaration_locations(caller, vec![target], epoch)
        .await
        .unwrap();
    assert_eq!(stale, [] as [tower_lsp_server::ls_types::Location; 0]);
}
