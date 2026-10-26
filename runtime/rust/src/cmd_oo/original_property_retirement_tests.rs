// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual property member/cache callbacks and selected source release stages.

use super::*;
use crate::interp::{native_operation_currency::NativeOperationScope, WeakInterp};
use std::cell::Cell;

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static CALLS: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static CACHES: Cell<[*mut TclObj; 2]> = const { Cell::new([core::ptr::null_mut(); 2]) };
    static CACHE_LIVE: RefCell<Vec<[bool; 2]>> = const { RefCell::new(Vec::new()) };
    static TARGET: Cell<OoId> = const { Cell::new(OoId(0)) };
}

extern "C" fn retire(original: *mut TclObj) {
    let role = obj::internal_rep(original);
    let first = CALLS.with(|calls| {
        let mut calls = calls.borrow_mut();
        calls.push(role);
        calls.len() == 1
    });
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    assert!(interpreter.oo.try_borrow_mut().is_ok());
    CACHE_LIVE.with(|seen| {
        seen.borrow_mut().push(
            CACHES
                .with(Cell::get)
                .map(|header| !header.is_null() && obj::allocation_is_live(header)),
        );
    });
    if first {
        match MODE.with(Cell::get) {
            1 => {
                interpreter.refuse_host_command("first original property retirement");
            }
            2 => {
                let original = interpreter.runtime_context();
                let mut changed = original.clone();
                changed
                    .packages
                    .push(("property-retirement".into(), "1.0".into()));
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&original).unwrap();
            }
            3 => {
                interpreter.oo_destroy(TARGET.with(Cell::get));
            }
            _ => {}
        }
    }
}

static MEMBER: obj::TclObjType = obj::TclObjType {
    name: c"originalPropertyRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

fn native() -> Interp {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl9.1"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
        interpreter
            .eval_str(b"oo::class create RetirementClass; RetirementClass create RetirementObject"),
        Code::Ok
    );
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|mode| mode.set(0));
    CALLS.with(|calls| calls.borrow_mut().clear());
    CACHE_LIVE.with(|seen| seen.borrow_mut().clear());
    CACHES.with(|caches| caches.set([core::ptr::null_mut(); 2]));
    interpreter
}

fn install_members(interpreter: &mut Interp, target: OoId, class: bool) {
    for (writable, name) in [(false, b"-r".as_slice()), (true, b"-w".as_slice())] {
        let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, u64::from(writable)));
        // SAFETY: the genuine extension supplies its resident counted String.
        unsafe { obj::set_string_rep(member.as_ptr(), name) };
        let input = obj::Owned::fresh(interpreter.new_list_object(&[member.as_ptr()]));
        native_properties::set_property_slot(interpreter, target, class, writable, input.as_ptr())
            .unwrap();
        drop(input);
        assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 2);
        drop(member);
    }
}

fn cache_headers(interpreter: &mut Interp) -> [obj::NativeObjectLifetime; 2] {
    let mut headers = [core::ptr::null_mut(); 2];
    let leases = [false, true].map(|writable| {
        let script = if writable {
            b"info object properties RetirementObject -all -writable".as_slice()
        } else {
            b"info object properties RetirementObject -all -readable".as_slice()
        };
        assert_eq!(interpreter.eval_str(script), Code::Ok);
        let header = interpreter.get_obj_result();
        assert!(crate::list::native_list_backing(header).is_some());
        headers[usize::from(writable)] = header;
        let lease = obj::NativeObjectLifetime::retain(header);
        assert_eq!(
            unsafe { (*header).ref_count },
            2,
            "actual cache plus public result"
        );
        interpreter.reset_result();
        assert_eq!(
            unsafe { (*header).ref_count },
            1,
            "actual cache only; allocation lease adds no native role"
        );
        lease
    });
    CACHES.with(|caches| caches.set(headers));
    leases
}

fn assert_first(interpreter: &Interp, mode: u8) {
    match mode {
        1 => {
            let Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)) =
                interpreter.native_execution_refusal()
            else {
                panic!("actual original property Host cause missing");
            };
            assert_eq!(first.reason, "first original property retirement");
        }
        2 => assert_eq!(
            interpreter.native_execution_refusal(),
            Some(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation"
                )
                .into()
            )
        ),
        _ => assert!(interpreter.native_execution_refusal().is_none()),
    }
}

#[test]
fn property_registration_refusal_stops_later_direction_and_epoch_publication() {

    // Genuine reached owner callbacks; registration chronology is not a Native probe result.
    for mode in 0..4 {
        let mut interpreter = native();
        let target = interpreter.oo_resolve_object(b"RetirementObject");
        TARGET.with(|slot| slot.set(target));
        install_members(&mut interpreter, target, false);
        let _leases = cache_headers(&mut interpreter);
        interpreter.set_result_bytes(b"PRIOR");
        let prior = obj::Owned::retain(interpreter.get_obj_result());
        let epoch = interpreter.oo.borrow().property_foundation_epoch;
        MODE.with(|slot| slot.set(mode));
        let outcome = native_properties::register_property(
            &mut interpreter,
            target,
            false,
            b"-r",
            false,
            false,
        );
        assert_eq!(outcome.is_ok(), mode == 0);
        assert_eq!(
            CALLS.with(|calls| calls.borrow().as_slice().to_vec()),
            if mode == 3 { vec![0, 1] } else { vec![0] }
        );
        if mode == 1 || mode == 2 {
            let oo = interpreter.oo.borrow();
            assert!(!oo.property_members[&(target, false, false)].contains_key(b"-r".as_slice()));
            assert!(oo.property_members[&(target, false, true)].contains_key(b"-w".as_slice()));
            assert_eq!(
                oo.objects[&target].readable_properties,
                vec![b"-r".to_vec()]
            );
            assert_eq!(
                oo.objects[&target].writable_properties,
                vec![b"-w".to_vec()]
            );
            assert_eq!(oo.property_foundation_epoch, epoch);
            drop(oo);
            assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
            assert_first(&interpreter, mode);
        } else if mode == 3 {
            assert!(!interpreter.oo.borrow().objects.contains_key(&target));
            let error = outcome.err().unwrap();
            assert_eq!(
                error.error_code_bytes(),
                Some(b"TCL OO MONKEY_BUSINESS".as_slice())
            );
            assert_first(&interpreter, 0);
        } else {
            assert!(interpreter.oo.borrow().objects[&target]
                .readable_properties
                .is_empty());
            assert_first(&interpreter, 0);
        }
        MODE.with(|slot| slot.set(0));
        let _ = interpreter.retire_native_properties(target);
        assert_eq!(
            CALLS.with(|calls| calls.borrow().as_slice().to_vec()),
            vec![0, 1]
        );
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        OWNER.with(|owner| *owner.borrow_mut() = None);
    }
}

#[test]
fn mandatory_property_retirement_releases_both_caches_before_original_members() {

    // Release order is inspected C91 source, not a measured teardown callback tape.
    for mode in 0..3 {
        let mut interpreter = native();
        let target = interpreter.oo_resolve_object(b"RetirementObject");
        install_members(&mut interpreter, target, false);
        let _leases = cache_headers(&mut interpreter);
        interpreter.set_result_bytes(b"PRIOR");
        let prior = obj::Owned::retain(interpreter.get_obj_result());
        MODE.with(|slot| slot.set(mode));
        // Direct mandatory cleanup retains its own original operation before callbacks.
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        let outcome = interpreter.retire_native_properties(target);
        assert_eq!(outcome.is_ok(), mode == 0);
        assert_eq!(
            CALLS.with(|calls| calls.borrow().as_slice().to_vec()),
            vec![0, 1]
        );
        assert_eq!(
            CACHE_LIVE.with(|seen| seen.borrow().clone()),
            vec![[false, false], [false, false]]
        );
        assert!(!interpreter
            .oo
            .borrow()
            .property_members
            .keys()
            .any(|(id, _, _)| *id == target));
        assert!(!interpreter
            .oo
            .borrow()
            .property_caches
            .keys()
            .any(|(id, _)| *id == target));
        assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
        assert_first(&interpreter, mode);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        OWNER.with(|owner| *owner.borrow_mut() = None);
    }
}

#[test]
fn property_invalidation_retires_stale_lists_outside_borrows_and_transports_first_cause() {

    // Actual derived cache survives a Foundation change; no speculative cache owner is planted.
    for mode in 0..3 {
        let mut interpreter = native();
        let class = interpreter.oo_resolve_object(b"RetirementClass");
        let target = interpreter.oo_resolve_object(b"RetirementObject");
        install_members(&mut interpreter, class, true);
        let _leases = cache_headers(&mut interpreter);
        let empty = obj::Owned::fresh(interpreter.new_list_object(&[]));
        for writable in [false, true] {
            native_properties::set_property_slot(
                &mut interpreter,
                class,
                true,
                writable,
                empty.as_ptr(),
            )
            .unwrap();
        }
        assert!(CALLS.with(|calls| calls.borrow().is_empty()));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = obj::Owned::retain(interpreter.get_obj_result());
        MODE.with(|slot| slot.set(mode));
        let outcome = interpreter.native_property_structure_changed(target, false);
        assert_eq!(outcome.is_ok(), mode == 0);
        assert_eq!(
            CALLS.with(|calls| calls.borrow().as_slice().to_vec()),
            vec![0, 1]
        );
        assert_eq!(
            CACHE_LIVE.with(|seen| seen.borrow().clone()),
            vec![[false, true], [false, false]]
        );
        assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
        assert_first(&interpreter, mode);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        OWNER.with(|owner| *owner.borrow_mut() = None);
    }
}

#[test]
fn property_retirement_keeps_its_originating_release_recipe_after_profile_change() {
    // Release order comes from the actual C91 storage producer, not the current profile.
    let mut interpreter = native();
    let target = interpreter.oo_resolve_object(b"RetirementObject");
    install_members(&mut interpreter, target, false);
    let _leases = cache_headers(&mut interpreter);
    let changed = tcl_registry::model::runtime_context_for_profile(
        crate::environment::profile_for_dialect("tcl8.6"),
    );
    interpreter.pin_context(&changed).unwrap();
    assert!(interpreter
        .native_invocation_dialect()
        .native_property_lookup_protocol()
        .is_none());
    interpreter.retire_native_properties(target).unwrap();
    assert_eq!(CALLS.with(|calls| calls.borrow().clone()), vec![0, 1]);
    assert_eq!(
        CACHE_LIVE.with(|seen| seen.borrow().clone()),
        vec![[false, false], [false, false]]
    );
    assert_eq!(interpreter.entered_native_operation_count(), 0);
    OWNER.with(|owner| *owner.borrow_mut() = None);
}

#[test]
fn exhausted_property_cleanup_retains_entry_refusal_before_all_payload_callbacks() {
    // Actual epoch exhaustion is a software owner control, not a Tcl chronology claim.
    let mut interpreter = native();
    let target = interpreter.oo_resolve_object(b"RetirementObject");
    install_members(&mut interpreter, target, false);
    let _leases = cache_headers(&mut interpreter);
    interpreter.set_result_bytes(b"PRIOR");
    let prior = obj::Owned::retain(interpreter.get_obj_result());
    interpreter.exhaust_native_operation_context_for_test();
    assert_eq!(interpreter.native_operation_context_epoch(), None);
    assert!(interpreter.native_execution_refusal().is_none());
    let expected = match NativeOperationScope::enter(&interpreter) {
        Ok(_) => panic!("exhausted original operation must refuse"),
        Err(first) => first,
    };
    assert!(interpreter.native_execution_refusal().is_none());
    MODE.with(|mode| mode.set(1));
    let outcome = interpreter.retire_native_properties(target);
    assert!(outcome.is_err());
    assert_eq!(interpreter.native_execution_refusal(), Some(expected));
    assert_eq!(CALLS.with(|calls| calls.borrow().clone()), vec![0, 1]);
    assert_eq!(
        CACHE_LIVE.with(|seen| seen.borrow().clone()),
        vec![[false, false], [false, false]]
    );
    assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
    assert!(!interpreter
        .oo
        .borrow()
        .property_members
        .keys()
        .any(|(id, _, _)| *id == target));
    assert!(!interpreter
        .oo
        .borrow()
        .property_caches
        .keys()
        .any(|(id, _)| *id == target));
    assert_eq!(interpreter.entered_native_operation_count(), 0);
    OWNER.with(|owner| *owner.borrow_mut() = None);
}
