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

//! State behind the `namespace` command: exports and imports, binding, hierarchy, command generations, unknown handlers and stored identities.

use super::{CommandBinding, Namespaces, NativeCommandCreation, NsId};
use crate::interp::Command;
use std::collections::BTreeSet;

impl Namespaces {
    // the `namespace` command surface

    /// The fully-qualified name a command `name` resolves to from `current`
    /// (`namespace which -command`), or `None` if it doesn't resolve.
    #[must_use]
    pub fn which_command(&self, current: NsId, name: &[u8]) -> Option<Vec<u8>> {
        let (namespace, simple) = self.home_of(current, name)?;
        Some(self.command_fqn(namespace, &simple))
    }

    /// `namespace export` — append a pattern (deduplicated). `-clear` first is the
    /// caller's job via [`clear_exports`](Self::clear_exports).
    pub fn export(&mut self, ns: NsId, pattern: &[u8]) {
        if !self.arena[ns].exports.iter().any(|p| p == pattern) {
            self.note_guard_command_lookup_policy_mutation();
            self.arena[ns].exports.push(pattern.to_vec());
            self.advance_ensemble_export_epoch(ns);
        }
    }

    /// Drop all of `ns`'s export patterns (`namespace export -clear`).
    pub fn clear_exports(&mut self, ns: NsId) {
        if !self.arena[ns].exports.is_empty() {
            self.note_guard_command_lookup_policy_mutation();
            self.advance_ensemble_export_epoch(ns);
        }
        self.arena[ns].exports.clear();
    }

    /// `ns`'s export patterns (`namespace export` with no args).
    #[must_use]
    pub fn exports(&self, ns: NsId) -> &[Vec<u8>] {
        &self.arena[ns].exports
    }

    /// The sorted command names in `ns` that match its export patterns — the
    /// default subcommand set of an ensemble over `ns` (`namespace ensemble`).
    #[must_use]
    pub fn exported_commands(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.arena[ns]
            .commands
            .keys()
            .filter(|k| self.is_exported(ns, k))
            .cloned()
            .collect()
    }

    /// Does `name` match any of `ns`'s export patterns (`string match` glob)?
    #[must_use]
    pub fn is_exported(&self, ns: NsId, name: &[u8]) -> bool {
        let Ok(name_s) = core::str::from_utf8(name) else {
            return false;
        };
        self.arena[ns].exports.iter().any(|pat| {
            core::str::from_utf8(pat).is_ok_and(|p| tcl_syntax::glob::string_match(p, name_s))
        })
    }

    /// Bind `command` under simple `name` directly in namespace `ns` (no
    /// qualified parsing — `import` inserting a redirect into the importing ns).
    pub fn bind(&mut self, ns: NsId, name: &[u8], command: Command) {
        self.insert_bound(ns, name.to_vec(), command);
    }

    /// Take only the actual object record at the original occupied creation
    /// slot. The generation identifies the callback-created binding; no name
    /// resolution or a recreated namespace holder supplies this permission.
    pub(crate) fn take_recreated_object_for_creation(
        &mut self,
        entry: &NativeCommandCreation,
        name: &[u8],
        generation: u64,
    ) -> Option<CommandBinding> {
        if !entry.occupied || entry.original_tail != name {
            return None;
        }
        let binding = self.arena.get(entry.namespace)?.commands.binding(name)?;
        if binding.generation != generation
            || !matches!(&binding.command, Command::ObjCmd(command)
                if command.creation_purpose() == Some(tcl_native_model::native_command_creation::NativeObjectCommandCreationPurpose::LegacyPublicObjectCommand))
        {
            return None;
        }
        // The owned binding is returned before its record can be retired;
        // callers release it outside the namespace RefCell borrow. Jim's
        // Jim_ReplaceHashEntry overwrites only the value of an occupied key,
        // including a value recreated by the original delete procedure.
        // The shared replacement worker preserves that key; C removes its
        // independent entry using the existing selected replacement recipe.
        if self
            .variable_name_protocol
            .is_some_and(|protocol| protocol.is_jim084())
        {
            self.take_native_binding_for_replacement(entry.namespace, name)
        } else {
            // This separately admitted ObjCmd must remain uniquely transferred:
            // its disposal purpose does not execute a second delete procedure.
            self.unlink_native_binding(entry.namespace, name)
        }
    }

    /// Consume the original creation entry after its callbacks have returned.
    pub(crate) fn bind_after_native_creation_entry(
        &mut self,
        entry: NativeCommandCreation,
        name: &[u8],
        command: Command,
    ) -> u64 {
        self.insert_bound_after_entry(entry, name.to_vec(), command)
    }

    /// `ns`'s `namespace path` (the resolver's step-b list).
    #[must_use]
    pub fn path(&self, ns: NsId) -> &[NsId] {
        &self.arena[ns].path
    }

    /// `ns`'s parent (`namespace parent`); `None` only for the global root.
    #[must_use]
    pub fn parent(&self, ns: NsId) -> Option<NsId> {
        self.arena[ns].parent
    }

    /// `ns`'s direct child namespaces (`namespace children`).
    #[must_use]
    pub fn children(&self, ns: NsId) -> Vec<NsId> {
        let mut children: Vec<NsId> = self.arena[ns].children.values().copied().collect();
        children.sort_unstable();
        children
    }

    /// Probe the actual child table without parsing the original member key.
    pub(crate) fn child_token(&self, parent: NsId, member: &[u8]) -> Option<NsId> {
        self.arena.get(parent)?.children.get(member).copied()
    }

    /// `ns`'s live children in Tcl string-hash enumeration order.
    #[must_use]
    pub fn children_hash_order(&self, ns: NsId) -> Vec<NsId> {
        self.arena[ns]
            .child_order
            .keys()
            .into_iter()
            .filter_map(|name| self.arena[ns].children.get(name).copied())
            .collect()
    }

    /// `ns`'s live command slots as `(simple name, generation)` in Tcl
    /// string-hash enumeration order — the snapshot `TclTeardownNamespace`
    /// takes of `cmdTable` before deleting each token.
    pub(crate) fn command_hash_order(&self, ns: NsId) -> Vec<(Vec<u8>, u64)> {
        self.arena[ns].commands.hash_order()
    }

    /// The token generation currently bound at `(ns, name)`. A teardown
    /// snapshot compares it before firing: a different generation means a
    /// delete callback replaced the token, so the replacement belongs to the
    /// next pass (C's `Tcl_DeleteCommandFromToken` returns early on
    /// `CMD_DYING`).
    pub(crate) fn command_generation(&self, ns: NsId, name: &[u8]) -> Option<u64> {
        self.arena[ns].commands.generation(name)
    }

    /// The generation of the command token `name` resolves to from `current`
    /// — the token identity a command trace is registered against, and the one
    /// a deletion frees the trace list of.
    pub(crate) fn resolve_generation(&self, current: NsId, name: &[u8]) -> Option<u64> {
        let (ns, simple) = self.home_of(current, name)?;
        self.arena[ns].commands.generation(&simple)
    }

    /// Share actual node and retained-table traversal across result projections.
    fn command_by_generation_using<T>(
        &self,
        generation: u64,
        native_project: impl Fn(&Command, NsId, &[u8]) -> T,
        table_project: impl Fn(&Command, NsId, &[u8]) -> T,
    ) -> Option<T> {
        if let Some(selected) = self.native_command_at_node_using(generation, native_project) {
            return Some(selected);
        }
        self.arena.iter().enumerate().find_map(|(ns, node)| {
            node.commands.entries.iter().find_map(|(name, slot)| {
                let binding = slot.binding.as_ref()?;
                (binding.generation == generation)
                    .then(|| table_project(&binding.command, ns, name))
            })
        })
    }

    /// Resolve an interpreter-unique command generation in any visible or
    /// retained namespace table, including its current Tcl-facing location.
    pub(crate) fn command_by_generation(&self, generation: u64) -> Option<(Vec<u8>, Command)> {
        if let Some((command, namespace, simple)) = self.retained_jim_command_at_node(generation) {
            let command = command.clone();
            return Some((self.command_fqn(namespace, simple), command));
        }
        self.command_by_generation_using(
            generation,
            |command, namespace, simple| {
                let command = command.clone();
                (self.command_fqn(namespace, simple), command)
            },
            |command, namespace, simple| (self.command_fqn(namespace, simple), command.clone()),
        )
    }

    /// Report the same generation's actual location without retaining its callable.
    pub(crate) fn command_fqn_by_generation(&self, generation: u64) -> Option<Vec<u8>> {
        self.command_by_generation_using(
            generation,
            |_, namespace, simple| self.command_fqn(namespace, simple),
            |_, namespace, simple| self.command_fqn(namespace, simple),
        )
    }

    /// Every command generation currently resident in a visible or retained
    /// namespace table.
    pub(crate) fn command_generations(&self) -> std::collections::HashSet<u64> {
        self.arena
            .iter()
            .flat_map(|node| node.commands.entries.values())
            .filter_map(|slot| slot.binding.as_ref().map(|binding| binding.generation))
            .collect()
    }

    /// The fully-qualified name of the binding at `(ns, name)`.
    pub(crate) fn command_fqn_at(&self, ns: NsId, name: &[u8]) -> Vec<u8> {
        self.command_fqn(ns, name)
    }

    /// `ns`'s `namespace unknown` handler, if one is set (an empty/`None` handler
    /// means "use the interpreter default `::unknown`").
    #[must_use]
    pub(crate) fn unknown_handler(&self, ns: NsId) -> Option<*mut crate::obj::TclObj> {
        self.arena[ns]
            .unknown
            .as_ref()
            .map(crate::obj::Owned::as_ptr)
    }

    /// Transfer the original handler root; release the retired root after the
    /// namespace borrow ends so native free hooks may re-enter the interpreter.
    pub(crate) fn set_unknown_handler(
        &mut self,
        ns: NsId,
        handler: Option<crate::obj::Owned>,
    ) -> Option<crate::obj::Owned> {
        if self.arena[ns].unknown.is_some() || handler.is_some() {
            self.note_guard_command_lookup_policy_mutation();
        } else {
            self.note_guard_lookup_environment_mutation();
        }
        std::mem::replace(&mut self.arena[ns].unknown, handler)
    }

    /// Find every ensemble command whose configured namespace is in `victims`,
    /// returning each command's fully-qualified name and stable token.
    /// An ensemble command is tied to its namespace, so deleting the namespace
    /// deletes the command — even when the command itself lives elsewhere (e.g.
    /// `::ns` in the global table for an ensemble created inside `ns`). Mirrors
    /// C's ensemble namespace-deletion hook.
    pub(crate) fn ensembles_for(
        &self,
        victims: &std::collections::HashSet<NsId>,
    ) -> Vec<(Vec<u8>, std::rc::Rc<crate::ensemble::EnsembleToken>)> {
        let mut hits = Vec::new();
        for (id, node) in self.arena.iter().enumerate() {
            for (name, cmd) in node.commands.iter() {
                if let Command::Ensemble(token) = cmd {
                    if victims.contains(&token.config().ns) {
                        hits.push((self.command_fqn(id, name), std::rc::Rc::clone(token)));
                    }
                }
            }
        }
        hits
    }

    /// Remove one ensemble command by stable token identity wherever a delete
    /// callback may have renamed it, returning that binding's live FQN. A
    /// replacement at the old name carries a different token and survives.
    pub(crate) fn remove_ensemble_identity(
        &mut self,
        identity: &std::rc::Rc<crate::ensemble::EnsembleToken>,
    ) -> Option<(Vec<u8>, u64)> {
        let mut found = None;
        for (ns, node) in self.arena.iter().enumerate() {
            if let Some(name) = node.commands.iter().find_map(|(name, command)| {
                matches!(
                    command,
                    Command::Ensemble(current)
                        if std::rc::Rc::ptr_eq(current, identity)
                )
                .then(|| name.clone())
            }) {
                found = Some((ns, name));
                break;
            }
        }
        let (ns, name) = found?;
        let binding = self.remove_native_binding(ns, &name)?;
        Some((self.command_fqn(ns, &name), binding.generation))
    }

    /// Locate one ensemble command by stable identity, including the exact
    /// command generation that owns its trace sidecars.
    pub(crate) fn ensemble_identity_location(
        &self,
        identity: &std::rc::Rc<crate::ensemble::EnsembleToken>,
    ) -> Option<(Vec<u8>, u64)> {
        self.arena.iter().enumerate().find_map(|(ns, node)| {
            node.commands.iter().find_map(|(name, command)| {
                matches!(
                    command,
                    Command::Ensemble(current)
                        if std::rc::Rc::ptr_eq(current, identity)
                )
                .then(|| {
                    (
                        self.command_fqn(ns, name),
                        node.commands
                            .generation(name)
                            .expect("iterated command has a generation"),
                    )
                })
            })
        })
    }

    /// Imported aliases whose immediate source generation is in `origins` or
    /// whose retained ensemble identity is in `tokens`. Each result includes the
    /// import binding's stable identity so callers can fire delete traces while
    /// it is still visible, then remove only that original command after any
    /// reentrant replacement performed by the callback.
    pub(crate) fn imports_for_origins(
        &self,
        origins: &std::collections::HashSet<u64>,
        tokens: &[std::rc::Rc<crate::ensemble::EnsembleToken>],
    ) -> Vec<(Vec<u8>, std::rc::Rc<crate::interp::ImportToken>)> {
        let mut hits = Vec::new();
        for (id, node) in self.arena.iter().enumerate() {
            for (name, command) in node.commands.iter() {
                let Command::Imported {
                    source_generation,
                    ensemble,
                    identity,
                    ..
                } = command
                else {
                    continue;
                };
                let retains_token = ensemble.as_ref().is_some_and(|imported| {
                    tokens
                        .iter()
                        .any(|victim| std::rc::Rc::ptr_eq(imported, victim))
                });
                if origins.contains(source_generation) || retains_token {
                    hits.push((self.command_fqn(id, name), std::rc::Rc::clone(identity)));
                }
            }
        }
        hits
    }

    /// Remove the imported command identified by `identity` wherever a
    /// delete-trace callback may have renamed it, returning that binding's live
    /// FQN. A replacement has a fresh identity and therefore survives.
    pub(crate) fn remove_import_identity(
        &mut self,
        identity: &std::rc::Rc<crate::interp::ImportToken>,
    ) -> Option<(Vec<u8>, u64)> {
        let mut found = None;
        for (ns, node) in self.arena.iter().enumerate() {
            if let Some(name) = node.commands.iter().find_map(|(name, command)| {
                matches!(
                    command,
                    Command::Imported { identity: current, .. }
                        if std::rc::Rc::ptr_eq(current, identity)
                )
                .then(|| name.clone())
            }) {
                found = Some((ns, name));
                break;
            }
        }
        let (ns, name) = found?;
        let binding = self.remove_native_binding(ns, &name)?;
        Some((self.command_fqn(ns, &name), binding.generation))
    }

    /// Locate one imported command by stable identity, including the exact
    /// command generation that owns its trace sidecars.
    pub(crate) fn import_identity_location(
        &self,
        identity: &std::rc::Rc<crate::interp::ImportToken>,
    ) -> Option<(Vec<u8>, u64)> {
        self.arena.iter().enumerate().find_map(|(ns, node)| {
            node.commands.iter().find_map(|(name, command)| {
                matches!(
                    command,
                    Command::Imported { identity: current, .. }
                        if std::rc::Rc::ptr_eq(current, identity)
                )
                .then(|| {
                    (
                        self.command_fqn(ns, name),
                        node.commands
                            .generation(name)
                            .expect("iterated command has a generation"),
                    )
                })
            })
        })
    }

    /// The reporting `(simple_name, source_fqn)` of every imported redirect in `ns`.
    #[must_use]
    pub fn imported_in(&self, ns: NsId) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.arena[ns]
            .commands
            .iter()
            .filter_map(|(k, c)| match c {
                Command::Imported {
                    source, ensemble, ..
                } => Some((
                    k.clone(),
                    ensemble
                        .as_ref()
                        .filter(|token| !token.is_deleted())
                        .map_or_else(|| source.clone(), |token| token.name()),
                )),
                _ => None,
            })
            .collect()
    }

    fn original_import_origin_slot(&self, mut generation: u64) -> Option<(NsId, Vec<u8>)> {
        let mut visited = BTreeSet::new();
        loop {
            if !visited.insert(generation) {
                return None;
            }
            let (namespace, simple) = self.native_command_slot_at_node(generation)?;
            match self.arena[namespace].commands.get(&simple)? {
                Command::Imported {
                    source_generation, ..
                } => generation = *source_generation,
                _ => return Some((namespace, simple)),
            }
        }
    }

    pub(crate) fn imported_forget_bindings(
        &self,
        namespace: NsId,
        source: Option<NsId>,
        pattern: &[u8],
        matcher: tcl_syntax::native_glob::NativeGlobProtocol,
    ) -> Result<Vec<(Vec<u8>, u64)>, tcl_syntax::value::ValueError> {
        use tcl_syntax::native_glob::NativeNameGlobPurpose;
        use tcl_syntax::value::ValueError;

        let mut bindings = Vec::new();
        for (simple, command) in self.arena[namespace].commands.iter() {
            let Command::Imported {
                source_generation, ..
            } = command
            else {
                continue;
            };
            let matches = if let Some(source) = source {
                let (origin_namespace, origin_tail) = self
                    .original_import_origin_slot(*source_generation)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "namespace forget original import origin",
                    ))?;
                let (match_namespace, match_tail) = if origin_namespace == source {
                    (origin_namespace, origin_tail)
                } else {
                    self.native_command_slot_at_node(*source_generation).ok_or(
                        ValueError::CommandProtocolUnavailable(
                            "namespace forget immediate import source",
                        ),
                    )?
                };
                match_namespace == source
                    && matcher
                        .match_name_pattern(
                            NativeNameGlobPurpose::ForgetOriginFilter,
                            pattern,
                            &match_tail,
                        )
                        .map_err(|_| {
                            ValueError::CommandProtocolUnavailable("namespace forget origin filter")
                        })?
            } else {
                matcher
                    .match_name_pattern(NativeNameGlobPurpose::ForgetOwnSearch, pattern, simple)
                    .map_err(|_| {
                        ValueError::CommandProtocolUnavailable("namespace forget own-name search")
                    })?
            };
            if matches {
                let generation = self.arena[namespace].commands.generation(simple).ok_or(
                    ValueError::CommandProtocolUnavailable("namespace forget import binding"),
                )?;
                bindings.push((simple.clone(), generation));
            }
        }
        Ok(bindings)
    }

    /// Remove the simple-named command `name` directly from `ns` (no resolution
    /// walk); returns whether it existed. For `namespace forget`.
    pub fn remove_in(&mut self, ns: NsId, name: &[u8]) -> bool {
        self.remove_native_binding(ns, name).is_some_and(|binding| {
            if matches!(&binding.command, Command::Imported { .. }) {
                self.note_guard_command_lookup_policy_mutation();
            }
            Self::mark_command_deleted(&binding.command, &self.retired_ensemble_roles);
            true
        })
    }
}
