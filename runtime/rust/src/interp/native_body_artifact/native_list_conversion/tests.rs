// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked original compiled List and real activation observations.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use std::cell::Cell;
#[test]
fn original_compiled_list_length_retirement_checks_actual_frame_birth_before_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport

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

#[test]
fn original_compiled_expansion_members_keep_same_frame_and_first_cause_before_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual compiled expansion helper; extension callbacks are software controls.
    for mode in [0, 1, 2, 3] {
        let mut interpreter = fixture::native("tcl9.0");
        let _current = fixture::install(&mut interpreter, mode);
        interpreter
            .frames
            .borrow_mut()
            .push(crate::namespace::GLOBAL);
        let value = fixture::original(b"A B");
        let entered = BodyListArguments::capture(&interpreter, value.as_ptr(), None).unwrap();
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let before = crate::counters::live_objs();
        let reached = entered.members(
            &interpreter,
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
        );
        assert_eq!(fixture::FREES.with(Cell::get), 1);
        assert_eq!(fixture::REFS.with(Cell::get), 1);
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        assert_eq!(fixture::GETTERS.with(Cell::get), 0);
        assert_eq!(interpreter.get_obj_result(), prior);
        if mode == 0 {
            assert_eq!(reached.unwrap().len(), 2);
            assert!(entered.check(&interpreter).is_ok());
            assert!(core::ptr::eq(
                obj::obj_type_ptr(value.as_ptr()),
                &crate::list::TCL_LIST_TYPE
            ));
        } else {
            let first = reached.unwrap_err();
            assert_eq!(
                first.native_execution_refusal(),
                interpreter.native_execution_refusal().as_ref()
            );
            if mode == 1 {
                fixture::host(first.native_execution_refusal().unwrap());
            } else if mode == 3 {
                assert_eq!(
                    first,
                    CmdError::from(ValueError::CommandProtocolUnavailable(
                        "original compiled List frame or temporary"
                    ))
                );
            } else {
                assert!(matches!(first.native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation")))));
            }
            assert!(entered.check(&interpreter).is_err());
            assert!(!core::ptr::eq(
                obj::obj_type_ptr(value.as_ptr()),
                &crate::list::TCL_LIST_TYPE
            ));
            assert_eq!(crate::counters::live_objs(), before);
        }
    }
}

#[test]
fn original_compiled_dictionary_prepare_retains_frame_and_first_cause_before_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Software callback and real preparation roles, not a Native free schedule.
    use tcl_cmd_core::native_dictionary::NativeDictionaryObjects;
    for mode in [0, 1, 2, 3] {
        let mut interpreter = fixture::native("tcl9.0");
        let _current = fixture::install(&mut interpreter, mode);
        interpreter
            .frames
            .borrow_mut()
            .push(crate::namespace::GLOBAL);
        let operation = BodyObjectOperation::capture(&interpreter).unwrap();
        let value = fixture::original(b"A B");
        let borrowed = crate::value_ops::RuntimeAppendValue::borrowed(value.as_ptr());
        let objects = operation.objects(&interpreter).unwrap();
        let reached = objects.prepare(Some(&borrowed));
        match reached {
            Ok(prepared) => {
                assert_eq!(mode, 0);
                assert_eq!(prepared.original(), value.as_ptr());
                assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 2);
                assert_eq!(
                    crate::dict::native_dict_pairs(value.as_ptr(), objects.protocol)
                        .unwrap()
                        .len(),
                    1
                );
                drop(prepared);
                operation.check(&interpreter).unwrap();
            }
            Err(first) => {
                assert_ne!(mode, 0);
                assert_eq!(
                    first.native_execution_refusal(),
                    interpreter.native_execution_refusal().as_ref()
                );
                if mode == 1 {
                    fixture::host(first.native_execution_refusal().unwrap());
                }
                assert!(!core::ptr::eq(
                    obj::obj_type_ptr(value.as_ptr()),
                    &crate::dict::TCL_DICT_TYPE
                ));
                assert!(operation.check(&interpreter).is_err());
            }
        }
        assert_eq!(fixture::FREES.with(Cell::get), 1);
        assert_eq!(fixture::REFS.with(Cell::get), 1);
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    }
}

thread_local! {
    static ORIGINAL_COMPOUND: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
    static ORIGINAL_UPDATES: Cell<usize> = const { Cell::new(0) };
    static ORIGINAL_CHILD_REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
}
struct OriginalCompoundObservation;
impl Drop for OriginalCompoundObservation {
    fn drop(&mut self) {
        ORIGINAL_COMPOUND.with(|root| root.set(core::ptr::null_mut()));
    }
}
extern "C" fn replace_original_compound(child: *mut TclObj) {
    ORIGINAL_UPDATES.with(|count| count.set(count.get() + 1));
    ORIGINAL_CHILD_REFS.with(|refs| refs.set(unsafe { (*child).ref_count }));
    unsafe { obj::set_native_updater_string_rep(child, b"A", false) };
    let root = ORIGINAL_COMPOUND.with(Cell::get);
    assert!(!root.is_null());
    obj::change_type(root, core::ptr::null(), 0);
    unsafe { obj::set_native_updater_string_rep(root, b"REACHED", false) };
}
static ORIGINAL_COMPOUND_CHILD: obj::TclObjType = obj::TclObjType {
    name: c"originalCompiledCompoundChild".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(replace_original_compound),
    set_from_any_proc: None,
};

#[test]
fn original_compiled_string_child_retirement_blocks_later_updater_and_parent_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport

    let mut interpreter = fixture::native("tcl9.0");
    let _current = fixture::install(&mut interpreter, 0);
    let operation = BodyObjectOperation::capture(&interpreter).unwrap();
    let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0);
    let child = obj::alloc_typed(&ORIGINAL_COMPOUND_CHILD, 0);
    let later = fixture::later();
    let later_raw = later.as_ptr();
    let later_lifetime = obj::NativeObjectLifetime::retain(later_raw);
    let root = obj::Owned::fresh(crate::list::new_list_obj_native(
        &[child, later_raw],
        protocol,
    ));
    drop(later);
    ORIGINAL_COMPOUND.with(|selected| selected.set(root.as_ptr()));
    ORIGINAL_UPDATES.with(|count| count.set(0));
    let _observation = OriginalCompoundObservation;
    let first = operation.bytes(&interpreter, root.as_ptr()).unwrap_err();
    assert_eq!(
        first.native_execution_refusal(),
        interpreter.native_execution_refusal().as_ref()
    );
    assert_eq!(ORIGINAL_UPDATES.with(Cell::get), 1);
    assert_eq!(ORIGINAL_CHILD_REFS.with(Cell::get), 1);
    assert_eq!(fixture::GETTERS.with(Cell::get), 0);
    assert_eq!(obj::bytes_of(root.as_ptr()), b"REACHED");
    assert!(obj::check_native_liveness(later_raw).is_err());
    assert_eq!(unsafe { (*root.as_ptr()).ref_count }, 1);
    assert_eq!(unsafe { (*later_lifetime.as_ptr()).ref_count }, 0);
}

#[test]
fn original_prepared_dictionary_key_updater_checks_parent_before_member_callback() {
    // naming.embedding.original-host-publication-and-fact-transport

    use tcl_cmd_core::native_dictionary::NativeDictionaryObjects;
    let interpreter = fixture::native("tcl9.0");
    let operation = BodyObjectOperation::capture(&interpreter).unwrap();
    let source = fixture::string(b"A B");
    let objects = operation.objects(&interpreter).unwrap();
    let dictionary = objects
        .prepare(Some(&crate::value_ops::RuntimeAppendValue::borrowed(
            source.as_ptr(),
        )))
        .unwrap();
    let key = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL_COMPOUND_CHILD, 0));
    ORIGINAL_COMPOUND.with(|root| root.set(dictionary.original()));
    ORIGINAL_UPDATES.with(|count| count.set(0));
    let _observation = OriginalCompoundObservation;
    let called = Cell::new(0);
    let reached = objects.with_member(
        &dictionary,
        &crate::value_ops::RuntimeAppendValue::borrowed(key.as_ptr()),
        |_| {
            called.set(called.get() + 1);
            Ok(())
        },
    );
    let first = reached.unwrap_err();
    assert_eq!(
        first.native_execution_refusal(),
        interpreter.native_execution_refusal().as_ref()
    );
    assert_eq!(called.get(), 0);
    assert_eq!(ORIGINAL_UPDATES.with(Cell::get), 1);
    assert_eq!(ORIGINAL_CHILD_REFS.with(Cell::get), 1);
    assert_eq!(obj::bytes_of(dictionary.original()), b"REACHED");
    assert!(dictionary.check_prepared().is_err());
    assert_eq!(unsafe { (*key.as_ptr()).ref_count }, 1);
}

#[test]
fn original_compiled_return_integer_cache_checks_frame_during_real_primary_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport

    use tcl_cmd_core::return_options::ReturnOptionsOps;
    for mode in [0, 3] {
        let mut interpreter = fixture::native("tcl9.0");
        let _current = fixture::install(&mut interpreter, mode);
        interpreter
            .frames
            .borrow_mut()
            .push(crate::namespace::GLOBAL);
        let original = BodyObjectOperation::capture(&interpreter).unwrap();
        let value = fixture::original(b"0");
        let (ops, _) = crate::return_options::NativeReturnOps::selected(&interpreter).unwrap();
        let mut ops = ops.with_original_objects(original.objects(&interpreter).unwrap());
        let reached = ops.integer_probe(&value, true);
        if mode == 0 {
            assert_eq!(reached.unwrap(), Some(0));
            assert!(obj::native_scalar_cache(value.as_ptr()).unwrap().is_some());
        } else {
            let first = reached.unwrap_err();
            assert_eq!(
                first.native_execution_refusal(),
                interpreter.native_execution_refusal().as_ref()
            );
            assert!(obj::native_scalar_cache(value.as_ptr()).unwrap().is_none());
        }
        assert_eq!(fixture::FREES.with(Cell::get), 1);
        assert_eq!(fixture::REFS.with(Cell::get), 1);
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    }
}
