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
fn original_path_and_document_sources_share_one_provider_and_byte_borrow() {
    let store = MemoryStore::new();
    let uri = "file:///studio/active.tclspec";
    store.upsert_uri(uri, "/studio/active.tclspec", b"studio".to_vec());
    store.upsert("/ordinary/requested.tclspec", b"ordinary".to_vec());
    let document = store.document_uri_path(uri).unwrap();
    let path = store
        .source_path_origin(Path::new("/ordinary/requested.tclspec"))
        .unwrap();
    let mut published = 0;
    store
        .with_current_saved_source_inputs(
            &[(&document, b"studio")],
            &[(&path, b"ordinary")],
            &mut || {
                assert!(store.aliases.try_lock().is_err());
                assert!(store.files.try_lock().is_err());
                published += 1;
            },
        )
        .unwrap();
    assert_eq!(published, 1);
    store.upsert("/ordinary/requested.tclspec", b"changed".to_vec());
    assert_eq!(
        store.with_current_saved_source_inputs(
            &[(&document, b"studio")],
            &[(&path, b"ordinary")],
            &mut || published += 1
        ),
        Err(SourceUriPathUnavailable::SourceImageUnavailable)
    );
    assert_eq!(published, 1);
    store.upsert("/ordinary/requested.tclspec", b"ordinary".to_vec());
    store.upsert_uri(uri, "/studio/replacement.tclspec", b"studio".to_vec());
    assert!(
        store
            .with_current_saved_source_inputs(
                &[(&document, b"studio")],
                &[(&path, b"ordinary")],
                &mut || published += 1
            )
            .is_err()
    );
    assert_eq!(published, 1);
    let foreign = MemoryStore::new();
    foreign.upsert("/ordinary/requested.tclspec", b"ordinary".to_vec());
    assert_eq!(
        foreign.with_current_saved_source_inputs(&[], &[(&path, b"ordinary")], &mut || {
            published += 1;
        }),
        Err(SourceUriPathUnavailable::ForeignProvider)
    );
    assert_eq!(published, 1);
}
