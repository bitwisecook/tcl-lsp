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

//! Hidden-command frames and namespaces through `interp invokehidden`, per
//! release.

use crate::prelude::*;

#[test]
fn hidden_global_frame_levels_match_original_c_releases() {
    let source = r"
interp hide {} info hiddenInfo
proc observe {} {
    list [interp invokehidden {} -global hiddenInfo level] [interp invokehidden {} hiddenInfo level]
}
list [observe] [interp invokehidden {} -global hiddenInfo level]
";
    for version in tcl_dialect::TclVersion::ALL {
        let profile = tcl_vm::test_support::profile(version.dialect_profile_name());
        let mut vm = crate::fixture::interpreter(profile);
        let completion = crate::fixture::eval(&mut vm, source);
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            if version == tcl_dialect::TclVersion::V8_4 {
                b"{0 1} 0".as_slice()
            } else {
                b"{2 1} 1".as_slice()
            },
        );
    }
}

#[test]
fn hidden_namespace_context_matches_original_child_and_current_inputs() {
    let source = r"
interp create kid
kid eval {interp hide {} set hiddenSet}
set out [list [kid invokehidden -namespace chosen hiddenSet item CHILD]]
lappend out [kid invokehidden hiddenSet ::chosen::item] [kid eval {info exists ::item}]
lappend out [kid invokehidden -namespace chosen -global hiddenSet globalItem GLOBAL]
lappend out [kid invokehidden hiddenSet ::globalItem]
lappend out [kid invokehidden -global -namespace :literal: hiddenSet item COLON]
lappend out [kid invokehidden -namespace :literal: hiddenSet item]
lappend out [kid invokehidden -namespace {} hiddenSet emptyItem EMPTY]
lappend out [kid invokehidden hiddenSet ::emptyItem] [kid eval {namespace current}]
interp hide {} namespace hiddenNamespace
lappend out [interp invokehidden {} hiddenNamespace eval outer {
    interp invokehidden {} -namespace chosen hiddenNamespace current
}]
set out
";
    for version in tcl_dialect::TclVersion::ALL {
        if version == tcl_dialect::TclVersion::V8_4 {
            continue;
        }
        let profile = tcl_vm::test_support::profile(version.dialect_profile_name());
        let mut vm = crate::fixture::interpreter(profile);
        let completion = crate::fixture::eval(&mut vm, source);
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            b"CHILD CHILD 0 GLOBAL GLOBAL COLON COLON EMPTY EMPTY :: ::chosen",
        );
    }
}

#[test]
fn hidden_namespace_context_keeps_the_original_dying_namespace_token() {
    let source = r"
set ::observations {}
proc during {name1 name2 operation} {
    foreach script {
        {interp invokehidden {} -namespace retiring hiddenNamespace current}
        {interp invokehidden {} hiddenNamespace exists retiring}
        {interp invokehidden {} hiddenNamespace eval retiring {}}
    } {
        set code [catch {uplevel #0 $script} value]
        lappend ::observations [list $code $value]
    }
}
namespace eval retiring {
    variable sentinel HELD
    trace add variable sentinel unset ::during
}
interp hide {} namespace hiddenNamespace
set deleted [catch {interp invokehidden {} hiddenNamespace delete retiring} message]
list $deleted $message $::observations [interp invokehidden {} hiddenNamespace exists retiring]
";
    for version in tcl_dialect::TclVersion::ALL {
        if version == tcl_dialect::TclVersion::V8_4 {
            continue;
        }
        let profile = tcl_vm::test_support::profile(version.dialect_profile_name());
        let mut vm = crate::fixture::interpreter(profile);
        let completion = crate::fixture::eval(&mut vm, source);
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(
                completion.result.string_bytes().as_ref(),
                br#"0 {} {{0 ::retiring} {0 0} {1 {can't create namespace "retiring": already exists}}} 0"#,
            );
    }
}

#[test]
fn public_namespace_conflict_keeps_the_original_operand_and_error_code() {
    for version in tcl_dialect::TclVersion::ALL {
        if version == tcl_dialect::TclVersion::V8_4 {
            continue;
        }
        for (seed, operand) in [
            ("retiring", "retiring"),
            ("retiring", "::retiring"),
            ("retiring", "::::retiring"),
            ("retiring:tail", "retiring:tail"),
        ] {
            let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(
                version.dialect_profile_name(),
            ));
            let source = format!(
                r"
set ::observations {{}}
proc during {{name1 name2 operation}} {{
    set status [catch {{interp invokehidden {{}} hiddenNamespace eval $::operand {{}}}} value]
    set ::observations [list $status $value $::errorCode]
}}
set ::operand {operand}
namespace eval {seed} {{
    variable sentinel HELD
    trace add variable sentinel unset ::during
}}
interp hide {{}} namespace hiddenNamespace
interp invokehidden {{}} hiddenNamespace delete {seed}
set ::observations
"
            );
            let completion = crate::fixture::eval(&mut vm, &source);
            assert_eq!(completion.code, Code::Ok);
            let code = if version >= tcl_dialect::TclVersion::V8_6 {
                "{TCL OPERATION NAMESPACE CREATEEXISTING}"
            } else {
                "NONE"
            };
            let expected =
                format!("1 {{can't create namespace \"{operand}\": already exists}} {code}");
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                expected.as_bytes()
            );
        }
    }
}

#[test]
fn original_trailing_creation_keeps_the_dying_parent_token_and_cstring_extent() {
    for version in tcl_dialect::TclVersion::ALL {
        if version == tcl_dialect::TclVersion::V8_4 {
            continue;
        }
        for (original, expected) in [
            (b"retiring::".as_slice(), b"0 ::retiring".as_slice()),
            (b"retiring::::", b"0 ::retiring"),
            (b"retiring::\0Tail", b"0 ::retiring"),
            (
                b"retiring\0::::",
                b"1 {can't create namespace \"retiring\": already exists}",
            ),
        ] {
            let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(
                version.dialect_profile_name(),
            ));
            vm.set_var_bytes(b"::operand", Value::new_native_string_bytes(original))
                .unwrap();
            let source = r"
set ::observations {}
proc during {name1 name2 operation} {
    set status [catch {namespace eval $::operand {namespace current}} value]
    set ::observations [list $status $value]
}
namespace eval retiring {
    variable sentinel HELD
    trace add variable sentinel unset ::during
}
namespace delete retiring
set ::observations
";
            let completion = crate::fixture::eval(&mut vm, source);
            assert_eq!(completion.code, Code::Ok);
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                expected,
                "{version:?}"
            );
            let final_exists = crate::fixture::eval(&mut vm, "namespace exists retiring");
            assert_eq!(final_exists.code, Code::Ok);
            assert_eq!(final_exists.result.string_bytes().as_ref(), b"0");
        }
    }
}
