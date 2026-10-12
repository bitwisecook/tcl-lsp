// SPDX-License-Identifier: AGPL-3.0-or-later
//! Whole-object return comparisons against actual counted formal/body inputs.

use crate::{Vm, value::Value};
use tcl_runtime_api::Completion;

fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn call(vm: &mut Vm, head: &[u8], arguments: &[Value]) -> Completion<Value> {
    vm.invoke_host_original_object_vector(&Value::new_native_string_bytes(head), arguments)
}

fn assert_row(
    vm: &Vm,
    result: &Completion<Value>,
    argument: Option<&Value>,
    rows: &str,
    label: &str,
) {
    let fields: Vec<_> = rows
        .lines()
        .find(|row| row.starts_with(&format!("{label}|")))
        .unwrap()
        .split('|')
        .collect();
    // Sample the retained result object before its only string getter.
    let same = argument.is_some_and(|argument| result.result.is_same_object(argument));
    assert_eq!(
        result.code.as_int(),
        fields[1].parse::<i64>().unwrap(),
        "{label}"
    );
    assert_eq!(same, fields[2] == "1", "{label}");
    let protocol = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
        .unwrap();
    assert_eq!(
        result
            .result
            .native_string_bytes(protocol)
            .unwrap()
            .as_ref(),
        unhex(fields[3]),
        "{label}"
    );
}

#[test]
fn counted_formal_renaming_keeps_whole_object_return_and_binding_boundaries() {
    // Native proof: naming.formal.whole-object-return-renaming
    // docs/design/analysis/name-resolution-proofs/formal-whole-object-return-renaming.md
    // Native proof: naming.formal.raw-zero-list-and-binding-renaming
    // docs/design/analysis/name-resolution-proofs/formal-raw-zero-list-and-binding-renaming.md
    for (engine, rows) in [
        (
            "tcl8.4",
            include_str!("../../tests/data/native_formal_alpha/v2/8.4.20/stdout.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_formal_alpha/v2/8.5.19/stdout.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_formal_alpha/v2/8.6.18/stdout.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_formal_alpha/v2/9.0.4/stdout.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_formal_alpha/v2/9.1.0/stdout.tsv"),
        ),
        (
            "jimtcl",
            include_str!("../../tests/data/native_formal_alpha/v2/jim/stdout.tsv"),
        ),
    ] {
        for input in rows.lines().filter(|row| row.starts_with("INPUT|")) {
            let fields: Vec<_> = input.split('|').collect();
            let name = fields[1];
            let parameter = unhex(fields[2]);
            let body = unhex(fields[3]);
            let mut vm =
                crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine));
            for (head, parameter, body, label) in [
                (
                    b"original".as_slice(),
                    parameter.as_slice(),
                    body.as_slice(),
                    format!("{name}_DECLARE_ORIGINAL"),
                ),
                (
                    b"renamed".as_slice(),
                    b"a".as_slice(),
                    b"return $a".as_slice(),
                    format!("{name}_DECLARE_RENAMED"),
                ),
            ] {
                let arguments = [head, parameter, body].map(Value::new_native_string_bytes);
                let result = call(&mut vm, b"proc", &arguments);
                assert_row(&vm, &result, None, rows, &label);
            }
            let prefix = format!("VALUE|{name}_");
            for value in rows.lines().filter(|row| row.starts_with(&prefix)) {
                let fields: Vec<_> = value.split('|').collect();
                let bytes = unhex(fields[2]);
                let argument = Value::new_native_string_bytes(bytes.as_slice());
                for (head, suffix) in [
                    (b"original".as_slice(), "ORIGINAL"),
                    (b"renamed".as_slice(), "RENAMED"),
                ] {
                    let result = call(&mut vm, head, std::slice::from_ref(&argument));
                    assert_row(
                        &vm,
                        &result,
                        Some(&argument),
                        rows,
                        &format!("{}_{suffix}", fields[1]),
                    );
                }
            }
        }
    }
}

#[test]
fn formal_renaming_observer_controls_report_the_distinct_names() {
    // Native proof: naming.formal.rename-observer-name-differences
    // docs/design/analysis/name-resolution-proofs/formal-rename-observer-name-differences.md
    for (engine, rows) in [
        (
            "tcl8.4",
            include_str!("../../tests/data/native_formal_alpha/v2/8.4.20/stdout.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_formal_alpha/v2/8.5.19/stdout.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_formal_alpha/v2/8.6.18/stdout.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_formal_alpha/v2/9.0.4/stdout.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_formal_alpha/v2/9.1.0/stdout.tsv"),
        ),
        (
            "jimtcl",
            include_str!("../../tests/data/native_formal_alpha/v2/jim/stdout.tsv"),
        ),
    ] {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine));
        let argument = Value::new_native_string_bytes(b"VALUE".as_slice());
        for (head, parameter) in [
            (b"original".as_slice(), b"longargument".as_slice()),
            (b"renamed".as_slice(), b"a".as_slice()),
        ] {
            assert_eq!(
                call(
                    &mut vm,
                    b"proc",
                    &[head, parameter, b"info locals"].map(Value::new_native_string_bytes)
                )
                .code,
                tcl_runtime_api::Code::Ok
            );
        }
        for (head, suffix) in [
            (b"original".as_slice(), "ORIGINAL"),
            (b"renamed".as_slice(), "RENAMED"),
        ] {
            let result = call(&mut vm, head, std::slice::from_ref(&argument));
            assert_row(
                &vm,
                &result,
                Some(&argument),
                rows,
                &format!("LOCALS_{suffix}"),
            );
            let result = call(
                &mut vm,
                b"info",
                &[b"args".as_slice(), head].map(Value::new_native_string_bytes),
            );
            assert_row(&vm, &result, None, rows, &format!("ARGS_{suffix}"));
        }
        if vm.actual_native_invocation_dialect().family() == Some(tcl_dialect::model::Family::Jim) {
            let result = call(
                &mut vm,
                b"info",
                &[b"commands".as_slice(), b"trace"].map(Value::new_native_string_bytes),
            );
            assert_row(&vm, &result, None, rows, "TRACE_AVAILABILITY");
            continue;
        }
        assert_eq!(
            call(
                &mut vm,
                b"proc",
                &[b"observe".as_slice(), b"name index op", b"set ::seen $name"]
                    .map(Value::new_native_string_bytes)
            )
            .code,
            tcl_runtime_api::Code::Ok
        );
        let legacy = vm.actual_native_invocation_dialect().tcl_version
            == Some(tcl_dialect::TclVersion::V8_4);
        for (head, parameter, suffix) in [
            (
                b"original".as_slice(),
                b"longargument".as_slice(),
                "ORIGINAL",
            ),
            (b"renamed".as_slice(), b"a".as_slice(), "RENAMED"),
        ] {
            let body = [
                if legacy {
                    b"trace variable ".as_slice()
                } else {
                    b"trace add variable ".as_slice()
                },
                parameter,
                if legacy {
                    b" r observe;return $".as_slice()
                } else {
                    b" read observe;return $".as_slice()
                },
                parameter,
            ]
            .concat();
            assert_eq!(
                call(
                    &mut vm,
                    b"proc",
                    &[head, parameter, body.as_slice()].map(Value::new_native_string_bytes)
                )
                .code,
                tcl_runtime_api::Code::Ok
            );
            let result = call(&mut vm, head, std::slice::from_ref(&argument));
            assert_row(
                &vm,
                &result,
                Some(&argument),
                rows,
                &format!("TRACE_RETURN_{suffix}"),
            );
            let result = call(
                &mut vm,
                b"set",
                &[Value::new_native_string_bytes(b"::seen".as_slice())],
            );
            assert_row(&vm, &result, None, rows, &format!("TRACE_NAME_{suffix}"));
        }
    }
}
