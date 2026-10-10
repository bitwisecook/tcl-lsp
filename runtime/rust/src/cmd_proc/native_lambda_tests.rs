// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original lambda conversion diagnostics, independent of successful activation.

fn bytes(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn row<'a>(rows: &'a str, label: &str) -> Vec<&'a str> {
    rows.lines()
        .find(|row| row.split('|').next() == Some(label))
        .unwrap()
        .split('|')
        .collect()
}

#[test]
fn lambda_conversion_errors_match_original_counted_values_and_parameter_frames() {
    // Native proof: naming.lambda.original-conversion-diagnostic
    // docs/design/analysis/name-resolution-proofs/lambda-original-conversion-diagnostic.md
    // Native proof: naming.lambda.original-parameter-error-frame
    // docs/design/analysis/name-resolution-proofs/lambda-original-parameter-error-frame.md
    for (engine, captured) in [
        (
            "tcl8.5",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.1.0/stdout.tsv"
            ),
        ),
        (
            "jimtcl",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/jim/stdout.tsv"
            ),
        ),
    ] {
        for input in captured.lines().filter(|line| {
            line.starts_with("DIRECT_") && line.split('|').next().unwrap().ends_with("_INPUT")
        }) {
            let fields: Vec<_> = input.split('|').collect();
            let label = fields[0].strip_suffix("_INPUT").unwrap();
            let native = row(captured, label);
            if native[1] == "0" {
                continue;
            } // Successful activation has its own native object/cache contract.
            let original = bytes(fields[2]);
            let mut interp = crate::interp::Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let head = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"apply"));
            let lambda = crate::obj::Owned::fresh(crate::obj::new_string_bytes(&original));
            let code = super::apply_cmd(&mut interp, &[head.as_ptr(), lambda.as_ptr()]);
            assert_eq!(
                code.as_int(),
                native[1].parse::<i64>().unwrap(),
                "{engine}/{label}"
            );
            assert_eq!(interp.result_bytes(), bytes(native[2]), "{engine}/{label}");
            assert!(!interp.host_refusal_pending(), "{engine}/{label}");
            if engine != "jimtcl" {
                let expected = row(captured, &format!("{label}_ERRORCODE"));
                assert_eq!(
                    interp.error_code(),
                    bytes(expected[2]),
                    "{engine}/{label}/errorcode"
                );
                if label == "DIRECT_PARAM_BODY_RAW_ZERO" {
                    let frame = b"\n    (parsing lambda expression \"{{a b c}} {x\")";
                    let actual = interp.error_info();
                    assert!(
                        actual.windows(frame.len()).any(|part| part == frame),
                        "{engine}/CString parameter frame"
                    );
                }
            }
        }
    }
}
