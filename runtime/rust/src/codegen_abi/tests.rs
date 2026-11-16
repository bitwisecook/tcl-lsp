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
use crate::capi::{tcl_runtime_create_interp, tcl_runtime_delete_interp};
use crate::counters;
use crate::interp::Code;
use std::cell::RefCell;
use std::rc::Rc;
use tcl_dialect::TclVersion;
use tcl_host_native::NativeHost;
use tcl_platform::{Capabilities, Clock, Env, Filesystem, Host, Process, StdIo};
use tcl_runtime_api::codegen_abi::{NATIVE_PROC_STATUS_DECLINED, NATIVE_PROC_STATUS_RAN};
use tcl_runtime_api::guard::GuardDomain;

#[test]
fn reached_unicode_refusal_keeps_argv_completion_output_untouched() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let words = [b"string".as_slice(), b"toupper", b"A\xff"].map(|bytes| {
            tcl_obj_new_string_owned(bytes.as_ptr(), i32::try_from(bytes.len()).unwrap())
        });
        let mut out = TclCompletionAbi {
            code: 77,
            result: core::ptr::null_mut(),
            options: core::ptr::null_mut(),
        };
        assert_eq!(
            tcl_invoke_argv(words.as_ptr(), 3, &mut out),
            TCL_INVOKE_ABI_HOST_REFUSED
        );
        assert_eq!(tcl_codegen_host_refusal_pending(), 1);
        assert_eq!(out.code, 77);
        assert!(out.result.is_null());
        assert!(out.options.is_null());
        assert_eq!((*interp).unicode_access_refusal().unwrap().valid_up_to, 1);
        for word in words {
            tcl_obj_release(word);
        }
        tcl_runtime_set_current_interp(core::ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

#[test]
fn reached_refusal_bypasses_capture_finally_and_rmw_read_error_recovery() {
    for script in [
            b"incr prior; catch {string toupper $raw} captured options; set after YES".as_slice(),
            b"incr prior; try {string toupper $raw} finally {set final YES}; set after YES",
            b"incr prior; set x 10; proc cb {args} {string toupper $::raw}; trace add variable x read cb; incr x; set after YES",
            b"incr prior; array set a {k 1}; proc cb {args} {string toupper $::raw}; trace add variable a(k) read cb; catch {array get a} captured options; set after YES",
        ] {
            leak_free(|| {
                let mut interp = Interp::new();
                let raw = new_string_bytes(b"A\xff");
                interp.var_set(b"raw", raw).unwrap();
                interp.var_set(b"prior", crate::obj::new_wide_int_obj(0)).unwrap();
                assert_eq!(interp.eval_str(script), Code::Error);
                assert!(interp.host_refusal_pending());
                let original_refusal = interp.native_execution_refusal().unwrap();
                assert_eq!(interp.current_level(), 0);
                assert_eq!(interp.frames.borrow().top_level(), 0);
                assert_eq!(interp.current_ns(), crate::namespace::GLOBAL);
                assert!(interp.var_get(b"prior").is_none());
                let prior = crate::vars::get_at(
                    &interp.frames.borrow(),
                    &interp.namespaces(),
                    b"prior",
                    0,
                )
                .unwrap();
                assert!(!prior.is_null());
                if obj::has_string_rep(prior) {
                    // SAFETY: the unchanged root table retains this resident String.
                    assert_eq!(unsafe {
    std::slice::from_raw_parts(
        (*prior).bytes.cast::<u8>(),
        usize::try_from((*prior).length).unwrap(),
    )
}, b"1");
                } else {
                    assert!(core::ptr::eq(obj::obj_type_ptr(prior), &obj::TCL_INT_TYPE));
                    assert_eq!(obj::wide_of(prior), 1);
                }
                for name in [b"captured".as_slice(), b"options", b"final", b"after"] {
                    assert!(!interp.var_exists(name));
                    assert!(crate::vars::get_at(
                        &interp.frames.borrow(),
                        &interp.namespaces(),
                        name,
                        0,
                    )
                    .is_none());
                }
                let x = crate::vars::get_at(
                    &interp.frames.borrow(),
                    &interp.namespaces(),
                    b"x",
                    0,
                );
                if let Some(x) = x {
                    assert!(interp.var_get(b"x").is_none());
                    assert!(!x.is_null());
                    if obj::has_string_rep(x) {
                        // SAFETY: the unchanged root table retains this resident String.
                        assert_eq!(unsafe {
    std::slice::from_raw_parts(
        (*x).bytes.cast::<u8>(),
        usize::try_from((*x).length).unwrap(),
    )
}, b"10");
                    } else {
                        assert!(core::ptr::eq(obj::obj_type_ptr(x), &obj::TCL_INT_TYPE));
                        assert_eq!(obj::wide_of(x), 10);
                    }
                }
                assert_eq!(interp.native_execution_refusal(), Some(original_refusal));
            });
        }
}

#[test]
fn byte_guest_error_metadata_is_captured_without_host_refusal() {
    fn fail(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        interp.report_cmd_error(tcl_cmd_core::CmdError::with_byte_error_details(
            b"BOOM\xff".to_vec(),
            b"RAW \xfe".to_vec(),
            Some(b"TRACE\xfd".to_vec()),
            Some(7),
        ))
    }
    leak_free(|| {
        let mut interp = Interp::new();
        interp.register_builtin(b"rawfail", fail);
        assert_eq!(interp.eval_str(b"catch {rawfail} r opts"), Code::Ok);
        assert_eq!(interp.result_bytes(), b"1");
        assert!(!interp.host_refusal_pending());
        assert_eq!(obj_bytes(interp.var_get(b"r").unwrap()), b"BOOM\xff");
        assert_eq!(interp.eval_str(b"dict get $opts -errorcode"), Code::Ok);
        assert_eq!(interp.result_bytes(), b"RAW \xfe");
        assert_eq!(interp.eval_str(b"dict get $opts -errorinfo"), Code::Ok);
        assert!(interp.result_bytes().starts_with(b"TRACE\xfd"));
    });
}

struct CaptureHost {
    native: NativeHost,
    stdout: Rc<RefCell<Vec<u8>>>,
}

impl StdIo for CaptureHost {
    fn write_stdout(&self, bytes: &[u8]) {
        self.stdout.borrow_mut().extend_from_slice(bytes);
    }

    fn write_stderr(&self, _bytes: &[u8]) {}
}

impl Host for CaptureHost {
    fn capabilities(&self) -> Capabilities {
        self.native.capabilities()
    }

    fn clock(&self) -> &dyn Clock {
        self.native.clock()
    }

    fn stdio(&self) -> &dyn StdIo {
        self
    }

    fn env(&self) -> &dyn Env {
        self.native.env()
    }

    fn filesystem(&self) -> Option<&dyn Filesystem> {
        self.native.filesystem()
    }

    fn process(&self) -> Option<&dyn Process> {
        self.native.process()
    }
}

/// Run `body` under the alloc/free counters and assert zero residual — the
/// codegen ABI's references must balance exactly like the rest of the runtime.
fn leak_free(body: impl FnOnce()) {
    counters::reset();
    body();
    assert_eq!(
        counters::finalize(),
        0,
        "residual: {} objs, {} bufs",
        counters::live_objs(),
        counters::live_bufs()
    );
    assert_eq!(counters::double_free_count(), 0, "double frees detected");
}

/// Box a `&[u8]` as the emitter would (a fresh `rc 0` script/expr object).
unsafe fn box_str(s: &[u8]) -> *mut TclObj {
    // SAFETY: `s` is a valid readable slice.
    unsafe { tcl_obj_new_string(s.as_ptr(), s.len() as i32) }
}

#[cfg(have_tommath)]
unsafe fn owned_str(s: &[u8]) -> *mut TclObj {
    // SAFETY: `s` is a valid readable slice.
    unsafe { tcl_value_new_string(s.as_ptr(), s.len() as i32) }
}

/// Box one borrowed argv word and take the caller's owning reference.
unsafe fn owned_word(s: &[u8]) -> *mut TclObj {
    // SAFETY: `box_str` copies valid bytes into a fresh object.
    let word = unsafe { box_str(s) };
    // SAFETY: the test owns the fresh object and keeps this +1 until it
    // releases the argv after invoking.
    unsafe { obj::incr_ref_count(word) };
    word
}

fn empty_completion() -> TclCompletionAbi {
    TclCompletionAbi {
        code: 0,
        result: ptr::null_mut(),
        options: ptr::null_mut(),
    }
}

unsafe fn invoke(words: &[*mut TclObj]) -> TclCompletionAbi {
    let mut completion = empty_completion();
    // SAFETY: `words` is a Rust slice of live caller-owned object handles;
    // completion storage is local and correctly aligned.
    assert_eq!(
        unsafe {
            tcl_invoke_argv(
                words.as_ptr(),
                i32::try_from(words.len()).expect("test argv fits i32"),
                &mut completion,
            )
        },
        TCL_INVOKE_ABI_OK
    );
    completion
}

unsafe fn invoke_intrinsic(
    intrinsic: IntrinsicId,
    words: &[*mut TclObj],
) -> (i32, TclCompletionAbi) {
    let mut completion = empty_completion();
    // SAFETY: `words` is a Rust slice of live caller-owned object handles;
    // completion storage is local and correctly aligned.
    let status = unsafe {
        tcl_intrinsic_invoke_argv(
            intrinsic.stable_id(),
            words.as_ptr(),
            i32::try_from(words.len()).expect("test argv fits i32"),
            &mut completion,
        )
    };
    (status, completion)
}

unsafe fn prepare_intrinsic_guard(
    intrinsic: IntrinsicId,
    words: &[*mut TclObj],
    domains: i32,
) -> u64 {
    let interp = current_interp();
    assert!(
        !interp.is_null(),
        "test helper requires a current interpreter"
    );
    // SAFETY: tests install a live current interpreter before using this helper.
    let characters = unsafe { (*interp).native_invocation_dialect() }
        .characters
        .expect("test runtime has a selected character protocol");
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        intrinsic.stable_id(),
        intrinsic.guard_semantics_key_for_characters(characters),
    );
    // SAFETY: `words` is a Rust slice of live caller-owned object handles.
    unsafe {
        tcl_codegen_guard_prepare(
            intrinsic.stable_id(),
            words.as_ptr(),
            i32::try_from(words.len()).expect("test argv fits i32"),
            identity.namespace(),
            identity.value(),
            domains,
        )
    }
}

unsafe fn release_words(words: &[*mut TclObj]) {
    for &word in words {
        // SAFETY: balances `owned_word`'s caller-owned reference.
        unsafe { obj::decr_ref_count(word) };
    }
}

fn option(completion: &TclCompletionAbi, key: &[u8]) -> Vec<u8> {
    let value = crate::dict::dict_get(completion.options, key)
        .expect("completion options are a dict")
        .expect("expected completion option");
    obj_bytes(value)
}

#[test]
fn native_wide_int_value_is_owned_and_keeps_its_internal_rep_after_shimmering() {
    leak_free(|| unsafe {
        let value = tcl_value_new_wide_int(i64::MIN);
        assert!(!value.is_null());
        assert_eq!((*value).ref_count, 1, "generated code owns the result");
        assert!(std::ptr::eq((*value).type_ptr, &obj::TCL_INT_TYPE));
        assert!((*value).bytes.is_null(), "wide integers begin without text");
        assert_eq!(obj_bytes(value), i64::MIN.to_string().as_bytes());
        assert!(
            std::ptr::eq((*value).type_ptr, &obj::TCL_INT_TYPE),
            "string materialisation must retain the wide internal representation"
        );
        assert_eq!((*value).internal_rep as i64, i64::MIN);
        tcl_obj_release(value);
    });
}

#[test]
fn guarded_intrinsic_string_length_hits_and_releases_exact_completion_and_token() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let words = [
            owned_word(b"string"),
            owned_word(b"length"),
            owned_word(b"abc"),
        ];
        let counts = words.map(|word| (*word).ref_count);
        let domains = i32::from(GuardDomains::one(GuardDomain::CommandEnvironment).bits());
        let native = (*interp).native_invocation_dialect();
        assert!(
            resolve_intrinsic_argv(
                &mut CodegenOperation::enter().unwrap(),
                &words,
                native.authoring_query()
            )
            .unwrap()
            .is_some(),
            "actual Jim StringLength form is retained"
        );
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            IntrinsicId::StringLength.stable_id(),
            IntrinsicId::StringLength
                .guard_semantics_key_for_characters(native.characters.unwrap()),
        );
        let direct = (*interp).prepare_command_guard(
            b"string",
            identity,
            GuardDomains::one(GuardDomain::Interpreter),
        );
        assert!(direct.is_ok(), "native stock string identity: {direct:?}");
        tcl_codegen_guard_release(direct.unwrap().raw());
        let token = prepare_intrinsic_guard(IntrinsicId::StringLength, &words, domains);
        assert_ne!(token, 0);
        assert_eq!(
            tcl_codegen_guard_check(
                token,
                IntrinsicId::StringLength.stable_id(),
                words.as_ptr(),
                i32::try_from(words.len()).unwrap(),
            ),
            1
        );

        let (status, mut completion) = invoke_intrinsic(IntrinsicId::StringLength, &words);
        assert_eq!(status, TCL_INVOKE_ABI_OK);
        assert_eq!(completion.code, 0);
        assert_eq!(obj_bytes(completion.result), b"3");
        assert_eq!(option(&completion, b"-code"), b"0");
        assert_eq!(words.map(|word| (*word).ref_count), counts);

        tcl_completion_release(&mut completion);
        assert!(completion.result.is_null());
        assert!(completion.options.is_null());
        tcl_codegen_guard_release(token);
        assert_eq!(
            tcl_codegen_guard_check(
                token,
                IntrinsicId::StringLength.stable_id(),
                words.as_ptr(),
                i32::try_from(words.len()).unwrap(),
            ),
            0
        );
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

#[test]
fn guarded_intrinsic_string_length_identity_and_count_follow_runtime_version() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let words = [
            owned_word(b"string"),
            owned_word(b"length"),
            owned_word("é🙂".as_bytes()),
        ];

        let (status, mut completion) = invoke_intrinsic(IntrinsicId::StringLength, &words);
        assert_eq!(status, TCL_INVOKE_ABI_OK);
        assert_eq!(completion.code, 0);
        assert_eq!(obj_bytes(completion.result), b"2");
        tcl_completion_release(&mut completion);

        let tcl9_identity = GuardIdentity::registry_intrinsic_with_semantics(
            IntrinsicId::StringLength.stable_id(),
            IntrinsicId::StringLength.guard_semantics_key(tcl_dialect::TclVersion::V9_0),
        );
        (*interp).set_runtime_version(tcl_dialect::TclVersion::V8_6);
        let domains = i32::from(
            GuardDomains::one(GuardDomain::CommandEnvironment)
                .with(GuardDomain::Namespace)
                .with(GuardDomain::CommandTrace)
                .with(GuardDomain::Interpreter)
                .bits(),
        );
        assert_eq!(
            tcl_codegen_guard_prepare(
                IntrinsicId::StringLength.stable_id(),
                words.as_ptr(),
                i32::try_from(words.len()).unwrap(),
                tcl9_identity.namespace(),
                tcl9_identity.value(),
                domains,
            ),
            0,
            "a Tcl 9 scalar-count identity must not enter a Tcl 8 runtime",
        );
        let (status, mut completion) = invoke_intrinsic(IntrinsicId::StringLength, &words);
        assert_eq!(status, TCL_INVOKE_ABI_HOST_REFUSED);
        assert_eq!(
            (*interp).native_access_refusal(),
            Some(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native string count cache origin",
                ),
            )
        );
        tcl_completion_release(&mut completion);
        // A retained Host refusal stays on its original interpreter. Start
        // the independent C8.6 success control on a fresh actual owner.
        let refused_interp = interp;
        let interp = tcl_runtime_create_interp();
        (*interp).set_runtime_version(tcl_dialect::TclVersion::V8_6);
        tcl_runtime_set_current_interp(interp);
        assert_eq!(
            (*refused_interp).native_access_refusal(),
            Some(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native string count cache origin",
                ),
            )
        );
        assert_eq!((*interp).eval_str(b""), Code::Ok);
        let c86_words = [
            owned_word(b"string"),
            owned_word(b"length"),
            owned_word("é🙂".as_bytes()),
        ];
        let (status, mut completion) = invoke_intrinsic(IntrinsicId::StringLength, &c86_words);
        assert_eq!(status, TCL_INVOKE_ABI_OK);
        assert_eq!(obj_bytes(completion.result), b"3");
        tcl_completion_release(&mut completion);
        release_words(&words);
        release_words(&c86_words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
        tcl_runtime_delete_interp(refused_interp);
    });
}

#[test]
fn guarded_intrinsic_jim_character_identity_ignores_the_c_assistance_release() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        (*interp).set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        tcl_runtime_set_current_interp(interp);
        let words = [
            owned_word(b"string"),
            owned_word(b"length"),
            owned_word("é🙂".as_bytes()),
        ];
        let domains = i32::from(GuardDomains::one(GuardDomain::Interpreter).bits());
        let c_identity = GuardIdentity::registry_intrinsic_with_semantics(
            IntrinsicId::StringLength.stable_id(),
            IntrinsicId::StringLength.guard_semantics_key((*interp).runtime_version()),
        );
        assert_eq!(
            tcl_codegen_guard_prepare(
                IntrinsicId::StringLength.stable_id(),
                words.as_ptr(),
                3,
                c_identity.namespace(),
                c_identity.value(),
                domains,
            ),
            0
        );
        let native = (*interp).native_invocation_dialect();
        assert!(
            resolve_intrinsic_argv(
                &mut CodegenOperation::enter().unwrap(),
                &words,
                native.authoring_query()
            )
            .unwrap()
            .is_some(),
            "actual Jim StringLength form is retained"
        );
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            IntrinsicId::StringLength.stable_id(),
            IntrinsicId::StringLength
                .guard_semantics_key_for_characters(native.characters.unwrap()),
        );
        let direct = (*interp).prepare_command_guard(
            b"string",
            identity,
            GuardDomains::one(GuardDomain::Interpreter),
        );
        assert!(direct.is_ok(), "native stock string identity: {direct:?}");
        tcl_codegen_guard_release(direct.unwrap().raw());
        let token = prepare_intrinsic_guard(IntrinsicId::StringLength, &words, domains);
        assert_ne!(token, 0);
        assert_eq!(
            tcl_codegen_guard_check(
                token,
                IntrinsicId::StringLength.stable_id(),
                words.as_ptr(),
                3
            ),
            1
        );
        let (status, mut completion) = invoke_intrinsic(IntrinsicId::StringLength, &words);
        assert_eq!(status, TCL_INVOKE_ABI_OK);
        assert_eq!(obj_bytes(completion.result), b"2");
        tcl_completion_release(&mut completion);
        tcl_codegen_guard_release(token);
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

#[test]
fn guarded_intrinsic_unsupported_form_declines_without_completion_or_source_replay() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let words = [owned_word(b"llength"), owned_word(b"a b c")];
        let (status, completion) = invoke_intrinsic(IntrinsicId::ListLength, &words);

        assert_eq!(status, TCL_INTRINSIC_ABI_DECLINED);
        assert_eq!(completion.code, 0);
        assert!(completion.result.is_null());
        assert!(completion.options.is_null());
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

#[test]
fn guarded_intrinsic_guards_fail_after_rename_or_trace_registration() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let words = [
            owned_word(b"string"),
            owned_word(b"length"),
            owned_word(b"abc"),
        ];
        let command_domains = i32::from(GuardDomains::one(GuardDomain::CommandEnvironment).bits());
        let token = prepare_intrinsic_guard(IntrinsicId::StringLength, &words, command_domains);
        assert_ne!(token, 0);
        assert_eq!(tcl_eval_code(box_str(b"rename string string2")), 0);
        assert_eq!(
            tcl_codegen_guard_check(
                token,
                IntrinsicId::StringLength.stable_id(),
                words.as_ptr(),
                i32::try_from(words.len()).unwrap(),
            ),
            0
        );
        tcl_codegen_guard_release(token);
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });

    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        assert_eq!(
            tcl_eval_code(box_str(b"trace add command string rename callback")),
            0
        );
        let words = [
            owned_word(b"string"),
            owned_word(b"length"),
            owned_word(b"abc"),
        ];
        let trace_domains = i32::from(GuardDomains::one(GuardDomain::CommandTrace).bits());
        assert_eq!(
            prepare_intrinsic_guard(IntrinsicId::StringLength, &words, trace_domains),
            0
        );
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// A guard is bound to `string`'s command token: defining, renaming or
/// aliasing another command leaves the exact check an emitted module makes
/// answering one, and rebinding `string` itself answers zero.
#[test]
fn guarded_intrinsic_guards_survive_unrelated_command_mutation() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let words = [
            owned_word(b"string"),
            owned_word(b"length"),
            owned_word(b"abc"),
        ];
        let base_domains = i32::from(
            GuardDomains::one(GuardDomain::CommandEnvironment)
                .with(GuardDomain::Namespace)
                .with(GuardDomain::CommandTrace)
                .with(GuardDomain::Interpreter)
                .bits(),
        );
        let token = prepare_intrinsic_guard(IntrinsicId::StringLength, &words, base_domains);
        assert_ne!(token, 0);
        let check = || {
            tcl_codegen_guard_check(
                token,
                IntrinsicId::StringLength.stable_id(),
                words.as_ptr(),
                i32::try_from(words.len()).unwrap(),
            )
        };
        assert_eq!(check(), 1);
        for script in [
            &b"proc unrelated {} {return 1}"[..],
            b"rename unrelated other",
            b"interp alias {} alias_of_other {} other",
        ] {
            assert_eq!(tcl_eval_code(box_str(script)), 0);
            assert_eq!(check(), 1, "{}", String::from_utf8_lossy(script));
        }
        assert_eq!(
            tcl_eval_code(box_str(b"proc string args {return shadow}")),
            0
        );
        assert_eq!(check(), 0);
        tcl_codegen_guard_release(token);
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

#[test]
#[cfg(have_tommath)]
fn compiled_slots_and_named_access_share_one_cell() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        tcl_codegen_frame_push();

        assert_eq!(
            tcl_codegen_local_bind(0, b"b".as_ptr(), 1, owned_str(b"2")),
            0
        );
        assert_eq!(
            tcl_codegen_local_bind(1, b"c".as_ptr(), 1, owned_str(b"4")),
            0
        );
        assert_eq!(tcl_eval_code(box_str(b"set b 5")), 0);

        let sum = tcl_codegen_expr_add(tcl_codegen_local_get(0), tcl_codegen_local_get(1));
        assert_eq!(obj_bytes(sum), b"9");
        tcl_value_release(sum);

        tcl_codegen_frame_pop();
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// The read-modify-write slot ABI is the runtime's own `incr` / `append` /
/// `lappend` addressed by index: full Tcl semantics, including the bignum
/// promotion `incr` gets from the numeric tower, and a by-name view of the
/// very same cell.
#[cfg(have_tommath)]
#[test]
fn slot_read_modify_write_runs_the_real_commands_on_the_same_cell() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        tcl_codegen_frame_push();

        assert_eq!(
            tcl_codegen_slot_bind(0, b"n".as_ptr(), 1, owned_str(b"1")),
            0
        );
        let mut value: i64 = 0;
        assert_eq!(
            tcl_codegen_slot_incr_i64(0, 41, &mut value),
            TCL_VALUE_GET_OK
        );
        assert_eq!(value, 42);
        // The named view sees the indexed write, and vice versa.
        assert_eq!(tcl_eval_code(box_str(b"set n")), 0);
        assert_eq!((*interp).result_bytes(), b"42");
        assert_eq!(tcl_eval_code(box_str(b"incr n")), 0);
        let indexed = tcl_codegen_slot_get(0);
        assert_eq!(obj_bytes(indexed), b"43");
        tcl_value_release(indexed);

        // `incr` promotes past the wide range instead of wrapping, and the
        // i64 read of that new value reports C's overflow error.
        assert_eq!(tcl_eval_code(box_str(b"set n 9223372036854775807")), 0);
        assert_eq!(
            tcl_codegen_slot_incr_i64(0, 1, &mut value),
            TCL_VALUE_GET_ERROR
        );
        assert_eq!(
            (*interp).result_bytes(),
            b"integer value too large to represent"
        );
        assert_eq!(tcl_eval_code(box_str(b"set n")), 0);
        assert_eq!((*interp).result_bytes(), b"9223372036854775808");

        // `incr` on a non-integer cell keeps C's message.
        assert_eq!(tcl_eval_code(box_str(b"set n abc")), 0);
        assert_eq!(
            tcl_codegen_slot_incr_i64(0, 1, &mut value),
            TCL_VALUE_GET_ERROR
        );
        assert_eq!(
            (*interp).result_bytes(),
            b"expected integer but got \"abc\""
        );

        // `append` / `lappend` through the slot, seen by name.
        assert_eq!(
            tcl_codegen_slot_bind(1, b"s".as_ptr(), 1, owned_str(b"a")),
            0
        );
        let piece = owned_str(b"bc");
        assert_eq!(tcl_codegen_slot_append(1, piece), 0);
        tcl_value_release(piece);
        assert_eq!(tcl_eval_code(box_str(b"set s")), 0);
        assert_eq!((*interp).result_bytes(), b"abc");

        assert_eq!(
            tcl_codegen_slot_bind(2, b"l".as_ptr(), 1, owned_str(b"x")),
            0
        );
        let item = owned_str(b"y z");
        assert_eq!(tcl_codegen_slot_lappend(2, item), 0);
        tcl_value_release(item);
        assert_eq!(tcl_eval_code(box_str(b"llength $l")), 0);
        assert_eq!((*interp).result_bytes(), b"2");
        assert_eq!(tcl_eval_code(box_str(b"lindex $l 1")), 0);
        assert_eq!((*interp).result_bytes(), b"y z");

        tcl_codegen_frame_pop();
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// An indexed slot keeps addressing one cell across `unset` and re-creation,
/// and a variable trace, an `upvar` link, and `info locals` all reach that
/// same cell — the indexed path must never become a second variable.
#[cfg(have_tommath)]
#[test]
fn an_indexed_slot_survives_unset_and_stays_the_named_cell() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        tcl_codegen_frame_push();

        assert_eq!(
            tcl_codegen_slot_bind(0, b"v".as_ptr(), 1, owned_str(b"first")),
            0
        );
        assert_eq!(tcl_eval_code(box_str(b"unset v")), 0);
        assert!(
            tcl_codegen_slot_get(0).is_null(),
            "an unset cell reads as a missing variable"
        );
        assert_eq!(tcl_eval_code(box_str(b"set v second")), 0);
        let value = tcl_codegen_slot_get(0);
        assert_eq!(
            obj_bytes(value),
            b"second",
            "the re-created variable refills the same cell"
        );
        tcl_value_release(value);

        // A read trace observes the indexed read: the fast path declines
        // whenever anything can see the difference.
        assert_eq!(
            tcl_eval_code(box_str(
                b"set ::hits 0; trace add variable v read {apply {{a b op} {incr ::hits}}}"
            )),
            0
        );
        let traced = tcl_codegen_slot_get(0);
        assert!(!traced.is_null());
        tcl_value_release(traced);
        assert_eq!(tcl_eval_code(box_str(b"set ::hits")), 0);
        assert_eq!((*interp).result_bytes(), b"1");

        tcl_codegen_frame_pop();
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// Ask the trace barrier about a name. Only the `have_tommath` tests
/// below reach it.
#[cfg(have_tommath)]
unsafe fn var_traced(name: &[u8]) -> i32 {
    // SAFETY: `name` is a valid readable slice.
    unsafe { tcl_codegen_var_traced(name.as_ptr(), name.len() as i32) }
}

/// The per-cell trace bit answers for a name and for a slot, follows an
/// `upvar` link to its target's traces, covers an array's elements, and —
/// the case a hand-maintained bit would get wrong — goes back to `0` when a
/// proc frame's teardown drops its locals' traces.
#[cfg(have_tommath)]
#[test]
fn the_trace_barrier_tracks_adds_removes_links_and_frame_teardown() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        assert_eq!(tcl_eval_code(box_str(b"set g 1; set ::hits 0")), 0);
        assert_eq!(var_traced(b"g"), 0, "an untraced global");

        assert_eq!(
            tcl_eval_code(box_str(
                b"trace add variable g write {apply {{a b op} {incr ::hits}}}"
            )),
            0
        );
        assert_eq!(var_traced(b"g"), 1);
        assert_eq!(var_traced(b"other"), 0, "the bit is per cell, not global");

        // An array reports traced when any element is: the guard is
        // conservative in the safe direction.
        assert_eq!(tcl_eval_code(box_str(b"set arr(k) 1")), 0);
        assert_eq!(var_traced(b"arr"), 0);
        assert_eq!(
            tcl_eval_code(box_str(
                b"trace add variable arr(k) write {apply {{a b op} {incr ::hits}}}"
            )),
            0
        );
        assert_eq!(var_traced(b"arr"), 1);
        assert_eq!(var_traced(b"arr(k)"), 1);

        assert_eq!(
            tcl_eval_code(box_str(
                b"trace remove variable g write {apply {{a b op} {incr ::hits}}}"
            )),
            0
        );
        assert_eq!(var_traced(b"g"), 0, "removing the last trace clears it");

        // A trace added inside a proc frame is visible through the local
        // name and through an `upvar` alias to it, and both answers go back
        // to untraced when the frame is popped.
        assert_eq!(
            tcl_eval_code(box_str(
                b"proc probe {} {\n\
                        set loc 1\n\
                        upvar 0 loc alias\n\
                        set ::before [tcltrace loc],[tcltrace alias]\n\
                        trace add variable loc write {apply {{a b op} {incr ::hits}}}\n\
                        set ::during [tcltrace loc],[tcltrace alias]\n\
                      }"
            )),
            0
        );
        // A tiny Tcl-visible probe over the same ABI entry point.
        assert_eq!(
            tcl_eval_code(box_str(
                b"proc tcltrace {n} { uplevel 1 [list ::tcl::probe_traced $n] }"
            )),
            0
        );
        (*interp).register_builtin(b"::tcl::probe_traced", |interp, argv| {
            let name = obj_bytes(argv[1]);
            let traced = interp.var_is_traced(&name);
            interp.set_result_bytes(if traced { b"1" } else { b"0" });
            crate::interp::Code::Ok
        });

        assert_eq!(tcl_eval_code(box_str(b"probe")), 0);
        assert_eq!(tcl_eval_code(box_str(b"set ::before")), 0);
        assert_eq!((*interp).result_bytes(), b"0,0");
        assert_eq!(tcl_eval_code(box_str(b"set ::during")), 0);
        assert_eq!(
            (*interp).result_bytes(),
            b"1,1",
            "an upvar alias reports its target's traces"
        );
        // The frame is gone, so its local's trace is too.
        assert_eq!(var_traced(b"loc"), 0);

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// The slot spelling answers from the cell's own bit, and that bit is
/// re-derived — never trusted — after the trace set changes.
#[cfg(have_tommath)]
#[test]
fn the_slot_trace_bit_is_revalidated_when_the_trace_set_changes() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        tcl_codegen_frame_push();

        assert_eq!(
            tcl_codegen_slot_bind(0, b"v".as_ptr(), 1, owned_str(b"1")),
            0
        );
        assert_eq!(tcl_codegen_slot_traced(0), 0);
        // Asked twice: the second answer comes from the cached bit.
        assert_eq!(tcl_codegen_slot_traced(0), 0);

        assert_eq!(tcl_eval_code(box_str(b"set ::hits 0")), 0);
        assert_eq!(
            tcl_eval_code(box_str(
                b"trace add variable v write {apply {{a b op} {incr ::hits}}}"
            )),
            0
        );
        assert_eq!(
            tcl_codegen_slot_traced(0),
            1,
            "the epoch moved, so the cached 0 was recomputed"
        );
        assert_eq!(tcl_codegen_slot_traced(0), 1);

        assert_eq!(
            tcl_eval_code(box_str(
                b"trace remove variable v write {apply {{a b op} {incr ::hits}}}"
            )),
            0
        );
        assert_eq!(tcl_codegen_slot_traced(0), 0);
        assert_eq!(tcl_codegen_slot_traced(9), 0, "an unbound slot is untraced");

        tcl_codegen_frame_pop();
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// The prebuilt argv boundary reaches the normal builtin dispatcher and
/// leaves each caller-owned argv reference untouched.
#[test]
fn invoke_argv_dispatches_builtin_without_adopting_words() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let words = [
            owned_word(b"string"),
            owned_word(b"length"),
            owned_word(b"abc"),
        ];
        let counts = words.map(|word| (*word).ref_count);

        let mut completion = invoke(&words);
        assert_eq!(completion.code, 0);
        assert_eq!(obj_bytes(completion.result), b"3");
        assert_eq!(option(&completion, b"-code"), b"0");
        assert_eq!(option(&completion, b"-level"), b"0");
        assert_eq!(words.map(|word| (*word).ref_count), counts);

        tcl_completion_release(&mut completion);
        assert!(completion.result.is_null());
        assert!(completion.options.is_null());
        // Reset output storage makes this release form deliberately
        // idempotent (unlike mixing it with individual handle releases).
        tcl_completion_release(&mut completion);
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

#[test]
fn compiled_puts_uses_channel_conversion_and_balances_its_owned_value() {
    leak_free(|| unsafe {
        let stdout = Rc::new(RefCell::new(Vec::new()));
        let interp = tcl_runtime_create_interp();
        (*interp).set_runtime_version(TclVersion::V9_0);
        (*interp).set_host(Rc::new(CaptureHost {
            native: NativeHost::new(),
            stdout: Rc::clone(&stdout),
        }));
        tcl_runtime_set_current_interp(interp);

        assert_eq!(
            tcl_eval_code(box_str(b"fconfigure stdout -translation binary")),
            0
        );
        assert_eq!(tcl_codegen_puts(owned_word(&[0xff, b'A'])), 0);
        assert_eq!(&*stdout.borrow(), &[0xff, b'A', b'\n']);

        assert_eq!(
            tcl_eval_code(box_str(
                b"fconfigure stdout -translation lf -encoding iso8859-1 -profile strict"
            )),
            0
        );
        assert_eq!(tcl_codegen_puts(owned_word("A\u{0178}B".as_bytes())), 1);
        assert_eq!(&*stdout.borrow(), &[0xff, b'A', b'\n', b'A']);
        assert_eq!(
            (*interp).result_bytes(),
            b"error writing \"stdout\": invalid or incomplete multibyte or wide character"
        );
        assert_eq!(
            (*interp).error_code(),
            b"POSIX EILSEQ {invalid or incomplete multibyte or wide character}"
        );

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

#[test]
#[cfg(have_tommath)]
fn nested_add_propagates_the_inner_arithmetic_error() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        let inner = tcl_codegen_expr_add(owned_str(b"not-a-number"), owned_str(b"1"));
        assert!(inner.is_null());
        let error = (*interp).result_bytes();
        assert!(!error.is_empty());

        let outer = tcl_codegen_expr_add(inner, owned_str(b"2"));
        assert!(outer.is_null());
        assert_eq!((*interp).result_bytes(), error);

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// Command lookup happens after argv construction, so a renamed command is
/// still resolved through the existing namespace table rather than a
/// compiler-side command-name table.
#[test]
fn invoke_argv_follows_renamed_command() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        assert_eq!(
            tcl_eval_code(box_str(
                b"proc original {x} {list got:$x}; rename original renamed"
            )),
            0
        );
        let words = [owned_word(b"renamed"), owned_word(b"value")];

        let mut completion = invoke(&words);
        assert_eq!(completion.code, 0);
        assert_eq!(obj_bytes(completion.result), b"got:value");

        tcl_completion_release(&mut completion);
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// Alias dispatch is resolved by the interpreter after argv construction;
/// the prebuilt path neither knows nor cares which command it redirects to.
#[test]
fn invoke_argv_follows_interp_alias() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        assert_eq!(
            tcl_eval_code(box_str(b"interp alias {} prefixed {} list prefix")),
            0
        );
        let words = [owned_word(b"prefixed"), owned_word(b"value")];

        let mut completion = invoke(&words);
        assert_eq!(completion.code, 0);
        assert_eq!(obj_bytes(completion.result), b"prefix value");

        tcl_completion_release(&mut completion);
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// A resolver miss takes the usual `unknown`/error path and returns the
/// real error options instead of an empty placeholder.
#[test]
fn invoke_argv_reports_unknown_error_with_options() {
    // naming.runtime.original-procedure-error-context
    // docs/design/analysis/name-resolution-proofs/runtime-original-procedure-error-context.md
    // The default ABI interpreter selects C9.0. Native295's ordinary-miss
    // public errorCode differs from the namespace-origin error producer.
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        assert_eq!(
            (*interp).native_invocation_dialect().tcl_version,
            Some(tcl_dialect::TclVersion::V9_0)
        );
        tcl_runtime_set_current_interp(interp);
        let words = [owned_word(b"definitely_missing_command")];

        let mut completion = invoke(&words);
        assert_eq!(completion.code, 1);
        assert!(obj_bytes(completion.result).starts_with(b"invalid command name"));
        assert_eq!(option(&completion, b"-code"), b"1");
        assert_eq!(option(&completion, b"-level"), b"0");
        assert_eq!(
            option(&completion, b"-errorcode"),
            b"TCL LOOKUP COMMAND definitely_missing_command"
        );
        assert!(option(&completion, b"-errorinfo").starts_with(b"invalid command name"));

        tcl_completion_release(&mut completion);
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// An uncaught error taken through the argv path leaves the same
/// interpreter error state as the source-eval path: `::errorInfo` and
/// `::errorCode` exist as globals.
#[test]
fn invoke_argv_publishes_uncaught_error_globals() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let words = [owned_word(b"error"), owned_word(b"boom")];

        let mut completion = invoke(&words);
        assert_eq!(completion.code, 1);

        // The source-eval path publishes these at the outermost eval; the
        // argv path must not diverge.
        assert_eq!(tcl_eval_code(box_str(b"set ::errorInfo")), 0);
        assert!((*interp).result_bytes().starts_with(b"boom"));

        tcl_completion_release(&mut completion);
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// Read one typed scalar through the C ABI, returning the status and the
/// value written (untouched storage on a refusal).
unsafe fn get_wide(value: *mut TclObj) -> (i32, i64) {
    let mut out: i64 = i64::MIN;
    // SAFETY: `value` is live and `out` is local aligned storage.
    let status = unsafe { tcl_value_get_wide_int(value, &mut out) };
    (status, out)
}

unsafe fn get_double(value: *mut TclObj) -> (i32, f64) {
    let mut out: f64 = f64::NAN;
    // SAFETY: `value` is live and `out` is local aligned storage.
    let status = unsafe { tcl_value_get_double(value, &mut out) };
    (status, out)
}

unsafe fn get_bool(value: *mut TclObj) -> (i32, i32) {
    let mut out: i32 = -1;
    // SAFETY: `value` is live and `out` is local aligned storage.
    let status = unsafe { tcl_value_get_bool(value, &mut out) };
    (status, out)
}

fn scalar_interpreter(environment: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        tcl_registry::model::resolve_environment(environment).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[test]
fn boxed_primitive_reads_share_actual_environment_and_raw_boolean_projection() {
    // Software adapters: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    leak_free(|| unsafe {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let mut interp = scalar_interpreter(environment);
            tcl_runtime_set_current_interp(&mut interp);
            let wide = owned_word(b"0x10");
            let double = owned_word(b"1.5");
            assert_eq!(get_wide(wide), (TCL_VALUE_GET_OK, 16), "{environment}");
            assert_eq!(get_double(double), (TCL_VALUE_GET_OK, 1.5), "{environment}");
            assert_eq!(obj_bytes(wide), b"0x10");
            assert_eq!(obj_bytes(double), b"1.5");
            release_words(&[wide, double]);
            for number in [17, 4_294_967_296] {
                let primitive = obj::Owned::fresh(obj::new_wide_int_obj(number));
                let boxed = obj::Owned::fresh(obj::new_wide_int_obj(number));
                let mut raw = 777;
                assert_eq!(
                    crate::capi::Tcl_GetBooleanFromObj(&mut interp, primitive.as_ptr(), &mut raw,),
                    0
                );
                let expected = if environment == "jim" {
                    if number == 17 {
                        17
                    } else {
                        0
                    }
                } else {
                    1
                };
                assert_eq!(raw, expected, "{environment}, {number}");
                assert_eq!(
                    get_bool(boxed.as_ptr()),
                    (TCL_VALUE_GET_OK, i32::from(raw != 0))
                );
                assert!(!interp.host_refusal_pending());
            }
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

struct ScalarNumericHost {
    actual: Rc<dyn Host>,
    interpreter: RefCell<Option<Interp>>,
    double_calls: std::cell::Cell<usize>,
    abi_calls: std::cell::Cell<usize>,
    change_abi_context: std::cell::Cell<bool>,
    abi_query_fails: std::cell::Cell<bool>,
    abi_available: bool,
    change_context: bool,
}

impl Host for ScalarNumericHost {
    fn capabilities(&self) -> Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn StdIo {
        self.actual.stdio()
    }
    fn env(&self) -> &dyn Env {
        self.actual.env()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.abi_available.then_some(self)
    }
}

impl ScalarNumericHost {
    fn actual(&self) -> &dyn tcl_platform::NumericEnvironment {
        self.actual
            .numeric_environment()
            .expect("actual native numeric environment")
    }
    fn change_and_restore_context(&self) {
        let mut interp = self.interpreter.borrow().as_ref().unwrap().clone();
        let original = interp.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("boxed-scalar-currency".to_owned(), "1.0".to_owned())];
        interp.pin_context(&changed).unwrap();
        interp.pin_context(&original).unwrap();
    }
}

impl tcl_platform::NumericEnvironment for ScalarNumericHost {
    fn c_integer_abi(
        &self,
    ) -> Result<tcl_platform::NativeCIntegerAbi, tcl_platform::NumericEnvironmentUnavailable> {
        let result = self.actual().c_integer_abi();
        self.abi_calls.set(self.abi_calls.get() + 1);
        if self.change_abi_context.get() {
            self.change_and_restore_context();
        }
        if self.abi_query_fails.get() {
            Err(tcl_platform::NumericEnvironmentUnavailable::Target)
        } else {
            result
        }
    }
    fn state(
        &self,
    ) -> Result<tcl_platform::NumericErrorState, tcl_platform::NumericEnvironmentUnavailable> {
        self.actual().state()
    }
    fn reset(&self) -> Result<(), tcl_platform::NumericEnvironmentUnavailable> {
        self.actual().reset()
    }
    fn unsigned_c84(
        &self,
        input: &[u8],
        offset: usize,
        long: bool,
    ) -> Result<tcl_platform::UnsignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.actual().unsigned_c84(input, offset, long)
    }
    fn unsigned(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<tcl_platform::UnsignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.actual().unsigned(input, offset, base)
    }
    fn signed_long(
        &self,
        input: &[u8],
        base: u32,
    ) -> Result<tcl_platform::SignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.actual().signed_long(input, base)
    }
    fn double(
        &self,
        input: &[u8],
        reset: bool,
    ) -> Result<tcl_platform::DoubleNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        let result = self.actual().double(input, reset);
        self.double_calls.set(self.double_calls.get() + 1);
        if self.change_context {
            self.change_and_restore_context();
        }
        result
    }
}

fn scalar_host(
    interp: &Interp,
    abi_available: bool,
    change_context: bool,
) -> Rc<ScalarNumericHost> {
    let host = Rc::new(ScalarNumericHost {
        actual: interp.host(),
        interpreter: RefCell::new(None),
        double_calls: std::cell::Cell::new(0),
        abi_calls: std::cell::Cell::new(0),
        change_abi_context: std::cell::Cell::new(false),
        abi_query_fails: std::cell::Cell::new(false),
        abi_available,
        change_context,
    });
    interp.set_host(host.clone());
    host
}

#[test]
fn boxed_primitive_reads_keep_missing_abi_and_first_host_terminal() {
    // Software owner: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    leak_free(|| unsafe {
        for first_host in [false, true] {
            let mut interp = scalar_interpreter("tcl8.4");
            let host = scalar_host(&interp, false, false);
            interp.set_result_bytes(b"PRIOR\0\xff");
            if first_host {
                interp.refuse_host_command("original first refusal");
            }
            let first = interp.native_execution_refusal();
            let value = owned_word(b"17");
            let before = obj::native_object_snapshot(value).unwrap();
            tcl_runtime_set_current_interp(&mut interp);
            assert_eq!(get_wide(value), (TCL_VALUE_GET_ERROR, i64::MIN));
            let double = get_double(value);
            assert_eq!(double.0, TCL_VALUE_GET_ERROR);
            assert!(double.1.is_nan());
            assert_eq!(get_bool(value), (TCL_VALUE_GET_ERROR, -1));
            assert_eq!(host.double_calls.get(), 0);
            assert_eq!(obj::native_object_snapshot(value).unwrap(), before);
            assert_eq!(interp.result_bytes(), b"PRIOR\0\xff");
            assert!(interp.host_refusal_pending());
            if first_host {
                assert_eq!(interp.native_execution_refusal(), first);
            }
            release_words(&[value]);
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

#[test]
fn boxed_primitive_reads_refuse_foreign_and_restored_engine_issuers() {
    // Software owner: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    leak_free(|| unsafe {
        for foreign in [false, true] {
            let mut owner = scalar_interpreter("tcl8.6");
            let mut other = scalar_interpreter("tcl8.6");
            let value = owned_word(b"17");
            crate::capi::bind_scalar_getter_context(&owner, value).unwrap();
            let before = obj::native_object_snapshot(value).unwrap();
            let original = owner.runtime_context();
            if !foreign {
                let mut changed = original.clone();
                changed.packages = vec![("boxed-scalar-owner".to_owned(), "1.0".to_owned())];
                owner.pin_context(&changed).unwrap();
                owner.pin_context(&original).unwrap();
            }
            let selected = if foreign { &mut other } else { &mut owner };
            selected.set_result_bytes(b"UNCHANGED");
            tcl_runtime_set_current_interp(selected);
            assert_eq!(get_wide(value), (TCL_VALUE_GET_ERROR, i64::MIN));
            assert_eq!(get_bool(value), (TCL_VALUE_GET_ERROR, -1));
            assert!(selected.host_refusal_pending());
            assert_eq!(selected.result_bytes(), b"UNCHANGED");
            assert_eq!(obj::native_object_snapshot(value).unwrap(), before);
            release_words(&[value]);
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

#[cfg(debug_assertions)]
#[test]
fn boxed_primitive_host_change_and_restore_stops_before_cache_and_output() {
    // Software currency: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|| unsafe {
        for environment in ["tcl8.4", "jim"] {
            let mut interp = scalar_interpreter(environment);
            let host = scalar_host(&interp, true, true);
            *host.interpreter.borrow_mut() = Some(interp.clone());
            let context = interp.runtime_context();
            let profile = interp.dialect_profile();
            interp.set_result_bytes(b"PRIOR\0\xff");
            let prior = interp.get_obj_result();
            let value = owned_word(b"1.5");
            let before = obj::native_object_snapshot(value).unwrap();
            tcl_runtime_set_current_interp(&mut interp);
            let outcome = get_double(value);
            assert_eq!(outcome.0, TCL_VALUE_GET_ERROR);
            assert!(outcome.1.is_nan());
            assert_eq!(host.double_calls.get(), 1, "{environment}");
            assert_eq!(interp.runtime_context(), context);
            assert!(std::ptr::eq(interp.dialect_profile(), profile));
            let first = interp.native_execution_refusal().unwrap();
            assert_eq!(
                first,
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "stale entered native operation",
                    ),
                )
            );
            assert_eq!(obj::native_object_snapshot(value).unwrap(), before);
            assert_eq!(interp.get_obj_result(), prior);
            assert_eq!(interp.result_bytes(), b"PRIOR\0\xff");
            assert_eq!(get_bool(value), (TCL_VALUE_GET_ERROR, -1));
            assert_eq!(host.double_calls.get(), 1);
            assert_eq!(interp.native_execution_refusal(), Some(first));
            tcl_runtime_set_current_interp(ptr::null_mut());
            release_words(&[value]);
            host.interpreter.borrow_mut().take();
        }
    });
}

#[cfg(debug_assertions)]
#[test]
fn scalar_abi_query_change_and_restore_cannot_issue_or_refresh_a_receipt() {
    // Software currency: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|| unsafe {
        for retained in [false, true] {
            for query_fails in [false, true] {
                let mut interp = scalar_interpreter("tcl8.6");
                let host = scalar_host(&interp, true, false);
                *host.interpreter.borrow_mut() = Some(interp.clone());
                let value = owned_word(b"1.5");
                if retained {
                    crate::capi::bind_scalar_getter_context(&interp, value).unwrap();
                }
                let before = obj::native_object_snapshot(value).unwrap();
                let context = interp.runtime_context();
                let calls = host.abi_calls.get();
                host.change_abi_context.set(true);
                host.abi_query_fails.set(query_fails);
                interp.set_result_bytes(b"PRIOR");
                let error = crate::capi::probe_scalar_getter(
                    if retained { None } else { Some(&interp) },
                    value,
                    tcl_syntax::scalar_getter::NativeScalarGetterKind::Double,
                )
                .unwrap_err();
                let first = interp.native_execution_refusal().unwrap();
                assert!(
                    matches!(error, crate::capi::NativeScalarObjectAccessError::Execution(ref cause) if *cause == first)
                );
                assert_eq!(first, tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "stale entered native operation",
                        ),
                    ));
                assert_eq!(host.abi_calls.get(), calls + 1);
                assert_eq!(host.double_calls.get(), 0);
                assert_eq!(interp.runtime_context(), context);
                assert_eq!(obj::native_object_snapshot(value).unwrap(), before);
                assert_eq!(interp.result_bytes(), b"PRIOR");
                if !retained {
                    assert!(obj::scalar_object_context(value).unwrap().is_none());
                }
                tcl_runtime_set_current_interp(&mut interp);
                let outcome = get_double(value);
                assert_eq!(outcome.0, TCL_VALUE_GET_ERROR);
                assert!(outcome.1.is_nan());
                assert_eq!(host.abi_calls.get(), calls + 1);
                assert_eq!(interp.native_execution_refusal(), Some(first));
                tcl_runtime_set_current_interp(ptr::null_mut());
                release_words(&[value]);
                host.interpreter.borrow_mut().take();
            }
        }
    });
}

thread_local! {
    static SCALAR_UPDATER_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(debug_assertions)]
extern "C" fn scalar_context_changing_updater(value: *mut TclObj) {
    SCALAR_UPDATER_CALLS.with(|calls| calls.set(calls.get() + 1));
    // SAFETY: the actual caller retains this original interpreter for the
    // updater. The clone shares its real state without borrowing it.
    let mut interp = unsafe { current_interp().as_ref() }.unwrap().clone();
    let original = interp.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("boxed-scalar-updater".to_owned(), "1.0".to_owned())];
    interp.pin_context(&changed).unwrap();
    interp.pin_context(&original).unwrap();
    // SAFETY: this actual descriptor owns the live original header.
    unsafe { obj::set_native_updater_string_rep(value, b"17", false) };
}

#[cfg(debug_assertions)]
static SCALAR_CONTEXT_TYPE: obj::TclObjType = obj::TclObjType {
    name: c"originalScalarCurrency".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(scalar_context_changing_updater),
    set_from_any_proc: None,
};

#[cfg(debug_assertions)]
#[test]
fn boxed_primitive_updater_change_and_restore_keeps_only_reached_string_effects() {
    // Software stages: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|| unsafe {
        let mut interp = scalar_interpreter("tcl8.6");
        let original = obj::Owned::fresh(obj::alloc_typed(&SCALAR_CONTEXT_TYPE, 0));
        let value = original.as_ptr();
        let context = interp.runtime_context();
        let references = (*value).ref_count;
        interp.set_result_bytes(b"PRIOR");
        let result = interp.get_obj_result();
        SCALAR_UPDATER_CALLS.with(|calls| calls.set(0));
        tcl_runtime_set_current_interp(&mut interp);
        assert_eq!(get_wide(value), (TCL_VALUE_GET_ERROR, i64::MIN));
        let first = interp.native_execution_refusal().unwrap();
        assert_eq!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation",
                ),
            )
        );
        SCALAR_UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
        assert_eq!(interp.runtime_context(), context);
        assert_eq!(obj::bytes_of(value), b"17");
        assert_eq!(obj::obj_type_ptr(value), &SCALAR_CONTEXT_TYPE as *const _);
        assert!(obj::native_scalar_cache(value).unwrap().is_none());
        assert_eq!((*value).ref_count, references);
        assert_eq!(interp.get_obj_result(), result);
        assert_eq!(interp.result_bytes(), b"PRIOR");
        assert_eq!(get_bool(value), (TCL_VALUE_GET_ERROR, -1));
        SCALAR_UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
        assert_eq!(interp.native_execution_refusal(), Some(first));
        tcl_runtime_set_current_interp(ptr::null_mut());
    });
}

#[cfg(debug_assertions)]
#[test]
fn entered_expression_number_updater_stops_before_cache_and_truth_output() {
    // Software stages: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|| unsafe {
        for production in [0, 1] {
            let mut interp = scalar_interpreter("tcl8.6");
            let original = obj::Owned::fresh(obj::alloc_typed(&SCALAR_CONTEXT_TYPE, 0));
            let value = original.as_ptr();
            let context = interp.runtime_context();
            let references = (*value).ref_count;
            interp.set_result_bytes(b"PRIOR");
            let result = interp.get_obj_result();
            SCALAR_UPDATER_CALLS.with(|calls| calls.set(0));
            tcl_runtime_set_current_interp(&mut interp);
            let mut out = -17;
            assert_eq!(
                tcl_value_get_expression_bool(value, production, &mut out),
                TCL_VALUE_GET_ERROR
            );
            assert_eq!(out, -17);
            let first = interp.native_execution_refusal().unwrap();
            assert_eq!(
                first,
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "stale entered native operation",
                    ),
                )
            );
            SCALAR_UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
            assert_eq!(interp.runtime_context(), context);
            assert_eq!(obj::bytes_of(value), b"17");
            assert_eq!(obj::obj_type_ptr(value), &SCALAR_CONTEXT_TYPE as *const _);
            assert!(obj::native_scalar_cache(value).unwrap().is_none());
            assert_eq!((*value).ref_count, references);
            assert_eq!(interp.get_obj_result(), result);
            assert_eq!(interp.result_bytes(), b"PRIOR");
            assert_eq!(
                tcl_value_get_expression_bool(value, production, &mut out),
                TCL_VALUE_GET_ERROR
            );
            assert_eq!(out, -17);
            SCALAR_UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
            assert_eq!(interp.native_execution_refusal(), Some(first));
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

/// A successful typed read caches the parsed rep onto the object — C's
/// `TclParseNumber` write-back — and keeps the spelling, so a hot loop
/// parses once and `puts` still prints what the script wrote.
#[test]
fn typed_value_reads_cache_the_parsed_rep_and_keep_the_spelling() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        let decimal = owned_word(b"12");
        assert!((*decimal).type_ptr.is_null(), "starts as a plain string");
        assert_eq!(get_wide(decimal), (TCL_VALUE_GET_OK, 12));
        assert!(
            std::ptr::eq((*decimal).type_ptr, &obj::TCL_INT_TYPE),
            "the parsed integer rep is cached back onto the object"
        );
        assert_eq!(obj_bytes(decimal), b"12", "the spelling survives");

        // A radix spelling keeps its own text, exactly as in C Tcl.
        let hex = owned_word(b"0x10");
        assert_eq!(get_wide(hex), (TCL_VALUE_GET_OK, 16));
        assert!(std::ptr::eq((*hex).type_ptr, &obj::TCL_INT_TYPE));
        assert_eq!(obj_bytes(hex), b"0x10");

        let scientific = owned_word(b"1e3");
        assert_eq!(get_double(scientific), (TCL_VALUE_GET_OK, 1000.0));
        assert!(std::ptr::eq((*scientific).type_ptr, &obj::TCL_DOUBLE_TYPE));
        assert_eq!(obj_bytes(scientific), b"1e3");

        // Now that it carries a double rep, the integer read reports C's
        // double-branch wording and code (`tclObj.c`).
        assert_eq!(get_wide(scientific).0, TCL_VALUE_GET_ERROR);
        assert_eq!(
            (*interp).result_bytes(),
            b"expected integer but got \"1e3\""
        );
        assert_eq!((*interp).error_code(), b"TCL VALUE INTEGER");

        release_words(&[decimal, hex, scientific]);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// Beyond-wide integers and non-numbers report C's exact message and
/// `-errorcode` through the interpreter, leaving the out storage untouched.
#[test]
fn typed_value_reads_report_overflow_and_non_numbers_like_c() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        // `tclsh9.0`: `string repeat a [expr {2**200}]` →
        // "integer value too large to represent" /
        // ARITH IOVERFLOW {integer value too large to represent}.
        let huge = owned_word(b"99999999999999999999999");
        let (status, untouched) = get_wide(huge);
        assert_eq!(status, TCL_VALUE_GET_ERROR);
        assert_eq!(untouched, i64::MIN, "the out storage is left alone");
        assert_eq!(
            (*interp).result_bytes(),
            b"integer value too large to represent"
        );
        assert_eq!(
            (*interp).error_code(),
            b"ARITH IOVERFLOW {integer value too large to represent}"
        );
        // The same value still widens to a double.
        assert_eq!(get_double(huge), (TCL_VALUE_GET_OK, 1e23));

        let word = owned_word(b"abc");
        assert_eq!(get_wide(word).0, TCL_VALUE_GET_ERROR);
        assert_eq!(
            (*interp).result_bytes(),
            b"expected integer but got \"abc\""
        );
        assert_eq!((*interp).error_code(), b"TCL VALUE NUMBER");
        assert_eq!(get_double(word).0, TCL_VALUE_GET_ERROR);
        assert_eq!(
            (*interp).result_bytes(),
            b"expected floating-point number but got \"abc\""
        );
        assert!(
            (*word).type_ptr.is_null(),
            "a refused read caches nothing onto the object"
        );

        // C renders a well-formed multi-token value as `a list`.
        let listy = owned_word(b"a b");
        assert_eq!(get_bool(listy).0, TCL_VALUE_GET_ERROR);
        assert_eq!(
            (*interp).result_bytes(),
            b"expected boolean value but got a list"
        );

        release_words(&[huge, word, listy]);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// The boolean read is the boolean-*context* acceptor over
/// `tcl_syntax::boolean`'s one word table: unique prefixes resolve, the
/// ambiguous `o` does not, and any number compares against zero.
#[test]
fn typed_value_boolean_read_uses_the_shared_word_table() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        for (spelling, expected) in [
            (&b"tru"[..], 1),
            (b"t", 1),
            (b"ye", 1),
            (b"YES", 1),
            (b"of", 0),
            (b"n", 0),
            (b"2", 1),
            (b"-1", 1),
            (b"0.0", 0),
            (b"0x0", 0),
        ] {
            let value = owned_word(spelling);
            assert_eq!(
                get_bool(value),
                (TCL_VALUE_GET_OK, expected),
                "{:?}",
                String::from_utf8_lossy(spelling)
            );
            release_words(&[value]);
        }

        // `on`/`off` share the prefix `o`, so tclsh rejects it.
        let ambiguous = owned_word(b"o");
        assert_eq!(get_bool(ambiguous).0, TCL_VALUE_GET_ERROR);
        assert_eq!(
            (*interp).result_bytes(),
            b"expected boolean value but got \"o\""
        );

        // `tclsh9.0`: `expr {NaN ? 1 : 0}` is a domain error, not truthy.
        let nan = owned_word(b"NaN");
        assert_eq!(get_bool(nan).0, TCL_VALUE_GET_ERROR);
        assert_eq!(
            (*interp).result_bytes(),
            b"floating point value is Not a Number"
        );
        assert_eq!(
            get_double(nan).0,
            TCL_VALUE_GET_ERROR,
            "the actual C9 primitive double read rejects cached NaN"
        );

        release_words(&[ambiguous, nan]);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

#[test]
fn actual_cached_getter_preserves_private_c85_code_and_exact_result_extent() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        (*interp).set_dialect_profile(crate::environment::profile_for_dialect("tcl8.5"));
        (*interp).set_error_state(b"KEEP CODE");
        let value = owned_word(b"1.5\0tail");
        assert_eq!(get_double(value), (TCL_VALUE_GET_OK, 1.5));
        assert_eq!(get_wide(value), (TCL_VALUE_GET_ERROR, i64::MIN));
        assert_eq!(
            (*interp).result_bytes(),
            b"expected integer but got \"1.5\0tail\""
        );
        assert_eq!((*interp).error_code(), b"KEEP CODE");
        assert!(core::ptr::eq(
            obj::obj_type_ptr(value),
            &obj::TCL_DOUBLE_TYPE
        ));
        assert!(!(*interp).host_refusal_pending());
        release_words(&[value]);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// The native constructors produce owned, typed objects whose string rep is
/// materialised only on demand.
#[test]
fn native_double_and_boolean_values_are_owned_and_typed() {
    leak_free(|| unsafe {
        let double = tcl_value_new_double(1.5);
        assert_eq!((*double).ref_count, 1, "generated code owns the result");
        assert!(std::ptr::eq((*double).type_ptr, &obj::TCL_DOUBLE_TYPE));
        assert!((*double).bytes.is_null(), "doubles begin without text");
        assert_eq!(obj_bytes(double), b"1.5");
        assert!(
            std::ptr::eq((*double).type_ptr, &obj::TCL_DOUBLE_TYPE),
            "materialising the text keeps the double internal rep"
        );
        tcl_obj_release(double);

        // C's `Tcl_NewBooleanObj` normalises any non-zero input to 1.
        let truthy = tcl_value_new_bool(7);
        assert_eq!((*truthy).ref_count, 1);
        assert_eq!(obj_bytes(truthy), b"1");
        tcl_obj_release(truthy);

        let falsy = tcl_value_new_bool(0);
        assert_eq!(obj_bytes(falsy), b"0");
        tcl_obj_release(falsy);
    });
}

/// A `catch` dispatched from generated code must read its **own** error
/// state. Before compiled activations existed, `tcl_invoke_argv` dispatched
/// at `eval_depth == 0`, so `catch`'s `eval_control_body` returned to depth
/// 0 and the eval loop's outermost rule published-and-reset the exception
/// before `catch` read `error_code()` — `-errorcode` came back `NONE`.
///
/// Oracle (`tclsh9.0`): `catch {error m NEGATIVE {MYERR NEG}} r opts` sets
/// `-errorcode` to `MYERR NEG` and `-errorinfo` to `NEGATIVE`.
#[test]
fn invoke_argv_catch_reads_the_raised_errorcode_at_compiled_top_level() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let words = [
            owned_word(b"catch"),
            owned_word(b"error m NEGATIVE {MYERR NEG}"),
            owned_word(b"r"),
            owned_word(b"opts"),
        ];

        let mut completion = invoke(&words);
        assert_eq!(completion.code, 0);
        assert_eq!(obj_bytes(completion.result), b"1", "catch reports an error");

        assert_eq!(tcl_eval_code(box_str(b"dict get $opts -errorcode")), 0);
        assert_eq!((*interp).result_bytes(), b"MYERR NEG");
        assert_eq!(tcl_eval_code(box_str(b"dict get $opts -errorinfo")), 0);
        assert_eq!((*interp).result_bytes(), b"NEGATIVE");
        assert_eq!(tcl_eval_code(box_str(b"set r")), 0);
        assert_eq!((*interp).result_bytes(), b"m");

        tcl_completion_release(&mut completion);
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// A compiled activation's matching leave retains the native error
/// objects. Public original-name reads reach their hidden core read hooks
/// without starting an eval loop or resetting the error episode.
#[test]
fn compiled_activation_leave_retains_native_errors_for_public_read() {
    // naming.runtime.compiled-procedure-fallback-and-source-log
    // docs/design/analysis/name-resolution-proofs/compiled-procedure-fallback-and-source-log.md
    leak_free(|| unsafe {
        let native = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl9.0"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let interp = Box::into_raw(Box::new(native));
        tcl_runtime_set_current_interp(interp);

        assert_eq!(tcl_codegen_activation_enter(), 0);
        let words = [owned_word(b"error"), owned_word(b"boom")];
        let mut completion = invoke(&words);
        assert_eq!(completion.code, 1);

        // The ordinary cell has no eager write. This is not a claim that
        // a public Read cannot reach an active native private exception.
        // An intervening eval would reset the error episode under test.
        assert!(
            (*interp).var_get(b"::errorInfo").is_none(),
            "no eager ordinary error-variable cell write"
        );

        tcl_codegen_activation_leave(1);
        let info_name = obj::Owned::fresh(obj::new_string_bytes(b"::errorInfo"));
        let published = (*interp)
            .read_original_named_variable(info_name.as_ptr())
            .expect("the public Read reaches the published native trace");
        assert!(obj_bytes(published).starts_with(b"boom"));
        let code_name = obj::Owned::fresh(obj::new_string_bytes(b"::errorCode"));
        let published = (*interp)
            .read_original_named_variable(code_name.as_ptr())
            .expect("the public Read reaches the published native error code");
        assert_eq!(obj_bytes(published), b"NONE");

        tcl_completion_release(&mut completion);
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

fn hold_actual_activation_bound() -> usize {
    // Reach the real entry bound without modifying interpreter depth fields.
    for entered in 0..1024 {
        if tcl_codegen_activation_enter() != 0 {
            assert!(entered > 0, "the genuine interpreter enters an activation");
            return entered;
        }
    }
    panic!("the bounded software fixture did not reach the actual activation refusal");
}

fn selected_c86_activation_interpreter() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[test]
fn activation_refusal_preserves_first_host_and_all_caller_owned_outputs() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // This reaches the actual software activation bound; it does not assert
    // a C Tcl nesting limit or an original provider callback chronology.
    leak_free(|| unsafe {
        let mut interp = selected_c86_activation_interpreter();
        tcl_runtime_set_current_interp(&raw mut interp);
        let entered = hold_actual_activation_bound();
        interp.set_result_bytes(b"PRIOR\0\xff");
        let prior = interp.result_obj();
        let first = tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable;
        interp.refuse_native_access(first);
        let words = [
            owned_word(b"string"),
            owned_word(b"length"),
            owned_word(b"abc"),
        ];
        let counts = words.map(|word| (*word).ref_count);
        let result = owned_word(b"CALLER RESULT");
        let options = owned_word(b"-caller YES");
        let output_counts = ((*result).ref_count, (*options).ref_count);
        let mut out = TclCompletionAbi {
            code: 77,
            result,
            options,
        };
        for intrinsic in [false, true] {
            let status = if intrinsic {
                tcl_intrinsic_invoke_argv(
                    IntrinsicId::StringLength.stable_id(),
                    words.as_ptr(),
                    3,
                    &raw mut out,
                )
            } else {
                tcl_invoke_argv(words.as_ptr(), 3, &raw mut out)
            };
            assert_eq!(status, TCL_INVOKE_ABI_HOST_REFUSED);
            assert_eq!(
                interp.native_execution_refusal(),
                Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    first
                ),)
            );
            assert_eq!(out.code, 77);
            assert_eq!(out.result, result);
            assert_eq!(out.options, options);
            assert_eq!(((*result).ref_count, (*options).ref_count), output_counts);
            assert_eq!(words.map(|word| (*word).ref_count), counts);
            assert_eq!(interp.result_obj(), prior);
            assert_eq!(interp.result_bytes(), b"PRIOR\0\xff");
        }
        for _ in 0..entered {
            tcl_codegen_activation_leave(0);
        }
        tcl_completion_release(&raw mut out);
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
    });
}

#[test]
fn original_refused_abi_activation_keeps_every_genuinely_held_entry() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software entry refusal and paired ownership only; this records
    // no C Tcl nesting limit or original provider callback chronology.
    leak_free(|| unsafe {
        let mut interp = selected_c86_activation_interpreter();
        tcl_runtime_set_current_interp(&raw mut interp);
        let entered = hold_actual_activation_bound();
        for _ in 0..3 {
            assert!(AbiActivation::enter(&raw mut interp).is_none());
            assert_ne!(
                tcl_codegen_activation_enter(),
                0,
                "a refused ABI attempt cannot retire a genuinely held entry"
            );
        }
        tcl_codegen_activation_leave(0);
        assert_eq!(
            tcl_codegen_activation_enter(),
            0,
            "only a matching leave makes room for the next actual entry"
        );
        tcl_codegen_activation_leave(0);
        for _ in 1..entered {
            tcl_codegen_activation_leave(0);
        }
        assert!(interp.native_execution_refusal().is_none());
        tcl_runtime_set_current_interp(ptr::null_mut());
    });
}

#[test]
fn activation_guest_refusal_keeps_its_real_owned_completion() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Both public routes reach the genuine software entry bound; this only
    // checks its Guest/Host separation and ownership, not a native depth claim.
    leak_free(|| unsafe {
        let mut interp = selected_c86_activation_interpreter();
        tcl_runtime_set_current_interp(&raw mut interp);
        let entered = hold_actual_activation_bound();
        let words = [
            owned_word(b"string"),
            owned_word(b"length"),
            owned_word(b"abc"),
        ];
        let counts = words.map(|word| (*word).ref_count);
        for intrinsic in [false, true] {
            let mut out = TclCompletionAbi {
                code: 77,
                result: ptr::null_mut(),
                options: ptr::null_mut(),
            };
            let status = if intrinsic {
                tcl_intrinsic_invoke_argv(
                    IntrinsicId::StringLength.stable_id(),
                    words.as_ptr(),
                    3,
                    &raw mut out,
                )
            } else {
                tcl_invoke_argv(words.as_ptr(), 3, &raw mut out)
            };
            assert_eq!(status, TCL_INVOKE_ABI_OK);
            assert_eq!(out.code, 1);
            assert!(!out.result.is_null());
            assert!(!out.options.is_null());
            assert_eq!(
                obj_bytes(out.result),
                b"too many nested evaluations (infinite loop?)"
            );
            assert_eq!(option(&out, b"-code"), b"1");
            assert!(interp.native_execution_refusal().is_none());
            assert_eq!(words.map(|word| (*word).ref_count), counts);
            tcl_completion_release(&raw mut out);
        }
        for _ in 0..entered {
            tcl_codegen_activation_leave(0);
        }
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
    });
}

/// Without a current interpreter the activation entry declines rather than
/// silently pretending to hold one, and the paired leave is a no-op.
#[test]
fn compiled_activation_declines_without_a_current_interpreter() {
    leak_free(|| {
        tcl_runtime_set_current_interp(ptr::null_mut());
        assert_ne!(tcl_codegen_activation_enter(), 0);
        tcl_codegen_activation_leave(0);
    });
}

/// Non-standard completion codes remain raw `i32`s at the ABI boundary,
/// with a matching return-options `-code` value.
#[test]
fn invoke_argv_preserves_custom_completion_code() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let words = [
            owned_word(b"return"),
            owned_word(b"-level"),
            owned_word(b"0"),
            owned_word(b"-code"),
            owned_word(b"73"),
            owned_word(b"custom result"),
        ];

        let mut completion = invoke(&words);
        assert_eq!(completion.code, 73);
        assert_eq!(obj_bytes(completion.result), b"custom result");
        assert_eq!(option(&completion, b"-code"), b"73");
        assert_eq!(option(&completion, b"-level"), b"0");

        tcl_completion_release(&mut completion);
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// Malformed ABI inputs do not read a null argv and still provide an owned
/// error completion whenever output storage is available.
#[test]
fn invoke_argv_validates_boundary_inputs_without_leaking() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let mut completion = empty_completion();

        assert_eq!(
            tcl_invoke_argv(ptr::null(), -1, &mut completion),
            TCL_INVOKE_ABI_INVALID_ARGC
        );
        assert_eq!(completion.code, 1);
        tcl_completion_release(&mut completion);

        assert_eq!(
            tcl_invoke_argv(ptr::null(), 1, &mut completion),
            TCL_INVOKE_ABI_NULL_ARGV
        );
        assert_eq!(completion.code, 1);
        tcl_completion_release(&mut completion);

        assert_eq!(
            tcl_invoke_argv(ptr::null(), 1, ptr::null_mut()),
            TCL_INVOKE_ABI_NULL_OUT
        );
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// The eval-fallback round-trip: box → eval → release, against the current
/// interp, leaves zero residual *and* the evaluated side effect is real
/// (the variable it set is observable on a second eval).
#[test]
fn eval_box_release_round_trip() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        let result = tcl_eval(box_str(b"set x 42"));
        assert_eq!(obj_bytes(result), b"42");
        tcl_obj_release(result);

        // The side effect persisted: reading the var back yields 42.
        let read = tcl_eval(box_str(b"set x"));
        assert_eq!(obj_bytes(read), b"42");
        tcl_obj_release(read);

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// `tcl_expr_bool` evaluates real Tcl boolean expressions and frees its
/// (adopted) operand object.
#[test]
#[cfg(have_tommath)]
fn expr_bool_true_and_false() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        assert_eq!(tcl_expr_bool(box_str(b"1 < 2")), 1);
        assert_eq!(tcl_expr_bool(box_str(b"5 == 0")), 0);
        assert_eq!(tcl_expr_bool(box_str(b"2 + 2 == 4")), 1);
        // A malformed expression is false, not a panic.
        assert_eq!(tcl_expr_bool(box_str(b"1 +")), 0);

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// `tcl_eval_code` reports the completion code of the evaluated script (so
/// the AOT control flow can honour it) and stays leak-balanced — the result
/// stays the interp's own, with no owned reference to release.
#[test]
fn eval_code_reports_completion_and_side_effects() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        // Ok (0): a plain command, and its side effect persisted.
        assert_eq!(tcl_eval_code(box_str(b"set x 7")), 0);
        let read = tcl_eval(box_str(b"set x"));
        assert_eq!(obj_bytes(read), b"7");
        tcl_obj_release(read);

        // Error (1), return (2), break (3), continue (4).
        assert_eq!(tcl_eval_code(box_str(b"error boom")), 1);
        assert_eq!(tcl_eval_code(box_str(b"return 9")), 2);
        assert_eq!(tcl_eval_code(box_str(b"break")), 3);
        assert_eq!(tcl_eval_code(box_str(b"continue")), 4);
        // `return -code error` (default -level 1) is a *deferred* return: it
        // completes with `return` (2) and the `-code` is applied only at a
        // proc/source boundary. `-level 0` applies the code immediately.
        assert_eq!(tcl_eval_code(box_str(b"return -code error boom")), 2);
        assert_eq!(tcl_eval_code(box_str(b"return -level 0 -code 42 x")), 42);

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// With no current interp set, the ABI stays leak-safe (an owned empty result
/// the caller releases; conditions are false; `tcl_eval_code` reports ok).
#[test]
fn no_current_interp_is_leak_safe() {
    leak_free(|| unsafe {
        tcl_runtime_set_current_interp(ptr::null_mut());
        let result = tcl_eval(box_str(b"set x 42"));
        assert_eq!(obj_bytes(result), b"");
        tcl_obj_release(result);
        assert_eq!(tcl_expr_bool(box_str(b"1 < 2")), 0);
        assert_eq!(tcl_eval_code(box_str(b"error boom")), 0);
    });
}

/// Call-frame storage is dynamically distinct, so a nested generated call
/// cannot overwrite the argv or completion storage held by its caller.
#[test]
fn call_frames_are_reentrant_and_layout_checked() {
    unsafe {
        let before = tcl_codegen_call_frame_outstanding();
        let outer = tcl_codegen_call_frame_alloc(32, 8);
        assert!(!outer.is_null());
        assert_eq!(tcl_codegen_call_frame_outstanding(), before + 1);
        outer.write_bytes(0xA5, 32);

        let inner = tcl_codegen_call_frame_alloc(48, 16);
        assert!(!inner.is_null());
        assert_ne!(outer, inner);
        assert_eq!(tcl_codegen_call_frame_outstanding(), before + 2);
        inner.write_bytes(0x5A, 48);
        assert_eq!(outer.read(), 0xA5);

        assert_eq!(tcl_codegen_call_frame_free(inner), 0);
        assert_eq!(tcl_codegen_call_frame_free(outer.wrapping_add(1)), -1);
        assert_eq!(tcl_codegen_call_frame_free(outer), 0);
        assert_eq!(tcl_codegen_call_frame_outstanding(), before);
        assert_eq!(tcl_codegen_call_frame_free(outer), -1);
        assert!(tcl_codegen_call_frame_alloc(0, 4).is_null());
        assert!(tcl_codegen_call_frame_alloc(8, 3).is_null());
    }
}

/// Model the frame an emitted leaf statement allocates: `slots` owned
/// object slots followed by `completions` completion records.
///
/// This mirrors `codegen::wasm::leaf_invoke`'s layout, so the round-trip
/// tests below exercise the same allocation, adoption, and release order
/// the emitted module performs. The emitter's byte offsets are the wasm32
/// ones fixed in `tcl-runtime-api`; a native test frame uses the host's own
/// pointer width for the same shape.
struct CompiledFrame {
    base: *mut u8,
    slots: usize,
}

impl CompiledFrame {
    const STRIDE: usize = core::mem::size_of::<*mut TclObj>();
    const ALIGN: usize =
        if core::mem::align_of::<TclCompletionAbi>() > core::mem::align_of::<*mut TclObj>() {
            core::mem::align_of::<TclCompletionAbi>()
        } else {
            core::mem::align_of::<*mut TclObj>()
        };

    fn new(slots: usize, completions: usize) -> Self {
        let objects = slots * Self::STRIDE;
        let completion_base = objects.next_multiple_of(Self::ALIGN);
        let bytes = completion_base + completions * core::mem::size_of::<TclCompletionAbi>();
        let base = tcl_codegen_call_frame_alloc(
            i32::try_from(bytes).expect("test frame fits i32"),
            i32::try_from(Self::ALIGN).expect("test frame alignment fits i32"),
        );
        assert!(!base.is_null());
        let frame = Self { base, slots };
        // The emitted prologue nulls every object slot so the single
        // cleanup path can release them null-safely.
        for slot in 0..slots {
            frame.store(slot, ptr::null_mut());
        }
        frame
    }

    fn slot_ptr(&self, slot: usize) -> *mut *mut TclObj {
        assert!(slot < self.slots);
        // SAFETY: `slot` is inside the allocation this frame owns.
        unsafe { self.base.add(slot * Self::STRIDE).cast::<*mut TclObj>() }
    }

    fn store(&self, slot: usize, value: *mut TclObj) {
        // SAFETY: the slot is inside this frame's allocation.
        unsafe { self.slot_ptr(slot).write(value) };
    }

    fn load(&self, slot: usize) -> *mut TclObj {
        // SAFETY: every slot was written by `new` before any read.
        unsafe { self.slot_ptr(slot).read() }
    }

    fn completion(&self, index: usize) -> *mut TclCompletionAbi {
        let base = (self.slots * Self::STRIDE).next_multiple_of(Self::ALIGN);
        // SAFETY: completion storage follows the object slots.
        unsafe {
            self.base
                .add(base + index * core::mem::size_of::<TclCompletionAbi>())
                .cast::<TclCompletionAbi>()
        }
    }

    /// The emitted epilogue: release every object slot, then free the frame.
    fn release(self) {
        for slot in 0..self.slots {
            // SAFETY: each slot is null or one owned reference.
            unsafe { tcl_obj_release(self.load(slot)) };
        }
        // SAFETY: the frame came from the matching allocator.
        assert_eq!(unsafe { tcl_codegen_call_frame_free(self.base) }, 0);
    }
}

/// Run one compiled invocation exactly as the emitter lays it out: dispatch
/// the argv run starting at `argv_slot`, stash the code, and adopt the
/// completion's owned result and options into their frame slots.
unsafe fn compiled_invoke(
    frame: &CompiledFrame,
    argv_slot: usize,
    argc: usize,
    completion: usize,
    result_slot: usize,
    options_slot: usize,
) -> i32 {
    let out = frame.completion(completion);
    // SAFETY: the argv run and completion record are inside the frame.
    unsafe {
        tcl_invoke_argv(
            frame.slot_ptr(argv_slot).cast_const(),
            i32::try_from(argc).expect("test argc fits i32"),
            out,
        );
        frame.store(result_slot, (*out).result);
        frame.store(options_slot, (*out).options);
        (*out).code
    }
}

/// `$a(b)` reads the array element, fires its read traces, and hands
/// generated code exactly one releasable reference.
#[test]
fn var_get_element_reads_an_array_element() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        assert_eq!(tcl_eval_code(box_str(b"set a(b) element")), 0);

        let value = tcl_codegen_var_get_element(b"a".as_ptr(), 1, b"b".as_ptr(), 1);
        assert!(!value.is_null());
        assert_eq!(obj_bytes(value), b"element");
        tcl_obj_release(value);

        // A missing element reports the C-faithful read error, not a scalar
        // miss on a name that happens to contain parentheses.
        assert!(tcl_codegen_var_get_element(b"a".as_ptr(), 1, b"z".as_ptr(), 1).is_null());
        assert_eq!(
            (*interp).result_bytes(),
            b"can't read \"a(z)\": no such element in array"
        );

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

#[test]
fn var_get_named_preserves_a_literal_element_key() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        assert_eq!(
            tcl_eval_code(box_str(
                b"set a(k) dynamic; set {a($key)} literal; set key k"
            )),
            0
        );
        let value = tcl_codegen_var_get(b"a($key)".as_ptr(), 7);
        assert!(!value.is_null());
        assert_eq!(obj_bytes(value), b"literal");
        tcl_obj_release(value);
        let value = tcl_codegen_var_get_element(b"a".as_ptr(), 1, b"k".as_ptr(), 1);
        assert!(!value.is_null());
        assert_eq!(obj_bytes(value), b"dynamic");
        tcl_obj_release(value);

        let value = owned_word(b"updated");
        assert_eq!(tcl_codegen_var_set(b"a($key)".as_ptr(), 7, value), 0);
        assert_eq!(tcl_eval_code(box_str(b"list ${a($key)} $a($key)")), 0);
        assert_eq!((*interp).result_bytes(), b"updated dynamic");

        assert!(tcl_codegen_var_get(b"a(missing)".as_ptr(), 10).is_null());
        assert_eq!(
            (*interp).result_bytes(),
            b"can't read \"a(missing)\": no such element in array"
        );
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

#[test]
fn named_element_store_runs_its_observer_before_reporting_the_error() {
    for engine in tcl_test_support::available_tclshs() {
        let trace = if engine.version == TclVersion::V8_4 {
            "trace variable {arr($i)} rw observer"
        } else {
            "trace add variable {arr($i)} {read write} observer"
        };
        let setup = format!(
            "set {{arr($i)}} OLD; set i k; set arr(k) DYNAMIC; set events {{}}; \
                 proc observer {{name key op}} {{lappend ::events [list $name $key $op]; \
                 if {{$op eq \"w\" || $op eq \"write\"}} {{error REJECT}}}}; {trace}"
        );
        let expected = tcl_test_support::run_script(
            &engine.path,
            format!(
                "{setup}; set before ${{arr($i)}}; \
                     set code [catch {{set {{arr($i)}} NEW}} result]; \
                     puts [list $before $code $result ${{arr($i)}} $arr($i) $events]\n"
            )
            .as_bytes(),
        )
        .expect("native named-element observer execution");
        assert!(
            expected.success() && expected.stderr.is_empty(),
            "{expected:?}"
        );
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            (*interp).set_runtime_version(engine.version);
            tcl_runtime_set_current_interp(interp);
            assert_eq!(tcl_eval_code(box_str(setup.as_bytes())), 0);
            let before = tcl_codegen_var_get(b"arr($i)".as_ptr(), 7);
            assert!(!before.is_null());
            let before_bytes = obj_bytes(before);
            tcl_obj_release(before);
            let code = tcl_codegen_var_set(b"arr($i)".as_ptr(), 7, owned_word(b"NEW"));
            let message = (*interp).result_bytes();
            let after = tcl_codegen_var_get(b"arr($i)".as_ptr(), 7);
            assert!(!after.is_null());
            let after_bytes = obj_bytes(after);
            tcl_obj_release(after);
            let sibling = tcl_codegen_var_get_element(b"arr".as_ptr(), 3, b"k".as_ptr(), 1);
            assert!(!sibling.is_null());
            let sibling_bytes = obj_bytes(sibling);
            tcl_obj_release(sibling);
            assert_eq!(tcl_eval_code(box_str(b"set events")), 0);
            let values = [
                before_bytes,
                code.to_string().into_bytes(),
                message,
                after_bytes,
                sibling_bytes,
                (*interp).result_bytes(),
            ];
            let mut observed = Vec::new();
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    observed.push(b' ');
                }
                crate::list::append_list_element(&mut observed, value, index == 0);
            }
            assert_eq!(
                observed,
                expected
                    .stdout
                    .strip_suffix(b"\n")
                    .unwrap_or(&expected.stdout),
                "{:?}",
                engine.path
            );
            assert!(!(*interp).host_refusal_pending());
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }
}

/// `tcl_codegen_word_concat` borrows its parts and returns one owned join.
#[test]
fn word_concat_borrows_its_parts() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let parts = [owned_word(b"hi "), owned_word(b"world")];
        let counts = parts.map(|part| (*part).ref_count);

        let joined = tcl_codegen_word_concat(parts.as_ptr(), 2);
        assert_eq!(obj_bytes(joined), b"hi world");
        assert_eq!((*joined).ref_count, 1);
        assert_eq!(parts.map(|part| (*part).ref_count), counts);
        tcl_obj_release(joined);

        let empty = tcl_codegen_word_concat(ptr::null(), 0);
        assert_eq!(obj_bytes(empty), b"");
        tcl_obj_release(empty);
        assert!(tcl_codegen_word_concat(ptr::null(), 1).is_null());

        release_words(&parts);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// The whole compiled leaf-statement shape — allocate one frame, evaluate
/// every word into it, dispatch, adopt the completion, release everything —
/// balances its allocations and leaves no outstanding call frame.
#[test]
fn compiled_leaf_statement_round_trip_is_leak_free() {
    let frames_before = tcl_codegen_call_frame_outstanding();
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        assert_eq!(tcl_eval_code(box_str(b"set value abc")), 0);

        // `string length $value`: three argv slots, then the root result
        // and options, and one completion record.
        let frame = CompiledFrame::new(5, 1);
        frame.store(0, tcl_obj_new_string_owned(b"string".as_ptr(), 6));
        frame.store(1, tcl_obj_new_string_owned(b"length".as_ptr(), 6));
        frame.store(2, tcl_codegen_var_get(b"value".as_ptr(), 5));
        assert!(!frame.load(2).is_null());

        let code = compiled_invoke(&frame, 0, 3, 0, 3, 4);
        assert_eq!(code, 0);
        assert_eq!(obj_bytes(frame.load(3)), b"3");
        frame.release();

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
    assert_eq!(tcl_codegen_call_frame_outstanding(), frames_before);
}

/// A nested `[…]` word adopts the inner completion's owned result as the
/// outer word's value; both invocations share one frame and one cleanup.
#[test]
fn compiled_nested_word_round_trip_is_leak_free() {
    let frames_before = tcl_codegen_call_frame_outstanding();
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        // `string length [string tolower ABC]`:
        //   slots 0..2  outer argv (slot 2 receives the nested result)
        //   slot  3     outer options, slot 4 outer result
        //   slots 5..7  nested argv, slot 8 nested options
        let frame = CompiledFrame::new(9, 2);
        frame.store(0, tcl_obj_new_string_owned(b"string".as_ptr(), 6));
        frame.store(1, tcl_obj_new_string_owned(b"length".as_ptr(), 6));
        frame.store(5, tcl_obj_new_string_owned(b"string".as_ptr(), 6));
        frame.store(6, tcl_obj_new_string_owned(b"tolower".as_ptr(), 7));
        frame.store(7, tcl_obj_new_string_owned(b"ABC".as_ptr(), 3));

        assert_eq!(compiled_invoke(&frame, 5, 3, 1, 2, 8), 0);
        assert_eq!(obj_bytes(frame.load(2)), b"abc");
        assert_eq!(compiled_invoke(&frame, 0, 3, 0, 4, 3), 0);
        assert_eq!(obj_bytes(frame.load(4)), b"3");
        frame.release();

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
    assert_eq!(tcl_codegen_call_frame_outstanding(), frames_before);
}

/// The abrupt-completion paths are the ones that can leak: the statement
/// leaves part-way through word evaluation, or the invocation itself
/// completes `error` / `break` / `return`. Every one still runs the single
/// cleanup path, so the counters and the frame ledger both balance.
#[test]
fn compiled_abrupt_completions_release_every_partial_word() {
    let frames_before = tcl_codegen_call_frame_outstanding();
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        // A missing variable aborts mid-argv: two words are already owned
        // by the frame and the third read failed.
        let frame = CompiledFrame::new(5, 1);
        frame.store(0, tcl_obj_new_string_owned(b"string".as_ptr(), 6));
        frame.store(1, tcl_obj_new_string_owned(b"length".as_ptr(), 6));
        frame.store(2, tcl_codegen_var_get(b"missing".as_ptr(), 7));
        assert!(frame.load(2).is_null(), "the read must report an error");
        frame.release();

        // `error`, `break`, and `return` all complete through the same
        // adopt-then-release path before the emitted dispatch branches.
        for (command, expected) in [(&b"error"[..], 1), (&b"break"[..], 3), (&b"return"[..], 2)] {
            let frame = CompiledFrame::new(3, 1);
            frame.store(
                0,
                tcl_obj_new_string_owned(
                    command.as_ptr(),
                    i32::try_from(command.len()).expect("test word fits i32"),
                ),
            );
            assert_eq!(compiled_invoke(&frame, 0, 1, 0, 1, 2), expected);
            frame.release();
        }

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
    assert_eq!(tcl_codegen_call_frame_outstanding(), frames_before);
}

/// The argv constructor transfers exactly one owned reference, which is
/// the reference the generated cleanup path releases after dispatch.
#[test]
fn owned_string_constructor_has_one_releasable_reference() {
    leak_free(|| unsafe {
        let word = tcl_obj_new_string_owned(b"word".as_ptr(), 4);
        assert_eq!((*word).ref_count, 1);
        assert_eq!(obj_bytes(word), b"word");
        tcl_obj_release(word);
    });
}

// native proc entries

thread_local! {
    /// The argv [`stub_body`] dispatches, and how many times a stub ran.
    static STUB_ARGV: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    static STUB_CALLS: Cell<u32> = const { Cell::new(0) };
    /// `(line, source text)` [`stub_body`] logs on an error completion, as
    /// the emitter will on a statement's error edge. `None` logs nothing.
    static STUB_LOG: RefCell<Option<(i32, Vec<u8>)>> = const { RefCell::new(None) };
}

/// Arm the stub with the one command its body runs, and reset the counter.
fn stub_argv(words: &[&[u8]]) {
    STUB_ARGV.with(|argv| {
        *argv.borrow_mut() = words.iter().map(|word| word.to_vec()).collect();
    });
    STUB_LOG.with(|log| *log.borrow_mut() = None);
    STUB_CALLS.with(|calls| calls.set(0));
}

/// Give the armed stub the statement site it logs when its command fails.
fn stub_logs(line: i32, source: &[u8]) {
    STUB_LOG.with(|log| *log.borrow_mut() = Some((line, source.to_vec())));
}

fn stub_calls() -> u32 {
    STUB_CALLS.with(Cell::get)
}

/// A stub compiled proc body.
///
/// It dispatches one prebuilt argv through the same [`tcl_invoke_argv`] an
/// emitted body uses for a statement it could not lower, and forwards that
/// completion as its own. Nothing here is test-shaped: the completion
/// protocol, the borrowed argv, and the absent frame/activation prologue
/// are exactly what `native_emit` must emit for a proc entry.
unsafe extern "C" fn stub_body(
    _argv: *const *mut TclObj,
    _argc: i32,
    out: *mut TclCompletionAbi,
) -> i32 {
    STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
    let words = STUB_ARGV.with(|argv| argv.borrow().clone());
    // SAFETY: each word is a valid readable slice; `owned_word` boxes it
    // and takes the caller-owned reference the borrowed-argv ABI needs.
    let boxed: Vec<*mut TclObj> = words
        .iter()
        .map(|word| unsafe { owned_word(word) })
        .collect();
    // SAFETY: `boxed` are live caller-owned words and `out` is the
    // runtime's own zeroed completion storage.
    let status = unsafe {
        tcl_invoke_argv(
            boxed.as_ptr(),
            i32::try_from(boxed.len()).expect("stub argv fits i32"),
            out,
        )
    };
    assert_eq!(status, TCL_INVOKE_ABI_OK);
    // SAFETY: balances `owned_word`'s reference on each word.
    unsafe { release_words(&boxed) };
    // The statement's error edge: an emitted body logs its own source site
    // here, because nothing else will.
    // SAFETY: `out` was written by the invoke above.
    if unsafe { (*out).code } == 1 {
        if let Some((line, source)) = STUB_LOG.with(|log| log.borrow().clone()) {
            // SAFETY: `source` is a live readable slice.
            unsafe {
                tcl_codegen_log_command(
                    line,
                    source.as_ptr(),
                    i32::try_from(source.len()).expect("source fits i32"),
                );
            }
        }
    }
    NATIVE_PROC_STATUS_RAN
}

/// A stub in the shape the emitter produces for a body whose last command
/// leaves no result of its own — a bare `return`, or a `puts`.
///
/// It reports success with `out` untouched, which the ABI defines as "the
/// runtime's own result is the body's answer".
unsafe extern "C" fn stub_no_result(
    _argv: *const *mut TclObj,
    _argc: i32,
    _out: *mut TclCompletionAbi,
) -> i32 {
    STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
    NATIVE_PROC_STATUS_RAN
}

/// A stub in the shape the emitter produces for a plain `return v`: it
/// records the pending return state the `return` command records, then
/// completes with the raw `Code::Return` the interpreted body reports.
unsafe extern "C" fn stub_plain_return(
    _argv: *const *mut TclObj,
    _argc: i32,
    out: *mut TclCompletionAbi,
) -> i32 {
    STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
    tcl_codegen_return_state(1, Code::Ok.as_int() as i32);
    // SAFETY: `out` is the runtime's own zeroed completion storage, and
    // `owned_word` hands over one owned reference on the result.
    unsafe {
        (*out).code = Code::Return.as_int() as i32;
        (*out).result = owned_word(b"v");
    }
    NATIVE_PROC_STATUS_RAN
}

/// The same stub without the state write — what an emitter that treated
/// `Code::Return` as nothing but a completion code would produce.
unsafe extern "C" fn stub_return_without_state(
    _argv: *const *mut TclObj,
    _argc: i32,
    out: *mut TclCompletionAbi,
) -> i32 {
    STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
    // SAFETY: as above.
    unsafe {
        (*out).code = Code::Return.as_int() as i32;
        (*out).result = owned_word(b"v");
    }
    NATIVE_PROC_STATUS_RAN
}

/// A stub that declines before doing anything at all — the only shape a
/// decline may take, since the runtime then runs the source body in the
/// frame it already prepared.
pub(super) unsafe extern "C" fn stub_declines(
    _argv: *const *mut TclObj,
    _argc: i32,
    _out: *mut TclCompletionAbi,
) -> i32 {
    STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
    NATIVE_PROC_STATUS_DECLINED
}

/// The call an emitted module makes for a `proc` statement.
unsafe fn define_native(name: &[u8], params: &[u8], body: &[u8], entry: Option<NativeProcEntry>) {
    // SAFETY: all three ranges are live readable slices.
    let code = unsafe {
        tcl_codegen_proc_define_native(
            name.as_ptr(),
            i32::try_from(name.len()).expect("name fits i32"),
            params.as_ptr(),
            i32::try_from(params.len()).expect("params fit i32"),
            body.as_ptr(),
            i32::try_from(body.len()).expect("body fits i32"),
            entry,
        )
    };
    assert_eq!(code, 0, "defining {}", String::from_utf8_lossy(name));
}

/// Evaluate `script` and return its completion code and result text.
unsafe fn eval(interp: *mut Interp, script: &str) -> (Code, Vec<u8>) {
    // SAFETY: the caller holds a live interpreter.
    unsafe {
        (
            (*interp).eval_str(script.as_bytes()),
            (*interp).result_bytes(),
        )
    }
}

/// A fresh interpreter, installed as the current one for the codegen ABI.
unsafe fn with_current_interp(body: impl FnOnce(*mut Interp)) {
    let interp = tcl_runtime_create_interp();
    // SAFETY: a live interpreter this helper owns for the call's duration.
    unsafe {
        tcl_runtime_set_current_interp(interp);
        body(interp);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    }
}

#[test]
fn a_native_entry_runs_instead_of_the_source_body_and_is_counted() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            assert_eq!(tcl_codegen_native_proc_dispatches(), 0);
            stub_argv(&[b"return", b"native"]);
            define_native(b"p", b"a", b"return source", Some(stub_body));

            assert_eq!(eval(interp, "p x"), (Code::Ok, b"native".to_vec()));
            assert_eq!(stub_calls(), 1);
            assert_eq!(tcl_codegen_native_proc_dispatches(), 1);

            // The source body is still the proc's body: it is what a step
            // trace forces, what a decline falls back to, and what
            // introspection reports.
            assert_eq!(eval(interp, "info body p").1, b"return source");
            assert_eq!(eval(interp, "info args p").1, b"a");
        });
    });
}

#[test]
fn proc_register_is_proc_define_native_with_no_entry() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            let (name, params, body) = (b"only", b"a", b"return source");
            assert_eq!(
                tcl_codegen_proc_register(
                    name.as_ptr(),
                    i32::try_from(name.len()).unwrap(),
                    params.as_ptr(),
                    i32::try_from(params.len()).unwrap(),
                    body.as_ptr(),
                    i32::try_from(body.len()).unwrap(),
                ),
                0
            );
            assert_eq!(eval(interp, "only x"), (Code::Ok, b"source".to_vec()));
            assert_eq!(tcl_codegen_native_proc_dispatches(), 0);
        });
    });
}

#[test]
fn the_native_entry_runs_in_the_call_frame_run_proc_prepared() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            // `info level 0` is the double-frame detector: it reads the
            // *current* frame's recorded invocation words, and only the
            // frame `run_proc` pushed has any. An entry that pushed its
            // own would report the wrong level and have no words at all.
            stub_argv(&[b"info", b"level", b"0"]);
            define_native(b"lvl", b"a b", b"return source", Some(stub_body));
            assert_eq!(eval(interp, "lvl 7 8").1, b"lvl 7 8");

            stub_argv(&[b"info", b"level"]);
            define_native(b"depth", b"", b"return source", Some(stub_body));
            assert_eq!(eval(interp, "depth").1, b"1");

            // Formals are bound by name before the entry runs, so the body
            // reads them as ordinary cells.
            stub_argv(&[b"set", b"b"]);
            define_native(b"second", b"a b", b"return source", Some(stub_body));
            assert_eq!(eval(interp, "second 7 8").1, b"8");

            // The body runs in the proc's defining namespace.
            assert_eq!(eval(interp, "namespace eval ns {}").0, Code::Ok);
            stub_argv(&[b"namespace", b"current"]);
            define_native(b"ns::inner", b"", b"return source", Some(stub_body));
            assert_eq!(eval(interp, "ns::inner").1, b"::ns");
        });
    });
}

#[test]
fn wrong_number_of_arguments_never_reaches_the_native_entry() {
    // The message is `run_proc`'s, produced before any body runs, so the
    // native and source paths cannot differ. tclsh 9.0.4 and 8.6.16 both
    // print `wrong # args: should be "greet name"`.
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            stub_argv(&[b"return", b"native"]);
            define_native(b"greet", b"name", b"return source", Some(stub_body));

            assert_eq!(
                eval(interp, "greet"),
                (
                    Code::Error,
                    b"wrong # args: should be \"greet name\"".to_vec()
                )
            );
            assert_eq!(
                eval(interp, "greet a b"),
                (
                    Code::Error,
                    b"wrong # args: should be \"greet name\"".to_vec()
                )
            );
            assert_eq!(stub_calls(), 0, "the entry must never see a bad arity");
            assert_eq!(tcl_codegen_native_proc_dispatches(), 0);
        });
    });
}

/// A multi-line body whose error carries a `(procedure ... line N)` frame.
const BOOM_BODY: &[u8] = b"\n    set b 1\n    error \"bad $a\"\n";

#[test]
fn a_declining_entry_falls_back_to_the_source_body_observably_unchanged() {
    // naming.runtime.compiled-procedure-fallback-and-source-log
    // docs/design/analysis/name-resolution-proofs/compiled-procedure-fallback-and-source-log.md
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            // The same proc, same name, same body — once with a
            // declining entry and once with none. Comparing under one name
            // is what makes the comparison total: the proc name appears in
            // the `(procedure ...)` frame and in the caller's own.
            stub_argv(&[]);
            define_native(b"boom", b"a", BOOM_BODY, Some(stub_declines));
            assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
            let declined_msg = eval(interp, "set m").1;
            let declined_info = eval(interp, "dict get $o -errorinfo").1;
            let declined_stack = eval(interp, "dict get $o -errorstack").1;
            let declined_code = eval(interp, "dict get $o -errorcode").1;
            assert_eq!(stub_calls(), 1, "the entry was asked, and declined");
            assert_eq!(
                tcl_codegen_native_proc_dispatches(),
                0,
                "a decline is not a native run"
            );

            define_native(b"boom", b"a", BOOM_BODY, None);
            assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
            assert_eq!(eval(interp, "set m").1, declined_msg);
            assert_eq!(eval(interp, "dict get $o -errorinfo").1, declined_info);
            assert_eq!(eval(interp, "dict get $o -errorstack").1, declined_stack);
            assert_eq!(eval(interp, "dict get $o -errorcode").1, declined_code);

            // And that shared answer is the interpreted one: tclsh 9.0.4
            // and 8.6.16 print exactly these five lines.
            assert_eq!(declined_msg, b"bad Q");
            assert_eq!(
                String::from_utf8_lossy(&declined_info),
                "bad Q\n    while executing\n\"error \"bad $a\"\"\n    \
                     (procedure \"boom\" line 3)\n    invoked from within\n\"boom Q\""
            );
        });
    });
}

#[test]
fn redefining_the_proc_drops_the_native_entry() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            stub_argv(&[b"return", b"native"]);
            define_native(b"twice", b"x", b"return source", Some(stub_body));
            assert_eq!(eval(interp, "twice 4").1, b"native");
            assert_eq!(tcl_codegen_native_proc_dispatches(), 1);

            // The `proc` command builds a definition with no entry, so the
            // new source body runs interpreted. Nothing invalidates
            // anything: the entry lived on the definition that just went.
            assert_eq!(
                eval(interp, "proc twice {x} {return redefined}").0,
                Code::Ok
            );
            assert_eq!(eval(interp, "twice 4").1, b"redefined");
            assert_eq!(tcl_codegen_native_proc_dispatches(), 1);
            assert_eq!(stub_calls(), 1);
        });
    });
}

#[test]
fn rename_carries_the_native_entry_and_deleting_the_proc_drops_it() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            stub_argv(&[b"return", b"native"]);
            define_native(b"twice", b"x", b"return source", Some(stub_body));

            assert_eq!(eval(interp, "rename twice thrice").0, Code::Ok);
            assert_eq!(eval(interp, "thrice 4").1, b"native");
            assert_eq!(tcl_codegen_native_proc_dispatches(), 1);
            assert_eq!(eval(interp, "catch {twice 4} m"), (Code::Ok, b"1".to_vec()));
            assert_eq!(eval(interp, "set m").1, b"invalid command name \"twice\"");

            assert_eq!(eval(interp, "rename thrice {}").0, Code::Ok);
            assert_eq!(eval(interp, "catch {thrice 4} m").1, b"1");
            assert_eq!(eval(interp, "set m").1, b"invalid command name \"thrice\"");

            // A whole namespace going away takes its definitions, and the
            // entries on them, with it.
            assert_eq!(eval(interp, "namespace eval ns {}").0, Code::Ok);
            define_native(b"ns::inner", b"", b"return source", Some(stub_body));
            assert_eq!(eval(interp, "ns::inner").1, b"native");
            assert_eq!(eval(interp, "namespace delete ns").0, Code::Ok);
            assert_eq!(eval(interp, "catch {ns::inner} m").1, b"1");
            assert_eq!(
                eval(interp, "set m").1,
                b"invalid command name \"ns::inner\""
            );
            assert_eq!(tcl_codegen_native_proc_dispatches(), 2);
        });
    });
}

#[test]
fn a_live_step_trace_forces_the_source_body() {
    // A natively lowered `set`/`list`/`return` never reaches `dispatch`,
    // so it would fire no `enterstep` and the transcript would silently
    // lose lines. `dispatch_traced` installs the proc's own step traces
    // before dispatching, so one read of `step_active` in `run_proc`
    // covers both this proc's traces and an outer one's.
    //
    // Transcript pinned against tclsh 9.0.4 and 8.6.16: step traces fire
    // on the *substituted* command, after its `[...]` words have run.
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            stub_argv(&[b"return", b"NATIVE"]);
            define_native(
                b"stepped",
                b"x",
                b"set y $x; return [list $y $y]",
                Some(stub_body),
            );
            assert_eq!(eval(interp, "stepped ab").1, b"NATIVE");
            assert_eq!(tcl_codegen_native_proc_dispatches(), 1);

            assert_eq!(
                eval(
                    interp,
                    "trace add execution stepped enterstep \
                         {apply {{cmd op} {lappend ::steps $cmd}}}"
                )
                .0,
                Code::Ok
            );
            assert_eq!(
                eval(interp, "stepped ab").1,
                b"ab ab",
                "the traced call must run the source body"
            );
            assert_eq!(
                eval(interp, "set ::steps").1,
                b"{set y ab} {list ab ab} {return {ab ab}}"
            );
            assert_eq!(
                tcl_codegen_native_proc_dispatches(),
                1,
                "the traced call must have run the source body"
            );
            assert_eq!(stub_calls(), 1);

            // Removing the trace restores the compiled body.
            assert_eq!(
                eval(
                    interp,
                    "trace remove execution stepped enterstep \
                         {apply {{cmd op} {lappend ::steps $cmd}}}"
                )
                .0,
                Code::Ok
            );
            assert_eq!(eval(interp, "stepped ab").1, b"NATIVE");
            assert_eq!(tcl_codegen_native_proc_dispatches(), 2);
        });
    });
}

/// An error out of a compiled body takes `run_proc`'s ordinary error tail
/// — with the two gaps a compiled statement's missing `log_command_info`
/// leaves, both pinned here as a gate that fails in either direction.
#[test]
fn an_error_from_the_native_body_unwinds_through_the_procedure_frame() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            stub_argv(&[b"error", b"bad Q"]);
            define_native(b"boom", b"a", BOOM_BODY, Some(stub_body));

            assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
            assert_eq!(eval(interp, "set m").1, b"bad Q");
            // `run_proc`'s error tail is shared by both bodies: the
            // `(procedure ...)` frame and the TIP 348 `CALL` entry are
            // appended exactly as they are for the source body.
            let info = eval(interp, "dict get $o -errorinfo").1;
            assert!(
                info.starts_with(b"bad Q"),
                "errorInfo seeds from the message: {}",
                String::from_utf8_lossy(&info)
            );
            assert!(
                info.windows(26)
                    .any(|w| w == b"(procedure \"boom\" line 1)\n"),
                "errorInfo must carry the procedure frame: {}",
                String::from_utf8_lossy(&info)
            );
            // A compiled statement calls no `log_command_info`, and that
            // one absence has two visible consequences here, both closed
            // by `tcl_codegen_log_command` once the emitter calls it on a
            // statement's error edge:
            //
            //  - no `while executing "<source text>"` frame, and
            //    `error_line` never advances, so the procedure frame reads
            //    `line 1` where the interpreted body reads `line 3`;
            //  - no TIP 348 `CALL` entry either. `error_stack_push_call`
            //    still runs, but it deliberately refuses to chain a `CALL`
            //    onto an error stack with no inner context yet
            //    (`reset_error_stack`), so the errorstack holds only the
            //    caller's own `INNER`.
            assert!(
                !info.windows(16).any(|w| w == b"while executing "),
                "unexpected `while executing` frame: {}",
                String::from_utf8_lossy(&info)
            );
            assert_eq!(eval(interp, "dict get $o -errorstack").1, b"INNER {boom Q}");
        });
    });
}

/// Source logging reproduces the public source trace. A generic argv
/// invocation retains its own inner error-stack context, independently from
/// the original body's selected native instruction.
#[test]
fn a_logged_statement_site_preserves_the_source_trace_without_instruction_identity() {
    // naming.runtime.compiled-procedure-fallback-and-source-log
    // docs/design/analysis/name-resolution-proofs/compiled-procedure-fallback-and-source-log.md
    // This ABI stub exercises source logging, not native returnImm admission.
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            define_native(b"boom", b"a", BOOM_BODY, None);
            assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
            let source_info = eval(interp, "dict get $o -errorinfo").1;
            let source_stack = eval(interp, "dict get $o -errorstack").1;

            // `error "bad $a"` is the third line of the body and the exact
            // text of the failing statement — what the emitter carries.
            stub_argv(&[b"error", b"bad Q"]);
            stub_logs(3, b"error \"bad $a\"");
            define_native(b"boom", b"a", BOOM_BODY, Some(stub_body));
            assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
            assert_eq!(tcl_codegen_native_proc_dispatches(), 1);

            assert_eq!(eval(interp, "dict get $o -errorinfo").1, source_info);
            assert_eq!(
                source_stack, b"INNER {returnImm {bad Q} {}} CALL {boom Q}",
                "the source body owns its selected native instruction context"
            );
            assert_eq!(
                eval(interp, "dict get $o -errorstack").1,
                b"INNER {error \"bad $a\"} CALL {boom Q}",
                "logging source text does not grant an instruction context"
            );
            assert_eq!(
                String::from_utf8_lossy(&source_info),
                "bad Q\n    while executing\n\"error \"bad $a\"\"\n    \
                     (procedure \"boom\" line 3)\n    invoked from within\n\"boom Q\"",
                "and that shared answer is tclsh 9.0.4's and 8.6.16's"
            );
        });
    });
}

#[test]
fn a_logged_command_is_truncated_and_deduplicated_like_an_interpreted_one() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            // One `log_command_bytes` protocol for both callers: the first
            // frame seeds errorInfo from the message and truncates the
            // command at 150 bytes with `...`; a second call for the same
            // error is a no-op (`already_logged`).
            let long = format!("error nope ;# {}", "x".repeat(200));
            stub_argv(&[b"error", b"nope"]);
            stub_logs(1, long.as_bytes());
            define_native(b"long", b"", b"error nope", Some(stub_body));
            assert_eq!(eval(interp, "catch {long} m o").1, b"1");
            let info = eval(interp, "dict get $o -errorinfo").1;
            let expected = format!(
                "nope\n    while executing\n\"{}...\"\n    (procedure \"long\" line 1)",
                &long[..150]
            );
            assert!(
                info.starts_with(expected.as_bytes()),
                "{}",
                String::from_utf8_lossy(&info)
            );
        });
    });
}

#[test]
fn the_native_path_holds_one_activation_per_level_and_refuses_at_the_bound() {
    // `run_native_body` holds the activation the eval loop would hold, and
    // the body's own `tcl_invoke_argv` holds one more: two eval-depth
    // units per Tcl level, the same as the interpreted plain-recursion
    // shape. A level that cost more would fail sooner and one that cost
    // less would fail later, so the reached depth pins the accounting.
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            stub_argv(&[b"recur"]);
            define_native(b"recur", b"", b"return source", Some(stub_body));

            assert_eq!(eval(interp, "catch {recur} m").1, b"1");
            assert_eq!(
                eval(interp, "set m").1,
                b"too many nested evaluations (infinite loop?)",
                "the refusal is the eval loop's own message"
            );
            assert_eq!(
                tcl_codegen_native_proc_dispatches(),
                63,
                "the outermost script and the `catch` body cost one \
                     eval-depth unit each, leaving 126 of the native bound's \
                     128 for 63 levels at two apiece"
            );
        });
    });
}

/// A completion that comes back with no result answers with the empty
/// string, never the caller's.
///
/// `Tcl_EvalEx` resets the result at entry and the eval loop can skip that
/// because its first command always sets one. A compiled body cannot make
/// that promise — `tcl_codegen_var_set` and `tcl_codegen_puts` set no
/// interpreter result — so `run_native_body` resets it instead. Without
/// that a `puts`-ending body would answer with whatever the caller's last
/// command left, which is the kind of wrong answer no result assertion on
/// the body itself would catch.
#[test]
fn a_native_entry_that_writes_no_result_answers_with_the_empty_string() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            define_native(b"quiet", b"", b"return source", Some(stub_no_result));
            assert_eq!(eval(interp, "set marker abc"), (Code::Ok, b"abc".to_vec()));
            assert_eq!(eval(interp, "quiet"), (Code::Ok, Vec::new()));
            assert_eq!(stub_calls(), 1);
            // The caller's own live result is the interesting case: the
            // list's first element sets it, and the second must not report
            // it back.
            assert_eq!(
                eval(interp, "list [set marker] [quiet]").1,
                b"abc {}".to_vec()
            );
        });
    });
}

/// A compiled `return` has to record the pending return state, because
/// the return boundary consumes it whether or not anything set it.
///
/// `catch {return -level 2 …}` leaves `(level 2, code error)` behind:
/// `catch` reports the code without crossing a boundary, so nothing
/// decrements it. An entry that then completes with `Code::Return` and no
/// state write has its level counted down from 2 instead of 1, and the
/// call propagates `Code::Return` to its caller instead of returning `v`.
/// tclsh 9.0.4 and 8.6.16 both answer `0 v` here.
#[test]
fn a_compiled_return_records_the_state_the_return_command_records() {
    leak_free(|| unsafe {
        with_current_interp(|interp| {
            define_native(b"p", b"", b"return source", Some(stub_plain_return));
            define_native(
                b"bad",
                b"",
                b"return source",
                Some(stub_return_without_state),
            );

            // Poison the pending state exactly as a caught deferred
            // return does, then ask each entry for its answer.
            let poison = "catch {return -level 2 -code error boom}";
            assert_eq!(eval(interp, poison).1, b"2");
            assert_eq!(eval(interp, "list [catch {p} m] $m").1, b"0 v");

            assert_eq!(eval(interp, poison).1, b"2");
            assert_eq!(
                eval(interp, "list [catch {bad} m] $m").1,
                b"2 v",
                "without the state write the boundary consumes the stale \
                     level and the call propagates a return"
            );
        });
    });
}

#[test]
fn the_dispatch_counter_reports_a_missing_interpreter_distinctly() {
    tcl_runtime_set_current_interp(ptr::null_mut());
    assert_eq!(tcl_codegen_native_proc_dispatches(), -1);
}

#[test]
fn the_runtime_states_its_identity_to_a_host() {
    use tcl_runtime_api::codegen_abi::CODEGEN_ABI_VERSION;
    use tcl_runtime_api::manifest::EMBEDDED_STDLIB_REVISION;
    use tcl_runtime_api::ArtefactIdentityManifest;

    // SAFETY: every buffer is as long as the capacity it is passed with.
    unsafe {
        tcl_runtime_set_current_interp(ptr::null_mut());
        assert_eq!(
            tcl_runtime_identity(ptr::null_mut(), 0),
            0,
            "no interp is current, so there is nothing to state"
        );

        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        (*interp).set_runtime_version(TclVersion::V8_6);
        let needed = tcl_runtime_identity(ptr::null_mut(), 0);
        assert!(needed > 0);

        let mut short = vec![0xAA_u8; usize::try_from(needed).unwrap() - 1];
        assert_eq!(tcl_runtime_identity(short.as_mut_ptr(), needed - 1), needed);
        assert!(
            short.iter().all(|byte| *byte == 0xAA),
            "a short buffer is not written"
        );

        let mut buffer = vec![0_u8; usize::try_from(needed).unwrap()];
        assert_eq!(tcl_runtime_identity(buffer.as_mut_ptr(), needed), needed);
        let manifest = ArtefactIdentityManifest::from_bytes(&buffer).expect("it decodes");
        assert_eq!(manifest, (*interp).held_identity());
        assert_eq!(manifest.environment, "tcl8.6");
        assert_eq!(manifest.release, "8.6");
        assert_eq!(manifest.abi_version, CODEGEN_ABI_VERSION);
        assert_eq!(
            manifest.intrinsic_table_hash,
            tcl_registry::intrinsic_table_hash()
        );
        assert_eq!(manifest.embedded_stdlib_revision, EMBEDDED_STDLIB_REVISION);
        assert!(manifest.packs.is_empty());

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    }
}
unsafe fn define_guarded_words(words: &[*mut TclObj], expected: GuardIdentity) -> TclCompletionAbi {
    let mut out = TclCompletionAbi {
        code: 777,
        result: ptr::null_mut(),
        options: ptr::null_mut(),
    };
    let status = unsafe {
        tcl_codegen_proc_define_guarded_argv(
            words.as_ptr(),
            i32::try_from(words.len()).unwrap(),
            Some(stub_body),
            expected.value(),
            &mut out,
        )
    };
    assert_eq!(status, TCL_INVOKE_ABI_OK);
    out
}

#[test]
fn guarded_original_definition_uses_stock_selection_and_preserves_generic_handlers() {
    leak_free(|| unsafe {
        for (setup, head, compiled, answer) in [
            ("", "proc", true, ""),
            ("rename proc original", "original", true, ""),
            ("interp alias {} forwarded {} proc", "forwarded", false, ""),
            (
                "rename proc original; original proc args {return REPLACED}",
                "proc",
                false,
                "REPLACED",
            ),
            (
                "trace add execution proc enter {apply {{args} {lappend ::observed ENTER}}}",
                "proc",
                false,
                "",
            ),
        ] {
            let mut interp = scalar_interpreter("tcl9.0");
            tcl_runtime_set_current_interp(&mut interp);
            assert_eq!(interp.eval_str(setup.as_bytes()), Code::Ok);
            stub_argv(&[b"set", b"::compiled_seen", b"yes"]);
            let expected = tcl_registry::NativeLowering::Definition
                .definition_guard_identity(interp.native_invocation_dialect())
                .unwrap();
            let words = [
                owned_word(head.as_bytes()),
                owned_word(b"p"),
                owned_word(b""),
                owned_word(b"set ::compiled_seen yes"),
            ];
            let mut out = define_guarded_words(&words, expected);
            assert_eq!(out.code, i32::try_from(Code::Ok.as_int()).unwrap());
            assert_eq!(obj_bytes(out.result), answer.as_bytes());
            tcl_completion_release(&mut out);
            release_words(&words);
            let actual = match interp.resolve_dispatchable(crate::namespace::GLOBAL, b"p") {
                Some(crate::interp::Command::Proc(proc)) => proc.declaration().native.is_some(),
                _ => false,
            };
            assert_eq!(actual, compiled, "{setup}");
            if answer.is_empty() {
                assert_eq!(interp.eval_str(b"p"), Code::Ok);
                assert_eq!(interp.result_bytes(), b"yes");
                assert_eq!(stub_calls() > 0, compiled, "{setup}");
            }
            if setup.starts_with("trace") {
                assert_eq!(interp.eval_str(b"set ::observed"), Code::Ok);
                assert_eq!(interp.result_bytes(), b"ENTER");
            }
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

#[test]
fn guarded_original_definition_refuses_foreign_namespace_policy_and_unknown_head() {
    leak_free(|| unsafe {
        for kind in 0..3 {
            let mut interp = scalar_interpreter("tcl9.0");
            tcl_runtime_set_current_interp(&mut interp);
            let expected = tcl_registry::NativeLowering::Definition
                .definition_guard_identity(interp.native_invocation_dialect())
                .unwrap();
            let mut head = b"proc".as_slice();
            if kind == 0 {
                let holder = interp
                    .namespaces_mut()
                    .ensure_namespace(crate::namespace::GLOBAL, b"holder");
                interp.set_current_ns(holder);
            } else if kind == 1 {
                interp.set_runtime_version(TclVersion::V8_6);
            } else {
                head = b"missing_original_definer";
            }
            let words = [
                owned_word(head),
                owned_word(b"p"),
                owned_word(b""),
                owned_word(b"set ::compiled_seen yes"),
            ];
            let mut out = define_guarded_words(&words, expected);
            if kind == 2 {
                assert_eq!(out.code, i32::try_from(Code::Error.as_int()).unwrap());
            } else {
                assert_eq!(out.code, i32::try_from(Code::Ok.as_int()).unwrap());
                let command = interp
                    .resolve_dispatchable(interp.current_ns(), b"p")
                    .unwrap();
                let crate::interp::Command::Proc(proc) = command else {
                    panic!("ordinary procedure definition");
                };
                assert!(proc.declaration().native.is_none());
            }
            tcl_completion_release(&mut out);
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

unsafe fn define_original_words(
    words: &[*mut TclObj],
    entry: Option<NativeProcEntry>,
) -> TclCompletionAbi {
    let mut out = TclCompletionAbi {
        code: 777,
        result: ptr::null_mut(),
        options: ptr::null_mut(),
    };
    // SAFETY: caller-owned live objects and writable local completion storage.
    let status = unsafe {
        tcl_codegen_proc_define_original_argv(
            words.as_ptr(),
            i32::try_from(words.len()).unwrap(),
            entry,
            &mut out,
        )
    };
    assert_eq!(status, TCL_INVOKE_ABI_OK);
    out
}

#[test]
fn original_definition_argv_uses_actual_name_and_current_holder() {
    leak_free(|| unsafe {
        for environment in ["tcl8.6", "tcl9.0"] {
            let mut interp = scalar_interpreter(environment);
            tcl_runtime_set_current_interp(&mut interp);
            for holder_name in [b"holder".as_slice(), b":".as_slice(), b":ns".as_slice()] {
                let holder = interp
                    .namespaces_mut()
                    .ensure_namespace(crate::namespace::GLOBAL, holder_name);
                interp.set_current_ns(holder);
                let name = if holder_name == b"holder" {
                    b":audit".as_slice()
                } else {
                    b"audit".as_slice()
                };
                let words = [
                    owned_word(b"proc"),
                    owned_word(name),
                    owned_word(b""),
                    owned_word(b"return ORIGINAL"),
                ];
                let mut completion = define_original_words(&words, None);
                assert_eq!(completion.code, i32::try_from(Code::Ok.as_int()).unwrap());
                tcl_completion_release(&mut completion);
                // The ABI borrows these objects; they stay live until cleanup.
                release_words(&words);
                assert_eq!(interp.eval_str(name), Code::Ok);
                assert_eq!(interp.result_bytes(), b"ORIGINAL");
                assert_eq!(interp.current_ns(), holder);
            }
            interp.set_current_ns(crate::namespace::GLOBAL);
            assert_eq!(interp.eval_str(b"info commands :audit"), Code::Ok);
            assert!(interp.result_bytes().is_empty());
            assert_eq!(interp.eval_str(b"info commands audit"), Code::Ok);
            assert!(interp.result_bytes().is_empty());
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

#[test]
fn original_definition_argv_does_not_create_a_missing_holder() {
    leak_free(|| unsafe {
        for environment in ["tcl8.6", "tcl9.0"] {
            let mut interp = scalar_interpreter(environment);
            tcl_runtime_set_current_interp(&mut interp);
            let words = [
                owned_word(b"proc"),
                owned_word(b"missing::audit"),
                owned_word(b""),
                owned_word(b"return NEVER"),
            ];
            let mut completion = define_original_words(&words, None);
            assert_eq!(
                completion.code,
                i32::try_from(Code::Error.as_int()).unwrap()
            );
            tcl_completion_release(&mut completion);
            release_words(&words);
            assert_eq!(interp.eval_str(b"namespace exists missing"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"0");
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

#[test]
fn original_root_name_object_retains_the_installed_software_entry() {
    // This callable is an existing Rust software ABI stub, not Native evidence.
    leak_free(|| unsafe {
        for environment in ["tcl8.6", "tcl9.0"] {
            let mut interp = scalar_interpreter(environment);
            tcl_runtime_set_current_interp(&mut interp);
            stub_argv(&[b"return", b"compiled"]);
            let words = [
                owned_word(b"proc"),
                owned_word(b":audit"),
                owned_word(b""),
                owned_word(b"return source"),
            ];
            let before = tcl_codegen_native_proc_dispatches();
            let mut completion = define_original_words(&words, Some(stub_body));
            assert_eq!(completion.code, i32::try_from(Code::Ok.as_int()).unwrap());
            tcl_completion_release(&mut completion);
            release_words(&words);
            assert_eq!(interp.eval_str(b":audit"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"compiled");
            assert_eq!(tcl_codegen_native_proc_dispatches(), before + 1);
            assert_eq!(interp.eval_str(b"info body :audit"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"return source");
            assert_eq!(interp.eval_str(b"info commands audit"), Code::Ok);
            assert!(interp.result_bytes().is_empty());
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

#[test]
fn original_definition_argv_replacement_retires_its_compiled_entry() {
    // The existing callable is a Rust software ABI stub, not Native provenance.
    leak_free(|| unsafe {
        for environment in ["tcl8.6", "tcl9.0"] {
            let mut interp = scalar_interpreter(environment);
            tcl_runtime_set_current_interp(&mut interp);
            stub_argv(&[b"info", b"level", b"0"]);
            let original = [
                owned_word(b"proc"),
                owned_word(b":audit"),
                owned_word(b"name"),
                owned_word(b"return OLD_SOURCE"),
            ];
            let before = tcl_codegen_native_proc_dispatches();
            let mut completion = define_original_words(&original, Some(stub_body));
            assert_eq!(completion.code, i32::try_from(Code::Ok.as_int()).unwrap());
            tcl_completion_release(&mut completion);
            assert_eq!(interp.eval_str(b":audit first"), Code::Ok);
            assert_eq!(interp.result_bytes(), b":audit first");
            assert_eq!(stub_calls(), 1);
            assert_eq!(tcl_codegen_native_proc_dispatches(), before + 1);
            assert_eq!(interp.eval_str(b"info body :audit"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"return OLD_SOURCE");

            // Reuse the live original head/name/formal argument objects. Their
            // caller-owned references span both borrowed ABI calls; only the
            // new source body has an additional caller-owned reference.
            let body = owned_word(b"return [list REPLACED $name [info level 0]]");
            let replacement = [original[0], original[1], original[2], body];
            let mut completion = define_original_words(&replacement, None);
            assert_eq!(completion.code, i32::try_from(Code::Ok.as_int()).unwrap());
            tcl_completion_release(&mut completion);
            assert_eq!(interp.eval_str(b":audit second"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"REPLACED second {:audit second}");
            assert_eq!(
                stub_calls(),
                1,
                "replacement must not enter the retired callable"
            );
            assert_eq!(tcl_codegen_native_proc_dispatches(), before + 1);
            assert_eq!(interp.eval_str(b"info body :audit"), Code::Ok);
            assert_eq!(
                interp.result_bytes(),
                b"return [list REPLACED $name [info level 0]]"
            );
            assert_eq!(interp.eval_str(b"info args :audit"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"name");
            assert_eq!(interp.eval_str(b"info commands audit"), Code::Ok);
            assert!(interp.result_bytes().is_empty());
            // Replacement retires the procedure's old references, but cannot
            // consume or mutate either caller's original argument objects.
            assert_eq!(obj_bytes(original[1]), b":audit");
            assert_eq!(obj_bytes(original[3]), b"return OLD_SOURCE");
            assert_eq!(
                obj_bytes(body),
                b"return [list REPLACED $name [info level 0]]"
            );
            release_words(&[body]);
            release_words(&original);
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

#[test]
fn original_definition_argv_validates_inputs_without_consuming_arguments() {
    leak_free(|| unsafe {
        tcl_runtime_set_current_interp(ptr::null_mut());
        let mut out = empty_completion();
        assert_eq!(
            tcl_codegen_proc_define_original_argv(ptr::null(), 1, None, &mut out),
            TCL_INVOKE_ABI_NO_CURRENT_INTERP,
        );
        assert_eq!(out.code, 1);
        tcl_completion_release(&mut out);

        let mut interp = scalar_interpreter("tcl9.0");
        tcl_runtime_set_current_interp(&mut interp);
        for argc in [-1, 0] {
            assert_eq!(
                tcl_codegen_proc_define_original_argv(ptr::null(), argc, None, &mut out),
                TCL_INVOKE_ABI_INVALID_ARGC,
            );
            assert_eq!(out.code, 1);
            tcl_completion_release(&mut out);
        }
        assert_eq!(
            tcl_codegen_proc_define_original_argv(ptr::null(), 1, None, &mut out),
            TCL_INVOKE_ABI_NULL_ARGV,
        );
        tcl_completion_release(&mut out);
        let invalid = [ptr::null_mut::<TclObj>()];
        assert_eq!(
            tcl_codegen_proc_define_original_argv(invalid.as_ptr(), 1, None, &mut out),
            TCL_INVOKE_ABI_NULL_WORD,
        );
        tcl_completion_release(&mut out);

        let words = [
            owned_word(b"proc"),
            owned_word(b":audit"),
            owned_word(b""),
            owned_word(b"return ORIGINAL"),
        ];
        assert_eq!(
            tcl_codegen_proc_define_original_argv(words.as_ptr(), 4, None, ptr::null_mut()),
            TCL_INVOKE_ABI_NULL_OUT,
        );
        assert_eq!(obj_bytes(words[1]), b":audit");
        assert_eq!(interp.eval_str(b"info commands :audit"), Code::Ok);
        assert!(interp.result_bytes().is_empty());
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
    });
}

#[test]
fn original_definition_argv_preserves_pending_host_refusal_and_untouched_output() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let failing = [
            owned_word(b"string"),
            owned_word(b"toupper"),
            owned_word(b"A\xff"),
        ];
        let words = [
            owned_word(b"proc"),
            owned_word(b":audit"),
            owned_word(b""),
            owned_word(b"return NEVER"),
        ];
        let mut out = TclCompletionAbi {
            code: 77,
            result: ptr::null_mut(),
            options: ptr::null_mut(),
        };
        assert_eq!(
            tcl_invoke_argv(failing.as_ptr(), 3, &mut out),
            TCL_INVOKE_ABI_HOST_REFUSED,
        );
        assert_eq!(
            tcl_codegen_proc_define_original_argv(words.as_ptr(), 4, None, &mut out),
            TCL_INVOKE_ABI_HOST_REFUSED,
        );
        assert_eq!(out.code, 77);
        assert!(out.result.is_null() && out.options.is_null());
        assert_eq!(tcl_codegen_host_refusal_pending(), 1);
        assert_eq!((*interp).unicode_access_refusal().unwrap().valid_up_to, 1);
        assert_eq!(obj_bytes(words[1]), b":audit");
        assert_eq!(obj_bytes(words[3]), b"return NEVER");
        release_words(&failing);
        release_words(&words);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}
