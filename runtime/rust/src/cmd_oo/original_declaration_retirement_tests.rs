// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual declaration replacement, reached payload retirement and continuation.
use super::*;
use crate::interp::WeakInterp;
use std::cell::Cell;

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static CLASS: Cell<bool> = const { Cell::new(false) };
    static PRIVATE: Cell<bool> = const { Cell::new(false) };
    static TARGET: Cell<OoId> = const { Cell::new(OoId(0)) };
    static SLOT: Cell<OoId> = const { Cell::new(OoId(0)) };
    static INCOMING: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static FREES: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static REPORT_COUNTS: RefCell<Vec<(isize, isize, isize)>> = const { RefCell::new(Vec::new()) };
    static REACHED: RefCell<Option<obj::Owned>> = const { RefCell::new(None) };
}
extern "C" fn retire(original: *mut TclObj) {
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    FREES.with(|frees| frees.borrow_mut().push(obj::internal_rep(original)));
    assert!(interpreter.oo.try_borrow_mut().is_ok());
    let incoming = INCOMING.with(Cell::get);
    let before = unsafe { (*incoming).ref_count };
    assert!(before >= 2);
    assert_eq!(
        slot_inst_get(&mut interpreter, SLOT.with(Cell::get), &[]),
        Code::Ok
    );
    let returned = interpreter
        .list_elements(&interpreter.get_obj_result())
        .unwrap();
    assert_eq!(returned, [incoming]);
    let reported = unsafe { (*incoming).ref_count };
    assert_eq!(reported, before + 1);
    interpreter.set_result_bytes(b"REACHED DECLARATION RETIREMENT");
    let after = unsafe { (*incoming).ref_count };
    assert_eq!(after, before);
    REPORT_COUNTS.with(|counts| counts.borrow_mut().push((before, reported, after)));
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first declaration retirement");
        }
        2 => {
            let original_context = interpreter.runtime_context();
            let mut changed = original_context.clone();
            changed
                .packages
                .push(("declaration-retirement".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original_context).unwrap();
        }
        3 => {
            if CLASS.with(Cell::get) {
                interpreter.oo_destroy_class(TARGET.with(Cell::get));
            } else {
                assert_eq!(interpreter.oo_destroy(TARGET.with(Cell::get)), Code::Ok);
            }
            interpreter.set_result_bytes(b"REACHED TARGET WITHDRAWAL");
        }
        4 => {
            let source = match (CLASS.with(Cell::get), PRIVATE.with(Cell::get)) {
                (true, true) => {
                    b"oo::define DeclarationClass {private variable -set replacement}".as_slice()
                }
                (true, false) => {
                    b"oo::define DeclarationClass {variable -set replacement}".as_slice()
                }
                (false, true) => {
                    b"oo::objdefine DeclarationObject {private variable -set replacement}"
                        .as_slice()
                }
                (false, false) => {
                    b"oo::objdefine DeclarationObject {variable -set replacement}".as_slice()
                }
            };
            assert_eq!(interpreter.eval_str(source), Code::Ok);
        }
        _ => {}
    }
    REACHED.with(|result| {
        *result.borrow_mut() = Some(obj::Owned::retain(interpreter.get_obj_result()))
    });
}
static DECLARATION: obj::TclObjType = obj::TclObjType {
    name: c"originalDeclarationRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn native(class: bool, private: bool) -> Interp {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl9.1"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
        interpreter.eval_str(
            b"oo::class create DeclarationClass; DeclarationClass create DeclarationObject"
        ),
        Code::Ok
    );
    let target = interpreter.oo_resolve_object(if class {
        b"DeclarationClass"
    } else {
        b"DeclarationObject"
    });
    let slot = interpreter.oo_resolve_object(if class {
        b"::oo::define::variable"
    } else {
        b"::oo::objdefine::variable"
    });
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    CLASS.with(|selected| selected.set(class));
    PRIVATE.with(|selected| selected.set(private));
    TARGET.with(|selected| selected.set(target));
    SLOT.with(|selected| selected.set(slot));
    MODE.with(|mode| mode.set(0));
    FREES.with(|frees| frees.borrow_mut().clear());
    REPORT_COUNTS.with(|counts| counts.borrow_mut().clear());
    REACHED.with(|result| *result.borrow_mut() = None);
    interpreter
}
fn replace(interpreter: &mut Interp, original: *mut TclObj) -> Code {
    let mut words = vec![
        if CLASS.with(Cell::get) {
            b"oo::define".as_slice()
        } else {
            b"oo::objdefine".as_slice()
        },
        if CLASS.with(Cell::get) {
            b"DeclarationClass".as_slice()
        } else {
            b"DeclarationObject".as_slice()
        },
    ];
    if PRIVATE.with(Cell::get) {
        words.push(b"private");
    }
    words.extend([b"variable".as_slice(), b"-set"]);
    let words: Vec<_> = words
        .iter()
        .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
        .collect();
    let mut argv: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
    argv.push(original);
    interpreter.dispatch(&argv)
}
fn install_old(interpreter: &mut Interp) -> obj::NativeObjectLifetime {
    let old = obj::Owned::fresh(obj::alloc_typed(&DECLARATION, 17));
    unsafe { obj::set_string_rep(old.as_ptr(), b"oldDeclaration") };
    assert_eq!(replace(interpreter, old.as_ptr()), Code::Ok);
    assert_eq!(unsafe { (*old.as_ptr()).ref_count }, 2);
    let lease = obj::NativeObjectLifetime::retain(old.as_ptr());
    drop(old);
    assert!(FREES.with(|frees| frees.borrow().is_empty()));
    lease
}
fn current(interpreter: &Interp) -> Vec<(Vec<u8>, *mut TclObj)> {
    let target = if CLASS.with(Cell::get) {
        DefTarget::Class(TARGET.with(Cell::get))
    } else {
        DefTarget::Object(TARGET.with(Cell::get))
    };
    let state = interpreter.oo.borrow();
    let values = match target {
        DefTarget::Class(target) => {
            let owner = &state.classes[&target];
            if PRIVATE.with(Cell::get) {
                &owner.private_variables
            } else {
                &owner.variables
            }
        }
        DefTarget::Object(target) => {
            let owner = &state.objects[&target];
            if PRIVATE.with(Cell::get) {
                &owner.private_variables
            } else {
                &owner.variables
            }
        }
    };
    values
        .iter()
        .map(|value| (value.name.clone(), value.original.as_ptr()))
        .collect()
}
fn reset_observers() {
    OWNER.with(|owner| *owner.borrow_mut() = None);
    REACHED.with(|result| *result.borrow_mut() = None);
    INCOMING.with(|member| member.set(std::ptr::null_mut()));
}
#[test]
fn declaration_replacement_retires_old_headers_outside_borrow_after_new_adoption() {
    // Software adoption and callback transport; Native variable-map chronology is separate.
    for class in [false, true] {
        for private in [false, true] {
            for mode in 0..3 {
                let mut interpreter = native(class, private);
                let old = install_old(&mut interpreter);
                let incoming = obj::Owned::fresh(obj::new_string_bytes(b"newDeclaration"));
                INCOMING.with(|member| member.set(incoming.as_ptr()));
                MODE.with(|selected| selected.set(mode));
                let code = replace(&mut interpreter, incoming.as_ptr());
                assert_eq!(code, if mode == 0 { Code::Ok } else { Code::Error });
                assert_eq!(FREES.with(|frees| frees.borrow().clone()), [17]);
                assert_eq!(REPORT_COUNTS.with(|counts| counts.borrow().len()), 1);
                assert!(!obj::allocation_is_live(old.as_ptr()));
                assert_eq!(
                    current(&interpreter),
                    [(b"newDeclaration".to_vec(), incoming.as_ptr())]
                );
                assert_eq!(unsafe { (*incoming.as_ptr()).ref_count }, 2);
                match mode {
                    0 => {
                        assert_eq!(interpreter.result_bytes(), b"");
                        assert!(interpreter.native_execution_refusal().is_none());
                    }
                    1 => {
                        assert!(matches!(interpreter.native_execution_refusal(),
                            Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first))
                                if first.reason == "first declaration retirement"));
                    }
                    _ => assert!(interpreter.native_execution_refusal().is_some()),
                }
                if mode != 0 {
                    REACHED.with(|result| {
                        assert_eq!(
                            interpreter.get_obj_result(),
                            result.borrow().as_ref().unwrap().as_ptr()
                        )
                    });
                }
                assert_eq!(interpreter.entered_native_operation_count(), 0);
                reset_observers();
            }
        }
    }
}
#[test]
fn declaration_callback_can_replace_metadata_without_rejecting_current_target() {
    for class in [false, true] {
        for private in [false, true] {
            let mut interpreter = native(class, private);
            let old = install_old(&mut interpreter);
            let incoming = obj::Owned::fresh(obj::new_string_bytes(b"newDeclaration"));
            INCOMING.with(|member| member.set(incoming.as_ptr()));
            MODE.with(|selected| selected.set(4));
            assert_eq!(replace(&mut interpreter, incoming.as_ptr()), Code::Ok);
            assert_eq!(FREES.with(|frees| frees.borrow().clone()), [17]);
            assert!(!obj::allocation_is_live(old.as_ptr()));
            let values = current(&interpreter);
            assert_eq!(values.len(), 1);
            assert_eq!(values[0].0, b"replacement");
            assert_ne!(values[0].1, incoming.as_ptr());
            assert_eq!(unsafe { (*incoming.as_ptr()).ref_count }, 1);
            assert_eq!(interpreter.result_bytes(), b"");
            assert!(interpreter.native_execution_refusal().is_none());
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            reset_observers();
        }
    }
}
#[test]
fn declaration_retirement_target_withdrawal_stops_later_result_publication() {
    for class in [false, true] {
        for private in [false, true] {
            let mut interpreter = native(class, private);
            let old = install_old(&mut interpreter);
            let incoming = obj::Owned::fresh(obj::new_string_bytes(b"newDeclaration"));
            INCOMING.with(|member| member.set(incoming.as_ptr()));
            MODE.with(|selected| selected.set(3));
            assert_eq!(replace(&mut interpreter, incoming.as_ptr()), Code::Error);
            assert_eq!(FREES.with(|frees| frees.borrow().clone()), [17]);
            assert!(!obj::allocation_is_live(old.as_ptr()));
            assert!(!interpreter
                .oo
                .borrow()
                .objects
                .contains_key(&TARGET.with(Cell::get)));
            assert!(!interpreter
                .oo
                .borrow()
                .classes
                .contains_key(&TARGET.with(Cell::get)));
            assert_eq!(unsafe { (*incoming.as_ptr()).ref_count }, 1);
            assert!(interpreter.native_execution_refusal().is_none());
            assert!(interpreter
                .result_bytes()
                .windows(b"deleted".len())
                .any(|word| word == b"deleted"));
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            reset_observers();
        }
    }
}
