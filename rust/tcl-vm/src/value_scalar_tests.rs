// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original-object primitive storage against measured native getter fixtures.
use super::*;
use tcl_dialect::TclVersion;
use tcl_syntax::scalar_getter::{
    NativeScalarGetterKind as Kind, NativeScalarGetterValue as Returned,
};
use tcl_syntax::value::ValueError;
const VERSIONS: [TclVersion; 5] = [
    TclVersion::V8_4,
    TclVersion::V8_5,
    TclVersion::V8_6,
    TclVersion::V9_0,
    TclVersion::V9_1,
];

#[test]
fn neutral_probe_retains_absent_string_and_exact_nan_payload() {
    let dialect = tcl_registry::InvocationDialect::for_version(TclVersion::V8_6);
    let bits = 0xfff8_0000_0000_0042;
    let value = Value::native_double(f64::from_bits(bits), dialect);
    let alias = value.clone();
    assert!(
        value
            .native_scalar_probe(dialect, Kind::Int)
            .unwrap()
            .is_err()
    );
    assert!(alias.resident_string_bytes().is_none());
    assert!(
        matches!(alias.native_scalar_cache(), Some(NativeScalarCache::Number(Number::Double(number))) if number.to_bits() == bits)
    );
    assert!(matches!(
        value.native_scalar_getter(dialect, Kind::Int),
        Err(ValueError::NativeScalarGetter(_))
    ));
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // C86 original case24/getter0 has no String access for constant overflow.
    assert!(alias.resident_string_bytes().is_none());
}

#[test]
fn completion_cache_preserves_original_resident_storage() {
    use tcl_cmd_core::return_options::CompletionCodeCache;
    use tcl_syntax::native_string::NativeStringProtocol;
    let value = Value::int(7);
    value
        .adopt_completion_code_cache(CompletionCodeCache::Jim(7))
        .unwrap();
    assert_eq!(
        value.completion_code_cache(),
        Some(CompletionCodeCache::Jim(7))
    );
    assert!(value.native_scalar_cache().is_none());
    assert!(
        value
            .native_string_bytes(NativeStringProtocol::Jim084)
            .is_err()
    );
    assert!(value.resident_string_bytes().is_none());
    let named = Value::new_native_string_bytes(b"return".as_slice());
    named
        .adopt_completion_code_cache(CompletionCodeCache::TclKeyword(2))
        .unwrap();
    assert_eq!(
        named
            .native_string_bytes(NativeStringProtocol::C(TclVersion::V8_6))
            .unwrap()
            .as_ref(),
        b"return"
    );
}

#[test]
fn native_binary_conversion_preserves_nul_units_and_original_cache_on_error() {
    use tcl_registry::native_binary_value::NativeBinaryByteConversion as Conversion;
    use tcl_syntax::native_string::NativeStringProtocol;
    for version in VERSIONS {
        let protocol = NativeStringProtocol::C(version);
        let value = Value::new_native_string_bytes(b"1\xc0\x80\xff".as_slice());
        assert_eq!(
            value
                .as_native_byte_array(Conversion::CheckedLatin1, protocol)
                .unwrap()
                .as_ref(),
            [b'1', 0, 255]
        );
        assert_eq!(
            value.resident_string_bytes().unwrap().as_ref(),
            b"1\xc0\x80\xff"
        );
        assert_eq!(
            value.byte_array_representation().unwrap().as_ref(),
            [b'1', 0, 255]
        );
    }
    let protocol = NativeStringProtocol::C(TclVersion::V9_0);
    let wide = Value::new_native_string_bytes("€".as_bytes());
    let before = wide.native_object_snapshot();
    assert!(matches!(
        wide.as_native_byte_array(Conversion::CheckedLatin1, protocol),
        Err(ByteArrayAccessError::Conversion(_))
    ));
    assert_eq!(wide.native_object_snapshot(), before);
    let jim = Value::new_native_string_bytes(b"\xc0\x80\xff".as_slice());
    assert_eq!(
        jim.as_native_byte_array(Conversion::Utf8, NativeStringProtocol::Jim084)
            .unwrap()
            .as_ref(),
        b"\xc0\x80\xff"
    );
    assert!(jim.byte_array_representation().is_none());
}

#[test]
fn frame_cache_keeps_signed_relative_failures_and_actual_release() {
    use tcl_registry::NativeFrameLevelCache as Cache;
    let c85 = tcl_registry::InvocationDialect::for_version(TclVersion::V8_5);
    for (spelling, distance) in [
        (b"2147483648".as_slice(), i32::MIN),
        (b"4294967295".as_slice(), -1),
    ] {
        let value = Value::new_native_string_bytes(spelling);
        value
            .install_native_frame_level_cache(Cache::Relative(distance), c85)
            .unwrap();
        assert_eq!(
            value.native_frame_level_cache_in(c85),
            Ok(Some(Cache::Relative(distance)))
        );
        assert_eq!(value.resident_string_bytes().unwrap().as_ref(), spelling);
        assert!(value.native_scalar_cache().is_none());
        assert!(
            value
                .native_frame_level_cache_in(tcl_registry::InvocationDialect::for_version(
                    TclVersion::V8_6
                ))
                .is_err()
        );
    }
    let pure = Value::int(1);
    assert!(
        pure.install_native_frame_level_cache(Cache::Absolute(1), c85)
            .is_err()
    );
    assert_eq!(
        pure.native_scalar_cache(),
        Some(NativeScalarCache::Number(Number::Int(1)))
    );
}

#[test]
fn frame_reference_duplication_preserves_payload_without_native_owner_hooks() {
    use tcl_registry::NativeFrameLevelCache as Cache;
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let original = Value::new_native_string_bytes(b"#1".as_slice());
        original
            .install_native_frame_level_cache(Cache::Absolute(1), dialect)
            .unwrap();
        assert!(!original.native_primary_has_free_hook());
        let duplicate = original.duplicate_native_object_in(NativeStringProtocol::C(version));
        assert!(!original.is_same_object(&duplicate));
        assert_eq!(original.native_object_reference_count(), 1);
        assert_eq!(duplicate.native_object_reference_count(), 1);
        drop(original);
        assert_eq!(
            duplicate.native_frame_level_cache_in(dialect),
            Ok(Some(Cache::Absolute(1)))
        );
        assert!(matches!(duplicate.native_object_snapshot().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::FrameReference {
                version: original_version, relative: false, level: 1,
            } if original_version == version));
        assert_eq!(
            duplicate
                .native_string_bytes(NativeStringProtocol::C(version))
                .unwrap()
                .as_ref(),
            b"#1"
        );
        assert!(
            duplicate
                .native_string_bytes(NativeStringProtocol::Jim084)
                .is_err()
        );
    }
}
const WIDE: [&str; 5] = [
    include_str!("../../tcl-syntax/tests/data/native_scalar_getters/wide-8.4.txt"),
    include_str!("../../tcl-syntax/tests/data/native_scalar_getters/wide-8.5.txt"),
    include_str!("../../tcl-syntax/tests/data/native_scalar_getters/wide-8.6.txt"),
    include_str!("../../tcl-syntax/tests/data/native_scalar_getters/wide-9.0.txt"),
    include_str!("../../tcl-syntax/tests/data/native_scalar_getters/wide-9.1.txt"),
];
const DOUBLE: [&str; 5] = [
    include_str!("../../tcl-syntax/tests/data/native_scalar_getters/double-8.4.txt"),
    include_str!("../../tcl-syntax/tests/data/native_scalar_getters/double-8.5.txt"),
    include_str!("../../tcl-syntax/tests/data/native_scalar_getters/double-8.6.txt"),
    include_str!("../../tcl-syntax/tests/data/native_scalar_getters/double-9.0.txt"),
    include_str!("../../tcl-syntax/tests/data/native_scalar_getters/double-9.1.txt"),
];
fn field<'a>(line: &'a str, key: &str) -> &'a str {
    line.split_ascii_whitespace()
        .find_map(|word| {
            let (name, value) = word.split_once('=')?;
            (name == key).then_some(value)
        })
        .expect("measured getter field")
}

fn cache_kind(value: &Value) -> &'static str {
    match value.native_scalar_cache() {
        None => "string",
        Some(NativeScalarCache::Tcl84Long(_) | NativeScalarCache::Number(Number::Int(_))) => {
            "integer"
        }
        Some(NativeScalarCache::Number(Number::Big { .. })) => "bignum",
        Some(NativeScalarCache::Number(_)) => "double",
        Some(NativeScalarCache::WordBoolean(_)) => "boolean",
        Some(NativeScalarCache::JimCoercedInteger(_)) => "coerced-double",
    }
}
#[test]
fn physical_fresh_getters_match_measured_c_values_and_original_caches() {
    let mut count = 0;
    for (kind, fixtures) in [(Kind::Wide, WIDE), (Kind::Double, DOUBLE)] {
        for (index, fixture) in fixtures.iter().enumerate() {
            for line in fixture.lines() {
                count += 1;
                let input = line.split_once('\t').unwrap().0.as_bytes();
                let value = Value::from_string_bytes(input);
                let alias = value.clone();
                let result = value.native_scalar_getter(
                    tcl_registry::InvocationDialect::for_version(VERSIONS[index]),
                    kind,
                );
                let native_type = field(line, "type");
                assert_eq!(
                    result.is_ok(),
                    field(line, "code") == "0",
                    "engine {index}: {line}: {result:?}"
                );
                if let Ok(returned) = result {
                    let bits = match returned {
                        Returned::Wide(integer) => integer.cast_unsigned(),
                        Returned::Double(double) => double.to_bits(),
                        Returned::Boolean(boolean) => {
                            u64::from(boolean.returned_integer().cast_unsigned())
                        }
                    };
                    assert_eq!(
                        bits,
                        u64::from_str_radix(field(line, "bits"), 16).unwrap(),
                        "{line}"
                    );
                }
                let expected = match native_type {
                    "int" | "wideInt" => "integer",
                    "booleanString" | "boolean" => "boolean",
                    other => other,
                };
                assert_eq!(cache_kind(&alias), expected, "engine {index}: {line}");
                assert_eq!(
                    value.string_bytes().as_ref(),
                    input,
                    "retained original spelling: {line}"
                );
            }
        }
    }
    assert_eq!(count, 260);
}
#[test]
fn primitive_boolean_storage_and_followup_wide_match_all_native_fixtures() {
    const FIXTURES: [&str; 6] = [
        include_str!("../../tcl-syntax/tests/data/native_scalar_getters/boolean-8.4.txt"),
        include_str!("../../tcl-syntax/tests/data/native_scalar_getters/boolean-8.5.txt"),
        include_str!("../../tcl-syntax/tests/data/native_scalar_getters/boolean-8.6.txt"),
        include_str!("../../tcl-syntax/tests/data/native_scalar_getters/boolean-9.0.txt"),
        include_str!("../../tcl-syntax/tests/data/native_scalar_getters/boolean-9.1.txt"),
        include_str!("../../tcl-syntax/tests/data/native_scalar_getters/jim-boolean-native.txt"),
    ];
    let inputs: [&[u8]; 7] = [
        b"1\0X",
        b"true\0X",
        b"false\0X",
        b"true",
        b"false",
        b"2",
        b"1.5",
    ];
    let native = crate::native_fixture::core(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile(),
    );
    let environment = native
        .host()
        .numeric_environment()
        .expect("the actual native host numeric environment");
    let mut count = 0;
    for (index, fixture) in FIXTURES.iter().enumerate() {
        let dialect = VERSIONS.get(index).map_or_else(
            || {
                tcl_registry::InvocationDialect::of_profile(
                    tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
                )
            },
            |&version| tcl_registry::InvocationDialect::for_version(version),
        );
        for line in fixture.lines() {
            let input = inputs[field(line, "case").parse::<usize>().unwrap()];
            // The C fixture has both storage kinds; Jim's seven rows
            // are raw strings and its schema intentionally has no kind.
            let object = if index < VERSIONS.len() && field(line, "kind") == "bytearray" {
                Value::byte_array(input)
            } else {
                Value::from_string_bytes(input)
            };
            let value = object;
            // Fresh C84 numeric Boolean requires the reached actual C stages;
            // the host layout does not donate object or expression authority.
            let boolean = value
                .native_scalar_getter_with_environment(dialect, Kind::Boolean, Some(environment))
                .map(|returned| match returned {
                    Returned::Boolean(value) => value,
                    _ => panic!("Boolean getter result"),
                });
            assert_eq!(
                boolean.is_ok(),
                field(line, "boolcode") == "0",
                "engine {index}: {line}: {boolean:?}"
            );
            if let Ok(boolean) = boolean {
                assert_eq!(
                    boolean.returned_integer().to_string(),
                    field(line, "bool"),
                    "{line}"
                );
            }
            let expected_cache = match field(line, "booltype") {
                "int" | "wideInt" => "integer",
                "booleanString" | "boolean" => "boolean",
                "bytearray" => "string",
                other => other,
            };
            assert_eq!(cache_kind(&value), expected_cache, "engine {index}: {line}");
            let wide =
                value
                    .native_scalar_getter(dialect, Kind::Wide)
                    .map(|returned| match returned {
                        Returned::Wide(value) => value,
                        _ => panic!("Wide getter result"),
                    });
            assert_eq!(
                wide.is_ok(),
                field(line, "widecode") == "0",
                "engine {index}: {line}: {wide:?}"
            );
            if let Ok(wide) = wide {
                assert_eq!(wide, field(line, "wide").parse::<i64>().unwrap(), "{line}");
            }
            count += 1;
        }
    }
    assert_eq!(count, 77);
}

#[test]
fn primitive_int_matches_native_width_cache_and_failure_on_original_objects() {
    use tcl_syntax::scalar_getter::NativeScalarGetterErrorCode;
    const FIXTURES: [&str; 5] = [
        include_str!("../../tcl-syntax/tests/data/native_scalar_getters/int/8.4.20.txt"),
        include_str!("../../tcl-syntax/tests/data/native_scalar_getters/int/8.5.19.txt"),
        include_str!("../../tcl-syntax/tests/data/native_scalar_getters/int/8.6.18.txt"),
        include_str!("../../tcl-syntax/tests/data/native_scalar_getters/int/9.0.4.txt"),
        include_str!("../../tcl-syntax/tests/data/native_scalar_getters/int/9.1.0.txt"),
    ];
    const INPUTS: [&[u8]; 11] = [
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
    fn decode_hex(field: &str) -> Vec<u8> {
        if field == "-" {
            return Vec::new();
        }
        field
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let mut count = 0;
    for (index, fixture) in FIXTURES.iter().enumerate() {
        let version = VERSIONS[index];
        let dialect = tcl_registry::InvocationDialect::for_version(version);

        for line in fixture.lines() {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 6, "{line}");
            let case: usize = fields[0].parse().unwrap();
            let value = match case {
                11 => Value::native_double(1.0, dialect),
                12 => Value::int(1),
                13 => Value::native_double(f64::NAN, dialect),
                _ => Value::from_string_bytes(INPUTS[case]),
            };
            let alias = value.clone();
            let result = value.native_scalar_getter(dialect, Kind::Int);
            count += 1;
            assert_eq!(
                result.is_ok(),
                fields[1] == "0",
                "{version:?}: {line}: {result:?}"
            );
            match result {
                Ok(Returned::Wide(integer)) => {
                    assert_eq!(integer, fields[2].parse::<i64>().unwrap(), "{line}");
                }
                Err(ValueError::NativeScalarGetter(error)) => {
                    assert_eq!(error.getter_kind(), Kind::Int);
                    assert_eq!(
                        error.message_bytes(),
                        decode_hex(fields[4]),
                        "{version:?}: {line}"
                    );
                    let code = match error.error_code_update() {
                        NativeScalarGetterErrorCode::Unchanged => b"SEEDED CODE".as_slice(),
                        NativeScalarGetterErrorCode::Set(bytes) => bytes.as_slice(),
                    };
                    assert_eq!(code, decode_hex(fields[5]), "{version:?}: {line}");
                }
                other => panic!("unexpected Int result {version:?}: {line}: {other:?}"),
            }
            let native_cache = match fields[3] {
                "int" | "wideInt" => "integer",
                other => other,
            };
            assert_eq!(cache_kind(&alias), native_cache, "{version:?}: {line}");
            if case < INPUTS.len() {
                assert_eq!(alias.string_bytes().as_ref(), INPUTS[case], "{line}");
            }
            // Width extraction wraps independently of the complete cache.
            if (1..=4).contains(&case) {
                let magnitude = core::str::from_utf8(INPUTS[case])
                    .unwrap()
                    .parse::<i64>()
                    .unwrap();
                assert_eq!(
                    alias.native_scalar_cache(),
                    Some(if version == TclVersion::V8_4 && fields[3] == "int" {
                        NativeScalarCache::Tcl84Long(magnitude)
                    } else {
                        NativeScalarCache::Number(tcl_syntax::number::Number::Int(magnitude))
                    }),
                    "{line}"
                );
            }
        }
    }
    assert_eq!(count, 70);
}

#[test]
fn physical_big_nan_and_word_caches_retain_full_payloads() {
    let dialect = tcl_registry::InvocationDialect::for_version(TclVersion::V8_6);
    let big = Value::from_string_bytes(b"18446744073709551615".as_slice());
    let alias = big.clone();
    assert_eq!(
        big.native_scalar_getter(dialect, Kind::Wide),
        Ok(Returned::Wide(-1))
    );
    let full = alias.native_scalar_cache().unwrap();
    assert!(
        matches!(&full, NativeScalarCache::Number(Number::Big { digits, .. }) if digits == "18446744073709551615")
    );
    assert!(big.native_scalar_getter(dialect, Kind::Double).is_ok());
    assert_eq!(alias.native_scalar_cache(), Some(full));
    let nan = Value::from_string_bytes(b"-NaN(123)".as_slice());
    assert!(matches!(
        nan.native_scalar_getter(dialect, Kind::Double),
        Err(ValueError::NativeScalarGetter(_))
    ));
    assert_eq!(
        nan.double_representation().unwrap().to_bits(),
        0xfff8_0000_0000_0123
    );
    for version in VERSIONS {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let word = Value::from_string_bytes(b"true".as_slice());
        assert_eq!(
            word.native_scalar_getter(dialect, Kind::Boolean),
            Ok(Returned::Boolean(
                tcl_syntax::scalar_getter::NativeBooleanGetterValue::normalized(true)
            ))
        );
        assert_eq!(word.native_word_boolean_version(), Some(version));
        assert_eq!(
            word.native_scalar_cache(),
            Some(NativeScalarCache::WordBoolean(true))
        );
        assert_eq!(word.string_bytes().as_ref(), b"true");
    }
}

#[test]
fn native_storage_and_missing_engine_refusals_preserve_original_objects() {
    for version in VERSIONS {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let raw = Value::from_string_bytes(b"1\0X".as_slice());
        let binary = Value::byte_array(b"1\0X".as_slice());
        assert_eq!(
            raw.native_scalar_getter(dialect, Kind::Wide).is_ok(),
            version != TclVersion::V8_4
        );
        assert!(matches!(
            binary.native_scalar_getter(dialect, Kind::Wide),
            Err(ValueError::NativeScalarGetter(_))
        ));
        assert_eq!(raw.string_bytes().as_ref(), b"1\0X");
        assert_eq!(binary.string_bytes().as_ref(), b"1\xc0\x80X");
        assert_eq!(
            binary.byte_array_representation().unwrap().as_ref(),
            b"1\0X"
        );
    }
    let jim = tcl_registry::InvocationDialect::of_profile(
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
    );
    for input in [b"9223372036854775807".as_slice(), b"-9223372036854775808"] {
        let value = Value::from_string_bytes(input);
        assert_eq!(
            value.native_scalar_getter(jim, Kind::Wide),
            Err(ValueError::ScalarNumericInputUnavailable)
        );
        assert_eq!(value.native_scalar_cache(), None);
        assert_eq!(value.string_bytes().as_ref(), input);
    }
    let vendor = tcl_registry::InvocationDialect::of_profile(
        tcl_registry::model::ingress::resolve_environment("f5-irules").unit_profile(),
    );
    let value = Value::int(3);
    assert_eq!(
        value.native_scalar_getter(vendor, Kind::Wide),
        Err(ValueError::ScalarNumericInputUnavailable)
    );
    assert!(value.resident_string_bytes().is_none());
}

#[test]
fn transport_reconstruction_keeps_resident_identity_and_descriptor_origin() {
    use tcl_syntax::native_string::NativeStringStorageIdentity as Storage;
    let dialect = tcl_registry::InvocationDialect::for_version(TclVersion::V8_6);
    assert!(
        Value::from_native_scalar_cache(
            NativeScalarCache::WordBoolean(true),
            Some(TclVersion::V8_6),
            dialect
        )
        .is_err()
    );
    let value = Value::from_native_scalar_cache_with_storage(
        NativeScalarCache::WordBoolean(true),
        Some(TclVersion::V8_6),
        dialect,
        Some((Rc::from(b"true\0\xff".as_slice()), Storage::Allocated)),
    )
    .unwrap();
    assert_eq!(
        value.resident_string_storage_identity(),
        Some(Storage::Allocated)
    );
    assert_eq!(
        value.resident_string_bytes().unwrap().as_ref(),
        b"true\0\xff"
    );
    assert_eq!(value.native_word_boolean_version(), Some(TclVersion::V8_6));
    assert!(
        Value::from_native_scalar_cache_with_storage(
            NativeScalarCache::WordBoolean(true),
            Some(TclVersion::V8_5),
            dialect,
            Some((Rc::from(b"true".as_slice()), Storage::Allocated)),
        )
        .is_err()
    );
    let unknown = Value::from_string_bytes(b"".as_slice());
    assert_eq!(
        unknown.resident_string_storage_identity(),
        Some(Storage::Unknown)
    );
    assert_eq!(
        Value::empty().resident_string_storage_identity(),
        Some(Storage::CanonicalEmpty)
    );
    assert!(
        unknown
            .with_resident_string_bytes_and_storage(
                Rc::from(b"nonempty".as_slice()),
                Storage::CanonicalEmpty
            )
            .is_err()
    );
}

#[test]
fn snapshot_retains_string_units_and_actual_list_canonical_state() {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    use tcl_syntax::native_string::NativeStringProtocol;
    let value = Value::from_string_bytes(b"a\xc0\x80\xff".as_slice());
    assert_eq!(value.native_object_snapshot().cache, Cache::None);
    let protocol = NativeStringProtocol::C(TclVersion::V8_6);
    assert_eq!(
        value.native_character_count_with_protocol(
            protocol,
            tcl_registry::native_string_length::NativeStringLengthRepresentation::AnyByteArray
        ),
        Ok(3)
    );
    assert!(matches!(
        value.native_object_snapshot().cache,
        Cache::String {
            num_chars: Some(3),
            unicode: None,
            ..
        }
    ));
    assert_eq!(
        value.native_unicode_units(protocol).unwrap().as_ref(),
        &[97, 0, 255]
    );
    assert!(matches!(
        value.native_object_snapshot().cache,
        Cache::String {
            unicode: Some(_),
            ..
        }
    ));
    let parsed = Value::from_string_bytes(b" a  b ".as_slice());
    parsed.native_object_list_elements(protocol).unwrap();
    assert!(matches!(
        parsed.native_object_snapshot().cache,
        Cache::List {
            length: 2,
            canonical: false
        }
    ));
    let pure = Value::list(vec![Value::int(1)]);
    assert!(matches!(
        pure.native_object_snapshot().cache,
        Cache::List {
            length: 1,
            canonical: true
        }
    ));
    assert!(pure.resident_string_bytes().is_none());
}
