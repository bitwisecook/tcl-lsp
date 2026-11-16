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

//! The binding a recorded point or a retained lookup state projects for a command word.

use super::command_identity::CommandAllocationSite;
use super::compiled_invocation::{CompiledExecutionResidual, NativeInlineRejection};
use super::compiler_inventory::SourceRuntimeReachability;
use super::invocation_binding::{
    OriginalArgumentCompletion, SourceCompilerNameClosure, SourceObserverExecution,
};
use super::namespace_context::{NamespaceKeyQuery, SourceNamespaceKey};
use super::{
    MayBinding, ModuleCommandBindings, PointBindingMemo, ReceiverNamespaceContents,
    SourceBindingPoint, SourceCommandTarget, SourceInvocationBinding, SourceLookupSnapshot,
    logical_prefix, original_command_table,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

impl Clone for PointBindingMemo {
    fn clone(&self) -> Self {
        Self::default()
    }
}

/// Whether a point was recorded by a walk of a body the source walk proves
/// never entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PointWalkEntry {
    Entered,
    NeverEntered,
}

impl PointWalkEntry {
    pub(super) const fn at_depth(unreached_walk_depth: usize) -> Self {
        if unreached_walk_depth == 0 {
            Self::Entered
        } else {
            Self::NeverEntered
        }
    }
}

// Derived lookup caches do not change the retained source interpretation.
impl PartialEq for SourceBindingPoint {
    fn eq(&self, other: &Self) -> bool {
        self.declaration_preview == other.declaration_preview
            && self.original_arguments_complete_normally
                == other.original_arguments_complete_normally
            && self.observer_execution == other.observer_execution
            && self.realm == other.realm
            && self.compilation == other.compilation
            && self.compiled_execution == other.compiled_execution
            && self.head == other.head
            && self.original_head_input == other.original_head_input
            && self.original_written_words == other.original_written_words
            && self.evaluated_argument_values == other.evaluated_argument_values
            && self.evaluated_argument_words == other.evaluated_argument_words
            && self.frozen_written_words == other.frozen_written_words
            && self.frozen_written_names == other.frozen_written_names
            && self.frozen_head_object == other.frozen_head_object
            && self.method_prefix_arguments == other.method_prefix_arguments
            && self.offset == other.offset
            && self.end == other.end
            && self.namespace_key == other.namespace_key
            && self.state == other.state
            && self.dispatch == other.dispatch
            && self.continuation_of == other.continuation_of
    }
}

impl Eq for SourceBindingPoint {}

impl SourceBindingPoint {
    pub(super) fn lookup_binding(&self, head: &str) -> SourceInvocationBinding {
        let mut proof = if self.dispatch && self.head.as_deref() == Some(head) {
            self.original_lookup_binding()
        } else {
            source_binding_projection(&self.state, head, &self.namespace_key)
        };
        proof.compiled_execution_residual = CompiledExecutionResidual::Unknown;
        proof.lookup_state = Some(Arc::clone(self.lookup_snapshot.get_or_init(|| {
            Arc::new(SourceLookupSnapshot::in_realm(
                self.state.clone(),
                self.realm,
            ))
        })));
        proof
    }

    pub(super) fn binding(&self, head: &str) -> SourceInvocationBinding {
        self.binding_projection(head, false)
    }

    pub(super) fn original_logical_lookup_head(&self) -> Option<&str> {
        let input = self.state.logical_source_name_advice_input()?;
        // The complete source owner remains independent of Native head input.
        // A withdrawn or different supplied owner cannot reopen this projection.
        if self.state.baseline.metadata_context.source_analysis_input() != Some(input) {
            return None;
        }
        self.head.as_deref()
    }

    fn original_lookup_binding(&self) -> SourceInvocationBinding {
        // naming.consumer.original-workspace-diagnostic-refinement
        // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
        // The immutable Logical entry owns this interpreted head. It does
        // not supply a Native name input or permit a source-label fallback.
        if let Some(head) = self.original_logical_lookup_head() {
            return source_binding_projection(&self.state, head, &self.namespace_key);
        }
        self.original_head_input
            .as_ref()
            .and_then(|input| {
                source_binding_from_original_input(&self.state, input, &self.namespace_key)
            })
            .unwrap_or_else(|| SourceInvocationBinding {
                unknown: true,
                lookup_namespace: self.namespace.clone(),
                lookup_namespace_key: self.namespace_key.clone(),
                variable_context: Arc::clone(&self.state.source_variables),
                variable_frame: self.state.variable_frame.clone(),
                ..Default::default()
            })
    }

    pub(super) fn original_dispatch_binding(&self) -> SourceInvocationBinding {
        self.original_binding
            .0
            .get_or_init(|| self.binding_projection("", true))
            .clone()
    }

    /// Drop the derived projections after an in-place change to the point.
    pub(super) fn invalidate_derived(&mut self) {
        self.lookup_snapshot = OnceLock::new();
        self.original_binding = PointBindingMemo::default();
    }

    fn binding_projection(&self, head: &str, original: bool) -> SourceInvocationBinding {
        let mut proof = if original {
            self.original_lookup_binding()
        } else {
            self.lookup_binding(head)
        };
        if original {
            proof.lookup_state = Some(Arc::clone(self.lookup_snapshot.get_or_init(|| {
                Arc::new(SourceLookupSnapshot::in_realm(
                    self.state.clone(),
                    self.realm,
                ))
            })));
        }
        if self.dispatch {
            proof.dispatch_site =
                self.state
                    .current_source_origin
                    .as_ref()
                    .map(|source| CommandAllocationSite {
                        source: Arc::clone(source),
                        offset: self.offset,
                    });
        }
        if self.dispatch && (original || self.head.as_deref() == Some(head)) {
            proof.original_argument_completion =
                OriginalArgumentCompletion::from_normal(self.original_arguments_complete_normally);
            proof
                .method_prefix_arguments
                .clone_from(&self.method_prefix_arguments);
            proof.entered_execution_observer = self.observer_execution;
            // A retained Logical source continuation is conditional even at
            // the root. Its interpreted target supplies no entered handler.
            proof.runtime_reachability = if self.walk_entry == PointWalkEntry::NeverEntered {
                SourceRuntimeReachability::NotEntered
            } else if self.declaration_preview || self.original_logical_lookup_head().is_some() {
                SourceRuntimeReachability::Conditional
            } else {
                SourceRuntimeReachability::Reached
            };
            proof
                .frozen_written_words
                .clone_from(&self.frozen_written_words);
            proof
                .frozen_written_names
                .clone_from(&self.frozen_written_names);
            proof
                .original_written_words
                .clone_from(&self.original_written_words);
            proof
                .compiled_candidates
                .clone_from(&self.compiled_execution.proofs);
            proof.compiled_named_candidates = self
                .compiled_execution
                .named
                .iter()
                .map(|candidate| candidate.with_late_lookup(&self.state))
                .collect();
            proof.may_use_live_dispatch = self.compiled_execution.live;
            proof.compiled_execution_residual = self.compiled_execution.unknown.into();
            proof.native_compilation_admission = self.compiled_execution.admission;
            proof.original_compiler_preserves_names =
                self.compiled_execution.compiler_preserves_names().into();
            proof
                .native_operand_layout
                .clone_from(&self.compiled_execution.operand_layout);
            proof
                .native_compiler_admission
                .clone_from(&self.compiled_execution.admitted);
            proof
                .native_structured_preparation
                .clone_from(&self.compiled_execution.structured);
            proof
                .native_switch_preparation
                .clone_from(&self.compiled_execution.switch);
            proof
                .compiler_policy
                .clone_from(&self.compiled_execution.policy);
            proof
                .original_compiler_words
                .clone_from(&self.compiled_execution.original_words);
            proof.native_inline_rejection = self.compiled_execution.rejection;
            proof.native_handler_envelope = self
                .compiled_execution
                .stable_handler_after_original_arguments(
                    self.original_head_input.as_ref(),
                    &self.state,
                    &self.namespace_key,
                )
                .cloned();
        }
        proof
    }
}

/// Materialise the complete dispatch carrier before a recursive target walk.
#[inline(never)]
/// Lookup the authoritative command table from a current original producer.
/// Unicode metadata is presentation; it never reconstructs the selected slot.
pub(super) fn source_binding_from_original_input(
    state: &ModuleCommandBindings,
    head: &crate::signature_scan::scope::SignatureSourceNameInput,
    namespace: &SourceNamespaceKey,
) -> Option<SourceInvocationBinding> {
    source_binding_from_original_input_in(state, head, namespace, CommandTargetLookup::WithFallback)
}

/// Keep the complete original binding outside frames retained by nested bodies.
#[inline(never)]
pub(super) fn source_binding_from_original_input_boxed(
    state: &ModuleCommandBindings,
    head: &crate::signature_scan::scope::SignatureSourceNameInput,
    namespace: &SourceNamespaceKey,
) -> Option<Box<SourceInvocationBinding>> {
    source_binding_from_original_input(state, head, namespace).map(Box::new)
}

pub(super) fn source_binding_from_original_input_in(
    state: &ModuleCommandBindings,
    head: &crate::signature_scan::scope::SignatureSourceNameInput,
    namespace: &SourceNamespaceKey,
    lookup: CommandTargetLookup,
) -> Option<SourceInvocationBinding> {
    #[cfg(test)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
        eprintln!(
            "ORIGINAL_COMMAND_BINDING input={:?} current={} namespace={:?} origin={} variable_policy={:?} baseline_policy={:?} opaque={}",
            head.bytes(),
            head.is_current(&state.source_variables),
            namespace,
            state.current_source_origin.is_some(),
            state.source_variables.execution_name_policy,
            state.baseline.execution_name_policy,
            state.has_opaque_domain()
        );
    }
    head.is_current(&state.source_variables).then_some(())?;
    let (declared_command, catalogue_command) =
        state.original_advice_candidates_for_input(namespace, head);
    let selected = state
        .original_targets_for_input(head, namespace, lookup)
        .or_else(|| {
            (declared_command.is_some() || catalogue_command.is_some()).then(|| {
                original_command_table::OriginalCommandTargetSelection {
                    targets: BTreeSet::new(),
                    original_prefixes: BTreeMap::new(),
                    may_be_absent: true,
                    unknown: true,
                }
            })
        })?;
    Some(SourceInvocationBinding {
        declared_command,
        catalogue_command,
        lookup_state: Some(Arc::new(SourceLookupSnapshot::new(state.clone()))),
        lookup_namespace: namespace.display().unwrap_or_default(),
        lookup_namespace_key: namespace.clone(),
        lookup_word: std::str::from_utf8(head.bytes()).ok().map(str::to_owned),
        targets: selected
            .targets
            .into_iter()
            .map(|target| {
                let original_prepended = (!target.prepended.is_empty())
                    .then(|| selected.original_prefixes.get(&target).cloned().flatten())
                    .flatten();
                SourceCommandTarget {
                    runtime_implementation_generation: state
                        .runtime_implementation_generation(target.token.as_ref()),
                    command: target.command,
                    prepended: target.prepended,
                    original_prepended,
                    logical_original_prefix: None,
                    registry_backed: target.registry_backed,
                    kind: target.kind,
                    identity: target.token,
                    implementation_generation: target.implementation_generation,
                    implementation_allocation: target.implementation_allocation,
                }
            })
            .collect(),
        may_be_absent: selected.may_be_absent,
        unknown: selected.unknown,
        provider_state_taint: state.tainted_provider_state.iter().cloned().collect(),
        variable_context: Arc::clone(&state.source_variables),
        variable_frame: state.variable_frame.clone(),
        namespace_cell_presence: state.source_variables.namespace_cells.clone(),
        known_namespaces: state.namespaces.advisory_names(),
        ..SourceInvocationBinding::default()
    })
}

pub(super) fn source_binding(
    state: &ModuleCommandBindings,
    head: &str,
    namespace: &(impl NamespaceKeyQuery + ?Sized),
) -> SourceInvocationBinding {
    let mut projection = source_binding_projection(state, head, namespace);
    projection.lookup_state = Some(Arc::new(SourceLookupSnapshot::new(state.clone())));
    projection
}

#[derive(Clone, Copy)]
pub(super) enum CommandTargetLookup {
    WithFallback,
    NamedSlots,
    CompilerSlots,
}

pub(super) fn source_binding_projection(
    state: &ModuleCommandBindings,
    head: &str,
    namespace: &(impl NamespaceKeyQuery + ?Sized),
) -> SourceInvocationBinding {
    source_binding_projection_in(state, head, namespace, CommandTargetLookup::WithFallback)
}

fn source_binding_targets(
    state: &ModuleCommandBindings,
    selection: logical_prefix::LogicalPrefixResolution,
) -> Vec<SourceCommandTarget> {
    selection
        .targets
        .into_iter()
        .map(|target| {
            let logical_original_prefix = selection
                .prefixes
                .get(&target)
                .cloned()
                .flatten()
                .filter(|_| !target.prepended.is_empty());
            SourceCommandTarget {
                runtime_implementation_generation: state
                    .runtime_implementation_generation(target.token.as_ref()),
                command: target.command,
                prepended: target.prepended,
                original_prepended: None,
                logical_original_prefix,
                registry_backed: target.registry_backed,
                kind: target.kind,
                identity: target.token,
                implementation_generation: target.implementation_generation,
                implementation_allocation: target.implementation_allocation,
            }
        })
        .collect()
}

pub(super) fn source_binding_projection_in(
    state: &ModuleCommandBindings,
    head: &str,
    namespace: &(impl NamespaceKeyQuery + ?Sized),
    lookup: CommandTargetLookup,
) -> SourceInvocationBinding {
    let namespace_key = namespace.namespace_key();
    let namespace_display = namespace_key.display().unwrap_or_default();
    let (declared_command, catalogue_command) =
        state.original_advice_candidates_for_metadata(namespace_key.as_ref(), head);
    let keys = state.source_keys(head, namespace);
    let selection =
        state.resolve_targets_with_prefixes_in(head, namespace, &mut BTreeSet::new(), lookup);
    let may_be_absent = keys.last().is_none_or(|key| {
        state
            .binding_alternatives(key)
            .contains(&MayBinding::Missing)
    });
    SourceInvocationBinding {
        compiled_candidates: Vec::new(),
        compiled_named_candidates: Vec::new(),
        may_use_live_dispatch: true,
        compiled_execution_residual: CompiledExecutionResidual::Closed,
        native_compilation_admission: None,
        native_compiler_admission: None,
        original_compiler_preserves_names: SourceCompilerNameClosure::Unknown,
        native_structured_preparation: None,
        native_switch_preparation: None,
        native_operand_layout: None,
        runtime_reachability: SourceRuntimeReachability::Unknown,
        entered_execution_observer: SourceObserverExecution::Unobserved,
        native_inline_rejection: NativeInlineRejection::None,
        native_handler_envelope: None,
        invocation_variable_reads: None,
        lookup_state: None,
        compiler_lookup_state: None,
        original_logical_source_advice: None,
        original_authored_source_body_scope: None,
        original_analysis_owner: None,
        original_source_operand_attachment: None,
        original_argument_syntax: None,
        original_native_source_command: None,
        declaration_layout_observations: None,
        declaration_flow_inventory: None,
        original_logical_declaration_targets: None,
        compiler_policy: None,
        original_compiler_words: None,
        original_written_words: None,
        conditional_expression_evaluations: None,
        receiver_namespace: ReceiverNamespaceContents::Created,
        lookup_namespace: namespace_display.clone(),
        lookup_namespace_key: namespace_key.into_owned(),
        lookup_word: Some(head.to_owned()),
        dispatch_site: None,
        provider_state_taint: state.tainted_provider_state.iter().cloned().collect(),
        declared_command,
        catalogue_command,
        variable_context: Arc::clone(&state.source_variables),
        original_argument_completion: OriginalArgumentCompletion::Unknown,
        normal_variable_continuation: None,
        original_variable_operands: None,
        normal_result_observations: None,
        rhs_read_store_observations: None,
        object_callback_effects: None,
        variable_frame: state.variable_frame.clone(),
        existing_namespace_cells: {
            let mut cells = state
                .source_variables
                .namespace_cells
                .present
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            cells.sort_by_cached_key(|key| format!("{key:?}"));
            cells
        },
        namespace_cell_presence: state.source_variables.namespace_cells.clone(),
        known_namespaces: state.namespaces.advisory_names(),
        evaluated_argument_values: Arc::default(),
        evaluated_argument_words: Arc::default(),
        frozen_written_words: None,
        frozen_written_names: None,
        frozen_head_object: None,
        method_prefix_arguments: Vec::new(),
        targets: source_binding_targets(state, selection),
        may_be_absent,
        unknown: state.target_may_be_unknown(head, namespace)
            || (!tcl_syntax::naming::is_absolute(head)
                && namespace.namespace_key().native_context().is_none()
                && !state.source_variables.namespace_known
                && state.receiver_command_lookup_path(head).is_none()),
    }
}

pub(super) fn command_holder(command: &str) -> String {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(command);
    if holder.is_empty() {
        "::".to_owned()
    } else {
        holder.to_owned()
    }
}
