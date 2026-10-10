// SPDX-License-Identifier: AGPL-3.0-or-later
//! Materialise purpose-selected command qualifiers through actual token edges.

use super::{NsId, ROOT_NS, Vm};
use tcl_syntax::naming::{NativeCommandNamespaceRoute, NativeCommandSlotProjection};

impl Vm {
    pub(super) fn native_command_projection_holder(
        &self,
        current: NsId,
        selected: &NativeCommandSlotProjection,
    ) -> Option<NsId> {
        let base = match selected.namespace_route() {
            NativeCommandNamespaceRoute::Root => ROOT_NS,
            NativeCommandNamespaceRoute::Context => current,
            NativeCommandNamespaceRoute::ContextParent => self
                .name_world
                .borrow()
                .physical_namespace_parent(current)
                .unwrap_or(current),
        };
        self.namespace_descendant_token(base, selected.qualifiers())
    }

    pub(super) fn materialise_native_command_projection(
        &mut self,
        current: NsId,
        selected: &NativeCommandSlotProjection,
    ) -> NsId {
        if let Some(namespace) = self.native_command_projection_holder(current, selected) {
            return namespace;
        }
        let absolute = selected.namespace_route() == NativeCommandNamespaceRoute::Root;
        self.declare_namespace_path_with_origin(selected.slot().namespace.clone(), absolute);
        self.definition_namespace_token_at_path(&selected.slot().namespace, absolute)
    }
}
