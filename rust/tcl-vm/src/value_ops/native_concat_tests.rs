// SPDX-License-Identifier: AGPL-3.0-or-later
//! Paired unchanged native inputs and physical windows, before observation getters.
use crate::{interp::Vm, value::Value};
use tcl_syntax::value::ValueOps;
include!("../../../tcl-cmd-core/tests/data/native_concat/cases.rs");
fn original(vm: &mut Vm, input: Input) -> Value {
    let protocol = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
        .unwrap();
    match input {
        Input::Bytes(bytes) => Value::new_native_string_bytes(bytes),
        Input::List(members) | Input::Rendered(members) => {
            let value = Value::native_list_constructor(
                members
                    .iter()
                    .map(|bytes| Value::new_native_string_bytes(*bytes))
                    .collect(),
                protocol,
            );
            if matches!(input, Input::Rendered(_)) {
                vm.native_concat_string_bytes(&value).unwrap();
            }
            value
        }
        Input::Parsed(bytes) => {
            let value = Value::new_native_string_bytes(bytes);
            vm.native_object_list_elements_in(&value, protocol).unwrap();
            value
        }
    }
}
#[test]
fn original_concat_matches_all_twenty_native_windows_per_engine() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = crate::native_fixture::core(
            tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
        );
        let protocol = vm
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        for case in 0..20 {
            let input = inputs(case);
            let values: Vec<_> = input.iter().map(|&item| original(&mut vm, item)).collect();
            let children: Vec<_> = input.iter().zip(&values).map(|(input, value)| {
                if matches!(input, Input::List(members) | Input::Rendered(members) if !members.is_empty()) {
                    value.native_list_backing_in(protocol).unwrap()
                } else { None }
            }).collect();
            let before = values
                .iter()
                .map(|value| {
                    state(
                        value.native_object_snapshot(),
                        value.native_object_reference_count(),
                        protocol.tcl_version(),
                    )
                })
                .collect::<Vec<_>>()
                .join(";");
            assert_eq!(
                before,
                row(engine, case, "before")[3],
                "{engine}/{case} before"
            );
            let result = tcl_cmd_core::list::concat_selected(&mut vm, &values).unwrap();
            let expected = row(engine, case, "result");
            assert_eq!(
                state(
                    result.native_object_snapshot(),
                    result.native_object_reference_count(),
                    protocol.tcl_version()
                ),
                expected[3],
                "{engine}/{case} result"
            );
            assert!(
                values
                    .first()
                    .is_none_or(|value| !result.is_same_object(value))
            );
            let shared = values.first().is_some_and(|first| {
                let left = first.native_list_backing_in(protocol).unwrap();
                let right = result.native_list_backing_in(protocol).unwrap();
                match (left, right) {
                    (Some(left), Some(right)) => {
                        left.elements().unwrap().as_ptr() == right.elements().unwrap().as_ptr()
                    }
                    _ => false,
                }
            });
            assert_eq!(
                format!("backing_first={}", usize::from(shared)),
                expected[5],
                "{engine}/{case} backing"
            );
            let cref = children
                .iter()
                .map(|child| {
                    child.as_ref().map_or_else(
                        || "-1".to_owned(),
                        |items| {
                            items.elements().unwrap()[0]
                                .native_object_reference_count()
                                .to_string()
                        },
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            assert_eq!(
                format!("children={cref}"),
                expected[6],
                "{engine}/{case} children"
            );
            let after = values
                .iter()
                .map(|value| {
                    state(
                        value.native_object_snapshot(),
                        value.native_object_reference_count(),
                        protocol.tcl_version(),
                    )
                })
                .collect::<Vec<_>>()
                .join(";");
            assert_eq!(
                after,
                row(engine, case, "after")[3],
                "{engine}/{case} after"
            );
            assert_eq!(
                hex(&vm.native_concat_string_bytes(&result).unwrap()),
                row(engine, case, "value")[3],
                "{engine}/{case} value"
            );
        }
    }
}
#[test]
fn compiled_concat_preserves_all_eight_original_c_result_headers() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for (case, &body) in COMPILED_BODIES.iter().enumerate() {
            let mut vm = crate::native_fixture::interpreter(
                tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
            );
            let proc_head = Value::new_native_string_bytes(b"proc".as_slice());
            let words = [
                Value::new_native_string_bytes(b"p".as_slice()),
                Value::new_native_string_bytes(b"".as_slice()),
                Value::new_native_string_bytes(body),
            ];
            let definition = vm.invoke_host_original_object_vector(&proc_head, &words);
            assert_eq!(
                definition.code,
                tcl_runtime_api::Code::Ok,
                "{engine}/{case} definition {definition:?}"
            );
            drop(definition);
            let head = Value::new_native_string_bytes(b"p".as_slice());
            let result = vm.invoke_host_original_object_vector(&head, &[]);
            let expected = compiled_row(engine, case);
            assert_eq!(
                result.code,
                tcl_runtime_api::Code::Ok,
                "{engine}/{case} {result:?}"
            );
            let version = vm.actual_native_invocation_dialect().tcl_version;
            assert_eq!(
                state(
                    result.result.native_object_snapshot(),
                    result.result.native_object_reference_count(),
                    version
                ),
                expected[4],
                "{engine}/{case}"
            );
        }
    }
}
#[test]
fn string_concat_does_not_issue_a_canonical_list_guarantee() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = crate::native_fixture::core(
            tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
        );
        let values = [
            Value::new_native_string_bytes(b"{".as_slice()),
            Value::new_native_string_bytes(b"foo".as_slice()),
        ];
        let result = tcl_cmd_core::list::concat_selected(&mut vm, &values).unwrap();
        assert_eq!(&*vm.native_concat_string_bytes(&result).unwrap(), b"{ foo");
        let protocol = vm
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let parsed = vm.native_object_list_elements_in(&result, protocol);
        if protocol.is_jim084() {
            assert_eq!(parsed.unwrap().elements().unwrap().len(), 1);
        } else {
            assert!(matches!(
                parsed,
                Err(tcl_syntax::value::ValueError::NativeListParse { .. })
            ));
        }
    }
}

include!("../../../tcl-registry/tests/data/native_concat_expansion/cases.rs");
fn expansion_cache_name(
    cache: &tcl_syntax::native_object::NativeObjectCacheSnapshot,
) -> &'static str {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    match cache {
        Cache::None => "NULL",
        Cache::String { .. } => "string",
        other => panic!("unexpected actual expanded concat result {other:?}"),
    }
}
#[test]
fn compiled_concat_expansion_preserves_all_48_original_native_windows() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for (case, body) in EXPANSION_BODIES.iter().enumerate() {
            let mut vm = crate::native_fixture::interpreter(
                tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
            );
            let proc_head = Value::new_native_string_bytes(b"proc".as_slice());
            let words = [
                Value::new_native_string_bytes(b"p".as_slice()),
                Value::new_native_string_bytes(b"a c".as_slice()),
                Value::new_native_string_bytes(*body),
            ];
            let definition = vm.invoke_host_original_object_vector(&proc_head, &words);
            assert_eq!(
                definition.code,
                tcl_runtime_api::Code::Ok,
                "{engine}/{case}: {definition:?}"
            );
            drop(definition);
            let head = Value::new_native_string_bytes(b"p".as_slice());
            let arguments = [
                Value::new_native_string_bytes(b"D E".as_slice()),
                Value::new_native_string_bytes(b"C".as_slice()),
            ];
            let result = vm.invoke_host_original_object_vector(&head, &arguments);
            let expected = expansion_result(engine, case);
            assert_eq!(
                result.code.as_int().to_string(),
                expected[2],
                "{engine}/{case}: {result:?}"
            );
            let snapshot = result.result.native_object_snapshot();
            assert_eq!(
                format!(
                    "{}\t{}\t{}",
                    expansion_cache_name(&snapshot.cache),
                    usize::from(snapshot.resident.is_some()),
                    result.result.native_object_reference_count()
                ),
                expected[3..6].join("\t"),
                "{engine}/{case} original result header"
            );
            assert_eq!(
                hex(&vm.native_concat_string_bytes(&result.result).unwrap()),
                expected[6],
                "{engine}/{case} original result"
            );
        }
    }
}
