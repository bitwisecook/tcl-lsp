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

//! Original native rename operands select actual slots and procedure holders.

use super::{CommandSlot, NativeCommandLookupUnavailable, ROOT_NS, Vm};
use tcl_core_types::NameBytes;
use tcl_syntax::naming::{NativeCommandSlotProjection, NativeNameContext};

impl Vm {
    pub(crate) fn native_rename_diagnostic(
        &self,
        failure: tcl_syntax::naming::NativeRenameFailure,
        original: &[u8],
    ) -> Result<tcl_cmd_core::CmdError, NativeCommandLookupUnavailable> {
        let policy = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?;
        let error = policy.recipe().rename_diagnostic(failure, original);
        Ok(match error.error_code {
            Some(code) => tcl_cmd_core::CmdError::with_error_code_bytes(error.message, code),
            None => tcl_cmd_core::CmdError::new_bytes(error.message),
        })
    }

    pub(super) fn native_rename_destination_slot(
        &self,
        original: &[u8],
    ) -> Result<NativeCommandSlotProjection, NativeCommandLookupUnavailable> {
        let policy = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?;
        let world = self.name_world.borrow();
        let namespace = self.current_ns_id();
        let path = self.ns_path(namespace);
        let context = if policy.recipe().is_jim084() {
            let holder = world
                .jim_namespace_objects
                .get(&namespace)
                .ok_or(NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
            NativeNameContext::with_jim_namespace(&path, holder.as_bytes())
        } else {
            NativeNameContext::new(&path)
        };
        policy
            .recipe()
            .rename_destination_projection(context, original)
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)
    }

    pub(super) fn native_rename_source_key(
        &self,
        original: &[u8],
    ) -> Result<Option<String>, NativeCommandLookupUnavailable> {
        let policy = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?;
        if !policy.recipe().is_jim084() {
            let path = self.ns_path(self.current_ns_id());
            let selected = policy
                .recipe()
                .rename_source_input(NativeNameContext::new(&path), original)
                .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
            return self.resolve_command_bytes_checked(
                self.current_ns_id(),
                selected.selected(),
                true,
            );
        }
        let world = self.name_world.borrow();
        let namespace = self.current_ns_id();
        let path = self.ns_path(namespace);
        let holder = world
            .jim_namespace_objects
            .get(&namespace)
            .ok_or(NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        let selected = policy
            .recipe()
            .rename_source_input(
                NativeNameContext::with_jim_namespace(&path, holder.as_bytes()),
                original,
            )
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        let simple = NameBytes::from(
            selected
                .jim_flat_key()
                .ok_or(NativeCommandLookupUnavailable::NamespaceContextUnavailable)?,
        );
        drop(world);
        self.command_at_exact_slot_checked(
            &CommandSlot {
                namespace: ROOT_NS,
                simple,
            },
            true,
        )
    }
}
