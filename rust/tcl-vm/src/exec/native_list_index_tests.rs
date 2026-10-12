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

//! Captured original compiled List/index execution and header ownership.
use crate::Value;
include!("../../../tcl-registry/tests/data/native_list_index_compilation/cases.rs");
const TABLES: &[(&str, &str)] = &[
    (
        "tcl8.4",
        include_str!("../../../tcl-registry/tests/data/native_list_index_compilation/8.4.20.txt"),
    ),
    (
        "tcl8.5",
        include_str!("../../../tcl-registry/tests/data/native_list_index_compilation/8.5.19.txt"),
    ),
    (
        "tcl8.6",
        include_str!("../../../tcl-registry/tests/data/native_list_index_compilation/8.6.18.txt"),
    ),
    (
        "tcl9.0",
        include_str!("../../../tcl-registry/tests/data/native_list_index_compilation/9.0.4.txt"),
    ),
    (
        "tcl9.1",
        include_str!("../../../tcl-registry/tests/data/native_list_index_compilation/9.1.0.txt"),
    ),
];
fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn original_compiled_list_index_preserves_95_native_headers_and_completions() {
    // Native proof naming.list-index.literal-native-instruction-coordinates:
    // docs/design/analysis/name-resolution-proofs/list-index-literal-native-instruction-coordinates.md
    // Native proof naming.list-index.multi-path-and-empty-validation:
    // docs/design/analysis/name-resolution-proofs/list-index-multi-path-and-empty-validation.md
    // Native proof naming.list-index.expansion-and-abrupt-child-boundaries:
    // docs/design/analysis/name-resolution-proofs/list-index-expansion-and-abrupt-child-boundaries.md
    let mut windows = 0;
    for &(engine, table) in TABLES {
        let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
        let mut vm = crate::native_fixture::interpreter(profile);
        for row in table.lines() {
            let fields: Vec<_> = row.split('|').collect();
            let case = fields[0].parse::<usize>().unwrap();
            vm.try_eval_source(&format!("proc p {{x i}} {{{}}}", CASES[case]))
                .unwrap();
            let head = Value::new_native_string_bytes(b"p".as_slice());
            let arguments = [
                Value::new_native_string_bytes(b"{{A B} C} D".as_slice()),
                Value::new_native_string_bytes(b"0".as_slice()),
            ];
            let completion = vm.invoke_host_original_object_vector(&head, &arguments);
            assert!(
                vm.execution_refusal.is_none(),
                "{engine}/{case}: {:?}",
                vm.execution_refusal
            );
            assert_eq!(
                completion.code.as_int().to_string(),
                fields[1],
                "{engine}/{case}"
            );
            let value = &completion.result;
            assert_eq!(
                value.native_object_type_name(),
                fields[2],
                "{engine}/{case} primary"
            );
            assert_eq!(
                usize::from(value.resident_string_bytes().is_some()).to_string(),
                fields[3],
                "{engine}/{case} resident"
            );
            assert_eq!(
                value.native_object_reference_count().to_string(),
                fields[4],
                "{engine}/{case} references"
            );
            assert_eq!(
                usize::from(value.is_same_object(&arguments[0])).to_string(),
                fields[5],
                "{engine}/{case} original List"
            );
            assert_eq!(
                value.resident_string_bytes().unwrap().as_ref(),
                unhex(fields[6]),
                "{engine}/{case} result"
            );
            windows += 1;
        }
    }
    assert_eq!(windows, 95);
}
#[test]
fn original_namespace_array_index_compilation_enters_its_retained_child() {
    for &(engine, _) in TABLES {
        let mut vm =
            crate::native_fixture::interpreter(tcl_dialect::DialectProfile::find(engine).unwrap());
        let source = "namespace eval ::orch {variable _event_index; array set _event_index {}; variable _idx 0; foreach _entry {{A {}}} {\n        set _event_index([lindex $_entry 0]) $_idx\n        incr _idx\n    }; set _event_index(A)}";
        let completion = vm.try_eval_source(source).unwrap();
        assert_eq!(completion.code, tcl_runtime_api::Code::Ok, "{engine}");
        assert_eq!(completion.result.to_str().as_ref(), "0", "{engine}");
        assert!(vm.execution_refusal.is_none(), "{engine}");
    }
}

mod original_objects {
    use crate::Value;
    include!("../../../tcl-registry/tests/data/native_list_index_original_objects/inputs.rs");
    const TABLES: &[(&str, &str)] = &[
        (
            "tcl8.4",
            include_str!(
                "../../../tcl-registry/tests/data/native_list_index_original_objects/8.4.20.txt"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../tcl-registry/tests/data/native_list_index_original_objects/8.5.19.txt"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_list_index_original_objects/8.6.18.txt"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_list_index_original_objects/9.0.4.txt"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_list_index_original_objects/9.1.0.txt"
            ),
        ),
    ];
    #[test]
    fn original_index_getters_match_45_native_header_and_result_windows() {
        // Native proof naming.list-index.original-integer-and-end-offset-headers:
        // docs/design/analysis/name-resolution-proofs/list-index-original-integer-and-end-offset-headers.md
        // Native proof naming.list-index.original-list-path-and-invalid-conversion:
        // docs/design/analysis/name-resolution-proofs/list-index-original-list-path-and-invalid-conversion.md
        let mut count = 0;
        for &(engine, table) in TABLES {
            let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
            let mut vm = crate::native_fixture::interpreter(profile);
            vm.try_eval_source("proc p {x i} {lindex $x $i}").unwrap();
            for row in table.lines() {
                let columns = row.split('|').collect::<Vec<_>>();
                let case = columns[0].parse::<usize>().unwrap();
                let head = Value::new_native_string_bytes(b"p".as_slice());
                let arguments = [
                    Value::new_native_string_bytes(b"{{A B} C} D".as_slice()),
                    Value::new_native_string_bytes(INPUTS[case].as_bytes()),
                ];
                let completion = vm.invoke_host_original_object_vector(&head, &arguments);
                assert!(
                    vm.execution_refusal.is_none(),
                    "{engine}/{case}: {:?}",
                    vm.execution_refusal
                );
                let result = &completion.result;
                let observed = [
                    completion.code.as_int().to_string(),
                    arguments[1].native_object_type_name().to_owned(),
                    usize::from(arguments[1].resident_string_bytes().is_some()).to_string(),
                    arguments[1].native_object_reference_count().to_string(),
                    result.native_object_type_name().to_owned(),
                    usize::from(result.resident_string_bytes().is_some()).to_string(),
                    result.native_object_reference_count().to_string(),
                ];
                assert_eq!(
                    observed.iter().map(String::as_str).collect::<Vec<_>>(),
                    columns[1..8],
                    "{engine}/{case} original headers"
                );
                assert_eq!(
                    result.string_bytes().as_ref(),
                    super::unhex(columns[8]),
                    "{engine}/{case} result"
                );
                count += 1;
            }
        }
        assert_eq!(count, 45);
    }
    #[test]
    fn original_index_list_and_returned_child_have_independent_real_owners() {
        for &(engine, _) in TABLES {
            let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
            let mut vm = crate::native_fixture::interpreter(profile);
            let child = Value::new_native_string_bytes(b"MEMBER".as_slice());
            let list = Value::list(vec![child.clone()]);
            let index = Value::list(vec![Value::new_native_string_bytes(b"0".as_slice())]);
            let before = child.native_object_reference_count();
            let selected = vm
                .original_list_index(&list, std::slice::from_ref(&index), true)
                .unwrap();
            assert_eq!(index.native_object_type_name(), "list", "{engine}");
            assert!(selected.is_same_object(&child), "{engine}");
            assert_eq!(
                child.native_object_reference_count(),
                before + 1,
                "{engine}"
            );
            drop(list);
            assert_eq!(selected.string_bytes().as_ref(), b"MEMBER", "{engine}");
            drop(selected);
            assert_eq!(child.native_object_reference_count(), 1, "{engine}");
        }
    }
}
