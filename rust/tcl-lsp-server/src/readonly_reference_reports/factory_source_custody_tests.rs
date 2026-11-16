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
use crate::{DocumentState, vfs};

const CLOSED_SOURCE: &str = "proc factory_header {} {}\n";
const OPEN_SOURCE: &str = "proc caller {} {}\n";
const CLOSED_PATH: &str = "/factory-source-custody/closed.tcl";
const OPEN_PATH: &str = "/factory-source-custody/open.tcl";

struct Fixture {
    backend: Backend,
    store: Arc<vfs::MemoryStore>,
    closed: Uri,
    open: Uri,
}

impl Fixture {
    async fn new() -> Self {
        let mut backend = crate::tests::test_backend();
        let store = Arc::new(vfs::MemoryStore::new());
        let closed: Uri = "file:///factory-source-custody/closed.tcl".parse().unwrap();
        let open: Uri = "file:///factory-source-custody/open.tcl".parse().unwrap();
        store.upsert_uri(
            closed.as_str(),
            CLOSED_PATH,
            CLOSED_SOURCE.as_bytes().to_vec(),
        );
        store.upsert_uri(open.as_str(), OPEN_PATH, b"saved backing".to_vec());
        backend.state_mut_for_test().store = store.clone();
        for (uri, source) in [(&closed, CLOSED_SOURCE), (&open, OPEN_SOURCE)] {
            backend
                .db_set_source(uri, source, "tcl8.6".to_owned())
                .await
                .unwrap();
        }
        backend
            .documents
            .lock("factory source custody open")
            .await
            .insert(
                open.clone(),
                DocumentState::with_version(OPEN_SOURCE.to_owned(), "tcl8.6".to_owned(), 7),
            );
        Self {
            backend,
            store,
            closed,
            open,
        }
    }

    async fn leases(&self) -> Vec<ReadonlySourceDocument> {
        let mut leases = Vec::new();
        for (uri, source) in [(&self.closed, CLOSED_SOURCE), (&self.open, OPEN_SOURCE)] {
            leases.push(
                self.backend
                    .capture_readonly_source_document(uri.as_str(), Arc::from(source))
                    .await
                    .unwrap()
                    .unwrap(),
            );
        }
        leases
    }

    async fn publish(&self, leases: &[ReadonlySourceDocument]) -> DocumentSourceResult<Option<u8>> {
        let revision = tcl_lsp_db::database_revision(&*self.backend.db.lock().await);
        let epoch = self.backend.diag_inputs_epoch();
        let generation = *self.backend.class_factory_generation.read().await;
        let caller = self.backend.read_document(&self.open).await?.unwrap();
        let caller_generation = self.backend.live_publication_uri_generation(&self.open);
        self.backend
            .publish_factory_source_if_current(
                leases,
                epoch,
                generation,
                revision,
                (&self.open, &caller, caller_generation),
                17,
            )
            .await
    }
}

#[tokio::test]
async fn factory_source_final_lease_refuses_saved_image_change_with_unchanged_owners() {
    let fixture = Fixture::new().await;
    let leases = fixture.leases().await;
    assert_eq!(fixture.publish(&leases).await.unwrap(), Some(17));
    let admission = leases[0]
        .publication
        .document
        .path_admission
        .as_ref()
        .unwrap();
    for bytes in [
        CLOSED_SOURCE
            .replace("factory_header", "another_header")
            .into_bytes(),
        CLOSED_SOURCE.replace('\n', "\r").into_bytes(),
    ] {
        fixture.store.upsert(CLOSED_PATH, bytes);
        admission
            .check_for(&fixture.closed, fixture.store.as_ref())
            .unwrap();
        assert!(matches!(
            fixture.publish(&leases).await,
            Err(super::super::DocumentSourceUnavailable::Store {
                cause: vfs::SourceUriPathUnavailable::SourceImageUnavailable,
                ..
            })
        ));
    }
    fixture
        .store
        .upsert(CLOSED_PATH, CLOSED_SOURCE.as_bytes().to_vec());
    assert_eq!(fixture.publish(&leases).await.unwrap(), Some(17));
}

#[tokio::test]
async fn factory_source_refuses_original_closed_caller_after_normalised_equal_change() {
    let fixture = Fixture::new().await;
    let caller = fixture
        .backend
        .read_document(&fixture.closed)
        .await
        .unwrap()
        .unwrap();
    let caller_generation = fixture
        .backend
        .live_publication_uri_generation(&fixture.closed);
    let original_leases = fixture.leases().await;
    let revision = tcl_lsp_db::database_revision(&*fixture.backend.db.lock().await);
    let epoch = fixture.backend.diag_inputs_epoch();
    let generation = *fixture.backend.class_factory_generation.read().await;
    assert_eq!(
        fixture
            .backend
            .publish_factory_source_if_current(
                &original_leases,
                epoch,
                generation,
                revision,
                (&fixture.closed, &caller, caller_generation),
                17,
            )
            .await
            .unwrap(),
        Some(17)
    );
    fixture
        .store
        .upsert(CLOSED_PATH, CLOSED_SOURCE.replace('\n', "\r").into_bytes());
    let leases = fixture.leases().await;
    let revision = tcl_lsp_db::database_revision(&*fixture.backend.db.lock().await);
    let epoch = fixture.backend.diag_inputs_epoch();
    let generation = *fixture.backend.class_factory_generation.read().await;
    assert!(
        fixture
            .backend
            .publish_factory_source_if_current(
                &leases,
                epoch,
                generation,
                revision,
                (&fixture.closed, &caller, caller_generation),
                17,
            )
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn factory_source_saved_lease_keeps_independent_unsaved_open_mapping() {
    let fixture = Fixture::new().await;
    let leases = fixture.leases().await;
    assert_eq!(fixture.publish(&leases).await.unwrap(), Some(17));
    assert!(fixture.store.remove_uri(fixture.open.as_str()));
    fixture
        .store
        .upsert_uri(fixture.open.as_str(), OPEN_PATH, b"saved backing".to_vec());
    assert!(matches!(
        fixture.publish(&leases).await,
        Err(super::super::DocumentSourceUnavailable::Store {
            cause: vfs::SourceUriPathUnavailable::WithdrawnMapping,
            ..
        })
    ));
}
