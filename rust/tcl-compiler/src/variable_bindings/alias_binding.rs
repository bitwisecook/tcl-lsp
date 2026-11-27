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

//! Binding one cell to another: caller-reference formals, alias validity, closed registrations and installing the link.

use super::alias_targets::{alias_name_target, alias_target, original_alias_target};
use super::{AliasBindingValues, OriginalVariableInvocation};
use crate::place;
use crate::place::{CellGeneration, CellOwner, Place, PlaceKind};
use crate::var_resolve::{ResolveContext, cell_key, resolve_literal_access};
use tcl_registry::{
    CallerFrameSelection, CommandRegistry, StateTransition, VariableAliasTarget,
    VariableCellAliasTransition,
};

/// Result of validating and installing a reference formal in an actual call activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerReferenceBinding {
    /// Caller contents are defined and the selected link was installed.
    Bound,
    /// The unobserved actual target definitely has no contents.
    Missing,
    /// The name, caller, contents or observer is not proved.
    Unknown,
}

/// Bind a caller-reference formal through the shared selected-frame and alias owner.
/// Missing literal knowledge is never treated as missing caller contents.
pub fn bind_caller_reference(
    state: &mut ResolveContext,
    local: &str,
    actual: Option<&str>,
    registry: &CommandRegistry,
) -> CallerReferenceBinding {
    let (Some(actual), Some(parent)) = (actual, state.caller.as_deref()) else {
        return CallerReferenceBinding::Unknown;
    };
    let target = resolve_literal_access(
        actual,
        parent,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    match parent.contents_presence(&target) {
        crate::var_resolve::ContentsPresence::Undefined => return CallerReferenceBinding::Missing,
        crate::var_resolve::ContentsPresence::Unknown
        | crate::var_resolve::ContentsPresence::DefinedOrUndefined => {
            return CallerReferenceBinding::Unknown;
        }
        crate::var_resolve::ContentsPresence::Defined => {}
    }
    let local = tcl_registry::TransitionSubject::Literal(local.to_owned());
    let target = VariableAliasTarget::CallerSelectedFrame {
        frame: CallerFrameSelection::Explicit(tcl_registry::TransitionSubject::Literal(
            "1".to_owned(),
        )),
        variable: tcl_registry::TransitionSubject::Literal(actual.to_owned()),
    };
    bind_authored_alias_values(
        state,
        AliasBindingValues {
            destination: tcl_registry::VariableAliasDestination::ProcedureLocal,
            local: &local,
            target: &target,
        },
        registry,
    );
    CallerReferenceBinding::Bound
}

/// Whether a proved alias operation can install its destination on normal completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasBindingValidity {
    /// The destination and target owners permit the link.
    Valid,
    /// The destination already has direct contents and cannot become a link.
    DefinedDestination,
    /// C Tcl forbids a namespace slot retaining a procedure-local variable.
    NamespaceToLocal,
    /// An unresolved destination, target, or interpreter policy prevents proof.
    Unknown,
}

/// Whether an alias destination still denotes its own direct slot.
/// This does not infer contents, trace absence, or successful alias registration.
pub(crate) fn alias_destination_is_direct(
    state: &ResolveContext,
    local: &str,
    slot: &Place,
) -> bool {
    let raw = state
        .raw_bindings
        .bindings
        .get(&cell_key(slot))
        .and_then(|key| state.raw_bindings.slots.get(key));
    raw.map_or_else(
        || {
            !state.alias_bindings.contains_key(local)
                && !state.name_alias_bindings.contains_key(local)
                && !state.namespace_alias_bindings.contains_key(&cell_key(slot))
                && !state
                    .namespace_name_alias_bindings
                    .contains_key(&cell_key(slot))
        },
        |slot| {
            matches!(&slot.contents,
                crate::raw_binding::RawBindingContents::Direct(target)
                    if !matches!(target.cell.as_ref().map(|cell| &cell.owner),
                        Some(CellOwner::AllocatedInstance(_))))
        },
    )
}

/// A native alias-pair registration is normal-only when its actual frame,
/// every destination and target, and the complete literal layout are proved.
/// This does not license an opcode or infer success from a catalogue role.
pub(crate) fn alias_registration_is_closed(
    state: &ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    registry: &CommandRegistry,
) -> bool {
    use tcl_registry::frame_effect::{FrameArgLayout, FrameArgumentResolution};
    let Some(effect) = facts
        .frame_effect
        .filter(|effect| effect.layout == FrameArgLayout::AliasPairs)
    else {
        return false;
    };
    let FrameArgumentResolution::Valid {
        level_word_len,
        level,
    } = effect.resolve_arguments(arguments)
    else {
        return false;
    };
    let Some(count) = arguments.exact_argv_len() else {
        return false;
    };
    if state.binding_identity != crate::var_resolve::BindingIdentity::Bound
        || !arguments.are_all_literals()
        || state.dynamic_bindings
        || state.selected_frame_context(level).is_none()
        || facts
            .successful_handler_effects(arguments, state.alias_frame())
            .is_none()
    {
        return false;
    }
    let Some(transitions) = facts.state_transitions.declared() else {
        return false;
    };
    let aliases = transitions
        .facts()
        .iter()
        .filter_map(|fact| {
            if let StateTransition::VariableCellAlias(alias) = &fact.transition {
                Some(alias)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if aliases.len() != (count - level_word_len) / 2 {
        return false;
    }
    let mut candidate = state.clone();
    for alias in aliases {
        if !alias_registration_address_is_closed(&candidate, alias, registry) {
            return false;
        }
        bind_alias(&mut candidate, alias, registry);
    }
    true
}

fn alias_registration_address_is_closed(
    state: &ResolveContext,
    alias: &VariableCellAliasTransition,
    registry: &CommandRegistry,
) -> bool {
    let Some(local) = alias.local.literal() else {
        return false;
    };
    if alias
        .destination
        .is_active_in_frame_with_policy(state.alias_frame(), state.invocation_dialect)
        != Some(true)
        || alias_binding_validity(state, alias, registry) != AliasBindingValidity::Valid
    {
        return false;
    }
    let mut slot = crate::var_resolve::resolve_alias_destination_slot(local, state, registry);
    let mut target = alias_target(state, &alias.target, registry);
    slot.observed = false;
    target.observed = false;
    let Some(cell) = target.cell.as_ref() else {
        return false;
    };
    if target.dynamic
        || target.kind == PlaceKind::Unknown
        || cell.generation == CellGeneration::Unknown
        || matches!(cell.owner, CellOwner::SelectedFrame(_))
        || (target.is_global() && !state.known_namespaces.contains(&target.ns))
        || (target.index.is_some() && state.store_would_error(&target))
        || (slot.cell == target.cell)
    {
        return false;
    }
    let established = state.alias_bindings.contains_key(local)
        || state
            .namespace_alias_bindings
            .contains_key(&cell_key(&slot));
    if !established
        && state.contents_presence(&slot) != crate::var_resolve::ContentsPresence::Undefined
    {
        return false;
    }
    if target.index.is_some() && state.root_contents_kind(&target).is_none() {
        let root = target.base();
        if state.contents_presence(&root) != crate::var_resolve::ContentsPresence::Undefined {
            return false;
        }
    }
    !slot.is_global() || state.known_namespaces.contains(&slot.ns)
}

/// Validate owner compatibility without manufacturing or mutating either cell.
#[must_use]
pub fn alias_binding_validity(
    state: &ResolveContext,
    alias: &VariableCellAliasTransition,
    registry: &CommandRegistry,
) -> AliasBindingValidity {
    alias_value_binding_validity(state, alias.into(), registry)
}

pub(super) fn alias_value_binding_validity(
    state: &ResolveContext,
    alias: AliasBindingValues<'_>,
    registry: &CommandRegistry,
) -> AliasBindingValidity {
    let Some(local) = alias.local.literal() else {
        return AliasBindingValidity::Unknown;
    };
    let policy = state.invocation_dialect.or_else(|| {
        registry
            .profile()
            .map(tcl_native_model::InvocationDialect::of_profile)
    });
    if alias
        .destination
        .is_active_in_frame_with_policy(state.alias_frame(), policy)
        == Some(false)
    {
        return AliasBindingValidity::Valid;
    }
    if policy.and_then(|dialect| dialect.variable_lookup_policy)
        == Some(tcl_dialect::VariableLookupPolicy::Tcl)
        && (state.namespace_scope() || state.global_frame())
        && matches!(alias.target, VariableAliasTarget::CurrentNamespace { .. })
    {
        return AliasBindingValidity::Valid;
    }
    let destination = crate::var_resolve::resolve_literal_place(local, state, false, registry);
    let slot = crate::var_resolve::resolve_alias_destination_slot(local, state, registry);
    let direct = alias_destination_is_direct(state, local, &slot);
    if direct
        && !destination.observed
        && state.contents_presence(&destination) == crate::var_resolve::ContentsPresence::Defined
    {
        return AliasBindingValidity::DefinedDestination;
    }
    if state.frame_kind == crate::var_resolve::VariableFrameKind::Local
        && !tcl_syntax::naming::is_qualified(local.as_bytes())
    {
        return AliasBindingValidity::Valid;
    }
    let slot = crate::var_resolve::resolve_alias_destination_slot(local, state, registry);
    let target = alias_target(state, alias.target, registry);
    if slot.kind == PlaceKind::Unknown || target.kind == PlaceKind::Unknown {
        return AliasBindingValidity::Unknown;
    }
    if !slot.is_global() || target.is_global() {
        return AliasBindingValidity::Valid;
    }
    let policy = state
        .invocation_dialect
        .and_then(|dialect| dialect.variable_lookup_policy)
        .or_else(|| {
            registry
                .profile()
                .and_then(tcl_dialect::DialectProfile::variable_lookup_policy)
        });
    match policy {
        Some(tcl_dialect::VariableLookupPolicy::Tcl) => {
            match target.cell.as_ref().map(|cell| &cell.owner) {
                Some(CellOwner::Activation(_) | CellOwner::CurrentFrame) => {
                    AliasBindingValidity::NamespaceToLocal
                }
                _ => AliasBindingValidity::Unknown,
            }
        }
        Some(tcl_dialect::VariableLookupPolicy::Jim) => AliasBindingValidity::Valid,
        None => AliasBindingValidity::Unknown,
    }
}

pub(super) fn bind_alias(
    state: &mut ResolveContext,
    alias: &VariableCellAliasTransition,
    registry: &CommandRegistry,
) {
    bind_alias_with_compiled_local(state, alias, registry, None, None);
}

pub(super) fn bind_authored_alias_values(
    state: &mut ResolveContext,
    alias: AliasBindingValues<'_>,
    registry: &CommandRegistry,
) {
    bind_alias_values_with_compiled_local(state, alias, registry, None, None);
}

/// Install `global NAME` in a procedure frame a driver evaluates a document
/// inside, as the `global` command would before the document's first line:
/// the local slot NAME resolves to in that frame aliases the global cell.
pub(crate) fn bind_entry_global_link(
    state: &mut ResolveContext,
    name: &str,
    registry: &CommandRegistry,
) {
    let subject = tcl_registry::TransitionSubject::Literal(name.to_owned());
    let alias = VariableCellAliasTransition {
        destination: tcl_registry::VariableAliasDestination::ProcedureLocal,
        local: subject.clone(),
        target: tcl_registry::VariableAliasTarget::Global { variable: subject },
        writes_value: false,
        words: tcl_registry::AliasWords::same(0),
    };
    let slot = crate::var_resolve::resolve_alias_destination_slot(name, state, registry);
    let target = alias_target(state, &alias.target, registry);
    if slot.kind == PlaceKind::Unknown || target.kind == PlaceKind::Unknown {
        state.widen();
        return;
    }
    install_alias_binding(
        state,
        (&alias).into(),
        registry,
        Some(&slot),
        cell_key(&slot),
        target,
        None,
    );
}

pub(super) fn bind_alias_with_compiled_local(
    state: &mut ResolveContext,
    alias: &VariableCellAliasTransition,
    registry: &CommandRegistry,
    compiled_local: Option<
        &crate::command_binding::original_variable_compilation::OriginalCompiledNamespaceLocal,
    >,
    operands: Option<&OriginalVariableInvocation>,
) {
    bind_alias_values_with_compiled_local(state, alias.into(), registry, compiled_local, operands);
}

fn bind_alias_values_with_compiled_local(
    state: &mut ResolveContext,
    alias: AliasBindingValues<'_>,
    registry: &CommandRegistry,
    compiled_local: Option<
        &crate::command_binding::original_variable_compilation::OriginalCompiledNamespaceLocal,
    >,
    operands: Option<&OriginalVariableInvocation>,
) {
    let dialect = state.invocation_dialect.or_else(|| {
        registry
            .profile()
            .map(tcl_native_model::InvocationDialect::of_profile)
    });
    let frame = state.alias_frame();
    let active = alias
        .destination
        .is_active_in_frame_with_policy(frame, dialect);
    if active == Some(false) {
        return;
    }
    let original_slot = if let Some(local) = compiled_local {
        if let Some(slot) = local.binding_slot(state) {
            Some(slot)
        } else {
            #[cfg(test)]
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
                eprintln!(
                    "ORIGINAL_VARIABLE_ALIAS_BIND invalid_compiled_local frame={frame:?} policy={:?} dynamic={}",
                    state.execution_name_policy, state.dynamic_bindings
                );
            }
            state.widen();
            return;
        }
    } else {
        operands.map(|operands| original_alias_destination(state, alias, operands, registry))
    };
    if original_slot
        .as_ref()
        .is_some_and(|slot| slot.kind == PlaceKind::Unknown)
    {
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
            eprintln!(
                "ORIGINAL_VARIABLE_ALIAS_BIND unknown_destination frame={frame:?} local_ordinal={:?} original_input={} dynamic={}",
                alias.local.argument_index(),
                operands
                    .and_then(|operands| operands.subject_input(alias.local, state))
                    .is_some(),
                state.dynamic_bindings
            );
        }
        state.widen();
        return;
    }
    let local = alias.local.literal();
    let key = match (&original_slot, local) {
        (Some(slot), _) => cell_key(slot),
        (None, Some(local)) => crate::var_resolve::VariableCellKey::Authored(local.to_owned()),
        (None, None) => {
            state.widen();
            return;
        }
    };
    if active.is_none() {
        state.unknown_bindings.insert(key.clone());
        state.alias_bindings.remove(&key);
        return;
    }
    let target = operands.map_or_else(
        || alias_target(state, alias.target, registry),
        |operands| original_alias_target(state, alias.target, operands, registry, false),
    );
    #[cfg(test)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
        eprintln!(
            "ORIGINAL_VARIABLE_ALIAS_BIND frame={frame:?} target_kind={:?} target_cell={} local_kind={:?} dynamic={}",
            target.kind,
            target.cell.is_some(),
            original_slot.as_ref().map(|slot| slot.kind),
            state.dynamic_bindings
        );
    }
    let validity = selected_alias_binding_validity(
        state,
        alias,
        registry,
        original_slot.as_ref(),
        &key,
        &target,
    );
    if matches!(
        validity,
        AliasBindingValidity::NamespaceToLocal | AliasBindingValidity::DefinedDestination
    ) {
        state.unknown_bindings.insert(key.clone());
        state.alias_bindings.remove(&key);
        return;
    }
    install_alias_binding(
        state,
        alias,
        registry,
        original_slot.as_ref(),
        key,
        target,
        operands,
    );
}

fn selected_alias_binding_validity(
    state: &ResolveContext,
    alias: AliasBindingValues<'_>,
    registry: &CommandRegistry,
    original_slot: Option<&Place>,
    key: &crate::var_resolve::VariableCellKey,
    target: &Place,
) -> AliasBindingValidity {
    original_slot.map_or_else(
        || alias_value_binding_validity(state, alias, registry),
        |slot| {
            let direct = !state.alias_bindings.contains_key(key)
                && !state.name_alias_bindings.contains_key(key)
                && !state.raw_bindings.bindings.contains_key(key);
            if direct
                && state.contents_presence(slot) == crate::var_resolve::ContentsPresence::Defined
            {
                AliasBindingValidity::DefinedDestination
            } else if slot.is_global()
                && !target.is_global()
                && state
                    .invocation_dialect
                    .and_then(|dialect| dialect.variable_lookup_policy)
                    == Some(tcl_dialect::VariableLookupPolicy::Tcl)
            {
                AliasBindingValidity::NamespaceToLocal
            } else {
                AliasBindingValidity::Valid
            }
        },
    )
}

fn install_alias_binding(
    state: &mut ResolveContext,
    alias: AliasBindingValues<'_>,
    registry: &CommandRegistry,
    original_slot: Option<&Place>,
    key: crate::var_resolve::VariableCellKey,
    target: Place,
    operands: Option<&OriginalVariableInvocation>,
) {
    let local = alias.local.literal();
    let name_link = state
        .invocation_dialect
        .and_then(|dialect| dialect.variable_link_binding)
        .or_else(|| {
            registry
                .profile()
                .and_then(tcl_dialect::DialectProfile::variable_link_binding)
        })
        == Some(tcl_dialect::VariableLinkBinding::SelectedFrameName);
    let named_target = name_link.then(|| {
        operands.map_or_else(
            || alias_name_target(state, alias.target, registry),
            |operands| original_alias_target(state, alias.target, operands, registry, true),
        )
    });
    if let Some(target) = &named_target {
        state
            .name_alias_bindings
            .insert(key.clone(), target.clone());
    } else {
        state.name_alias_bindings.remove(&key);
    }
    if let Some(named) = &named_target {
        if let Some(slot) = original_slot {
            state.retarget_raw_binding_at_slot(slot, named);
        } else if let Some(local) = local {
            state.retarget_raw_binding(local, named, registry);
        }
    }
    if target.kind == PlaceKind::Unknown {
        // Importing an unknown name can allocate an undefined cell, but it
        // does not delete known cells or alter their contents and observers.
        state.namespace_cells.closed = false;
        state.namespace_cells.closed_namespaces.clear();
    }
    state.allocate_namespace_alias_target(&target);
    if state.global_frame()
        || state.namespace_scope()
        || original_slot.is_some_and(Place::is_global)
        || (operands.is_none()
            && local.is_some_and(|local| tcl_syntax::naming::is_qualified(local.as_bytes())))
    {
        let slot = original_slot.cloned().unwrap_or_else(|| {
            crate::var_resolve::resolve_alias_destination_slot(
                local.expect("authored alias local"),
                state,
                registry,
            )
        });
        if slot.is_global() && !slot.dynamic {
            state
                .namespace_alias_bindings
                .insert(cell_key(&slot), target.clone());
            if let Some(target) = &named_target {
                state
                    .namespace_name_alias_bindings
                    .insert(cell_key(&slot), target.clone());
            } else {
                state.namespace_name_alias_bindings.remove(&cell_key(&slot));
            }
            state.namespace_cells.present.insert(cell_key(&slot));
        }
    }
    state.unknown_bindings.remove(&key);
    state.alias_bindings.insert(key, target);
}

fn original_alias_destination(
    state: &ResolveContext,
    alias: AliasBindingValues<'_>,
    operands: &OriginalVariableInvocation,
    registry: &CommandRegistry,
) -> Place {
    let Some(input) = operands.subject_input(alias.local, state) else {
        return place::unknown_top();
    };
    let bytes = match alias.target {
        VariableAliasTarget::Global { .. } => {
            tcl_syntax::naming::global_local_name_bytes(input.policy().recipe(), input.bytes())
        }
        VariableAliasTarget::CurrentNamespace { .. } => Some(
            tcl_syntax::naming::variable_local_name_bytes(input.policy().recipe(), input.bytes()),
        ),
        _ => Some(input.bytes().to_vec()),
    };
    let Some(bytes) = bytes else {
        return place::unknown_top();
    };
    crate::var_resolve::resolve_original_alias_destination_bytes(&bytes, state, registry)
}
