// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// Proposed additions inside runtime/rust/src/codegen_abi.rs::tests.
// Uncompiled and unexecuted; scalar_interpreter retains an actual Runtime core.

unsafe fn define_original_words(words: &[*mut TclObj], entry: Option<NativeProcEntry>) -> TclCompletionAbi {
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
                let holder = interp.namespaces_mut().ensure_namespace(
                    crate::namespace::GLOBAL,
                    holder_name,
                );
                interp.set_current_ns(holder);
                let name = if holder_name == b"holder" { b":audit".as_slice() } else { b"audit".as_slice() };
                let words = [owned_word(b"proc"), owned_word(name), owned_word(b""), owned_word(b"return ORIGINAL")];
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
            let words = [owned_word(b"proc"), owned_word(b"missing::audit"), owned_word(b""), owned_word(b"return NEVER")];
            let mut completion = define_original_words(&words, None);
            assert_eq!(completion.code, i32::try_from(Code::Error.as_int()).unwrap());
            tcl_completion_release(&mut completion);
            release_words(&words);
            assert_eq!(interp.eval_str(b"namespace exists missing"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"0");
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

// Additional Runtime obligations before declaring the additive ABI validated:
// - zero/no-current/null argc/argv/word/out cases preserve the existing status
//   and untouched-output Host rules, with the same cleanup assertions as Invoke;
// - Some(stub_body) is an explicitly labelled Rust software ABI callable,
//   never native provenance; verify the actual installed generation keeps this
//   entry until replacement/rename/delete and source metadata remains intact;
// - name/formal/body invalid-order tests compare through the actual shared
//   proc_cmd_original handler, including Jim's original objects and result;
// - body/formal callbacks that enter Host refusal bypass completion adoption
//   while borrowed argv and callback-held originals release after mutable scope;
// - complete original CLI comparison uses the independently retained records,
//   not this test's generated object bytes to claim an original compiler pass.

#[test]
fn original_root_name_object_retains_the_installed_software_entry() {
    // This callable is an existing Rust software ABI stub, not Native evidence.
    leak_free(|| unsafe {
        for environment in ["tcl8.6", "tcl9.0"] {
            let mut interp = scalar_interpreter(environment);
            tcl_runtime_set_current_interp(&mut interp);
            stub_argv(&[b"return", b"compiled"]);
            let words = [owned_word(b"proc"), owned_word(b":audit"), owned_word(b""), owned_word(b"return source")];
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
            assert_eq!(stub_calls(), 1, "replacement must not enter the retired callable");
            assert_eq!(tcl_codegen_native_proc_dispatches(), before + 1);
            assert_eq!(interp.eval_str(b"info body :audit"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"return [list REPLACED $name [info level 0]]");
            assert_eq!(interp.eval_str(b"info args :audit"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"name");
            assert_eq!(interp.eval_str(b"info commands audit"), Code::Ok);
            assert!(interp.result_bytes().is_empty());
            // Replacement retires the procedure's old references, but cannot
            // consume or mutate either caller's original argument objects.
            assert_eq!(obj_bytes(original[1]), b":audit");
            assert_eq!(obj_bytes(original[3]), b"return OLD_SOURCE");
            assert_eq!(obj_bytes(body), b"return [list REPLACED $name [info level 0]]");
            release_words(&[body]);
            release_words(&original);
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}
