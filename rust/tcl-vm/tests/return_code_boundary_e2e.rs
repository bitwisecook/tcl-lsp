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

//! `return -code return` crossing procedure boundaries: once its level is
//! spent the completion is a plain `return` (`-code 0 -level 1`), which the
//! next procedure boundary absorbs, as C's `TclUpdateReturnInfo` resets it.
//!
//! Every expectation is the same script's result under real tclsh 8.4.20,
//! 8.5.19, 8.6.18, 9.0.4 and 9.1.0 for the releases each case names.

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;

const ALL_RELEASES: [TclVersion; 5] = [
    TclVersion::V8_4,
    TclVersion::V8_5,
    TclVersion::V8_6,
    TclVersion::V9_0,
    TclVersion::V9_1,
];
const FROM_8_5: [TclVersion; 4] = [
    TclVersion::V8_5,
    TclVersion::V8_6,
    TclVersion::V9_0,
    TclVersion::V9_1,
];
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

/// `return -code return` makes its procedure return `TCL_RETURN`, a plain
/// `return` one level out: the calling procedure returns there.
#[test]
fn return_code_return_is_absorbed_by_the_next_procedure() {
    assert_releases(
        &ALL_RELEASES,
        r"proc inner {} {return -code return x}
proc outer {} {inner; return after}
list [catch outer m] $m",
        r"0 x",
    );
}

/// `-level 2` carries the return out through two procedure boundaries.
#[test]
fn return_level_two_is_absorbed_two_procedures_out() {
    assert_releases(
        &FROM_8_5,
        r"proc p3 {} {return -code return -level 2 y}
proc o3 {} {p3; return after}
proc oo3 {} {o3; return after2}
list [catch oo3 m] $m",
        r"0 y",
    );
}

/// A substituted command completing with `TCL_RETURN` returns its caller.
#[test]
fn substituted_return_code_return_returns_the_caller() {
    assert_releases(
        &ALL_RELEASES,
        r#"proc p {} {return -code return x}
proc o {} {set v [p]; return "after:$v"}
proc o2 {} {if {[p] eq "x"} {return yes}; return no}
list [o] [o2]"#,
        r"x x",
    );
}

/// The options of that completion are a plain `return`'s.
#[test]
fn return_code_return_options_are_a_plain_return() {
    assert_releases(
        &FROM_8_5,
        r"proc p {} {return -code return x}
proc w {} { list [catch p m o] $m [dict get $o -code] [dict get $o -level] }
w",
        r"2 x 0 1",
    );
}

/// A coroutine body ending in `return -code return` hands the resume a
/// plain return, which returns the procedure that resumed it.
#[test]
fn coroutine_body_return_code_return_reaches_the_resumer_as_return() {
    assert_releases(
        &FROM_8_6,
        r#"proc body {} { yield; return -code return x }
proc drive {} { coroutine c body; set r [c]; return "drive:$r" }
proc outer {} { list [drive] after }
outer"#,
        r"x after",
    );
}
