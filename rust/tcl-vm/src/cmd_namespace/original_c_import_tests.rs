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

use super::*;
use std::{cell::RefCell, rc::Rc};

fn actual(release: &str) -> Vm {
    crate::native_fixture::interpreter(crate::environment::profile_for_dialect(release))
}
fn import(vm: &mut Vm, patterns: &[Value]) -> Completion<Value> {
    let mut arguments = vec![Value::string("import")];
    arguments.extend_from_slice(patterns);
    // Handler getter order ends before the public evaluator logs an Error argv.
    vm.invoke_command_value_at(
        vm.current_ns_id(),
        &Value::string("namespace"),
        &arguments,
        &[],
        tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
    )
}
fn setup(vm: &mut Vm) {
    assert_eq!(vm.eval_source("namespace eval source {proc first {} {return SOURCE}; proc victim {} {return IMPORTED}; namespace export first victim}; namespace eval destination {}").unwrap().code, Code::Ok);
}

#[test]
fn import_commits_each_original_pattern_before_reaching_the_next_getter() {
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(release);
        setup(&mut vm);
        let destination = vm.activate_namespace_operand(b"::destination").unwrap();
        vm.push_ns_token("destination", destination);
        let later = Value::int(42);
        let completion = import(
            &mut vm,
            &[
                Value::string("::source::first"),
                Value::string("unqualified"),
                later.clone(),
            ],
        );
        assert_eq!(completion.code, Code::Error, "{release}");
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            b"no namespace specified in import pattern \"unqualified\""
        );
        assert!(later.resident_string_bytes().is_none());
        let imported = vm.invoke_host_original_object_vector(&Value::string("first"), &[]);
        assert_eq!(imported.code, Code::Ok);
        assert_eq!(imported.result.string_bytes().as_ref(), b"SOURCE");
        let from =
            tcl_runtime_api::Namespaces::find_command_bytes(&vm, destination, b"::source::first")
                .unwrap();
        let to =
            tcl_runtime_api::Namespaces::find_command_bytes(&vm, destination, b"first").unwrap();
        assert_eq!(
            tcl_runtime_api::Namespaces::command_origin(&vm, to),
            Some(from)
        );
        assert!(vm.refused_completion().is_none());
        vm.pop_ns();
    }
}

#[test]
fn import_force_uses_exact_leading_c_extent_and_later_force_remains_a_pattern() {
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(release);
        setup(&mut vm);
        assert_eq!(
            vm.eval_source("proc first {} {return OWN}").unwrap().code,
            Code::Ok
        );
        let flag = Value::new_native_string_bytes(b"-force\0ignored".as_slice());
        let pattern = Value::new_native_string_bytes(b"::source::first\0ignored".as_slice());
        let completion = import(&mut vm, &[flag.clone(), pattern.clone()]);
        assert_eq!(completion.code, Code::Ok, "{release}");
        assert_eq!(
            flag.resident_string_bytes().unwrap().as_ref(),
            b"-force\0ignored"
        );
        assert_eq!(
            pattern.resident_string_bytes().unwrap().as_ref(),
            b"::source::first\0ignored"
        );
        let imported = vm.invoke_host_original_object_vector(&Value::string("first"), &[]);
        assert_eq!(imported.result.string_bytes().as_ref(), b"SOURCE");
        for spelling in [b"-for".as_slice(), b"--", b"-force\xc0\x80tail"] {
            let later = Value::int(42);
            let completion = import(
                &mut vm,
                &[Value::new_native_string_bytes(spelling), later.clone()],
            );
            assert_eq!(completion.code, Code::Error);
            assert!(later.resident_string_bytes().is_none());
        }
        let later = Value::int(42);
        let completion = import(
            &mut vm,
            &[
                Value::string("::source::victim"),
                Value::string("-force"),
                later.clone(),
            ],
        );
        assert_eq!(completion.code, Code::Error);
        assert!(
            vm.imported_command_tails()
                .iter()
                .any(|tail| tail.as_bytes() == b"victim")
        );
        assert!(later.resident_string_bytes().is_none());
        let query = import(&mut vm, &[]);
        assert_eq!(query.code, Code::Ok);
        if release == "tcl8.4" {
            assert_eq!(query.result.resident_string_bytes().unwrap().as_ref(), b"");
        } else {
            assert!(
                query
                    .result
                    .as_list()
                    .unwrap()
                    .iter()
                    .any(|word| word.string_bytes().as_ref() == b"first")
            );
        }
    }
}

#[test]
fn import_keeps_opaque_original_pattern_bytes_and_actual_source_token() {
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(release);
        setup(&mut vm);
        let declared = vm.invoke_host_original_object_vector(
            &Value::string("proc"),
            &[
                Value::new_native_string_bytes(b"::source::p\xff".as_slice()),
                Value::empty(),
                Value::string("return OPAQUE"),
            ],
        );
        assert_eq!(declared.code, Code::Ok, "{release}");
        let source = vm.activate_namespace_operand(b"::source").unwrap();
        vm.push_ns_token("source", source);
        let exported = vm.invoke_host_original_object_vector(
            &Value::string("namespace"),
            &[
                Value::string("export"),
                Value::new_native_string_bytes(b"p\xff".as_slice()),
            ],
        );
        assert_eq!(exported.code, Code::Ok);
        vm.pop_ns();
        let pattern = Value::new_native_string_bytes(b"::source::p\xff\0ignored".as_slice());
        assert_eq!(
            import(&mut vm, std::slice::from_ref(&pattern)).code,
            Code::Ok
        );
        assert_eq!(
            pattern.resident_string_bytes().unwrap().as_ref(),
            b"::source::p\xff\0ignored"
        );
        let called = vm.invoke_host_original_object_vector(
            &Value::new_native_string_bytes(b"p\xff".as_slice()),
            &[],
        );
        assert_eq!(called.code, Code::Ok);
        assert_eq!(called.result.string_bytes().as_ref(), b"OPAQUE");
        let original =
            tcl_runtime_api::Namespaces::find_command_bytes(&vm, source, b"::source::p\xff")
                .unwrap();
        let imported = tcl_runtime_api::Namespaces::find_command_bytes(
            &vm,
            tcl_runtime_api::ROOT_NS,
            b"p\xff",
        )
        .unwrap();
        assert_eq!(
            tcl_runtime_api::Namespaces::command_origin(&vm, imported),
            Some(original)
        );
    }
}

#[test]
fn import_preload_observes_actual_release_shadow_selection_before_validation() {
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(release);
        let completion = vm.eval_source("proc auto_import {pattern} {error GLOBAL}; namespace eval destination {proc auto_import {pattern} {error LOCAL}; namespace import ::missing::*}").unwrap();
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
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(release);
        let completion = vm.eval_source("set entered 0; proc count args {incr ::entered}; proc auto_import pattern {return}; trace add execution auto_import enter count; catch {namespace import ::missing::*}; set entered").unwrap();
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            if release == "tcl8.4" {
                b"0".as_slice()
            } else {
                b"1".as_slice()
            },
            "{release}"
        );
    }
}

struct OriginalPreload {
    original: Value,
    calls: Rc<RefCell<Vec<Vec<u8>>>>,
    c84: bool,
    nested: bool,
}
impl crate::command::NativeCommand for OriginalPreload {
    fn invoke(&self, vm: &mut Vm, arguments: &[Value]) -> Completion<Value> {
        assert_eq!(arguments.len(), 1);
        let vector = vm
            .native_invocation
            .arguments
            .as_ref()
            .unwrap()
            .lifetime_view();
        let words = vector.elements().unwrap();
        assert_eq!(words.len(), 2);
        assert_eq!(
            words[0].resident_string_bytes().unwrap().as_ref(),
            b"auto_import"
        );
        assert!(words[1].is_same_object(&arguments[0]));
        assert!(!words[1].is_same_object(&self.original));
        assert_eq!(
            words[1].native_object_snapshot().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::None
        );
        assert_eq!(words[1].native_object_reference_count(), 1);
        if self.c84 {
            assert_eq!(words[0].native_object_reference_count(), 1);
        }
        let identities: Vec<_> = words.iter().map(Value::native_object_identity).collect();
        self.calls
            .borrow_mut()
            .push(arguments[0].resident_string_bytes().unwrap().to_vec());
        if self.nested {
            let nested = vm.invoke_host_original_object_vector(
                &Value::string("set"),
                &[Value::string("nestedImport"), Value::string("yes")],
            );
            assert_eq!(nested.code, Code::Ok);
            let unknown =
                vm.invoke_host_original_object_vector(&Value::string("missingImportProbe"), &[]);
            assert_eq!(unknown.code, Code::Ok);
            assert_eq!(unknown.result.string_bytes().as_ref(), b"UNKNOWN_OK");
            let restored: Vec<_> = vm
                .native_invocation
                .arguments
                .as_ref()
                .unwrap()
                .elements()
                .unwrap()
                .iter()
                .map(Value::native_object_identity)
                .collect();
            assert_eq!(restored, identities);
        }
        err("PRELOAD91")
    }
}
fn unknown_ok(_: &mut Vm, _: &[Value]) -> Completion<Value> {
    ok(Value::string("UNKNOWN_OK"))
}

#[test]
fn import_preload_uses_fresh_c_extent_objects_and_restores_nested_original_vectors() {
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(release);
        let original = Value::new_native_string_bytes(b"::missing::item\0ignored".as_slice());
        let calls = Rc::new(RefCell::new(Vec::new()));
        vm.register("unknown", unknown_ok);
        vm.register_native_command(
            "auto_import",
            Rc::new(OriginalPreload {
                original: original.clone(),
                calls: Rc::clone(&calls),
                c84: release == "tcl8.4",
                nested: true,
            }),
        );
        let later = Value::int(42);
        let completed = import(&mut vm, &[original.clone(), later.clone()]);
        assert_eq!(completed.code, Code::Error, "{release}");
        assert_eq!(completed.result.string_bytes().as_ref(), b"PRELOAD91");
        assert_eq!(&*calls.borrow(), &[b"::missing::item".to_vec()]);
        assert_eq!(
            original.resident_string_bytes().unwrap().as_ref(),
            b"::missing::item\0ignored"
        );
        assert!(later.resident_string_bytes().is_none());
        assert!(vm.native_invocation.arguments.is_none());
        assert!(vm.refused_completion().is_none());
    }
}

#[test]
fn import_preload_can_publish_source_before_validation_and_absence_never_enters_unknown() {
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(release);
        assert_eq!(vm.eval_source("proc unknown args {error WRONG_UNKNOWN}; proc auto_import pattern {namespace eval generated {proc member {} {return GENERATED}; namespace export member}}").unwrap().code, Code::Ok);
        let imported = import(&mut vm, &[Value::string("::generated::member")]);
        assert_eq!(imported.code, Code::Ok, "{release}");
        assert_eq!(
            vm.invoke_host_original_object_vector(&Value::string("member"), &[])
                .result
                .string_bytes()
                .as_ref(),
            b"GENERATED"
        );
        assert_eq!(
            vm.eval_source("rename auto_import {}").unwrap().code,
            Code::Ok
        );
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
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(release);
        setup(&mut vm);
        vm.register("refuseImport", refuse_import);
        assert_eq!(
            vm.eval_source(
                "proc victim {} {return OWN}; trace add command victim delete refuseImport"
            )
            .unwrap()
            .code,
            Code::Ok
        );
        let later = Value::int(42);
        let completion = import(
            &mut vm,
            &[
                Value::string("-force"),
                Value::string("::source::victim"),
                later.clone(),
            ],
        );
        assert_eq!(completion.code, Code::Error);
        assert!(vm.refused_completion().is_some());
        assert!(later.resident_string_bytes().is_none());
        assert!(
            !vm.imported_command_tails()
                .iter()
                .any(|name| name.as_bytes() == b"victim")
        );
        let first = vm.take_host_boundary_refusal().unwrap();
        assert!(format!("{first:?}").contains("original import publication first cause"));
    }
}
