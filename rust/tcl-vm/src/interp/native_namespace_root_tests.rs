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
