// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual whole-record payload retirement and mandatory cleanup controls.
use super::*;
use crate::interp::WeakInterp;
use std::cell::Cell;

type OriginalRecordStage = (u64, bool, bool, bool, bool);

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static TARGET: Cell<OoId> = const { Cell::new(OoId(0)) };
    static NAMESPACE: Cell<NsId> = const { Cell::new(GLOBAL) };
    static STAGES: RefCell<Vec<OriginalRecordStage>> = const { RefCell::new(Vec::new()) };
}
extern "C" fn retire(original: *mut TclObj) {
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    assert!(interpreter.oo.try_borrow_mut().is_ok());
    drop(interpreter.namespaces_mut());
    let target = TARGET.with(Cell::get);
    let (class, object, torn_down) = {
        let state = interpreter.oo.borrow();
        (
            state.classes.contains_key(&target),
            state.objects.contains_key(&target),
            state
                .objects
                .get(&target)
                .is_some_and(|object| object.torn_down),
        )
    };
    let namespace = interpreter
        .namespaces()
        .namespace_is_live(NAMESPACE.with(Cell::get));
    let first = STAGES.with(|stages| {
        let mut stages = stages.borrow_mut();
        stages.push((
            obj::internal_rep(original),
            class,
            object,
            torn_down,
            namespace,
        ));
        stages.len() == 1
    });
    if first {
        match MODE.with(Cell::get) {
            1 => {
                interpreter.refuse_host_command("first whole OO payload retirement");
            }
            2 => {
                let current = interpreter.runtime_context();
                let mut changed = current.clone();
                changed
                    .packages
                    .push(("whole-oo-retirement".into(), "1.0".into()));
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&current).unwrap();
            }
            3 => {
                interpreter.oo_destroy(target);
            }
            _ => {}
        }
    }
}
static PAYLOAD: obj::TclObjType = obj::TclObjType {
    name: c"originalOoRecordPayload".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn payload(role: u64, name: &[u8]) -> obj::Owned {
    let header = obj::Owned::fresh(obj::alloc_typed(&PAYLOAD, role));
    unsafe { obj::set_string_rep(header.as_ptr(), name) };
    header
}
fn native() -> Interp {
    let interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl9.1"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|mode| mode.set(0));
    STAGES.with(|stages| stages.borrow_mut().clear());
    interpreter
}
fn select(interpreter: &Interp, target: OoId) {
    TARGET.with(|slot| slot.set(target));
    NAMESPACE.with(|slot| slot.set(interpreter.oo.borrow().objects[&target].var_ns));
}
fn dispatch_original(interpreter: &mut Interp, words: &[&[u8]], last: *mut TclObj) {
    let words: Vec<_> = words
        .iter()
        .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
        .collect();
    let mut argv: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
    argv.push(last);
    assert_eq!(interpreter.dispatch(&argv), Code::Ok);
}
fn install(interpreter: &mut Interp, class: bool) -> Vec<obj::NativeObjectLifetime> {
    let definition = if class {
        b"oo::define".as_slice()
    } else {
        b"oo::objdefine".as_slice()
    };
    let target = if class {
        b"RetireClass".as_slice()
    } else {
        b"RetireObject".as_slice()
    };
    let variable = payload(0, b"privatePayload");
    dispatch_original(
        interpreter,
        &[definition, target, b"private", b"variable"],
        variable.as_ptr(),
    );
    let forward = payload(1, b"unusedRetirementTarget");
    dispatch_original(
        interpreter,
        &[definition, target, b"forward", b"forwarded"],
        forward.as_ptr(),
    );
    let method = payload(2, b"namedPayload");
    let words: Vec<_> = [definition, target, b"method".as_slice()]
        .iter()
        .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
        .collect();
    let formals = obj::Owned::fresh(obj::new_string_bytes(b""));
    let body = obj::Owned::fresh(obj::new_string_bytes(b"return"));
    let argv = [
        words[0].as_ptr(),
        words[1].as_ptr(),
        words[2].as_ptr(),
        method.as_ptr(),
        formals.as_ptr(),
        body.as_ptr(),
    ];
    assert_eq!(interpreter.dispatch(&argv), Code::Ok);
    assert_eq!(unsafe { (*variable.as_ptr()).ref_count }, 2);
    assert_eq!(unsafe { (*forward.as_ptr()).ref_count }, 2);
    assert_eq!(unsafe { (*method.as_ptr()).ref_count }, 3);
    let leases = Vec::from(
        [&variable, &forward, &method]
            .map(|header| obj::NativeObjectLifetime::retain(header.as_ptr())),
    );
    drop(variable);
    drop(forward);
    drop(method);
    assert!(STAGES.with(|stages| stages.borrow().is_empty()));
    leases
}
fn cause(interpreter: &Interp, mode: u8) {
    match mode {
        1 => assert!(
            matches!(interpreter.native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)) if first.reason == "first whole OO payload retirement")
        ),
        2 => assert!(interpreter.native_execution_refusal().is_some()),
        _ => assert!(interpreter.native_execution_refusal().is_none()),
    }
}
#[test]
fn whole_object_and_class_payloads_retire_outside_borrows_after_actual_lifecycle_withdrawal() {
    // Source lifecycle and software callback transport, not Native aggregate order.
    for class in [false, true] {
        for mode in 0..4 {
            let mut interpreter = native();
            assert_eq!(
                interpreter
                    .eval_str(b"oo::class create RetireClass; RetireClass create RetireObject"),
                Code::Ok
            );
            let target = interpreter.oo_resolve_object(if class {
                b"RetireClass"
            } else {
                b"RetireObject"
            });
            select(&interpreter, target);
            let leases = install(&mut interpreter, class);
            MODE.with(|slot| slot.set(mode));
            if class {
                interpreter.oo_destroy_class(target);
            } else {
                assert_eq!(
                    interpreter.oo_destroy(target),
                    if mode == 1 || mode == 2 {
                        Code::Error
                    } else {
                        Code::Ok
                    }
                );
            }
            let stages = STAGES.with(|stages| stages.borrow().clone());
            let mut roles: Vec<_> = stages.iter().map(|stage| stage.0).collect();
            roles.sort();
            assert_eq!(roles, [0, 1, 2]);
            for (_, class_live, object_live, torn_down, _) in stages {
                assert!(!class_live);
                assert!(!object_live || torn_down);
            }
            assert!(!interpreter.oo.borrow().objects.contains_key(&target));
            assert!(!interpreter.oo.borrow().classes.contains_key(&target));
            assert!(!interpreter
                .namespaces()
                .namespace_is_live(NAMESPACE.with(Cell::get)));
            for lease in &leases {
                assert!(!obj::allocation_is_live(lease.as_ptr()));
            }
            cause(&interpreter, mode);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            OWNER.with(|owner| *owner.borrow_mut() = None);
        }
    }
}
#[test]
fn cached_original_name_retirement_stops_later_rename_publication_on_first_refusal() {
    for mode in 0..3 {
        let mut interpreter = native();
        assert_eq!(
            interpreter.eval_str(b"oo::class create RetireClass; RetireClass create RetireObject"),
            Code::Ok
        );
        let target = interpreter.oo_resolve_object(b"RetireObject");
        select(&interpreter, target);
        let original = interpreter.oo_original_name(target);
        obj::change_type(original, &PAYLOAD, 9);
        let lease = obj::NativeObjectLifetime::retain(original);
        let name = interpreter.oo_name(target);
        interpreter.set_result_bytes(b"PRIOR");
        let prior = obj::Owned::retain(interpreter.get_obj_result());
        MODE.with(|slot| slot.set(mode));
        interpreter.oo_command_renamed(target, Some(b"::RenamedObject"));
        assert_eq!(STAGES.with(|stages| stages.borrow().len()), 1);
        assert_eq!(
            interpreter.oo_name(target),
            if mode == 0 {
                b"::RenamedObject".to_vec()
            } else {
                name
            }
        );
        assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
        assert!(!obj::allocation_is_live(original));
        cause(&interpreter, mode);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        drop(lease);
        OWNER.with(|owner| *owner.borrow_mut() = None);
    }
}

fn install_partial(interpreter: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    let (target, class) = match interpreter.active_def_target() {
        Some(DefTarget::Class(target)) => (target, true),
        Some(DefTarget::Object(target)) => (target, false),
        None => (
            interpreter.oo.borrow().call_stack.last().unwrap().object,
            false,
        ),
    };
    select(interpreter, target);
    let name = interpreter.oo_name(target);
    let forward = payload(7, b"unusedPartialTarget");
    let definition = if class {
        b"oo::define".as_slice()
    } else {
        b"oo::objdefine".as_slice()
    };
    dispatch_original(
        interpreter,
        &[definition, &name, b"forward", b"partial"],
        forward.as_ptr(),
    );
    drop(forward);
    Code::Ok
}
#[test]
fn partial_definition_and_constructor_failures_finish_original_record_and_namespace_cleanup() {
    for constructor in [false, true] {
        for mode in 0..3 {
            let mut interpreter = native();
            interpreter.ns_register(
                b"::InstallPartialPayload",
                Command::Builtin(install_partial),
            );
            if constructor {
                assert_eq!(interpreter.eval_str(b"oo::class create PartialClass {constructor {} {InstallPartialPayload; error CONSTRUCTOR_FAILURE}}"), Code::Ok);
            }
            MODE.with(|slot| slot.set(mode));
            let code = interpreter.eval_str(if constructor {
                b"PartialClass create PartialObject".as_slice()
            } else {
                b"oo::class create PartialClass {InstallPartialPayload; error DEFINITION_FAILURE}"
                    .as_slice()
            });
            assert_eq!(code, Code::Error);
            let target = TARGET.with(Cell::get);
            assert_eq!(
                STAGES.with(|stages| stages
                    .borrow()
                    .iter()
                    .map(|stage| stage.0)
                    .collect::<Vec<_>>()),
                [7]
            );
            assert!(!interpreter.oo.borrow().objects.contains_key(&target));
            assert!(!interpreter.oo.borrow().classes.contains_key(&target));
            assert!(!interpreter
                .namespaces()
                .namespace_is_live(NAMESPACE.with(Cell::get)));
            assert!(interpreter
                .namespaces()
                .command_in(
                    crate::namespace::GLOBAL,
                    if constructor {
                        b"PartialObject".as_slice()
                    } else {
                        b"PartialClass".as_slice()
                    }
                )
                .is_none());
            if mode == 0 {
                assert!(interpreter
                    .result_bytes()
                    .windows(7)
                    .any(|word| word == b"FAILURE"));
            }
            cause(&interpreter, mode);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            OWNER.with(|owner| *owner.borrow_mut() = None);
        }
    }
}
thread_local! { static ACTIVE_HEADER: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) }; }
fn destroy_active_class(interpreter: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interpreter.oo_destroy_class(TARGET.with(Cell::get));
    assert!(STAGES.with(|stages| stages.borrow().is_empty()));
    assert!(obj::allocation_is_live(ACTIVE_HEADER.with(Cell::get)));
    interpreter.set_result_bytes(b"ACTIVE");
    Code::Ok
}
#[test]
fn active_forward_record_retains_its_existing_payload_until_real_chain_cleanup() {
    // The Rust forward owner is distinct from Native mutable MethodType clientData.
    let mut interpreter = native();
    interpreter.ns_register(
        b"::DestroyActiveClass",
        Command::Builtin(destroy_active_class),
    );
    assert_eq!(
        interpreter.eval_str(b"oo::class create RetireClass; RetireClass create RetireObject"),
        Code::Ok
    );
    let class = interpreter.oo_resolve_object(b"RetireClass");
    select(&interpreter, class);
    let forward = payload(8, b"unreadActivePayload");
    let lease = obj::NativeObjectLifetime::retain(forward.as_ptr());
    ACTIVE_HEADER.with(|slot| slot.set(forward.as_ptr()));
    dispatch_original(
        &mut interpreter,
        &[
            b"oo::define",
            b"RetireClass",
            b"forward",
            b"active",
            b"::DestroyActiveClass",
        ],
        forward.as_ptr(),
    );
    drop(forward);
    let _code = interpreter.eval_str(b"RetireObject active");
    assert_eq!(
        STAGES.with(|stages| stages
            .borrow()
            .iter()
            .map(|stage| stage.0)
            .collect::<Vec<_>>()),
        [8]
    );
    assert!(!obj::allocation_is_live(lease.as_ptr()));
    assert!(interpreter.native_execution_refusal().is_none());
    assert_eq!(interpreter.entered_native_operation_count(), 0);
    ACTIVE_HEADER.with(|slot| slot.set(core::ptr::null_mut()));
    OWNER.with(|owner| *owner.borrow_mut() = None);
}
