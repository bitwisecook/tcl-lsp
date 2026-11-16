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

//! The prebuilt-argv tier: guarded intrinsics, guard tokens and `tcl_invoke_argv` dispatch, errors and boundary checks.

use super::*;

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
