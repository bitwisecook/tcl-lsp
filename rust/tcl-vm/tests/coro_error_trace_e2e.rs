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
//! Every expectation is the same script's result under tclsh 8.6.18, 9.0.4
//! and 9.1.0. errorInfo is matched by its frames rather than compared
//! verbatim, since the VM's command spellings differ from C's in ways that
//! do not depend on coroutines.

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::{CompileService, Vm};

const COROUTINE_RELEASES: [TclVersion; 3] = [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1];

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
