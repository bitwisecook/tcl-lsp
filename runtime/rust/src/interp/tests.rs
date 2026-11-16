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
use crate::counters;

thread_local! {
    static EXPOSURE_CONFIG_OWNER: std::cell::RefCell<Option<WeakInterp>> = const { std::cell::RefCell::new(None) };
    static EXPOSURE_CONFIG_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static EXPOSURE_CONFIG_FIRST: std::cell::RefCell<Option<tcl_runtime_api::NativeExecutionError>> = const { std::cell::RefCell::new(None) };
}

struct PublicationRetirement {
    interpreter: std::rc::Weak<InterpState>,
    calls: Rc<std::cell::Cell<usize>>,
    visible: Rc<std::cell::Cell<Option<u64>>>,
    active_calls: Rc<std::cell::Cell<usize>>,
    mode: u8,
}

unsafe extern "C" fn publication_noop(
    _: *mut c_void,
    _: *mut Interp,
    _: c_int,
    _: *const *mut TclObj,
) -> c_int {
    0
}

unsafe extern "C" fn publication_retire(data: *mut c_void) {
    // SAFETY: this command registered one boxed client record, consumed
    // exactly once by its delete procedure.
    let state = unsafe { Box::from_raw(data.cast::<PublicationRetirement>()) };
    state.calls.set(state.calls.get() + 1);
    let mut interpreter = state.interpreter.upgrade().map(Interp).unwrap();
    state
        .visible
        .set(interpreter.resolve_cmd_token(b"retiring"));
    match state.mode {
        1 => {
            interpreter.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original delete procedure refusal",
                ),
            );
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("publication-retirement".into(), "1.0".into())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
}

unsafe extern "C" fn publication_active(
    data: *mut c_void,
    interpreter: *mut Interp,
    _: c_int,
    _: *const *mut TclObj,
) -> c_int {
    // SAFETY: the client record is live at callback entry. Replacement may
    // consume it, so retain only the observers before reentry.
    let state = unsafe { &*data.cast::<PublicationRetirement>() };
    let active_calls = state.active_calls.clone();
    let calls = state.calls.clone();
    let interpreter = unsafe { &mut *interpreter };
    let created = interpreter.create_obj_command(
        b"retiring",
        ObjCommand::new(publication_noop, core::ptr::null_mut(), None),
    );
    active_calls.set(calls.get());
    c_int::from(created.is_none())
}

fn retirement_state(interpreter: &Interp, mode: u8) -> Box<PublicationRetirement> {
    Box::new(PublicationRetirement {
        interpreter: Rc::downgrade(&interpreter.0),
        calls: Rc::new(std::cell::Cell::new(0)),
        visible: Rc::new(std::cell::Cell::new(None)),
        active_calls: Rc::new(std::cell::Cell::new(usize::MAX)),
        mode,
    })
}

const GUARDED_IDENTITY: GuardIdentity = GuardIdentity::new(1, 71);

fn guarded_builtin(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.set_result_bytes(b"");
    Code::Ok
}

fn resultless_builtin(_interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    Code::Ok
}

fn prepare_interpreter_guard(interp: &mut Interp) -> GuardToken {
    interp.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
    interp
        .prepare_command_guard(
            b"guarded",
            GUARDED_IDENTITY,
            GuardDomains::one(GuardDomain::Interpreter),
        )
        .expect("interpreter domain should be guardable")
}

fn assert_interpreter_guard_stale(interp: &mut Interp, token: GuardToken) {
    // The command stays attested through every mutation below, so a failed
    // check proves the Interpreter epoch changed.
    assert!(interp.attested_identities(b"guarded").is_some());
    assert!(!interp.check_command_guard(token, b"guarded"));
}

std::thread_local! {
    static REGISTRATION_NESTED_RETIREMENT_MODE: std::cell::Cell<u8> = const { std::cell::Cell::new(3) };
    static REGISTRATION_NESTED_RETIREMENT_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

mod bootstrap;
mod builtin_attestation;
mod command_exposure;
mod command_guards;
mod command_publication;
mod error_publication;
mod evaluation;
mod interpreter_guards;
mod intrinsic_guards;
mod release_surface;
mod safe_interpreters;
mod stock_registration;

fn leak_free(body: impl FnOnce(&mut Interp)) {
    counters::reset();
    {
        let mut interp = Interp::new();
        body(&mut interp);
    }
    assert_eq!(
        counters::finalize(),
        0,
        "residual: {} objs, {} bufs",
        counters::live_objs(),
        counters::live_bufs()
    );
    assert_eq!(counters::double_free_count(), 0);
}

/// Evaluate `src`, requiring success, and return the result bytes.
fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
    assert_eq!(
        i.eval_str(src),
        Code::Ok,
        "eval {:?} -> {:?}",
        String::from_utf8_lossy(src),
        String::from_utf8_lossy(&i.result_bytes())
    );
    i.result_bytes()
}
