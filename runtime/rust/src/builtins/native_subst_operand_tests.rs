// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original option and catch-output operands against direct six-engine controls.
use super::*;

fn rows() -> [(&'static str, &'static str); 6] {
    [
        (
            "tcl8.4",
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.4.20/stdout.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.1.0/stdout.tsv"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/jim/stdout.tsv"
            ),
        ),
    ]
}

fn call(interp: &mut Interp, head: &[u8], arguments: &[&[u8]]) -> Code {
    let words: Vec<_> = std::iter::once(head)
        .chain(arguments.iter().copied())
        .map(|bytes| crate::obj::Owned::fresh(obj::new_string_bytes(bytes)))
        .collect();
    let vector: Vec<_> = words.iter().map(crate::obj::Owned::as_ptr).collect();
    interp.dispatch(&vector)
}
fn assert_row(interp: &Interp, code: Code, captured: &str, label: &str, engine: &str) {
    let fields = captured
        .lines()
        .find(|row| row.starts_with(&format!("{label}|")))
        .unwrap()
        .split('|')
        .collect::<Vec<_>>();
    assert_eq!(
        code.as_int(),
        fields[1].parse::<i64>().unwrap(),
        "{engine}/{label}: {:?}",
        interp.result_bytes()
    );
    let expected: Vec<_> = fields[2]
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(interp.result_bytes(), expected, "{engine}/{label}");
    assert!(!interp.host_refusal_pending(), "{engine}/{label}");
}
fn actual(engine: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(engine),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
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
            let mut vm = actual(engine);
            assert_eq!(call(&mut vm, b"set", &[b"x", b"VALUE"]), Code::Ok);
            let mut arguments = options;
            arguments.push(b"$x");
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
            let mut vm = actual(engine);
            for (key, value) in [
                (b"r".as_slice(), b"PLAIN".as_slice()),
                (b"r\xef\xbf\xbd", b"REPLACEMENT"),
            ] {
                assert_eq!(call(&mut vm, b"set", &[key, value]), Code::Ok);
            }
            let result = call(&mut vm, b"catch", &[b"error BOOM", name]);
            assert_row(&vm, result, captured, label, engine);
            for (suffix, key) in [
                ("PRIMARY", name),
                ("PLAIN", b"r".as_slice()),
                ("REPLACEMENT", b"r\xef\xbf\xbd"),
            ] {
                let result = call(&mut vm, b"set", &[key]);
                assert_row(&vm, result, captured, &format!("{label}_{suffix}"), engine);
            }
        }
    }
}
