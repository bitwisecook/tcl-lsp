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

//! Original grouped keys and actual returned comparator objects, bounded to651.
use crate::Value;
use std::cell::{Cell, RefCell};
use tcl_cmd_core::index::NativeIndexOps;
use tcl_runtime_api::{Code, Completion};
include!(
    "../../../../runtime/rust/tests/data/native_original_list_group_command651/software_controls.rs"
);
thread_local! {
    static RETURNED: RefCell<Option<Value>> = const { RefCell::new(None) };
    static CALLBACK_ERROR: Cell<bool> = const { Cell::new(false) };
}
fn make(input: (&str, &str), dialect: tcl_registry::InvocationDialect) -> Option<Value> {
    match input.0 {
        "absent" => None,
        "counted-string" => Some(Value::new_native_string_bytes(unhex(input.1))),
        "typed-wide" => Some(Value::int(input.1.parse().unwrap()).into_native_unowned_lifetime()),
        "typed-double" => Some(
            Value::native_double(
                if input.1 == "NaN" {
                    f64::NAN
                } else {
                    input.1.parse().unwrap()
                },
                dialect,
            )
            .into_native_unowned_lifetime(),
        ),
        _ => unreachable!(),
    }
}
fn argument(value: &Value) -> Value {
    value.native_lifetime_lease().into_value()
}
fn word(input: &[u8]) -> Value {
    Value::new_native_string_bytes(input).into_native_reference()
}
fn callback(vm: &mut crate::interp::Vm, arguments: &[Value]) -> Completion<Value> {
    assert_eq!(arguments.len(), 2);
    // Native651 deliberately gets both actual callback arguments before return.
    for argument in arguments {
        vm.index_operand_bytes(argument).unwrap();
    }
    RETURNED.with(|returned| {
        Completion::new(
            if CALLBACK_ERROR.get() {
                Code::Error
            } else {
                Code::Ok
            },
            returned.borrow().as_ref().unwrap().clone(),
            Value::empty(),
        )
    })
}
#[test]
fn grouped_sort_and_original_comparator_result_public_predicates_match_native651() {
    // naming.list.original-grouped-stride-and-command-result-getters
    // docs/design/analysis/name-resolution-proofs/list-original-grouped-stride-and-command-result-getters.md
    // Actual containing List-only key holds and ONE deliberate returned-object
    // observer are retained. Compare only Guest/bytes and public primary
    // category and successful-key residency, plus returned-object residency.
    // Failed-call argv logging is outside this direct command control; no
    // refcount, alias or foreign callback order claim.
    // Jim case12 has no provider output and supplies no expected software result.
    assert_eq!(ROWS.len(), 179);
    for row in ROWS {
        let mut vm =
            crate::native_fixture::core(crate::environment::profile_for_dialect(row.dialect));
        let dialect = vm.actual_native_invocation_dialect();
        let protocol = dialect.native_string_protocol().unwrap();
        let originals = row.input.map(|input| make(input, dialect));
        let members = originals[..row.count]
            .iter()
            .flatten()
            .map(argument)
            .collect();
        let container = Value::native_list_constructor(members, protocol).into_native_reference();
        let returned = make(row.returned, dialect).map(Value::into_native_reference);
        RETURNED.set(returned.as_ref().map(argument));
        CALLBACK_ERROR.set(row.error);
        vm.register("r2286cmp", callback);
        let command_member = Value::new_native_string_bytes(b"r2286cmp");
        let prefix = Value::native_list_constructor(vec![argument(&command_member)], protocol)
            .into_native_reference();
        let mode = word(if row.command {
            b"-command"
        } else if row.real == "real" {
            b"-real"
        } else if row.real == "ascii" {
            b"-ascii"
        } else {
            b"-integer"
        });
        let stride = word(b"-stride");
        let two = word(b"2");
        let index = word(b"-index");
        let path = row.index.map(|path| word(path.as_bytes()));
        let unique = word(b"-unique");
        let mut arguments = vec![argument(&mode)];
        if row.command {
            arguments.push(argument(&prefix));
        }
        if row.stride {
            arguments.extend([argument(&stride), argument(&two)]);
        }
        if let Some(path) = &path {
            arguments.extend([argument(&index), argument(path)]);
        }
        if row.unique {
            arguments.push(argument(&unique));
        }
        arguments.push(argument(&container));
        let completion = super::cmd_lsort(&mut vm, &arguments);
        let label = format!("{}:{}:{}", row.dialect, row.case, row.stream_sha256);
        assert_eq!(
            completion.code,
            if row.code == 0 { Code::Ok } else { Code::Error },
            "{label}"
        );
        for (original, effect) in originals.iter().zip(row.effect) {
            if let (Some(original), Some((expected, resident))) = (original, effect) {
                assert_eq!(
                    category(original.native_object_type_name()),
                    category(expected),
                    "{label}"
                );
                if row.code == 0 {
                    assert_eq!(
                        original.resident_string_bytes().is_some(),
                        resident,
                        "{label}"
                    );
                }
            }
        }
        if let (Some(returned), Some((expected, resident))) = (&returned, row.return_effect) {
            assert_eq!(
                category(returned.native_object_type_name()),
                category(expected),
                "{label}"
            );
            assert_eq!(
                returned.resident_string_bytes().is_some(),
                resident,
                "{label}"
            );
        }
        assert_eq!(
            vm.index_operand_bytes(&completion.result).unwrap().as_ref(),
            unhex(row.result),
            "{label}"
        );
        RETURNED.set(None);
    }
}

const SIZE_MESSAGE: &[u8] = b"list size must be a multiple of the stride length";
const INDEX_MESSAGE: &[u8] =
    b"when used with \"-stride\", the leading \"-index\" value must be within the group";
const OBSERVED_SIZE_ERROR_INFO: &str = "6c6973742073697a65206d7573742062652061206d756c7469706c65206f662074686520737472696465206c656e6774680a202020207768696c6520657865637574696e670a226c736f7274202d696e7465676572202d7374726964652032202d696e6465782030207b32204120317d22";

fn unreached_group_callback(_: &mut crate::interp::Vm, _: &[Value]) -> Completion<Value> {
    panic!("group validation must precede the selected comparison");
}
#[test]
fn original_grouping_failure_metadata_and_pre_frame_boundary_follow_native651_and_source() {
    // naming.list.original-grouped-stride-and-command-result-getters
    // docs/design/analysis/name-resolution-proofs/list-original-grouped-stride-and-command-result-getters.md
    // Size count3/index0 matches observed651 case7 C86/9 exact code/message.
    // Leading index2 and simultaneous size/index errors are pinned-source
    // software controls only; no failing leading-index native651 output exists.
    // Assert exact carried code and absent explicit errorInfo before dispatcher
    // framing. The observed full native errorInfo remains a separate Eval field.
    // Cold key primaries and unreachable callback are local software predicates;
    // neither callback0 nor these fields grants native getter/qsort chronology.
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for (count, path, expected_code, expected_message) in [
            (
                3,
                "0",
                b"TCL OPERATION LSORT BADSTRIDE".as_slice(),
                SIZE_MESSAGE,
            ),
            (
                4,
                "2",
                b"TCL OPERATION LSORT BADINDEX".as_slice(),
                INDEX_MESSAGE,
            ),
            (
                3,
                "2",
                b"TCL OPERATION LSORT BADSTRIDE".as_slice(),
                SIZE_MESSAGE,
            ),
        ] {
            for command in [false, true] {
                let mut vm =
                    crate::native_fixture::core(crate::environment::profile_for_dialect(profile));
                let protocol = vm
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap();
                let keys: Vec<_> = [b"2".as_slice(), b"A", b"1", b"B"]
                    .into_iter()
                    .take(count)
                    .map(Value::new_native_string_bytes)
                    .collect();
                let container = Value::native_list_constructor(
                    keys[..count].iter().map(argument).collect(),
                    protocol,
                )
                .into_native_reference();
                vm.register("r2286groupcmp", unreached_group_callback);
                let head = Value::new_native_string_bytes(b"r2286groupcmp");
                let prefix = Value::native_list_constructor(vec![argument(&head)], protocol)
                    .into_native_reference();
                let mode = word(if command { b"-command" } else { b"-integer" });
                let stride = word(b"-stride");
                let width = word(b"2");
                let index = word(b"-index");
                let path = word(path.as_bytes());
                let mut arguments = vec![argument(&mode)];
                if command {
                    arguments.push(argument(&prefix));
                }
                arguments.extend([
                    argument(&stride),
                    argument(&width),
                    argument(&index),
                    argument(&path),
                    argument(&container),
                ]);
                let completion = super::cmd_lsort(&mut vm, &arguments);
                assert_eq!(completion.code, Code::Error);
                assert_eq!(category(width.native_object_type_name()), "integer");
                for key in &keys[..count] {
                    assert_eq!(category(key.native_object_type_name()), "string");
                }
                assert_eq!(
                    vm.index_operand_bytes(&completion.result).unwrap().as_ref(),
                    expected_message
                );
                let carried = crate::command::opt_get(&completion.options, "-errorcode").unwrap();
                assert_eq!(
                    vm.index_operand_bytes(&carried).unwrap().as_ref(),
                    expected_code
                );
                assert!(crate::command::opt_get(&completion.options, "-errorinfo").is_none());
                if expected_message == SIZE_MESSAGE {
                    let full_native = unhex(OBSERVED_SIZE_ERROR_INFO);
                    assert!(full_native.starts_with(expected_message));
                    assert_ne!(full_native.as_slice(), expected_message);
                }
            }
        }
    }
}

include!(
    "../../../../runtime/rust/tests/data/native_original_list_group_cli656/software_controls.rs"
);
#[test]
fn original_cli_group_fields_match_native656_source_capture() {
    // naming.list.original-cli-group-boundary-results
    // docs/design/analysis/name-resolution-proofs/list-original-cli-group-boundary-results.md
    // Same fixed Tcl capture prefix with separately initialised argv; compare
    // exact script/code/result/errorCode/errorInfo/presence and callback count.
    // Native process startup and private getter/header chronology are excluded.
    // Jim651 case12 is a different input and remains unavailable.
    let probe = include_str!(
        "../../../../runtime/rust/tests/data/native_original_list_group_cli656/probe.tcl"
    );
    let capture = probe.split("puts \"case\\t").next().unwrap();
    assert_eq!(CLI_ROWS.len(), 66);
    for row in CLI_ROWS {
        let mut vm = crate::native_fixture::interpreter(crate::environment::profile_for_dialect(
            row.dialect,
        ));
        let startup = format!("set argv {{{}}}", row.case);
        assert_eq!(vm.eval_source(&startup).unwrap().code, Code::Ok);
        let label = format!("{}:{}:{}", row.dialect, row.case, row.stream_sha256);
        assert_eq!(vm.eval_source(capture).unwrap().code, Code::Ok, "{label}");
        for (name, expected) in CLI_VARIABLES.into_iter().zip(row.fields) {
            let original = vm.get_var(name).unwrap();
            assert_eq!(
                vm.index_operand_bytes(&original).unwrap().as_ref(),
                unhex(expected),
                "{label}:{name}"
            );
        }
    }
}

#[test]
fn original_guest_compare_failure_preserves_result_and_native651_context() {
    // naming.list.original-grouped-stride-and-command-result-getters
    // docs/design/analysis/name-resolution-proofs/list-original-grouped-stride-and-command-result-getters.md
    // Native651 case27 records the same returned original, NONE and the exact
    // compare suffix. This direct adapter boundary excludes Eval's outer frame.
    // Carried options must preserve the same suffix for the next restore owner.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(profile));
        let protocol = vm
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let returned = word(b"CALLBACK");
        RETURNED.set(Some(argument(&returned)));
        CALLBACK_ERROR.set(true);
        vm.register("r2286cmp", callback);
        let head = Value::new_native_string_bytes(b"r2286cmp");
        let prefix =
            Value::native_list_constructor(vec![argument(&head)], protocol).into_native_reference();
        let container = Value::native_list_constructor(
            vec![
                Value::new_native_string_bytes(b"B"),
                Value::new_native_string_bytes(b"A"),
            ],
            protocol,
        )
        .into_native_reference();
        let mode = word(b"-command");
        let completion = super::cmd_lsort(
            &mut vm,
            &[argument(&mode), argument(&prefix), argument(&container)],
        );
        assert_eq!(completion.code, Code::Error);
        assert!(completion.result.is_same_object(&returned));
        let info = vm.error_info_value().unwrap();
        assert!(info.starts_with(b"CALLBACK"));
        assert!(info.ends_with(b"\n    (-compare command)"));
        let expected = info.to_vec();
        let carried = crate::command::opt_get(&completion.options, "-errorinfo").unwrap();
        assert_eq!(vm.index_operand_bytes(&carried).unwrap().as_ref(), expected);
        let code = crate::command::opt_get(&completion.options, "-errorcode").unwrap();
        assert_eq!(vm.index_operand_bytes(&code).unwrap().as_ref(), b"NONE");
        RETURNED.set(None);
    }
}
