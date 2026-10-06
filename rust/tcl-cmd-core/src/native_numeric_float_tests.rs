// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixed native error-state observations and reached host conversion effects.

use super::*;
use std::cell::Cell;
use tcl_platform::{
    DoubleNumericConversion, NumericEnvironmentUnavailable, NumericErrorState,
    UnsignedNumericConversion,
};
use tcl_syntax::expr::errors::NativeFloatError;
use tcl_syntax::scalar_getter::{NativeScalarGetterFailure, NativeScalarGetterValue};

fn decode(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

struct ObservedEnvironment {
    state: Result<NumericErrorState, NumericEnvironmentUnavailable>,
    queries: Cell<usize>,
}
impl NumericEnvironment for ObservedEnvironment {
    fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable> {
        self.queries.set(self.queries.get() + 1);
        self.state
    }
    fn unsigned(
        &self,
        _: &[u8],
        _: usize,
        _: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        Err(NumericEnvironmentUnavailable::Target)
    }
    fn double(
        &self,
        _: &[u8],
        _: bool,
    ) -> Result<DoubleNumericConversion, NumericEnvironmentUnavailable> {
        Err(NumericEnvironmentUnavailable::Target)
    }
}

struct FloatErrorObservations {
    diagnostic: &'static str,
    owners: &'static str,
    original_post_lookup: &'static str,
    owner_kind: &'static str,
    getter_changes_owner: bool,
}

const FLOAT_ERROR_OBSERVATIONS: [FloatErrorObservations; 5] = [
    FloatErrorObservations {
        diagnostic: include_str!("../testdata/native_float_errors/8.4.20.tsv"),
        owners: include_str!("../testdata/native_float_errors/owners-8.4.20.tsv"),
        original_post_lookup: include_str!(
            "../testdata/native_float_errors/owners-post-lookup-8.4.20.tsv"
        ),
        owner_kind: "namespace-cell",
        getter_changes_owner: false,
    },
    FloatErrorObservations {
        diagnostic: include_str!("../testdata/native_float_errors/8.5.19.tsv"),
        owners: include_str!("../testdata/native_float_errors/owners-8.5.19.tsv"),
        original_post_lookup: include_str!(
            "../testdata/native_float_errors/owners-post-lookup-8.5.19.tsv"
        ),
        owner_kind: "private-interp",
        getter_changes_owner: false,
    },
    FloatErrorObservations {
        diagnostic: include_str!("../testdata/native_float_errors/8.6.18.tsv"),
        owners: include_str!("../testdata/native_float_errors/owners-8.6.18.tsv"),
        original_post_lookup: include_str!(
            "../testdata/native_float_errors/owners-post-lookup-8.6.18.tsv"
        ),
        owner_kind: "private-interp",
        getter_changes_owner: true,
    },
    FloatErrorObservations {
        diagnostic: include_str!("../testdata/native_float_errors/9.0.4.tsv"),
        owners: include_str!("../testdata/native_float_errors/owners-9.0.4.tsv"),
        original_post_lookup: include_str!(
            "../testdata/native_float_errors/owners-post-lookup-9.0.4.tsv"
        ),
        owner_kind: "private-interp",
        getter_changes_owner: true,
    },
    FloatErrorObservations {
        diagnostic: include_str!("../testdata/native_float_errors/9.1.0.tsv"),
        owners: include_str!("../testdata/native_float_errors/owners-9.1.0.tsv"),
        original_post_lookup: include_str!(
            "../testdata/native_float_errors/owners-post-lookup-9.1.0.tsv"
        ),
        owner_kind: "private-interp",
        getter_changes_owner: true,
    },
];

#[test]
fn float_error_precedence_matches_thirty_native_diagnostics() {
    let values = [f64::INFINITY, f64::INFINITY, 0.0, 1.0, f64::NAN, 1.0];
    let mut count = 0;
    let mut owner_changes = 0;
    for table in FLOAT_ERROR_OBSERVATIONS {
        let original = table
            .diagnostic
            .lines()
            .filter(|line| line.starts_with("DIRECT\t"))
            .collect::<Vec<_>>();
        let observed = table.owners.lines().collect::<Vec<_>>();
        let post_lookup = table.original_post_lookup.lines().collect::<Vec<_>>();
        assert_eq!(original.len(), observed.len());
        assert_eq!(original.len(), post_lookup.len());
        for ((line, owner), old_owner) in original.into_iter().zip(observed).zip(post_lookup) {
            let row: Vec<_> = line.split('\t').collect();
            let owned: Vec<_> = owner.split('\t').collect();
            let old: Vec<_> = old_owner.split('\t').collect();
            assert_eq!(owned.len(), 11, "{owner}");
            assert_eq!(old.len(), 8, "{old_owner}");
            // Preserve both original public diagnostics and the independently
            // observed owner after the public getter on the same interpreter.
            assert_eq!(owned[..7], row, "{owner}");
            assert_eq!(owned[..7], old[..7], "{owner}");
            assert_eq!(owned[8], old[7], "{owner}");
            assert_eq!(owned[9], table.owner_kind, "{owner}");
            assert_eq!(owned[10], "before-public-getter/after-public-getter");
            let changed = owned[7] != owned[8];
            assert_eq!(changed, table.getter_changes_owner, "{owner}");
            owner_changes += usize::from(changed);
            let index: usize = row[1].parse().unwrap();
            let error = NativeFloatError::classify(
                values[index],
                row[2].parse().unwrap(),
                row[3] == "1",
                row[4] == "1",
            );
            let (message, code) = error.diagnostic();
            assert_eq!(message.as_bytes(), decode(row[5]), "{line}");
            // Classification belongs to the selected producer's owner before
            // the public missing-variable lookup can replace that error state.
            assert_eq!(code.as_bytes(), decode(owned[7]), "{owner}");
            count += 1;
        }
    }
    assert_eq!(count, 30);
    assert_eq!(owner_changes, 18);
}

#[test]
fn c84_reached_nonfinite_matches_six_native_state_windows() {
    let protocol = NativeScalarGetterProtocol::for_tcl_version(tcl_dialect::TclVersion::V8_4);
    let values = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NAN,
        1.0,
        0.0,
    ];
    let mut count = 0;
    for line in include_str!("../testdata/native_float_errors/8.4.20.tsv")
        .lines()
        .filter(|line| line.starts_with("ENTER\t"))
    {
        let row: Vec<_> = line.split('\t').collect();
        let index: usize = row[1].parse().unwrap();
        let environment = ObservedEnvironment {
            state: Ok(NumericErrorState {
                errno: row[3].parse().unwrap(),
                domain_error: row[4] == "1",
                range_error: row[5] == "1",
            }),
            queries: Cell::new(0),
        };
        let failure = c84_nonfinite_error(protocol, values[index], Some(&environment)).unwrap();
        assert_eq!(failure.is_some(), row[2] == "1", "{line}");
        if let Some(failure) = failure {
            let (message, code) = failure.diagnostic();
            assert_eq!(message.as_bytes(), decode(row[6]), "{line}");
            assert_eq!(code.as_bytes(), decode(row[7]), "{line}");
        }
        assert_eq!(
            environment.queries.get(),
            usize::from(values[index].is_infinite()),
            "{line}"
        );
        count += 1;
    }
    assert_eq!(count, 6);
    let unavailable = ObservedEnvironment {
        state: Err(NumericEnvironmentUnavailable::Target),
        queries: Cell::new(0),
    };
    assert!(c84_nonfinite_error(protocol, f64::INFINITY, Some(&unavailable)).is_err());
    assert!(c84_nonfinite_error(protocol, f64::INFINITY, None).is_err());
    assert_eq!(c84_nonfinite_error(protocol, 1.0, None).unwrap(), None);
    assert_eq!(
        c84_nonfinite_error(protocol, f64::NAN, None).unwrap(),
        Some(NativeFloatError::Domain)
    );
    assert!(
        fresh_c84_conversion(
            protocol,
            NativeScalarGetterKind::Double,
            b"1.0",
            &unavailable
        )
        .is_err()
    );
}

#[test]
fn c84_fresh_calls_reset_and_cached_observations_preserve_real_thread_state() {
    let protocol = NativeScalarGetterProtocol::for_tcl_version(tcl_dialect::TclVersion::V8_4);
    let environment = tcl_host_c_abi::NativeNumericEnvironment;
    let mut count = 0;
    for line in include_str!("../testdata/native_float_errors/fresh84.tsv")
        .lines()
        .filter(|line| line.starts_with("ENTER\t"))
    {
        let row: Vec<_> = line.split('\t').collect();
        let index: usize = row[1].parse().unwrap();
        environment.double(b"1e9999", true).unwrap();
        let before = environment.state().unwrap();
        assert!(before.range_error);
        if matches!(index, 0 | 2) {
            let failure = c84_nonfinite_error(protocol, f64::INFINITY, Some(&environment)).unwrap();
            assert_eq!(failure, Some(NativeFloatError::Overflow));
            assert_eq!(environment.state().unwrap(), before);
        } else {
            let input: &[u8] = match index {
                1 => b"Inf",
                3 => b"1e9999",
                4 => b"0x10",
                5 => b"xyz",
                _ => unreachable!(),
            };
            let kind = if index == 4 {
                NativeScalarGetterKind::Wide
            } else {
                NativeScalarGetterKind::Double
            };
            let conversion = fresh_c84_conversion(protocol, kind, input, &environment).unwrap();
            match index {
                1 => assert_eq!(
                    conversion.outcome(),
                    Ok(NativeScalarGetterValue::Double(f64::INFINITY))
                ),
                3 => assert_eq!(
                    conversion.outcome(),
                    Err(NativeScalarGetterFailure::FloatingPointRange {
                        result_is_zero: false
                    })
                ),
                4 => assert_eq!(conversion.outcome(), Ok(NativeScalarGetterValue::Wide(16))),
                5 => assert!(conversion.outcome().is_err()),
                _ => unreachable!(),
            }
            let after = environment.state().unwrap();
            assert_eq!(after.errno, row[3].parse::<i32>().unwrap(), "{line}");
            assert_eq!(after.domain_error, row[4] == "1", "{line}");
            assert_eq!(after.range_error, row[5] == "1", "{line}");
        }
        count += 1;
    }
    assert_eq!(count, 6);
    environment.reset().unwrap();
}

#[test]
fn c84_fresh_range_failures_precede_trailing_spelling_checks() {
    let protocol = NativeScalarGetterProtocol::for_tcl_version(tcl_dialect::TclVersion::V8_4);
    let environment = tcl_host_c_abi::NativeNumericEnvironment;
    for (input, zero) in [
        (b"1e9999x".as_slice(), false),
        (b"1e-9999x".as_slice(), true),
    ] {
        let conversion = fresh_c84_conversion(
            protocol,
            NativeScalarGetterKind::Double,
            input,
            &environment,
        )
        .unwrap();
        assert_eq!(
            conversion.outcome(),
            Err(NativeScalarGetterFailure::FloatingPointRange {
                result_is_zero: zero
            })
        );
        assert!(environment.state().unwrap().range_error);
    }
    let conversion = fresh_c84_conversion(
        protocol,
        NativeScalarGetterKind::Wide,
        b"18446744073709551616x",
        &environment,
    )
    .unwrap();
    assert_eq!(
        conversion.outcome(),
        Err(NativeScalarGetterFailure::IntegerOverflow)
    );
    assert!(environment.state().unwrap().range_error);
    let conversion =
        fresh_c84_conversion(protocol, NativeScalarGetterKind::Wide, b"xyz", &environment).unwrap();
    assert!(conversion.outcome().is_err());
    assert_eq!(environment.state().unwrap().errno, 0);
}
