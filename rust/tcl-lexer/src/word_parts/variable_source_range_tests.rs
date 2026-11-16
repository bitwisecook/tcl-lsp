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
fn borrowed_reference_extents_use_the_native_scanner() {
    let config = LexerConfig::default();
    for source in [b"$name".as_slice(), b"${braced}", b"$a($i)", b"${}"] {
        let WordBody::Parts(parts) = decompose(source, SubstFlags::default(), config) else {
            panic!("source contains a variable reference");
        };
        let WordPart::Variable(variable) = &parts[0] else {
            panic!("first component is a reference");
        };
        assert_eq!(
            variable.source_range_in(source, config),
            Some(0..source.len())
        );
        let different_allocation = source.to_vec();
        assert_eq!(
            variable.source_range_in(&different_allocation, config),
            None
        );
        if let Some(index) = &variable.index {
            let WordPart::Variable(nested) = &index[0] else {
                panic!("nested index reference");
            };
            assert_eq!(nested.source_range_in(source, config), Some(3..5));
        }
    }
}
