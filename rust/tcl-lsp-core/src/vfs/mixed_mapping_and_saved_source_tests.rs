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
fn unsaved_document_mapping_and_saved_sibling_share_one_actual_provider_borrow() {
    let store = MemoryStore::new();
    store.upsert_uri(
        "file:///caller.tcl",
        "/caller.tcl",
        b"saved caller".to_vec(),
    );
    store.upsert_uri(
        "file:///sibling.tcl",
        "/sibling.tcl",
        b"proc helper {} {}".to_vec(),
    );
    let caller = store.document_uri_path("file:///caller.tcl").unwrap();
    let sibling = store.document_uri_path("file:///sibling.tcl").unwrap();
    let mut commits = 0;
    store
        .with_current_source_inputs(
            &[&caller],
            &[(&sibling, b"proc helper {} {}")],
            &[],
            &mut || {
                assert!(store.aliases.try_lock().is_err());
                assert!(store.files.try_lock().is_err());
                commits += 1;
            },
        )
        .unwrap();
    assert_eq!(commits, 1);
    store.upsert("/sibling.tcl", b"proc replacement {} {}".to_vec());
    assert_eq!(
        store.with_current_source_inputs(
            &[&caller],
            &[(&sibling, b"proc helper {} {}")],
            &[],
            &mut || commits += 1
        ),
        Err(SourceUriPathUnavailable::SourceImageUnavailable)
    );
    assert_eq!(commits, 1);
    store.upsert_uri(
        "file:///caller.tcl",
        "/different.tcl",
        b"saved caller".to_vec(),
    );
    assert!(
        store
            .with_current_source_inputs(&[&caller], &[], &[], &mut || commits += 1)
            .is_err()
    );
    assert_eq!(commits, 1);
}
