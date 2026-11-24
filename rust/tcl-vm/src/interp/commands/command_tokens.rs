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

//! Command slots, storage keys, generations and token identity: where one exact command token lives in the visible, hidden and retained tables.

use super::super::{CommandSlot, CommandTokenIdentity, ImportBinding, Vm, jim_table_keys};
use super::{CommandSidecarKey, NativeCommandLookupUnavailable};
use crate::command::Command;
use std::rc::Rc;
use tcl_runtime_api::NsId;

impl Vm {
    pub(super) fn mint_command_generation(&mut self) -> u64 {
        let mut world = self.name_world.borrow_mut();
        let identity = &mut world.command_identity;
        identity.next_generation = identity.next_generation.wrapping_add(1);
        if identity.next_generation == 0 {
            identity.next_generation = 1;
        }
        identity.next_generation
    }

    /// Host `CString` publication uses its selected C API purpose and actual holder.
    pub(super) fn native_host_command_slot(
        &mut self,
        original: &[u8],
        command: &Command,
    ) -> Result<CommandSlot, NativeCommandLookupUnavailable> {
        if let Command::Proc(procedure) = command {
            return Ok(procedure.declaration().actual_command_slot());
        }
        let current = self.current_ns_id();
        let path = self.namespace_path_for_token(current);
        let protocol = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?
            .recipe();
        let selected = protocol
            .command_c_api_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        let namespace = self.materialise_native_command_projection(current, &selected);
        Ok(CommandSlot {
            namespace,
            simple: selected.into_slot().simple,
        })
    }

    /// Publish one structured slot into the private String-keyed storage used
    /// by the mature command lifecycle code. The opaque collision key is never
    /// parsed or rendered; all ingress/egress goes through the slot/display
    /// indexes beside it.
    pub(in super::super) fn storage_key_for_command_slot(
        &mut self,
        slot: CommandSlot,
        display: &[u8],
    ) -> String {
        let mut world = self.name_world.borrow_mut();
        let identity = &mut world.command_identity;
        if let Some(key) = identity.by_slot.get(&slot) {
            return key.clone();
        }
        let preferred = core::str::from_utf8(display).ok();
        let key = if let Some(preferred) =
            preferred.filter(|candidate| !identity.slot_by_key.contains_key(*candidate))
        {
            preferred.to_owned()
        } else {
            loop {
                identity.next_storage_key = identity.next_storage_key.wrapping_add(1);
                let candidate = format!("\0tcl-command:{}", identity.next_storage_key);
                if !identity.slot_by_key.contains_key(&candidate) {
                    break candidate;
                }
            }
        };
        identity.by_slot.insert(slot.clone(), key.clone());
        identity.slot_by_key.insert(key.clone(), slot);
        identity.display_by_key.insert(key.clone(), display.into());
        key
    }

    /// Forget a command slot after the exact token occupying it is retired.
    pub(in super::super) fn forget_command_storage_key(&mut self, key: &str) {
        let retired = {
            let mut world = self.name_world.borrow_mut();
            let slot = world.command_identity.slot_by_key.remove(key);
            if let Some(slot) = &slot {
                world.command_identity.by_slot.remove(slot);
                world.command_api_publications.remove(slot);
            }
            world.command_identity.display_by_key.remove(key);
            world.jim_command_table_keys.remove(key);
            if world.jim_local_depth == 0 {
                slot.and_then(|slot| world.jim_command_original_keys.remove(&slot))
            } else {
                None
            }
        };
        drop(retired);
    }

    /// Render one already-resolved command slot for Tcl-facing output. This is
    /// the only native projection from namespace-token identity back to a
    /// command name; publication must never reuse the written ingress spelling.
    pub(crate) fn command_slot_display_bytes(&self, slot: &CommandSlot) -> Vec<u8> {
        tcl_syntax::naming::native_command_full_name_bytes(&tcl_core_types::NativeByteCommandSlot {
            namespace: self.ns_path(slot.namespace).clone(),
            simple: slot.simple.clone(),
        })[2..]
            .to_vec()
    }

    /// Reserve the destination hash-table slot used by `rename`. This occurs
    /// before alias-loop validation just like C's `Tcl_CreateHashEntry`, but
    /// returns the private injective key rather than the ambiguous display.
    pub(crate) fn note_rename_destination(
        &mut self,
        written: &[u8],
        original_key: Option<jim_table_keys::OriginalJimCommandKey>,
    ) -> Result<String, NativeCommandLookupUnavailable> {
        let selected = self.native_rename_destination_slot(written)?;
        let namespace = self.materialise_native_command_projection(self.current_ns_id(), &selected);
        let slot = CommandSlot {
            namespace,
            simple: selected.into_slot().simple,
        };
        let display = self.command_slot_display_bytes(&slot);
        let key = self.storage_key_for_command_slot(slot, &display);
        if let Some(original) = original_key {
            let slot = self
                .command_slot(&key)
                .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?;
            if original.table.comparison_bytes() != slot.simple.as_bytes() {
                return Err(NativeCommandLookupUnavailable::ProtocolUnavailable);
            }
            let retired = {
                let mut world = self.name_world.borrow_mut();
                world
                    .jim_command_table_keys
                    .insert(key.clone(), original.table.clone());
                world.jim_command_original_keys.insert(slot, original)
            };
            drop(retired);
        }
        self.note_command_bound(&key, false);
        Ok(key)
    }

    pub(crate) fn command_slot_parts(
        &self,
        key: &str,
    ) -> Option<(NsId, tcl_core_types::NameBytes)> {
        self.command_slot(key)
            .map(|slot| (slot.namespace, slot.simple.clone()))
    }

    /// Whether the exact table slot named as a `rename` destination is
    /// occupied. Tcl does not apply the command-resolution fallback here: a
    /// global `p` does not prevent `namespace eval n {rename q p}`.
    pub(crate) fn rename_destination_exists(
        &self,
        written: &[u8],
    ) -> Result<bool, NativeCommandLookupUnavailable> {
        let selected = self.native_rename_destination_slot(written)?;
        let Some(namespace) =
            self.native_command_projection_holder(self.current_ns_id(), &selected)
        else {
            return Ok(false);
        };
        self.command_at_exact_slot_checked(
            &CommandSlot {
                namespace,
                simple: selected.into_slot().simple,
            },
            true,
        )
        .map(|key| key.is_some())
    }

    pub(in super::super) fn command_token_identity(
        &self,
        key: &CommandSidecarKey,
    ) -> Option<CommandTokenIdentity> {
        let generation = match key {
            CommandSidecarKey::Visible(name) => self.visible_command_generation(name),
            CommandSidecarKey::Hidden(name) => self.hidden.generations.get(name).copied(),
        }?;
        Some(CommandTokenIdentity {
            key: key.clone(),
            generation,
        })
    }

    /// Find the current visible, hidden, or retained location of one exact
    /// command token after a re-entrant delete callback may have moved it.
    /// Generations are interpreter-unique and travel with rename/hide/expose.
    pub(in super::super) fn command_token_at_generation(
        &self,
        generation: u64,
    ) -> Option<CommandTokenIdentity> {
        let live_key = {
            let world = self.name_world.borrow();
            world
                .command_identity
                .generations
                .key_of(generation)
                .map(CommandSidecarKey::visible)
        };
        let raw_key = live_key
            .or_else(|| {
                self.hidden
                    .generations
                    .iter()
                    .find_map(|(name, candidate)| {
                        (*candidate == generation).then(|| CommandSidecarKey::hidden(name))
                    })
            })
            .or_else(|| {
                let world = self.name_world.borrow();
                world.ns_deferral.retained.values().find_map(|record| {
                    record.generations.iter().find_map(|(name, candidate)| {
                        (*candidate == generation).then(|| CommandSidecarKey::visible(name))
                    })
                })
            })?;
        // During a rename callback both Tcl hash entries refer to one command.
        // The source is only a temporary alias; resolve it to the destination
        // and reject it if a nested delete already removed that destination.
        let key = self.renamed_command_key(raw_key);
        (self.command_token_identity(&key)?.generation == generation)
            .then_some(CommandTokenIdentity { key, generation })
    }

    pub(crate) fn command_generation_for_cleanup(&self, key: &CommandSidecarKey) -> Option<u64> {
        self.command_token_identity(key)
            .map(|token| token.generation)
    }

    pub(in super::super) fn command_at_sidecar_key(
        &self,
        key: &CommandSidecarKey,
    ) -> Option<Command> {
        match key {
            CommandSidecarKey::Visible(name) => self.visible_command_at_key(name),
            CommandSidecarKey::Hidden(name) => self.hidden.commands.get(name).cloned(),
        }
    }

    pub(in super::super) fn import_binding_at_sidecar_key(
        &self,
        key: &CommandSidecarKey,
    ) -> Option<ImportBinding> {
        match key {
            CommandSidecarKey::Visible(name) => self.visible_import_binding(name),
            CommandSidecarKey::Hidden(name) => self.hidden.imports.get(name).cloned(),
        }
    }

    pub(in super::super) fn visible_import_binding(&self, key: &str) -> Option<ImportBinding> {
        let root = self.retained_root_for_visible_key(key);
        let retained = root.and_then(|root| {
            self.name_world
                .borrow()
                .ns_deferral
                .retained
                .get(&root)
                .and_then(|record| record.imported.get(key))
                .cloned()
        });
        retained.or_else(|| self.imported_commands.get(key).cloned())
    }

    pub(super) fn set_visible_command_generation(&mut self, key: &str, generation: u64) {
        let root = self.retained_root_for_visible_key(key);
        let mut world = self.name_world.borrow_mut();
        if let Some(record) = root.and_then(|root| world.ns_deferral.retained.get_mut(&root))
            && record.commands.contains_key(key)
        {
            record.generations.insert(key.to_owned(), generation);
        } else {
            world
                .command_identity
                .generations
                .insert(key.to_owned(), generation);
        }
    }

    pub(in super::super) fn set_visible_import_binding(
        &mut self,
        key: &str,
        binding: ImportBinding,
    ) {
        let root = self.retained_root_for_visible_key(key);
        let world_handle = Rc::clone(&self.name_world);
        let mut world = world_handle.borrow_mut();
        if let Some(record) = root.and_then(|root| world.ns_deferral.retained.get_mut(&root))
            && record.commands.contains_key(key)
        {
            record.imported.insert(key.to_owned(), binding);
        } else {
            self.imported_commands.insert(key.to_owned(), binding);
        }
    }

    pub(in super::super) fn set_visible_builtin_identity(&mut self, key: &str, identity: String) {
        let root = self.retained_root_for_visible_key(key);
        let world_handle = Rc::clone(&self.name_world);
        let mut world = world_handle.borrow_mut();
        if let Some(record) = root.and_then(|root| world.ns_deferral.retained.get_mut(&root))
            && record.commands.contains_key(key)
        {
            record.builtin_identities.insert(key.to_owned(), identity);
        } else {
            self.builtin_identities.insert(key.to_owned(), identity);
        }
    }

    /// Move one exact retained binding out before callback-bearing lifecycle.
    pub(in super::super) fn materialise_retained_binding(&mut self, key: &str) -> Option<NsId> {
        let root = self.retained_root_for_visible_key(key)?;
        let (command, generation, imported, identity) = {
            let mut world = self.name_world.borrow_mut();
            let record = world.ns_deferral.retained.get_mut(&root)?;
            (
                record.commands.remove(key)?,
                record.generations.remove(key),
                record.imported.remove(key),
                record.builtin_identities.remove(key),
            )
        };
        self.commands.insert(key.to_owned(), command);
        if let Some(generation) = generation {
            self.name_world
                .borrow_mut()
                .command_identity
                .generations
                .insert(key.to_owned(), generation);
        }
        if let Some(binding) = imported {
            self.imported_commands.insert(key.to_owned(), binding);
        }
        if let Some(identity) = identity {
            self.builtin_identities.insert(key.to_owned(), identity);
        }
        Some(root)
    }
}

impl Vm {
    /// Intern one stable command-token generation as a dense raw `CommandId`.
    pub(in super::super) fn intern_cmd(&self, token: u64) -> u32 {
        let world = self.name_world.borrow();
        let mut handles = world.command_identity.handles.borrow_mut();
        if let Some(&id) = handles.ids.get(&token) {
            return id;
        }
        let id = u32::try_from(handles.tokens.len()).expect("command count fits u32");
        handles.tokens.push(token);
        handles.ids.insert(token, id);
        id
    }

    /// Copy an interned token without retaining either of the shared table guards.
    fn interned_command_token(&self, id: u32) -> Option<u64> {
        let world = self.name_world.borrow();

        world
            .command_identity
            .handles
            .borrow()
            .tokens
            .get(id as usize)
            .copied()
    }

    /// The current private placement of an interned command token. A rename can
    /// move it and a deletion makes the old id stale; name reuse cannot retarget
    /// it because each publication mints a fresh generation.
    pub(in super::super) fn command_fqn(&self, id: u32) -> Option<String> {
        let token = self.interned_command_token(id)?;
        let world = self.name_world.borrow();
        world
            .command_identity
            .generations
            .key_of(token)
            .cloned()
            .or_else(|| {
                world.ns_deferral.retained.values().find_map(|record| {
                    record
                        .generations
                        .iter()
                        .find_map(|(key, generation)| (*generation == token).then(|| key.clone()))
                })
            })
    }

    pub(in super::super) fn command_sidecar_key(&self, id: u32) -> Option<CommandSidecarKey> {
        let token = self.interned_command_token(id)?;
        self.command_fqn(id)
            .map(CommandSidecarKey::visible)
            .or_else(|| {
                self.hidden
                    .generations
                    .iter()
                    .find_map(|(key, generation)| {
                        (*generation == token).then(|| CommandSidecarKey::hidden(key))
                    })
            })
    }
}

impl tcl_runtime_api::semantic_identity::CommandIdentityTokens for Vm {
    fn interpreter_ref(&self) -> tcl_core_types::InterpreterRef {
        let (owner, interpreter) = self.native_object_bridge_identity();
        tcl_runtime_api::native_compilation::NativeInterpreterIdentity { owner, interpreter }
            .interpreter_ref()
    }

    fn command_ref(
        &self,
        command: tcl_runtime_api::CommandId,
    ) -> tcl_core_types::Answer<tcl_core_types::CommandRef> {
        use tcl_core_types::{Answer, NamingAuthority, NamingProvenance, UnknownReason};
        let Some(token) = self.interned_command_token(command.0) else {
            return Answer::unknown(UnknownReason::UnavailableContext);
        };
        let (owner, interpreter) = self.native_object_bridge_identity();
        let runtime =
            tcl_runtime_api::native_compilation::NativeInterpreterIdentity { owner, interpreter };
        Answer::proven(runtime.command_ref(token)).with_evidence(
            NamingProvenance::Runtime { owner, interpreter },
            NamingAuthority::Runtime,
        )
    }
}

#[cfg(test)]
mod semantic_identity_tests {
    use super::*;
    use tcl_runtime_api::{Namespaces, ROOT_NS, semantic_identity::CommandIdentityTokens};

    #[test]
    fn command_refs_survive_rename_and_split_on_recreation() {
        let mut vm = Vm::new();
        vm.set_compiler(Box::new(
            tcl_codegen::compile_service::BytecodeCompileService::default(),
        ));
        assert!(
            vm.try_eval_source("proc p {} {return p}")
                .unwrap()
                .code
                .is_ok()
        );
        let initial_handle = vm.find_command(ROOT_NS, "p").unwrap();
        let initial = vm.command_ref(initial_handle).into_proven().unwrap();
        assert!(vm.try_eval_source("rename p q").unwrap().code.is_ok());
        let moved_handle = vm.find_command(ROOT_NS, "q").unwrap();
        assert_eq!(initial, vm.command_ref(moved_handle).into_proven().unwrap());
        assert!(
            vm.try_eval_source("proc p {} {return newp}")
                .unwrap()
                .code
                .is_ok()
        );
        let recreated = vm.find_command(ROOT_NS, "p").unwrap();
        assert_ne!(initial, vm.command_ref(recreated).into_proven().unwrap());
        assert!(
            vm.try_eval_source("rename q {}; proc q {} {return fresh}")
                .unwrap()
                .code
                .is_ok()
        );
        let fresh = vm.find_command(ROOT_NS, "q").unwrap();
        assert_ne!(initial, vm.command_ref(fresh).into_proven().unwrap());
        assert_eq!(
            initial,
            vm.command_ref(initial_handle).into_proven().unwrap()
        );
        let mut counts = tcl_core_types::answer::ConformanceCounts::default();
        counts.observe(
            &vm.command_ref(moved_handle)
                .map(|reference| reference == initial),
            Some(&true),
        );
        counts.observe(
            &vm.command_ref(recreated)
                .map(|reference| reference != initial),
            Some(&true),
        );
        counts.observe(
            &vm.command_ref(fresh).map(|reference| reference != initial),
            Some(&true),
        );
        println!("vm: {counts}");
    }
}
