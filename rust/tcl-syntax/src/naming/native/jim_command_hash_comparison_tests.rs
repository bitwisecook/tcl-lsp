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
fn original_jim_hash_comparison_retains_counted_zero_bytes_and_root_geometry() {
    let jim = NativeNameProtocol::Jim084;
    for (original, key) in [
        (b"p".as_slice(), b"p".as_slice()),
        (b"::::p".as_slice(), b"p".as_slice()),
        (b"p\0x".as_slice(), b"p\0x".as_slice()),
        (b"::p\0x".as_slice(), b"p\0x".as_slice()),
        (b"p\0y".as_slice(), b"p\0y".as_slice()),
    ] {
        assert_eq!(jim.jim_command_table_comparison(original).unwrap(), key);
    }
    assert_ne!(
        jim.jim_command_table_comparison(b"p\0x"),
        jim.jim_command_table_comparison(b"p\0y")
    );
    for version in TclVersion::ALL {
        assert_eq!(
            NativeNameProtocol::C(version).jim_command_table_comparison(b"p"),
            Err(NameProjectionUnavailable::PurposeNotModelled)
        );
    }
}
