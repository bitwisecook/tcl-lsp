// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original introspection receivers survive only their genuine lifecycle.
use super::*;
use crate::interp::WeakInterp;
use std::cell::Cell;

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static CLASS: Cell<bool> = const { Cell::new(false) };
    static TARGET: Cell<OoId> = const { Cell::new(OoId(0)) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static ROLE: Cell<usize> = const { Cell::new(0) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static REACHED: RefCell<Option<obj::Owned>> = const { RefCell::new(None) };
}

fn native(class: bool) -> Interp {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl9.1"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(interpreter.eval_str(b"oo::class create ReportClass {method m {} {return M}; forward fwd list HELD}; ReportClass create ReportObject; oo::objdefine ReportObject {method m {} {return OBJECT}; forward fwd list OBJECT}"), Code::Ok);
    CLASS.with(|selected| selected.set(class));
    let name = if class {
        b"ReportClass".as_slice()
    } else {
        b"ReportObject"
    };
    TARGET.with(|selected| selected.set(interpreter.oo_resolve_object(name)));
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    CALLS.with(|calls| calls.set(0));
    MODE.with(|mode| mode.set(0));
    ROLE.with(|role| role.set(0));
    REACHED.with(|result| *result.borrow_mut() = None);
    interpreter
}

fn destroy(interpreter: &mut Interp) {
    let target = TARGET.with(Cell::get);
    if CLASS.with(Cell::get) {
        interpreter.oo_destroy_class(target);
    } else {
        interpreter.oo_destroy_bg(target);
    }
}

extern "C" fn update(original: *mut TclObj) {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    assert!(interpreter.oo.try_borrow_mut().is_ok());
    interpreter.set_result_bytes(b"REACHED ORIGINAL REPORT OPERAND");
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first original report receiver");
        }
        2 => {
            let original_context = interpreter.runtime_context();
            let mut changed = original_context.clone();
            changed
                .packages
                .push(("original-report-operand".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original_context).unwrap();
        }
        3 => destroy(&mut interpreter),
        4 if CLASS.with(Cell::get) => {
            let target = TARGET.with(Cell::get);
            let creation = interpreter.oo.borrow().objects[&target].creation_id;
            let birth = Rc::clone(&interpreter.oo.borrow().classes[&target].facet_birth);
            assert_eq!(
                interpreter.eval_str(b"oo::objdefine ReportClass {class ::oo::object}"),
                Code::Ok
            );
            assert_eq!(
                interpreter.eval_str(b"oo::objdefine ReportClass {class ::oo::class}"),
                Code::Ok
            );
            assert_eq!(
                interpreter.oo.borrow().objects[&target].creation_id,
                creation
            );
            assert!(!Rc::ptr_eq(
                &birth,
                &interpreter.oo.borrow().classes[&target].facet_birth
            ));
            interpreter.set_result_bytes(b"REACHED ORIGINAL REPORT OPERAND");
        }
        4 | 6 => {
            let original_target = TARGET.with(Cell::get);
            destroy(&mut interpreter);
            let source = if CLASS.with(Cell::get) {
                b"oo::class create ReportClass {method m {} {return NEW}; forward fwd list NEW}"
                    .as_slice()
            } else {
                b"ReportClass create ReportObject; oo::objdefine ReportObject {method m {} {return NEW}; forward fwd list NEW}"
            };
            assert_eq!(interpreter.eval_str(source), Code::Ok);
            let name = if CLASS.with(Cell::get) {
                b"ReportClass".as_slice()
            } else {
                b"ReportObject"
            };
            assert_ne!(interpreter.oo_resolve_object(name), original_target);
            interpreter.set_result_bytes(b"REACHED ORIGINAL REPORT OPERAND");
        }
        5 => {
            let source = if CLASS.with(Cell::get) {
                b"oo::define ReportClass {method added {} {return ADDED}}".as_slice()
            } else {
                b"oo::objdefine ReportObject {method added {} {return ADDED}}"
            };
            assert_eq!(interpreter.eval_str(source), Code::Ok);
            interpreter.set_result_bytes(b"REACHED ORIGINAL REPORT OPERAND");
        }
        7 => {
            let namespace = interpreter.oo.borrow().objects[&TARGET.with(Cell::get)].var_ns;
            interpreter.delete_namespace_by_id(namespace);
        }
        _ => {}
    }
    REACHED.with(|result| {
        *result.borrow_mut() = Some(obj::Owned::retain(interpreter.get_obj_result()))
    });
    let bytes = match ROLE.with(Cell::get) {
        0 => b"m".as_slice(),
        1 => b"fwd",
        2 => b"-all",
        3 => b"*",
        4 => b"-readable",
        5 => b"ReportClass",
        _ => panic!("fixed original report role"),
    };
    unsafe { obj::set_native_updater_string_rep(original, bytes, false) };
}

static ARGUMENT: obj::TclObjType = obj::TclObjType {
    name: c"originalReportReceiverOperand".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn word(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
fn argument() -> obj::Owned {
    obj::Owned::fresh(obj::alloc_typed(&ARGUMENT, 0))
}
fn reset_observers() {
    OWNER.with(|owner| *owner.borrow_mut() = None);
    REACHED.with(|result| *result.borrow_mut() = None);
}
fn check_reached(interpreter: &Interp, argument: &obj::Owned, mode: u8, class: bool) {
    assert_eq!(CALLS.with(Cell::get), 1);
    assert!(obj::has_string_rep(argument.as_ptr()));
    assert_eq!(unsafe { (*argument.as_ptr()).ref_count }, 1);
    match mode {
        0 | 5 => assert!(interpreter.native_execution_refusal().is_none()),
        1 => assert!(matches!(interpreter.native_execution_refusal(),
            Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first))
                if first.reason == "first original report receiver")),
        2 => assert!(interpreter.native_execution_refusal().is_some()),
        4 if class => assert_eq!(
            interpreter.native_execution_refusal(),
            Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original TclOO class facet",
                ),
            ))
        ),
        3 | 4 | 6 | 7 => {
            assert!(interpreter.native_execution_refusal().is_none());
            assert_eq!(
                interpreter.snapshot_error().code_bytes(),
                b"TCL OO MONKEY_BUSINESS"
            );
        }
        _ => unreachable!(),
    }
    if mode == 1 || mode == 2 || (mode == 4 && class) {
        REACHED.with(|result| {
            assert_eq!(
                interpreter.get_obj_result(),
                result.borrow().as_ref().unwrap().as_ptr()
            )
        });
    }
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}

#[test]
fn shared_method_reports_keep_original_receiver_before_name_getters_and_table_access() {
    // naming.tcloo.original-introspection-target-continuation
    // docs/design/analysis/name-resolution-proofs/tcloo-original-introspection-target-continuation.md
    // Software receiver/refusal control; no Native pointer-lifetime conclusion.
    for class in [false, true] {
        for (member, role) in [
            (b"forward".as_slice(), 1),
            (b"definition", 0),
            (b"methodtype", 0),
            (b"call", 0),
        ] {
            for mode in 0..8 {
                let mut interpreter = native(class);
                ROLE.with(|selected| selected.set(role));
                MODE.with(|selected| selected.set(mode));
                let head = word(b"info");
                let kind = word(if class { b"class" } else { b"object" });
                let operation = word(member);
                let target = word(if class {
                    b"ReportClass"
                } else {
                    b"ReportObject"
                });
                let argument = argument();
                let argv = [
                    head.as_ptr(),
                    kind.as_ptr(),
                    operation.as_ptr(),
                    target.as_ptr(),
                    argument.as_ptr(),
                ];
                let code = if class {
                    info_class(&mut interpreter, &argv)
                } else {
                    info_object(&mut interpreter, &argv)
                };
                assert_eq!(
                    code,
                    if mode == 0 || mode == 5 {
                        Code::Ok
                    } else {
                        Code::Error
                    }
                );
                check_reached(&interpreter, &argument, mode, class);
                reset_observers();
            }
        }
    }
}

#[test]
fn object_reports_refuse_withdrawn_receivers_before_relation_namespace_or_report_derivation() {
    // naming.tcloo.original-introspection-target-continuation
    // docs/design/analysis/name-resolution-proofs/tcloo-original-introspection-target-continuation.md
    for (member, role) in [
        (b"methods".as_slice(), 2),
        (b"vars", 3),
        (b"properties", 4),
        (b"class", 5),
    ] {
        for mode in 0..8 {
            let mut interpreter = native(false);
            ROLE.with(|selected| selected.set(role));
            MODE.with(|selected| selected.set(mode));
            let head = word(b"info");
            let kind = word(b"object");
            let operation = word(member);
            let target = word(b"ReportObject");
            let argument = argument();
            let code = info_object(
                &mut interpreter,
                &[
                    head.as_ptr(),
                    kind.as_ptr(),
                    operation.as_ptr(),
                    target.as_ptr(),
                    argument.as_ptr(),
                ],
            );
            assert_eq!(
                code,
                if mode == 0 || mode == 5 {
                    Code::Ok
                } else {
                    Code::Error
                }
            );
            check_reached(&interpreter, &argument, mode, false);
            reset_observers();
        }
    }
}

#[test]
fn isa_missing_precondition_leaves_relation_unread_but_selected_receiver_withdrawal_refuses() {
    // naming.tcloo.original-introspection-target-continuation
    // docs/design/analysis/name-resolution-proofs/tcloo-original-introspection-target-continuation.md
    for category in [b"mixin".as_slice(), b"typeof"] {
        for missing in [false, true] {
            for mode in 0..8 {
                let mut interpreter = native(false);
                ROLE.with(|selected| selected.set(5));
                MODE.with(|selected| selected.set(mode));
                let head = word(b"info");
                let kind = word(b"object");
                let operation = word(b"isa");
                let category = word(category);
                let target = word(if missing {
                    b"NotAnObject"
                } else {
                    b"ReportObject"
                });
                let argument = argument();
                let code = info_object(
                    &mut interpreter,
                    &[
                        head.as_ptr(),
                        kind.as_ptr(),
                        operation.as_ptr(),
                        category.as_ptr(),
                        target.as_ptr(),
                        argument.as_ptr(),
                    ],
                );
                if missing {
                    assert_eq!(code, Code::Ok);
                    assert_eq!(interpreter.result_bytes(), b"0");
                    assert_eq!(CALLS.with(Cell::get), 0);
                    assert!(!obj::has_string_rep(argument.as_ptr()));
                    assert_eq!(unsafe { (*argument.as_ptr()).ref_count }, 1);
                    assert!(interpreter.native_execution_refusal().is_none());
                    assert_eq!(interpreter.entered_native_operation_count(), 0);
                } else {
                    assert_eq!(
                        code,
                        if mode == 0 || mode == 5 {
                            Code::Ok
                        } else {
                            Code::Error
                        }
                    );
                    check_reached(&interpreter, &argument, mode, false);
                }
                reset_observers();
            }
        }
    }
}
