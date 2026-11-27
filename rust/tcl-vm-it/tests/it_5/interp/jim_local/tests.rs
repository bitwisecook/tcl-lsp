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

//! Jim's `local` frames and nested up-calls, against the jimsh captures.

#[test]
fn local_frames_and_nested_upcalls_match_original_native_lineage() {
    let fixture =
        include_str!("../../../../../tcl-syntax/tests/data/native_jim_local/behaviour.tsv");
    let mut checked = 0;
    for row in fixture.lines() {
        let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
        let expected: Vec<u8> = fields[1]
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        let profile = tcl_vm::test_support::profile("jim");
        let mut vm = crate::fixture::interpreter(profile);
        let result = crate::fixture::eval(&mut vm, fields[2]);
        assert!(
            result.code.is_ok(),
            "{}: {:?}",
            fields[2],
            result.result.string_bytes()
        );
        assert_eq!(
            result.result.string_bytes().as_ref(),
            expected.as_slice(),
            "{}",
            fields[2]
        );
        checked += 1;
    }
    assert_eq!(checked, 5);
}
