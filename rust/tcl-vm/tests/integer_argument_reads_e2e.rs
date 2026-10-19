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
use tcl_vm::{CompileService, Vm};

const IOVERFLOW: &str = "{integer value too large to represent} \
                         {ARITH IOVERFLOW {integer value too large to represent}}";

/// `script`'s completion code, result and `-errorcode`, caught as a list.
fn caught(script: &str) -> String {
    caught_under(script, None)
}

fn caught_under(script: &str, release: Option<tcl_dialect::TclVersion>) -> String {
    let src = format!(
        "set rc [catch {{{script}}} m o]\n\
         list $rc $m [expr {{$rc == 1 ? [dict get $o -errorcode] : {{}}}}]\n"
    );
    let mut vm = Vm::new();
    let asm = if let Some(release) = release {
        let profile = tcl_registry::model::ingress::resolve_environment(release.dialect_name())
            .analyser_profile();
        let service = BytecodeCompileService::for_profile(profile);
        let asm = service
            .compile_for_profile(&src, profile)
            .expect("the script compiles for its release");
        vm.set_compiler(Box::new(service));
        vm.set_runtime_version(release);
        asm
    } else {
        let registry = CommandRegistry::build_default();
        let ir = lower_to_ir(&src, &registry);
        let cfg = build_cfg_codegen(&ir, false);
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
        codegen_module(&cfg, &ir, &registry).expect("original source namespace")
    };
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

/// The shared `clock` core reads its clock value, `-base` and `clock add`
/// counts as C 9.0's `ClockParseFmtScnArgs`/`ClockAddObjCmd` do.
#[test]
fn clock_reads_follow_clock_parse_fmt_scn_args() {
    expect(&[
        (
            "clock format 99999999999999999999 -gmt 1",
            "1 {integer value too large to represent} \
             {CLOCK badOption 99999999999999999999}",
        ),
        (
            "clock format [expr {1.5}] -gmt 1",
            "1 {bad seconds \"1.5\": must be now or integer} {CLOCK badOption 1.5}",
        ),
        (
            "clock add 0 abc seconds -gmt 1",
            "1 {bad option \"abc\": must be -gmt, -locale, or -timezone} \
             {CLOCK badOption abc}",
        ),
        (
            "clock add 0 1 w -gmt 1",
            "1 {ambiguous unit \"w\": must be years, months, week, weeks, days, \
             weekdays, hours, minutes, or seconds} {TCL LOOKUP INDEX unit w}",
        ),
        (
            "clock add 0 2147483648 days -gmt 1",
            "1 {integer value too large to represent} NONE",
        ),
        (
            "clock scan 1970 -format %Y -base abc -gmt 1",
            "1 {bad seconds \"abc\": must be now or integer} {CLOCK badOption abc}",
        ),
        (
            "clock scan abc -format %Y -gmt 1",
            "1 {input string does not match supplied format} {CLOCK badInputString}",
        ),
        (
            "clock format 0 -gmt",
            "1 {wrong # args: should be \"clock format clockval|now ?-format string? \
             ?-gmt boolean? ?-locale LOCALE? ?-timezone ZONE?\"} {CLOCK wrongNumArgs}",
        ),
        ("clock add 0 5 weekdays -gmt 1", "0 604800 {}"),
        ("clock format 0x10 -gmt 1 -format %s", "0 16 {}"),
        ("string is integer [clock format now -format %s]", "0 1 {}"),
    ]);
}

/// A value already holding a double is `TCL VALUE INTEGER` to every integer
/// read, the shared cores' and `incr`'s alike.
#[test]
fn a_held_double_is_tcl_value_integer() {
    for (script, word) in [
        ("string repeat x [expr {1.5}]", "1.5"),
        ("lrepeat [expr {1.5}] a", "1.5"),
        ("format %c [expr {1.5}]", "1.5"),
        ("format %d [expr {1.5}]", "1.5"),
        ("format %x [expr {2.0}]", "2.0"),
        ("lsort -stride [expr {2.0}] {a b c d}", "2.0"),
        ("set v [expr {2.0}]; incr v", "2.0"),
        ("proc p {} {set v [expr {2.0}]; incr v}; p", "2.0"),
        ("set d {a 1}; dict incr d a [expr {1.5}]", "1.5"),
        ("set d {a 1}; dict incr d b [expr {2.0}]", "2.0"),
        (
            "proc p {} {set d [dict create a [expr {2.0}]]; dict incr d a}; p",
            "2.0",
        ),
    ] {
        assert_eq!(
            caught(script),
            format!("1 {{expected integer but got \"{word}\"}} {{TCL VALUE INTEGER}}"),
            "{script}"
        );
    }
}

/// `regexp`/`regsub` refusals carry C 9.0's codes through the shared core.
#[test]
fn regexp_refusals_carry_their_codes() {
    let bad_index = |word: &str| {
        format!(
            "1 {{bad index \"{word}\": must be integer?[+-]integer? or end?[+-]integer?}} \
             {{TCL VALUE INDEX}}"
        )
    };
    let unbalanced = "1 {cannot compile regular expression pattern: parentheses () not balanced} \
                      {REGEXP REG_EPAREN {parentheses () not balanced}}";
    expect(&[
        ("regexp -start abc a abc", &bad_index("abc")),
        ("regsub -start 1.5 a abc b", &bad_index("1.5")),
        (
            "regexp -inline a abc x",
            "1 {regexp match variables not allowed when using -inline} \
             {TCL OPERATION REGEXP MIX_VAR_INLINE}",
        ),
        ("regexp {(} abc", unbalanced),
        ("regsub {(} abc x", unbalanced),
        ("regexp -start 99999999999999999999 a abc", "0 0 {}"),
    ]);
}

/// `lseq` reads its words as numbers with C 9.0's codes, never as expressions.
#[test]
fn lseq_words_read_as_numbers_with_c_codes() {
    overflows(&[
        "lseq 99999999999999999999",
        "lseq 0 count 99999999999999999999",
    ]);
    expect(&[
        (
            "lseq 1+2 to 10",
            "1 {expected number but got \"1+2\"} {TCL VALUE NUMBER}",
        ),
        (
            "lseq 1.5",
            "1 {expected integer but got \"1.5\"} {TCL VALUE INTEGER}",
        ),
        ("lseq 0 to", "1 {missing \"to\" value.} NONE"),
        (
            "lseq 1 2 3 4",
            "1 {bad operation \"3\": must be .., to, count, or by} \
             {TCL LOOKUP INDEX operation 3}",
        ),
        ("lseq 0 to 3 by 0", "0 0 {}"),
        ("lseq 0 count 2.0", "0 {0 1} {}"),
    ]);
}

/// `return -level`/`-code` are merged last-wins, then read as C `int`s.
#[test]
fn return_level_and_code_words_are_validated_once_merged() {
    for word in [
        "99999999999999999999",
        "4294967295",
        "2147483648",
        "abc",
        "1.5",
        "-1",
    ] {
        assert_eq!(
            caught(&format!("return -level {word} x")),
            format!(
                "1 {{bad -level value: expected non-negative integer but got \"{word}\"}} \
                 {{TCL RESULT ILLEGAL_LEVEL}}"
            ),
            "{word}"
        );
    }
    for word in ["99999999999999999999", "4294967296", "abc", "1.5"] {
        assert_eq!(
            caught(&format!("return -code {word} x")),
            format!(
                "1 {{bad completion code \"{word}\": must be ok, error, return, break, \
                 continue, or an integer}} {{TCL RESULT ILLEGAL_CODE}}"
            ),
            "{word}"
        );
    }
    expect(&[
        (
            "proc p {} {return -level abc x}; p",
            "1 {bad -level value: expected non-negative integer but got \"abc\"} \
             {TCL RESULT ILLEGAL_LEVEL}",
        ),
        (
            "return -options {-code abc} x",
            "1 {bad completion code \"abc\": must be ok, error, return, break, continue, \
             or an integer} {TCL RESULT ILLEGAL_CODE}",
        ),
        ("list [catch {return -level 0x1 x} m] $m", "0 {2 x} {}"),
        (
            "list [catch {return -level abc -level 0 x} m] $m",
            "0 {0 x} {}",
        ),
        (
            "list [catch {return -level 0 -code 0x3 x} m] $m",
            "0 {3 x} {}",
        ),
    ]);
}

/// `uplevel`/`upvar` levels resolve through the shared `TclObjGetFrame`
/// owner, compiled `upvar` included.
#[test]
fn level_words_resolve_as_tcl_obj_get_frame_does() {
    let bad = |word: &str| format!("1 {{bad level \"{word}\"}} {{TCL LOOKUP LEVEL {word}}}");
    expect(&[
        (
            "uplevel 99999999999999999999 {set a 1}",
            &bad("99999999999999999999"),
        ),
        ("uplevel 0x10 {set a 1}", &bad("0x10")),
        (
            "proc p {} {uplevel 4294967295 {set x 3}}; p",
            &bad("4294967295"),
        ),
        ("upvar 5 ::x y", &bad("5")),
        ("upvar ::x y", &bad("1")),
        ("upvar #1 x y", &bad("#1")),
        ("proc p {} {upvar 1.0 x y}; p", &bad("1.0")),
        ("proc p {} {upvar 2 x y}; p", &bad("2")),
        ("proc p {} {upvar #2 x y}; p", &bad("#2")),
        ("uplevel abc {set a 1}", &bad("1")),
        (
            "proc p {} {upvar 0x1 x y; set y 1}; set x 0; p; set x",
            "0 1 {}",
        ),
        ("proc p {} {uplevel +1 {set x 4}}; p; set x", "0 4 {}"),
        ("proc p {} {upvar 1 x y z}; p", "0 {} {}"),
    ]);
}

/// `after` reads its delay wide and names a bad word under
/// `TCL LOOKUP INDEX argument`.
#[test]
fn after_delays_read_wide_with_their_code() {
    for word in ["99999999999999999999", "abc", "1.5"] {
        assert_eq!(
            caught(&format!("after {word} {{}}")),
            format!(
                "1 {{bad argument \"{word}\": must be cancel, idle, info, or an integer}} \
                 {{TCL LOOKUP INDEX argument {word}}}"
            ),
            "{word}"
        );
    }
    expect(&[
        ("after 0x10 {}; set r ok", "0 ok {}"),
        (
            "after idle",
            "1 {wrong # args: should be \"after idle script ?script ...?\"} {TCL WRONGARGS}",
        ),
    ]);
}

/// A missing key on a `dict get`/`unset` path is `TCL LOOKUP DICT <key>`,
/// interpreted and compiled.
#[test]
fn dict_path_misses_are_tcl_lookup_dict() {
    let miss = |key: &str| {
        format!("1 {{key \"{key}\" not known in dictionary}} {{TCL LOOKUP DICT {key}}}")
    };
    expect(&[
        ("dict get {a {b 1}} a c", &miss("c")),
        ("proc p {} {set d {a {b 1}}; dict get $d x}; p", &miss("x")),
        ("set d {a {b 1}}; dict unset d x y", &miss("x")),
        (
            "proc p {} {set d {a {b 1}}; dict unset d x y}; p",
            &miss("x"),
        ),
        ("set d {a {b 1}}; dict unset d a y; set d", "0 {a {b 1}} {}"),
    ]);
}

/// `r+`, `w+`, `a+` and `RDWR` channels both read and write one offset.
#[test]
fn read_write_channels_read_and_write() {
    let path = std::env::temp_dir().join(format!("tclvm_chan_rw_{}.txt", std::process::id()));
    let p = path.display();
    let cases: &[(&str, &str, &str)] = &[
        ("w", "puts -nonewline $f hello; set r {}", "0 {} {}"),
        (
            "r+",
            "set r [read $f 2]; puts -nonewline $f XY; seek $f 0; append r | [read $f]",
            "0 he|heXYo {}",
        ),
        (
            "w+",
            "puts -nonewline $f new; seek $f 0; set r [read $f]",
            "0 new {}",
        ),
        (
            "a+",
            "puts -nonewline $f +tail; seek $f 0; set r [read $f]",
            "0 new+tail {}",
        ),
        ("a+", "set r [list [tell $f] [read $f]]", "0 {8 {}} {}"),
        (
            "RDWR",
            "set r [read $f 3]; puts -nonewline $f Q; seek $f 0; append r | [read $f]",
            "0 new|newQtail {}",
        ),
        (
            "{RDWR TRUNC}",
            "puts $f l1; puts $f l2; seek $f 0; set r [list [gets $f] [gets $f] [gets $f] [eof $f]]",
            "0 {l1 l2 {} 1} {}",
        ),
    ];
    for &(mode, body, expected) in cases {
        let script = format!("set f [open {p} {mode}]; {body}; close $f; set r");
        assert_eq!(caught(&script), expected, "{mode}: {body}");
    }
    std::fs::remove_file(&path).ok();
}

/// The two answers the releases split on, as tclsh 8.5.19, 8.6.18, 9.0.4 and
/// 9.1.0 give them: how a failed `-index` step is named (and whether it is
/// coded), and which `clock add` unit words are read.
#[test]
fn index_steps_and_clock_units_follow_the_release() {
    use tcl_dialect::TclVersion::{V8_5, V8_6, V9_0, V9_1};
    let unit_miss = |word: &str| {
        format!(
            "1 {{unknown unit \"{word}\", must be  years, months, weeks, days, hours, minutes or seconds}} \
             {{CLOCK badUnit {word}}}"
        )
    };
    let rows: Vec<(&str, tcl_dialect::TclVersion, String)> = vec![
        (
            "lsort -index end-5 {{a b} {c d}}",
            V8_5,
            "1 {element -4 missing from sublist \"a b\"} NONE".into(),
        ),
        (
            "lsort -index end-5 {{a b} {c d}}",
            V8_6,
            "1 {element -4 missing from sublist \"a b\"} {TCL OPERATION LSORT INDEXFAILED}".into(),
        ),
        (
            "lsearch -index end-5 {{a b} {c d}} x",
            V9_0,
            "1 {element end-5 missing from sublist \"a b\"} {TCL OPERATION LSORT INDEXFAILED}"
                .into(),
        ),
        ("clock add 0 1 weekdays -gmt 1", V8_6, unit_miss("weekdays")),
        ("clock add 0 1 d -gmt 1", V8_5, unit_miss("d")),
        ("clock add 0 1 days -gmt 1", V8_6, "0 86400 {}".into()),
        ("clock add 0 1 d -gmt 1", V9_0, "0 86400 {}".into()),
        ("clock add 0 1 weekdays -gmt 1", V9_1, "0 86400 {}".into()),
    ];
    for (script, release, expected) in rows {
        assert_eq!(
            caught_under(script, Some(release)),
            expected,
            "{script} {release:?}"
        );
    }
}
