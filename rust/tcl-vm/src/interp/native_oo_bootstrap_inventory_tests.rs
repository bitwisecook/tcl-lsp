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
use tcl_core_types::{ByteNamespacePath, NameBytes};
use tcl_runtime_api::native_oo::{
    NativeOoBootstrapRole, NativeOoIntrinsicMethod, NativeOoLifecycleObservation,
};

fn vm(version: tcl_dialect::TclVersion) -> Vm {
    let profile =
        tcl_registry::model::ingress::resolve_environment(version.dialect_name()).unit_profile();
    crate::native_fixture::interpreter(profile)
}

fn assert_original_oo_factory(
    entry: &tcl_runtime_api::NativeCompilationEntry,
    factory: &tcl_runtime_api::native_oo::NativeOoClassObservation,
) {
    assert_eq!(
        entry.original_oo_class(factory.command_token, factory.implementation_generation),
        Some(factory)
    );
    assert!(
        matches!(&factory.constructor, NativeOoLifecycleObservation::Present(method) if method.intrinsic == Some(NativeOoIntrinsicMethod::ClassConstructor))
    );
    assert_eq!(factory.destructor, NativeOoLifecycleObservation::Absent);
    assert!(factory.class_methods.iter().any(|method| {
        method
            .name
            .as_ref()
            .is_some_and(|name| name.as_bytes() == b"create")
            && method.intrinsic == Some(NativeOoIntrinsicMethod::Create)
    }));
    assert!(factory.object_methods.iter().any(|method| {
        method.visibility_only
            && method.intrinsic.is_none()
            && !method.public
            && method
                .name
                .as_ref()
                .is_some_and(|name| name.as_bytes() == b"new")
    }));
}

#[test]
fn original_oo_bootstrap_inventory_keeps_roles_methods_and_namespace_currency() {
    // Implementation contract: naming.runtime.native-tcloo-class-capability
    // docs/design/analysis/name-resolution-proofs/native-tcloo-class-capability.md
    // Native proof: naming.tcloo.bootstrap-factory-and-support-allocations
    // docs/design/analysis/name-resolution-proofs/tcloo-bootstrap-factory-and-support-allocations.md
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let vm = vm(version);
        let entry = vm.native_compilation_entry_for_namespace_token(ROOT_NS, false);
        let inventory = entry.oo_classes.as_ref().expect("actual OO allocations");
        let factory = inventory
            .classes
            .iter()
            .find(|class| class.bootstrap_role == Some(NativeOoBootstrapRole::ClassFactory))
            .unwrap();
        assert_original_oo_factory(&entry, factory);
        let namespace = entry
            .namespaces
            .iter()
            .find(|namespace| namespace.token == factory.namespace.token)
            .unwrap();
        let paths = namespace
            .command_path
            .iter()
            .map(|token| entry.retained_namespace_context(*token).unwrap().path)
            .collect::<Vec<_>>();
        let helpers = entry
            .namespaces
            .iter()
            .find(|namespace| {
                namespace.path
                    == ByteNamespacePath::from_segments([
                        NameBytes::from(b"oo".as_slice()),
                        NameBytes::from(b"Helpers".as_slice()),
                    ])
            })
            .unwrap();
        let oo = entry
            .namespaces
            .iter()
            .find(|namespace| {
                namespace.path
                    == ByteNamespacePath::from_segments([NameBytes::from(b"oo".as_slice())])
            })
            .unwrap();
        assert_eq!(paths, vec![helpers.path.clone(), oo.path.clone()]);
        let support = inventory
            .classes
            .iter()
            .find(|class| class.bootstrap_role == Some(NativeOoBootstrapRole::ConfigurableSupport));
        assert_eq!(support.is_some(), version >= tcl_dialect::TclVersion::V9_0);
        if let Some(support) = support {
            let root = inventory
                .classes
                .iter()
                .find(|class| class.bootstrap_role == Some(NativeOoBootstrapRole::ObjectRoot))
                .unwrap();
            assert_eq!(support.superclasses, vec![root.object]);
            assert_eq!(support.constructor, NativeOoLifecycleObservation::Absent);
            assert_eq!(support.destructor, NativeOoLifecycleObservation::Absent);
            assert!(support.class_definition_namespace.is_some());
            assert!(support.object_definition_namespace.is_some());
            assert!(
                support
                    .class_methods
                    .iter()
                    .any(|method| method.intrinsic == Some(NativeOoIntrinsicMethod::Configure))
            );
        }
        let mut foreign = entry.clone();
        foreign.oo_classes.as_mut().unwrap().interpreter.owner += 1;
        assert!(
            foreign
                .original_oo_class(factory.command_token, factory.implementation_generation)
                .is_none()
        );
        let mut moved = entry.clone();
        moved.oo_classes.as_mut().unwrap().epoch += 1;
        assert!(
            moved
                .original_oo_class(factory.command_token, factory.implementation_generation)
                .is_none()
        );
        let mut replaced = entry.clone();
        replaced
            .commands
            .iter_mut()
            .find(|binding| binding.token == factory.command_token)
            .unwrap()
            .implementation_generation += 1;
        assert!(
            replaced
                .original_oo_class(factory.command_token, factory.implementation_generation)
                .is_none()
        );
    }
}
