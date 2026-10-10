// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original option and catch-output operands against direct six-engine controls.
use super::*;

fn rows() -> [(&'static str, &'static str); 6] {
    [
        (
            "tcl8.4",
            include_str!("../../tests/data/native_subst_catch_operands/v2/8.4.20/stdout.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_subst_catch_operands/v2/8.5.19/stdout.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_subst_catch_operands/v2/8.6.18/stdout.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_subst_catch_operands/v2/9.0.4/stdout.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_subst_catch_operands/v2/9.1.0/stdout.tsv"),
        ),
        (
            "jim",
            include_str!("../../tests/data/native_subst_catch_operands/v2/jim/stdout.tsv"),
        ),
    ]
}

fn original(bytes: &[u8]) -> Value {
    Value::new_native_string_bytes(bytes)
}
fn call(vm: &mut Vm, head: &[u8], arguments: &[Value]) -> Completion<Value> {
    vm.invoke_host_original_object_vector(&original(head), arguments)
}
fn assert_row(vm: &Vm, result: Completion<Value>, captured: &str, label: &str, engine: &str) {
    let fields = captured
        .lines()
        .find(|row| row.starts_with(&format!("{label}|")))
        .unwrap()
        .split('|')
        .collect::<Vec<_>>();
    assert_eq!(
        result.code.as_int(),
        fields[1].parse::<i64>().unwrap(),
        "{engine}/{label}: {result:?}"
    );
    let expected: Vec<_> = fields[2]
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    let actual = result
        .result
        .native_string_bytes(
            vm.actual_native_invocation_dialect()
                .native_string_protocol()
                .unwrap(),
        )
        .unwrap_or_else(|error| panic!("{engine}/{label}: {error:?}"));
    assert_eq!(actual.as_ref(), expected, "{engine}/{label}");
}

#[test]
fn original_subst_options_match_counted_native_families() {
    // Native proof: naming.substitution.original-options-and-flag-families
    // docs/design/analysis/name-resolution-proofs/substitution-original-options-and-flag-families.md
    for (engine, captured) in rows() {
        for (label, options) in [
            (
                "SUBST_OPTION_RAW_ZERO",
                vec![b"-novariables\0tail".as_slice()],
            ),
            (
                "SUBST_OPTION_ENCODED_ZERO",
                vec![b"-novariables\xc0\x80tail".as_slice()],
            ),
            ("SUBST_OPTION_FF", vec![b"-novariables\xff".as_slice()]),
            ("SUBST_POSITIVE_0", vec![b"-backslashes".as_slice()]),
            ("SUBST_POSITIVE_1", vec![b"-commands".as_slice()]),
            ("SUBST_POSITIVE_2", vec![b"-variables".as_slice()]),
            (
                "SUBST_MIXED",
                vec![b"-variables".as_slice(), b"-nocommands"],
            ),
        ] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
            assert_eq!(
                call(&mut vm, b"set", &[original(b"x"), original(b"VALUE")]).code,
                Code::Ok
            );
            let mut arguments: Vec<_> = options.into_iter().map(original).collect();
            arguments.push(original(b"$x"));
            let result = call(&mut vm, b"subst", &arguments);
            assert_row(&vm, result, captured, label, engine);
        }
    }
}

#[test]
fn catch_original_output_names_match_counted_native_controls() {
    // Native proof: naming.variable.catch-original-output-name
    // docs/design/analysis/name-resolution-proofs/catch-original-output-name.md
    for (engine, captured) in rows() {
        for (label, name) in [
            ("CATCH_NAME_FF", b"r\xff".as_slice()),
            ("CATCH_NAME_RAW_ZERO", b"r\0tail"),
            ("CATCH_NAME_ENCODED_ZERO", b"r\xc0\x80tail"),
        ] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
            for (key, value) in [
                (b"r".as_slice(), b"PLAIN".as_slice()),
                (b"r\xef\xbf\xbd", b"REPLACEMENT"),
            ] {
                assert_eq!(
                    call(&mut vm, b"set", &[original(key), original(value)]).code,
                    Code::Ok
                );
            }
            let result = call(
                &mut vm,
                b"catch",
                &[original(b"error BOOM"), original(name)],
            );
            assert_row(&vm, result, captured, label, engine);
            for (suffix, key) in [
                ("PRIMARY", name),
                ("PLAIN", b"r".as_slice()),
                ("REPLACEMENT", b"r\xef\xbf\xbd"),
            ] {
                let result = call(&mut vm, b"set", &[original(key)]);
                assert_row(&vm, result, captured, &format!("{label}_{suffix}"), engine);
            }
        }
    }
}
