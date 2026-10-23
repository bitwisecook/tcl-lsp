// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual class-facet births and checked original class reporting.
use super::*;
use crate::interp::native_operation_currency::NativeOperationScope;
use crate::interp::WeakInterp;
use std::cell::Cell;

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static ROLE: Cell<usize> = const { Cell::new(0) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static TARGET: Cell<OoId> = const { Cell::new(OoId(0)) };
    static REACHED: RefCell<Option<obj::Owned>> = const { RefCell::new(None) };
}
fn native() -> Interp {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl9.1"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
        interpreter.eval_str(b"oo::class create FacetClass"),
        Code::Ok
    );
    TARGET.with(|target| target.set(interpreter.oo_resolve_object(b"FacetClass")));
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|mode| mode.set(0));
    ROLE.with(|role| role.set(0));
    CALLS.with(|calls| calls.set(0));
    REACHED.with(|result| *result.borrow_mut() = None);
    interpreter
}
fn replace_facet(interpreter: &mut Interp) {
    let target = TARGET.with(Cell::get);
    let original_context = interpreter.runtime_context();
    let original_creation = interpreter.oo.borrow().objects[&target].creation_id;
    let original_birth = Rc::clone(&interpreter.oo.borrow().classes[&target].facet_birth);
    let scope = NativeOperationScope::enter(interpreter).unwrap();
    assert_eq!(
        interpreter.eval_str(b"oo::objdefine FacetClass {class ::oo::object}"),
        Code::Ok
    );
    assert!(!interpreter.oo.borrow().classes.contains_key(&target));
    assert_eq!(
        interpreter.eval_str(b"oo::objdefine FacetClass {class ::oo::class}"),
        Code::Ok
    );
    assert_eq!(interpreter.runtime_context(), original_context);
    assert_eq!(
        interpreter.oo.borrow().objects[&target].creation_id,
        original_creation
    );
    assert!(!Rc::ptr_eq(
        &original_birth,
        &interpreter.oo.borrow().classes[&target].facet_birth
    ));
    assert!(scope.currency().ensure_current_or_refuse().is_ok());
}
extern "C" fn update(original: *mut TclObj) {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    assert!(interpreter.oo.try_borrow_mut().is_ok());
    interpreter.set_result_bytes(b"REACHED CLASS REPORT OPERAND");
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first original class report operand");
        }
        2 => {
            let original_context = interpreter.runtime_context();
            let mut changed = original_context.clone();
            changed
                .packages
                .push(("class-report-operand".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original_context).unwrap();
        }
        3 => interpreter.oo_destroy_class(TARGET.with(Cell::get)),
        4 => {
            replace_facet(&mut interpreter);
            interpreter.set_result_bytes(b"REACHED CLASS REPORT OPERAND");
        }
        5 => {
            assert_eq!(
                interpreter.eval_str(b"oo::define FacetClass {method extra {} {return EXTRA}}"),
                Code::Ok
            );
            interpreter.set_result_bytes(b"REACHED CLASS REPORT OPERAND");
        }
        _ => {}
    }
    REACHED.with(|result| {
        *result.borrow_mut() = Some(obj::Owned::retain(interpreter.get_obj_result()))
    });
    let bytes = match ROLE.with(Cell::get) {
        0 => b"-instance".as_slice(),
        1 => b"-all",
        2 => b"*",
        3 => b"-readable",
        _ => panic!("fixed class report operand"),
    };
    unsafe { obj::set_native_updater_string_rep(original, bytes, false) };
}
static ARGUMENT: obj::TclObjType = obj::TclObjType {
    name: c"originalClassReportOperand".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn word(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
fn reset_observers() {
    OWNER.with(|owner| *owner.borrow_mut() = None);
    REACHED.with(|result| *result.borrow_mut() = None);
}

#[test]
fn actual_class_allocation_and_copy_issue_fresh_births_while_observations_share_identity() {
    // naming.tcloo.original-introspection-target-continuation

    // Actual Runtime lifecycle roles; no Native Class pointer/refcount inference.
    let first_default = Class::default();
    let second_default = Class::default();
    assert!(!Rc::ptr_eq(
        &first_default.facet_birth,
        &second_default.facet_birth
    ));
    let mut interpreter = native();
    let target = TARGET.with(Cell::get);
    let observed = interpreter.oo.borrow().classes[&target].clone();
    assert!(Rc::ptr_eq(
        &observed.facet_birth,
        &interpreter.oo.borrow().classes[&target].facet_birth
    ));
    let retained = native_context::OriginalOoTarget::capture(&interpreter, target, true).unwrap();
    assert_eq!(
        interpreter.eval_str(b"oo::define FacetClass {method extra {} {return EXTRA}}"),
        Code::Ok
    );
    assert!(retained.check(&interpreter).is_ok());
    assert_eq!(
        interpreter.eval_str(b"oo::copy FacetClass FacetCopy"),
        Code::Ok
    );
    let copied = interpreter.oo_resolve_object(b"FacetCopy");
    assert_ne!(target, copied);
    assert!(!Rc::ptr_eq(
        &observed.facet_birth,
        &interpreter.oo.borrow().classes[&copied].facet_birth
    ));
    assert!(retained.check(&interpreter).is_ok());
    let state = interpreter.oo.borrow();
    let object_root = state.object_root.unwrap();
    let class_root = state.class_root.unwrap();
    assert!(!Rc::ptr_eq(
        &state.classes[&object_root].facet_birth,
        &state.classes[&class_root].facet_birth
    ));
    drop(state);
    reset_observers();
}

#[test]
fn demotion_and_promotion_cannot_refresh_an_original_class_facet_continuation() {
    // naming.tcloo.original-introspection-target-continuation

    let mut interpreter = native();
    let target = TARGET.with(Cell::get);
    let observed = interpreter.oo.borrow().classes[&target].clone();
    let original = native_context::OriginalOoTarget::capture(&interpreter, target, true).unwrap();
    replace_facet(&mut interpreter);
    // A shallow old observation may still exist, but it is not the current map owner.
    assert!(!Rc::ptr_eq(
        &observed.facet_birth,
        &interpreter.oo.borrow().classes[&target].facet_birth
    ));
    let refusal = original.check(&interpreter).unwrap_err();
    assert_eq!(
        refusal.native_execution_refusal(),
        Some(&tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original TclOO class facet"
            ),
        ))
    );
    assert!(
        native_context::OriginalOoTarget::capture(&interpreter, target, true)
            .unwrap()
            .check(&interpreter)
            .is_ok()
    );
    assert!(interpreter.native_execution_refusal().is_none());
    assert_eq!(interpreter.entered_native_operation_count(), 0);
    reset_observers();
}

#[test]
fn original_class_reports_check_reached_operands_before_current_facet_access() {
    // naming.tcloo.original-introspection-target-continuation

    for (member, role) in [
        (b"definitionnamespace".as_slice(), 0),
        (b"methods", 1),
        (b"instances", 2),
        (b"subclasses", 2),
        (b"properties", 3),
    ] {
        for mode in 0..6 {
            let mut interpreter = native();
            ROLE.with(|selected| selected.set(role));
            let head = word(b"info");
            let class = word(b"class");
            let operation = word(member);
            let target = word(b"FacetClass");
            let argument = obj::Owned::fresh(obj::alloc_typed(&ARGUMENT, 0));
            MODE.with(|selected| selected.set(mode));
            assert_eq!(
                info_class(
                    &mut interpreter,
                    &[
                        head.as_ptr(),
                        class.as_ptr(),
                        operation.as_ptr(),
                        target.as_ptr(),
                        argument.as_ptr()
                    ]
                ),
                if mode == 0 || mode == 5 {
                    Code::Ok
                } else {
                    Code::Error
                }
            );
            assert_eq!(CALLS.with(Cell::get), 1);
            assert!(obj::has_string_rep(argument.as_ptr()));
            assert_eq!(unsafe { (*argument.as_ptr()).ref_count }, 1);
            match mode {
                0 | 5 => assert!(interpreter.native_execution_refusal().is_none()),
                1 => assert!(matches!(interpreter.native_execution_refusal(),
                    Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first))
                        if first.reason == "first original class report operand")),
                2 => assert!(interpreter.native_execution_refusal().is_some()),
                3 => {
                    assert!(!interpreter.oo.borrow().classes.contains_key(&TARGET.with(Cell::get)));
                    assert!(interpreter.native_execution_refusal().is_none());
                    assert_eq!(interpreter.snapshot_error().code_bytes(), b"TCL OO MONKEY_BUSINESS");
                }
                4 => assert_eq!(interpreter.native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("original TclOO class facet"),
                ))),
                _ => unreachable!(),
            }
            if mode == 1 || mode == 2 || mode == 4 {
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
