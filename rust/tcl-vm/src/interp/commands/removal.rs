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

//! Removing a command: by name, by exact key, by lifecycle key, and unlinking its sidecar for retirement.

use super::super::Vm;
use super::CommandSidecarKey;
use crate::command::Command;
use tcl_core_types::NameBytes;

impl Vm {
    /// Owned storage keys and exact display bytes, with no table guard for consumers.
    pub(crate) fn registered_command_entries(&self) -> Vec<(String, NameBytes)> {
        self.commands
            .keys()
            .map(|key| (key.clone(), self.command_display_key_bytes(key)))
            .collect()
    }

    /// Remove `name` from the command table by exact key — no namespace-path
    /// resolution, which is what a whitelist sweep over
    /// [`Self::registered_command_entries`] needs.
    pub(crate) fn remove_registered_command(&mut self, name: &str) {
        let key = CommandSidecarKey::visible(name);
        let token = self.command_token_identity(&key);
        let command = self.visible_command_at_key(name);
        let (Some(token), Some(command)) = (token, command) else {
            return;
        };
        self.invalidate_compiled_command_semantics_for_key(name);
        if self.import_binding_at_sidecar_key(&key).is_some() {
            self.retire_import_binding(&key);
        } else {
            self.retire_registered_real_command(&token, &command);
        }
    }

    /// Resolve and remove the command `name`, returning it (for `rename`).
    /// Resolution is the full shared rule — current namespace, `namespace
    /// path`, then global — matching C's `TclRenameCommand`, which finds the
    /// source with `Tcl_FindCommand` (tclsh-pinned: `rename viap viap2`
    /// inside a namespace whose path reaches `::pr::viap` moves that
    /// command).
    #[cfg(test)]
    pub(crate) fn take_command(&mut self, name: &str) -> Option<Command> {
        let key = self.resolve_command_fqn(self.current_ns(), name)?;
        // A command that is present in the engine table may still be absent
        // from the emulated release's surface.  Treat it exactly like a
        // missing command here: otherwise `rename` (and `interp hide`, which
        // uses the same removal seam) can move a hidden builtin to a registry-
        // unknown name and make it callable again.
        if self
            .commands
            .get(&key)
            .is_some_and(|command| !self.builtin_command_visible_for_surface(&key, command))
        {
            return None;
        }
        self.take_command_unchecked_key(&key)
    }

    /// Resolve and remove a command without consulting the emulated command
    /// surface.  This is for VM-owned teardown only (temporary `apply`
    /// procedures, coroutine state, and `TclOO` objects); those paths are
    /// deleting an implementation that is already unreachable, not exposing
    /// a command to Tcl code.  User-facing removal uses the selected native command-removal transaction.
    pub(crate) fn take_command_unchecked(&mut self, name: &str) -> Option<Command> {
        let key = self.resolve_command_fqn_raw(self.current_ns(), name)?;
        self.remove_command_exact(&key)
    }

    /// Retire a VM-owned command whether its lifecycle key is currently
    /// visible or hidden. Completion is a real command deletion, so delete
    /// traces fire exactly once before all trace/deopt sidecars are dropped.
    pub(crate) fn retire_command_lifecycle_key(
        &mut self,
        key: &CommandSidecarKey,
    ) -> Option<Command> {
        if let CommandSidecarKey::Visible(name) = key {
            self.materialise_retained_binding(name);
        }
        let command = self.command_at_sidecar_key(key)?;
        if self.import_binding_at_sidecar_key(key).is_some() {
            self.retire_import_binding(key);
        } else {
            let source = self.command_token_identity(key)?;
            self.retire_registered_real_command(&source, &command);
        }
        Some(command)
    }

    /// Remove a command by its already-resolved table key.
    pub(in super::super) fn take_command_unchecked_key(&mut self, key: &str) -> Option<Command> {
        self.take_command_unchecked_key_with_native_lookup(key, true)
    }

    pub(super) fn take_command_unchecked_key_with_native_lookup(
        &mut self,
        key: &str,
        invalidate_lookup: bool,
    ) -> Option<Command> {
        self.take_command_unchecked_key_with_retirement(key, invalidate_lookup, false)
    }

    fn take_command_unchecked_key_with_retirement(
        &mut self,
        key: &str,
        invalidate_lookup: bool,
        original_retirement: bool,
    ) -> Option<Command> {
        let retired = self.visible_command_generation(key).is_some_and(|token| {
            let retired = self
                .name_world
                .borrow_mut()
                .command_name_retiring
                .remove(&token);
            if !retired && !original_retirement {
                self.note_native_command_name_node_changed(token);
            }
            retired
        });
        if !retired
            && !original_retirement
            && invalidate_lookup
            && let Some(slot) = self.command_slot(key)
        {
            self.note_native_namespace_command_lookup(slot.namespace);
        }
        let deletion_published = self.visible_command_generation(key).is_some_and(|token| {
            self.name_world
                .borrow_mut()
                .command_compiler_deletion_published
                .remove(&token)
        });
        if !deletion_published && !original_retirement {
            self.note_native_command_token_changed(&CommandSidecarKey::visible(key));
        }
        if self.commands.contains_key(key) {
            self.invalidate_compiled_command_semantics_for_key(key);
        }
        self.bump_cmd_epoch();
        self.imported_commands.remove(key);
        let removed_generation = self
            .name_world
            .borrow_mut()
            .command_identity
            .generations
            .remove(key);
        self.builtin_identities.remove(key);
        // Clear the `TclOO` root marking with the entry it describes, exactly
        // as the builtin identity above is cleared. A rename re-declares it at
        // the destination; leaving it stranded under the vacated name would
        // gate an unrelated command that later takes that name.
        self.registry_object_roots.remove(key);
        let command = self.commands.remove(key);
        if command.is_some() {
            self.note_command_unbound(key);
            self.forget_command_storage_key(key);
        }
        if let Some(token) = removed_generation {
            let remains_bound = self
                .name_world
                .borrow()
                .command_identity
                .generations
                .is_bound(token);
            if !remains_bound {
                self.note_jim_command_unpublished(token);
            }
        }
        command
    }

    /// Unlink one already-resolved command sidecar without running its
    /// implementation lifecycle or command traces. The caller owns both and
    /// uses this only after following the token's generation through any
    /// re-entrant relocation.
    pub(in super::super) fn take_command_sidecar_for_retirement(
        &mut self,
        key: &CommandSidecarKey,
        original_retirement: bool,
    ) -> Option<Command> {
        match key {
            CommandSidecarKey::Visible(name) => {
                self.materialise_retained_binding(name);
                self.take_command_unchecked_key_with_retirement(name, true, original_retirement)
            }
            CommandSidecarKey::Hidden(name) => {
                if let Some(token) = self.hidden.generations.get(name).copied() {
                    self.name_world
                        .borrow_mut()
                        .command_compiler_deletion_published
                        .remove(&token);
                    let retired = self
                        .name_world
                        .borrow_mut()
                        .command_name_retiring
                        .remove(&token);
                    if !retired && !original_retirement {
                        self.note_native_command_name_node_changed(token);
                    }
                }
                self.bump_cmd_epoch();
                self.hidden.imports.remove(name);
                self.hidden.generations.remove(name);
                self.hidden.builtin_identities.remove(name);
                if let Some(display) = self.hidden.names_by_key.get(name).cloned() {
                    self.hidden.tokens_by_name.remove(&display);
                }
                self.hidden.commands.remove(name)
            }
        }
    }

    /// Remove a command by its exact table key (no name resolution) — the
    /// rollback path for a refused alias creation.
    pub(crate) fn remove_command_exact(&mut self, key: &str) -> Option<Command> {
        let imported = self
            .import_binding_at_sidecar_key(&CommandSidecarKey::visible(key))
            .is_some();
        let command = self.visible_command_at_key(key)?;
        let token = self.command_token_identity(&CommandSidecarKey::visible(key));
        if !imported && let Some(token) = token {
            self.retire_registered_real_command(&token, &command);
        } else {
            self.take_command_unchecked_key(key);
            self.detach_active_sidecars(&CommandSidecarKey::visible(key));
        }
        Some(command)
    }
}
