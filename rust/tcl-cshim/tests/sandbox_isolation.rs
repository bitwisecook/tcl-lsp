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

//! The trust posture, proven negatively: a shimmed command is registered only
//! into a host-owned interpreter by host code, and neither a pack program nor
//! a hook body can reach it — or `load` anything — from its sandbox.
//!
//! The extension here is Rust-defined through the shim's own exports, so the
//! proof runs on every platform, C compiler or not.

use std::cell::RefCell;
use std::ffi::{c_int, c_void};
use std::rc::Rc;

use tcl_cshim::{Interp, InterpState, Obj, ffi};
use tcl_engine_api::{CompileUnit, Engine, EngineError, Value};
use tcl_engine_tclvm::TclVmEngine;
use tcl_spec_hooks::SANDBOX_COMMANDS;
use tcl_spec_hooks::pack_eval::{
    PackEvalConfig, PackEvalFailure, UnknownHandler, run_pack_program,
};

unsafe extern "C" fn host_only(
    _client_data: *mut c_void,
    interp: *mut InterpState,
    _objc: c_int,
    _objv: *const *mut Obj,
) -> c_int {
    // SAFETY: the shim passes a live interpreter.
    unsafe { ffi::tclshim_set_result_string(interp, c"host answered".as_ptr()) };
    ffi::TCL_OK
}

unsafe extern "C" fn init(interp: *mut InterpState) -> c_int {
    // SAFETY: as above.
    unsafe {
        ffi::tcl_create_obj_command(
            interp,
            c"host_only".as_ptr(),
            host_only,
            std::ptr::null_mut(),
            None,
        );
    }
    ffi::TCL_OK
}

/// A host interpreter with the shimmed command registered and working.
fn host_interp() -> Interp<TclVmEngine> {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: `init` is written against the shim's own exports.
    unsafe { interp.load_static(init) }.expect("loads");
    let answer = interp.eval("host_only").expect("callable in the host");
    assert_eq!(answer.as_str(), Some("host answered"));
    interp
}

/// Run a pack program with no vocabulary, recording every unresolved word.
fn run_pack(source: &str) -> (Result<(), PackEvalFailure>, Vec<String>) {
    let unresolved = Rc::new(RefCell::new(Vec::new()));
    let seen = Rc::clone(&unresolved);
    let unknown: UnknownHandler = Rc::new(move |_ctx, name, _args| {
        seen.borrow_mut().push(name.to_owned());
        Err(format!("\"{name}\" is not a pack word"))
    });
    let outcome = run_pack_program(source, &[], &unknown, &PackEvalConfig::default());
    let names = unresolved.borrow().clone();
    (outcome, names)
}

#[test]
fn smoke_a_pack_program_cannot_load_or_reach_a_shimmed_command() {
    let _host = host_interp();

    let (outcome, unresolved) = run_pack("load ./libpkga.so Pkga\n");
    assert!(
        matches!(&outcome, Err(PackEvalFailure::Script(message)) if message.contains("load")),
        "{outcome:?}"
    );
    assert_eq!(unresolved, ["load"], "`load` is not a command a pack has");

    let (outcome, unresolved) = run_pack("host_only\n");
    assert!(
        matches!(&outcome, Err(PackEvalFailure::Script(message)) if message.contains("host_only")),
        "{outcome:?}"
    );
    assert_eq!(
        unresolved,
        ["host_only"],
        "the host interpreter's command is invisible to the pack sandbox"
    );
}

#[test]
fn a_hook_body_cannot_reach_a_shimmed_command_either() {
    let _host = host_interp();

    // A hook engine as the hook host builds one: fresh, whitelisted.
    let mut engine = TclVmEngine::new();
    engine
        .restrict_commands(SANDBOX_COMMANDS)
        .expect("the whitelist applies");
    for body in ["host_only", "load ./libpkga.so Pkga"] {
        let handle = engine
            .compile(CompileUnit {
                name: "hook",
                parameters: &["words", "ctx"],
                body,
            })
            .expect("a body naming an absent command still compiles");
        let error = engine
            .invoke(&handle, &[Value::list([]), Value::dict_of::<&str>([])])
            .expect_err("but cannot run it");
        assert!(
            matches!(&error, EngineError::Script { message, .. }
                if message.contains("invalid command name")),
            "{body}: {error:?}"
        );
    }
}

#[test]
fn only_the_interpreter_that_loaded_the_extension_has_it() {
    let _host = host_interp();
    let mut other = Interp::new(TclVmEngine::new());
    let error = other
        .eval("host_only")
        .expect_err("a second engine has nothing");
    assert!(
        matches!(&error, EngineError::Script { message, .. }
            if message == "invalid command name \"host_only\""),
        "{error:?}"
    );
    assert!(other.commands().is_empty());
}
