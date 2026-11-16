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

//! Resolving a written or literal variable name to a place: alias destinations, namespace access origins, accesses and dictionary paths.

use super::observer_projection::project_access;
use super::scalar_binding::{bind_scalar, classify_index, has_subst, inner_read_places};
use super::{PossibleVariableAccessDomain, ResolveContext};
use crate::naming::split_array_name;
use crate::place;
use crate::place::{CellOwner, Index, Place, PlaceKind};
use tcl_registry::CommandRegistry;

/// Resolve an alias destination slot, whose namespace creation grammar never
/// uses ordinary read fallback to a same-named global cell.
#[must_use]
pub fn resolve_alias_destination_slot(
    reference: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Place {
    let mut slot = ctx.clone();
    slot.raw_bindings = crate::raw_binding::RawBindingArena::default();
    slot.alias_bindings.clear();
    slot.namespace_alias_bindings.clear();
    slot.name_alias_bindings.clear();
    slot.namespace_name_alias_bindings.clear();
    slot.globals.clear();
    slot.ns_vars.clear();
    slot.unknown_bindings.clear();
    slot.dynamic_bindings = false;
    let jim = slot
        .invocation_dialect
        .and_then(|dialect| dialect.variable_lookup_policy)
        .or_else(|| {
            registry
                .profile()
                .and_then(tcl_dialect::DialectProfile::variable_lookup_policy)
        })
        == Some(tcl_dialect::VariableLookupPolicy::Jim);
    if !jim && (slot.namespace_scope() || tcl_syntax::naming::is_qualified(reference.as_bytes())) {
        slot.ns_vars.insert(reference.to_owned());
    }
    resolve_literal_place(reference, &slot, false, registry)
}

/// How a proved namespace access was selected from an evaluated name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceAccessOrigin {
    /// The literal name explicitly selected a namespace table.
    ExplicitName,
    /// An alias or lookup selected a namespace cell outside the direct slot.
    NonLocalBinding,
    /// The current namespace activation's ordinary unqualified slot.
    CurrentFrame,
    /// No namespace target or direct-slot relationship was proved.
    Unknown,
}

/// Preserve namespace selection provenance independently of contents effects.
/// The name is an evaluated literal; source quoting and substitution syntax
/// never determine this result. Unknown targets remain unknown.
#[must_use]
pub fn literal_namespace_access_origin(
    reference: &str,
    context: &ResolveContext,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
) -> NamespaceAccessOrigin {
    let target = resolve_literal_access(reference, context, false, registry, operation);
    if target.dynamic
        || !matches!(
            target.cell.as_ref().map(|cell| &cell.owner),
            Some(CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_))
        )
    {
        return NamespaceAccessOrigin::Unknown;
    }
    let (root, _) = tcl_syntax::naming::split_array_name(reference);
    if tcl_syntax::naming::is_qualified(root.as_bytes()) {
        return NamespaceAccessOrigin::ExplicitName;
    }
    let direct = resolve_alias_destination_slot(reference, context, registry);
    if target.cell == direct.cell {
        NamespaceAccessOrigin::CurrentFrame
    } else if direct.cell.is_some() && !direct.dynamic {
        NamespaceAccessOrigin::NonLocalBinding
    } else {
        NamespaceAccessOrigin::Unknown
    }
}

/// Bound the storage domain of an already evaluated literal variable name.
/// C Tcl forbids a namespace binding slot from linking to a procedure-local
/// cell, even when the target link is unknown after evaluated code. Namespace
/// aliases can still select any namespace, including host worker namespaces.
/// Do not use this result as a Must address or successful-store proof.
#[must_use]
pub fn possible_access_domain(
    reference: &str,
    context: &ResolveContext,
    registry: &CommandRegistry,
) -> PossibleVariableAccessDomain {
    if context
        .invocation_dialect
        .and_then(|dialect| dialect.variable_lookup_policy)
        != Some(tcl_dialect::VariableLookupPolicy::Tcl)
    {
        return PossibleVariableAccessDomain::Any;
    }
    let slot = resolve_alias_destination_slot(reference, context, registry);
    if !slot.dynamic
        && matches!(
            slot.cell.as_ref().map(|cell| &cell.owner),
            Some(CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_))
        )
    {
        PossibleVariableAccessDomain::NamespaceOnly
    } else {
        PossibleVariableAccessDomain::Any
    }
}

pub(super) fn authored_namespace_place(name: &str, namespace: &str, observed: bool) -> Place {
    let fq = tcl_syntax::naming::qualify(namespace, name);
    let (ns, tail) = tcl_syntax::naming::key_holder_and_tail(&fq);
    place::scalar(tail, if ns.is_empty() { "::" } else { ns }, observed)
}

/// Resolve a variable reference *reference* (as written) to a canonical
/// [`Place`].
///
/// `whole_array` marks a reference the command touches as an entire array
/// (`array get a` / `array set a` / a bare `$a` passed to such), so it becomes
/// `ARRAY_WHOLE` rather than a scalar.
#[must_use]
pub fn resolve_place(
    reference: &str,
    ctx: &ResolveContext,
    whole_array: bool,
    registry: &CommandRegistry,
) -> Place {
    resolve_place_value(reference, ctx, whole_array, registry, false, false)
}

/// Resolve an already-evaluated literal variable name, including literal dollar signs.
#[must_use]
pub fn resolve_literal_place(
    reference: &str,
    ctx: &ResolveContext,
    whole_array: bool,
    registry: &CommandRegistry,
) -> Place {
    resolve_place_value(reference, ctx, whole_array, registry, true, true)
}

/// Resolve a written variable-name word, preserving whether braces suppress
/// name and array-index substitutions. The resulting access retains reads
/// needed to form a dynamic target rather than inventing a literal cell.
#[must_use]
pub fn resolve_target_access(
    reference: &str,
    braced: bool,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
) -> Place {
    project_access(
        resolve_place_value(reference, ctx, false, registry, braced, braced),
        ctx,
        operation,
    )
}

pub(super) fn resolve_place_value(
    reference: &str,
    ctx: &ResolveContext,
    whole_array: bool,
    registry: &CommandRegistry,
    literal_base: bool,
    literal_index: bool,
) -> Place {
    if ctx.observed_variable_storage_unavailable() {
        return place::unknown_top();
    }
    if reference.is_empty() && !literal_base {
        return place::unknown_top();
    }

    // A target whose *name* is value-substituted (`set $X v`, `set ${tok}(k) v`)
    // denotes the variable named by the substitution's value — a place that
    // cannot be pinned down — but it *reads* the name variable(s).  Read
    // references arrive de-sigilled from the scanner, so a leading `$` here
    // always marks a write-target dynamic name.
    if !literal_base && reference.starts_with('$') {
        return place::unknown(inner_read_places(reference, ctx, registry));
    }

    let (base, elem) = tcl_syntax::naming::split_array_name_braced(reference, literal_base);

    if !literal_base && (base.is_empty() || has_subst(base)) {
        return place::unknown(inner_read_places(reference, ctx, registry));
    }

    if let Some(elem) = elem {
        let index = if literal_index {
            Index::literal(elem)
        } else {
            classify_index(elem, ctx, registry)
        };
        let bound = bind_scalar(base, ctx, None, registry);
        if bound.kind == PlaceKind::Unknown || bound.kind == PlaceKind::ArrayElem {
            return place::unknown(index.read_places);
        }
        // Carry namespace/observed/dynamic/owner from the bound scalar.
        return Place {
            kind: PlaceKind::ArrayElem,
            ns: bound.ns,
            name: bound.name,
            index: Some(index),
            keys: Vec::new(),
            owner: bound.owner,
            observed: bound.observed,
            dynamic: bound.dynamic,
            name_reads: Vec::new(),
            cell: bound.cell,
        };
    }

    if whole_array {
        let bound = bind_scalar(base, ctx, None, registry);
        if bound.kind == PlaceKind::Unknown || bound.kind == PlaceKind::ArrayElem {
            return place::unknown_top();
        }
        return Place {
            kind: PlaceKind::ArrayWhole,
            ns: bound.ns,
            name: bound.name,
            index: None,
            keys: Vec::new(),
            owner: bound.owner,
            observed: bound.observed,
            dynamic: bound.dynamic,
            name_reads: Vec::new(),
            cell: bound.cell,
        };
    }

    bind_scalar(base, ctx, None, registry)
}

/// Resolve a source variable read/write and select applicable trace callbacks.
#[must_use]
pub fn resolve_access(
    reference: &str,
    ctx: &ResolveContext,
    whole_array: bool,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
) -> Place {
    let read = operation == tcl_registry::TraceOperation::Read;
    project_access(
        resolve_place_value(reference, ctx, whole_array, registry, read, false),
        ctx,
        operation,
    )
}

/// Resolve one original variable-substitution spelling without losing an array index.
/// Callers retain the source spelling: canonical `${...}` display wrappers cannot
/// establish whether an original array index was braced or substituted.
#[must_use]
pub fn resolve_substitution_access(
    spelling: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
) -> Place {
    let Some(config) = ctx.read_lexer_config(registry) else {
        return place::unknown_top();
    };
    let name = tcl_syntax::naming::var_reference_for_style(spelling, config.braced_var);
    let braced = tcl_syntax::naming::split_braced_var_ref(spelling, config.braced_var).is_some();
    project_access(
        resolve_place_value(name, ctx, false, registry, true, braced),
        ctx,
        operation,
    )
}

/// Resolve a literal name operand and select its applicable trace callbacks.
#[must_use]
pub fn resolve_literal_access(
    reference: &str,
    ctx: &ResolveContext,
    whole_array: bool,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
) -> Place {
    project_access(
        resolve_literal_place(reference, ctx, whole_array, registry),
        ctx,
        operation,
    )
}

/// Resolve a proved evaluated array-name shape without inventing its element value.
/// Alias lookup happens in the dispatch context, after all original words evaluated.
#[must_use]
pub fn resolve_array_element_root(
    root: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
) -> Place {
    let mut bound = bind_scalar(root, ctx, None, registry);
    if matches!(bound.kind, PlaceKind::Unknown | PlaceKind::ArrayElem) {
        return place::unknown_top();
    }
    bound.kind = PlaceKind::ArrayElem;
    bound.index = Some(Index::any());
    project_access(bound, ctx, operation)
}

/// Resolve a `dict get/set $d k1 k2 …` access to a `DICT_PATH` [`Place`].
#[must_use]
pub fn resolve_dict_path(
    root_ref: &str,
    keys: &[String],
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Place {
    let (base, _) = split_array_name(root_ref);
    if base.is_empty() || has_subst(base) {
        return place::unknown(inner_read_places(root_ref, ctx, registry));
    }
    let bound = resolve_place(root_ref, ctx, false, registry);
    if bound.kind == PlaceKind::Unknown {
        return bound;
    }
    let key_indices: Vec<Index> = keys
        .iter()
        .map(|k| classify_index(k, ctx, registry))
        .collect();
    let mut path = place::dict_path(bound.name, key_indices, bound.ns);
    path.cell = bound.cell;
    path.index = bound.index;
    path.observed = bound.observed;
    path.dynamic = bound.dynamic;
    path
}
