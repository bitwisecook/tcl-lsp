// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original apply arguments and selected coroutine dispatch at the object-vector boundary.
use crate::{
    interp::{Code, Interp},
    obj::{self, Owned},
};
fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn row<'a>(rows: &'a str, label: &str) -> Vec<&'a str> {
    rows.lines()
        .find(|line| line.split('|').next() == Some(label))
        .unwrap()
        .split('|')
        .collect()
}

fn fresh(engine: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(engine),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn call(interp: &mut Interp, head: &[u8], args: &[*mut obj::TclObj]) -> Code {
    let head = Owned::fresh(obj::new_string_bytes(head));
    let mut words = vec![head.as_ptr()];
    words.extend_from_slice(args);
    interp.dispatch(&words)
}
fn assert_row(
    interp: &Interp,
    code: Code,
    argument: Option<*mut obj::TclObj>,
    rows: &str,
    label: &str,
) {
    let native = row(rows, label);
    // Inspect identity before the only selected native result getter.
    let same = argument.is_some_and(|argument| interp.get_obj_result() == argument);
    assert_eq!(code.as_int(), native[1].parse::<i64>().unwrap(), "{label}");
    assert_eq!(same, native[2] == "1", "{label}/identity");
    assert_eq!(interp.result_bytes(), unhex(native[3]), "{label}/bytes");
    assert!(!interp.host_refusal_pending(), "{label}/host");
}

#[test]
fn apply_returns_original_counted_argument_objects() {
    // Native proof: naming.lambda.original-argument-object-return
    // docs/design/analysis/name-resolution-proofs/lambda-original-argument-object-return.md
    let mut comparisons = 0;
    for (engine, rows) in [
        (
            "tcl8.5",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/stdout.tsv"
            ),
        ),
        (
            "jimtcl",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_apply_original_argv/jim/stdout.tsv"
            ),
        ),
    ] {
        for input in rows.lines().filter(|line| line.starts_with("INPUT_")) {
            let fields: Vec<_> = input.split('|').collect();
            let case = fields[0].strip_prefix("INPUT_").unwrap();
            let argument = Owned::fresh(obj::new_string_bytes(&unhex(fields[2])));
            let lambda = Owned::fresh(obj::new_string_bytes(b"{x} {return $x}"));
            let mut interp = fresh(engine);
            let code = call(&mut interp, b"apply", &[lambda.as_ptr(), argument.as_ptr()]);
            assert_row(
                &interp,
                code,
                Some(argument.as_ptr()),
                rows,
                &format!("APPLY_{case}"),
            );
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 35);
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn apply_coroutine_keeps_arguments_and_selects_the_actual_shadowed_handler() {
    // Native proof: naming.lambda.original-coroutine-call-and-shadowing
    // docs/design/analysis/name-resolution-proofs/lambda-original-coroutine-call-and-shadowing.md
    let mut comparisons = 0;
    for (engine, rows) in [
        (
            "tcl8.6",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/stdout.tsv"
            ),
        ),
    ] {
        for input in rows.lines().filter(|line| line.starts_with("INPUT_")) {
            let fields: Vec<_> = input.split('|').collect();
            let case = fields[0].strip_prefix("INPUT_").unwrap();
            let argument = Owned::fresh(obj::new_string_bytes(&unhex(fields[2])));
            let words = [b"c".as_slice(), b"apply", b"{x} {yield READY; return $x}"]
                .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)));
            let mut args = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
            args.push(argument.as_ptr());
            let mut interp = fresh(engine);
            let start = call(&mut interp, b"coroutine", &args);
            assert_row(
                &interp,
                start,
                Some(argument.as_ptr()),
                rows,
                &format!("COROUTINE_START_{case}"),
            );
            let finish = call(&mut interp, b"c", &[]);
            assert_row(
                &interp,
                finish,
                Some(argument.as_ptr()),
                rows,
                &format!("COROUTINE_RETURN_{case}"),
            );
            comparisons += 1;
        }
        let mut interp = fresh(engine);
        let shadow =
            interp.eval_str(b"proc apply args {return SHADOW}; coroutine c apply {} VALUE");
        assert_row(&interp, shadow, None, rows, "SHADOWED_APPLY_COROUTINE");
    }
    assert_eq!(comparisons, 21);
}

const LAMBDA_NAMESPACE_INPUTS: [(&str, &[u8]); 9] = [
    ("PLAIN", b"ns"),
    ("COLON", b":ns"),
    ("ABSOLUTE", b"::ns"),
    ("ABSOLUTE_COLON", b":::ns"),
    ("EMPTY", b""),
    ("RELATIVE_ZERO", b":ns\0suffix"),
    ("ABSOLUTE_ZERO", b"::ns\0suffix"),
    ("LEADING_ZERO", b"\0suffix"),
    ("UNICODE_COLON", b":\xc3\xa9"),
];

const LAMBDA_NAMESPACE_ROWS: [(&str, &str); 5] = [
    (
        "tcl8.5",
        include_str!(
            "../../../../rust/tcl-registry/tests/data/native_lambda_object_getter/8.5.19/stdout.tsv"
        ),
    ),
    (
        "tcl8.6",
        include_str!(
            "../../../../rust/tcl-registry/tests/data/native_lambda_object_getter/8.6.18/stdout.tsv"
        ),
    ),
    (
        "tcl9.0",
        include_str!(
            "../../../../rust/tcl-registry/tests/data/native_lambda_object_getter/9.0.4/stdout.tsv"
        ),
    ),
    (
        "tcl9.1",
        include_str!(
            "../../../../rust/tcl-registry/tests/data/native_lambda_object_getter/9.1.0/stdout.tsv"
        ),
    ),
    (
        "jimtcl",
        include_str!(
            "../../../../rust/tcl-registry/tests/data/native_lambda_object_getter/jim/stdout.tsv"
        ),
    ),
];

#[test]
fn original_lambda_namespace_objects_match_constructor_and_getter_controls() {
    // naming.lambda.original-namespace-constructor-and-getter
    // docs/design/analysis/name-resolution-proofs/lambda-original-namespace-constructor-and-getter.md
    // Compare actual original object-vector completion/result and the retained
    // namespace object's subsequent getter. BEFORE/AFTER header rows are not
    // compared; Jim has no tested native ByteArray constructor.
    let mut comparisons = 0;
    for (engine, rows) in LAMBDA_NAMESPACE_ROWS {
        for (name, original) in LAMBDA_NAMESPACE_INPUTS {
            for byte_array in [false, true] {
                if byte_array && engine == "jimtcl" {
                    continue;
                }
                let constructor = if byte_array {
                    "BYTE_ARRAY"
                } else {
                    "RAW_STRING"
                };
                let apply_label = format!("{constructor}_{name}_APPLY");
                let getter_label = format!("{constructor}_{name}_GETTER");
                let native_apply = row(rows, &apply_label);
                let native_getter = row(rows, &getter_label);
                let mut interp = fresh(engine);
                assert_eq!(interp.eval_str(b"namespace eval ns {}; namespace eval :ns {}; namespace eval \\u00e9 {}; namespace eval :\\u00e9 {}"), Code::Ok);
                let namespace = Owned::fresh(if byte_array {
                    crate::bytearray::new_byte_array(
                        original,
                        interp
                            .native_invocation_dialect()
                            .byte_array_string_recipe(None)
                            .unwrap(),
                    )
                } else {
                    obj::new_string_bytes(original)
                });
                let parameters = Owned::fresh(obj::new_string_bytes(b""));
                let body = Owned::fresh(obj::new_string_bytes(b"namespace current"));
                let protocol = interp
                    .native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap();
                let lambda = Owned::fresh(crate::list::new_list_obj_native(
                    &[parameters.as_ptr(), body.as_ptr(), namespace.as_ptr()],
                    protocol,
                ));
                let code = call(&mut interp, b"apply", &[lambda.as_ptr()]);
                assert_eq!(
                    code.as_int(),
                    native_apply[1].parse::<i64>().unwrap(),
                    "{engine}/{apply_label}"
                );
                assert_eq!(
                    interp.result_bytes(),
                    unhex(native_apply[2]),
                    "{engine}/{apply_label}"
                );
                assert_eq!(
                    interp
                        .native_object_string_bytes(namespace.as_ptr())
                        .unwrap()
                        .as_ref(),
                    unhex(native_getter[2]),
                    "{engine}/{getter_label}"
                );
                assert!(!interp.host_refusal_pending(), "{engine}/{apply_label}");
                comparisons += 1;
            }
        }
    }
    assert_eq!(comparisons, 81);
}
