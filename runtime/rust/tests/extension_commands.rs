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

//! Commands an extension registers through `Tcl_CreateObjCommand`: the runtime
//! side of the C-extension ABI's registration seam
//! (`docs/design/runtime/c-extension-abi.md` §4.5 and §12).
//!
//! The extension is Rust written against the C ABI — `unsafe extern "C"`
//! procedures, called through the exports a C extension would import — so the
//! dispatch, the completion codes, the `clientData` and the delete procedure are
//! exercised without a C compiler. On `wasm32` the same procedure pointer is an
//! index into the shared function table, which `wasm_real_link.rs` in
//! `tcl-compiler` runs against the real runtime.

use core::ffi::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};

use tcl_runtime::capi::{
    Tcl_CreateObjCommand, Tcl_DeleteCommand, Tcl_GetString, Tcl_NewStringObj, Tcl_SetObjResult,
};
use tcl_runtime::interp::{Code, Interp};
use tcl_runtime::obj::TclObj;

const TCL_OK: c_int = 0;
const TCL_ERROR: c_int = 1;
const TCL_BREAK: c_int = 3;

/// Set the interpreter's result to `text`, as an extension does.
fn answer(interp: *mut Interp, text: &str) {
    // SAFETY: `text` addresses `len` live bytes; the interpreter is live for the
    // call it is passed to.
    unsafe {
        let obj = Tcl_NewStringObj(text.as_ptr().cast::<c_char>(), text.len() as isize);
        Tcl_SetObjResult(interp, obj);
    }
}

/// The words of a call, as text.
fn words(objc: c_int, objv: *const *mut TclObj) -> Vec<String> {
    (0..usize::try_from(objc).expect("a count"))
        .map(|index| {
            // SAFETY: `objv` addresses `objc` live objects.
            let bytes = unsafe { std::ffi::CStr::from_ptr(Tcl_GetString(*objv.add(index))) };
            bytes.to_string_lossy().into_owned()
        })
        .collect()
}

/// `join_words a b …` — answers its words (the command name included) joined by
/// a bar.
unsafe extern "C" fn join_words(
    _client_data: *mut c_void,
    interp: *mut Interp,
    objc: c_int,
    objv: *const *mut TclObj,
) -> c_int {
    answer(interp, &words(objc, objv).join("|"));
    TCL_OK
}

/// `fail …` — an error whose message is its first argument.
unsafe extern "C" fn fail(
    _client_data: *mut c_void,
    interp: *mut Interp,
    objc: c_int,
    objv: *const *mut TclObj,
) -> c_int {
    let words = words(objc, objv);
    answer(interp, words.get(1).map_or("failed", String::as_str));
    TCL_ERROR
}

/// `brk` — a `break`.
unsafe extern "C" fn brk(
    _client_data: *mut c_void,
    _interp: *mut Interp,
    _objc: c_int,
    _objv: *const *mut TclObj,
) -> c_int {
    TCL_BREAK
}

/// `tick` — counts its calls in the `AtomicUsize` its `clientData` points at.
unsafe extern "C" fn tick(
    client_data: *mut c_void,
    interp: *mut Interp,
    _objc: c_int,
    _objv: *const *mut TclObj,
) -> c_int {
    // SAFETY: the tests register a live `AtomicUsize` as the client data.
    let count = unsafe { &*client_data.cast::<AtomicUsize>() }.fetch_add(1, Ordering::SeqCst) + 1;
    answer(interp, &count.to_string());
    TCL_OK
}

/// The delete procedure of `tick`: adds one to a second counter the client data
/// is the first of.
unsafe extern "C" fn count_deletion(client_data: *mut c_void) {
    // SAFETY: the tests register a pair of live `AtomicUsize`s as the client data.
    unsafe { &*client_data.cast::<AtomicUsize>().add(1) }.fetch_add(1, Ordering::SeqCst);
}

/// A command and its delete procedure share a pair of counters: calls, deletions.
struct Counters([AtomicUsize; 2]);

impl Counters {
    const fn new() -> Self {
        Self([AtomicUsize::new(0), AtomicUsize::new(0)])
    }

    fn pointer(&self) -> *mut c_void {
        self.0.as_ptr().cast_mut().cast::<c_void>()
    }

    fn calls(&self) -> usize {
        self.0[0].load(Ordering::SeqCst)
    }

    fn deletions(&self) -> usize {
        self.0[1].load(Ordering::SeqCst)
    }
}

type Proc = unsafe extern "C" fn(*mut c_void, *mut Interp, c_int, *const *mut TclObj) -> c_int;

/// Register `name`, panicking when the runtime declines.
fn register(
    interp: &mut Interp,
    name: &std::ffi::CStr,
    proc_: Proc,
    client_data: *mut c_void,
    delete: Option<unsafe extern "C" fn(*mut c_void)>,
) {
    // SAFETY: the interpreter and the name are live; the procedure is a valid
    // `Tcl_ObjCmdProc`.
    let token = unsafe {
        Tcl_CreateObjCommand(
            std::ptr::from_mut(interp),
            name.as_ptr(),
            Some(proc_),
            client_data,
            delete,
        )
    };
    assert!(!token.is_null(), "{name:?} was registered");
}

/// Evaluate `script`, answering the completion code and the result text.
fn run(interp: &mut Interp, script: &str) -> (Code, String) {
    let code = interp.eval_str(script.as_bytes());
    (
        code,
        String::from_utf8_lossy(&interp.result_bytes()).into_owned(),
    )
}

#[test]
fn a_registered_command_is_called_with_its_words_and_answers_its_result() {
    let mut interp = Interp::new();
    assert_eq!(
        run(&mut interp, "catch {join_words a b} m; set m"),
        (Code::Ok, "invalid command name \"join_words\"".to_owned()),
        "before registration the command is not there"
    );
    register(
        &mut interp,
        c"join_words",
        join_words,
        std::ptr::null_mut(),
        None,
    );
    assert_eq!(
        run(&mut interp, "join_words a {b c} [string cat d e]"),
        (Code::Ok, "join_words|a|b c|de".to_owned())
    );
    assert_eq!(
        run(&mut interp, "info cmdtype join_words"),
        (Code::Ok, "native".to_owned())
    );
    assert_eq!(
        run(&mut interp, "info commands join_words"),
        (Code::Ok, "join_words".to_owned())
    );
}

#[test]
fn the_completion_code_a_procedure_answers_is_the_commands_completion() {
    let mut interp = Interp::new();
    register(&mut interp, c"fail", fail, std::ptr::null_mut(), None);
    register(&mut interp, c"brk", brk, std::ptr::null_mut(), None);
    assert_eq!(
        run(&mut interp, "fail oops"),
        (Code::Error, "oops".to_owned())
    );
    assert_eq!(
        run(
            &mut interp,
            "catch {fail \"two words\"} m opts; list $m [dict get $opts -code]"
        ),
        (Code::Ok, "{two words} 1".to_owned())
    );
    assert_eq!(
        run(
            &mut interp,
            "set n 0; foreach i {1 2 3} { incr n; brk; incr n 100 }; set n"
        ),
        (Code::Ok, "1".to_owned()),
        "a TCL_BREAK ends the loop it is in"
    );
}

#[test]
fn client_data_reaches_the_procedure_and_the_delete_procedure_runs_once() {
    static COUNTERS: Counters = Counters::new();
    let mut interp = Interp::new();
    register(
        &mut interp,
        c"tick",
        tick,
        COUNTERS.pointer(),
        Some(count_deletion),
    );
    assert_eq!(
        run(&mut interp, "tick; tick; tick"),
        (Code::Ok, "3".to_owned())
    );
    assert_eq!((COUNTERS.calls(), COUNTERS.deletions()), (3, 0));

    assert_eq!(
        run(&mut interp, "rename tick {}"),
        (Code::Ok, String::new())
    );
    assert_eq!(
        COUNTERS.deletions(),
        1,
        "deleting the command ran the delete procedure"
    );
    assert_eq!(
        run(&mut interp, "catch {tick} m; set m"),
        (Code::Ok, "invalid command name \"tick\"".to_owned())
    );
    drop(interp);
    assert_eq!(COUNTERS.deletions(), 1, "and the teardown ran nothing more");
}

#[test]
fn tcl_delete_command_answers_zero_for_a_command_and_minus_one_for_none() {
    static COUNTERS: Counters = Counters::new();
    let mut interp = Interp::new();
    register(
        &mut interp,
        c"tick",
        tick,
        COUNTERS.pointer(),
        Some(count_deletion),
    );
    let interp_ptr = std::ptr::from_mut(&mut interp);
    // SAFETY: the interpreter and the names are live.
    unsafe {
        assert_eq!(Tcl_DeleteCommand(interp_ptr, c"tick".as_ptr()), 0);
        assert_eq!(Tcl_DeleteCommand(interp_ptr, c"tick".as_ptr()), -1);
        assert_eq!(Tcl_DeleteCommand(interp_ptr, c"never_there".as_ptr()), -1);
    }
    assert_eq!(COUNTERS.deletions(), 1);
    assert_eq!(
        run(&mut interp, "catch {tick} m; set m"),
        (Code::Ok, "invalid command name \"tick\"".to_owned())
    );
}

#[test]
fn registering_a_name_again_replaces_the_command_and_deletes_the_old_one() {
    static COUNTERS: Counters = Counters::new();
    let mut interp = Interp::new();
    register(
        &mut interp,
        c"thing",
        tick,
        COUNTERS.pointer(),
        Some(count_deletion),
    );
    assert_eq!(run(&mut interp, "thing"), (Code::Ok, "1".to_owned()));
    register(
        &mut interp,
        c"thing",
        join_words,
        std::ptr::null_mut(),
        None,
    );
    assert_eq!(COUNTERS.deletions(), 1, "the displaced command was deleted");
    assert_eq!(
        run(&mut interp, "thing x"),
        (Code::Ok, "thing|x".to_owned()),
        "the new procedure answers"
    );
    assert_eq!(COUNTERS.calls(), 1);
}

#[test]
fn a_qualified_name_lands_in_its_namespace_and_a_bare_one_in_the_current_one() {
    let mut interp = Interp::new();
    register(
        &mut interp,
        c"::ext::xjoin",
        join_words,
        std::ptr::null_mut(),
        None,
    );
    assert_eq!(
        run(&mut interp, "ext::xjoin a; namespace eval ext { xjoin b }"),
        (Code::Ok, "xjoin|b".to_owned())
    );
    assert_eq!(
        run(&mut interp, "catch {xjoin a} m; set m"),
        (Code::Ok, "invalid command name \"xjoin\"".to_owned()),
        "it is not a global command"
    );
    assert_eq!(
        run(&mut interp, "namespace eval ext { info commands ::ext::*}"),
        (Code::Ok, "::ext::xjoin".to_owned())
    );
}

#[test]
fn deleting_a_command_from_inside_its_own_procedure_leaves_its_client_data_live() {
    static COUNTERS: Counters = Counters::new();

    /// `selfdelete` — deletes itself, then reads its client data and answers
    /// whether the delete procedure had run yet.
    unsafe extern "C" fn selfdelete(
        client_data: *mut c_void,
        interp: *mut Interp,
        _objc: c_int,
        _objv: *const *mut TclObj,
    ) -> c_int {
        // SAFETY: the interpreter is live and the name terminated.
        let deleted = unsafe { Tcl_DeleteCommand(interp, c"selfdelete".as_ptr()) };
        // SAFETY: the client data is a live pair of counters, and still is.
        let deletions =
            unsafe { &*client_data.cast::<AtomicUsize>().add(1) }.load(Ordering::SeqCst);
        answer(interp, &format!("{deleted} {deletions}"));
        TCL_OK
    }

    let mut interp = Interp::new();
    register(
        &mut interp,
        c"selfdelete",
        selfdelete,
        COUNTERS.pointer(),
        Some(count_deletion),
    );
    assert_eq!(
        run(&mut interp, "selfdelete"),
        (Code::Ok, "0 0".to_owned()),
        "the command is gone and the delete procedure has not yet run"
    );
    assert_eq!(COUNTERS.deletions(), 1, "it ran when the call returned");
}

#[test]
fn a_delete_trace_that_recreates_the_command_leaves_the_new_one_alone() {
    static COUNTERS: Counters = Counters::new();
    let mut interp = Interp::new();
    register(
        &mut interp,
        c"thing",
        tick,
        COUNTERS.pointer(),
        Some(count_deletion),
    );
    assert_eq!(
        run(
            &mut interp,
            "trace add command thing delete {apply {{old new op} {proc thing {} {return reborn}}}}\n\
             rename thing {}\n\
             thing"
        ),
        (Code::Ok, "reborn".to_owned()),
        "the deletion removed the extension's command, not the one the trace made"
    );
    assert_eq!(COUNTERS.deletions(), 1);
}

#[test]
fn a_registration_with_nothing_to_bind_is_declined() {
    let mut interp = Interp::new();
    let interp_ptr = std::ptr::from_mut(&mut interp);
    // SAFETY: the interpreter is live; each call is the refusal under test.
    unsafe {
        assert!(Tcl_CreateObjCommand(
            interp_ptr,
            c"nothing".as_ptr(),
            None,
            std::ptr::null_mut(),
            None
        )
        .is_null());
        assert!(Tcl_CreateObjCommand(
            interp_ptr,
            std::ptr::null(),
            Some(join_words),
            std::ptr::null_mut(),
            None
        )
        .is_null());
        assert!(Tcl_CreateObjCommand(
            std::ptr::null_mut(),
            c"nothing".as_ptr(),
            Some(join_words),
            std::ptr::null_mut(),
            None
        )
        .is_null());
    }
    assert_eq!(
        run(&mut interp, "catch {nothing} m; set m"),
        (Code::Ok, "invalid command name \"nothing\"".to_owned())
    );
}
