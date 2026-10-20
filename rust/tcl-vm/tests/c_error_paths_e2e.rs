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

//! Error paths as C reports them: `return`'s options and the codes a
//! procedure boundary cannot take, per release; the command and procedure
//! names errorInfo logs, through callbacks, tailcalls and `source`.
//!
//! Every expectation is the result of the same script under real tclsh
//! 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0, grouped where releases agree.

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;

/// The result of `src` under the `version` pin; panics if it errors.
fn result_for(version: TclVersion, src: &str) -> String {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
        .analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::stdout()));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    let completion = vm.try_eval_source(src).expect("original source provider");
    assert!(
        completion.code.is_ok(),
        "{version:?}: script errored: {}",
        completion.result.to_str()
    );
    String::from_utf8_lossy(
        &tcl_syntax::value::ValueOps::native_string_bytes(&mut vm, &completion.result)
            .expect("result string"),
    )
    .into_owned()
}

fn assert_releases(releases: &[TclVersion], src: &str, expected: &str) {
    for &version in releases {
        assert_eq!(result_for(version, src), expected, "{version:?}");
    }
}

/// A directory holding the scripts the `source` case reads.
fn sourced_scripts() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tclvm-c-error-paths-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create the script directory");
    for (name, text) in [
        ("err.tcl", "proc p {} {\n  error deep\n}\n\np\n"),
        ("rerr.tcl", "set a 1\nreturn -code error boom\n"),
        ("rval.tcl", "return val\nset never 1\n"),
        ("rbrk.tcl", "return -code break\n"),
    ] {
        std::fs::write(dir.join(name), text).expect("write a sourced script");
    }
    dir
}

/// `return` reads TIP 90's options dictionary from 8.5; 8.4 takes only
/// `-code`, `-errorinfo` and `-errorcode` and rejects any other name.
#[test]
fn return_options_follow_the_release() {
    let src = r"proc p1 {} {return -level 1 x}
proc p2 {} {return -code error -foo bar x}
proc p3 {} {return -errorcode {A B} -code error y}
list [list [catch p1 m] $m] [list [catch p2 m] $m] [list [catch p3 m] $m $::errorCode]";
    assert_releases(
        &[TclVersion::V8_4],
        src,
        r#"{1 {bad option "-level": must be -code, -errorcode, or -errorinfo}} {1 {bad option "-foo": must be -code, -errorcode, or -errorinfo}} {1 y {A B}}"#,
    );
    assert_releases(
        &[
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r"{0 x} {1 x} {1 y {A B}}",
    );
}

/// 8.5 merges an odd `-options` list's complete pairs; 8.6 rejects it and
/// gives `return`'s option errors their `TCL RESULT` errorCode.
#[test]
fn odd_return_options_merge_in_8_5_and_fail_from_8_6() {
    let src = r"proc p {} {return -options {-code error -level} x}
proc q {} {return -code bogus x}
list [catch p m] $m [catch q m] $m $::errorCode";
    assert_releases(
        &[TclVersion::V8_5],
        src,
        r#"1 x 1 {bad completion code "bogus": must be ok, error, return, break, continue, or an integer} NONE"#,
    );
    assert_releases(
        &[TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1],
        src,
        r#"1 {expected dict but got "-code error -level"} 1 {bad completion code "bogus": must be ok, error, return, break, continue, or an integer} {TCL RESULT ILLEGAL_CODE}"#,
    );
}

/// `invoked "break" outside of a loop` carries `TCL RESULT UNEXPECTED` from
/// 8.6; earlier releases leave `NONE`.
#[test]
fn break_out_of_a_procedure_sets_the_release_error_code() {
    let src = r"proc p {} break
proc q {} continue
list [catch p m] $m $::errorCode [catch q m] $m $::errorCode";
    assert_releases(
        &[TclVersion::V8_4, TclVersion::V8_5],
        src,
        r#"1 {invoked "break" outside of a loop} NONE 1 {invoked "continue" outside of a loop} NONE"#,
    );
    assert_releases(
        &[TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1],
        src,
        r#"1 {invoked "break" outside of a loop} {TCL RESULT UNEXPECTED} 1 {invoked "continue" outside of a loop} {TCL RESULT UNEXPECTED}"#,
    );
}

/// A `return -code error` logs the `return` command where the procedure
/// boundary raises it, and an unknown command logs itself.
#[test]
fn return_code_error_logs_the_return_command() {
    let src = r"proc ei {script} { catch {uplevel 1 $script} m; return $::errorInfo }
proc p {} {return -code error boom}
proc q {} {p}
list [ei q] [ei nosuch]";
    assert_releases(
        &[
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r#"{boom
    while executing
"p"
    (procedure "q" line 1)
    invoked from within
"q"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {invalid command name "nosuch"
    while executing
"nosuch"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"}"#,
    );
}

/// A procedure's errorInfo frame names it by the word that invoked it.
#[test]
fn procedure_frame_names_the_invoking_word() {
    let src = r"proc ei {script} { catch {uplevel 1 $script} m; return $::errorInfo }
proc r {} { error x }
namespace eval n { proc r2 {} { error y } }
proc a {} { ::r }
proc b {} { n::r2 }
namespace eval n { proc d {} { r2 } }
list [ei a] [ei b] [ei n::d]";
    assert_releases(
        &[
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r#"{x
    while executing
"error x "
    (procedure "::r" line 1)
    invoked from within
"::r "
    (procedure "a" line 1)
    invoked from within
"a"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {y
    while executing
"error y "
    (procedure "n::r2" line 1)
    invoked from within
"n::r2 "
    (procedure "b" line 1)
    invoked from within
"b"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {y
    while executing
"error y "
    (procedure "r2" line 1)
    invoked from within
"r2 "
    (procedure "n::d" line 1)
    invoked from within
"n::d"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"}"#,
    );
}

/// 8.4's `catch command ?varName?` has no options variable, compiled or not.
#[test]
fn catch_takes_the_release_arity() {
    let src = r"set c catch
proc p {} {catch {error a} m o; return $m}
list [catch {$c {error a} m o} r] $r [catch p r] $r";
    assert_releases(
        &[TclVersion::V8_4],
        src,
        r#"1 {wrong # args: should be "catch command ?varName?"} 1 {wrong # args: should be "catch command ?varName?"}"#,
    );
    assert_releases(
        &[
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r"0 1 0 a",
    );
}

/// A condition word's command substitution logs only its command.
#[test]
fn condition_substitution_logs_no_empty_command() {
    let src = r"proc ei {script} { catch {uplevel 1 $script} m; return $::errorInfo }
proc e {} {error x}
list [ei {if {[e]} {}}] [ei {while {[e]} {}}]";
    assert_releases(
        &[
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r#"{x
    while executing
"error x"
    (procedure "e" line 1)
    invoked from within
"e"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {x
    while executing
"error x"
    (procedure "e" line 1)
    invoked from within
"e"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"}"#,
    );
}

/// An `lsort -command` comparator's error logs the callback and lsort's
/// `(-compare command)` frame.
#[test]
fn lsort_comparator_error_logs_the_callback() {
    let src = r#"proc ei {script} { catch {uplevel 1 $script} m; return $::errorInfo }
proc cmp {a b} { error "cmp:$a" }
list [ei {lsort -command cmp {b a}}] [ei {lsort -command nosuch {b a}}]"#;
    assert_releases(
        &[
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r#"{cmp:b
    while executing
"error "cmp:$a" "
    (procedure "cmp" line 1)
    invoked from within
"cmp b a"
    (-compare command)
    invoked from within
"lsort -command cmp {b a}"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {invalid command name "nosuch"
    while executing
"nosuch b a"
    (-compare command)
    invoked from within
"lsort -command nosuch {b a}"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"}"#,
    );
}

/// A `regsub -command` prefix's error logs the callback it ran.
#[test]
fn regsub_command_error_logs_the_callback() {
    let src = r#"proc ei {script} { catch {uplevel 1 $script} m; return $::errorInfo }
proc boomp {x} { error "boom $x" }
ei {regsub -command {.x.} abcxdef boomp}"#;
    assert_releases(
        &[TclVersion::V9_0, TclVersion::V9_1],
        src,
        r#"boom cxd
    while executing
"error "boom $x" "
    (procedure "boomp" line 1)
    invoked from within
"boomp cxd"
    (-command substitution computation script)
    invoked from within
"regsub -command {.x.} abcxdef boomp"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script""#,
    );
}

/// A tailcall's replacement logs its own words before the replaced
/// procedure's call, whether it runs at once or as scheduled.
#[test]
fn tailcall_replacement_logs_its_own_words() {
    let src = r#"proc ei {script} { catch {uplevel 1 $script} m; return $::errorInfo }
proc r {a b} { error "r:$a" }
proc q {} { tailcall r "x y" {z} }
proc p {} { tailcall q }
proc s {} { tailcall apply {{v} { error lam$v }} 7 }
proc t {} { catch {tailcall q}; return }
proc c {} { tailcall set }
proc d {} { catch {tailcall set}; return }
list [ei p] [ei s] [ei t] [ei c] [ei d]"#;
    assert_releases(
        &[TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1],
        src,
        r#"{r:x y
    while executing
"error "r:$a" "
    (procedure "r" line 1)
    invoked from within
"r {x y} z"
    invoked from within
"p"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {lam7
    while executing
"error lam$v "
    (lambda term "{v} { error lam$v }" line 1)
    invoked from within
"apply {{v} { error lam$v }} 7"
    invoked from within
"s"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {r:x y
    while executing
"error "r:$a" "
    (procedure "r" line 1)
    invoked from within
"r {x y} z"
    invoked from within
"t"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {wrong # args: should be "set varName ?newValue?"
    while executing
"set"
    invoked from within
"c"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {wrong # args: should be "set varName ?newValue?"
    while executing
"set"
    invoked from within
"d"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"}"#,
    );
}

/// The tailcalled command resolves from the issuing namespace but keeps the
/// words as written, in `info level 0` and errorInfo.
#[test]
fn tailcall_words_stay_as_written() {
    let src = r"proc lv {args} { info level 0 }
namespace eval ns { proc lv {args} { list ns [info level 0] } }
namespace eval ns { proc a {} { tailcall lv 1 2 } }
namespace eval ns { proc b {} { catch {tailcall lv 3}; return no } }
namespace eval ns { proc helper {} { error inner } }
namespace eval ns { proc q {} { catch {tailcall helper}; return x } }
proc helper {} { error outer }
list [ns::a] [ns::b] [catch ns::q m] $m $::errorInfo";
    assert_releases(
        &[TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1],
        src,
        r#"{ns {lv 1 2}} {ns {lv 3}} 1 inner {inner
    while executing
"error inner "
    (procedure "helper" line 1)
    invoked from within
"helper"
    invoked from within
"ns::q"}"#,
    );
}

/// `source` adds `(file "…" line N)` to an error, but an error its `return`
/// settles to names no file.
#[test]
fn sourced_file_frames_and_returns() {
    let dir = sourced_scripts();
    let src = r"set dir __DIR__
proc src {name} { set c [catch {source [file join $::dir $name]} m]; list $name $c $m [string map [list $::dir DIR] $::errorInfo] }
list [src err.tcl] [src rerr.tcl] [src rval.tcl] [src rbrk.tcl]".replace("__DIR__", &format!("{{{}}}", dir.display()));
    assert_releases(
        &[
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        &src,
        r#"{err.tcl 1 deep {deep
    while executing
"error deep"
    (procedure "p" line 2)
    invoked from within
"p"
    (file "DIR/err.tcl" line 5)
    invoked from within
"source [file join $::dir $name]"}} {rerr.tcl 1 boom {boom
    while executing
"source [file join $::dir $name]"}} {rval.tcl 0 val {boom
    while executing
"source [file join $::dir $name]"}} {rbrk.tcl 3 {} {boom
    while executing
"source [file join $::dir $name]"}}"#,
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// A loop the release compiles runs its test and body inline: the failing
/// command is logged, the loop adds no body frame and is not logged itself.
#[test]
fn compiled_loops_log_only_the_failing_command() {
    let src = r"proc e {} {error x}
proc p1 {} {while {[e]} {}}
proc p2 {} {while {[set a 1]} {e}}
proc p3 {} {for {} {[e]} {} {}}
proc p4 {} {set i 0; while {$i < 1} {incr i; e}}
set r {}
foreach p {p1 p2 p3 p4} { catch $p; lappend r $::errorInfo }
set r";
    assert_releases(
        &[
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r#"{x
    while executing
"error x"
    (procedure "e" line 1)
    invoked from within
"e"
    (procedure "p1" line 1)
    invoked from within
"p1"
    invoked from within
"catch $p"} {x
    while executing
"error x"
    (procedure "e" line 1)
    invoked from within
"e"
    (procedure "p2" line 1)
    invoked from within
"p2"
    invoked from within
"catch $p"} {x
    while executing
"error x"
    (procedure "e" line 1)
    invoked from within
"e"
    (procedure "p3" line 1)
    invoked from within
"p3"
    invoked from within
"catch $p"} {x
    while executing
"error x"
    (procedure "e" line 1)
    invoked from within
"e"
    (procedure "p4" line 1)
    invoked from within
"p4"
    invoked from within
"catch $p"}"#,
    );
}

/// A compiled `catch` evaluating a script word logs the `catch` as the error
/// leaves the script; an inline or invoked `catch` logs only the failing
/// command, inside a substitution too.
#[test]
fn catch_logs_its_command_only_when_it_evaluates_a_word() {
    let src = r"proc e {} {error x}
proc p1 {} {set s e; catch $s; set ::errorInfo}
proc p2 {} {catch {e}; set ::errorInfo}
proc p3 {} {set c catch; set s e; $c $s; set ::errorInfo}
proc p4 {} {set c [catch {e} m]; set ::errorInfo}
list [p1] [p2] [p3] [p4]";
    assert_releases(
        &[
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r#"{x
    while executing
"error x"
    (procedure "e" line 1)
    invoked from within
"e"
    invoked from within
"catch $s"} {x
    while executing
"error x"
    (procedure "e" line 1)
    invoked from within
"e"} {x
    while executing
"error x"
    (procedure "e" line 1)
    invoked from within
"e"} {x
    while executing
"error x"
    (procedure "e" line 1)
    invoked from within
"e"}"#,
    );
}

/// A compiled command's word substitution logs the failing command, not the
/// command around it.
#[test]
fn word_substitution_in_compiled_code_logs_once() {
    let src = r#"proc e {} {error x}
proc p1 {} {set y "a[e]b"}
catch p1
set ::errorInfo"#;
    assert_releases(
        &[
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r#"x
    while executing
"error x"
    (procedure "e" line 1)
    invoked from within
"e"
    (procedure "p1" line 1)
    invoked from within
"p1""#,
    );
}

/// A command ending in a quoted word is named through the whitespace after
/// its closing quote.
#[test]
fn quoted_last_word_keeps_trailing_whitespace() {
    let src = r#"proc ei {script} { catch {uplevel 1 $script} m; return $::errorInfo }
proc p1 {} {error "x"   }
proc p2 {} {if 1 {error "y"  }}
list [ei p1] [ei p2]"#;
    assert_releases(
        &[
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r#"{x
    while executing
"error "x"   "
    (procedure "p1" line 1)
    invoked from within
"p1"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {y
    while executing
"error "y"  "
    (procedure "p2" line 1)
    invoked from within
"p2"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"}"#,
    );
}

/// A failing substitution reports the line it is on, not the first textual
/// match or the statement's first line.
#[test]
fn substitution_line_is_its_own() {
    let src = r#"proc ei {script} { catch {uplevel 1 $script} m; return $::errorInfo }
proc h {} {error hh}
proc p1 {} {list {[h]
foo
} [h]}
proc p2 {} {set x "[string length ok]
[h]"}
list [ei p1] [ei p2]"#;
    assert_releases(
        &[
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ],
        src,
        r#"{hh
    while executing
"error hh"
    (procedure "h" line 1)
    invoked from within
"h"
    (procedure "p1" line 3)
    invoked from within
"p1"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {hh
    while executing
"error hh"
    (procedure "h" line 1)
    invoked from within
"h"
    (procedure "p2" line 2)
    invoked from within
"p2"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"}"#,
    );
}

/// Every logged command adds where it ran to `-errorstack` — `UP n` in an
/// `uplevel`ed frame, else the procedure's `CALL` — as C's
/// `TclLogCommandInfo` does; 9.1 names the wide instructions it emits.
#[test]
fn error_stack_records_every_logged_command() {
    let src = r"proc s {script} { catch {uplevel 1 $script} m o; return [dict get $o -errorstack] }
proc q1 {} {foreach i 1 {error x}}
proc q3 {} {eval {nosuch 1}}
proc r2 {} {nosuch 3}
proc q5 {} {set a [r2]}
proc q6 {} {uplevel 1 {nosuch 4}}
proc q7 {} {q6}
proc t1 {} { catch {nosuch 5} m o; dict get $o -errorstack }
list [s q1] [s q3] [s q5] [s q7] [t1]";
    assert_releases(
        &[TclVersion::V8_6, TclVersion::V9_0],
        src,
        r"{INNER {returnImm x {}} CALL q1 UP 1 CALL {s q1}} {INNER {invokeStk1 nosuch 1} CALL q3 CALL q3 UP 1 CALL {s q3}} {INNER {invokeStk1 nosuch 3} CALL r2 CALL q5 UP 1 CALL {s q5}} {INNER {invokeStk1 nosuch 4} UP 1 CALL q6 CALL q7 UP 1 CALL {s q7}} {INNER {invokeStk1 nosuch 5} CALL t1}",
    );
    assert_releases(
        &[TclVersion::V9_1],
        src,
        r"{INNER {returnImm x {}} CALL q1 UP 1 CALL {s q1}} {INNER {invokeStk nosuch 1} CALL q3 CALL q3 UP 1 CALL {s q3}} {INNER {invokeStk nosuch 3} CALL r2 CALL q5 UP 1 CALL {s q5}} {INNER {invokeStk nosuch 4} UP 1 CALL q6 CALL q7 UP 1 CALL {s q7}} {INNER {invokeStk nosuch 5} CALL t1}",
    );
}

/// `INNER` is the failing instruction with the operands it consumed
/// (`TclGetInnerContext`): `returnImm` for a compiled `error`/`throw`,
/// `syntax` for a constant fold, an invoke with its words.
#[test]
fn error_stack_inner_names_the_failing_instruction() {
    let src = r"proc s {script} { catch {uplevel 1 $script} m o; return [dict get $o -errorstack] }
proc a1 {} {error m1 i1}
proc a3 {} {error m3 {} {C D}}
proc a4 {} {throw {X Y} m4}
proc a5 {} {set c error; $c m5}
proc a6 {} {expr {1/0}}
proc a7 {} {set a 1; expr {$a/0}}
proc a8 {} {set s abc; string index $s [list x]}
list [s a1] [s a3] [s a4] [s a5] [s a6] [s a7]";
    assert_releases(
        &[TclVersion::V8_6, TclVersion::V9_0],
        src,
        r"{INNER {invokeStk1 a1} UP 1 CALL {s a1}} {INNER {returnImm m3 {-errorinfo {} -errorcode {C D}}} CALL a3 UP 1 CALL {s a3}} {INNER {returnImm m4 {-errorcode {X Y}}} CALL a4 UP 1 CALL {s a4}} {INNER {invokeStk1 error m5} CALL a5 UP 1 CALL {s a5}} {INNER {syntax {divide by zero} {-code 1 -level 0 -errorcode {ARITH DIVZERO {divide by zero}} -errorinfo {divide by zero} -errorline 1}} CALL a6 UP 1 CALL {s a6}} {INNER {div 1 0} CALL a7 UP 1 CALL {s a7}}",
    );
    assert_releases(
        &[TclVersion::V9_1],
        src,
        r"{INNER {invokeStk a1} UP 1 CALL {s a1}} {INNER {returnImm m3 {-errorinfo {} -errorcode {C D}}} CALL a3 UP 1 CALL {s a3}} {INNER {returnImm m4 {-errorcode {X Y}}} CALL a4 UP 1 CALL {s a4}} {INNER {invokeStk error m5} CALL a5 UP 1 CALL {s a5}} {INNER {syntax {divide by zero} {-code 1 -level 0 -errorcode {ARITH DIVZERO {divide by zero}} -errorinfo {divide by zero} -errorline 1}} CALL a6 UP 1 CALL {s a6}} {INNER {div 1 0} CALL a7 UP 1 CALL {s a7}}",
    );
}

/// From 8.6 an `enter`/`leave` execution trace's error adds its own frame
/// in place of the traced command's; 8.5 logs the command.
#[test]
fn execution_trace_errors_name_the_trace() {
    let src = r"proc foo args {}
proc goo {} foo
trace add execution foo enter {error bar;#}
catch goo m d
set r [list [dict get $d -errorinfo]]
trace remove execution foo enter {error bar;#}
trace add execution foo leave {error baz;#}
catch goo m d
lappend r [dict get $d -errorinfo]";
    assert_releases(
        &[TclVersion::V8_5],
        src,
        r#"{bar
    while executing
"error bar"
    invoked from within
"foo"
    (procedure "goo" line 1)
    invoked from within
"goo"} {baz
    while executing
"error baz"
    invoked from within
"foo"
    (procedure "goo" line 1)
    invoked from within
"goo"}"#,
    );
    assert_releases(
        &[TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1],
        src,
        r#"{bar
    while executing
"error bar"
    (enter trace on "foo")
    (procedure "goo" line 1)
    invoked from within
"goo"} {baz
    while executing
"error baz"
    (leave trace on "foo")
    (procedure "goo" line 1)
    invoked from within
"goo"}"#,
    );
}
