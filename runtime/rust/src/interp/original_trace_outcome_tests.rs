// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual callback outcomes and entered-operation currency, separate from
//! external Tcl callback chronology and parser observations.

use super::*;
use tcl_runtime_api::{NativeExecutionError, Traces};
use tcl_syntax::raw_string::NativeValueAccessRefusal;

thread_local! {
    static FIRST_CALLS: Cell<usize> = const { Cell::new(0) };
    static LATER_CALLS: Cell<usize> = const { Cell::new(0) };
}

fn native(name: &str) -> Interp {
    let environment = tcl_registry::model::ingress::resolve_known_environment(name)
        .expect("actual known environment ingress");
    Interp::with_native_core(
        default_host(),
        environment.unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("genuine selected native bootstrap")
}

fn first_host(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    FIRST_CALLS.with(|calls| calls.set(calls.get() + 1));
    interpreter.refuse_host_command("original typed variable trace refusal")
}

fn changed_world(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    FIRST_CALLS.with(|calls| calls.set(calls.get() + 1));
    let original = interpreter.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("original-typed-trace".into(), "1.0".into())];
    interpreter.pin_context(&changed).unwrap();
    interpreter.pin_context(&original).unwrap();
    Code::Ok
}

fn later(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    LATER_CALLS.with(|calls| calls.set(calls.get() + 1));
    interpreter.set_result_bytes(b"LATER CALLBACK MUST NOT RUN");
    Code::Ok
}

fn register(interpreter: &mut Interp, source: &[u8], first: BuiltinFn) {
    interpreter.register_builtin(b"original_first", first);
    interpreter.register_builtin(b"original_later", later);
    assert_eq!(interpreter.eval_str(source), Code::Ok);
    FIRST_CALLS.with(|calls| calls.set(0));
    LATER_CALLS.with(|calls| calls.set(0));
}

fn stale() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::CommandProtocolUnavailable(
        "stale entered native operation",
    ))
}

#[test]
fn original_trace_facades_keep_the_complete_host_cause_outside_guest_values() {

    // Actual selected trace callbacks and full software Host payload, not a provider chronology measurement.
    for unicode in [false, true] {
        let mut interpreter = native("tcl8.6");
        register(
            &mut interpreter,
            b"set x original; trace add variable x read original_later; trace add variable x read original_first",
            first_host,
        );
        interpreter.set_result_bytes(b"ORIGINAL RESULT");
        let prior = interpreter.get_obj_result();
        let first = if unicode {
            Traces::fire(&mut interpreter, "x", "read")
        } else {
            Traces::fire_bytes(&mut interpreter, b"x", "read")
        }
        .expect_err("the actual Host refusal is outside the callback's Guest value");
        assert_eq!(interpreter.native_execution_refusal(), Some(first.clone()));
        let NativeExecutionError::HostCommandRefusal(payload) = first else {
            panic!("the complete actual Host command cause must survive the facade")
        };
        assert_eq!(payload.reason, "original typed variable trace refusal");
        assert_eq!(FIRST_CALLS.with(Cell::get), 1);
        assert_eq!(LATER_CALLS.with(Cell::get), 0);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert!(interpreter.traces.borrow().pending_err.is_none());
        assert!(interpreter.active_var_trace_scopes.borrow().is_empty());
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_trace_read_update_and_array_wrappers_refuse_before_guest_wrapping() {

    // The real changed-and-restored context invalidates the owned callback stage; no mutable receipt is fabricated.
    for route in 0..4 {
        let mut interpreter = native("tcl8.6");
        register(
            &mut interpreter,
            if route == 3 {
                b"array set x {k original}; trace add variable x array original_later; trace add variable x array original_first"
            } else {
                b"set x original; trace add variable x read original_later; trace add variable x read original_first"
            },
            changed_world,
        );
        interpreter.set_result_bytes(b"ORIGINAL RESULT");
        let prior = interpreter.get_obj_result();
        match route {
            0 => assert_eq!(interpreter.fire_read_trace(b"x", None), Some(Code::Error)),
            1 => assert_eq!(interpreter.read_for_update(b"x", None), Err(stale())),
            2 => assert_eq!(interpreter.lappend_read(b"x", None), Err(stale())),
            _ => assert_eq!(interpreter.fire_array_trace(b"x"), Some(Code::Error)),
        }
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        assert_eq!(FIRST_CALLS.with(Cell::get), 1);
        assert_eq!(LATER_CALLS.with(Cell::get), 0);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert!(interpreter.traces.borrow().pending_err.is_none());
        assert!(interpreter.active_var_trace_scopes.borrow().is_empty());
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_c84_trace_refusal_keeps_reached_header_and_settles_the_real_flag() {

    // Software C8.4 global setter ownership under the actual callback; external publication-order observations remain separate.
    let mut interpreter = native("tcl8.4");
    register(
        &mut interpreter,
        b"trace add variable ::errorCode write original_later; trace add variable ::errorCode write original_first",
        first_host,
    );
    interpreter.set_result_bytes(b"ORIGINAL RESULT");
    let prior = interpreter.get_obj_result();
    let code = obj::Owned::fresh(new_string(b"ORIGINAL\0\xff CODE"));
    let counter = interpreter.c_api_errors.get();
    let first = interpreter
        .set_original_c_error_code(code.as_ptr())
        .expect_err("the actual write callback refused after publication");
    assert_eq!(interpreter.native_execution_refusal(), Some(first.clone()));
    assert!(matches!(first, NativeExecutionError::HostCommandRefusal(_)));
    assert_eq!(FIRST_CALLS.with(Cell::get), 1);
    assert_eq!(LATER_CALLS.with(Cell::get), 0);
    assert_eq!(
        crate::vars::get_at(
            &interpreter.frames.borrow(),
            &interpreter.namespaces.borrow(),
            b"::errorCode",
            0,
        ),
        Some(code.as_ptr())
    );
    assert_eq!(unsafe { (*code.as_ptr()).ref_count }, 2);
    assert!(interpreter.exc.borrow().native.global_code_set);
    assert_eq!(interpreter.c_api_errors.get(), counter);
    assert_eq!(interpreter.get_obj_result(), prior);
    assert!(interpreter.traces.borrow().pending_err.is_none());
    assert!(interpreter.active_var_trace_scopes.borrow().is_empty());
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}
