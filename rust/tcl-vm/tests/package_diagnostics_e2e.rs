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

//! `package` diagnostics per release: message and `errorCode`, pinned against
//! real tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 (#2229).
//!
//! Structured package codes (`TCL PACKAGE …`, `TCL VALUE VERSION…`) arrive at
//! 8.6; 8.4 and 8.5 leave `errorCode` at `NONE`, except that 8.5 already
//! reports `TCL LOOKUP PACKAGE name` for `package present`.

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::Vm;

fn run_for_version(src: &str, version: TclVersion) -> String {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
        .analyser_profile();
    let mut vm = Vm::with_native_core(
        Box::new(std::io::stdout()),
        std::rc::Rc::new(tcl_vm::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("native core for the selected profile");
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    let completion = vm.try_eval_source(src).expect("original source provider");
    assert!(
        completion.code.is_ok(),
        "{version:?}: {}",
        completion.result.to_str()
    );
    String::from_utf8_lossy(
        &tcl_syntax::value::ValueOps::native_string_bytes(&mut vm, &completion.result)
            .expect("result string"),
    )
    .into_owned()
}

/// Runs `body` at the global level and answers `{code message errorCode}`.
const PROBE: &str = r"
proc probe {body} {
    global errorCode
    set errorCode SENTINEL
    set code [catch {uplevel #0 $body} msg]
    list $code $msg $errorCode
}
";

fn probe(version: TclVersion, setup: &str, body: &str) -> String {
    let src = format!("{PROBE}\n{setup}\nprobe {{{body}}}");
    run_for_version(&src, version)
}

/// `(setup, body, message, errorCode on 8.6+)`; every case answers `NONE` on
/// 8.4 and 8.5.
const STRUCTURED_FROM_8_6: &[(&str, &str, &str, &str)] = &[
    (
        "package forget foo; package ifneeded foo 1 break",
        "package require foo 1",
        "attempt to provide package foo 1 failed: bad return code: 3",
        "TCL PACKAGE BADRESULT",
    ),
    (
        "package forget foo; package ifneeded foo 1 continue",
        "package require foo 1",
        "attempt to provide package foo 1 failed: bad return code: 4",
        "TCL PACKAGE BADRESULT",
    ),
    (
        "package forget foo; package ifneeded foo 1 return",
        "package require foo 1",
        "attempt to provide package foo 1 failed: bad return code: 2",
        "TCL PACKAGE BADRESULT",
    ),
    (
        "package forget foo; package ifneeded foo 1 {package provide foo 2}",
        "package require foo 1",
        "attempt to provide package foo 1 failed: package foo 2 provided instead",
        "TCL PACKAGE WRONGPROVIDE",
    ),
    (
        "package forget foo; package ifneeded foo 1 {}",
        "package require foo 1",
        "attempt to provide package foo 1 failed: no version of package foo provided",
        "TCL PACKAGE UNPROVIDED",
    ),
    (
        "package forget foo; package ifneeded foo 1 {package require foo 1}",
        "package require foo 1",
        "circular package dependency: attempt to provide foo 1 requires foo 1",
        "TCL PACKAGE CIRCULARITY",
    ),
    (
        "package forget foo; package provide foo 1",
        "package require foo 2",
        "version conflict for package \"foo\": have 1, need 2",
        "TCL PACKAGE VERSIONCONFLICT",
    ),
    (
        "package forget foo; package provide foo 1",
        "package present foo 2",
        "version conflict for package \"foo\": have 1, need 2",
        "TCL PACKAGE VERSIONCONFLICT",
    ),
    (
        "package forget p; package provide p 1.2",
        "package provide p 1.3",
        "conflicting versions provided for package \"p\": 1.2, then 1.3",
        "TCL PACKAGE VERSIONCONFLICT",
    ),
    (
        "package forget nope",
        "package require nope",
        "can't find package nope",
        "TCL PACKAGE UNFOUND",
    ),
    (
        "package forget nope",
        "package require nope 1.2",
        "can't find package nope 1.2",
        "TCL PACKAGE UNFOUND",
    ),
    (
        "",
        "package require foo x.y",
        "expected version number but got \"x.y\"",
        "TCL VALUE VERSION",
    ),
    (
        "",
        "package provide foo 1.x",
        "expected version number but got \"1.x\"",
        "TCL VALUE VERSION",
    ),
    (
        "",
        "package ifneeded foo 1.x {}",
        "expected version number but got \"1.x\"",
        "TCL VALUE VERSION",
    ),
    (
        "",
        "package vcompare a 1",
        "expected version number but got \"a\"",
        "TCL VALUE VERSION",
    ),
    (
        "proc unk args {return -code break}; package forget bar; package unknown unk",
        "package require bar",
        "bad return code: 3",
        "TCL PACKAGE BADRESULT",
    ),
];

#[test]
fn structured_package_error_codes_begin_at_tcl_eight_six() {
    for version in TclVersion::ALL {
        for &(setup, body, message, code) in STRUCTURED_FROM_8_6 {
            let expected_code = if version >= TclVersion::V8_6 {
                code
            } else {
                "NONE"
            };
            let expected = tcl_syntax::list::join_list(["1", message, expected_code]);
            assert_eq!(
                probe(version, setup, body),
                expected,
                "{version:?}: {setup}; {body}"
            );
        }
    }
}

/// A malformed range is a 8.5+ diagnostic of its own (8.4 has no ranges and
/// reports the whole word as a bad version); its code is structured from 8.6.
#[test]
fn version_range_diagnostic_per_release() {
    for version in TclVersion::ALL {
        let expected = match version {
            TclVersion::V8_4 => tcl_syntax::list::join_list([
                "1",
                "expected version number but got \"1-2-3\"",
                "NONE",
            ]),
            TclVersion::V8_5 => tcl_syntax::list::join_list([
                "1",
                "expected versionMin-versionMax but got \"1-2-3\"",
                "NONE",
            ]),
            _ => tcl_syntax::list::join_list([
                "1",
                "expected versionMin-versionMax but got \"1-2-3\"",
                "TCL VALUE VERSIONRANGE",
            ]),
        };
        assert_eq!(
            probe(version, "", "package vsatisfies 1 1-2-3"),
            expected,
            "{version:?}"
        );
    }
}

/// `package present` names the package in a `TCL LOOKUP PACKAGE` code from 8.5.
#[test]
fn package_present_lookup_code_begins_at_tcl_eight_five() {
    for version in TclVersion::ALL {
        let code = if version >= TclVersion::V8_5 {
            "TCL LOOKUP PACKAGE nope"
        } else {
            "NONE"
        };
        let expected = tcl_syntax::list::join_list(["1", "package nope is not present", code]);
        assert_eq!(
            probe(version, "package forget nope", "package present nope"),
            expected,
            "{version:?}"
        );
    }
}

/// A loader's own error keeps its own code in every release.
#[test]
fn loader_error_keeps_its_own_error_code() {
    for version in TclVersion::ALL {
        let expected = tcl_syntax::list::join_list(["1", "boom", "NONE"]);
        assert_eq!(
            probe(
                version,
                "package forget foo; package ifneeded foo 1 {error boom}",
                "package require foo 1"
            ),
            expected,
            "{version:?}"
        );
        let expected = tcl_syntax::list::join_list(["1", "boom", "MY CODE"]);
        assert_eq!(
            probe(
                version,
                "package forget foo; package ifneeded foo 1 {error boom {} {MY CODE}}",
                "package require foo 1"
            ),
            expected,
            "{version:?}"
        );
    }
}

/// 8.4 predates TIP 268's `exactly` spelling: an `-exact` miss or conflict
/// names the bare version there.
#[test]
fn exact_requirement_spelling_per_release() {
    for version in TclVersion::ALL {
        let (need, code) = match version {
            TclVersion::V8_4 => ("1.1", "NONE"),
            TclVersion::V8_5 => ("exactly 1.1", "NONE"),
            _ => ("exactly 1.1", "TCL PACKAGE VERSIONCONFLICT"),
        };
        let message = format!("version conflict for package \"foo\": have 1, need {need}");
        assert_eq!(
            probe(
                version,
                "package forget foo; package provide foo 1",
                "package require -exact foo 1.1"
            ),
            tcl_syntax::list::join_list(["1", &message, code]),
            "{version:?}"
        );
        let code = if version >= TclVersion::V8_6 {
            "TCL PACKAGE UNFOUND"
        } else {
            "NONE"
        };
        let message = format!("can't find package nope {need}");
        assert_eq!(
            probe(
                version,
                "package forget nope",
                "package require -exact nope 1.1"
            ),
            tcl_syntax::list::join_list(["1", &message, code]),
            "{version:?}"
        );
    }
}
