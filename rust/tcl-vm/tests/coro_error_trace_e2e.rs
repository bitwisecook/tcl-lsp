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

//! A coroutine that finishes with an error hands the trace it built to its
//! resumer: the errorInfo frames and TIP 348 `-errorstack` entries from
//! inside the coroutine, then the resume command's own frame.
//!
//! A `coroinject` command that fails is raised at the suspend point, where
//! the coroutine's own `catch` can take it, rather than tearing the
//! coroutine down.
//!
//! Every expectation is the same script's result under tclsh 8.6.18, 9.0.4
//! and 9.1.0 (9.0.4 and 9.1.0 for `coroinject`). errorInfo is matched by its frames rather than compared
//! verbatim, since the VM's command spellings differ from C's in ways that
//! do not depend on coroutines.

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;

const COROUTINE_RELEASES: [TclVersion; 3] = [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1];
/// `coroinject` arrived in 9.0.
const INJECT_RELEASES: [TclVersion; 2] = [TclVersion::V9_0, TclVersion::V9_1];

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

/// An error raised inside `uplevel` after a resume carries the body,
/// `uplevel` and proc frames out through the resume command.
#[test]
fn error_after_resume_inside_uplevel_keeps_its_trace() {
    assert_releases(
        &COROUTINE_RELEASES,
        r#"proc e4 {} { uplevel 1 { yield; error boom4 } }
coroutine c4 apply {{} { e4 }}
list [catch c4 m o] $m [string match {boom4*("uplevel" body line 1)*(procedure "e4" line 1)*invoked from within*"c4"} [dict get $o -errorinfo]]"#,
        r"1 boom4 1",
    );
}

/// An error raised after a resume carries every frame it unwound inside
/// the coroutine, in errorInfo and in TIP 348's `-errorstack`.
#[test]
fn error_after_resume_keeps_the_coroutine_frames() {
    assert_releases(
        &COROUTINE_RELEASES,
        r#"proc helper2 {} { error boom2 }
proc body2 {} { yield; set x [helper2] }
coroutine c2 body2
list [catch c2 m o] $m [string match {boom2*while executing*"error boom2*(procedure "helper2" line 1)*invoked from within*(procedure "body2" line 1)*invoked from within*"body2"*invoked from within*"c2"} [dict get $o -errorinfo]] [string match {INNER * CALL helper2 CALL body2} [dict get $o -errorstack]]"#,
        r"1 boom2 1 1",
    );
}

/// An error in the coroutine's first run keeps the frames inside it, then
/// the `coroutine` command's own.
#[test]
fn error_in_coroutine_creation_keeps_its_trace() {
    assert_releases(
        &COROUTINE_RELEASES,
        r#"proc boom {} { error early }
list [catch {coroutine c1 boom} m o] $m [string match {early*(procedure "boom" line 1)*invoked from within*"boom"*invoked from within*"coroutine c1 boom"} [dict get $o -errorinfo]] [info commands c1]"#,
        r"1 early 1 {}",
    );
}

/// The resume command logs its own `invoked from within` frame after the
/// coroutine's, then its caller's frames follow.
#[test]
fn resume_command_logs_its_own_frame() {
    assert_releases(
        &COROUTINE_RELEASES,
        r#"proc caller {} { cc }
coroutine cc apply {{} { yield; error boom5 }}
list [catch caller m o] [string match {boom5*error boom5*invoked from within*"cc*(procedure "caller" line 1)*invoked from within*"caller"} [dict get $o -errorinfo]]"#,
        r"1 1",
    );
}

/// An error caught inside the coroutine before it suspends does not leak
/// into the trace of a later error.
#[test]
fn caught_error_inside_the_coroutine_leaves_the_resumer_clean() {
    assert_releases(
        &COROUTINE_RELEASES,
        r#"coroutine cx apply {{} { catch {error inner}; yield ok; error outer }}
list [catch cx m o] $m [string match {outer*error outer*"cx"} [dict get $o -errorinfo]] [string first {"error inner"} [dict get $o -errorinfo]]"#,
        r"1 outer 1 -1",
    );
}

/// A failing injected command completes the parked `yield` with its error,
/// so the coroutine's own `catch` takes it and the coroutine runs on.
#[test]
fn injected_error_is_raised_at_the_yield() {
    assert_releases(
        &INJECT_RELEASES,
        r"proc bad {args} { error injected }
coroutine c apply {{} { set r [catch {yield} m]; yield [list caught $r $m]; return done }}
coroinject c bad
list [c] [info commands c] [c] [info commands c]",
        "{caught 1 injected} c done {}",
    );
}

/// Uncaught, the injected command's frames come first, then the `yield` it
/// completed and the coroutine's own frames, then the resume command.
#[test]
fn uncaught_injected_error_keeps_both_traces() {
    assert_releases(
        &INJECT_RELEASES,
        r#"proc bad {args} { error injected }
proc mid {args} { bad }
coroutine c apply {{} { yield; yield; return done }}
coroinject c mid
list [catch c m o] $m [info commands c] [string match {injected*(procedure "bad" line 1)*invoked from within*(procedure "mid" line 1)*invoked from within*"mid yield {}"*invoked from within*"yield"*invoked from within*"c"} [dict get $o -errorinfo]] [string match {INNER * CALL bad CALL {mid yield {}} CALL *} [dict get $o -errorstack]]"#,
        "1 injected {} 1 1",
    );
}

/// An injected `break` or `continue` is the `yield`'s own completion: the
/// enclosing loop breaks, and outside a loop `continue` is the usual error.
#[test]
fn injected_break_and_continue_complete_at_the_yield() {
    assert_releases(
        &INJECT_RELEASES,
        r"coroutine c apply {{} { while 1 { yield x }; return out }}
coroinject c apply {{args} {return -code break}}
coroutine e apply {{} { yield x; return e-done }}
coroinject e apply {{args} {return -code continue}}
list [catch c m] $m [catch e m2] $m2",
        r#"0 out 1 {invoked "continue" outside of a loop}"#,
    );
}
