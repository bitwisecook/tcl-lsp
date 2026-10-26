// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software transport at authentic child, result and expression boundaries.
use super::*;
use std::cell::{Cell, RefCell};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static GETS: RefCell<[usize; 4]> = const { RefCell::new([0; 4]) };
    static REFS: RefCell<[obj::TclSize; 4]> = const { RefCell::new([0; 4]) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static FREE_REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static CALLBACK_OK: Cell<bool> = const { Cell::new(true) };
    static REACHED_RESULT: RefCell<Option<Rc<[u8]>>> = const { RefCell::new(None) };
}
struct OriginalTls;
impl OriginalTls {
    fn enter(interpreter: &mut Interp, mode: u8) -> Self {
        MODE.with(|value| value.set(mode));
        GETS.with(|value| *value.borrow_mut() = [0; 4]);
        REFS.with(|value| *value.borrow_mut() = [0; 4]);
        FREES.with(|value| value.set(0));
        FREE_REFS.with(|value| value.set(-1));
        CALLBACK_OK.with(|value| value.set(true));
        REACHED_RESULT.with(|value| *value.borrow_mut() = None);
        crate::codegen_abi::tcl_runtime_set_current_interp(interpreter);
        Self
    }
}
impl Drop for OriginalTls {
    fn drop(&mut self) {
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
    }
}
fn native() -> Interp {
    Interp::with_native_core(
        default_host(),
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn change_and_restore(interpreter: &mut Interp) {
    let original = interpreter.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("original-result-control".to_owned(), "1.0".to_owned())];
    let changed = interpreter.pin_context(&changed).is_ok();
    let restored = interpreter.pin_context(&original).is_ok();
    CALLBACK_OK.with(|value| value.set(value.get() && changed && restored));
}
extern "C" fn update(original: *mut TclObj) {
    let role = obj::internal_rep(original) as usize;
    if role >= 4 {
        return;
    }
    GETS.with(|value| value.borrow_mut()[role] += 1);
    REFS.with(|value| value.borrow_mut()[role] = unsafe { (*original).ref_count });
    if let Some(interpreter) = unsafe { crate::codegen_abi::current_interp().as_ref() } {
        let mut interpreter = interpreter.clone();
        match MODE.with(Cell::get) {
            1 => {
                interpreter.refuse_host_command("first original result updater");
            }
            2 => change_and_restore(&mut interpreter),
            3 => interpreter.set_result_bytes(b"REPLACED"),
            4 => {
                let deleted = interpreter.delete_child(b"kid");
                let created = interpreter.create_child(Some(b"kid".to_vec()));
                CALLBACK_OK.with(|value| value.set(value.get() && deleted && created == b"kid"));
            }
            _ => {}
        }
    } else {
        CALLBACK_OK.with(|value| value.set(false));
    }
    let spelling: &[u8] = match role {
        0 => b"kid",
        1 => b"issafe",
        2 => b"S",
        _ => b"EXPR",
    };
    unsafe { obj::set_native_updater_string_rep(original, spelling, false) };
}
extern "C" fn retire(original: *mut TclObj) {
    FREES.with(|value| value.set(value.get() + 1));
    FREE_REFS.with(|value| value.set(unsafe { (*original).ref_count }));
    let Some(interpreter) = (unsafe { crate::codegen_abi::current_interp().as_ref() }) else {
        return;
    };
    let mut interpreter = interpreter.clone();
    if MODE.with(Cell::get) == 6 {
        let resident = obj::native_object_snapshot(interpreter.result_obj())
            .ok()
            .and_then(|value| value.resident);
        REACHED_RESULT.with(|value| *value.borrow_mut() = resident);
        interpreter.refuse_host_command("original result retirement");
    }
}
static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"originalResultBoundary".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn value(role: u64) -> obj::Owned {
    obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, role))
}
fn child(parent: &mut Interp) -> ChildInterpreterCommand {
    assert_eq!(parent.create_child(Some(b"kid".to_vec())), b"kid");
    let Some(Command::ChildInterp(command)) = parent.namespaces.borrow().command_in(GLOBAL, b"kid")
    else {
        panic!("actual child command allocation");
    };
    command
}
fn stale() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::CommandProtocolUnavailable(
        "stale entered native operation",
    ))
}
fn assert_updater_cause(interpreter: &Interp, mode: u8) {
    let first = interpreter.native_execution_refusal().unwrap();
    if mode == 1 {
        assert!(
            matches!(first, NativeExecutionError::HostCommandRefusal(ref reason) if reason.reason == "first original result updater")
        );
    } else {
        assert_eq!(first, stale());
    }
    assert!(CALLBACK_OK.with(Cell::get));
}

#[test]
fn original_child_usage_getter_keeps_first_host_and_restored_context() {

    for mode in [1, 2] {
        let mut interpreter = native();
        let command = child(&mut interpreter);
        let head = value(0);
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.result_obj();
        let outer = native_arguments::OriginalArguments::enter(&interpreter, &[]).unwrap();
        let _tls = OriginalTls::enter(&mut interpreter, mode);
        assert_eq!(
            interpreter.dispatch_original_child(&command, &[head.as_ptr()]),
            Code::Error
        );
        assert_updater_cause(&interpreter, mode);
        assert_eq!(GETS.with(|value| *value.borrow()), [1, 0, 0, 0]);
        assert_eq!(REFS.with(|value| value.borrow()[0]), 1);
        assert_eq!(interpreter.result_obj(), prior);
        assert!(command.is_current_for(&interpreter));
        assert!(outer.check_for(&interpreter).is_err());
    }
}
#[test]
fn original_child_option_getter_refuses_recreated_allocation_before_query() {

    let mut interpreter = native();
    let command = child(&mut interpreter);
    let head = obj::Owned::fresh(new_string(b"kid"));
    let option = value(1);
    interpreter.set_result_bytes(b"PRIOR");
    let prior = interpreter.result_obj();
    let _outer = native_arguments::OriginalArguments::enter(&interpreter, &[]).unwrap();
    let _tls = OriginalTls::enter(&mut interpreter, 4);
    assert_eq!(
        interpreter.dispatch_original_child(&command, &[head.as_ptr(), option.as_ptr()]),
        Code::Error
    );
    let first = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable(
            "changed original child command allocation",
        ),
    );
    assert_eq!(interpreter.native_execution_refusal(), Some(first));
    assert_eq!(GETS.with(|value| *value.borrow()), [0, 1, 0, 0]);
    assert_eq!(REFS.with(|value| value.borrow()[1]), 1);
    assert!(!command.is_current_for(&interpreter));
    assert!(interpreter
        .children
        .borrow()
        .contains_key(b"kid".as_slice()));
    assert!(CALLBACK_OK.with(Cell::get));
    assert_eq!(interpreter.result_obj(), prior);
}
#[test]
fn original_child_invokehidden_usage_refuses_before_later_hidden_name() {

    for mode in [1, 2] {
        let mut interpreter = native();
        let command = child(&mut interpreter);
        let head = value(0);
        let option = obj::Owned::fresh(new_string(b"invokehidden"));
        let later = value(3);
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.result_obj();
        let _outer = native_arguments::OriginalArguments::enter(&interpreter, &[]).unwrap();
        let _tls = OriginalTls::enter(&mut interpreter, mode);
        assert_eq!(
            interpreter.dispatch_original_child(
                &command,
                &[head.as_ptr(), option.as_ptr(), later.as_ptr()]
            ),
            Code::Error
        );
        assert_updater_cause(&interpreter, mode);
        assert_eq!(GETS.with(|value| *value.borrow()), [1, 0, 0, 0]);
        assert_eq!(REFS.with(|value| value.borrow()[0]), 1);
        assert_eq!(interpreter.result_obj(), prior);
        assert!(command.is_current_for(&interpreter));
    }
}
#[test]
fn original_result_append_keeps_actual_inplace_and_shared_header_roles() {

    for shared in [false, true] {
        let mut interpreter = native();
        interpreter.set_result_bytes(b"S");
        let prior = interpreter.result_obj();
        let caller = shared.then(|| obj::Owned::retain(prior));
        interpreter.append_result_bytes_checked(b"+").unwrap();
        assert_eq!(interpreter.result_bytes(), b"S+");
        if shared {
            assert_ne!(interpreter.result_obj(), prior);
            assert_eq!(obj::bytes_of(caller.as_ref().unwrap().as_ptr()), b"S");
            assert_eq!(unsafe { (*prior).ref_count }, 1);
        } else {
            assert_eq!(interpreter.result_obj(), prior);
        }
        assert_eq!(unsafe { (*interpreter.result_obj()).ref_count }, 1);
        assert!(interpreter.native_execution_refusal().is_none());
    }
}
#[test]
fn original_result_append_getter_refuses_before_suffix_or_replacement_adoption() {

    for mode in [1, 2, 3] {
        let mut interpreter = native();
        let original = value(2);
        interpreter.set_result(original.as_ptr());
        let _outer = native_arguments::OriginalArguments::enter(&interpreter, &[]).unwrap();
        let _tls = OriginalTls::enter(&mut interpreter, mode);
        let failure = interpreter.append_result_bytes_checked(b"+").unwrap_err();
        let first = interpreter.native_execution_refusal().unwrap();
        assert_eq!(failure.native_execution_refusal(), Some(&first));
        if mode == 3 {
            assert_eq!(
                first,
                NativeExecutionError::ValueAccessRefusal(
                    NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "changed original appended result slot"
                    )
                )
            );
            assert_ne!(interpreter.result_obj(), original.as_ptr());
            assert_eq!(
                obj::native_object_snapshot(interpreter.result_obj())
                    .unwrap()
                    .resident
                    .as_deref(),
                Some(b"REPLACED".as_slice())
            );
            assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
        } else {
            assert_updater_cause(&interpreter, mode);
            assert_eq!(interpreter.result_obj(), original.as_ptr());
            assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 2);
        }
        assert_eq!(GETS.with(|value| *value.borrow()), [0, 0, 1, 0]);
        assert_eq!(REFS.with(|value| value.borrow()[2]), 2);
        assert_eq!(
            obj::native_object_snapshot(original.as_ptr())
                .unwrap()
                .resident
                .as_deref(),
            Some(b"S".as_slice())
        );
    }
}
#[test]
fn original_c_api_result_append_retirement_observes_reached_publication_and_first_cause() {

    let mut interpreter = native();
    let original = obj::alloc_typed(&ORIGINAL, 2);
    unsafe { obj::set_native_updater_string_rep(original, b"S", false) };
    let allocation = obj::NativeObjectLifetime::retain(original);
    interpreter.set_result(original);
    assert_eq!(unsafe { (*original).ref_count }, 1);
    let _outer = native_arguments::OriginalArguments::enter(&interpreter, &[]).unwrap();
    let _tls = OriginalTls::enter(&mut interpreter, 6);
    unsafe { crate::capi::TclHost_AppendResultString(&mut interpreter, c"+".as_ptr()) };
    assert!(
        matches!(interpreter.native_execution_refusal(), Some(NativeExecutionError::HostCommandRefusal(ref reason)) if reason.reason == "original result retirement")
    );
    assert_eq!(FREES.with(Cell::get), 1);
    assert_eq!(FREE_REFS.with(Cell::get), 0);
    assert!(!obj::allocation_is_live(allocation.as_ptr()));
    assert_eq!(
        REACHED_RESULT
            .with(|value| value.borrow().clone())
            .as_deref(),
        Some(b"S+".as_slice())
    );
    assert_eq!(
        obj::native_object_snapshot(interpreter.result_obj())
            .unwrap()
            .resident
            .as_deref(),
        Some(b"S+".as_slice())
    );
    assert_eq!(GETS.with(|value| *value.borrow()), [0; 4]);
}
#[cfg(have_tommath)]
#[test]
fn original_word_expression_result_consumes_true_owned_role_after_getter_and_retirement() {

    // The actual evaluator produces the +1; the subsequent extension is software transport only.
    for mode in [0, 1, 2, 6] {
        let mut interpreter = native();
        let arguments = native_arguments::OriginalArguments::enter(&interpreter, &[]).unwrap();
        let result = interpreter.eval_word_expression(b"2").unwrap();
        assert_eq!(unsafe { (*result).ref_count }, 1);
        let allocation = obj::NativeObjectLifetime::retain(result);
        obj::change_type(result, &ORIGINAL, 3);
        obj::invalidate_string(result);
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.result_obj();
        let _tls = OriginalTls::enter(&mut interpreter, mode);
        let reached = interpreter.word_expression_result_bytes_checked(result, &arguments);
        assert_eq!(GETS.with(|value| *value.borrow()), [0, 0, 0, 1]);
        assert_eq!(REFS.with(|value| value.borrow()[3]), 1);
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(FREE_REFS.with(Cell::get), 0);
        assert!(!obj::allocation_is_live(allocation.as_ptr()));
        assert_eq!(interpreter.result_obj(), prior);
        if mode == 0 {
            assert_eq!(reached.unwrap(), b"EXPR");
        } else {
            let failure = reached.unwrap_err();
            let first = interpreter.native_execution_refusal().unwrap();
            assert_eq!(failure.native_execution_refusal(), Some(&first));
            if mode == 6 {
                assert!(
                    matches!(first, NativeExecutionError::HostCommandRefusal(ref reason) if reason.reason == "original result retirement")
                );
            } else {
                assert_updater_cause(&interpreter, mode);
            }
        }
    }
}
#[cfg(not(have_tommath))]
#[test]
fn original_word_expression_missing_engine_refuses_before_result_worker() {

    let mut interpreter = native();
    interpreter.set_result_bytes(b"PRIOR");
    let prior = interpreter.result_obj();
    let _outer = native_arguments::OriginalArguments::enter(&interpreter, &[]).unwrap();
    let _tls = OriginalTls::enter(&mut interpreter, 0);
    assert_eq!(interpreter.word_expression_bytes(b"2"), Err(Code::Error));
    assert_eq!(
        interpreter.native_execution_refusal(),
        Some(NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::ExpressionEngineUnavailable
        ))
    );
    assert_eq!(GETS.with(|value| *value.borrow()), [0; 4]);
    assert_eq!(FREES.with(Cell::get), 0);
    assert_eq!(interpreter.result_obj(), prior);
}
