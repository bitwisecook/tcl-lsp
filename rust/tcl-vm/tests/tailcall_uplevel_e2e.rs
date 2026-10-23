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

//! `tailcall` in a body `uplevel` redirected to another level is scheduled
//! on that level's procedure frame, as C stores it on `varFramePtr`, and
//! replaces that procedure's result once it completes `ok`.
//!
//! Every expectation is the same script's result under real tclsh 8.6.18,
//! 9.0.4 and 9.1.0.

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;

const FROM_8_6: [TclVersion; 3] = [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1];

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

/// A `tailcall` in a body `uplevel 1` ran is scheduled on the caller: the
/// procedure in between returns empty at once, the caller finishes its
/// body, and the tailcalled command then runs in its place at its level.
#[test]
fn tailcall_under_uplevel_replaces_the_target_procedure() {
    assert_releases(
        &FROM_8_6,
        r#"proc tgt {args} { lappend ::log [list tgt $args [info level]]; return tgt-ran }
proc tc {} { set r [uplevel 1 { tailcall tgt a b }]; lappend ::log tc-after; return tc-ret }
proc otc {} { set r [tc]; lappend ::log [list otc-after r=$r]; return "otc:$r" }
set ::log {}
list [otc] $::log"#,
        r"tgt-ran {{otc-after r=} {tgt {a b} 1}}",
    );
}

/// Through `uplevel 2` the procedure two levels up is the one replaced.
#[test]
fn tailcall_under_uplevel_2_runs_after_the_target_completes() {
    assert_releases(
        &FROM_8_6,
        r"proc tgt {args} { lappend ::log [list tgt $args [info level]]; return tgt-ran }
proc t2 {} { uplevel 2 { tailcall tgt deep }; lappend ::log t2-after; return t2 }
proc m2 {} { t2; lappend ::log m2-after; return m2 }
proc o2 {} { m2; lappend ::log o2-after; return o2 }
set ::log {}
list [o2] $::log",
        r"tgt-ran {m2-after o2-after {tgt deep 1}}",
    );
}

/// The global frame is no procedure, so it cannot be tailcalled.
#[test]
fn tailcall_under_uplevel_global_is_rejected() {
    assert_releases(
        &FROM_8_6,
        r"proc tgt {} { return tgt }
proc g0 {} { uplevel #0 { tailcall tgt } }
list [catch g0 m] $m",
        r"1 {tailcall can only be called from a proc, lambda or method}",
    );
}

/// The tailcalled command resolves in the target frame's namespace.
#[test]
fn tailcall_under_uplevel_resolves_in_the_target_namespace() {
    assert_releases(
        &FROM_8_6,
        r#"proc tgt {args} { return global-tgt }
namespace eval ns {
    proc tgt {args} { return "ns-tgt:$args" }
    proc target {} { ::helper; return target-body }
}
proc helper {} { uplevel 1 { tailcall tgt x } }
ns::target"#,
        r"ns-tgt:x",
    );
}

/// A `tailcall` with no command cancels the one already scheduled.
#[test]
fn argument_less_tailcall_cancels_a_scheduled_one() {
    assert_releases(
        &FROM_8_6,
        r"proc tgt {args} { return tgt }
proc clr {} { uplevel 1 { tailcall tgt one } }
proc clr2 {} { uplevel 1 { tailcall } }
proc oclr {} { clr; clr2; return oclr-body }
oclr",
        r"oclr-body",
    );
}

/// A target that ends in error or `break` never runs its scheduled tailcall.
#[test]
fn scheduled_tailcall_is_dropped_when_the_target_fails() {
    assert_releases(
        &FROM_8_6,
        r"proc tgt {args} { return tgt }
proc te {} { uplevel 1 { tailcall tgt e } }
proc ote {} { te; error boom }
proc tb {} { uplevel 1 { tailcall tgt b } }
proc otb {} { foreach i {1} { tb; return -code break } ; return otb }
list [catch ote m] $m [catch otb m2] $m2",
        r"1 boom 3 {}",
    );
}

/// The scheduled tailcall survives the target suspending in a coroutine.
#[test]
fn scheduled_tailcall_survives_a_coroutine_suspension() {
    assert_releases(
        &FROM_8_6,
        r#"proc tgt {args} { return "tgt:$args" }
proc ty {} { uplevel 1 { tailcall tgt y } }
proc oty {} { ty; yield mid; return oty-body }
list [coroutine cty oty] [cty]"#,
        r"mid tgt:y",
    );
}
