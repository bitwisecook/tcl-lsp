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

//! The extension every host is held to, `pkga.c`, compiled for this host
//! against the header's WASM leg (`build.rs`) and loaded into the runtime
//! through its C API exports, as a `wasm32` side module is: the same source,
//! the same declarations, and the vectors `tclsh9.0` gave for it
//! (`rust/tcl-cshim/tests/vectors/pkga.rs`), so each export is held to C Tcl's
//! bytes before anything runs under WASM.

#![cfg(runtime_c_tests)]

use core::ffi::{c_int, c_void};

use tcl_runtime::capi::{tcl_test_double_free_count, tcl_test_finalize, tcl_test_reset_counters};
use tcl_runtime::interp::{Code, Interp};

#[link(name = "runtime_pkga", kind = "static")]
unsafe extern "C" {
    // Declared over `void *`: the interpreter is opaque to C, and an `extern`
    // block naming a Rust type would trip the FFI-safety lint.
    fn Pkga_Init(interp: *mut c_void) -> c_int;
}

include!("../../../rust/tcl-cshim/tests/vectors/pkga.rs");

/// Where the runtime's answer is not C Tcl's, what it is instead and why: the
/// script, its code, result and `-errorcode` here, and the reason. C Tcl keeps
/// a NUL in a value as the two bytes `C0 80`, so C code reading the value as a C
/// string sees all of it; the runtime keeps it as one byte, so `strncpy` and
/// `Tcl_AppendResult` over `Tcl_GetString` stop there. When the runtime's string
/// representation changes, each row moves back to [`CASES`].
const DIVERGENCES: &[(&str, i64, &str, &str, &str)] = &[
    (
        "pkga_calc join a\\u0000b c",
        0,
        "a+c",
        "",
        "`Tcl_AppendResult` reads `a\\0b` as the C string `a`",
    ),
    (
        "pkga_calc fail x\\u0000y",
        1,
        "x",
        "PKGA FAIL x",
        "`strncpy` copies `x\\0y` up to its NUL, and the code is built from the copy",
    ),
];

/// Run the entry point on `interp`, answering its code.
fn init(interp: &mut Interp) -> c_int {
    // SAFETY: `Pkga_Init` is the extension built against the runtime's own
    // exports, and the interpreter is live for the call.
    unsafe { Pkga_Init(std::ptr::from_mut(interp).cast::<c_void>()) }
}

fn loaded() -> Interp {
    let mut interp = Interp::new();
    assert_eq!(
        init(&mut interp),
        0,
        "{}",
        String::from_utf8_lossy(&interp.result_bytes())
    );
    interp
}

/// Evaluate `script`, answering its code and result.
fn run(interp: &mut Interp, script: &str) -> (Code, String) {
    let code = interp.eval_str(script.as_bytes());
    (
        code,
        String::from_utf8_lossy(&interp.result_bytes()).into_owned(),
    )
}

/// Run `script` under `catch`, answering `(code, result, errorCode)` the way the
/// reference probe recorded them.
fn catching(interp: &mut Interp, script: &str) -> (i64, String, String) {
    let (status, code) = run(interp, &format!("catch {{{script}}} ::r"));
    assert_eq!(status, Code::Ok, "catch runs");
    let code: i64 = code.parse().expect("a code");
    let (_, result) = run(interp, "set ::r");
    let error_code = if code == 0 {
        String::new()
    } else {
        run(interp, "set ::errorCode").1
    };
    (code, result, error_code)
}

fn diverges(script: &str) -> bool {
    DIVERGENCES.iter().any(|row| row.0 == script)
}

/// Whether `script` calls `expr`, which a runtime built without the numeric
/// tower does not have.
fn needs_the_numeric_tower(script: &str) -> bool {
    !cfg!(have_tommath) && script.contains("expr")
}

#[test]
fn every_vector_answers_what_c_tcl_answers() {
    let mut interp = loaded();
    let mut differ = Vec::new();
    for &(script, code, result, error_code) in CASES {
        if diverges(script) || needs_the_numeric_tower(script) {
            continue;
        }
        let wanted = (code, result.to_owned(), error_code.to_owned());
        let got = catching(&mut interp, script);
        if got != wanted {
            differ.push(format!(
                "{script:?}\n    C Tcl {wanted:?}\n    here  {got:?}"
            ));
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
fn where_the_runtime_differs_the_difference_is_the_recorded_one() {
    let mut interp = loaded();
    for &(script, code, result, error_code, why) in DIVERGENCES {
        let c_tcl = CASES
            .iter()
            .find(|row| row.0 == script)
            .expect("a recorded difference is a vector");
        let here = (code, result.to_owned(), error_code.to_owned());
        assert_ne!(
            (c_tcl.1, c_tcl.2.to_owned(), c_tcl.3.to_owned()),
            here,
            "{script}: not a difference"
        );
        assert_eq!(catching(&mut interp, script), here, "{script}: {why}");
    }
}

#[test]
fn the_entry_point_provides_the_package_and_registers_its_commands() {
    let mut interp = Interp::new();
    assert_eq!(
        run(&mut interp, "info commands pkga_*"),
        (Code::Ok, String::new())
    );
    assert_eq!(init(&mut interp), 0);
    assert_eq!(
        run(&mut interp, "lsort [info commands pkga_*]"),
        (
            Code::Ok,
            "pkga_calc pkga_count pkga_eq pkga_forget pkga_quote".to_owned()
        )
    );
    assert_eq!(
        run(&mut interp, "package provide pkga"),
        (Code::Ok, "1.0".to_owned())
    );
    assert_eq!(
        run(&mut interp, "info cmdtype pkga_calc"),
        (Code::Ok, "native".to_owned())
    );
    assert_eq!(
        init(&mut interp),
        0,
        "a second entry provides the same version, which package provide allows"
    );
}

#[test]
fn a_package_provided_at_another_version_fails_the_entry_point_before_it_registers_anything() {
    let mut interp = Interp::new();
    assert_eq!(
        run(&mut interp, "package provide pkga 2.0"),
        (Code::Ok, String::new())
    );
    assert_eq!(init(&mut interp), 1);
    assert_eq!(
        String::from_utf8_lossy(&interp.result_bytes()),
        "conflicting versions provided for package \"pkga\": 2.0, then 1.0",
        "the entry point returned the package command's own error"
    );
    assert_eq!(
        run(&mut interp, "info commands pkga_*"),
        (Code::Ok, String::new()),
        "and it returned before it created a command"
    );
    assert_eq!(
        run(&mut interp, "package provide pkga"),
        (Code::Ok, "2.0".to_owned())
    );
}

#[test]
fn client_data_and_the_delete_procedure_work_across_calls() {
    let mut interp = loaded();
    // The counter is a C static shared by every interpreter in the process, so
    // only the increment between two calls is this test's to assert.
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
            "wrong # args: should be \"pkga_count\"".to_owned(),
            "TCL WRONGARGS".to_owned()
        )
    );
    assert_eq!(
        catching(&mut interp, "pkga_forget"),
        (0, "1".to_owned(), String::new()),
        "the delete procedure ran when the command went"
    );
    assert_eq!(
        run(&mut interp, "info commands pkga_count"),
        (Code::Ok, String::new())
    );
    assert_eq!(
        catching(&mut interp, "pkga_forget"),
        (0, "0".to_owned(), String::new())
    );
}

#[test]
fn each_error_the_c_code_reports_carries_its_own_code() {
    let mut interp = loaded();
    assert_eq!(
        run(
            &mut interp,
            "catch {error first {} {MY CODE}}; list [catch {pkga_calc fail boom} m] $m $::errorCode"
        ),
        (Code::Ok, "1 boom {PKGA FAIL boom}".to_owned())
    );
    assert_eq!(
        run(
            &mut interp,
            "list [catch {pkga_calc add x 1} m] $m $::errorCode"
        ),
        (
            Code::Ok,
            "1 {expected integer but got \"x\"} {TCL VALUE NUMBER}".to_owned()
        ),
        "the code the last error stated, not the one before it"
    );
}

#[test]
fn the_vectors_leave_nothing_allocated() {
    tcl_test_reset_counters();
    {
        let mut interp = loaded();
        for &(script, ..) in CASES {
            catching(&mut interp, script);
        }
    }
    assert_eq!(
        tcl_test_finalize(),
        0,
        "every object the calls made is freed"
    );
    assert_eq!(tcl_test_double_free_count(), 0);
}
