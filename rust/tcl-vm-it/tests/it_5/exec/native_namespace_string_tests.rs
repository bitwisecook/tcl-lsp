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

//! `namespace tail` and `namespace qualifiers`, compiled in a procedure and
//! invoked directly, against the pinned releases' counted results.

use tcl_syntax::value::ValueOps;
use tcl_vm::Value;

const INPUTS: [&[u8]; 10] = [
    b"a::tail",
    b"no_separator",
    b"a:::tail",
    b"a::",
    b"a::\xed\xa0\x80",
    b"a::\xed\xa0\x81",
    b"a::\xff",
    b"a\0::tail",
    b"",
    b":",
];

#[test]
// Native proof: naming.namespace.original-counted-tail-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
fn original_namespace_tail_matches_one_hundred_counted_native_windows() {
    let mut windows = 0;
    for (engine, table) in tail_windows() {
        let mut vm =
            crate::fixture::interpreter(tcl_dialect::DialectProfile::find(engine).unwrap());
        vm.try_eval_source("proc originalTail {value} {namespace tail $value}")
            .unwrap();
        for row in table.lines().filter(|row| row.starts_with("R|")) {
            let fields: Vec<_> = row.split('|').collect();
            let form = fields[1];
            let case = fields[2].parse::<usize>().unwrap();
            let input = Value::new_native_string_bytes(INPUTS[case]);
            let head = if form == "1" {
                "originalTail"
            } else {
                "namespace"
            };
            let args = if form == "1" {
                vec![input.clone()]
            } else {
                vec![
                    Value::new_native_string_bytes(b"tail".as_slice()),
                    input.clone(),
                ]
            };
            let completion = crate::fixture::call(&mut vm, head, &args);
            assert_eq!(
                completion.code.as_int().to_string(),
                fields[3],
                "{engine}/{form}/{case}"
            );
            let bytes = vm.native_string_bytes(&completion.result).unwrap();
            assert_eq!(
                bytes.as_ref(),
                crate::fixture::unhex(fields[7]),
                "{engine}/{form}/{case}: counted result"
            );
            let original = table
                .lines()
                .find(|row| row.starts_with(&format!("I|{form}|{case}|")))
                .unwrap()
                .split('|')
                .collect::<Vec<_>>();
            assert_eq!(
                vm.native_string_bytes(&input).unwrap().as_ref(),
                crate::fixture::unhex(original[5]),
                "{engine}/{form}/{case}: original counted bytes"
            );
            windows += 1;
        }
    }
    assert_eq!(windows, 100);
}

const QUALIFIERS_INPUTS: [&[u8]; 20] = [
    b"a::tail",
    b"no_separator",
    b"a:::tail",
    b"a::",
    b"a::\xed\xa0\x80",
    b"a::\xed\xa0\x81",
    b"a::\xff",
    b"a\0::tail",
    b"",
    b":",
    b"::a",
    b":::a",
    b"::::",
    b"a::::b",
    b"a::b::c",
    b"::a::b",
    b"\xff::tail",
    b"\xed\xa0\x80::tail",
    b"\xed\xa0\x81::tail",
    b"a::b\0::tail",
];

#[test]
// Native proof: naming.namespace.original-counted-qualifiers-compiler-and-runtime
// docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
fn original_namespace_qualifiers_matches_two_hundred_forty_counted_native_windows() {
    // naming.namespace.original-counted-qualifiers-compiler-and-runtime
    // docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
    let mut windows = 0;
    for (engine, table) in qualifiers_windows() {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
        vm.try_eval_source("proc originalQualifiers {value} {namespace qualifiers $value}")
            .unwrap();
        for row in table.lines().filter(|row| row.starts_with("R|")) {
            let fields: Vec<_> = row.split('|').collect();
            let form = fields[1];
            let case = fields[2].parse::<usize>().unwrap();
            let input = Value::new_native_string_bytes(QUALIFIERS_INPUTS[case]);
            let head = if form == "1" {
                "originalQualifiers"
            } else {
                "namespace"
            };
            let args = if form == "1" {
                vec![input.clone()]
            } else {
                vec![
                    Value::new_native_string_bytes(b"qualifiers".as_slice()),
                    input.clone(),
                ]
            };
            let completion = crate::fixture::call(&mut vm, head, &args);
            assert_eq!(
                completion.code.as_int().to_string(),
                fields[3],
                "{engine}/{form}/{case}"
            );
            let bytes = vm.native_string_bytes(&completion.result).unwrap();
            assert_eq!(
                bytes.as_ref(),
                crate::fixture::unhex(fields[7]),
                "{engine}/{form}/{case}: counted result"
            );
            let original = table
                .lines()
                .find(|row| row.starts_with(&format!("I|{form}|{case}|")))
                .unwrap()
                .split('|')
                .collect::<Vec<_>>();
            assert_eq!(
                vm.native_string_bytes(&input).unwrap().as_ref(),
                crate::fixture::unhex(original[5]),
                "{engine}/{form}/{case}: original counted bytes"
            );
            windows += 1;
        }
    }
    assert_eq!(windows, 240);
}

fn tail_windows() -> [(&'static str, &'static str); 5] {
    [
        (
            "tcl8.4",
            include_str!(
                "../../../../tcl-registry/tests/data/native_namespace_string_compilation/tcl8.4/stdout.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../tcl-registry/tests/data/native_namespace_string_compilation/tcl8.5/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../tcl-registry/tests/data/native_namespace_string_compilation/tcl8.6/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../tcl-registry/tests/data/native_namespace_string_compilation/tcl9.0/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../tcl-registry/tests/data/native_namespace_string_compilation/tcl9.1/stdout.tsv"
            ),
        ),
    ]
}

fn qualifiers_windows() -> [(&'static str, &'static str); 6] {
    [
        (
            "tcl8.4",
            include_str!(
                "../../../../tcl-registry/tests/data/native_namespace_qualifiers_compilation/tcl8.4/stdout.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../tcl-registry/tests/data/native_namespace_qualifiers_compilation/tcl8.5/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../tcl-registry/tests/data/native_namespace_qualifiers_compilation/tcl8.6/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../tcl-registry/tests/data/native_namespace_qualifiers_compilation/tcl9.0/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../tcl-registry/tests/data/native_namespace_qualifiers_compilation/tcl9.1/stdout.tsv"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../../tcl-registry/tests/data/native_namespace_qualifiers_compilation/jim/stdout.tsv"
            ),
        ),
    ]
}
