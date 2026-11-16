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

//! Namespace deletion: the places it destroys and the retirement of cells, observers and links in the original namespace tree.

use crate::ir::Statement;
use crate::place;
use crate::place::{CellGeneration, Place};
use crate::var_resolve::{ResolveContext, cell_key};
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{
    CommandRegistry, NamespaceTransition, NamespaceTransitionTarget, StateTransition,
};

/// Cell-store destruction declared by a namespace-deletion transition.
#[must_use]
pub fn namespace_destruction_places(
    statement: &Statement,
    state: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    namespace_destruction_places_with_metadata_context(
        statement,
        state,
        registry,
        Some(
            registry
                .profile()
                .map(SemanticContext::for_profile)
                .map(crate::registry_invocation::InvocationMetadataContext::from)
                .into(),
        ),
    )
}

/// Namespace destruction under the caller's complete retained source input.
#[must_use]
pub(crate) fn namespace_destruction_places_with_metadata_context(
    statement: &Statement,
    state: &ResolveContext,
    registry: &CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataSelection<'_>>,
) -> Vec<Place> {
    let Some(metadata) = metadata else {
        return Vec::new();
    };
    let Some(invocation) =
        crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
            registry,
            metadata.context(),
            statement,
        )
    else {
        return Vec::new();
    };
    namespace_destruction_from_facts(&invocation.facts, state)
}

pub(super) fn namespace_destruction_from_facts(
    facts: &tcl_registry::InvocationFacts,
    state: &ResolveContext,
) -> Vec<Place> {
    let Some(transitions) = facts.state_transitions.declared() else {
        return Vec::new();
    };
    transitions
        .facts()
        .iter()
        .filter_map(|fact| {
            let StateTransition::Namespace(NamespaceTransition::Delete { namespace }) =
                &fact.transition
            else {
                return None;
            };
            Some(match namespace {
                NamespaceTransitionTarget::Current if state.namespace_known => {
                    place::unknown_namespace(state.namespace.clone())
                }
                NamespaceTransitionTarget::Named(subject) => subject
                    .literal()
                    .filter(|namespace| {
                        state.namespace_known || tcl_syntax::naming::is_absolute(namespace)
                    })
                    .map_or_else(place::unknown_top, |namespace| {
                        place::unknown_namespace(tcl_syntax::naming::qualify(
                            &state.namespace,
                            namespace,
                        ))
                    }),
                NamespaceTransitionTarget::Current => place::unknown_top(),
            })
        })
        .collect()
}

pub(super) fn apply_namespace(
    state: &mut ResolveContext,
    transition: &NamespaceTransition,
    source_offset: u32,
) {
    let (target, deleted) = match transition {
        NamespaceTransition::Ensure { namespace } => (namespace, false),
        NamespaceTransition::Delete { namespace } => (namespace, true),
        _ => return,
    };
    let identity = match target {
        NamespaceTransitionTarget::Current => state.namespace_identity.clone().or_else(|| {
            Some(crate::command_binding::SourceNamespaceKey::authored(
                &state.namespace,
            ))
        }),
        NamespaceTransitionTarget::Named(subject) => subject.literal().and_then(|name| {
            state.namespace_identity_for_written(name).or_else(|| {
                state.namespace_identity.is_none().then(|| {
                    crate::command_binding::SourceNamespaceKey::authored(
                        tcl_syntax::naming::qualify_namespace(&state.namespace, name),
                    )
                })
            })
        }),
    };
    let Some(identity) = identity else {
        state.widen();
        return;
    };
    if deleted {
        delete_namespace_cells_in_identity(state, &identity, source_offset);
    } else {
        if let Some(display) = identity.display() {
            state.known_namespaces.insert(display);
        }
        state.namespace_identities.insert(identity);
    }
}

/// Retire cells in the exact original namespace tree before unset callbacks.
pub(crate) fn delete_namespace_cells_in_identity(
    state: &mut ResolveContext,
    namespace: &crate::command_binding::SourceNamespaceKey,
    source_offset: u32,
) {
    use crate::command_binding::SourceNamespaceKey;
    let mut original: crate::var_resolve::VariableNamespaceSet = state
        .namespace_identities
        .iter()
        .filter(|identity| crate::var_resolve::namespace_contains(namespace, identity))
        .cloned()
        .collect();
    original.insert(namespace.clone());
    state
        .namespace_addressable_identities
        .retain(|identity| !original.contains(identity));
    // Native active namespaces detach now, but their original contents remain until pop.
    let mut held = Vec::new();
    let mut frame = Some(&*state);
    while let Some(current) = frame {
        if matches!(
            current.frame_kind,
            crate::var_resolve::VariableFrameKind::Namespace
                | crate::var_resolve::VariableFrameKind::Local
        ) && let Some(key) = &current.namespace_identity
            && !matches!(key, SourceNamespaceKey::Authored(_))
            && original.contains(key)
        {
            held.push(key.clone());
        }
        frame = current.caller.as_deref();
    }
    for owner in held {
        let tree: crate::var_resolve::VariableNamespaceSet = original
            .iter()
            .filter(|key| crate::var_resolve::namespace_contains(&owner, key))
            .cloned()
            .collect();
        original.retain(|key| !tree.contains(key));
        state.pending_namespace_retirements.entry(owner).or_insert(
            crate::var_resolve::PendingNamespaceRetirement {
                namespaces: tree,
                source: source_offset,
                conditional: false,
            },
        );
    }
    retire_namespace_cell_incarnations(state, &original, source_offset, false);
}

/// Pop-time retirement of the captured original tree, never a later equal-path namespace.
pub(crate) fn complete_pending_namespace_retirements(state: &mut ResolveContext) {
    let mut active = std::collections::HashSet::new();
    let mut frame = Some(&*state);
    while let Some(current) = frame {
        if matches!(
            current.frame_kind,
            crate::var_resolve::VariableFrameKind::Namespace
                | crate::var_resolve::VariableFrameKind::Local
        ) && let Some(key) = &current.namespace_identity
        {
            active.insert(key.clone());
        }
        frame = current.caller.as_deref();
    }
    let completed: Vec<_> = state
        .pending_namespace_retirements
        .keys()
        .filter(|key| !active.contains(*key))
        .cloned()
        .collect();
    for key in completed {
        let pending = state
            .pending_namespace_retirements
            .remove(&key)
            .expect("selected pending owner");
        // A child activation can outlive its detached parent; defer that child's tree.
        let mut remaining = pending.namespaces;
        for child in &active {
            if remaining.contains(child) {
                let tree: crate::var_resolve::VariableNamespaceSet = remaining
                    .iter()
                    .filter(|key| crate::var_resolve::namespace_contains(child, key))
                    .cloned()
                    .collect();
                remaining.retain(|key| !tree.contains(key));
                state
                    .pending_namespace_retirements
                    .entry(child.clone())
                    .or_insert(crate::var_resolve::PendingNamespaceRetirement {
                        namespaces: tree,
                        source: pending.source,
                        conditional: pending.conditional,
                    });
            }
        }
        retire_namespace_cell_incarnations(state, &remaining, pending.source, pending.conditional);
    }
}

fn retire_namespace_cell_incarnations(
    state: &mut ResolveContext,
    namespaces: &crate::var_resolve::VariableNamespaceSet,
    source_offset: u32,
    conditional: bool,
) {
    let selected = |key: &crate::var_resolve::VariableCellKey| {
        namespaces
            .iter()
            .any(|namespace| key.is_owned_by_namespace(namespace))
    };
    state
        .captured_cells
        .delete_namespace_incarnations(namespaces);
    for namespace in namespaces {
        if !conditional
            && let crate::command_binding::SourceNamespaceKey::Authored(name) = namespace
        {
            state.captured_cells.delete_namespace(name);
        }
    }
    state
        .outward_namespace_destructions
        .extend(namespaces.iter().cloned());
    state.retain_literal_values(|key| !selected(key));
    state.value_representations.retain(|key, _| !selected(key));
    state.contents_kinds.retain(|key, _| !selected(key));
    for (key, presence) in &mut state.contents_presence {
        if selected(key) {
            *presence = if conditional {
                crate::var_resolve::ContentsPresence::Unknown
            } else {
                crate::var_resolve::ContentsPresence::Undefined
            };
        }
    }
    for (key, origin) in &mut state.contents_origins {
        if selected(key) {
            *origin = if conditional {
                crate::var_resolve::ContentsOrigin::Unknown
            } else {
                crate::var_resolve::ContentsOrigin::WrittenAt(source_offset)
            };
        }
    }
    for key in state
        .namespace_cells
        .present
        .iter()
        .chain(&state.namespace_cells.possible)
        .filter(|key| selected(key))
        .cloned()
        .collect::<Vec<_>>()
    {
        state.generations.insert(
            key,
            if conditional {
                CellGeneration::Unknown
            } else {
                CellGeneration::After(source_offset)
            },
        );
    }
    state.namespace_cells.present.retain(|key| !selected(key));
    state.namespace_cells.possible.retain(|key| !selected(key));
    state
        .namespace_cells
        .closed_namespaces
        .retain(|key| !namespaces.contains(key));
    state.closed_array_roots.retain(|key| !selected(key));
    state
        .namespace_identities
        .retain(|identity| !namespaces.contains(identity));
    state
        .namespace_addressable_identities
        .retain(|identity| !namespaces.contains(identity));
    state.namespace_objects.retain(|identity, _| {
        state.namespace_identity.as_ref() == Some(identity) || !namespaces.contains(identity)
    });
    for namespace in namespaces {
        if let crate::command_binding::SourceNamespaceKey::Authored(namespace) = namespace {
            state.known_namespaces.retain(|known| {
                known != namespace && !known.starts_with(&format!("{namespace}::"))
            });
        }
    }
    retire_namespace_observers_and_links(state, selected, source_offset);
}

fn retire_namespace_observers_and_links(
    state: &mut ResolveContext,
    selected: impl Fn(&crate::var_resolve::VariableCellKey) -> bool,
    source_offset: u32,
) {
    state.traced.retain(|key| !selected(key));
    state.trace_registrations.retain(|key, _| !selected(key));
    state
        .trace_registration_receivers
        .retain(|_, receiver| !selected(&cell_key(receiver)));
    state.untracked_traces.retain(|key| !selected(key));
    state
        .namespace_alias_bindings
        .retain(|slot, _| !selected(slot));
    state
        .namespace_name_alias_bindings
        .retain(|slot, _| !selected(slot));
    let name_following = state
        .invocation_dialect
        .and_then(|dialect| dialect.variable_link_binding)
        == Some(tcl_dialect::VariableLinkBinding::SelectedFrameName);
    for alias in state.namespace_alias_bindings.values_mut() {
        if selected(&cell_key(alias)) {
            if name_following {
                if let Some(cell) = &mut alias.cell {
                    cell.generation = CellGeneration::After(source_offset);
                }
            } else {
                *alias = place::unknown_top();
            }
        }
    }
    for (name, alias) in &mut state.alias_bindings {
        if selected(&cell_key(alias)) {
            if name_following {
                if let Some(cell) = &mut alias.cell {
                    cell.generation = CellGeneration::After(source_offset);
                }
            } else {
                state.unknown_bindings.insert(name.clone());
            }
        }
    }
}
