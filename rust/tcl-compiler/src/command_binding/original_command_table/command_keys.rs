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

//! Command keys and lookup paths of an original name operand under its namespace geometry.

use super::super::{
    BTreeSet, MayBinding, ModuleCommandBindings, SourceCommandKey, SourceNamespaceKey,
};
#[cfg(any(test, debug_assertions))]
use super::OriginalGeometryTiming;
use super::{OriginalAliasLoopPaths, OriginalLookupSlot};
use crate::signature_scan::scope::{SignatureNamespaceScope, SignatureSourceNameInput};
use tcl_registry::CommandBindingDefinitionKind;

impl ModuleCommandBindings {
    pub(super) fn original_definition_key(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
        kind: CommandBindingDefinitionKind,
    ) -> Option<SourceCommandKey> {
        match input {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        let scope = if tcl_syntax::naming::is_absolute_bytes(input.bytes()) {
            SignatureNamespaceScope::root(Some(input.policy()))
        } else {
            self.original_namespace_geometry(current, input.policy())?
        };
        let slot = super::super::source_command_world::publication_slot(&scope, input, kind)?;
        self.original_command_key_for_slot(&slot, input.policy())
    }

    /// Conditional declaration namespace geometry from a genuine original
    /// operand. The original observation/frame owner must admit the caller.
    /// Current table existence, installation and Normal completion remain
    /// separate; this facade must never be used for an operation transfer.
    pub(in super::super) fn original_conditional_procedure_key(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> Option<SourceCommandKey> {
        self.original_conditional_procedure_coordinates(current, input)
            .map(|(key, _)| key)
    }

    pub(in super::super) fn original_conditional_procedure_coordinates(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> Option<(SourceCommandKey, tcl_core_types::ByteCommandSlot)> {
        let policy = input.policy();
        if self.baseline.execution_name_policy?.native_recipe()? != policy {
            return None;
        }
        if let SourceNamespaceKey::Native(context) = current {
            let entry = self.baseline.native_entry.as_ref()?;
            if entry.command_name_policy()? != policy
                || entry.retained_namespace_context(context.token).ok()? != *context
            {
                return None;
            }
        }
        let scope = if tcl_syntax::naming::is_absolute_bytes(input.bytes()) || current.is_root() {
            SignatureNamespaceScope::root(Some(policy))
        } else {
            self.original_command_world
                .conditional_scope(current, policy)?
        };
        let slot = super::super::source_command_world::publication_slot(
            &scope,
            input,
            tcl_registry::CommandBindingDefinitionKind::Procedure,
        )?;
        let simple = slot.simple.clone();
        let key = |namespace| SourceCommandKey::slot(namespace, simple.clone());
        let holder = match policy.recipe() {
            tcl_syntax::naming::NativeNameProtocol::C(_) => {
                SignatureNamespaceScope::C(slot.namespace.clone())
            }
            tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                return Some((key(current.clone()), slot));
            }
        };
        if holder == scope && !tcl_syntax::naming::is_absolute_bytes(input.bytes()) {
            return Some((key(current.clone()), slot));
        }
        if matches!(&holder, SignatureNamespaceScope::C(path) if path.is_root()) {
            return self
                .source_root_namespace_key()
                .map(|namespace| (key(namespace), slot));
        }
        self.original_command_world
            .conditional_namespace_for_scope(&holder, policy)
            .map(|namespace| (key(namespace), slot))
    }

    pub(in super::super) fn original_publication_key(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
        purpose: super::super::namespace_slots::PublicationPurpose,
    ) -> Option<SourceCommandKey> {
        match input {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        let scope = if tcl_syntax::naming::is_absolute_bytes(input.bytes()) {
            SignatureNamespaceScope::root(Some(input.policy()))
        } else {
            self.original_namespace_geometry(current, input.policy())?
        };
        let recipe = input.policy().recipe();
        let slot = match purpose {
            super::super::namespace_slots::PublicationPurpose::Procedure => {
                recipe.command_publication_slot(scope.context()?, input.bytes())
            }
            super::super::namespace_slots::PublicationPurpose::Rename => {
                recipe.rename_destination_slot(scope.context()?, input.bytes())
            }
            super::super::namespace_slots::PublicationPurpose::Alias => {
                recipe.alias_publication_slot(scope.context()?, input.bytes())
            }
            super::super::namespace_slots::PublicationPurpose::Command => {
                recipe.command_c_api_publication_slot(scope.context()?, input.bytes())
            }
            super::super::namespace_slots::PublicationPurpose::Object => {
                recipe.oo_object_publication_slot(scope.context()?, input.bytes())
            }
        }
        .ok()?;
        if matches!(
            purpose,
            super::super::namespace_slots::PublicationPurpose::Procedure
        ) && let tcl_syntax::naming::NativeNameProtocol::C(version) = recipe
        {
            tcl_registry::native_procedure::procedure_name_creation_error(
                tcl_registry::InvocationDialect::for_version(version),
                slot.namespace.is_root(),
                slot.simple.as_bytes(),
            )?
            .ok()?;
        }
        self.original_command_key_for_slot(&slot, input.policy())
    }

    /// Select a modeled table identity from exact retained geometry. Registry
    /// ASCII metadata is a separate baseline purpose, never a source producer.
    pub(in super::super) fn original_command_key_for_slot(
        &self,
        slot: &tcl_core_types::ByteCommandSlot,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<SourceCommandKey> {
        #[cfg(any(test, debug_assertions))]
        let geometry_timing = OriginalGeometryTiming::at_query(0);
        let query = self.original_command_world.query_view(self);
        #[cfg(any(test, debug_assertions))]
        if let Some(timing) = geometry_timing.as_ref() {
            timing.record_clone(query.clone_nanos());
        }
        if query.selected_policy()? != policy {
            return None;
        }
        let world = query.world();
        let mut namespaces =
            world.keys_with_c_scope_view(&self.namespaces, &slot.namespace, policy);
        if slot.namespace.is_root() {
            let root = self.source_root_namespace_key().unwrap_or_default();
            if let Some(root) = self.namespaces.get(&root)
                && world.has_jim_scope(root, policy)
            {
                namespaces.push(root);
            }
        }
        let [namespace] = namespaces.as_slice() else {
            return None;
        };
        Some(SourceCommandKey::slot(
            (*namespace).clone(),
            slot.simple.clone(),
        ))
    }

    pub(in super::super) fn original_command_paths_for_input(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> Option<Vec<Vec<SourceCommandKey>>> {
        match input {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        self.original_command_paths_for_bytes(current, input.bytes(), input.policy())
    }

    /// Alias loop topology may end at a known missing target namespace.
    /// None for a cell key means that modeled holder is absent; it grants no
    /// namespace, callable target, or later dispatch. Unknown geometry declines.
    pub(in super::super) fn original_alias_loop_paths_for_input(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> Option<OriginalAliasLoopPaths> {
        match input {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        self.original_command_slot_paths_for_bytes(current, input.bytes(), input.policy())?
            .into_iter()
            .map(|path| {
                path.into_iter()
                    .map(|slot| {
                        let key = self
                            .original_lookup_key_for_slot(&slot, input.policy())?
                            .key();
                        Some((slot, key))
                    })
                    .collect()
            })
            .collect()
    }

    /// Retained cells of exact lookup candidates computed outside this
    /// table's namespace paths (a runtime receiver's member body). A missing
    /// holder namespace contributes no cell; unknown geometry declines.
    pub(in super::super) fn original_command_paths_for_slots(
        &self,
        candidates: &[Vec<tcl_core_types::ByteCommandSlot>],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<Vec<Vec<SourceCommandKey>>> {
        if self.has_opaque_domain() {
            return None;
        }
        candidates
            .iter()
            .map(|path| {
                let mut retained = Vec::new();
                for slot in path {
                    if let OriginalLookupSlot::Retained(key) =
                        self.original_lookup_key_for_slot(slot, policy)?
                    {
                        retained.push(key);
                    }
                }
                Some(retained)
            })
            .collect()
    }

    /// Explicit fixed Registry metadata proposal. It shares the original
    /// table path geometry without manufacturing an original source operand.
    pub(in super::super) fn original_registry_command_paths(
        &self,
        current: &SourceNamespaceKey,
        name: &str,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<Vec<Vec<SourceCommandKey>>> {
        if !name.is_ascii() || name.as_bytes().contains(&0) {
            return None;
        }
        self.original_command_paths_for_bytes(current, name.as_bytes(), policy)
    }

    fn original_command_paths_for_bytes(
        &self,
        current: &SourceNamespaceKey,
        bytes: &[u8],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<Vec<Vec<SourceCommandKey>>> {
        self.original_command_slot_paths_for_bytes(current, bytes, policy)?
            .into_iter()
            .map(|path| {
                let mut retained = Vec::new();
                for slot in path {
                    match self.original_lookup_key_for_slot(&slot, policy)? {
                        OriginalLookupSlot::Retained(key) => retained.push(key),
                        OriginalLookupSlot::MissingHolder => {}
                    }
                }
                Some(retained)
            })
            .collect()
    }

    /// The slot an authored namespace publishes `name` into under the
    /// selected name policy: the protocol's own publication slot for the
    /// namespace's scope, keyed as [`Self::original_command_key_for_slot`]
    /// keys the lookup that reads it. `None` where lookup itself takes no
    /// policy path (a non-ASCII or NUL name, no selected policy, an opaque
    /// domain) or the slot has no unique retained namespace.
    pub(in super::super) fn original_authored_publication_key(
        &self,
        current: &SourceNamespaceKey,
        name: &str,
        purpose: super::super::namespace_slots::PublicationPurpose,
    ) -> Option<SourceCommandKey> {
        use super::super::namespace_slots::PublicationPurpose;
        if !name.is_ascii() || name.as_bytes().contains(&0) {
            return None;
        }
        let policy = self
            .baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)?;
        let query = self.original_command_world.query_view(self);
        if query.selected_policy()? != policy || self.has_opaque_domain() {
            return None;
        }
        let bytes = name.as_bytes();
        let scope = if tcl_syntax::naming::is_absolute_bytes(bytes) {
            SignatureNamespaceScope::root(Some(policy))
        } else {
            query.world().scope(current, policy)?
        };
        let context = scope.context()?;
        let recipe = policy.recipe();
        let slot = match purpose {
            PublicationPurpose::Procedure => recipe.command_publication_slot(context, bytes),
            PublicationPurpose::Rename => recipe.rename_destination_slot(context, bytes),
            PublicationPurpose::Alias => recipe.alias_publication_slot(context, bytes),
            PublicationPurpose::Command => recipe.command_c_api_publication_slot(context, bytes),
            PublicationPurpose::Object => recipe.oo_object_publication_slot(context, bytes),
        }
        .ok()?;
        self.original_command_key_for_slot(&slot, policy)
    }

    pub(super) fn original_command_slot_paths_for_bytes(
        &self,
        current: &SourceNamespaceKey,
        bytes: &[u8],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<Vec<Vec<tcl_core_types::ByteCommandSlot>>> {
        use tcl_syntax::naming::NativeNameProtocol;
        #[cfg(any(test, debug_assertions))]
        let geometry_timing = OriginalGeometryTiming::at_query(1);
        let query = self.original_command_world.query_view(self);
        #[cfg(any(test, debug_assertions))]
        if let Some(timing) = geometry_timing.as_ref() {
            timing.record_clone(query.clone_nanos());
        }
        if query.selected_policy()? != policy || self.has_opaque_domain() {
            return None;
        }
        let scope = if tcl_syntax::naming::is_absolute_bytes(bytes) {
            SignatureNamespaceScope::root(Some(policy))
        } else {
            query.world().scope(current, policy)?
        };
        let recipe = policy.recipe();
        let mut paths = Vec::new();
        match recipe {
            NativeNameProtocol::Jim084 => {
                paths.push(
                    recipe
                        .jim_command_lookup_keys(scope.context()?, bytes)
                        .ok()?
                        .into_iter()
                        .map(|simple| tcl_core_types::ByteCommandSlot {
                            namespace: tcl_core_types::ByteNamespacePath::root(),
                            simple,
                        })
                        .collect::<Vec<_>>(),
                );
            }
            NativeNameProtocol::C(_) => {
                paths = self.original_c_command_slot_paths(
                    current, bytes, policy, recipe, &query, &scope,
                )?;
            }
        }
        Some(paths)
    }

    fn original_c_command_slot_paths(
        &self,
        current: &SourceNamespaceKey,
        bytes: &[u8],
        policy: tcl_syntax::naming::NamePolicyProtocol,
        recipe: tcl_syntax::naming::NativeNameProtocol,
        query: &super::super::source_command_world::OriginalSourceCommandQuery<'_>,
        scope: &SignatureNamespaceScope,
    ) -> Option<Vec<Vec<tcl_core_types::ByteCommandSlot>>> {
        use tcl_syntax::naming::{
            CommandLookupBase, NativeCommandNamespaceRoute, NativeNameContext,
        };
        #[derive(Clone)]
        enum LookupScope<'a> {
            Current(&'a SignatureNamespaceScope),
            Path(&'a SourceNamespaceKey),
        }
        let selected = recipe
            .command_lookup_projection(scope.context()?, bytes)
            .ok()?;
        let absolute = selected.namespace_route() == NativeCommandNamespaceRoute::Root;
        if !absolute && self.unknown_lookup_namespaces.contains(current) {
            return None;
        }
        let local = (!absolute).then(|| selected.slot().clone());
        let occupied = |slot: &tcl_core_types::ByteCommandSlot| {
            self.original_command_key_for_slot(slot, policy)
                .and_then(|key| {
                    self.original_bindings_view(&key).map(|bindings| {
                        !bindings.is_empty() && !bindings.contains(&MayBinding::Missing)
                    })
                })
                .is_some_and(|occupied| occupied)
        };
        // An occupied current cell stops lookup before path uncertainty.
        if let Some(local) = &local
            && occupied(local)
        {
            return Some(vec![vec![local.clone()]]);
        }
        let uses_paths = selected.lookup_uses_namespace_path().ok()?;
        if uses_paths && self.unknown_namespace_paths.contains(current) {
            return None;
        }
        let empty = BTreeSet::from([Vec::new()]);
        let mut paths = Vec::new();
        for path in self.namespace_paths.get(current).unwrap_or(&empty) {
            let path_scopes: Vec<_> = path.iter().map(LookupScope::Path).collect();
            let mut slots = Vec::new();
            for base in selected
                .lookup_bases(LookupScope::Current(scope), &path_scopes)
                .ok()?
            {
                let (slot, stopped) = match base {
                    CommandLookupBase::Context(LookupScope::Current(selected)) => (
                        recipe
                            .command_lookup_slot(selected.context()?, bytes)
                            .ok()?,
                        false,
                    ),
                    CommandLookupBase::Context(LookupScope::Path(namespace)) => {
                        if !self.namespaces.contains(namespace) {
                            continue;
                        }
                        let selected = query.world().scope(namespace, policy)?;
                        let slot = recipe
                            .command_lookup_slot(selected.context()?, bytes)
                            .ok()?;
                        let stopped = occupied(&slot);
                        (slot, stopped)
                    }
                    CommandLookupBase::Root => (
                        recipe
                            .command_lookup_slot(NativeNameContext::root(), bytes)
                            .ok()?,
                        false,
                    ),
                };
                if !slots.contains(&slot) {
                    slots.push(slot);
                }
                if stopped {
                    break;
                }
            }
            paths.push(slots);
        }
        Some(paths)
    }

    /// Retained source-only namespace geometry under the independently selected
    /// naming policy. Authored labels never supply bytes or native incarnation.
    pub(crate) fn original_namespace_geometry(
        &self,
        namespace: &SourceNamespaceKey,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<SignatureNamespaceScope> {
        #[cfg(any(test, debug_assertions))]
        let geometry_timing = OriginalGeometryTiming::at_query(2);
        if !self.namespaces.contains(namespace) {
            return None;
        }
        let query = self.original_command_world.query_view(self);
        #[cfg(any(test, debug_assertions))]
        if let Some(timing) = geometry_timing.as_ref() {
            timing.record_clone(query.clone_nanos());
        }
        (query.selected_policy()? == policy).then_some(())?;
        query.world().scope(namespace, policy)
    }

    // None is unknown geometry; MissingHolder is a known absent modelled holder.
    // Readonly lookup may omit a known absent holder. Publication still
    // requires original_command_key_for_slot and its actual retained owner.
    pub(super) fn original_lookup_key_for_slot(
        &self,
        slot: &tcl_core_types::ByteCommandSlot,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<OriginalLookupSlot> {
        #[cfg(any(test, debug_assertions))]
        let geometry_timing = OriginalGeometryTiming::at_query(4);
        if let Some(key) = self.original_command_key_for_slot(slot, policy) {
            if let SourceCommandKey::Slot { namespace, .. } = &key
                && self.unknown_lookup_namespaces.contains(namespace)
            {
                return None;
            }
            return Some(OriginalLookupSlot::Retained(key));
        }
        let query = self.original_command_world.query_view(self);
        #[cfg(any(test, debug_assertions))]
        if let Some(timing) = geometry_timing.as_ref() {
            timing.record_clone(query.clone_nanos());
        }
        if query.selected_policy()? != policy || self.baseline.unknown_entry {
            return None;
        }
        for namespace in self.namespaces.iter() {
            let matches = match query.world().scope(namespace, policy)? {
                SignatureNamespaceScope::C(path) => path == slot.namespace,
                SignatureNamespaceScope::Jim(_) => slot.namespace.is_root(),
                SignatureNamespaceScope::Symbolic(_) => return None,
            };
            if matches {
                return None;
            }
        }
        Some(OriginalLookupSlot::MissingHolder)
    }
}
