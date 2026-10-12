// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native counted property names, values and independent factory entry channels.

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
fn original_property_names_and_values_match_native_c9_counted_access() {
    // naming.property.original-c9-counted-name-access
    // docs/design/analysis/name-resolution-proofs/property-original-c9-counted-name-access.md
    for (engine, captured) in [
        (
            "tcl8.4",
            include_str!(
                "../../../tcl-registry/tests/data/native_property_counted_original/8.4.20/stdout.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../tcl-registry/tests/data/native_property_counted_original/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_property_counted_original/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_property_counted_original/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_property_counted_original/9.1.0/stdout.tsv"
            ),
        ),
        (
            "jimtcl",
            include_str!(
                "../../../tcl-registry/tests/data/native_property_counted_original/jim/stdout.tsv"
            ),
        ),
    ] {
        let interp = instance(engine);
        // The original property capture enters C9 source before counted
        // access. Unavailable factory argv has its own native channel below.
        if interp
            .actual_native_invocation_dialect()
            .native_property_name_protocol()
            .is_none()
        {
            continue;
        }
        for label in ["FF", "D800", "D801", "ZERO"] {
            let mut interp = instance(engine);
            let result = interp
                .eval_source("oo::configurable create C; C create o")
                .unwrap();
            let entered = result.code == Code::Ok;
            assert_row(
                &mut interp,
                &result,
                None,
                captured,
                &format!("{label}_CREATE"),
            );
            if !entered {
                continue;
            }
            let bytes = unhex(
                captured
                    .lines()
                    .find(|line| line.starts_with(&format!("INPUT|{label}|")))
                    .unwrap()
                    .split('|')
                    .nth(2)
                    .unwrap(),
            );
            let name = string(&bytes);
            let class = string(b"C");
            let property = string(b"property");
            let result = call(&mut interp, b"oo::define", &[&class, &property, &name]);
            assert_row(
                &mut interp,
                &result,
                None,
                captured,
                &format!("{label}_DEFINE"),
            );
            let mut dashed = b"-".to_vec();
            dashed.extend_from_slice(&bytes);
            let option = string(&dashed);
            let configure = string(b"configure");
            let member = string(b"VALUE");
            let value = list(&interp, &[&member]);
            let result = call(&mut interp, b"o", &[&configure, &option, &value]);
            assert_row(
                &mut interp,
                &result,
                None,
                captured,
                &format!("{label}_WRITE"),
            );
            let result = call(&mut interp, b"o", &[&configure, &option]);
            assert_row(
                &mut interp,
                &result,
                Some(&value),
                captured,
                &format!("{label}_READ"),
            );
            let result = call(
                &mut interp,
                b"info",
                &[&string(b"class"), &string(b"properties"), &class],
            );
            assert_row(
                &mut interp,
                &result,
                None,
                captured,
                &format!("{label}_ROSTER"),
            );
        }
        let mut interp = instance(engine);
        let result = interp
            .eval_source("oo::configurable create C; C create o")
            .unwrap();
        let entered = result.code == Code::Ok;
        assert_row(&mut interp, &result, None, captured, "DISTINCT_CREATE");
        if !entered {
            continue;
        }
        for (index, (name, body)) in [
            (b"p\xed\xa0\x80".as_slice(), b"return FIRST".as_slice()),
            (b"p\xed\xa0\x81".as_slice(), b"return SECOND".as_slice()),
        ]
        .into_iter()
        .enumerate()
        {
            let result = call(
                &mut interp,
                b"oo::define",
                &[
                    &string(b"C"),
                    &string(b"property"),
                    &string(name),
                    &string(b"-get"),
                    &string(body),
                ],
            );
            assert_row(
                &mut interp,
                &result,
                None,
                captured,
                &format!("DISTINCT_DEFINE_{index}"),
            );
        }
        for (index, name) in [b"-p\xed\xa0\x80".as_slice(), b"-p\xed\xa0\x81".as_slice()]
            .into_iter()
            .enumerate()
        {
            let result = call(&mut interp, b"o", &[&string(b"configure"), &string(name)]);
            assert_row(
                &mut interp,
                &result,
                None,
                captured,
                &format!("DISTINCT_READ_{index}"),
            );
        }
        let result = call(
            &mut interp,
            b"info",
            &[&string(b"class"), &string(b"properties"), &string(b"C")],
        );
        assert_row(&mut interp, &result, None, captured, "DISTINCT_ROSTER");
        for (index, invalid) in [
            b"p[bad]".as_slice(),
            b"p\\x",
            b"",
            b"-bad",
            b"p::q",
            b"p(q)",
        ]
        .into_iter()
        .enumerate()
        {
            let result = call(
                &mut interp,
                b"oo::define",
                &[&string(b"C"), &string(b"property"), &string(invalid)],
            );
            assert_row(
                &mut interp,
                &result,
                None,
                captured,
                &format!("INVALID_{index}"),
            );
        }
    }
}

#[test]
fn original_configurable_availability_matches_native_counted_argv_without_source_preflight() {
    // naming.property.original-configurable-availability-counted-argv
    // docs/design/analysis/name-resolution-proofs/property-original-configurable-availability-counted-argv.md
    for (engine, captured) in [
        (
            "tcl8.4",
            include_str!(
                "../../../tcl-registry/tests/data/native_property_availability_original/8.4.20/stdout.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../tcl-registry/tests/data/native_property_availability_original/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_property_availability_original/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_property_availability_original/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_property_availability_original/9.1.0/stdout.tsv"
            ),
        ),
        (
            "jimtcl",
            include_str!(
                "../../../tcl-registry/tests/data/native_property_availability_original/jim/stdout.tsv"
            ),
        ),
    ] {
        let mut interp = instance(engine);
        let result = call(
            &mut interp,
            b"info",
            &[&string(b"commands"), &string(b"::oo::configurable")],
        );
        assert_row(&mut interp, &result, None, captured, "AVAILABILITY_OBJV");
        let result = call(
            &mut interp,
            b"oo::configurable",
            &[&string(b"create"), &string(b"C")],
        );
        assert_row(&mut interp, &result, None, captured, "FACTORY_OBJV");
        let available = interp
            .actual_native_invocation_dialect()
            .native_property_name_protocol()
            .is_some();
        assert_eq!(
            result.code == Code::Ok,
            available,
            "{engine}: pure declaration availability remains separate from compilation"
        );
        if result.code == Code::Ok {
            let result = call(&mut interp, b"C", &[&string(b"create"), &string(b"o")]);
            assert_row(&mut interp, &result, None, captured, "OBJECT_OBJV");
        }
    }
}
