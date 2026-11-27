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

//! Jim `dict unset` and `unset` windows, against the jimsh 0.84 captures.

#[test]
fn original_dictionary_unset_matches_nine_native_jim_controls() {
    fn decode(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let mut compared = 0;
    for row in
        include_str!("../../../../../tcl-syntax/tests/data/native_jim_unset/windows.tsv").lines()
    {
        let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
        let profile = tcl_vm::test_support::profile("jim");
        let mut vm = crate::fixture::interpreter(profile);
        let completion = vm
            .try_eval_source_bytes(&decode(fields[1]))
            .unwrap_or_else(|error| panic!("original Jim unset {}: {error:?}", fields[0]));
        assert_eq!(
            completion.code.as_int(),
            fields[2].parse::<i64>().unwrap(),
            "{}",
            fields[0]
        );
        assert_eq!(
            crate::fixture::bytes(&vm, &completion.result),
            decode(fields[3]),
            "{}",
            fields[0]
        );
        compared += 1;
    }
    assert_eq!(compared, 9);
}
