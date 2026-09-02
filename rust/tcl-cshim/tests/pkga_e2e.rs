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

//! The real C extension (`tests/c/pkga.c`, compiled by `build.rs` against
//! `include/tclshim.h`) loaded into a `tcl-vm`-backed shim interpreter and
//! driven from Tcl.
//!
//! Every expected string below was captured by building the same `pkga.c`
//! against Tcl 9.0.4's own `tcl.h`, loading it into `tclsh9.0`, and
//! recording the result and `$errorCode` of each call — so these assertions
//! are byte-for-byte against C Tcl, not against Tcl's documentation.

#![cfg(cshim_c_tests)]

use std::ffi::{c_int, c_void};

use tcl_cshim::{Interp, InterpState};
use tcl_engine_tclvm::TclVmEngine;

unsafe extern "C" {
    // Declared over `void *`: the interpreter is opaque to C, and an `extern`
    // block naming a Rust type would trip the FFI-safety lint.
    fn Pkga_Init(interp: *mut c_void) -> c_int;
}

unsafe extern "C" fn pkga_init(interp: *mut InterpState) -> c_int {
    // SAFETY: the shim passes its live interpreter pointer through.
    unsafe { Pkga_Init(interp.cast::<c_void>()) }
}

fn loaded() -> Interp<TclVmEngine> {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: `Pkga_Init` is the test extension built against the shim header.
    let loaded = unsafe { interp.load_static(pkga_init) }.expect("pkga loads");
    assert_eq!(
        loaded.commands,
        [
            "pkga_calc",
            "pkga_count",
            "pkga_eq",
            "pkga_forget",
            "pkga_quote"
        ]
    );
    interp
}

/// Run `script` under `catch`, returning `(code, result, errorCode)` the way
/// the reference probe recorded them.
fn catching(interp: &mut Interp<TclVmEngine>, script: &str) -> (i64, String, String) {
    // Each `eval` is its own unit with its own locals, so the result goes
    // through a global.
    let code = interp
        .eval(&format!("catch {{{script}}} ::r"))
        .expect("catch runs");
    let result = interp.eval("set ::r").expect("result readable");
    let error_code = if code.as_str() == Some("0") {
        String::new()
    } else {
        interp
            .eval("set ::errorCode")
            .expect("errorCode readable")
            .as_str()
            .unwrap_or_default()
            .to_owned()
    };
    (
        code.as_str().and_then(|c| c.parse().ok()).expect("a code"),
        result.as_str().unwrap_or_default().to_owned(),
        error_code,
    )
}

#[test]
fn smoke_pkga_round_trips_through_the_vm() {
    let mut interp = loaded();
    assert_eq!(
        catching(&mut interp, "pkga_eq abc abc"),
        (0, "1".into(), String::new())
    );
    assert_eq!(
        catching(&mut interp, "pkga_calc add 5 [pkga_calc add 1 2]"),
        (0, "8".into(), String::new())
    );
}

/// `(script, code, result, errorCode)` as `tclsh9.0` reported them.
const CASES: &[(&str, i64, &str, &str)] = &[
    ("pkga_eq abc abd", 0, "0", ""),
    ("pkga_eq héllo héllo", 0, "1", ""),
    ("pkga_eq é e", 0, "0", ""),
    (
        "pkga_eq a",
        1,
        "wrong # args: should be \"pkga_eq string1 string2\"",
        "TCL WRONGARGS",
    ),
    (
        "pkga_eq {a b} c d",
        1,
        "wrong # args: should be \"pkga_eq string1 string2\"",
        "TCL WRONGARGS",
    ),
    ("pkga_quote {a b c}", 0, "a b c", ""),
    (
        "pkga_quote",
        1,
        "wrong # args: should be \"pkga_quote value\"",
        "TCL WRONGARGS",
    ),
    (
        "pkga_calc",
        1,
        "wrong # args: should be \"pkga_calc subcommand ?arg ...?\"",
        "TCL WRONGARGS",
    ),
    (
        "pkga_calc bogus",
        1,
        "bad subcommand \"bogus\": must be add, sub, range, sum, neg, not, fail, join, len, or dup",
        "TCL LOOKUP INDEX subcommand bogus",
    ),
    (
        "pkga_calc s",
        1,
        "ambiguous subcommand \"s\": must be add, sub, range, sum, neg, not, fail, join, len, or dup",
        "TCL LOOKUP INDEX subcommand s",
    ),
    (
        "pkga_calc {} 1",
        1,
        "ambiguous subcommand \"\": must be add, sub, range, sum, neg, not, fail, join, len, or dup",
        "TCL LOOKUP INDEX subcommand {}",
    ),
    (
        "pkga_calc {a d} 1",
        1,
        "bad subcommand \"a d\": must be add, sub, range, sum, neg, not, fail, join, len, or dup",
        "TCL LOOKUP INDEX subcommand {a d}",
    ),
    ("pkga_calc ad 2 3", 0, "5", ""),
    ("pkga_calc add 0x10 1", 0, "17", ""),
    ("pkga_calc add \" 12 \" 1", 0, "13", ""),
    ("pkga_calc add 1_000 1", 0, "1001", ""),
    ("pkga_calc add 2147483648 0", 0, "-2147483648", ""),
    (
        "pkga_calc add 2",
        1,
        "wrong # args: should be \"pkga_calc add n m\"",
        "TCL WRONGARGS",
    ),
    (
        "pkga_calc ad 2",
        1,
        "wrong # args: should be \"pkga_calc add n m\"",
        "TCL WRONGARGS",
    ),
    (
        "pkga_calc add abc 1",
        1,
        "expected integer but got \"abc\"",
        "TCL VALUE NUMBER",
    ),
    (
        "pkga_calc add 1.0 1",
        1,
        "expected integer but got \"1.0\"",
        "TCL VALUE NUMBER",
    ),
    (
        "pkga_calc add {} 1",
        1,
        "expected integer but got \"\"",
        "TCL VALUE NUMBER",
    ),
    (
        "pkga_calc add 99999999999 1",
        1,
        "integer value too large to represent",
        "ARITH IOVERFLOW {integer value too large to represent}",
    ),
    ("pkga_calc sub 10 3", 0, "7", ""),
    ("pkga_calc range 4", 0, "0 1 2 3", ""),
    ("pkga_calc range 0", 0, "", ""),
    (
        "pkga_calc range x",
        1,
        "expected integer but got \"x\"",
        "TCL VALUE NUMBER",
    ),
    ("pkga_calc sum {1 2 3 4}", 0, "10", ""),
    (
        "pkga_calc sum {9223372036854775807 1}",
        0,
        "-9223372036854775808",
        "",
    ),
    (
        "pkga_calc sum {1 2 x}",
        1,
        "expected integer but got \"x\"",
        "TCL VALUE NUMBER",
    ),
    (
        "pkga_calc sum \"a \\{b\"",
        1,
        "unmatched open brace in list",
        "TCL VALUE LIST BRACE",
    ),
    (
        "pkga_calc sum {99999999999999999999}",
        1,
        "integer value too large to represent",
        "ARITH IOVERFLOW {integer value too large to represent}",
    ),
    ("pkga_calc sum {}", 0, "0", ""),
    ("pkga_calc neg 1.5", 0, "-1.5", ""),
    ("pkga_calc neg 2", 0, "-2.0", ""),
    ("pkga_calc neg 0x10", 0, "-16.0", ""),
    ("pkga_calc neg 1e300", 0, "-1e+300", ""),
    ("pkga_calc neg 1e-7", 0, "-1e-7", ""),
    (
        "pkga_calc neg 123456789012345678",
        0,
        "-1.2345678901234568e+17",
        "",
    ),
    ("pkga_calc neg 99999999999999999999", 0, "-1e+20", ""),
    ("pkga_calc neg Inf", 0, "-Inf", ""),
    ("pkga_calc neg -0.0", 0, "0.0", ""),
    (
        "pkga_calc neg NaN",
        1,
        "floating point value is Not a Number",
        "TCL VALUE DOUBLE NAN",
    ),
    (
        "pkga_calc neg abc",
        1,
        "expected floating-point number but got \"abc\"",
        "TCL VALUE NUMBER",
    ),
    ("pkga_calc not yes", 0, "0", ""),
    ("pkga_calc not 0", 0, "1", ""),
    ("pkga_calc not tr", 0, "0", ""),
    ("pkga_calc not 5", 0, "0", ""),
    ("pkga_calc not 1.5", 0, "0", ""),
    ("pkga_calc not 0.0", 0, "1", ""),
    (
        "pkga_calc not maybe",
        1,
        "expected boolean value but got \"maybe\"",
        "TCL VALUE NUMBER",
    ),
    (
        "pkga_calc not {}",
        1,
        "expected boolean value but got \"\"",
        "TCL VALUE NUMBER",
    ),
    ("pkga_calc fail boom", 1, "boom", "PKGA FAIL boom"),
    ("pkga_calc join a b", 0, "a+b", ""),
    ("pkga_calc j 1 2", 0, "1+2", ""),
    ("pkga_calc join {x y} z", 0, "x y+z", ""),
    ("pkga_calc len {a b c}", 0, "3", ""),
    ("pkga_calc len \"a\\\"b\"", 0, "1", ""),
    (
        "pkga_calc len \"{a}b\"",
        1,
        "list element in braces followed by \"b\" instead of space",
        "TCL VALUE LIST JUNK",
    ),
    (
        "pkga_calc len \"\\\"a\\\"b\"",
        1,
        "list element in quotes followed by \"b\" instead of space",
        "TCL VALUE LIST JUNK",
    ),
    ("pkga_calc dup {a b} {c d}", 0, "{a b} {a b {c d}}", ""),
    (
        "pkga_calc dup {a {b c}} {}",
        0,
        "{a {b c}} {a {b c} {}}",
        "",
    ),
    ("pkga_calc dup {} x", 0, "{} x", ""),
    ("pkga_calc dup {{}} {{}}", 0, "{{}} {{} {{}}}", ""),
    (
        "pkga_calc dup {a b}",
        1,
        "wrong # args: should be \"pkga_calc dup list element\"",
        "TCL WRONGARGS",
    ),
    ("llength [pkga_calc range 3]", 0, "3", ""),
    (
        "lindex [lindex [pkga_calc dup {a b} {c d}] 1] 2",
        0,
        "c d",
        "",
    ),
    ("expr {[pkga_calc neg 1.5] + 1}", 0, "-0.5", ""),
];

#[test]
fn results_and_errors_match_c_tcl_byte_for_byte() {
    let mut interp = loaded();
    for &(script, code, result, error_code) in CASES {
        assert_eq!(
            catching(&mut interp, script),
            (code, result.to_owned(), error_code.to_owned()),
            "{script}"
        );
    }
}

#[test]
fn client_data_and_delete_procs_work_across_calls() {
    let mut interp = loaded();
    // The counter is a C static shared by every interpreter in the process,
    // so only the increment between two calls is this test's to assert.
    let first: i64 = catching(&mut interp, "pkga_count")
        .1
        .parse()
        .expect("a count");
    assert_eq!(
        catching(&mut interp, "pkga_count"),
        (0, (first + 1).to_string(), String::new())
    );
    assert_eq!(
        catching(&mut interp, "pkga_count extra"),
        (
            1,
            "wrong # args: should be \"pkga_count\"".into(),
            "TCL WRONGARGS".into()
        )
    );
    assert_eq!(
        catching(&mut interp, "pkga_forget"),
        (0, "1".into(), String::new())
    );
    assert_eq!(
        catching(&mut interp, "pkga_count"),
        (
            1,
            "invalid command name \"pkga_count\"".into(),
            "TCL LOOKUP COMMAND pkga_count".into()
        )
    );
    assert_eq!(
        catching(&mut interp, "pkga_forget"),
        (0, "0".into(), String::new())
    );
    assert_eq!(
        interp.commands(),
        ["pkga_calc", "pkga_eq", "pkga_forget", "pkga_quote"]
    );
}

#[test]
fn provided_packages_are_recorded_shim_side() {
    let interp = loaded();
    assert_eq!(
        interp.provided_packages(),
        [("pkga".to_owned(), "1.0".to_owned())]
    );
}
