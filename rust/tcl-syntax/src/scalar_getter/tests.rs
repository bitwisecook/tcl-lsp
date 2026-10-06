// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use std::fmt::Write as _;

mod error_tests;

#[test]
fn expression_integer84_preserves_existing_shape_and_converts_reached_spelling() {
    let protocol = NativeScalarGetterProtocol::for_tcl_version(TclVersion::V8_4);
    for (bytes, integer) in [(b"0x10".as_slice(), 16), (b"010".as_slice(), 8)] {
        let conversion = protocol
            .expression_integer_conversion84(None, bytes)
            .unwrap();
        assert_eq!(
            conversion.cache(),
            Some(&NativeScalarCache::Tcl84Long(integer))
        );
        assert_eq!(
            conversion.outcome(),
            Ok(NativeScalarGetterValue::Wide(integer))
        );
    }
    for current in [
        NativeScalarCache::Tcl84Long(16),
        NativeScalarCache::Number(Number::Int(16)),
    ] {
        let conversion = protocol
            .expression_integer_conversion84(Some(&current), b"not reached")
            .unwrap();
        assert!(conversion.cache().is_none());
        assert_eq!(conversion.outcome(), Ok(NativeScalarGetterValue::Wide(16)));
    }
    assert!(
        NativeScalarGetterProtocol::for_tcl_version(TclVersion::V8_5)
            .expression_integer_conversion84(None, b"16")
            .is_none()
    );
    assert_eq!(
        protocol.expression_integer_result84(24, &[Some(NativeScalarCache::Tcl84Long(16))]),
        Some(NativeScalarCache::Tcl84Long(24))
    );
    assert_eq!(
        protocol
            .expression_integer_result84(24, &[Some(NativeScalarCache::Number(Number::Int(16)))]),
        Some(NativeScalarCache::Number(Number::Int(24)))
    );
}

#[test]
fn expression_integer84_uses_original_double_spelling_before_payload() {
    let protocol = NativeScalarGetterProtocol::for_tcl_version(TclVersion::V8_4);
    let double = NativeScalarCache::Number(Number::Double(2.0));
    let reached = protocol
        .expression_integer_conversion84(Some(&double), b"2")
        .unwrap();
    assert_eq!(reached.cache(), Some(&NativeScalarCache::Tcl84Long(2)));
    assert_eq!(reached.outcome(), Ok(NativeScalarGetterValue::Wide(2)));
    for spelling in [b"2.0".as_slice(), b"2e0", b".2"] {
        assert!(
            protocol
                .expression_integer_conversion84(Some(&double), spelling)
                .is_none()
        );
    }
    // TclLooksLikeInt selects GetWideOrInt even when the full conversion fails.
    for spelling in [b"2tail".as_slice(), b"0x", b"08"] {
        assert!(protocol.expression_integer_spelling84(spelling));
        assert!(
            protocol
                .expression_integer_conversion84(None, spelling)
                .unwrap()
                .outcome()
                .is_err()
        );
    }
}

const VERSIONS: [TclVersion; 5] = [
    TclVersion::V8_4,
    TclVersion::V8_5,
    TclVersion::V8_6,
    TclVersion::V9_0,
    TclVersion::V9_1,
];
const DOUBLE: [&str; 6] = [
    include_str!("../../tests/data/native_scalar_getters/double-8.4.txt"),
    include_str!("../../tests/data/native_scalar_getters/double-8.5.txt"),
    include_str!("../../tests/data/native_scalar_getters/double-8.6.txt"),
    include_str!("../../tests/data/native_scalar_getters/double-9.0.txt"),
    include_str!("../../tests/data/native_scalar_getters/double-9.1.txt"),
    include_str!("../../tests/data/native_scalar_getters/jim-double-probe.txt"),
];
const WIDE: [&str; 6] = [
    include_str!("../../tests/data/native_scalar_getters/wide-8.4.txt"),
    include_str!("../../tests/data/native_scalar_getters/wide-8.5.txt"),
    include_str!("../../tests/data/native_scalar_getters/wide-8.6.txt"),
    include_str!("../../tests/data/native_scalar_getters/wide-9.0.txt"),
    include_str!("../../tests/data/native_scalar_getters/wide-9.1.txt"),
    include_str!("../../tests/data/native_scalar_getters/jim-wide-native.txt"),
];
const BOOLEAN: [&str; 6] = [
    include_str!("../../tests/data/native_scalar_getters/boolean-8.4.txt"),
    include_str!("../../tests/data/native_scalar_getters/boolean-8.5.txt"),
    include_str!("../../tests/data/native_scalar_getters/boolean-8.6.txt"),
    include_str!("../../tests/data/native_scalar_getters/boolean-9.0.txt"),
    include_str!("../../tests/data/native_scalar_getters/boolean-9.1.txt"),
    include_str!("../../tests/data/native_scalar_getters/jim-boolean-native.txt"),
];
const CACHED: [&str; 5] = [
    include_str!("../../tests/data/native_scalar_getters/cached-8.4.txt"),
    include_str!("../../tests/data/native_scalar_getters/cached-8.5.txt"),
    include_str!("../../tests/data/native_scalar_getters/cached-8.6.txt"),
    include_str!("../../tests/data/native_scalar_getters/cached-9.0.txt"),
    include_str!("../../tests/data/native_scalar_getters/cached-9.1.txt"),
];
const NUL: [&str; 5] = [
    include_str!("../../tests/data/native_scalar_getters/nul-8.4.txt"),
    include_str!("../../tests/data/native_scalar_getters/nul-8.5.txt"),
    include_str!("../../tests/data/native_scalar_getters/nul-8.6.txt"),
    include_str!("../../tests/data/native_scalar_getters/nul-9.0.txt"),
    include_str!("../../tests/data/native_scalar_getters/nul-9.1.txt"),
];

fn protocol(index: usize) -> NativeScalarGetterProtocol {
    VERSIONS.get(index).map_or_else(
        || {
            NativeScalarGetterProtocol::for_point(DialectPoint::canonical(Release::JIM_0_84))
                .unwrap()
        },
        |&version| NativeScalarGetterProtocol::for_tcl_version(version),
    )
}
fn field<'a>(line: &'a str, key: &str) -> &'a str {
    line.split_ascii_whitespace()
        .find_map(|word| {
            let (name, value) = word.split_once('=')?;
            (name == key).then_some(value)
        })
        .unwrap_or_else(|| panic!("missing {key} in {line}"))
}
fn value_bits(value: NativeScalarGetterValue) -> u64 {
    match value {
        NativeScalarGetterValue::Wide(value) => value.cast_unsigned(),
        NativeScalarGetterValue::Double(value) => value.to_bits(),
        NativeScalarGetterValue::Boolean(value) => u64::from(value),
    }
}
fn cache_kind(cache: Option<&NativeScalarCache>) -> &'static str {
    match cache {
        None => "string",
        Some(NativeScalarCache::Number(Number::Int(_)) | NativeScalarCache::Tcl84Long(_)) => {
            "integer"
        }
        Some(NativeScalarCache::Number(Number::Big { .. })) => "bignum",
        Some(NativeScalarCache::Number(Number::Double(_) | Number::Nan { .. })) => "double",
        Some(NativeScalarCache::WordBoolean(_)) => "boolean",
        Some(NativeScalarCache::JimCoercedInteger(_)) => "coerced-double",
    }
}
fn native_cache_kind(name: &str) -> &str {
    match name {
        "int" | "wideInt" => "integer",
        "booleanString" | "boolean" => "boolean",
        "bytearray" => "string",
        name => name,
    }
}
fn verify(conversion: &NativeScalarGetterConversion, code: &str, bits: &str, context: &str) {
    assert_eq!(
        conversion.outcome().is_ok(),
        code == "0",
        "{context}: {conversion:?}"
    );
    if let Ok(value) = conversion.outcome() {
        assert_eq!(
            value_bits(value),
            u64::from_str_radix(bits, 16).unwrap(),
            "{context}"
        );
    }
}

#[test]
fn fresh_native_getter_grammar_value_and_cache_match_all_six_engines() {
    let mut count = 0;
    for (kind, fixtures) in [
        (NativeScalarGetterKind::Wide, WIDE),
        (NativeScalarGetterKind::Double, DOUBLE),
    ] {
        for (index, fixture) in fixtures.iter().enumerate() {
            for line in fixture.lines() {
                let input = line.split_once('\t').unwrap().0.as_bytes();
                // The pinned sequential Jim Wide probe retained ERANGE after
                // the preceding unsigned-overflow cases before its MIN case.
                let conversion =
                    protocol(index).fresh_conversion_with_range_error(kind, input, true);
                verify(&conversion, field(line, "code"), field(line, "bits"), line);
                assert_eq!(
                    cache_kind(conversion.cache()),
                    native_cache_kind(field(line, "type")),
                    "engine {index}: {line}"
                );
                count += 1;
            }
        }
    }
    assert_eq!(count, 312);
}

#[test]
fn storage_and_boolean_cache_are_independent_getter_axes() {
    let cases: [&[u8]; 7] = [
        b"1\0X",
        b"true\0X",
        b"false\0X",
        b"true",
        b"false",
        b"2",
        b"1.5",
    ];
    let mut count = 0;
    for (index, fixture) in BOOLEAN.iter().enumerate() {
        let recipe = protocol(index);
        for line in fixture.lines() {
            let at = field(line, "case").parse::<usize>().unwrap();
            let storage = if index != 5 && field(line, "kind") == "bytearray" {
                NativeScalarStringStorage::ByteArray
            } else {
                NativeScalarStringStorage::RawString
            };
            let input = recipe.materialize(storage, cases[at]).unwrap();
            let boolean = recipe
                .fresh_conversion(NativeScalarGetterKind::Boolean, &input)
                .unwrap();
            verify(&boolean, field(line, "boolcode"), field(line, "bool"), line);
            assert_eq!(
                cache_kind(boolean.cache()),
                native_cache_kind(field(line, "booltype")),
                "engine {index}: {line}"
            );
            let wide = boolean
                .cache()
                .and_then(|cache| recipe.cached_conversion(NativeScalarGetterKind::Wide, cache))
                .unwrap_or_else(|| {
                    recipe
                        .fresh_conversion(NativeScalarGetterKind::Wide, &input)
                        .unwrap()
                });
            assert_eq!(
                wide.outcome().is_ok(),
                field(line, "widecode") == "0",
                "{line}: {wide:?}"
            );
            if let Ok(NativeScalarGetterValue::Wide(value)) = wide.outcome() {
                assert_eq!(value, field(line, "wide").parse::<i64>().unwrap(), "{line}");
            }
            count += 1;
        }
    }
    assert_eq!(count, 77);
}

#[test]
fn existing_native_cache_conversion_preserves_value_and_preparation_obligations() {
    let numbers = [
        Number::Int(i64::MAX),
        Number::Int(2),
        Number::Double(1.0),
        Number::Double(1.5),
        Number::Double(f64::NAN),
    ];
    let strings: [&[u8]; 7] = [
        b"9223372036854775807",
        b"2",
        b"1.0",
        b"1.5",
        b"NaN",
        b"true",
        b"1",
    ];
    let mut count = 0;
    for (index, fixture) in CACHED.iter().enumerate() {
        let recipe = protocol(index);
        for line in fixture.lines() {
            let at = field(line, "kind").parse::<usize>().unwrap();
            let kind = match field(line, "getter") {
                "0" => NativeScalarGetterKind::Wide,
                "1" => NativeScalarGetterKind::Double,
                _ => NativeScalarGetterKind::Boolean,
            };
            let prior = if at < 5 {
                NativeScalarCache::Number(numbers[at].clone())
            } else if at == 5 || index == 0 {
                NativeScalarCache::WordBoolean(true)
            } else {
                NativeScalarCache::Number(Number::Int(1))
            };
            let conversion = recipe
                .cached_conversion(kind, &prior)
                .unwrap_or_else(|| recipe.fresh_conversion(kind, strings[at]).unwrap());
            verify(&conversion, field(line, "code"), field(line, "bits"), line);
            assert_eq!(
                cache_kind(conversion.cache().or(Some(&prior))),
                native_cache_kind(field(line, "after")),
                "engine {index}: {line}"
            );
            let (materialize, _, _) = conversion.into_parts();
            assert_eq!(
                materialize,
                index == 0 && kind == NativeScalarGetterKind::Boolean && at < 5,
                "{line}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 105);
}

#[test]
fn original_storage_materializes_before_numeric_extent_selection() {
    let cases: [&[u8]; 3] = [b"1\0X", b"x\0after", b"\xff\0after"];
    let mut count = 0;
    for (index, fixture) in NUL.iter().enumerate() {
        let recipe = protocol(index);
        for line in fixture.lines() {
            let at = field(line, "case").parse::<usize>().unwrap();
            let storage = if field(line, "kind") == "bytearray" {
                NativeScalarStringStorage::ByteArray
            } else {
                NativeScalarStringStorage::RawString
            };
            let bytes = recipe.materialize(storage, cases[at]).unwrap();
            let hex: String = bytes.iter().fold(String::new(), |mut output, byte| {
                write!(output, "{byte:02x}").unwrap();
                output
            });
            assert_eq!(hex, field(line, "input"));
            for (kind, key) in [
                (NativeScalarGetterKind::Wide, "widecode"),
                (NativeScalarGetterKind::Double, "doublecode"),
            ] {
                let conversion = recipe.fresh_conversion(kind, &bytes).unwrap();
                assert_eq!(
                    conversion.outcome().is_ok(),
                    field(line, key) == "0",
                    "engine {index}: {line}"
                );
                count += 1;
            }
        }
    }
    assert_eq!(count, 60);
}

#[test]
fn unknown_native_range_state_and_foreign_storage_abstain() {
    let jim = protocol(5);
    assert_eq!(
        jim.fresh_conversion(NativeScalarGetterKind::Wide, b"9223372036854775808"),
        None
    );
    assert!(
        jim.fresh_conversion_with_range_error(
            NativeScalarGetterKind::Wide,
            b"9223372036854775808",
            false
        )
        .outcome()
        .is_ok()
    );
    assert_eq!(
        jim.fresh_conversion_with_range_error(
            NativeScalarGetterKind::Wide,
            b"9223372036854775808",
            true
        )
        .outcome(),
        Err(NativeScalarGetterFailure::IntegerOverflow)
    );
    assert_eq!(
        jim.materialize(NativeScalarStringStorage::ByteArray, b"1"),
        None
    );
    assert_eq!(
        protocol(0)
            .fresh_conversion(NativeScalarGetterKind::Boolean, b"")
            .unwrap()
            .outcome(),
        Err(NativeScalarGetterFailure::Invalid)
    );
}

#[test]
fn retained_native_range_state_affects_fresh_boundaries_and_not_cached_integers() {
    let jim = protocol(5);
    let inputs = [b"9223372036854775807".as_slice(), b"-9223372036854775808"];
    let values = [i64::MAX, i64::MIN];
    let fixture = include_str!("../../tests/data/native_scalar_getters/jim-range-state.txt");
    let mut count = 0;
    for line in fixture.lines() {
        let at = field(line, "case").parse::<usize>().unwrap();
        let cached = field(line, "kind") == "1";
        let prior = NativeScalarCache::Number(Number::Int(values[at]));
        let conversion = if cached {
            jim.cached_conversion(NativeScalarGetterKind::Wide, &prior)
                .unwrap()
        } else {
            assert_eq!(
                jim.fresh_conversion(NativeScalarGetterKind::Wide, inputs[at]),
                None
            );
            jim.fresh_conversion_with_range_error(
                NativeScalarGetterKind::Wide,
                inputs[at],
                field(line, "state") == "1",
            )
        };
        verify(&conversion, field(line, "code"), field(line, "bits"), line);
        assert_eq!(
            cache_kind(conversion.cache().or(cached.then_some(&prior))),
            native_cache_kind(field(line, "type"))
        );
        count += 1;
    }
    assert_eq!(count, 8);
}

fn primitive_int_unhex(text: &str) -> Vec<u8> {
    if text == "-" {
        return Vec::new();
    }
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = core::str::from_utf8(pair).unwrap();
            u8::from_str_radix(text, 16).unwrap()
        })
        .collect()
}

fn assert_primitive_int_row(
    protocol: NativeScalarGetterProtocol,
    version: TclVersion,
    row: &str,
    inputs: &[&[u8]],
) {
    let fields: Vec<_> = row.split('\t').collect();
    let case: usize = fields[0].parse().unwrap();
    let prior = match case {
        11 => Some(NativeScalarCache::Number(Number::Double(1.0))),
        12 => Some(NativeScalarCache::Number(Number::Int(1))),
        // The native fixture constructs Tcl_NewDoubleObj(NAN), not
        // a parsed NaN spelling. Preserve that actual cache category.
        13 => Some(NativeScalarCache::Number(Number::Double(f64::NAN))),
        _ => None,
    };
    let original = inputs.get(case).copied().unwrap_or_else(|| match case {
        11 => b"1.0",
        12 => b"1",
        _ if version == TclVersion::V8_4 => b"nan",
        _ => b"NaN",
    });
    let conversion = prior
        .as_ref()
        .and_then(|cache| protocol.cached_conversion(NativeScalarGetterKind::Int, cache))
        .or_else(|| protocol.fresh_conversion(NativeScalarGetterKind::Int, original))
        .unwrap();
    let effective_cache = conversion.cache().or(prior.as_ref());
    if case == 13 {
        assert!(
            conversion.cache().is_none(),
            "{version:?}: unchanged NaN cache"
        );
        let Some(NativeScalarCache::Number(Number::Double(value))) = effective_cache else {
            panic!("{version:?}: native Double NaN cache was replaced");
        };
        assert_eq!(value.to_bits(), f64::NAN.to_bits());
    }
    if case == 12 {
        // Tcl_NewWideIntObj(1) retains C8.4's wideInt storage.
        assert!(
            conversion.cache().is_none(),
            "{version:?}: unchanged integer cache"
        );
    }
    let cache_type = match effective_cache {
        Some(NativeScalarCache::Tcl84Long(_) | NativeScalarCache::Number(Number::Int(_))) => {
            "integer"
        }
        Some(NativeScalarCache::Number(Number::Big { .. })) => "bignum",
        Some(NativeScalarCache::Number(Number::Double(_) | Number::Nan { .. })) => "double",
        _ => "string",
    };
    assert_eq!(
        cache_type,
        native_cache_kind(fields[3]),
        "{version:?} case{case}: cache category"
    );
    if fields[3] == "int" && version == TclVersion::V8_4 {
        assert!(matches!(
            effective_cache,
            Some(NativeScalarCache::Tcl84Long(_))
        ));
    } else if matches!(fields[3], "int" | "wideInt") {
        assert!(matches!(
            effective_cache,
            Some(NativeScalarCache::Number(Number::Int(_)))
        ));
    }
    if fields[1] == "0" {
        assert_eq!(
            conversion.outcome(),
            Ok(NativeScalarGetterValue::Wide(fields[2].parse().unwrap())),
            "{version:?} case{case}"
        );
    } else {
        let failure = conversion.outcome().unwrap_err();
        let record = protocol
            .failure_presentation(NativeScalarGetterKind::Int, failure, original)
            .unwrap();
        assert_eq!(
            record.message_bytes(),
            primitive_int_unhex(fields[4]),
            "{version:?} case{case}: message"
        );
        let code = match record.error_code_update() {
            NativeScalarGetterErrorCode::Unchanged => b"SEEDED CODE".to_vec(),
            NativeScalarGetterErrorCode::Set(code) => code.clone(),
        };
        assert_eq!(
            code,
            primitive_int_unhex(fields[5]),
            "{version:?} case{case}: error-code update"
        );
    }
}

#[test]
fn primitive_int_retains_native_width_cache_and_error_stage() {
    let rows = [
        include_str!("../../tests/data/native_scalar_getters/int/8.4.20.txt"),
        include_str!("../../tests/data/native_scalar_getters/int/8.5.19.txt"),
        include_str!("../../tests/data/native_scalar_getters/int/8.6.18.txt"),
        include_str!("../../tests/data/native_scalar_getters/int/9.0.4.txt"),
        include_str!("../../tests/data/native_scalar_getters/int/9.1.0.txt"),
    ];
    let inputs: [&[u8]; 11] = [
        b"1\0X",
        b"2147483648",
        b"4294967295",
        b"4294967296",
        b"-4294967295",
        b"9223372036854775808",
        b"1.0",
        b"NaN",
        b"08",
        b"0x1",
        b"bad",
    ];
    let mut checked = 0;
    for (version, rows) in VERSIONS.into_iter().zip(rows) {
        let protocol = NativeScalarGetterProtocol::for_tcl_version(version);
        for row in rows.lines() {
            assert_primitive_int_row(protocol, version, row, &inputs);
            checked += 1;
        }
    }
    assert_eq!(checked, 70);
    assert!(
        NativeScalarGetterProtocol {
            engine: Engine::Jim084
        }
        .fresh_conversion(NativeScalarGetterKind::Int, b"1")
        .is_none()
    );
}

#[test]
fn jim_decimal_string_switch_keeps_its_own_native_stage() {
    let protocol = NativeScalarGetterProtocol {
        engine: Engine::Jim084,
    };
    let inputs: [&[u8]; 16] = [
        b"0",
        b"1",
        b"-1",
        b"+1",
        b" 1 \t",
        b"1.0",
        b"0x10",
        b"010",
        b"9223372036854775807",
        b"9223372036854775808",
        b"18446744073709551615",
        b"18446744073709551616",
        b"-9223372036854775808",
        b"1\0X",
        b"",
        b"bad",
    ];
    let mut checked = 0;
    for row in
        include_str!("../../tests/data/native_scalar_getters/decimal-switch/jim0.84.txt").lines()
    {
        let fields: Vec<_> = row.split('\t').collect();
        let case: usize = fields[0].parse().unwrap();
        let outcome = protocol.jim_decimal_wide_probe(inputs[case]).unwrap();
        if fields[2] == "0" {
            assert_eq!(outcome, Ok(fields[3].parse().unwrap()), "{row}");
        } else {
            assert_eq!(outcome, Err(NativeScalarGetterFailure::Invalid), "{row}");
        }
        checked += 1;
    }
    assert_eq!(checked, 32);
    assert!(
        NativeScalarGetterProtocol::for_tcl_version(TclVersion::V9_0)
            .jim_decimal_wide_probe(b"1")
            .is_none()
    );
}

#[test]
fn legacy_long_and_wide_retain_distinct_native_primary_caches() {
    let protocol = NativeScalarGetterProtocol::for_tcl_version(TclVersion::V8_4);
    let fixture = include_str!("../../tests/data/native_scalar_getters/long84/8.4.20.tsv");
    let mut count = 0;
    for line in fixture.lines().filter(|line| line.starts_with("ROW\t")) {
        let fields: Vec<_> = line.split('\t').collect();
        let case = fields[1].parse::<usize>().unwrap();
        let kind = match fields[2] {
            "0" => NativeScalarGetterKind::Int,
            "1" => NativeScalarGetterKind::Long,
            "2" => NativeScalarGetterKind::Wide,
            _ => unreachable!(),
        };
        let cache = match case {
            0 => Some(NativeScalarCache::Tcl84Long(17)),
            1 => Some(NativeScalarCache::Tcl84Long(4_294_967_295)),
            2 => Some(NativeScalarCache::Number(Number::Int(17))),
            3 => Some(NativeScalarCache::Number(Number::Int(4_294_967_296))),
            11 => Some(NativeScalarCache::Number(Number::Double(17.0))),
            _ => None,
        };
        let bytes: &[u8] = match case {
            0 | 2 | 4 => b"17",
            1 | 5 => b"4294967295",
            3 | 6 => b"4294967296",
            7 => b"18446744073709551615",
            8 => b"-18446744073709551615",
            9 => b"18446744073709551616",
            10 => b"1\0X",
            11 => b"17.0",
            12 => b"08",
            _ => unreachable!(),
        };
        let conversion = cache
            .as_ref()
            .and_then(|cache| protocol.cached_conversion(kind, cache))
            .unwrap_or_else(|| protocol.fresh_conversion(kind, bytes).unwrap());
        let final_cache = conversion.cache().or(cache.as_ref());
        let final_type = match final_cache {
            Some(NativeScalarCache::Tcl84Long(_)) => "int",
            Some(NativeScalarCache::Number(Number::Int(_))) => "wideInt",
            Some(NativeScalarCache::Number(Number::Double(_))) => "double",
            None => "NULL",
            other => panic!("unexpected C84 cache {other:?}"),
        };
        assert_eq!(final_type, fields[4], "{line}");
        assert_eq!(conversion.outcome().is_ok(), fields[10] == "0", "{line}");
        match conversion.outcome() {
            Ok(NativeScalarGetterValue::Wide(value)) => {
                assert_eq!(value.to_string(), fields[8], "{line}");
            }
            Err(failure) => {
                let presentation = protocol.failure_presentation(kind, failure, bytes).unwrap();
                let expected: Vec<_> = fields[9]
                    .as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| {
                        u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap()
                    })
                    .collect();
                assert_eq!(presentation.message_bytes(), expected, "{line}");
            }
            other => panic!("unexpected integer getter result {other:?}"),
        }
        count += 1;
    }
    assert_eq!(count, 39);
}

#[test]
fn selected_jim_numeric_term_kind_does_not_borrow_getter_coercion() {
    use tcl_dialect::JimExpressionNumberKind::{Double, Integer};
    assert_eq!(
        jim_expression_number_for_kind(b"0755", Integer),
        Some(JimExpressionNumber::Integer(755))
    );
    assert_eq!(
        jim_expression_number_for_kind(b"0755", Double),
        Some(JimExpressionNumber::Double(755.0))
    );
    assert_eq!(jim_expression_number_for_kind(b"1e0", Integer), None);
    assert_eq!(jim_expression_number_for_kind(b"1\0X", Integer), None);
    assert_eq!(
        jim_expression_number_for_kind(b"0xFFFFFFFFFFFFFFFF", Integer),
        Some(JimExpressionNumber::Integer(-1))
    );
}

#[test]
fn jim_expression_numeric_terms_use_fresh_native_constructor_values() {
    for (source, expected) in [
        (b"0".as_slice(), 0),
        (b"0755", 755),
        (b"0xFFFFFFFFFFFFFFFF", -1),
        (b"18446744073709551616", -1),
        (b"0b101", 5),
        (b"0o755", 493),
        (b"0d755", 755),
    ] {
        assert_eq!(
            jim_expression_number(source),
            Some(JimExpressionNumber::Integer(expected))
        );
    }
    for (source, expected) in [
        (b"1e0".as_slice(), 1.0),
        (b"1e400", f64::INFINITY),
        (b"1e-400", 0.0),
        (b"Inf", f64::INFINITY),
        (b".5", 0.5),
        (b"0x1.8p1", 3.0),
        (b"0x1.", 1.0),
    ] {
        assert_eq!(
            jim_expression_number(source),
            Some(JimExpressionNumber::Double(expected))
        );
    }
    assert!(
        matches!(jim_expression_number(b"NaN"), Some(JimExpressionNumber::Double(value)) if value.is_nan())
    );
    for source in [
        b"true".as_slice(),
        b"yes",
        b"1\0X",
        b"1 ",
        b"0x1p4",
        b"0x.p4",
        b"Infinity",
        b"NaNfoo",
        b"0b2",
        b"12x",
        b"1e+",
        b".",
    ] {
        assert_eq!(jim_expression_number(source), None);
    }
}
