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

//! Oracle-pinned coverage for the integer arguments commands read: counts,
//! levels, limits, option values and `-index` paths, each failing with C's
//! message and `-errorcode`.
//!
//! Every expectation was measured on tclsh 9.0.4, each script in a fresh
//! interpreter; 8.6.18 and 9.1.0 agree unless a test says otherwise. The
//! readers split three ways in C: a wide read (`Tcl_GetWideIntFromObj`, so
//! past `INT64_MAX` is `ARITH IOVERFLOW`), a C `int` read (`Tcl_GetIntFromObj`,
//! which truncates up to `UINT_MAX`, so `4294967295` reads -1 and
//! `4294967296` overflows), and an index read (`TclGetIntForIndex`, whose
//! failure is `bad index` with `TCL VALUE INDEX`).

#![cfg(have_tommath)]

use tcl_runtime::interp::{Code, Interp};

const IOVERFLOW: (&str, &str) = (
    "integer value too large to represent",
    "ARITH IOVERFLOW {integer value too large to represent}",
);

fn run(script: &str) -> (Code, String, String) {
    let mut interp = Interp::new();
    let code = interp.eval_str(script.as_bytes());
    let result = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
    let error_code = if code == Code::Error {
        interp.eval_str(b"set ::errorCode");
        String::from_utf8_lossy(&interp.result_bytes()).into_owned()
    } else {
        String::new()
    };
    (code, result, error_code)
}

#[track_caller]
fn fails(cases: &[(&str, &str, &str)]) {
    for &(script, message, error_code) in cases {
        let (code, result, actual_code) = run(script);
        assert_eq!(code, Code::Error, "{script} → {result}");
        assert_eq!(result, message, "{script} message");
        assert_eq!(actual_code, error_code, "{script} errorCode");
    }
}

#[track_caller]
fn overflows(scripts: &[&str]) {
    for &script in scripts {
        fails(&[(script, IOVERFLOW.0, IOVERFLOW.1)]);
    }
}

#[track_caller]
fn yields(cases: &[(&str, &str)]) {
    for &(script, expected) in cases {
        let (code, result, _) = run(script);
        assert_eq!((code, result.as_str()), (Code::Ok, expected), "{script}");
    }
}

/// A count read wide: past the wide range is C's overflow, not a non-integer,
/// and every spelling `Tcl_GetWideIntFromObj` takes is a count.
#[test]
fn wide_counts_overflow_with_the_arith_code() {
    overflows(&[
        "string repeat x 99999999999999999999",
        "string repeat x 9223372036854775808",
        "string repeat x -9999999999999999999",
        "lrepeat 99999999999999999999 a",
        "lrepeat -99999999999999999999 a",
        "format %*d 99999999999999999999 1",
        "format %.*f 99999999999999999999 1.0",
        "string equal -length 9223372036854775808 a b",
        "string equal -length -99999999999999999999 a b",
        "string compare -length 99999999999999999999 a b",
        "lsort -stride 99999999999999999999 {a b c d}",
        "lsearch -stride 99999999999999999999 {a b c d} a",
        "lsort -integer {99999999999999999999 1}",
        "lsearch -exact -integer {1 2} 99999999999999999999",
        "lsearch -exact -integer {99999999999999999999 2} 2",
        "lsearch -sorted -integer {1 99999999999999999999 3} 3",
        "timerate {} 99999999999999999999",
        "timerate {} 1 99999999999999999999",
    ]);
    yields(&[
        ("string repeat x 0x2", "xx"),
        ("string repeat x { 2 }", "xx"),
        ("string repeat x +2", "xx"),
        ("string repeat x -1", ""),
        ("lrepeat 0x2 a", "a a"),
        ("lrepeat { 2 } a", "a a"),
        ("string equal -length 0x1 ab ac", "1"),
        ("lsort -integer {1 2 0x10}", "1 2 0x10"),
        ("format %d 99999999999999999999", "1661992959"),
    ]);
}

/// tclsh 9.0/9.1 (8.6 codes a string non-integer `TCL VALUE INTEGER`).
#[test]
fn a_non_integer_count_is_tcl_value_number() {
    for script in [
        "string repeat x abc",
        "lrepeat abc a",
        "format %c abc",
        "format %*d abc 1",
        "format %d abc",
        "string equal -length abc a b",
        "lsort -stride abc {a b c d}",
        "lsearch -stride abc {a b c d} a",
        "time {} abc",
        "info level abc",
        "info frame abc",
        "interp recursionlimit {} abc",
        "zlib compress x abc",
        "binary format c abc",
        "timerate {} abc",
        "timerate {} 1 abc",
        "exit abc",
    ] {
        let word = if script.contains("1.5") { "1.5" } else { "abc" };
        fails(&[(
            script,
            &format!("expected integer but got \"{word}\""),
            "TCL VALUE NUMBER",
        )]);
    }
    fails(&[
        (
            "string repeat x 1.5",
            "expected integer but got \"1.5\"",
            "TCL VALUE NUMBER",
        ),
        (
            "lsort -integer {1.5 1}",
            "expected integer but got \"1.5\"",
            "TCL VALUE NUMBER",
        ),
        (
            "lsearch -exact -integer {abc 2} 2",
            "expected integer but got \"abc\"",
            "TCL VALUE NUMBER",
        ),
        (
            "lsearch -sorted -integer {1 2} 1.5",
            "expected integer but got \"1.5\"",
            "TCL VALUE NUMBER",
        ),
        (
            "lsearch -exact -real {1 2} abc",
            "expected floating-point number but got \"abc\"",
            "TCL VALUE NUMBER",
        ),
        (
            "binary format c2 {1 abc}",
            "expected integer but got \"abc\"",
            "TCL VALUE NUMBER",
        ),
    ]);
}

/// A C `int` read takes `INT_MIN..=UINT_MAX`, truncating the unsigned half:
/// `4294967295` is -1 and `4294967296` overflows.
#[test]
fn c_int_reads_truncate_to_uint_max_and_overflow_past_it() {
    overflows(&[
        "format %c 4294967296",
        "format %c 99999999999999999999",
        "time {} 4294967296",
        "time {} 99999999999999999999",
        "info level 4294967296",
        "info frame 4294967296",
        "zlib compress x 4294967296",
        "zlib crc32 abc 4294967296",
        "interp create c; interp limit c commands -granularity 4294967296",
    ]);
    yields(&[
        ("time {} 4294967295", "0 microseconds per iteration"),
        ("time {} 2147483648", "0 microseconds per iteration"),
        ("set n 0; time {incr n} 0x3; set n", "3"),
        ("format %c 0x41", "A"),
        ("zlib crc32 abc 4294967295", "899311407"),
        (
            "lsort -command {apply {{a b} {return 4294967295}}} {1 2}",
            "1 2",
        ),
    ]);
    fails(&[
        (
            "zlib compress x 4294967295",
            "level must be 0 to 9",
            "TCL VALUE COMPRESSIONLEVEL",
        ),
        (
            "interp create c; interp limit c commands -granularity 4294967295",
            "granularity must be at least 1",
            "TCL OPERATION INTERP BADVALUE",
        ),
        (
            "info level 4294967295",
            "bad level \"4294967295\"",
            "TCL LOOKUP STACK_LEVEL 4294967295",
        ),
    ]);
}

/// The out-of-range errors beside the integer reads carry their own codes.
#[test]
fn count_range_errors_carry_their_codes() {
    fails(&[
        (
            "lrepeat -1 a",
            "bad count \"-1\": must be integer >= 0",
            "TCL OPERATION LREPEAT NEGARG",
        ),
        (
            "lsort -stride 3 {a b c d}",
            "list size must be a multiple of the stride length",
            "TCL OPERATION LSORT BADSTRIDE",
        ),
        (
            "lsort -stride -1 {a b c d}",
            "stride length must be at least 2",
            "TCL OPERATION LSORT BADSTRIDE",
        ),
        (
            "lsearch -stride 3 {a b c d} a",
            "list size must be a multiple of the stride length",
            "TCL OPERATION LSEARCH BADSTRIDE",
        ),
        (
            "lsearch -stride -1 {a b c d} a",
            "stride length must be at least 1",
            "TCL OPERATION LSEARCH BADSTRIDE",
        ),
        (
            "format %.*f 2147483648 1.0",
            "max size for a Tcl value exceeded",
            "TCL FORMAT OVERFLOW",
        ),
        (
            "format %.*f 2147483647 1.0",
            "max size for a Tcl value exceeded",
            "TCL FORMAT OVERFLOW",
        ),
        (
            "format %.2147483648f 1.0",
            "max size for a Tcl value exceeded",
            "TCL FORMAT OVERFLOW",
        ),
        (
            "lsort -command {apply {{a b} {return x}}} {1 2}",
            "-compare command returned non-integer result",
            "TCL OPERATION LSORT COMPARISONFAILED",
        ),
        (
            "lsort -command {apply {{a b} {return 99999999999999999999}}} {1 2}",
            "-compare command returned non-integer result",
            "TCL OPERATION LSORT COMPARISONFAILED",
        ),
    ]);
    // A `%s` precision only truncates, so any that parses is taken.
    yields(&[
        ("format %.2147483648s abc", "abc"),
        ("format %.*s 2147483648 abc", "abc"),
    ]);
}

/// `TclIncrObj` reads the value as a number first, so a non-integer number is
/// refused as the double it then is; `dict incr` on a new key only checks the
/// increment is an integer.
#[test]
fn incr_operands_follow_tcl_incr_obj() {
    fails(&[
        (
            "set x 1; incr x abc",
            "expected integer but got \"abc\"",
            "TCL VALUE NUMBER",
        ),
        (
            "set x 1; incr x 1.5",
            "expected integer but got \"1.5\"",
            "TCL VALUE INTEGER",
        ),
        (
            "set x 1.5; incr x",
            "expected integer but got \"1.5\"",
            "TCL VALUE INTEGER",
        ),
        (
            "set x abc; incr x",
            "expected integer but got \"abc\"",
            "TCL VALUE NUMBER",
        ),
        (
            "dict incr d k 1.5",
            "expected integer but got \"1.5\"",
            "TCL VALUE NUMBER",
        ),
        (
            "set d {k 1}; dict incr d k 1.5",
            "expected integer but got \"1.5\"",
            "TCL VALUE INTEGER",
        ),
        (
            "set d {k 1.5}; dict incr d k",
            "expected integer but got \"1.5\"",
            "TCL VALUE INTEGER",
        ),
        (
            "set d {k 1}; dict incr d k abc",
            "expected integer but got \"abc\"",
            "TCL VALUE NUMBER",
        ),
    ]);
    yields(&[
        (
            "set x 1; incr x 99999999999999999999",
            "100000000000000000000",
        ),
        (
            "set d {k 99999999999999999999}; dict incr d k",
            "k 100000000000000000000",
        ),
    ]);
}

/// `uplevel`/`upvar` resolve their level through `TclObjGetFrame`: an `int`
/// read, `#N` absolute, an integer too large for the read a bad level, and any
/// other word no level at all (`upvar` then reports the default `1`).
#[test]
fn level_words_resolve_as_tcl_obj_get_frame_does() {
    fails(&[
        (
            "uplevel 99999999999999999999 {set a 1}",
            "bad level \"99999999999999999999\"",
            "TCL LOOKUP LEVEL 99999999999999999999",
        ),
        (
            "uplevel -1 {set a 1}",
            "bad level \"-1\"",
            "TCL LOOKUP LEVEL -1",
        ),
        (
            "uplevel 0x10 {set a 1}",
            "bad level \"0x10\"",
            "TCL LOOKUP LEVEL 0x10",
        ),
        (
            "uplevel { 5 } {set a 1}",
            "bad level \" 5 \"",
            "TCL LOOKUP LEVEL { 5 }",
        ),
        (
            "proc p {} {uplevel 4294967295 {set x 3}}; p",
            "bad level \"4294967295\"",
            "TCL LOOKUP LEVEL 4294967295",
        ),
        (
            "proc p {} {uplevel #-1 {set x 3}}; p",
            "bad level \"#-1\"",
            "TCL LOOKUP LEVEL #-1",
        ),
        ("upvar 5 ::x y", "bad level \"5\"", "TCL LOOKUP LEVEL 5"),
        ("upvar abc ::x y", "bad level \"1\"", "TCL LOOKUP LEVEL 1"),
        ("upvar ::x y", "bad level \"1\"", "TCL LOOKUP LEVEL 1"),
        ("upvar #1 x y", "bad level \"#1\"", "TCL LOOKUP LEVEL #1"),
        (
            "proc p {} {upvar 1.0 x y}; p",
            "bad level \"1.0\"",
            "TCL LOOKUP LEVEL 1.0",
        ),
        (
            "uplevel abc {set a 1}",
            "bad level \"1\"",
            "TCL LOOKUP LEVEL 1",
        ),
        (
            "info level 0x10",
            "bad level \"0x10\"",
            "TCL LOOKUP STACK_LEVEL 0x10",
        ),
        (
            "info level -1",
            "bad level \"-1\"",
            "TCL LOOKUP STACK_LEVEL -1",
        ),
        (
            "info frame 0x10",
            "bad level \"0x10\"",
            "TCL LOOKUP LEVEL 0x10",
        ),
    ]);
    yields(&[
        ("upvar #0 ::x y", ""),
        ("upvar #-0 x y", ""),
        ("proc p {} {upvar 0x1 x y; set y 1}; set x 0; p; set x", "1"),
        ("proc p {} {uplevel { 1 } {set x 2}}; p; set x", "2"),
        ("proc p {} {uplevel +1 {set x 4}}; p; set x", "4"),
    ]);
    // Not a level, so the command one frame up.
    let (_, result, _) = run("proc p {} {uplevel abc {set a 1}}; p");
    assert_eq!(result, "invalid command name \"abc\"");
}

/// `return -level` is a non-negative C `int`, `-code` a completion code; both
/// are merged last-wins across the flags and `-options`, then validated.
#[test]
fn return_level_and_code_words_are_validated_once_merged() {
    for word in [
        "99999999999999999999",
        "9223372036854775808",
        "4294967296",
        "4294967295",
        "2147483648",
        "abc",
        "1.5",
        "-1",
    ] {
        fails(&[(
            &format!("return -level {word} x"),
            &format!("bad -level value: expected non-negative integer but got \"{word}\""),
            "TCL RESULT ILLEGAL_LEVEL",
        )]);
    }
    for word in ["99999999999999999999", "4294967296", "abc", "1.5"] {
        fails(&[(
            &format!("return -code {word} x"),
            &format!(
                "bad completion code \"{word}\": must be ok, error, return, break, continue, or an integer"
            ),
            "TCL RESULT ILLEGAL_CODE",
        )]);
    }
    fails(&[
        (
            "return -options {-level abc} x",
            "bad -level value: expected non-negative integer but got \"abc\"",
            "TCL RESULT ILLEGAL_LEVEL",
        ),
        (
            "return -options {-code abc} x",
            "bad completion code \"abc\": must be ok, error, return, break, continue, or an integer",
            "TCL RESULT ILLEGAL_CODE",
        ),
    ]);
    yields(&[
        ("list [catch {return -level 0x1 x} m] $m", "2 x"),
        ("list [catch {return -code bad -code ok x} m] $m", "2 x"),
        ("list [catch {return -level abc -level 0 x} m] $m", "0 x"),
        ("list [catch {return -level 0 -code 0x3 x} m] $m", "3 x"),
    ]);
}

/// tclsh 8.6.18/9.0.4: `interp recursionlimit` reads its limit wide (8.6
/// reads a C `int`, so it refuses `2147483648` as not positive), answers a
/// new limit with the word it was given, and keeps its codes in both forms.
#[test]
fn recursion_limits_read_wide_with_c_codes() {
    overflows(&[
        "interp recursionlimit {} 99999999999999999999",
        "interp recursionlimit {} 9223372036854775808",
        "interp create k; k recursionlimit 99999999999999999999",
    ]);
    fails(&[
        (
            "interp recursionlimit {} 0",
            "recursion limit must be > 0",
            "TCL OPERATION INTERP BADLIMIT",
        ),
        (
            "interp create k; interp recursionlimit k -1",
            "recursion limit must be > 0",
            "TCL OPERATION INTERP BADLIMIT",
        ),
        (
            "interp create k; k recursionlimit abc",
            "expected integer but got \"abc\"",
            "TCL VALUE NUMBER",
        ),
    ]);
    yields(&[
        ("interp recursionlimit {} 0x10", "0x10"),
        ("interp recursionlimit {} { 5 }", " 5 "),
        ("interp recursionlimit {} +5", "+5"),
        (
            "interp recursionlimit {} 0x10; interp recursionlimit {}",
            "16",
        ),
        ("interp recursionlimit {} 2147483648", "2147483648"),
        ("interp create k; k recursionlimit 0x20", "0x20"),
        (
            "interp create k; interp recursionlimit k 0x20; k recursionlimit",
            "32",
        ),
    ]);
}

/// `interp limit`: `-granularity` is a C `int`, `-value`, `-seconds` and
/// `-milliseconds` are wide, and a value out of range is `BADVALUE`.
#[test]
fn interp_limit_values_read_through_the_typed_owner() {
    overflows(&[
        "interp create c; interp limit c commands -value 99999999999999999999",
        "interp create c; interp limit c time -seconds 99999999999999999999",
        "interp create c; interp limit c time -milliseconds 9223372036854775808",
        "interp create c; interp limit c commands -granularity 99999999999999999999",
    ]);
    fails(&[
        (
            "interp create c; interp limit c commands -value abc",
            "expected integer but got \"abc\"",
            "TCL VALUE NUMBER",
        ),
        (
            "interp create c; interp limit c commands -value -1",
            "command limit value must be at least 0",
            "TCL OPERATION INTERP BADVALUE",
        ),
        (
            "interp create c; interp limit c time -seconds -1",
            "seconds must be non-negative",
            "TCL OPERATION INTERP BADVALUE",
        ),
        (
            "interp create c; interp limit c time -milliseconds -1",
            "milliseconds must be non-negative",
            "TCL OPERATION INTERP BADVALUE",
        ),
        (
            "interp create c; interp limit c time -granularity 0",
            "granularity must be at least 1",
            "TCL OPERATION INTERP BADVALUE",
        ),
    ]);
    yields(&[
        (
            "interp create c; interp limit c commands -value 0x10; interp limit c commands -value",
            "16",
        ),
        (
            "interp create c; interp limit c time -seconds 4294967296",
            "",
        ),
    ]);
}

/// `after` reads its delay wide and, failing that, reports the word as a bad
/// subcommand under `TCL LOOKUP INDEX argument`.
#[test]
fn after_delays_read_wide() {
    for word in ["99999999999999999999", "9223372036854775808", "abc", "1.5"] {
        fails(&[(
            &format!("after {word} {{}}"),
            &format!("bad argument \"{word}\": must be cancel, idle, info, or an integer"),
            &format!("TCL LOOKUP INDEX argument {word}"),
        )]);
    }
    for script in ["after 0x10 {}", "after -1 {}", "after 4294967296 {}"] {
        let (code, result, _) = run(&format!(
            "{script}; foreach id [after info] {{after cancel $id}}; set r ok"
        ));
        assert_eq!((code, result.as_str()), (Code::Ok, "ok"), "{script}");
    }
}

/// `read` counts and `seek` offsets are wide reads, taken after the channel
/// is found.
#[test]
fn channel_counts_and_offsets_read_wide() {
    let probe = |body: &str| {
        format!(
            "set f [open /dev/null r]; set r [catch {{{body}}} m o]; close $f; \
             list $r $m [expr {{$r ? [dict get $o -errorcode] : {{}}}}]"
        )
    };
    yields(&[
        (
            &probe("read $f abc"),
            "1 {expected non-negative integer but got \"abc\"} {TCL VALUE NUMBER}",
        ),
        (
            &probe("read $f -1"),
            "1 {expected non-negative integer but got \"-1\"} {TCL VALUE NUMBER}",
        ),
        (&probe("read $f 0x10"), "0 {} {}"),
        (
            &probe("seek $f abc"),
            "1 {expected integer but got \"abc\"} {TCL VALUE NUMBER}",
        ),
        (
            &probe("seek $f 99999999999999999999"),
            "1 {integer value too large to represent} {ARITH IOVERFLOW {integer value too large to represent}}",
        ),
    ]);
    let (code, result, _) = run("read nosuchchan abc");
    assert_eq!(
        (code, result.as_str()),
        (Code::Error, "can not find channel named \"nosuchchan\"")
    );
}

/// `file atime`/`mtime` read a new time wide.
#[test]
fn file_times_read_wide() {
    fails(&[(
        "file mtime /tmp abc",
        "expected integer but got \"abc\"",
        "TCL VALUE NUMBER",
    )]);
    overflows(&["file atime /tmp 99999999999999999999"]);
}

/// A failed `TclGetIntForIndex` read is `bad index` with `TCL VALUE INDEX`
/// (from 8.6); 9.0 takes a beyond-wide index as an index past the end.
#[test]
fn index_reads_carry_tcl_value_index() {
    for (script, word) in [
        ("string index abc abc", "abc"),
        ("string first a abc abc", "abc"),
        ("string range abc 1.5 2", "1.5"),
        ("lindex {a b} 1.5", "1.5"),
        ("lrange {a b} abc 1", "abc"),
        ("lsearch -start abc {a b} a", "abc"),
        ("lsort -index abc {}", "abc"),
        ("lsearch -index 1.5 {{a b}} a", "1.5"),
    ] {
        fails(&[(
            script,
            &format!("bad index \"{word}\": must be integer?[+-]integer? or end?[+-]integer?"),
            "TCL VALUE INDEX",
        )]);
    }
    yields(&[
        ("lindex {a b} 99999999999999999999", ""),
        ("string first a abc 99999999999999999999", "-1"),
    ]);
}

/// `lsort`/`lsearch -index` encode each path element at option parse
/// (`TclIndexEncode`): outside `0..=INT_MAX`, `end+N`, or `end-N` past
/// `INT_MAX` is `out of range`; a step past its sublist names the decoded
/// index (`SelectObjFromSublists`). tclsh 9.0.4/9.1.0; 8.6.18 refuses these
/// under other words (`cannot select an element from any list`).
#[test]
fn index_paths_encode_as_tcl_index_encode() {
    for spec in [
        "99999999999999999999",
        "9223372036854775808",
        "2147483648",
        "-1",
        "end+1",
        "end-99999999999",
        "5-10",
    ] {
        fails(&[(
            &format!("lsort -index {spec} {{}}"),
            &format!("index \"{spec}\" out of range"),
            "TCL VALUE INDEX OUTOFRANGE",
        )]);
    }
    fails(&[
        (
            "lsearch -index 2147483648 {} a",
            "index \"2147483648\" out of range",
            "TCL VALUE INDEX OUTOFRANGE",
        ),
        (
            "lsort -index {0 -1} {{{a b} c}}",
            "index \"-1\" out of range",
            "TCL VALUE INDEX OUTOFRANGE",
        ),
    ]);
    for (script, named) in [
        ("lsort -index 0x10 {{a b}}", "16"),
        ("lsort -index { 5 } {{a b}}", "5"),
        ("lsort -index 1+1 {{a b}}", "2"),
        ("lsort -index 2147483647 {{a b}}", "2147483647"),
        ("lsort -index end-5 {{a b}}", "end-5"),
        ("lsort -index {0 0x10} {{{a b} c}}", "16"),
        ("lsearch -index 2 {{a b}} a", "2"),
        ("lsearch -index end-5 {{a b}} a", "end-5"),
    ] {
        let sublist = "a b";
        fails(&[(
            script,
            &format!("element {named} missing from sublist \"{sublist}\""),
            "TCL OPERATION LSORT INDEXFAILED",
        )]);
    }
    fails(&[
        (
            "lsort -index end {{}}",
            "element end-0 missing from sublist \"\"",
            "TCL OPERATION LSORT INDEXFAILED",
        ),
        (
            "lsort -stride 2 -index 5 {a b c d}",
            "when used with \"-stride\", the leading \"-index\" value must be within the group",
            "TCL OPERATION LSORT BADINDEX",
        ),
        (
            "lsearch -stride 2 -index 5 {a b c d} a",
            "when used with \"-stride\", the leading \"-index\" value must be within the group",
            "TCL OPERATION LSEARCH BADINDEX",
        ),
    ]);
    yields(&[
        ("lsort -index 0x10 {}", ""),
        ("lsort -stride 2 -index 1 {a b c d}", "a b c d"),
    ]);
}

/// The namespace-ensemble option parser keeps each option's own code, for
/// `create` and `configure` alike (tclsh 8.6.18/9.0.4/9.1.0).
#[test]
fn ensemble_options_keep_their_error_codes() {
    let create = "namespace ensemble create -command ::q";
    let configure =
        "namespace ensemble create -command ::q -map {}; namespace ensemble configure ::q";
    for prefix in [create, configure] {
        fails(&[
            (
                &format!("{prefix} -prefixes {{yes }}"),
                "expected boolean value but got \"yes \"",
                "TCL VALUE NUMBER",
            ),
            (
                &format!("{prefix} -map {{a}}"),
                "missing value to go with key",
                "TCL VALUE DICTIONARY",
            ),
            (
                &format!("{prefix} -subcommands \"\\{{\""),
                "unmatched open brace in list",
                "TCL VALUE LIST BRACE",
            ),
            (
                &format!("{prefix} -unknown \"\\{{\""),
                "unmatched open brace in list",
                "TCL VALUE LIST BRACE",
            ),
            (
                &format!("{prefix} -parameters \"\\{{\""),
                "unmatched open brace in list",
                "TCL VALUE LIST BRACE",
            ),
        ]);
    }
    fails(&[
        (
            "namespace ensemble create -command ::q -map {} -prefixes abc",
            "expected boolean value but got \"abc\"",
            "TCL VALUE NUMBER",
        ),
        (
            "namespace ensemble create -command ::q -map \"a \\{\"",
            "unmatched open brace in dict",
            "TCL VALUE DICTIONARY BRACE",
        ),
        (
            "namespace ensemble create -command ::q -map {a {b}c}",
            "dict element in braces followed by \"c\" instead of space",
            "TCL VALUE DICTIONARY JUNK",
        ),
        (
            "namespace ensemble create -command ::q -subcommands {{a}b}",
            "list element in braces followed by \"b\" instead of space",
            "TCL VALUE LIST JUNK",
        ),
        (
            "namespace ensemble create -command ::q -foo x",
            "bad option \"-foo\": must be -command, -map, -parameters, -prefixes, -subcommands, or -unknown",
            "TCL LOOKUP INDEX option -foo",
        ),
        (
            "namespace ensemble create -command ::q -p x",
            "ambiguous option \"-p\": must be -command, -map, -parameters, -prefixes, -subcommands, or -unknown",
            "TCL LOOKUP INDEX option -p",
        ),
        (
            "namespace ensemble create -command ::q -map",
            "wrong # args: should be \"namespace ensemble create ?option value ...?\"",
            "TCL WRONGARGS",
        ),
        (
            &format!("{configure} -foo x"),
            "bad option \"-foo\": must be -map, -namespace, -parameters, -prefixes, -subcommands, or -unknown",
            "TCL LOOKUP INDEX option -foo",
        ),
        (
            &format!("{configure} -foo"),
            "bad option \"-foo\": must be -map, -namespace, -parameters, -prefixes, -subcommands, or -unknown",
            "TCL LOOKUP INDEX option -foo",
        ),
        (
            &format!("{configure} -namespace x"),
            "option -namespace is read-only",
            "TCL ENSEMBLE READ_ONLY",
        ),
    ]);
    yields(&[(
        "namespace ensemble create -command ::q -map {} -prefixes 2",
        "::q",
    )]);
}

/// `clock format`/`add`/`scan` read their clock value and `-base` as C 9.0's
/// `ClockParseFmtScnArgs` does: a wide integer within `±0x00F0000000000000`
/// or exactly `now`/`-now`, every refusal carrying `CLOCK badOption <word>`.
/// tclsh 8.6.18's script `clock` differs throughout (`expected integer`,
/// no `now`, `CLOCK badUnit`); 9.1.0 agrees with 9.0.4.
#[test]
fn clock_values_read_as_clock_parse_fmt_scn_args_does() {
    let too_large = "integer value too large to represent";
    fails(&[
        (
            "clock format 99999999999999999999 -gmt 1",
            too_large,
            "CLOCK badOption 99999999999999999999",
        ),
        (
            "clock format 67553994410557441 -gmt 1",
            too_large,
            "CLOCK badOption 67553994410557441",
        ),
        (
            "clock format abc -gmt 1",
            "bad seconds \"abc\": must be now or integer",
            "CLOCK badOption abc",
        ),
        (
            "clock format [expr {1.5}] -gmt 1",
            "bad seconds \"1.5\": must be now or integer",
            "CLOCK badOption 1.5",
        ),
        (
            "clock format nOw -gmt 1",
            "bad seconds \"nOw\": must be now or integer",
            "CLOCK badOption nOw",
        ),
        (
            "clock format {} -gmt 1",
            "bad seconds \"\": must be now or integer",
            "CLOCK badOption {}",
        ),
        (
            "clock add abc 1 seconds -gmt 1",
            "bad seconds \"abc\": must be now or integer",
            "CLOCK badOption abc",
        ),
        (
            "clock scan 1970-01-01 -format %Y-%m-%d -base -67553994410557441 -gmt 1",
            too_large,
            "CLOCK badOption -67553994410557441",
        ),
        (
            "clock scan 1970 -format %Y -base 1.5 -gmt 1",
            "bad seconds \"1.5\": must be now or integer",
            "CLOCK badOption 1.5",
        ),
    ]);
    yields(&[
        ("clock format 0x10 -gmt 1 -format %s", "16"),
        ("clock format { 5 } -gmt 1 -format %s", "5"),
        (
            "clock format 67553994410557440 -gmt 1 -format %s",
            "67553994410557440",
        ),
        ("clock format 4294967296 -gmt 1 -format %Y", "2106"),
        ("string is integer [clock format now -format %s]", "1"),
        ("string is integer [clock format -now -format %s]", "1"),
        (
            "clock scan 1970-01-01 -format %Y-%m-%d -gmt 1 -base -now",
            "0",
        ),
        (
            "clock scan 1970-01-01 -format %Y-%m-%d -base 0x10 -gmt 1",
            "0",
        ),
    ]);
}

/// `clock add` passes over a word that is no wide integer as an option, so a
/// bad count is a bad option; a count past C's bound (a C `int` for the date
/// units, `0x00F0000000000000` for the time units) overflows with no code.
#[test]
fn clock_add_counts_and_units_follow_clock_add_obj_cmd() {
    let units = "must be years, months, week, weeks, days, weekdays, hours, minutes, or seconds";
    let too_large = "integer value too large to represent";
    fails(&[
        (
            "clock add 0 99999999999999999999 seconds -gmt 1",
            "bad option \"99999999999999999999\": must be -gmt, -locale, or -timezone",
            "CLOCK badOption 99999999999999999999",
        ),
        (
            "clock add 0 1.5 seconds -gmt 1",
            "bad option \"1.5\": must be -gmt, -locale, or -timezone",
            "CLOCK badOption 1.5",
        ),
        (
            "clock add 0 5 bogus -gmt 1",
            &format!("bad unit \"bogus\": {units}"),
            "TCL LOOKUP INDEX unit bogus",
        ),
        (
            "clock add 0 1 w -gmt 1",
            &format!("ambiguous unit \"w\": {units}"),
            "TCL LOOKUP INDEX unit w",
        ),
        (
            "clock add 0 1 secs -gmt 1",
            &format!("bad unit \"secs\": {units}"),
            "TCL LOOKUP INDEX unit secs",
        ),
        (
            "clock add 0 1 -gmt",
            &format!("bad unit \"-gmt\": {units}"),
            "TCL LOOKUP INDEX unit -gmt",
        ),
        ("clock add 0 2147483648 days -gmt 1", too_large, "NONE"),
        ("clock add 0 -2147483648 days -gmt 1", too_large, "NONE"),
        ("clock add 0 67553994410557441 seconds -gmt 1", too_large, "NONE"),
        ("clock add 0 99999999999 years -gmt 1", too_large, "NONE"),
        (
            "clock add 0 5 -gmt 1",
            "wrong # args: should be \"clock add clockval|now ?number units?...?-gmt boolean? ?-locale LOCALE? ?-timezone ZONE?\"",
            "CLOCK wrongNumArgs",
        ),
        (
            "clock add 0 1 seconds -format %Y -gmt 1",
            "bad option \"-format\": must be -gmt, -locale, or -timezone",
            "CLOCK badOption -format",
        ),
        (
            "clock add 0 1 seconds -base 5 -gmt 1",
            "bad option \"-base\": must be -gmt, -locale, or -timezone",
            "CLOCK badOption -base",
        ),
    ]);
    yields(&[
        ("clock add 0 0x10 seconds -gmt 1", "16"),
        ("clock add 0 2147483647 days -gmt 1", "185542587100800"),
        (
            "clock add 0 67553994410557440 seconds -gmt 1",
            "67553994410557440",
        ),
        ("clock add 0 1 sec -gmt 1", "1"),
        ("clock add 0 1 week -gmt 1", "604800"),
        ("clock add 0 1 weekdays -gmt 1", "86400"),
        ("clock add 0 5 weekdays -gmt 1", "604800"),
        ("clock add 0 -1 weekdays -gmt 1", "-86400"),
        ("clock add 259200 1 weekdays -gmt 1", "345600"),
        ("clock add 0 -gmt 1 1 hours", "3600"),
        ("clock add 0 1 hours 2 minutes 3 seconds -gmt 1", "3723"),
        ("clock add 0", "0"),
    ]);
}

/// The option parse `clock format`/`scan`/`add` share, and `clock scan`'s
/// input refusals, with C 9.0's codes.
#[test]
fn clock_options_and_scan_refusals_carry_clock_codes() {
    fails(&[
        (
            "clock format 0 -foo 1",
            "bad option \"-foo\": must be -format, -gmt, -locale, or -timezone",
            "CLOCK badOption -foo",
        ),
        (
            "clock format 0 - 1",
            "bad option \"-\": must be -format, -gmt, -locale, or -timezone",
            "CLOCK badOption -",
        ),
        (
            "clock format 0 -gmt 1 -gmt 1",
            "bad option \"-gmt\": doubly present",
            "CLOCK badOption -gmt",
        ),
        (
            "clock format 0 -gmt 1 -validate 1",
            "bad option \"-validate\": must be -format, -gmt, -locale, or -timezone",
            "CLOCK badOption -validate",
        ),
        (
            "clock format 0 -gmt",
            "wrong # args: should be \"clock format clockval|now ?-format string? ?-gmt boolean? ?-locale LOCALE? ?-timezone ZONE?\"",
            "CLOCK wrongNumArgs",
        ),
        (
            "clock format 0 -gmt abc",
            "expected boolean value but got \"abc\"",
            "TCL VALUE NUMBER",
        ),
        (
            "clock format 0 -timezone :UTC -gmt 1",
            "cannot use -gmt and -timezone in same call",
            "CLOCK gmtWithTimezone",
        ),
        (
            "clock scan 1970 -format %Y -foo 1",
            "bad option \"-foo\": must be -base, -format, -gmt, -locale, -timezone or -validate",
            "CLOCK badOption -foo",
        ),
        (
            "clock scan 1970-01-01 -format %Y-%m-%d -gmt 1 -base 0 -base 5",
            "bad option \"-base\": doubly present",
            "CLOCK badOption -base",
        ),
        (
            "clock scan",
            "wrong # args: should be \"clock scan string ?-base seconds? ?-format string? ?-gmt boolean? ?-locale LOCALE? ?-timezone ZONE? ?-validate boolean?\"",
            "CLOCK wrongNumArgs",
        ),
        (
            "clock scan abc -format %Y -gmt 1",
            "input string does not match supplied format",
            "CLOCK badInputString",
        ),
        (
            "clock scan Foo -format %b -gmt 1",
            "input string does not match supplied format",
            "CLOCK badInputString",
        ),
        (
            "clock scan 99999999999999999999 -format %s -gmt 1",
            "integer value too large to represent",
            "CLOCK dateTooLarge",
        ),
        (
            "clock scan 1970-13-01 -format %Y-%m-%d -gmt 1",
            "unable to convert input string: invalid month",
            "CLOCK invInpStr month",
        ),
        (
            "clock scan 1970-02-30 -format %Y-%m-%d -gmt 1",
            "unable to convert input string: invalid day",
            "CLOCK invInpStr day",
        ),
        (
            "clock clicks -foo",
            "bad option \"-foo\": must be -milliseconds or -microseconds",
            "TCL LOOKUP INDEX option -foo",
        ),
        (
            "clock clicks -m",
            "ambiguous option \"-m\": must be -milliseconds or -microseconds",
            "TCL LOOKUP INDEX option -m",
        ),
        (
            "clock clicks a b",
            "wrong # args: should be \"clock clicks ?-switch?\"",
            "TCL WRONGARGS",
        ),
    ]);
    yields(&[
        ("clock format 0 -g 1 -f %Y", "1970"),
        ("clock scan 1970-01-01 -f %Y-%m-%d -g 1", "0"),
        (
            "clock scan 1970-02-30 -format %Y-%m-%d -gmt 1 -validate 0",
            "5184000",
        ),
        (
            "clock scan 1970-13-01 -format %Y-%m-%d -gmt 1 -validate 0",
            "31536000",
        ),
        (
            "clock scan 1970-00-00 -format %Y-%m-%d -gmt 1 -validate 0",
            "-2764800",
        ),
        ("string is integer [clock clicks -mil]", "1"),
    ]);
}

/// An object already holding a double is refused by every integer read as
/// `TCL VALUE INTEGER` before its spelling is reparsed (`tclObj.c`), where a
/// fresh string that is no integer is `TCL VALUE NUMBER`.
#[test]
fn a_held_double_is_tcl_value_integer() {
    for (script, word) in [
        ("string repeat x [expr {1.5}]", "1.5"),
        ("string repeat x [expr {2.0}]", "2.0"),
        ("lrepeat [expr {1.5}] a", "1.5"),
        ("format %c [expr {1.5}]", "1.5"),
        ("format %d [expr {1.5}]", "1.5"),
        ("format %x [expr {2.0}]", "2.0"),
        ("format %*d [expr {2.0}] 1", "2.0"),
        ("set z 2.0; expr {$z+0}; string repeat x $z", "2.0"),
        ("lsort -stride [expr {2.0}] {a b c d}", "2.0"),
        ("string equal -length [expr {2.0}] abc abd", "2.0"),
        ("set v [expr {2.0}]; incr v", "2.0"),
        ("set v 1; incr v [expr {2.0}]", "2.0"),
        ("set d {a 1}; dict incr d a [expr {1.5}]", "1.5"),
        ("set d {a 1}; dict incr d b [expr {2.0}]", "2.0"),
        ("set d [dict create a [expr {2.0}]]; dict incr d a", "2.0"),
    ] {
        fails(&[(
            script,
            &format!("expected integer but got \"{word}\""),
            "TCL VALUE INTEGER",
        )]);
    }
    fails(&[
        (
            "format %d 1.5",
            "expected integer but got \"1.5\"",
            "TCL VALUE NUMBER",
        ),
        (
            "set d {a 1}; dict incr d b abc",
            "expected integer but got \"abc\"",
            "TCL VALUE NUMBER",
        ),
    ]);
}

/// `regexp`/`regsub` refusals carry C 9.0's codes: a bad `-start` is
/// `TCL VALUE INDEX`, a compile failure `REGEXP <REG_*> {detail}` (8.6 says
/// `couldn't compile`), and the option lookups `TCL LOOKUP INDEX option`.
#[test]
fn regexp_refusals_carry_their_codes() {
    let bad_index = |word: &str| {
        format!("bad index \"{word}\": must be integer?[+-]integer? or end?[+-]integer?")
    };
    fails(&[
        (
            "regexp -start abc a abc",
            &bad_index("abc"),
            "TCL VALUE INDEX",
        ),
        (
            "regexp -start 1.5 a abc",
            &bad_index("1.5"),
            "TCL VALUE INDEX",
        ),
        (
            "regsub -start abc a abc b",
            &bad_index("abc"),
            "TCL VALUE INDEX",
        ),
        (
            "regexp -inline a abc x",
            "regexp match variables not allowed when using -inline",
            "TCL OPERATION REGEXP MIX_VAR_INLINE",
        ),
        (
            "regexp {(} abc",
            "cannot compile regular expression pattern: parentheses () not balanced",
            "REGEXP REG_EPAREN {parentheses () not balanced}",
        ),
        (
            "regsub {(} abc x",
            "cannot compile regular expression pattern: parentheses () not balanced",
            "REGEXP REG_EPAREN {parentheses () not balanced}",
        ),
    ]);
    for (script, word) in [
        ("regexp -foo a abc", "-foo"),
        ("regsub -foo a abc b", "-foo"),
    ] {
        let (code, _, error_code) = run(script);
        assert_eq!(code, Code::Error, "{script}");
        assert_eq!(
            error_code,
            format!("TCL LOOKUP INDEX option {word}"),
            "{script}"
        );
    }
    yields(&[
        ("regexp -start 99999999999999999999 a abc", "0"),
        ("regexp -start end+1 a abc", "0"),
        ("regsub -start 0x1 a aac b", "abc"),
    ]);
}

/// `lseq` reads every word as a number (`Tcl_GetNumberFromObj`), never an
/// expression, and its counts and bounds as C 9.0/9.1 read them.
#[test]
fn lseq_words_read_as_numbers_with_c_codes() {
    overflows(&[
        "lseq 99999999999999999999",
        "lseq 0 99999999999999999999",
        "lseq 0 to 5 by 99999999999999999999",
        "lseq 0 count 99999999999999999999",
        "lseq 99999999999999999999 .. 0",
    ]);
    fails(&[
        (
            "lseq abc",
            "expected number but got \"abc\"",
            "TCL VALUE NUMBER",
        ),
        (
            "lseq 1+2 to 10",
            "expected number but got \"1+2\"",
            "TCL VALUE NUMBER",
        ),
        (
            "lseq {1 +}",
            "expected number but got a list",
            "TCL VALUE NUMBER",
        ),
        (
            "lseq 1.5",
            "expected integer but got \"1.5\"",
            "TCL VALUE INTEGER",
        ),
        (
            "lseq 0 count Inf",
            "expected integer but got \"Inf\"",
            "TCL VALUE INTEGER",
        ),
        ("lseq 0 to", "missing \"to\" value.", "NONE"),
        (
            "lseq 1 2 3 4",
            "bad operation \"3\": must be .., to, count, or by",
            "TCL LOOKUP INDEX operation 3",
        ),
        (
            "lseq 9223372036854775807 count 2",
            "invalid arithmetic series parameter values",
            "ARITH DOMAIN {invalid arithmetic series parameter values}",
        ),
        (
            "lseq NaN count 2",
            "domain error: argument not in valid range",
            "ARITH DOMAIN {domain error: argument not in valid range}",
        ),
    ]);
    yields(&[
        ("lseq 0x3", "0 1 2"),
        ("lseq { 3 }", "0 1 2"),
        ("lseq -3", ""),
        ("lseq 0 1.5", "0.0 1.0"),
        ("lseq 0 count 2.0", "0 1"),
        ("lseq 2147483648 count 2", "2147483648 2147483649"),
        ("lseq 0 to 3 by 0", "0"),
        ("lseq 1e3 count 2", "1000.0 1001.0"),
        ("lseq Inf count 2", "Inf Inf"),
    ]);
}

/// A missing key on a `dict get`/`unset`/`with` path is `TCL LOOKUP DICT
/// <key>` (`TclTraceDictPath`, `DictGetCmd`).
#[test]
fn dict_path_misses_are_tcl_lookup_dict() {
    for (script, key) in [
        ("dict get {a 1} b", "b"),
        ("dict get {a {b 1}} a c", "c"),
        ("dict get {a {b 1}} x c", "x"),
        ("set d {a {b 1}}; dict unset d x y", "x"),
        ("set d {a {b 1}}; dict with d x {}", "x"),
        ("set d {a {b 1}}; dict with d a x {}", "x"),
        ("proc p {} {set d {a {b 1}}; dict get $d a c}; p", "c"),
        ("dict get {a 1} {b c}", "b c"),
    ] {
        fails(&[(
            script,
            &format!("key \"{key}\" not known in dictionary"),
            &format!("TCL LOOKUP DICT {}", tcl_syntax::list::join_list([key])),
        )]);
    }
    yields(&[("set d {a {b 1}}; dict unset d a y; set d", "a {b 1}")]);
}

/// `seek`'s origin lookup keeps its code; `after idle` names its scripts.
#[test]
fn seek_origins_and_after_idle_report_as_c_does() {
    let path = std::env::temp_dir().join(format!("tclrt_seek_code_{}.txt", std::process::id()));
    let p = path.display();
    yields(&[(
        &format!(
            "set f [open {p} w]; catch {{seek $f 0 bogus}} m o; close $f; \
             list $m [dict get $o -errorcode]"
        ),
        "{bad origin \"bogus\": must be start, current, or end} {TCL LOOKUP INDEX origin bogus}",
    )]);
    std::fs::remove_file(&path).ok();
    fails(&[(
        "after idle",
        "wrong # args: should be \"after idle script ?script ...?\"",
        "TCL WRONGARGS",
    )]);
}
