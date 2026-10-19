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

//! `uplevel` runs its body on the explicit stack at any level: the frames
//! above the target travel on the body's activation until it completes, so
//! a `yield` inside `uplevel N` suspends the coroutine instead of failing
//! with `cannot yield: C stack busy`.
//!
//! Every expectation is the same script's result under real tclsh: 8.6.18,
//! 9.0.4 and 9.1.0 for the coroutine cases, and 8.4.20 through 9.1.0 for
//! the plain `uplevel` cases, which every release agrees on.

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::{CompileService, Vm};

const COROUTINE_RELEASES: [TclVersion; 3] = [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1];
const ALL_RELEASES: [TclVersion; 5] = [
    TclVersion::V8_4,
    TclVersion::V8_5,
    TclVersion::V8_6,
    TclVersion::V9_0,
    TclVersion::V9_1,
];

/// The result of `src` under the `version` pin; panics if it errors.
fn result_for(version: TclVersion, src: &str) -> String {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
        .analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let asm = service
        .compile_for_profile(src, profile)
        .expect("test script compiles for its selected profile");
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(service));
    vm.set_runtime_version(version);
    let completion = vm.run_module(&asm);
    assert!(
        completion.code.is_ok(),
        "{version:?}: script errored: {}",
        completion.result.to_str()
    );
    completion.result.to_str().to_string()
}

fn assert_releases(releases: &[TclVersion], src: &str, expected: &str) {
    for &version in releases {
        assert_eq!(result_for(version, src), expected, "{version:?}");
    }
}

/// A `yield` inside `uplevel 1` suspends the coroutine; the resume value
/// lands in the caller's frame, whose local the body then returns.
#[test]
fn yield_inside_uplevel_1() {
    assert_releases(
        &COROUTINE_RELEASES,
        r"proc inner {} { uplevel 1 { set x [yield a]; lappend log $x } }
proc outer {} { set log start; inner; return $log }
list [coroutine c1 outer] [c1 resumed] [info commands c1]",
        r"a {start resumed} {}",
    );
}

/// `uplevel #0` from a coroutine body: the global write before the `yield`
/// is visible while it is parked, and the body is back at level 1 after.
#[test]
fn yield_inside_uplevel_global() {
    assert_releases(
        &COROUTINE_RELEASES,
        r"set g 0
set made [coroutine c2 apply {{} {
    set r [uplevel #0 { incr g; yield mid; incr g }]
    list r $r [info level]
}}]
list $made $g [c2] $g",
        r"mid 1 {r 2 1} 2",
    );
}

/// `uplevel 2` resolves `k` in the frame two levels up, before and after
/// the suspension, and `info level` reports that frame.
#[test]
fn uplevel_2_keeps_target_frame_across_resume() {
    assert_releases(
        &COROUTINE_RELEASES,
        r"proc a6 {} { set k a6; b6 }
proc b6 {} { set k b6; c6 }
proc c6 {} { uplevel 2 { set q [yield $k]; list $k $q [info level] } }
list [coroutine co6 a6] [co6 QQ]",
        r"a6 {a6 QQ 1}",
    );
}

/// The body runs in the target frame's namespace on both sides of the
/// `yield`.
#[test]
fn uplevel_runs_in_target_namespace_across_resume() {
    assert_releases(
        &COROUTINE_RELEASES,
        r"namespace eval ns11 { proc p {} { ::q11 } }
proc q11 {} { uplevel 1 { list [namespace current] [yield] [info level] } }
coroutine c11 ::ns11::p
c11 r11",
        r"::ns11 r11 1",
    );
}

/// An `uplevel #0` nested in an `uplevel 1` suspends; on resume each
/// redirect restores its own frames in turn.
#[test]
fn nested_uplevels_restore_levels() {
    assert_releases(
        &COROUTINE_RELEASES,
        r"proc i13 {} { uplevel 1 { uplevel #0 { set ::g13 [yield s13] }; info level } }
list [coroutine c13 apply {{} { list [i13] [info level] }}] [c13 G] $::g13",
        r"s13 {1 1} G",
    );
}

/// Deleting a coroutine parked inside `uplevel` retires the frames it set
/// aside; later redirects resolve levels as before.
#[test]
fn deleting_a_coroutine_parked_in_uplevel_restores_frames() {
    assert_releases(
        &COROUTINE_RELEASES,
        r"proc d8 {} { set mine 1; uplevel 1 { yield } }
coroutine c8 apply {{} { set caller 1; d8 }}
rename c8 {}
proc probe {} { uplevel 1 { info level } }
proc via {} { probe }
list [info level] [info commands c8] [via]",
        r"0 {} 1",
    );
}

/// TIP 348: the redirect records one `UP` entry, for the command that ran
/// at the target level, not another for the `uplevel` command itself.
#[test]
fn uplevel_error_stack_records_one_up() {
    assert_releases(
        &COROUTINE_RELEASES,
        r"proc e9 {} { uplevel 1 { error boom9 } }
proc f9 {} { e9 }
catch f9 m o
string match {INNER * UP 1 CALL e9 CALL f9} [dict get $o -errorstack]",
        r"1",
    );
}

/// `uplevel 1` reads and writes the caller's locals.
#[test]
fn uplevel_1_reads_caller_locals() {
    assert_releases(
        &ALL_RELEASES,
        r"proc p5 {} { set acc {}; foreach i {1 2 3} { p5i; if {$i == 2} { break } }; return $acc }
proc p5i {} { uplevel 1 {lappend acc $i} }
p5",
        r"1 2",
    );
}

/// A completion code raised inside the body is the `uplevel`'s own.
#[test]
fn uplevel_passes_error_code_out() {
    assert_releases(
        &ALL_RELEASES,
        r"proc p5b {} { set r [catch {uplevel 1 {return -code error xx}} m]; list $r $m }
p5b",
        r"2 xx",
    );
}

/// A `break` passed out by `uplevel` from a proc is not the caller's loop's
/// `break`: the proc boundary turns it into an error.
#[test]
fn uplevel_break_leaves_the_proc() {
    assert_releases(
        &ALL_RELEASES,
        r"proc w {} { set acc {}; foreach i {1 2 3} { p5c }; return $acc }
proc p5c {} { uplevel 1 { lappend acc $i; if {$i == 2} break } }
list [catch w m] $m",
        r#"1 {invoked "break" outside of a loop}"#,
    );
}

/// As with `eval`, the commands ahead of one that does not parse run at the
/// target level before the parse error (#1603).
#[test]
fn uplevel_runs_clean_prefix_before_parse_error() {
    assert_releases(
        &ALL_RELEASES,
        r#"proc t10 {} { catch {uplevel 1 {set pre ran; set x "}} m; return $m }
proc o10 {} { set pre no; set r [t10]; list $pre $r }
o10"#,
        r#"ran {missing "}"#,
    );
}

/// A level beyond the stack is still rejected.
#[test]
fn uplevel_bad_level() {
    assert_releases(
        &ALL_RELEASES,
        r"proc b14 {} { uplevel 5 {set x} }
list [catch b14 m] $m",
        r#"1 {bad level "5"}"#,
    );
}

/// The body runs in the target frame's namespace and level.
#[test]
fn uplevel_target_namespace_and_level() {
    assert_releases(
        &ALL_RELEASES,
        r"namespace eval ns15 { proc p {} { set loc L; ::q15 } }
proc q15 {} { uplevel 1 { list [namespace current] $loc [info level] } }
ns15::p",
        r"::ns15 L 1",
    );
}

/// `uplevel #0` from nested procs runs at level 0.
#[test]
fn uplevel_global_from_nested_procs() {
    assert_releases(
        &ALL_RELEASES,
        r"proc a16 {} { b16 }
proc b16 {} { uplevel #0 { set ::g16 [info level] } }
a16",
        r"0",
    );
}

/// Two `uplevel 1` bodies share the caller's frame.
#[test]
fn uplevel_writes_reach_the_caller() {
    assert_releases(
        &ALL_RELEASES,
        r"proc u17 {} { uplevel 1 {set y 7}; uplevel 1 {incr y} }
proc o17 {} { u17; set y }
o17",
        r"8",
    );
}
