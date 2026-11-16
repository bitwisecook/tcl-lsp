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

#[test]
fn selected_memory_source_holds_mapping_and_bytes_and_refuses_in_place_change() {
    let store = MemoryStore::new();
    let uri = "file:///studio/active.tclspec";
    store.upsert_uri(uri, "/studio/active.tclspec", b"original".to_vec());
    let selected = store.document_uri_path(uri).unwrap();
    let mut published = 0;
    store
        .with_current_document_sources(&[(&selected, b"original")], &mut || {
            assert!(store.aliases.try_lock().is_err());
            assert!(store.files.try_lock().is_err());
            published += 1;
        })
        .unwrap();
    assert_eq!(published, 1);
    store.upsert("/studio/active.tclspec", b"changed".to_vec());
    store.validate_document_uri_path(&selected).unwrap();
    assert_eq!(
        store.with_current_document_sources(&[(&selected, b"original")], &mut || published += 1),
        Err(SourceUriPathUnavailable::SourceImageUnavailable)
    );
    assert_eq!(published, 1);
    store
        .with_current_document_sources(&[(&selected, b"changed")], &mut || published += 1)
        .unwrap();
    assert_eq!(published, 2);
}
