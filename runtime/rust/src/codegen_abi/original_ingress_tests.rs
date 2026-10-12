// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software controls for original compiler-ABI ingress and completion ownership.

use super::*;
use std::cell::Cell;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

thread_local! {
    static UPDATES: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static COMMANDS: Cell<usize> = const { Cell::new(0) };
    static UPDATE_BYTES: Cell<&'static [u8]> = const { Cell::new(b"string") };
    static OTHER: Cell<*mut Interp> = const { Cell::new(ptr::null_mut()) };
}

fn original() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

fn change_and_restore(interpreter: &mut Interp) {
    let original = interpreter.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("original-codegen-entry".to_owned(), "1.0".to_owned())];
    interpreter.pin_context(&changed).unwrap();
    interpreter.pin_context(&original).unwrap();
}

fn stale() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::CommandProtocolUnavailable(
        "stale entered native operation",
    ))
}

fn sentinel() -> TclCompletionAbi {
    TclCompletionAbi {
        code: 73,
        result: ptr::null_mut(),
        options: ptr::null_mut(),
    }
}

fn changed_command(interpreter: &mut Interp, _: &[*mut TclObj]) -> crate::interp::Code {
    COMMANDS.with(|calls| calls.set(calls.get() + 1));
    interpreter.set_result_bytes(b"reached\0result");
    change_and_restore(interpreter);
    crate::interp::Code::Ok
}

fn later_command(interpreter: &mut Interp, _: &[*mut TclObj]) -> crate::interp::Code {
    COMMANDS.with(|calls| calls.set(calls.get() + 1));
    interpreter.set_result_bytes(b"LATER");
    crate::interp::Code::Ok
}

extern "C" fn updater(value: *mut TclObj) {
    UPDATES.with(|calls| calls.set(calls.get() + 1));
    let mut interpreter = unsafe { current_interp().as_ref() }.unwrap().clone();
    change_and_restore(&mut interpreter);
    UPDATE_BYTES
        .with(|bytes| unsafe { obj::set_native_updater_string_rep(value, bytes.get(), false) });
}

extern "C" fn free_original_rep(_: *mut TclObj) {
    FREES.with(|calls| calls.set(calls.get() + 1));
}

static ORIGINAL_UPDATER: obj::TclObjType = obj::TclObjType {
    name: c"originalCodegenIngress".as_ptr(),
    free_int_rep_proc: Some(free_original_rep),
    dup_int_rep_proc: None,
    update_string_proc: Some(updater),
    set_from_any_proc: None,
};

#[test]
fn original_argv_callback_refusal_keeps_borrowed_words_and_completion_output() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Genuine registered callback and source effects; no external Tcl claim.
    let mut interpreter = original();
    interpreter.register_builtin(b"change_context", changed_command);
    tcl_runtime_set_current_interp(&mut interpreter);
    COMMANDS.with(|calls| calls.set(0));
    let words = [obj::Owned::fresh(new_string_bytes(b"change_context"))];
    let counts = words
        .each_ref()
        .map(|word| unsafe { (*word.as_ptr()).ref_count });
    let pointers = words.each_ref().map(obj::Owned::as_ptr);
    let mut out = sentinel();
    assert_eq!(
        unsafe { tcl_invoke_argv(pointers.as_ptr(), 1, &mut out) },
        TCL_INVOKE_ABI_HOST_REFUSED
    );
    assert_eq!(COMMANDS.with(Cell::get), 1);
    assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
    assert_eq!(out.code, 73);
    assert!(out.result.is_null() && out.options.is_null());
    assert_eq!(
        words
            .each_ref()
            .map(|word| unsafe { (*word.as_ptr()).ref_count }),
        counts
    );
    assert_eq!(interpreter.result_bytes(), b"reached\0result");
    tcl_runtime_set_current_interp(ptr::null_mut());
}

#[test]
fn original_intrinsic_and_guard_getters_refuse_before_dispatch_or_fallback() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The original extension updater is reached, while no later command or
    // completion is published. A form decline cannot swallow a Host refusal.
    for route in 0..3 {
        let mut interpreter = original();
        tcl_runtime_set_current_interp(&mut interpreter);
        UPDATES.with(|calls| calls.set(0));
        UPDATE_BYTES.with(|bytes| bytes.set(b"string"));
        let original_words = [b"string".as_slice(), b"length", b"abc"]
            .map(|bytes| obj::Owned::fresh(new_string_bytes(bytes)));
        let original_pointers = original_words.each_ref().map(obj::Owned::as_ptr);
        let native = interpreter.native_invocation_dialect();
        let expected = GuardIdentity::registry_intrinsic_with_semantics(
            IntrinsicId::StringLength.stable_id(),
            IntrinsicId::StringLength
                .guard_semantics_key_for_characters(native.characters.unwrap()),
        );
        let token = unsafe {
            tcl_codegen_guard_prepare(
                IntrinsicId::StringLength.stable_id(),
                original_pointers.as_ptr(),
                3,
                expected.namespace(),
                expected.value(),
                i32::from(
                    GuardDomains::one(tcl_runtime_api::guard::GuardDomain::CommandEnvironment)
                        .bits(),
                ),
            )
        };
        assert_ne!(token, 0, "positive genuine guard must precede the updater");
        let lazy = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL_UPDATER, 0));
        let words = [
            lazy.as_ptr(),
            original_words[1].as_ptr(),
            original_words[2].as_ptr(),
        ];
        let counts = words.map(|word| unsafe { (*word).ref_count });
        let mut out = sentinel();
        match route {
            0 => assert_eq!(
                unsafe {
                    tcl_codegen_guard_prepare(
                        IntrinsicId::StringLength.stable_id(),
                        words.as_ptr(),
                        3,
                        expected.namespace(),
                        expected.value(),
                        i32::from(
                            GuardDomains::one(
                                tcl_runtime_api::guard::GuardDomain::CommandEnvironment,
                            )
                            .bits(),
                        ),
                    )
                },
                0
            ),
            1 => assert_eq!(
                unsafe {
                    tcl_codegen_guard_check(
                        token,
                        IntrinsicId::StringLength.stable_id(),
                        words.as_ptr(),
                        3,
                    )
                },
                0
            ),
            2 => assert_eq!(
                unsafe {
                    tcl_intrinsic_invoke_argv(
                        IntrinsicId::StringLength.stable_id(),
                        words.as_ptr(),
                        3,
                        &mut out,
                    )
                },
                TCL_INVOKE_ABI_HOST_REFUSED
            ),
            _ => unreachable!(),
        }
        assert_eq!(UPDATES.with(Cell::get), 1);
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        assert_eq!(out.code, 73);
        assert!(out.result.is_null() && out.options.is_null());
        assert_eq!(words.map(|word| unsafe { (*word).ref_count }), counts);
        assert_eq!(obj::bytes_of(lazy.as_ptr()), b"string");
        tcl_codegen_guard_release(token);
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        tcl_runtime_set_current_interp(ptr::null_mut());
    }
}

#[test]
fn original_compound_word_preserves_counted_bytes_and_refuses_after_updater_effects() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Counted String transport is independent of command/name syntax and of
    // original Native cache observations.
    let mut interpreter = original();
    tcl_runtime_set_current_interp(&mut interpreter);
    let parts = [b"A\0".as_slice(), "café".as_bytes(), b"\xff\\\n"]
        .map(|bytes| obj::Owned::fresh(new_string_bytes(bytes)));
    let words = parts.each_ref().map(obj::Owned::as_ptr);
    let counts = words.map(|word| unsafe { (*word).ref_count });
    let joined = unsafe { obj::Owned::from_raw(tcl_codegen_word_concat(words.as_ptr(), 3)) };
    assert!(!joined.as_ptr().is_null());
    assert_eq!(obj::bytes_of(joined.as_ptr()), b"A\0caf\xc3\xa9\xff\\\n");
    assert_eq!(words.map(|word| unsafe { (*word).ref_count }), counts);
    let lazy = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL_UPDATER, 0));
    UPDATES.with(|calls| calls.set(0));
    UPDATE_BYTES.with(|bytes| bytes.set(b"reached\0part"));
    let attempted = [parts[0].as_ptr(), lazy.as_ptr(), parts[1].as_ptr()];
    let before = attempted.map(|word| unsafe { (*word).ref_count });
    assert!(unsafe { tcl_codegen_word_concat(attempted.as_ptr(), 3) }.is_null());
    assert_eq!(UPDATES.with(Cell::get), 1);
    assert_eq!(obj::bytes_of(lazy.as_ptr()), b"reached\0part");
    assert_eq!(attempted.map(|word| unsafe { (*word).ref_count }), before);
    assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
    tcl_runtime_set_current_interp(ptr::null_mut());
}

#[test]
#[cfg(have_tommath)]
fn original_legacy_eval_getter_refusal_adopts_input_without_running_source() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual extension updater and transferred rc-0 input; no successful
    // evaluation, result or Boolean output is inferred from its rendered bytes.
    for route in 0..3 {
        let mut interpreter = original();
        interpreter.register_builtin(b"later", later_command);
        tcl_runtime_set_current_interp(&mut interpreter);
        COMMANDS.with(|calls| calls.set(0));
        UPDATES.with(|calls| calls.set(0));
        FREES.with(|calls| calls.set(0));
        UPDATE_BYTES.with(|bytes| bytes.set(b"later"));
        let lazy = obj::alloc_typed(&ORIGINAL_UPDATER, 0);
        assert_eq!(unsafe { (*lazy).ref_count }, 0);
        match route {
            0 => assert!(unsafe { tcl_eval(lazy) }.is_null()),
            1 => assert_eq!(unsafe { tcl_eval_code(lazy) }, TCL_INVOKE_ABI_HOST_REFUSED),
            2 => assert_eq!(unsafe { tcl_expr_bool(lazy) }, TCL_INVOKE_ABI_HOST_REFUSED),
            _ => unreachable!(),
        }
        assert_eq!(UPDATES.with(Cell::get), 1);
        assert_eq!(COMMANDS.with(Cell::get), 0);
        assert_eq!(
            FREES.with(Cell::get),
            1,
            "the transferred input is retired exactly once"
        );
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        tcl_runtime_set_current_interp(ptr::null_mut());
    }
}

fn retarget_command(interpreter: &mut Interp, _: &[*mut TclObj]) -> crate::interp::Code {
    OTHER.with(|other| tcl_runtime_set_current_interp(other.get()));
    interpreter.report_cmd_error(tcl_cmd_core::CmdError::with_byte_error_details(
        b"ORIGINAL\0\xff".to_vec(),
        b"USER ORIGINAL".to_vec(),
        Some(b"TRACE\xfe".to_vec()),
        Some(9),
    ))
}

#[test]
fn original_guest_completion_uses_its_retained_interpreter_after_tls_retarget() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Mutable ambient presentation cannot choose the completion owner.
    let mut interpreter = original();
    let mut other = original();
    let unrelated = NativeValueAccessRefusal::CommandProtocolUnavailable("unrelated interpreter");
    other.refuse_native_access(unrelated.clone());
    OTHER.with(|slot| slot.set(&mut other));
    interpreter.register_builtin(b"retarget", retarget_command);
    tcl_runtime_set_current_interp(&mut interpreter);
    let word = obj::Owned::fresh(new_string_bytes(b"retarget"));
    let before = unsafe { (*word.as_ptr()).ref_count };
    let words = [word.as_ptr()];
    let mut out = sentinel();
    assert_eq!(
        unsafe { tcl_invoke_argv(words.as_ptr(), 1, &mut out) },
        TCL_INVOKE_ABI_OK
    );
    assert_eq!(out.code, 1);
    assert_eq!(obj::bytes_of(out.result), b"ORIGINAL\0\xff");
    let pairs = tcl_syntax::value::ValueOps::dict_pairs(&mut interpreter, &out.options).unwrap();
    let code = pairs
        .iter()
        .find_map(|(key, value)| (obj::bytes_of(*key) == b"-errorcode").then_some(*value))
        .unwrap();
    assert_eq!(obj::bytes_of(code), b"USER ORIGINAL");
    assert!(interpreter.native_execution_refusal().is_none());
    assert_eq!(
        other.native_execution_refusal(),
        Some(NativeExecutionError::ValueAccessRefusal(unrelated))
    );
    assert_eq!(unsafe { (*word.as_ptr()).ref_count }, before);
    unsafe { tcl_completion_release(&mut out) };
    assert!(out.result.is_null() && out.options.is_null());
    OTHER.with(|slot| slot.set(ptr::null_mut()));
    tcl_runtime_set_current_interp(ptr::null_mut());
}
