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

//! Renaming a command: publishing the destination, retiring the source, the alias-loop probe, and retargeting imports and alias registrations.

use super::{
    AliasRenameProbe, CommandBinding, Namespaces, NsId, RenamePublication, RetiredCommandSlot,
    GLOBAL,
};
use crate::interp::Command;

impl Namespaces {
    /// Publish the destination half of `rename old new` — both names resolved
    /// relative to `current`, absolute when `::`-led — and leave the source's
    /// table entry standing. C's `TclRenameCommand` creates the destination
    /// hash entry, fires the `rename` traces, and only then deletes the source
    /// one, with *both* entries referencing the one `Command`; the caller
    /// closes that window with [`Self::retire_rename_source`]. `None` when
    /// `old` names no command (or `new` has no tail to bind).
    ///
    /// A command moved across namespaces is re-homed: a `Command::Proc`'s
    /// `ns`/`fqn` are updated to the destination so `namespace current` inside
    /// its body reports the new namespace, mirroring C's `cmdPtr->nsPtr`
    /// reassignment — and because C re-homes the shared `Command` before the
    /// traces run, the vacating name reports the *destination's* namespace for
    /// the callbacks' duration too (tclsh 8.6/9.0-pinned).
    ///
    /// Occupancy protection is the caller's job (see
    /// [`Self::destination_occupant_fqn`]) — a script-visible "already exists"
    /// refusal needs release-gate context (a TclOO root this release hides is
    /// not really taken) that this table-only layer does not have, so this
    /// unconditionally overwrites whatever is at the destination, same as
    /// [`Self::insert_bound`].
    ///
    /// Built-in protection lives in the `rename` builtin, not here — this is
    /// the pure table operation, as is the `rename old ""` deletion
    /// ([`Self::delete`]).
    #[cfg(test)]
    pub(crate) fn publish_rename_destination(
        &mut self,
        current: NsId,
        old: &[u8],
        new: &[u8],
    ) -> Option<RenamePublication> {
        self.publish_rename_destination_with_original_key(current, old, new, None)
    }

    pub(crate) fn publish_rename_destination_with_original_key(
        &mut self,
        current: NsId,
        old: &[u8],
        new: &[u8],
        original_jim_key: Option<std::rc::Rc<crate::obj::Owned>>,
    ) -> Option<RenamePublication> {
        let (old_ns, old_simple) = self.original_rename_source_of(current, old)?;
        // Unreachable for a non-empty, non-separator-terminated name; nothing
        // has moved yet, so there is nothing to put back.
        let (ns, simple) = self.destination_of(current, new)?;
        // C's `TclRenameCommand` creates the destination hash entry *before*
        // deleting the source's, so the transient extra entry can trigger a
        // rebuild a delete-first order would not.
        self.arena[ns].commands.reserve_entry(&simple);
        // SAFETY of expect: `home_of` reported the binding exists.
        let source_generation = self.arena[old_ns]
            .commands
            .generation(&old_simple)
            .expect("home_of reported a binding at the rename source");
        let cmd = self.arena[old_ns]
            .commands
            .get(&old_simple)
            .cloned()
            .expect("home_of reported a binding at the rename source");
        let destination_fqn = self.command_fqn(ns, &simple);
        let cmd = self.rehome_command(cmd, ns, &simple, &destination_fqn);
        // One command under two names, as C has it: the source slot takes the
        // re-homed binding too, and an ensemble token is shared outright.
        self.arena[old_ns]
            .commands
            .rebind_slot(&old_simple, cmd.clone());
        let jim_table_key = self.jim_command_table_key_at(
            current,
            new,
            tcl_syntax::naming::NativeNamePurpose::RenameDestination,
        );
        self.insert_moved_binding(
            ns,
            simple,
            CommandBinding {
                generation: source_generation,
                command: cmd,
                jim_table_key,
            },
            original_jim_key,
        );
        self.reset_native_shadowed_references(
            ns,
            &self.native_command_nodes[&source_generation]
                .placement
                .as_ref()
                .expect("published rename destination")
                .1
                .clone(),
        );
        self.invalidate_native_command_lookup(old_ns);
        self.invalidate_native_command_lookup(ns);
        Some(RenamePublication {
            source: (old_ns, old_simple),
            destination_fqn,
            generation: source_generation,
        })
    }

    /// Delete the source entry [`Self::publish_rename_destination_with_original_key`] left
    /// standing — C's `Tcl_DeleteHashEntry(oldHPtr)` at the tail of
    /// `TclRenameCommand`. A plain table removal: no `delete` trace fires and
    /// no token dies, because the command lives on under its new name. A
    /// callback may have removed or rebound the slot itself, so whatever is
    /// there goes and an already-vacated entry is not an error.
    pub(crate) fn retire_rename_source(&mut self, publication: &RenamePublication) {
        self.invalidate_original_publications();
        let (ns, simple) = &publication.source;
        if let Some(removed) = self.arena[*ns].commands.remove_binding(simple) {
            let replaced = removed.generation != publication.generation;
            if replaced {
                self.retire_native_command_node(removed.generation);
            }
            if self.variable_name_protocol != Some(tcl_syntax::naming::NativeNameProtocol::Jim084) {
                // The moved source view does not withdraw its surviving binding.
                // A callback-created different token owns a distinct Proc role.
                let procedure = if replaced {
                    match &removed.command {
                        Command::Proc(procedure) => procedure.take_for_retirement(),
                        _ => None,
                    }
                } else {
                    None
                };
                self.arena[*ns]
                    .commands
                    .retired
                    .borrow_mut()
                    .push(RetiredCommandSlot {
                        procedure,
                        original_jim_key: None,
                        binding: Some(removed),
                    });
            }
        }
        self.increment_native_command_epoch(publication.generation);
        if self.variable_name_protocol == Some(tcl_syntax::naming::NativeNameProtocol::Jim084) {
            self.advance_jim_procedure_epoch();
        }
    }

    /// The fully-qualified name of whatever is currently bound at the
    /// destination `rename old new` would write to, or `None` when it is
    /// free — read-only (beyond creating intermediate namespaces, same as
    /// C's `TCL_CREATE_NS_IF_UNKNOWN`, which `rename` re-resolves
    /// idempotently) so the `rename` builtin can decide occupancy (folding
    /// in release-gate context this layer does not have — see
    /// [`crate::interp::Interp::is_gate_hidden_object_root`]) *before* firing
    /// a rename trace or moving anything. `old`'s own binding still counts as
    /// occupying its slot here (it is not removed until the real `rename`
    /// call), so a same-slot self-rename (`rename foo foo`) reads as
    /// occupied too, matching tclsh 9.0.4.
    pub(crate) fn destination_occupant_fqn(
        &mut self,
        current: NsId,
        new: &[u8],
    ) -> Option<Vec<u8>> {
        if new.is_empty() {
            return None;
        }
        let (ns, simple) = self.destination_of(current, new)?;
        self.arena[ns]
            .commands
            .contains_key(&simple)
            .then(|| self.command_fqn(ns, &simple))
    }

    /// Re-point a command that carries its own binding identity at the site a
    /// `rename` moved it to — C's `cmdPtr->nsPtr` reassignment, which happens
    /// to the one shared `Command` and so is visible through *both* names for
    /// the `rename` traces' duration.
    ///
    /// A `Command::Proc` carries its home namespace and FQN, so `namespace
    /// current` inside its body reports the new namespace. A
    /// `Command::OoObject` carries a stable identity, so it deliberately passes
    /// through unchanged; `OoState` updates only its Tcl-facing name projection.
    /// Every other variant carries no site of its own and passes through unchanged.
    fn rehome_command(&mut self, command: Command, ns: NsId, simple: &[u8], fqn: &[u8]) -> Command {
        if let Command::Proc(definition) = &command {
            let declaration = definition.declaration();
            let mut location = declaration.location();
            location.qualified_name = fqn.to_vec();
            if let Some(protocol) = self
                .variable_name_protocol
                .filter(|protocol| protocol.is_jim084())
            {
                if let Some(bytes) = protocol
                    .jim_procedure_relocation_namespace(simple)
                    .expect("selected Jim relocation recipe")
                {
                    let holder = crate::obj::Owned::fresh(crate::obj::new_string_bytes(bytes));
                    let namespace = self.retain_jim_namespace(holder, std::rc::Rc::from(bytes));
                    location.namespace = namespace;
                    location.jim_namespace = self.take_jim_namespace_owner(namespace);
                }
            } else {
                location.namespace = ns;
            }
            declaration.relocate(location);
        }
        command
    }

    /// Select the original rename operand, then retain the token's current slot.
    /// During a C rename callback both entries refer to that same token.
    pub(crate) fn original_rename_source_of(
        &self,
        current: NsId,
        old: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        if self
            .variable_name_protocol
            .is_some_and(|protocol| protocol.is_jim084())
        {
            return self.original_jim_rename_source_of(current, old);
        }
        let (holder, simple) = self.home_of(current, old)?;
        let generation = self.arena[holder].commands.generation(&simple)?;
        self.native_command_slot_at_node(generation)
    }

    /// Jim rename uses its original qualified source key without global fallback.
    pub(crate) fn original_jim_rename_source_of(
        &self,
        current: NsId,
        old: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self
            .variable_name_protocol
            .filter(|protocol| protocol.is_jim084())?;
        let selected = protocol
            .rename_source_input(self.jim_context(current)?, old)
            .ok()?;
        let key = selected.jim_flat_key()?;
        self.arena[GLOBAL]
            .commands
            .contains_key(key)
            .then(|| (GLOBAL, key.to_vec()))
    }

    fn destination_of(&mut self, current: NsId, new: &[u8]) -> Option<(NsId, Vec<u8>)> {
        if let Some(protocol) = self
            .variable_name_protocol
            .filter(|protocol| protocol.is_jim084())
        {
            let selected = protocol
                .rename_destination_input(self.jim_context(current)?, new)
                .ok()?;
            return Some((GLOBAL, selected.jim_flat_key()?.to_vec()));
        }
        let protocol = self.variable_name_protocol?;
        let path = self.native_context_path(current)?;
        let selected = protocol
            .rename_destination_projection(tcl_syntax::naming::NativeNameContext::new(&path), new)
            .ok()?;
        let mut namespace = self.command_projection_base(current, &selected);
        for part in selected.qualifiers() {
            namespace = self.ensure_child(namespace, part.as_bytes());
        }
        Some((namespace, selected.slot().simple.as_bytes().to_vec()))
    }

    /// Place only the tentative destination for C's alias-loop check. The
    /// caller runs original target getters after releasing this table borrow.
    /// The old entry still refers to the actual same command token.
    pub(crate) fn begin_alias_rename_probe(
        &mut self,
        current: NsId,
        old: &[u8],
        new: &[u8],
    ) -> Option<AliasRenameProbe> {
        if new.is_empty()
            || self
                .variable_name_protocol
                .is_some_and(|protocol| protocol.is_jim084())
        {
            return None;
        }
        let (old_ns, old_simple) = self.original_rename_source_of(current, old)?;
        let binding = self.arena[old_ns].commands.binding(&old_simple)?.clone();
        if !matches!(binding.command, Command::Alias { .. }) {
            return None;
        }
        let (destination_ns, destination_simple) = self.destination_of(current, new)?;
        if destination_ns == old_ns && destination_simple == old_simple {
            return None;
        }
        self.invalidate_original_publications();
        let displaced = self.arena[destination_ns]
            .commands
            .restore_slot(destination_simple.clone(), binding.clone());
        Some(AliasRenameProbe {
            destination: (destination_ns, destination_simple),
            binding,
            displaced,
        })
    }

    /// Restore only the actual tentative slot. A reached getter may have
    /// replaced it; such a Guest mutation is retained. Returned owners must
    /// retire after the namespace borrow ends.
    pub(crate) fn restore_alias_rename_probe(
        &mut self,
        probe: AliasRenameProbe,
    ) -> Vec<CommandBinding> {
        self.invalidate_original_publications();
        let (namespace, simple) = probe.destination;
        let mut retired = Vec::new();
        let owns_probe = self.arena[namespace]
            .commands
            .binding(&simple)
            .is_some_and(|binding| {
                binding.generation == probe.binding.generation
                    && binding.command.is_same_binding(&probe.binding.command)
            });
        if owns_probe {
            if let Some(binding) = self.arena[namespace].commands.take_slot(&simple) {
                retired.push(binding);
            }
            if let Some(displaced) = probe.displaced {
                if let Some(binding) = self.arena[namespace]
                    .commands
                    .restore_slot(simple, displaced)
                {
                    retired.push(binding);
                }
            } else {
                self.arena[namespace].commands.drop_entry(&simple);
            }
        } else if let Some(displaced) = probe.displaced {
            retired.push(displaced);
        }
        retired.push(probe.binding);
        retired
    }

    /// Rewrite redirects retaining one exact source generation. The FQN is a
    /// projection only: retained and recreated namespace tokens may expose the
    /// same spelling simultaneously, so it cannot identify the imports to move.
    pub fn retarget_imports(&mut self, source_generation: u64, new_fqn: &[u8]) {
        for ns in &mut self.arena {
            for cmd in ns.commands.values_mut() {
                if let Command::Imported {
                    source,
                    source_generation: candidate,
                    ..
                } = cmd
                {
                    if *candidate == source_generation {
                        *source = new_fqn.to_vec();
                    }
                }
            }
        }
    }

    /// Attach by-name fallback redirects to the fresh token that replaced
    /// their now-retired source generation. A simultaneously retained token
    /// with the same FQN remains live and is deliberately left alone.
    pub(crate) fn reattach_missing_imports(
        &mut self,
        source_fqn: &[u8],
        new_generation: u64,
        live_generations: &std::collections::HashSet<u64>,
    ) {
        for ns in &mut self.arena {
            for command in ns.commands.values_mut() {
                let Command::Imported {
                    source,
                    source_generation,
                    ..
                } = command
                else {
                    continue;
                };
                if source == source_fqn && !live_generations.contains(source_generation) {
                    *source_generation = new_generation;
                }
            }
        }
    }

    /// Attach imports of one exact source generation to its ensemble token.
    /// This changes alias metadata only; same-FQN imports of another retained
    /// namespace generation remain untouched.
    pub(crate) fn retarget_imports_to_ensemble(
        &mut self,
        source_generation: u64,
        new: &std::rc::Rc<crate::ensemble::EnsembleToken>,
    ) {
        for ns in &mut self.arena {
            for command in ns.commands.values_mut() {
                let Command::Imported {
                    source_generation: candidate,
                    ensemble,
                    ..
                } = command
                else {
                    continue;
                };
                if *candidate == source_generation {
                    *ensemble = Some(std::rc::Rc::clone(new));
                }
            }
        }
    }

    pub(crate) fn replace_alias_registration(
        &mut self,
        generation: u64,
        original: &std::rc::Rc<crate::interp::NativeAliasHeader>,
        replacement: &std::rc::Rc<crate::interp::NativeAliasHeader>,
        report: &[u8],
    ) -> bool {
        for namespace in &mut self.arena {
            for slot in namespace.commands.entries.values_mut() {
                let Some(binding) = slot
                    .binding
                    .as_mut()
                    .filter(|binding| binding.generation == generation)
                else {
                    continue;
                };
                let (registration_name, publication_name) = match &mut binding.command {
                    Command::Alias {
                        registration_name,
                        publication_name,
                        ..
                    }
                    | Command::ParentAlias {
                        registration_name,
                        publication_name,
                        ..
                    } => (registration_name, publication_name),
                    _ => return false,
                };
                if !registration_name
                    .as_ref()
                    .is_some_and(|actual| std::rc::Rc::ptr_eq(actual, original))
                {
                    return false;
                }
                *registration_name = Some(std::rc::Rc::clone(replacement));
                *publication_name = report.to_vec();
                return true;
            }
        }
        false
    }

    /// Selected alias-record publication names, independently of lookup slots.
    /// Original C interp aliases preserves its registration spelling; command
    /// inventory has its own normalized reporting and lookup purpose.
    #[must_use]
    pub fn alias_names(&self) -> Vec<Vec<u8>> {
        self.arena
            .iter()
            .flat_map(|namespace| namespace.commands.iter())
            .filter_map(|(_, command)| match command {
                Command::Alias {
                    publication_name, ..
                }
                | Command::ParentAlias {
                    publication_name, ..
                } => Some(publication_name.clone()),
                _ => None,
            })
            .collect()
    }
}
