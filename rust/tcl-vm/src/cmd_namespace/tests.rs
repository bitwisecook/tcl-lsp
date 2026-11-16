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

fn capture_namespace_frame(vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
    ok(Value::list(vm.frame_argv(vm.current_level()).unwrap()))
}

#[test]
fn namespace_frames_retain_actual_public_and_worker_argument_objects() {
    for version in tcl_dialect::TclVersion::ALL {
        for member in ["eval", "inscope"] {
            let profile = crate::environment::profile_for_dialect(version.dialect_profile_name());
            for worker in [false, true] {
                if worker
                    && matches!(
                        version,
                        tcl_dialect::TclVersion::V8_4 | tcl_dialect::TclVersion::V8_5
                    )
                {
                    continue;
                }
                let mut vm = crate::native_fixture::interpreter(profile);
                vm.register("captureNamespaceFrame", capture_namespace_frame);
                let target = Value::string("n");
                assert_eq!(
                    vm.eval_source("namespace eval n {}").unwrap().code,
                    Code::Ok
                );
                let head = Value::new_native_string_bytes(b"renamed namespace\xff".as_slice());
                let source = if worker {
                    format!("::tcl::namespace::{member}")
                } else {
                    "namespace".to_owned()
                };
                let renamed = vm.invoke_host_original_object_vector(
                    &Value::string("rename"),
                    &[Value::string(source), head.clone()],
                );
                assert_eq!(renamed.code, Code::Ok, "{version:?}/{member}/{worker}");
                let selector = Value::string(if member == "eval" { "ev" } else { "insc" });
                let body = Value::list(vec![Value::string("::captureNamespaceFrame")]);
                let mut arguments = Vec::new();
                if !worker {
                    arguments.push(selector.clone());
                }
                arguments.extend([target.clone(), body.clone()]);
                let completion = vm.invoke_host_original_object_vector(&head, &arguments);
                assert_eq!(
                    completion.code,
                    Code::Ok,
                    "{version:?}/{member}/{worker}: {:?}",
                    completion.result.string_bytes()
                );
                let returned = completion.result.as_list().unwrap();
                if version == tcl_dialect::TclVersion::V8_4 && member == "inscope" {
                    assert!(returned.is_empty());
                } else {
                    assert_eq!(returned.len(), arguments.len() + 1);
                    assert!(returned[0].is_same_object(&head));
                    for (actual, original) in returned[1..].iter().zip(&arguments) {
                        assert!(
                            actual.is_same_object(original),
                            "{version:?}/{member}/{worker}"
                        );
                    }
                }
                assert_eq!(vm.current_ns_id(), tcl_runtime_api::ROOT_NS);
            }
        }
    }
}

#[test]
fn jim_namespace_frames_retain_the_actual_eval_and_scripted_tailcall_vectors() {
    use tcl_registry::native_scripted_distribution::NativeScriptedLibrary;
    let mut vm = crate::native_fixture::interpreter_with_scripted_libraries(
        crate::environment::profile_for_dialect("jimtcl"),
        &[NativeScriptedLibrary::NamespaceHelpers],
    );
    vm.register("captureNamespaceFrame", capture_namespace_frame);
    let body = Value::list(vec![Value::string("::captureNamespaceFrame")]);
    let target = Value::string("n");
    let selector = Value::string("eval");
    let head = Value::string("namespace");
    let arguments = [selector.clone(), target.clone(), body.clone()];
    let completion = vm.invoke_host_original_object_vector(&head, &arguments);
    assert_eq!(completion.code, Code::Ok);
    let returned = completion.result.as_list().unwrap();
    assert_eq!(returned.len(), 4);
    assert!(returned[0].is_same_object(&head));
    for (actual, original) in returned[1..].iter().zip(&arguments) {
        assert!(actual.is_same_object(original));
    }
    let inscope = vm.invoke_host_original_object_vector(
        &head,
        &[Value::string("inscope"), target.clone(), body.clone()],
    );
    assert_eq!(inscope.code, Code::Ok);
    let returned = inscope.result.as_list().unwrap();
    // The original distribution helper tailcalls namespace eval with its
    // actual args List as a fourth operand; its caller head is not retained.
    assert_eq!(returned.len(), 5);
    assert_eq!(returned[0].string_bytes().as_ref(), b"namespace");
    assert_eq!(returned[1].string_bytes().as_ref(), b"eval");
    assert!(returned[2].is_same_object(&target));
    assert!(returned[3].is_same_object(&body));
    assert!(returned[4].as_list().unwrap().is_empty());
}

#[test]
fn namespace_frame_transport_borrows_headers_and_requires_original_invocation() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    let scope =
        crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm).unwrap();
    assert!(namespace_frame_arguments(&mut vm, NativeNamespaceFramePurpose::Eval, &scope).is_err());
    let words = std::rc::Rc::new(vec![
        Value::string("worker"),
        Value::string("n"),
        Value::string("body"),
    ]);
    let references: Vec<_> = words
        .iter()
        .map(Value::native_object_reference_count)
        .collect();
    vm.native_invocation.arguments = Some(crate::NativeListItems::invocation_view(
        std::rc::Rc::clone(&words),
    ));
    let transported =
        namespace_frame_arguments(&mut vm, NativeNamespaceFramePurpose::Eval, &scope).unwrap();
    for ((original, borrowed), references) in words.iter().zip(&transported).zip(references) {
        assert!(original.is_same_object(borrowed));
        assert_eq!(original.native_object_reference_count(), references);
    }
    drop(transported);
    scope.finish(&mut vm).unwrap();
}

#[test]
fn namespace_frame_fetch_observes_withdrawn_ensemble_rewrite_at_publication() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    let scope =
        crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let worker = Value::string("actual worker");
    let original = Value::string("outer original");
    vm.native_invocation.arguments =
        Some(crate::NativeListItems::invocation_view(std::rc::Rc::new(
            vec![worker.clone(), Value::string("n"), Value::string("body")],
        )));
    vm.native_invocation.usage_rewrites = vec![crate::command::NativeArgumentUsageRewrite {
        original_prefix: vec![original.clone(), Value::string("member")],
        removed_words: 1,
    }];
    let before =
        namespace_frame_arguments(&mut vm, NativeNamespaceFramePurpose::Eval, &scope).unwrap();
    assert!(before[0].is_same_object(&original));
    vm.reset_native_ensemble_rewrite(
        tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::BeforeOrdinaryLookup,
    )
    .unwrap();
    let after =
        namespace_frame_arguments(&mut vm, NativeNamespaceFramePurpose::Eval, &scope).unwrap();
    assert!(after[0].is_same_object(&worker));
    assert_eq!(after.len(), 3);
    scope.finish(&mut vm).unwrap();
}

#[test]
fn native_namespace_bodies_propagate_return_and_inscope_requires_c_target() {
    for profile in tcl_dialect::TclVersion::ALL
        .map(|version| crate::environment::profile_for_dialect(version.dialect_profile_name()))
        .into_iter()
        .chain(std::iter::once(crate::environment::profile_for_dialect(
            "jim",
        )))
    {
        let mut vm = crate::native_fixture::interpreter(profile);
        let arguments = [Value::string("n"), Value::string("return x")];
        // These direct handler tests install the real caller vector explicitly;
        // frame construction no longer invents a public command head.
        let mut words = vec![Value::string("namespace"), Value::string("eval")];
        words.extend_from_slice(&arguments);
        vm.native_invocation.arguments = Some(crate::NativeListItems::invocation_view(
            std::rc::Rc::new(words),
        ));
        let returned = ns_eval(&mut vm, &arguments);
        assert_eq!(returned.code, Code::Return, "{}", profile.name);
        assert_eq!(returned.result.string_bytes().as_ref(), b"x");
        assert_eq!(vm.current_ns_id(), tcl_runtime_api::ROOT_NS);
        let mut words = vec![Value::string("namespace"), Value::string("inscope")];
        words.extend_from_slice(&arguments);
        vm.native_invocation.arguments = Some(crate::NativeListItems::invocation_view(
            std::rc::Rc::new(words),
        ));
        let returned = ns_inscope(&mut vm, &arguments);
        assert_eq!(returned.code, Code::Return, "{}", profile.name);
        let absent = [Value::string("absent"), Value::string("return y")];
        let mut words = vec![Value::string("namespace"), Value::string("inscope")];
        words.extend_from_slice(&absent);
        vm.native_invocation.arguments = Some(crate::NativeListItems::invocation_view(
            std::rc::Rc::new(words),
        ));
        let result = ns_inscope(&mut vm, &absent);
        if vm.name_policy_protocol().unwrap().recipe().is_jim084() {
            assert_eq!(result.code, Code::Return);
            assert_eq!(result.result.string_bytes().as_ref(), b"y");
        } else {
            assert_eq!(result.code, Code::Error);
            assert!(
                result
                    .result
                    .string_bytes()
                    .windows(b"absent".len())
                    .any(|word| word == b"absent")
            );
            assert!(vm.namespace_object_lookup(&absent[0]).unwrap().is_none());
        }
    }
}

#[test]
fn namespace_text_preserves_original_native_byte_names() {
    let profiles = tcl_dialect::TclVersion::ALL
        .map(|version| crate::environment::profile_for_dialect(version.dialect_profile_name()));
    for profile in
        profiles
            .into_iter()
            .chain(std::iter::once(crate::environment::profile_for_dialect(
                "jim",
            )))
    {
        let mut vm = Vm::with_output(Box::new(std::io::sink()));
        vm.set_dialect_profile(profile);
        let raw = Value::from_native_string_bytes(b"a\0z::p".as_slice());
        let tail = ns_text_op(&mut vm, std::slice::from_ref(&raw), true);
        assert_eq!(tail.code, Code::Ok);
        let expected: &[u8] = if vm.name_policy_protocol().unwrap().recipe().is_jim084() {
            b"a\0z::p"
        } else {
            b"a"
        };
        assert_eq!(tail.result.string_bytes().as_ref(), expected);
        let opaque = Value::from_native_string_bytes(b"n::\xff".as_slice());
        let tail = ns_text_op(&mut vm, std::slice::from_ref(&opaque), true);
        assert_eq!(tail.result.string_bytes().as_ref(), b"\xff");
        assert_eq!(opaque.string_bytes().as_ref(), b"n::\xff");
        let modified = Value::from_native_string_bytes(b"a\xc0\x80z::p".as_slice());
        let qualifiers = ns_text_op(&mut vm, std::slice::from_ref(&modified), false);
        assert_eq!(qualifiers.result.string_bytes().as_ref(), b"a\xc0\x80z");
    }
}

#[test]
fn code_retains_original_objects_for_all_measured_native_prefixes() {
    let specimens: [&[u8]; 11] = [
        b"::namespace inscope ",
        b"::namespace inscope :: cmd",
        b"namespace inscope :: cmd",
        b":namespace inscope :: cmd",
        b"::::namespace inscope :: cmd",
        b"namespace    inscope :: cmd",
        b"namespace\tinscope :: cmd",
        b"namespaceinscopeXX",
        b"namespaceinscopeX",
        b"::namespace inscopeX",
        b"::namespace inscope \0",
    ];
    let jim = crate::environment::profile_for_dialect("jim");
    let mut rows = 0;
    for profile in tcl_dialect::TclVersion::ALL
        .map(|version| crate::environment::profile_for_dialect(version.dialect_profile_name()))
        .into_iter()
        .chain(std::iter::once(jim))
    {
        let mut vm = if std::ptr::eq(profile, jim) {
            crate::native_fixture::interpreter_with_scripted_libraries(
                    profile,
                    &[tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceHelpers],
                )
        } else {
            let mut vm = Vm::with_output(Box::new(std::io::sink()));
            vm.set_dialect_profile(profile);
            vm
        };
        let preserved: &[usize] = if profile.vm_runtime_version == tcl_dialect::TclVersion::V8_4
            && !std::ptr::eq(profile, jim)
        {
            &[0, 1, 2, 3, 4, 5, 7, 9, 10]
        } else if std::ptr::eq(profile, jim) {
            &[0, 1, 10]
        } else {
            &[1, 10]
        };
        for (index, bytes) in specimens.into_iter().enumerate() {
            let script = Value::from_string_bytes(bytes);
            let result = if std::ptr::eq(profile, jim) {
                // Jim's actual namespace code command is a distribution
                // procedure; native C construction cannot supply its literals.
                vm.invoke_host_original_object_vector(
                    &Value::string("namespace"),
                    &[Value::string("code"), script.clone()],
                )
            } else {
                namespace_code_invocation(
                    &mut vm,
                    std::slice::from_ref(&script),
                    NamespaceInvocation::Public,
                )
            };
            assert_eq!(result.code, Code::Ok, "{}: {index}", profile.name);
            if preserved.contains(&index) {
                assert!(
                    result.result.is_same_object(&script),
                    "{}: {index}",
                    profile.name
                );
            } else {
                let elements = result.result.as_list().expect("command-prefix list");
                assert_eq!(elements.len(), 4);
                assert_eq!(elements[0].string_bytes().as_ref(), b"::namespace");
                assert_eq!(elements[1].string_bytes().as_ref(), b"inscope");
                assert_eq!(elements[2].string_bytes().as_ref(), b"::");
                assert!(
                    elements[3].is_same_object(&script),
                    "{}: {index}",
                    profile.name
                );
            }
            assert_eq!(script.string_bytes().as_ref(), bytes);
            rows += 1;
        }
    }
    assert_eq!(rows, 66);
}

#[test]
fn code_construction_allocates_fresh_literal_headers_and_retains_original_roles() {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    use tcl_syntax::native_string::NativeStringStorageIdentity;
    for version in tcl_dialect::TclVersion::ALL {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(
            version.dialect_profile_name(),
        ));
        let namespace = vm.activate_namespace_operand(b"opaque\xff").unwrap();
        vm.push_ns_eval_token_frame(namespace, Vec::new());
        let script = Value::new_native_string_bytes(b"command\xff\0tail".as_slice());
        let before = script.native_object_snapshot();
        let first = namespace_code_invocation(
            &mut vm,
            std::slice::from_ref(&script),
            NamespaceInvocation::Public,
        );
        let second = namespace_code_invocation(
            &mut vm,
            std::slice::from_ref(&script),
            NamespaceInvocation::Public,
        );
        assert_eq!(first.code, Code::Ok, "{version:?}");
        assert_eq!(second.code, Code::Ok, "{version:?}");
        assert!(!first.result.is_same_object(&second.result));
        let snapshot = first.result.native_object_snapshot();
        assert!(snapshot.resident.is_none());
        assert!(matches!(snapshot.cache, Cache::List { length: 4, .. }));
        let first_members = first.result.as_list().unwrap();
        let second_members = second.result.as_list().unwrap();
        for (index, bytes) in [b"::namespace".as_slice(), b"inscope".as_slice()]
            .into_iter()
            .enumerate()
        {
            assert!(!first_members[index].is_same_object(&second_members[index]));
            let snapshot = first_members[index].native_object_snapshot();
            assert_eq!(snapshot.resident.as_deref(), Some(bytes));
            assert_eq!(
                snapshot.storage,
                Some(NativeStringStorageIdentity::Allocated)
            );
            assert_eq!(snapshot.cache, Cache::None);
        }
        assert_eq!(
            first_members[2].resident_string_bytes().unwrap().as_ref(),
            b"::opaque\xff"
        );
        if matches!(
            version,
            tcl_dialect::TclVersion::V9_0 | tcl_dialect::TclVersion::V9_1
        ) {
            assert_eq!(
                first_members[2].native_object_snapshot().cache,
                Cache::NamespaceName {
                    version,
                    resolved: true
                }
            );
        } else {
            assert_eq!(first_members[2].native_object_snapshot().cache, Cache::None);
        }
        assert!(first_members[3].is_same_object(&script));
        assert!(second_members[3].is_same_object(&script));
        assert_eq!(script.native_object_snapshot(), before);
        vm.pop_ns();
        vm.pop_call_frame();
    }
}

#[test]
fn jim_code_public_helper_retains_its_original_parsed_literals() {
    use tcl_registry::native_scripted_distribution::NativeScriptedLibrary;
    let mut vm = crate::native_fixture::interpreter_with_scripted_libraries(
        crate::environment::profile_for_dialect("jimtcl"),
        &[NativeScriptedLibrary::NamespaceHelpers],
    );
    let script = Value::new_native_string_bytes(b"command\xff\0tail".as_slice());
    let head = Value::string("namespace");
    let selector = Value::string("code");
    let arguments = [selector.clone(), script.clone()];
    let first = vm.invoke_host_original_object_vector(&head, &arguments);
    // Public completion transports borrow the result owner until retained.
    let _retained_first_result = first.result.clone();
    let second = vm.invoke_host_original_object_vector(&head, &arguments);
    assert_eq!(first.code, Code::Ok);
    assert_eq!(second.code, Code::Ok);
    assert!(!first.result.is_same_object(&second.result));
    let first_members = first.result.as_list().unwrap();
    let second_members = second.result.as_list().unwrap();
    assert_eq!(first_members.len(), 4);
    for index in 0..2 {
        assert!(first_members[index].is_same_object(&second_members[index]));
    }
    assert_eq!(first_members[0].string_bytes().as_ref(), b"::namespace");
    assert_eq!(first_members[1].string_bytes().as_ref(), b"inscope");
    assert!(first_members[3].is_same_object(&script));
    assert!(second_members[3].is_same_object(&script));
    assert_eq!(script.string_bytes().as_ref(), b"command\xff\0tail");
    assert!(!first_members[0].is_same_object(&head));
    assert!(!first_members[1].is_same_object(&selector));
}

#[test]
fn jim_code_direct_native_route_preserves_selected_recognition_but_requires_the_helper_for_construction()
 {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jimtcl"));
    let recognised = Value::string("::namespace inscope ");
    let preserved = namespace_code_invocation(
        &mut vm,
        std::slice::from_ref(&recognised),
        NamespaceInvocation::Public,
    );
    assert_eq!(preserved.code, Code::Ok);
    assert!(preserved.result.is_same_object(&recognised));
    let script = Value::string("command");
    let unsupported = namespace_code_invocation(
        &mut vm,
        std::slice::from_ref(&script),
        NamespaceInvocation::Public,
    );
    assert_eq!(unsupported.code, Code::Error);
    assert!(vm.refused_completion().is_some());
}

#[test]
fn code_keeps_non_unicode_bytes_and_uses_native_arity() {
    let mut vm = Vm::with_output(Box::new(std::io::sink()));
    vm.set_runtime_version(tcl_dialect::TclVersion::V9_0);
    for bytes in [
        b"\xff\0script".as_slice(),
        b"::namespace inscope \xff\0".as_slice(),
    ] {
        let script = Value::from_string_bytes(bytes);
        let completion = namespace_code_invocation(
            &mut vm,
            std::slice::from_ref(&script),
            NamespaceInvocation::Public,
        );
        assert_eq!(completion.code, Code::Ok);
        let retained = if completion.result.is_same_object(&script) {
            completion.result
        } else {
            completion.result.as_list().unwrap()[3].clone()
        };
        assert!(retained.is_same_object(&script));
        assert_eq!(retained.string_bytes().as_ref(), bytes);
    }
    let completion = namespace_code_invocation(&mut vm, &[], NamespaceInvocation::Public);
    assert_eq!(completion.code, Code::Error);
    assert_eq!(
        completion.result.string_bytes().as_ref(),
        b"wrong # args: should be \"namespace code arg\""
    );
}

#[test]
fn current_and_code_wrong_arity_retain_original_header_words() {
    for version in tcl_dialect::TclVersion::ALL {
        let mut vm = Vm::with_output(Box::new(std::io::sink()));
        vm.set_runtime_version(version);
        vm.set_invoked_name("n s\0tail");
        let expected: &[u8] = match version {
            tcl_dialect::TclVersion::V8_4 => b"n s",
            tcl_dialect::TclVersion::V8_5 | tcl_dialect::TclVersion::V8_6 => b"n s\0tail",
            tcl_dialect::TclVersion::V9_0 | tcl_dialect::TclVersion::V9_1 => b"{n s\0tail}",
        };
        let completion = namespace_current_invocation(
            &mut vm,
            &[Value::string("extra")],
            NamespaceInvocation::Public,
        );
        let mut message = b"wrong # args: should be \"".to_vec();
        message.extend_from_slice(expected);
        message.extend_from_slice(b" current\"");
        assert_eq!(completion.result.string_bytes().as_ref(), message);
        let completion = namespace_code_invocation(&mut vm, &[], NamespaceInvocation::Public);
        let mut message = b"wrong # args: should be \"".to_vec();
        message.extend_from_slice(expected);
        message.extend_from_slice(b" code arg\"");
        assert_eq!(completion.result.string_bytes().as_ref(), message);
    }
}

#[test]
fn code_refuses_an_unaudited_engine_outside_guest_completion() {
    let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )));
    let mut vm = Vm::with_output(Box::new(std::io::sink()));
    vm.set_dialect_profile(profile);
    let script = Value::string("puts example");
    let completion = namespace_code_invocation(
        &mut vm,
        std::slice::from_ref(&script),
        NamespaceInvocation::Public,
    );
    assert_eq!(completion.code, Code::Error);
    assert!(vm.refused_completion().is_some());
    assert_eq!(completion.result.string_bytes().as_ref(), b"");
    assert_eq!(script.string_bytes().as_ref(), b"puts example");
}

#[test]
fn code_uses_the_explicit_f5_logical_provider() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    assert!(vm.set_native_engine_profile(tcl_dialect::DialectProfile::find("tcl9.0").unwrap()));
    assert!(
        vm.set_logical_name_provider(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
            tcl_dialect::TclVersion::V8_4
        ))
    );
    assert!(vm.execution_refusal.is_none(), "{:?}", vm.execution_refusal);
    let script = Value::string("namespaceinscopeXX");
    let completion = namespace_code_invocation(
        &mut vm,
        std::slice::from_ref(&script),
        NamespaceInvocation::Public,
    );
    assert_eq!(completion.code, Code::Ok);
    assert!(completion.result.is_same_object(&script));
    assert!(
        vm.native_invocation_dialect()
            .native_namespace_code_policy()
            .is_none()
    );
}
