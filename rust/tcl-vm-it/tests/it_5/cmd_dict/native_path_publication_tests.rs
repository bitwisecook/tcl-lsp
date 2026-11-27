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

use tcl_registry::native_dictionary::NativeDictionaryDialect;
fn original_result(rows: &str) -> (i64, Vec<u8>) {
    let fields = rows
        .lines()
        .find(|row| row.starts_with("ORIGINAL|"))
        .unwrap()
        .split('|')
        .collect::<Vec<_>>();
    let bytes = fields[2].as_bytes();
    assert_eq!(bytes.len() % 2, 0);
    (
        fields[1].parse().unwrap(),
        bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect(),
    )
}

#[test]
fn original_path_missing_variable_publication_matches_all_15_available_native_windows() {
    // naming.dictionary.original-path-missing-variable-publication
    // docs/design/analysis/name-resolution-proofs/dictionary-original-path-missing-variable-publication.md
    // Each complete original caller result keeps the error, existence,
    // caller read and alias/member publication together. No private object
    // header or C lookup receipt follows from these public observations.
    // C84's three original NA rows are retained independently: this test
    // checks absent purpose, without normalising a raw Return to C API Ok.
    let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
    let cases = [("missing-root-failed-path-unset", include_bytes!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/missing-root-failed-path-unset.tcl").as_slice(), [include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl8.4/missing-root-failed-path-unset/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl8.5/missing-root-failed-path-unset/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl8.6/missing-root-failed-path-unset/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl9.0/missing-root-failed-path-unset/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl9.1/missing-root-failed-path-unset/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/jim/missing-root-failed-path-unset/stdout")]),
("missing-member-failed-path-unset", include_bytes!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/missing-member-failed-path-unset.tcl").as_slice(), [include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl8.4/missing-member-failed-path-unset/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl8.5/missing-member-failed-path-unset/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl8.6/missing-member-failed-path-unset/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl9.0/missing-member-failed-path-unset/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl9.1/missing-member-failed-path-unset/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/jim/missing-member-failed-path-unset/stdout")]),
("linked-root-path-set", include_bytes!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/linked-root-path-set.tcl").as_slice(), [include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl8.4/linked-root-path-set/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl8.5/linked-root-path-set/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl8.6/linked-root-path-set/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl9.0/linked-root-path-set/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/tcl9.1/linked-root-path-set/stdout"),
include_str!("../../../../tcl-registry/tests/data/native_dictionary_path_publication319/jim/linked-root-path-set/stdout")])];
    let mut compared = 0;
    let mut unsupported = 0;
    for (case, source, columns) in cases {
        for (engine, column) in providers.iter().zip(columns) {
            let (expected_code, expected_result) = original_result(column);
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            if *engine == "tcl8.4" {
                assert_eq!(expected_code, 0, "{engine}/{case} native API boundary");
                assert_eq!(expected_result, b"NOT_APPLICABLE");
                assert!(
                    tcl_native_model::InvocationDialect::of_profile(profile)
                        .native_dictionary_path_publication()
                        .is_none()
                );
                unsupported += 1;
                continue;
            }
            let mut vm = crate::fixture::interpreter(profile);
            let completion = vm
                .try_eval_source_bytes(source)
                .unwrap_or_else(|error| panic!("{engine}/{case}: {error:?}"));
            assert_eq!(completion.code.as_int(), expected_code, "{engine}/{case}");
            let actual = vm.native_name_operand_bytes(&completion.result).unwrap();
            assert_eq!(actual.as_ref(), expected_result, "{engine}/{case}");
            compared += 1;
        }
    }
    assert_eq!(compared, 15);
    assert_eq!(unsupported, 3);
}
