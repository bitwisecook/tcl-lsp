// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software controls for genuine argument, rewrite and cleanup owners.

use super::*;
use crate::{
    codegen_abi,
    interp::{Code, GLOBAL},
};
use std::cell::{Cell, RefCell};
use tcl_runtime_api::NativeExecutionError;

thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static GETS: RefCell<[usize; 4]> = const { RefCell::new([0; 4]) };
    static REFS: RefCell<[obj::TclSize; 4]> = const { RefCell::new([0; 4]) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static BORROWS: Cell<bool> = const { Cell::new(true) };
    static EMPTY: Cell<bool> = const { Cell::new(true) };
    static NESTED: Cell<bool> = const { Cell::new(true) };
    static CALLER: RefCell<Option<obj::Owned>> = const { RefCell::new(None) };
}

struct OriginalTls;
impl OriginalTls {
    fn enter(interpreter: &mut Interp, mode: u8) -> Self {
        MODE.with(|value| value.set(mode));
        GETS.with(|value| *value.borrow_mut() = [0; 4]);
        REFS.with(|value| *value.borrow_mut() = [0; 4]);
        FREES.with(|value| value.set(0));
        BORROWS.with(|value| value.set(true));
        EMPTY.with(|value| value.set(true));
        NESTED.with(|value| value.set(true));
        codegen_abi::tcl_runtime_set_current_interp(interpreter);
        Self
    }
}
impl Drop for OriginalTls {
    fn drop(&mut self) {
        codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
        let caller = CALLER.with(|value| value.borrow_mut().take());
        drop(caller);
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

fn change_and_restore(interpreter: &mut Interp) {
    let original = interpreter.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("original-usage-control".to_owned(), "1.0".to_owned())];
    let changed_ok = interpreter.pin_context(&changed).is_ok();
    let restored_ok = interpreter.pin_context(&original).is_ok();
    NESTED.with(|value| value.set(value.get() && changed_ok && restored_ok));
}

extern "C" fn update(original: *mut TclObj) {
    let role = obj::internal_rep(original) as usize;
    if role >= 4 {
        return;
    }
    GETS.with(|value| value.borrow_mut()[role] += 1);
    REFS.with(|value| value.borrow_mut()[role] = unsafe { (*original).ref_count });
    if role == 0 || (role == 1 && MODE.with(Cell::get) >= 5) {
        if let Some(interpreter) = unsafe { codegen_abi::current_interp().as_ref() } {
            let mut interpreter = interpreter.clone();
            match MODE.with(Cell::get) {
                1 | 5 => {
                    interpreter.refuse_host_command("first original usage updater");
                }
                2 | 6 => change_and_restore(&mut interpreter),
                3 => {
                    interpreter.begin_ensemble_rewrite(Vec::new(), 0, 0);
                }
                4 => {
                    let replacement = obj::Owned::retain(original);
                    let _ = interpreter.clear_ensemble_rewrite_checked();
                    interpreter.begin_ensemble_rewrite(vec![replacement], 1, 1);
                }
                _ => {}
            }
        } else {
            NESTED.with(|value| value.set(false));
        }
    }
    unsafe {
        obj::set_native_updater_string_rep(
            original,
            if role == 0 { b"original" } else { b"later" },
            false,
        )
    };
}

static UPDATER: obj::TclObjType = obj::TclObjType {
    name: c"originalUsageUpdater".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};

extern "C" fn retire(original: *mut TclObj) {
    FREES.with(|value| value.set(value.get() + 1));
    let Some(interpreter) = (unsafe { codegen_abi::current_interp().as_ref() }) else {
        BORROWS.with(|value| value.set(false));
        return;
    };
    let mut interpreter = interpreter.clone();
    let rewrite_free = obj::internal_rep(original) == 0;
    let available = if rewrite_free {
        interpreter.ensemble_rewrite.try_borrow_mut().is_ok()
    } else {
        interpreter.handler_usage_adapters.try_borrow_mut().is_ok()
    };
    BORROWS.with(|value| value.set(value.get() && available));
    if available {
        let empty = if rewrite_free {
            interpreter.ensemble_rewrite.borrow().is_none()
        } else {
            interpreter.handler_usage_adapters.borrow().is_empty()
        };
        EMPTY.with(|value| value.set(value.get() && empty));
        let query = obj::Owned::fresh(crate::interp::new_string(b"nested"));
        let observed = interpreter.argument_usage_prefix(&[query.as_ptr()], 1);
        NESTED.with(|value| {
            value.set(value.get() && observed.as_deref() == Ok(b"nested".as_slice()))
        });
    }
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first actual usage retirement");
        }
        2 => change_and_restore(&mut interpreter),
        _ => {}
    }
}

static RETIRE: obj::TclObjType = obj::TclObjType {
    name: c"originalUsageRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

fn prior_result(interpreter: &mut Interp) -> *mut TclObj {
    interpreter.set_result_bytes(b"PRIOR");
    interpreter.get_obj_result()
}

fn assert_prior(interpreter: &Interp, prior: *mut TclObj) {
    assert_eq!(interpreter.get_obj_result(), prior);
    assert_eq!(
        obj::native_object_snapshot(prior)
            .unwrap()
            .resident
            .as_deref(),
        Some(b"PRIOR".as_slice())
    );
}

#[test]
fn original_usage_projection_keeps_actual_rewrite_counts_and_leaves_operands_unread() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software sharing/control only; no external wrong-args chronology grant.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = native(profile);
        let _tls = OriginalTls::enter(&mut interpreter, 0);
        let first = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 0));
        let later = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 1));
        let target = obj::Owned::fresh(crate::interp::new_string(b"target"));
        let unused = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 3));
        let pointers = [first.as_ptr(), later.as_ptr()];
        assert!(interpreter.begin_ensemble_rewrite(vec![first, later], 2, 1));
        assert_eq!(
            interpreter
                .argument_usage_prefix(&[target.as_ptr(), unused.as_ptr()], 1)
                .unwrap(),
            b"original later"
        );
        assert_eq!(GETS.with(|value| *value.borrow()), [1, 1, 0, 0]);
        assert_eq!(REFS.with(|value| *value.borrow()), [1, 1, 0, 0]);
        assert_eq!(unsafe { (*pointers[0]).ref_count }, 1);
        assert_eq!(unsafe { (*pointers[1]).ref_count }, 1);
        assert!(!obj::has_string_rep(unused.as_ptr()));
        interpreter.clear_ensemble_rewrite_checked().unwrap();
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_usage_first_getter_refusal_stops_later_header_and_guest_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [1, 2] {
        let mut interpreter = native("tcl8.6");
        let _tls = OriginalTls::enter(&mut interpreter, mode);
        let first = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 0));
        let later = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 1));
        let prior = prior_result(&mut interpreter);
        assert_eq!(
            interpreter.wrong_args_for_prefix(&[first.as_ptr(), later.as_ptr()], 2, b"value"),
            Code::Error
        );
        assert_eq!(GETS.with(|value| *value.borrow()), [1, 0, 0, 0]);
        assert_eq!(REFS.with(|value| value.borrow()[0]), 1);
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert_prior(&interpreter, prior);
        assert!(interpreter.native_execution_refusal().is_some());
        if mode == 1 {
            assert!(matches!(
                interpreter.native_execution_refusal(),
                Some(NativeExecutionError::HostCommandRefusal(_))
            ));
        }
        assert!(NESTED.with(Cell::get));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_procedure_argv_zero_refusal_precedes_real_body_getter_and_activation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [1, 2] {
        let mut interpreter = native("tcl8.6");
        let _tls = OriginalTls::enter(&mut interpreter, mode);
        let body = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 1));
        let owner =
            interpreter.create_native_callable_from_chosen(Vec::new(), body, GLOBAL, None, 0);
        let declaration = owner.declaration();
        let original_body = declaration.body.checked_ptr().unwrap();
        let called = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 0));
        let prior = prior_result(&mut interpreter);
        let depth = interpreter.cmd_frame_depth();
        assert_eq!(
            interpreter.call_proc(&declaration, &[called.as_ptr()]),
            Code::Error
        );
        assert_eq!(GETS.with(|value| *value.borrow()), [1, 0, 0, 0]);
        assert!(!obj::has_string_rep(original_body));
        assert_eq!(unsafe { (*original_body).ref_count }, 1);
        assert_prior(&interpreter, prior);
        assert_eq!(interpreter.cmd_frame_depth(), depth);
        assert_eq!(interpreter.current_ns(), GLOBAL);
        assert!(!interpreter.in_proc());
        assert!(interpreter.native_execution_refusal().is_some());
    }
}

#[test]
fn original_usage_rewrite_birth_refuses_nested_and_clear_recreate_aba() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [3, 4] {
        let mut interpreter = native("tcl8.6");
        let _tls = OriginalTls::enter(&mut interpreter, mode);
        let first = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 0));
        let later = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 1));
        assert!(interpreter.begin_ensemble_rewrite(vec![first, later], 2, 1));
        let original = OriginalArgumentUsage::capture(&interpreter, &[], 0, false).unwrap();
        assert!(original.procedure_prefix(0, 1).is_err());
        assert_eq!(GETS.with(|value| *value.borrow()), [1, 0, 0, 0]);
        assert_eq!(REFS.with(|value| value.borrow()[0]), 1);
        assert!(interpreter.native_execution_refusal().is_some());
        assert!(interpreter.clear_ensemble_rewrite_checked().is_err());
        assert!(interpreter.ensemble_rewrite.borrow().is_none());
    }
}

#[test]
fn original_rewrite_retirement_reenters_extracted_owner_and_keeps_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [0, 1, 2] {
        let mut interpreter = native("tcl8.6");
        let _tls = OriginalTls::enter(&mut interpreter, mode);
        let word = obj::Owned::fresh(obj::alloc_typed(&RETIRE, 0));
        let pointer = word.as_ptr();
        assert!(interpreter.begin_ensemble_rewrite(vec![word], 1, 1));
        let projection = OriginalArgumentUsage::capture(&interpreter, &[], 0, false).unwrap();
        let prior = prior_result(&mut interpreter);
        let cleared = interpreter.clear_ensemble_rewrite_checked();
        assert_eq!(cleared.is_err(), mode != 0);
        assert_eq!(FREES.with(Cell::get), 1);
        assert!(BORROWS.with(Cell::get));
        assert!(EMPTY.with(Cell::get));
        assert!(NESTED.with(Cell::get));
        assert!(obj::check_native_liveness(pointer).is_err());
        assert!(projection.check().is_err());
        assert_prior(&interpreter, prior);
        assert!(interpreter.ensemble_rewrite.borrow().is_none());
    }
}

#[test]
fn original_handler_prefix_retirement_reenters_after_extraction_before_success() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    fn release_caller(_: &mut Interp, _: &[*mut TclObj]) -> Code {
        let caller = CALLER.with(|value| value.borrow_mut().take());
        drop(caller);
        Code::Ok
    }
    for mode in [0, 1, 2] {
        let mut interpreter = native("tcl8.6");
        let _tls = OriginalTls::enter(&mut interpreter, mode);
        let caller = obj::Owned::fresh(obj::alloc_typed(&RETIRE, 1));
        let pointer = caller.as_ptr();
        CALLER.with(|value| *value.borrow_mut() = Some(caller));
        let prior = prior_result(&mut interpreter);
        assert_eq!(
            interpreter.invoke_stock_worker(&[pointer], &[b"parser"], release_caller),
            if mode == 0 { Code::Ok } else { Code::Error }
        );
        assert_eq!(FREES.with(Cell::get), 1);
        assert!(BORROWS.with(Cell::get));
        assert!(EMPTY.with(Cell::get));
        assert!(NESTED.with(Cell::get));
        assert!(interpreter.handler_usage_adapters.borrow().is_empty());
        assert_prior(&interpreter, prior);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        assert_eq!(interpreter.native_execution_refusal().is_some(), mode != 0);
    }
}

#[test]
fn original_ensemble_arity_and_selector_refusals_precede_target_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for arity in [true, false] {
        let mut interpreter = native("tcl8.6");
        let configuration = crate::ensemble::EnsembleConfig {
            originals: crate::ensemble::NativeEnsembleObjects::default(),
            ns: GLOBAL,
            map: None,
            subcommands: None,
            prefixes: true,
            parameters: if arity {
                vec![b"parameter".to_vec()]
            } else {
                Vec::new()
            },
            unknown: Vec::new(),
        };
        interpreter.create_ensemble_in_slot(GLOBAL, b"consumerOriginalUsage", configuration);
        let token = match interpreter
            .namespaces()
            .command_in(GLOBAL, b"consumerOriginalUsage")
        {
            Some(crate::interp::Command::Ensemble(token)) => token,
            _ => panic!("actual ensemble publication"),
        };
        let _tls = OriginalTls::enter(&mut interpreter, 1);
        let reached = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 0));
        let later = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 1));
        let plain = obj::Owned::fresh(crate::interp::new_string(b"consumerOriginalUsage"));
        let prior = prior_result(&mut interpreter);
        let count = interpreter.cmd_count.get();
        let arguments = if arity {
            vec![reached.as_ptr()]
        } else {
            vec![plain.as_ptr(), reached.as_ptr(), later.as_ptr()]
        };
        assert_eq!(
            interpreter.dispatch_ensemble(&token, &arguments),
            Code::Error
        );
        assert_eq!(GETS.with(|value| *value.borrow()), [1, 0, 0, 0]);
        assert_eq!(REFS.with(|value| value.borrow()[0]), 1);
        assert_eq!(interpreter.cmd_count.get(), count);
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert_prior(&interpreter, prior);
        assert!(matches!(
            interpreter.native_execution_refusal(),
            Some(NativeExecutionError::HostCommandRefusal(_))
        ));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_procedure_body_getter_refusal_keeps_real_declaration_and_frame_cleanup() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [5, 6] {
        let mut interpreter = native("tcl8.6");
        let _tls = OriginalTls::enter(&mut interpreter, mode);
        let body = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 1));
        let owner =
            interpreter.create_native_callable_from_chosen(Vec::new(), body, GLOBAL, None, 0);
        let declaration = owner.declaration();
        let original_body = declaration.body.checked_ptr().unwrap();
        let called = obj::Owned::fresh(crate::interp::new_string(b"called"));
        let prior = prior_result(&mut interpreter);
        let depth = interpreter.cmd_frame_depth();
        assert_eq!(
            interpreter.call_proc(&declaration, &[called.as_ptr()]),
            Code::Error
        );
        assert_eq!(GETS.with(|value| *value.borrow()), [0, 1, 0, 0]);
        assert_eq!(REFS.with(|value| value.borrow()[1]), 1);
        assert!(obj::has_string_rep(original_body));
        assert_eq!(unsafe { (*original_body).ref_count }, 1);
        assert!(declaration.check_native_liveness().is_ok());
        assert_prior(&interpreter, prior);
        assert_eq!(interpreter.cmd_frame_depth(), depth);
        assert_eq!(interpreter.current_ns(), GLOBAL);
        assert!(!interpreter.in_proc());
        assert!(interpreter.native_execution_refusal().is_some());
        assert!(NESTED.with(Cell::get));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_usage_actual_receiver_refuses_foreign_interpreter_before_getter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let original = native("tcl8.6");
    let mut foreign = native("tcl8.6");
    assert_eq!(original.runtime_context(), foreign.runtime_context());
    let _tls = OriginalTls::enter(&mut foreign, 0);
    let word = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 0));
    let projection = OriginalArgumentUsage::capture(&original, &[word.as_ptr()], 1, false).unwrap();
    let protocol = original
        .native_invocation_dialect()
        .usage_protocol(None)
        .unwrap();
    let _foreign_scope = NativeOperationScope::enter(&foreign).unwrap();
    let prior = prior_result(&mut foreign);
    assert!(projection.render_header(&mut foreign, protocol).is_err());
    assert_eq!(GETS.with(|value| *value.borrow()), [0; 4]);
    assert!(!obj::has_string_rep(word.as_ptr()));
    assert_eq!(unsafe { (*word.as_ptr()).ref_count }, 1);
    assert_prior(&foreign, prior);
    assert!(matches!(
        foreign.native_execution_refusal(),
        Some(NativeExecutionError::ValueAccessRefusal(_))
    ));
    assert!(projection.check().is_ok());
    assert!(original.native_execution_refusal().is_none());
    assert_eq!(original.entered_native_operation_count(), 1);
}
