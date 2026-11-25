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

//! Equality, hashing and the observation stamp of a binding state.

use super::super::command_identity::{CommandIdentity, CommandToken};
use super::super::ensemble_dispatch::EnsembleDispatches;
use super::super::namespace_context::{SourceNamespaceKey, SourceNamespaceMap, SourceNamespaceSet};
use super::super::{
    ClassDefinitionReceipt, EnsembleManagement, MayBinding, ModuleCommandBindings,
    PackageProvision, ResolvedCommandTarget, command_observers, literal_object_pool,
    logical_prefix, object_instance, source_command_world,
};
use super::bounded_classes::BoundedClassConstruction;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Compact identity for every mutable observation axis other than the live
/// binding map. All set-valued axes below are monotone during the binding
/// walk, so their cardinality changes exactly when their semantic value grows.
/// The immutable baseline and the once-published root-boundary map are omitted:
/// neither can change during one invocation transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // Mirrors independent monotone observation axes.
pub(super) struct NonBindingObservationStamp {
    pub(in super::super) original_command_world:
        Arc<source_command_world::OriginalSourceCommandWorld>,
    pub(in super::super) logical_alias_prefixes:
        Arc<BTreeMap<ResolvedCommandTarget, Option<Arc<logical_prefix::LogicalAliasPrefix>>>>,
    pub(in super::super) ordinary_literal_pool:
        Option<literal_object_pool::SourceOrdinaryLiteralPool>,
    stock_tclsh_stdout: bool,
    pub(in super::super) class_definitions: Arc<BTreeMap<CommandIdentity, ClassDefinitionReceipt>>,
    pub(in super::super) object_instances: Arc<object_instance::SourceObjectState>,
    pub(in super::super) bounded_constructions:
        Arc<BTreeMap<CommandIdentity, BoundedClassConstruction>>,
    pub(in super::super) tainted_object_dispatch: Arc<BTreeSet<String>>,
    pub(in super::super) tainted_provider_state: Arc<BTreeSet<String>>,
    pub(in super::super) command_observers: Arc<command_observers::SourceCommandObservers>,
    pub(in super::super) source_variables: Arc<crate::var_resolve::ResolveContext>,
    pub(in super::super) variable_frame: crate::var_resolve::VariableExecutionFrame,
    pub(in super::super) objects: Arc<BTreeMap<CommandToken, BTreeSet<MayBinding>>>,
    pub(in super::super) namespace_exports:
        Arc<SourceNamespaceMap<BTreeSet<Vec<tcl_core_types::NameBytes>>>>,
    pub(in super::super) namespace_paths:
        Arc<SourceNamespaceMap<BTreeSet<Vec<SourceNamespaceKey>>>>,
    pub(in super::super) namespaces: Arc<SourceNamespaceSet>,
    pub(in super::super) unknown_lookup_namespaces: Arc<SourceNamespaceSet>,
    pub(in super::super) unknown_namespace_paths: Arc<SourceNamespaceSet>,
    pub(in super::super) namespace_unknown_handlers: Arc<SourceNamespaceSet>,
    pub(in super::super) ensemble_owner_namespaces: Arc<SourceNamespaceSet>,
    pub(in super::super) ensemble_dispatches: Arc<EnsembleDispatches>,
    pub(in super::super) unknown_export_namespaces: Arc<SourceNamespaceSet>,
    pub(in super::super) packages: Arc<BTreeMap<String, BTreeSet<PackageProvision>>>,
    pub(in super::super) revoked_loaders: Arc<BTreeSet<String>>,
    pub(in super::super) loader_handler_unknown: bool,
    pub(in super::super) opaque_domain: bool,
    pub(in super::super) opaque_binding_mutation: bool,
    pub(in super::super) dynamic_proc_binding: bool,
    pub(in super::super) unnameable_rebinding_subject: bool,
    pub(in super::super) unclosed_command_world: bool,
    namespace_changed: bool,
    namespace_dynamic: bool,
    namespace_ensembles: EnsembleManagement,
    namespace_rebound_count: usize,
    namespace_opaque_count: usize,
    procedure_body_count: usize,
    rebound_count: usize,
    proc_rebound_count: usize,
    pub(in super::super) has_redefined_procedures: bool,
}

impl PartialEq for ModuleCommandBindings {
    fn eq(&self, other: &Self) -> bool {
        // This public state participates in semantic interning. Two
        // independent analyses of the same registry must therefore compare
        // equal even though each owns a distinct baseline allocation.
        self.baseline == other.baseline
            && self.original_command_world == other.original_command_world
            && self.logical_alias_prefixes == other.logical_alias_prefixes
            && self.current_source_origin == other.current_source_origin
            && self.ordinary_literal_pool == other.ordinary_literal_pool
            && self.stock_tclsh_stdout == other.stock_tclsh_stdout
            && self.allocation_counts == other.allocation_counts
            && self.namespace_allocation_counts == other.namespace_allocation_counts
            && self.class_definitions == other.class_definitions
            && self.object_instances == other.object_instances
            && self.bounded_constructions == other.bounded_constructions
            && self.tainted_object_dispatch == other.tainted_object_dispatch
            && self.tainted_provider_state == other.tainted_provider_state
            && self.command_observers == other.command_observers
            && self.source_variables == other.source_variables
            && self.variable_frame == other.variable_frame
            && self.bindings == other.bindings
            && self.objects == other.objects
            && self.namespace_exports == other.namespace_exports
            && self.namespace_paths == other.namespace_paths
            && self.namespaces == other.namespaces
            && self.unknown_lookup_namespaces == other.unknown_lookup_namespaces
            && self.unknown_namespace_paths == other.unknown_namespace_paths
            && self.namespace_unknown_handlers == other.namespace_unknown_handlers
            && self.ensemble_owner_namespaces == other.ensemble_owner_namespaces
            && self.ensemble_dispatches == other.ensemble_dispatches
            && self.unknown_export_namespaces == other.unknown_export_namespaces
            && self.packages == other.packages
            && self.revoked_loaders == other.revoked_loaders
            && self.loader_handler_unknown == other.loader_handler_unknown
            && self.root_boundary_bindings == other.root_boundary_bindings
            && self.original_entry_bindings == other.original_entry_bindings
            && self.original_entry_packages == other.original_entry_packages
            && self.opaque_domain == other.opaque_domain
            && self.opaque_binding_mutation == other.opaque_binding_mutation
            && self.dynamic_proc_binding == other.dynamic_proc_binding
            && self.unnameable_rebinding_subject == other.unnameable_rebinding_subject
            && self.unclosed_command_world == other.unclosed_command_world
            && self.diagnostic_command_world_closed == other.diagnostic_command_world_closed
            && self.receiver_frame_opacity == other.receiver_frame_opacity
            && self.namespace_resolution == other.namespace_resolution
            && self.procedure_bodies == other.procedure_bodies
            && self.rebound_names == other.rebound_names
            && self.proc_rebound_names == other.proc_rebound_names
            && self.has_redefined_procedures == other.has_redefined_procedures
            && self.source_order_user_call_effects == other.source_order_user_call_effects
    }
}

impl Eq for ModuleCommandBindings {}

impl ModuleCommandBindings {
    /// Fast equality for clones and branches inside *one* analysis. Public
    /// equality remains semantic for Salsa/context interning; the fixpoint
    /// owns one baseline [`Arc`], so its allocation identity is sufficient.
    pub(in super::super) fn same_state(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.baseline, &other.baseline)
            && (Arc::ptr_eq(&self.original_command_world, &other.original_command_world)
                || self.original_command_world == other.original_command_world)
            && self.logical_alias_prefixes == other.logical_alias_prefixes
            && self.ordinary_literal_pool == other.ordinary_literal_pool
            && self.stock_tclsh_stdout == other.stock_tclsh_stdout
            && self.class_definitions == other.class_definitions
            && self.object_instances == other.object_instances
            && self.bounded_constructions == other.bounded_constructions
            && self.tainted_object_dispatch == other.tainted_object_dispatch
            && self.tainted_provider_state == other.tainted_provider_state
            && self.command_observers == other.command_observers
            && self.source_variables == other.source_variables
            && self.variable_frame == other.variable_frame
            && (Arc::ptr_eq(&self.bindings, &other.bindings) || self.bindings == other.bindings)
            && self.objects == other.objects
            && self.namespace_exports == other.namespace_exports
            && self.namespace_paths == other.namespace_paths
            && self.namespaces == other.namespaces
            && self.unknown_lookup_namespaces == other.unknown_lookup_namespaces
            && self.unknown_namespace_paths == other.unknown_namespace_paths
            && self.namespace_unknown_handlers == other.namespace_unknown_handlers
            && self.ensemble_owner_namespaces == other.ensemble_owner_namespaces
            && self.ensemble_dispatches == other.ensemble_dispatches
            && self.unknown_export_namespaces == other.unknown_export_namespaces
            && self.packages == other.packages
            && self.revoked_loaders == other.revoked_loaders
            && self.loader_handler_unknown == other.loader_handler_unknown
            && (Arc::ptr_eq(&self.root_boundary_bindings, &other.root_boundary_bindings)
                || self.root_boundary_bindings == other.root_boundary_bindings)
            && self.opaque_domain == other.opaque_domain
            && self.opaque_binding_mutation == other.opaque_binding_mutation
            && self.dynamic_proc_binding == other.dynamic_proc_binding
            && self.unnameable_rebinding_subject == other.unnameable_rebinding_subject
            && self.unclosed_command_world == other.unclosed_command_world
            && self.diagnostic_command_world_closed == other.diagnostic_command_world_closed
            && self.receiver_frame_opacity == other.receiver_frame_opacity
            && self.namespace_resolution == other.namespace_resolution
            && (Arc::ptr_eq(&self.procedure_bodies, &other.procedure_bodies)
                || self.procedure_bodies == other.procedure_bodies)
            && self.rebound_names == other.rebound_names
            && self.proc_rebound_names == other.proc_rebound_names
            && self.has_redefined_procedures == other.has_redefined_procedures
            && self.source_order_user_call_effects == other.source_order_user_call_effects
    }

    pub(super) fn non_binding_observation_stamp(&self) -> NonBindingObservationStamp {
        NonBindingObservationStamp {
            original_command_world: Arc::clone(&self.original_command_world),
            logical_alias_prefixes: Arc::clone(&self.logical_alias_prefixes),
            ordinary_literal_pool: self.ordinary_literal_pool.clone(),
            stock_tclsh_stdout: self.stock_tclsh_stdout,
            class_definitions: Arc::clone(&self.class_definitions),
            object_instances: Arc::clone(&self.object_instances),
            bounded_constructions: Arc::clone(&self.bounded_constructions),
            tainted_object_dispatch: Arc::clone(&self.tainted_object_dispatch),
            tainted_provider_state: Arc::clone(&self.tainted_provider_state),
            command_observers: Arc::clone(&self.command_observers),
            source_variables: Arc::clone(&self.source_variables),
            variable_frame: self.variable_frame.clone(),
            objects: Arc::clone(&self.objects),
            namespace_exports: Arc::clone(&self.namespace_exports),
            namespace_paths: Arc::clone(&self.namespace_paths),
            namespaces: Arc::clone(&self.namespaces),
            unknown_lookup_namespaces: Arc::clone(&self.unknown_lookup_namespaces),
            unknown_namespace_paths: Arc::clone(&self.unknown_namespace_paths),
            namespace_unknown_handlers: Arc::clone(&self.namespace_unknown_handlers),
            ensemble_owner_namespaces: Arc::clone(&self.ensemble_owner_namespaces),
            ensemble_dispatches: Arc::clone(&self.ensemble_dispatches),
            unknown_export_namespaces: Arc::clone(&self.unknown_export_namespaces),
            packages: Arc::clone(&self.packages),
            revoked_loaders: Arc::clone(&self.revoked_loaders),
            loader_handler_unknown: self.loader_handler_unknown,
            opaque_domain: self.opaque_domain,
            opaque_binding_mutation: self.opaque_binding_mutation,
            dynamic_proc_binding: self.dynamic_proc_binding,
            unnameable_rebinding_subject: self.unnameable_rebinding_subject,
            unclosed_command_world: self.unclosed_command_world,
            namespace_changed: self.namespace_resolution.changed,
            namespace_dynamic: self.namespace_resolution.dynamic,
            namespace_ensembles: self.namespace_resolution.ensembles,
            namespace_rebound_count: self.namespace_resolution.rebound_names.len(),
            namespace_opaque_count: self.namespace_resolution.opaque_namespaces.len(),
            procedure_body_count: self.procedure_bodies.len(),
            rebound_count: self.rebound_names.len(),
            proc_rebound_count: self.proc_rebound_names.len(),
            has_redefined_procedures: self.has_redefined_procedures,
        }
    }
}

impl std::hash::Hash for ModuleCommandBindings {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.original_command_world.hash(state);
        self.logical_alias_prefixes.hash(state);
        self.current_source_origin.hash(state);
        self.ordinary_literal_pool.hash(state);
        self.stock_tclsh_stdout.hash(state);
        self.allocation_counts.hash(state);
        self.namespace_allocation_counts.hash(state);
        self.class_definitions.hash(state);
        self.object_instances.hash(state);
        self.bounded_constructions.hash(state);
        self.tainted_object_dispatch.hash(state);
        self.tainted_provider_state.hash(state);
        self.command_observers.hash(state);
        self.source_variables.hash(state);
        self.variable_frame.hash(state);
        let mut entries: Vec<_> = self.bindings.iter().collect();
        entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
        entries.hash(state);
        self.objects.hash(state);
        self.namespace_exports.hash(state);
        self.namespace_paths.hash(state);
        self.namespaces.hash(state);
        self.unknown_lookup_namespaces.hash(state);
        self.unknown_namespace_paths.hash(state);
        self.namespace_unknown_handlers.hash(state);
        self.ensemble_owner_namespaces.hash(state);
        self.ensemble_dispatches.hash(state);
        self.unknown_export_namespaces.hash(state);
        self.packages.hash(state);
        self.revoked_loaders.hash(state);
        self.loader_handler_unknown.hash(state);
        let mut boundary_entries: Vec<_> = self.root_boundary_bindings.iter().collect();
        boundary_entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
        boundary_entries.hash(state);
        self.baseline.hash(state);
        self.opaque_domain.hash(state);
        self.opaque_binding_mutation.hash(state);
        self.dynamic_proc_binding.hash(state);
        self.unnameable_rebinding_subject.hash(state);
        self.unclosed_command_world.hash(state);
        self.diagnostic_command_world_closed.hash(state);
        self.namespace_resolution.hash(state);
        self.procedure_bodies.hash(state);
        self.rebound_names.hash(state);
        self.proc_rebound_names.hash(state);
        self.has_redefined_procedures.hash(state);
        self.source_order_user_call_effects.hash(state);
    }
}

impl ModuleCommandBindings {
    /// Widen both command resolution and the optimiser's command-trust
    /// projection because an unbounded command-binding effect may occur.
    /// Equality of the command world, independent of variable frames and
    /// source-position bookkeeping.
    pub(in super::super) fn same_command_world(&self, other: &Self) -> bool {
        self.bindings == other.bindings
            && self.objects == other.objects
            && self.namespace_exports == other.namespace_exports
            && self.namespace_paths == other.namespace_paths
            && self.namespaces == other.namespaces
            && self.unknown_lookup_namespaces == other.unknown_lookup_namespaces
            && self.unknown_namespace_paths == other.unknown_namespace_paths
            && self.namespace_unknown_handlers == other.namespace_unknown_handlers
            && self.ensemble_owner_namespaces == other.ensemble_owner_namespaces
            && self.ensemble_dispatches == other.ensemble_dispatches
            && self.unknown_export_namespaces == other.unknown_export_namespaces
            && self.packages == other.packages
            && self.revoked_loaders == other.revoked_loaders
            && self.loader_handler_unknown == other.loader_handler_unknown
            && self.stock_tclsh_stdout == other.stock_tclsh_stdout
            && self.class_definitions == other.class_definitions
            && self.object_instances == other.object_instances
            && self.bounded_constructions == other.bounded_constructions
            && self.original_command_world == other.original_command_world
            && self.opaque_domain == other.opaque_domain
            && self.opaque_binding_mutation == other.opaque_binding_mutation
            && self.dynamic_proc_binding == other.dynamic_proc_binding
            && self.unnameable_rebinding_subject == other.unnameable_rebinding_subject
            && self.unclosed_command_world == other.unclosed_command_world
            && self.diagnostic_command_world_closed == other.diagnostic_command_world_closed
    }
}
