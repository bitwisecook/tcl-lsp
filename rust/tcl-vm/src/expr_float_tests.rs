// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual C8.4 reached normalization using complete independent host facts.

use super::*;
use tcl_platform::{NumericEnvironmentUnavailable, NumericErrorState};
use tcl_syntax::native_object::NativeObjectCacheSnapshot;
use tcl_syntax::scalar_getter::NativeScalarCache;
use tcl_test_support::numeric_environment::RecordedNumericEnvironment;

fn decode(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn c84_normalization_matches_six_original_float_error_windows() {
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
    let values = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NAN,
        1.0,
        0.0,
    ];
    let mut count = 0;
    for line in include_str!("../../tcl-cmd-core/testdata/native_float_errors/8.4.20.tsv")
        .lines()
        .filter(|line| line.starts_with("ENTER\t"))
    {
        let row: Vec<_> = line.split('\t').collect();
        let index: usize = row[1].parse().unwrap();
        let environment = RecordedNumericEnvironment::new(Ok(NumericErrorState {
            errno: row[3].parse().unwrap(),
            domain_error: row[4] == "1",
            range_error: row[5] == "1",
        }));
        let value = Value::from_native_scalar_cache(
            NativeScalarCache::Number(Number::Double(values[index])),
            None,
            dialect,
        )
        .unwrap();
        let result = cvt_to_numeric_in(
            NumericContext {
                dialect,
                simulation: None,
                environment: Some(&environment),
            },
            value,
        );
        assert_eq!(result.is_err(), row[2] == "1", "{line}");
        match result {
            Err(error) => {
                assert_eq!(
                    error.message_bytes().unwrap().as_ref(),
                    decode(row[6]),
                    "{line}"
                );
                assert_eq!(
                    error.error_code_bytes().unwrap().as_ref(),
                    decode(row[7]),
                    "{line}"
                );
                assert!(
                    matches!(
                        error
                            .guest_completion()
                            .unwrap()
                            .result
                            .native_object_snapshot()
                            .cache,
                        NativeObjectCacheSnapshot::String { .. }
                    ),
                    "{line}"
                );
            }
            Ok(value) => {
                assert_eq!(value.double_representation(), Some(values[index]), "{line}");
                assert_eq!(value.resident_string_bytes(), None, "{line}");
            }
        }
        assert_eq!(
            environment.queries(),
            usize::from(values[index].is_infinite()),
            "{line}"
        );
        count += 1;
    }
    assert_eq!(count, 6);
    let unavailable = RecordedNumericEnvironment::new(Err(NumericEnvironmentUnavailable::Target));
    let value = Value::from_native_scalar_cache(
        NativeScalarCache::Number(Number::Double(f64::INFINITY)),
        None,
        dialect,
    )
    .unwrap();
    assert!(
        cvt_to_numeric_in(
            NumericContext {
                dialect,
                simulation: None,
                environment: Some(&unavailable)
            },
            value
        )
        .unwrap_err()
        .is_host()
    );
}

#[test]
fn c84_cached_resident_double_normalizes_without_fresh_c_call() {
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
    let environment = RecordedNumericEnvironment::new(Err(NumericEnvironmentUnavailable::Target));
    let original = Value::from_native_scalar_cache(
        NativeScalarCache::Number(Number::Double(1.0)),
        None,
        dialect,
    )
    .unwrap();
    let protocol = dialect.native_string_protocol().unwrap();
    let resident = original.native_string_bytes(protocol).unwrap();
    let result = cvt_to_numeric_in(
        NumericContext {
            dialect,
            simulation: None,
            environment: Some(&environment),
        },
        original.clone(),
    )
    .unwrap();
    assert!(!result.is_same_object(&original));
    assert_eq!(result.double_representation(), Some(1.0));
    assert_eq!(result.resident_string_bytes(), None);
    assert_eq!(
        original.resident_string_bytes().unwrap().as_ref(),
        resident.as_ref()
    );
    assert_eq!(environment.queries(), 0);
}
