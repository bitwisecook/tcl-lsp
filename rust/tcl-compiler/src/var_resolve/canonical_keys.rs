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

//! Stable keys and compatibility names for bound places: cell keys, binding-value keys and namespace-variable identities.

use super::ResolveContext;
use super::place_resolution::{resolve_literal_place, resolve_place, resolve_place_value};
use tcl_core_types::NameBytes;
use tcl_names::place;
use tcl_names::place::{CellGeneration, CellIdentity, CellOwner, Place, PlaceKind};
use tcl_names::variable::{VariableCellKey, VariableCellKeyQuery, VariableNamespaceQuery};
use tcl_registry::CommandRegistry;

/// Stable name for a root's contents and lifetime state.
#[must_use]
pub fn cell_key(place: &Place) -> VariableCellKey {
    if let Some(cell) = &place.cell {
        let simple = cell.name.clone();
        match &cell.owner {
            CellOwner::NamespaceIdentity(identity) => {
                return VariableCellKey::Namespace {
                    identity: (**identity).clone(),
                    simple,
                };
            }
            CellOwner::AllocatedInstance(allocation) => {
                return VariableCellKey::AllocatedInstance {
                    allocation: allocation.clone(),
                    simple,
                };
            }
            CellOwner::RetainedSlot(identity) => {
                return VariableCellKey::RetainedSlot(identity.clone());
            }
            CellOwner::SelectedFrame(selector) => {
                return VariableCellKey::SelectedFrame {
                    selector: *selector,
                    simple,
                };
            }
            CellOwner::Instance(identity) => {
                return VariableCellKey::Instance {
                    identity: identity.clone(),
                    simple,
                };
            }
            CellOwner::Activation(identity) => {
                return VariableCellKey::Activation {
                    identity: identity.clone(),
                    simple,
                };
            }
            CellOwner::CurrentFrame | CellOwner::Namespace(_) => {}
        }
    }
    if place.ns == place::LOCAL_NS {
        place.name.clone().into()
    } else {
        tcl_syntax::naming::qualify(&place.ns, &place.name).into()
    }
}

/// Whether an owner-authored storage key denotes a cell under this namespace.
/// Lifetime wrappers retain the underlying namespace membership.
#[must_use]
pub(crate) fn binding_key_in_namespace<
    Q: VariableCellKeyQuery + ?Sized,
    N: VariableNamespaceQuery + ?Sized,
>(
    key: &Q,
    namespace: &N,
) -> bool {
    key.variable_cell_key()
        .is_in_namespace(namespace.variable_namespace_key().as_ref())
}

/// Project a proven bound access into the compatibility name vocabulary.
/// Unknown or foreign-frame targets never acquire a local name.
#[must_use]
pub fn canonical_variable_name(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<String> {
    canonical_variable_key(reference, ctx, registry).map(|key| key.compatibility_name())
}

/// Canonical scalar key for an already-evaluated literal name operand.
#[must_use]
pub fn canonical_literal_variable_name(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<String> {
    canonical_binding_value_name(&resolve_literal_place(reference, ctx, false, registry))
}

/// Exact binding-value address selected from an original written variable operand.
#[must_use]
pub fn canonical_literal_variable_key(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<VariableCellKey> {
    canonical_binding_value_key(&resolve_literal_place(reference, ctx, false, registry))
}

/// Physical array-root identity, preserving captured lifetimes.
#[must_use]
pub(crate) fn physical_array_key(place: &Place) -> Option<VariableCellKey> {
    let mut root = place.clone();
    root.index = None;
    root.kind = PlaceKind::Scalar;
    canonical_place_key(&root)
}

/// Scalar contents key of the current binding slot. A successful scalar store
/// defines the slot's value even when a loop joins different physical lifetimes.
/// Captured element links remain lifetime-sensitive because deletion can dangle them.
#[must_use]
pub fn canonical_binding_value_name(bound: &Place) -> Option<String> {
    canonical_binding_value_key(bound).map(|key| key.compatibility_name())
}

/// Exact binding-value slot, retaining native namespace and member identity.
#[must_use]
pub fn canonical_binding_value_key(bound: &Place) -> Option<VariableCellKey> {
    if bound.index.is_none() && matches!(bound.kind, PlaceKind::Scalar | PlaceKind::ArrayWhole) {
        let mut binding = bound.clone();
        if let Some(cell) = &mut binding.cell {
            cell.generation = CellGeneration::Incoming;
        }
        canonical_place_key(&binding)
    } else {
        canonical_place_key(bound)
    }
}

/// Project an immutable bound access into compatibility scalar SSA names.
#[must_use]
pub fn canonical_place_name(bound: &Place) -> Option<String> {
    canonical_place_key(bound).map(|key| key.compatibility_name())
}

/// Exact physical storage slot, including selected lifetime and array member.
#[must_use]
pub fn canonical_place_key(bound: &Place) -> Option<VariableCellKey> {
    if bound
        .cell
        .as_ref()
        .is_some_and(|cell| matches!(cell.owner, CellOwner::SelectedFrame(_)))
        || bound.dynamic
        || matches!(
            bound.kind,
            PlaceKind::Unknown | PlaceKind::UpvarAlias | PlaceKind::InstanceVar
        )
    {
        return None;
    }
    let mut name = cell_key(bound);
    if let Some(cell) = &bound.cell {
        match cell.generation {
            CellGeneration::Incoming => {}
            CellGeneration::After(offset) => name = name.with_lifetime(offset),
            CellGeneration::Unknown => return None,
        }
    }
    if let Some(index) = &bound.index {
        if index.kind != place::IndexKind::Literal {
            return None;
        }
        name = name.with_index(index.value.clone());
    }
    Some(name)
}

/// Borrow a namespace root from its retained owner, including an actual variable link.
fn retained_namespace_variable_root(cell: &CellIdentity) -> Option<(&CellOwner, &NameBytes)> {
    match &cell.owner {
        CellOwner::NamespaceIdentity(_) | CellOwner::Namespace(_) => {
            Some((&cell.owner, &cell.name))
        }
        CellOwner::RetainedSlot(slot) => match slot.as_ref() {
            tcl_names::variable::RawBindingSlotId::Variable(cell) => {
                retained_namespace_variable_root(cell)
            }
            tcl_names::variable::RawBindingSlotId::Callable { .. } => None,
        },
        _ => None,
    }
}

/// Checked UTF-8 of an already resolved namespace root key, independent of its display.
/// Authored places without a retained cell remain descriptive source advice.
#[must_use]
pub fn namespace_variable_simple_name(place: &Place) -> Option<&str> {
    match place.cell.as_ref() {
        Some(cell) => retained_namespace_variable_root(cell)?.1.try_utf8().ok(),
        None => place.is_global().then_some(place.name.as_str()),
    }
}

/// Original simple key in the actual root namespace table, including resolved links.
/// Selected frames, locals and displayed namespace names do not grant root identity.
#[must_use]
pub fn root_namespace_variable_simple_name(place: &Place) -> Option<&str> {
    match place.cell.as_ref() {
        Some(cell) => {
            let (owner, name) = retained_namespace_variable_root(cell)?;
            let root = match owner {
                CellOwner::NamespaceIdentity(namespace) => match namespace.as_ref() {
                    tcl_names::namespace::SourceNamespaceKey::Authored(namespace) => {
                        namespace == "::"
                    }
                    _ => namespace
                        .exact_native_path()
                        .is_some_and(tcl_core_types::ByteNamespacePath::is_root),
                },
                CellOwner::Namespace(namespace) => namespace == "::",
                _ => false,
            };
            root.then(|| name.try_utf8().ok()).flatten()
        }
        // Legacy symbolic places have no native owner to project. The
        // already resolved namespace field is authored advice only.
        None => (place.is_global() && place.ns == "::").then_some(place.name.as_str()),
    }
}

/// Fully qualified identity for a proven namespace-variable access.
#[must_use]
pub fn canonical_namespace_variable(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<String> {
    canonical_namespace_variable_key(reference, ctx, registry).map(|key| key.compatibility_name())
}

/// Original resolved namespace receiver, retaining its actual cell and storage domain.
#[must_use]
pub fn canonical_namespace_variable_place(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<Place> {
    let place = resolve_place(reference, ctx, false, registry);
    (place.is_global() && !place.dynamic && canonical_place_key(&place).is_some()).then_some(place)
}

/// Exact namespace cell address; its compatibility display grants no lookup authority.
#[must_use]
pub fn canonical_namespace_variable_key(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<VariableCellKey> {
    canonical_place_key(&canonical_namespace_variable_place(
        reference, ctx, registry,
    )?)
}

/// Exact selected storage slot for a source variable reference.
#[must_use]
pub fn canonical_variable_key(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<VariableCellKey> {
    canonical_binding_value_key(&resolve_place_value(
        reference, ctx, false, registry, true, false,
    ))
}
