// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked original compiled List and real activation observations.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use std::cell::Cell;
#[test]
fn original_compiled_list_length_retirement_checks_actual_frame_birth_before_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Same logical depth after pop/push cannot revive this original activation.
    let mut interpreter = fixture::native("tcl9.0");
    let _current = fixture::install(&mut interpreter, 3);
    interpreter
        .frames
        .borrow_mut()
        .push(crate::namespace::GLOBAL);
    let value = fixture::original(b"A B");
    let entered = BodyListArguments::capture(&interpreter, value.as_ptr(), None).unwrap();
    let frame = interpreter.frames.borrow().current_activation();
    let failure = entered.length(&mut interpreter).unwrap_err();
    assert_eq!(
        failure,
        CmdError::from(ValueError::CommandProtocolUnavailable(
            "original compiled List frame or temporary"
        ))
    );
    assert_eq!(
        failure.native_execution_refusal(),
        interpreter.native_execution_refusal().as_ref()
    );
    assert_ne!(interpreter.frames.borrow().current_activation(), frame);
    assert_eq!(interpreter.frames.borrow().top_level(), 1);
    assert_eq!(fixture::FREES.with(Cell::get), 1);
    assert_eq!(fixture::REFS.with(Cell::get), 1);
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    assert!(!core::ptr::eq(
        obj::obj_type_ptr(value.as_ptr()),
        &crate::list::TCL_LIST_TYPE
    ));
    assert!(entered.check(&interpreter).is_err());
}
#[test]
fn original_compiled_list_current_length_has_one_native_input_role_and_no_frame_hold() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interpreter = fixture::native("tcl9.0");
    let _current = fixture::install(&mut interpreter, 0);
    interpreter
        .frames
        .borrow_mut()
        .push(crate::namespace::GLOBAL);
    let value = fixture::original(b"A B");
    let frame = interpreter.frames.borrow().current_activation();
    let entered = BodyListArguments::capture(&interpreter, value.as_ptr(), None).unwrap();
    assert_eq!(entered.length(&mut interpreter).unwrap(), 2);
    assert_eq!(fixture::FREES.with(Cell::get), 1);
    assert_eq!(fixture::REFS.with(Cell::get), 1);
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    assert_eq!(interpreter.frames.borrow().current_activation(), frame);
    assert!(entered.check(&interpreter).is_ok());
    assert!(interpreter.native_execution_refusal().is_none());
}
#[test]
fn original_compiled_list_temporary_projection_refuses_replaced_slot_without_new_lookup() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_runtime_api::native_compilation::{
        NativeCompiledLocalLayout, NativeCompiledLocalLayoutKind, NativeInterpreterIdentity,
    };
    let interpreter = fixture::native("tcl9.0");
    let value = fixture::string(b"A B");
    let replacement = fixture::string(b"C D");
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
    {
        let mut frames = interpreter.frames.borrow_mut();
        frames.push(crate::namespace::GLOBAL);
        frames
            .install_native_compiled_local_layout(&layout)
            .unwrap();
        frames
            .store_native_compiled_temporary(0, value.as_ptr())
            .unwrap();
    }
    let entered = BodyListArguments::capture(&interpreter, value.as_ptr(), Some(0)).unwrap();
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 2);
    interpreter
        .frames
        .borrow_mut()
        .store_native_compiled_temporary(0, replacement.as_ptr())
        .unwrap();
    let first = entered.check(&interpreter).unwrap_err();
    assert_eq!(
        first,
        CmdError::from(ValueError::CommandProtocolUnavailable(
            "original compiled List frame or temporary"
        ))
    );
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    assert_eq!(unsafe { (*replacement.as_ptr()).ref_count }, 2);
    assert_eq!(
        interpreter
            .frames
            .borrow()
            .native_compiled_temporary(0)
            .unwrap(),
        Some(replacement.as_ptr())
    );
}

#[test]
fn original_compiled_group_retirement_keeps_frame_check_after_header_is_released() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The real native header is retired before this pure frame check; the
    // allocation must not gain a protective reference to keep cleanup current.
    let mut interpreter = fixture::native("tcl9.0");
    let _current = fixture::install(&mut interpreter, 3);
    interpreter
        .frames
        .borrow_mut()
        .push(crate::namespace::GLOBAL);
    let arguments = OriginalArguments::enter(&interpreter, &[]).unwrap();
    let frame = BodyListFrame::capture(&interpreter);
    let value = fixture::original(b"A B");
    interpreter.set_result_bytes(b"PRIOR");
    let prior = interpreter.get_obj_result();
    drop(value);
    let first = frame.check(&interpreter, &arguments).unwrap_err();
    assert_eq!(
        first,
        CmdError::from(ValueError::CommandProtocolUnavailable(
            "original compiled List frame or temporary"
        ))
    );
    assert_eq!(
        first.native_execution_refusal(),
        interpreter.native_execution_refusal().as_ref()
    );
    assert_eq!(fixture::FREES.with(Cell::get), 1);
    assert_eq!(fixture::REFS.with(Cell::get), 0);
    assert_eq!(interpreter.get_obj_result(), prior);
    assert_eq!(interpreter.frames.borrow().top_level(), 1);
}
