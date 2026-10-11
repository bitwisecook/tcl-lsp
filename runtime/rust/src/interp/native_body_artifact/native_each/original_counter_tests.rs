// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software observations of genuine compiled counter primary retirement.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use crate::interp::native_arguments::OriginalArguments;
use std::cell::Cell;
use tcl_runtime_api::native_compilation::{
    NativeCompiledLocalLayout, NativeCompiledLocalLayoutKind, NativeInterpreterIdentity,
};

fn prepare(interpreter: &Interp) {
    let layout = NativeCompiledLocalLayout {
        owner: NativeInterpreterIdentity {
            owner: NativeInterpreterIdentity::fresh_owner(),
            interpreter: 0,
        },
        token: 1,
        epoch: 0,
        kind: NativeCompiledLocalLayoutKind::Procedure,
        names: vec![None],
    };
    let mut frames = interpreter.frames.borrow_mut();
    frames.push(crate::namespace::GLOBAL);
    frames
        .install_native_compiled_local_layout(&layout)
        .unwrap();
}

#[test]
fn original_compiled_counter_retirement_checks_same_header_temporary_and_frame_before_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Direct layout setup observes the software storage API, not Native admission.
    for mode in [0, 1, 3] {
        let mut interpreter = fixture::native("tcl9.0");
        let _current = fixture::install(&mut interpreter, mode);
        prepare(&interpreter);
        let value = fixture::original(b"OLD");
        interpreter
            .frames
            .borrow_mut()
            .store_native_compiled_temporary(0, value.as_ptr())
            .unwrap();
        let arguments = OriginalArguments::enter(&interpreter, &[value.as_ptr()]).unwrap();
        let frame = interpreter.frames.borrow().current_activation();
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let reached = interpreter.each_counter(0, 17);
        assert_eq!(fixture::FREES.with(Cell::get), 1);
        assert_eq!(fixture::REFS.with(Cell::get), 2);
        assert_eq!(fixture::GETTERS.with(Cell::get), 0);
        assert_eq!(interpreter.get_obj_result(), prior);
        if mode == 0 {
            assert_eq!(reached, Ok(()));
            assert!(core::ptr::eq(
                obj::obj_type_ptr(value.as_ptr()),
                &obj::TCL_INT_TYPE
            ));
            assert_eq!(obj::internal_rep(value.as_ptr()), 17);
            assert!(!obj::has_string_rep(value.as_ptr()));
            assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 2);
            assert_eq!(interpreter.frames.borrow().current_activation(), frame);
            assert_eq!(
                interpreter
                    .frames
                    .borrow()
                    .native_compiled_temporary(0)
                    .unwrap(),
                Some(value.as_ptr())
            );
            assert!(arguments.check_for(&interpreter).is_ok());
            assert!(interpreter.native_execution_refusal().is_none());
        } else {
            assert_eq!(reached, Err(Code::Error));
            let first = interpreter.native_execution_refusal().unwrap();
            if mode == 1 {
                fixture::host(&first);
                assert_eq!(interpreter.frames.borrow().current_activation(), frame);
                assert_eq!(
                    interpreter
                        .frames
                        .borrow()
                        .native_compiled_temporary(0)
                        .unwrap(),
                    Some(value.as_ptr())
                );
                assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 2);
            } else {
                assert_eq!(first, tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original compiled List frame or temporary")));
                assert_ne!(interpreter.frames.borrow().current_activation(), frame);
                assert_eq!(interpreter.frames.borrow().top_level(), 1);
                assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
            }
            assert!(arguments.check_for(&interpreter).is_err());
            assert!(obj::obj_type_ptr(value.as_ptr()).is_null());
            assert!(obj::has_string_rep(value.as_ptr()));
            assert_eq!(&*obj::bytes_of(value.as_ptr()), b"OLD");
        }
    }
}

#[test]
fn original_compiled_fresh_counter_keeps_its_single_temporary_native_role() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interpreter = fixture::native("tcl9.0");
    let _current = fixture::install(&mut interpreter, 0);
    prepare(&interpreter);
    let arguments = OriginalArguments::enter(&interpreter, &[]).unwrap();
    let frame = interpreter.frames.borrow().current_activation();
    interpreter.set_result_bytes(b"PRIOR");
    let prior = interpreter.get_obj_result();
    assert_eq!(interpreter.each_counter(0, 17), Ok(()));
    let value = interpreter
        .frames
        .borrow()
        .native_compiled_temporary(0)
        .unwrap()
        .unwrap();
    assert!(core::ptr::eq(obj::obj_type_ptr(value), &obj::TCL_INT_TYPE));
    assert_eq!(obj::internal_rep(value), 17);
    assert_eq!(unsafe { (*value).ref_count }, 1);
    assert_eq!(fixture::FREES.with(Cell::get), 0);
    assert_eq!(fixture::GETTERS.with(Cell::get), 0);
    assert_eq!(interpreter.get_obj_result(), prior);
    assert_eq!(interpreter.frames.borrow().current_activation(), frame);
    assert!(arguments.check_for(&interpreter).is_ok());
    assert!(interpreter.native_execution_refusal().is_none());
}
