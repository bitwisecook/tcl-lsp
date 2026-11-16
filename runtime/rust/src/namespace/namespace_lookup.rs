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

//! Namespace paths and finding a namespace by name or components, creating the missing ones.

use super::{Namespaces, NsId, GLOBAL};
use tcl_syntax::naming::{qualifier_segments as split_qualifier, written_command_parts};

impl Namespaces {
    /// Set namespace `ns`'s `namespace path` to the given namespaces.
    pub fn set_path(&mut self, ns: NsId, path: Vec<NsId>) {
        self.note_guard_command_lookup_policy_mutation();
        self.arena[ns].path = path;
        if self
            .native_command_version
            .is_some_and(|version| version != tcl_dialect::TclVersion::V8_4)
        {
            self.note_native_compiler_mutation(
                Some(ns),
                tcl_registry::native_procedure::NativeCompilerCacheMutation::NamespacePath,
            );
            self.increment_command_reference(ns);
        }
    }

    /// Select one authored support slot from counted written components.
    /// The actual holder token survives ambiguous reports and dying namespaces.
    pub(crate) fn support_command_publication_at(
        &mut self,
        current: NsId,
        written: &[u8],
    ) -> (NsId, Vec<u8>) {
        let parts = written_command_parts(written);
        let namespace = self.command_home_parts(current, &parts);
        (namespace, parts.simple.to_vec())
    }

    fn command_home_parts(
        &mut self,
        current: NsId,
        parts: &tcl_syntax::naming::WrittenCommandParts<'_>,
    ) -> NsId {
        let mut ns = if parts.absolute { GLOBAL } else { current };
        // Retain a complete dying holder; a missing longer chain creates
        // visible children from the supplied token anchor.
        let mut existing = ns;
        let mut complete = true;
        for part in &parts.qualifiers {
            let next = self.arena[existing]
                .children
                .get(*part)
                .copied()
                .or_else(|| self.dying_children.get(&(existing, part.to_vec())).copied());
            let Some(next) = next else {
                complete = false;
                break;
            };
            existing = next;
        }
        if complete {
            return existing;
        }
        for part in &parts.qualifiers {
            ns = self.ensure_child(ns, part);
        }
        ns
    }

    /// Create the namespace owned by a command at `name` under the exact
    /// namespace token that receives the command binding. During synchronous
    /// namespace teardown that may be a detached dying token; TclOO objects
    /// created by a delete trace must join that generation and its fixed-point
    /// sweep, not recreate the visible qualifier chain.
    pub(crate) fn ensure_command_owned_namespace(&mut self, current: NsId, name: &[u8]) -> NsId {
        let parts = written_command_parts(name);
        let home = self.command_home_parts(current, &parts);
        if parts.simple.is_empty() {
            return home;
        }
        self.ensure_child(home, parts.simple)
    }

    /// Find (creating if needed) the namespace named `qualified`, rooted at
    /// `current` (absolute if it leads with `::`). For `namespace eval`.
    pub fn ensure_namespace(&mut self, current: NsId, qualified: &[u8]) -> NsId {
        // A live child remains a public namespace while an already-dying
        // parent token is being torn down. Resolve through the retained dying
        // edge before creating; a path whose *final* token is dying instead
        // falls through and builds a fresh visible tree.
        if let Some(existing) = self.find_namespace(current, qualified) {
            if self.namespace_is_live(existing) {
                return existing;
            }
        }
        let absolute = written_command_parts(qualified).absolute;
        let mut ns = if absolute { GLOBAL } else { current };
        for part in split_qualifier(qualified) {
            ns = self.ensure_child(ns, part);
        }
        ns
    }

    /// Resolve `qualified` to a live namespace, or `None`. A retained dying
    /// edge may be traversed to reach a child whose own token is still live,
    /// but a dying/dead final token is never returned as a public namespace.
    #[must_use]
    pub fn find_namespace(&self, current: NsId, qualified: &[u8]) -> Option<NsId> {
        let absolute = written_command_parts(qualified).absolute;
        let mut ns = if absolute { GLOBAL } else { current };
        for part in split_qualifier(qualified) {
            ns = self.arena[ns]
                .children
                .get(part)
                .copied()
                .or_else(|| self.dying_children.get(&(ns, part.to_vec())).copied())?;
        }
        self.namespace_is_live(ns).then_some(ns)
    }

    pub(crate) fn namespace_at_path(
        &self,
        path: &tcl_core_types::ByteNamespacePath,
    ) -> Option<NsId> {
        self.namespace_at_components(GLOBAL, path.as_segments())
    }

    pub(crate) fn namespace_at_components(
        &self,
        current: NsId,
        components: &[tcl_core_types::NameBytes],
    ) -> Option<NsId> {
        let namespace = self.namespace_token_at_components(current, components)?;
        self.namespace_is_live(namespace).then_some(namespace)
    }

    /// Follow physical child tokens; callers apply the selected lifetime policy.
    pub(crate) fn namespace_token_at_components(
        &self,
        current: NsId,
        components: &[tcl_core_types::NameBytes],
    ) -> Option<NsId> {
        self.arena.get(current)?;
        let mut namespace = current;
        for part in components {
            namespace = self
                .arena
                .get(namespace)?
                .children
                .get(part.as_bytes())
                .copied()
                .or_else(|| {
                    self.dying_children
                        .get(&(namespace, part.as_bytes().to_vec()))
                        .copied()
                })?;
        }
        Some(namespace)
    }

    pub(crate) fn dying_namespace_at_components(
        &self,
        current: NsId,
        components: &[tcl_core_types::NameBytes],
    ) -> Option<NsId> {
        let namespace = self.namespace_token_at_components(current, components)?;
        self.dying.contains(&namespace).then_some(namespace)
    }

    pub(crate) fn ensure_namespace_at_components(
        &mut self,
        current: NsId,
        components: &[tcl_core_types::NameBytes],
    ) -> NsId {
        if let Some(existing) = self.namespace_at_components(current, components) {
            return existing;
        }
        components
            .iter()
            .enumerate()
            .fold(current, |parent, (index, part)| {
                if index + 1 < components.len() {
                    if let Some(existing) =
                        self.namespace_token_at_components(parent, std::slice::from_ref(part))
                    {
                        return existing;
                    }
                }
                self.ensure_child(parent, part.as_bytes())
            })
    }

    /// Whether `ns` still denotes a public namespace token. A dying arena node
    /// remains command-addressable during delete callbacks, but namespace
    /// introspection must reject even an empty relative name resolved from its
    /// retained current-namespace handle.
    #[must_use]
    pub(crate) fn namespace_is_live(&self, ns: NsId) -> bool {
        !self.dead.contains(&ns) && !self.dying.contains(&ns)
    }

    /// Whether namespace teardown has destroyed this token's variable cells.
    pub(crate) fn namespace_variables_are_deleted(&self, ns: NsId) -> bool {
        self.retired_variable_namespaces.contains(&ns)
    }
}
