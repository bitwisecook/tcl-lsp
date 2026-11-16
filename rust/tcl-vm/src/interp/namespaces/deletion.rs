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

//! Deleting a namespace: retiring its commands and ensembles in Tcl's order, variable teardown, deferral while a frame is active, and retained tokens.

use super::super::commands::CommandSidecarKey;
use super::super::{NamespacePath, RetainedNamespace, Vm, variable_cleanup};
use super::{retained_children_of, root_hash_order};
use crate::command::{Command, EnsembleDef};
use crate::value::Value;
use std::rc::Rc;
use tcl_core_types::NameBytes;
use tcl_runtime_api::{Completion, Namespaces, NsId, ROOT_NS};

impl Vm {
    /// Unlink one ensemble token from its implementation namespace's ordered
    /// ownership list. This is the native counterpart of
    /// `DeleteEnsembleConfig` splicing `EnsembleConfig` from `nsPtr->ensembles`.
    pub(in super::super) fn untrack_namespace_ensemble(
        &mut self,
        token: &Rc<tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, NameBytes>>,
    ) {
        self.namespace_ensembles.retain(|_, tokens| {
            tokens.retain(|candidate| {
                candidate
                    .upgrade()
                    .is_some_and(|live| !Rc::ptr_eq(&live, token))
            });
            !tokens.is_empty()
        });
    }

    /// Find the real command-table location currently carrying an ensemble
    /// token. Imported bindings share the same `Rc` and are deliberately
    /// excluded: the namespace list owns the source token, never its imports.
    pub(in super::super) fn ensemble_lifecycle_key(
        &self,
        token: &Rc<tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, NameBytes>>,
    ) -> Option<CommandSidecarKey> {
        self.commands
            .iter()
            .find_map(|(key, command)| {
                (!self.imported_commands.contains_key(key)
                    && matches!(command, Command::Ensemble(live) if Rc::ptr_eq(live, token)))
                .then(|| CommandSidecarKey::visible(key))
            })
            .or_else(|| {
                self.hidden.commands.iter().find_map(|(key, command)| {
                    (!self.hidden.imports.contains_key(key)
                        && matches!(command, Command::Ensemble(live) if Rc::ptr_eq(live, token)))
                    .then(|| CommandSidecarKey::hidden(key))
                })
            })
    }

    /// Retire one namespace's snapshotted command tokens in the order
    /// `TclTeardownNamespace` took them. Namespace-owned ensembles live in
    /// their own ordered list and are drained separately.
    ///
    /// Each entry is addressed by `(key, generation)`: a callback fired by an
    /// earlier entry may have deleted this one (C's snapshot then holds a
    /// `CMD_DYING` token and `Tcl_DeleteCommandFromToken` returns early) or
    /// replaced it with a distinct token the current snapshot does not name.
    /// Both skip; the next snapshot picks a replacement up.
    ///
    /// Returns how many snapshotted tokens were still current when the loop
    /// reached them, so the caller can stop rather than spin if a pass turns
    /// out to reach nothing at all.
    fn retire_namespace_command_bindings(&mut self, snapshot: &[(String, u64)]) -> usize {
        let mut retired = 0;
        for (key, generation) in snapshot {
            if self.live_command_generation(key) != Some(*generation) {
                continue;
            }
            retired += 1;
            // An import merely living inside the deleted namespace is removed
            // at its own hash position and never marks its shared source token
            // dead; a real command traverses the complete source lifecycle, so
            // its delete traces, backrefs and sidecars retire exactly once and
            // cannot attach to a later command at the same name.
            let key = CommandSidecarKey::visible(key);
            if self.imported_commands.contains_key(key.name()) {
                self.retire_import_binding(&key);
            } else {
                self.retire_command_lifecycle_key(&key);
            }
        }
        retired
    }

    /// Drain one namespace's ensemble-token list before that namespace is
    /// marked dying. The vector's back is C's list head. Pop it before firing
    /// the delete callback: a callback-created token is pushed there and is
    /// therefore the next head, ahead of every older token.
    fn retire_namespace_owned_ensembles(&mut self, namespace: NsId) {
        loop {
            let mut remove_entry = false;
            let token = self
                .namespace_ensembles
                .get_mut(&namespace)
                .and_then(|tokens| {
                    loop {
                        let Some(candidate) = tokens.pop() else {
                            remove_entry = true;
                            break None;
                        };
                        if let Some(token) = candidate.upgrade()
                            && !token.is_deleted()
                        {
                            break Some(token);
                        }
                    }
                });
            if remove_entry {
                self.namespace_ensembles.remove(&namespace);
            }
            let Some(token) = token else {
                break;
            };
            if let Some(key) = self.ensemble_lifecycle_key(&token) {
                self.retire_command_lifecycle_key(&key);
            } else {
                // Defensive parity with a source token that was already
                // unlinked from the command table: it cannot stay live merely
                // because an imported spelling still holds the shared `Rc`.
                token.mark_deleted();
            }
        }
    }

    pub(in super::super) fn delete_namespace_path(&mut self, path: &NamespacePath) -> bool {
        let deleting_root = path.is_empty();
        if !deleting_root && !self.name_world.borrow().namespaces.contains(path) {
            return false;
        }
        let id = self
            .name_world
            .borrow()
            .ns_intern
            .get(path)
            .copied()
            .unwrap_or(ROOT_NS);
        // Empty namespaces can still change qualified lookup and path
        // resolution. Publish deletion even when no command retirement below
        // would otherwise advance the compilation world.
        self.bump_cmd_epoch();
        self.invalidate_compiled_command_semantics();
        self.invalidate_namespace_structure_guards();
        self.delete_namespace_token(id, deleting_root);
        true
    }

    /// Execute the globals-unset list authored by Jim's namespace-delete helper.
    /// The native globals scan receives the canonical object as a glob pattern.
    pub(crate) fn delete_jim_namespace_globals_pattern(
        &mut self,
        canonical: &[u8],
    ) -> Result<(), Completion<Value>> {
        use tcl_syntax::native_glob::{NativeGlobProtocol, NativeNameGlobPurpose};
        let Some(policy) = self.name_policy_protocol() else {
            return Err(self
                .refuse_host_command("Jim namespace delete naming issuer is unavailable".into()));
        };
        if !policy.recipe().is_jim084() {
            return Err(self.refuse_host_command(
                "Jim namespace delete globals protocol is unavailable".into(),
            ));
        }
        let mut pattern = canonical.to_vec();
        pattern.extend_from_slice(b"::*");
        let matcher = NativeGlobProtocol::from_name_policy(policy);
        let names = Namespaces::vars_in_bytes(self, ROOT_NS);
        let mut selected = Vec::new();
        for name in names {
            match matcher.match_name_pattern(
                NativeNameGlobPurpose::InfoVariablesScan,
                &pattern,
                &name,
            ) {
                Ok(true) => selected.push(name),
                Ok(false) => {}
                Err(error) => return Err(self.refuse_host_command(error.to_string())),
            }
        }
        for name in selected {
            let original = tcl_syntax::naming::root_unrooted_key_bytes(&name);
            self.unset_one_bytes(&original, true)?;
        }
        Ok(())
    }

    /// Destroy one namespace's variable cells in Tcl's trace-visible order.
    ///
    /// `TclDeleteNamespaceVars` removes one old cell, fires its unset traces,
    /// then forcibly purges any same-name value or traces recreated by those
    /// callbacks before selecting the next cell. A callback-created *different*
    /// variable remains in the table and receives its own teardown pass.
    pub(in super::super) fn teardown_namespace_variables(
        &mut self,
        canonical: impl AsRef<[u8]>,
        id: NsId,
    ) {
        let original = variable_cleanup::OriginalVariableCleanup::enter(self);
        original.namespace(self, canonical.as_ref(), id);
    }

    /// Tear down one exact namespace token in Tcl's recursive order. C drains
    /// `nsPtr->ensembles` while the token is live, marks this namespace dying,
    /// tears down this token's commands and state, and only then recursively
    /// invokes `Tcl_DeleteNamespace` for each child. In particular, deleting a
    /// parent must not bulk-mark descendants dead before their own ensemble
    /// callbacks run.
    pub(in super::super) fn delete_namespace_token(&mut self, id: NsId, deleting_root: bool) {
        let command_traversal = self
            .name_policy_protocol()
            .and_then(|policy| policy.recipe().namespace_command_teardown());
        self.delete_namespace_token_using(id, deleting_root, command_traversal);
    }

    pub(in super::super) fn delete_namespace_token_using(
        &mut self,
        id: NsId,
        deleting_root: bool,
        command_traversal: Option<
            tcl_syntax::native_namespace_name::NativeNamespaceCommandTeardown,
        >,
    ) {
        let native_error_reset =
            deleting_root && !self.variable_observers.native_error_cells.is_empty();
        if native_error_reset {
            self.suspend_native_error_variable_traces();
        }
        let path = self.ns_path(id);
        let canonical = self.ns_name_bytes(id);
        // `Tcl_CreateEnsemble` pushes a new config at the live list head. Pop
        // and retire one head at a time so an ensemble created by that head's
        // delete callback is visited next, ahead of every older token.
        self.retire_namespace_owned_ensembles(id);
        let retired = self.ns_unknowns.remove(&id);
        drop(retired);

        // A frame is still running in this token: unpublish it now and keep
        // everything else for that frame (C's `activationCount` branch).
        if !deleting_root && self.defer_namespace_token(id) {
            return;
        }

        // `TclTeardownNamespace` makes only this exact token non-existent. Its
        // commands remain table-visible while their delete callbacks run, and
        // existing child namespace tokens remain live until recursion reaches
        // each child.
        self.begin_native_namespace_name_retirement(id, false);
        self.name_world.borrow_mut().dying_namespaces.insert(id);
        if id != ROOT_NS {
            self.name_world.borrow_mut().dead_namespaces.insert(id);
        }
        if !deleting_root {
            self.name_world.borrow_mut().namespaces.remove(&path);
            let mut parent_path = path.clone();
            let tail = parent_path.pop().expect("non-root namespace has a tail");
            let parent = self
                .name_world
                .borrow()
                .ns_intern
                .get(&parent_path)
                .copied()
                .unwrap_or(ROOT_NS);
            if self
                .name_world
                .borrow()
                .ns_children
                .get(&(parent, tail.clone()))
                == Some(&id)
            {
                self.name_world
                    .borrow_mut()
                    .ns_children
                    .remove(&(parent, tail.clone()));
            }
            if let Some(order) = self.name_world.borrow_mut().ns_child_order.get_mut(&parent) {
                order.remove(tail.as_bytes());
            }
        }

        // Tcl destroys the variable table before the command table. Variable
        // unset callbacks see this namespace as dying but can still address
        // its cells; each cell is removed separately so later variables remain
        // visible until their own turn.
        self.teardown_namespace_variables(canonical.as_bytes(), id);

        // Delete this namespace's command table to a fixed point. A command
        // callback may replace the dying command or install another owned
        // ensemble; recomputing after every lifecycle pass matches C's live
        // table/list traversal without an unordered subtree snapshot.
        loop {
            self.retire_namespace_owned_ensembles(id);
            // `Tcl_FirstHashEntry` order, not the flat map's iteration order:
            // the delete traces of a namespace's commands fire in the retained
            // bucket order of the table they lived in.
            let mut frontier = self.command_table_hash_order(id);
            if let Some(traversal) = command_traversal {
                frontier.truncate(traversal.frontier_len(frontier.len()));
            }
            if frontier.is_empty() {
                break;
            }
            if self.retire_namespace_command_bindings(&frontier) == 0 {
                // A fresh snapshot always names at least one current token, so
                // this is unreachable; stop rather than spin if the generation
                // map ever disagrees with the order owner.
                break;
            }
        }

        // Drop this token's path/unknown state and remove it from every other
        // live path before child teardown, mirroring `UnlinkNsPath`.
        let path_mutated = self.unlink_namespace_path_token(id);
        self.ns_unknowns.remove(&id);
        if path_mutated {
            self.invalidate_compiled_command_semantics();
        }

        // Parent commands have finished while children were still live. Now
        // each child gets its own full ensemble-before-dying lifecycle.
        let children = self.child_namespace_ids_hash_order(id);
        for child in children {
            if self
                .name_world
                .borrow()
                .namespaces
                .contains(&self.ns_path(child))
            {
                self.delete_namespace_token_using(child, false, command_traversal);
            }
        }

        self.ns_exports.remove(&id);
        self.name_world.borrow_mut().ns_child_order.remove(&id);
        self.name_world.borrow_mut().ns_command_order.remove(&id);
        if deleting_root {
            self.name_world
                .borrow_mut()
                .ns_intern
                .insert(NamespacePath::root(), ROOT_NS);
            self.name_world.borrow_mut().ns_children.clear();
            self.name_world.borrow_mut().ns_child_order = root_hash_order();
            self.name_world.borrow_mut().ns_command_order = root_hash_order();
        } else if self.name_world.borrow().ns_intern.get(&path) == Some(&id) {
            self.name_world.borrow_mut().ns_intern.remove(&path);
        }
        self.name_world.borrow_mut().dying_namespaces.remove(&id);
        self.finish_native_namespace_name_retirement(id, deleting_root);
        if native_error_reset {
            self.install_native_error_variable_traces();
        }
    }

    // Deferred teardown: retaining a token for its live frames.

    /// Whether a call frame is running in the namespace token.
    fn namespace_token_is_active(&self, id: NsId) -> bool {
        self.name_world
            .borrow()
            .ns_deferral
            .counts
            .get(&id)
            .is_some_and(|count| *count > 0)
    }

    /// C's `activationCount > (nsPtr == globalNsPtr)` branch: move the token's
    /// whole subtree out of the live tables into a record only its frames can
    /// reach, unpublish its name, and return. Reports whether the teardown was
    /// deferred.
    fn retain_namespace_commands(&mut self, record: &mut RetainedNamespace) {
        let keys: Vec<String> = self
            .commands
            .keys()
            .filter(|key| {
                self.command_slot(key).is_some_and(|slot| {
                    record
                        .subtree
                        .values()
                        .any(|namespace| *namespace == slot.namespace)
                })
            })
            .cloned()
            .collect();
        for key in keys {
            if let Some(command) = self.commands.remove(&key) {
                record.commands.insert(key.clone(), command);
            }
            let generation = self
                .name_world
                .borrow_mut()
                .command_identity
                .generations
                .remove(&key);
            if let Some(generation) = generation {
                record.generations.insert(key.clone(), generation);
            }
            if let Some(binding) = self.imported_commands.remove(&key) {
                record.imported.insert(key.clone(), binding);
            }
            if let Some(identity) = self.builtin_identities.remove(&key) {
                record.builtin_identities.insert(key, identity);
            }
        }
    }

    fn retire_deferred_namespace_coroutines(&mut self, id: NsId) {
        let retained_keys: Vec<String> = self
            .name_world
            .borrow()
            .ns_deferral
            .retained
            .get(&id)
            .map_or_default(|record| record.commands.keys().cloned().collect());
        let coroutine_keys: Vec<String> = retained_keys
            .into_iter()
            .filter(|key| crate::cmd_coro::is_coroutine(self, key))
            .collect();
        for key in coroutine_keys {
            self.materialise_retained_binding(&key);
            self.retire_command_lifecycle_key(&CommandSidecarKey::visible(key));
        }
    }

    fn defer_namespace_token(&mut self, id: NsId) -> bool {
        if !self.namespace_token_is_active(id) {
            return false;
        }
        self.begin_native_namespace_name_retirement(id, true);
        let canonical = self.ns_path(id);
        let names: Vec<NamespacePath> = self
            .name_world
            .borrow()
            .namespaces
            .iter()
            .filter(|name| name.starts_with(&canonical))
            .cloned()
            .collect();
        let mut record = RetainedNamespace::new(id);
        for name in &names {
            let removed = self.name_world.borrow_mut().ns_intern.remove(name);
            let member = removed.unwrap_or_else(|| self.intern_ns_path(name.clone()));
            record.subtree.insert(name.clone(), member);
            self.name_world
                .borrow_mut()
                .ns_deferral
                .owners
                .insert(member, id);
            self.name_world.borrow_mut().namespaces.remove(name);
            let child_link = {
                let world = self.name_world.borrow();
                world
                    .ns_parents
                    .get(member.0 as usize)
                    .copied()
                    .flatten()
                    .zip(name.last())
                    .filter(|(parent, tail)| {
                        world.ns_children.get(&(*parent, (*tail).clone())) == Some(&member)
                    })
                    .map(|(parent, tail)| (parent, tail.clone()))
            };
            if let Some(link) = child_link {
                self.name_world.borrow_mut().ns_children.remove(&link);
            }
            if let Some(order) = self
                .name_world
                .borrow_mut()
                .ns_command_order
                .remove(&member)
            {
                record.command_orders.insert(member, order);
            }
            if let Some(order) = self.name_world.borrow_mut().ns_child_order.remove(&member) {
                record.child_orders.insert(member, order);
            }
            if let Some(exports) = self.ns_exports.remove(&member) {
                record.exports.insert(member, exports);
            }
            if let Some(path) = self.name_world.borrow_mut().ns_paths.remove(&member) {
                record.paths.insert(member, path);
            }
            // `Tcl_DeleteNamespace` frees `unknownHandlerPtr` before it looks
            // at the activation count, so a retained token has no handler of
            // its own (tclsh 9.0.4 and 8.6.16: an unresolvable command in the
            // retained frame reaches the interpreter default, not the handler
            // the namespace had). A handler installed *after* deferral belongs
            // to the retained token and is kept in the record.
            self.ns_unknowns.remove(&member);
        }
        self.retain_namespace_commands(&mut record);
        // The parent edge goes at once — the spelling is free for a wholly
        // separate token straight away (C nulls `parentPtr`).
        let mut parent_path = canonical;
        let tail = parent_path.pop().expect("deferred namespace has a tail");
        let parent = self
            .name_world
            .borrow()
            .ns_intern
            .get(&parent_path)
            .copied()
            .unwrap_or(ROOT_NS);
        if let Some(order) = self.name_world.borrow_mut().ns_child_order.get_mut(&parent) {
            order.remove(tail.as_bytes());
        }
        self.name_world.borrow_mut().dead_namespaces.insert(id);
        self.name_world
            .borrow_mut()
            .ns_deferral
            .retained
            .insert(id, record);
        // A suspended coroutine whose command is owned by this token forms a
        // lifecycle cycle: its parked frame retains the namespace, while the
        // namespace owns the command that releases that frame. Tcl breaks the
        // cycle while deferring the namespace. Route those commands through
        // the same real-command lifecycle as every other deletion so parked
        // unset traces and nested implementation state are not skipped.
        self.retire_deferred_namespace_coroutines(id);
        self.bump_cmd_epoch();
        true
    }

    /// The command bound at an already-resolved structured slot. Namespace
    /// path resolution can select a retained target even when the caller is
    /// live; the slot, not the written spelling or caller, selects its table.
    pub(in super::super) fn command_at_resolved_key(
        &self,
        _written: &str,
        key: &str,
    ) -> Option<Command> {
        self.visible_command_at_key(key)
    }

    /// Tear down a token whose deletion waited for its last frame. C simply
    /// calls `Tcl_DeleteNamespace` again; the VM's tables moved into the record
    /// when the token was retained, so the same teardown runs over the record —
    /// commands in `Tcl_FirstHashEntry` order to a fixed point, then variables,
    /// then each child in child-table order.
    pub(super) fn delete_retained_namespace(&mut self, id: NsId) {
        let Some(mut record) = self
            .name_world
            .borrow_mut()
            .ns_deferral
            .retained
            .remove(&id)
        else {
            return;
        };
        self.bump_cmd_epoch();
        let root = record.root;
        self.delete_retained_token(&mut record, root);
        for member in record.subtree.values() {
            self.name_world
                .borrow_mut()
                .ns_deferral
                .owners
                .remove(member);
            self.name_world.borrow_mut().dead_namespaces.insert(*member);
        }
    }

    /// One retained token's teardown, mirroring [`Self::delete_namespace_token`]
    /// over the record's tables.
    fn delete_retained_token(&mut self, record: &mut RetainedNamespace, id: NsId) {
        let command_traversal = self
            .name_policy_protocol()
            .and_then(|policy| policy.recipe().namespace_command_teardown());
        // Variable tables remain attached to their stable namespace tokens,
        // so this final teardown cannot touch a same-spelled live recreation.
        let canonical = self.ns_name_bytes(id);
        self.begin_native_namespace_name_retirement(id, false);
        self.name_world.borrow_mut().dying_namespaces.insert(id);
        self.teardown_namespace_variables(canonical.as_bytes(), id);
        loop {
            let mut frontier = self.retained_command_hash_order(record, id);
            if let Some(traversal) = command_traversal {
                frontier.truncate(traversal.frontier_len(frontier.len()));
            }
            if frontier.is_empty() {
                break;
            }
            let mut retired = 0;
            for (key, generation) in frontier {
                if record.generations.get(&key) != Some(&generation) {
                    continue;
                }
                retired += 1;
                self.retire_retained_command(record, &key);
            }
            if retired == 0 {
                break;
            }
        }
        let path_mutated = Self::unlink_namespace_path_table(&mut record.paths, id)
            | self.unlink_namespace_path_token(id);
        record.unknowns.remove(&id);
        if path_mutated {
            self.invalidate_compiled_command_semantics();
        }
        let children = retained_children_of(
            record,
            id,
            &self.name_world.borrow().ns_parents,
            &self.name_world.borrow().ns_arena,
        );
        for child in children {
            self.delete_retained_token(record, child);
        }
        record.exports.remove(&id);
        record.child_orders.remove(&id);
        record.command_orders.remove(&id);
        self.name_world.borrow_mut().dying_namespaces.remove(&id);
        self.finish_native_namespace_name_retirement(id, false);
    }

    /// Retire one retained command token. The shared lifecycle
    /// (`Tcl_DeleteCommandFromToken`: delete traces while the entry is still in
    /// the table, then its imports depth-first) addresses a command by key, so
    /// the token is spliced into the live table for the duration of its own
    /// deletion and whatever it displaced — a recreation's binding at the same
    /// spelling — is put back afterwards.
    fn retire_retained_command(&mut self, record: &mut RetainedNamespace, key: &str) {
        let Some(command) = record.commands.remove(key) else {
            return;
        };
        let generation = record.generations.remove(key);
        let imported = record.imported.remove(key);
        let identity = record.builtin_identities.remove(key);
        let Some(slot) = self.command_slot(key) else {
            return;
        };
        let tail = slot.simple;
        if let Some(order) = record.command_orders.get_mut(&slot.namespace) {
            order.remove(tail.as_bytes());
        }
        let displaced_command = self.commands.insert(key.to_owned(), command);
        let displaced_generation = generation.and_then(|g| {
            self.name_world
                .borrow_mut()
                .command_identity
                .generations
                .insert(key.to_owned(), g)
        });
        let displaced_import = imported
            .clone()
            .and_then(|b| self.imported_commands.insert(key.to_owned(), b));
        let displaced_identity =
            identity.and_then(|i| self.builtin_identities.insert(key.to_owned(), i));
        let sidecar = CommandSidecarKey::visible(key);
        if imported.is_some() {
            self.retire_import_binding(&sidecar);
        } else {
            self.retire_command_lifecycle_key(&sidecar);
        }
        self.commands.remove(key);
        self.name_world
            .borrow_mut()
            .command_identity
            .generations
            .remove(key);
        self.imported_commands.remove(key);
        self.builtin_identities.remove(key);
        if let Some(command) = displaced_command {
            self.commands.insert(key.to_owned(), command);
        }
        if let Some(generation) = displaced_generation {
            self.name_world
                .borrow_mut()
                .command_identity
                .generations
                .insert(key.to_owned(), generation);
        }
        if let Some(binding) = displaced_import {
            self.imported_commands.insert(key.to_owned(), binding);
        }
        if let Some(identity) = displaced_identity {
            self.builtin_identities.insert(key.to_owned(), identity);
        }
    }

    /// The direct command (or proc) members of a retained token, for the
    /// introspection a frame still holding it can reach. `None` when `ns` is
    /// not retained.
    pub(in super::super) fn retained_names_directly_in(
        &self,
        ns: NsId,
        procs_only: bool,
    ) -> Option<Vec<tcl_core_types::NameBytes>> {
        let record = self.retained_record_of(ns)?;
        Some(
            record
                .commands
                .iter()
                .filter(|(key, command)| self.builtin_command_visible_for_surface(key, command))
                .filter(|(_, command)| !procs_only || matches!(command, Command::Proc(_)))
                .filter_map(|(key, _)| {
                    self.command_slot(key)
                        .filter(|slot| slot.namespace == ns)
                        .map(|slot| slot.simple.clone())
                })
                .collect(),
        )
    }

    pub(in super::super) fn child_namespace_ids(&self, parent: NsId) -> Vec<NsId> {
        let mut children: Vec<NsId> = self
            .name_world
            .borrow()
            .ns_children
            .iter()
            .filter_map(|((owner, _), id)| (*owner == parent).then_some(*id))
            .collect();
        children.sort_by_key(|id| id.0);
        children
    }

    pub(in super::super) fn child_namespace_ids_hash_order(&self, parent: NsId) -> Vec<NsId> {
        self.name_world
            .borrow()
            .ns_child_order
            .get(&parent)
            .map_or_default(|order| {
                order
                    .keys()
                    .into_iter()
                    .filter_map(|tail| {
                        self.name_world
                            .borrow()
                            .ns_children
                            .get(&(parent, NameBytes::from(tail)))
                            .copied()
                    })
                    .collect()
            })
    }
}
