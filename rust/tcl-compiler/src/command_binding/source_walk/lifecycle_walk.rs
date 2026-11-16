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

//! Lifecycle bodies: their phases, iteration and the coverage recorded at each phase entry.

use super::super::binding_projection::PointWalkEntry;
use super::super::command_identity::CommandAllocationSite;
use super::super::invocation_binding::SourceObserverExecution;
use super::super::trusted_packages::TrustedPackageLoader;
use super::super::{
    ModuleCommandBindings, PointBindingMemo, SourceBindingPoint, SourceCommandBindings,
    SourceExecutionContext, SourceLifecycleCoverage, SourceLifecycleInvocation,
    SourceLifecyclePhase, SourceOutcomes, SourcePhasePoint, SourceScriptOperands,
    compiled_invocation,
};
use super::outcomes::boxed_source_branch;
use std::hash::Hash;
use std::sync::{Arc, OnceLock};

/// Execution phase selected by a registry-owned caller-frame lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceBodyPhase {
    /// Setup; normal completion permits the main phase.
    Setup,
    /// Main script reached after setup completes normally.
    Body,
    /// Cleanup reached after catchable completions of the earlier phases.
    Cleanup,
}

/// Coverage of a lifecycle phase across every retained selected traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceBodyPhaseReachability {
    /// Every completed traversal skipped this phase.
    NotEntered,
    /// At least one completed traversal entered this phase.
    MayEntered,
    /// A traversal or its lifecycle selection remains unresolved.
    Unknown,
}

impl SourceCommandBindings {
    fn record_phase(
        &mut self,
        site: u32,
        phase: SourceBodyPhase,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) {
        if let Some(previous) = self.phases.iter_mut().find(|point| {
            point.site == site
                && point.phase == phase
                && point.point.namespace_key == context.namespace_identity()
                && point.point.realm == context.realm()
                && point.point.state.current_source_origin == state.current_source_origin
                && point.point.declaration_preview == (self.declaration_preview_depth != 0)
                && point.point.state.variable_frame == state.variable_frame
        }) {
            previous.point.state.join(state);
            previous.point.invalidate_derived();
        } else {
            self.phases.push(SourcePhasePoint {
                site,
                phase,
                point: SourceBindingPoint {
                    declaration_preview: self.declaration_preview_depth != 0,
                    walk_entry: PointWalkEntry::at_depth(self.unreached_walk_depth),
                    original_arguments_complete_normally: false,
                    observer_execution: SourceObserverExecution::Unobserved,
                    realm: context.realm(),
                    compilation: context.compilation(),
                    compiled_execution: compiled_invocation::CompiledInvocationSelection::default(),
                    head: None,
                    original_head_input: None,
                    original_written_words: None,
                    evaluated_argument_values: Arc::default(),
                    evaluated_argument_words: Arc::default(),
                    frozen_written_words: None,
                    frozen_written_names: None,
                    frozen_head_object: None,
                    method_prefix_arguments: Vec::new(),
                    offset: site,
                    end: site,
                    namespace: context.namespace.to_owned(),
                    namespace_key: context.namespace_identity(),
                    state: state.clone(),
                    lookup_snapshot: OnceLock::new(),
                    original_binding: PointBindingMemo::default(),
                    dispatch: false,
                    continuation_of: None,
                },
            });
        }
    }

    pub(super) fn walk_lifecycle(
        &mut self,
        invocation: SourceLifecycleInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let repetition = state
            .loaded_provider(invocation.spec.provider.package)
            .map_or(
                tcl_registry::body_execution::BodyRepetition::MayRepeat,
                |loader| invocation.spec.repetition(loader.version.as_deref()),
            );
        let mut outcomes = SourceOutcomes::normal(state);
        let mut head = boxed_source_branch(state);
        loop {
            if crate::analysis_control::checkpoint(
                crate::analysis_control::AnalysisCheckpoint::Fixpoint,
            ) {
                break;
            }
            let before = head.clone();
            // A transfer may publish into its mutable input, including an
            // early unknown-provider path. Keep the loop predecessor intact
            // so differing issuance histories meet at the back edge rather
            // than comparing a newly issued stamp with itself.
            let mut iteration_state = head.clone();
            let iteration = self.walk_lifecycle_iteration(
                invocation,
                &mut iteration_state,
                context.conditional(),
            );
            outcomes.join(&iteration);
            if let Some(normal) = iteration.normal {
                head.join(&normal);
            }
            if repetition == tcl_registry::body_execution::BodyRepetition::Once
                || head.same_state(&before)
            {
                break;
            }
        }
        outcomes.publish(state);
        outcomes
    }

    fn begin_lifecycle_coverage(
        &mut self,
        invocation_site: u32,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<(CommandAllocationSite, usize)> {
        state.current_source_origin.as_ref().map(|origin| {
            let site = CommandAllocationSite {
                source: Arc::clone(origin),
                offset: invocation_site,
            };
            let entries = self.lifecycle_coverage.entry(site.clone()).or_default();
            let index = entries.len();
            entries.push(SourceLifecycleCoverage {
                declaration_preview: self.declaration_preview_depth != 0,
                frame: state.variable_frame.clone(),
                namespace: context.namespace_identity(),
                entered: None,
            });
            (site, index)
        })
    }

    fn walk_lifecycle_iteration(
        &mut self,
        invocation: SourceLifecycleInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::body_execution::BodyExecutionSelection;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let coverage = self.begin_lifecycle_coverage(invocation.site, state, context);
        let mut entered = [false; 3];
        let spec = invocation.spec;
        if spec.has_unmodelled_pre_phase_effects(invocation.arguments) {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        }
        let Some(loader) = state.loaded_provider(spec.provider.package).cloned() else {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        };
        if loader.implementation_id != spec.provider.implementation_id
            || !state.provider_surface_is_live(&loader)
        {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        }
        let phases = match spec.select(invocation.arguments) {
            BodyExecutionSelection::CapturedLifecycle(phases) => phases,
            BodyExecutionSelection::InvalidArguments => {
                return SourceOutcomes::invocation(
                    state,
                    Route::Tcl(tcl_registry::completion::CompletionCode::Error),
                );
            }
            BodyExecutionSelection::UnknownArguments => {
                state.mark_opaque_binding_mutation();
                return SourceOutcomes::invocation(state, Route::Unknown);
            }
        };
        let incoming = boxed_source_branch(state);
        let mut pending = SourceOutcomes::normal(state);
        for (phase, operand, hook) in [
            (
                SourceBodyPhase::Setup,
                phases.setup,
                spec.hook_commands.first(),
            ),
            (
                SourceBodyPhase::Body,
                phases.body,
                spec.hook_commands.get(1),
            ),
        ] {
            let Some(mut normal) = pending.normal.take() else {
                break;
            };
            entered[match phase {
                SourceBodyPhase::Setup => 0,
                SourceBodyPhase::Body => 1,
                SourceBodyPhase::Cleanup => 2,
            }] = true;
            let executed = self.walk_lifecycle_phase(
                invocation,
                SourceLifecyclePhase {
                    phase,
                    operand,
                    hook: hook.copied(),
                },
                &loader,
                &mut normal,
                context,
            );
            pending.join(&executed);
        }
        // Stock lifecycle catches phase completions. Only normal setup enters
        // the body; every catchable completion proceeds through cleanup.
        let mut escaped = pending.capture_tcl_completions();
        if let Some(mut cleanup_entry) = escaped.normal.take() {
            entered[2] = true;
            let cleanup = self.walk_lifecycle_phase(
                invocation,
                SourceLifecyclePhase {
                    phase: SourceBodyPhase::Cleanup,
                    operand: phases.cleanup,
                    hook: spec.hook_commands.get(2).copied(),
                },
                &loader,
                &mut cleanup_entry,
                context,
            );
            escaped.join(&cleanup.capture_tcl_completions());
        }
        if let Some((site, index)) = coverage {
            self.lifecycle_coverage
                .get_mut(&site)
                .expect("recorded lifecycle traversal")[index]
                .entered = Some(entered);
        }
        // Test-selection skip is an independently possible successful path.
        escaped.join(&SourceOutcomes::normal(&incoming));
        escaped.publish(state);
        escaped
    }

    fn walk_lifecycle_phase(
        &mut self,
        invocation: SourceLifecycleInvocation<'_>,
        selected: SourceLifecyclePhase<'_>,
        loader: &TrustedPackageLoader,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let SourceLifecyclePhase {
            phase,
            operand,
            hook,
        } = selected;
        if !state.provider_surface_is_live(loader) {
            state.mark_opaque_binding_mutation();
        }
        self.record_phase(invocation.site, phase, state, context);
        let hooks = invocation
            .spec
            .required_absent_hooks(loader.version.as_deref());
        if hook.is_some_and(|hook| {
            hooks.contains(&hook) && !state.definitely_absent(hook, context.namespace)
        }) {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        }
        let Some(operand) = operand else {
            return SourceOutcomes::normal(state);
        };
        if operand.list_element.is_some() {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        }
        self.walk_body_operand(
            operand.argument,
            SourceScriptOperands {
                compilation_spec: Some(&tcl_registry::native_compilation::NativeCompilationSpec {
                    grammar: tcl_registry::native_compilation::NativeCompilationGrammar::NoHook,
                    operation: tcl_registry::SemanticOperationId::Invoke,
                    body: invocation.spec.phase_compilation,
                }),
                compilation_selection:
                    &tcl_registry::native_compilation::NativeCompilationSelection::Generic,
                words: invocation.words,
                written_arguments: invocation.written_arguments,
                target: invocation.target,
                arguments: &invocation.arguments,
            },
            state,
            &SourceExecutionContext {
                control: (context)
                    .control
                    .with_source_purpose(tcl_lexer::SourceSegmentationPurpose::NestedScript),
                depth: context.depth + 1,
                ..context
            },
        )
    }
}
