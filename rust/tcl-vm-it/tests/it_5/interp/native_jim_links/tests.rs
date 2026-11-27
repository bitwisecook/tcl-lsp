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

//! Jim alias reads and `upvar` error publication, against the tclsh and
//! jimsh captures.

fn decode(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn original_alias_read_errors_match_thirty_six_native_results() {
    let cases =
        include_str!("../../../../../tcl-syntax/tests/data/native_jim_alias_read/cases.tsv");
    let engines = [
        (
            "tcl8.4",
            include_str!("../../../../../tcl-syntax/tests/data/native_jim_alias_read/8.4.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../../../tcl-syntax/tests/data/native_jim_alias_read/8.5.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../../../tcl-syntax/tests/data/native_jim_alias_read/8.6.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../../../tcl-syntax/tests/data/native_jim_alias_read/9.0.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../../tcl-syntax/tests/data/native_jim_alias_read/9.1.tsv"),
        ),
        (
            "jim",
            include_str!("../../../../../tcl-syntax/tests/data/native_jim_alias_read/jim.tsv"),
        ),
    ];
    let mut count = 0;
    for (engine, expected) in engines {
        assert_eq!(cases.lines().count(), expected.lines().count());
        for (case, result) in cases.lines().zip(expected.lines()) {
            let (name, source) = tcl_test_support::fixture::tsv_split_once(case).unwrap();
            let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(result).collect();
            assert_eq!(fields[0], name);
            let profile = tcl_vm::test_support::profile(engine);
            let mut vm = crate::fixture::interpreter(profile);
            let completion = vm.try_eval_source_bytes(&decode(source)).unwrap();
            assert_eq!(
                completion.code.as_int(),
                fields[1].parse::<i64>().unwrap(),
                "{engine}/{name}"
            );
            let bytes = crate::fixture::bytes(&vm, &completion.result);
            assert_eq!(bytes.as_ref(), decode(fields[2]), "{engine}/{name}");
            count += 1;
        }
    }
    assert_eq!(count, 36);
}
