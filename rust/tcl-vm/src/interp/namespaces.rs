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

//! Namespaces: paths and the unknown handler, namespace identity and operands, the resolution stack, declaration, exports and imports, and deletion.

use super::namespace_context::NamespaceExecutionContext;
use super::{InterpState, NamespacePath, NamespaceResolutionStacks, RetainedNamespace, Vm};
use std::collections::HashMap;
use std::rc::Rc;
use tcl_core_types::NameBytes;
use tcl_runtime_api::{NsId, ROOT_NS};

mod declaration;
mod deletion;
mod export_import;
mod identity;
mod import_graph;
mod path_and_unknown;

/// Native namespace address models selected before runtime token lookup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NativeNamespaceAddress {
    /// C namespace traversal starts from constructed retained segments.
    C { path: NamespacePath, absolute: bool },
    /// Jim compares its actual flat namespace object; C paths do not identify it.
    Jim {
        object: NameBytes,
        original: NameBytes,
    },
}

/// Native namespace addressing can refuse an unavailable host capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeNamespaceAddressError {
    ProtocolUnavailable,
    ContextUnavailable,
    Projection(tcl_syntax::naming::NameProjectionUnavailable),
}

/// The retained `TCL_STRING_KEYS` tables an interpreter root starts with: the
/// global holder always has one, empty and at Tcl's four static buckets.
pub(super) fn root_hash_order() -> HashMap<NsId, tcl_cmd_core::namespace::TclStringHashOrder> {
    HashMap::from([(
        ROOT_NS,
        tcl_cmd_core::namespace::TclStringHashOrder::default(),
    )])
}

impl RetainedNamespace {
    pub(super) fn new(root: NsId) -> Self {
        Self {
            root,
            subtree: HashMap::new(),
            commands: HashMap::new(),
            generations: HashMap::new(),
            imported: HashMap::new(),
            builtin_identities: HashMap::new(),
            command_orders: HashMap::new(),
            child_orders: HashMap::new(),
            exports: HashMap::new(),
            paths: HashMap::new(),
            unknowns: HashMap::new(),
        }
    }
}

/// A retained token's immediate children, in child-table order.
pub(super) fn retained_children_of(
    record: &RetainedNamespace,
    parent: NsId,
    parents: &[Option<NsId>],
    arena: &[NamespacePath],
) -> Vec<NsId> {
    record.child_orders.get(&parent).map_or_default(|order| {
        order
            .keys()
            .into_iter()
            .filter_map(|tail| {
                record.subtree.values().find_map(|id| {
                    (parents.get(id.0 as usize).copied().flatten() == Some(parent)
                        && arena
                            .get(id.0 as usize)
                            .and_then(|path| path.last())
                            .is_some_and(|segment| segment.as_bytes() == tail))
                    .then_some(*id)
                })
            })
            .collect()
    })
}

impl Default for NamespaceResolutionStacks {
    fn default() -> Self {
        Self {
            ns_stack: vec![Some(NameBytes::default())],
            ns_id_stack: vec![NamespaceExecutionContext::Resolved(ROOT_NS)],
            activation_births: vec![Rc::new(())],
        }
    }
}

impl InterpState {
    pub(super) fn current_ns_id(&self) -> NsId {
        self.resolution_stacks
            .ns_id_stack
            .last()
            .and_then(NamespaceExecutionContext::resolved_token)
            .expect("operation requires a materialized current namespace")
    }

    pub(super) fn current_ns(&self) -> &str {
        self.resolution_stacks
            .ns_stack
            .last()
            .and_then(Option::as_ref)
            .expect("operation requires a materialized namespace report")
            .try_utf8()
            .expect("Unicode namespace compatibility view requires checked native bytes")
    }
}

impl Vm {
    /// Ensure an already-constructed namespace key has a live VM namespace
    /// entry and return its stable token.
    fn activate_namespace_key(&mut self, ns: &str) -> NsId {
        let path = NamespacePath::from_segments(tcl_syntax::naming::key_segments(ns));
        self.activate_namespace_path(path)
    }

    fn activate_namespace_path(&mut self, path: NamespacePath) -> NsId {
        if path.is_empty() {
            return ROOT_NS;
        }
        let mut parent = path.clone();
        let tail = parent.pop().expect("non-root namespace has a tail");
        let parent_id = self.activate_namespace_path(parent);
        if self.namespace_in_dying_subtree_path(&path) {
            let existing = self.name_world.borrow().ns_intern.get(&path).copied();
            return existing.unwrap_or_else(|| self.intern_ns_path(path));
        }
        let created = self.name_world.borrow_mut().namespaces.insert(path.clone());
        let id = self.intern_ns_path(path);
        if created {
            self.name_world
                .borrow_mut()
                .ns_children
                .insert((parent_id, tail.clone()), id);
            self.note_namespace_created(parent_id, &tail);
        }
        id
    }

    /// Push a canonical, unrooted **constructed namespace key** onto the
    /// resolution stack (created if new).
    ///
    /// Written-name canonicalisation belongs at the boundary that resolved the
    /// name. Repeating it here would reinterpret a valid key such as
    /// `outer:::` (namespace `:` below `outer`) as the written spelling
    /// `outer:::` and collapse it back to `outer`.
    pub(crate) fn push_ns(&mut self, ns: String) {
        let path = NamespacePath::from_segments(tcl_syntax::naming::key_segments(&ns));
        // A body whose namespace is only reachable through the retained token
        // the caller holds enters *that* token, never a fresh one minted from
        // its spelling: C pushes `procPtr->cmdPtr->nsPtr`, a pointer.
        if !self.name_world.borrow().namespaces.contains(&path)
            && let Some(id) = self.retained_token_at_path(&path)
        {
            self.push_ns_token(ns, id);
            return;
        }
        let id = self.activate_namespace_key(&ns);
        self.push_ns_token(ns, id);
    }

    /// The namespace token a command bound in `holder` belongs to — C's
    /// `cmdPtr->nsPtr`. A definition the current frame can only make through a
    /// retained token belongs to that token, whose spelling may meanwhile have
    /// been taken by a wholly separate namespace.
    pub(crate) fn definition_namespace_token(&mut self, holder: &str) -> NsId {
        let path = NamespacePath::from_segments(tcl_syntax::naming::key_segments(holder));
        self.definition_namespace_token_at_path(&path, false)
    }

    /// Enter the namespace token a procedure's body runs in.
    /// `TclProcInterpProc` hands `Tcl_PushCallFrame` a `Namespace *`, so a
    /// procedure retained past its namespace's deletion runs in the retained
    /// token — not in whatever now answers to that spelling. Only a retained
    /// token is entered by id; every live one still goes through the ordinary
    /// name path, which creates the namespace if a rename left the procedure
    /// homed in a deleted one.
    pub(crate) fn push_proc_ns(&mut self, _path: &NamespacePath, id: NsId) {
        self.frames.last_mut().expect("entered procedure frame").ns =
            NamespaceExecutionContext::Resolved(id);
        self.push_ns_token(self.ns_name_bytes(id), id);
    }

    /// Push one namespace token onto the resolution stack, counting the
    /// activation `Tcl_PushCallFrame` records against it.
    pub(crate) fn push_ns_token(&mut self, name: impl Into<NameBytes>, id: NsId) {
        *self
            .name_world
            .borrow_mut()
            .ns_deferral
            .counts
            .entry(id)
            .or_insert(0) += 1;
        self.resolution_stacks.push_activation(name.into(), id);
    }

    /// Enter a transparent stale-command replay's resolution namespace without
    /// changing call-frame depth. The global stack entry is immutable: replacing
    /// it would make `uplevel #0` cease to mean global and would leak the replay
    /// namespace across [`Self::swap_flow`], which deliberately shares slot zero
    /// between coroutine flows. A cross-namespace replay at global level
    /// therefore fails closed until it can be represented without inventing a
    /// Tcl frame (which would change variable and `info level` semantics).
    pub(crate) fn enter_replay_namespace(
        &mut self,
        context: &tcl_runtime_api::CompiledNamespaceContext,
    ) -> Result<Option<(Option<NameBytes>, NamespaceExecutionContext)>, &'static str> {
        let id = self
            .resolve_compiled_namespace_context(context)
            .ok_or("native replay namespace context is unavailable")?;
        if self.current_ns_id() == id {
            return Ok(None);
        }
        if self.current_level() == 0 {
            return Err("cannot replay bytecode from another namespace at global level");
        }
        let name = self.ns_name_bytes(id);
        let previous_ns = self
            .resolution_stacks
            .ns_stack
            .last_mut()
            .expect("global namespace frame exists")
            .replace(name);
        let previous_id = std::mem::replace(
            self.resolution_stacks
                .ns_id_stack
                .last_mut()
                .expect("global namespace token exists"),
            NamespaceExecutionContext::Resolved(id),
        );
        Ok(Some((previous_ns, previous_id)))
    }

    /// Restore the namespace saved by [`Self::enter_replay_namespace`].
    pub(crate) fn leave_replay_namespace(
        &mut self,
        previous: (Option<NameBytes>, NamespaceExecutionContext),
    ) {
        *self
            .resolution_stacks
            .ns_stack
            .last_mut()
            .expect("global namespace frame exists") = previous.0;
        *self
            .resolution_stacks
            .ns_id_stack
            .last_mut()
            .expect("global namespace token exists") = previous.1;
    }

    /// Pop the current namespace (the global base is never popped). The
    /// activation goes back to its token, and a deletion that was waiting for
    /// it runs here — C's `Tcl_PopCallFrame` calls `Tcl_DeleteNamespace` again
    /// with the frame already gone.
    pub(crate) fn pop_ns(&mut self) {
        let Some(id) = self.pop_ns_token() else {
            return;
        };
        self.retire_namespace_activation_role(id);
    }

    pub(super) fn retire_namespace_activation_role(&mut self, id: NsId) {
        let settle = {
            let mut world = self.name_world.borrow_mut();
            let count = world.ns_deferral.counts.entry(id).or_insert(0);
            *count = count.saturating_sub(1);
            *count == 0 && world.ns_deferral.retained.contains_key(&id)
        };
        if settle {
            self.delete_retained_namespace(id);
        }
    }

    /// Pop the namespace stack without settling anything: the caller is
    /// discarding frames C frees outright, without `Tcl_PopCallFrame` — a
    /// deleted coroutine's parked stack, whose namespace C therefore never
    /// tears down at all.
    pub(crate) fn pop_ns_token(&mut self) -> Option<NsId> {
        self.resolution_stacks
            .pop_activation()
            .and_then(|entry| match entry.namespace {
                NamespaceExecutionContext::Resolved(token) => Some(token),
                NamespaceExecutionContext::OriginalJim(_) => None,
            })
    }

    /// Enter/leave a `namespace eval`/`inscope` body (around its evaluation).
    /// The body runs in the current frame, so we remember that frame depth.
    pub(crate) fn enter_ns_script(&mut self) {
        let depth = self.frames.len();
        self.ns_script_frames.push(depth);
    }

    pub(crate) fn leave_ns_script(&mut self) {
        self.ns_script_frames.pop();
    }
}
