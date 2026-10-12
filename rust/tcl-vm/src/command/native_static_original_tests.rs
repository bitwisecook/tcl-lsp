// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native original-list static declarations and retained value identity.

use crate::{Vm, value::Value};
use tcl_runtime_api::{Code, Completion};

fn instance(engine: &str) -> Vm {
    crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine))
}
fn string(bytes: &[u8]) -> Value {
    Value::new_native_string_bytes(bytes)
}
fn list(vm: &Vm, members: &[&Value]) -> Value {
    Value::native_list_constructor(
        members.iter().map(|value| (*value).clone()).collect(),
        vm.actual_native_invocation_dialect()
            .native_string_protocol()
            .unwrap(),
    )
}
fn call(vm: &mut Vm, head: &[u8], arguments: &[&Value]) -> Completion<Value> {
    vm.invoke_host_original_object_vector(
        &string(head),
        &arguments
            .iter()
            .map(|value| (*value).clone())
            .collect::<Vec<_>>(),
    )
}
fn assert_row(
    vm: &mut Vm,
    result: &Completion<Value>,
    expected: Option<&Value>,
    captured: &str,
    label: &str,
) {
    let native = row(captured, label);
    let same = expected.is_some_and(|value| result.result.is_same_object(value));
    assert_eq!(
        result.code.as_int(),
        native[1].parse::<i64>().unwrap(),
        "{label}"
    );
    assert_eq!(same, native[2] == "1", "{label}");
    assert_eq!(
        result
            .result
            .native_string_bytes(
                vm.actual_native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap()
            )
            .unwrap()
            .as_ref(),
        unhex(native[3]),
        "{label}"
    );
}
fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn row<'a>(captured: &'a str, label: &str) -> Vec<&'a str> {
    captured
        .lines()
        .find(|line| line.split('|').next() == Some(label))
        .unwrap()
        .split('|')
        .collect()
}

#[test]
fn original_static_member_objects_match_native_counted_names_and_value_capture() {
    // naming.procedure-static.original-member-object-boundaries
    // docs/design/analysis/name-resolution-proofs/procedure-static-original-member-object-boundaries.md
    for (engine, captured) in [
        (
            "tcl8.4",
            include_str!(
                "../../../tcl-registry/tests/data/native_procedure_static_original/8.4.20/stdout.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../tcl-registry/tests/data/native_procedure_static_original/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_procedure_static_original/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_procedure_static_original/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_procedure_static_original/9.1.0/stdout.tsv"
            ),
        ),
        (
            "jimtcl",
            include_str!(
                "../../../tcl-registry/tests/data/native_procedure_static_original/jim/stdout.tsv"
            ),
        ),
    ] {
        for input in captured.lines().filter(|line| line.starts_with("INPUT|")) {
            let fields: Vec<_> = input.split('|').collect();
            let label = fields[1];
            let bytes = unhex(fields[2]);
            let mut interp = instance(engine);
            let name = string(&bytes);
            let member = string(b"VALUE");
            let literal = list(&interp, &[&member]);
            let specifier = list(&interp, &[&name, &literal]);
            let statics = list(&interp, &[&specifier]);
            let command = string(b"p");
            let parameters = string(b"");
            let mut body = b"return ${".to_vec();
            body.extend_from_slice(&bytes);
            body.push(b'}');
            let body = string(&body);
            let result = call(
                &mut interp,
                b"proc",
                &[&command, &parameters, &statics, &body],
            );
            let entered = result.code == Code::Ok;
            assert_row(
                &mut interp,
                &result,
                None,
                captured,
                &format!("{label}_LITERAL_DEFINE"),
            );
            if entered {
                let result = call(&mut interp, b"p", &[]);
                assert_row(
                    &mut interp,
                    &result,
                    Some(&literal),
                    captured,
                    &format!("{label}_LITERAL_CALL"),
                );
            }
        }
        for reference in [false, true] {
            let mut interp = instance(engine);
            let name = string(b"x\xff");
            let first = string(b"FIRST");
            let result = call(&mut interp, b"set", &[&name, &first]);
            assert_row(&mut interp, &result, None, captured, "SOURCE_SET");
            let specifier = string(if reference { b"&x\xff" } else { b"x\xff" });
            let statics = list(&interp, &[&specifier]);
            let command = string(b"p");
            let parameters = string(b"");
            let body = string(b"return ${x\xff}");
            let result = call(
                &mut interp,
                b"proc",
                &[&command, &parameters, &statics, &body],
            );
            let entered = result.code == Code::Ok;
            let label = if reference { "REFERENCE" } else { "COPY" };
            assert_row(
                &mut interp,
                &result,
                None,
                captured,
                &format!("{label}_DEFINE"),
            );
            if entered {
                let second = string(b"SECOND");
                let result = call(&mut interp, b"set", &[&name, &second]);
                assert_row(&mut interp, &result, None, captured, "SOURCE_REPLACE");
                let result = call(&mut interp, b"p", &[]);
                assert_row(
                    &mut interp,
                    &result,
                    Some(if reference { &second } else { &first }),
                    captured,
                    &format!("{label}_CALL"),
                );
            }
        }
        for duplicate in [false, true] {
            let mut interp = instance(engine);
            let name = string(b"x\xff");
            let value = string(b"V");
            let specifier = if duplicate {
                list(&interp, &[&name, &value])
            } else {
                name.clone()
            };
            let statics = if duplicate {
                list(&interp, &[&specifier, &specifier])
            } else {
                list(&interp, &[&specifier])
            };
            let command = string(b"p");
            let parameters = string(b"");
            let body = string(b"return ${x\xff}");
            let result = call(
                &mut interp,
                b"proc",
                &[&command, &parameters, &statics, &body],
            );
            let label = if duplicate { "DUPLICATE" } else { "MISSING" };
            assert_row(
                &mut interp,
                &result,
                None,
                captured,
                &format!("{label}_DEFINE"),
            );
            let result = call(&mut interp, b"info", &[&string(b"commands"), &command]);
            assert_row(
                &mut interp,
                &result,
                None,
                captured,
                &format!("{label}_COMMAND"),
            );
        }
    }
}

#[test]
fn original_jim_static_links_match_native_frame_storage_reuse_boundaries() {
    // naming.procedure-static.jim-original-link-frame-storage
    // docs/design/analysis/name-resolution-proofs/procedure-static-jim-original-link-frame-storage.md
    // naming.procedure-static.jim-native-link-frame-reuse-boundaries
    // docs/design/analysis/name-resolution-proofs/procedure-static-jim-native-link-frame-reuse-boundaries.md
    for (engine, captured) in [
        (
            "tcl8.4",
            include_str!(
                "../../../tcl-registry/tests/data/native_jim_link_frame_storage/8.4.20/stdout.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../tcl-registry/tests/data/native_jim_link_frame_storage/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_jim_link_frame_storage/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_jim_link_frame_storage/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_jim_link_frame_storage/9.1.0/stdout.tsv"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../tcl-registry/tests/data/native_jim_link_frame_storage/jim/stdout.tsv"
            ),
        ),
    ] {
        let rows = captured.lines().collect::<Vec<_>>();
        assert_eq!(rows.len(), 18, "{engine}");
        for rows in rows.chunks_exact(3) {
            assert!(rows[0].starts_with("VERSION|0|"), "{engine}");
            let input = rows[1].split('|').collect::<Vec<_>>();
            let expected = rows[2].split('|').collect::<Vec<_>>();
            assert_eq!(input[0], "INPUT");
            assert_eq!(input[1], expected[0]);
            let source = String::from_utf8(unhex(input[2])).unwrap();
            let mut vm = instance(engine);
            let completion = vm.try_eval_native_host_source(&source).unwrap();
            assert_eq!(
                completion.code.as_int(),
                expected[1].parse::<i64>().unwrap(),
                "{engine}: {}",
                expected[0]
            );
            assert_eq!(
                completion
                    .result
                    .native_string_bytes(
                        vm.actual_native_invocation_dialect()
                            .native_string_protocol()
                            .unwrap()
                    )
                    .unwrap()
                    .as_ref(),
                unhex(expected[2]),
                "{engine}: {}",
                expected[0]
            );
        }
    }
}
