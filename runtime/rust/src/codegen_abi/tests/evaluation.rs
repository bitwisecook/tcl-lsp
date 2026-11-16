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

//! Imports that run the runtime's own implementations: the eval-fallback tier, `expr` addition and `puts`.

use super::*;

struct CaptureHost {
    pub(super) native: NativeHost,
    pub(super) stdout: Rc<RefCell<Vec<u8>>>,
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
