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

//! The runtime as an engine of the extension interface, natively: the cases
//! every runtime engine is held to (`common/engine_cases.rs`, which the WASM
//! engine's suite runs too), and what only the native one has — the release
//! it reports, and the frame-addressed store a command makes through the
//! runtime's own state traits. The bodies need `expr` and the loop commands,
//! so the suite needs the numeric tower.
#![cfg(all(feature = "engine", have_tommath))]

#[path = "common/engine_cases.rs"]
mod engine_cases;

use core::ffi::{c_int, c_void};

use tcl_engine_api::Engine;
use tcl_runtime::engine::RuntimeEngine;
use tcl_runtime::interp::Interp;
use tcl_runtime::obj::TclObj;
use tcl_runtime_api::{FrameId, Frames, VarStore};

/// The engine the cases run on: the runtime's own, natively.
fn runtime_engine() -> Option<impl Fn() -> RuntimeEngine> {
    Some(RuntimeEngine::new)
}

engine_cases::engine_cases!(runtime_engine);

/// The engine names itself, and reports the catalogue profile it pinned and
/// none when a pin is refused.
#[test]
fn the_engine_names_itself_and_the_release_it_pinned() {
    let mut engine = RuntimeEngine::new();
    assert_eq!(engine.name(), "runtime");
    assert_eq!(engine.release(), None);
    for profile in ["tcl8.6", "tcl9.0", "f5-irules"] {
        let mut pinned = RuntimeEngine::new();
        pinned
            .set_release(profile)
            .expect("a catalogue profile pins");
        assert_eq!(pinned.release(), Some(profile));
    }
    assert!(engine.set_release("f5-bigip").is_err());
    assert_eq!(engine.release(), None);
}

/// A command that stores `after` into the variable its first word names in the
/// frame its second word names, through the frame-addressed store the shared
/// command core and compiled code use (`VarStore::set` at another frame), and
/// not through a script command.
unsafe extern "C" fn store_at(
    _client_data: *mut c_void,
    interp: *mut Interp,
    objc: c_int,
    objv: *const *mut TclObj,
) -> c_int {
    // SAFETY: dispatch passes its live interpreter and `objc` live words.
    let (interp, words) = unsafe {
        (
            &mut *interp,
            std::slice::from_raw_parts(objv, usize::try_from(objc).unwrap_or(0)),
        )
    };
    let text = |word: *mut TclObj| {
        // SAFETY: each word is live for the call.
        unsafe { core::ffi::CStr::from_ptr(tcl_runtime::capi::Tcl_GetString(word)) }
            .to_string_lossy()
            .into_owned()
    };
    let (name, up) = (text(words[1]), text(words[2]));
    let level = Frames::current(interp).0 - up.parse::<usize>().expect("a level");
    // SAFETY: a fresh object the store takes its own hold on.
    let value = unsafe { tcl_runtime::capi::Tcl_NewStringObj(c"after".as_ptr(), -1) };
    VarStore::set(interp, FrameId(level), &name, value);
    0
}

/// The frame-addressed stores are confined too: a store into the caller's
/// frame, or the global one, made through `VarStore::set` by a command a
/// procedure calls is refused as a store from a script is, where an open
/// engine makes it.
#[test]
fn a_frame_addressed_store_outside_the_activation_is_refused() {
    for (confined, expected) in [(false, "after after"), (true, "before before")] {
        let mut engine = RuntimeEngine::new();
        // SAFETY: the interpreter is live and the procedure is this file's.
        unsafe {
            tcl_runtime::capi::Tcl_CreateObjCommand(
                engine.interp_mut(),
                c"store_at".as_ptr(),
                Some(store_at),
                core::ptr::null_mut(),
                None,
            );
        }
        engine_cases::run(
            &mut engine,
            "set ::g before\n\
             proc outer {} { set x before; inner; return $x }\n\
             proc inner {} { store_at x 1 }\n\
             proc global {} { store_at ::g 1 }",
        )
        .expect("an open engine sets up");
        if confined {
            engine.confine_stores().expect("confines");
        }
        assert_eq!(
            engine_cases::run(&mut engine, "global\nreturn [list [outer] $::g]"),
            Ok(expected.to_owned()),
            "confined: {confined}"
        );
    }
}
