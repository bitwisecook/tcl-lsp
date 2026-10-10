// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual object/cache transport keeps target availability outside guest failure.

use super::*;
use tcl_platform::{
    DoubleNumericConversion, NativeCIntegerAbi, NumericEnvironment, NumericEnvironmentUnavailable,
    NumericErrorState, UnsignedNumericConversion,
};
use tcl_syntax::scalar_getter::{
    NativeScalarGetterKind as Kind, NativeScalarGetterValue as Returned,
};

struct TargetOnly(Option<NativeCIntegerAbi>);
impl NumericEnvironment for TargetOnly {
    fn c_integer_abi(&self) -> Result<NativeCIntegerAbi, NumericEnvironmentUnavailable> {
        self.0.ok_or(NumericEnvironmentUnavailable::Target)
    }
    fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable> {
        Err(NumericEnvironmentUnavailable::Target)
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
fn jim() -> tcl_registry::InvocationDialect {
    tcl_registry::InvocationDialect::of_profile(
        tcl_registry::model::ingress::resolve_known_environment("jim")
            .unwrap()
            .unit_profile(),
    )
}
fn original(integer: i64) -> Value {
    Value::from_native_scalar_cache(NativeScalarCache::Number(Number::Int(integer)), None, jim())
        .unwrap()
}

#[test]
fn original_jim_cached_boolean_uses_independent_target_without_string_access() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // Software transport of the bounded public primitive; no ExprBool or
    // runtime admission is inferred from this supplied descriptive target.
    let environment = TargetOnly(Some(NativeCIntegerAbi {
        char_bits: 8,
        int_bytes: 4,
        long_bytes: 8,
    }));
    for (integer, expected) in [(17, 17), (4_294_967_296, 0), (-17, -17)] {
        let value = original(integer);
        let alias = value.clone();
        let count = value.native_object_reference_count();
        let returned = value
            .native_scalar_probe_with_environment(jim(), Kind::Boolean, Some(&environment))
            .unwrap()
            .unwrap();
        let Returned::Boolean(boolean) = returned else {
            panic!("primitive Boolean")
        };
        assert_eq!(boolean.returned_integer(), expected);
        assert!(alias.is_same_object(&value));
        assert_eq!(
            alias.native_scalar_cache(),
            Some(NativeScalarCache::Number(Number::Int(integer)))
        );
        assert!(alias.resident_string_bytes().is_none());
        assert_eq!(value.native_object_reference_count(), count);
    }
}

#[test]
fn original_cached_getter_refuses_missing_or_unsupported_target_before_mutation() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    for environment in [
        TargetOnly(None),
        TargetOnly(Some(NativeCIntegerAbi {
            char_bits: 8,
            int_bytes: 4,
            long_bytes: 4,
        })),
    ] {
        for kind in [Kind::Boolean, Kind::Long] {
            let value = original(17);
            let before = value.native_object_snapshot();
            assert!(matches!(
                value.native_scalar_probe_with_environment(jim(), kind, Some(&environment)),
                Err(ValueError::ScalarNumericInputUnavailable)
            ));
            assert_eq!(value.native_object_snapshot(), before);
            assert_eq!(
                value.native_scalar_cache(),
                Some(NativeScalarCache::Number(Number::Int(17)))
            );
            assert!(value.resident_string_bytes().is_none());
        }
    }
    let value = original(17);
    assert!(value.native_scalar_probe(jim(), Kind::Boolean).is_err());
    assert!(value.resident_string_bytes().is_none());
}

#[test]
fn descriptive_target_does_not_admit_retired_or_foreign_original_cache() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    let environment = TargetOnly(Some(NativeCIntegerAbi {
        char_bits: 8,
        int_bytes: 4,
        long_bytes: 8,
    }));
    let original = original(17);
    let retired = original.native_lifetime_lease();
    drop(original);
    assert!(
        retired
            .value()
            .native_scalar_probe_with_environment(jim(), Kind::Boolean, Some(&environment))
            .is_err()
    );
    let c = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let foreign = Value::from_native_scalar_cache_with_storage(
        NativeScalarCache::WordBoolean(true),
        Some(tcl_dialect::TclVersion::V8_6),
        c,
        Some((
            Rc::from(b"true".as_slice()),
            NativeStringStorageIdentity::Allocated,
        )),
    )
    .unwrap();
    let before = foreign.native_object_snapshot();
    assert!(
        foreign
            .native_scalar_probe_with_environment(jim(), Kind::Boolean, Some(&environment))
            .is_err()
    );
    assert_eq!(foreign.native_object_snapshot(), before);
}

#[cfg(all(target_os = "linux", target_env = "gnu", target_pointer_width = "64"))]
#[test]
fn retained_jim_context_queries_its_actual_host_target_without_default_width() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // Actual software context supplies the independent host; descriptive layout
    // does not supply handler admission or a foreign original object.
    let context = NativeJimObjectContext::new(jim()).unwrap();
    let value = original(17);
    value.bind_native_jim_context(&context).unwrap();
    assert!(matches!(
        value.native_scalar_probe(jim(), Kind::Boolean),
        Err(ValueError::ScalarNumericInputUnavailable)
    ));
    assert!(value.resident_string_bytes().is_none());
    context.select_numeric_host(Rc::new(tcl_host_native::NativeHost::new()));
    let Returned::Boolean(boolean) = value
        .native_scalar_probe(jim(), Kind::Boolean)
        .unwrap()
        .unwrap()
    else {
        panic!("primitive Boolean")
    };
    assert_eq!(boolean.returned_integer(), 17);
    assert!(value.resident_string_bytes().is_none());
}

#[test]
fn original_cached_nan_boolean_failure_keeps_absent_string_and_public_error_fields() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // C85+ original case24/getter4 live fields. This primitive receipt does not
    // select a NOT/conditional/ExprBool result or interpreter result header.
    const FIXTURES: [(tcl_dialect::TclVersion, &str); 4] = [
        (
            tcl_dialect::TclVersion::V8_5,
            include_str!(
                "../../tcl-registry/tests/data/native_capi_scalar_publication_original/8.5.19/execute-live.stdout"
            ),
        ),
        (
            tcl_dialect::TclVersion::V8_6,
            include_str!(
                "../../tcl-registry/tests/data/native_capi_scalar_publication_original/8.6.18/execute-live.stdout"
            ),
        ),
        (
            tcl_dialect::TclVersion::V9_0,
            include_str!(
                "../../tcl-registry/tests/data/native_capi_scalar_publication_original/9.0.4/execute-live.stdout"
            ),
        ),
        (
            tcl_dialect::TclVersion::V9_1,
            include_str!(
                "../../tcl-registry/tests/data/native_capi_scalar_publication_original/9.1.0/execute-live.stdout"
            ),
        ),
    ];
    fn field<'a>(row: &'a str, key: &str) -> &'a str {
        row.split('\t')
            .find_map(|part| {
                let (name, value) = part.split_once('=')?;
                (name == key).then_some(value)
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
    for (version, fixture) in FIXTURES {
        let row = fixture
            .lines()
            .find(|row| row.starts_with("ROW\tcase=24\tgetter=4\t"))
            .unwrap();
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let value = Value::native_double(f64::NAN, dialect);
        let alias = value.clone();
        let Err(ValueError::NativeScalarGetter(record)) =
            value.native_scalar_getter(dialect, Kind::Boolean)
        else {
            panic!("{row}")
        };
        assert_eq!(field(row, "code"), "1");
        assert_eq!(field(row, "string_before"), "0");
        assert_eq!(field(row, "string_after"), "0");
        assert!(alias.resident_string_bytes().is_none());
        assert!(
            matches!(alias.native_scalar_cache(),Some(NativeScalarCache::Number(Number::Double(n))) if n.is_nan())
        );
        assert_eq!(hex(record.message_bytes()), field(row, "result"));
        let error_code = match record.error_code_update() {
            tcl_syntax::scalar_getter::NativeScalarGetterErrorCode::Unchanged => {
                b"SEEDED CODE".as_slice()
            }
            tcl_syntax::scalar_getter::NativeScalarGetterErrorCode::Set(code) => code.as_slice(),
        };
        assert_eq!(hex(error_code), field(row, "error_code"));
    }
}

#[test]
fn operand_dependent_scalar_failure_uses_original_getter_after_a_single_neutral_probe() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // Contrasting software transport of C86 Long's original cached Double
    // failure: null probe leavesString0, live rendering reaches its operand.
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let environment = TargetOnly(Some(NativeCIntegerAbi {
        char_bits: 8,
        int_bytes: 4,
        long_bytes: 8,
    }));
    let value = Value::native_double(f64::NAN, dialect);
    let failure = value
        .native_scalar_probe_with_environment(dialect, Kind::Long, Some(&environment))
        .unwrap()
        .unwrap_err();
    assert!(value.resident_string_bytes().is_none());
    let record = value
        .native_scalar_failure_presentation(dialect, Kind::Long, failure)
        .unwrap();
    assert!(value.resident_string_bytes().is_some());
    assert_eq!(record.message_bytes(), b"expected integer but got \"NaN\"");
    assert!(
        matches!(value.native_scalar_cache(),Some(NativeScalarCache::Number(Number::Double(n))) if n.is_nan())
    );
}
