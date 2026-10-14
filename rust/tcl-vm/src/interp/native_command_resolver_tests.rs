// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resolver absence is a lookup-owner observation, separate from command rows.

use super::*;
use tcl_runtime_api::native_compilation::{
    NativeCommandImplementation, NativeCommandResolverInventory, NativeCommandResolverPresence,
    NativeVariableObserverPresence,
};

fn opaque_handler(_vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
    panic!("capturing lookup state must not execute a handler")
}

fn declare_vector_namespace(vm: &mut Vm, key: &str) -> NsId {
    let path = NamespacePath::from_segments(tcl_syntax::naming::key_segments(key));
    vm.declare_namespace_path_with_origin(path.clone(), true);
    vm.name_world.borrow().ns_intern[&path]
}

#[test]
fn live_lookup_matches_shared_oracle_command_vectors() {
    use tcl_syntax::naming::{NativeNameContext, NativeNameProtocol, key_holder_and_tail};

    for release in tcl_dialect::TclVersion::ALL {
        let mut compared = 0;
        let mut setup_boundaries = Vec::new();
        for vector in tcl_syntax::naming::conformance::vectors() {
            if vector.wants.get(release) == "!ERROR" {
                let reason = if !release.has_namespace_path() && !vector.path.is_empty() {
                    "namespace path is unavailable"
                } else {
                    "original command-definition setup is rejected"
                };
                setup_boundaries.push((vector.line, reason));
                continue;
            }
            let profile = crate::environment::profile_for_dialect(&format!(
                "tcl{}",
                release.version_string()
            ));
            let mut vm = crate::native_fixture::core(profile);
            let context = declare_vector_namespace(&mut vm, &vector.ns);
            let mut declared = HashMap::new();
            let mut slots = HashSet::new();
            for definition in &vector.defs {
                let (holder, simple) = key_holder_and_tail(definition);
                let namespace = declare_vector_namespace(&mut vm, holder);
                let slot = CommandSlot::new(namespace, NameBytes::from(simple));
                let key =
                    vm.register_command_in_slot(slot.clone(), Command::Builtin(opaque_handler));
                let registered = vm.command_slot(&key).expect("the actual registered slot");
                assert_eq!(registered, slot);
                assert!(
                    slots.insert(registered.clone()),
                    "fixture slots must be distinct"
                );
                assert!(
                    declared.insert(definition.clone(), registered).is_none(),
                    "fixture definition labels must be unique"
                );
            }
            let context_path = vm.ns_path(context);
            let path = vector
                .path
                .iter()
                .map(|written| {
                    NativeNameProtocol::C(release)
                        .namespace_address_path(
                            NativeNameContext::new(&context_path),
                            written.as_bytes(),
                        )
                        .expect("the oracle vector has a valid namespace path")
                })
                .map(|path| {
                    vm.declare_namespace_path_with_origin(path.clone(), true);
                    vm.name_world.borrow().ns_intern[&path]
                })
                .collect();
            vm.push_ns_token(vm.ns_name_bytes(context), context);
            vm.ns_path_set(path);
            let actual = vm
                .resolve_command_bytes_checked(context, vector.call.as_bytes(), true)
                .expect("the actual core owns command lookup")
                .and_then(|key| vm.command_slot(&key));
            let expected = vector.want_for(release).map(|label| {
                declared
                    .get(&label)
                    .expect("an oracle winner must select an original fixture label")
                    .clone()
            });
            assert_eq!(
                actual,
                expected,
                "vector {} on Tcl {}",
                vector.line,
                release.version_string()
            );
            compared += 1;
        }
        assert!(compared > 0, "each C release must compare live lookup rows");
        eprintln!(
            "live command lookup Tcl {}: {compared} compared, {} unreached setup rows: {setup_boundaries:?}",
            release.version_string(),
            setup_boundaries.len()
        );
    }
}

#[test]
fn native_command_resolver_inventory_requires_exact_entry_and_no_target_donation() {
    // Implementation proof: naming.compiler.native-command-resolver-inventory
    // docs/design/analysis/name-resolution-proofs/native-command-resolver-inventory.md
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(engine));
        vm.register("opaque_lookup_probe", opaque_handler);
        let entry = vm.native_compilation_entry_for_namespace_token(ROOT_NS, false);
        let inventory = entry
            .command_resolvers
            .expect("the VM owns its lookup surface");
        assert!(inventory.permits_no_callbacks(&entry));
        let opaque = entry
            .lookup_command_bytes(entry.current_namespace, b"opaque_lookup_probe")
            .unwrap()
            .unwrap();
        assert_eq!(opaque.implementation, NativeCommandImplementation::Opaque);
        assert!(opaque.compiler.is_none());

        let mut stale = entry.clone();
        stale.epoch += 1;
        assert!(!inventory.permits_no_callbacks(&stale));
        let mut foreign = entry.clone();
        foreign.interpreter.owner += 1;
        assert!(!inventory.permits_no_callbacks(&foreign));
        foreign.interpreter = entry.interpreter;
        foreign.interpreter.interpreter += 1;
        assert!(!inventory.permits_no_callbacks(&foreign));
        for presence in [
            NativeCommandResolverPresence::Present,
            NativeCommandResolverPresence::Unknown,
        ] {
            let observed =
                NativeCommandResolverInventory::captured(entry.interpreter, entry.epoch, presence);
            assert!(!observed.permits_no_callbacks(&entry));
            let mut changed = entry.clone();
            changed.command_resolvers = Some(observed);
            assert!(!entry.same_compilation_world(&changed));
        }
        let mut missing = entry.clone();
        missing.command_resolvers = None;
        assert!(!inventory.permits_no_callbacks(&missing));
        assert!(!entry.same_compilation_world(&missing));

        // Resolver absence supplies no variable-observer closure and does not
        // erase other compilation-world differences.
        let mut observed = entry.clone();
        observed.variable_observers = NativeVariableObserverPresence::Present;
        assert!(inventory.permits_no_callbacks(&observed));
        assert!(!entry.same_compilation_world(&observed));
    }
}
