// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software invocation-owner controls using actual VM callbacks and cleanup.

use super::*;
use crate::command::NativeCommand;
use std::cell::Cell;

fn native() -> Vm {
    crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl8.6"))
}

struct Observe {
    calls: Rc<Cell<usize>>,
    withdraw: bool,
    guest_error: bool,
}
impl NativeCommand for Observe {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        NativeOperationScope::current_entered(vm)
            .unwrap()
            .expect("the actual invocation is entered");
        self.calls.set(self.calls.get() + 1);
        if self.withdraw {
            vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
        }
        if self.guest_error {
            err("GUEST")
        } else {
            ok(Value::string("ANSWER"))
        }
    }
}
fn register(vm: &mut Vm, name: &str, withdraw: bool, guest_error: bool) -> Rc<Cell<usize>> {
    let calls = Rc::new(Cell::new(0));
    vm.register_native_command(
        name,
        Rc::new(Observe {
            calls: calls.clone(),
            withdraw,
            guest_error,
        }),
    );
    calls
}
fn retained_first(
    vm: &mut Vm,
    original: &NativeOperationScope,
    first: &tcl_runtime_api::NativeExecutionError,
) {
    assert_eq!(vm.execution_refusal.as_ref(), Some(first));
    assert_eq!(original.check_native(vm).unwrap_err(), *first);
    assert_eq!(vm.current_level(), 0);
    assert_eq!(vm.current_ns_id(), tcl_core_types::ROOT_NS);
    assert!(vm.pending_exec_leave.is_none());
    assert!(vm.exec_step_scopes.is_empty());
}

#[test]
fn original_name_dispatch_keeps_direct_success_and_guest_error_separate() {

    // Actual closed VM values and software callbacks; no external extension issuer.
    for guest_error in [false, true] {
        let mut vm = native();
        let calls = register(&mut vm, "observe", false, guest_error);
        let completion = vm.try_invoke_command("observe", &[]).unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(
            completion.code,
            if guest_error { Code::Error } else { Code::Ok }
        );
        assert_eq!(
            completion.result.resident_string_bytes().unwrap().as_ref(),
            if guest_error {
                b"GUEST".as_slice()
            } else {
                b"ANSWER".as_slice()
            }
        );
        assert!(vm.execution_refusal.is_none());
        assert!(
            NativeOperationScope::current_entered(&mut vm)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn original_name_dispatch_checks_enter_and_leave_before_later_work() {

    for operation in ["enter", "leave"] {
        let mut vm = native();
        let work = register(&mut vm, "work", false, false);
        let observer = register(&mut vm, "observer", true, false);
        let later = register(&mut vm, "later", false, false);
        let setup = vm
            .try_eval_source(&format!("trace add execution work {operation} observer"))
            .unwrap();
        assert_eq!(setup.code, Code::Ok);
        let original = NativeOperationScope::enter(&mut vm).unwrap();
        let first = vm.try_eval_source("work; later").unwrap_err();
        assert_eq!(observer.get(), 1);
        assert_eq!(work.get(), usize::from(operation == "leave"));
        assert_eq!(later.get(), 0);
        retained_first(&mut vm, &original, &first);
    }
}

#[test]
fn original_name_dispatch_retains_deferred_body_scope_through_refused_cleanup() {

    for body in [
        "deferred",
        "eval {withdraw; later}",
        "catch {withdraw; later} message; later",
    ] {
        let mut vm = native();
        let withdraw = register(&mut vm, "withdraw", true, false);
        let later = register(&mut vm, "later", false, false);
        let setup = vm
            .try_eval_source("proc deferred {} {withdraw; later}")
            .unwrap();
        assert_eq!(setup.code, Code::Ok);
        let original = NativeOperationScope::enter(&mut vm).unwrap();
        let first = vm.try_eval_source(body).unwrap_err();
        assert_eq!(withdraw.get(), 1);
        assert_eq!(later.get(), 0);
        retained_first(&mut vm, &original, &first);
    }
}

#[test]
fn original_name_dispatch_carries_tailcall_owner_into_replacement() {

    for withdraw in [false, true] {
        let mut vm = native();
        let target = register(&mut vm, "target", withdraw, false);
        let setup = vm
            .try_eval_source("proc replace {} {tailcall target}")
            .unwrap();
        assert_eq!(setup.code, Code::Ok);
        let original = NativeOperationScope::enter(&mut vm).unwrap();
        let reached = vm.try_invoke_command("replace", &[]);
        assert_eq!(target.get(), 1);
        if withdraw {
            retained_first(&mut vm, &original, &reached.unwrap_err());
        } else {
            let completion = reached.unwrap();
            assert_eq!(completion.code, Code::Ok);
            assert_eq!(
                completion.result.resident_string_bytes().unwrap().as_ref(),
                b"ANSWER"
            );
            original.check(&mut vm).unwrap();
            assert_eq!(vm.current_level(), 0);
            assert!(vm.pending_exec_leave.is_none());
        }
    }
}

#[test]
fn original_name_dispatch_keeps_parked_coroutine_receipt_without_reissuing_it() {

    for change in 0..3 {
        let mut vm = native();
        let target = register(&mut vm, "target", false, false);
        let unrelated = register(&mut vm, "unrelated", false, false);
        let suspended = vm
            .try_eval_source("coroutine suspended apply {{} {yield READY; target}}")
            .unwrap();
        assert_eq!(suspended.code, Code::Ok);
        assert_eq!(
            suspended.result.resident_string_bytes().unwrap().as_ref(),
            b"READY"
        );
        assert_eq!(target.get(), 0);
        assert!(
            NativeOperationScope::current_entered(&mut vm)
                .unwrap()
                .is_none()
        );
        let original_host = vm.host_rc();
        if change != 0 {
            vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
            if change == 2 {
                vm.set_host(original_host);
            }
        }
        let fresh = vm.try_invoke_command("unrelated", &[]).unwrap();
        assert_eq!(fresh.code, Code::Ok);
        assert_eq!(unrelated.get(), 1);
        assert!(
            NativeOperationScope::current_entered(&mut vm)
                .unwrap()
                .is_none()
        );
        let resumed = vm.try_invoke_command("suspended", &[]);
        if change != 0 {
            assert!(matches!(
                resumed,
                Err(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "entered VM original execution context"
                    )
                ))
            ));
            assert_eq!(target.get(), 0);
            assert_eq!(vm.current_level(), 0);
        } else {
            let completion = resumed.unwrap();
            assert_eq!(completion.code, Code::Ok);
            assert_eq!(
                completion.result.resident_string_bytes().unwrap().as_ref(),
                b"ANSWER"
            );
            assert_eq!(target.get(), 1);
        }
    }
}

#[test]
fn original_name_dispatch_refuses_retired_head_before_lookup_or_guest_error() {

    let mut vm = native();
    let original = NativeOperationScope::enter(&mut vm).unwrap();
    let head = Value::string("unregistered");
    let loan = head.native_lifetime_lease();
    drop(head);
    assert!(!loan.value().native_object_is_live());
    let completion = vm.invoke_host_original_object_vector(loan.value(), &[]);
    let first = vm.finish_host_execution(completion).unwrap_err();
    assert_eq!(
        first,
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "retired native object header"
            )
        )
    );
    retained_first(&mut vm, &original, &first);
    assert!(loan.value().resident_string_bytes().is_some());
}

#[test]
fn original_return_code_branch_uses_selected_integer_and_keeps_guest_table() {

    // A fixed software bytecode specimen exercises the real opcode, not compiler admission.
    for code in [1, 2, 3, 4, 5, 17] {
        let mut vm = native();
        let mut branch = Instruction::new(Op::RETURN_CODE_BRANCH, vec![]);
        branch.offset = 0;
        let mut instructions = vec![branch];
        for slot in 1..=5 {
            let mut jump = Instruction::new(Op::JUMP1, vec![]);
            jump.offset = 2 * slot - 1;
            instructions.push(jump);
        }
        let asm = FunctionAsm {
            instructions,
            ..FunctionAsm::default()
        };
        let unit = vm.compiled_unit(Rc::new(asm), vm.source_namespace_path());
        let mut frame = Frame::new(unit, false);
        frame.stack.push(Value::int(code));
        assert!(matches!(vm.tick(&mut frame), Tick::Continue));
        assert_eq!(
            frame.pc,
            usize::try_from(if code <= 4 { code } else { 5 }).unwrap()
        );
        assert!(frame.stack.is_empty());
        assert!(vm.execution_refusal.is_none());
    }
    for (value, message) in [
        (
            Value::string("not-a-code"),
            b"returnCodeBranch: TOS not a return code".as_slice(),
        ),
        (Value::int(0), b"returnCodeBranch: TOS is TCL_OK".as_slice()),
    ] {
        let mut vm = native();
        let mut branch = Instruction::new(Op::RETURN_CODE_BRANCH, vec![]);
        branch.offset = 0;
        let unit = vm.compiled_unit(
            Rc::new(FunctionAsm {
                instructions: vec![branch],
                ..FunctionAsm::default()
            }),
            vm.source_namespace_path(),
        );
        let mut frame = Frame::new(unit, false);
        frame.stack.push(value);
        let Tick::Return(completion) = vm.tick(&mut frame) else {
            panic!("a Guest return-code error must keep its completion");
        };
        assert_eq!(completion.code, Code::Error);
        assert_eq!(
            completion.result.resident_string_bytes().unwrap().as_ref(),
            message
        );
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn original_return_code_branch_stops_retired_operand_before_jump() {

    let mut vm = native();
    let original = NativeOperationScope::enter(&mut vm).unwrap();
    let value = Value::int(1);
    let loan = value.native_lifetime_lease();
    drop(value);
    let mut branch = Instruction::new(Op::RETURN_CODE_BRANCH, vec![]);
    branch.offset = 0;
    let unit = vm.compiled_unit(
        Rc::new(FunctionAsm {
            instructions: vec![branch],
            ..FunctionAsm::default()
        }),
        vm.source_namespace_path(),
    );
    let mut frame = Frame::new(unit, false);
    frame.stack.push(loan.into_value());
    let Tick::Return(completion) = vm.tick(&mut frame) else {
        panic!("withdrawal must precede the jump table");
    };
    let first = vm.finish_host_execution(completion).unwrap_err();
    assert_eq!(
        first,
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "retired native object header"
            )
        )
    );
    assert_eq!(frame.pc, 1);
    assert_eq!(original.check_native(&mut vm).unwrap_err(), first);
}

#[test]
fn original_name_dispatch_parks_caught_tailcall_owner_before_unrelated_entry() {

    // A caught TCL_RETURN retains the genuine request on the actual procedure frame.
    let mut vm = native();
    let target = register(&mut vm, "target", false, false);
    let unrelated = register(&mut vm, "unrelated", false, false);
    let parked = vm
        .try_eval_source(
            "coroutine pending apply {{} {catch {tailcall target}; yield READY; return END}}",
        )
        .unwrap();
    assert_eq!(parked.code, Code::Ok);
    assert_eq!(
        parked.result.resident_string_bytes().unwrap().as_ref(),
        b"READY"
    );
    assert_eq!(target.get(), 0);
    assert!(
        NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        vm.try_invoke_command("unrelated", &[]).unwrap().code,
        Code::Ok
    );
    assert_eq!(unrelated.get(), 1);
    assert!(
        NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .is_none()
    );
    let resumed = vm.try_invoke_command("pending", &[]).unwrap();
    assert_eq!(resumed.code, Code::Ok);
    assert_eq!(
        resumed.result.resident_string_bytes().unwrap().as_ref(),
        b"ANSWER"
    );
    assert_eq!(target.get(), 1);
    assert_eq!(vm.current_level(), 0);
}

#[test]
fn original_name_dispatch_probe_checks_saved_currency_before_callback() {

    for withdraw in [false, true] {
        let mut vm = native();
        let observer = register(&mut vm, "observer", false, false);
        assert_eq!(
            vm.try_eval_source("coroutine paused apply {{} {yield READY; return END}}")
                .unwrap()
                .code,
            Code::Ok
        );
        assert!(
            NativeOperationScope::current_entered(&mut vm)
                .unwrap()
                .is_none()
        );
        if withdraw {
            let host = vm.host_rc();
            vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
            vm.set_host(host);
        }
        let reached = vm.try_invoke_command(
            "coroprobe",
            &[Value::string("paused"), Value::string("observer")],
        );
        if withdraw {
            assert!(matches!(
                reached,
                Err(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "entered VM original execution context"
                    )
                ))
            ));
            assert_eq!(observer.get(), 0);
        } else {
            let reached = reached.unwrap();
            assert_eq!(reached.code, Code::Ok);
            assert_eq!(observer.get(), 1);
            assert!(
                NativeOperationScope::current_entered(&mut vm)
                    .unwrap()
                    .is_none()
            );
            let resumed = vm.try_invoke_command("paused", &[]).unwrap();
            assert_eq!(resumed.code, Code::Ok);
            assert_eq!(
                resumed.result.resident_string_bytes().unwrap().as_ref(),
                b"END"
            );
        }
        assert_eq!(vm.current_level(), 0);
        assert_eq!(vm.current_ns_id(), tcl_core_types::ROOT_NS);
    }
}

#[test]
fn original_name_dispatch_injection_refusal_stops_older_callbacks_and_body() {

    // Selected C9 injection ordering with software Host callbacks, not an external extension.
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl9.0"));
    let withdraw = register(&mut vm, "withdraw", true, false);
    let older = register(&mut vm, "older", false, false);
    let body = register(&mut vm, "body", false, false);
    assert_eq!(vm.try_eval_source(
        "coroutine paused apply {{} {yield READY; body}}; coroinject paused older; coroinject paused withdraw"
    ).unwrap().code, Code::Ok);
    let original = NativeOperationScope::enter(&mut vm).unwrap();
    let first = vm.try_invoke_command("paused", &[]).unwrap_err();
    assert_eq!(withdraw.get(), 1);
    assert_eq!(older.get(), 0);
    assert_eq!(body.get(), 0);
    retained_first(&mut vm, &original, &first);
    assert!(vm.coro.parked_lambda_registrations().is_empty());
}
