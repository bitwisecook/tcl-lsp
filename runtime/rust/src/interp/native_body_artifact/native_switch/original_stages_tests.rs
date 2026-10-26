// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real original compiled switch stages; software callbacks carry no Native chronology.
use super::super::native_list_conversion::BodyObjectOperation;
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use std::cell::Cell;
use tcl_syntax::scalar_getter::{
    NativeScalarGetterKind as Kind, NativeScalarGetterValue as Scalar,
};

#[test]
fn original_switch_exact_lower_keeps_actual_shared_and_unshared_roles() {

    for shared in [false, true] {
        let mut interpreter = fixture::native("tcl9.1");
        let _current = fixture::install(&mut interpreter, 0);
        let operation = BodyObjectOperation::capture(&interpreter).unwrap();
        let original = fixture::original(b"ABC\0TAIL");
        let extra = shared.then(|| obj::Owned::retain(original.as_ptr()));
        let (key, transformed) = interpreter
            .body_switch_exact_key(
                &operation,
                original.as_ptr(),
                tcl_dialect::TclVersion::V9_1,
                true,
            )
            .unwrap();
        assert_eq!(key, b"abc");
        assert_eq!(
            unsafe { (*original.as_ptr()).ref_count },
            if shared { 2 } else { 1 }
        );
        if shared {
            let transformed = transformed.unwrap();
            assert_eq!(obj::bytes_of(transformed.as_ptr()), b"abc");
            assert_eq!(unsafe { (*transformed.as_ptr()).ref_count }, 1);
            assert_eq!(obj::bytes_of(original.as_ptr()), b"ABC\0TAIL");
            assert_eq!(fixture::FREES.with(Cell::get), 0);
            drop(transformed);
        } else {
            assert!(transformed.is_none());
            assert_eq!(obj::bytes_of(original.as_ptr()), b"abc");
            assert_eq!(fixture::FREES.with(Cell::get), 1);
            assert_eq!(fixture::REFS.with(Cell::get), 1);
        }
        drop(extra);
        operation.check(&interpreter).unwrap();
        assert!(interpreter.native_execution_refusal().is_none());
    }
}

#[test]
fn original_switch_scalar_retirement_checks_frame_and_first_host_before_cache() {

    for (kind, bytes) in [
        (Kind::Wide, b"42".as_slice()),
        (Kind::Boolean, b"1".as_slice()),
    ] {
        for mode in [0, 1, 3] {
            let mut interpreter = fixture::native("tcl9.1");
            let _current = fixture::install(&mut interpreter, mode);
            interpreter
                .frames
                .borrow_mut()
                .push(crate::namespace::GLOBAL);
            let operation = BodyObjectOperation::capture(&interpreter).unwrap();
            let original = fixture::original(bytes);
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            let reached = operation.scalar(&interpreter, original.as_ptr(), kind);
            if mode == 0 {
                match reached.unwrap() {
                    Scalar::Wide(value) => assert_eq!(value, 42),
                    Scalar::Boolean(value) => assert!(value.is_true()),
                    _ => panic!("selected original scalar kind"),
                }
                assert!(obj::native_scalar_cache(original.as_ptr())
                    .unwrap()
                    .is_some());
                operation.check(&interpreter).unwrap();
            } else {
                let first = reached.unwrap_err();
                assert_eq!(
                    first.native_execution_refusal(),
                    interpreter.native_execution_refusal().as_ref()
                );
                if mode == 1 {
                    fixture::host(first.native_execution_refusal().unwrap());
                } else {
                    assert_eq!(
                        first,
                        tcl_cmd_core::CmdError::from(ValueError::CommandProtocolUnavailable(
                            "original compiled List frame or temporary",
                        ))
                    );
                }
                assert!(obj::native_scalar_cache(original.as_ptr())
                    .unwrap()
                    .is_none());
                assert!(operation.check(&interpreter).is_err());
            }
            assert_eq!(fixture::FREES.with(Cell::get), 1);
            assert_eq!(fixture::REFS.with(Cell::get), 1);
            assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
            assert_eq!(obj::bytes_of(original.as_ptr()), bytes);
            assert_eq!(interpreter.get_obj_result(), prior);
        }
    }
}

#[test]
fn original_switch_lower_refusal_precedes_string_mutation_and_later_stage() {

    for mode in [1, 2, 3] {
        let mut interpreter = fixture::native("tcl9.1");
        let _current = fixture::install(&mut interpreter, mode);
        interpreter
            .frames
            .borrow_mut()
            .push(crate::namespace::GLOBAL);
        let operation = BodyObjectOperation::capture(&interpreter).unwrap();
        let original = fixture::original(b"ABC");
        let later = fixture::later();
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let reached = interpreter.body_switch_exact_key(
            &operation,
            original.as_ptr(),
            tcl_dialect::TclVersion::V9_1,
            true,
        );
        let first = match reached {
            Ok(_) => {
                let _ = operation.bytes(&interpreter, later.as_ptr());
                panic!("withdrawn original lower must refuse");
            }
            Err(first) => first,
        };
        assert_eq!(
            first.native_execution_refusal(),
            interpreter.native_execution_refusal().as_ref()
        );
        if mode == 1 {
            fixture::host(first.native_execution_refusal().unwrap());
        }
        assert_eq!(fixture::FREES.with(Cell::get), 1);
        assert_eq!(fixture::REFS.with(Cell::get), 1);
        assert_eq!(fixture::GETTERS.with(Cell::get), 0);
        assert_eq!(obj::bytes_of(original.as_ptr()), b"ABC");
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert!(operation.check(&interpreter).is_err());
    }
}

#[test]
fn original_switch_guest_scalar_failure_preserves_existing_public_getter_fields() {

    let interpreter = fixture::native("tcl9.1");
    let operation = BodyObjectOperation::capture(&interpreter).unwrap();
    let original = fixture::string(b"not-a-number");
    let baseline = fixture::string(b"not-a-number");
    let expected =
        match crate::capi::read_scalar_for_interpreter(&interpreter, baseline.as_ptr(), Kind::Wide)
        {
            Err(crate::capi::ScalarReadError::Guest(error)) => tcl_cmd_core::CmdError::from(error),
            _ => panic!("the real public getter must reject this original number"),
        };
    let actual = operation
        .scalar(&interpreter, original.as_ptr(), Kind::Wide)
        .unwrap_err();
    assert_eq!(actual, expected);
    assert!(actual.native_execution_refusal().is_none());
    assert!(interpreter.native_execution_refusal().is_none());
    assert_eq!(obj::bytes_of(original.as_ptr()), b"not-a-number");
    assert!(obj::native_scalar_cache(original.as_ptr())
        .unwrap()
        .is_none());
    assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
    operation.check(&interpreter).unwrap();
}

#[test]
fn original_switch_selected_bodies_keep_exact_integer_and_empty_regexp_results() {

    let mut interpreter = fixture::native("tcl9.1");
    for script in [
        b"proc p x {switch -exact -nocase $x {abc {return YES} default {return NO}}}; p ABC"
            .as_slice(),
        b"proc p x {switch -integer $x {42 {return YES} default {return NO}}}; p 42".as_slice(),
        b"proc p x {switch -regexp $x {{} {return YES} default {return NO}}}; p ABC".as_slice(),
    ] {
        assert_eq!(interpreter.eval_str(script), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"YES");
        assert!(interpreter.native_execution_refusal().is_none());
    }
}
