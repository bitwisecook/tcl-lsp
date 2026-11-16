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

#[test]
fn configuration_namespace_results_keep_exact_selected_bytes_and_primary() {
    for release in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(release));
        for (name, command) in [
            (b"::raw\xff".as_slice(), "::E1"),
            (b"::raw\xfe".as_slice(), "::E2"),
        ] {
            let selected = vm.activate_namespace_operand(name).unwrap();
            vm.push_ns_token(&name[2..], selected);
            let created = ns_ensemble_create(
                &mut vm,
                &[Value::string("-command"), Value::string(command)],
            );
            assert_eq!(created.code, Code::Ok, "{release}");
            vm.pop_ns();
            assert_ne!(vm.current_ns_id(), selected);
            let query = ns_ensemble_configure(
                &mut vm,
                &[Value::string(command), Value::string("-namespace")],
            );
            assert_eq!(query.code, Code::Ok, "{release}");
            let snapshot = ns_ensemble_configure(&mut vm, &[Value::string(command)]);
            assert_eq!(snapshot.code, Code::Ok, "{release}");
            let protocol = vm.actual_native_string_protocol().unwrap();
            let entries = snapshot
                .result
                .native_object_list_elements(protocol)
                .unwrap();
            let configured = entries
                .as_chunks::<2>()
                .0
                .iter()
                .find(|pair| pair[0].resident_string_bytes().unwrap().as_ref() == b"-namespace")
                .unwrap()[1]
                .clone();
            for result in [query.result, configured] {
                assert_eq!(result.resident_string_bytes().unwrap().as_ref(), name);
                assert_eq!(
                    result.native_namespace_name_cache().is_some(),
                    matches!(release, "tcl9.0" | "tcl9.1"),
                );
                if let Some(cache) = result.native_namespace_name_cache() {
                    assert!(
                        cache
                            .namespace()
                            .unwrap()
                            .same_token(&vm.native_namespace_name_token(selected).unwrap())
                    );
                    assert!(cache.reference().is_none());
                }
                assert_eq!(vm.namespace_object_lookup(&result).unwrap(), Some(selected));
            }
        }
    }
}

#[test]
fn configuration_namespace_result_retains_bytes_after_retirement_and_refuses_dead_owner() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    let selected = vm.activate_namespace_operand(b"::held").unwrap();
    vm.push_ns_token("held", selected);
    let created = ns_ensemble_create(&mut vm, &[Value::string("-command"), Value::string("::E")]);
    assert_eq!(created.code, Code::Ok);
    vm.pop_ns();
    let (_, crate::command::Command::Ensemble(token)) = vm
        .lookup_command_bytes_checked(vm.current_ns_id(), b"::E")
        .unwrap()
        .unwrap()
    else {
        panic!("actual ensemble token");
    };
    let def = token.config();
    let live = ensemble_option_value(
        &mut vm,
        &def,
        tcl_cmd_core::ensemble::ConfigOption::Namespace,
        NativeEnsembleNamespaceResultStage::Option,
    )
    .unwrap();
    let cache = live.native_namespace_name_cache().unwrap();
    assert!(vm.delete_namespace_operand(b"::held").unwrap());
    assert_eq!(
        cache.namespace().unwrap().lifecycle(),
        tcl_syntax::native_namespace_name::NativeNamespaceLifecycle::Dead,
    );
    assert_eq!(live.resident_string_bytes().unwrap().as_ref(), b"::held");
    assert!(
        ensemble_option_value(
            &mut vm,
            &def,
            tcl_cmd_core::ensemble::ConfigOption::Namespace,
            NativeEnsembleNamespaceResultStage::Option,
        )
        .is_err()
    );
}

#[test]
fn configuration_namespace_results_preserve_distinct_owners_with_equal_name_bytes() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    let mut results = Vec::new();
    let mut namespaces = Vec::new();
    for (segments, command) in [
        ([b"a:".as_slice(), b"b".as_slice()], "::E1"),
        ([b"a".as_slice(), b":b".as_slice()], "::E2"),
    ] {
        let path = tcl_core_types::ByteNamespacePath::from_segments(segments);
        let selected = vm.install_constructed_namespace_test_fixture(path);
        let name = tcl_runtime_api::Namespaces::name_bytes(&vm, selected);
        vm.push_ns_token(&name[2..], selected);
        let created = ns_ensemble_create(
            &mut vm,
            &[Value::string("-command"), Value::string(command)],
        );
        assert_eq!(created.code, Code::Ok);
        vm.pop_ns();
        let queried = ns_ensemble_configure(
            &mut vm,
            &[Value::string(command), Value::string("-namespace")],
        );
        assert_eq!(queried.code, Code::Ok);
        let result = queried.result;
        assert!(
            result
                .native_namespace_name_cache()
                .unwrap()
                .namespace()
                .unwrap()
                .same_token(&vm.native_namespace_name_token(selected).unwrap())
        );
        assert_eq!(vm.namespace_object_lookup(&result).unwrap(), Some(selected));
        results.push(result);
        namespaces.push(selected);
    }
    assert_ne!(namespaces[0], namespaces[1]);
    assert_eq!(
        results[0].resident_string_bytes(),
        results[1].resident_string_bytes()
    );
    assert!(
        !results[0]
            .native_namespace_name_cache()
            .unwrap()
            .namespace()
            .unwrap()
            .same_token(
                results[1]
                    .native_namespace_name_cache()
                    .unwrap()
                    .namespace()
                    .unwrap()
            )
    );
}

#[test]
fn configuration_namespace_result_refuses_component_with_no_native_c_spelling() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    let actual = vm.activate_namespace_operand(b"::a::b").unwrap();
    let impossible = tcl_core_types::ByteNamespacePath::from_segments([b"a::b".as_slice()]);
    let constructed = vm.install_constructed_namespace_test_fixture(impossible);
    assert_ne!(actual, constructed);
    assert_eq!(
        tcl_runtime_api::Namespaces::name_bytes(&vm, actual),
        tcl_runtime_api::Namespaces::name_bytes(&vm, constructed),
    );
    let result = vm
        .original_ensemble_namespace_result(actual, NativeEnsembleNamespaceResultStage::Snapshot)
        .unwrap();
    assert!(
        result
            .native_namespace_name_cache()
            .unwrap()
            .namespace()
            .unwrap()
            .same_token(&vm.native_namespace_name_token(actual).unwrap())
    );
    assert!(
        vm.original_ensemble_namespace_result(
            constructed,
            NativeEnsembleNamespaceResultStage::Snapshot,
        )
        .is_err()
    );
}

#[test]
fn map_queries_keep_the_same_original_and_do_not_own_retired_configuration() {
    for version in [
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let mut vm = crate::native_fixture::core(
            tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
        );
        let args: Vec<_> = ["-command", "::E", "-map", "a ::target"]
            .into_iter()
            .map(Value::string)
            .collect();
        let created = ns_ensemble_create(&mut vm, &args);
        assert_eq!(created.code, Code::Ok, "{version:?}");
        let (_, crate::command::Command::Ensemble(token)) = vm
            .lookup_command_bytes_checked(vm.current_ns_id(), b"::E")
            .unwrap()
            .unwrap()
        else {
            panic!("actual ensemble token");
        };
        let query = token.config();
        let references = args[3].native_object_reference_count();
        let another = query.clone();
        assert_eq!(args[3].native_object_reference_count(), references);
        let original = ensemble_option_value(
            &mut vm,
            &query,
            tcl_cmd_core::ensemble::ConfigOption::Map,
            NativeEnsembleNamespaceResultStage::Snapshot,
        )
        .unwrap();
        assert_eq!(
            original.native_object_identity(),
            args[3].native_object_identity()
        );
        drop(original);
        let deleted = vm.invoke_command("rename", &[Value::string("::E"), Value::string("")]);
        assert_eq!(deleted.code, Code::Ok, "{version:?}");
        assert!(
            query
                .originals
                .map
                .as_ref()
                .unwrap()
                .inspect(|_| ())
                .is_none()
        );
        assert!(
            another
                .originals
                .map
                .as_ref()
                .unwrap()
                .inspect(|_| ())
                .is_none()
        );
        assert_eq!(args[3].native_object_reference_count(), references - 1);
    }
}

#[test]
fn original_ensemble_options_use_actual_release_tables() {
    for version in [
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let mut vm = Vm::new();
        vm.set_dialect_profile(
            tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
        );
        assert_eq!(
            ns_ensemble_create(&mut vm, &[Value::string("-command"), Value::string("::E")]).code,
            Code::Ok
        );
        let query = ns_ensemble_configure(
            &mut vm,
            &[Value::string("::E"), Value::string("-parameters")],
        );
        assert_eq!(
            query.code,
            if version == tcl_dialect::TclVersion::V8_5 {
                Code::Error
            } else {
                Code::Ok
            }
        );
        let query = ns_ensemble_configure(&mut vm, &[Value::string("::E"), Value::string("-p")]);
        assert_eq!(
            query.code,
            if version == tcl_dialect::TclVersion::V8_5 {
                Code::Ok
            } else {
                Code::Error
            }
        );
        let query = ns_ensemble_configure(&mut vm, &[Value::string("::E")]);
        let recipe = vm
            .native_invocation_dialect()
            .native_string_materialization(None)
            .unwrap();
        let entries = query
            .result
            .native_object_list_elements(recipe.protocol())
            .unwrap();
        assert_eq!(
            entries.len(),
            if version == tcl_dialect::TclVersion::V8_5 {
                10
            } else {
                12
            }
        );
    }
}

#[test]
fn counted_map_roots_and_native_cstring_collision_owners_are_independent() {
    for version in [
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let mut vm = crate::native_fixture::core(
            tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
        );
        let protocol = vm
            .native_invocation_dialect()
            .native_string_materialization(None)
            .unwrap()
            .protocol();
        let left = Value::from_native_string_bytes(b"m\0left".as_slice());
        let right = Value::from_native_string_bytes(b"m\0right".as_slice());
        let first = Value::native_list_constructor(
            vec![
                Value::string("::set"),
                Value::string("selected"),
                Value::string("FIRST"),
            ],
            protocol,
        );
        let second = Value::native_list_constructor(
            vec![
                Value::string("::set"),
                Value::string("selected"),
                Value::string("SECOND"),
            ],
            protocol,
        );
        let mapping = Value::native_dictionary_constructor(
            vec![
                (left.clone(), first.clone()),
                (right.clone(), second.clone()),
            ],
            None,
            protocol,
        )
        .unwrap();
        let args = [
            Value::string("-command"),
            Value::string("::E"),
            Value::string("-map"),
            mapping.clone(),
        ];
        assert_eq!(ns_ensemble_create(&mut vm, &args).code, Code::Ok);
        let result = vm.invoke_command("::E", &[Value::string("m")]);
        assert_eq!(result.code, Code::Ok);
        assert_eq!(result.result.to_str().as_ref(), "SECOND");
        assert_eq!(first.native_object_reference_count(), 3);
        assert_eq!(second.native_object_reference_count(), 3);
        let original =
            ns_ensemble_configure(&mut vm, &[Value::string("::E"), Value::string("-map")]);
        assert_eq!(
            original.result.native_object_identity(),
            mapping.native_object_identity()
        );
        assert_eq!(
            original
                .result
                .with_cached_dictionary_representation(|pairs, _| pairs.len()),
            Some(2)
        );
        drop(original);
        let subcommands =
            Value::native_list_constructor(vec![left.clone(), right.clone()], protocol);
        assert_eq!(
            ns_ensemble_configure(
                &mut vm,
                &[
                    Value::string("::E"),
                    Value::string("-subcommands"),
                    subcommands
                ]
            )
            .code,
            Code::Ok
        );
        let result = vm.invoke_command("::E", &[Value::string("m")]);
        assert_eq!(result.result.to_str().as_ref(), "FIRST");
        assert_eq!(first.native_object_reference_count(), 4);
        assert_eq!(second.native_object_reference_count(), 2);
        drop(args);
        drop(mapping);
        drop(vm);
        assert_eq!(first.native_object_reference_count(), 2);
        assert_eq!(second.native_object_reference_count(), 1);
        assert!(first.native_object_is_live());
    }
}

#[test]
fn explicit_subcommands_restrict_the_actual_map() {
    for version in [
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let mut vm = crate::native_fixture::core(
            tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
        );
        let args: Vec<_> = [
            "-command",
            "::E",
            "-map",
            "a ::target b ::target",
            "-subcommands",
            "a",
        ]
        .into_iter()
        .map(Value::string)
        .collect();
        assert_eq!(ns_ensemble_create(&mut vm, &args).code, Code::Ok);
        let result = vm.invoke_command("::E", &[Value::string("b")]);
        assert_eq!(result.code, Code::Error);
        assert_eq!(
            result.result.to_str().as_ref(),
            "unknown or ambiguous subcommand \"b\": must be a"
        );
    }
}
