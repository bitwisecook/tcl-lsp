// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Software invocation-owner controls using actual VM callbacks and cleanup.

use crate::prelude::*;
use std::cell::Cell;
use tcl_vm::NativeCommand;

fn native() -> Vm {
    crate::fixture::interpreter(tcl_vm::test_support::profile("tcl8.6"))
}

struct Observe {
    calls: Rc<Cell<usize>>,
    withdraw: bool,
    guest_error: bool,
}
impl NativeCommand for Observe {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        if self.withdraw {
            vm.set_host(Rc::new(tcl_vm::host_native::NativeHost::new()));
        }
        if self.guest_error {
            Completion::new(Code::Error, Value::string("GUEST"), Value::empty())
        } else {
            Completion::new(Code::Ok, Value::string("ANSWER"), Value::empty())
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

fn level(vm: &mut Vm) -> String {
    vm.try_eval_source("info level")
        .unwrap()
        .result
        .to_str()
        .to_string()
}

#[test]
fn original_name_dispatch_keeps_direct_success_and_guest_error_separate() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
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
            completion.result.string_bytes().as_ref(),
            if guest_error {
                b"GUEST".as_slice()
            } else {
                b"ANSWER".as_slice()
            }
        );
    }
}

// The withdrawn host is found by the restored-context re-proof, which runs
// only under `VERIFY_CURRENCY`.
#[cfg(debug_assertions)]
#[test]
fn original_name_dispatch_checks_enter_and_leave_before_later_work() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for operation in ["enter", "leave"] {
        let mut vm = native();
        let work = register(&mut vm, "work", false, false);
        let observer = register(&mut vm, "observer", true, false);
        let later = register(&mut vm, "later", false, false);
        let setup = vm
            .try_eval_source(&format!("trace add execution work {operation} observer"))
            .unwrap();
        assert_eq!(setup.code, Code::Ok);
        assert!(vm.try_eval_source("work; later").is_err());
        assert_eq!(observer.get(), 1);
        assert_eq!(work.get(), usize::from(operation == "leave"));
        assert_eq!(later.get(), 0);
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_name_dispatch_retains_deferred_body_scope_through_refused_cleanup() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
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
        assert!(vm.try_eval_source(body).is_err());
        assert_eq!(withdraw.get(), 1);
        assert_eq!(later.get(), 0);
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_name_dispatch_carries_tailcall_owner_into_replacement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for withdraw in [false, true] {
        let mut vm = native();
        let target = register(&mut vm, "target", withdraw, false);
        let setup = vm
            .try_eval_source("proc replace {} {tailcall target}")
            .unwrap();
        assert_eq!(setup.code, Code::Ok);
        let reached = vm.try_invoke_command("replace", &[]);
        assert_eq!(target.get(), 1);
        if withdraw {
            assert!(reached.is_err());
        } else {
            let completion = reached.unwrap();
            assert_eq!(completion.code, Code::Ok);
            assert_eq!(completion.result.string_bytes().as_ref(), b"ANSWER");
            assert_eq!(level(&mut vm), "0");
        }
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_name_dispatch_keeps_parked_coroutine_receipt_without_reissuing_it() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for change in 0..3 {
        let mut vm = native();
        let target = register(&mut vm, "target", false, false);
        let unrelated = register(&mut vm, "unrelated", false, false);
        let suspended = vm
            .try_eval_source("coroutine suspended apply {{} {yield READY; target}}")
            .unwrap();
        assert_eq!(suspended.code, Code::Ok);
        assert_eq!(suspended.result.string_bytes().as_ref(), b"READY");
        assert_eq!(target.get(), 0);
        let original_host: Rc<dyn tcl_platform::Host> =
            Rc::new(tcl_vm::host_native::NativeHost::new());
        if change != 0 {
            vm.set_host(Rc::new(tcl_vm::host_native::NativeHost::new()));
            if change == 2 {
                vm.set_host(original_host);
            }
        }
        let fresh = vm.try_invoke_command("unrelated", &[]).unwrap();
        assert_eq!(fresh.code, Code::Ok);
        assert_eq!(unrelated.get(), 1);
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
            assert_eq!(level(&mut vm), "0");
        } else {
            let completion = resumed.unwrap();
            assert_eq!(completion.code, Code::Ok);
            assert_eq!(completion.result.string_bytes().as_ref(), b"ANSWER");
            assert_eq!(target.get(), 1);
        }
    }
}

#[test]
fn original_name_dispatch_parks_caught_tailcall_owner_before_unrelated_entry() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
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
    assert_eq!(parked.result.string_bytes().as_ref(), b"READY");
    assert_eq!(target.get(), 0);
    assert_eq!(
        vm.try_invoke_command("unrelated", &[]).unwrap().code,
        Code::Ok
    );
    assert_eq!(unrelated.get(), 1);
    let resumed = vm.try_invoke_command("pending", &[]).unwrap();
    assert_eq!(resumed.code, Code::Ok);
    assert_eq!(resumed.result.string_bytes().as_ref(), b"ANSWER");
    assert_eq!(target.get(), 1);
    assert_eq!(level(&mut vm), "0");
}

#[test]
fn original_name_dispatch_probe_checks_saved_currency_before_callback() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // `coroprobe` is Tcl 9's (tclsh8.6: `invalid command name "coroprobe"`);
    // the withdrawn host is found only under `VERIFY_CURRENCY`.
    let withdrawals: &[bool] = if cfg!(debug_assertions) {
        &[false, true]
    } else {
        &[false]
    };
    for &withdraw in withdrawals {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.0"));
        let observer = register(&mut vm, "observer", false, false);
        assert_eq!(
            vm.try_eval_source("coroutine paused apply {{} {yield READY; return END}}")
                .unwrap()
                .code,
            Code::Ok
        );
        if withdraw {
            let host: Rc<dyn tcl_platform::Host> = Rc::new(tcl_vm::host_native::NativeHost::new());
            vm.set_host(Rc::new(tcl_vm::host_native::NativeHost::new()));
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
            let resumed = vm.try_invoke_command("paused", &[]).unwrap();
            assert_eq!(resumed.code, Code::Ok);
            assert_eq!(resumed.result.string_bytes().as_ref(), b"END");
        }
        assert_eq!(level(&mut vm), "0");
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_name_dispatch_injection_refusal_stops_older_callbacks_and_body() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Selected C9 injection ordering with software Host callbacks, not an external extension.
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.0"));
    let withdraw = register(&mut vm, "withdraw", true, false);
    let older = register(&mut vm, "older", false, false);
    let body = register(&mut vm, "body", false, false);
    assert_eq!(vm.try_eval_source(
        "coroutine paused apply {{} {yield READY; body}}; coroinject paused older; coroinject paused withdraw"
    ).unwrap().code, Code::Ok);
    assert!(vm.try_invoke_command("paused", &[]).is_err());
    assert_eq!(withdraw.get(), 1);
    assert_eq!(older.get(), 0);
    assert_eq!(body.get(), 0);
}
