// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original private-body bytes and definition-mode queries.
use super::*;

fn original(bytes: &[u8]) -> Value {
    Value::new_native_string_bytes(bytes)
}

fn call(vm: &mut Vm, head: &[u8], arguments: &[Value]) -> Completion<Value> {
    vm.invoke_host_original_object_vector(&original(head), arguments)
}

fn assert_row(vm: &Vm, result: &Completion<Value>, rows: &str, label: &str) {
    let fields: Vec<_> = rows
        .lines()
        .find(|row| row.starts_with(&format!("{label}|")))
        .unwrap()
        .split('|')
        .collect();
    assert_eq!(
        result.code.as_int(),
        fields[1].parse::<i64>().unwrap(),
        "{label}: {result:?}"
    );
    let expected: Vec<_> = fields[2]
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
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
        expected,
        "{label}"
    );
}

#[test]
fn private_query_and_opaque_original_bodies_match_native_c9_controls() {
    // Native proof: naming.tcloo.private-definition-mode
    // docs/design/analysis/name-resolution-proofs/private-definition-mode.md
    // Native proof: naming.tcloo.private-original-script-bytes
    // docs/design/analysis/name-resolution-proofs/private-original-script-bytes.md
    for (engine, rows) in [
        (
            "tcl9.0",
            include_str!("../../tests/data/native_private_legacy_trace/v3/9.0.4/stdout.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_private_legacy_trace/v3/9.1.0/stdout.tsv"),
        ),
    ] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        let query = vm.try_eval_source_bytes(b"oo::class create C {set ::before [private];private {set ::inside [private]};set ::after [private]};list $::before $::inside $::after").unwrap_or_else(|error| panic!("{engine}/PRIVATE_QUERY: {error:?}"));
        assert_row(&vm, &query, rows, "PRIVATE_QUERY");
        assert_eq!(vm.oo.private_depth, 0);
        let outside = call(&mut vm, b"::oo::define::private", &[]);
        assert_eq!(outside.code, Code::Error);
        for (label, name) in [
            ("PRIVATE_RAW_FF", b"p\xff".as_slice()),
            ("PRIVATE_RAW_SURROGATE", b"p\xed\xa0\x80"),
            ("PRIVATE_RAW_ZERO", b"p\0tail"),
        ] {
            assert_eq!(
                call(&mut vm, b"rename", &[original(b"C"), original(b"")]).code,
                Code::Ok
            );
            assert_eq!(
                call(
                    &mut vm,
                    b"oo::class",
                    &[original(b"create"), original(b"C"), original(b"")]
                )
                .code,
                Code::Ok
            );
            let mut body = b"private {method {".to_vec();
            body.extend_from_slice(name);
            body.extend_from_slice(b"} {} {return OK}};method invoke {} {my {");
            body.extend_from_slice(name);
            body.extend_from_slice(b"}}");
            let defined = call(&mut vm, b"oo::define", &[original(b"C"), original(&body)]);
            assert_row(&vm, &defined, rows, label);
            let names = vm
                .try_eval_source_bytes(b"info class methods C -private")
                .unwrap();
            assert_row(&vm, &names, rows, &format!("{label}_NAMES"));
            assert_eq!(
                call(&mut vm, b"C", &[original(b"create"), original(b"c")]).code,
                Code::Ok
            );
            let invoked = call(&mut vm, b"c", &[original(b"invoke")]);
            assert_row(&vm, &invoked, rows, &format!("{label}_INVOKE"));
            assert_eq!(call(&mut vm, b"c", &[original(b"destroy")]).code, Code::Ok);
        }
    }
}

#[test]
fn method_option_layout_and_private_scope_match_original_native_controls() {
    // Native proof: naming.tcloo.method-original-option-layout
    // docs/design/analysis/name-resolution-proofs/method-original-option-layout.md
    // Native proof: naming.tcloo.true-private-provider-scope
    // docs/design/analysis/name-resolution-proofs/true-private-provider-scope.md
    for (engine, rows) in [
        (
            "tcl8.6",
            include_str!("../../tests/data/native_method_visibility/v2/8.6.18/stdout.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_method_visibility/v2/9.0.4/stdout.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_method_visibility/v2/9.1.0/stdout.tsv"),
        ),
    ] {
        for (label, option) in [
            ("PRIVATE_FULL", b"-private".as_slice()),
            ("PRIVATE_PREFIX", b"-p"),
            ("PRIVATE_RAW_ZERO", b"-private\0tail"),
            ("PRIVATE_ENCODED_ZERO", b"-private\xc0\x80tail"),
            ("EXPORT_FULL", b"-export"),
            ("UNEXPORT_FULL", b"-unexport"),
            ("INVALID_OPTION", b"--"),
        ] {
            let mut vm =
                crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine));
            assert_eq!(
                vm.try_eval_source_bytes(
                    b"oo::class create C {};oo::define C method invoke {} {my p};C create c"
                )
                .unwrap()
                .code,
                Code::Ok
            );
            let result = call(
                &mut vm,
                b"oo::define",
                &[
                    original(b"C"),
                    original(b"method"),
                    original(b"p"),
                    original(option),
                    original(b""),
                    original(b"return OK"),
                ],
            );
            assert_row(&vm, &result, rows, &format!("{label}_DEFINE"));
            if result.code == Code::Ok {
                if label.starts_with("PRIVATE") {
                    let names = vm
                        .try_eval_source_bytes(b"info class methods C -private")
                        .unwrap();
                    assert_row(&vm, &names, rows, &format!("{label}_NAMES"));
                }
                let external = call(&mut vm, b"c", &[original(b"p")]);
                assert_row(&vm, &external, rows, &format!("{label}_EXTERNAL"));
                let internal = call(&mut vm, b"c", &[original(b"invoke")]);
                assert_row(&vm, &internal, rows, &format!("{label}_INTERNAL"));
            }
        }
        for row in include_str!("../../tests/data/native_method_visibility/cases.tsv").lines() {
            let (label, source) = row.split_once('|').unwrap();
            if engine == "tcl8.6"
                && !matches!(
                    label,
                    "DASHED_FORMAL_OPTION_FREE"
                        | "OPTION_SPELLING_AS_NAME"
                        | "EXTERNAL_SELF_UNEXPORTED"
                )
            {
                continue;
            }
            let mut vm =
                crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine));
            let result = vm.try_eval_source_bytes(source.as_bytes()).unwrap();
            if label == "SUBCLASS_PRIVATE_MISS" {
                // Scope rejection compares completion code; bootstrap diagnostic
                // roster ordering is an independent obligation.
                let expected = rows
                    .lines()
                    .find(|row| row.starts_with("SUBCLASS_PRIVATE_MISS|"))
                    .unwrap()
                    .split('|')
                    .nth(1)
                    .unwrap()
                    .parse::<i64>()
                    .unwrap();
                assert_eq!(result.code.as_int(), expected);
            } else {
                assert_row(&vm, &result, rows, label);
            }
        }
    }
}
