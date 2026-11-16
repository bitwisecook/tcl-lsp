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

//! Establishing a written cell's lifetime and destroying roots, captured cells and retained destructions.

use super::OriginalVariableInvocation;
use crate::place::{CellGeneration, Place, PlaceKind};
use crate::var_resolve::{
    ResolveContext, cell_key, resolve_literal_access, resolve_literal_place, trace_key,
};
use tcl_registry::{CommandRegistry, Traits};

pub(super) fn establish_written_lifetime(state: &mut ResolveContext, target: &Place, offset: u32) {
    if target.dynamic
        || target.kind == PlaceKind::Unknown
        || !target
            .cell
            .as_ref()
            .is_some_and(|cell| cell.generation == CellGeneration::Unknown)
    {
        return;
    }
    let key = cell_key(target);
    let generation = CellGeneration::After(offset);
    state.generations.insert(key.clone(), generation);
    for alias in state
        .alias_bindings
        .values_mut()
        .chain(state.namespace_alias_bindings.values_mut())
    {
        if cell_key(alias) == key
            && let Some(cell) = &mut alias.cell
        {
            cell.generation = generation;
        }
    }
}

/// Apply a reached literal unset through the shared cell lifetime/alias owner.
/// Dictionary entry and captured output protocols use this without inventing
/// a command invocation or re-running its argument evaluation.
pub fn destroy_literal_binding(
    state: &mut ResolveContext,
    name: &str,
    source: u32,
    registry: &CommandRegistry,
) {
    let target = resolve_literal_access(
        name,
        state,
        false,
        registry,
        tcl_registry::TraceOperation::Unset,
    );
    state.invalidate_contents_literals(&target);
    destroy_root(state, name, source, registry);
}

pub(super) fn destroy_root(
    state: &mut ResolveContext,
    name: &str,
    offset: u32,
    registry: &CommandRegistry,
) {
    if state.raw_static_unset_error(name, registry)
        || state.detach_retained_binding(name, offset, registry)
    {
        return;
    }
    let target = resolve_literal_access(
        name,
        state,
        false,
        registry,
        tcl_registry::TraceOperation::Unset,
    );
    destroy_captured_cell(state, &target, offset, registry);
}

/// Clear the selected cell without resolving its original variable name again.
/// The caller owns the captured unset callback list and completion protocol.
pub(crate) fn destroy_captured_cell(
    state: &mut ResolveContext,
    target: &Place,
    offset: u32,
    registry: &CommandRegistry,
) {
    state.record_presence_slot(target);
    if target.index.is_none()
        && let Some(root) = crate::var_resolve::physical_array_key(target)
    {
        state.closed_array_roots.remove(&root);
    }
    let root_kind = state.contents_kinds.get(&cell_key(target)).copied();
    state.captured_cells.destroy(target, root_kind);
    if target.observed {
        state.widen();
        return;
    }
    if let Some(name) = crate::var_resolve::canonical_binding_value_key(target) {
        state
            .contents_presence
            .insert(name, crate::var_resolve::ContentsPresence::Undefined);
    }
    let key = cell_key(target);
    let trace = trace_key(target);
    let removed: Vec<_> = state
        .traced
        .iter()
        .filter(|registered| {
            *registered == &trace
                || (target.kind != PlaceKind::ArrayElem && registered.is_member_of(&key))
        })
        .cloned()
        .collect();
    for registered in removed {
        state.traced.remove(&registered);
        state.trace_registrations.remove(&registered);
        state.trace_registration_receivers.remove(&registered);
        state.untracked_traces.remove(&registered);
    }
    if target.kind == PlaceKind::ArrayElem {
        return;
    }
    state.contents_kinds.remove(&key);
    state
        .generations
        .insert(key.clone(), CellGeneration::After(offset));
    let jim = state
        .invocation_dialect
        .and_then(|dialect| dialect.variable_link_binding)
        .or_else(|| {
            registry
                .profile()
                .and_then(tcl_dialect::DialectProfile::variable_link_binding)
        })
        == Some(tcl_dialect::VariableLinkBinding::SelectedFrameName);
    for (name, alias) in &mut state.alias_bindings {
        if cell_key(alias) == key {
            if alias.kind == PlaceKind::ArrayElem && !jim {
                state.unknown_bindings.insert(name.clone());
            } else if let Some(cell) = &mut alias.cell {
                cell.generation = CellGeneration::After(offset);
            }
        }
    }
}

pub(super) fn detach_retained_destructions(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    writes: &mut Vec<Place>,
    source: u32,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) {
    if !facts.traits.contains(Traits::DESTROYS_VARIABLE) {
        return;
    }
    for &(index, role) in &facts.arg_roles {
        if role != tcl_registry::ArgRole::VarWrite {
            continue;
        }
        let index = facts.argument_offset + usize::from(index);
        if let Some(operands) = operands {
            let Some(slot) = operands.raw_unset_slot(index, state, registry) else {
                continue;
            };
            let before = operands.access(
                index,
                state,
                registry,
                tcl_registry::TraceOperation::Unset,
                false,
            );
            if !before.observed && state.detach_retained_binding_at_slot(&slot, source) {
                writes.retain(|target| target != &before);
            }
        } else if let Some(name) = arguments.literal_at(index) {
            let before = resolve_literal_place(name, state, false, registry);
            if !before.observed && state.detach_retained_binding(name, source, registry) {
                writes.retain(|target| target != &before);
            }
        }
    }
}

pub(super) fn invalidate_literal_writes(state: &mut ResolveContext, writes: &[Place]) {
    for place in writes {
        state.invalidate_contents_literals(place);
    }
}
