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

//! Deleting namespaces: activations and deferred teardown, dying tokens, and clearing a subtree's commands.

use super::{Namespaces, NsId, GLOBAL};
use crate::frame::VarTable;
use std::collections::BTreeSet;
use tcl_syntax::naming::{qualifier_segments as split_qualifier, written_command_parts};

impl Namespaces {
    // activations and deferred teardown (C's `activationCount`)

    /// Count the activation `Tcl_PushCallFrame` adds when a call frame starts
    /// running in `ns`.
    pub(crate) fn activation_enter(&mut self, ns: NsId) {
        self.activations[ns] += 1;
    }

    /// Drop the activation a popped frame held, reporting whether that was the
    /// last one holding a deferred token — the caller then runs the teardown
    /// `Tcl_PopCallFrame` re-enters `Tcl_DeleteNamespace` for.
    pub(crate) fn activation_leave(&mut self, ns: NsId) -> bool {
        let count = &mut self.activations[ns];
        *count = count.saturating_sub(1);
        *count == 0 && self.deferred.remove(&ns)
    }

    /// Whether a call frame is still running in `ns`. The global namespace is
    /// never deferred (C compares against `nsPtr == globalNsPtr`), so its
    /// permanent frame does not count.
    #[must_use]
    pub(crate) fn namespace_is_active(&self, ns: NsId) -> bool {
        ns != GLOBAL && self.activations[ns] > 0
    }

    /// Retain a deleted token for the frames still running in it: the public
    /// parent edge goes immediately, so `namespace exists` and every absolute
    /// name stop resolving, but the command table, variables, children and
    /// exports stay exactly as they are until the last activation pops. No
    /// `dying_children` edge is recorded — unlike the synchronous window, the
    /// name is free for a fresh token straight away (C nulls `parentPtr`).
    pub(crate) fn defer_namespace(&mut self, ns: NsId) -> Option<crate::obj::Owned> {
        self.note_guard_lookup_environment_mutation();
        self.namespace_name_begin_deletion(ns);
        self.dead.insert(ns);
        self.deferred.insert(ns);
        // C frees `unknownHandlerPtr` before it looks at the activation count.
        let retired = self.arena[ns].unknown.take();
        if let Some(parent) = self.arena[ns].parent {
            let fqn = self.qualified_name(ns);
            let name = self.arena[ns].name.clone();
            self.arena[parent].children.remove(&name);
            self.arena[parent].child_order.remove(&name);
            // C nulls `parentPtr`: the spelling is free for a wholly separate
            // token straight away, and this one answers from its own name.
            self.arena[ns].parent = None;
            self.namespace_name_detach_parent(ns);
            self.arena[ns].retained_fqn = Some(fqn);
        }
        retired
    }

    /// Whether `ns` is a retained token, or lies inside one. A deferred token
    /// keeps its whole subtree, so the enclosing teardown must step over it.
    #[must_use]
    pub(crate) fn under_deferred_token(&self, ns: NsId) -> bool {
        let mut cur = Some(ns);
        while let Some(id) = cur {
            if self.deferred.contains(&id) {
                return true;
            }
            cur = self.arena[id].parent;
        }
        false
    }

    /// `namespace delete name` — delete the namespace `qualified` resolves to
    /// (relative to `current`), with its child namespaces, commands, and
    /// variables. Returns `false` if it does not exist. The arena slot is
    /// tombstoned (contents cleared, unlinked from its parent) rather than
    /// removed, so the `NsId` indices of other namespaces stay valid.
    pub fn delete_namespace(&mut self, current: NsId, qualified: &[u8]) -> bool {
        let Some(ns) = self.find_namespace(current, qualified) else {
            return false;
        };
        self.delete_namespace_by_id(ns);
        true
    }

    /// Mark a synchronous token dying while its original variable table and
    /// parent edge remain available to the variable deletion callbacks.
    pub(crate) fn begin_namespace_variable_teardown(&mut self, ns: NsId) {
        self.note_guard_lookup_environment_mutation();
        self.namespace_name_begin_deletion(ns);
        self.dying.insert(ns);
        if ns != GLOBAL {
            self.dead.insert(ns);
        }
    }

    /// Detach the exact token after its variable callbacks. Descendants remain
    /// live through its retained edge until recursive teardown reaches them.
    pub(crate) fn begin_namespace_teardown(&mut self, ns: NsId) {
        self.begin_namespace_variable_teardown(ns);
        if let Some(parent) = self.arena[ns].parent {
            let name = self.arena[ns].name.clone();
            self.arena[parent].children.remove(&name);
            self.arena[parent].child_order.remove(&name);
            self.dying_children.insert((parent, name), ns);
        }
    }

    /// Drop the temporary lookup edges after the final post-callback sweep.
    pub(crate) fn finish_namespace_teardown(&mut self, victims: &[NsId]) {
        if !victims.is_empty() {
            self.note_guard_lookup_environment_mutation();
        }
        let victim_set: BTreeSet<NsId> = victims.iter().copied().collect();
        self.dying.retain(|id| !victim_set.contains(id));
        self.dying_children.retain(|_, id| !victim_set.contains(id));
    }

    /// If `qualified` names a detached dying namespace, return its retained
    /// arena identity. Public lookup may traverse the same retained edge to a
    /// still-live child, but checks the final token's liveness; this helper
    /// specifically requires the final token itself to be dying.
    pub(crate) fn dying_namespace(&self, current: NsId, qualified: &[u8]) -> Option<NsId> {
        let absolute = written_command_parts(qualified).absolute;
        let mut ns = if absolute { GLOBAL } else { current };
        for part in split_qualifier(qualified) {
            ns = self.arena[ns]
                .children
                .get(part)
                .copied()
                .or_else(|| self.dying_children.get(&(ns, part.to_vec())).copied())?;
        }
        self.dying.contains(&ns).then_some(ns)
    }

    /// Delete the namespace `ns` (and its subtree), unlinking it from its parent.
    /// Deleting the global namespace clears its contents but keeps the node
    /// (it has no parent to unlink from) — matching `namespace delete ::`.
    pub fn delete_namespace_by_id(&mut self, ns: NsId) {
        self.note_guard_lookup_environment_mutation();
        let victims = self.descendant_ids(ns);
        // The global namespace is a permanent interpreter root: deleting it
        // clears its contents but does not invalidate its token. Every other
        // arena identity remains tombstoned after the temporary dying lookup
        // edges are dropped, including when the same spelling is recreated.
        self.dead
            .extend(victims.iter().copied().filter(|id| *id != GLOBAL));
        self.delete_subtree(ns);
        // Unlink from the parent so the name no longer resolves by lookup, but
        // keep the node's own `name`/`parent` intact: a call frame still active
        // in this (now dying) namespace must keep reporting its fully-qualified
        // name from `namespace current` until it pops (C keeps the dying
        // `Namespace` alive via its activation count — namespace-7.1).
        if let Some(parent) = self.arena[ns].parent {
            let name = self.arena[ns].name.clone();
            self.arena[parent].children.remove(&name);
            self.arena[parent].child_order.remove(&name);
        }
        // Drop the deleted namespaces from every other namespace's `namespace
        // path` — a dangling id would otherwise resolve to the global `::`
        // (`TclResetNamespaceParameters` / path fixup in `tclNamesp.c`).
        for node in &mut self.arena {
            if !node.path.is_empty() {
                node.path.retain(|p| !victims.contains(p));
            }
        }
        for victim in victims {
            self.namespace_name_finish_deletion(victim);
        }
    }

    /// `ns` and all of its descendant namespace ids (for destroying every OO
    /// object whose instance namespace lies in a namespace being deleted).
    #[must_use]
    pub fn descendant_ids(&self, ns: NsId) -> Vec<NsId> {
        let mut out = vec![ns];
        let mut i = 0;
        while i < out.len() {
            for child in self.children(out[i]) {
                out.push(child);
            }
            i += 1;
        }
        out
    }

    /// Fully-qualified locations and exact command generations in retained
    /// arena nodes. Teardown uses these to discard only the sidecars of the
    /// bindings it is about to clear.
    pub(crate) fn command_locations_in_ids(&self, ids: &[NsId]) -> Vec<(Vec<u8>, u64)> {
        let mut locations = Vec::new();
        for &id in ids {
            locations.extend(
                self.arena[id]
                    .commands
                    .hash_order()
                    .into_iter()
                    .map(|(name, generation)| (self.command_fqn(id, &name), generation)),
            );
        }
        locations
    }

    /// The same bindings as `(namespace, simple name)` pairs. A retained token
    /// and a same-named recreation share every fully-qualified name they hold,
    /// so a teardown that must fire exactly one of them addresses the slot.
    pub(crate) fn command_slots_in_ids(&self, ids: &[NsId]) -> Vec<(NsId, Vec<u8>)> {
        let mut slots = Vec::new();
        for &id in ids {
            slots.extend(
                self.arena[id]
                    .commands
                    .hash_order()
                    .into_iter()
                    .map(|(name, _)| (id, name)),
            );
        }
        slots
    }

    /// Clear callback-created state from explicit dying arena nodes. The
    /// caller has already fired command traces and removed dependent imports.
    pub(crate) fn clear_namespace_ids(
        &mut self,
        ids: &[NsId],
    ) -> (Vec<VarTable>, Vec<crate::obj::Owned>) {
        self.invalidate_original_publications();
        if !ids.is_empty() {
            self.note_guard_lookup_environment_mutation();
        }
        let mut retired = Vec::new();
        let mut tables = Vec::new();
        for &id in ids.iter().rev() {
            if id != GLOBAL {
                self.retired_variable_namespaces.insert(id);
            }
            let commands = self.arena[id].commands.hash_order();
            for (simple, _) in commands {
                if let Some(binding) = self.remove_native_binding(id, &simple) {
                    Self::mark_command_deleted(&binding.command, &self.retired_ensemble_roles);
                }
            }
            let n = &mut self.arena[id];
            n.children.clear();
            n.child_order.clear();
            for command in n.commands.values() {
                Self::mark_command_deleted(command, &self.retired_ensemble_roles);
            }
            n.commands.clear();
            n.path.clear();
            n.exports.clear();
            tables.push(std::mem::take(&mut n.vars));
            retired.extend(n.unknown.take());
        }
        (tables, retired)
    }

    /// Clear one dying namespace's own variable table, retaining paths and
    /// child links through the ordinary command deletion callbacks. The
    /// recursive delete callbacks still to run. The command table is emptied
    /// one token at a time afterwards, in hash order; the final fixed-point
    /// sweep clears the retained metadata and links.
    pub(crate) fn clear_namespace_token(&mut self, ns: NsId) -> VarTable {
        if ns != GLOBAL {
            self.retired_variable_namespaces.insert(ns);
        }
        std::mem::take(&mut self.arena[ns].vars)
    }

    /// Unpublish this token and its resolution paths after its commands retire.
    pub(crate) fn finish_native_namespace_command_path(&mut self, ns: NsId) {
        if !self.arena[ns].path.is_empty() || self.arena.iter().any(|node| node.path.contains(&ns))
        {
            self.note_guard_command_lookup_policy_mutation();
        } else {
            self.note_guard_lookup_environment_mutation();
        }
        self.dying_children.retain(|_, child| *child != ns);
        if self.arena[ns].parent.is_some() {
            let fqn = self.qualified_name(ns);
            self.arena[ns].parent = None;
            self.arena[ns].retained_fqn = Some(fqn);
            self.namespace_name_detach_parent(ns);
        }
        self.arena[ns].path.clear();
        self.invalidate_native_path_dependents(ns);
        for node in &mut self.arena {
            node.path.retain(|entry| *entry != ns);
        }
    }

    /// Recursively clear a namespace and its descendants: dropping the `VarTable`
    /// releases the variables' object references (`TclFreeVar`); commands and
    /// child links are dropped.
    pub(super) fn delete_subtree(&mut self, ns: NsId) {
        self.invalidate_original_publications();
        if ns != GLOBAL {
            self.retired_variable_namespaces.insert(ns);
        }
        for child in self.children(ns) {
            self.delete_subtree(child);
        }
        let commands = self.arena[ns].commands.hash_order();
        for (simple, _) in commands {
            if let Some(binding) = self.remove_native_binding(ns, &simple) {
                Self::mark_command_deleted(&binding.command, &self.retired_ensemble_roles);
            }
        }
        let n = &mut self.arena[ns];
        n.children.clear();
        n.child_order.clear();
        for command in n.commands.values() {
            Self::mark_command_deleted(command, &self.retired_ensemble_roles);
        }
        n.commands.clear();
        // Keep namespace metadata alive through delete callbacks. The node is
        // detached, so introspection cannot see it, but command-token operations
        // such as importing a newly-created command still consult its export
        // patterns/path until the final dying-namespace sweep clears them.
        n.vars = VarTable::default(); // drop → release variable refcounts
    }
}
