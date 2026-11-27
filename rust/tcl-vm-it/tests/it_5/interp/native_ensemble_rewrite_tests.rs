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

//! The usage message of `array default`, against the tclsh 9.0.4 and 9.1.0
//! captures.

use crate::prelude::*;

fn decode_hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn selected_vm(engine: &str) -> Vm {
    let profile = tcl_vm::test_support::profile(engine);
    crate::fixture::interpreter(profile)
}

#[test]
fn array_default_usage_matches_sixteen_native_original_headers() {
    let mut count = 0;
    for row in include_str!("../../../../../runtime/rust/tests/data/native_array_default/usage.tsv")
        .lines()
    {
        let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
        let engine = match fields[0] {
            "9.0.4" => "tcl9.0",
            "9.1.0" => "tcl9.1",
            _ => unreachable!(),
        };
        let mut vm = selected_vm(engine);
        let completion = vm.try_eval_source_bytes(&decode_hex(fields[2])).unwrap();
        assert_eq!(completion.code, Code::Ok, "{row}");
        assert_eq!(
            crate::fixture::bytes(&vm, &completion.result),
            decode_hex(fields[3]),
            "{row}"
        );
        count += 1;
    }
    assert_eq!(count, 16);
}
