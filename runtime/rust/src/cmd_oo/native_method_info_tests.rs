// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original counted method options, local scope and target lookup boundaries.
use super::*;
use tcl_syntax::value::ValueOps;

fn captured(rows: &str, label: &str) -> (i64, Vec<u8>) {
    let mut fields = rows
        .lines()
        .find(|row| row.split('|').next() == Some(label))
        .unwrap()
        .split('|');
    fields.next();
    let code = fields.next().unwrap().parse().unwrap();
    let bytes = fields
        .next()
        .unwrap()
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    (code, bytes)
}

const CASES: &[(&str, &[&[u8]], bool)] = &[
    ("BASE", &[], false),
    ("ALL", &[b"\x2d\x61\x6c\x6c"], false),
    ("ALL_PREFIX", &[b"\x2d\x61"], false),
    (
        "LOCALPRIVATE",
        &[b"\x2d\x6c\x6f\x63\x61\x6c\x70\x72\x69\x76\x61\x74\x65"],
        false,
    ),
    ("PRIVATE", &[b"\x2d\x70\x72\x69\x76\x61\x74\x65"], false),
    ("PRIVATE_PREFIX", &[b"\x2d\x70"], false),
    (
        "PRIVATE_RAW_ZERO",
        &[b"\x2d\x70\x72\x69\x76\x61\x74\x65\x00\xff"],
        false,
    ),
    (
        "PRIVATE_ENCODED_ZERO",
        &[b"\x2d\x70\x72\x69\x76\x61\x74\x65\xc0\x80"],
        false,
    ),
    ("AMBIGUOUS", &[b"\x2d"], false),
    (
        "BAD_RAW_ZERO",
        &[b"\x2d\x62\x61\x64\xff\x00\x54\x41\x49\x4c"],
        false,
    ),
    (
        "SCOPE_PUBLIC",
        &[b"\x2d\x73\x63\x6f\x70\x65", b"\x70\x75\x62\x6c\x69\x63"],
        false,
    ),
    (
        "SCOPE_UNEXPORTED",
        &[
            b"\x2d\x73\x63\x6f\x70\x65",
            b"\x75\x6e\x65\x78\x70\x6f\x72\x74\x65\x64",
        ],
        false,
    ),
    (
        "SCOPE_PRIVATE",
        &[b"\x2d\x73\x63\x6f\x70\x65", b"\x70\x72\x69\x76\x61\x74\x65"],
        false,
    ),
    ("SCOPE_AMBIGUOUS", &[b"\x2d\x73", b"\x70"], false),
    (
        "SCOPE_RAW_ZERO",
        &[
            b"\x2d\x73\x63\x6f\x70\x65\x00\x58",
            b"\x70\x75\x62\x6c\x69\x63\x00\xff",
        ],
        false,
    ),
    (
        "SCOPE_ENCODED_ZERO",
        &[
            b"\x2d\x73\x63\x6f\x70\x65",
            b"\x70\x75\x62\x6c\x69\x63\xc0\x80",
        ],
        false,
    ),
    ("SCOPE_MISSING", &[b"\x2d\x73\x63\x6f\x70\x65"], false),
    (
        "SCOPE_BAD",
        &[b"\x2d\x73\x63\x6f\x70\x65", b"\x2d\x61\x6c\x6c"],
        false,
    ),
    (
        "SCOPE_LAST",
        &[
            b"\x2d\x73\x63\x6f\x70\x65",
            b"\x70\x75\x62\x6c\x69\x63",
            b"\x2d\x73\x63\x6f\x70\x65",
            b"\x75\x6e\x65\x78\x70\x6f\x72\x74\x65\x64",
        ],
        false,
    ),
    (
        "SCOPE_THEN_ALL",
        &[
            b"\x2d\x73\x63\x6f\x70\x65",
            b"\x75\x6e\x65\x78\x70\x6f\x72\x74\x65\x64",
            b"\x2d\x61\x6c\x6c",
        ],
        false,
    ),
    (
        "ALL_THEN_SCOPE",
        &[
            b"\x2d\x61\x6c\x6c",
            b"\x2d\x73\x63\x6f\x70\x65",
            b"\x75\x6e\x65\x78\x70\x6f\x72\x74\x65\x64",
        ],
        false,
    ),
    (
        "LOCAL_THEN_PRIVATE",
        &[
            b"\x2d\x6c\x6f\x63\x61\x6c\x70\x72\x69\x76\x61\x74\x65",
            b"\x2d\x70\x72\x69\x76\x61\x74\x65",
        ],
        false,
    ),
    (
        "PRIVATE_THEN_LOCAL",
        &[
            b"\x2d\x70\x72\x69\x76\x61\x74\x65",
            b"\x2d\x6c\x6f\x63\x61\x6c\x70\x72\x69\x76\x61\x74\x65",
        ],
        false,
    ),
    (
        "MISSING_BAD",
        &[b"\x2d\x62\x61\x64\xff\x00\x54\x41\x49\x4c"],
        true,
    ),
    ("MISSING_SCOPE", &[b"\x2d\x73\x63\x6f\x70\x65"], true),
];

#[test]
fn method_info_matches_original_counted_option_and_scope_controls() {
    // Native proof: naming.tcloo.method-info-option-selection
    // docs/design/analysis/name-resolution-proofs/method-info-option-selection.md
    // Native proof: naming.tcloo.method-info-scope-ordering
    // docs/design/analysis/name-resolution-proofs/method-info-scope-ordering.md
    for (engine, rows) in [
        (
            "tcl8.6",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0/stdout.tsv"
            ),
        ),
    ] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(b"\x6f\x6f\x3a\x3a\x63\x6c\x61\x73\x73\x20\x63\x72\x65\x61\x74\x65\x20\x42\x20\x7b\x6d\x65\x74\x68\x6f\x64\x20\x69\x6e\x68\x65\x72\x69\x74\x65\x64\x20\x7b\x7d\x20\x7b\x7d\x3b\x20\x65\x78\x70\x6f\x72\x74\x20\x69\x6e\x68\x65\x72\x69\x74\x65\x64\x7d\x3b\x20\x6f\x6f\x3a\x3a\x63\x6c\x61\x73\x73\x20\x63\x72\x65\x61\x74\x65\x20\x43\x20\x7b\x73\x75\x70\x65\x72\x63\x6c\x61\x73\x73\x20\x42\x3b\x20\x6d\x65\x74\x68\x6f\x64\x20\x50\x75\x62\x6c\x69\x63\x20\x7b\x7d\x20\x7b\x7d\x3b\x20\x65\x78\x70\x6f\x72\x74\x20\x50\x75\x62\x6c\x69\x63\x3b\x20\x6d\x65\x74\x68\x6f\x64\x20\x68\x69\x64\x64\x65\x6e\x20\x7b\x7d\x20\x7b\x7d\x3b\x20\x75\x6e\x65\x78\x70\x6f\x72\x74\x20\x68\x69\x64\x64\x65\x6e\x7d\x3b\x20\x43\x20\x63\x72\x65\x61\x74\x65\x20\x4f\x3b\x20\x6f\x6f\x3a\x3a\x6f\x62\x6a\x64\x65\x66\x69\x6e\x65\x20\x4f\x20\x7b\x6d\x65\x74\x68\x6f\x64\x20\x4f\x50\x75\x62\x20\x7b\x7d\x20\x7b\x7d\x3b\x20\x65\x78\x70\x6f\x72\x74\x20\x4f\x50\x75\x62\x3b\x20\x6d\x65\x74\x68\x6f\x64\x20\x6f\x68\x69\x64\x65\x20\x7b\x7d\x20\x7b\x7d\x3b\x20\x75\x6e\x65\x78\x70\x6f\x72\x74\x20\x6f\x68\x69\x64\x65\x7d"),Code::Ok);
        if engine != "tcl8.6" {
            assert_eq!(interp.eval_str(b"oo::define C private method Secret {} {}; oo::objdefine O private method OSecret {} {}"),Code::Ok);
        }
        for (kind, category, target) in [
            (InfoOoEnsembleKind::Object, "object", b"O".as_slice()),
            (InfoOoEnsembleKind::Class, "class", b"C".as_slice()),
        ] {
            for &(case, options, missing) in CASES {
                let mut args = vec![
                    obj::Owned::fresh(obj::new_string_bytes(b"info")),
                    obj::Owned::fresh(obj::new_string_bytes(category.as_bytes())),
                    obj::Owned::fresh(obj::new_string_bytes(b"methods")),
                    obj::Owned::fresh(obj::new_string_bytes(if missing {
                        b"::missing"
                    } else {
                        target
                    })),
                ];
                args.extend(
                    options
                        .iter()
                        .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes))),
                );
                let argv = args.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                let actual_code = match kind {
                    InfoOoEnsembleKind::Object => info_object(&mut interp, &argv),
                    InfoOoEnsembleKind::Class => info_class(&mut interp, &argv),
                };
                let label = format!("DIRECT_{category}_{case}");
                let (code, expected) = captured(rows, &label);
                assert_eq!(actual_code.as_int(), code, "{engine}/{label}");
                if code == 0 {
                    let result = interp.result_obj();
                    let mut actual = interp
                        .list_elements(&result)
                        .unwrap()
                        .iter()
                        .map(|value| interp.native_string_bytes(value).unwrap().to_vec())
                        .collect::<Vec<_>>();
                    let owned = obj::Owned::fresh(obj::new_string_bytes(&expected));
                    let mut expected = interp
                        .list_elements(&owned.as_ptr())
                        .unwrap()
                        .iter()
                        .map(|value| interp.native_string_bytes(value).unwrap().to_vec())
                        .collect::<Vec<_>>();
                    actual.sort();
                    expected.sort();
                    assert_eq!(actual, expected, "{engine}/{label}");
                } else {
                    assert_eq!(interp.result_bytes(), expected, "{engine}/{label}");
                }
            }
        }
    }
}
