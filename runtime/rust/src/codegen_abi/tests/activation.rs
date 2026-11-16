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

//! The eval-loop activation an ABI dispatch holds: the depth bound, refusals and what `leave` retains.

use super::*;

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
