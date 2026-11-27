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

fn core(engine: &str) -> Vm {
    Vm::with_native_core(
        Box::new(std::io::sink()),
        Rc::new(crate::host_native::NativeHost::new()),
        tcl_registry::model::resolve_environment(engine).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: b"original-library".to_vec(),
            default_library: Some(b"original-default".to_vec()),
        },
    )
    .unwrap()
}

#[test]
fn child_core_publication_uses_its_retained_constructor_inputs_and_engine() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = core(engine);
        vm.write_scalar_raw("tcl_pkgPath", Value::string("changed-parent"));
        vm.write_scalar_raw("auto_path", Value::string("changed-parent"));
        let id = vm
            .create_child_named_bytes(NameBytes::from(b"child"), false)
            .unwrap();
        vm.in_interp(id, |child| {
            let NativeBootstrapPurpose { protocol, inputs } = &child.bootstrap;
            assert_eq!(inputs.package_path, b"original-library");
            assert_eq!(
                inputs.default_library.as_deref(),
                Some(b"original-default".as_slice())
            );
            assert_eq!(
                child
                    .actual_native_invocation_dialect()
                    .native_bootstrap_protocol(),
                Some(*protocol),
            );
            assert!(
                child
                    .command_at_exact_slot(
                        &CommandSlot {
                            namespace: ROOT_NS,
                            simple: NameBytes::from(b"set"),
                        },
                        false
                    )
                    .is_some()
            );
            if protocol.names().is_jim084() {
                let world = child.name_world.borrow();
                assert!(world.jim_command_original_keys.contains_key(&CommandSlot {
                    namespace: ROOT_NS,
                    simple: NameBytes::from(b"set"),
                }));
            }
        });
    }
}

#[test]
fn jim_child_extension_library_has_its_own_original_command_key() {
    let mut vm = core("jim");
    let slot = CommandSlot {
        namespace: ROOT_NS,
        simple: NameBytes::from(b"namespace info"),
    };
    assert!(vm.command_at_exact_slot(&slot, false).is_none());
    let id = vm
        .create_child_named_bytes(NameBytes::from(b"child"), false)
        .unwrap();
    vm.in_interp(id, |child| {
        assert!(child.command_at_exact_slot(&slot, false).is_some());
        let world = child.name_world.borrow();
        let original = &world.jim_command_original_keys[&slot].original;
        assert_eq!(original.string_bytes().as_ref(), b"namespace info");
    });
    assert!(vm.command_at_exact_slot(&slot, false).is_none());
}

#[test]
fn child_constructor_refuses_a_foreign_engine_before_allocating_a_slot() {
    let mut vm = core("tcl9.0");
    let foreign = tcl_registry::model::resolve_environment("tcl8.6").unit_profile();
    assert!(vm.set_native_engine_profile(foreign));
    let count = vm.interps.len();
    assert!(matches!(
        vm.create_child_named_bytes(NameBytes::from(b"child"), false),
        Err(NativeCommandLookupUnavailable::ProtocolUnavailable),
    ));
    assert_eq!(vm.interps.len(), count);
    assert!(!vm.children.contains_key(b"child".as_slice()));
}

#[test]
fn jim_child_bootstrap_values_have_the_child_allocator_birth() {
    let mut vm = core("jim");
    let id = vm
        .create_child_named_bytes(NameBytes::from(b"child"), false)
        .unwrap();
    let parent = vm.native_jim_object_context().unwrap();
    vm.in_interp(id, |child| {
        let context = child.native_jim_object_context().unwrap();
        let value = child.get_var("tcl_interactive").unwrap();
        assert!(value.bind_native_jim_context(&context).is_ok());
        assert!(value.bind_native_jim_context(&parent).is_err());
        assert!(child.execution_refusal.is_none());
    });
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn jim_child_copies_only_existing_parent_globals_with_the_original_cstring_extent() {
    let mut vm = core("jim");
    let parent = vm.native_jim_object_context().unwrap();
    let original = {
        let _scope = native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
        let original = Value::new_native_string_bytes(b"parent\0tail".as_slice());
        vm.write_scalar_raw("argv", original.clone());
        vm.write_scalar_raw("argc", Value::new_native_string_bytes(b"9".as_slice()));
        vm.write_scalar_raw(
            "jim::argv0",
            Value::new_native_string_bytes(b"\xc3\xa9".as_slice()),
        );
        original
    };
    let count = original.native_object_reference_count();
    assert!(vm.get_var("jim::lineedit").is_none());
    let id = vm
        .create_child_named_bytes(NameBytes::from(b"child"), false)
        .unwrap();
    vm.in_interp(id, |child| {
        let context = child.native_jim_object_context().unwrap();
        for (name, expected) in [
            ("argv", b"parent".as_slice()),
            ("argc", b"9".as_slice()),
            ("jim::argv0", b"\xc3\xa9".as_slice()),
        ] {
            let value = child.get_var(name).unwrap();
            assert_eq!(value.resident_string_bytes().unwrap().as_ref(), expected);
            assert!(value.bind_native_jim_context(&context).is_ok());
            assert!(value.bind_native_jim_context(&parent).is_err());
            assert!(!value.is_same_object(&original));
        }
        assert!(child.get_var("jim::lineedit").is_none());
    });
    assert_eq!(original.native_object_reference_count(), count);
    assert_eq!(
        original.resident_string_bytes().unwrap().as_ref(),
        b"parent\0tail"
    );
    assert!(vm.execution_refusal.is_none());
}
