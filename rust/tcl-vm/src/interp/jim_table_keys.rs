// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Jim table keys retain publication spelling separately from storage addresses.

use super::{Command, CommandSlot, NativeCommandLookupUnavailable, NsId, ROOT_NS, Vm};
use tcl_syntax::naming::{
    NativeJimCommandTableKey, NativeNameContext, NativeNameProtocol, NativeNamePurpose,
};

impl Vm {
    pub(crate) fn jim_command_table_key_for_original(
        &self,
        original: &[u8],
        purpose: NativeNamePurpose,
    ) -> Result<Option<NativeJimCommandTableKey>, NativeCommandLookupUnavailable> {
        if !self.uses_native_jim_lookup() {
            return Ok(None);
        }
        let protocol = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?
            .recipe();
        let world = self.name_world.borrow();
        let current = self.current_ns_id();
        let path = self.ns_path(current);
        let holder = world
            .jim_namespace_objects
            .get(&current)
            .ok_or(NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        let context = NativeNameContext::with_jim_namespace(&path, holder.as_bytes());
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
            _ => return Err(NativeCommandLookupUnavailable::ProtocolUnavailable),
        }
        .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        Ok(NativeJimCommandTableKey::from_projection(&selected))
    }

    pub(super) fn jim_command_table_key_for_slot(
        &self,
        slot: &CommandSlot,
        incoming: Option<NativeJimCommandTableKey>,
    ) -> Option<NativeJimCommandTableKey> {
        if !self.uses_native_jim_lookup() {
            return None;
        }
        if slot.namespace != ROOT_NS {
            return None;
        }
        let incoming = incoming.or_else(|| {
            NativeJimCommandTableKey::from_comparison_key(
                NativeNameProtocol::Jim084,
                slot.simple.as_bytes(),
            )
        })?;
        assert_eq!(
            incoming.comparison_bytes(),
            slot.simple.as_bytes(),
            "actual Jim table comparison slot"
        );
        let occupied = self.command_storage_key_at_slot(slot).and_then(|key| {
            self.visible_command_at_key(&key)?;
            self.name_world
                .borrow()
                .jim_command_table_keys
                .get(&key)
                .cloned()
        });
        Some(incoming.retain_for_replacement(occupied.as_ref()))
    }

    pub(super) fn jim_command_table_report_names(
        &self,
        namespace: NsId,
        procs_only: bool,
    ) -> Vec<Vec<u8>> {
        self.jim_command_table_report_names_filtered(namespace, |command| {
            !procs_only || matches!(command, Command::Proc(_))
        })
    }

    pub(super) fn jim_command_table_report_names_filtered(
        &self,
        namespace: NsId,
        selected_kind: impl Fn(&Command) -> bool,
    ) -> Vec<Vec<u8>> {
        if namespace != ROOT_NS {
            return Vec::new();
        }
        let world = self.name_world.borrow();
        self.commands
            .iter()
            .filter(|(key, command)| self.builtin_command_visible_for_surface(key, command))
            .filter(|(_, command)| selected_kind(command))
            .filter_map(|(key, _)| {
                self.command_slot(key)
                    .filter(|slot| slot.namespace == ROOT_NS)?;
                world
                    .jim_command_table_keys
                    .get(key)
                    .map(|key| key.report_bytes().to_vec())
            })
            .collect()
    }
}
