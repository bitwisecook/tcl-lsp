// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software scalar-context controls and independently recorded public outputs.

use super::*;
use crate::{
    capi,
    obj::{self, Owned},
};
use std::{collections::BTreeMap, rc::Rc};
use tcl_platform::{Capabilities, Clock, Env, Host, StdIo};
use tcl_syntax::{number::Number, scalar_getter::NativeScalarCache, value::ValueError};

fn core(environment: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        tcl_registry::model::resolve_environment(environment).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

fn string(bytes: &[u8]) -> Owned {
    Owned::fresh(obj::new_string_bytes(bytes))
}

fn bind(interpreter: &Interp, value: &Owned) {
    // SAFETY: both original owners remain alive.
    unsafe { bind_scalar_getter_context(interpreter, value.as_ptr()) }.unwrap();
}

fn probe(
    value: &Owned,
    kind: NativeScalarGetterKind,
) -> Result<Result<NativeScalarGetterValue, NativeScalarGetterFailure>, NativeScalarObjectAccessError>
{
    // SAFETY: the original owner remains alive throughout the checked read.
    unsafe { probe_scalar_getter(None, value.as_ptr(), kind) }
}

#[test]
fn scalar_null_access_requires_original_engine_and_changes_no_guest_state() {
    // Software owner: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    let mut interpreter = core("tcl8.6");
    interpreter.set_result_bytes(b"SEEDED RESULT");
    interpreter.set_c_error_code(b"SEEDED CODE");
    let value = string(b"17");
    let before = obj::native_object_snapshot(value.as_ptr()).unwrap();
    let refs = unsafe { (*value.as_ptr()).ref_count };
    assert!(matches!(
        probe(&value, NativeScalarGetterKind::Int),
        Err(NativeScalarObjectAccessError::Value(
            ValueError::CommandProtocolUnavailable("native scalar C API object issuer")
        ))
    ));
    let mut out = 777;
    assert_eq!(
        unsafe { capi::Tcl_GetIntFromObj(core::ptr::null_mut(), value.as_ptr(), &mut out) },
        capi::TCL_ERROR
    );
    assert_eq!(out, 777);
    assert_eq!(obj::native_object_snapshot(value.as_ptr()).unwrap(), before);
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, refs);
    assert_eq!(interpreter.result_bytes(), b"SEEDED RESULT");
    assert_eq!(interpreter.error_code(), b"SEEDED CODE");
    bind(&interpreter, &value);
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, refs);
    assert_eq!(
        unsafe { capi::Tcl_GetIntFromObj(core::ptr::null_mut(), value.as_ptr(), &mut out) },
        capi::TCL_OK
    );
    assert_eq!(out, 17);
    assert!(!obj::obj_type_ptr(value.as_ptr()).is_null());
    assert_eq!(interpreter.result_bytes(), b"SEEDED RESULT");
    assert_eq!(interpreter.error_code(), b"SEEDED CODE");
}

#[test]
fn scalar_context_rejects_foreign_engine_and_policy_change_and_restore() {
    // Software owner: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    let mut interpreter = core("tcl8.6");
    let foreign = core("tcl8.6");
    let value = string(b"17");
    bind(&interpreter, &value);
    let before = obj::native_object_snapshot(value.as_ptr()).unwrap();
    assert!(
        unsafe { probe_scalar_getter(Some(&foreign), value.as_ptr(), NativeScalarGetterKind::Int) }
            .is_err()
    );
    assert_eq!(obj::native_object_snapshot(value.as_ptr()).unwrap(), before);
    assert!(probe(&value, NativeScalarGetterKind::Int).unwrap().is_ok());
    let value = string(b"19");
    bind(&interpreter, &value);
    interpreter
        .set_dialect_profile(tcl_registry::model::resolve_environment("tcl8.4").unit_profile());
    interpreter
        .set_dialect_profile(tcl_registry::model::resolve_environment("tcl8.6").unit_profile());
    assert!(matches!(
        probe(&value, NativeScalarGetterKind::Int),
        Err(NativeScalarObjectAccessError::Value(
            ValueError::CommandProtocolUnavailable("stale native scalar object issuer")
        ))
    ));
    assert!(obj::obj_type_ptr(value.as_ptr()).is_null());
    let value = string(b"23");
    bind(&interpreter, &value);
    let original_host = interpreter.host();
    interpreter.set_host(foreign.host());
    interpreter.set_host(original_host);
    assert!(probe(&value, NativeScalarGetterKind::Int).is_err());
    assert!(obj::obj_type_ptr(value.as_ptr()).is_null());
}

#[test]
fn scalar_context_recheck_retains_reached_cache_and_refuses_later_policy_change() {
    // Software stage currency: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // This direct API control makes no native callback chronology claim.
    let interpreter = core("tcl8.6");
    let value = string(b"17");
    let access = native_scalar_context::scalar_access(Some(&interpreter), value.as_ptr()).unwrap();
    let outcome = typed_value::native_scalar_probe_with_environment(
        value.as_ptr(),
        access.dialect,
        NativeScalarGetterKind::Int,
        access.host.numeric_environment(),
    )
    .unwrap();
    assert!(matches!(outcome, Ok(NativeScalarGetterValue::Wide(17))));
    assert!(!obj::obj_type_ptr(value.as_ptr()).is_null());
    interpreter.set_host(interpreter.host());
    assert!(access.ensure_current().is_err());
    assert!(!obj::obj_type_ptr(value.as_ptr()).is_null());
}

#[test]
fn scalar_context_same_profile_world_change_and_restore_cannot_revive_issuer() {
    // Software currency: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    const OVERLAY: u64 = 0x461_C_AB1;
    let mut interpreter = core("tcl8.6");
    let profile = interpreter.dialect_profile();
    let original = interpreter.runtime_context();
    tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |_| {});
    let value = string(b"17");
    bind(&interpreter, &value);
    let before = obj::native_object_snapshot(value.as_ptr()).unwrap();
    let mut changed = original.clone();
    changed.overlay_generation = OVERLAY;
    changed.packages = vec![("scalar-owner-control".to_owned(), "1.0".to_owned())];
    interpreter.pin_context(&changed).unwrap();
    assert!(std::ptr::eq(interpreter.dialect_profile(), profile));
    assert!(probe(&value, NativeScalarGetterKind::Int).is_err());
    interpreter.pin_context(&original).unwrap();
    assert_eq!(interpreter.runtime_context(), original);
    assert!(std::ptr::eq(interpreter.dialect_profile(), profile));
    assert!(matches!(
        probe(&value, NativeScalarGetterKind::Int),
        Err(NativeScalarObjectAccessError::Value(
            ValueError::CommandProtocolUnavailable("stale native scalar object issuer")
        ))
    ));
    assert_eq!(obj::native_object_snapshot(value.as_ptr()).unwrap(), before);
    let fresh = string(b"19");
    bind(&interpreter, &fresh);
    assert!(probe(&fresh, NativeScalarGetterKind::Int).unwrap().is_ok());
}

#[test]
fn scalar_context_duplicate_and_replacement_keep_owner_without_native_refs() {
    // Software owner: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    let interpreter = core("tcl8.6");
    let foreign = core("tcl8.6");
    let original = string(b"17");
    bind(&interpreter, &original);
    let refs = unsafe { (*original.as_ptr()).ref_count };
    let duplicate = Owned::fresh(obj::duplicate(original.as_ptr()));
    assert!(
        probe(&duplicate, NativeScalarGetterKind::Wide)
            .unwrap()
            .is_ok()
    );
    assert_eq!(unsafe { (*original.as_ptr()).ref_count }, refs);
    let unbound = string(b"19");
    obj::duplicate_into(original.as_ptr(), unbound.as_ptr());
    assert!(matches!(
        probe(&original, NativeScalarGetterKind::Wide).unwrap(),
        Ok(NativeScalarGetterValue::Wide(19))
    ));
    let other = string(b"23");
    bind(&foreign, &other);
    obj::duplicate_into(original.as_ptr(), other.as_ptr());
    assert!(probe(&original, NativeScalarGetterKind::Wide).is_err());
    obj::duplicate_into(original.as_ptr(), unbound.as_ptr());
    assert!(probe(&original, NativeScalarGetterKind::Wide).is_err());
    assert!(probe(&other, NativeScalarGetterKind::Wide).unwrap().is_ok());
}

#[test]
fn scalar_context_does_not_keep_engine_or_retired_object_alive() {
    // Software owner: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    let value = string(b"17");
    {
        let interpreter = core("tcl8.6");
        bind(&interpreter, &value);
    }
    assert!(matches!(
        probe(&value, NativeScalarGetterKind::Int),
        Err(NativeScalarObjectAccessError::Value(
            ValueError::CommandProtocolUnavailable("retired native scalar interpreter")
        ))
    ));
    let interpreter = core("tcl8.6");
    let value = string(b"23");
    bind(&interpreter, &value);
    let lifetime = obj::ProcedureObject::retain_lifetime(&value);
    drop(value);
    assert!(
        unsafe { probe_scalar_getter(None, lifetime.as_ptr(), NativeScalarGetterKind::Int) }
            .is_err()
    );
    drop(lifetime);
}

struct NoNumeric(Rc<dyn Host>);
impl Host for NoNumeric {
    fn capabilities(&self) -> Capabilities {
        self.0.capabilities()
    }
    fn clock(&self) -> &dyn Clock {
        self.0.clock()
    }
    fn stdio(&self) -> &dyn StdIo {
        self.0.stdio()
    }
    fn env(&self) -> &dyn Env {
        self.0.env()
    }
}

#[test]
fn scalar_context_missing_numeric_abi_and_first_host_cause_are_terminal() {
    // Software owner: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    let mut interpreter = core("tcl8.6");
    interpreter.set_host(Rc::new(NoNumeric(interpreter.host())));
    let value = string(b"17");
    assert!(matches!(
        unsafe {
            probe_scalar_getter(
                Some(&interpreter),
                value.as_ptr(),
                NativeScalarGetterKind::Int,
            )
        },
        Err(NativeScalarObjectAccessError::Value(
            ValueError::ScalarNumericInputUnavailable
        ))
    ));
    assert!(obj::obj_type_ptr(value.as_ptr()).is_null());
    assert!(
        crate::engine_abi::value_carriers::scalar(
            &mut interpreter,
            &tcl_core_types::NativeScalarCache::Integer(17)
        )
        .is_err()
    );
    let mut interpreter = core("tcl8.6");
    bind(&interpreter, &value);
    let first = tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
        "original selected refusal",
    );
    interpreter.refuse_native_access(first.clone());
    assert!(
        matches!(probe(&value, NativeScalarGetterKind::Int), Err(NativeScalarObjectAccessError::Execution(
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(cause)
    )) if cause == first)
    );
    let mut out = 777;
    assert_eq!(
        unsafe { capi::Tcl_GetIntFromObj(&mut interpreter, value.as_ptr(), &mut out) },
        capi::TCL_ERROR
    );
    assert_eq!(out, 777);
    assert_eq!(
        interpreter.native_execution_refusal(),
        Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            first
        ))
    );
    assert!(obj::obj_type_ptr(value.as_ptr()).is_null());
}

#[test]
fn scalar_engine_importer_issues_context_only_for_selected_scalar_purpose() {
    // Software owner: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    let mut interpreter = core("tcl8.6");
    let value = crate::engine_abi::value_carriers::scalar(
        &mut interpreter,
        &tcl_core_types::NativeScalarCache::Integer(17),
    )
    .unwrap();
    assert!(matches!(
        probe(&value, NativeScalarGetterKind::Wide).unwrap(),
        Ok(NativeScalarGetterValue::Wide(17))
    ));
    interpreter.set_host(Rc::new(NoNumeric(interpreter.host())));
    let binary = crate::engine_abi::value_carriers::byte_array(&mut interpreter, b"17").unwrap();
    assert!(
        obj::scalar_object_context(binary.as_ptr())
            .unwrap()
            .is_none()
    );
    assert!(probe(&binary, NativeScalarGetterKind::Wide).is_err());
}

fn fixture_row(fixture: &str, case: usize, getter: usize) -> BTreeMap<&str, &str> {
    fixture
        .lines()
        .filter(|row| row.starts_with("ROW\t"))
        .map(|row| {
            row.split('\t')
                .skip(1)
                .filter_map(|field| field.split_once('='))
                .collect::<BTreeMap<_, _>>()
        })
        .find(|row| {
            row["case"].parse::<usize>().unwrap() == case
                && row["getter"].parse::<usize>().unwrap() == getter
        })
        .expect("exact original public fixture row")
}

fn decode_hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn original_case(case: usize) -> Owned {
    match case {
        1 => string(b"17"),
        16 => string(b"bad"),
        19 => string(b"1\0X"),
        23 => Owned::fresh(obj::new_double_obj(17.0)),
        24 => Owned::fresh(obj::new_double_obj(f64::NAN)),
        26 => Owned::fresh(obj::new_wide_int_obj(4_294_967_296)),
        _ => unreachable!("bounded original constructor matrix"),
    }
}

fn cache_name(value: &Owned, version: tcl_dialect::TclVersion) -> &'static str {
    match obj::native_scalar_cache(value.as_ptr()).unwrap() {
        Some(NativeScalarCache::Tcl84Long(_)) => "int",
        Some(NativeScalarCache::Number(Number::Int(_)))
            if version == tcl_dialect::TclVersion::V8_4 =>
        {
            "wideInt"
        }
        Some(NativeScalarCache::Number(Number::Int(_))) => "int",
        Some(NativeScalarCache::Number(Number::Double(_) | Number::Nan { .. })) => "double",
        Some(NativeScalarCache::WordBoolean(_))
            if matches!(
                version,
                tcl_dialect::TclVersion::V8_4
                    | tcl_dialect::TclVersion::V9_0
                    | tcl_dialect::TclVersion::V9_1
            ) =>
        {
            "boolean"
        }
        Some(NativeScalarCache::WordBoolean(_)) => "booleanString",
        None => "NULL",
        other => panic!("unexpected bounded cache: {other:?}"),
    }
}

unsafe fn call(interpreter: *mut Interp, value: &Owned, getter: usize) -> (c_int, String) {
    let (mut int, mut long, mut wide, mut double) = (777, 777, 777, 777.0_f64);
    let code = match getter {
        0 => unsafe { capi::Tcl_GetIntFromObj(interpreter, value.as_ptr(), &mut int) },
        1 => unsafe { capi::Tcl_GetLongFromObj(interpreter, value.as_ptr(), &mut long) },
        2 => unsafe { capi::Tcl_GetWideIntFromObj(interpreter, value.as_ptr(), &mut wide) },
        3 => unsafe { capi::Tcl_GetDoubleFromObj(interpreter, value.as_ptr(), &mut double) },
        4 => unsafe { capi::Tcl_GetBooleanFromObj(interpreter, value.as_ptr(), &mut int) },
        _ => unreachable!("five public scalar getters"),
    };
    let output = match getter {
        0 | 4 => int.to_string(),
        1 => long.to_string(),
        2 => wide.to_string(),
        3 => format!("{:016x}", double.to_bits()),
        _ => unreachable!(),
    };
    (code, output)
}

fn compare_original_row(environment: &str, fixture: &str, null: bool, case: usize, getter: usize) {
    let row = fixture_row(fixture, case, getter);
    let mut interpreter = core(environment);
    interpreter.set_result_bytes(b"SEEDED RESULT");
    interpreter.set_c_error_code(b"SEEDED CODE");
    let original = original_case(case);
    bind(&interpreter, &original);
    let version = interpreter.native_invocation_dialect().tcl_version.unwrap();
    let original_string = unsafe { (*original.as_ptr()).bytes };
    let pointer = if null {
        core::ptr::null_mut()
    } else {
        &mut interpreter
    };
    let (code, output) = unsafe { call(pointer, &original, getter) };
    assert!(
        !interpreter.host_refusal_pending(),
        "{environment}: {row:?}: {:?}",
        interpreter.native_execution_refusal()
    );
    assert_eq!(
        code,
        row["code"].parse::<c_int>().unwrap(),
        "{environment}: {row:?}"
    );
    let field = ["int", "long", "wide", "double_bits", "int"][getter];
    assert_eq!(output, row[field], "{environment}: {row:?}");
    // This comparison projects the reached shared cache model, not a native header issuer.
    assert_eq!(
        cache_name(&original, version),
        row["after"],
        "{environment}: {row:?}"
    );
    assert_eq!(
        obj::has_string_rep(original.as_ptr()),
        row["string_after"] == "1",
        "{environment}: {row:?}"
    );
    assert_eq!(
        !original_string.is_null() && unsafe { (*original.as_ptr()).bytes } == original_string,
        row["string_same"] == "1",
        "{environment}: {row:?}"
    );
    assert_eq!(
        interpreter.result_bytes(),
        decode_hex(row["result"]),
        "{environment}: {row:?}"
    );
    assert_eq!(
        interpreter.error_code(),
        decode_hex(row["error_code"]),
        "{environment}: {row:?}"
    );
}

#[test]
fn scalar_public_facade_matches_recorded_live_and_null_c_primitives() {
    // Native public rows: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // This software facade control needs its actual engine/Host/ABI issuer; rows do not issue it.
    let fixtures = [
        (
            "tcl8.4",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/8.4.20/execute-live.stdout"
            ),
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/8.4.20/execute-null.stdout"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/8.5.19/execute-live.stdout"
            ),
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/8.5.19/execute-null.stdout"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/8.6.18/execute-live.stdout"
            ),
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/8.6.18/execute-null.stdout"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/9.0.4/execute-live.stdout"
            ),
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/9.0.4/execute-null.stdout"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/9.1.0/execute-live.stdout"
            ),
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/9.1.0/execute-null.stdout"
            ),
        ),
    ];
    for (environment, live, null) in fixtures {
        for (null_interpreter, fixture) in [(false, live), (true, null)] {
            for case in [1, 16, 19, 23, 24, 26] {
                for getter in 0..5 {
                    compare_original_row(environment, fixture, null_interpreter, case, getter);
                }
            }
        }
    }
}

#[test]
fn scalar_jim_context_cannot_borrow_a_foreign_original_numeric_host() {
    // Software owner: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    let interpreter = core("jim");
    let foreign = core("jim");
    let value = string(b"17");
    foreign
        .associate_native_jim_arguments(&[value.as_ptr()])
        .unwrap();
    let before = obj::native_object_snapshot(value.as_ptr()).unwrap();
    assert!(unsafe { bind_scalar_getter_context(&interpreter, value.as_ptr()) }.is_err());
    assert!(
        obj::scalar_object_context(value.as_ptr())
            .unwrap()
            .is_none()
    );
    assert_eq!(obj::native_object_snapshot(value.as_ptr()).unwrap(), before);
    bind(&foreign, &value);
    assert!(matches!(
        probe(&value, NativeScalarGetterKind::Long).unwrap(),
        Ok(NativeScalarGetterValue::Wide(17))
    ));
}

#[test]
fn scalar_jim_public_boolean_keeps_raw_integer_separate_from_expression_truth() {
    // Native public rows: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // Expression459 is independently measured; this control reaches only the primitive.
    let fixture = include_str!(
        "../../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/linked-jim458/jim/execute-live.stdout"
    );
    let mut interpreter = core("jim");
    for (case, integer) in [(22, 17), (26, 4_294_967_296)] {
        let value = Owned::fresh(obj::new_wide_int_obj(integer));
        interpreter
            .associate_native_jim_arguments(&[value.as_ptr()])
            .unwrap();
        let row = fixture_row(fixture, case, 4);
        let mut returned = 777;
        assert_eq!(
            unsafe { capi::Tcl_GetBooleanFromObj(&mut interpreter, value.as_ptr(), &mut returned) },
            capi::TCL_OK
        );
        assert_eq!(returned, row["int"].parse::<i32>().unwrap());
        assert_eq!(returned, if case == 22 { 17 } else { 0 });
        assert!(!obj::has_string_rep(value.as_ptr()));
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    }
}
