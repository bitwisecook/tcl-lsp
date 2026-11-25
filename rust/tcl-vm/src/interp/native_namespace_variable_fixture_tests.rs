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

fn bytes_from_hex(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

// Native proof: naming.variable.namespace-which-variable-original-object-and-storage-state
// docs/design/analysis/name-resolution-proofs/variable.namespace-which-variable-original-object-and-storage-state.md
#[test]
fn namespace_queries_match_660_fixed_native_original_and_short_results() {
    let engines = [
        (
            "tcl8.4",
            include_str!("../../tests/data/native_namespace_variables/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_namespace_variables/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_namespace_variables/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_namespace_variables/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_namespace_variables/9.1.0.tsv"),
        ),
        (
            "jim",
            include_str!("../../tests/data/native_namespace_variables/Jim.tsv"),
        ),
    ];
    let mut compared = 0;
    for (engine, expected) in engines {
        let jim = engine == "jim";
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        for row in expected.lines() {
            let parts: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
            assert_eq!(parts.len(), 4);
            let (name, setup) = parts[0].split_once('/').unwrap();
            let spelling: &[u8] = match name {
                "raw00" => b"a\0z",
                "modified00" => b"a\xc0\x80z",
                "rawFF" => b"a\xff",
                "encodedFF" => b"a\xc3\xbf",
                "plain" => b"a",
                _ => panic!("unknown native input {name}"),
            };
            let mut vm = crate::native_fixture::core(profile);
            let operand = if parts[1] == "pure-byte-array" {
                Value::byte_array(Rc::<[u8]>::from(spelling))
            } else {
                Value::from_string_bytes(spelling)
            };
            let original = vm.native_name_operand_bytes(&operand).unwrap();
            match setup {
                "full-only" | "both" => {
                    vm.set_var_bytes(&original, Value::string("FULL")).unwrap();
                }
                "declared-undefined" if !jim => {
                    assert!(
                        vm.link_namespace_variable_bytes(&original, &original)
                            .is_ok()
                    );
                }
                "trace-undefined" if !jim => vm.ensure_trace_variable_bytes(&original).unwrap(),
                "absent" | "short-only" | "declared-undefined" | "trace-undefined" => {}
                _ => panic!("unknown native setup {setup}"),
            }
            if setup == "short-only" || setup == "both" {
                vm.set_var_bytes(b"a", Value::string("SHORT")).unwrap();
            }
            for (query, wanted) in [(original.as_ref(), parts[2]), (b"a".as_slice(), parts[3])] {
                let result = Namespaces::namespace_variable_name_bytes_checked(&vm, ROOT_NS, query)
                    .unwrap()
                    .unwrap_or_default();
                assert_eq!(
                    result,
                    bytes_from_hex(wanted),
                    "{engine}/{}:{}",
                    parts[0],
                    parts[1]
                );
                compared += 1;
            }
        }
    }
    assert_eq!(compared, 660);
}
