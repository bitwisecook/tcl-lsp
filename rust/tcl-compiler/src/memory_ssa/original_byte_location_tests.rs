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
use tcl_names::place::{CellGeneration, CellIdentity, CellOwner, Index};

#[test]
fn identical_display_does_not_merge_opaque_root_or_index_storage() {
    let place = |name: &[u8], index: &[u8]| {
        let mut place = tcl_names::place::scalar("display", tcl_names::place::LOCAL_NS, false);
        place.cell = Some(CellIdentity {
            owner: CellOwner::Activation("same".to_owned()),
            name: name.into(),
            generation: CellGeneration::Incoming,
            interpreter: None,
            storage_domain: None,
            execution: None,
        });
        place.index = Some(Index::literal(tcl_core_types::NameBytes::from(index)));
        place
    };
    let first = memory_location_of_place(&place(b"\xed\xa0\x80", b"\0"));
    let other_root = memory_location_of_place(&place(b"\xed\xa0\x81", b"\0"));
    let other_index = memory_location_of_place(&place(b"\xed\xa0\x80", b"\xc0\x80"));
    assert!(first.storage_key.is_some());
    assert_ne!(first.storage_key, other_root.storage_key);
    assert_ne!(first.storage_key, other_index.storage_key);
}
