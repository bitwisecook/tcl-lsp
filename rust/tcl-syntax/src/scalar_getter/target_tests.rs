// SPDX-License-Identifier: AGPL-3.0-or-later
//! Primitive result/layout comparisons, separately from expression truth.

use super::*;

const C_LIVE: [&str; 5] = [
    include_str!(
        "../../../tcl-registry/tests/data/native_capi_scalar_publication_original/8.4.20/execute-live.stdout"
    ),
    include_str!(
        "../../../tcl-registry/tests/data/native_capi_scalar_publication_original/8.5.19/execute-live.stdout"
    ),
    include_str!(
        "../../../tcl-registry/tests/data/native_capi_scalar_publication_original/8.6.18/execute-live.stdout"
    ),
    include_str!(
        "../../../tcl-registry/tests/data/native_capi_scalar_publication_original/9.0.4/execute-live.stdout"
    ),
    include_str!(
        "../../../tcl-registry/tests/data/native_capi_scalar_publication_original/9.1.0/execute-live.stdout"
    ),
];
const C_NULL: [&str; 5] = [
    include_str!(
        "../../../tcl-registry/tests/data/native_capi_scalar_publication_original/8.4.20/execute-null.stdout"
    ),
    include_str!(
        "../../../tcl-registry/tests/data/native_capi_scalar_publication_original/8.5.19/execute-null.stdout"
    ),
    include_str!(
        "../../../tcl-registry/tests/data/native_capi_scalar_publication_original/8.6.18/execute-null.stdout"
    ),
    include_str!(
        "../../../tcl-registry/tests/data/native_capi_scalar_publication_original/9.0.4/execute-null.stdout"
    ),
    include_str!(
        "../../../tcl-registry/tests/data/native_capi_scalar_publication_original/9.1.0/execute-null.stdout"
    ),
];
const JIM: &str = include_str!(
    "../../../tcl-registry/tests/data/native_capi_scalar_publication_original/linked-jim458/jim/execute-live.stdout"
);

fn field<'a>(line: &'a str, key: &str) -> &'a str {
    line.split('\t')
        .find_map(|field| {
            let (name, value) = field.split_once('=')?;
            (name == key).then_some(value)
        })
        .unwrap_or_else(|| panic!("missing {key}: {line}"))
}
// The older callback capture has space-delimited key/value fields; the public
// primitive capture above has tab-delimited fields. Their record formats and
// observation boundaries are independent.
fn callback_field<'a>(line: &'a str, key: &str) -> &'a str {
    line.split_ascii_whitespace()
        .find_map(|field| {
            let (name, value) = field.split_once('=')?;
            (name == key).then_some(value)
        })
        .unwrap_or_else(|| panic!("missing {key}: {line}"))
}
fn target(fixture: &str) -> NativeScalarGetterTarget {
    let abi = fixture
        .lines()
        .find(|line| line.starts_with("ABI\t"))
        .unwrap();
    NativeScalarGetterTarget::from_c_integer_abi(
        field(abi, "CHAR_BIT").parse().unwrap(),
        field(abi, "int").parse().unwrap(),
        field(abi, "long").parse().unwrap(),
    )
    .unwrap()
}
fn row(fixture: &str, case: usize, getter: usize) -> &str {
    fixture
        .lines()
        .find(|line| {
            line.starts_with("ROW\t")
                && field(line, "case").parse::<usize>().unwrap() == case
                && field(line, "getter").parse::<usize>().unwrap() == getter
        })
        .unwrap()
}
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        write!(text, "{byte:02x}").unwrap();
        text
    })
}

#[test]
fn original_jim_cached_boolean_returns_the_measured_public_integer() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // These two public primitive rows describe raw output and cache effects;
    // they do not answer the independent ExprBool or command-condition question.
    let protocol =
        NativeScalarGetterProtocol::for_point(DialectPoint::canonical(Release::JIM_0_84)).unwrap();
    let target = target(JIM);
    for (case, integer) in [(20, 17), (26, 4_294_967_296)] {
        let row = row(JIM, case, 4);
        let cache = NativeScalarCache::Number(Number::Int(integer));
        let conversion = protocol
            .cached_conversion(NativeScalarGetterKind::Boolean, &cache, Some(target))
            .unwrap()
            .unwrap();
        let Ok(NativeScalarGetterValue::Boolean(boolean)) = conversion.outcome() else {
            panic!("{row}")
        };
        assert_eq!(field(row, "code"), "0");
        assert_eq!(
            boolean.returned_integer().to_string(),
            field(row, "int"),
            "{row}"
        );
        assert_eq!(field(row, "before"), "int");
        assert_eq!(field(row, "after"), "int");
        assert!(conversion.cache().is_none());
        assert!(!conversion.requires_string_materialization());
        assert_eq!(field(row, "string_before"), "0");
        assert_eq!(field(row, "string_after"), "0");
    }
}

#[test]
fn primitive_target_absence_is_terminal_and_does_not_select_fresh_parsing() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // Software availability contract, not an additional Native capture.
    for fields in [(8, 4, 4), (8, 8, 8), (16, 4, 8), (0, 4, 8)] {
        assert!(
            NativeScalarGetterTarget::from_c_integer_abi(fields.0, fields.1, fields.2).is_err()
        );
    }
    let jim =
        NativeScalarGetterProtocol::for_point(DialectPoint::canonical(Release::JIM_0_84)).unwrap();
    let cache = NativeScalarCache::Number(Number::Int(17));
    assert_eq!(
        jim.cached_conversion(NativeScalarGetterKind::Boolean, &cache, None),
        Err(NativeScalarGetterTargetUnavailable)
    );
    assert_eq!(
        jim.cached_conversion(NativeScalarGetterKind::Long, &cache, None),
        Err(NativeScalarGetterTargetUnavailable)
    );
    assert_eq!(
        jim.fresh_conversion_with_target(NativeScalarGetterKind::Long, b"17", None),
        Err(NativeScalarGetterTargetUnavailable)
    );
    assert!(
        jim.fresh_conversion_with_range_error(NativeScalarGetterKind::Long, b"17", false)
            .is_none()
    );
    let c = NativeScalarGetterProtocol::for_tcl_version(TclVersion::V8_6);
    let conversion = c
        .cached_conversion(NativeScalarGetterKind::Boolean, &cache, None)
        .unwrap()
        .unwrap();
    assert_eq!(
        conversion.outcome(),
        Ok(NativeScalarGetterValue::Boolean(
            NativeBooleanGetterValue::normalized(true)
        ))
    );
}

fn input(case: usize, version: TclVersion) -> (Vec<u8>, Option<NativeScalarCache>) {
    match case {
        0 => (b"".to_vec(), None),
        8 => (b"9223372036854775808".to_vec(), None),
        9 => (b"18446744073709551615".to_vec(), None),
        10 => (b"18446744073709551616".to_vec(), None),
        11 => (b"-18446744073709551615".to_vec(), None),
        12 => (b"-9223372036854775809".to_vec(), None),
        13 => (b"08".to_vec(), None),
        16 => (b"bad".to_vec(), None),
        20 | 21 => (
            b"17".to_vec(),
            Some(if version == TclVersion::V8_4 {
                NativeScalarCache::Tcl84Long(17)
            } else {
                NativeScalarCache::Number(Number::Int(17))
            }),
        ),
        22 => (
            b"17".to_vec(),
            Some(NativeScalarCache::Number(Number::Int(17))),
        ),
        23 => (
            b"17.0".to_vec(),
            Some(NativeScalarCache::Number(Number::Double(17.0))),
        ),
        24 => (
            (if version == TclVersion::V8_4 {
                b"nan"
            } else {
                b"NaN"
            })
            .to_vec(),
            Some(NativeScalarCache::Number(Number::Double(f64::NAN))),
        ),
        26 => (
            b"4294967296".to_vec(),
            Some(NativeScalarCache::Number(Number::Int(4_294_967_296))),
        ),
        _ => panic!("unselected constructor {case}"),
    }
}
fn cache_type(cache: Option<&NativeScalarCache>) -> &'static str {
    match cache {
        None => "NULL",
        Some(NativeScalarCache::Tcl84Long(_)) => "int",
        Some(NativeScalarCache::Number(Number::Int(_))) => "integer",
        Some(NativeScalarCache::Number(Number::Big { .. })) => "bignum",
        Some(NativeScalarCache::Number(Number::Double(_) | Number::Nan { .. })) => "double",
        _ => panic!("unselected cache"),
    }
}

#[test]
fn original_long64_public_value_cache_and_live_failure_fields_match_captures() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // Exact selected Long rows from both modes. Live failures compare primitive
    // result/errorCode; null rows compare conversion/cache, without publication.
    // Target fields come from each original ABI row, not its profile label.
    // C8.4 invalid Long retains the seeded code; overflow replaces it. These
    // direct public calls do not sample a later callback/Tcl_Eval projection.
    let versions = [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ];
    let cases = [0, 8, 9, 10, 11, 12, 13, 16, 20, 21, 22, 23, 24, 26];
    let mut compared = 0;
    for (index, version) in versions.into_iter().enumerate() {
        for fixture in [C_LIVE[index], C_NULL[index]] {
            let target = target(fixture);
            let protocol = NativeScalarGetterProtocol::for_tcl_version(version);
            for case in cases {
                let row = row(fixture, case, 1);
                let (bytes, prior) = input(case, version);
                let cached = prior
                    .as_ref()
                    .map(|cache| {
                        protocol.cached_conversion(
                            NativeScalarGetterKind::Long,
                            cache,
                            Some(target),
                        )
                    })
                    .transpose()
                    .unwrap()
                    .flatten();
                let conversion = cached.unwrap_or_else(|| {
                    protocol
                        .fresh_conversion_with_target(
                            NativeScalarGetterKind::Long,
                            &bytes,
                            Some(target),
                        )
                        .unwrap()
                        .unwrap()
                });
                assert_eq!(
                    conversion.outcome().is_ok(),
                    field(row, "code") == "0",
                    "{version:?}: {row}"
                );
                if let Ok(NativeScalarGetterValue::Wide(value)) = conversion.outcome() {
                    assert_eq!(value.to_string(), field(row, "long"), "{row}");
                } else if field(row, "mode") == "live" {
                    let failure = conversion.outcome().unwrap_err();
                    let presentation = protocol
                        .failure_presentation(NativeScalarGetterKind::Long, failure, &bytes)
                        .unwrap_or_else(|| panic!("{version:?} {failure:?}: {row}"));
                    assert_eq!(
                        hex(presentation.message_bytes()),
                        field(row, "result"),
                        "{row}"
                    );
                    let expected_code = match presentation.error_code_update() {
                        NativeScalarGetterErrorCode::Unchanged => b"SEEDED CODE".as_slice(),
                        NativeScalarGetterErrorCode::Set(code) => code.as_slice(),
                    };
                    assert_eq!(hex(expected_code), field(row, "error_code"), "{row}");
                }
                let cache = conversion.cache().or(prior.as_ref());
                let actual_kind = cache_type(cache);
                let expected_kind = match field(row, "after") {
                    "wideInt" | "int" if actual_kind == "integer" => "integer",
                    name => name,
                };
                assert_eq!(actual_kind, expected_kind, "{row}");
                // A fresh Long result can wrap while retaining its full Big
                // cache; a subsequent cached Long preserves that same primary.
                if let Some(cache) = cache {
                    if version != TclVersion::V8_4 {
                        let again = protocol
                            .cached_conversion(NativeScalarGetterKind::Long, cache, Some(target))
                            .unwrap()
                            .unwrap();
                        assert_eq!(again.outcome(), conversion.outcome(), "{row}");
                        assert!(again.cache().is_none());
                    }
                }
                compared += 1;
            }
        }
    }
    assert_eq!(compared, 140);
}

#[test]
fn original_long_octal_failure_keeps_its_integer_purpose() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // Seven original live failures separate the integer-only Long parser from
    // Double/Boolean hints. Two C9 Long successes retain their measured cache.
    // These public-field comparisons supply no object or expression authority.
    let mut failures = 0;
    for (index, version, kinds) in [
        (
            0,
            TclVersion::V8_4,
            [Some(NativeScalarGetterKind::Long), None, None],
        ),
        (
            1,
            TclVersion::V8_5,
            [
                Some(NativeScalarGetterKind::Long),
                Some(NativeScalarGetterKind::Double),
                Some(NativeScalarGetterKind::Boolean),
            ],
        ),
        (
            2,
            TclVersion::V8_6,
            [
                Some(NativeScalarGetterKind::Long),
                Some(NativeScalarGetterKind::Double),
                Some(NativeScalarGetterKind::Boolean),
            ],
        ),
    ] {
        let fixture = C_LIVE[index];
        let protocol = NativeScalarGetterProtocol::for_tcl_version(version);
        for kind in kinds.into_iter().flatten() {
            let getter = match kind {
                NativeScalarGetterKind::Long => 1,
                NativeScalarGetterKind::Double => 3,
                NativeScalarGetterKind::Boolean => 4,
                _ => unreachable!(),
            };
            let row = row(fixture, 13, getter);
            let conversion = protocol
                .fresh_conversion_with_target(kind, b"08", Some(target(fixture)))
                .unwrap()
                .unwrap();
            assert_eq!(field(row, "code"), "1", "{row}");
            let failure = conversion.outcome().unwrap_err();
            assert_eq!(
                failure,
                if version == TclVersion::V8_4 || kind != NativeScalarGetterKind::Long {
                    NativeScalarGetterFailure::InvalidOctal
                } else {
                    NativeScalarGetterFailure::Invalid
                },
                "{row}"
            );
            let presentation = protocol
                .failure_presentation(kind, failure, b"08")
                .unwrap_or_else(|| panic!("{version:?} {kind:?} {failure:?}: {row}"));
            assert_eq!(
                hex(presentation.message_bytes()),
                field(row, "result"),
                "{row}"
            );
            let code = match presentation.error_code_update() {
                NativeScalarGetterErrorCode::Unchanged => b"SEEDED CODE".as_slice(),
                NativeScalarGetterErrorCode::Set(code) => code.as_slice(),
            };
            assert_eq!(hex(code), field(row, "error_code"), "{row}");
            assert!(conversion.cache().is_none(), "{row}");
            assert_eq!(field(row, "after"), "NULL", "{row}");
            failures += 1;
        }
    }
    assert_eq!(failures, 7);
    let mut successes = 0;
    for (index, version) in [(3, TclVersion::V9_0), (4, TclVersion::V9_1)] {
        let fixture = C_LIVE[index];
        let row = row(fixture, 13, 1);
        let conversion = NativeScalarGetterProtocol::for_tcl_version(version)
            .fresh_conversion_with_target(
                NativeScalarGetterKind::Long,
                b"08",
                Some(target(fixture)),
            )
            .unwrap()
            .unwrap();
        let Ok(NativeScalarGetterValue::Wide(value)) = conversion.outcome() else {
            panic!("{row}")
        };
        assert_eq!(field(row, "code"), "0", "{row}");
        assert_eq!(value.to_string(), field(row, "long"), "{row}");
        assert_eq!(cache_type(conversion.cache()), "integer", "{row}");
        assert_eq!(field(row, "after"), "int", "{row}");
        successes += 1;
    }
    assert_eq!(successes, 2);
}

#[test]
fn original_c84_primitive_code_and_callback_eval_code_are_separate_observations() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // naming.numeric.seeded-wide-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-seeded-wide-frontier.md
    // naming.numeric.seeded-double-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-seeded-double-frontier.md
    // naming.numeric.seeded-boolean-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-seeded-boolean-frontier.md
    // Five original public primitive fields and three independent final Eval
    // fields. The Int/Long Eval recipe is bounded by the shared original C
    // reset/error-log source, not an additional provider observation here.
    let eval_fixture =
        include_str!("../../tests/data/native_scalar_getters/errors/error-state-8.4.20.txt");
    let protocol = NativeScalarGetterProtocol::for_tcl_version(TclVersion::V8_4);
    let mut direct_fields = 0;
    let mut eval_fields = 0;
    for (kind, getter, callback_getter) in [
        (NativeScalarGetterKind::Int, 0, None),
        (NativeScalarGetterKind::Long, 1, None),
        (NativeScalarGetterKind::Wide, 2, Some("0")),
        (NativeScalarGetterKind::Double, 3, Some("1")),
        (NativeScalarGetterKind::Boolean, 4, Some("2")),
    ] {
        let direct = row(C_LIVE[0], 0, getter);
        let error = protocol
            .failure_presentation(kind, NativeScalarGetterFailure::Invalid, b"")
            .unwrap();
        assert_eq!(field(direct, "code"), "1", "{direct}");
        assert_eq!(hex(error.message_bytes()), field(direct, "result"));
        assert_eq!(
            error.error_code_update(),
            &NativeScalarGetterErrorCode::Unchanged
        );
        assert_eq!(hex(b"SEEDED CODE"), field(direct, "error_code"));
        direct_fields += 1;
        if let Some(callback_getter) = callback_getter {
            let propagated = eval_fixture
                .lines()
                .find(|line| {
                    callback_field(line, "getter") == callback_getter
                        && callback_field(line, "storage") == "0"
                        && callback_field(line, "case") == "9"
                })
                .unwrap();
            assert_eq!(callback_field(propagated, "input"), "", "{propagated}");
            assert_eq!(callback_field(propagated, "code"), "1", "{propagated}");
            let NativeScalarGetterErrorCode::Set(code) = error.eval_error_code_update() else {
                panic!("selected actual Eval update")
            };
            assert_eq!(
                hex(code),
                callback_field(propagated, "errorCode"),
                "{propagated}"
            );
            assert_eq!(
                hex(error.eval_message_bytes()),
                callback_field(propagated, "message")
            );
            eval_fields += 1;
        }
    }
    assert_eq!((direct_fields, eval_fields), (5, 3));
    let overflow = protocol
        .failure_presentation(
            NativeScalarGetterKind::Long,
            NativeScalarGetterFailure::IntegerOverflow,
            b"18446744073709551616",
        )
        .unwrap();
    let NativeScalarGetterErrorCode::Set(code) = overflow.error_code_update() else {
        panic!("selected overflow code")
    };
    assert_eq!(hex(code), field(row(C_LIVE[0], 10, 1), "error_code"));
    assert_eq!(
        overflow.eval_error_code_update(),
        &NativeScalarGetterErrorCode::Unchanged
    );
}

#[test]
fn native_long_unsigned_edge_is_independent_of_native_wide_acceptance() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    let fixture = C_LIVE[3];
    let target = target(fixture);
    let protocol = NativeScalarGetterProtocol::for_tcl_version(TclVersion::V9_0);
    for (case, bytes) in [
        (8, b"9223372036854775808".as_slice()),
        (9, b"18446744073709551615"),
    ] {
        let long = protocol
            .fresh_conversion_with_target(NativeScalarGetterKind::Long, bytes, Some(target))
            .unwrap()
            .unwrap();
        let wide = protocol
            .fresh_conversion(NativeScalarGetterKind::Wide, bytes)
            .unwrap();
        assert!(long.outcome().is_ok());
        assert!(wide.outcome().is_err());
        assert_eq!(field(row(fixture, case, 1), "code"), "0");
        assert_eq!(field(row(fixture, case, 2), "code"), "1");
        assert_eq!(long.cache(), wide.cache());
    }
}

#[test]
fn primitive_failure_string_obligation_refuses_operand_free_rendering_for_quoted_errors() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // Availability/purpose contract; the constant recipe supplies no original
    // bytes, while invalid/cached-noninteger diagnostics require their operand.
    let protocol = NativeScalarGetterProtocol::for_tcl_version(TclVersion::V8_6);
    for (kind, failure) in [
        (
            NativeScalarGetterKind::Boolean,
            NativeScalarGetterFailure::FloatingPointNaN,
        ),
        (
            NativeScalarGetterKind::Int,
            NativeScalarGetterFailure::IntWidthOverflow,
        ),
        (
            NativeScalarGetterKind::Long,
            NativeScalarGetterFailure::IntegerOverflow,
        ),
    ] {
        assert_eq!(
            protocol.failure_requires_original_string(kind, failure),
            Some(false)
        );
        assert!(
            protocol
                .failure_presentation_without_original_string(kind, failure)
                .is_some()
        );
    }
    for (kind, failure) in [
        (
            NativeScalarGetterKind::Boolean,
            NativeScalarGetterFailure::Invalid,
        ),
        (
            NativeScalarGetterKind::Long,
            NativeScalarGetterFailure::CachedNonInteger,
        ),
    ] {
        assert_eq!(
            protocol.failure_requires_original_string(kind, failure),
            Some(true)
        );
        assert!(
            protocol
                .failure_presentation_without_original_string(kind, failure)
                .is_none()
        );
    }
    let jim =
        NativeScalarGetterProtocol::for_point(DialectPoint::canonical(Release::JIM_0_84)).unwrap();
    assert_eq!(
        jim.failure_requires_original_string(
            NativeScalarGetterKind::Int,
            NativeScalarGetterFailure::Invalid
        ),
        None
    );
}

#[test]
fn original_c84_fresh_boolean_narrows_signed_long_before_truth_and_cached_wide_does_not() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // Public primitive value/cache fields only; expression and command truth
    // follow their own measured purposes. The original C84 library config owns
    // TCL_WIDE_INT_IS_LONG=1 independently of the probe translation unit macro.
    let protocol = NativeScalarGetterProtocol::for_tcl_version(TclVersion::V8_4);
    let original_strings: [&[u8]; 20] = [
        b"",
        b"17",
        b"2147483648",
        b"4294967295",
        b"4294967296",
        b"-4294967295",
        b"-2147483649",
        b"9223372036854775807",
        b"9223372036854775808",
        b"18446744073709551615",
        b"18446744073709551616",
        b"-18446744073709551615",
        b"-9223372036854775809",
        b"08",
        b"0o10",
        b"true",
        b"bad",
        b"NaN",
        b"1.0",
        b"1\0X",
    ];
    let library_config = include_str!(
        "../../../tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--unix--tclConfig.sh"
    );
    assert!(
        library_config
            .lines()
            .any(|line| line.starts_with("TCL_DEFS=") && line.contains("-DTCL_WIDE_INT_IS_LONG=1"))
    );
    let mut count = 0;
    for fixture in [C_LIVE[0], C_NULL[0]] {
        let target = target(fixture);
        for case in 0..29 {
            let (bytes, cache) = if case < 20 {
                (original_strings[case].to_vec(), None)
            } else {
                match case {
                    20 | 21 => (b"17".to_vec(), Some(NativeScalarCache::Tcl84Long(17))),
                    22 => (
                        b"17".to_vec(),
                        Some(NativeScalarCache::Number(Number::Int(17))),
                    ),
                    23 => (
                        b"17.0".to_vec(),
                        Some(NativeScalarCache::Number(Number::Double(17.0))),
                    ),
                    24 => (
                        b"nan".to_vec(),
                        Some(NativeScalarCache::Number(Number::Double(f64::NAN))),
                    ),
                    25 => (b"1".to_vec(), Some(NativeScalarCache::WordBoolean(true))),
                    26 => (
                        b"4294967296".to_vec(),
                        Some(NativeScalarCache::Number(Number::Int(4_294_967_296))),
                    ),
                    27 => (
                        protocol
                            .materialize(NativeScalarStringStorage::ByteArray, b"1\0X")
                            .unwrap()
                            .into_owned(),
                        None,
                    ),
                    28 => ([b'a'; 50].into_iter().chain("😀Z".bytes()).collect(), None),
                    _ => unreachable!(),
                }
            };
            let actual = cache
                .as_ref()
                .and_then(|cache| {
                    protocol
                        .cached_conversion(NativeScalarGetterKind::Boolean, cache, None)
                        .unwrap()
                })
                .unwrap_or_else(|| {
                    protocol
                        .fresh_conversion_with_target(
                            NativeScalarGetterKind::Boolean,
                            &bytes,
                            Some(target),
                        )
                        .unwrap()
                        .unwrap()
                });
            let observed = row(fixture, case, 4);
            assert_eq!(
                actual.outcome().is_ok(),
                field(observed, "code") == "0",
                "{observed}"
            );
            if let Ok(NativeScalarGetterValue::Boolean(value)) = actual.outcome() {
                assert_eq!(
                    value.returned_integer().to_string(),
                    field(observed, "int"),
                    "{observed}"
                );
                assert!(matches!(
                    actual.cache().or(cache.as_ref()),
                    Some(NativeScalarCache::WordBoolean(_))
                ));
                assert_eq!(field(observed, "after"), "boolean");
            } else {
                assert!(actual.cache().is_none());
                assert_eq!(field(observed, "after"), field(observed, "before"));
            }
            if (20..=26).contains(&case) {
                let expected_string = field(observed, "string_before") == "1"
                    || actual.requires_string_materialization();
                assert_eq!(
                    expected_string,
                    field(observed, "string_after") == "1",
                    "{observed}"
                );
            }
            count += 1;
        }
    }
    assert_eq!(count, 58);
    assert_eq!(field(row(C_LIVE[0], 4, 4), "int"), "0");
    assert_eq!(field(row(C_LIVE[0], 26, 4), "int"), "1");
}

#[test]
fn c84_fresh_numeric_boolean_requires_explicit_target_while_word_and_invalid_do_not() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // Availability/value projection contract, not a Native execution receipt.
    let protocol = NativeScalarGetterProtocol::for_tcl_version(TclVersion::V8_4);
    for input in [b"17".as_slice(), b"4294967296", b"1.5", b"NaN", b"08"] {
        assert!(
            protocol
                .fresh_conversion(NativeScalarGetterKind::Boolean, input)
                .is_none()
        );
        assert_eq!(
            protocol.fresh_conversion_with_target(NativeScalarGetterKind::Boolean, input, None),
            Err(NativeScalarGetterTargetUnavailable)
        );
        assert!(
            protocol
                .fresh_conversion_with_range_error(NativeScalarGetterKind::Boolean, input, false)
                .is_none()
        );
    }
    for input in [
        b"true".as_slice(),
        b"1\0X",
        b"",
        b"bad",
        b"offending",
        b"\xff17",
    ] {
        assert!(
            protocol
                .fresh_conversion(NativeScalarGetterKind::Boolean, input)
                .is_some()
        );
        assert!(
            protocol
                .fresh_conversion_with_target(NativeScalarGetterKind::Boolean, input, None)
                .unwrap()
                .is_some()
        );
    }
    assert_eq!(
        number::native_signed64_saturating_integer(&Number::Double(17.0)),
        None
    );
    assert_eq!(
        number::native_signed64_saturating_integer(&Number::Big {
            negative: false,
            radix: Radix::Dec,
            digits: "x".into()
        }),
        None
    );
    // Geometry remains counted even though both libc stages stop at first NUL.
    let target = target(C_LIVE[0]);
    assert!(
        protocol
            .c84_boolean_from_long_host(b"17\0X", target, 17, 2)
            .is_none()
    );
    assert_eq!(
        protocol
            .c84_boolean_from_double_host(b"17\0X", 17.0, 2)
            .unwrap()
            .outcome(),
        Err(NativeScalarGetterFailure::Invalid)
    );
    assert!(
        protocol
            .c84_boolean_from_long_host(b"17", target, 17, 3)
            .is_none()
    );
}
