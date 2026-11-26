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

mod tests {
    use crate::prelude::*;

    const OBSERVED: &str = include_str!(
        "../../../../../tcl-vm/tests/data/native_jim_namespace_parent/getter-identity-20261015.tsv"
    );

    fn observation(kind: &str, case: &str) -> Vec<&'static str> {
        OBSERVED
            .lines()
            .find_map(|line| {
                let fields: Vec<_> = line.split('|').collect();
                (fields[0] == kind && fields[1] == case).then(|| fields[2..].to_vec())
            })
            .unwrap_or_else(|| panic!("missing original Parent observation {kind}/{case}"))
    }

    #[test]
    fn parent_keeps_original_result_primary_and_empty_identity() {
        for (case, input) in [
            ("RELATIVE_REUSE", b"absent::child".as_slice()),
            ("PLAIN_REUSE", b"absent"),
            ("ABSOLUTE_REUSE", b"::absent::child"),
            ("ROOT_REUSE", b"::"),
            ("EMPTY_REUSE", b""),
            ("NUL_ROOT_REUSE", b"::\0tail"),
            ("NUL_RELATIVE_REUSE", b"absent::child\0tail"),
            ("ONE_COLON_REUSE", b":"),
            ("TRIPLE_COLON_REUSE", b":::"),
        ] {
            let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("jimtcl"));
            let result = vm
                .try_invoke_command(
                    "namespace",
                    &[
                        Value::string("parent"),
                        Value::new_native_string_bytes(input),
                    ],
                )
                .unwrap();
            let expected = observation("RESULT", case);
            let bytes: Vec<u8> = expected[1]
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            assert_eq!(result.result.string_bytes().as_ref(), bytes, "{case}");
        }
    }
}
