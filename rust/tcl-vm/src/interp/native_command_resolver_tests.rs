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

#[test]
fn native_command_resolver_inventory_requires_exact_entry_and_no_target_donation() {
    // Implementation proof: naming.compiler.native-command-resolver-inventory
    // docs/design/analysis/name-resolution-proofs/native-command-resolver-inventory.md
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(engine));
        vm.register("opaque_lookup_probe", opaque_handler);
        let entry = vm.native_compilation_entry_for_namespace_token(Some(ROOT_NS), false);
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
