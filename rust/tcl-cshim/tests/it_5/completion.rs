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

//! The completion code a C command answers, through the bytecode VM: a
//! `TCL_BREAK` ends the loop the command is in, a `TCL_CONTINUE` goes on to the
//! next iteration, and a `TCL_RETURN` returns from the procedure that called it,
//! as they do in C Tcl; any other code is the command's own, and a `catch` reports
//! it.

use std::ffi::{c_int, c_void};

use tcl_cshim::{Interp, InterpState, Obj, ffi};
use tcl_engine_api::EngineError;
use tcl_engine_tclvm::TclVmEngine;

/// `completes code` — answers "done" and returns the integer `code`.
unsafe extern "C" fn completes(
    _client_data: *mut c_void,
    interp: *mut InterpState,
    word_count: c_int,
    words: *const *mut Obj,
) -> c_int {
    // SAFETY: the shim passes a live interpreter and `word_count` live objects.
    unsafe {
        let mut code: c_int = 0;
        if word_count != 2
            || ffi::tcl_get_int_from_obj(interp, *words.add(1), &raw mut code) != ffi::TCL_OK
        {
            ffi::tcl_wrong_num_args(interp, 1, words, c"code".as_ptr());
            return ffi::TCL_ERROR;
        }
        ffi::tcl_set_obj_result(interp, ffi::tcl_new_string_obj(c"done".as_ptr(), 4));
        code
    }
}

unsafe extern "C" fn init(interp: *mut InterpState) -> c_int {
    // SAFETY: the shim passes a live interpreter.
    unsafe {
        ffi::tcl_create_obj_command(
            interp,
            c"completes".as_ptr(),
            completes,
            std::ptr::null_mut(),
            None,
        );
    }
    ffi::TCL_OK
}

fn interp() -> Interp<TclVmEngine> {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: `init` is written against the shim's own exports.
    unsafe { interp.load_static(init) }.expect("loads");
    interp
}

fn run(interp: &mut Interp<TclVmEngine>, script: &str) -> Result<String, EngineError> {
    interp
        .eval(script)
        .map(|value| value.as_str().unwrap_or_default().to_owned())
}

fn script_error(message: &str) -> EngineError {
    EngineError::Script {
        message: message.to_owned(),
        code: Some("TCL RESULT UNEXPECTED".to_owned()),
    }
}

#[test]
fn a_tcl_break_ends_the_loop_the_command_is_in() {
    let mut interp = interp();
    assert_eq!(
        run(
            &mut interp,
            "set i 0; foreach x {a b c} { incr i; completes 3; incr i 100 }; set i"
        ),
        Ok("1".to_owned())
    );
}

#[test]
fn a_tcl_continue_goes_on_to_the_next_iteration() {
    let mut interp = interp();
    assert_eq!(
        run(
            &mut interp,
            "set i 0; foreach x {a b c} { incr i; completes 4; incr i 100 }; set i"
        ),
        Ok("3".to_owned())
    );
}

#[test]
fn a_tcl_return_returns_from_the_procedure_that_called_it() {
    let mut interp = interp();
    assert_eq!(
        run(&mut interp, "proc p {} { completes 2; return nope }; p"),
        Ok("done".to_owned()),
        "the procedure ends with the command's result"
    );
}

#[test]
fn a_tcl_ok_leaves_the_loop_alone() {
    let mut interp = interp();
    assert_eq!(
        run(
            &mut interp,
            "set i 0; foreach x {a b c} { incr i; completes 0; incr i 100 }; set i"
        ),
        Ok("303".to_owned())
    );
}

#[test]
fn a_break_or_a_continue_outside_a_loop_is_what_tcl_says_of_it() {
    let mut interp = interp();
    assert_eq!(
        run(&mut interp, "completes 3"),
        Err(script_error("invoked \"break\" outside of a loop"))
    );
    assert_eq!(
        run(&mut interp, "completes 4"),
        Err(script_error("invoked \"continue\" outside of a loop"))
    );
}

#[test]
fn a_code_of_the_commands_own_reaches_the_catch_that_reports_it() {
    let mut interp = interp();
    assert_eq!(
        run(&mut interp, "list [catch {completes 9} m] $m"),
        Ok("9 done".to_owned()),
        "as it does in C Tcl"
    );
    assert_eq!(
        run(
            &mut interp,
            "proc p {} {completes 9; return nope}; list [catch p m] $m"
        ),
        Ok("9 done".to_owned()),
        "through the procedure that called it"
    );
}
