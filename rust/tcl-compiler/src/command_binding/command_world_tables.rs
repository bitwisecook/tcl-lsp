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

//! Whether two lookup states share one command world.

use std::sync::Arc;

use super::ModuleCommandBindings;

impl ModuleCommandBindings {
    /// The command table, namespaces, paths, aliases, imports and object
    /// dispatch tables of `other` are the very allocations of `self`. Every
    /// transition of those tables goes through `Arc::make_mut`, so a shared
    /// allocation proves no transition happened between the two states; an
    /// unshared one may still hold equal contents.
    pub(super) fn same_command_world_tables(&self, other: &Self) -> bool {
        // These two are rewritten by ordinary commands that change nothing
        // in them, so equal contents also count.
        (Arc::ptr_eq(&self.original_command_world, &other.original_command_world)
            || self.original_command_world == other.original_command_world)
            && Arc::ptr_eq(&self.logical_alias_prefixes, &other.logical_alias_prefixes)
            && Arc::ptr_eq(&self.class_definitions, &other.class_definitions)
            && (Arc::ptr_eq(&self.object_instances, &other.object_instances)
                || self.object_instances == other.object_instances)
            && Arc::ptr_eq(&self.bindings, &other.bindings)
            && Arc::ptr_eq(
                &self.original_entry_bindings,
                &other.original_entry_bindings,
            )
            && Arc::ptr_eq(&self.objects, &other.objects)
            && Arc::ptr_eq(&self.namespace_exports, &other.namespace_exports)
            && Arc::ptr_eq(&self.namespace_paths, &other.namespace_paths)
            && Arc::ptr_eq(&self.namespaces, &other.namespaces)
            && Arc::ptr_eq(
                &self.unknown_lookup_namespaces,
                &other.unknown_lookup_namespaces,
            )
            && Arc::ptr_eq(
                &self.unknown_namespace_paths,
                &other.unknown_namespace_paths,
            )
            && Arc::ptr_eq(
                &self.namespace_unknown_handlers,
                &other.namespace_unknown_handlers,
            )
            && Arc::ptr_eq(
                &self.ensemble_owner_namespaces,
                &other.ensemble_owner_namespaces,
            )
            && Arc::ptr_eq(&self.ensemble_dispatches, &other.ensemble_dispatches)
            && Arc::ptr_eq(
                &self.unknown_export_namespaces,
                &other.unknown_export_namespaces,
            )
            && Arc::ptr_eq(&self.packages, &other.packages)
            && Arc::ptr_eq(&self.root_boundary_bindings, &other.root_boundary_bindings)
            && Arc::ptr_eq(&self.baseline, &other.baseline)
            && self.loader_handler_unknown == other.loader_handler_unknown
            && self.stock_tclsh_stdout == other.stock_tclsh_stdout
            && self.opaque_domain == other.opaque_domain
            && self.opaque_binding_mutation == other.opaque_binding_mutation
            && self.dynamic_proc_binding == other.dynamic_proc_binding
            && self.unnameable_rebinding_subject == other.unnameable_rebinding_subject
            && self.unclosed_command_world == other.unclosed_command_world
            && self.diagnostic_command_world_closed == other.diagnostic_command_world_closed
            && self.receiver_frame_opacity == other.receiver_frame_opacity
            && self.variable_frame == other.variable_frame
            && self.current_source_origin == other.current_source_origin
    }
}
