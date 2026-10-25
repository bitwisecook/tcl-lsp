// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public original procedure windows and independent retained-object guards.

use super::*;

fn actual(engine: &str) -> Vm {
    crate::native_fixture::interpreter(
        tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
    )
}
fn unhex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn row<'a>(capture: &'a str, case: usize, phase: &str) -> Vec<&'a str> {
    capture
        .lines()
        .find(|line| {
            let fields: Vec<_> = line.split('|').collect();
            fields.first() == Some(&"ROW")
                && fields[1].parse::<usize>() == Ok(case)
                && fields[2] == phase
        })
        .unwrap()
        .split('|')
        .collect()
}

#[test]
fn original_procedure_public_completions_match_all_eighteen_native_definition_windows() {
    // naming.invocation.original-naming-argument-getter-and-effect-order
    // docs/design/analysis/name-resolution-proofs/invocation-original-naming-argument-getter-and-effect-order.md
    // Public worker and invocation code/result only: these values already have
    // Strings and cannot assert the original extension's getter tape or counts.
    let providers = [
        (
            "tcl8.4",
            include_str!(
                "../../../tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/8.4.20/execute.stdout"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/8.5.19/execute.stdout"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/8.6.18/execute.stdout"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/9.0.4/execute.stdout"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/9.1.0/execute.stdout"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499d/providers/jim/execute.stdout"
            ),
        ),
    ];
    let mut compared = 0;
    for (engine, capture) in providers {
        for case in [9, 10, 11] {
            let mut vm = actual(engine);
            vm.eval_source("namespace eval ::d {}").unwrap();
            let completion = cmd_proc(
                &mut vm,
                &[
                    Value::new_native_string_bytes(if case == 9 {
                        b"::missing::p".as_slice()
                    } else {
                        b"::d::p".as_slice()
                    }),
                    Value::new_native_string_bytes(if case == 11 {
                        b"".as_slice()
                    } else {
                        b"{bad".as_slice()
                    }),
                    Value::new_native_string_bytes(b"return OK".as_slice()),
                ],
            );
            let original = row(capture, case, "worker");
            assert_eq!(
                completion.code.as_int(),
                original[3].parse::<i64>().unwrap(),
                "{engine}/{case}"
            );
            assert!(vm.refused_completion().is_none(), "{engine}/{case}");
            assert_eq!(
                vm.native_name_operand_bytes(&completion.result)
                    .unwrap()
                    .as_ref(),
                unhex(original[4]),
                "{engine}/{case}"
            );
            if case == 11 {
                let completion = vm.eval_source("::d::p").unwrap();
                let original = row(capture, case, "invoke");
                assert_eq!(
                    completion.code.as_int(),
                    original[3].parse::<i64>().unwrap(),
                    "{engine}/{case}"
                );
                assert_eq!(
                    vm.native_name_operand_bytes(&completion.result)
                        .unwrap()
                        .as_ref(),
                    unhex(original[4]),
                    "{engine}/{case}"
                );
            }
            compared += 1;
        }
    }
    assert_eq!(compared, 18);
}

#[test]
fn original_definition_guards_do_not_read_later_unselected_name_or_body_objects() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software ordering controls use actual original compound backings. They
    // do not establish extension updater timing or native sharing counts.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(engine);
        let foreign = tcl_syntax::native_string::NativeStringProtocol::Jim084;
        let formals = Value::native_list_constructor(
            vec![Value::new_native_string_bytes(b"x".as_slice())],
            foreign,
        );
        let body = Value::native_list_constructor(
            vec![Value::new_native_string_bytes(b"unread".as_slice())],
            foreign,
        );
        let completion = cmd_proc(
            &mut vm,
            &[
                Value::new_native_string_bytes(b"::missing::p".as_slice()),
                formals.native_lifetime_lease().into_value(),
                body.native_lifetime_lease().into_value(),
            ],
        );
        assert_eq!(completion.code, Code::Error, "{engine}");
        assert!(vm.refused_completion().is_none(), "{engine}");
        assert!(formals.resident_string_bytes().is_none(), "{engine}");
        assert!(body.resident_string_bytes().is_none(), "{engine}");
        assert_eq!(formals.native_object_type_name(), "list", "{engine}");
        assert_eq!(body.native_object_type_name(), "list", "{engine}");
    }
    let mut vm = actual("jim");
    let protocol = tcl_syntax::native_string::NativeStringProtocol::Jim084;
    let foreign = tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1);
    let name = Value::native_list_constructor(
        vec![Value::new_native_string_bytes(b"unread".as_slice())],
        foreign,
    );
    let body = Value::native_list_constructor(
        vec![Value::new_native_string_bytes(b"unread".as_slice())],
        protocol,
    );
    let formal = Value::native_list_constructor(
        vec![
            Value::new_native_string_bytes(b"x".as_slice()),
            Value::new_native_string_bytes(b"y".as_slice()),
            Value::new_native_string_bytes(b"z".as_slice()),
        ],
        protocol,
    );
    let formals = Value::native_list_constructor(vec![formal], protocol);
    let completion = cmd_proc(
        &mut vm,
        &[
            name.native_lifetime_lease().into_value(),
            formals.native_lifetime_lease().into_value(),
            body.native_lifetime_lease().into_value(),
        ],
    );
    assert_eq!(completion.code, Code::Error);
    assert!(vm.refused_completion().is_none());
    assert!(
        vm.native_name_operand_bytes(&completion.result)
            .unwrap()
            .starts_with(b"too many fields in argument specifier")
    );
    assert!(name.resident_string_bytes().is_none());
    assert!(body.resident_string_bytes().is_none());
    assert_eq!(name.native_object_type_name(), "list");
    assert_eq!(body.native_object_type_name(), "list");
}
