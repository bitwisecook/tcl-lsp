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

//! The catalogue and advice candidates an original name may select.

use super::super::{MayBinding, ModuleCommandBindings};
#[cfg(any(test, debug_assertions))]
use super::OriginalGeometryTiming;
use super::{
    AdviceRows, RegistryAdviceSlotIndex, original_registry_metadata_slot, registry_metadata_slot_is,
};
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_names::command::{BindingKind, SourceCommandKey};
use tcl_names::namespace::{SignatureNamespaceScope, SourceNamespaceKey};

enum CatalogueCandidate<'a> {
    Absent,
    Present((&'a String, &'a String)),
}

impl<'a> CatalogueCandidate<'a> {
    pub(super) fn row(self) -> Option<(&'a String, &'a String)> {
        match self {
            Self::Absent => None,
            Self::Present(row) => Some(row),
        }
    }
}

/// One barrier classifier shared by separately owned Native and vendor source
/// coordinates. It grants schema applicability only, never a live lookup.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum CatalogueSourceCell {
    Pass,
    Alternative,
    Barrier,
}

pub(in super::super) fn classify_catalogue_source_cell(
    bindings: &[&MayBinding],
    catalogue: Option<&str>,
) -> CatalogueSourceCell {
    classify_catalogue_source_cell_with(bindings, catalogue, |target, descriptor| {
        target == descriptor
    })
}

fn classify_catalogue_source_cell_with(
    bindings: &[&MayBinding],
    catalogue: Option<&str>,
    same_descriptor: impl Fn(&str, &str) -> bool,
) -> CatalogueSourceCell {
    let unknown = bindings
        .iter()
        .any(|binding| matches!(binding, MayBinding::Unknown));
    let missing = bindings
        .iter()
        .any(|binding| matches!(binding, MayBinding::Missing));
    let compatible = |binding: &&MayBinding| {
        matches!(binding,MayBinding::Target(target)
        if catalogue.is_some_and(|name| target.registry_backed && target.kind==BindingKind::Builtin
            && target.prepended.is_empty() && same_descriptor(&target.command, name)))
    };
    if bindings.is_empty() {
        return CatalogueSourceCell::Pass;
    }
    if catalogue.is_some() {
        if !unknown && bindings.iter().any(|binding| !compatible(binding)) {
            CatalogueSourceCell::Barrier
        } else if unknown || missing || bindings.iter().any(|binding| !compatible(binding)) {
            CatalogueSourceCell::Alternative
        } else {
            CatalogueSourceCell::Pass
        }
    } else if !unknown && !missing {
        CatalogueSourceCell::Barrier
    } else if unknown
        || bindings
            .iter()
            .any(|binding| !matches!(binding, MayBinding::Missing))
    {
        CatalogueSourceCell::Alternative
    } else {
        CatalogueSourceCell::Pass
    }
}

impl ModuleCommandBindings {
    fn original_catalogue_slot_for_key(
        &self,
        key: &SourceCommandKey,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<tcl_core_types::ByteCommandSlot> {
        match key {
            SourceCommandKey::Authored(name) => original_registry_metadata_slot(policy, name),
            SourceCommandKey::Slot { namespace, simple } => {
                let scope = self
                    .original_command_world
                    .retained_scope_for_source_advice(namespace, policy)?;
                match scope {
                    SignatureNamespaceScope::C(namespace) => {
                        Some(tcl_core_types::ByteCommandSlot {
                            namespace,
                            simple: simple.clone(),
                        })
                    }
                    SignatureNamespaceScope::Jim(_) => Some(tcl_core_types::ByteCommandSlot {
                        namespace: tcl_core_types::ByteNamespacePath::root(),
                        simple: simple.clone(),
                    }),
                    SignatureNamespaceScope::Symbolic(_) => None,
                }
            }
        }
    }

    pub(in super::super) fn original_catalogue_bindings_for_slot(
        &self,
        slot: &tcl_core_types::ByteCommandSlot,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Vec<&MayBinding> {
        self.bindings
            .iter()
            .filter(|(key, _)| match key {
                SourceCommandKey::Authored(name) => registry_metadata_slot_is(policy, name, slot),
                // A slot key projects its own simple name unchanged.
                SourceCommandKey::Slot { simple, .. } => {
                    simple == &slot.simple
                        && self.original_catalogue_slot_for_key(key, policy).as_ref() == Some(slot)
                }
            })
            .flat_map(|(_, alternatives)| alternatives.iter())
            .collect()
    }

    fn original_imported_catalogue_candidate<'a>(
        &'a self,
        implementations: &[&'a MayBinding],
        policy: tcl_syntax::naming::NamePolicyProtocol,
        index: Option<&'a RegistryAdviceSlotIndex>,
    ) -> Option<CatalogueCandidate<'a>> {
        let mut imported_name = None;
        for binding in implementations {
            let MayBinding::Target(target) = binding else {
                continue;
            };
            if !target.registry_backed
                || target.kind != BindingKind::Builtin
                || !target.prepended.is_empty()
                || imported_name.is_some_and(|name| name != target.command)
            {
                return None;
            }
            imported_name = Some(target.command.as_str());
        }
        let Some(name) = imported_name else {
            return Some(CatalogueCandidate::Absent);
        };
        let origin = original_registry_metadata_slot(policy, name)?;
        let candidate = if let Some(index) = index {
            index
                .slots
                .get(&origin)?
                .as_ref()
                .map(|(metadata, name)| (metadata, name))
        } else {
            let mut candidates = self
                .baseline
                .catalogue_commands
                .iter()
                .filter(|(name, _)| registry_metadata_slot_is(policy, name, &origin));
            let first = candidates.next();
            if candidates.next().is_some() {
                return None;
            }
            first
        };
        Some(candidate.map_or(CatalogueCandidate::Absent, CatalogueCandidate::Present))
    }

    fn original_catalogue_path_candidate(
        &self,
        current: &SourceNamespaceKey,
        path: &[tcl_core_types::ByteCommandSlot],
        policy: tcl_syntax::naming::NamePolicyProtocol,
        obligations: &mut Vec<super::super::OriginalCatalogueSourceObligation>,
    ) -> Option<super::super::DeclaredCommandCandidate> {
        use super::super::OriginalCatalogueSourceObligation as Obligation;
        let index = self
            .baseline
            .advice_slots
            .index(policy, &self.baseline.catalogue_commands);
        let declared_index = self
            .baseline
            .advice_slots
            .index(policy, &self.baseline.declared_commands);
        let mut selected = None;
        for slot in path {
            let mut candidate = if let Some(index) = &index {
                match index.slots.get(slot) {
                    Some(Some((metadata, name))) => Some((metadata, name)),
                    Some(None) => return None,
                    None => None,
                }
            } else {
                let mut candidates = self
                    .baseline
                    .catalogue_commands
                    .iter()
                    .filter(|(name, _)| registry_metadata_slot_is(policy, name, slot));
                let first = candidates.next();
                if candidates.next().is_some() {
                    return None;
                }
                first
            };
            let bindings = self.original_catalogue_bindings_for_slot(slot, policy);
            let mut implementations = Vec::new();
            for binding in bindings {
                if let MayBinding::Imported(imported) = binding {
                    implementations.extend(self.objects.get(&imported.origin)?.iter());
                } else {
                    implementations.push(binding);
                }
            }
            if candidate.is_none() {
                candidate = self
                    .original_imported_catalogue_candidate(
                        &implementations,
                        policy,
                        index.as_deref(),
                    )?
                    .row();
            }
            match classify_catalogue_source_cell_with(
                &implementations,
                candidate.map(|(_, name)| name.as_str()),
                |target, descriptor| {
                    let slot = original_registry_metadata_slot(policy, target);
                    slot.is_some() && slot == original_registry_metadata_slot(policy, descriptor)
                },
            ) {
                CatalogueSourceCell::Barrier => return None,
                CatalogueSourceCell::Alternative if candidate.is_some() => {
                    obligations.push(Obligation::SelectedBindingAlternative(slot.clone()));
                }
                CatalogueSourceCell::Alternative => {
                    obligations.push(Obligation::EarlierBindingAlternative(slot.clone()));
                }
                CatalogueSourceCell::Pass => {}
            }
            if declared_index.as_ref().map_or_else(
                || {
                    self.baseline
                        .declared_commands
                        .keys()
                        .any(|name| registry_metadata_slot_is(policy, name, slot))
                },
                |index| index.slots.contains_key(slot),
            ) {
                return None;
            }
            if let Some((metadata, name)) = candidate {
                if self.original_command_key_for_slot(slot, policy).is_none() {
                    obligations.push(Obligation::UnretainedHolder(slot.clone()));
                }
                selected = Some(super::super::DeclaredCommandCandidate {
                    name: name.clone(),
                    slot: metadata.clone(),
                    lookup_namespace: current.display().unwrap_or_default(),
                });
                break;
            }
        }
        selected
    }

    /// Conditional source catalogue matching is separate from closed lookup.
    /// Existing exact cells, tombstones and document overrides remain barriers;
    /// unknown state is retained on the returned readonly source schema.
    pub(in super::super) fn original_catalogue_source_candidate(
        &self,
        current: &SourceNamespaceKey,
        namespace: &SignatureNamespaceScope,
        input: &SignatureSourceNameInput,
    ) -> Option<(
        super::super::DeclaredCommandCandidate,
        Vec<Vec<tcl_core_types::ByteCommandSlot>>,
        Vec<super::super::OriginalCatalogueSourceObligation>,
    )> {
        use super::super::OriginalCatalogueSourceObligation as Obligation;
        match input {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        let policy = input.policy();
        if self
            .baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            != Some(policy)
        {
            return None;
        }
        let mut obligations = Vec::new();
        if self.has_opaque_domain()
            || self.baseline.unknown_entry
            || self.unknown_lookup_namespaces.contains(current)
        {
            obligations.push(Obligation::UnknownCurrentLookup);
        }
        if self
            .original_command_world
            .retained_scope_for_source_advice(current, policy)
            .as_ref()
            != Some(namespace)
        {
            obligations.push(Obligation::UnknownExecutingNamespace);
        }
        let paths = if let Some(paths) =
            self.original_command_slot_paths_for_bytes(current, input.bytes(), policy)
        {
            paths
        } else {
            // A missing path owner cannot donate an empty namespace path. This
            // bounded schema accepts only independently known empty path rows.
            if self.unknown_namespace_paths.contains(current)
                || self
                    .namespace_paths
                    .get(current)
                    .is_some_and(|paths| paths.iter().any(|path| !path.is_empty()))
            {
                return None;
            }
            obligations.push(Obligation::UnknownLookupCoordinates);
            vec![
                crate::signature_scan::scope::SignatureSourceLookup::from_input(
                    namespace.clone(),
                    input,
                )?
                .candidates()?,
            ]
        };
        let mut unanimous = None;
        for path in &paths {
            let selected =
                self.original_catalogue_path_candidate(current, path, policy, &mut obligations)?;
            if unanimous.as_ref().is_some_and(|prior| prior != &selected) {
                return None;
            }
            unanimous = Some(selected);
        }
        Some((unanimous?, paths, obligations))
    }

    /// Readonly catalogue or document advice at genuine original geometry.
    /// A missing holder may support an unloaded catalogue candidate; it never
    /// supplies a runtime namespace, table entry or implementation.
    pub(in super::super) fn original_advice_candidates_for_input(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> (
        Option<super::super::DeclaredCommandCandidate>,
        Option<super::super::DeclaredCommandCandidate>,
    ) {
        if !input.is_current(&self.source_variables) {
            return (None, None);
        }
        self.original_advice_candidates_for_bytes(current, input.bytes(), input.policy())
    }

    /// Explicit future/document declaration metadata at an original operand
    /// and genuine namespace identity. The subset supplies no runtime binding.
    pub(in super::super) fn original_declared_candidate_from_input(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
        declarations: &super::super::BTreeMap<String, String>,
    ) -> Option<super::super::DeclaredCommandCandidate> {
        input.is_current(&self.source_variables).then_some(())?;
        let paths =
            self.original_command_slot_paths_for_bytes(current, input.bytes(), input.policy())?;
        self.original_advice_candidate_from_paths(
            current,
            &paths,
            input.policy(),
            declarations,
            false,
        )
    }

    /// Fixed metadata proposals retain their authored provenance. No original
    /// source word or value is reconstructed from this compatibility argument.
    pub(in super::super) fn original_advice_candidates_for_metadata(
        &self,
        current: &SourceNamespaceKey,
        name: &str,
    ) -> (
        Option<super::super::DeclaredCommandCandidate>,
        Option<super::super::DeclaredCommandCandidate>,
    ) {
        #[cfg(any(test, debug_assertions))]
        let geometry_timing = OriginalGeometryTiming::at_query(3);
        if !name.is_ascii() || name.as_bytes().contains(&0) {
            return (None, None);
        }
        let query = self.original_command_world.query_view(self);
        #[cfg(any(test, debug_assertions))]
        if let Some(timing) = geometry_timing.as_ref() {
            timing.record_clone(query.clone_nanos());
        }
        let Some(policy) = query.selected_policy() else {
            return (None, None);
        };
        self.original_advice_candidates_for_bytes(current, name.as_bytes(), policy)
    }

    fn original_advice_candidates_for_bytes(
        &self,
        current: &SourceNamespaceKey,
        bytes: &[u8],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> (
        Option<super::super::DeclaredCommandCandidate>,
        Option<super::super::DeclaredCommandCandidate>,
    ) {
        let Some(paths) = self.original_command_slot_paths_for_bytes(current, bytes, policy) else {
            return (None, None);
        };
        (
            self.original_advice_candidate_from_paths(
                current,
                &paths,
                policy,
                &self.baseline.declared_commands,
                false,
            ),
            self.original_advice_candidate_from_paths(
                current,
                &paths,
                policy,
                &self.baseline.catalogue_commands,
                true,
            ),
        )
    }

    fn original_advice_index_for_rows(
        &self,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        rows: &AdviceRows,
    ) -> Option<std::sync::Arc<RegistryAdviceSlotIndex>> {
        let original = if std::ptr::eq(rows, self.baseline.declared_commands.as_ref()) {
            &self.baseline.declared_commands
        } else if std::ptr::eq(rows, self.baseline.catalogue_commands.as_ref()) {
            &self.baseline.catalogue_commands
        } else {
            return None;
        };
        self.baseline.advice_slots.index(policy, original)
    }

    fn original_advice_candidate_from_paths(
        &self,
        current: &SourceNamespaceKey,
        paths: &[Vec<tcl_core_types::ByteCommandSlot>],
        policy: tcl_syntax::naming::NamePolicyProtocol,
        declarations: &super::super::BTreeMap<String, String>,
        catalogue: bool,
    ) -> Option<super::super::DeclaredCommandCandidate> {
        let index = self.original_advice_index_for_rows(policy, declarations);
        let declared_index = if catalogue {
            self.baseline
                .advice_slots
                .index(policy, &self.baseline.declared_commands)
        } else {
            None
        };
        let mut unanimous = None;
        for path in paths {
            let mut selected = None;
            for slot in path {
                let key = self.original_lookup_key_for_slot(slot, policy)?.key();
                if key
                    .as_ref()
                    .is_some_and(|key| self.bindings.contains_key(key))
                {
                    return None;
                }
                let metadata_slot = |name: &str| registry_metadata_slot_is(policy, name, slot);
                if self
                    .bindings
                    .authored_keys()
                    .iter()
                    .any(|key| key.authored_spelling().is_some_and(metadata_slot))
                {
                    return None;
                }
                if catalogue
                    && declared_index.as_ref().map_or_else(
                        || {
                            self.baseline
                                .declared_commands
                                .keys()
                                .any(|name| metadata_slot(name))
                        },
                        |index| index.slots.contains_key(slot),
                    )
                {
                    return None;
                }
                let candidate = if let Some(index) = &index {
                    match index.slots.get(slot) {
                        Some(Some((metadata, name))) => Some((metadata, name)),
                        Some(None) => return None,
                        None => None,
                    }
                } else {
                    let mut matching = declarations.iter().filter(|(name, _)| metadata_slot(name));
                    let first = matching.next();
                    if matching.next().is_some() {
                        return None;
                    }
                    first
                };
                if let Some((metadata, name)) = candidate {
                    selected = Some(super::super::DeclaredCommandCandidate {
                        name: name.clone(),
                        lookup_namespace: current.display().unwrap_or_default(),
                        slot: metadata.clone(),
                    });
                    break;
                }
                if let Some(key) = key
                    && !self.original_bindings_view(&key)?.is_only_missing()
                {
                    return None;
                }
            }
            let selected = selected?;
            if unanimous
                .as_ref()
                .is_some_and(|previous| previous != &selected)
            {
                return None;
            }
            unanimous = Some(selected);
        }
        unanimous
    }

    /// Actual target selected by an explicit fixed Registry name proposal.
    /// Original source/value producers remain separate. Alias wrappers cannot
    /// supply the immutable direct worker, even when their bytes agree.
    pub(in super::super) fn original_registry_metadata_target(
        &self,
        current: &SourceNamespaceKey,
        name: &str,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<super::super::ResolvedCommandTarget> {
        let paths = self.original_registry_command_paths(current, name, policy)?;
        let mut unanimous = None;
        for path in paths {
            let mut selected = None;
            let mut absent = true;
            for key in path {
                let bindings = self.original_bindings_for_key(&key)?;
                if bindings.is_empty() {
                    return None;
                }
                for binding in &bindings {
                    let implementations = match binding {
                        MayBinding::Target(target) => vec![target],
                        MayBinding::Imported(imported) => self
                            .objects
                            .get(&imported.origin)?
                            .iter()
                            .map(|implementation| match implementation {
                                MayBinding::Target(target) => Some(target),
                                _ => None,
                            })
                            .collect::<Option<Vec<_>>>()?,
                        MayBinding::Missing => continue,
                        MayBinding::Unknown => return None,
                    };
                    if implementations.is_empty() {
                        return None;
                    }
                    for target in implementations {
                        if !target.terminal
                            || !target.registry_backed
                            || target.kind == tcl_names::command::BindingKind::Alias
                            || !target.prepended.is_empty()
                        {
                            return None;
                        }
                        if selected.as_ref().is_some_and(|previous| previous != target) {
                            return None;
                        }
                        selected = Some(target.clone());
                    }
                }
                if !bindings.contains(&MayBinding::Missing) {
                    absent = false;
                    break;
                }
            }
            if absent {
                return None;
            }
            let selected = selected?;
            if unanimous
                .as_ref()
                .is_some_and(|previous| previous != &selected)
            {
                return None;
            }
            unanimous = Some(selected);
        }
        unanimous
    }
}
