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

//! Publishing a command into a slot: registration from the host and from scripts, replacement, ensembles, and the publication slots of procedures, aliases and objects.

use super::super::{
    CommandSlot, CommandTokenIdentity, Vm, command_cleanup, jim_table_keys,
    native_operation_currency, oo_cleanup,
};
use super::{CommandPublicationOutcome, CommandSidecarKey, NativeCommandLookupUnavailable};
use crate::command::{Command, EnsembleDef, ProcDef};
use std::rc::Rc;
use tcl_core_types::NameBytes;
use tcl_runtime_api::{NsId, ROOT_NS};

impl Vm {
    pub(crate) fn register_command(
        &mut self,
        name: &str,
        cmd: Command,
    ) -> CommandPublicationOutcome {
        self.register_command_with_host_input(name, cmd)
    }

    pub(super) fn register_command_with_host_input(
        &mut self,
        original_input: &str,
        cmd: Command,
    ) -> CommandPublicationOutcome {
        // Jim retains the original CString input independently of C's
        // already-constructed address and the hash comparison spelling.
        if self.execution_refusal.is_some() {
            drop(cmd);
            return CommandPublicationOutcome::Unavailable;
        }
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => {
                oo_cleanup::retain_error(self, &error);
                drop(cmd);
                return CommandPublicationOutcome::Unavailable;
            }
        };
        let incoming = match self.jim_command_table_key_for_original(
            original_input.as_bytes(),
            tcl_syntax::naming::NativeNamePurpose::CommandCApiPublication,
        ) {
            Ok(key) => key,
            Err(error) => {
                let _ = self.refuse_host_command(error.to_string());
                drop(cmd);
                return CommandPublicationOutcome::Unavailable;
            }
        };
        let slot = match incoming.as_ref() {
            Some(key) => CommandSlot {
                namespace: ROOT_NS,
                simple: key.table.comparison_bytes().into(),
            },
            None => match self.native_host_command_slot(original_input.as_bytes(), &cmd) {
                Ok(slot) => slot,
                Err(error) => {
                    let _ = self.refuse_host_command(error.to_string());
                    drop(cmd);
                    return CommandPublicationOutcome::Unavailable;
                }
            },
        };
        let key = self.register_command_in_slot_with_jim_key(slot.clone(), cmd, incoming);
        if let Err(error) = scope.check(self) {
            oo_cleanup::retain_error(self, &error);
            return CommandPublicationOutcome::Unavailable;
        }
        if self.execution_refusal.is_some() {
            return CommandPublicationOutcome::Unavailable;
        }
        if let Some(generation) = self.visible_command_generation(&key) {
            self.name_world
                .borrow_mut()
                .command_api_publications
                .insert(
                    slot,
                    (
                        generation,
                        tcl_core_types::c_string_extent(original_input.as_bytes()).to_vec(),
                    ),
                );
        }
        CommandPublicationOutcome::Published(key)
    }

    /// Publish a host-written name through the selected script declaration
    /// purpose. This preserves the embedder's procedure-like naming contract;
    /// it does not select the distinct C API global-publication purpose.
    pub(crate) fn register_written_command(
        &mut self,
        written: &str,
        command: Command,
    ) -> Result<String, NativeCommandLookupUnavailable> {
        if !self.name_world.borrow().is_live() {
            return Err(NativeCommandLookupUnavailable::Retired);
        }
        let report = self.jim_command_table_key_for_original(
            written.as_bytes(),
            tcl_syntax::naming::NativeNamePurpose::CommandPublication,
        )?;
        let (_, slot, _) = self.native_procedure_publication(written.as_bytes())?;
        Ok(self.register_command_in_slot_with_jim_key(slot, command, report))
    }

    fn retire_command_replaced_by_callback(
        &mut self,
        token: &CommandTokenIdentity,
        command: &Command,
        replacing_retained: bool,
        replacing_real_command: bool,
        original: &command_cleanup::OriginalCommandCleanup,
    ) {
        // Delete callbacks are arbitrary Tcl. If one moves the old
        // generation, replacing the original slot still has to retire
        // that exact command before the new generation is published.
        // An unchanged non-retained command uses Tcl's atomic
        // replacement lifecycle so its imports retarget below.
        if original.unlink_recursive_c_delete(self, token) {
            return;
        }
        let _ = original.in_original(self, |vm| {
            if let Some(live_token) = vm.command_token_at_generation(token.generation) {
                if replacing_retained && replacing_real_command {
                    vm.retire_real_command_after_trace(&live_token, command);
                } else if replacing_retained {
                    vm.retire_downstream_imports(&live_token);
                } else if replacing_real_command && live_token.key == token.key {
                    vm.retire_replaced_real_command(&live_token, command);
                } else if replacing_real_command {
                    vm.retire_real_command_after_trace(&live_token, command);
                }
            } else if replacing_real_command && matches!(command, Command::ChildInterp(_)) {
                // A delete trace unlinked the name by deleting the interp again.
                // The replaced command's own deletion still retires the interp.
                vm.retire_replaced_real_command(token, command);
            }
        });
    }

    pub(crate) fn register_command_in_slot(&mut self, slot: CommandSlot, cmd: Command) -> String {
        self.register_command_in_slot_with_jim_key(slot, cmd, None)
    }

    fn publish_jim_command_table_key(
        &mut self,
        name: &str,
        slot: &CommandSlot,
        jim_table_key: Option<jim_table_keys::OriginalJimCommandKey>,
    ) {
        if let Some(key) = jim_table_key {
            let unused = {
                let mut world = self.name_world.borrow_mut();
                if let Some(original) = world.jim_command_original_keys.get(slot) {
                    let table = key
                        .table
                        .clone()
                        .retain_for_replacement(Some(&original.table));
                    world.jim_command_table_keys.insert(name.to_owned(), table);
                    Some(key)
                } else {
                    world
                        .jim_command_table_keys
                        .insert(name.to_owned(), key.table.clone());
                    world.jim_command_original_keys.insert((*slot).clone(), key);
                    None
                }
            };
            drop(unused);
        }
    }

    fn finish_command_publication(
        &mut self,
        name: &str,
        installed_generation: u64,
        replaced_token: Option<CommandTokenIdentity>,
        replacing_retained: bool,
    ) {
        let installed_token = CommandTokenIdentity {
            key: CommandSidecarKey::visible(name),
            generation: installed_generation,
        };
        if let Some(replaced_token) = replaced_token
            && !replacing_retained
        {
            let installed = self
                .commands
                .get(name)
                .expect("the command was just installed")
                .clone();
            self.replace_import_implementations(&replaced_token, &installed_token, &installed);
        }
        self.absorb_retained_binding(name);
        if replacing_retained {
            self.materialise_retained_binding(name);
            if self
                .command_token_identity(&CommandSidecarKey::visible(name))
                .as_ref()
                == Some(&installed_token)
            {
                self.retire_command_lifecycle_key(&CommandSidecarKey::visible(name));
            }
        }
    }

    pub(crate) fn register_command_in_slot_with_jim_key(
        &mut self,
        slot: CommandSlot,
        cmd: Command,
        incoming: Option<jim_table_keys::OriginalJimCommandKey>,
    ) -> String {
        let original_cleanup = command_cleanup::OriginalCommandCleanup::enter(self);
        let jim_table_key = self.jim_command_table_key_for_slot(&slot, incoming);
        if self.uses_native_jim_lookup() && jim_table_key.is_none() {
            let _ = self.retain_name_query_refusal("original Jim publication key unavailable");
            let _ = original_cleanup.in_original(self, |_| drop(cmd));
            return self.command_storage_key_at_slot(&slot).unwrap_or_default();
        }
        if self.uses_native_jim_lookup() && self.name_world.borrow().jim_local_depth != 0 {
            return self.register_jim_local_command(slot, cmd, jim_table_key);
        }
        let display = self.command_slot_display_bytes(&slot);
        let original_key = self.storage_key_for_command_slot(slot.clone(), &display);
        let name = original_key.as_str();
        let invalidates_path_before_create = self
            .native_scalar_carrier_dialect()
            .native_command_name_protocol()
            .is_some_and(tcl_native_model::native_command_literal::NativeCommandNameProtocol::invalidates_path_before_object_create);
        if invalidates_path_before_create {
            self.note_native_command_path_dependents(slot.namespace);
        }
        let replacing_retained = self.materialise_retained_binding(name).is_some();
        let replacing_command = self.commands.contains_key(name);
        // Invalidate before delete callbacks can re-enter an already-running
        // frame while the old binding is still installed. A second publication
        // after insertion below prevents that frame from acknowledging this
        // intermediate state and then retaining the replaced implementation.
        if replacing_command {
            self.invalidate_compiled_command_semantics_for_key(name);
        }
        let replaced_token = replacing_command
            .then(|| self.command_token_identity(&CommandSidecarKey::visible(name)))
            .flatten();
        let replacing_real_command =
            replacing_command && !self.imported_commands.contains_key(name);
        let replaced_command = replacing_command
            .then(|| self.commands.get(name).cloned())
            .flatten();
        if replacing_command {
            let recursive = replaced_token
                .as_ref()
                .is_some_and(|token| original_cleanup.unlink_recursive_c_delete(self, token));
            if !recursive {
                if let Some(Command::ChildInterp(child)) = &replaced_command
                    && let Some(slot) = self.interps.get_mut(child.0)
                {
                    slot.command_dying = true;
                }
                self.detach_active_sidecars(&CommandSidecarKey::visible(name));
                self.on_command_removed(name);
                if let (Some(token), Some(command)) = (&replaced_token, &replaced_command) {
                    self.retire_command_replaced_by_callback(
                        token,
                        command,
                        replacing_retained,
                        replacing_real_command,
                        &original_cleanup,
                    );
                }
            }
        }
        if original_cleanup.check(self).is_err() {
            let _ = original_cleanup.in_original(self, |_| drop(cmd));
            return original_key;
        }
        // A callback above may have renamed the dying command, which moves the
        // structured slot reservation with it. Re-reserve the requested
        // destination only after the old generation's callback-bearing
        // lifecycle is complete; the fresh command must never inherit the
        // relocated token's private storage key.
        let storage_key = self.storage_key_for_command_slot(slot.clone(), &display);
        let name = storage_key.as_str();
        let publishing_replacement = self.commands.contains_key(name);
        // Overwriting a cross-interp alias drops its target-death backref (the
        // alias-create path re-adds one for the new alias afterwards). Gated
        // on the backref table so the builtin sweep and ordinary command
        // registration pay nothing.
        if !self.alias_backrefs.is_empty() {
            self.drop_alias_backref(name);
        }
        self.imported_commands.remove(name);
        self.builtin_identities.remove(name);
        // The TclOO root marking is an identity, not a reservation on the
        // *name*: it says "the engine installed this entry on the registry's
        // behalf, so date it by the registry". Overwriting the entry replaces
        // that identity with a script-created one, which is release-invariant
        // like any proc — so the marking must not outlive the entry it
        // described. Leaving it behind makes `oo::class create ::oo::
        // configurable {…}` at 8.6 (where no such builtin exists, so the name
        // is the user's to take) create an object the availability gate then
        // hides forever. The engine's own installs re-declare the root
        // immediately after registering, so clearing here cannot unmark them.
        self.registry_object_roots.remove(name);
        if !replacing_command && let Some(slot) = self.command_slot(name) {
            self.invalidate_native_command_literal_in_slot(&slot);
            self.note_native_namespace_command_lookup(slot.namespace);
            if !invalidates_path_before_create {
                self.note_native_command_path_dependents(slot.namespace);
            }
        }
        if let Some(token) = replaced_token.as_ref() {
            self.note_jim_command_unpublished(token.generation);
        }
        self.publish_jim_command_table_key(name, &slot, jim_table_key);
        self.commands.insert(name.to_owned(), cmd);
        self.note_command_bound(name, publishing_replacement);
        let installed_generation = self.mint_command_generation();
        self.name_world
            .borrow_mut()
            .command_identity
            .generations
            .insert(name.to_owned(), installed_generation);
        if let Some(command) = self.commands.get(name) {
            self.note_jim_command_publication(installed_generation, command, name, &display);
        }
        self.finish_command_publication(
            name,
            installed_generation,
            replaced_token,
            replacing_retained,
        );
        // Publish the fully-installed table state. Delete callbacks and import
        // retargeting are re-entrant and may have populated resolution caches or
        // acknowledged the pre-install deopt epoch. A retained binding has been
        // moved into its token's table by this point, so the same publication
        // covers both stores.
        self.bump_cmd_epoch();
        self.invalidate_compiled_command_semantics_for_key(name);
        storage_key
    }

    /// A command bound at a name the current frame reaches only through its
    /// retained namespace token belongs to that token's own table. Registration
    /// runs the ordinary lifecycle over the flat map — the whole
    /// replacement/import/ensemble machinery is keyed there — and the fresh
    /// binding then moves across, so nothing of it stays publicly addressable.
    fn absorb_retained_binding(&mut self, key: &str) {
        let Some(namespace) = self.command_slot(key).map(|slot| slot.namespace) else {
            return;
        };
        let root = self
            .name_world
            .borrow()
            .ns_deferral
            .owners
            .get(&namespace)
            .copied();
        let Some(root) = root else {
            return;
        };
        let command = self.commands.remove(key);
        let imported = self.imported_commands.remove(key);
        let identity = self.builtin_identities.remove(key);
        let mut world = self.name_world.borrow_mut();
        let generation = world.command_identity.generations.remove(key);
        let Some(record) = world.ns_deferral.retained.get_mut(&root) else {
            return;
        };
        if let Some(command) = command {
            record.commands.insert(key.to_owned(), command);
        }
        if let Some(generation) = generation {
            record.generations.insert(key.to_owned(), generation);
        }
        if let Some(binding) = imported {
            record.imported.insert(key.to_owned(), binding);
        }
        if let Some(identity) = identity {
            record.builtin_identities.insert(key.to_owned(), identity);
        }
    }

    /// Install a newly-created real ensemble and link its stable token at the
    /// head of its implementation namespace's ensemble list. Imports and
    /// rename/hide/expose relocations use [`Self::register_command`] directly:
    /// they share or move an existing token and must not create a second list
    /// entry.
    pub(crate) fn register_namespace_ensemble(
        &mut self,
        name: &str,
        token: &Rc<tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, NameBytes>>,
    ) {
        if self
            .register_command(name, Command::Ensemble(Rc::clone(token)))
            .into_key()
            .is_none()
        {
            return;
        }
        let namespace = token.config().namespace;
        self.namespace_ensembles
            .entry(namespace)
            .or_default()
            .push(Rc::downgrade(token));
    }

    pub(crate) fn register_namespace_ensemble_slot(
        &mut self,
        slot: CommandSlot,
        token: &Rc<tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, NameBytes>>,
    ) {
        self.register_command_in_slot(slot, Command::Ensemble(Rc::clone(token)));
        self.advance_native_ensemble_export_epoch(token.config().namespace);
        self.namespace_ensembles
            .entry(token.config().namespace)
            .or_default()
            .push(Rc::downgrade(token));
    }

    pub(crate) fn native_namespace_ensemble_publication_slot(
        &mut self,
        original: Option<&[u8]>,
    ) -> Result<CommandSlot, NativeCommandLookupUnavailable> {
        let context = self.current_ns_id();
        let protocol = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?
            .recipe();
        let path = self.ns_path(context);
        let jim = self
            .name_world
            .borrow()
            .jim_namespace_objects
            .get(&context)
            .cloned();
        let native_context = if protocol.is_jim084() {
            tcl_syntax::naming::NativeNameContext::with_jim_namespace(
                &path,
                jim.as_ref()
                    .ok_or(NativeCommandLookupUnavailable::NamespaceContextUnavailable)?
                    .as_bytes(),
            )
        } else {
            tcl_syntax::naming::NativeNameContext::new(&path)
        };
        let parent = self.name_world.borrow().physical_namespace_parent(context);
        let parent_path = parent.map(|parent| self.ns_path(parent));
        let selected = protocol
            .ensemble_publication_projection(native_context, original, parent_path.as_ref())
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        let namespace = self.materialise_native_command_projection(context, &selected);
        Ok(CommandSlot {
            namespace,
            simple: selected.slot().simple.clone(),
        })
    }

    /// Check a procedure holder before parsing formals or creating publication state.
    pub(crate) fn native_procedure_holder_exists(
        &self,
        original: &[u8],
    ) -> Result<bool, NativeCommandLookupUnavailable> {
        let protocol = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?
            .recipe();
        if protocol.is_jim084() {
            return Ok(true);
        }
        let context = self.current_ns_id();
        let path = self.ns_path(context);
        let selected = protocol
            .command_lookup_projection(tcl_syntax::naming::NativeNameContext::new(&path), original)
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        Ok(self
            .native_command_projection_holder(context, &selected)
            .is_some())
    }

    /// Select the publication slot and actual procedure activation namespace.
    /// Tcl 8.4/8.5 publication reparses its constructed C registration spelling;
    /// later C releases keep the selected holder token.
    pub(crate) fn native_procedure_publication(
        &mut self,
        original: &[u8],
    ) -> Result<(String, CommandSlot, NsId), NativeCommandLookupUnavailable> {
        let protocol = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?
            .recipe();
        let context = self.current_ns_id();
        let path = self.ns_path(context);
        let jim = self
            .name_world
            .borrow()
            .jim_namespace_objects
            .get(&context)
            .cloned();
        let native_context = if protocol.is_jim084() {
            tcl_syntax::naming::NativeNameContext::with_jim_namespace(
                &path,
                jim.as_ref()
                    .ok_or(NativeCommandLookupUnavailable::NamespaceContextUnavailable)?
                    .as_bytes(),
            )
        } else {
            tcl_syntax::naming::NativeNameContext::new(&path)
        };
        let selected = protocol
            .command_publication_projection(native_context, original)
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        let (namespace, activation) = if protocol.is_jim084() {
            let holder = protocol
                .jim_procedure_namespace(selected.slot().simple.as_bytes())
                .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
            (ROOT_NS, self.intern_jim_namespace_object(holder))
        } else {
            let token = self.materialise_native_command_projection(context, &selected);
            (token, token)
        };
        let slot = CommandSlot {
            namespace,
            simple: selected.into_slot().simple,
        };
        let display = self.command_slot_display_bytes(&slot);
        let key = self.storage_key_for_command_slot(slot.clone(), &display);
        Ok((key, slot, activation))
    }

    /// Select alias publication without borrowing procedure declaration rules.
    pub(crate) fn native_alias_publication_slot(
        &mut self,
        original: &[u8],
    ) -> Result<CommandSlot, NativeCommandLookupUnavailable> {
        let recipe = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?
            .recipe();
        let context = self.current_ns_id();
        let path = self.namespace_path_for_token(context);
        let selected = recipe
            .alias_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        let namespace = self.materialise_native_command_projection(context, &selected);
        Ok(CommandSlot {
            namespace,
            simple: selected.into_slot().simple,
        })
    }

    pub(in super::super) fn native_child_alias_publication_slot(
        &mut self,
        original: &[u8],
    ) -> Result<CommandSlot, NativeCommandLookupUnavailable> {
        let recipe = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?
            .recipe();
        let context = self.current_ns_id();
        let path = self.namespace_path_for_token(context);
        let selected = recipe
            .child_alias_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        let namespace = self.materialise_native_command_projection(context, &selected);
        Ok(CommandSlot {
            namespace,
            simple: selected.into_slot().simple,
        })
    }

    /// Resolve a `TclOO` declaration using its own name purpose and actual token context.
    pub(crate) fn native_object_publication_slot(
        &mut self,
        original: &[u8],
    ) -> Result<CommandSlot, NativeCommandLookupUnavailable> {
        let recipe = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?
            .recipe();
        let context = self.current_ns_id();
        let path = self.namespace_path_for_token(context);
        let selected = recipe
            .oo_object_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        let namespace = self.materialise_native_command_projection(context, &selected);
        Ok(CommandSlot {
            namespace,
            simple: selected.into_slot().simple,
        })
    }
}

impl Vm {
    /// Retain the binding created by this actual publication, before any caller can reselect it.
    pub(crate) fn define_proc_binding(
        &mut self,
        proc: ProcDef,
    ) -> (crate::command::NativeProcedureCommand, String) {
        self.define_proc_binding_with_jim_key(proc, None)
    }

    pub(crate) fn define_proc_binding_with_jim_key(
        &mut self,
        proc: ProcDef,
        table_key: Option<jim_table_keys::OriginalJimCommandKey>,
    ) -> (crate::command::NativeProcedureCommand, String) {
        self.define_proc_binding_with_jim_key_and_namespace_helper(proc, table_key, None)
    }

    pub(in super::super) fn define_proc_binding_with_jim_key_and_namespace_helper(
        &mut self,
        mut proc: ProcDef,
        table_key: Option<jim_table_keys::OriginalJimCommandKey>,
        helper: Option<tcl_native_model::native_jim_lookup::NativeJimNamespaceHelper>,
    ) -> (crate::command::NativeProcedureCommand, String) {
        if self.uses_native_jim_lookup()
            && proc.native_jim_namespace.is_none()
            && let Some(token) = proc.execution_namespace.context().resolved_token()
        {
            proc.native_jim_namespace = Some(self.new_jim_declaration_namespace(token));
        }
        let slot = CommandSlot {
            namespace: proc.command_ns_id,
            simple: proc.simple_name.clone(),
        };
        let binding =
            crate::command::NativeProcedureCommand::new_with_namespace_helper(proc, helper);
        let publication = self.register_command_in_slot_with_jim_key(
            slot,
            Command::Proc(binding.clone()),
            table_key,
        );
        (binding, publication)
    }
}
