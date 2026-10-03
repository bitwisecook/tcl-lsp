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

// The test extension `tests/c/pkga.c`'s conformance vectors: each script, run
// under `catch` in `tclsh9.0` with the same `pkga.c` built against Tcl 9.0.4's
// own `tcl.h` loaded, and the code, result and `$errorCode` it gave. Included
// by every host's test of the extension (`tests/pkga_e2e.rs` here, the WASM
// runtime's `tests/pkga_extension.rs`, and the runtime under wasmtime in
// `rust/tcl-engine-wasm`'s `tests/under_wasm.rs`), so each is held to the same
// bytes.

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
        "pkga_calc not NaN",
        1,
        "floating point value is Not a Number",
        "TCL VALUE DOUBLE NAN",
    ),
    (
        "pkga_calc not -NaN",
        1,
        "floating point value is Not a Number",
        "TCL VALUE DOUBLE NAN",
    ),
    ("pkga_calc not Inf", 0, "0", ""),
    ("pkga_eq a\\u0000b a\\u0000b", 0, "1", ""),
    ("pkga_eq a\\u0000b a\\u0000c", 0, "0", ""),
    ("pkga_eq a\\u0000b a", 0, "0", ""),
    ("pkga_calc join a\\u0000b c", 0, "a\0b+c", ""),
    ("pkga_calc fail x\\u0000y", 1, "x\0y", "PKGA FAIL x\0y"),
    ("pkga_calc len \"a\\u0000b c\"", 0, "2", ""),
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
