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

//! The targets of an original name operand: occupied commands and resolved handlers.

use super::super::{
    BTreeSet, MayBinding, ModuleCommandBindings, ResolvedCommandTarget, SourceCommandKey,
    SourceNamespaceKey,
};
#[cfg(any(test, debug_assertions))]
use super::OriginalGeometryTiming;
use crate::signature_scan::scope::{SignatureNamespaceScope, SignatureSourceNameInput};

pub(in super::super) struct OriginalCommandTargetSelection {
    pub(in super::super) targets: BTreeSet<ResolvedCommandTarget>,
    pub(in super::super) original_prefixes:
        super::super::BTreeMap<ResolvedCommandTarget, Option<Vec<SignatureSourceNameInput>>>,
    pub(in super::super) may_be_absent: bool,
    pub(in super::super) unknown: bool,
}

impl OriginalCommandTargetSelection {
    pub(super) fn insert(
        &mut self,
        target: ResolvedCommandTarget,
        inputs: Option<Vec<SignatureSourceNameInput>>,
    ) {
        self.original_prefixes
            .entry(target.clone())
            .and_modify(|previous| {
                if *previous != inputs {
                    *previous = None;
                }
            })
            .or_insert(inputs);
        self.targets.insert(target);
    }
}

#[derive(Clone, Copy)]
struct OriginalTargetLookupContext<'a> {
    pub(super) policy: tcl_syntax::naming::NamePolicyProtocol,
    pub(super) namespace: &'a SourceNamespaceKey,
    lookup: super::super::CommandTargetLookup,
}

impl ModuleCommandBindings {
    /// Candidate table cells that can win one of the genuine namespace paths.
    /// Missing alternatives continue to later cells; occupied cells stop that
    /// path. This projection retains original table keys for compiler owners.
    pub(in super::super) fn original_command_keys_for_input(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> Option<Vec<SourceCommandKey>> {
        let mut keys = Vec::new();
        for path in self.original_command_paths_for_input(current, input)? {
            for key in path {
                let bindings = self.original_bindings_for_key(&key)?;
                if !keys.contains(&key) {
                    keys.push(key);
                }
                if !bindings.contains(&MayBinding::Missing) {
                    break;
                }
            }
        }
        Some(keys)
    }

    /// Actual occupied raw cell for one original operand. Unlike the compiler
    /// traversal, earlier proven Missing cells are not returned as targets.
    /// Every possible path must reach the same complete current cell; alias
    /// resolution and imported implementation selection remain separate.
    pub(in super::super) fn original_occupied_command_for_input(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> Option<(SourceCommandKey, BTreeSet<MayBinding>)> {
        if !input.is_current(&self.source_variables) || self.has_opaque_domain() {
            return None;
        }
        self.original_occupied_command_in_paths(
            self.original_command_paths_for_input(current, input)?,
        )
    }

    /// Current occupied cell for explicit fixed Registry metadata. This retains
    /// table occupancy only; bootstrap roles and handlers are independent.
    pub(in super::super) fn original_occupied_registry_metadata_command(
        &self,
        current: &SourceNamespaceKey,
        name: &str,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<(SourceCommandKey, BTreeSet<MayBinding>)> {
        if self.has_opaque_domain() {
            return None;
        }
        self.original_occupied_command_in_paths(
            self.original_registry_command_paths(current, name, policy)?,
        )
    }

    fn original_occupied_command_in_paths(
        &self,
        paths: Vec<Vec<SourceCommandKey>>,
    ) -> Option<(SourceCommandKey, BTreeSet<MayBinding>)> {
        let mut unanimous = None;
        for path in paths {
            let mut selected = None;
            for key in path {
                let bindings = self.original_bindings_for_key(&key)?;
                if bindings == BTreeSet::from([MayBinding::Missing]) {
                    continue;
                }
                if bindings.is_empty()
                    || bindings.contains(&MayBinding::Missing)
                    || bindings.contains(&MayBinding::Unknown)
                {
                    return None;
                }
                selected = Some((key, bindings));
                break;
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

    /// Exact source table geometry, independently of its reporting label.
    pub(in super::super) fn original_byte_slot_for_key(
        &self,
        key: &SourceCommandKey,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<tcl_core_types::ByteCommandSlot> {
        let SourceCommandKey::Slot { namespace, simple } = key else {
            return None;
        };
        let scope = self.original_namespace_geometry(namespace, policy)?;
        let namespace = match scope {
            SignatureNamespaceScope::C(path) => path,
            SignatureNamespaceScope::Jim(_) => tcl_core_types::ByteNamespacePath::root(),
            SignatureNamespaceScope::Symbolic(_) => return None,
        };
        Some(tcl_core_types::ByteCommandSlot {
            namespace,
            simple: simple.clone(),
        })
    }

    pub(in super::super) fn original_targets_for_input(
        &self,
        input: &SignatureSourceNameInput,
        namespace: &SourceNamespaceKey,
        lookup: super::super::CommandTargetLookup,
    ) -> Option<OriginalCommandTargetSelection> {
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            let paths = self.original_command_paths_for_input(namespace, input);
            eprintln!(
                "ORIGINAL_COMMAND_TARGET_QUERY input={:?} policy={:?} namespace={:?} opaque={} paths={:?} cells={:?}",
                input.bytes(),
                input.policy(),
                namespace,
                self.has_opaque_domain(),
                paths,
                paths.as_ref().map(|paths| paths
                    .iter()
                    .flat_map(|path| path.iter())
                    .map(|key| (key, self.original_bindings_for_key(key)))
                    .collect::<Vec<_>>())
            );
        }
        let paths = self.original_command_paths_for_input(namespace, input)?;
        self.original_targets_for_paths(
            input.policy(),
            namespace,
            lookup,
            paths,
            &mut BTreeSet::new(),
        )
    }

    pub(in super::super) fn original_targets_for_receiver(
        &self,
        input: &SignatureSourceNameInput,
        helper_path: &[&str],
        namespace: &SourceNamespaceKey,
        lookup: super::super::CommandTargetLookup,
    ) -> Option<OriginalCommandTargetSelection> {
        let (slots, local) = super::super::source_name_lookup::original_receiver_candidates(
            input.bytes(),
            input.policy(),
            helper_path,
        )?;
        if local
            && tcl_registry::definer::DefinerFamily::TclOo
                .receiver_namespace_commands()
                .iter()
                .any(|name| name.as_bytes() == input.bytes())
        {
            return None;
        }
        let paths = self.original_command_paths_for_slots(&[slots], input.policy())?;
        self.original_targets_for_paths(
            input.policy(),
            namespace,
            lookup,
            paths,
            &mut BTreeSet::new(),
        )
    }

    fn original_targets_for_paths(
        &self,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        namespace: &SourceNamespaceKey,
        lookup: super::super::CommandTargetLookup,
        paths: Vec<Vec<SourceCommandKey>>,
        visiting: &mut BTreeSet<SourceCommandKey>,
    ) -> Option<OriginalCommandTargetSelection> {
        let mut selected = OriginalCommandTargetSelection {
            targets: BTreeSet::new(),
            original_prefixes: super::super::BTreeMap::new(),
            may_be_absent: false,
            unknown: false,
        };
        for path in paths {
            let mut absent = true;
            for key in path {
                if !visiting.insert(key.clone()) {
                    selected.unknown = true;
                    absent = false;
                    break;
                }
                let bindings = self.original_bindings_view(&key)?;
                for binding in bindings.iter() {
                    match binding {
                        MayBinding::Target(target) => self.original_resolve_target(
                            target,
                            &key,
                            OriginalTargetLookupContext {
                                policy,
                                namespace,
                                lookup,
                            },
                            visiting,
                            &mut selected,
                        )?,
                        MayBinding::Imported(import) => {
                            let Some(implementations) = self.objects.get(&import.origin) else {
                                selected.unknown = true;
                                continue;
                            };
                            for implementation in implementations {
                                match implementation {
                                    MayBinding::Target(target) => self.original_resolve_target(
                                        target,
                                        &key,
                                        OriginalTargetLookupContext {
                                            policy,
                                            namespace,
                                            lookup,
                                        },
                                        visiting,
                                        &mut selected,
                                    )?,
                                    MayBinding::Missing => selected.may_be_absent = true,
                                    MayBinding::Imported(_) | MayBinding::Unknown => {
                                        selected.unknown = true;
                                    }
                                }
                            }
                        }
                        MayBinding::Missing => {}
                        MayBinding::Unknown => selected.unknown = true,
                    }
                }
                visiting.remove(&key);
                absent = bindings.contains(&MayBinding::Missing);
                if !absent {
                    break;
                }
            }
            selected.may_be_absent |= absent;
        }
        // No fallback implementation is invented from an opaque report name.
        // The generic driver keeps the independently possible unknown handler.
        selected.unknown |= selected.may_be_absent
            && matches!(lookup, super::super::CommandTargetLookup::WithFallback)
            && self.original_unresolved_handler_may_run(namespace);
        Some(selected)
    }

    /// Whether a missing command can reach an unresolved-command handler in
    /// this world. The registry's native fallback policy names the handler and
    /// says whether a fresh interpreter binds it (C's init.tcl `::unknown`,
    /// none in Jim 0.84). Only a handler definitely absent from every lookup
    /// slot ends in an error. An unclosed or opaque world may hold unrecorded
    /// commands.
    fn original_unresolved_handler_may_run(&self, namespace: &SourceNamespaceKey) -> bool {
        if self.unclosed_command_world
            || self.opaque_domain
            || self.baseline.unknown_entry
            || self.namespace_unknown_handlers.contains(namespace)
            || self.unknown_lookup_namespaces.contains(namespace)
        {
            return true;
        }
        let Some(policy) = self.baseline.dialect.and_then(|dialect| {
            tcl_registry::command_lookup::native_lookup_fallback_policy(
                dialect,
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            )
        }) else {
            return true;
        };
        let Ok(keys) = self.source_keys_checked(policy.default_handler, namespace) else {
            return true;
        };
        // A handler the fresh interpreter binds stays possible until this
        // world changes its slot; an explicit removal is definite.
        keys.is_empty()
            || keys.iter().any(|key| {
                self.original_bindings_for_key(key).is_none_or(|bindings| {
                    bindings
                        .iter()
                        .any(|binding| *binding != MayBinding::Missing)
                        || (policy.default_handler_bound
                            && self.original_initial_bindings_for_key(key).as_ref()
                                == Some(&bindings))
                })
            })
    }

    fn original_resolve_target(
        &self,
        target: &ResolvedCommandTarget,
        key: &SourceCommandKey,
        context: OriginalTargetLookupContext<'_>,
        visiting: &mut BTreeSet<SourceCommandKey>,
        selected: &mut OriginalCommandTargetSelection,
    ) -> Option<()> {
        let OriginalTargetLookupContext {
            policy,
            namespace,
            lookup,
        } = context;
        // C compiles a command with execution traces as an ordinary
        // invocation; only its callable effects stay unknown.
        if matches!(lookup, super::super::CommandTargetLookup::CompilerSlots) {
            selected.insert(target.clone(), None);
            return Some(());
        }
        if self.runtime_execution_observed(target.token.as_ref()) {
            selected.unknown = true;
        }
        if target.terminal {
            selected.insert(target.clone(), target.prepended.is_empty().then(Vec::new));
            return Some(());
        }
        let slot = self.original_slot_for_command_key(key, policy)?;
        let publication = self.original_publication_at(&slot, policy)?;
        if !publication.has_implementation_allocation(target.implementation_allocation.as_ref()?) {
            return None;
        }
        let alias = publication.alias_target()?;
        let target_namespace = match alias.lookup() {
            tcl_registry::AliasTargetLookup::Global => self.source_root_namespace_key()?,
            tcl_registry::AliasTargetLookup::CallerNamespace => namespace.clone(),
        };
        let paths = self.original_command_paths_for_input(&target_namespace, alias.name_input())?;
        let result = self.original_targets_for_paths(
            alias.name_input().policy(),
            &target_namespace,
            lookup,
            paths,
            visiting,
        )?;
        selected.may_be_absent |= result.may_be_absent;
        selected.unknown |= result.unknown;
        for mut terminal in result.targets {
            let inputs = result
                .original_prefixes
                .get(&terminal)
                .cloned()
                .flatten()
                .and_then(|mut inputs| {
                    if alias.arguments().len() != target.prepended.len()
                        || alias.arguments().iter().any(|input| {
                            input.policy() != policy || !input.is_current(&self.source_variables)
                        })
                    {
                        return None;
                    }
                    inputs.extend_from_slice(alias.arguments());
                    Some(inputs)
                });
            terminal.prepended.extend(target.prepended.iter().cloned());
            selected.insert(terminal, inputs);
        }
        Some(())
    }

    pub(in super::super) fn original_slot_for_command_key(
        &self,
        key: &SourceCommandKey,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<tcl_core_types::ByteCommandSlot> {
        #[cfg(any(test, debug_assertions))]
        let geometry_timing = OriginalGeometryTiming::at_query(6);
        let SourceCommandKey::Slot { namespace, simple } = key else {
            return None;
        };
        let query = self.original_command_world.query_view(self);
        #[cfg(any(test, debug_assertions))]
        if let Some(timing) = geometry_timing.as_ref() {
            timing.record_clone(query.clone_nanos());
        }
        if query.selected_policy()? != policy {
            return None;
        }
        let namespace = match query.world().scope(namespace, policy)? {
            SignatureNamespaceScope::C(path) => path,
            SignatureNamespaceScope::Jim(_) => tcl_core_types::ByteNamespacePath::root(),
            SignatureNamespaceScope::Symbolic(_) => return None,
        };
        Some(tcl_core_types::ByteCommandSlot {
            namespace,
            simple: simple.clone(),
        })
    }
}
