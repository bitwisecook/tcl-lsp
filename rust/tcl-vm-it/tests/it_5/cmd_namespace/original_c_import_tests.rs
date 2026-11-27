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

//! C's `namespace import`, through object-word calls and scripts.

use crate::prelude::*;

const RELEASES: [&str; 5] = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"];

fn actual(release: &str) -> Vm {
    crate::fixture::interpreter(tcl_vm::test_support::profile(release))
}
fn bytes(word: &[u8]) -> Value {
    Value::new_native_string_bytes(word)
}
fn import(vm: &mut Vm, patterns: &[Value]) -> Completion<Value> {
    let mut arguments = vec![Value::string("import")];
    arguments.extend_from_slice(patterns);
    crate::fixture::call(vm, "namespace", &arguments)
}
fn eval(vm: &mut Vm, script: &str) -> Vec<u8> {
    let completion = crate::fixture::eval(vm, script);
    assert_eq!(completion.code, Code::Ok, "{script}");
    crate::fixture::bytes(vm, &completion.result)
}
fn setup(vm: &mut Vm) {
    eval(
        vm,
        "namespace eval source {proc first {} {return SOURCE}; proc victim {} {return IMPORTED}; namespace export first victim}; namespace eval destination {}",
    );
}

#[test]
fn import_commits_each_original_pattern_before_reaching_the_next_getter() {
    for release in RELEASES {
        let mut vm = actual(release);
        setup(&mut vm);
        assert_eq!(
            eval(
                &mut vm,
                "namespace eval destination {list [catch {namespace import ::source::first unqualified 42} r] $r [first] [namespace origin first]}",
            ),
            b"1 {no namespace specified in import pattern \"unqualified\"} SOURCE ::source::first",
            "{release}"
        );
    }
}

#[test]
fn import_force_uses_exact_leading_c_extent_and_later_force_remains_a_pattern() {
    for release in RELEASES {
        let mut vm = actual(release);
        setup(&mut vm);
        eval(&mut vm, "proc first {} {return OWN}");
        let completion = import(
            &mut vm,
            &[
                bytes(b"-force\0ignored"),
                bytes(b"::source::first\0ignored"),
            ],
        );
        assert_eq!(completion.code, Code::Ok, "{release}");
        assert_eq!(eval(&mut vm, "first"), b"SOURCE");
        for spelling in [b"-for".as_slice(), b"--", b"-force\xc0\x80tail"] {
            let completion = import(&mut vm, &[bytes(spelling), Value::int(42)]);
            assert_eq!(completion.code, Code::Error);
        }
        let completion = import(
            &mut vm,
            &[
                Value::string("::source::victim"),
                Value::string("-force"),
                Value::int(42),
            ],
        );
        assert_eq!(completion.code, Code::Error);
        assert_eq!(
            eval(&mut vm, "namespace origin victim"),
            b"::source::victim"
        );
        let query = import(&mut vm, &[]);
        assert_eq!(query.code, Code::Ok);
        let names = crate::fixture::bytes(&vm, &query.result);
        if release == "tcl8.4" {
            assert_eq!(names, b"");
        } else {
            assert!(
                names
                    .split(|byte| *byte == b' ')
                    .any(|word| word == b"first")
            );
        }
    }
}

#[test]
fn import_keeps_opaque_original_pattern_bytes_and_actual_source_token() {
    for release in RELEASES {
        let mut vm = actual(release);
        setup(&mut vm);
        let declared = crate::fixture::call(
            &mut vm,
            "proc",
            &[
                bytes(b"::source::p\xff"),
                Value::empty(),
                Value::string("return OPAQUE"),
            ],
        );
        assert_eq!(declared.code, Code::Ok, "{release}");
        crate::fixture::call(&mut vm, "set", &[Value::string("name"), bytes(b"p\xff")]);
        eval(
            &mut vm,
            "namespace eval ::source {namespace export $::name}",
        );
        assert_eq!(
            import(&mut vm, &[bytes(b"::source::p\xff\0ignored")]).code,
            Code::Ok
        );
        assert_eq!(eval(&mut vm, "eval [list $name]"), b"OPAQUE");
        assert_eq!(eval(&mut vm, "namespace origin $name"), b"::source::p\xff");
    }
}

#[test]
fn import_preload_observes_actual_release_shadow_selection_before_validation() {
    for release in RELEASES {
        let mut vm = actual(release);
        let completion = crate::fixture::eval(
            &mut vm,
            "proc auto_import {pattern} {error GLOBAL}; namespace eval destination {proc auto_import {pattern} {error LOCAL}; namespace import ::missing::*}",
        );
        assert_eq!(completion.code, Code::Error);
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            if release == "tcl8.4" {
                b"GLOBAL".as_slice()
            } else {
                b"LOCAL".as_slice()
            },
            "{release}"
        );
    }
}

#[test]
fn import_preload_c84_direct_call_bypasses_ordinary_execution_trace() {
    for release in RELEASES {
        let mut vm = actual(release);
        assert_eq!(
            eval(
                &mut vm,
                "set entered 0; proc count args {incr ::entered}; proc auto_import pattern {return}; trace add execution auto_import enter count; catch {namespace import ::missing::*}; set entered",
            ),
            if release == "tcl8.4" {
                b"0".as_slice()
            } else {
                b"1".as_slice()
            },
            "{release}"
        );
    }
}

#[test]
fn import_preload_uses_fresh_c_extent_objects_and_restores_nested_original_vectors() {
    for release in RELEASES {
        let mut vm = actual(release);
        eval(
            &mut vm,
            "proc unknown args {return UNKNOWN_OK}; proc auto_import pattern {lappend ::calls $pattern [llength [info level 0]]; set ::nestedImport yes; lappend ::calls [missingImportProbe]; error PRELOAD91}",
        );
        let completed = import(
            &mut vm,
            &[bytes(b"::missing::item\0ignored"), Value::int(42)],
        );
        assert_eq!(completed.code, Code::Error, "{release}");
        assert_eq!(completed.result.string_bytes().as_ref(), b"PRELOAD91");
        assert_eq!(
            eval(&mut vm, "list $calls $nestedImport"),
            b"{::missing::item 2 UNKNOWN_OK} yes",
            "{release}"
        );
    }
}

#[test]
fn import_preload_can_publish_source_before_validation_and_absence_never_enters_unknown() {
    for release in RELEASES {
        let mut vm = actual(release);
        eval(
            &mut vm,
            "proc unknown args {error WRONG_UNKNOWN}; proc auto_import pattern {namespace eval generated {proc member {} {return GENERATED}; namespace export member}}",
        );
        let imported = import(&mut vm, &[Value::string("::generated::member")]);
        assert_eq!(imported.code, Code::Ok, "{release}");
        assert_eq!(eval(&mut vm, "member"), b"GENERATED");
        eval(&mut vm, "rename auto_import {}");
        let missing = import(&mut vm, &[Value::string("::absent::member")]);
        assert_eq!(missing.code, Code::Error);
        assert!(
            missing
                .result
                .string_bytes()
                .windows(b"unknown namespace".len())
                .any(|word| word == b"unknown namespace")
        );
    }
}

fn refuse_import(vm: &mut Vm, _: &[Value]) -> Completion<Value> {
    vm.refuse_host_command("original import publication first cause".into())
}
#[test]
fn import_publication_callback_refusal_stops_before_import_stamp_and_later_getter() {
    for release in RELEASES {
        let mut vm = actual(release);
        setup(&mut vm);
        vm.register("refuseImport", refuse_import);
        eval(
            &mut vm,
            "proc victim {} {return OWN}; trace add command victim delete refuseImport",
        );
        let refusal = vm
            .try_invoke_command(
                "namespace",
                &[
                    Value::string("import"),
                    Value::string("-force"),
                    Value::string("::source::victim"),
                    Value::int(42),
                ],
            )
            .expect_err("the trace callback refuses");
        assert!(
            format!("{refusal:?}").contains("original import publication first cause"),
            "{release}: {refusal:?}"
        );
        assert_eq!(eval(&mut vm, "namespace origin victim"), b"::victim");
    }
}
