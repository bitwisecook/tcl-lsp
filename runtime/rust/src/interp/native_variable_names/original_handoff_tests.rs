// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original selection survives the real name-primary retirement boundary.
use super::*;
use crate::{
    interp::{native_operation_currency::NativeOperationScope, WeakInterp},
    namespace::GLOBAL,
};
use std::cell::{Cell, RefCell};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static REFERENCES: Cell<obj::TclSize> = const { Cell::new(-1) };
    static REACHED: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static PART_FREES: Cell<usize> = const { Cell::new(0) };
    static PART_REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static PART_FIRST: Cell<bool> = const { Cell::new(false) };
}
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        OWNER.with(|owner| owner.borrow_mut().take());
    }
}
fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn text(bytes: &[u8]) -> Owned {
    Owned::fresh(obj::new_string_bytes(bytes))
}
fn arm(interpreter: &Interp, mode: u8) -> Reset {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|selected| selected.set(mode));
    CALLS.with(|calls| calls.set(0));
    REFERENCES.with(|refs| refs.set(-1));
    REACHED.with(|result| result.set(std::ptr::null_mut()));
    Reset
}
extern "C" fn retired(original: *mut TclObj) {
    let previous = CALLS.with(|calls| {
        let previous = calls.get();
        calls.set(previous + 1);
        previous
    });
    if previous != 0 {
        return;
    }
    REFERENCES.with(|refs| refs.set(unsafe { (*original).ref_count }));
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    assert!(interpreter.original_variable_storage_borrows_are_available());
    let mode = MODE.with(Cell::get);
    if mode == 6 {
        obj::change_type(original, &obj::TCL_INT_TYPE, 777);
        return;
    }
    if mode == 3 {
        assert!(interpreter.var_unset(b"handoff"));
    }
    if mode <= 3 {
        let reached = text(b"REACHED VALUE");
        interpreter.var_set(b"handoff", reached.as_ptr()).unwrap();
    } else if mode == 4 {
        let target = text(b"other");
        let local = text(b"handoff");
        assert_eq!(
            interpreter.link_original_c_upvar_objects(target.as_ptr(), 0, local.as_ptr()),
            Code::Ok
        );
    } else if mode == 5 {
        interpreter.frames.borrow_mut().pop().unwrap();
        interpreter.frames.borrow_mut().push(GLOBAL);
        let protocol = interpreter
            .native_invocation_dialect()
            .native_compiled_variable_protocol()
            .unwrap();
        interpreter
            .frames
            .borrow_mut()
            .install_formal_cells(&[b"handoff".to_vec()], protocol);
    }
    interpreter.set_result_bytes(b"REACHED RESULT");
    REACHED.with(|result| result.set(interpreter.get_obj_result()));
    if mode == 1 {
        interpreter.refuse_host_command("first original variable cache retirement");
    } else if mode == 2 {
        let original = interpreter.runtime_context();
        let mut changed = original.clone();
        changed
            .packages
            .push(("original-name-handoff".into(), "1.0".into()));
        interpreter.pin_context(&changed).unwrap();
        interpreter.pin_context(&original).unwrap();
    }
}
static OLD: obj::TclObjType = obj::TclObjType {
    name: c"originalVariableNameHandoff".as_ptr(),
    free_int_rep_proc: Some(retired),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn original() -> Owned {
    let name = Owned::fresh(obj::alloc_typed(&OLD, 0));
    unsafe { obj::set_string_rep(name.as_ptr(), b"handoff") };
    name
}
fn storage(interpreter: &Interp, name: &[u8]) -> Option<*mut TclObj> {
    crate::vars::get(
        &interpreter.frames.borrow(),
        &interpreter.namespaces(),
        interpreter.current_ns(),
        name,
    )
}
fn first(interpreter: &Interp, mode: u8) {
    let cause = interpreter
        .native_execution_refusal()
        .expect("actual retained first cause");
    if mode == 1 {
        assert!(
            matches!(cause, NativeExecutionError::HostCommandRefusal(first) if first.reason == "first original variable cache retirement")
        );
    } else if mode == 2 {
        assert!(matches!(
            cause,
            NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation"
                )
            )
        ));
    } else {
        assert!(matches!(
            cause,
            NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(_)
            )
        ));
    }
}
#[test]
fn original_name_cache_retirement_keeps_reached_store_result_and_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..3 {
            let mut interpreter = native(profile);
            let value = text(b"OLD");
            interpreter.var_set(b"handoff", value.as_ptr()).unwrap();
            let name = original();
            let _reset = arm(&interpreter, mode);
            let reached = interpreter
                .prepare_original_c_name_for(name.as_ptr(), NativeVariableNameLookupPurpose::Read);
            assert_eq!(reached.is_ok(), mode == 0, "{profile}/{mode}");
            if mode == 0 {
                assert!(reached.unwrap().is_some());
                assert!(interpreter.native_execution_refusal().is_none());
            } else {
                first(&interpreter, mode);
                assert!(obj::native_variable_name::with_parsed(name.as_ptr(), |_| ()).is_none());
            }
            assert_eq!(CALLS.with(Cell::get), 1);
            assert_eq!(REFERENCES.with(Cell::get), 1);
            assert_eq!(
                unsafe { (*name.as_ptr()).ref_count },
                1,
                "allocation observations add no Native reference"
            );
            assert_eq!(
                obj::bytes_of(storage(&interpreter, b"handoff").unwrap()),
                b"REACHED VALUE"
            );
            assert_eq!(interpreter.get_obj_result(), REACHED.with(Cell::get));
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }
}
#[test]
fn original_c9_selected_root_rebirth_refuses_while_c8_selects_after_its_old_free() {
    // naming.embedding.original-host-publication-and-fact-transport

    // C8 lookup follows this retirement; C9 already selected its original cell.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = native(profile);
        let value = text(b"OLD");
        interpreter.var_set(b"handoff", value.as_ptr()).unwrap();
        let name = original();
        let _reset = arm(&interpreter, 3);
        let reached = interpreter
            .prepare_original_c_name_for(name.as_ptr(), NativeVariableNameLookupPurpose::Read);
        let prelookup = interpreter
            .native_c_variable_name_protocol()
            .unwrap()
            .retires_before_simple_lookup(true);
        assert_eq!(reached.is_ok(), prelookup, "{profile}");
        if prelookup {
            assert!(reached.unwrap().is_some());
            assert!(interpreter.native_execution_refusal().is_none());
        } else {
            first(&interpreter, 3);
            assert!(obj::native_variable_name::with_parsed(name.as_ptr(), |_| ()).is_none());
        }
        assert_eq!(CALLS.with(Cell::get), 1);
        assert_eq!(REFERENCES.with(Cell::get), 1);
        assert_eq!(
            obj::bytes_of(storage(&interpreter, b"handoff").unwrap()),
            b"REACHED VALUE"
        );
        assert_eq!(interpreter.get_obj_result(), REACHED.with(Cell::get));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
#[test]
fn original_c9_alias_handoff_refuses_retarget_before_cache_and_receiver_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl9.0", "tcl9.1"] {
        let mut interpreter = native(profile);
        let old = text(b"OLD TARGET");
        let other = text(b"OTHER TARGET");
        interpreter.var_set(b"target", old.as_ptr()).unwrap();
        interpreter.var_set(b"other", other.as_ptr()).unwrap();
        let target = text(b"target");
        let local = text(b"handoff");
        assert_eq!(
            interpreter.link_original_c_upvar_objects(target.as_ptr(), 0, local.as_ptr()),
            Code::Ok
        );
        let name = original();
        let _reset = arm(&interpreter, 4);
        assert!(interpreter
            .prepare_original_c_name_for(name.as_ptr(), NativeVariableNameLookupPurpose::Read)
            .is_err());
        first(&interpreter, 4);
        assert_eq!(CALLS.with(Cell::get), 1);
        assert!(obj::native_variable_name::with_parsed(name.as_ptr(), |_| ()).is_none());
        assert_eq!(storage(&interpreter, b"handoff"), Some(other.as_ptr()));
        assert_eq!(storage(&interpreter, b"target"), Some(old.as_ptr()));
        assert_eq!(interpreter.get_obj_result(), REACHED.with(Cell::get));
        assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
#[test]
fn original_indexed_selection_retirement_refuses_reborn_frame_at_the_same_slot() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual compiled ordinal observation; this supplies no fabricated Local cache or Proc issuer.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 5] {
            let interpreter = native(profile);
            let protocol = interpreter
                .native_invocation_dialect()
                .native_compiled_variable_protocol()
                .unwrap();
            interpreter.frames.borrow_mut().push(GLOBAL);
            interpreter
                .frames
                .borrow_mut()
                .install_formal_cells(&[b"handoff".to_vec()], protocol);
            let selected = crate::vars::prepare_original_c_indexed_name_cell(
                &mut interpreter.frames.borrow_mut(),
                &interpreter.namespaces(),
                0,
            )
            .unwrap();
            let scope = NativeOperationScope::enter(&interpreter).unwrap();
            let name = original();
            let _reset = arm(&interpreter, mode);
            let mut current = || selected.check(&interpreter, scope.currency());
            let reached = obj::discard_native_internal_representation_with_stage_check(
                name.as_ptr(),
                &mut current,
            );
            assert_eq!(reached.is_ok(), mode == 0, "{profile}/{mode}");
            if mode == 0 {
                assert!(interpreter.native_execution_refusal().is_none());
            } else {
                first(&interpreter, mode);
            }
            assert_eq!(CALLS.with(Cell::get), 1);
            assert_eq!(REFERENCES.with(Cell::get), 1);
            assert_eq!(interpreter.get_obj_result(), REACHED.with(Cell::get));
            assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 1);
            assert!(interpreter.frames.borrow().compiled_slot_name(0).is_some());
            drop(scope);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }
}

extern "C" fn retired_part(original: *mut TclObj) {
    PART_FREES.with(|calls| calls.set(calls.get() + 1));
    PART_REFS.with(|refs| refs.set(unsafe { (*original).ref_count }));
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    assert!(interpreter.original_variable_storage_borrows_are_available());
    PART_FIRST.with(|seen| {
        seen.set(matches!(
            interpreter.native_execution_refusal(),
            Some(NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "changed native primary publication slot"
                )
            ))
        ))
    });
    interpreter.refuse_host_command("later unused parsed-name part retirement");
}
static PART: obj::TclObjType = obj::TclObjType {
    name: c"originalUnusedParsedNamePart".as_ptr(),
    free_int_rep_proc: Some(retired_part),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
#[test]
fn original_declined_cache_retirement_keeps_intrinsic_refusal_before_unused_part_free() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Genuine prepared part +1 in this private cache seam; no external Native recipe claim.
    let mut interpreter = native("tcl9.1");
    let name = original();
    let _reset = arm(&interpreter, 6);
    let root = Owned::fresh(obj::alloc_typed(&PART, 0));
    let lifetime = obj::NativeObjectLifetime::retain(root.as_ptr());
    let element = text(b"k");
    let protocol = interpreter.native_c_variable_name_protocol().unwrap();
    let dialect = interpreter.native_invocation_dialect();
    let scope = NativeOperationScope::enter(&interpreter).unwrap();
    interpreter.set_result_bytes(b"PRIOR");
    let prior = interpreter.get_obj_result();
    PART_FREES.with(|calls| calls.set(0));
    PART_REFS.with(|refs| refs.set(-1));
    PART_FIRST.with(|seen| seen.set(false));
    let mut current = || {
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(tcl_cmd_core::CmdError::from_execution_refusal)
    };
    let retain = |error: &tcl_cmd_core::CmdError| {
        if let Some(first) = error.native_execution_refusal() {
            interpreter.clone().refuse_native_execution(first.clone());
        }
    };
    let failure = obj::native_variable_name::install_parsed_with_stage_check(
        name.as_ptr(),
        NativeParsedVariableName {
            protocol,
            array: Some((root, NativeParsedVariableElement::Object(element))),
        },
        dialect,
        &mut current,
        &retain,
    )
    .unwrap_err();
    let first = interpreter.native_execution_refusal().unwrap();
    assert!(matches!(
        first,
        NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable(
                "changed native primary publication slot"
            )
        )
    ));
    assert_eq!(failure.native_execution_refusal(), Some(&first));
    assert!(PART_FIRST.with(Cell::get));
    assert_eq!(PART_FREES.with(Cell::get), 1);
    assert_eq!(PART_REFS.with(Cell::get), 0);
    assert!(!obj::allocation_is_live(lifetime.as_ptr()));
    assert_eq!(CALLS.with(Cell::get), 2);
    assert_eq!(REFERENCES.with(Cell::get), 1);
    assert!(std::ptr::eq(
        obj::obj_type_ptr(name.as_ptr()),
        &obj::TCL_INT_TYPE
    ));
    assert_eq!(obj::internal_rep(name.as_ptr()), 777);
    assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 1);
    assert_eq!(interpreter.get_obj_result(), prior);
    drop(scope);
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}
