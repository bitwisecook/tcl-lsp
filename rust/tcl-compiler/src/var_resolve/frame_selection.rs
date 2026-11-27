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

//! Selecting and entering activations: namespace and function frames, called frames and proved selected frames.

use super::{
    ActivationObserverInventory, BindingIdentity, ContentsWorld, ResolveContext, VariableFrameKind,
};
use tcl_names::variable::{SharedSet, VariableCellKey, VariableExecutionFrame};
use tcl_registry::FrameLevel;

impl ResolveContext {
    /// Namespace-variable frame, distinct from a procedure activation.
    #[must_use]
    pub fn for_namespace(namespace: impl Into<String>) -> Self {
        let namespace = namespace.into();
        Self {
            known_namespaces: SharedSet::from(["::".to_owned(), namespace.clone()]),
            namespace,
            frame_kind: VariableFrameKind::Namespace,
            binding_identity: BindingIdentity::Bound,
            ..Self::default()
        }
    }

    /// Project the actual selected activation into registry alias/completion policy.
    #[must_use]
    pub const fn alias_frame(&self) -> tcl_registry::VariableAliasFrame {
        match self.frame_kind {
            VariableFrameKind::Local => tcl_registry::VariableAliasFrame::Procedure,
            VariableFrameKind::Namespace => tcl_registry::VariableAliasFrame::Namespace,
            VariableFrameKind::Global => tcl_registry::VariableAliasFrame::Global,
            VariableFrameKind::Unknown => tcl_registry::VariableAliasFrame::Unknown,
        }
    }

    /// Entry environment for a compiler function. `::top` uses the global table.
    #[must_use]
    pub fn for_function(name: &str) -> Self {
        let namespace = tcl_syntax::naming::key_holder_and_tail(name).0;
        let namespace = if namespace.is_empty() {
            "::"
        } else {
            namespace
        };
        let mut context = Self::for_namespace(namespace);
        context.frame_kind = if name == "::top" {
            VariableFrameKind::Global
        } else {
            VariableFrameKind::Local
        };
        context.activation = (name != "::top").then(|| name.to_owned());
        context
    }

    /// Select a typed execution activation while preserving shared namespace facts.
    #[must_use]
    pub fn in_frame(&self, frame: &VariableExecutionFrame) -> Self {
        if let VariableExecutionFrame::NamespaceIdentity { namespace, frame } = frame {
            let mut selected = self.in_frame(frame);
            selected.retain_namespace_world(
                namespace.clone(),
                self.namespace_identities.iter().cloned(),
                self.namespace_name_protocol,
            );
            return selected;
        }
        if let VariableExecutionFrame::Selected { selector, .. } = frame
            && let Some(mut selected) = self.selected_frame_context(*selector)
        {
            selected.source_read_configuration = self.source_read_configuration;
            return selected;
        }
        let mut selected = match frame {
            VariableExecutionFrame::Global => Self::for_function("::top"),
            VariableExecutionFrame::Namespace(namespace) => Self::for_namespace(namespace.clone()),
            VariableExecutionFrame::NamespaceActivation {
                namespace,
                identity,
            } => {
                let mut selected = Self::for_namespace(namespace.clone());
                selected.activation = Some(identity.clone());
                selected
            }
            VariableExecutionFrame::Procedure {
                namespace,
                identity,
            } => {
                let mut selected = Self::for_namespace(namespace.clone());
                selected.frame_kind = VariableFrameKind::Local;
                selected.activation = Some(identity.clone());
                selected
            }
            VariableExecutionFrame::ReceiverMethod { identity } => Self {
                namespace_known: false,
                frame_kind: VariableFrameKind::Local,
                activation: Some(identity.clone()),
                binding_identity: BindingIdentity::Bound,
                ..Self::default()
            },
            VariableExecutionFrame::Selected { selector, .. } if selector.is_current_frame() => {
                return self.clone();
            }
            VariableExecutionFrame::Selected { selector, .. } if selector.is_global_frame() => {
                Self::for_function("::top")
            }
            VariableExecutionFrame::Selected {
                selector,
                namespace,
            } => Self {
                namespace: namespace.clone().unwrap_or_else(|| "::".to_owned()),
                namespace_known: namespace.is_some(),
                selected_frame: Some(*selector),
                frame_kind: VariableFrameKind::Unknown,
                binding_identity: BindingIdentity::Bound,
                ..Self::default()
            },
            VariableExecutionFrame::NamespaceIdentity { .. } => {
                unreachable!("identity wrapper handled above")
            }
            VariableExecutionFrame::Unknown => Self {
                frame_kind: VariableFrameKind::Unknown,
                namespace_known: false,
                dynamic_bindings: true,
                binding_identity: BindingIdentity::Bound,
                ..Self::default()
            },
        };
        self.inherit_namespace_world(&mut selected);
        selected
    }

    fn inherit_namespace_world(&self, selected: &mut Self) {
        let instance_keys = crate::allocated_instance::outward_cell_keys(self);
        let outward = |key: &VariableCellKey| {
            instance_keys.contains(key)
                || match key.root() {
                    VariableCellKey::Namespace { .. } | VariableCellKey::RetainedSlot(_) => true,
                    VariableCellKey::Authored(name) => tcl_syntax::naming::is_absolute(name),
                    _ => false,
                }
        };
        selected
            .namespace_identities
            .clone_from(&self.namespace_identities);
        selected
            .namespace_addressable_identities
            .clone_from(&self.namespace_addressable_identities);
        selected.namespace_inventory = self.namespace_inventory;
        selected.namespace_name_protocol = self.namespace_name_protocol;
        selected.execution_name_policy = self.execution_name_policy;
        selected.expression_parser = self.expression_parser;
        selected.hosted_execution_context = self.hosted_execution_context;
        selected
            .namespace_objects
            .clone_from(&self.namespace_objects);
        selected
            .pending_namespace_retirements
            .clone_from(&self.pending_namespace_retirements);
        selected
            .authored_tmm_static
            .clone_from(&self.authored_tmm_static);
        if selected.global_frame() {
            selected.namespace_identity = self
                .namespace_identities
                .iter()
                .find(|key| {
                    key.exact_native_path()
                        .is_some_and(tcl_core_types::ByteNamespacePath::is_root)
                })
                .cloned();
        }
        selected
            .closed_observer_allocations
            .clone_from(&self.closed_observer_allocations);
        selected.raw_bindings.clone_from(&self.raw_bindings);
        selected.captured_cells.clone_from(&self.captured_cells);
        selected
            .possible_trace_registrations
            .clone_from(&self.possible_trace_registrations);
        selected
            .namespace_alias_bindings
            .clone_from(&self.namespace_alias_bindings);
        selected
            .namespace_name_alias_bindings
            .clone_from(&self.namespace_name_alias_bindings);
        selected.generations.extend(
            self.generations
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, generation)| (key.clone(), *generation)),
        );
        selected
            .traced
            .extend(self.traced.iter().filter(|key| outward(key)).cloned());
        selected.trace_registrations.extend(
            self.trace_registrations
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, registrations)| (key.clone(), registrations.clone())),
        );
        selected.trace_registration_receivers.extend(
            self.trace_registration_receivers
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, value)| (key.clone(), std::sync::Arc::clone(value))),
        );
        selected.untracked_traces.extend(
            self.untracked_traces
                .iter()
                .filter(|key| outward(key))
                .cloned(),
        );
        selected.dynamic_traces = self.dynamic_traces;
        selected.caller.clone_from(&self.caller);
        selected
            .value_representations
            .clone_from(&self.value_representations);
        selected.original_contents_epoch = self.original_contents_epoch;
        selected.original_contents_epoch_high_water = self.original_contents_epoch_high_water;
        selected.representation_epoch = self.representation_epoch;
        selected.representation_epoch_high_water = self.representation_epoch_high_water;
        self.inherit_outward_contents(selected, outward);
    }

    fn inherit_outward_contents(
        &self,
        selected: &mut Self,
        outward: impl Fn(&VariableCellKey) -> bool,
    ) {
        selected.constant_values.extend(
            self.constant_values
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        selected.closed_literal_contents.extend(
            self.closed_literal_contents
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        selected.original_name_values.extend(
            self.original_name_values
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        selected.contents_presence.extend(
            self.contents_presence
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, &presence)| (key.clone(), presence)),
        );
        selected.contents_presence_slots.extend(
            self.contents_presence_slots
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, place)| (key.clone(), std::sync::Arc::clone(place))),
        );
        selected
            .contents_source_proofs
            .restore(&self.contents_source_proofs, &|key| outward(key));
        selected.contents_origins.extend(
            self.contents_origins
                .iter()
                .filter(|(key, _)| outward(key))
                .map(|(key, origin)| (key.clone(), origin.clone())),
        );
        selected.contents_world = self.contents_world;
        selected
            .contents_unknown_namespaces
            .clone_from(&self.contents_unknown_namespaces);
        selected
            .contents_unknown_arrays
            .clone_from(&self.contents_unknown_arrays);
        selected
            .closed_array_roots
            .clone_from(&self.closed_array_roots);
        selected.contents_kinds.clone_from(&self.contents_kinds);
        selected.invocation_dialect = self.invocation_dialect;
        selected.source_read_configuration = self.source_read_configuration;
        selected.namespace_cells.clone_from(&self.namespace_cells);
        selected
            .pending_namespace_retirements
            .clone_from(&self.pending_namespace_retirements);
        selected.known_namespaces.clone_from(&self.known_namespaces);
        selected
            .namespace_identities
            .clone_from(&self.namespace_identities);
        selected
            .namespace_addressable_identities
            .clone_from(&self.namespace_addressable_identities);
        selected.namespace_inventory = self.namespace_inventory;
        selected
            .authored_tmm_static
            .clone_from(&self.authored_tmm_static);
        selected.interpreter.clone_from(&self.interpreter);
        selected.hosted_execution_context = self.hosted_execution_context;
        selected.execution = self.execution;
    }

    /// Enter an actually invoked frame, retaining its selected caller identity.
    #[must_use]
    pub fn enter_called_frame(&self, frame: &VariableExecutionFrame) -> Self {
        let mut selected = self.in_frame(frame);
        if matches!(
            frame.layout(),
            VariableExecutionFrame::Global | VariableExecutionFrame::Selected { .. }
        ) {
            return selected;
        }
        selected.caller = Some(std::sync::Arc::new(self.clone()));
        if matches!(
            frame.layout(),
            VariableExecutionFrame::Procedure { .. }
                | VariableExecutionFrame::ReceiverMethod { .. }
        ) {
            selected.activation_contents_world = Some(ContentsWorld::Tracked);
            selected.activation_observer_inventory = ActivationObserverInventory::Closed;
        }
        selected.constant_values.clone_from(&self.constant_values);
        selected
            .original_name_values
            .clone_from(&self.original_name_values);
        selected
            .closed_literal_contents
            .clone_from(&self.closed_literal_contents);
        selected
            .value_representations
            .clone_from(&self.value_representations);
        selected.original_contents_epoch = self.original_contents_epoch;
        selected.original_contents_epoch_high_water = self.original_contents_epoch_high_water;
        selected.representation_epoch = self.representation_epoch;
        selected.representation_epoch_high_water = self.representation_epoch_high_water;
        selected.contents_origins.clone_from(&self.contents_origins);
        selected
            .contents_source_proofs
            .clone_from(&self.contents_source_proofs);
        selected
            .contents_presence
            .clone_from(&self.contents_presence);
        selected
            .contents_presence_slots
            .clone_from(&self.contents_presence_slots);
        selected.contents_world = self.contents_world;
        selected
            .contents_unknown_namespaces
            .clone_from(&self.contents_unknown_namespaces);
        selected
            .contents_unknown_arrays
            .clone_from(&self.contents_unknown_arrays);
        selected
            .closed_array_roots
            .clone_from(&self.closed_array_roots);
        selected.contents_kinds.clone_from(&self.contents_kinds);
        selected.generations.clone_from(&self.generations);
        selected.traced.clone_from(&self.traced);
        selected
            .trace_registrations
            .clone_from(&self.trace_registrations);
        selected
            .trace_registration_receivers
            .clone_from(&self.trace_registration_receivers);
        selected.untracked_traces.clone_from(&self.untracked_traces);
        selected
    }

    /// Select a proved actual frame; unknown call depth never invents one.
    #[must_use]
    pub fn selected_frame_context(&self, level: FrameLevel) -> Option<Self> {
        let distance = match level {
            FrameLevel::Relative(distance) => distance,
            FrameLevel::Absolute(target) => {
                let mut current = self;
                let mut depth = 0_u32;
                while !current.global_frame() {
                    current = current.caller.as_deref()?;
                    depth = depth.checked_add(1)?;
                }
                depth.checked_sub(target)?
            }
            FrameLevel::Dynamic => return None,
        };
        let mut current = self;
        for _ in 0..distance {
            if current.global_frame() {
                return None;
            }
            current = current.caller.as_deref()?;
        }
        let mut selected = current.clone();
        selected.source_read_configuration = self.source_read_configuration;
        selected.captured_cells.clone_from(&self.captured_cells);
        selected
            .possible_trace_registrations
            .clone_from(&self.possible_trace_registrations);
        selected.namespace_cells.clone_from(&self.namespace_cells);
        selected
            .namespace_alias_bindings
            .clone_from(&self.namespace_alias_bindings);
        selected
            .namespace_name_alias_bindings
            .clone_from(&self.namespace_name_alias_bindings);
        selected.known_namespaces.clone_from(&self.known_namespaces);
        selected.generations.extend(
            self.generations
                .iter()
                .map(|(key, generation)| (key.clone(), *generation)),
        );
        selected.constant_values.clone_from(&self.constant_values);
        selected
            .original_name_values
            .clone_from(&self.original_name_values);
        selected
            .closed_literal_contents
            .clone_from(&self.closed_literal_contents);
        selected
            .value_representations
            .clone_from(&self.value_representations);
        selected.original_contents_epoch = self.original_contents_epoch;
        selected.original_contents_epoch_high_water = self.original_contents_epoch_high_water;
        selected.representation_epoch = self.representation_epoch;
        selected.representation_epoch_high_water = self.representation_epoch_high_water;
        selected.contents_origins.clone_from(&self.contents_origins);
        selected
            .contents_source_proofs
            .clone_from(&self.contents_source_proofs);
        selected
            .contents_presence
            .clone_from(&self.contents_presence);
        selected
            .contents_presence_slots
            .clone_from(&self.contents_presence_slots);
        selected.contents_world = self.contents_world;
        selected
            .contents_unknown_namespaces
            .clone_from(&self.contents_unknown_namespaces);
        selected
            .contents_unknown_arrays
            .clone_from(&self.contents_unknown_arrays);
        selected
            .closed_array_roots
            .clone_from(&self.closed_array_roots);
        selected.contents_kinds.clone_from(&self.contents_kinds);
        selected.traced.clone_from(&self.traced);
        selected
            .trace_registrations
            .clone_from(&self.trace_registrations);
        selected
            .trace_registration_receivers
            .clone_from(&self.trace_registration_receivers);
        selected.untracked_traces.clone_from(&self.untracked_traces);
        selected.dynamic_traces = self.dynamic_traces;
        Some(selected)
    }
}
