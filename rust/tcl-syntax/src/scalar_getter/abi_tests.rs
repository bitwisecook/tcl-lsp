/tmp/grammar-wasi-numeric-adapter444/candidate/rust/tcl-syntax/src/scalar_getter/abi_tests.rs:

// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

#[test]
fn queried_output_layout_does_not_supply_long64_or_a_c84_build_recipe() {
    // naming.numeric.wasi-libc-environment
    // docs/design/analysis/name-resolution-proofs/numeric-wasi-libc-environment.md
    // Descriptive layout/software eligibility only: the standalone Rust guest
    // observation supplies no original Tcl32 getter or engine/header issuer.
    let c86 = NativeScalarGetterProtocol::for_tcl_version(TclVersion::V8_6);
    let cached = NativeScalarCache::Number(Number::Int(16));
    for kind in [
        NativeScalarGetterKind::Int,
        NativeScalarGetterKind::Wide,
        NativeScalarGetterKind::Double,
        NativeScalarGetterKind::Boolean,
    ] {
        assert!(
            c86.validate_primitive_abi(kind, NativeScalarGetterStage::Cached(&cached), 8, 4, 4)
                .is_ok()
        );
    }
    assert!(
        c86.validate_primitive_abi(
            NativeScalarGetterKind::Long,
            NativeScalarGetterStage::Cached(&cached),
            8,
            4,
            4
        )
        .is_err()
    );
    let jim =
        NativeScalarGetterProtocol::for_point(DialectPoint::canonical(Release::JIM_0_84)).unwrap();
    assert!(
        jim.validate_primitive_abi(
            NativeScalarGetterKind::Wide,
            NativeScalarGetterStage::Cached(&cached),
            8,
            4,
            4
        )
        .is_ok()
    );
    assert!(
        jim.validate_primitive_abi(
            NativeScalarGetterKind::Boolean,
            NativeScalarGetterStage::Cached(&cached),
            8,
            4,
            4
        )
        .is_err()
    );
    assert!(NativeScalarGetterTarget::from_c_integer_abi(8, 4, 4).is_err());
    assert!(
        c86.validate_primitive_abi(
            NativeScalarGetterKind::Int,
            NativeScalarGetterStage::Cached(&cached),
            8,
            8,
            8
        )
        .is_err()
    );
}

#[test]
fn c84_primary_width_and_reached_fresh_recipe_are_independent() {
    // naming.numeric.wasi-libc-environment
    // docs/design/analysis/name-resolution-proofs/numeric-wasi-libc-environment.md
    // Source equations/software contract: a representable retained long is a
    // direct return, while fresh original build-dependent parsing is distinct.
    let c84 = NativeScalarGetterProtocol::for_tcl_version(TclVersion::V8_4);
    for value in [i64::from(i32::MIN), 16, i64::from(i32::MAX)] {
        let cache = NativeScalarCache::Tcl84Long(value);
        c84.validate_cache_abi(&cache, 8, 4, 4).unwrap();
        c84.validate_primitive_abi(
            NativeScalarGetterKind::Long,
            NativeScalarGetterStage::Cached(&cache),
            8,
            4,
            4,
        )
        .unwrap();
        assert_eq!(
            c84.cached_conversion(NativeScalarGetterKind::Long, &cache, None)
                .unwrap()
                .unwrap()
                .outcome(),
            Ok(NativeScalarGetterValue::Wide(value))
        );
    }
    let outside = i64::from(i32::MAX) + 1;
    assert!(
        c84.validate_cache_abi(&NativeScalarCache::Tcl84Long(outside), 8, 4, 4)
            .is_err()
    );
    let genuine_wide = NativeScalarCache::Number(Number::Int(outside));
    c84.validate_cache_abi(&genuine_wide, 8, 4, 4).unwrap();
    c84.validate_primitive_abi(
        NativeScalarGetterKind::Wide,
        NativeScalarGetterStage::Cached(&genuine_wide),
        8,
        4,
        4,
    )
    .unwrap();
    assert!(
        c84.validate_primitive_abi(
            NativeScalarGetterKind::Long,
            NativeScalarGetterStage::Cached(&genuine_wide),
            8,
            4,
            4
        )
        .is_err()
    );
    for kind in [
        NativeScalarGetterKind::Int,
        NativeScalarGetterKind::Long,
        NativeScalarGetterKind::Wide,
        NativeScalarGetterKind::Boolean,
    ] {
        assert!(
            c84.validate_primitive_abi(kind, NativeScalarGetterStage::Fresh, 8, 4, 4)
                .is_err()
        );
    }
    c84.validate_primitive_abi(
        NativeScalarGetterKind::Double,
        NativeScalarGetterStage::Fresh,
        8,
        4,
        4,
    )
    .unwrap();
    c84.validate_cache_abi(&NativeScalarCache::Tcl84Long(i64::MAX), 8, 4, 8)
        .unwrap();
}
