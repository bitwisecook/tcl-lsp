// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual host numeric stages selected by the independent native getter owner.

use tcl_platform::NumericEnvironment;
use tcl_syntax::{
    scalar_getter::{
        NativeScalarGetterConversion, NativeScalarGetterKind, NativeScalarGetterProtocol,
    },
    value::ValueError,
};

/// Observe C8.4's reached nonfinite result without resetting thread state.
/// Finite values and known NaN domain errors require no environment query;
/// infinity requires an actual EDOM fact before selecting overflow.
pub fn c84_nonfinite_error(
    protocol: NativeScalarGetterProtocol,
    value: f64,
    environment: Option<&dyn NumericEnvironment>,
) -> Result<Option<tcl_syntax::expr::errors::NativeFloatError>, ValueError> {
    if protocol.tcl_version() != Some(tcl_dialect::TclVersion::V8_4) {
        return Err(ValueError::ScalarNumericInputUnavailable);
    }
    if value.is_finite() {
        return Ok(None);
    }
    if value.is_nan() {
        return Ok(Some(tcl_syntax::expr::errors::NativeFloatError::Domain));
    }
    let state = environment
        .ok_or(ValueError::ScalarNumericInputUnavailable)?
        .state()
        .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
    Ok(Some(tcl_syntax::expr::errors::NativeFloatError::classify(
        value,
        state.errno,
        state.domain_error,
        state.range_error,
    )))
}

/// Execute only the fresh C8.4 primitive selected after original cache lookup.
/// Reset and C-call effects are retained even when conversion rejects spelling.
pub fn fresh_c84_conversion(
    protocol: NativeScalarGetterProtocol,
    kind: NativeScalarGetterKind,
    original: &[u8],
    environment: &dyn NumericEnvironment,
) -> Result<NativeScalarGetterConversion, ValueError> {
    let unavailable = || ValueError::ScalarNumericInputUnavailable;
    if protocol.tcl_version() != Some(tcl_dialect::TclVersion::V8_4) {
        return Err(unavailable());
    }
    if kind == NativeScalarGetterKind::Double {
        let executed = environment
            .double(original, true)
            .map_err(|_| unavailable())?;
        return protocol
            .c84_double_from_host(
                original,
                executed.value,
                executed.end,
                executed.after.errno,
                executed.after.domain_error,
                executed.after.range_error,
            )
            .ok_or_else(unavailable);
    }
    let conversion = protocol
        .fresh_conversion(kind, original)
        .ok_or_else(unavailable)?;
    if !matches!(
        kind,
        NativeScalarGetterKind::Int | NativeScalarGetterKind::Long | NativeScalarGetterKind::Wide
    ) {
        return Ok(conversion);
    }
    environment.reset().map_err(|_| unavailable())?;
    let mut offset = original
        .iter()
        .take_while(|byte| byte.is_ascii_whitespace())
        .count();
    let negative = original.get(offset) == Some(&b'-');
    if matches!(original.get(offset), Some(b'+' | b'-')) {
        offset += 1;
    }
    if !original.get(offset).is_some_and(u8::is_ascii_digit) {
        return Ok(conversion);
    }
    let executed = environment
        .unsigned_c84(original, offset, kind != NativeScalarGetterKind::Wide)
        .map_err(|_| unavailable())?;
    let conversion = protocol
        .c84_integer_from_host(
            kind,
            original,
            offset,
            executed.end,
            executed.after.range_error,
        )
        .ok_or_else(unavailable)?;
    if let Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(value)) =
        conversion.outcome()
    {
        let actual = if negative {
            executed.value.wrapping_neg()
        } else {
            executed.value
        }
        .cast_signed();
        if executed.after.range_error || actual != value {
            return Err(unavailable());
        }
    }
    Ok(conversion)
}

/// Execute fresh Jim getter C calls against the retained actual Host. Existing
/// original caches must be inspected by the caller before reaching this door.
/// Host callbacks/interpreters share the real thread state; this operation
/// neither initializes nor mirrors errno in an interpreter.
pub fn fresh_jim_conversion(
    protocol: NativeScalarGetterProtocol,
    kind: NativeScalarGetterKind,
    materialized: &[u8],
    environment: &dyn NumericEnvironment,
) -> Result<NativeScalarGetterConversion, ValueError> {
    let unavailable = || ValueError::ScalarNumericInputUnavailable;
    protocol
        .jim_unsigned_stage(materialized)
        .ok_or_else(unavailable)?;
    match kind {
        NativeScalarGetterKind::Wide => {
            let stage = protocol
                .jim_unsigned_stage(materialized)
                .ok_or_else(unavailable)?;
            let mut conversion = environment
                .unsigned(materialized, stage.offset, stage.base)
                .map_err(|_| unavailable())?;
            let mut negate = stage.negate;
            if stage.offset != 0 && conversion.end == stage.offset {
                conversion = environment
                    .unsigned(materialized, 0, 10)
                    .map_err(|_| unavailable())?;
                negate = false;
            }
            let value = if negate {
                conversion.value.wrapping_neg()
            } else {
                conversion.value
            }
            .cast_signed();
            protocol
                .jim_wide_from_host(
                    materialized,
                    value,
                    conversion.end,
                    conversion.after.range_error,
                )
                .ok_or_else(unavailable)
        }
        NativeScalarGetterKind::Double => {
            let decimal = environment
                .unsigned(materialized, 0, 10)
                .map_err(|_| unavailable())?;
            if protocol.jim_host_conversion_complete(materialized, decimal.end) {
                protocol
                    .jim_double_from_host(
                        materialized,
                        decimal.end,
                        0.0,
                        Some(decimal.value.cast_signed()),
                    )
                    .ok_or_else(unavailable)
            } else {
                let double = environment
                    .double(materialized, true)
                    .map_err(|_| unavailable())?;
                protocol
                    .jim_double_from_host(materialized, double.end, double.value, None)
                    .ok_or_else(unavailable)
            }
        }
        _ => protocol
            .fresh_conversion(kind, materialized)
            .ok_or_else(unavailable),
    }
}

#[cfg(all(
    test,
    target_os = "linux",
    target_env = "gnu",
    target_pointer_width = "64"
))]
mod tests {
    use super::*;
    use tcl_dialect::model::{DialectPoint, Release};
    use tcl_syntax::{
        number::Number,
        scalar_getter::{NativeScalarCache, NativeScalarGetterValue},
    };

    #[test]
    fn original_jim_getters_match_native_numeric_state_frontiers() {
        let inputs: [&[u8]; 26] = [
            b"0",
            b"1",
            b"-1",
            b"9223372036854775807",
            b"-9223372036854775808",
            b"18446744073709551615",
            b"18446744073709551616",
            b"-18446744073709551616",
            b"0x7fffffffffffffff",
            b"-0x8000000000000000",
            b"0xffffffffffffffff",
            b"0b101",
            b"0o17",
            b"0d19",
            b"  +17  ",
            b"0x-5",
            b"0x",
            b"1.0",
            b"1e9999",
            b"1e-9999",
            b"nan",
            b"inf",
            b"",
            b"1x",
            b"A\0B",
            b"0x1.8p1",
        ];
        let protocol =
            NativeScalarGetterProtocol::for_point(DialectPoint::canonical(Release::JIM_0_84))
                .unwrap();
        let host = tcl_host_c_abi::NativeNumericEnvironment;
        let mut count = 0;
        for line in
            include_str!("../testdata/native_jim_numeric_environment/observations.tsv").lines()
        {
            let row: Vec<_> = line.split('\t').collect();
            let case: usize = row[0].parse().unwrap();
            let seed = row[1] == "1";
            host.double(if seed { b"1e9999" } else { b"0" }, true)
                .unwrap();
            let before = host.state().unwrap();
            let kind = if row[2] == "0" {
                NativeScalarGetterKind::Wide
            } else {
                NativeScalarGetterKind::Double
            };
            let converted = fresh_jim_conversion(protocol, kind, inputs[case], &host).unwrap();
            let after = host.state().unwrap();
            assert_eq!(before.range_error, row[6] == "1", "{line}");
            assert_eq!(after.range_error, row[7] == "1", "{line}");
            assert_eq!(converted.outcome().is_ok(), row[3] == "0", "{line}");
            let cache = match converted.cache() {
                None => "NULL",
                Some(NativeScalarCache::Number(Number::Int(_))) => "int",
                Some(NativeScalarCache::JimCoercedInteger(_)) => "coerced-double",
                Some(NativeScalarCache::Number(Number::Double(_))) => "double",
                other => panic!("unexpected original Jim cache: {other:?}"),
            };
            assert_eq!(cache, row[5], "{line}");
            if let Ok(value) = converted.outcome() {
                let bits = match value {
                    NativeScalarGetterValue::Wide(value) => value.cast_unsigned(),
                    NativeScalarGetterValue::Double(value) => value.to_bits(),
                    _ => panic!("unexpected numeric getter result"),
                };
                assert_eq!(format!("{bits:016x}"), row[4], "{line}");
            }
            count += 1;
        }
        assert_eq!(count, 104);
    }
}

#[cfg(all(
    test,
    target_os = "linux",
    target_env = "gnu",
    target_pointer_width = "64"
))]
#[path = "native_numeric_float_tests.rs"]
mod float_tests;
