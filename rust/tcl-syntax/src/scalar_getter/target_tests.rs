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
                        .unwrap();
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
