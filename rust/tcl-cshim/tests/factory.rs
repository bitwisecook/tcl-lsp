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

//! A command that creates and deletes commands from inside its own
//! invocation — the factory shape C extensions use routinely — published
//! through the engine's registration door before the next statement runs.

use std::ffi::{c_int, c_void};

use tcl_cshim::{Interp, InterpState, Obj, ffi};
use tcl_engine_api::EngineError;
use tcl_engine_tclvm::TclVmEngine;

/// `made ?arg …?` — answers with its arguments as a list.
unsafe extern "C" fn made(
    _client_data: *mut c_void,
    interp: *mut InterpState,
    word_count: c_int,
    words: *const *mut Obj,
) -> c_int {
    // SAFETY: the shim passes a live interpreter and `word_count` objects.
    unsafe {
        let list = ffi::tcl_new_list_obj(
            isize::try_from(word_count).expect("small") - 1,
            words.add(1),
        );
        ffi::tcl_set_obj_result(interp, list);
    }
    ffi::TCL_OK
}

/// `factory NAME` — creates command NAME; `forget NAME` — deletes it.
unsafe extern "C" fn factory(
    client_data: *mut c_void,
    interp: *mut InterpState,
    word_count: c_int,
    words: *const *mut Obj,
) -> c_int {
    // SAFETY: as above; `client_data` is the non-null marker for `forget`.
    unsafe {
        if word_count != 2 {
            ffi::tcl_wrong_num_args(interp, 1, words, c"name".as_ptr());
            return ffi::TCL_ERROR;
        }
        let name = ffi::tcl_get_string(*words.add(1));
        if client_data.is_null() {
            ffi::tcl_create_obj_command(interp, name, made, std::ptr::null_mut(), None);
        } else {
            let deleted = ffi::tcl_delete_command(interp, name);
            ffi::tcl_set_obj_result(interp, ffi::tcl_new_int_obj(c_int::from(deleted == 0)));
        }
    }
    ffi::TCL_OK
}

unsafe extern "C" fn init(interp: *mut InterpState) -> c_int {
    // SAFETY: the shim passes a live interpreter.
    unsafe {
        ffi::tcl_create_obj_command(
            interp,
            c"factory".as_ptr(),
            factory,
            std::ptr::null_mut(),
            None,
        );
        ffi::tcl_create_obj_command(
            interp,
            c"forget".as_ptr(),
            factory,
            std::ptr::from_ref(&init).cast_mut().cast::<c_void>(),
            None,
        );
    }
    ffi::TCL_OK
}

#[test]
fn smoke_a_command_created_mid_script_is_callable_by_the_next_statement() {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: `init` is written against the shim's own exports.
    unsafe { interp.load_static(init) }.expect("loads");
    // A script's result crosses back as text (the engine's rule); the
    // factory's product is what is under test.
    let answer = interp
        .eval("factory made_one\nmade_one a {b c}")
        .expect("the factory's product is callable in the same script");
    assert_eq!(answer.as_str(), Some("a {b c}"));
    assert_eq!(interp.commands(), ["factory", "forget", "made_one"]);
}

#[test]
fn a_command_deleted_mid_script_is_gone_for_the_next_statement() {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: as above.
    unsafe { interp.load_static(init) }.expect("loads");
    interp.eval("factory doomed").expect("creates");
    let error = interp
        .eval("forget doomed\ndoomed")
        .expect_err("the deletion is visible to the next statement");
    assert!(
        matches!(&error, EngineError::Script { message, .. }
            if message == "invalid command name \"doomed\""),
        "{error:?}"
    );
    assert_eq!(interp.commands(), ["factory", "forget"]);
}

#[test]
fn a_command_may_delete_itself() {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: as above.
    unsafe { interp.load_static(init) }.expect("loads");
    let answer = interp
        .eval("forget forget")
        .expect("deleting the running command is safe");
    assert_eq!(answer.as_str(), Some("1"));
    assert_eq!(interp.commands(), ["factory"]);
}
