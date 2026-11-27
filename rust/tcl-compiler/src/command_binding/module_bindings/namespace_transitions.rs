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

//! Namespace transitions of the may-state and the command resolution they change.

use super::super::binding_projection::command_holder;
use super::super::{
    CommandToken, EnsembleManagement, ImportedCommandBinding, MayBinding, ModuleCommandBindings,
    NamespaceResolutionProjection, ResolvedCommandTarget, namespace_slots, original_command_table,
    original_namespace_binding, original_namespace_path, source_command_world,
};
use crate::naming::normalise_qualified_name as nqn;
use std::collections::BTreeSet;
use std::sync::Arc;
use tcl_names::command::{BindingKind, SourceCommandKey};
use tcl_names::namespace::SourceNamespaceKey;
use tcl_registry::{NamespaceTransition, TransitionSubject};

fn is_default_namespace_unknown_handler(
    state: &ModuleCommandBindings,
    handler: &TransitionSubject,
) -> bool {
    handler.literal().is_some_and(|handler| {
        state.baseline.dialect.is_some_and(|dialect| {
            dialect
                .word_values
                .split_list(handler)
                .is_ok_and(|prefix| prefix.is_empty())
        })
    })
}

impl NamespaceResolutionProjection {
    pub(in super::super) fn record(
        &mut self,
        transition: &NamespaceTransition,
        namespace: &crate::ir_helpers::ExecutionNamespace,
    ) {
        let site = match namespace {
            crate::ir_helpers::ExecutionNamespace::Exact(site) => Some(site.as_str()),
            crate::ir_helpers::ExecutionNamespace::SourceContext(_)
            | crate::ir_helpers::ExecutionNamespace::RuntimeSelected => None,
        };
        if let NamespaceTransition::Ensemble { operation, .. } = transition {
            use tcl_registry::state_transition::EnsembleManagementOperation as Operation;
            match operation {
                Some(Operation::Exists) => {}
                Some(Operation::Create) => self.ensembles.created = true,
                _ => self.ensembles.reconfigured = true,
            }
        }
        let mut rebound = std::collections::HashSet::new();
        let mut opaque = std::collections::HashSet::new();
        let mut changed = false;
        if !collect_namespace_resolution(transition, site, &mut rebound, &mut changed, &mut opaque)
        {
            self.dynamic = true;
        }
        self.changed |= changed;
        self.rebound_names.extend(rebound);
        self.opaque_namespaces.extend(opaque);
    }

    pub(in super::super) fn join(&mut self, other: &Self) -> bool {
        let mut joined = false;
        if other.changed && !self.changed {
            self.changed = true;
            joined = true;
        }
        if other.dynamic && !self.dynamic {
            self.dynamic = true;
            joined = true;
        }
        let ensembles = EnsembleManagement {
            reconfigured: self.ensembles.reconfigured || other.ensembles.reconfigured,
            created: self.ensembles.created || other.ensembles.created,
        };
        if ensembles != self.ensembles {
            self.ensembles = ensembles;
            joined = true;
        }
        let rebound_count = self.rebound_names.len();
        self.rebound_names
            .extend(other.rebound_names.iter().cloned());
        joined |= self.rebound_names.len() != rebound_count;
        let opaque_count = self.opaque_namespaces.len();
        self.opaque_namespaces
            .extend(other.opaque_namespaces.iter().cloned());
        joined |= self.opaque_namespaces.len() != opaque_count;
        joined
    }
}

fn apply_namespace_ensure(
    state: &mut ModuleCommandBindings,
    namespace: &tcl_registry::NamespaceTransitionTarget,
    current: &SourceNamespaceKey,
    offset: u32,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    if let (Some(operands), tcl_registry::NamespaceTransitionTarget::Named(subject)) =
        (original_operands, namespace)
        && matches!(
            state
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                .map(tcl_syntax::naming::NamePolicyProtocol::recipe),
            Some(tcl_syntax::naming::NativeNameProtocol::C(_))
        )
        && (subject.literal().is_none() || !matches!(current, SourceNamespaceKey::Authored(_)))
    {
        let selected = operands
            .namespace_ensure_input(subject)
            .and_then(|input| state.ensure_original_namespace_key_at(current, input, offset));
        if selected.is_none() {
            state.mark_opaque_resolution();
        }
        return;
    }
    if let SourceNamespaceKey::Authored(_) = current {
        if let Some(SourceNamespaceKey::Authored(mut ancestor)) =
            state.namespace_target_key_at(namespace, current)
        {
            loop {
                Arc::make_mut(&mut state.namespaces).insert(ancestor.clone());
                source_command_world::OriginalSourceCommandWorld::ensure_authored_namespace(
                    state, &ancestor,
                );
                if ancestor == "::" {
                    break;
                }
                ancestor = command_holder(&ancestor);
            }
        }
    } else if state
        .ensure_namespace_key_at(namespace, current, offset)
        .is_none()
    {
        state.mark_opaque_resolution();
    }
}

fn apply_namespace_path(
    state: &mut ModuleCommandBindings,
    namespace: &tcl_registry::NamespaceTransitionTarget,
    path: &TransitionSubject,
    current: &SourceNamespaceKey,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    let Some(name) = state.namespace_target_key_at(namespace, current) else {
        state.mark_opaque_resolution();
        return;
    };
    if matches!(
        state.baseline.execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            .map(tcl_syntax::naming::NamePolicyProtocol::recipe),
        Some(tcl_syntax::naming::NativeNameProtocol::C(version))
            if version >= tcl_dialect::TclVersion::V8_5
    ) {
        let entries = original_operands
            .and_then(|operands| operands.namespace_input(path))
            .and_then(|input| original_namespace_path::select(state, &name, input));
        match entries {
            Some(entries) => {
                Arc::make_mut(&mut state.unknown_namespace_paths).remove(&name);
                Arc::make_mut(&mut state.namespace_paths).insert(name, BTreeSet::from([entries]));
            }
            None => {
                Arc::make_mut(&mut state.unknown_namespace_paths).insert(name);
            }
        }
        return;
    }
    let Some(path) = path
        .literal()
        .zip(state.baseline.dialect)
        .and_then(|(value, dialect)| {
            let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
            tcl_syntax::list::split_list_with_syntax(value, config.list_parse, config.escapes).ok()
        })
    else {
        Arc::make_mut(&mut state.unknown_namespace_paths).insert(name);
        return;
    };
    let Some(entries) = path
        .iter()
        .map(|entry| {
            state.namespace_target_key_at(
                &tcl_registry::NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                    entry.to_string(),
                )),
                &name,
            )
        })
        .collect::<Option<Vec<_>>>()
    else {
        Arc::make_mut(&mut state.unknown_namespace_paths).insert(name);
        return;
    };
    Arc::make_mut(&mut state.unknown_namespace_paths).remove(&name);
    Arc::make_mut(&mut state.namespace_paths).insert(name, BTreeSet::from([entries]));
}

pub(in super::super) fn apply_namespace_transition(
    state: &mut ModuleCommandBindings,
    transition: &NamespaceTransition,
    execution: &crate::ir_helpers::ExecutionNamespace,
    offset: u32,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    if matches!(
        transition,
        NamespaceTransition::SetPath { .. }
            | NamespaceTransition::SetUnknown { .. }
            | NamespaceTransition::Ensemble { .. }
    ) {
        Arc::make_mut(&mut state.object_instances).invalidate_receiver_dispatcher();
    }
    let Some(current) = execution.for_head_context("") else {
        state.mark_opaque_resolution();
        return;
    };
    let current = current.as_ref();
    match transition {
        NamespaceTransition::Ensure { namespace } => {
            apply_namespace_ensure(state, namespace, current, offset, original_operands);
        }
        NamespaceTransition::Export {
            namespace,
            patterns,
        } => {
            apply_namespace_exports(state, namespace, patterns, current, original_operands);
        }
        NamespaceTransition::Import {
            namespace,
            force,
            patterns,
        } => {
            let Some(name) = state.namespace_target_key_at(namespace, current) else {
                state.mark_opaque_resolution();
                return;
            };
            for pattern in patterns {
                let Some(pattern) = namespace_pattern_operand_bytes(
                    pattern,
                    original_operands,
                    state.baseline.native_entry.is_some(),
                ) else {
                    Arc::make_mut(&mut state.unknown_lookup_namespaces).insert(name.clone());
                    break;
                };
                if !import_pattern(state, &name, pattern, *force) {
                    break;
                }
            }
        }
        NamespaceTransition::Forget {
            namespace,
            patterns,
        } => {
            let Some(name) = state.namespace_target_key_at(namespace, current) else {
                state.mark_opaque_resolution();
                return;
            };
            for pattern in patterns {
                let Some(pattern) = namespace_pattern_operand_bytes(
                    pattern,
                    original_operands,
                    state.baseline.native_entry.is_some(),
                ) else {
                    Arc::make_mut(&mut state.unknown_lookup_namespaces).insert(name.clone());
                    break;
                };
                forget_pattern(state, &name, pattern);
            }
        }
        NamespaceTransition::SetPath { namespace, path } => {
            apply_namespace_path(state, namespace, path, current, original_operands);
        }
        NamespaceTransition::Delete { namespace } => {
            Arc::make_mut(&mut state.object_instances).invalidate_class_delegates(false);
            Arc::make_mut(&mut state.object_instances).invalidate_dispatch(false);
            apply_namespace_deletion(state, namespace, current, offset, original_operands);
        }
        NamespaceTransition::SetUnknown { namespace, handler } => {
            if let Some(name) = state.namespace_target_key_at(namespace, current) {
                let default = is_default_namespace_unknown_handler(state, handler);
                if default {
                    Arc::make_mut(&mut state.namespace_unknown_handlers).remove(&name);
                } else {
                    Arc::make_mut(&mut state.namespace_unknown_handlers).insert(name);
                }
            } else {
                state.mark_opaque_resolution();
            }
        }
        NamespaceTransition::Ensemble { .. } => {
            apply_ensemble_transition(state, transition, current, offset, original_operands);
        }
    }
}

/// C Tcl's ensemble operations create at most one command and change no other
/// name's resolution; `configure` only rewrites the dispatch configuration of
/// an existing ensemble.
fn apply_ensemble_transition(
    state: &mut ModuleCommandBindings,
    transition: &NamespaceTransition,
    current: &SourceNamespaceKey,
    offset: u32,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    let NamespaceTransition::Ensemble {
        namespace,
        operation,
        created,
        dispatch,
        ..
    } = transition
    else {
        return;
    };
    if let Some(created) = created
        && let Some(key) =
            install_created_ensemble(state, created, current, offset, original_operands)
    {
        Arc::make_mut(&mut state.ensemble_owner_namespaces).insert(current.clone());
        state.freeze_ensemble_dispatch(&key, offset, current, dispatch.as_ref());
        return;
    }
    // Any other operation may rewrite an existing ensemble's dispatch.
    state.withdraw_ensemble_dispatches();
    if *operation == Some(tcl_registry::state_transition::EnsembleManagementOperation::Configure) {
        return;
    }
    if let Some(name) = state.namespace_target_key_at(namespace, current) {
        Arc::make_mut(&mut state.unknown_lookup_namespaces).insert(name);
    } else {
        state.mark_opaque_resolution();
    }
}

fn install_created_ensemble(
    state: &mut ModuleCommandBindings,
    created: &tcl_registry::EnsembleCommandName,
    current: &SourceNamespaceKey,
    declaration: u32,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) -> Option<SourceCommandKey> {
    if let Some(operands) = original_operands {
        return operands.apply_ensemble_creation(state, created, current, declaration);
    }
    let name = match created {
        tcl_registry::EnsembleCommandName::Named(name) => name.literal()?,
        tcl_registry::EnsembleCommandName::CurrentNamespace => match current {
            SourceNamespaceKey::Authored(namespace) if namespace != "::" => namespace.as_str(),
            _ => return None,
        },
    };
    let key =
        state.publication_key_at(current, name, namespace_slots::PublicationPurpose::Command)?;
    state.install_created_command(key.clone(), declaration);
    Some(key)
}

fn apply_namespace_deletion(
    state: &mut ModuleCommandBindings,
    namespace: &tcl_registry::NamespaceTransitionTarget,
    current: &SourceNamespaceKey,
    offset: u32,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    let name = match (namespace, original_operands) {
        (tcl_registry::NamespaceTransitionTarget::Named(subject), Some(operands)) => operands
            .namespace_input(subject)
            .and_then(|input| state.original_namespace_key_for_input(current, input)),
        _ => state.namespace_target_key_at(namespace, current),
    };
    let Some(name) = name else {
        state.mark_opaque_resolution();
        return;
    };
    // A retained retired activation must not delete a replacement object at
    // the same component path. The deletion target is its actual incarnation.
    if !matches!(name, SourceNamespaceKey::Authored(_)) && !state.namespaces.contains(&name) {
        return;
    }
    // Deleting a namespace deletes every ensemble it created, wherever its
    // `-command` placed it; this lattice does not keep those slots.
    if state
        .ensemble_owner_namespaces
        .iter()
        .any(|owner| owner == &name || owner.is_descendant_of(&name))
    {
        state.mark_opaque_resolution();
        return;
    }
    crate::variable_bindings::delete_namespace_cells_in_identity(
        Arc::make_mut(&mut state.source_variables),
        &name,
        offset,
    );
    let slots = state
        .all_command_keys()
        .into_iter()
        .filter(|slot| {
            let holder = slot.holder();
            holder.as_ref() == &name || holder.is_descendant_of(&name)
        })
        .collect::<Vec<_>>();
    for slot in slots {
        state.remove(slot);
    }
    Arc::make_mut(&mut state.namespaces).retain(|ns| ns != &name && !ns.is_descendant_of(&name));
    Arc::make_mut(&mut state.namespace_exports)
        .retain(|ns, _| ns != &name && !ns.is_descendant_of(&name));
    Arc::make_mut(&mut state.namespace_paths)
        .retain(|ns, _| ns != &name && !ns.is_descendant_of(&name));
}

/// Exact original values take precedence over logical transition advice.
/// The legacy authored adapter supports explicit ASCII metadata only; a
/// supplied native entry never borrows those bytes for an absent original.
fn namespace_pattern_operand_bytes<'a>(
    subject: &'a TransitionSubject,
    original_operands: Option<&'a original_command_table::OriginalCommandOperands>,
    native_entry: bool,
) -> Option<&'a [u8]> {
    match original_operands {
        Some(operands) => operands
            .namespace_input(subject)
            .map(crate::signature_scan::scope::SignatureSourceNameInput::bytes),
        None => subject.native_bytes().or_else(|| {
            (!native_entry).then_some(())?;
            let metadata = subject.literal()?;
            (metadata.is_ascii() && !metadata.as_bytes().contains(&0))
                .then_some(metadata.as_bytes())
        }),
    }
}

fn apply_namespace_exports(
    state: &mut ModuleCommandBindings,
    namespace: &tcl_registry::NamespaceTransitionTarget,
    patterns: &[TransitionSubject],
    current: &SourceNamespaceKey,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    let Some(namespace) = state.namespace_target_key_at(namespace, current) else {
        state.mark_opaque_resolution();
        return;
    };
    let inputs = patterns
        .iter()
        .map(|pattern| {
            namespace_pattern_operand_bytes(
                pattern,
                original_operands,
                state.baseline.native_entry.is_some(),
            )
        })
        .collect::<Option<Vec<_>>>();
    if inputs
        .as_ref()
        .and_then(|inputs| original_namespace_binding::apply_exports(state, &namespace, inputs))
        .is_none()
    {
        Arc::make_mut(&mut state.unknown_export_namespaces).insert(namespace);
    }
}

fn import_pattern(
    state: &mut ModuleCommandBindings,
    destination: &SourceNamespaceKey,
    pattern: &[u8],
    force: Option<bool>,
) -> bool {
    original_namespace_binding::apply_import(state, destination, pattern, force).is_some()
}

pub(in super::super) fn imported_binding_links(
    state: &mut ModuleCommandBindings,
    slot: &SourceCommandKey,
    implementations: BTreeSet<MayBinding>,
    name_binding: bool,
) -> (BTreeSet<MayBinding>, BTreeSet<MayBinding>) {
    let mut links = BTreeSet::new();
    let mut source_bindings = BTreeSet::new();
    let spelling = state.callable_spelling_for_key(slot);
    for mut implementation in implementations {
        if spelling.is_none()
            && (name_binding
                || matches!(&implementation, MayBinding::Target(target) if target.token.is_none()))
        {
            links.insert(MayBinding::Unknown);
            source_bindings.insert(implementation);
            continue;
        }
        if name_binding && !matches!(implementation, MayBinding::Missing | MayBinding::Unknown) {
            let token = CommandToken {
                runtime: None,
                origin: spelling.clone().unwrap_or_default(),
                declaration: u32::MAX,
                allocation: None,
            };
            Arc::make_mut(&mut state.objects).insert(
                token.clone(),
                BTreeSet::from([MayBinding::Target(ResolvedCommandTarget {
                    command: spelling.clone().unwrap_or_default(),
                    prepended: Vec::new(),
                    registry_backed: false,
                    terminal: false,
                    target_lookup: tcl_registry::AliasTargetLookup::Global,
                    token: None,
                    kind: BindingKind::Alias,
                    implementation_generation: 0,
                    implementation_allocation: None,
                })]),
            );
            links.insert(MayBinding::Imported(token.into()));
            source_bindings.insert(implementation);
            continue;
        }
        match &mut implementation {
            MayBinding::Imported(token) => {
                links.insert(MayBinding::Imported(ImportedCommandBinding {
                    origin: token.origin.clone(),
                    compiler: None,
                }));
            }
            MayBinding::Target(target) => match state.baseline.import_binding {
                Some(tcl_dialect::NamespaceImportBinding::CommandToken) => {
                    let token = target
                        .token
                        .get_or_insert_with(|| CommandToken {
                            runtime: None,
                            origin: spelling.clone().unwrap_or_default(),
                            declaration: 0,
                            allocation: None,
                        })
                        .clone();
                    Arc::make_mut(&mut state.objects)
                        .insert(token.clone(), BTreeSet::from([implementation.clone()]));
                    links.insert(MayBinding::Imported(token.into()));
                }
                Some(tcl_dialect::NamespaceImportBinding::SourceName) => {
                    let token = CommandToken {
                        runtime: None,
                        origin: spelling.clone().unwrap_or_default(),
                        declaration: u32::MAX,
                        allocation: None,
                    };
                    Arc::make_mut(&mut state.objects).insert(
                        token.clone(),
                        BTreeSet::from([MayBinding::Target(ResolvedCommandTarget {
                            command: spelling.clone().unwrap_or_default(),
                            prepended: Vec::new(),
                            registry_backed: false,
                            terminal: false,
                            target_lookup: tcl_registry::AliasTargetLookup::Global,
                            token: None,
                            kind: BindingKind::Alias,
                            implementation_generation: 0,
                            implementation_allocation: None,
                        })]),
                    );
                    links.insert(MayBinding::Imported(token.into()));
                }
                None => {
                    links.insert(MayBinding::Unknown);
                }
            },
            MayBinding::Unknown => {
                links.insert(MayBinding::Unknown);
            }
            MayBinding::Missing => {
                links.insert(MayBinding::Missing);
            }
        }
        source_bindings.insert(implementation);
    }
    (links, source_bindings)
}

fn forget_pattern(
    state: &mut ModuleCommandBindings,
    destination: &SourceNamespaceKey,
    pattern: &[u8],
) {
    if original_namespace_binding::apply_forget(state, destination, pattern).is_none() {
        Arc::make_mut(&mut state.unknown_lookup_namespaces).insert(destination.clone());
    }
}

/// Project literal import tails under the retained current namespace key.
/// Unknown patterns or targets widen lookup advice without selecting a command.
pub(in super::super) fn collect_namespace_resolution(
    transition: &NamespaceTransition,
    site: Option<&str>,
    rebound: &mut std::collections::HashSet<String>,
    resolution: &mut bool,
    opaque: &mut std::collections::HashSet<String>,
) -> bool {
    if let NamespaceTransition::Ensemble {
        namespace,
        created: Some(created),
        ..
    } = transition
    {
        *resolution = true;
        let Some(target) = transition_namespace(namespace, site) else {
            return false;
        };
        let name = match created {
            tcl_registry::EnsembleCommandName::Named(name) => {
                let Some(name) = literal_subject(name) else {
                    return false;
                };
                tcl_syntax::naming::qualify(&target, name)
            }
            tcl_registry::EnsembleCommandName::CurrentNamespace => target,
        };
        rebound.insert(name);
        return true;
    }
    let (namespace, patterns) = match transition {
        NamespaceTransition::Import {
            namespace,
            patterns,
            ..
        }
        | NamespaceTransition::Forget {
            namespace,
            patterns,
        } => (namespace, Some(patterns)),
        NamespaceTransition::SetPath { namespace, .. }
        | NamespaceTransition::SetUnknown { namespace, .. }
        | NamespaceTransition::Ensemble { namespace, .. }
        | NamespaceTransition::Delete { namespace } => (namespace, None),
        // Creating a namespace, or changing its export list, resolves nothing
        // differently for a caller in it.
        NamespaceTransition::Ensure { .. } | NamespaceTransition::Export { .. } => return true,
    };
    *resolution = true;
    let Some(target) = transition_namespace(namespace, site) else {
        return false;
    };
    let Some(patterns) = patterns else {
        opaque.insert(target);
        return true;
    };
    for pattern in patterns {
        match literal_subject(pattern) {
            Some(text) if !text.contains(['*', '?', '[']) => {
                let tail =
                    core::str::from_utf8(tcl_syntax::naming::written_command_tail(text.as_bytes()))
                        .expect("tail of a Unicode pattern");
                rebound.insert(tcl_syntax::naming::qualify(&target, tail));
            }
            _ => {
                opaque.insert(target.clone());
            }
        }
    }
    true
}

/// The namespace a [`NamespaceTransitionTarget`] denotes: the invocation's own
/// namespace for `Current`, the literal text for a `Named` operand, and `None`
/// when the operand is not literal (the caller then takes the lattice top).
fn transition_namespace(
    target: &tcl_registry::NamespaceTransitionTarget,
    current: Option<&str>,
) -> Option<String> {
    match target {
        tcl_registry::NamespaceTransitionTarget::Current => current.map(str::to_owned),
        tcl_registry::NamespaceTransitionTarget::Named(subject) => {
            literal_subject(subject).map(nqn)
        }
    }
}

fn literal_subject(subject: &TransitionSubject) -> Option<&str> {
    subject.literal()
}
