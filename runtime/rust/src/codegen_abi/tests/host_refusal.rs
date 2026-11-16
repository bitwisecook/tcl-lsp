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

//! A reached host refusal leaves caller outputs untouched and bypasses guest recovery; a guest error with raw bytes is not a refusal.

use super::*;

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
