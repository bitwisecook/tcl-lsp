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

//! The test extension `tests/c/doors.c` (compiled by `build.rs` against
//! `runtime/rust/include/tcl.h`) loaded into a `tcl-vm`-backed shim interpreter and
//! driven from Tcl: the calls a C command makes into the frame that called it,
//! `Tcl_GetVar2Ex`, `Tcl_ObjSetVar2`, `Tcl_UnsetVar2` and `Tcl_EvalObjEx`.
//!
//! Every expected string in [`CASES`] was captured by building the same
//! `doors.c` against Tcl 9.0.4's own `tcl.h`, loading it into `tclsh9.0`, and
//! running each script at the global level under `catch`, recording its code, its
//! result and the `-errorcode` of its options: the assertions are byte-for-byte
//! against C Tcl, not against its documentation.

#![cfg(cshim_c_tests)]

use std::ffi::{c_int, c_void};

use tcl_cshim::{InitProc, Interp, InterpState, StaticExtensions};
use tcl_engine_api::{Budget, BudgetKind, Engine, EngineError};
use tcl_engine_tclvm::TclVmEngine;

unsafe extern "C" {
    // Declared over `void *`: the interpreter is opaque to C, and an `extern`
    // block naming a Rust type would trip the FFI-safety lint.
    fn Doors_Init(interp: *mut c_void) -> c_int;
}

unsafe extern "C" fn doors_init(interp: *mut InterpState) -> c_int {
    // SAFETY: the shim passes its live interpreter pointer through.
    unsafe { Doors_Init(interp.cast::<c_void>()) }
}

const COMMANDS: [&str; 11] = [
    "doors_eval",
    "doors_eval_direct",
    "doors_eval_reset",
    "doors_eval_twice",
    "doors_get",
    "doors_global_set",
    "doors_keep",
    "doors_peek",
    "doors_set",
    "doors_try",
    "doors_unset",
];

fn loaded() -> Interp<TclVmEngine> {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: `Doors_Init` is the test extension built against the shim header.
    let loaded = unsafe { interp.load_static(doors_init) }.expect("doors loads");
    assert_eq!(loaded.commands, COMMANDS);
    interp
}

/// Run `script` at the global level under `catch`, returning `(code, result,
/// errorCode)` the way the reference probe recorded them.
fn probe(interp: &mut Interp<TclVmEngine>, script: &str) -> (i64, String, String) {
    let code = interp
        .eval(&format!("catch {{uplevel #0 {{{script}}}}} ::r ::o"))
        .expect("catch runs");
    let result = interp.eval("set ::r").expect("result readable");
    let error_code = interp
        .eval("if {[dict exists $::o -errorcode]} {dict get $::o -errorcode}")
        .expect("options readable");
    (
        code.as_str().and_then(|c| c.parse().ok()).expect("a code"),
        result.as_str().unwrap_or_default().to_owned(),
        error_code.as_str().unwrap_or_default().to_owned(),
    )
}

/// `(script, code, result, errorCode)` as `tclsh9.0` reported them.
const CASES: &[(&str, i64, &str, &str)] = &[
    ("doors_set x 5", 0, "5", ""),
    ("doors_set x 5; set x", 0, "5", ""),
    ("doors_set x 5; doors_get x", 0, "5", ""),
    ("doors_set x 1; doors_set x 2; set x", 0, "2", ""),
    ("doors_peek nosuch", 0, "<null>", ""),
    ("set x 5; doors_peek x", 0, "5", ""),
    ("set x 5; doors_get x", 0, "5", ""),
    ("doors_set x 0x10; set x", 0, "0x10", ""),
    ("doors_set x {a b}; llength $x", 0, "2", ""),
    ("doors_set ::g 5; set g", 0, "5", ""),
    ("proc p {} {doors_set ::g2 6}; p; set g2", 0, "6", ""),
    ("doors_set a k v; set a(k)", 0, "v", ""),
    ("doors_set a k v", 0, "v", ""),
    ("doors_set a k v; doors_get a k", 0, "v", ""),
    ("doors_set a(k) v; set a(k)", 0, "v", ""),
    ("doors_set a(k) v; array names a", 0, "k", ""),
    ("set a(k) 1; doors_get a(k)", 0, "1", ""),
    ("array set a {k 1}; doors_get a k", 0, "1", ""),
    ("array set a {k 1}; doors_set a k 2; set a(k)", 0, "2", ""),
    (
        "doors_set a(k) j v",
        1,
        "can't set \"a(k)(j)\": variable isn't array",
        "TCL VALUE VARNAME",
    ),
    (
        "doors_get a(k) j",
        1,
        "can't read \"a(k)(j)\": variable isn't array",
        "TCL VALUE VARNAME",
    ),
    (
        "doors_unset a(k) j",
        1,
        "can't unset \"a(k)(j)\": variable isn't array",
        "TCL VALUE VARNAME",
    ),
    ("set z 1; doors_unset z; info exists z", 0, "0", ""),
    ("doors_set a k v; doors_unset a k; array size a", 0, "0", ""),
    ("doors_set a k v; doors_unset a; info exists a", 0, "0", ""),
    ("doors_set a k v; doors_peek a", 0, "<null>", ""),
    ("doors_set a k v; doors_peek a nosuch", 0, "<null>", ""),
    (
        "doors_set",
        1,
        "wrong # args: should be \"doors_set name ?index? value\"",
        "TCL WRONGARGS",
    ),
    (
        "doors_get",
        1,
        "wrong # args: should be \"doors_get name ?index?\"",
        "TCL WRONGARGS",
    ),
    (
        "doors_unset",
        1,
        "wrong # args: should be \"doors_unset name ?index?\"",
        "TCL WRONGARGS",
    ),
    (
        "doors_global_set",
        1,
        "wrong # args: should be \"doors_global_set name value\"",
        "TCL WRONGARGS",
    ),
    (
        "doors_eval",
        1,
        "wrong # args: should be \"doors_eval script\"",
        "TCL WRONGARGS",
    ),
    (
        "proc p {} {doors_set loc 1; info exists ::loc}; p",
        0,
        "0",
        "",
    ),
    ("proc p {} {doors_set loc 1; set loc}; p", 0, "1", ""),
    (
        "proc p {} {doors_global_set glob 7}; p; set glob",
        0,
        "7",
        "",
    ),
    (
        "proc p {} {doors_global_set ::qual 8}; p; set qual",
        0,
        "8",
        "",
    ),
    ("proc p {} {set here 3; doors_get here}; p", 0, "3", ""),
    (
        "proc p {} {set here 3; doors_unset here; info exists here}; p",
        0,
        "0",
        "",
    ),
    (
        "proc p {} {uplevel 1 {doors_set up 1}}; p; set up",
        0,
        "1",
        "",
    ),
    (
        "namespace eval nsx {variable v 9}; namespace eval nsx {doors_get v}",
        0,
        "9",
        "",
    ),
    (
        "namespace eval nsx {variable v 9}; doors_global_set nsx::v 10; set ::nsx::v",
        0,
        "10",
        "",
    ),
    ("doors_global_set arr(k) v; set arr(k)", 0, "v", ""),
    (
        "trace add variable x write {apply {{n1 n2 op} {set ::traced \"$n1 $op\"}}}; doors_set x 1; set traced",
        0,
        "x write",
        "",
    ),
    (
        "set x 1; trace add variable x read {apply {{n1 n2 op} {set ::traced \"$n1 $op\"}}}; doors_get x; set traced",
        0,
        "x read",
        "",
    ),
    (
        "set x 1; trace add variable x unset {apply {{n1 n2 op} {set ::traced \"$n1 $op\"}}}; doors_unset x; set traced",
        0,
        "x unset",
        "",
    ),
    ("set x hello; doors_keep x", 0, "hello", ""),
    ("set x hello; doors_keep x; info exists x", 0, "0", ""),
    ("set doors_loaded", 0, "1", ""),
    ("doors_eval {expr {1+2}}", 0, "3", ""),
    ("doors_eval {}", 0, "", ""),
    ("doors_eval {set x 1; set y 2}", 0, "2", ""),
    ("doors_eval {list a {b c}}", 0, "a {b c}", ""),
    ("set v 4; doors_eval {incr v}; set v", 0, "5", ""),
    (
        "proc p {} {doors_eval {set inner 5}; set inner}; p",
        0,
        "5",
        "",
    ),
    (
        "proc p {} {doors_eval {upvar #0 g l; set l 3}}; p; set g",
        0,
        "3",
        "",
    ),
    (
        "doors_eval {namespace eval nsy {variable w 4}}; set ::nsy::w",
        0,
        "4",
        "",
    ),
    ("doors_eval {catch {error x} m; set m}", 0, "x", ""),
    (
        "doors_eval {foreach i {1 2 3} {if {$i == 2} break}; set i}",
        0,
        "2",
        "",
    ),
    ("doors_eval {error boom}", 1, "boom", "NONE"),
    ("doors_eval {error boom {} {MY CODE}}", 1, "boom", "MY CODE"),
    ("doors_eval {error {a b} {} {C D}}", 1, "a b", "C D"),
    (
        "doors_eval {expr {1/0}}",
        1,
        "divide by zero",
        "ARITH DIVZERO {divide by zero}",
    ),
    (
        "doors_eval {nosuchcmd}",
        1,
        "invalid command name \"nosuchcmd\"",
        "TCL LOOKUP COMMAND nosuchcmd",
    ),
    (
        "doors_eval {set}",
        1,
        "wrong # args: should be \"set varName ?newValue?\"",
        "TCL WRONGARGS",
    ),
    (
        "set i 0; foreach x {a b c} {incr i; doors_eval break; incr i 100}; set i",
        0,
        "1",
        "",
    ),
    (
        "set i 0; foreach x {a b c} {incr i; doors_eval continue; incr i 100}; set i",
        0,
        "3",
        "",
    ),
    ("doors_eval break", 3, "", ""),
    ("doors_eval continue", 4, "", ""),
    ("doors_eval {return top}", 2, "top", ""),
    (
        "proc p {} {doors_eval {return fromeval}; return nope}; p",
        0,
        "fromeval",
        "",
    ),
    ("doors_eval {return -level 0 -code 5 z}", 5, "z", ""),
    (
        "proc p {} {doors_eval {return -level 0 -code 5 z}}; p",
        5,
        "z",
        "",
    ),
    ("doors_eval_direct {expr {2*3}}", 0, "6", ""),
    ("doors_eval {doors_eval {set nested 1}}", 0, "1", ""),
    ("doors_try {expr {1+2}}", 0, "0 3", ""),
    ("doors_try break", 0, "3 {}", ""),
    ("doors_try continue", 0, "4 {}", ""),
    ("doors_try {return x}", 0, "2 x", ""),
    ("doors_try {error e}", 0, "1 e", ""),
    ("doors_try {return -level 0 -code 5 z}", 0, "5 z", ""),
    (
        "doors_try {nosuch}",
        0,
        "1 {invalid command name \"nosuch\"}",
        "",
    ),
    ("doors_try {expr {1/0}}", 0, "1 {divide by zero}", ""),
    (
        "doors_try {error e {} {A B}}; set ::errorCode",
        0,
        "A B",
        "",
    ),
    ("doors_try {set x \"}", 0, "1 {missing \"}", ""),
    ("doors_try {set x \"}; set ::errorCode", 0, "NONE", ""),
    ("doors_try {puts [}; set ::errorCode", 0, "NONE", ""),
    ("doors_try {if}; set ::errorCode", 0, "TCL WRONGARGS", ""),
    ("doors_try {error e}; info exists ::errorInfo", 0, "1", ""),
    ("doors_try {doors_eval {error deep}}", 0, "1 deep", ""),
    ("doors_try {doors_try {error inner}}", 0, "0 {1 inner}", ""),
    (
        "doors_eval {return -code error -errorcode {X Y} msg}",
        2,
        "msg",
        "X Y",
    ),
    (
        "proc p {} {doors_eval {return -code error -errorcode {X Y} boom}; return nope}; list [catch {p} m o] $m [dict get $o -errorcode]",
        0,
        "1 boom {X Y}",
        "",
    ),
    (
        "proc p {} {doors_eval {return -code error boom}; return nope}; list [catch {p} m o] $m [dict get $o -errorcode]",
        0,
        "1 boom NONE",
        "",
    ),
    (
        "set i 0; foreach x {a b c} {incr i; proc p {} {doors_eval {return -code break}}; p; incr i 100}; set i",
        0,
        "1",
        "",
    ),
    (
        "set i 0; foreach x {a b c} {incr i; proc p {} {doors_eval {return -code continue}}; p; incr i 100}; set i",
        0,
        "3",
        "",
    ),
    (
        "proc inner {} {doors_eval {return -level 2 deep}; return nope}; proc outer {} {inner; return nope2}; outer",
        0,
        "deep",
        "",
    ),
    (
        "proc inner {} {doors_eval {return -level 3 deep}; return nope}; proc mid {} {inner; return nope2}; proc outer {} {mid; return nope3}; outer",
        0,
        "deep",
        "",
    ),
    (
        "proc p {} {doors_eval {return -code 5 z}}; list [catch {p} m] $m",
        0,
        "5 z",
        "",
    ),
    (
        "proc p {} {doors_eval {return -code return z}}; list [catch {p} m] $m",
        0,
        "2 z",
        "",
    ),
    (
        "proc q {} {p}; proc p {} {doors_eval {return -code error -level 2 {two levels}}; return nope}; list [catch {q} m o] $m [dict get $o -level] [dict get $o -code]",
        0,
        "1 {two levels} 0 1",
        "",
    ),
    (
        "proc p {} {doors_eval {return -code break}; return nope}; list [catch {p} m o] $m [dict get $o -code]",
        0,
        "3 {} 3",
        "",
    ),
    (
        "proc p {} {doors_eval {doors_eval {return -code error -errorcode {X Y} deepmsg}}}; list [catch {p} m o] $m [dict get $o -errorcode]",
        0,
        "1 deepmsg {X Y}",
        "",
    ),
    (
        "proc p {} {doors_eval {return -code error -errorcode {X Y} first}; doors_eval {set ok 1}}; p",
        1,
        "first",
        "X Y",
    ),
    (
        "doors_eval {return -code error -errorinfo {my info} msg}",
        2,
        "msg",
        "NONE",
    ),
    (
        "doors_try {return -code error -errorcode {X Y} msg}",
        0,
        "2 msg",
        "",
    ),
    (
        "proc p {} {doors_try {return -code error -errorcode {X Y} msg}}; p",
        0,
        "2 msg",
        "",
    ),
    (
        "proc p {} {doors_eval_twice {return -code error first} {return second}; return nope}; list [catch {p} m o] $m",
        0,
        "0 second",
        "",
    ),
    (
        "proc p {} {doors_eval_twice {return -code error first} {set ok 1}; return nope}; list [catch {p} m o] $m",
        0,
        "0 nope",
        "",
    ),
    (
        "proc p {} {doors_eval_reset {return -code error -errorcode {X Y} boom}; return nope}; list [catch {p} m o] $m",
        0,
        "0 {}",
        "",
    ),
    (
        "doors_eval_reset {return -code error -errorcode {X Y} boom}",
        2,
        "",
        "",
    ),
    (
        "doors_eval_twice {return -code error first} {return -code error -errorcode {P Q} second}",
        2,
        "second",
        "P Q",
    ),
];

/// Where this host's error code is not C Tcl's, and why: the script, its code and
/// result (which agree), the `-errorcode` C Tcl gave, the one given here and the
/// reason. Each is a gap of the engine and not of the shim; when one closes, the
/// row moves to [`CASES`].
const DIVERGENCES: &[(&str, i64, &str, &str, &str, &str)] = &[
    (
        "doors_get nosuch",
        1,
        "can't read \"nosuch\": no such variable",
        "TCL LOOKUP VARNAME nosuch",
        "NONE",
        "the VM's `set` of a variable that is not there raises no `-errorcode`",
    ),
    (
        "doors_set a k v; doors_get a nosuch",
        1,
        "can't read \"a(nosuch)\": no such element in array",
        "TCL READ VARNAME",
        "NONE",
        "the same, for an element that is not there",
    ),
    (
        "doors_set a k v; doors_get a",
        1,
        "can't read \"a\": variable is array",
        "TCL READ VARNAME",
        "NONE",
        "the same, for an array read as a scalar",
    ),
    (
        "doors_set a k v; doors_set a w",
        1,
        "can't set \"a\": variable is array",
        "TCL WRITE VARNAME",
        "NONE",
        "the same, for an array written as a scalar",
    ),
    (
        "set s 1; doors_get s k",
        1,
        "can't read \"s(k)\": variable isn't array",
        "TCL LOOKUP VARNAME s",
        "NONE",
        "the same, for a scalar read as an array",
    ),
    (
        "doors_unset nosuch",
        1,
        "can't unset \"nosuch\": no such variable",
        "TCL LOOKUP VARNAME nosuch",
        "TCL UNSET VARNAME",
        "the VM's `unset` words its own `-errorcode`, which C Tcl does not",
    ),
    (
        "doors_set a k v; doors_unset a nosuch",
        1,
        "can't unset \"a(nosuch)\": no such element in array",
        "TCL LOOKUP ELEMENT nosuch",
        "TCL UNSET VARNAME",
        "the same, for an element that is not there",
    ),
    (
        "doors_global_set nsnone::q 1",
        1,
        "can't set \"nsnone::q\": parent namespace doesn't exist",
        "TCL LOOKUP VARNAME nsnone::q",
        "NONE",
        "the VM's `set` raises no `-errorcode` when the parent namespace is not there",
    ),
    (
        "doors_keep nosuch",
        1,
        "can't read \"nosuch\": no such variable",
        "TCL LOOKUP VARNAME nosuch",
        "NONE",
        "the VM's `set` of a variable that is not there raises no `-errorcode`",
    ),
];

#[test]
fn every_call_answers_what_c_tcl_answers() {
    let mut differ = Vec::new();
    for &(script, code, result, error_code) in CASES {
        let wanted = (code, result.to_owned(), error_code.to_owned());
        let got = probe(&mut loaded(), script);
        if got != wanted {
            differ.push(format!("{script}\n    C Tcl {wanted:?}\n    here  {got:?}"));
        }
    }
    assert!(
        differ.is_empty(),
        "{} of {} differ from C Tcl:\n{}",
        differ.len(),
        CASES.len(),
        differ.join("\n")
    );
}

#[test]
fn where_the_error_code_differs_from_c_tcl_the_difference_is_the_recorded_one() {
    for &(script, code, result, c_tcl, here, why) in DIVERGENCES {
        assert_ne!(here, c_tcl, "{script}: not a divergence");
        assert_eq!(
            probe(&mut loaded(), script),
            (code, result.to_owned(), here.to_owned()),
            "{script}: {why}"
        );
    }
}

/// The table above runs one script in a new interpreter each; these are the
/// properties that need more than one call or more than the answer.
#[test]
fn the_entry_point_reaches_the_frame_that_loaded_it_through_the_hosts_load() {
    static TABLE: &[(&str, InitProc)] = &[("Doors", doors_init)];
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: `Doors_Init` is the test extension built against the shim header.
    let extensions = unsafe { StaticExtensions::new(TABLE) };
    interp
        .enable_static_extensions(extensions)
        .expect("the engine takes the command");
    assert_eq!(
        probe(&mut interp, "info exists doors_loaded"),
        (0, "0".to_owned(), String::new()),
        "nothing has run its entry point"
    );
    assert_eq!(
        probe(
            &mut interp,
            "proc p {} {load {} Doors}; p; set doors_loaded"
        ),
        (0, "1".to_owned(), String::new()),
        "the entry point set the global through the door the engine gave `load`"
    );
    assert_eq!(
        probe(&mut interp, "doors_set x 5; doors_get x"),
        (0, "5".to_owned(), String::new())
    );
}

/// Run `script` as a unit of its own, answering its value as text.
fn run(interp: &mut Interp<TclVmEngine>, script: &str) -> Result<String, EngineError> {
    interp
        .eval(script)
        .map(|value| value.as_str().unwrap_or_default().to_owned())
}

#[test]
fn a_budget_a_command_swallows_still_fails_the_invocation() {
    let script = "doors_try {foreach i {1 2 3 4 5 6 7 8 9 10 11 12} {doors_peek i}}";
    let mut interp = loaded();
    interp
        .engine_mut()
        .set_budget(Budget::of_commands(5))
        .expect("sets the budget");
    assert_eq!(
        run(&mut interp, script),
        Err(EngineError::BudgetExceeded(BudgetKind::Commands)),
        "`doors_try` answered normally and the invocation still failed as the budget"
    );
    assert_eq!(
        run(&mut interp, "info exists ::errorCode"),
        Ok("0".to_owned()),
        "a budget is not a Tcl error, so the globals a caught error sets are not touched"
    );

    let mut interp = loaded();
    interp
        .engine_mut()
        .set_budget(Budget::of_commands(500))
        .expect("sets the budget");
    assert_eq!(
        run(&mut interp, script),
        Ok("0 {}".to_owned()),
        "under a budget the loop fits, the same script is its value"
    );
}
