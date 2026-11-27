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

//! Binding a base name to its cell: index classification, receivers, slot and name aliases, and cell identity.

use super::ResolveContext;
use super::canonical_keys::cell_key;
use super::place_resolution::{resolve_access, resolve_alias_destination_slot};
use crate::var_refs::{VarReferenceScanner, VarScanOptions};
use std::collections::HashSet;
use tcl_core_types::NameBytes;
use tcl_names::place;
use tcl_names::place::{CellGeneration, CellIdentity, CellOwner, Index, Place, PlaceKind};
use tcl_registry::CommandRegistry;

pub(super) fn has_subst(text: &str) -> bool {
    text.contains('$') || text.contains('[')
}

/// Protective lexical dependencies of a dynamic index or name.
/// Detached command text retains an unknown read rather than by-name cells.
pub(super) fn inner_read_places(
    text: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    let Some(config) = ctx.read_lexer_config(registry) else {
        return vec![place::unknown_top()];
    };
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions {
            include_var_read_roles: false,
            recurse_cmd_substitutions: true,
            include_reads_before_write: false,
            element_qualified: true,
        },
        config,
    );
    let names = scanner.scan_lexical_word(text, registry);
    let mut places: Vec<Place> = names
        .iter()
        .map(|name| {
            resolve_access(
                name,
                ctx,
                false,
                registry,
                tcl_registry::TraceOperation::Read,
            )
        })
        .collect();
    if !crate::var_refs::command_subst_texts_with_config(text, config).is_empty() {
        // Detached index text cannot select a command's by-name read operands.
        places.push(place::unknown_top());
    }
    places.sort_by(|a, b| (&a.ns, &a.name).cmp(&(&b.ns, &b.name)));
    places
}

/// Turn an array index / dict key text into an [`Index`].
pub(super) fn classify_index(
    elem: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Index {
    if has_subst(elem) {
        Index::dynamic(inner_read_places(elem, ctx, registry))
    } else {
        Index::literal(elem)
    }
}

/// Bind a (sigil-stripped, scalar) base name to a canonical [`Place`] using the
/// scope declarations in *ctx*.  `observed` overrides the trace-derived flag
/// when `Some`.
pub(super) fn bind_scalar(
    base: &str,
    ctx: &ResolveContext,
    observed: Option<bool>,
    registry: &CommandRegistry,
) -> Place {
    let mut bound = bind_scalar_receiver(base, ctx, observed, registry);
    if observed.is_none() {
        bound.observed |= ctx.unenumerated_observers_may_run(&bound);
    }
    bound
}

pub(super) fn bind_scalar_receiver(
    base: &str,
    ctx: &ResolveContext,
    observed: Option<bool>,
    registry: &CommandRegistry,
) -> Place {
    if ctx.observed_variable_storage_unavailable() {
        return place::unknown_top();
    }
    let obs = observed.unwrap_or_else(|| ctx.traced.contains(base));
    if ctx.unknown_bindings.contains(base) {
        return place::unknown_top();
    }
    if !ctx.raw_bindings.bindings.is_empty() {
        let slot = resolve_alias_destination_slot(base, ctx, registry);
        if let Some(key) = ctx.raw_bindings.bindings.get(&cell_key(&slot)) {
            let mut target = ctx.raw_bindings.resolve(key, ctx, registry);
            target.observed |= obs;
            return target;
        }
    }
    if let Some(mut target) = namespace_slot_alias(base, ctx, registry) {
        target.observed |= obs || ctx.traced.contains(&cell_key(&target));
        return target;
    }
    if let Some(target) = ctx.name_alias_bindings.get(base) {
        let mut target = resolve_name_alias(target, ctx);
        target.observed |= obs;
        return target;
    }
    if let Some(target) = ctx.alias_bindings.get(base) {
        let mut target = target.clone();
        target.observed |= obs || ctx.traced.contains(&cell_key(&target));
        return target;
    }
    if let Some(mut target) = activation_slot_alias(base, ctx, registry) {
        target.observed |= obs || ctx.traced.contains(&cell_key(&target));
        return target;
    }
    if ctx.dynamic_bindings && !tcl_syntax::naming::is_qualified(base.as_bytes()) {
        return place::unknown_top();
    }
    if let Some(target) = ctx.upvar_aliases.get(base) {
        return place::upvar_alias(base, target.clone(), false);
    }
    bind_unaliased_scalar(base, ctx, obs, registry)
}

fn bind_unaliased_scalar(
    base: &str,
    ctx: &ResolveContext,
    obs: bool,
    registry: &CommandRegistry,
) -> Place {
    let variable_policy = ctx
        .invocation_dialect
        .and_then(|dialect| dialect.variable_lookup_policy)
        .or_else(|| {
            registry
                .profile()
                .and_then(tcl_dialect::DialectProfile::variable_lookup_policy)
        });
    if variable_policy.is_none()
        && !tcl_syntax::naming::is_absolute(base)
        && !ctx.global_frame()
        && (ctx.namespace_scope() || tcl_syntax::naming::is_qualified(base.as_bytes()))
    {
        return place::unknown_top();
    }
    let jim_lookup = variable_policy == Some(tcl_dialect::VariableLookupPolicy::Jim);
    let bound = if ctx.instance_vars.contains(base) {
        place::instance_var(base, &ctx.instance_owner, obs)
    } else if ctx.globals.contains(base) {
        ctx.namespace_place(base, true, obs)
    } else if ctx.ns_vars.contains(base) {
        ctx.namespace_place(base, false, obs)
    } else if tcl_syntax::naming::is_absolute(base) {
        ctx.namespace_place(base, true, obs)
    } else if jim_lookup && !ctx.global_frame() {
        // Jim only treats an initial :: as a global qualifier. Namespace eval
        // runs a local activation; variable declarations install explicit links.
        place::scalar(base, place::LOCAL_NS, obs)
    } else if tcl_syntax::naming::is_qualified(base.as_bytes()) || ctx.namespace_scope() {
        if !ctx.namespace_known {
            return place::unknown_top();
        }
        let current = ctx.namespace_place(base, false, obs);
        let global = ctx.namespace_place(base, true, obs);
        let fallback = ctx
            .invocation_dialect
            .and_then(|dialect| dialect.namespace_var_global_fallback)
            .or_else(|| {
                registry
                    .profile()
                    .map(tcl_dialect::DialectProfile::namespace_var_global_fallback)
            });
        if fallback == Some(false)
            || cell_key(&current) == cell_key(&global)
            || ctx.ns_vars.contains(base)
            || current.kind != PlaceKind::Unknown
                && ctx.namespace_cells.present.contains(&cell_key(&current))
        {
            current
        } else if fallback == Some(true)
            && (current.kind != PlaceKind::Unknown
                && ctx.namespace_cells.absent(&cell_key(&current))
                || ctx.namespace_candidate_absent(base, false))
            && ctx.namespace_cells.present.contains(&cell_key(&global))
        {
            global
        } else if (global.kind != PlaceKind::Unknown
            && ctx.namespace_cells.absent(&cell_key(&global))
            || ctx.namespace_candidate_absent(base, true))
            && current.cell.as_ref().is_some_and(|cell| match &cell.owner {
                CellOwner::NamespaceIdentity(identity) => {
                    ctx.namespace_identities.contains(identity.as_ref())
                }
                CellOwner::Namespace(namespace) => ctx.known_namespaces.contains(namespace),
                _ => false,
            })
        {
            current
        } else {
            return place::unknown_top();
        }
    } else {
        place::scalar(base, place::LOCAL_NS, obs)
    };
    finish_bound_scalar(bound, ctx)
}

/// A link installed on a frame's own slot is keyed by that cell, which the
/// spelled name alone does not select.
fn activation_slot_alias(
    base: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<Place> {
    if ctx.alias_bindings.is_empty() && ctx.name_alias_bindings.is_empty() {
        return None;
    }
    let slot = resolve_alias_destination_slot(base, ctx, registry);
    if slot.kind == PlaceKind::Unknown || slot.is_global() {
        return None;
    }
    let key = cell_key(&slot);
    // A target the context cannot name stays with the slot, whose own
    // contents then answer unknown (`captured_contents_presence`).
    ctx.name_alias_bindings
        .get(&key)
        .map(|target| resolve_name_alias(target, ctx))
        .or_else(|| ctx.alias_bindings.get(&key).cloned())
        .filter(|target| target.kind != PlaceKind::Unknown && !target.dynamic)
}

fn namespace_slot_alias(
    base: &str,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<Place> {
    if (ctx.namespace_alias_bindings.is_empty() && ctx.namespace_name_alias_bindings.is_empty())
        || !(ctx.global_frame()
            || ctx.namespace_scope()
            || tcl_syntax::naming::is_qualified(base.as_bytes()))
    {
        return None;
    }
    // Namespace links belong to the slot, independently of a caller frame's
    // older spelling map retained across a selected-frame callback.
    let slot = resolve_alias_destination_slot(base, ctx, registry);
    if !slot.is_global() {
        return None;
    }
    let key = cell_key(&slot);
    ctx.namespace_name_alias_bindings
        .get(&key)
        .map(|target| resolve_name_alias(target, ctx))
        .or_else(|| ctx.namespace_alias_bindings.get(&key).cloned())
}

fn finish_bound_scalar(mut bound: Place, ctx: &ResolveContext) -> Place {
    if ctx.point_sensitive() {
        bind_cell_identity(&mut bound, ctx);
    }
    if bound.is_global()
        && let Some(target) = ctx.namespace_name_alias_bindings.get(&cell_key(&bound))
    {
        return resolve_name_alias(target, ctx);
    }
    if bound.is_global()
        && let Some(alias) = ctx.namespace_alias_bindings.get(&cell_key(&bound))
    {
        return alias.clone();
    }
    bound.observed |= ctx.traced.contains(&cell_key(&bound));
    bound
}

pub(super) fn resolve_name_alias(target: &Place, context: &ResolveContext) -> Place {
    let mut current = target.clone();
    let mut visited = HashSet::new();
    loop {
        if current.kind == PlaceKind::Unknown || !visited.insert(cell_key(&current)) {
            return place::unknown_top();
        }
        let selected = match current.cell.as_ref().map(|cell| &cell.owner) {
            Some(CellOwner::Activation(identity)) => {
                let mut frame = Some(context);
                while frame.is_some_and(|frame| frame.activation.as_ref() != Some(identity)) {
                    frame = frame.and_then(|frame| frame.caller.as_deref());
                }
                let Some(frame) = frame else {
                    return place::unknown_top();
                };
                frame
            }
            Some(
                CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_) | CellOwner::CurrentFrame,
            ) => context,
            _ => return place::unknown_top(),
        };
        let key = cell_key(&current);
        let authored_name = current
            .cell
            .as_ref()
            .map_or(Some(current.name.as_str()), |cell| {
                cell.name.try_utf8().ok()
            });
        if current.ns == place::LOCAL_NS
            && (selected.unknown_bindings.contains(&key)
                || authored_name.is_some_and(|name| selected.unknown_bindings.contains(name)))
        {
            return place::unknown_top();
        }
        let next = if current.ns == place::LOCAL_NS {
            selected
                .name_alias_bindings
                .get(&key)
                .or_else(|| authored_name.and_then(|name| selected.name_alias_bindings.get(name)))
        } else {
            context
                .namespace_name_alias_bindings
                .get(&cell_key(&current))
        };
        let Some(next) = next else {
            let generation = context
                .generations
                .get(&cell_key(&current))
                .copied()
                .unwrap_or_default();
            if let Some(cell) = &mut current.cell {
                cell.generation = generation;
            }
            return current;
        };
        let index = current.index.clone();
        current = next.clone();
        if let Some(index) = index {
            if current.index.is_some() {
                return place::unknown_top();
            }
            current.kind = PlaceKind::ArrayElem;
            current.index = Some(index);
        }
    }
}

pub(super) fn bind_cell_identity(bound: &mut Place, ctx: &ResolveContext) {
    let owner = if bound.kind == PlaceKind::InstanceVar {
        CellOwner::Instance(bound.owner.clone())
    } else if bound.ns == place::LOCAL_NS {
        ctx.selected_frame.map_or_else(
            || {
                ctx.activation
                    .as_ref()
                    .map_or(CellOwner::CurrentFrame, |identity| {
                        CellOwner::Activation(identity.clone())
                    })
            },
            CellOwner::SelectedFrame,
        )
    } else {
        bound
            .cell
            .as_ref()
            .and_then(|cell| match &cell.owner {
                CellOwner::NamespaceIdentity(identity) => {
                    Some(CellOwner::NamespaceIdentity(identity.clone()))
                }
                _ => None,
            })
            .unwrap_or_else(|| CellOwner::Namespace(bound.ns.clone()))
    };
    let storage_domain = if bound.ns == place::LOCAL_NS {
        None
    } else if let CellOwner::NamespaceIdentity(namespace) = &owner {
        if ctx.authored_tmm_static.as_ref().is_some_and(|receipt| {
            namespace.native_context() == Some(&receipt.namespace)
                && ctx.hosted_execution_context.is_none_or(|context| {
                    context == tcl_registry::f5::BigIpExecutionContext::TmmIRule
                })
        }) {
            Some(tcl_registry::f5::VariableStorageDomain::WorkerNamespace)
        } else if let Some(context) = ctx.hosted_execution_context {
            namespace
                .exact_native_path()
                .map(|path| tcl_registry::f5::namespace_storage_domain_in_path(context, path))
        } else {
            Some(tcl_registry::f5::VariableStorageDomain::InterpreterNamespace)
        }
    } else if ctx
        .hosted_execution_context
        .is_some_and(|context| context == tcl_registry::f5::BigIpExecutionContext::TmmIRule)
    {
        // A presentation namespace cannot classify a worker-owned table.
        None
    } else {
        Some(tcl_registry::f5::VariableStorageDomain::InterpreterNamespace)
    };
    bound.cell = Some(CellIdentity {
        owner,
        name: bound
            .cell
            .as_ref()
            .map_or_else(|| NameBytes::from(&bound.name), |cell| cell.name.clone()),
        generation: CellGeneration::Incoming,
        interpreter: ctx.interpreter.clone(),
        storage_domain,
        execution: ctx.execution.map(|mut execution| {
            if bound.ns != place::LOCAL_NS {
                execution.connection = None;
            }
            execution
        }),
    });
    let generation = ctx
        .generations
        .get(&cell_key(bound))
        .copied()
        .unwrap_or_default();
    if let Some(cell) = &mut bound.cell {
        cell.generation = generation;
    }
}
