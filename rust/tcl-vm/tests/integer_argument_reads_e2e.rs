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

//! The VM's integer reads (`Value::as_int`, `Value::as_c_int` and the shared
//! command cores over them) fail with C's message and `-errorcode`.
//!
//! Every expectation was measured on tclsh 9.0.4, each script caught inside
//! one interpreter; 8.6.18 and 9.1.0 agree unless a test says otherwise.

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::Vm;

const IOVERFLOW: &str = "{integer value too large to represent} \
                         {ARITH IOVERFLOW {integer value too large to represent}}";

/// `script`'s completion code, result and `-errorcode`, caught as a list.
fn caught(script: &str) -> String {
    let src = format!(
        "set rc [catch {{{script}}} m o]\n\
         list $rc $m [expr {{$rc == 1 ? [dict get $o -errorcode] : {{}}}}]\n"
    );
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(&src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    let completion = vm.run_module(&asm);
    assert!(
        completion.code.is_ok(),
        "{script}: {}",
        completion.result.to_str()
    );
    completion.result.to_str().to_string()
}

#[track_caller]
fn expect(cases: &[(&str, &str)]) {
    for &(script, expected) in cases {
        assert_eq!(caught(script), expected, "{script}");
    }
}

#[track_caller]
fn overflows(scripts: &[&str]) {
    for &script in scripts {
        assert_eq!(caught(script), format!("1 {IOVERFLOW}"), "{script}");
    }
}

/// tclsh 8.6.18/9.0.4 read `interp recursionlimit` wide (8.6 reads a C
/// `int`), answer a new limit with the word they were given, and keep the
/// typed read's code in every form.
#[test]
fn recursion_limits_read_through_the_shared_integer_read() {
    overflows(&[
        "interp recursionlimit {} 99999999999999999999",
        "interp recursionlimit {} 9223372036854775808",
        "interp create kj; interp recursionlimit kj 99999999999999999999",
        "interp create k; k recursionlimit 99999999999999999999",
    ]);
    expect(&[
        (
            "interp recursionlimit {} abc",
            "1 {expected integer but got \"abc\"} {TCL VALUE NUMBER}",
        ),
        (
            "interp recursionlimit {} 0",
            "1 {recursion limit must be > 0} {TCL OPERATION INTERP BADLIMIT}",
        ),
        ("interp recursionlimit {} 0x10", "0 0x10 {}"),
        (
            "interp create kk; interp recursionlimit kk 0x20",
            "0 0x20 {}",
        ),
        (
            "interp create ki; interp recursionlimit ki 0x20; interp recursionlimit ki",
            "0 32 {}",
        ),
        ("interp create k; k recursionlimit 0x10", "0 0x10 {}"),
    ]);
}

/// `interp limit`: `-granularity` is a C `int`, the values are wide, and a
/// value out of range is `BADVALUE`.
#[test]
fn interp_limit_values_read_through_the_shared_integer_read() {
    overflows(&[
        "interp create c; interp limit c commands -value 99999999999999999999",
        "interp create c; interp limit c commands -granularity 4294967296",
        "interp create c; interp limit c time -seconds 99999999999999999999",
    ]);
    expect(&[
        (
            "interp create c; interp limit c commands -value -1",
            "1 {command limit value must be at least 0} {TCL OPERATION INTERP BADVALUE}",
        ),
        (
            "interp create c; interp limit c commands -value 0x10; interp limit c commands -value",
            "0 16 {}",
        ),
    ]);
}

/// The shared command cores read counts through the VM's value seam, so a
/// beyond-wide count is the overflow and a non-integer `TCL VALUE NUMBER`.
#[test]
fn shared_cores_report_c_integer_read_failures() {
    overflows(&[
        "string repeat x 99999999999999999999",
        "lrepeat 99999999999999999999 a",
        "format %c 4294967296",
        "format %*d 99999999999999999999 1",
        "string equal -length 9223372036854775808 a b",
        "lsort -stride 99999999999999999999 {a b c d}",
        "lsort -integer {99999999999999999999 1}",
        "lsearch -exact -integer {1 2} 99999999999999999999",
        "info level 4294967296",
        "time {} 4294967296",
    ]);
    expect(&[
        (
            "string repeat x abc",
            "1 {expected integer but got \"abc\"} {TCL VALUE NUMBER}",
        ),
        (
            "string equal -length abc a b",
            "1 {expected integer but got \"abc\"} {TCL VALUE NUMBER}",
        ),
        (
            "set x 1; incr x abc",
            "1 {expected integer but got \"abc\"} {TCL VALUE NUMBER}",
        ),
        (
            "time {} abc",
            "1 {expected integer but got \"abc\"} {TCL VALUE NUMBER}",
        ),
        (
            "exit abc",
            "1 {expected integer but got \"abc\"} {TCL VALUE NUMBER}",
        ),
        (
            "lrepeat -1 a",
            "1 {bad count \"-1\": must be integer >= 0} {TCL OPERATION LREPEAT NEGARG}",
        ),
        (
            "lsort -stride 3 {a b c d}",
            "1 {list size must be a multiple of the stride length} {TCL OPERATION LSORT BADSTRIDE}",
        ),
        (
            "format %.*f 2147483648 1.0",
            "1 {max size for a Tcl value exceeded} {TCL FORMAT OVERFLOW}",
        ),
        ("format %.2147483648s abc", "0 abc {}"),
        ("time {} 4294967295", "0 {0 microseconds per iteration} {}"),
        ("set n 0; time {incr n} 0x3; set n", "0 3 {}"),
        (
            "lsort -command {apply {{a b} {return 99999999999999999999}}} {1 2}",
            "1 {-compare command returned non-integer result} {TCL OPERATION LSORT COMPARISONFAILED}",
        ),
        (
            "lsort -command {apply {{a b} {return 4294967295}}} {1 2}",
            "0 {1 2} {}",
        ),
        (
            "info level 4294967295",
            "1 {bad level \"4294967295\"} {TCL LOOKUP STACK_LEVEL 4294967295}",
        ),
    ]);
}

/// `-index` paths encode as `TclIndexEncode` does, and a bad index carries
/// `TCL VALUE INDEX` (tclsh 9.0.4/9.1.0).
#[test]
fn index_reads_carry_their_codes() {
    expect(&[
        (
            "string index abc abc",
            "1 {bad index \"abc\": must be integer?[+-]integer? or end?[+-]integer?} {TCL VALUE INDEX}",
        ),
        (
            "lsort -index 99999999999999999999 {{a b}}",
            "1 {index \"99999999999999999999\" out of range} {TCL VALUE INDEX OUTOFRANGE}",
        ),
        (
            "lsort -index 0x10 {{a b}}",
            "1 {element 16 missing from sublist \"a b\"} {TCL OPERATION LSORT INDEXFAILED}",
        ),
        (
            "lsort -index end-5 {{a b}}",
            "1 {element end-5 missing from sublist \"a b\"} {TCL OPERATION LSORT INDEXFAILED}",
        ),
        (
            "lsort -stride 2 -index 5 {a b c d}",
            "1 {when used with \"-stride\", the leading \"-index\" value must be within the group} \
             {TCL OPERATION LSORT BADINDEX}",
        ),
    ]);
}

/// The ensemble option parser keeps each option's code (tclsh
/// 8.6.18/9.0.4/9.1.0).
#[test]
fn ensemble_options_keep_their_error_codes() {
    expect(&[
        (
            "namespace ensemble create -command ::q -map {} -prefixes {yes }",
            "1 {expected boolean value but got \"yes \"} {TCL VALUE NUMBER}",
        ),
        (
            "namespace ensemble create -command ::q -map {}; \
             namespace ensemble configure ::q -prefixes abc",
            "1 {expected boolean value but got \"abc\"} {TCL VALUE NUMBER}",
        ),
        (
            "namespace ensemble create -command ::q -foo x",
            "1 {bad option \"-foo\": must be -command, -map, -parameters, -prefixes, \
             -subcommands, or -unknown} {TCL LOOKUP INDEX option -foo}",
        ),
        (
            "namespace ensemble create -command ::q -map {}; \
             namespace ensemble configure ::q -foo x",
            "1 {bad option \"-foo\": must be -map, -namespace, -parameters, -prefixes, \
             -subcommands, or -unknown} {TCL LOOKUP INDEX option -foo}",
        ),
    ]);
}
