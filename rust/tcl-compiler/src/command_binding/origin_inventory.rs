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

//! Exact point inventories for evaluated script source instances.

use super::{
    Arc, CommandAllocationSite, ExecutedScriptSource, OnceLock, SourceBindingPoint,
    SourceCommandBindings, SourceInvocationBinding, SourceOriginId,
};

/// Purpose retained by the original source binding or module entry. Execution
/// means a supplied Native entry owns the request, including unsupported naming
/// purposes; it grants no dispatch, handler entry, compilation or trace absence.
/// Logical and Standalone retain their own explicitly selected source models.
/// `AuthoredSimulation` retains a selected software naming policy for model source
/// advice only; it supplies no Native entry, dispatch or trace-absence owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OriginalInvocationSourcePurpose {
    Execution,
    Logical,
    Standalone,
    AuthoredSimulation,
}

impl OriginalInvocationSourcePurpose {
    // The complete input/registry/source geometry is checked by the actual
    // state or module owner before this pure authority projection. A dialect,
    // raw source flag or absent Native entry cannot select this policy.
    fn from_authored_simulation_policy(
        policy: Option<tcl_syntax::naming::ExecutionNamePolicy>,
    ) -> Option<Self> {
        (policy?.native_recipe()?.authority()
            == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation)
            .then_some(Self::AuthoredSimulation)
    }
}

/// Source selection and entered activation are separate obligations. Deferred
/// declarations retain the same source mapping without an execution owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EnteredScriptObservation {
    pub source: ExecutedScriptSource,
    pub frame: Option<crate::var_resolve::VariableExecutionFrame>,
    pub namespace: Option<super::SourceNamespaceKey>,
    pub receiver_context:
        Option<Arc<super::original_receiver_body_context::OriginalReceiverBodyContext>>,
    /// Namespace of the invoking command for a body that runs in that same
    /// frame but whose entry is conditional (a loop that may not iterate).
    /// It names where the body would run, never that it ran.
    pub conditional_namespace: Option<super::SourceNamespaceKey>,
}

impl SourceInvocationBinding {
    /// Join the request to this complete original vector before reading the
    /// purpose retained at its actual point. A raw input's Logical naming flag
    /// cannot replace a supplied Native entry, including Jim without a name
    /// protocol. Missing retained purposes decline without profile inference.
    pub(crate) fn original_invocation_source_purpose(
        &self,
        tokens: &crate::ir::CommandTokens,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalInvocationSourcePurpose> {
        if !metadata.matches_registry(registry) || tokens.source_binding.as_ref() != Some(self) {
            return None;
        }
        let original_input = if let Some(input) = metadata.source_analysis_input() {
            self.original_invocation_metadata_for_input(tokens, input, registry)?
                .source_analysis_input()
        } else {
            metadata.is_standalone().then_some(())?;
            self.original_invocation_metadata_for_owner(
                tokens,
                &crate::registry_invocation::OwnedInvocationMetadataContext::Standalone,
                registry,
            )
            .ok()?;
            None
        };
        let declaration = if self.lookup_state.is_none() {
            Some(
                self.declaration_operand_layout_advice(tokens)
                    .or_else(|| self.original_compilation_lookup_advice(tokens))?,
            )
        } else {
            None
        };
        let state = self
            .lookup_state
            .as_ref()
            .map(|point| &point.state)
            .or_else(|| declaration.as_ref().map(|advice| &advice.snapshot.state))?;
        if state.current_source_origin.as_ref() != Some(&self.invocation_site()?.source)
            || state.variable_frame != self.variable_frame
        {
            return None;
        }
        if original_input.is_some()
            && declaration
                .as_ref()
                .is_some_and(|advice| !advice.closed_logical_source_lookup())
            && state.baseline.native_entry.is_none()
        {
            return None;
        }
        state.retained_original_source_purpose(metadata, registry)
    }

    /// Source instance owning this immutable execution point.
    #[must_use]
    pub fn source_origin(&self) -> Option<&Arc<SourceOriginId>> {
        self.lookup_state
            .as_ref()
            .or(self.compiler_lookup_state.as_ref())?
            .state
            .current_source_origin
            .as_ref()
    }

    /// Exact original invocation owner and offset, independent of a lowered
    /// unit's relative spans. This supplies location only, never dispatch proof.
    #[must_use]
    pub fn invocation_site(&self) -> Option<&CommandAllocationSite> {
        self.dispatch_site.as_ref().or_else(|| {
            self.original_written_words
                .as_deref()
                .map(super::original_name_value::OriginalWrittenSourceWords::site)
        })
    }

    /// Actual runtime binding evidence excludes execution observers on this token.
    #[must_use]
    pub fn unobserved_native_dispatch(&self) -> bool {
        if self.entered_execution_observer.observed() {
            return false;
        }
        let Some(target) = self.proved_target() else {
            return false;
        };
        let Some(snapshot) = &self.lookup_state else {
            return false;
        };
        if snapshot.state.baseline.source_point_entry.is_some() {
            return false;
        }
        if snapshot
            .state
            .source_execution_observed(target.identity.as_ref())
            || snapshot.state.source_step_observed()
        {
            return false;
        }
        let Some(entry) = &snapshot.state.baseline.native_entry else {
            // This is the explicit fresh authoring entry contract already used
            // by command lookup, revoked by observed or opaque callbacks.
            return !snapshot.state.opaque_domain && target.registry_backed;
        };
        let Some(runtime) = target
            .identity
            .as_ref()
            .and_then(|identity| identity.runtime)
        else {
            return false;
        };
        if runtime.interpreter != entry.interpreter {
            return false;
        }
        let rows = entry
            .commands
            .iter()
            .filter(|row| row.token == runtime.token)
            .collect::<Vec<_>>();
        !rows.is_empty() && rows.iter().all(|row| !row.has_execution_trace)
    }

    /// The same observer owner for a caller's complete original invocation.
    /// Source instance, current frame and full lookup/execution implementation
    /// must agree before reborrowing the actual Native entry token ledger.
    pub(crate) fn original_unobserved_native_dispatch_for_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
        target: &super::SourceCommandTarget,
    ) -> bool {
        if tokens.source_binding.as_ref() != Some(self)
            || self.original_lexer_config_for_tokens(tokens).is_none()
        {
            return false;
        }
        let Some(head) = self.original_head_name_input(tokens) else {
            return false;
        };
        let Some(snapshot) = self.lookup_state.as_ref() else {
            return false;
        };
        let Some(site) = self.invocation_site() else {
            return false;
        };
        snapshot.state.current_source_origin.as_ref() == Some(&site.source)
            && snapshot.state.variable_frame == self.variable_frame
            && head.is_current(&self.variable_context)
            && target
                .identity
                .as_ref()
                .and_then(|identity| identity.runtime)
                .is_some()
            && self.proved_execution_target() == Some(target)
            && self.proved_target() == Some(target)
            && self.unobserved_native_dispatch()
    }

    /// Source-semantic absence for a separately sealed future fresh allocation.
    /// This does not borrow or manufacture a physical Native token ledger row.
    pub(crate) fn original_fresh_procedure_unobserved_for_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
        target: &super::SourceCommandTarget,
    ) -> bool {
        if tokens.source_binding.as_ref() != Some(self)
            || self.original_lexer_config_for_tokens(tokens).is_none()
            || self.entered_execution_observer.observed()
            || self.proved_execution_target() != Some(target)
            || self.proved_target() != Some(target)
            || self.direct_procedure_target(&target.command) != Some(target)
        {
            return false;
        }
        let Some(head) = self.original_head_name_input(tokens) else {
            return false;
        };
        let Some(snapshot) = self.lookup_state.as_ref() else {
            return false;
        };
        let Some(site) = self.invocation_site() else {
            return false;
        };
        let Some(allocation) = target.implementation_allocation.as_ref() else {
            return false;
        };
        snapshot.state.current_source_origin.as_ref() == Some(&site.source)
            && snapshot.state.variable_frame == self.variable_frame
            && head.is_current(&self.variable_context)
            && snapshot
                .state
                .command_observers
                .fresh_procedures
                .get(allocation)
                .is_some_and(|receipt| receipt.admits(&snapshot.state, target))
    }

    /// Whether this point belongs to original authored document bytes.
    #[must_use]
    pub fn is_authored_source(&self) -> bool {
        self.source_origin()
            .is_some_and(|origin| matches!(origin.kind(), super::SourceOriginKind::Authored(_)))
    }
}

impl SourceCommandBindings {
    /// Possible dispatches recorded inside bodies entered from this exact
    /// invocation. Each binding retains its original source and activation.
    /// Alternative entered observations remain possible calls. Deferred
    /// declarations lack an entered owner and are excluded; this inventory
    /// proves neither unconditional entry nor the parent's effects.
    #[must_use]
    pub(crate) fn possible_entered_body_invocations(
        &self,
        invocation: &CommandAllocationSite,
    ) -> Vec<SourceInvocationBinding> {
        let mut pending = vec![invocation.clone()];
        let mut visited = std::collections::BTreeSet::new();
        let mut dispatches = std::collections::BTreeSet::new();
        while let Some(parent) = pending.pop() {
            if !visited.insert(parent.clone()) {
                continue;
            }
            let Some(arguments) = self.entered_scripts.get(&parent) else {
                continue;
            };
            for observation in arguments.values().flatten() {
                let Some(frame) = &observation.frame else {
                    continue;
                };
                let script = &observation.source;
                let points = if self.root_origin.as_ref() == Some(&script.origin) {
                    self.points.as_slice()
                } else {
                    self.origin_points
                        .get(&script.origin)
                        .map_or(&[][..], Vec::as_slice)
                };
                for point in points.iter().filter(|point| {
                    point.dispatch
                        && &point.state.variable_frame == frame
                        && point.offset >= script.base()
                        && u64::from(point.offset)
                            < u64::from(script.base()) + script.text.len() as u64
                        && !self.deferred.values().any(|body| {
                            body.source_origin.as_ref() == Some(&script.origin)
                                && body.offset >= script.base()
                                && body.offset <= point.offset
                                && u64::from(point.offset)
                                    < u64::from(body.offset) + body.source.len() as u64
                        })
                }) {
                    let site = CommandAllocationSite {
                        source: Arc::clone(&script.origin),
                        offset: point.offset,
                    };
                    if dispatches.insert(site.clone()) {
                        pending.push(site);
                    }
                }
            }
        }
        dispatches
            .into_iter()
            .map(|site| self.invocation_at_origin(&site.source, site.offset))
            .collect()
    }

    /// Source instance selected by this proof inventory.
    #[must_use]
    pub fn source_origin(&self) -> Option<&Arc<SourceOriginId>> {
        self.root_origin.as_ref()
    }

    /// Resolve an implicit reached math call after all of its operands evaluated.
    /// The exact origin and AST start distinguish derived scripts and repeated calls.
    /// Missing inventory is explicit uncertainty, never a nominal built-in lookup.
    #[must_use]
    pub fn implicit_math_invocation_at(
        &self,
        origin: &Arc<SourceOriginId>,
        site: u32,
        function: &str,
    ) -> SourceInvocationBinding {
        let Some(bindings) =
            self.implicit_math_invocations
                .get(&(Arc::clone(origin), site, function.to_owned()))
        else {
            return SourceInvocationBinding::default();
        };
        let has_actual = bindings.iter().any(|binding| {
            binding.runtime_reachability != super::SourceRuntimeReachability::Conditional
        });
        let mut bindings = bindings.iter().filter(|binding| {
            !has_actual
                || binding.runtime_reachability != super::SourceRuntimeReachability::Conditional
        });
        let Some(first) = bindings.next() else {
            return SourceInvocationBinding::default();
        };
        let mut result = first.clone();
        for binding in bindings {
            result.join(binding);
        }
        result
    }

    /// Retain recorded original function occurrences in this source range.
    /// Actual observations and checked preview topology keep distinct purposes.
    #[must_use]
    pub fn implicit_math_invocations_for_script(
        &self,
        script: &ExecutedScriptSource,
    ) -> Vec<super::SourceMathInvocation> {
        let mut calls = Vec::new();
        for (origin, site, function) in self.implicit_math_invocations.keys() {
            if original_math_site_within(script, origin, *site)
                && self
                    .implicit_math_invocation_at(origin, *site, function)
                    .runtime_reachability
                    != super::SourceRuntimeReachability::Conditional
            {
                calls.push(super::SourceMathInvocation::from_reached(
                    super::SourceImplicitMathInvocation {
                        origin: Arc::clone(origin),
                        site: *site,
                        function: function.clone(),
                        unobserved: self
                            .implicit_math_invocation_at(origin, *site, function)
                            .unobserved_native_dispatch(),
                        object_callback_effects: self
                            .object_callback_effects
                            .get(&super::CommandAllocationSite {
                                source: Arc::clone(origin),
                                offset: *site,
                            })
                            .copied(),
                        command_binding: Some(
                            self.implicit_math_invocation_at(origin, *site, function),
                        ),
                        fixed_functions: None,
                        fixed_prerequisite: None,
                    },
                ));
            }
        }
        for ((origin, site, function), observations) in &self.implicit_fixed_math_invocations {
            let mut actual = observations
                .iter()
                .filter(|observation| !observation.declaration_preview);
            let Some(first) = actual.next() else {
                continue;
            };
            if !actual.all(|observation| observation == first) {
                continue;
            }
            let table = &first.prerequisite;
            if original_math_site_within(script, origin, *site) {
                calls.push(super::SourceMathInvocation::from_reached(
                    super::SourceImplicitMathInvocation {
                        origin: Arc::clone(origin),
                        site: *site,
                        function: function.clone(),
                        unobserved: table.is_some(),
                        object_callback_effects: self
                            .object_callback_effects
                            .get(&super::CommandAllocationSite {
                                source: Arc::clone(origin),
                                offset: *site,
                            })
                            .copied(),
                        command_binding: None,
                        fixed_functions: table.as_ref().map(|proof| proof.table.clone()),
                        fixed_prerequisite: table.clone(),
                    },
                ));
            }
        }
        self.extend_conditional_math_invocations(script, &mut calls);
        calls
            .sort_by(|left, right| (left.site, &left.function).cmp(&(right.site, &right.function)));
        calls
    }

    fn extend_conditional_math_invocations(
        &self,
        script: &ExecutedScriptSource,
        calls: &mut Vec<super::SourceMathInvocation>,
    ) {
        // Checked topology remains visible when actual function preparation or
        // dispatch is unavailable. It never replaces a reached proof at a site.
        for observations in self.conditional_expression_evaluations.values() {
            let Some(expression) = observations.first() else {
                continue;
            };
            if !observations.iter().all(|other| other == expression)
                || !matches!(
                    expression.source().mapping,
                    super::ExecutedScriptMapping::Contiguous { .. }
                )
            {
                continue;
            }
            for (function, start, arity) in expression.tree().function_calls() {
                let Some(site) = expression.source().base().checked_add(start) else {
                    continue;
                };
                if original_math_site_within(script, &expression.source().origin, site)
                    && !calls.iter().any(|call| {
                        call.origin == expression.source().origin
                            && call.site == site
                            && call.function == function
                    })
                {
                    calls.push(super::SourceMathInvocation::from_conditional(
                        expression.clone(),
                        function,
                        site,
                        arity,
                    ));
                }
            }
        }
    }

    /// Original checked function topology alongside actual reached proofs.
    /// Conditional occurrences retain an unmet binding-validation obligation.
    #[must_use]
    pub fn math_invocations_for_script(
        &self,
        registry: &tcl_registry::CommandRegistry,
        script: &ExecutedScriptSource,
    ) -> Vec<super::SourceMathInvocation> {
        let mut calls = self.implicit_math_invocations_for_script(script);
        let Some(config) = self.lexer_config else {
            return calls;
        };
        let start = script.base() as usize;
        if !matches!(
            script.mapping,
            super::ExecutedScriptMapping::Contiguous { .. }
        ) || script
            .origin
            .source_image()
            .bytes()
            .get(start..start + script.text.len())
            != Some(script.text.bytes())
        {
            return calls;
        }
        let Some(segments) = crate::segmenter::segment_commands_image_with_offset_and_config(
            &script.text,
            0,
            config.nested(),
        ) else {
            return calls;
        };
        let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
        for segment in segments {
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &script.text.source_map(),
                config,
                &segment,
            );
            crate::lattice_rebase::rebase_command_tokens(&mut tokens, i64::from(script.base()));
            self.stamp_original_tokens(&mut tokens);
            let mut originals = vec![tokens.clone()];
            originals.extend(
                crate::word_subst::original_lifted_calls_with_surface(&tokens, config, &surface)
                    .into_iter()
                    .filter_map(|call| call.tokens),
            );
            for tokens in originals {
                let Some(expression) = tokens.source_binding.as_ref().and_then(|binding| {
                    binding.conditional_expression_evaluation(registry, &tokens)
                }) else {
                    continue;
                };
                for (function, offset, arity) in expression.tree().function_calls() {
                    let Some(site) = expression.source().base().checked_add(offset) else {
                        continue;
                    };
                    if !calls.iter().any(|call| {
                        call.origin == expression.source().origin
                            && call.site == site
                            && call.function == function
                    }) {
                        calls.push(super::SourceMathInvocation::from_conditional(
                            expression.clone(),
                            function,
                            site,
                            arity,
                        ));
                    }
                }
            }
        }
        calls
            .sort_by(|left, right| (left.site, &left.function).cmp(&(right.site, &right.function)));
        calls
    }

    /// Select retained proofs in this evaluated body's actual source instance.
    /// No source interpretation or fresh entry assumption is performed.
    #[must_use]
    pub fn selected_source(&self, script: &ExecutedScriptSource) -> Option<Self> {
        let mut selected = self.clone();
        if self.root_origin.as_ref() != Some(&script.origin) {
            selected.points = self
                .origin_points
                .get(&script.origin)
                .cloned()
                .unwrap_or_default()
                .into();
            selected.variable_accesses = self
                .origin_variable_accesses
                .get(&script.origin)
                .cloned()
                .unwrap_or_default()
                .into();
            selected.root_origin = Some(Arc::clone(&script.origin));
            selected.dispatch_points.clear();
            for (index, point) in selected.points.iter().enumerate() {
                if point.dispatch {
                    selected
                        .dispatch_points
                        .entry(point.offset)
                        .or_default()
                        .push(index);
                }
            }
        }
        Some(selected)
    }

    /// Select retained body observations in one exact namespace incarnation.
    /// This filters existing source receipts and constructs no entry, body
    /// reachability, variable read, or native compilation evidence.
    #[must_use]
    pub fn selected_source_in_context(
        &self,
        script: &ExecutedScriptSource,
        namespace: &super::SourceNamespaceKey,
    ) -> Option<Self> {
        let mut selected = self.selected_source(script)?;
        selected.retain_pre_handler_failure_namespace(&script.origin, namespace);
        let points_changed = selected
            .points
            .iter()
            .any(|point| &point.namespace_key != namespace);
        if points_changed {
            selected
                .points
                .retain(|point| &point.namespace_key == namespace);
            selected.dispatch_points.clear();
            for (index, point) in selected.points.iter().enumerate() {
                if point.dispatch {
                    selected
                        .dispatch_points
                        .entry(point.offset)
                        .or_default()
                        .push(index);
                }
            }
        }
        selected.phases.update_if_needed(
            |phases| {
                phases
                    .iter()
                    .any(|phase| &phase.point.namespace_key != namespace)
            },
            |phases| phases.retain(|phase| &phase.point.namespace_key == namespace),
        );
        selected.compiler_invocations.update_if_needed(
            |invocations| {
                invocations.iter().any(|(site, visits)| {
                    site.source == script.origin
                        && visits.iter().any(|visit| &visit.namespace_key != namespace)
                })
            },
            |invocations| {
                for (site, visits) in invocations {
                    if site.source == script.origin {
                        visits.retain(|visit| &visit.namespace_key == namespace);
                    }
                }
            },
        );
        selected.implicit_math_invocations.update_if_needed(
            |invocations| {
                invocations.iter().any(|((origin, _, _), visits)| {
                    visits.is_empty()
                        || origin == &script.origin
                            && visits
                                .iter()
                                .any(|visit| &visit.lookup_namespace_key != namespace)
                })
            },
            |invocations| {
                for ((origin, _, _), visits) in invocations.iter_mut() {
                    if origin == &script.origin {
                        visits.retain(|visit| &visit.lookup_namespace_key == namespace);
                    }
                }
                invocations.retain(|_, visits| !visits.is_empty());
            },
        );
        selected.expression_preparations.update_if_needed(
            |preparations| {
                preparations.iter().any(|(site, proofs)| {
                    site.source == script.origin
                        && proofs.as_ref().is_some_and(|proofs| {
                            proofs.iter().any(|proof| &proof.namespace_key != namespace)
                        })
                })
            },
            |preparations| {
                for (site, proofs) in preparations {
                    if site.source == script.origin
                        && let Some(proofs) = proofs
                    {
                        proofs.retain(|proof| &proof.namespace_key == namespace);
                    }
                }
            },
        );
        // Selecting nonempty read alternatives still uses the original joined
        // context owner; namespace membership cannot recreate discarded proof.
        if !selected.variable_accesses.is_empty() {
            for accesses in selected.variable_accesses.values_mut() {
                *accesses = accesses
                    .iter()
                    .filter_map(|access| access.selected_namespace_context(namespace))
                    .collect();
            }
        }
        Some(selected)
    }

    /// Query dispatch in an exact evaluated source instance rather than an
    /// authored offset that may coincide with unrelated materialised text.
    #[must_use]
    pub fn invocation_at_origin(
        &self,
        origin: &Arc<SourceOriginId>,
        offset: u32,
    ) -> SourceInvocationBinding {
        if self.root_origin.as_ref() == Some(origin) {
            self.invocation_at_source("", offset)
        } else {
            self.attach_invocation_reads(
                Self::query_dispatch_points(
                    self.origin_points
                        .get(origin)
                        .into_iter()
                        .flatten()
                        .filter(|point| point.dispatch && point.offset == offset),
                ),
                Some(origin),
                offset,
            )
        }
    }

    /// Exact script selected by every recorded entry for an effective operand.
    /// Different evaluated values or source mappings withdraw this projection.
    #[must_use]
    pub fn executed_script_at(
        &self,
        invocation: &CommandAllocationSite,
        argument: usize,
    ) -> Option<&ExecutedScriptSource> {
        let scripts = self.entered_scripts.get(invocation)?.get(&argument)?;
        let first = &scripts.first()?.source;
        scripts
            .iter()
            .all(|observation| &observation.source == first)
            .then_some(first)
    }

    /// Exact namespace incarnation of the unanimous actual entered frame.
    /// This selects retained observations and supplies no body-entry authority.
    #[must_use]
    pub(crate) fn executed_script_entry_namespace_context_at(
        &self,
        invocation: &CommandAllocationSite,
        argument: usize,
        source: &ExecutedScriptSource,
    ) -> Option<&super::SourceNamespaceKey> {
        let observations = self.entered_scripts.get(invocation)?.get(&argument)?;
        let first = observations.first()?;
        let frame = first.frame.as_ref()?;
        if &first.source != source
            || !observations.iter().all(|observation| {
                &observation.source == source && observation.frame.as_ref() == Some(frame)
            })
        {
            return None;
        }
        let namespace = first.namespace.as_ref()?;
        observations
            .iter()
            .all(|observation| observation.namespace.as_ref() == Some(namespace))
            .then_some(namespace)
    }

    /// Retained actual entry context, or an explicitly authored frame when no
    /// native entry exists. Native presentations never cross this boundary.
    pub(crate) fn executed_script_entry_namespace_key_at(
        &self,
        invocation: &CommandAllocationSite,
        argument: usize,
        source: &ExecutedScriptSource,
    ) -> Option<super::SourceNamespaceKey> {
        self.executed_script_entry_namespace_context_at(invocation, argument, source)
            .cloned()
    }

    /// Unanimous retained context for an original body, independently of a
    /// particular call site. Unknown frames and distinct incarnations prevent
    /// selection; a displayed namespace never supplies the missing identity.
    pub(crate) fn executed_script_namespace_context(
        &self,
        source: &ExecutedScriptSource,
    ) -> Option<super::SourceNamespaceKey> {
        use crate::var_resolve::VariableExecutionFrame;
        let frame_key = |frame: &crate::var_resolve::VariableExecutionFrame| {
            if let Some(key) = frame.namespace_identity() {
                return Some(key.clone());
            }
            if self.final_state.baseline.native_entry.is_some() {
                return None;
            }
            match frame.layout() {
                VariableExecutionFrame::Global => Some(super::SourceNamespaceKey::authored("::")),
                VariableExecutionFrame::Namespace(namespace)
                | VariableExecutionFrame::NamespaceActivation { namespace, .. }
                | VariableExecutionFrame::Procedure { namespace, .. } => {
                    Some(super::SourceNamespaceKey::authored(namespace))
                }
                VariableExecutionFrame::Selected { namespace, .. } => {
                    namespace.as_ref().map(super::SourceNamespaceKey::authored)
                }
                _ => None,
            }
        };
        let mut entered = self.entered_scripts.values().flat_map(|arguments| {
            arguments.values().flatten().filter_map(|observation| {
                if &observation.source != source {
                    return None;
                }
                // None retains source selection for a declaration or a call
                // that did not enter. It supplies no actual frame alternative.
                observation
                    .frame
                    .as_ref()
                    .map(|_| observation.namespace.clone())
            })
        });
        if let Some(first) = entered.next() {
            let first = first?;
            return entered
                .all(|key| key.as_ref() == Some(&first))
                .then_some(first);
        }
        let mut conditional = self.entered_scripts.values().flat_map(|arguments| {
            arguments
                .values()
                .flatten()
                .filter(|observation| &observation.source == source && observation.frame.is_none())
                .map(|observation| observation.conditional_namespace.clone())
        });
        if let Some(first) = conditional.next().flatten() {
            let mut declared = self.deferred.values().filter(|body| {
                body.executed_script
                    .as_ref()
                    .is_some_and(|(_, _, retained)| retained == source)
            });
            if declared.next().is_none() && conditional.all(|key| key.as_ref() == Some(&first)) {
                return Some(first);
            }
        }
        let mut declared = self.deferred.values().filter_map(|body| {
            let (_, _, retained) = body.executed_script.as_ref()?;
            (retained == source).then(|| {
                if body.receiver_method {
                    frame_key(body.future_frame.as_ref()?)
                } else {
                    Some(body.namespace_key.clone())
                }
            })
        });
        let first = declared.next()??;
        declared
            .all(|context| context.as_ref() == Some(&first))
            .then_some(first)
    }

    /// Exact evaluated body entered from one original word in this source root.
    #[must_use]
    pub fn executed_script_for_word(&self, span: tcl_lexer::Span) -> Option<&ExecutedScriptSource> {
        self.executed_script_for_element(span, None)
    }

    /// Exact evaluated body entered from a selected native list element.
    #[must_use]
    pub fn executed_script_for_list_element(
        &self,
        span: tcl_lexer::Span,
        element: usize,
    ) -> Option<&ExecutedScriptSource> {
        self.executed_script_for_element(span, Some(element))
    }

    fn executed_script_for_element(
        &self,
        span: tcl_lexer::Span,
        element: Option<usize>,
    ) -> Option<&ExecutedScriptSource> {
        let scripts = self.entered_words.get(&(
            Arc::clone(self.root_origin.as_ref()?),
            (span.start(), span.end()),
            element,
        ))?;
        let first = scripts.first()?;
        scripts
            .iter()
            .all(|script| script == first)
            .then_some(first)
    }

    pub(super) fn restored_script_source(
        &self,
        script: &super::Script,
    ) -> Option<ExecutedScriptSource> {
        let original = script.executed_source.as_ref()?;
        let anchor = script
            .statements
            .first()
            .map(|statement| statement.span().start())
            .or_else(|| {
                script
                    .command_binding_sites
                    .iter()
                    .next()
                    .map(|site| site.span.start())
            });
        let root = self.root_origin.as_ref()?;
        let mut matching = self
            .entered_words
            .values()
            .flatten()
            .chain(
                self.entered_scripts
                    .values()
                    .flat_map(|arguments| arguments.values().flatten())
                    .map(|observation| &observation.source),
            )
            .chain(self.compilation_sources.values().filter_map(Option::as_ref))
            .filter(|source| {
                source.origin == *root
                    && source.text == original.text
                    && anchor.is_none_or(|anchor| {
                        anchor >= source.base()
                            && usize::try_from(anchor - source.base())
                                .is_ok_and(|relative| relative < source.text.len())
                    })
            });
        if let Some(first) = matching.next() {
            return matching
                .all(|source| source == first)
                .then(|| first.clone());
        }
        // A complete source-root script also has an exact retained carrier,
        // even when Direct entry does not create a compiler boundary record.
        match root.kind() {
            super::SourceOriginKind::Authored(text)
            | super::SourceOriginKind::Loaded { source: text, .. }
            | super::SourceOriginKind::Derived { source: text, .. }
                if text.bytes() == original.text.bytes() =>
            {
                ExecutedScriptSource::contiguous_image(Arc::clone(root), original.text.clone(), 0)
            }
            _ => None,
        }
    }

    pub(super) fn record_executed_word(
        &mut self,
        origin: &Arc<SourceOriginId>,
        span: tcl_lexer::Span,
        element: Option<usize>,
        script: ExecutedScriptSource,
    ) {
        let scripts = self
            .entered_words
            .entry((Arc::clone(origin), (span.start(), span.end()), element))
            .or_default();
        if !scripts.contains(&script) {
            scripts.push(script);
        }
    }

    pub(super) fn record_executed_script(
        &mut self,
        invocation: CommandAllocationSite,
        argument: usize,
        script: ExecutedScriptSource,
        frame: Option<&crate::var_resolve::VariableExecutionFrame>,
        namespace: Option<&super::SourceNamespaceKey>,
    ) {
        let scripts = self
            .entered_scripts
            .entry(invocation)
            .or_default()
            .entry(argument)
            .or_default();
        let observation = EnteredScriptObservation {
            source: script,
            frame: frame.cloned(),
            namespace: frame.and(namespace).cloned(),
            receiver_context: None,
            conditional_namespace: None,
        };
        if !scripts.contains(&observation) {
            scripts.push(observation);
        }
    }

    /// A body operand that runs in the invoking command's own frame under a
    /// conditional entry retains that frame's namespace, never an entry.
    pub(super) fn record_conditional_in_frame_script(
        &mut self,
        invocation: CommandAllocationSite,
        argument: usize,
        script: ExecutedScriptSource,
        namespace: Option<&super::SourceNamespaceKey>,
    ) {
        let scripts = self
            .entered_scripts
            .entry(invocation)
            .or_default()
            .entry(argument)
            .or_default();
        let observation = EnteredScriptObservation {
            source: script,
            frame: None,
            namespace: None,
            receiver_context: None,
            conditional_namespace: namespace.cloned(),
        };
        if !scripts.contains(&observation) {
            scripts.push(observation);
        }
    }

    pub(super) fn record_original_receiver_body_context(
        &mut self,
        invocation: &CommandAllocationSite,
        argument: usize,
        source: &ExecutedScriptSource,
        frame: &crate::var_resolve::VariableExecutionFrame,
        context: Option<Arc<super::original_receiver_body_context::OriginalReceiverBodyContext>>,
    ) {
        let Some(context) = context else {
            return;
        };
        let Some(observations) = self
            .entered_scripts
            .get_mut(invocation)
            .and_then(|arguments| arguments.get_mut(&argument))
        else {
            return;
        };
        let Some(index) = observations.iter().position(|observation| {
            &observation.source == source
                && observation.frame.as_ref() == Some(frame)
                && observation.receiver_context.is_none()
        }) else {
            if observations
                .iter()
                .any(|observation| observation.receiver_context.as_ref() == Some(&context))
            {
                return;
            }
            if let Some(previous) = observations.iter().find(|observation| {
                &observation.source == source && observation.frame.as_ref() == Some(frame)
            }) {
                let mut alternative = previous.clone();
                alternative.receiver_context = Some(context);
                observations.push(alternative);
            }
            return;
        };
        observations[index].receiver_context = Some(context);
    }

    /// Only unanimous actual entered receiver observations supply an entry
    /// snapshot. A lexical method body supplies no declaring provider.
    pub(crate) fn original_receiver_body_context(
        &self,
        source: &ExecutedScriptSource,
    ) -> Option<Arc<super::original_receiver_body_context::OriginalReceiverBodyContext>> {
        let mut observations = self
            .entered_scripts
            .values()
            .flat_map(|arguments| arguments.values().flatten())
            .filter(|observation| &observation.source == source && observation.frame.is_some());
        let first = observations.next()?.receiver_context.as_ref()?;
        observations
            .all(|observation| observation.receiver_context.as_ref() == Some(first))
            .then(|| Arc::clone(first))
    }

    /// Exact declaration `ParamList`, independent of method entry or provider.
    pub(crate) fn original_script_formals(
        &self,
        source: &ExecutedScriptSource,
    ) -> Option<Arc<super::formal_topology::OriginalFormalTopology>> {
        let mut matching = self.deferred.values().filter(|body| {
            body.executed_script
                .as_ref()
                .is_some_and(|(_, _, retained)| retained == source)
        });
        let first = matching.next()?.original_parameters.as_ref()?;
        matching
            .all(|body| body.original_parameters.as_ref() == Some(first))
            .then(|| Arc::new(first.clone()))
    }

    pub(super) fn record_origin_point(&mut self, point: SourceBindingPoint) {
        let Some(origin) = &point.state.current_source_origin else {
            return;
        };
        let points = self.origin_points.entry(Arc::clone(origin)).or_default();
        if let Some(previous) = points.iter_mut().find(|previous| {
            previous.dispatch == point.dispatch
                && previous.declaration_preview == point.declaration_preview
                && previous.offset == point.offset
                && previous.namespace == point.namespace
                && previous.realm == point.realm
                && previous.state.variable_frame == point.state.variable_frame
        }) {
            if previous.head != point.head {
                previous.head = None;
            }
            previous
                .method_prefix_arguments
                .retain(|prefix| point.method_prefix_arguments.contains(prefix));
            super::join_evaluated_words(
                &mut previous.evaluated_argument_values,
                &point.evaluated_argument_values,
            );
            previous.state.join(&point.state);
            previous.compiled_execution.join(&point.compiled_execution);
            super::compiled_invocation::join_argument_words(
                &mut previous.evaluated_argument_words,
                &point.evaluated_argument_words,
            );
            previous.lookup_snapshot = OnceLock::new();
        } else {
            points.push(point);
        }
    }
}

fn original_math_site_within(
    script: &ExecutedScriptSource,
    origin: &Arc<SourceOriginId>,
    site: u32,
) -> bool {
    origin == &script.origin
        && site >= script.base()
        && u64::from(site) < u64::from(script.base()) + script.text.len() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entered_namespace_keeps_constructed_colon_owner_and_deferred_source_separate() {
        let source = "namespace eval : {proc p {} {set x VALUE}}";
        let registry = tcl_registry::CommandRegistry::build_default();
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                ..super::super::SourceAnalysisOptions::default()
            },
        );
        let invocation = bindings.invocation_at_source("namespace", 0);
        let site = invocation.invocation_site().unwrap();
        let body = bindings.executed_script_at(site, 2).unwrap();
        assert_eq!(
            bindings
                .executed_script_entry_namespace_context_at(site, 2, body)
                .and_then(super::super::SourceNamespaceKey::display)
                .as_deref(),
            Some(":::")
        );
        let proc_offset = u32::try_from(source.find("proc p").unwrap()).unwrap();
        let procedure = bindings.invocation_at_source("proc", proc_offset);
        let site = procedure.invocation_site().unwrap();
        let body = bindings.executed_script_at(site, 2).unwrap();
        assert!(
            bindings
                .executed_script_entry_namespace_context_at(site, 2, body)
                .is_none()
        );
    }

    #[test]
    fn entered_lambda_inventory_keeps_its_own_namespace_and_inert_words_out() {
        let source = "namespace eval other {proc helper {} {return OTHER}}\nproc helper {} {return ROOT}\nset x [apply {{} {proc dormant {} {::helper}; helper} ::other}]\nset inert {helper}\n";
        let registry = tcl_registry::CommandRegistry::build_default();
        let analysis =
            SourceCommandBindings::analyse(source, tcl_lexer::LexerConfig::default(), &registry);
        let apply_offset = u32::try_from(source.find("apply").unwrap()).unwrap();
        let apply = analysis.invocation_at_source("apply", apply_offset);
        let children = analysis.possible_entered_body_invocations(apply.invocation_site().unwrap());
        let targets: Vec<_> = children
            .iter()
            .flat_map(crate::command_binding::SourceInvocationBinding::execution_targets)
            .filter(|target| !target.registry_backed)
            .map(|target| target.command.as_str())
            .collect();
        assert!(targets.contains(&"::other::helper"), "{targets:?}");
        assert!(!targets.contains(&"::helper"), "{targets:?}");
        let inert_offset = u32::try_from(source.find("set inert").unwrap()).unwrap();
        let inert = analysis.invocation_at_source("set", inert_offset);
        assert!(
            analysis
                .possible_entered_body_invocations(inert.invocation_site().unwrap())
                .is_empty()
        );
    }

    #[test]
    fn selected_body_shares_inventory_and_analysis_writes_detach() {
        let source = "proc p {} {set x VALUE}; p";
        let analysis = SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
        );
        let script = super::ExecutedScriptSource::contiguous(
            Arc::clone(analysis.source_origin().unwrap()),
            source,
            0,
        )
        .unwrap();
        let mut selected = analysis.selected_source(&script).unwrap();
        assert!(Arc::ptr_eq(&selected.final_state, &analysis.final_state));
        assert!(selected.points.shares_storage(&analysis.points));
        assert!(
            selected
                .entered_scripts
                .shares_storage(&analysis.entered_scripts)
        );
        assert!(
            selected
                .compiler_invocations
                .shares_storage(&analysis.compiler_invocations)
        );
        selected.points.clear();
        assert!(selected.points.is_empty());
        assert!(!analysis.points.is_empty());
        assert!(!selected.points.shares_storage(&analysis.points));
        let original_domain = analysis.final_state.opaque_domain;
        Arc::make_mut(&mut selected.final_state).opaque_domain = !original_domain;
        assert!(!Arc::ptr_eq(&selected.final_state, &analysis.final_state));
        assert_eq!(analysis.final_state.opaque_domain, original_domain);
        assert!(
            analysis
                .invocation_at_source("proc", 0)
                .proved_handler_target()
                .is_some()
        );
    }

    #[test]
    fn unchanged_namespace_selection_reuses_original_inventory_without_detachments() {

        // Count eight shared inventories over 32 identical whole-source context
        // selections. This proves no detachments in those collections, not
        // elapsed-time improvement, entered child levels or native execution.
        let source = "if {1} {if {1} {if {1} {set result VALUE}}}";
        let analysis = SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
        );
        let namespace = analysis.points.first().unwrap().namespace_key.clone();
        assert!(
            analysis
                .points
                .iter()
                .all(|point| point.namespace_key == namespace)
        );
        let script = ExecutedScriptSource::contiguous(
            Arc::clone(analysis.source_origin().unwrap()),
            source,
            0,
        )
        .unwrap();
        let original = analysis.invocation_at_source("", 0);
        let mut current = analysis.clone();
        let mut detachments = 0;
        for _ in 0..32 {
            let selected = current
                .selected_source_in_context(&script, &namespace)
                .unwrap();
            for shared in [
                selected.points.shares_storage(&current.points),
                selected.phases.shares_storage(&current.phases),
                selected
                    .dispatch_points
                    .shares_storage(&current.dispatch_points),
                selected
                    .compiler_invocations
                    .shares_storage(&current.compiler_invocations),
                selected
                    .implicit_math_invocations
                    .shares_storage(&current.implicit_math_invocations),
                selected
                    .expression_preparations
                    .shares_storage(&current.expression_preparations),
                selected
                    .pre_handler_failures
                    .shares_storage(&current.pre_handler_failures),
                selected
                    .unrepresented_entries
                    .shares_storage(&current.unrepresented_entries),
            ] {
                detachments += usize::from(!shared);
            }
            assert_eq!(selected.invocation_at_source("", 0), original);
            current = selected;
        }
        assert_eq!(detachments, 0);
        assert_eq!(analysis.invocation_at_source("", 0), original);
    }

    #[test]
    fn namespace_selection_detaches_changed_points_and_preserves_original_views() {

        // Genuine mixed namespaces remain filtered. An unrelated namespace
        // obtains no dispatch or expression preparation from a shared view.
        let source = "namespace eval a {expr {1 + 2}}; namespace eval b {expr {3 + 4}}";
        let analysis = SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
        );
        let script = ExecutedScriptSource::contiguous(
            Arc::clone(analysis.source_origin().unwrap()),
            source,
            0,
        )
        .unwrap();
        let preparations = analysis.expression_preparations_for_script(&script);
        assert_eq!(preparations.len(), 2);
        let namespace = &preparations[0].namespace_key;
        let selected = analysis
            .selected_source_in_context(&script, namespace)
            .unwrap();
        assert!(!selected.points.shares_storage(&analysis.points));
        assert!(
            !selected
                .expression_preparations
                .shares_storage(&analysis.expression_preparations)
        );
        assert!(
            selected
                .points
                .iter()
                .all(|point| &point.namespace_key == namespace)
        );
        assert_eq!(
            selected.expression_preparations_for_script(&script),
            vec![preparations[0].clone()]
        );
        let again = selected
            .selected_source_in_context(&script, namespace)
            .unwrap();
        assert!(again.points.shares_storage(&selected.points));
        assert!(
            again
                .expression_preparations
                .shares_storage(&selected.expression_preparations)
        );
        let unrelated = super::super::SourceNamespaceKey::authored("::unrelated");
        let absent = analysis
            .selected_source_in_context(&script, &unrelated)
            .unwrap();
        assert!(absent.points.is_empty());
        assert!(absent.invocation_at_source("", 0).unknown);
        assert!(
            absent
                .expression_preparations_for_script(&script)
                .is_empty()
        );
        assert_eq!(
            analysis.expression_preparations_for_script(&script),
            preparations
        );
    }

    #[test]
    fn selected_body_keeps_expression_preparation_in_its_original_namespace() {
        let source = "namespace eval a {expr {1 + 2}}; namespace eval b {expr {3 + 4}}";
        let analysis = SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
        );
        let script = ExecutedScriptSource::contiguous(
            Arc::clone(analysis.source_origin().unwrap()),
            source,
            0,
        )
        .unwrap();
        let preparations = analysis.expression_preparations_for_script(&script);
        assert_eq!(preparations.len(), 2, "{preparations:?}");
        assert_ne!(preparations[0].namespace_key, preparations[1].namespace_key);
        for original in &preparations {
            let selected = analysis
                .selected_source_in_context(&script, &original.namespace_key)
                .unwrap();
            assert_eq!(
                selected.expression_preparations_for_script(&script),
                vec![original.clone()]
            );
        }
        let unrelated = super::super::SourceNamespaceKey::authored("::unrelated");
        assert!(
            analysis
                .selected_source_in_context(&script, &unrelated)
                .unwrap()
                .expression_preparations_for_script(&script)
                .is_empty()
        );
        assert_eq!(
            analysis.expression_preparations_for_script(&script),
            preparations
        );
    }

    #[test]
    fn lookup_world_cache_key_retains_availability_phase() {
        use std::hash::{Hash, Hasher};

        let analysis = SourceCommandBindings::analyse(
            "set x VALUE",
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
        );
        let loader = analysis.final_state.as_ref();
        let mut runtime = loader.clone();
        let baseline = Arc::make_mut(&mut runtime.baseline);
        baseline.invocation_realm = tcl_dialect::model::InvocationRealm::InterpreterRuntime;
        baseline.refresh_fingerprint();
        assert_ne!(loader, &runtime);
        let same = loader.clone();
        assert_eq!(loader, &same);
        let fingerprint = |state: &super::super::ModuleCommandBindings| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            state.hash(&mut hasher);
            hasher.finish()
        };
        assert_eq!(fingerprint(loader), fingerprint(&same));
    }

    #[test]
    fn unknown_selected_namespace_preserves_only_absolute_command_lookup() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let frame = crate::var_resolve::VariableExecutionFrame::Selected {
            selector: tcl_registry::FrameLevel::Relative(1),
            namespace: None,
        };
        for (source, absolute) in [("::set ::fixed 1", true), ("set ::fixed 1", false)] {
            let analysis = SourceCommandBindings::analyse(
                source,
                tcl_lexer::LexerConfig::default(),
                &registry,
            );
            let selected = analysis
                .analyse_script_at_site(
                    source,
                    0,
                    0,
                    &frame,
                    tcl_lexer::LexerConfig::default(),
                    &registry,
                )
                .unwrap();
            let binding = selected.invocation_at_source("", 0);
            assert!(!binding.variable_context.namespace_known);
            assert_eq!(binding.proved_execution_target().is_some(), absolute);
        }
    }

    #[test]
    fn catch_output_cells_are_written_after_the_body_completes() {
        let source = "set r OLD; catch {unset r} r; list [info exists r]";
        let analysis = SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
        );
        let offset = u32::try_from(source.rfind("list").unwrap()).unwrap();
        let binding = analysis.invocation_at_source("list", offset);
        assert_eq!(
            binding.evaluated_argument_values,
            vec![Some("1".to_owned())]
        );
    }

    #[test]
    fn decoded_body_points_remain_in_their_actual_source_instance() {
        let source = "eval \"set x\\x20VALUE\"; list $x";
        let analysis = SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
        );
        let parent = CommandAllocationSite {
            source: Arc::clone(analysis.root_origin.as_ref().unwrap()),
            offset: 0,
        };
        let script = analysis
            .executed_script_at(&parent, 0)
            .expect("executed eval operand");
        assert_eq!(script.text.try_text().unwrap(), "set x VALUE");
        assert!(matches!(
            script.mapping,
            super::super::ExecutedScriptMapping::Materialised
        ));
        let selected = analysis.selected_source(script).unwrap();
        assert!(
            selected
                .invocation_at_source("set", 0)
                .proved_execution_target()
                .is_some_and(|target| target.registry_backed)
        );
        assert!(
            analysis
                .invocation_at_origin(&script.origin, 0)
                .proved_execution_target()
                .is_some()
        );
        assert_ne!(script.origin, parent.source);
    }
}

#[cfg(test)]
mod original_math_body_purpose_tests {
    use super::*;

    #[test]
    fn original_math_body_inventory_keeps_extracted_bom_and_parent_origin() {

        // Original conditional expression topology only; no native function registration.
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
            let config = tcl_lexer::LexerConfig {
                leading_bom: bom,
                braced_var: tcl_lexer::BracedVarStyle::FirstClose,
                ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
            };
            for (source, expected) in [
                ("eval {expr {abs(-3)}}", true),
                ("eval {\u{feff}expr {abs(-3)}}", false),
            ] {
                let bindings = SourceCommandBindings::analyse_with_options(
                    source,
                    config,
                    registry,
                    super::super::SourceAnalysisOptions {
                        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                            profile,
                        )),
                        ..Default::default()
                    },
                );
                let parent = super::super::CommandAllocationSite {
                    source: Arc::clone(bindings.source_origin().unwrap()),
                    offset: 0,
                };
                let body = bindings
                    .executed_script_at(&parent, 0)
                    .expect("genuine original eval operand");
                assert!(matches!(
                    body.mapping,
                    super::super::ExecutedScriptMapping::Contiguous { .. }
                ));
                let calls = bindings.math_invocations_for_script(registry, body);
                assert_eq!(
                    calls.iter().any(|call| call.function == "abs"),
                    expected,
                    "{bom:?}: {source:?}"
                );
                assert_eq!(body.origin, parent.source);
                assert!(bindings.matches_original_source_image(
                    &tcl_lexer::SourceImage::document(source),
                    config
                ));
                assert!(
                    super::super::ExecutedScriptSource::contiguous(
                        Arc::clone(&body.origin),
                        "expr {abs(-4)}",
                        body.base()
                    )
                    .is_none()
                );
            }
        }
    }
    #[test]
    fn retained_authored_simulation_requires_its_actual_policy_and_supplied_input() {

        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
        let registry = context.commands();
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let config = tcl_lexer::LexerConfig::for_dialect("tcl8.4");
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let policy = tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4),
        );
        let entry = super::super::SourceAnalysisEntry {
            metadata_context:
                crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                    &input,
                )),
            execution_name_policy: Some(policy),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            ..Default::default()
        };
        let source = "llength {a b}";
        let bindings =
            SourceCommandBindings::analyse_with_options(source, config, registry, entry.options());
        assert!(
            bindings
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config,)
        );
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
            registry,
            &input,
            config,
            Some(profile),
        )
        .unwrap();
        let state = bindings.final_state.as_ref();
        assert!(state.baseline.native_entry.is_none());
        assert!(state.baseline.logical_source_input.is_none());
        assert_eq!(state.baseline.execution_name_policy, Some(policy));
        assert_eq!(
            state.retained_original_source_purpose(metadata, registry),
            Some(OriginalInvocationSourcePurpose::AuthoredSimulation),
        );
        // Withdrawing the actual selected policy cannot recover it from the
        // unchanged profile, dialect, source input or registry availability.
        let mut missing_policy = state.clone();
        Arc::make_mut(&mut missing_policy.baseline).execution_name_policy = None;
        assert_eq!(
            missing_policy.retained_original_source_purpose(metadata, registry),
            None,
        );
        let foreign_context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let foreign_profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let foreign_input = crate::analyser::ResolvedAnalysisInput::new(
            foreign_profile,
            foreign_profile,
            Arc::clone(&foreign_context),
            tcl_lexer::LexerConfig::for_dialect("tcl9.0"),
        );
        let foreign = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            foreign_context.commands(),
            &foreign_input,
        )
        .unwrap();
        assert_eq!(
            state.retained_original_source_purpose(foreign, registry),
            None
        );
        let standalone = tcl_registry::model::semantic::SemanticContext::for_profile(profile);
        assert_eq!(
            state.retained_original_source_purpose(standalone.into(), registry),
            None
        );
    }
}

impl super::ModuleCommandBindings {
    /// The entry purpose retained in this source state. Callers must first join
    /// their original source/argument/declaration geometry to this same state;
    /// this projection supplies no invocation, layout, frame or execution grant.
    pub(crate) fn retained_original_source_purpose(
        &self,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalInvocationSourcePurpose> {
        if !metadata.matches_registry(registry)
            || self.baseline.registry_snapshot.as_ref() != Some(&registry.snapshot().semantic_key())
        {
            return None;
        }
        let original = &self.baseline.metadata_context;
        match (
            original.source_analysis_input(),
            metadata.source_analysis_input(),
        ) {
            (Some(original), Some(input))
                if original == input || original == &input.for_nested_source() => {}
            (None, None) if original.is_standalone() && metadata.is_standalone() => {}
            _ => return None,
        }
        if self.baseline.source_point_entry.is_some() {
            // Inherited naming state supplies no implicit invocation purpose.
            return None;
        }
        if self.baseline.native_entry.is_some() {
            return Some(OriginalInvocationSourcePurpose::Execution);
        }
        if self.baseline.unknown_entry {
            return None;
        }
        if let Some(input) = metadata.source_analysis_input() {
            if self.baseline.logical_source_input.is_some() {
                let logical = self.logical_source_name_advice_input()?;
                return (logical == input || logical == &input.for_nested_source())
                    .then_some(OriginalInvocationSourcePurpose::Logical);
            }
            return OriginalInvocationSourcePurpose::from_authored_simulation_policy(
                self.baseline.execution_name_policy,
            );
        }
        original
            .is_standalone()
            .then_some(OriginalInvocationSourcePurpose::Standalone)
    }
}

impl crate::ir::Module {
    /// Purpose of this module's retained entry for an implicit operation lacking
    /// an actual invocation receiver. Availability and raw Logical eligibility
    /// cannot select its purpose; Native entries still need a separate token
    /// owner before any trace absence is usable.
    pub(crate) fn original_invocation_source_purpose(
        &self,
        registry: &tcl_registry::CommandRegistry,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    ) -> Option<OriginalInvocationSourcePurpose> {
        let original =
            crate::registry_invocation::InvocationMetadataContext::for_module(registry, self)?;
        if !metadata.matches_registry(registry)
            || original.source_analysis_input() != metadata.source_analysis_input()
            || original.is_standalone() != metadata.is_standalone()
        {
            return None;
        }
        if self.source_entry.source_point_entry.is_some() {
            return None;
        }
        if self.source_entry.native_entry.is_some() {
            return Some(OriginalInvocationSourcePurpose::Execution);
        }
        if self.source_entry.unknown_entry {
            return None;
        }
        let options = self.source_entry.options();
        if let Some(input) = metadata.source_analysis_input() {
            if self.source_entry.logical_source_input.is_some() {
                let logical =
                    options.retained_logical_source_input(registry, Some(self.lexer_config))?;
                return (logical == *input || logical.for_nested_source() == *input)
                    .then_some(OriginalInvocationSourcePurpose::Logical);
            }
            let retained_input = self.source_entry.metadata_context.source_analysis_input()?;
            if retained_input != input && retained_input != &input.for_nested_source() {
                return None;
            }
            return OriginalInvocationSourcePurpose::from_authored_simulation_policy(
                self.source_entry.execution_name_policy,
            );
        }
        (metadata.is_standalone() && options.retained_metadata_context().is_standalone())
            .then_some(OriginalInvocationSourcePurpose::Standalone)
    }
}
