// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Jim top-frame namespace ownership.

use super::NativeNamespaceAddress;
use tcl_core_types::ROOT_NS;

#[test]
fn jim_top_namespace_owns_the_original_context_empty_object() {
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let vm = crate::native_fixture::core(profile);
    let context = vm.native_jim_object_context().unwrap();
    let identity = context.empty_object().native_object_identity();
    assert_eq!(
        vm.name_world
            .borrow()
            .jim_root_namespace_object
            .as_ref()
            .unwrap()
            .native_object_identity(),
        identity
    );
    assert!(vm.ns_name_bytes(ROOT_NS).as_bytes().is_empty());
    let NativeNamespaceAddress::Jim { object, .. } = vm
        .namespace_address_from_native_bytes(ROOT_NS, b"n\0z")
        .unwrap()
    else {
        panic!("actual Jim namespace lookup must retain its flat object name");
    };
    assert_eq!(object.as_bytes(), b"n\0z");
    assert_eq!(
        vm.native_jim_object_context()
            .unwrap()
            .empty_object()
            .native_object_identity(),
        identity
    );
}

#[test]
fn jim_root_command_context_requires_its_live_original_top_object() {
    // Implementation contract: naming.runtime.original-root-command-context
    // docs/design/analysis/name-resolution-proofs/original-root-command-context.md
    use tcl_runtime_api::Namespaces;
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let vm = crate::native_fixture::core(profile);
    let context = vm.native_jim_object_context().unwrap();
    let root = context.empty_object().clone();
    assert_eq!(vm.root_command_context_checked().unwrap(), Some(ROOT_NS));
    vm.name_world.borrow_mut().jim_root_namespace_object =
        Some(crate::Value::new_native_string_bytes(b"".as_slice()));
    assert!(
        vm.root_command_context_checked().is_err(),
        "equal bytes cannot donate the original object owner"
    );
    vm.name_world.borrow_mut().jim_root_namespace_object = Some(root);
    assert_eq!(vm.root_command_context_checked().unwrap(), Some(ROOT_NS));
    context.retire();
    assert!(
        vm.root_command_context_checked().is_err(),
        "retired interpreter cannot issue a context"
    );
    let vm = crate::native_fixture::core(profile);
    vm.native_jim_object_context().unwrap();
    vm.name_world.borrow_mut().retire();
    assert!(
        vm.root_command_context_checked().is_err(),
        "retired command world cannot issue a context"
    );
}
