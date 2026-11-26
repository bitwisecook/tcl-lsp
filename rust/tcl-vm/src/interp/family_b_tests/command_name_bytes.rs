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

//! Command and namespace names as exact bytes: lookup, snapshots, enumeration and the backing catalogue.

use super::*;

#[test]
fn procedure_publication_keeps_retained_holder_and_release_reparse() {
    for version in tcl_dialect::TclVersion::ALL {
        let mut vm = release_vm(version);
        let original = vm.activate_namespace_operand(b"a:").unwrap();
        vm.push_ns_eval_token_frame(original, Vec::new());
        assert!(vm.native_procedure_holder_exists(b"p").unwrap());
        let (_, slot, activation) = vm.native_procedure_publication(b"p").unwrap();
        if version <= tcl_dialect::TclVersion::V8_5 {
            assert_ne!(activation, original);
            assert_eq!(vm.namespace_object_bytes(activation).as_bytes(), b"a");
        } else {
            assert_eq!(activation, original);
        }
        assert_eq!(slot.namespace, activation);
        assert_eq!(slot.simple.as_bytes(), b"p");
        vm.pop_ns();
        vm.pop_call_frame();
    }
}

#[test]
fn jim_actual_namespace_objects_are_in_native_snapshots() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
    let context = vm.activate_namespace_operand(b"n\0z").unwrap();
    vm.push_ns_eval_token_frame(context, Vec::new());
    let entry = vm.native_compilation_entry_for_namespace_token(context, true);
    assert_eq!(entry.current_namespace, u64::from(context.0));
    let row = entry
        .namespaces
        .iter()
        .find(|row| row.token == u64::from(context.0))
        .unwrap();
    assert_eq!(
        row.jim_namespace_object.as_ref().unwrap().as_bytes(),
        b"n\0z"
    );
    let namespace = vm
        .with_jim_current_namespace(|original| Ok(original.clone()))
        .unwrap();
    let scope = native_name_consumers::OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let canonical = vm
        .original_jim_canonical_namespace_object(
            &namespace,
            &Value::from_native_string_bytes(b"p".as_slice()),
            &|vm| scope.check(vm),
        )
        .unwrap();
    assert_eq!(
        canonical.resident_string_bytes().unwrap().as_ref(),
        b"n\0z::p"
    );
    scope.finish(&mut vm).unwrap();
    let (_, slot, body) = vm.native_procedure_publication(b"p").unwrap();
    assert_eq!(slot.namespace, ROOT_NS);
    assert_eq!(slot.simple.as_bytes(), b"n\0z::p");
    assert_eq!(body, context);
    vm.pop_ns();
    vm.pop_call_frame();
}

#[test]
fn native_command_byte_lookup_and_snapshot_keep_distinct_original_keys() {
    use tcl_core_types::NameBytes;
    for version in tcl_dialect::TclVersion::ALL {
        let mut vm = Vm::new();
        vm.set_runtime_version(version);
        let names = [
            b"k".as_slice(),
            b"k\xff",
            b"k\xc3\xbf",
            b"k\xc0\x80z",
            b"k\xed\xa0\x80",
            b"k\xef\xbf\xbd",
        ];
        let keys: Vec<_> = names
            .iter()
            .map(|name| {
                vm.register_command_in_slot(
                    CommandSlot {
                        namespace: ROOT_NS,
                        simple: NameBytes::from(*name),
                    },
                    Command::Native(Rc::new(TestNative)),
                )
            })
            .collect();
        let mut tokens = HashSet::new();
        for (name, key) in names.iter().zip(&keys) {
            let token = Namespaces::find_command_bytes(&vm, ROOT_NS, name)
                .expect("exact native command key remains addressable");
            assert!(tokens.insert(token));
            assert_eq!(vm.command_display_key_bytes(key).as_bytes(), *name);
            let expected = tcl_syntax::naming::root_unrooted_key_bytes(name);
            assert_eq!(Namespaces::command_name_bytes(&vm, token), Some(expected));
        }
        assert_eq!(
            Namespaces::find_command_bytes(&vm, ROOT_NS, b"k\0ignored"),
            Namespaces::find_command_bytes(&vm, ROOT_NS, b"k")
        );
        let pure = Value::byte_array(b"k\xff".to_vec());
        let materialised = vm
            .native_name_operand_bytes(&pure)
            .expect("C byte-array materialisation");
        assert_eq!(materialised.as_ref(), b"k\xc3\xbf");
        assert_eq!(
            Namespaces::find_command_bytes(&vm, ROOT_NS, &materialised),
            Namespaces::find_command_bytes(&vm, ROOT_NS, b"k\xc3\xbf")
        );
        let entry = vm
            .native_compilation_entry_for_namespace("", false)
            .unwrap();
        for name in names {
            assert!(
                entry
                    .commands
                    .iter()
                    .any(|row| row.slot.simple.as_bytes() == name)
            );
        }
    }
}

#[test]
fn owned_command_queries_release_the_name_world_before_native_reentry() {
    struct RepublishCommand {
        slot: CommandSlot,
        entered: Rc<Cell<bool>>,
    }
    impl NativeCommand for RepublishCommand {
        fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
            // Dispatch must not retain a metadata guard while an embedder runs.
            assert!(vm.name_world.try_borrow_mut().is_ok());
            vm.register_command_in_slot(self.slot.clone(), Command::Native(Rc::new(TestNative)));
            self.entered.set(true);
            ok(Value::empty())
        }
    }
    let mut vm = Vm::new();
    let slot = CommandSlot {
        namespace: ROOT_NS,
        simple: NameBytes::from(b"opaque\xff"),
    };
    let key = vm.register_command_in_slot(slot.clone(), Command::Native(Rc::new(TestNative)));
    let old = Namespaces::find_command_bytes(&vm, ROOT_NS, slot.simple.as_bytes()).unwrap();
    let retained_slot = vm.command_slot(&key).unwrap();
    let retained_display = vm.command_display_key_bytes(&key);
    let retained_generation = vm.visible_command_generation(&key).unwrap();
    let entered = Rc::new(Cell::new(false));
    vm.register_native_command(
        "republish",
        Rc::new(RepublishCommand {
            slot: slot.clone(),
            entered: Rc::clone(&entered),
        }),
    );
    assert_eq!(
        vm.try_invoke_command("republish", &[]).unwrap().code,
        Code::Ok
    );
    assert!(entered.get());
    assert_eq!(retained_slot, slot);
    assert_eq!(retained_display.as_bytes(), b"opaque\xff");
    assert_ne!(
        vm.visible_command_generation(&key),
        Some(retained_generation)
    );
    assert!(Namespaces::command_name_bytes(&vm, old).is_none());
    let replacement = Namespaces::find_command_bytes(&vm, ROOT_NS, b"opaque\xff").unwrap();
    assert_ne!(replacement, old);
    assert_eq!(
        Namespaces::command_name_bytes(&vm, replacement).unwrap(),
        b"::opaque\xff"
    );
}

#[test]
fn unrelated_command_traces_do_not_decode_opaque_command_names() {
    let mut vm = Vm::new();
    vm.register_native_command("watched", Rc::new(TestNative));
    assert_eq!(
        vm.add_cmd_trace(false, "watched", vec!["delete".into()], "unused".into())
            .code,
        Code::Ok
    );
    let slot = CommandSlot {
        namespace: ROOT_NS,
        simple: NameBytes::from(b"opaque\xff"),
    };
    let key = vm.register_command_in_slot(slot.clone(), Command::Native(Rc::new(TestNative)));
    let generation = vm.visible_command_generation(&key).unwrap();
    let replacement = vm.register_command_in_slot(slot, Command::Native(Rc::new(TestNative)));
    assert_eq!(
        vm.command_display_key_bytes(&replacement).as_bytes(),
        b"opaque\xff"
    );
    assert_ne!(
        vm.visible_command_generation(&replacement),
        Some(generation)
    );
    assert!(vm.refused_completion().is_none());
    assert!(vm.name_world.try_borrow_mut().is_ok());
}

#[test]
fn embedder_command_enumeration_and_whitelists_preserve_exact_bytes() {
    let mut vm = Vm::new();
    vm.register_native_command("watched", Rc::new(TestNative));
    vm.register_command_in_slot(
        CommandSlot {
            namespace: ROOT_NS,
            simple: NameBytes::from(b"opaque\xff"),
        },
        Command::Native(Rc::new(TestNative)),
    );
    assert!(vm.command_names().is_err());
    let world = Rc::clone(&vm.name_world);
    vm.retain_commands_bytes(&|name| {
        assert!(world.try_borrow_mut().is_ok());
        name == b"opaque\xff" || name == b"watched"
    });
    assert_eq!(
        vm.command_names_bytes(),
        vec![NameBytes::from(b"opaque\xff"), NameBytes::from(b"watched")]
    );
    assert_eq!(vm.retain_commands(&|name| name == "watched"), 1);
    assert_eq!(vm.command_names().unwrap(), vec!["watched"]);
}

#[test]
fn backing_catalogue_uses_exact_public_names_and_never_private_storage_keys() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = Vm::new();
    vm.register_native_command("café", Rc::new(TestNative));
    let opaque_slot = CommandSlot {
        namespace: ROOT_NS,
        simple: NameBytes::from(b"opaque_\xff"),
    };
    let key = vm.register_command_in_slot(opaque_slot, Command::Native(Rc::new(TestNative)));
    assert!(key.starts_with("\0tcl-command:"));
    assert_eq!(
        vm.command_display_key_bytes(&key).as_bytes(),
        b"opaque_\xff"
    );
    let generation = vm.visible_command_generation(&key).unwrap();
    let report = vm.backing_report();
    assert!(
        report
            .iter()
            .any(|(name, backing)| name == "café" && *backing == RegisteredBacking::Builtin)
    );
    assert!(
        !report
            .iter()
            .any(|(name, _)| name.starts_with("\0tcl-command:") || name.starts_with("opaque_"))
    );
    assert!(Namespaces::find_command_bytes(&vm, ROOT_NS, b"opaque_\xff").is_some());
    assert_eq!(vm.visible_command_generation(&key), Some(generation));
    assert!(vm.name_world.try_borrow_mut().is_ok());
}

#[test]
fn reserved_command_slots_do_not_manufacture_live_generations() {
    let mut vm = Vm::new();
    let slot = CommandSlot {
        namespace: ROOT_NS,
        simple: NameBytes::from(b"reserved\xff"),
    };
    let key = vm.storage_key_for_command_slot(slot.clone(), slot.simple.as_bytes());
    assert_eq!(vm.command_slot(&key), Some(slot));
    assert_eq!(
        vm.command_display_key_bytes(&key).as_bytes(),
        b"reserved\xff"
    );
    assert_eq!(vm.live_command_generation(&key), None);
    assert_eq!(vm.visible_command_generation(&key), None);
    assert!(Namespaces::find_command_bytes(&vm, ROOT_NS, b"reserved\xff").is_none());
    vm.forget_command_storage_key(&key);
    assert!(vm.command_slot(&key).is_none());
    assert!(vm.name_world.try_borrow_mut().is_ok());
}

#[test]
fn byte_namespace_paths_keep_opaque_and_terminal_colon_token_contexts() {
    for version in tcl_dialect::TclVersion::ALL {
        let mut vm = Vm::new();
        vm.set_runtime_version(version);
        for segment in [b"N\xff".as_slice(), b"N\xc0\x80", b"a:"] {
            let path = NamespacePath::from_segments([segment]);
            vm.declare_namespace_path_with_origin(path.clone(), true);
            let namespace = vm.name_world.borrow().ns_intern[&path];
            let slot = CommandSlot {
                namespace,
                simple: tcl_core_types::NameBytes::from(b"p"),
            };
            vm.register_command_in_slot(slot, Command::Native(Rc::new(TestNative)));
            let command = Namespaces::find_command_bytes(&vm, namespace, b"p")
                .expect("local lookup keeps exact namespace token");
            let rooted = tcl_syntax::naming::root_unrooted_key_bytes(segment);
            assert_eq!(
                Namespaces::find_namespace_bytes(&vm, ROOT_NS, &rooted),
                Some(namespace)
            );
            assert_eq!(Namespaces::name_bytes(&vm, namespace), rooted);
            let mut reported = rooted;
            reported.extend_from_slice(b"::p");
            assert_eq!(Namespaces::command_name_bytes(&vm, command), Some(reported));
            let entry = vm.native_compilation_entry_for_namespace_token(namespace, false);
            assert!(
                entry
                    .namespaces
                    .iter()
                    .any(|row| row.token == u64::from(namespace.0) && row.path == path)
            );
        }
    }
}

#[test]
fn native_compilation_capture_refuses_missing_namespace_coordinates() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.6"));
    let root = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert_eq!(root.current_namespace, u64::from(ROOT_NS.0));
    assert_eq!(
        root.retained_namespace_context(root.current_namespace)
            .unwrap()
            .path,
        NamespacePath::root()
    );
    let literal_world = root.empty_literal_world.as_ref().unwrap();
    assert!(literal_world.is_current());
    let absent = NamespacePath::from_segments([b"unallocated_capture_namespace".as_slice()]);
    assert!(vm.namespace_token_at_path(&absent, true).is_none());
    for error in [
        vm.native_compilation_entry_for_namespace("unallocated_capture_namespace", false)
            .unwrap_err(),
        vm.compile_module_bytes(
            &tcl_lexer::SourceImage::document("set value 1"),
            &absent,
            false,
        )
        .unwrap_err(),
    ] {
        assert_eq!(
            error.native_access_refusal(),
            Some(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native compilation namespace coordinates"
                )
            )
        );
        assert!(error.guest_completion().is_none());
    }
    assert!(vm.namespace_token_at_path(&absent, true).is_none());
    assert!(literal_world.is_current());
    let current_root = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert_eq!(current_root.current_namespace, root.current_namespace);
    assert_eq!(
        current_root
            .retained_namespace_context(current_root.current_namespace)
            .unwrap(),
        root.retained_namespace_context(root.current_namespace)
            .unwrap()
    );
}
