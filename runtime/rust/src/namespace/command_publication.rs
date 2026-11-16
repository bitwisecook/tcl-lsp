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

//! Where a new command is published: the table slot selected for commands, C API registrations, objects, aliases, ensembles and procedures.

use super::{Namespaces, NsId, GLOBAL};

impl Namespaces {
    /// Original Jim table spelling selected before comparison-key normalisation.
    pub(crate) fn jim_command_table_key_at(
        &self,
        current: NsId,
        original: &[u8],
        purpose: tcl_syntax::naming::NativeNamePurpose,
    ) -> Option<tcl_syntax::naming::NativeJimCommandTableKey> {
        use tcl_syntax::naming::{NativeJimCommandTableKey, NativeNamePurpose};
        let protocol = self
            .variable_name_protocol
            .filter(|protocol| protocol.is_jim084())?;
        let context = self.jim_context(current)?;
        let selected = match purpose {
            NativeNamePurpose::CommandPublication => {
                protocol.command_publication_input(context, original)
            }
            NativeNamePurpose::AliasPublication => {
                protocol.alias_publication_input(context, original)
            }
            NativeNamePurpose::ChildAliasPublication => {
                protocol.child_alias_publication_input(context, original)
            }
            NativeNamePurpose::RenameDestination => {
                protocol.rename_destination_input(context, original)
            }
            _ => return None,
        }
        .ok()?;
        NativeJimCommandTableKey::from_projection(&selected)
    }

    /// Original report units of an actual table entry, with its slot kept separate.
    pub(crate) fn command_table_report_name<'a>(&'a self, ns: NsId, simple: &'a [u8]) -> &'a [u8] {
        self.arena[ns]
            .commands
            .binding(simple)
            .and_then(|binding| binding.jim_table_key.as_ref())
            .filter(|_| {
                self.variable_name_protocol
                    .is_some_and(|protocol| protocol.is_jim084())
            })
            .map_or(
                simple,
                tcl_syntax::naming::NativeJimCommandTableKey::report_bytes,
            )
    }

    /// Exact publication slot selected for the active native name recipe.
    pub(crate) fn command_publication_at(
        &mut self,
        current: NsId,
        original: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        if let Some(protocol) = self
            .variable_name_protocol
            .filter(|protocol| protocol.is_jim084())
        {
            let selected = protocol
                .command_publication_input(self.jim_context(current)?, original)
                .ok()?;
            return Some((GLOBAL, selected.jim_flat_key()?.to_vec()));
        }
        let protocol = self.variable_name_protocol?;
        let path = self.native_context_path(current)?;
        let selected = protocol
            .command_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .ok()?;
        Some(self.materialise_command_publication_slot(current, &selected))
    }

    /// C command registration uses global unqualified names and current qualified names.
    pub(crate) fn command_c_api_publication_at(
        &mut self,
        current: NsId,
        original: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.variable_name_protocol?;
        let path = self.native_context_path(current)?;
        let selected = protocol
            .command_c_api_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .ok()?;
        Some(self.materialise_command_publication_slot(current, &selected))
    }

    /// TclOO publication retains its actual selected holder and counted tail.
    pub(crate) fn oo_object_publication_at(
        &mut self,
        current: NsId,
        original: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.variable_name_protocol?;
        let path = self.native_context_path(current)?;
        let selected = protocol
            .oo_object_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .ok()?;
        Some(self.materialise_command_publication_slot(current, &selected))
    }

    /// Create an object's owned namespace from an already selected command slot.
    pub(crate) fn ensure_command_owned_namespace_in_slot(
        &mut self,
        namespace: NsId,
        simple: &[u8],
    ) -> NsId {
        self.ensure_child(namespace, simple)
    }

    /// Select standalone alias publication in the actual interpreter realm.
    pub(crate) fn alias_publication_at(
        &mut self,
        current: NsId,
        original: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.variable_name_protocol?;
        let current = if protocol.is_jim084() {
            GLOBAL
        } else {
            current
        };
        let path = self.native_context_path(current)?;
        let selected = protocol
            .alias_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .ok()?;
        Some(self.materialise_command_publication_slot(current, &selected))
    }

    /// Select the child extension's distinct registration constructor.
    pub(crate) fn child_alias_publication_at(
        &mut self,
        current: NsId,
        original: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.variable_name_protocol?;
        let current = if protocol.is_jim084() {
            GLOBAL
        } else {
            current
        };
        let path = self.native_context_path(current)?;
        let selected = protocol
            .child_alias_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .ok()?;
        Some(self.materialise_command_publication_slot(current, &selected))
    }

    pub(super) fn command_projection_base(
        &self,
        current: NsId,
        selected: &tcl_syntax::naming::NativeCommandSlotProjection,
    ) -> NsId {
        match selected.namespace_route() {
            tcl_syntax::naming::NativeCommandNamespaceRoute::Root => GLOBAL,
            tcl_syntax::naming::NativeCommandNamespaceRoute::Context => current,
            tcl_syntax::naming::NativeCommandNamespaceRoute::ContextParent => {
                self.parent(current).unwrap_or(current)
            }
        }
    }

    pub(crate) fn command_projection_holder(
        &self,
        current: NsId,
        selected: &tcl_syntax::naming::NativeCommandSlotProjection,
    ) -> Option<NsId> {
        let mut token = self.command_projection_base(current, selected);
        for segment in selected.qualifiers() {
            token = self
                .arena
                .get(token)?
                .children
                .get(segment.as_bytes())
                .copied()
                .or_else(|| {
                    self.dying_children
                        .get(&(token, segment.as_bytes().to_vec()))
                        .copied()
                })?;
        }
        Some(token)
    }

    fn materialise_command_publication_slot(
        &mut self,
        current: NsId,
        selected: &tcl_syntax::naming::NativeCommandSlotProjection,
    ) -> (NsId, Vec<u8>) {
        // Preserve an exact existing holder, including a synchronously dying
        // token. A missing longer qualifier chain creates visible children
        // from the operation's selected anchor.
        let namespace = self
            .command_projection_holder(current, selected)
            .unwrap_or_else(|| {
                let mut token = self.command_projection_base(current, selected);
                for segment in selected.qualifiers() {
                    token = self.ensure_child(token, segment.as_bytes());
                }
                token
            });
        (namespace, selected.slot().simple.as_bytes().to_vec())
    }

    /// Materialise a namespace ensemble's selected registration holder.
    /// Original default-parent and explicit-current contexts remain distinct.
    pub(crate) fn ensemble_publication_at(
        &mut self,
        current: NsId,
        original: Option<&[u8]>,
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.variable_name_protocol?;
        let path = self.native_context_path(current)?;
        let parent = self
            .parent(current)
            .and_then(|parent| self.native_context_path(parent));
        let selected = protocol
            .ensemble_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
                parent.as_ref(),
            )
            .ok()?;
        Some(self.materialise_command_publication_slot(current, &selected))
    }

    /// Jim's procedure namespace derives from the counted flat publication key.
    pub(crate) fn jim_procedure_context(&mut self, selected_key: &[u8]) -> Option<NsId> {
        let protocol = self
            .variable_name_protocol
            .filter(|protocol| protocol.is_jim084())?;
        let bytes = protocol.jim_procedure_namespace(selected_key).ok()?;
        if bytes.is_empty() {
            return Some(GLOBAL);
        }
        let holder = crate::obj::Owned::fresh(crate::obj::new_string_bytes(bytes));
        Some(self.retain_jim_namespace(holder, std::rc::Rc::from(bytes)))
    }

    /// Resolve a procedure's original holder before any declaration resources
    /// are accessed. The selected lookup slot retains the original holder.
    pub(crate) fn procedure_lookup_at(
        &self,
        current: NsId,
        original: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.variable_name_protocol?;
        let path = self.native_context_path(current)?;
        let selected = protocol
            .command_lookup_projection(tcl_syntax::naming::NativeNameContext::new(&path), original)
            .ok()?;
        let namespace = self.command_projection_holder(current, &selected)?;
        Some((namespace, selected.slot().simple.as_bytes().to_vec()))
    }

    /// Procedure publication uses the release-selected registration slot;
    /// general builtin C API publication has an independent entry.
    pub(crate) fn procedure_publication_at(
        &mut self,
        current: NsId,
        original: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.variable_name_protocol?;
        if protocol.is_jim084() {
            return self.command_publication_at(current, original);
        }
        let path = self.native_context_path(current)?;
        let selected = protocol
            .command_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .ok()?;
        Some(self.materialise_command_publication_slot(current, &selected))
    }
}
