// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual owning previous-node edges and live Jim upcall traversal.

use super::Namespaces;
use crate::interp::Command;

impl Namespaces {
    pub(crate) fn bind_jim_local(
        &mut self,
        entry: super::NativeCommandCreation,
        name: &[u8],
        command: Command,
    ) {
        let ns = entry.namespace;
        let previous = self.arena[ns].commands.take_slot(name);
        self.insert_bound_after_entry(entry, name.to_vec(), command);
        if let Some(previous) = previous {
            if let Some(node) = self.native_command_nodes.get_mut(&previous.generation) {
                node.placement = None;
            }
            let token = self.arena[ns]
                .commands
                .generation(name)
                .expect("new local node");
            self.jim_previous_locations
                .insert(previous.generation, (ns, name.to_vec()));
            self.jim_previous_commands.insert(token, previous);
            if tcl_syntax::native_jim_local::NativeJimLocalProtocol::jim084().changes_epoch(true) {
                self.advance_jim_procedure_epoch();
            }
        }
    }

    pub(crate) fn jim_node_command(&self, token: u64) -> Option<Command> {
        self.native_command_at_node(token)
            .map(|(command, _)| command)
            .or_else(|| {
                self.jim_previous_commands
                    .values()
                    .find(|node| node.generation == token)
                    .map(|node| node.command.clone())
            })
    }
    pub(crate) fn jim_has_previous(&self, token: u64) -> bool {
        self.jim_previous_commands.contains_key(&token)
    }
    pub(crate) fn jim_previous_selection(
        &self,
        mut token: u64,
        mut command: Command,
    ) -> Option<(Command, u64)> {
        let recipe = tcl_syntax::native_jim_local::NativeJimLocalProtocol::jim084();
        while recipe.follows_previous(
            matches!(command, Command::Proc(_)),
            self.jim_upcalls.get(&token).copied().unwrap_or(0),
        ) {
            let previous = self.jim_previous_commands.get(&token)?;
            token = previous.generation;
            command = previous.command.clone();
        }
        Some((command, token))
    }
    pub(crate) fn enter_jim_upcall(&mut self, token: u64) {
        *self.jim_upcalls.entry(token).or_default() += 1;
        self.enter_jim_command(token);
    }
    pub(crate) fn leave_jim_upcall(&mut self, token: u64) {
        let count = self.jim_upcalls.get_mut(&token).expect("owned Jim upcall");
        *count -= 1;
        if *count == 0 {
            self.jim_upcalls.remove(&token);
        }
        self.leave_jim_command(token);
    }
    pub(crate) fn clean_jim_local_key(&mut self, key: &[u8]) {
        // JimDeleteLocalProcs compares the original cleanup name directly in
        // the flat table, independently of the frame's current namespace.
        let protocol = tcl_syntax::naming::NativeNameProtocol::Jim084;
        let Ok(slot) =
            protocol.command_publication_slot(tcl_syntax::naming::NativeNameContext::root(), key)
        else {
            return;
        };
        let key = slot.simple.as_bytes();
        let Some(top) = self.arena[super::GLOBAL].commands.take_slot(key) else {
            return;
        };
        let top_generation = top.generation;
        if let Some(previous) = self.jim_previous_commands.remove(&top_generation) {
            let previous_token = previous.generation;
            self.arena[super::GLOBAL]
                .commands
                .restore_slot(key.to_vec(), previous);
            if let Some(node) = self.native_command_nodes.get_mut(&previous_token) {
                node.placement = Some((super::GLOBAL, key.to_vec()));
            }
            self.arena[super::GLOBAL]
                .commands
                .retired
                .borrow_mut()
                .push(super::RetiredCommandSlot {
                    original_jim_key: None,
                    binding: Some(top),
                });
        } else {
            self.arena[super::GLOBAL]
                .commands
                .drop_entry_with_binding(key, top);
        }
        if let Some(node) = self.native_command_nodes.get_mut(&top_generation) {
            node.placement = None;
        }
        self.note_jim_command_retirement(top_generation);
    }
    pub(super) fn release_jim_previous(&mut self, token: u64) {
        if let Some(previous) = self.jim_previous_commands.remove(&token) {
            let generation = previous.generation;
            // Preserve current-before-previous retirement, but extract the real
            // owning edge before its delete callback can reenter Namespace.
            self.arena[super::GLOBAL]
                .commands
                .retired
                .borrow_mut()
                .push(super::RetiredCommandSlot {
                    original_jim_key: None,
                    binding: Some(previous),
                });
            if self.native_command_at_node(generation).is_none() {
                self.note_jim_command_retirement(generation);
            }
        }
    }
}
