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

//! Declared bodies interpreted after the document: entry states, walks and retained outcomes.

#[cfg(any(test, debug_assertions))]
use super::root_owner_work;
use super::shared_inventory::SharedSourceInventory;
use super::source_origin::source_activation_name;
use super::source_walk::{SourceBodyEntryPurpose, boxed_source_branch};
use super::{
    DeferredImplementationId, DeferredSourceBody, ModuleCommandBindings, SourceCommandBindings,
    SourceDeferredOutcome, SourceExecutionContext, SourceOutcomes,
};
use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};
use tcl_registry::CommandRegistry;

impl DeferredSourceBody {
    pub(super) fn implementation_id(&self) -> DeferredImplementationId {
        DeferredImplementationId {
            command: self.identity.clone(),
            generation: self.implementation_generation,
            allocation: self.implementation_allocation.clone(),
        }
    }
}

pub(super) fn seed_deferred_inputs(
    branch: &mut ModuleCommandBindings,
    root: &DeferredSourceBody,
    frame: &crate::var_resolve::VariableExecutionFrame,
    registry: &CommandRegistry,
) {
    #[cfg(any(test, debug_assertions))]
    let _timing = root_owner_work::Timing::at(root_owner_work::Query::Frame);
    if root.event.is_none() {
        branch.source_variables = Arc::new(branch.source_variables.in_frame(frame));
        branch.variable_frame = frame.clone();
        Arc::make_mut(&mut branch.source_variables).widen_deferred_namespace_inputs();
        Arc::make_mut(&mut branch.source_variables).restart_representation_world();
        branch.seed_declared_receiver_variables(root);
        if let Some(statics) = &root.statics {
            Arc::make_mut(&mut branch.source_variables).install_callable_statics(statics, registry);
        }
        if let Some(topology) = &root.original_parameters {
            topology.seed_unknown(Arc::make_mut(&mut branch.source_variables), registry);
        } else if let Some(grammar) = branch
            .source_variables
            .invocation_dialect
            .and_then(tcl_native_model::InvocationDialect::parameter_grammar)
            && let Ok(parameters) = tcl_syntax::formal_params::bind_formal_arguments(
                &root.parameters,
                root.parameters.len(),
                grammar,
            )
        {
            use tcl_syntax::formal_params::FormalArgumentBinding as Binding;
            for parameter in parameters {
                match parameter {
                    Binding::Value { parameter, .. } | Binding::Default { parameter } => {
                        Arc::make_mut(&mut branch.source_variables)
                            .bind_unknown_incoming(&root.parameters[parameter].name, registry);
                    }
                    Binding::Rest { name, .. } => {
                        Arc::make_mut(&mut branch.source_variables)
                            .bind_unknown_incoming(&name, registry);
                    }
                    Binding::CallerLink { name, .. } => {
                        crate::variable_bindings::bind_caller_reference(
                            Arc::make_mut(&mut branch.source_variables),
                            &name,
                            None,
                            registry,
                        );
                    }
                }
            }
        }
    }
}

fn deferred_source_frame(
    root: &DeferredSourceBody,
    branch: &mut ModuleCommandBindings,
    parent: &crate::var_resolve::ResolveContext,
) -> crate::var_resolve::VariableExecutionFrame {
    use crate::var_resolve::VariableExecutionFrame;
    let frame = if root.receiver_method {
        VariableExecutionFrame::ReceiverMethod {
            identity: source_activation_name(
                root.source_origin.as_ref(),
                &root.identity,
                root.implementation_generation,
            ),
        }
    } else if let Some(event) = &root.event {
        let mut entry = crate::connection_scope::event_resolve_context(event);
        entry.invocation_dialect = branch.baseline.dialect;
        entry.expression_parser = branch.baseline.expression_parser;
        entry.source_read_configuration = parent.source_read_configuration;
        entry.namespace_cells = parent.namespace_cells.clone();
        let frame = if entry.dynamic_bindings {
            VariableExecutionFrame::Unknown
        } else if entry.global_frame() {
            VariableExecutionFrame::Global
        } else {
            VariableExecutionFrame::Procedure {
                namespace: root.namespace.clone(),
                identity: source_activation_name(
                    root.source_origin.as_ref(),
                    &root.identity,
                    root.implementation_generation,
                ),
            }
        };
        branch.source_variables = Arc::new(entry.in_frame(&frame));
        branch.variable_frame = frame.clone();
        frame
    } else {
        VariableExecutionFrame::Procedure {
            namespace: root.namespace.clone(),
            identity: source_activation_name(
                root.source_origin.as_ref(),
                &root.identity,
                root.implementation_generation,
            ),
        }
    };
    if root.receiver_method {
        frame
    } else {
        frame.with_namespace_identity(root.namespace_key.clone())
    }
}

#[cfg(any(test, debug_assertions))]
fn trace_source_deferred(
    started: Option<std::time::Instant>,
    stage: &str,
    bindings: &SourceCommandBindings,
) {
    if let Some(started) = started {
        eprintln!(
            "SOURCE_DEFERRED_PHASE stage={stage} ms={} roots={} outcomes={} layouts={}",
            started.elapsed().as_millis(),
            bindings.deferred.len(),
            bindings.deferred_outcomes.len(),
            bindings.declaration_layouts.len()
        );
    }
}

impl SourceCommandBindings {
    pub(super) fn analyse_deferred_entries(
        &mut self,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) {
        use crate::analysis_control::{AnalysisCheckpoint, checkpoint};
        #[cfg(any(test, debug_assertions))]
        let started = tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
            .is_some()
            .then(std::time::Instant::now);
        #[cfg(any(test, debug_assertions))]
        let mut last_report = started;
        #[cfg(any(test, debug_assertions))]
        let mut iteration = 0_usize;
        loop {
            if checkpoint(AnalysisCheckpoint::Fixpoint) {
                return;
            }
            let root_count = self.deferred.len();
            #[cfg(any(test, debug_assertions))]
            if let Some(started) = started {
                iteration += 1;
                eprintln!(
                    "SOURCE_DEFERRED_PROGRESS stage=select iteration={iteration} ms={} roots={root_count}",
                    started.elapsed().as_millis()
                );
            }
            for (root_index, (root, entry)) in
                self.deferred_entry_states(incoming).into_iter().enumerate()
            {
                #[cfg(not(any(test, debug_assertions)))]
                let _ = root_index;
                #[cfg(any(test, debug_assertions))]
                if let Some(started) = started
                    && (root_index == 0
                        || last_report.is_some_and(|last| last.elapsed().as_secs() >= 1))
                {
                    eprintln!(
                        "SOURCE_DEFERRED_PROGRESS stage=entry iteration={iteration} index={root_index} ms={} roots={root_count} bytes={} offset={} outcomes={} layouts={}",
                        started.elapsed().as_millis(),
                        root.source.len(),
                        root.offset,
                        self.deferred_outcomes.len(),
                        self.declaration_layouts.len()
                    );
                    root_owner_work::report("deferred-entry");
                    last_report = Some(std::time::Instant::now());
                }
                if checkpoint(AnalysisCheckpoint::Body) {
                    return;
                }
                if self
                    .called_implementations
                    .contains(&root.implementation_id())
                {
                    continue;
                }
                let after_load = self.uncalled_body_after_load(&root, &entry);
                self.walk_deferred_entry_with(&root, &entry, config, registry, after_load);
            }
            if root_count == self.deferred.len() {
                break;
            }
        }
        #[cfg(any(test, debug_assertions))]
        trace_source_deferred(started, "entries", self);
        self.analyse_called_declaration_results(incoming, config, registry);
        self.retain_called_receiver_source_catalogues(incoming, config, registry);
        #[cfg(any(test, debug_assertions))]
        trace_source_deferred(started, "called", self);
        self.retain_uninstalled_declaration_body_layouts(registry);
        self.analyse_logical_declared_body_entries(incoming, config, registry);
        self.retain_called_static_declaration_layouts(incoming, config, registry);
        #[cfg(any(test, debug_assertions))]
        trace_source_deferred(started, "layouts", self);
    }

    fn analyse_called_declaration_results(
        &mut self,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) {
        let entries = self
            .deferred_entry_states(incoming)
            .into_iter()
            .filter(|(root, _)| {
                self.called_implementations
                    .contains(&root.implementation_id())
                    && !root.receiver_method
                    && root.event.is_none()
                    && root.future_frame.is_none()
                    && root.statics.is_none()
            })
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return;
        }
        // The separate collector retains the original declaration and lookup
        // environment. seed_deferred_inputs installs unknown formals and
        // widens namespace contents before the shared body walker runs.
        // Its dispatch/read/compiler observations cannot become actual calls.
        let mut conditional = self.clone();
        conditional.conditional_procedure_results = SharedSourceInventory::default();
        conditional.active_calls.clear();
        conditional.active_variable_observers.clear();
        let mut declared = SourceCommandBindings {
            module_projection_memo: super::retained_source::ModuleProjectionMemo::default(),
            declaration_flow_cache: super::declaration_flow::SourceDeclarationFlowCache::default(),
            derived_indexes: super::shared_inventory::DerivedIndexes::default(),
            ..conditional.clone()
        };
        for (root, entry) in &entries {
            conditional.walk_deferred_entry(root, entry, config, registry);
        }
        self.merge_original_declaration_layouts(&conditional);
        self.merge_conditional_procedure_results(&conditional);
        // A separate collector enters each called body as an uncalled one is
        // entered, after the document has loaded, so its declared-frame
        // points carry the same selected bindings; only a declared-frame
        // view of the bindings reads them.
        for (root, entry) in &entries {
            let after_load = declared.uncalled_body_after_load(root, entry);
            declared.walk_deferred_entry_with(root, entry, config, registry, after_load);
        }
        self.declared_frame_walk = Some(Arc::new(declared));
    }

    pub(super) fn deferred_entry_states(
        &self,
        incoming: &ModuleCommandBindings,
    ) -> Vec<(DeferredSourceBody, Arc<ModuleCommandBindings>)> {
        let entry = Arc::new(incoming.clone());
        let mut entries = self
            .deferred_entry_bodies(incoming)
            .into_iter()
            .map(|root| (root, Arc::clone(&entry)))
            .collect::<Vec<_>>();
        let initial = entries
            .iter()
            .map(|(root, _)| root.implementation_id())
            .collect::<BTreeSet<_>>();
        if initial.len() == self.deferred.len() {
            return entries;
        }
        // A future invocation can install a new callable even when its final
        // completion is abrupt. Its exact outcome supplies that callable's
        // lookup table; the actual document continuation remains untouched.
        for outcome in &self.deferred_outcomes {
            if outcome.route
                == tcl_registry::completion_route::InvocationCompletionRoute::ProcessExit
            {
                continue;
            }
            for root in self.deferred_entry_bodies(&outcome.state) {
                if !initial.contains(&root.implementation_id())
                    && !entries.iter().any(|(known, state)| {
                        known.implementation_id() == root.implementation_id()
                            && known.namespace_key == root.namespace_key
                            && state.same_state(&outcome.state)
                    })
                {
                    entries.push((root, Arc::clone(&outcome.state)));
                }
            }
        }
        entries
    }

    pub(super) fn deferred_entry_bodies(
        &self,
        incoming: &ModuleCommandBindings,
    ) -> Vec<DeferredSourceBody> {
        let namespaces = OnceLock::new();
        self.deferred
            .values()
            .flat_map(|root| {
                if root.event.is_some() || root.receiver_method || root.future_frame.is_some() {
                    return vec![root.clone()];
                }
                namespaces
                    .get_or_init(|| {
                        self.deferred_namespace_index
                            .for_bindings(&incoming.bindings)
                    })
                    .for_implementation(&root.implementation_id())
                    .into_iter()
                    .flatten()
                    .cloned()
                    .map(|namespace_key| DeferredSourceBody {
                        namespace: namespace_key.display().unwrap_or_default(),
                        namespace_key,
                        ..root.clone()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }

    /// An uncalled procedure body is analysed as if called after the
    /// document finished running at top level (user decision
    /// 2026-10-21T08:20Z, "world after load"): its commands resolve in the
    /// procedure's namespace against the command world at the end of the
    /// script. That needs a normal completion of the whole document and this
    /// very end-of-script state; event handlers, receiver methods, future
    /// frames and bodies first installed by deferred outcomes keep their own
    /// conditional entries.
    fn uncalled_body_after_load(
        &self,
        root: &DeferredSourceBody,
        incoming: &ModuleCommandBindings,
    ) -> bool {
        incoming.baseline.source_point_entry.is_none()
            && root.event.is_none()
            && !root.receiver_method
            && root.future_frame.is_none()
            && self
                .original_completed_root_state
                .as_deref()
                .is_some_and(|completed| completed.same_state(incoming))
    }

    pub(super) fn walk_deferred_entry(
        &mut self,
        root: &DeferredSourceBody,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) {
        self.walk_deferred_entry_with(root, incoming, config, registry, false);
    }

    fn walk_deferred_entry_with(
        &mut self,
        root: &DeferredSourceBody,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        after_load: bool,
    ) {
        #[cfg(any(test, debug_assertions))]
        let _timing = root_owner_work::Timing::at(root_owner_work::Query::DeferredEntry);
        if root.future_frame.is_some() {
            self.walk_future_body(root, incoming, config, registry, None);
            return;
        }
        let mut branch = boxed_source_branch(incoming);
        branch.current_source_origin.clone_from(&root.source_origin);
        branch.receiver_frame_opacity = false;
        let parent = Arc::clone(&branch.source_variables);
        let frame = deferred_source_frame(root, &mut branch, &parent);
        seed_deferred_inputs(&mut branch, root, &frame, registry);
        if let Some((parent, argument, script)) = &root.executed_script {
            self.record_executed_script(parent.clone(), *argument, script.clone(), None, None);
            if let Some((origin, span)) = &root.executed_word {
                self.record_executed_word(origin, *span, None, script.clone());
            }
        }
        let compilation = branch.baseline.body_compilation();
        // This root is already being interpreted in its selected preview
        // frame. A recursive call must reach the ordinary backedge residual,
        // rather than re-enter this body once before noticing the cycle.
        // Preview ownership does not mark a document call as reached.
        let implementation = root.implementation_id();
        let activated = self.active_calls.insert(implementation.clone());
        let preview = usize::from(!after_load);
        self.declaration_preview_depth += preview;
        let outcomes = self.walk_source(
            &root.source,
            root.offset,
            &mut branch,
            &SourceExecutionContext {
                original_logical_body: None,
                control:
                    crate::command_binding::source_execution_control::SourceExecutionControl::new(
                        compilation,
                        tcl_lexer::SourceSegmentationPurpose::NestedScript,
                        if after_load {
                            SourceBodyEntryPurpose::Selected
                        } else {
                            SourceBodyEntryPurpose::Conditional
                        },
                        root.realm,
                    ),
                compilation_snapshot: None,
                selected_compilation: None,
                original_variable_compilation: None,
                namespace: &root.namespace,
                config,
                registry,
                depth: 0,
                frame: &frame,
                invocation_offset: 0,
                variable_read_owner: None,
                original_written_projection: None,
                written_arguments: None,
                written_values: None,
                written_name_values: None,
                written_representations: None,
                written_objects: None,
                written_method_prefixes: None,
                written_variable_reads: None,
                namespace_key: (!root.receiver_method).then_some(&root.namespace_key),
                expression_source: None,
            },
        );
        self.declaration_preview_depth -= preview;
        if activated {
            self.active_calls.remove(&implementation);
        }
        self.record_conditional_procedure_result(root, &outcomes, branch.baseline.dialect);
        let preserves = outcomes.preserves_command_world(incoming);
        self.receiver_body_changes_command_world |=
            root.receiver_namespace.is_some() && outcomes.may_publish_in_receiver_frame(incoming);
        self.world_preserving_bodies
            .entry(root.identity.clone())
            .and_modify(|(held, _)| *held &= preserves)
            .or_insert_with(|| (preserves, root.parameters.clone()));
        self.retain_deferred_outcomes(root, &frame, outcomes);
        // Declaration analysis supplies body points; it is not an execution
        // edge and does not alter the document's actual continuation.
    }

    /// Whether every declaration-time walk of the procedure `identity` (its
    /// formals unknown) leaves the command world as it found it: the body
    /// defines, renames and deletes no command, namespace or object, and
    /// reaches no code that might. `None` when no body of that name was walked.
    #[must_use]
    pub fn procedure_preserves_command_world(&self, identity: &str) -> Option<bool> {
        self.world_preserving_bodies
            .get(identity)
            .map(|(preserves, _)| *preserves)
    }

    /// Whether a declaration-time walk of a runtime receiver's member body may
    /// have published a command into that receiver's private namespace.
    #[must_use]
    pub(super) const fn receiver_body_changes_command_world(&self) -> bool {
        self.receiver_body_changes_command_world
    }

    /// Every procedure whose declaration-time body walks all left the
    /// command world unchanged, with its formals.
    pub fn world_preserving_procedures(
        &self,
    ) -> impl Iterator<Item = (&str, &[tcl_syntax::formal_params::FormalParameter])> {
        self.world_preserving_bodies
            .iter()
            .filter(|(_, (preserves, _))| *preserves)
            .map(|(identity, (_, formals))| (identity.as_str(), formals.as_slice()))
    }

    pub(super) fn retain_deferred_outcomes(
        &mut self,
        root: &DeferredSourceBody,
        frame: &crate::var_resolve::VariableExecutionFrame,
        outcomes: SourceOutcomes,
    ) {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let normal = outcomes.normal.into_iter().map(|state| {
            (
                Route::Tcl(tcl_registry::completion::CompletionCode::Ok),
                state,
            )
        });
        for (route, state) in normal.chain(outcomes.abrupt) {
            let implementation = root.implementation_id();
            if let Some(known) = self.deferred_outcomes.iter_mut().find(|known| {
                known.implementation == implementation
                    && known.namespace_key == root.namespace_key
                    && known.frame == *frame
                    && known.realm == root.realm
                    && known.route == route
            }) {
                Arc::make_mut(&mut known.state).join(&state);
            } else {
                self.deferred_outcomes.push(SourceDeferredOutcome {
                    implementation,
                    namespace_key: root.namespace_key.clone(),
                    frame: frame.clone(),
                    realm: root.realm,
                    route,
                    state: Arc::from(state),
                });
            }
        }
    }
}
