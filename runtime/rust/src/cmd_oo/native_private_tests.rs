// SPDX-License-Identifier: AGPL-3.0-or-later
//! Private original-body bytes and actual mode queries on the native runtime.
use super::*;
use crate::obj::Owned;

#[test]
fn private_query_and_opaque_original_bodies_match_native_c9_controls() {
    // Native proof: naming.tcloo.private-definition-mode
    // docs/design/analysis/name-resolution-proofs/private-definition-mode.md
    // Native proof: naming.tcloo.private-original-script-bytes
    // docs/design/analysis/name-resolution-proofs/private-original-script-bytes.md
    for engine in ["tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(b"oo::class create C {set ::before [private];private {set ::inside [private]};set ::after [private]};list $::before $::inside $::after"), Code::Ok);
        assert_eq!(interp.result_bytes(), b"0 1 0");
        assert_eq!(interp.oo.borrow().private_depth, 0);
        assert_eq!(interp.eval_str(b"::oo::define::private"), Code::Error);
        for name in [b"p\xff".as_slice(), b"p\xed\xa0\x80", b"p\0tail"] {
            assert_eq!(
                interp.eval_str(b"rename C {};oo::class create C {}"),
                Code::Ok
            );
            let mut body = b"private {method {".to_vec();
            body.extend_from_slice(name);
            body.extend_from_slice(b"} {} {return OK}};method invoke {} {my {");
            body.extend_from_slice(name);
            body.extend_from_slice(b"}}");
            let words = [b"oo::define".as_slice(), b"C", &body]
                .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)));
            let arguments: Vec<_> = words.iter().map(Owned::as_ptr).collect();
            assert_eq!(interp.dispatch(&arguments), Code::Ok, "{engine}/{name:?}");
            assert_eq!(interp.eval_str(b"info class methods C -private"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"invoke");
            assert_eq!(interp.eval_str(b"C create c;c invoke"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"OK");
            assert_eq!(interp.eval_str(b"c destroy"), Code::Ok);
            assert!(!interp.host_refusal_pending());
        }
    }
}

fn assert_row(interp: &Interp, code: Code, rows: &str, label: &str) {
    let fields: Vec<_> = rows
        .lines()
        .find(|row| row.starts_with(&format!("{label}|")))
        .unwrap()
        .split('|')
        .collect();
    assert_eq!(code.as_int(), fields[1].parse::<i64>().unwrap(), "{label}");
    let expected: Vec<_> = fields[2]
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(interp.result_bytes(), expected, "{label}");
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
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../rust/tcl-vm/tests/data/native_method_visibility/v2/9.1.0/stdout.tsv"
            ),
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
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(
                interp.eval_str(
                    b"oo::class create C {};oo::define C method invoke {} {my p};C create c"
                ),
                Code::Ok
            );
            let originals = [
                b"oo::define".as_slice(),
                b"C",
                b"method",
                b"p",
                option,
                b"",
                b"return OK",
            ]
            .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)));
            let arguments: Vec<_> = originals.iter().map(Owned::as_ptr).collect();
            let result = interp.dispatch(&arguments);
            assert_row(&interp, result, rows, &format!("{label}_DEFINE"));
            if result == Code::Ok {
                if label.starts_with("PRIVATE") {
                    let result = interp.eval_str(b"info class methods C -private");
                    assert_row(&interp, result, rows, &format!("{label}_NAMES"));
                }
                let result = interp.eval_str(b"c p");
                assert_row(&interp, result, rows, &format!("{label}_EXTERNAL"));
                let result = interp.eval_str(b"c invoke");
                assert_row(&interp, result, rows, &format!("{label}_INTERNAL"));
            }
        }
        for row in
            include_str!("../../../../rust/tcl-vm/tests/data/native_method_visibility/cases.tsv")
                .lines()
        {
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
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let result = interp.eval_str(source.as_bytes());
            if label == "SUBCLASS_PRIVATE_MISS" {
                let expected = rows
                    .lines()
                    .find(|row| row.starts_with("SUBCLASS_PRIVATE_MISS|"))
                    .unwrap()
                    .split('|')
                    .nth(1)
                    .unwrap()
                    .parse::<i64>()
                    .unwrap();
                assert_eq!(result.as_int(), expected);
            } else {
                assert_row(&interp, result, rows, label);
            }
            assert!(!interp.host_refusal_pending(), "{engine}/{label}");
        }
    }
}
