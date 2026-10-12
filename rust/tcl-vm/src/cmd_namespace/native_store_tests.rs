// SPDX-License-Identifier: AGPL-3.0-or-later
//! Namespace stores use the actual entered frame and release-specific fallback.
use super::*;

fn decode(bytes: &str) -> Vec<u8> {
    bytes
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn namespace_stores_match_native_existing_root_fresh_and_declared_controls() {
    // Native proof: naming.variable.namespace-store-frame-and-fallback
    // docs/design/analysis/name-resolution-proofs/namespace-store-frame-and-fallback.md
    let cases = [
        (
            "EXISTING_ROOT",
            "N",
            "set x VALUE; list [set x] [catch {set ::N::x} local] $local [set ::x]",
            true,
        ),
        (
            "FRESH_NO_ROOT",
            "Fresh",
            "set x VALUE; list [set x] [catch {set ::Fresh::x} local] $local [catch {set ::x} root] $root",
            false,
        ),
        (
            "EXPLICIT_VARIABLE",
            "Explicit",
            "variable x VALUE; list [set x] [catch {set ::Explicit::x} local] $local [set ::x]",
            true,
        ),
    ];
    for (engine, rows) in [
        (
            "tcl8.4",
            include_str!(
                "../../../tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/namespace.stdout"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/namespace.stdout"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/namespace.stdout"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/namespace.stdout"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/namespace.stdout"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/namespace.stdout"
            ),
        ),
    ] {
        for (label, namespace, script, has_root) in cases {
            let mut vm = crate::native_fixture::interpreter(
                tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
            );
            if has_root {
                assert_eq!(
                    vm.invoke_command(
                        "set",
                        &[
                            Value::new_native_string_bytes(b"::x".as_slice()),
                            Value::new_native_string_bytes(b"ROOT".as_slice())
                        ]
                    )
                    .code,
                    Code::Ok
                );
            }
            let result = vm
                .try_invoke_command(
                    "namespace",
                    &[
                        Value::new_native_string_bytes(b"eval".as_slice()),
                        Value::new_native_string_bytes(namespace.as_bytes()),
                        Value::new_native_string_bytes(script.as_bytes()),
                    ],
                )
                .unwrap_or_else(|error| panic!("{engine}/{label}: {error}"));
            let row = rows
                .lines()
                .find(|row| row.starts_with(&format!("CASE|{label}|")))
                .unwrap();
            let fields: Vec<_> = row.split('|').collect();
            assert_eq!(
                result.code.as_int(),
                fields[2].parse::<i64>().unwrap(),
                "{engine}/{label}: {result:?}"
            );
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
                decode(fields[3]),
                "{engine}/{label}"
            );
        }
    }
}
