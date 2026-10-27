// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A synchronous optional arithmetic hint inside the original increment kernel.

use super::{CodegenOperation, TclObj};
use crate::codegen_abi::{TCL_INVOKE_ABI_NULL_OUT, TCL_INVOKE_ABI_NULL_WORD};

/// Try one current cached integer receiver without changing a declined input.
/// Zero leaves output untouched; one ran the original command and writes its
/// owned result (or null on guest error). Negative host statuses are terminal.
///
/// # Safety
/// Name bytes and output must be valid; both object pointers must remain live
/// under the caller's existing owned references for the complete call.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_try_cached_incr(
    name_ptr: *const u8,
    name_len: i32,
    expected: *mut TclObj,
    delta: *mut TclObj,
    old: i64,
    sum: i64,
    out: *mut *mut TclObj,
) -> i32 {
    if out.is_null() {
        return TCL_INVOKE_ABI_NULL_OUT;
    }
    if expected.is_null() || delta.is_null() || name_ptr.is_null() || name_len < 0 {
        return TCL_INVOKE_ABI_NULL_WORD;
    }
    let mut operation = match CodegenOperation::enter() {
        Ok(operation) => operation,
        Err(status) => return status,
    };
    // SAFETY: the caller retains the readable original name range.
    let name = unsafe { super::input_bytes(name_ptr, name_len) };
    if !operation
        .interpreter
        .cached_increment_name_matches(name, expected, delta, old, sum)
    {
        return match operation.ensure_current() {
            Ok(()) => 0,
            Err(status) => status,
        };
    }
    // SAFETY: same existing boxed operands remain owned throughout the command.
    let code = unsafe { run_cached_command(&mut operation.interpreter, name, delta, (old, sum)) };
    operation.ensure_current().map_or_else(
        |status| status,
        |()| {
            let result = if code == crate::interp::Code::Ok {
                let result = operation.interpreter.result_obj();
                // SAFETY: the same original command result is live and owned.
                unsafe { crate::obj::incr_ref_count(result) };
                result
            } else {
                core::ptr::null_mut()
            };
            // SAFETY: caller provides a writable result slot.
            unsafe { out.write(result) };
            1
        },
    )
}

unsafe fn run_cached_command(
    interp: &mut crate::interp::Interp,
    name: &[u8],
    delta: *mut TclObj,
    hint: (i64, i64),
) -> crate::interp::Code {
    let head_obj = crate::obj::new_string_bytes(b"incr");
    let name_obj = crate::obj::new_string_bytes(name);
    let mut argv = vec![head_obj, name_obj];
    argv.extend_from_slice(&[delta]);
    // SAFETY: identical original command words stay live for the whole call.
    let borrowed = unsafe { super::BorrowedArgv::retain(&argv) };
    let code = crate::builtins::incr_with_cached_sum(interp, &argv, hint);
    drop(borrowed);
    code
}

#[cfg(all(test, have_tommath, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::interp::{Code, Interp};
    use crate::obj::{self, Owned};

    struct Installed;
    impl Drop for Installed {
        fn drop(&mut self) {
            crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
        }
    }
    fn native() -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl9.0"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    fn install(interp: &mut Interp) -> Installed {
        crate::codegen_abi::tcl_runtime_set_current_interp(interp);
        Installed
    }
    unsafe fn attempt(
        expected: *mut TclObj,
        delta: *mut TclObj,
        old: i64,
        sum: i64,
        out: &mut *mut TclObj,
    ) -> i32 {
        // SAFETY: tests retain live owned operands, name bytes and output.
        unsafe { tcl_codegen_try_cached_incr(b"a".as_ptr(), 1, expected, delta, old, sum, out) }
    }

    fn assert_hidden_callback_declines(interp: &mut Interp, delta: *mut TclObj) {
        for name in [b"errorInfo".as_slice(), b"errorCode".as_slice()] {
            let expected = Owned::fresh(obj::new_wide_int_obj(1));
            interp.var_set(name, expected.as_ptr()).unwrap();
            assert!(matches!(
                obj::native_scalar_cache(expected.as_ptr()).unwrap(),
                Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                    tcl_syntax::number::Number::Int(1)
                ))
            ));
            let mut out = expected.as_ptr();
            assert_eq!(
                unsafe {
                    tcl_codegen_try_cached_incr(
                        name.as_ptr(),
                        name.len() as i32,
                        expected.as_ptr(),
                        delta,
                        1,
                        2,
                        &mut out,
                    )
                },
                0
            );
            assert_eq!(out, expected.as_ptr());
            assert_eq!(obj::wide_of(expected.as_ptr()), 1);
            assert_eq!(interp.native_execution_refusal(), None);
        }
    }

    #[test]
    fn cached_increment_runs_original_cow_and_keeps_the_held_input() {
        let mut interp = native();
        let expected = Owned::fresh(obj::new_wide_int_obj(1));
        let delta = Owned::fresh(obj::new_wide_int_obj(1));
        interp.var_set(b"a", expected.as_ptr()).unwrap();
        let _installed = install(&mut interp);
        let mut out = core::ptr::null_mut();
        assert_eq!(
            unsafe { attempt(expected.as_ptr(), delta.as_ptr(), 1, 2, &mut out) },
            1
        );
        assert!(!out.is_null());
        let result = unsafe { Owned::from_raw(out) };
        assert_eq!(crate::interp::obj_bytes(result.as_ptr()), b"2");
        assert_eq!(obj::wide_of(expected.as_ptr()), 1);
        assert_ne!(interp.var_get(b"a"), Some(expected.as_ptr()));
        assert_eq!(interp.native_execution_refusal(), None);
        assert_hidden_callback_declines(&mut interp, delta.as_ptr());
    }

    #[test]
    fn cached_increment_decline_preserves_input_and_runs_fallback_once() {
        let mut interp = native();
        let expected = Owned::fresh(obj::new_string_bytes(b"1"));
        let delta = Owned::fresh(obj::new_wide_int_obj(1));
        interp.var_set(b"a", expected.as_ptr()).unwrap();
        let _installed = install(&mut interp);
        let mut out = expected.as_ptr();
        assert_eq!(
            unsafe { attempt(expected.as_ptr(), delta.as_ptr(), 1, 2, &mut out) },
            0
        );
        assert_eq!(out, expected.as_ptr());
        assert!(obj::native_scalar_cache(expected.as_ptr())
            .unwrap()
            .is_none());
        assert_eq!(interp.var_get(b"a"), Some(expected.as_ptr()));
        let result =
            unsafe { super::super::tcl_codegen_var_incr(b"a".as_ptr(), 1, delta.as_ptr()) };
        assert!(!result.is_null());
        let result = unsafe { Owned::from_raw(result) };
        assert_eq!(crate::interp::obj_bytes(result.as_ptr()), b"2");
        assert_eq!(crate::interp::obj_bytes(expected.as_ptr()), b"1");
    }

    #[test]
    fn cached_increment_hint_never_replaces_reached_operands_or_trace_fallback() {
        let mut interp = native();
        let expected = Owned::fresh(obj::new_wide_int_obj(1));
        let delta = Owned::fresh(obj::new_wide_int_obj(1));
        interp.var_set(b"a", expected.as_ptr()).unwrap();
        let _installed = install(&mut interp);
        // Deliberately stale private arithmetic data cannot donate a value.
        assert_eq!(
            unsafe { run_cached_command(&mut interp, b"a", delta.as_ptr(), (9, 10)) },
            Code::Ok
        );
        assert_eq!(crate::interp::obj_bytes(interp.result_obj()), b"2");
        assert_eq!(
            interp.eval_str(
                b"set calls 0; proc w {args} {incr ::calls}; trace add variable a write w"
            ),
            Code::Ok
        );
        let current = Owned::retain(interp.var_get(b"a").unwrap());
        let mut out = current.as_ptr();
        assert_eq!(
            unsafe { attempt(current.as_ptr(), delta.as_ptr(), 2, 3, &mut out) },
            0
        );
        assert_eq!(out, current.as_ptr());
        let result =
            unsafe { super::super::tcl_codegen_var_incr(b"a".as_ptr(), 1, delta.as_ptr()) };
        assert!(!result.is_null());
        let result = unsafe { Owned::from_raw(result) };
        assert_eq!(crate::interp::obj_bytes(result.as_ptr()), b"3");
        assert_eq!(
            crate::interp::obj_bytes(interp.var_get(b"calls").unwrap()),
            b"1"
        );
    }
}
