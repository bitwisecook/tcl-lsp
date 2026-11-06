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
