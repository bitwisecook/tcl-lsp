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

//! The cell an alias operand names: original targets, namespace slots and selected frames.

use super::OriginalVariableInvocation;
use crate::place;
use crate::place::{CellGeneration, CellIdentity, CellOwner, Place};
use crate::var_resolve::{ResolveContext, resolve_literal_place};
use tcl_registry::{CallerFrameSelection, CommandRegistry, FrameLevel, VariableAliasTarget};

pub(super) fn original_alias_target(
    state: &ResolveContext,
    target: &VariableAliasTarget,
    operands: &OriginalVariableInvocation,
    registry: &CommandRegistry,
    destination: bool,
) -> Place {
    let variable = match target {
        VariableAliasTarget::Global { variable }
        | VariableAliasTarget::CurrentNamespace { variable }
        | VariableAliasTarget::CallerSelectedFrame { variable, .. }
        | VariableAliasTarget::Namespace { variable, .. } => variable,
    };
    let Some(input) = operands.subject_input(variable, state) else {
        return place::unknown_top();
    };
    let mut result = if let VariableAliasTarget::CallerSelectedFrame { frame, .. } = target {
        let level = match frame {
            CallerFrameSelection::DefaultCaller => Some(FrameLevel::DEFAULT),
            CallerFrameSelection::Explicit(subject) => {
                operands.subject_input(subject, state).and_then(|input| {
                    FrameLevel::parse_native_bytes(input.bytes(), state.invocation_dialect?)
                        .flatten()
                })
            }
        };
        let Some(level) = level else {
            return place::unknown_top();
        };
        if level.is_global_frame() {
            let Some(root) = state
                .root_namespace_identity()
                .filter(|root| state.namespace_identities.contains(root))
            else {
                return place::unknown_top();
            };
            // Absolute zero selects the retained interpreter root table.
            // This is alias target geometry, not an invented caller frame.
            crate::var_resolve::resolve_original_namespace_variable_bytes(
                input.bytes(),
                &root,
                state,
                registry,
                destination,
            )
        } else {
            let Some(selected) = state.selected_frame_context(level) else {
                return place::unknown_top();
            };
            if destination {
                crate::var_resolve::resolve_original_alias_destination_bytes(
                    input.bytes(),
                    &selected,
                    registry,
                )
            } else {
                crate::var_resolve::resolve_evaluated_variable_input(
                    tcl_syntax::naming::NativeVariableInputForm::Combined(input.bytes()),
                    &selected,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Read,
                )
            }
        }
    } else {
        let namespace = match target {
            VariableAliasTarget::Global { .. } => state.root_namespace_identity(),
            VariableAliasTarget::CurrentNamespace { .. } => state
                .namespace_identity
                .clone()
                .filter(|_| state.namespace_known),
            VariableAliasTarget::Namespace { namespace, .. } => operands
                .subject_input(namespace, state)
                .and_then(|input| state.namespace_identity_for_original_input(input)),
            VariableAliasTarget::CallerSelectedFrame { .. } => unreachable!(),
        };
        let Some(namespace) = namespace else {
            return place::unknown_top();
        };
        crate::var_resolve::resolve_original_namespace_variable_bytes(
            input.bytes(),
            &namespace,
            state,
            registry,
            destination,
        )
    };
    if destination {
        result.observed = false;
        if let Some(cell) = &mut result.cell {
            cell.generation = CellGeneration::Incoming;
        }
    }
    result
}

fn selected_alias_frame(
    state: &ResolveContext,
    selection: &CallerFrameSelection,
    registry: &CommandRegistry,
) -> Option<FrameLevel> {
    match selection {
        CallerFrameSelection::DefaultCaller => Some(FrameLevel::DEFAULT),
        CallerFrameSelection::Explicit(word) => word.literal().and_then(|word| {
            state.invocation_dialect.map_or_else(
                || FrameLevel::parse_in(word, registry),
                |dialect| FrameLevel::parse_for_dialect(word, dialect),
            )
        }),
    }
}

pub(super) fn alias_name_target(
    state: &ResolveContext,
    target: &VariableAliasTarget,
    registry: &CommandRegistry,
) -> Place {
    let mut target = match target {
        VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
            let Some(variable) = variable.literal() else {
                return place::unknown_top();
            };
            let Some(level) = selected_alias_frame(state, frame, registry) else {
                return place::unknown_top();
            };
            let Some(selected) = state.selected_frame_context(level) else {
                return place::unknown_top();
            };
            crate::var_resolve::resolve_alias_destination_slot(variable, &selected, registry)
        }
        VariableAliasTarget::Global { variable } => {
            namespace_name_slot(state, "::", variable.literal(), registry)
        }
        VariableAliasTarget::CurrentNamespace { variable } => {
            namespace_name_slot(state, &state.namespace, variable.literal(), registry)
        }
        VariableAliasTarget::Namespace {
            namespace,
            variable,
        } => {
            let Some(namespace) = namespace.literal() else {
                return place::unknown_top();
            };
            if state.namespace_identity.is_some() {
                let Some(identity) = state.namespace_identity_for_written(namespace) else {
                    return place::unknown_top();
                };
                namespace_target_in_identity(state, &identity, variable.literal(), registry, true)
            } else {
                let namespace = tcl_syntax::naming::qualify_namespace(&state.namespace, namespace);
                namespace_name_slot(state, &namespace, variable.literal(), registry)
            }
        }
    };
    target.observed = false;
    if let Some(cell) = &mut target.cell {
        cell.generation = CellGeneration::Incoming;
    }
    target
}

fn namespace_name_slot(
    state: &ResolveContext,
    namespace: &str,
    variable: Option<&str>,
    registry: &CommandRegistry,
) -> Place {
    let Some(variable) = variable else {
        return place::unknown_top();
    };
    if let Some(identity) = if namespace == "::" {
        state.root_namespace_identity()
    } else {
        state
            .namespace_identity
            .clone()
            .filter(|_| namespace == state.namespace)
    } {
        return namespace_target_in_identity(state, &identity, Some(variable), registry, true);
    }
    let qualified = tcl_syntax::naming::qualify(namespace, variable);
    crate::var_resolve::resolve_alias_destination_slot(&qualified, state, registry)
}

pub(crate) fn alias_target(
    state: &ResolveContext,
    target: &VariableAliasTarget,
    registry: &CommandRegistry,
) -> Place {
    match target {
        VariableAliasTarget::Global { variable } => {
            namespace_target(state, "::", variable.literal(), registry)
        }
        VariableAliasTarget::CurrentNamespace { variable } => {
            if !state.namespace_known {
                return place::unknown_top();
            }
            namespace_target(state, &state.namespace, variable.literal(), registry)
        }
        VariableAliasTarget::Namespace {
            namespace,
            variable,
        } => {
            let Some(namespace) = namespace.literal() else {
                return place::unknown_top();
            };
            if !state.namespace_known && !tcl_syntax::naming::is_absolute(namespace) {
                return place::unknown_top();
            }
            if state.namespace_identity.is_some() {
                let Some(identity) = state.namespace_identity_for_written(namespace) else {
                    return place::unknown_top();
                };
                namespace_target_in_identity(state, &identity, variable.literal(), registry, false)
            } else {
                let namespace = tcl_syntax::naming::qualify_namespace(&state.namespace, namespace);
                namespace_target(state, &namespace, variable.literal(), registry)
            }
        }
        VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
            selected_frame_target(state, frame, variable.literal(), registry)
        }
    }
}

fn namespace_target(
    state: &ResolveContext,
    namespace: &str,
    variable: Option<&str>,
    registry: &CommandRegistry,
) -> Place {
    let Some(variable) = variable else {
        return place::unknown_top();
    };
    if let Some(identity) = if namespace == "::" {
        state.root_namespace_identity()
    } else {
        state
            .namespace_identity
            .clone()
            .filter(|_| namespace == state.namespace)
    } {
        return namespace_target_in_identity(state, &identity, Some(variable), registry, false);
    }
    let qualified = tcl_syntax::naming::qualify(namespace, variable);
    resolve_literal_place(&qualified, state, false, registry)
}

fn namespace_target_in_identity(
    state: &ResolveContext,
    identity: &crate::command_binding::SourceNamespaceKey,
    variable: Option<&str>,
    registry: &CommandRegistry,
    destination: bool,
) -> Place {
    let Some(variable) = variable else {
        return place::unknown_top();
    };
    let mut selected = state.clone();
    selected.retain_namespace_world(
        identity.clone(),
        state.namespace_identities.iter().cloned(),
        state.namespace_name_protocol,
    );
    selected.frame_kind = crate::var_resolve::VariableFrameKind::Namespace;
    selected.ns_vars.insert(variable.to_owned());
    if destination {
        crate::var_resolve::resolve_alias_destination_slot(variable, &selected, registry)
    } else {
        resolve_literal_place(variable, &selected, false, registry)
    }
}

fn selected_frame_target(
    state: &ResolveContext,
    selection: &CallerFrameSelection,
    variable: Option<&str>,
    registry: &CommandRegistry,
) -> Place {
    let Some(variable) = variable else {
        return place::unknown_top();
    };
    let Some(level) = selected_alias_frame(state, selection, registry) else {
        return place::unknown_top();
    };
    if level.is_current_frame() {
        return resolve_literal_place(variable, state, false, registry);
    }
    if let Some(selected) = state.selected_frame_context(level) {
        return resolve_literal_place(variable, &selected, false, registry);
    }

    if level.is_global_frame() || tcl_syntax::naming::is_absolute(variable) {
        return namespace_target(state, "::", Some(variable), registry);
    }
    if level == FrameLevel::Dynamic {
        return place::unknown_top();
    }
    let (name, index) = crate::naming::split_array_name(variable);
    let mut target = place::upvar_alias(name, format!("{level:?}:{variable}"), false);
    target.cell = Some(CellIdentity {
        owner: CellOwner::SelectedFrame(level),
        name: name.into(),
        generation: CellGeneration::Incoming,
        interpreter: state.interpreter.clone(),
        storage_domain: None,
        execution: state.execution,
    });
    if let Some(index) = index {
        target.index = Some(place::Index::literal(index));
    }
    target
}
