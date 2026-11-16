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

//! `namespace path` and `namespace unknown`: the command resolution path of a namespace and its unknown handler.

use super::super::namespace_context::NamespaceExecutionContext;
use super::super::{NativeUnknownHandler, Vm};
use crate::value::Value;
use std::collections::HashMap;
use std::rc::Rc;
use tcl_core_types::NameBytes;
use tcl_runtime_api::{NsId, ROOT_NS};

impl Vm {
    pub(crate) fn ns_path_tokens(&self) -> Vec<NsId> {
        let current = self.current_ns_id();
        self.retained_current_record()
            .and_then(|record| record.paths.get(&current).cloned())
            .or_else(|| self.name_world.borrow().ns_paths.get(&current).cloned())
            .unwrap_or_default()
    }

    /// Set the current namespace's command resolution path to `path` (canonical
    /// names, no leading `::`).
    pub(crate) fn ns_path_set(&mut self, path: Vec<NsId>) {
        self.note_native_command_reference_changed(self.current_ns_id());
        self.bump_cmd_epoch();
        self.invalidate_lookup_guards();
        self.invalidate_compiled_command_semantics();
        let cur = self.current_ns_id();
        self.note_native_namespace_cache_mutation(
            cur,
            tcl_registry::native_procedure::NativeCompilerCacheMutation::NamespacePath,
        );
        let mut world = self.name_world.borrow_mut();
        let root = world.ns_deferral.owners.get(&cur).copied();
        if let Some(record) = root.and_then(|root| world.ns_deferral.retained.get_mut(&root)) {
            record.paths.insert(cur, path);
        } else {
            world.ns_paths.insert(cur, path);
        }
    }

    /// Unlink a namespace token as both a path owner and a retained target in
    /// one path table. Values are exact `NsId`s, so a same-spelled recreation
    /// is never affected.
    pub(super) fn unlink_namespace_path_table(
        paths: &mut HashMap<NsId, Vec<NsId>>,
        namespace: NsId,
    ) -> bool {
        let mut mutated = paths.remove(&namespace).is_some();
        for targets in paths.values_mut() {
            let old_len = targets.len();
            targets.retain(|target| *target != namespace);
            mutated |= targets.len() != old_len;
        }
        mutated
    }

    /// Remove one finalised namespace token from every live and retained
    /// namespace-path graph in this interpreter.
    pub(super) fn unlink_namespace_path_token(&mut self, namespace: NsId) -> bool {
        let mut world = self.name_world.borrow_mut();
        let mut mutated = Self::unlink_namespace_path_table(&mut world.ns_paths, namespace);
        for record in world.ns_deferral.retained.values_mut() {
            mutated |= Self::unlink_namespace_path_table(&mut record.paths, namespace);
        }
        mutated
    }

    /// Return the original root; only the global getter installs the native
    /// default lazily. An unset child namespace still reports an empty result.
    pub(crate) fn ns_unknown_get(&mut self) -> Option<Value> {
        let current = self.current_ns_id();
        self.ns_unknown_root_at(current)
            .or_else(|| (current == ROOT_NS).then(|| self.default_unknown_root()))
    }

    fn ns_unknown_root_at(&self, namespace: NsId) -> Option<Value> {
        self.retained_record_of(namespace)
            .and_then(|record| {
                record
                    .unknowns
                    .get(&namespace)
                    .map(|handler| handler.root.clone())
            })
            .or_else(|| {
                self.ns_unknowns
                    .get(&namespace)
                    .map(|handler| handler.root.clone())
            })
    }

    fn default_unknown_root(&mut self) -> Value {
        if let Some(root) = self.ns_unknown_root_at(ROOT_NS) {
            return root;
        }
        let root = Value::string("::unknown");
        self.ns_unknown_set_at(ROOT_NS, root.clone(), 1);
        root
    }

    /// Publish the original root only after the selected native Length getter
    /// succeeds. Zero elements reset even when the original bytes are nonempty.
    pub(crate) fn ns_unknown_set(&mut self, root: Value, length: usize) {
        let cur = self.current_ns_id();
        self.ns_unknown_set_at(cur, root, length);
    }

    pub(in super::super) fn ns_unknown_set_at(&mut self, cur: NsId, root: Value, length: usize) {
        self.invalidate_namespace_structure_guards();
        let retired = {
            let world_handle = Rc::clone(&self.name_world);
            let mut world = world_handle.borrow_mut();
            let owner = world.ns_deferral.owners.get(&cur).copied();
            let table = owner
                .and_then(|owner| world.ns_deferral.retained.get_mut(&owner))
                .map_or(&mut self.ns_unknowns, |record| &mut record.unknowns);
            table.remove(&cur)
        };
        drop(retired);
        if length == 0 {
            return;
        }
        let retired = {
            let world_handle = Rc::clone(&self.name_world);
            let mut world = world_handle.borrow_mut();
            let owner = world.ns_deferral.owners.get(&cur).copied();
            let table = owner
                .and_then(|owner| world.ns_deferral.retained.get_mut(&owner))
                .map_or(&mut self.ns_unknowns, |record| &mut record.unknowns);
            table.insert(cur, NativeUnknownHandler { root, length })
        };
        drop(retired);
    }

    /// Select the original current-token handler, then the original global
    /// default. This does not convert the root or reconstruct its members.
    pub(crate) fn ns_unknown_handler_at(&mut self, selected: NsId) -> Value {
        self.ns_unknown_root_at(selected)
            .unwrap_or_else(|| self.default_unknown_root())
    }

    /// Temporarily give the existing variable frame the native missing-handler
    /// lookup context, without allocating a Tcl frame or namespace activation.
    pub(crate) fn enter_missing_handler_namespace(
        &mut self,
        context: NsId,
    ) -> (
        Option<NameBytes>,
        NamespaceExecutionContext,
        NamespaceExecutionContext,
    ) {
        let name = self.ns_name_bytes(context);
        let previous_name = self
            .resolution_stacks
            .ns_stack
            .last_mut()
            .expect("root namespace")
            .replace(name);
        let previous_token = std::mem::replace(
            self.resolution_stacks
                .ns_id_stack
                .last_mut()
                .expect("root token"),
            NamespaceExecutionContext::Resolved(context),
        );
        let previous_frame = std::mem::replace(
            &mut self.frames.last_mut().expect("root frame").ns,
            NamespaceExecutionContext::Resolved(context),
        );
        (previous_name, previous_token, previous_frame)
    }

    /// Restore the variable frame after the reached missing-handler invocation.
    pub(crate) fn leave_missing_handler_namespace(
        &mut self,
        previous: (
            Option<NameBytes>,
            NamespaceExecutionContext,
            NamespaceExecutionContext,
        ),
    ) {
        *self
            .resolution_stacks
            .ns_stack
            .last_mut()
            .expect("root namespace") = previous.0;
        *self
            .resolution_stacks
            .ns_id_stack
            .last_mut()
            .expect("root token") = previous.1;
        self.frames.last_mut().expect("root frame").ns = previous.2;
    }
}
