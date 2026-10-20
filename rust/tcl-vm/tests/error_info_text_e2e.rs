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

//! The command text errorInfo names, as C's `TclLogCommandInfo` does: the
//! command's source through the word separators before its terminator, the
//! command inside a substitution rather than the statement around it, and an
//! `apply` lambda's `(lambda term …)` frame.
//!
//! Every expectation is the whole errorInfo of the same script under real
//! tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 for the releases each case
//! names.

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::Vm;

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

/// The result of `src` under the `version` pin; panics if it errors.
fn result_for(version: TclVersion, src: &str) -> String {
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

/// errorInfo names a command through the whitespace before its
/// terminator, but not through a `;`.
#[test]
fn error_info_keeps_trailing_whitespace() {
    assert_releases(
        &ALL_RELEASES,
        r#"proc ei {script} { catch {uplevel 1 $script} m; return $::errorInfo }
proc a1 {} { error x1 }
proc a2 {} {set y $nope  ;}
proc a3 {} "error x3\t\t"
proc a4 {} {error x4;# c}
list [ei a1] [ei a2] [ei a3] [ei a4]"#,
        r#"{x1
    while executing
"error x1 "
    (procedure "a1" line 1)
    invoked from within
"a1"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {can't read "nope": no such variable
    while executing
"set y $nope  "
    (procedure "a2" line 1)
    invoked from within
"a2"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {x3
    while executing
"error x3		"
    (procedure "a3" line 1)
    invoked from within
"a3"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {x4
    while executing
"error x4"
    (procedure "a4" line 1)
    invoked from within
"a4"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"}"#,
    );
}

/// A command failing inside `[…]` is named itself, not the statement around
/// the substitution.
#[test]
fn error_info_names_the_command_inside_a_substitution() {
    assert_releases(
        &ALL_RELEASES,
        r"proc ei {script} { catch {uplevel 1 $script} m; return $::errorInfo }
proc h {} { error inner }
proc b1 {} { set x [h ] }
proc b2 {} { set a [error x6 ] }
proc b3 {} { puts [string length [h]] }
list [ei b1] [ei b2] [ei b3]",
        r#"{inner
    while executing
"error inner "
    (procedure "h" line 1)
    invoked from within
"h "
    (procedure "b1" line 1)
    invoked from within
"b1"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {x6
    while executing
"error x6 "
    (procedure "b2" line 1)
    invoked from within
"b2"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {inner
    while executing
"error inner "
    (procedure "h" line 1)
    invoked from within
"h"
    (procedure "b3" line 1)
    invoked from within
"b3"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"}"#,
    );
}

/// Commands in inline-compiled bodies follow the same rule.
#[test]
fn error_info_inline_body_command_keeps_trailing_whitespace() {
    assert_releases(
        &ALL_RELEASES,
        r"proc ei {script} { catch {uplevel 1 $script} m; return $::errorInfo }
proc b5 {} { if 1 { error x9 } }
proc b6 {} { foreach i {1} { set q $nope } }
list [ei b5] [ei b6]",
        r#"{x9
    while executing
"error x9 "
    (procedure "b5" line 1)
    invoked from within
"b5"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {can't read "nope": no such variable
    while executing
"set q $nope "
    (procedure "b6" line 1)
    invoked from within
"b6"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"}"#,
    );
}

/// An `apply` body adds a `(lambda term …)` frame and the `apply` command,
/// not the internal procedure it runs as.
#[test]
fn error_info_names_an_apply_lambda() {
    assert_releases(
        &FROM_8_5,
        r"proc ei {script} { catch {uplevel 1 $script} m; return $::errorInfo }
proc la {} { apply {{} { error lam }} }
list [ei {apply {{} { error lam }}}] [ei la]",
        r#"{lam
    while executing
"error lam "
    (lambda term "{} { error lam }" line 1)
    invoked from within
"apply {{} { error lam }}"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} {lam
    while executing
"error lam "
    (lambda term "{} { error lam }" line 1)
    invoked from within
"apply {{} { error lam }} "
    (procedure "la" line 1)
    invoked from within
"la"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"}"#,
    );
}

/// Procedure and lambda names are cut at 60 bytes, as C's `MakeProcError`.
#[test]
fn error_info_truncates_long_lambda_and_procedure_names() {
    assert_releases(
        &FROM_8_5,
        r"proc ei {script} { catch {uplevel 1 $script} m; return $::errorInfo }
proc a_procedure_name_that_is_considerably_longer_than_sixty_bytes_in_total {} { error long }
list [ei a_procedure_name_that_is_considerably_longer_than_sixty_bytes_in_total] [ei {apply {{} { error lam ; # a lambda term long enough to pass the sixty byte limit }}}]",
        r#"{long
    while executing
"error long "
    (procedure "a_procedure_name_that_is_considerably_longer_than_sixty_byte..." line 1)
    invoked from within
"a_procedure_name_that_is_considerably_longer_than_sixty_bytes_in_total"
    ("uplevel" body line 1)
    invoked from within
"uplevel 1 $script"} lam\n\ \ \ \ while\ executing\n\"error\ lam\ \"\n\ \ \ \ (lambda\ term\ \"\{\}\ \{\ error\ lam\ \;\ #\ a\ lambda\ term\ long\ enough\ to\ pass\ the\ six...\"\ line\ 1)\n\ \ \ \ invoked\ from\ within\n\"apply\ \{\{\}\ \{\ error\ lam\ \;\ #\ a\ lambda\ term\ long\ enough\ to\ pass\ the\ sixty\ byte\ limit\ \}\}\"\n\ \ \ \ (\"uplevel\"\ body\ line\ 1)\n\ \ \ \ invoked\ from\ within\n\"uplevel\ 1\ \$script\""#,
    );
}
