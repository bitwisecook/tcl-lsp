// SPDX-License-Identifier: AGPL-3.0-or-later
//! Counted explicit TclOO variable controls with actual result and namespace effects.

use super::*;

fn normalise_namespace(bytes: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"::oo::Obj") {
            let start = index + 9;
            let mut end = start;
            while bytes.get(end).is_some_and(u8::is_ascii_digit) {
                end += 1;
            }
            if end > start {
                result.extend_from_slice(b"::OBJECT");
                index = end;
                continue;
            }
        }
        result.push(bytes[index]);
        index += 1;
    }
    result
}

fn captured_result(rows: &str, label: &str) -> (i64, Vec<u8>) {
    let fields = rows
        .lines()
        .find(|line| line.split('|').next() == Some(label))
        .unwrap()
        .split('|')
        .collect::<Vec<_>>();
    (
        fields[1].parse().unwrap(),
        fields[2]
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect(),
    )
}

const SETUP: &str = "oo::class create C {method link {name} {set c [catch {my variable $name} r]; list $c $r [info vars]}; method report {name} {my varname $name}; method write {name} {my variable $name; set $name VALUE; my varname $name}}; C create O";
const INSPECT: &str = "set ns [info object namespace O]; list [info vars ${ns}::*] [namespace eval $ns {array exists a}] [namespace eval $ns {info exists k}]";
const CASES: &[(&str, &[u8])] = &[
    ("ASCII", b"k"),
    ("RAW_ZERO", b"k\0tail"),
    ("ENCODED_ZERO", b"k\xc0\x80tail"),
    ("ARRAY_ELEMENT", b"a(k)"),
    ("QUALIFIER_AFTER_ZERO", b"k\0::Q"),
    ("QUALIFIED", b"N::k"),
    ("ABSOLUTE_RAW_ZERO", b"::global\0tail"),
    ("OPAQUE_FF", b"k\xff"),
];

#[test]
fn explicit_variable_links_and_varname_match_original_counted_controls() {
    // Native proof: naming.tcloo.explicit-variable-link-counted-target
    // docs/design/analysis/name-resolution-proofs/explicit-variable-link-counted-target.md
    // Native proof: naming.tcloo.varname-cell-creation-and-reporting
    // docs/design/analysis/name-resolution-proofs/varname-cell-creation-and-reporting.md
    for (engine, rows) in [
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.1.0/stdout.tsv"
            ),
        ),
    ] {
        for (case, name) in CASES {
            for method in ["link", "report", "write"] {
                let label = format!("{method}_{case}");
                let profile =
                    tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
                let mut vm = crate::native_fixture::interpreter(profile);
                assert_eq!(vm.eval_source(SETUP).unwrap().code, Code::Ok);
                let completion = vm.invoke_command(
                    "O",
                    &[Value::string(method), Value::new_native_string_bytes(*name)],
                );
                let (code, result) = captured_result(rows, &label);
                assert_eq!(
                    completion.code.as_int(),
                    code,
                    "{engine}/{label}: {completion:?}; host={:?}",
                    vm.refused_completion()
                );
                assert_eq!(
                    normalise_namespace(&completion.result.string_bytes()),
                    normalise_namespace(&result),
                    "{engine}/{label}"
                );
                let after = vm.eval_source(INSPECT).unwrap();
                let (code, result) = captured_result(rows, &format!("{label}_AFTER"));
                assert_eq!(after.code.as_int(), code, "{engine}/{label}/after");
                assert_eq!(
                    normalise_namespace(&after.result.string_bytes()),
                    normalise_namespace(&result),
                    "{engine}/{label}/after"
                );
            }
        }
    }
}
