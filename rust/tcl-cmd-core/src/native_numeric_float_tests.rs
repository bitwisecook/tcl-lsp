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
        .as_chunks::<2>()
        .0
        .iter()
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

#[test]
fn original_c84_fresh_boolean_host_stages_match_public_primitive_value_cache_and_errno() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // Actual host conversion effects, separately from expression truth and
    // original object/handler admission. The pure value/cache recipe is joined
    // only after the reached native signed-long/double stages.
    let fixtures = [
        include_str!(
            "../../tcl-registry/tests/data/native_capi_scalar_publication_original/8.4.20/execute-live.stdout"
        ),
        include_str!(
            "../../tcl-registry/tests/data/native_capi_scalar_publication_original/8.4.20/execute-null.stdout"
        ),
    ];
    let inputs: [&[u8]; 20] = [
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
    let field = |line: &str, key: &str| -> String {
        line.split('\t')
            .find_map(|part| {
                let (name, value) = part.split_once('=')?;
                (name == key).then(|| value.to_owned())
            })
            .unwrap()
    };
    let protocol = NativeScalarGetterProtocol::for_tcl_version(tcl_dialect::TclVersion::V8_4);
    let environment = tcl_host_c_abi::NativeNumericEnvironment;
    let mut count = 0;
    for fixture in fixtures {
        let abi = fixture
            .lines()
            .find(|line| line.starts_with("ABI\t"))
            .unwrap();
        let actual = environment.c_integer_abi().unwrap();
        assert_eq!(actual.char_bits.to_string(), field(abi, "CHAR_BIT"));
        assert_eq!(actual.int_bytes.to_string(), field(abi, "int"));
        assert_eq!(actual.long_bytes.to_string(), field(abi, "long"));
        for case in (0..20).chain([27, 28]) {
            let bytes = match case {
                0..20 => inputs[case].to_vec(),
                27 => protocol
                    .materialize(
                        tcl_syntax::scalar_getter::NativeScalarStringStorage::ByteArray,
                        b"1\0X",
                    )
                    .unwrap(),
                28 => [b'a'; 50].into_iter().chain("😀Z".bytes()).collect(),
                _ => unreachable!(),
            };
            let observed = fixture
                .lines()
                .find(|line| {
                    line.starts_with("ROW\t")
                        && field(line, "case") == case.to_string()
                        && field(line, "getter") == "4"
                })
                .unwrap();
            // The original request explicitly resets errno before each getter.
            // This harness operation is independent of SetBooleanFromAny.
            environment.reset().unwrap();
            let conversion = fresh_c84_conversion(
                protocol,
                NativeScalarGetterKind::Boolean,
                &bytes,
                &environment,
            )
            .unwrap();
            assert_eq!(
                conversion.outcome().is_ok(),
                field(observed, "code") == "0",
                "{observed}"
            );
            if let Ok(NativeScalarGetterValue::Boolean(value)) = conversion.outcome() {
                assert_eq!(
                    value.returned_integer().to_string(),
                    field(observed, "int"),
                    "{observed}"
                );
                assert!(matches!(
                    conversion.cache(),
                    Some(tcl_syntax::scalar_getter::NativeScalarCache::WordBoolean(_))
                ));
                assert_eq!(field(observed, "after"), "boolean");
            } else {
                assert!(conversion.cache().is_none());
                assert_eq!(field(observed, "before"), field(observed, "after"));
            }
            assert_eq!(
                environment.state().unwrap().errno.to_string(),
                field(observed, "errno"),
                "{observed}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 44);
}

struct ObservedBooleanEnvironment {
    target: bool,
    signed_calls: Cell<usize>,
    double_calls: Cell<usize>,
    resets: Cell<usize>,
}
impl NumericEnvironment for ObservedBooleanEnvironment {
    fn c_integer_abi(
        &self,
    ) -> Result<tcl_platform::NativeCIntegerAbi, NumericEnvironmentUnavailable> {
        if self.target {
            tcl_host_c_abi::NativeNumericEnvironment.c_integer_abi()
        } else {
            Err(NumericEnvironmentUnavailable::Target)
        }
    }
    fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable> {
        tcl_host_c_abi::NativeNumericEnvironment.state()
    }
    fn reset(&self) -> Result<(), NumericEnvironmentUnavailable> {
        self.resets.set(self.resets.get() + 1);
        Err(NumericEnvironmentUnavailable::Target)
    }
    fn signed_long(
        &self,
        input: &[u8],
        base: u32,
    ) -> Result<tcl_platform::SignedNumericConversion, NumericEnvironmentUnavailable> {
        self.signed_calls.set(self.signed_calls.get() + 1);
        tcl_host_c_abi::NativeNumericEnvironment.signed_long(input, base)
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
        input: &[u8],
        reset: bool,
    ) -> Result<DoubleNumericConversion, NumericEnvironmentUnavailable> {
        self.double_calls.set(self.double_calls.get() + 1);
        assert!(!reset, "Boolean fallback must not reset errno");
        tcl_host_c_abi::NativeNumericEnvironment.double(input, reset)
    }
}

#[test]
fn c84_boolean_preserves_reached_errno_and_refuses_missing_target_before_numeric_calls() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // Native-stage reachability and host availability software control. The
    // deliberate prior ERANGE is not an inferred baseline or guest error code.
    let protocol = NativeScalarGetterProtocol::for_tcl_version(tcl_dialect::TclVersion::V8_4);
    let host = tcl_host_c_abi::NativeNumericEnvironment;
    host.reset().unwrap();
    let seed = host.signed_long(b"18446744073709551616", 0).unwrap();
    assert!(seed.after.range_error);
    let environment = ObservedBooleanEnvironment {
        target: true,
        signed_calls: Cell::new(0),
        double_calls: Cell::new(0),
        resets: Cell::new(0),
    };
    for (input, signed, double, expected) in [
        (b"true".as_slice(), 0, 0, true),
        (b"4294967296", 1, 0, false),
        (b"1.5", 2, 1, true),
        (b"NaN", 3, 2, true),
    ] {
        let conversion = fresh_c84_conversion(
            protocol,
            NativeScalarGetterKind::Boolean,
            input,
            &environment,
        )
        .unwrap();
        let Ok(NativeScalarGetterValue::Boolean(value)) = conversion.outcome() else {
            panic!("{conversion:?}");
        };
        assert_eq!(value.is_true(), expected);
        assert_eq!(environment.signed_calls.get(), signed);
        assert_eq!(environment.double_calls.get(), double);
        assert_eq!(environment.state().unwrap().errno, seed.after.errno);
        assert_eq!(environment.resets.get(), 0);
    }
    let unavailable = ObservedBooleanEnvironment {
        target: false,
        signed_calls: Cell::new(0),
        double_calls: Cell::new(0),
        resets: Cell::new(0),
    };
    assert!(
        fresh_c84_conversion(
            protocol,
            NativeScalarGetterKind::Boolean,
            b"4294967296",
            &unavailable
        )
        .is_err()
    );
    assert_eq!(unavailable.signed_calls.get(), 0);
    assert_eq!(unavailable.double_calls.get(), 0);
    assert!(
        fresh_c84_conversion(
            protocol,
            NativeScalarGetterKind::Boolean,
            b"true",
            &unavailable
        )
        .unwrap()
        .outcome()
        .is_ok()
    );
    assert!(
        fresh_c84_conversion(
            protocol,
            NativeScalarGetterKind::Boolean,
            b"offending",
            &unavailable
        )
        .unwrap()
        .outcome()
        .is_err()
    );
    assert_eq!(unavailable.signed_calls.get(), 0);
    host.reset().unwrap();
}
