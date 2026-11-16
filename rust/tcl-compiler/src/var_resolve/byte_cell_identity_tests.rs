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
// Implementation contract: naming.variable.byte-cell-correspondence
// docs/design/analysis/name-resolution-proofs/variable.byte-cell-correspondence.md
fn retained_cell_key_uses_exact_bytes_independently_of_display() {
    let owner = CellOwner::Activation("entered-call".into());
    let mut first = place::scalar("same display", place::LOCAL_NS, false);
    first.cell = Some(CellIdentity {
        owner: owner.clone(),
        name: b"v\xc0\x80tail".into(),
        generation: CellGeneration::Incoming,
        interpreter: None,
        storage_domain: None,
        execution: None,
    });
    let mut second = first.clone();
    second.cell.as_mut().unwrap().name = b"v\0tail".into();
    assert_ne!(cell_key(&first), cell_key(&second));
    assert!(!place::overlap(&first, &second));
    second.cell.as_mut().unwrap().name = b"v\xc0\x80tail".into();
    second.name = "different advice".into();
    assert_eq!(cell_key(&first), cell_key(&second));
    assert!(place::overlap(&first, &second));
    first.kind = PlaceKind::ArrayElem;
    first.index = Some(Index::literal(b"k\xed\xa0\x80"));
    second.kind = PlaceKind::ArrayElem;
    second.index = Some(Index::literal(b"k\xed\xa0\x81"));
    assert_ne!(canonical_place_key(&first), canonical_place_key(&second));
    assert!(!place::overlap(&first, &second));
    let selected = canonical_place_key(&first).unwrap();
    assert!(
        matches!(selected, VariableCellKey::Element { index, .. } if index.as_bytes() == b"k\xed\xa0\x80")
    );
}
