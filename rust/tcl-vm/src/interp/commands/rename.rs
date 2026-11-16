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

//! Renaming a command as a transaction: prepare, install the new half, retire or delete the source, and commit.

use super::super::{CommandRenameTransaction, Vm};
use super::{CommandSidecarKey, NativeCommandLookupUnavailable};
use crate::command::Command;
use tcl_core_types::NameBytes;

impl Vm {
    /// Prepare a rename without touching the command table.  Alias-loop
    /// validation runs against the returned logical command, so a rejected
    /// rename cannot fire callbacks or disturb traces, guards, or caches.
    pub(crate) fn prepare_command_rename(
        &self,
        name: &[u8],
    ) -> Result<Option<(Command, CommandRenameTransaction)>, NativeCommandLookupUnavailable> {
        let Some(key) = self.native_rename_source_key(name)? else {
            return Ok(None);
        };
        Ok(self.prepare_command_rename_at_key(key))
    }

    fn prepare_command_rename_at_key(
        &self,
        key: String,
    ) -> Option<(Command, CommandRenameTransaction)> {
        let old_key = self
            .renamed_command_key(CommandSidecarKey::visible(key))
            .into_name();
        let command = self.visible_command_at_key(&old_key)?;
        if !self.builtin_command_visible_for_surface(&old_key, &command) {
            return None;
        }
        let source_retained_root = self.retained_root_for_visible_key(&old_key).filter(|root| {
            self.name_world
                .borrow()
                .ns_deferral
                .retained
                .get(root)
                .is_some_and(|record| record.commands.contains_key(&old_key))
        });
        let source_import_binding = self.visible_import_binding(&old_key);
        let source_generation = self
            .visible_command_generation(&old_key)
            .expect("every registered command has a token generation");
        let source_builtin_identity = self.builtin_identity_for_key(&old_key);
        let source_registry_object_root = self.registry_object_roots.get(&old_key).cloned();
        let cross_alias_target = match &command {
            Command::CrossAlias { target, .. } => Some(*target),
            _ => None,
        };
        Some((
            command,
            CommandRenameTransaction {
                old_key,
                new_key: String::new(),
                source_retained_root,
                source_generation,
                source_import_binding,
                source_builtin_identity,
                source_registry_object_root,
                cross_alias_target,
            },
        ))
    }

    /// Install the new half of a rename and record every changed semantic map.
    pub(crate) fn install_renamed_command(
        &mut self,
        transaction: &mut CommandRenameTransaction,
        new_key: &str,
        command: Command,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let original_oo = if let Command::Object(object) = &command {
            Some(crate::cmd_oo::original_oo_rename(self, *object)?)
        } else {
            None
        };
        if transaction.source_retained_root.is_some() {
            self.materialise_retained_binding(&transaction.old_key);
        }
        let slot = self
            .command_slot(new_key)
            .expect("rename destination has a reserved command slot");
        // The destination hash entry was created by
        // [`Self::note_rename_destination`] before the alias-loop check, and
        // `register_command` re-inserts the same key, which the order owner
        // leaves where that call put it.
        //
        // The source entry stays put: C only deletes it once the `rename`
        // traces have run (`tclBasic.c` 9.0.4 `TclRenameCommand` — create
        // `hPtr`, fire the traces, then `Tcl_DeleteHashEntry(oldHPtr)`), so a
        // callback still resolves the old name.  `cmd_rename` closes the
        // window with [`Self::retire_renamed_command_source`].  The caller has
        // already completed every rejection path, so this is now allowed to
        // fire destination delete traces and invalidate command/trace guard
        // state.
        if let Some(old_slot) = self.command_slot(&transaction.old_key) {
            self.note_native_namespace_command_lookup(old_slot.namespace);
        }
        let new_key = if self.uses_native_jim_lookup() {
            let display = self.command_slot_display_bytes(&slot);
            let key = self.storage_key_for_command_slot(slot, &display);
            self.commands.insert(key.clone(), command.clone());
            self.note_command_bound(&key, false);
            self.set_visible_command_generation(&key, transaction.source_generation);
            self.relocate_jim_command_node(transaction.source_generation, &command, &key);
            key
        } else {
            self.register_command_in_slot(slot, command.clone())
        };
        new_key.clone_into(&mut transaction.new_key);
        if let Command::Object(object) = &command {
            let display = self.command_display_key_bytes(&new_key);
            crate::cmd_oo::oo_command_renamed(
                self,
                *object,
                new_key.clone(),
                display,
                original_oo.as_ref().expect("original Object rename"),
            )?;
        }
        // What that surviving entry points *at* does move, though: C re-homes
        // the one `Command` (`cmdPtr->nsPtr = newNsPtr`) before it fires, and
        // both hash entries reference it, so a proc invoked through the
        // vacating name reports the *destination's* `namespace current` — and
        // resolves `variable` there — for the callbacks' duration.
        self.rebind_rename_source(&transaction.old_key.clone(), command.clone());
        self.set_visible_command_generation(&new_key, transaction.source_generation);
        if transaction.source_import_binding.is_none()
            && let Command::Ensemble(token) = command
        {
            let name = tcl_syntax::naming::root_unrooted_key_bytes(
                self.command_display_key_bytes(&new_key).as_bytes(),
            );
            token.rename(NameBytes::from(name));
        }
        if let Some(binding) = &transaction.source_import_binding {
            self.restore_import_binding(&new_key, binding.clone());
        }
        if let Some(identity) = &transaction.source_builtin_identity {
            self.restore_builtin_identity(&new_key, identity.clone());
        }
        // The `TclOO` root marking is the same kind of fact as the builtin
        // identity above, so it travels the same way. Without this, `rename
        // oo::configurable myconf` produced a command the availability gate
        // read as script-created, so it survived a later switch to an 8.6
        // surface that has no `oo::configurable` at all.
        if let Some(identity) = &transaction.source_registry_object_root {
            self.declare_registry_object_root_as(&new_key, &identity.clone());
        }
        self.retarget_imports_key(
            &CommandSidecarKey::visible(&transaction.old_key),
            &CommandSidecarKey::visible(&new_key),
        );
        Ok(())
    }

    /// Point the still-standing source entry of an open rename at the re-homed
    /// command, without disturbing the entry itself.  Deliberately *not*
    /// [`Self::register_command`]: this is not a replacement, so no `delete`
    /// trace may fire, no generation may be minted and no sidecar may detach —
    /// it is the one command under both of its names, which is what C gets for
    /// free from the two hash entries sharing a `Command *`.  A callback that
    /// deleted the source itself leaves nothing to rebind, and that is not an
    /// error.
    fn rebind_rename_source(&mut self, key: &str, cmd: Command) {
        if self.commands.contains_key(key) {
            self.commands.insert(key.to_owned(), cmd);
        }
    }

    /// Drop the source half of a rename once its `rename` traces have run —
    /// C's `Tcl_DeleteHashEntry(oldHPtr)` at the tail of `TclRenameCommand`.
    /// This is a plain table removal, not a command deletion: no `delete`
    /// trace fires, and the command lives on under its new key.  A callback
    /// may have deleted the source itself (C guards the same case with a
    /// refcount), so a vacated entry is not an error.
    pub(crate) fn retire_renamed_command_source(&mut self, transaction: &CommandRenameTransaction) {
        if self.uses_native_jim_lookup() {
            self.name_world.borrow_mut().advance_jim_procedure_epoch();
        }
        self.take_command_unchecked_key_with_native_lookup(&transaction.old_key, false);
    }

    /// Remove the source for an already validated `rename old {}`. Deletion
    /// retires downstream imports; unlike a rename, it never retargets them.
    pub(crate) fn delete_prepared_renamed_command(
        &mut self,
        transaction: &CommandRenameTransaction,
    ) -> bool {
        if transaction.source_retained_root.is_some() {
            self.materialise_retained_binding(&transaction.old_key);
        }
        let key = CommandSidecarKey::visible(&transaction.old_key);
        if transaction.source_import_binding.is_some() {
            self.retire_import_binding(&key);
            return true;
        }
        let command = self
            .commands
            .get(&transaction.old_key)
            .cloned()
            .expect("the prepared delete source remains registered");
        let source = self
            .command_token_identity(&key)
            .expect("the prepared delete source has a token identity");
        self.retire_registered_real_command(&source, &command);
        true
    }

    /// Commit a rename after all semantic validation has succeeded.
    pub(crate) fn commit_renamed_command(&mut self, transaction: &CommandRenameTransaction) {
        if let Some(target) = transaction.cross_alias_target {
            self.retarget_alias_backref(&transaction.old_key, &transaction.new_key, target);
        }
    }
}
