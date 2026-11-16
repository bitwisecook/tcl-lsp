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
async fn saved_images_share_only_the_original_admission_and_byte_allocations() {
    let mut backend = crate::tests::test_backend();
    let store = Arc::new(crate::vfs::MemoryStore::new());
    let uri: Uri = "file:///factory-source-image-allocations.tcl"
        .parse()
        .unwrap();
    store.upsert_uri(
        uri.as_str(),
        "/factory-source-image-allocations.tcl",
        b"proc p {} {}\n".to_vec(),
    );
    backend.state_mut_for_test().store = store;
    let first = backend.read_document(&uri).await.unwrap().unwrap();
    let shared_first = first.clone();
    let independent = backend.read_document(&uri).await.unwrap().unwrap();
    assert_eq!(first.text, independent.text);
    assert_eq!(first.path_admission, independent.path_admission);
    assert_eq!(
        super::super::DocumentState::saved_source_inputs([&first, &shared_first, &independent])
            .len(),
        2,
    );
}
