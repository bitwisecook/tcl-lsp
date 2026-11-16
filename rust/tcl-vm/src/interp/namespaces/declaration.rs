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

//! Declaring namespaces and tracking what they hold: existence, dying subtrees, creation, and the Tcl hash order of a namespace's commands.

use super::super::{CommandSlot, NamespacePath, RetainedNamespace, Vm};
use tcl_core_types::NameBytes;
use tcl_runtime_api::{NsId, ROOT_NS};

impl Vm {
    /// Whether namespace `ns` (canonical, unrooted; `""` is the always-present
    /// global namespace) currently exists.
    pub(crate) fn namespace_exists(&self, ns: &str) -> bool {
        let path = NamespacePath::from_segments(tcl_syntax::naming::key_segments(ns));
        self.namespace_exists_path(&path)
    }

    fn namespace_exists_path(&self, path: &NamespacePath) -> bool {
        let id = self.name_world.borrow().ns_intern.get(path).copied();
        id.is_some_and(|id| !self.name_world.borrow().dying_namespaces.contains(&id))
            && (path.is_empty() || self.name_world.borrow().namespaces.contains(path))
    }

    /// Whether this entered namespace token is deleted or being torn down.
    /// A same-spelled recreation owns a different token and lifetime.
    pub(crate) fn namespace_token_is_dying(&self, namespace: NsId) -> bool {
        let world = self.name_world.borrow();
        world.dying_namespaces.contains(&namespace) || world.dead_namespaces.contains(&namespace)
    }

    /// Support-path activation does not republish a dying subtree. Native
    /// namespace creation uses the actual token-anchored publication owner.
    pub(super) fn namespace_in_dying_subtree_path(&self, ns: &NamespacePath) -> bool {
        let mut candidate = ns.clone();
        loop {
            if self
                .name_world
                .borrow()
                .ns_intern
                .get(&candidate)
                .is_some_and(|id| self.name_world.borrow().dying_namespaces.contains(id))
            {
                return true;
            }
            if candidate.is_empty() {
                return false;
            }
            candidate.pop();
        }
    }

    /// Register an existing namespace (and its ancestors). The name is
    /// normalised to canonical form first, and the ancestor walk uses the
    /// shared separator-run-aware split ([`tcl_cmd_core::namespace`]), so a
    /// colon-run name (`a:::b`) registers `a::b` under parent `a` — never a
    /// bogus `a:`, which a plain `rsplit_once("::")` would produce.
    /// [`Self::declare_namespace`] for an already-canonical **key** (a
    /// construction-inverse holder): no written-name canonicalisation, which
    /// would collapse a lone-colon segment, and the parent chain walks
    /// the construction-inverse split for the same reason.
    pub(crate) fn declare_namespace_key(&mut self, ns_key: &str) {
        let path = NamespacePath::from_segments(tcl_syntax::naming::key_segments(ns_key));
        self.declare_namespace_path_with_origin(path, false);
    }

    pub(in super::super) fn declare_namespace_path_with_origin(
        &mut self,
        path: NamespacePath,
        from_root: bool,
    ) {
        if path.is_empty()
            || self.namespace_in_dying_subtree_path(&path)
            // A name the current frame reaches only through its retained token
            // is that token's, and must not republish the spelling. An absolute
            // lookup is rooted outside that activation and may recreate it.
            || (!from_root && self.retained_owner_of_namespace(&path).is_some())
        {
            return;
        }
        let mut parent = path.clone();
        let tail = parent.pop().expect("non-root namespace has a tail");
        self.declare_namespace_path_with_origin(parent.clone(), from_root);
        let parent_id = self
            .name_world
            .borrow()
            .ns_intern
            .get(&parent)
            .copied()
            .unwrap_or(ROOT_NS);
        let created = self.name_world.borrow_mut().namespaces.insert(path.clone());
        let id = self.intern_ns_path(path);
        if created {
            self.name_world
                .borrow_mut()
                .ns_children
                .insert((parent_id, tail.clone()), id);
            self.note_namespace_created(parent_id, &tail);
        }
    }

    pub(crate) fn declare_namespace(&mut self, ns: &str) {
        let path =
            NamespacePath::from_segments(tcl_syntax::naming::qualifier_segments(ns.as_bytes()));
        self.declare_namespace_path_with_origin(
            path,
            tcl_syntax::naming::written_command_parts(ns.as_bytes()).absolute,
        );
    }

    /// Invalidate command resolution after actual namespace publication.
    pub(super) fn note_namespace_creation_environment(&mut self) {
        self.bump_cmd_epoch();
        self.invalidate_compiled_command_semantics();
        self.invalidate_namespace_structure_guards();
    }

    /// Insert a newly-created tail into its parent's retained Tcl string-hash table.
    pub(super) fn note_namespace_created(&mut self, parent: NsId, tail: &NameBytes) {
        // Creation changes qualified-name lookup before a command is installed.
        self.note_namespace_creation_environment();
        self.name_world
            .borrow_mut()
            .ns_child_order
            .entry(parent)
            .or_default()
            .insert(tail.as_bytes());
    }

    /// Delete a rename destination's hash entry again after the alias-loop
    /// check refused the move — C's `Tcl_DeleteHashEntry(cmdPtr->hPtr)` on the
    /// rollback path. The table keeps the capacity the transient entry gave it.
    pub(crate) fn forget_rename_destination(&mut self, key: &str) {
        self.note_command_unbound(key);
        if !self.commands.contains_key(key) {
            self.forget_command_storage_key(key);
        }
    }

    /// Insert a command's tail into its holder namespace's retained Tcl
    /// string-hash table. `replacing` marks C's redefinition
    /// (`TclCreateObjCommandInNs` deletes the existing hash entry and creates
    /// a new one, so the name moves to its bucket head); a fresh name is a
    /// plain insert.
    pub(in super::super) fn note_command_bound(&mut self, key: &str, replacing: bool) {
        self.note_native_shadowed_compiler_commands(key);
        let Some(slot) = self.command_slot(key) else {
            return;
        };
        let mut world = self.name_world.borrow_mut();
        let root = world.ns_deferral.owners.get(&slot.namespace).copied();
        let order = if let Some(root) = root {
            world
                .ns_deferral
                .retained
                .get_mut(&root)
                .expect("retained namespace owner has its record")
                .command_orders
                .entry(slot.namespace)
                .or_default()
        } else {
            world.ns_command_order.entry(slot.namespace).or_default()
        };
        if replacing {
            order.reinsert(slot.simple.as_bytes());
        } else {
            order.insert(slot.simple.as_bytes());
        }
    }

    pub(in super::super) fn note_command_unbound(&mut self, key: &str) {
        let Some(slot) = self.command_slot(key) else {
            return;
        };
        let mut world = self.name_world.borrow_mut();
        let root = world.ns_deferral.owners.get(&slot.namespace).copied();
        let order = if let Some(root) = root {
            world
                .ns_deferral
                .retained
                .get_mut(&root)
                .and_then(|record| record.command_orders.get_mut(&slot.namespace))
        } else {
            world.ns_command_order.get_mut(&slot.namespace)
        };
        if let Some(order) = order {
            order.remove(slot.simple.as_bytes());
        }
    }

    /// One namespace's live command keys with their token generations, in
    /// `Tcl_FirstHashEntry` order — the snapshot `TclTeardownNamespace` takes
    /// of `cmdTable` before deleting each token.
    pub(super) fn command_table_hash_order(&self, namespace: NsId) -> Vec<(String, u64)> {
        let world = self.name_world.borrow();
        world
            .ns_command_order
            .get(&namespace)
            .map_or_default(|order| {
                order
                    .keys()
                    .into_iter()
                    .filter_map(|tail| {
                        let slot = CommandSlot {
                            namespace,
                            simple: NameBytes::from(tail),
                        };
                        let key = world.command_identity.by_slot.get(&slot)?.clone();
                        let generation = world.command_identity.generations.get(&key).copied()?;
                        Some((key, generation))
                    })
                    .collect()
            })
    }

    /// One retained namespace token's command entries in its Tcl hash-table
    /// order. A rendered FQN cannot identify the entry, so the table's
    /// simple-name bytes are paired with the exact namespace token before the
    /// private storage key is selected.
    pub(super) fn retained_command_hash_order(
        &self,
        record: &RetainedNamespace,
        namespace: NsId,
    ) -> Vec<(String, u64)> {
        record
            .command_orders
            .get(&namespace)
            .map_or_default(|order| {
                order
                    .keys()
                    .into_iter()
                    .filter_map(|tail| {
                        let slot = CommandSlot {
                            namespace,
                            simple: tcl_core_types::NameBytes::from(tail),
                        };
                        let key = self.command_storage_key_at_slot(&slot)?;
                        let generation = *record.generations.get(&key)?;
                        Some((key, generation))
                    })
                    .collect()
            })
    }
}
