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
fn stock_destroy_allocation_controls_reflection_and_dispatch() {
    // Native proof: naming.tcloo.stock-destroy-method-allocation
    // docs/design/analysis/name-resolution-proofs/tcloo-stock-destroy-method-allocation.md
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        let before = vm.eval_source(
            "oo::class create C {}; C create O; list [info class methods C -all] [info object methods O -all]",
        ).unwrap();
        assert_eq!(before.code, Code::Ok, "{engine}: {before:?}");
        assert_eq!(
            before.result.string_bytes().as_ref(),
            b"destroy destroy",
            "{engine}"
        );
        let removed = vm.eval_source(
            "oo::define ::oo::object deletemethod destroy; list [info class methods C -all] [info object methods O -all] [catch {O destroy} r] $r",
        ).unwrap();
        assert_eq!(removed.code, Code::Ok, "{engine}: {removed:?}");
        assert_eq!(
            removed.result.string_bytes().as_ref(),
            b"{} {} 1 {object \"::O\" has no visible methods}",
            "{engine}"
        );
    }
}

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
                "../../../tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0/stdout.tsv"
            ),
        ),
    ] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        assert_eq!(vm.eval_source("oo::class create B {method inherited {} {}; export inherited}; oo::class create C {superclass B; method Public {} {}; export Public; method hidden {} {}; unexport hidden}; C create O; oo::objdefine O {method OPub {} {}; export OPub; method ohide {} {}; unexport ohide}").unwrap().code, Code::Ok);
        if engine != "tcl8.6" {
            assert_eq!(vm.eval_source("oo::define C private method Secret {} {}; oo::objdefine O private method OSecret {} {}").unwrap().code, Code::Ok);
        }
        for (kind, category, target) in [
            (InfoOoEnsembleKind::Object, "object", b"O".as_slice()),
            (InfoOoEnsembleKind::Class, "class", b"C".as_slice()),
        ] {
            for &(case, options, missing) in CASES {
                let mut args = vec![
                    Value::string("methods"),
                    Value::new_native_string_bytes(if missing { b"::missing" } else { target }),
                ];
                args.extend(
                    options
                        .iter()
                        .map(|bytes| Value::new_native_string_bytes(*bytes)),
                );
                let completion = match kind {
                    InfoOoEnsembleKind::Object => info_object(&mut vm, &args),
                    InfoOoEnsembleKind::Class => info_class(&mut vm, &args),
                };
                let label = format!("DIRECT_{category}_{case}");
                let (code, expected) = captured(rows, &label);
                assert_eq!(completion.code.as_int(), code, "{engine}/{label}");
                if code == 0 {
                    let mut actual = vm
                        .list_elements(&completion.result)
                        .unwrap()
                        .iter()
                        .map(|value| value.string_bytes().to_vec())
                        .collect::<Vec<_>>();
                    let value = Value::new_native_string_bytes(expected);
                    let mut expected = vm
                        .list_elements(&value)
                        .unwrap()
                        .iter()
                        .map(|value| value.string_bytes().to_vec())
                        .collect::<Vec<_>>();
                    actual.sort();
                    expected.sort();
                    assert_eq!(actual, expected, "{engine}/{label}");
                } else {
                    assert_eq!(
                        completion.result.string_bytes().as_ref(),
                        expected,
                        "{engine}/{label}"
                    );
                }
            }
        }
    }
}
