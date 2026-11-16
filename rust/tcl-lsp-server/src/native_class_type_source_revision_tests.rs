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
async fn same_text_dependency_revision_cannot_retag_the_old_class_snapshot() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///class-type-revision.tcl".parse().unwrap();
    let mut document = DocumentState::new("oo::class create C {}".to_owned(), "tcl8.6".to_owned());
    document.publication = DocumentPublication::Indexed;
    backend
        .documents
        .lock("class_type_revision_fixture")
        .await
        .insert(uri.clone(), document.clone());
    let generation = backend.live_publication_uri_generation(&uri);
    assert!(
        backend
            .class_type_source(&uri, document.clone(), generation)
            .await
            .is_some()
    );
    {
        let mut documents = backend.documents.lock("class_type_revision_bump").await;
        let current = documents.get_mut(&uri).unwrap();
        current.bump_analysis_generation();
        current.publication = DocumentPublication::Indexed;
        assert_eq!(current.text, document.text);
        assert_eq!(current.version, document.version);
    }
    assert!(
        backend
            .class_type_source(&uri, document, generation)
            .await
            .is_none()
    );
}
