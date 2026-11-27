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
use std::cell::RefCell;
use tcl_core_types::VarId;
use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;

fn actual(version: &str) -> Vm {
    let profile = tcl_registry::model::ingress::resolve_environment(version).unit_profile();
    let mut vm = Vm::new();
    assert!(vm.set_native_engine_profile(profile));
    vm.set_dialect_profile(profile);
    vm
}

#[test]
fn confined_quiet_unset_preserves_lookup_misses_and_refuses_foreign_receivers() {
    for version in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(version);
        vm.set_var("::seed", Value::string("old")).unwrap();
        vm.set_array_elem("::arr", "k", Value::string("held"))
            .unwrap();
        vm.push_call_frame(Some("confined".into()), Vec::new());
        vm.set_stores_confined(true);
        for name in ["::missing_ns::x", "::missing_ns::a(k)", "absent"] {
            vm.unset_original_c_variable(&Value::string(name), false)
                .unwrap_or_else(|error| panic!("{version} {name}: {error:?}"));
        }
        vm.unset_original_c_variable_parts(
            &Value::string("::missing_ns::a"),
            &Value::string("k"),
            false,
        )
        .unwrap();
        vm.set_var("local", Value::string("local")).unwrap();
        vm.unset_original_c_variable(&Value::string("local"), false)
            .unwrap();
        vm.set_array_elem("local_array", "k", Value::string("local"))
            .unwrap();
        vm.unset_original_c_variable_parts(
            &Value::string("local_array"),
            &Value::string("absent"),
            false,
        )
        .unwrap();
        vm.unset_original_c_variable_parts(
            &Value::string("local_array"),
            &Value::string("k"),
            false,
        )
        .unwrap();
        for complain in [false, true] {
            let error = vm
                .unset_original_c_variable(&Value::string("::seed"), complain)
                .unwrap_err();
            assert_eq!(
                error.result.string_bytes().as_ref(),
                b"can't unset \"::seed\": stores are confined to the activation",
                "{version}"
            );
            let error = vm
                .unset_original_c_variable_parts(
                    &Value::string("::arr"),
                    &Value::string("k"),
                    complain,
                )
                .unwrap_err();
            assert_eq!(
                error.result.string_bytes().as_ref(),
                b"can't unset \"::arr\": stores are confined to the activation",
                "{version}"
            );
        }
        assert_eq!(vm.get_var("::seed").unwrap().to_str().as_ref(), "old");
        assert_eq!(
            vm.get_array_elem("::arr", "k").unwrap().to_str().as_ref(),
            "held"
        );
        assert!(
            vm.var_binding_from_bytes(b"::missing_ns::x", vm.current_level())
                .is_none()
        );
    }
}

#[test]
fn confined_generic_capture_checks_name_link_before_following_its_local_target() {
    let mut vm = actual("tcl9.0");
    vm.push_call_frame(Some("confined".into()), Vec::new());
    vm.set_var("target", Value::string("old")).unwrap();
    vm.set_array_elem("target_array", "k", Value::string("held"))
        .unwrap();
    let level = vm.current_level();
    for (alias, target, element) in [
        ("alias", "target", None),
        ("array_alias", "target_array", Some(b"k".as_slice())),
    ] {
        let binding = vm.var_binding_from_bytes(alias.as_bytes(), level).unwrap();
        let target_binding = vm.var_binding_from_bytes(target.as_bytes(), level).unwrap();
        vm.bind_new_var(
            &binding,
            super::super::Local::NameLink(crate::vars::NameLink {
                original_target: None,
                binding: target_binding,
                element: None,
            }),
        )
        .unwrap();
        vm.set_stores_confined(true);
        assert!(
            vm.capture_update_cell(alias.as_bytes(), element).is_err(),
            "{alias}"
        );
        assert!(
            vm.capture_update_cell(target.as_bytes(), element).is_ok(),
            "{target}"
        );
    }
    assert_eq!(vm.get_var("target").unwrap().to_str().as_ref(), "old");
    assert_eq!(
        vm.get_array_elem("target_array", "k")
            .unwrap()
            .to_str()
            .as_ref(),
        "held"
    );
}

#[test]
fn confined_array_store_retains_the_first_host_refusal() {
    let mut vm = actual("tcl9.0");
    vm.push_call_frame(Some("confined".into()), Vec::new());
    vm.set_stores_confined(true);
    let prior = vm.refuse_host_command("terminal array write refusal".into());
    let error = vm
        .set_array_elem_bytes_from(vm.current_level(), b"::fresh", b"k", Value::int(1), true)
        .unwrap_err();
    assert_eq!(error.result.string_bytes(), prior.result.string_bytes());
    let error = vm
        .store_elem_result_bytes(b"::fresh", b"k", Value::int(1))
        .unwrap_err();
    assert_eq!(error.result.string_bytes(), prior.result.string_bytes());
    let binding = vm
        .var_binding_from_bytes(b"::fresh", vm.current_level())
        .unwrap();
    assert!(vm.raw_variable_binding(&binding).is_none());
}

#[test]
fn original_unset_parts_keep_part1_cache_and_quiet_missing_lookup() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        vm.set_array_elem_bytes(b"a(k)", b"index", Value::string("ORIGINAL"))
            .unwrap();
        vm.set_array_elem_bytes(b"a", b"k", Value::string("SIBLING"))
            .unwrap();
        let root = Value::new_native_string_bytes(b"a(k)".as_slice());
        let index = Value::new_native_string_bytes(b"index".as_slice());
        vm.unset_original_c_variable_parts(&root, &index, true)
            .unwrap();
        assert!(
            vm.get_array_elem_bytes_from(vm.current_level(), b"a(k)", b"index")
                .is_none(),
            "{engine}"
        );
        assert_eq!(
            vm.get_array_elem_bytes_from(vm.current_level(), b"a", b"k")
                .unwrap()
                .to_str()
                .as_ref(),
            "SIBLING"
        );
        assert!(
            !root
                .with_native_parsed_variable(|cache| cache.array.is_some())
                .unwrap_or(false)
        );
        let missing = Value::new_native_string_bytes(b"::absent::x".as_slice());
        assert!(
            vm.unset_original_c_variable(&missing, false).is_ok(),
            "{engine}"
        );
        assert!(vm.refused_completion().is_none());
        assert!(
            vm.unset_original_c_variable(&missing, true).is_err(),
            "{engine}"
        );
    }
}

thread_local! {
    static ALIAS_SIMPLE_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    static ALIAS_CANONICAL_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

thread_local! {
    static GLOBAL_CACHE_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[test]
fn namespace_original_links_preserve_array_parts_and_reject_defined_aliases() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = Vm::with_native_core(
            Box::new(Vec::<u8>::new()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let original = Value::new_native_string_bytes(b"arr(k)".as_slice());
        let value = Value::new_native_string_bytes(b"VALUE".as_slice());
        vm.store_original_named_variable(&original, value.clone())
            .unwrap();
        vm.push_call_frame(Some("p".into()), Vec::new());
        vm.install_compiled_local_slots(&[tcl_core_types::NameBytes::from(b"alias".as_slice())])
            .unwrap();
        vm.link_compiled_namespace_original(0, tcl_core_types::ROOT_NS, &original, false)
            .unwrap();
        assert!(
            vm.read_compiled_variable_result(0, None)
                .unwrap()
                .is_same_object(&value)
        );
        assert!(
            original
                .with_native_parsed_variable(|cache| cache.array.is_some())
                .unwrap()
        );
        vm.pop_call_frame();
        vm.push_call_frame(Some("p".into()), Vec::new());
        vm.install_compiled_local_slots(&[tcl_core_types::NameBytes::from(b"alias".as_slice())])
            .unwrap();
        vm.bind_compiled_formal_slot(0, Value::string("LOCAL"))
            .unwrap();
        let failure = vm
            .link_compiled_namespace_original(0, tcl_core_types::ROOT_NS, &original, true)
            .unwrap_err();
        assert_eq!(failure.code, tcl_runtime_api::Code::Error);
        assert_eq!(
            vm.read_compiled_variable_result(0, None)
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"LOCAL"
        );
        assert!(vm.refused_completion().is_none());
        vm.pop_call_frame();
    }
}

thread_local! {
    static CACHE_LOOKUP_CASE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static CACHE_LOOKUP_CALLBACKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static CACHE_LOOKUP_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, byte| {
        use std::fmt::Write;
        write!(output, "{byte:02x}").expect("writing into a String");
        output
    })
}
fn observe(version: &str, case: &str, phase: &str, original: &Value) -> String {
    let snapshot = original.native_object_snapshot();
    let kind = match snapshot.cache {
        Cache::None => "none",
        Cache::Numeric(_) => "int",
        Cache::List { .. } => "list",
        Cache::ByteArray { .. } => "bytearray",
        Cache::ParsedVariableName { .. } => "parsedVarName",
        ref other => panic!("unexpected actual original cache: {other:?}"),
    };
    format!(
        "{version}|{case}|{phase}|{kind}|{}|{}|{}|{}",
        original.native_object_reference_count(),
        usize::from(original.native_primary_has_free_hook()),
        usize::from(snapshot.resident.is_some()),
        snapshot.resident.as_deref().map_or_else(String::new, hex)
    )
}

use tcl_runtime_api::native_variable_trace::{
    NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
};
struct ElementCallbackWindows {
    version: &'static str,
    key_k: crate::value::NativeObjectLifetimeLease,
    key_j: crate::value::NativeObjectLifetimeLease,
    old_k: VarId,
    old_j: VarId,
    object_table: bool,
    rows: Rc<RefCell<Vec<String>>>,
    sequence: Rc<RefCell<usize>>,
}
impl ElementCallbackWindows {
    fn record(&self, vm: &Vm, phase: &str) {
        let (dead_k, defined_k, refs_k) = vm
            .var_arena
            .observe_native_element(self.old_k, self.object_table);
        let (dead_j, defined_j, refs_j) = vm
            .var_arena
            .observe_native_element(self.old_j, self.object_table);
        let current_k = vm
            .resolve_var_from_bytes(b"arr(k)", 0)
            .and_then(|resolved| resolved.id)
            .is_some();
        let current_j = vm
            .resolve_var_from_bytes(b"arr(j)", 0)
            .and_then(|resolved| resolved.id)
            .is_some();
        let key_k = self.key_k.value().native_object_reference_count();
        let key_j = self.key_j.value().native_object_reference_count();
        self.rows.borrow_mut().push(format!("{}|window|{phase}|{key_k}|{key_j}|{dead_k}|{dead_j}|{defined_k}|{defined_j}|{refs_k}|{refs_j}|{}|{}", self.version, i32::from(current_k), i32::from(current_j)));
    }
}
impl NativeVariableObserver<Vm> for ElementCallbackWindows {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        vm: &mut Vm,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        let phase = if access.name2.is_empty() {
            "root".to_owned()
        } else {
            *self.sequence.borrow_mut() += 1;
            format!(
                "element{}-{}",
                *self.sequence.borrow(),
                std::str::from_utf8(access.name2).unwrap()
            )
        };
        self.record(vm, &phase);
        // Raw C84 flags do not donate a value or an existence result.
        if !access.name2.is_empty() {
            assert!(!vm.exists_var_bytes_from(if access.name2 == b"k" { b"ak" } else { b"aj" }, 0));
        }
        Ok(())
    }
}

#[test]
fn cpp_table_and_var_roles_match_all_25_original_callback_windows() {
    // Native proof: naming.variable.original-element-unset-callback-roles
    // docs/design/analysis/name-resolution-proofs/variable-original-element-unset-callback-roles.md
    use std::cell::RefCell;
    let rows = Rc::new(RefCell::new(Vec::new()));
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut vm = actual(environment);
        let root = Value::new_native_string_bytes(b"arr".as_slice());
        let key_k = Value::new_native_string_bytes(b"k".as_slice());
        let key_j = Value::new_native_string_bytes(b"j".as_slice());
        let object_table = vm
            .native_c_variable_name_protocol()
            .unwrap()
            .element_table_retains_original();
        let (_, _, _, selected) = vm
            .prepare_native_original_variable(&root, NativeVariableNameLookupPurpose::Write)
            .unwrap();
        let array = selected.id.unwrap();
        let mut ids = Vec::new();
        for key in [&key_k, &key_j] {
            let id = vm
                .ensure_target_var_at_binding(&selected.binding, Some(&key.string_bytes()))
                .unwrap();
            vm.var_arena.replace_state(
                id,
                crate::vars::VarState::Scalar(Value::new_native_string_bytes(b"ONE".as_slice())),
            );
            if object_table {
                assert!(vm.var_arena.retain_original_array_key(
                    array,
                    &tcl_core_types::NameBytes::from(key.string_bytes().as_ref()),
                    key.clone()
                ));
            }
            ids.push(id);
        }
        let name_k = Value::new_native_string_bytes(b"arr(k)".as_slice());
        let name_j = Value::new_native_string_bytes(b"arr(j)".as_slice());
        vm.link_upvar_original(
            0,
            &name_k,
            &Value::new_native_string_bytes(b"ak".as_slice()),
        )
        .unwrap();
        vm.link_upvar_original(
            0,
            &name_j,
            &Value::new_native_string_bytes(b"aj".as_slice()),
        )
        .unwrap();
        let windows = Rc::new(ElementCallbackWindows {
            version,
            key_k: key_k.native_lifetime_lease(),
            key_j: key_j.native_lifetime_lease(),
            old_k: ids[0],
            old_j: ids[1],
            object_table,
            rows: Rc::clone(&rows),
            sequence: Rc::new(RefCell::new(0)),
        });
        for name in [&root, &name_k, &name_j] {
            vm.add_native_variable_observer(
                name,
                &[NativeVariableTraceOperation::Unset],
                windows.clone(),
            )
            .unwrap();
        }
        windows.record(&vm, "before");
        assert!(vm.unset_var_bytes(b"arr"));
        windows.record(&vm, "after");
    }
    let expected = include_str!(
        "../../../../tcl-syntax/tests/data/native_variable_name/element_alias_callbacks.txt"
    )
    .lines()
    .filter(|line| line.contains("|window|"))
    .collect::<Vec<_>>();
    assert_eq!(rows.borrow().len(), 25);
    assert_eq!(*rows.borrow(), expected);
}

#[test]
fn original_upvar_and_unset_retain_actual_target_cache_and_entry() {
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(environment);
        let target = Value::new_native_string_bytes(b"target".as_slice());
        let local = Value::new_native_string_bytes(b"alias".as_slice());
        vm.link_upvar_original(0, &target, &local).unwrap();
        assert!(
            target
                .with_native_parsed_variable(|cache| cache.array.is_none())
                .unwrap()
        );
        let binding = vm.var_binding_from_bytes(b"target", 0).unwrap();
        let selected = vm.raw_variable_binding(&binding).unwrap();
        assert!(vm.var_arena.has_link_refs(selected));
        vm.store_original_named_variable(&local, Value::int(7))
            .unwrap();
        vm.unset_original_named_variable(&target, true).unwrap();
        assert_eq!(vm.raw_variable_binding(&binding), Some(selected));
        assert!(
            target
                .with_native_parsed_variable(|cache| cache.array.is_none())
                .unwrap()
        );
        vm.store_original_named_variable(&local, Value::int(8))
            .unwrap();
        assert_eq!(
            vm.read_original_named_variable(&target)
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"8"
        );
    }
}

#[test]
fn search_free_slots_match_all_20_actual_c_windows() {
    // Native proof: naming.variable.array-search-free-slot-retirement
    // docs/design/analysis/name-resolution-proofs/variable-array-search-free-slot-retirement.md
    use tcl_cmd_core::native_array_search::NativeArraySearchBackend;
    let mut rows = Vec::new();
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut vm = actual(environment);
        let protocol = vm.array_search_protocol().unwrap();
        let name = Value::new_native_string_bytes(b"a(k)".as_slice());
        vm.store_original_named_variable(&name, Value::new_native_string_bytes(b"v".as_slice()))
            .unwrap();
        let array = Value::new_native_string_bytes(b"a".as_slice());
        let handle = tcl_cmd_core::native_array_search::dispatch(
            &mut vm,
            "startsearch",
            std::slice::from_ref(&array),
            None,
        )
        .unwrap()
        .unwrap();
        let result = tcl_cmd_core::native_array_search::dispatch(
            &mut vm,
            "anymore",
            &[array, handle.clone()],
            None,
        )
        .unwrap()
        .unwrap();
        drop(result);
        let observe = |phase: &str, original: &Value| {
            format!(
                "{version}|{phase}|{}|{}|{}",
                if original
                    .native_array_search_cache_in(protocol)
                    .unwrap()
                    .is_some()
                {
                    "array search"
                } else if matches!(
                    original.native_object_snapshot().cache,
                    Cache::String { .. }
                ) {
                    "string"
                } else {
                    "none"
                },
                usize::from(original.native_primary_has_free_hook()),
                usize::from(original.resident_string_bytes().is_some())
            )
        };
        rows.push(observe("converted", &handle));
        let copy = handle.duplicate_native_object_in(vm.actual_native_string_protocol().unwrap());
        rows.push(observe("duplicate", &copy));
        assert!(vm.read_original_named_variable(&handle).is_err());
        rows.push(observe("missing", &handle));
        assert!(vm.read_original_named_variable(&copy).is_err());
        rows.push(observe("duplicate-missing", &copy));
    }
    let expected = include_str!(
        "../../../../tcl-syntax/tests/data/native_variable_name/array_search_free_slot.txt"
    )
    .lines()
    .filter(|line| !line.is_empty())
    .collect::<Vec<_>>();
    assert_eq!(rows.len(), 20);
    assert_eq!(rows, expected);
}

#[test]
fn original_parsed_headers_match_all_150_actual_c_windows() {
    // Native proof: naming.variable.original-scalar-parsed-header
    // docs/design/analysis/name-resolution-proofs/variable-original-scalar-parsed-header.md
    // Native proof: naming.variable.original-counted-array-parsed-header
    // docs/design/analysis/name-resolution-proofs/variable-original-counted-array-parsed-header.md
    let mut rows = Vec::new();
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut vm = actual(environment);
        for case in ["string", "int", "list", "bytearray", "array"] {
            let original = match case {
                "string" => Value::new_native_string_bytes(b"missing".as_slice()),
                "int" => Value::int(42),
                "list" => Value::list(vec![Value::new_native_string_bytes(b"missing".as_slice())]),
                "bytearray" => Value::byte_array(b"missing".to_vec()),
                "array" => Value::new_native_string_bytes(b"arr(k\0z)".as_slice()),
                _ => unreachable!(),
            };
            rows.push(observe(version, case, "before", &original));
            assert!(vm.read_original_named_variable(&original).is_err());
            rows.push(observe(version, case, "missing", &original));
            let value = Value::new_native_string_bytes(b"VALUE".as_slice());
            assert!(
                vm.store_original_named_variable(&original, value.clone())
                    .is_ok()
            );
            rows.push(observe(version, case, "stored", &original));
            assert!(
                vm.read_original_named_variable(&original)
                    .unwrap()
                    .is_same_object(&value)
            );
            rows.push(observe(version, case, "read", &original));
            let copy =
                original.duplicate_native_object_in(vm.actual_native_string_protocol().unwrap());
            rows.push(observe(version, case, "duplicate", &copy));
            drop(copy);
            for name in [b"missing".as_slice(), b"42".as_slice(), b"arr".as_slice()] {
                vm.unset_var_bytes(name);
            }
            assert!(vm.read_original_named_variable(&original).is_err());
            rows.push(observe(version, case, "after-unset-missing", &original));
        }
    }
    let expected =
        include_str!("../../../../tcl-syntax/tests/data/native_variable_name/parsed_headers.txt")
            .lines()
            .collect::<Vec<_>>();
    assert_eq!(rows.len(), 150);
    assert_eq!(rows, expected);
}

#[test]
fn scalar_alias_entries_and_array_parts_match_all_95_native_windows() {
    // Native proof: naming.variable.original-scalar-alias-entry-lifetime
    // docs/design/analysis/name-resolution-proofs/variable-original-scalar-alias-entry-lifetime.md
    // Native proof: naming.variable.original-parsed-array-child-references
    // docs/design/analysis/name-resolution-proofs/variable-original-parsed-array-child-references.md
    let mut rows = Vec::new();
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut vm = actual(environment);
        for mode in 0..2 {
            let name = Value::new_native_string_bytes(b"k".as_slice());
            let value = Value::new_native_string_bytes(b"ONE".as_slice());
            vm.store_original_named_variable(&name, value.clone())
                .unwrap();
            let binding = vm.var_binding_from_bytes(b"k", 0).unwrap();
            let before = *vm.var_table_mut(binding.owner).unwrap().get(b"k").unwrap();
            let observe = |vm: &mut Vm, phase: &str, fresh: Option<&Value>| {
                let table = vm.var_table_mut(binding.owner).unwrap();
                let found = table.get(b"k").copied();
                assert!(matches!(
                    name.native_object_snapshot().cache,
                    Cache::ParsedVariableName { .. }
                ));
                format!(
                    "{version}|alias|{mode}|{phase}|parsedVarName|{}|{}|{}|{}|{}",
                    name.native_object_reference_count(),
                    fresh.map_or(-1, |fresh| i64::try_from(
                        fresh.native_object_reference_count()
                    )
                    .unwrap()),
                    usize::from(found.is_some()),
                    usize::from(found == Some(before)),
                    usize::from(table.native_key_is(b"k", &name))
                )
            };
            rows.push(observe(&mut vm, "created", None));
            vm.push_call_frame(Some("run".into()), Vec::new());
            vm.link_upvar_bytes(0, b"k", b"a").unwrap();
            rows.push(observe(&mut vm, "linked", None));
            assert!(vm.unset_var_bytes(b"::k"));
            rows.push(observe(&mut vm, "unset-target", None));
            let fresh = Value::new_native_string_bytes(b"k".as_slice());
            let written = Value::new_native_string_bytes(if mode == 0 {
                b"::k".as_slice()
            } else {
                b"a".as_slice()
            });
            vm.store_original_named_variable(&written, value.clone())
                .unwrap();
            rows.push(observe(&mut vm, "recreated", Some(&fresh)));
            assert!(vm.unset_var_bytes(b"::k"));
            rows.push(observe(&mut vm, "unset-recreated-target", Some(&fresh)));
            assert!(!vm.unset_var_bytes(b"a"));
            rows.push(observe(&mut vm, "unset-alias", Some(&fresh)));
            vm.pop_call_frame();
            rows.push(observe(&mut vm, "frame-popped", Some(&fresh)));
            assert!(vm.read_original_named_variable(&fresh).is_err());
            rows.push(observe(&mut vm, "fresh-read", Some(&fresh)));
        }
        let name = Value::new_native_string_bytes(b"arr(k)".as_slice());
        vm.store_original_named_variable(
            &name,
            Value::new_native_string_bytes(b"VALUE".as_slice()),
        )
        .unwrap();
        let observe = |phase: &str| {
            name.with_native_parsed_variable(|cache| {
                let (root, element) = cache.array.as_ref().unwrap();
                let element_refs = match element {
                    NativeParsedVariableElement::Object(element) => {
                        i64::try_from(element.native_object_reference_count()).unwrap()
                    }
                    NativeParsedVariableElement::Bytes(_) => -1,
                };
                format!(
                    "{version}|parts|{phase}|{}|{element_refs}",
                    root.native_object_reference_count()
                )
            })
            .unwrap()
        };
        rows.push(observe("stored"));
        let copy = name.duplicate_native_object_in(vm.actual_native_string_protocol().unwrap());
        rows.push(observe("duplicated"));
        drop(copy);
        assert!(vm.unset_var_bytes(b"arr"));
        rows.push(observe("unset"));
    }
    let expected = include_str!(
        "../../../../tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.txt"
    )
    .lines()
    .collect::<Vec<_>>();
    assert_eq!(rows.len(), 95);
    assert_eq!(rows, expected);
}
