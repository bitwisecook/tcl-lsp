// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual C8.4 reached normalization using complete independent host facts.

use super::*;
use tcl_platform::{NumericEnvironmentUnavailable, NumericErrorState};
use tcl_syntax::native_object::NativeObjectCacheSnapshot;
use tcl_test_support::numeric_environment::RecordedNumericEnvironment;

fn decode(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn c84_normalization_matches_six_original_float_error_windows() {
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
    let protocol = dialect.native_scalar_getter_protocol().unwrap();
    let values = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NAN,
        1.0,
        0.0,
    ];
    let mut interp = crate::interp::Interp::with_native_core(
        crate::interp::default_host(),
        tcl_dialect::DialectProfile::find("tcl8.4").unwrap(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let mut count = 0;
    for line in include_str!("../../../rust/tcl-cmd-core/testdata/native_float_errors/8.4.20.tsv")
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
        let value = Owned::fresh(obj::new_double_obj(values[index]));
        let result = normalize_compiled_primary84(value, dialect, Some(&environment));
        assert_eq!(result.is_err(), row[2] == "1", "{line}");
        match result {
            Err(error) => {
                assert_eq!(error.msg, decode(row[6]), "{line}");
                assert_eq!(error.code.as_ref().unwrap(), &decode(row[7]), "{line}");
                assert_eq!(interp.report_expr_error(error), crate::interp::Code::Error);
                assert!(
                    matches!(
                        obj::native_object_snapshot(interp.get_obj_result())
                            .unwrap()
                            .cache,
                        NativeObjectCacheSnapshot::String { .. }
                    ),
                    "{line}"
                );
            }
            Ok(value) => {
                assert_eq!(
                    obj::native_scalar_cache(value.as_ptr()).unwrap(),
                    Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                        tcl_syntax::number::Number::Double(values[index])
                    ))
                );
                assert!(!obj::has_string_rep(value.as_ptr()), "{line}");
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
    let error = normalize_compiled_primary84(
        Owned::fresh(obj::new_double_obj(f64::INFINITY)),
        dialect,
        Some(&unavailable),
    )
    .err()
    .expect("unavailable host facts must refuse infinity");
    assert!(error.native_access_refusal.is_some());
    assert_eq!(protocol.tcl_version(), Some(tcl_dialect::TclVersion::V8_4));
}

#[test]
fn c84_cached_resident_double_normalizes_without_fresh_c_call() {
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
    let environment = RecordedNumericEnvironment::new(Err(NumericEnvironmentUnavailable::Target));
    let original = Owned::fresh(obj::new_double_obj(1.0));
    let resident = obj::bytes_of(original.as_ptr());
    let result =
        normalize_compiled_primary84(original.clone(), dialect, Some(&environment)).unwrap();
    assert_ne!(result.as_ptr(), original.as_ptr());
    assert!(!obj::has_string_rep(result.as_ptr()));
    assert_eq!(obj::bytes_of(original.as_ptr()), resident);
    assert_eq!(environment.queries(), 0);
}
