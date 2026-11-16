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

use super::*;
use crate::interp::native_operation_currency::NativeOperationScope;
use crate::obj::Owned;
use std::cell::Cell;
use tcl_runtime_api::NativeExecutionError;

#[derive(Clone, Copy)]
enum Update {
    Host,
    #[cfg(debug_assertions)]
    Reenter,
}

thread_local! {
    static UPDATE: Cell<Update> = const { Cell::new(Update::Host) };
    static GETTERS: Cell<usize> = const { Cell::new(0) };
    static SCRIPT_CODE: Cell<i32> = const { Cell::new(0) };
}

extern "C" fn materialize(original: *mut TclObj) {
    GETTERS.with(|calls| calls.set(calls.get() + 1));
    // The original extension updater is invoked by the selected C8.4
    // completion-options getter, not by a fabricated cache observation.
    let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .expect("original live extension interpreter")
        .clone();
    match UPDATE.with(Cell::get) {
        Update::Host => {
            interpreter.refuse_host_command("original options getter first cause");
        }
        #[cfg(debug_assertions)]
        Update::Reenter => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("completion-currency-test".to_owned(), "1.0".to_owned())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
    }
    // Effects and the genuine original header remain reached on refusal.
    unsafe { obj::set_string_rep(original, b"REACHED\0\xff") };
}

static ORIGINAL_METADATA: obj::TclObjType = obj::TclObjType {
    name: c"originalCompletionOptionsGetter".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(materialize),
    set_from_any_proc: None,
};

struct Current(*mut Interp);
impl Current {
    fn enter(interpreter: &mut Interp) -> Self {
        let previous = crate::codegen_abi::current_interp();
        crate::codegen_abi::tcl_runtime_set_current_interp(interpreter);
        Self(previous)
    }
}
impl Drop for Current {
    fn drop(&mut self) {
        crate::codegen_abi::tcl_runtime_set_current_interp(self.0);
    }
}

fn original() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        tcl_registry::model::resolve_environment("tcl8.4").unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[cfg(debug_assertions)]
fn stale() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "stale entered native operation",
        ),
    )
}

#[test]
fn original_completion_capture_keeps_options_getter_host_cause_and_owned_handles() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for route in 0..3 {
        let mut interpreter = original();
        let original = Owned::fresh(obj::alloc_typed(&ORIGINAL_METADATA, 0));
        interpreter.set_error_state(b"ORIGINAL CODE");
        interpreter.set_result(original.as_ptr());
        interpreter.set_return_state(2, crate::interp::Code::Other(7));
        let refs = unsafe { (*original.as_ptr()).ref_count };
        let _current = Current::enter(&mut interpreter);
        UPDATE.with(|mode| mode.set(Update::Host));
        GETTERS.with(|calls| calls.set(0));
        let error = match route {
            0 => {
                let scope = NativeOperationScope::enter(&interpreter).unwrap();
                capture_completion_checked(
                    &mut interpreter,
                    scope.currency(),
                    crate::interp::Code::Error,
                )
                .unwrap_err()
            }
            1 => crate::completion::capture_bytes(&mut interpreter, crate::interp::Code::Error)
                .unwrap_err(),
            _ => {
                let mut out = crate::codegen_abi::TclCompletionAbi {
                    code: 91,
                    result: original.as_ptr(),
                    options: original.as_ptr(),
                };
                assert_eq!(
                    unsafe {
                        crate::engine_abi::value_carriers::tcl_engine_capture_original(
                            &mut interpreter,
                            1,
                            &mut out,
                        )
                    },
                    1
                );
                assert_eq!(out.code, 91);
                assert_eq!(out.result, original.as_ptr());
                assert_eq!(out.options, original.as_ptr());
                interpreter.native_execution_refusal().unwrap()
            }
        };
        GETTERS.with(|calls| assert_eq!(calls.get(), 1, "route {route}"));
        let NativeExecutionError::HostCommandRefusal(ref first) = error else {
            panic!("original options getter Host cause was replaced");
        };
        assert_eq!(first.reason, "original options getter first cause");
        assert_eq!(
            first.source_profile,
            interpreter.dialect_profile().cache_key()
        );
        assert_eq!(
            first.native_profile,
            interpreter.dialect_profile().cache_key()
        );
        assert_eq!(interpreter.native_execution_refusal(), Some(error.clone()));
        assert_eq!(interpreter.result_obj(), original.as_ptr());
        assert_eq!(obj::bytes_of(original.as_ptr()), b"REACHED\0\xff");
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, refs);
        assert_eq!(interpreter.pending_return_level(), 2);
        assert_eq!(
            interpreter.pending_return_code(),
            crate::interp::Code::Other(7)
        );
        interpreter.refuse_native_access(
            tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable,
        );
        assert_eq!(interpreter.native_execution_refusal(), Some(error));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_completion_capture_refuses_restored_context_before_abi_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for route in 0..3 {
        let mut interpreter = original();
        let original_context = interpreter.runtime_context();
        let original = Owned::fresh(obj::alloc_typed(&ORIGINAL_METADATA, 0));
        interpreter.set_error_state(b"ORIGINAL CODE");
        interpreter.set_result(original.as_ptr());
        let refs = unsafe { (*original.as_ptr()).ref_count };
        let _current = Current::enter(&mut interpreter);
        UPDATE.with(|mode| mode.set(Update::Reenter));
        GETTERS.with(|calls| calls.set(0));
        match route {
            0 => {
                let scope = NativeOperationScope::enter(&interpreter).unwrap();
                assert_eq!(
                    capture_completion_checked(
                        &mut interpreter,
                        scope.currency(),
                        crate::interp::Code::Error,
                    )
                    .unwrap_err(),
                    stale()
                );
            }
            1 => assert_eq!(
                crate::completion::capture_bytes(&mut interpreter, crate::interp::Code::Error)
                    .unwrap_err(),
                stale()
            ),
            _ => {
                let mut out = crate::codegen_abi::TclCompletionAbi {
                    code: 91,
                    result: original.as_ptr(),
                    options: original.as_ptr(),
                };
                assert_eq!(
                    unsafe {
                        crate::engine_abi::value_carriers::tcl_engine_capture_original(
                            &mut interpreter,
                            1,
                            &mut out,
                        )
                    },
                    1
                );
                assert_eq!(out.code, 91);
                assert_eq!(out.result, original.as_ptr());
                assert_eq!(out.options, original.as_ptr());
            }
        }
        GETTERS.with(|calls| assert_eq!(calls.get(), 1, "route {route}"));
        assert_eq!(interpreter.runtime_context(), original_context);
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        assert_eq!(interpreter.result_obj(), original.as_ptr());
        assert_eq!(obj::bytes_of(original.as_ptr()), b"REACHED\0\xff");
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, refs);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
#[test]
fn original_completion_capture_preserves_counted_guest_codes_options_and_references() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for code in 0..=7 {
        let mut interpreter = original();
        let original = Owned::fresh(crate::interp::new_string(b"GUEST\0\xff"));
        interpreter.set_error_state(b"ORIGINAL ERROR CODE");
        interpreter.set_result(original.as_ptr());
        interpreter.set_return_state(2, crate::interp::Code::Other(7));
        let refs = unsafe { (*original.as_ptr()).ref_count };
        let scope = NativeOperationScope::enter(&interpreter).unwrap();
        let completion = capture_completion_checked(
            &mut interpreter,
            scope.currency(),
            crate::interp::Code::from_int(code),
        )
        .unwrap();
        assert_eq!(completion.code, tcl_runtime_api::Code::from_int(code));
        assert_eq!(completion.result, original.as_ptr());
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, refs + 1);
        let result = unsafe { Owned::from_raw(completion.result) };
        let options = unsafe { Owned::from_raw(completion.options) };
        assert_eq!(
            interpreter
                .native_object_string_bytes(result.as_ptr())
                .unwrap()
                .as_ref(),
            b"GUEST\0\xff"
        );
        let actual_code = crate::dict::dict_get(options.as_ptr(), b"-code")
            .unwrap()
            .expect("captured completion includes -code");
        let actual_level = crate::dict::dict_get(options.as_ptr(), b"-level")
            .unwrap()
            .expect("captured completion includes -level");
        assert_eq!(
            obj::bytes_of(actual_code),
            if code == 2 {
                b"7".to_vec()
            } else {
                code.to_string().into_bytes()
            }
        );
        assert_eq!(
            obj::bytes_of(actual_level),
            if code == 2 {
                b"2".as_slice()
            } else {
                b"0".as_slice()
            }
        );
        if code == 1 {
            assert_eq!(
                obj::bytes_of(
                    crate::dict::dict_get(options.as_ptr(), b"-errorcode")
                        .unwrap()
                        .expect("original error completion includes -errorcode"),
                ),
                b"ORIGINAL ERROR CODE"
            );
        }
        drop(options);
        drop(result);
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, refs);
        let bytes =
            crate::completion::capture_bytes(&mut interpreter, crate::interp::Code::from_int(code))
                .unwrap();
        assert_eq!(bytes.code, tcl_runtime_api::Code::from_int(code));
        assert_eq!(bytes.result, b"GUEST\0\xff");
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, refs);
        assert!(interpreter.native_execution_refusal().is_none());
        scope.currency().ensure_current().unwrap();
        drop(scope);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

fn return_original(interpreter: &mut Interp, argv: &[*mut TclObj]) -> crate::interp::Code {
    interpreter.set_error_state(b"ORIGINAL CALLBACK CODE");
    interpreter.set_result(argv[1]);
    crate::interp::Code::Error
}

#[test]
fn original_command_roles_keep_typed_capture_refusal_and_borrowed_argv() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for route in 0..3 {
        let mut interpreter = original();
        interpreter.register_builtin(b"original_completion_probe", return_original);
        let metadata = Owned::fresh(obj::alloc_typed(&ORIGINAL_METADATA, 0));
        let head = Owned::fresh(crate::interp::new_string(b"original_completion_probe"));
        interpreter.set_result(metadata.as_ptr());
        let refs = unsafe { (*metadata.as_ptr()).ref_count };
        let head_refs = unsafe { (*head.as_ptr()).ref_count };
        let _current = Current::enter(&mut interpreter);
        UPDATE.with(|mode| mode.set(Update::Host));
        GETTERS.with(|calls| calls.set(0));
        let error = match route {
            0 => Commands::dispatch(
                &mut interpreter,
                "original_completion_probe",
                &[metadata.as_ptr()],
            )
            .unwrap_err(),
            1 => {
                let id = Namespaces::find_command(
                    &interpreter,
                    tcl_runtime_api::ROOT_NS,
                    "original_completion_probe",
                )
                .unwrap();
                Commands::dispatch_id(&mut interpreter, id, &[metadata.as_ptr()]).unwrap_err()
            }
            _ => dispatch_prebuilt_argv(&mut interpreter, &[head.as_ptr(), metadata.as_ptr()])
                .unwrap_err(),
        };
        let NativeExecutionError::HostCommandRefusal(ref first) = error else {
            panic!("command role lost the actual options getter cause");
        };
        assert_eq!(first.reason, "original options getter first cause");
        assert_eq!(interpreter.native_execution_refusal(), Some(error.clone()));
        GETTERS.with(|calls| assert_eq!(calls.get(), 1));
        assert_eq!(obj::bytes_of(metadata.as_ptr()), b"REACHED\0\xff");
        assert_eq!(unsafe { (*metadata.as_ptr()).ref_count }, refs);
        assert_eq!(unsafe { (*head.as_ptr()).ref_count }, head_refs);
        assert_eq!(interpreter.result_obj(), metadata.as_ptr());
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        assert_eq!(
            Commands::dispatch_id(&mut interpreter, CommandId(u32::MAX), &[]).unwrap_err(),
            error,
            "invalid command ids cannot replace an earlier Host cause"
        );
    }
}

#[cfg(debug_assertions)]
fn reenter_script(interpreter: &mut Interp, _: &[*mut TclObj]) -> crate::interp::Code {
    interpreter.set_result_bytes(b"REACHED SCRIPT\0\xff");
    let original = interpreter.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("completion-script-test".to_owned(), "1.0".to_owned())];
    interpreter.pin_context(&changed).unwrap();
    interpreter.pin_context(&original).unwrap();
    crate::interp::Code::from_int(SCRIPT_CODE.with(Cell::get))
}

#[cfg(debug_assertions)]
#[test]
fn original_public_completion_keeps_before_body_currency_and_reached_script_effects() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for (sourced, guest_code) in [
        (false, 0),
        (false, 1),
        (false, 7),
        (true, 0),
        (true, 1),
        (true, 7),
    ] {
        let mut interpreter = original();
        interpreter.register_builtin(b"original_reenter_script", reenter_script);
        SCRIPT_CODE.with(|code| code.set(guest_code));
        let original_context = interpreter.runtime_context();
        let script = b"set ::before ORIGINAL; original_reenter_script; set ::after LATER";
        let completion = if sourced {
            interpreter.eval_sourced_completion(script, b"original-source.tcl")
        } else {
            interpreter.eval_completion(script)
        };
        assert_eq!(completion.unwrap_err(), stale());
        assert_eq!(interpreter.runtime_context(), original_context);
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        assert_eq!(
            obj::bytes_of(interpreter.result_obj()),
            b"REACHED SCRIPT\0\xff"
        );
        let before = crate::vars::get(
            &interpreter.frames.borrow(),
            &interpreter.namespaces(),
            interpreter.current_ns(),
            b"::before",
        )
        .unwrap();
        assert_eq!(obj::bytes_of(before), b"ORIGINAL");
        assert!(crate::vars::get(
            &interpreter.frames.borrow(),
            &interpreter.namespaces(),
            interpreter.current_ns(),
            b"::after",
        )
        .is_none());
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
