// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual selected method entry and original Next callback controls.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use std::cell::Cell;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::value::ValueError;
thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static HEADS: Cell<usize> = const { Cell::new(0) };
    static CLASSES: Cell<usize> = const { Cell::new(0) };
    static RETIREMENTS: Cell<usize> = const { Cell::new(0) };
    static RETIRED_REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
}
extern "C" fn head(original: *mut TclObj) {
    HEADS.with(|count| count.set(count.get() + 1));
    let interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    if MODE.with(Cell::get) == 1 {
        // Legal software change before context selection: same variable frame
        // and actual entered chain, a different current step at late entry.
        let mut state = interpreter.oo.borrow_mut();
        let frame = state.call_stack.last_mut().unwrap();
        assert!(frame.chain.get(1).is_some());
        frame.index = 1;
    }
    unsafe {
        obj::set_native_updater_string_rep(original, b"next", false);
    }
}
extern "C" fn class_name(original: *mut TclObj) {
    CLASSES.with(|count| count.set(count.get() + 1));
    let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    match MODE.with(Cell::get) {
        2 | 5 => {
            let retired = {
                let mut state = interpreter.oo.borrow_mut();
                let frame = state.call_stack.last_mut().unwrap();
                std::mem::replace(&mut frame.name_owner, Rc::new(RefCell::new(None)))
            };
            drop(retired);
        }
        3 | 6 => {
            interpreter.refuse_host_command("original Next class getter");
        }
        _ => {}
    }
    unsafe {
        obj::set_native_updater_string_rep(original, b"Base", false);
    }
}
extern "C" fn class_free(original: *mut TclObj) {
    RETIREMENTS.with(|count| count.set(count.get() + 1));
    RETIRED_REFS.with(|refs| refs.set(unsafe { (*original).ref_count }));
    if MODE.with(Cell::get) == 4 {
        let interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        let retired = {
            let mut state = interpreter.oo.borrow_mut();
            let frame = state.call_stack.last_mut().unwrap();
            std::mem::replace(&mut frame.name_owner, Rc::new(RefCell::new(None)))
        };
        drop(retired);
    }
}
static HEAD: obj::TclObjType = obj::TclObjType {
    name: c"originalNextHead".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(head),
    set_from_any_proc: None,
};
static CLASS: obj::TclObjType = obj::TclObjType {
    name: c"originalNextClass".as_ptr(),
    free_int_rep_proc: Some(class_free),
    dup_int_rep_proc: None,
    update_string_proc: Some(class_name),
    set_from_any_proc: None,
};
fn selected(interpreter: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    let original = OriginalArguments::enter(interpreter, &[]).unwrap();
    let head = obj::Owned::fresh(obj::alloc_typed(&HEAD, 0));
    let class = obj::Owned::fresh(obj::alloc_typed(&CLASS, 0));
    if MODE.with(Cell::get) == 4 {
        unsafe {
            obj::set_native_updater_string_rep(class.as_ptr(), b"Base", false);
        }
    }
    let version = interpreter.native_invocation_dialect().tcl_version.unwrap();
    let class_mode = MODE.with(Cell::get) >= 2;
    let words = if class_mode {
        vec![head.as_ptr(), class.as_ptr()]
    } else {
        vec![head.as_ptr()]
    };
    let headers = OriginalArguments::enter(interpreter, &words).unwrap();
    interpreter.set_result_bytes(b"PRIOR");
    let current = |actual: &Interp| {
        original.check_for(actual)?;
        headers.check_for(actual)
    };
    let code = if MODE.with(Cell::get) >= 5 {
        nextto_cmd(interpreter, &words)
    } else {
        interpreter.execute_native_oo_next(
            &words,
            class_mode,
            version,
            &current,
            &|actual, head| {
                actual.native_object_string_bytes_using(head, &|| current(actual), &|error| {
                    original.retain_refusal(error.into())
                })
            },
        )
    };
    if MODE.with(Cell::get) >= 2 {
        assert_eq!(code, Code::Error);
        assert_eq!(interpreter.result_bytes(), b"PRIOR");
        assert!(obj::native_command_name_cache(class.as_ptr()).is_none());
        assert_eq!(
            RETIREMENTS.with(Cell::get),
            usize::from(MODE.with(Cell::get) == 4)
        );
        if MODE.with(Cell::get) == 4 {
            assert_eq!(RETIRED_REFS.with(Cell::get), 1);
        }
        let first = interpreter.native_execution_refusal().unwrap();
        if matches!(MODE.with(Cell::get), 3 | 6) {
            assert!(
                matches!(first, NativeExecutionError::HostCommandRefusal(reason)
                if reason.reason == "original Next class getter")
            );
        } else {
            assert_eq!(
                first,
                tcl_cmd_core::CmdError::from(ValueError::CommandProtocolUnavailable(
                    "original selected Next context or chain",
                ))
                .native_execution_refusal()
                .unwrap()
                .clone()
            );
        }
    }
    drop(class);
    drop(head);
    original.finish(code)
}
fn interpreter(dialect: &str, mode: u8) -> Interp {
    let mut interpreter = fixture::native(dialect);
    MODE.with(|selected| selected.set(mode));
    HEADS.with(|count| count.set(0));
    CLASSES.with(|count| count.set(0));
    RETIREMENTS.with(|count| count.set(0));
    RETIRED_REFS.with(|refs| refs.set(-1));
    interpreter.register_builtin(b"selected_next", selected);
    assert_eq!(
        interpreter.eval_str(
            br"oo::class create Base {method m {} {return BASE}}
        oo::class create Mid {superclass Base; method m {} {return MID}}
        oo::class create Leaf {superclass Mid; method m {} {selected_next}}
        Leaf create subject"
        ),
        Code::Ok
    );
    interpreter
}
#[test]
fn original_next_head_selects_current_context_only_after_c91_string() {
    let mut interpreter = interpreter("tcl9.1", 1);
    let _current = fixture::install(&mut interpreter, 0);
    let originals = OriginalArguments::enter(&interpreter, &[]).unwrap();
    assert_eq!(interpreter.eval_str(b"subject m"), Code::Ok);
    assert_eq!(interpreter.result_bytes(), b"BASE");
    assert_eq!(HEADS.with(Cell::get), 1);
    assert_eq!(CLASSES.with(Cell::get), 0);
    originals.check().unwrap();
}
#[test]
fn original_next_earlier_compiled_route_does_not_read_the_helper_head() {
    for dialect in ["tcl8.6", "tcl9.0"] {
        let mut interpreter = interpreter(dialect, 1);
        let _current = fixture::install(&mut interpreter, 0);
        let originals = OriginalArguments::enter(&interpreter, &[]).unwrap();
        assert_eq!(interpreter.eval_str(b"subject m"), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"MID");
        assert_eq!(HEADS.with(Cell::get), 0);
        assert_eq!(CLASSES.with(Cell::get), 0);
        originals.check().unwrap();
    }
}
#[test]
fn original_nextto_selected_context_withdrawal_stops_command_cache_adoption() {
    for mode in [2, 3] {
        let mut interpreter = interpreter("tcl9.1", mode);
        let _current = fixture::install(&mut interpreter, 0);
        let originals = OriginalArguments::enter(&interpreter, &[]).unwrap();
        assert_eq!(interpreter.eval_str(b"subject m"), Code::Error);
        assert_eq!(HEADS.with(Cell::get), 1);
        assert_eq!(CLASSES.with(Cell::get), 1);
        assert!(originals.check().is_err());
    }
}
fn receipt(interpreter: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    let steps = interpreter
        .oo
        .borrow()
        .call_stack
        .last()
        .unwrap()
        .chain
        .observed_steps();
    let strong = steps.strong_count();
    let selected = OriginalOoNextContext::capture(interpreter)
        .unwrap()
        .unwrap();
    assert_eq!(steps.strong_count(), strong);
    {
        let mut state = interpreter.oo.borrow_mut();
        let frame = state.call_stack.last_mut().unwrap();
        frame.index = 1;
        frame.target = b"current-payload".to_vec();
    }
    selected.check(interpreter).unwrap();
    assert_eq!(steps.strong_count(), strong);
    interpreter.set_result_bytes(b"WEAK_ONLY");
    Code::Ok
}
#[test]
fn original_next_receipt_observes_the_context_without_holding_method_roles_or_freezing_payload() {
    let mut interpreter = interpreter("tcl9.1", 0);
    let _current = fixture::install(&mut interpreter, 0);
    interpreter.register_builtin(b"selected_next", receipt);
    assert_eq!(interpreter.eval_str(b"subject m"), Code::Ok);
    assert_eq!(interpreter.result_bytes(), b"WEAK_ONLY");
}
#[test]
fn original_next_missing_context_reports_only_after_its_selected_head_stage() {
    for (dialect, expected) in [("tcl8.6", 0), ("tcl9.0", 0), ("tcl9.1", 1)] {
        let mut interpreter = fixture::native(dialect);
        let _current = fixture::install(&mut interpreter, 0);
        MODE.with(|selected| selected.set(0));
        HEADS.with(|count| count.set(0));
        let originals = OriginalArguments::enter(&interpreter, &[]).unwrap();
        let head = obj::Owned::fresh(obj::alloc_typed(&HEAD, 0));
        let version = interpreter.native_invocation_dialect().tcl_version.unwrap();
        assert_eq!(
            interpreter.execute_native_oo_next(
                &[head.as_ptr()],
                false,
                version,
                &|actual| originals.check_for(actual),
                &|actual, original| actual.native_object_string_bytes_using(
                    original,
                    &|| originals.check_for(actual),
                    &|error| originals.retain_refusal(error.into())
                )
            ),
            Code::Error
        );
        assert_eq!(
            interpreter.result_bytes(),
            b"next may only be called from inside a method"
        );
        assert_eq!(interpreter.error_code(), b"TCL OO CONTEXT_REQUIRED");
        assert_eq!(HEADS.with(Cell::get), expected);
        assert_eq!(unsafe { (*head.as_ptr()).ref_count }, 1);
        assert!(interpreter.native_execution_refusal().is_none());
        originals.check().unwrap();
    }
}

#[test]
fn original_nextto_cache_retirement_checks_the_selected_context_before_adoption() {
    let mut interpreter = interpreter("tcl9.1", 4);
    let _current = fixture::install(&mut interpreter, 0);
    let originals = OriginalArguments::enter(&interpreter, &[]).unwrap();
    assert_eq!(interpreter.eval_str(b"subject m"), Code::Error);
    assert_eq!(HEADS.with(Cell::get), 1);
    assert_eq!(CLASSES.with(Cell::get), 0);
    assert_eq!(RETIREMENTS.with(Cell::get), 1);
    assert_eq!(RETIRED_REFS.with(Cell::get), 1);
    assert!(originals.check().is_err());
}

#[test]
fn original_ordinary_nextto_uses_its_actual_c_command_head_and_class_stage_order() {
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [5, 6] {
            let mut interpreter = interpreter(dialect, mode);
            let _current = fixture::install(&mut interpreter, 0);
            let originals = OriginalArguments::enter(&interpreter, &[]).unwrap();
            assert_eq!(interpreter.eval_str(b"subject m"), Code::Error);
            assert_eq!(HEADS.with(Cell::get), usize::from(dialect == "tcl9.1"));
            assert_eq!(CLASSES.with(Cell::get), 1);
            assert!(originals.check().is_err());
        }
    }
}
#[test]
fn original_ordinary_nextto_missing_context_reads_the_diagnostic_head_once() {
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = fixture::native(dialect);
        let _current = fixture::install(&mut interpreter, 0);
        MODE.with(|selected| selected.set(0));
        HEADS.with(|count| count.set(0));
        let originals = OriginalArguments::enter(&interpreter, &[]).unwrap();
        let head = obj::Owned::fresh(obj::alloc_typed(&HEAD, 0));
        assert_eq!(nextto_cmd(&mut interpreter, &[head.as_ptr()]), Code::Error);
        assert_eq!(
            interpreter.result_bytes(),
            b"next may only be called from inside a method"
        );
        assert_eq!(interpreter.error_code(), b"TCL OO CONTEXT_REQUIRED");
        assert_eq!(HEADS.with(Cell::get), 1);
        assert!(interpreter.native_execution_refusal().is_none());
        originals.check().unwrap();
    }
}
