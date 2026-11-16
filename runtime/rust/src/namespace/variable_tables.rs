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

//! Per-namespace variable tables, the variable resolver's storage, and the qualified names and context paths reported for them.

use super::{Namespaces, NsId, GLOBAL};
use crate::frame::VarTable;
use crate::interp::Command;

impl Namespaces {
    // per-namespace variable tables (the variable resolver's storage)

    /// Selected variable components under the exact supplied namespace token.
    /// Table presence and read/write fallback are independently caller-owned.
    #[must_use]
    pub(crate) fn var_home(&self, current: NsId, name: &[u8]) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.variable_name_protocol?;
        let input = protocol.variable_root_input(name);
        if protocol.is_jim084() {
            let namespace = self.jim_namespace_bytes(current)?;
            let prefix = tcl_syntax::naming::root_unrooted_key_bytes(namespace);
            return Some((
                GLOBAL,
                tcl_syntax::naming::jim_global_variable_key_bytes(&prefix, input.selected()),
            ));
        }
        let projected = input.variable_namespace_projection().ok()?;
        let mut namespace =
            if projected.qualification() == tcl_syntax::naming::NativeNameQualification::Absolute {
                GLOBAL
            } else {
                current
            };
        for part in projected.qualifiers() {
            namespace = *self.arena[namespace].children.get(part.as_bytes())?;
        }
        Some((namespace, projected.simple().as_bytes().to_vec()))
    }

    /// Sorted variable names in namespace `ns` (`info vars`/`globals`).
    #[must_use]
    pub(crate) fn var_names(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.arena[ns]
            .vars
            .names()
            .into_iter()
            .map(<[u8]>::to_vec)
            .collect()
    }

    /// `const` scalar names in `ns` (`info consts`).
    pub(crate) fn const_names(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.arena[ns]
            .vars
            .const_names()
            .into_iter()
            .map(<[u8]>::to_vec)
            .collect()
    }

    /// Sorted names of the commands in `ns` that are procs (`info procs`).
    #[must_use]
    pub(crate) fn proc_names(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.arena[ns]
            .commands
            .iter()
            .filter(|(_, c)| matches!(c, Command::Proc(_)))
            .map(|(k, _)| self.command_table_report_name(ns, k).to_vec())
            .collect()
    }

    pub(crate) fn original_variable_table_at(&self, ns: NsId) -> Option<&VarTable> {
        if self.namespace_variables_are_deleted(ns) {
            return None;
        }
        Some(&self.arena.get(ns)?.vars)
    }

    /// Mutable access to the already selected physical table. The caller owns
    /// conversion/currentness and cannot acquire it by a new name lookup.
    pub(crate) fn original_variable_table_at_mut(&mut self, ns: NsId) -> Option<&mut VarTable> {
        if self.namespace_variables_are_deleted(ns) {
            return None;
        }
        self.arena.get_mut(ns).map(|namespace| &mut namespace.vars)
    }

    pub(crate) fn capture_container_model_at(
        &self,
        ns: NsId,
    ) -> Option<(
        std::rc::Weak<std::cell::RefCell<tcl_core_types::NativeEntryLedger>>,
        Option<crate::frame::CapturedContainerModel>,
    )> {
        let namespace = self.arena.get(ns)?;
        if self.namespace_variables_are_deleted(ns) {
            return None;
        }
        let table = &namespace.vars;
        table.set_hash_recipe(self.variable_hash_recipe);
        Some((
            table.original_table_ledger(),
            table.capture_container_model(
                self.variable_container_model,
                self.variable_string_protocol,
            ),
        ))
    }

    /// Namespace `ns`'s variable table (read).
    #[must_use]
    pub(crate) fn var_table(&self, ns: NsId) -> &VarTable {
        self.arena[ns]
            .vars
            .set_hash_recipe(self.variable_hash_recipe);
        self.arena[ns]
            .vars
            .set_container_model(self.variable_container_model, self.variable_string_protocol);
        &self.arena[ns].vars
    }

    /// Namespace `ns`'s variable table (mutable).
    pub(crate) fn var_table_mut(&mut self, ns: NsId) -> &mut VarTable {
        self.arena[ns]
            .vars
            .set_hash_recipe(self.variable_hash_recipe);
        self.arena[ns]
            .vars
            .set_container_model(self.variable_container_model, self.variable_string_protocol);
        &mut self.arena[ns].vars
    }

    /// The fully-qualified name of `ns` (`::a::b`; global is `::`).
    #[must_use]
    pub fn qualified_name(&self, ns: NsId) -> Vec<u8> {
        if ns != GLOBAL && self.arena[ns].jim_namespace.is_some() {
            let namespace = self
                .jim_namespace_bytes(ns)
                .expect("Jim namespace reporting requires an already reached namespace getter");
            return tcl_syntax::naming::root_unrooted_key_bytes(namespace);
        }
        tcl_syntax::naming::native_namespace_full_name_bytes(&self.arena[ns].constructed_path)
    }

    /// C command-table components under the same selected command recipe.
    /// This ignores variable-name/Jim holder policies and supplies no token receipt.
    pub(crate) fn native_c_command_context_path(
        &self,
        namespace: NsId,
        version: tcl_dialect::TclVersion,
    ) -> Option<tcl_core_types::ByteNamespacePath> {
        if self.native_command_version != Some(version) {
            return None;
        }
        self.arena
            .get(namespace)
            .filter(|node| namespace == GLOBAL || node.jim_namespace.is_none())
            .map(|node| node.constructed_path.clone())
    }

    /// Return the original constructed address of an actual arena token.
    /// Public liveness and physical command/variable tables remain separate;
    /// detached frames keep their own address without reparsing a display.
    pub(crate) fn native_context_path(
        &self,
        ns: NsId,
    ) -> Option<tcl_core_types::ByteNamespacePath> {
        if self
            .variable_name_protocol
            .is_some_and(|protocol| protocol.is_jim084())
        {
            let original = self.jim_namespace_bytes(ns)?;
            return Some(if original.is_empty() {
                tcl_core_types::ByteNamespacePath::root()
            } else {
                tcl_core_types::ByteNamespacePath::from_segments([original])
            });
        }
        self.arena
            .get(ns)
            .filter(|namespace| ns == GLOBAL || namespace.jim_namespace.is_none())
            .map(|namespace| namespace.constructed_path.clone())
    }
}
