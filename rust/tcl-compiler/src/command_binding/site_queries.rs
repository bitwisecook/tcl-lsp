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

//! Queries at a source site of a completed interpretation: binding, namespaces, absence and phase.

use super::binding_projection::source_binding;
use super::command_identity::CommandAllocationSite;
use super::compiled_invocation::CompiledExecutionResidual;
use super::source_analysis_entry::SourceAnalysisOptions;
use super::source_walk::{SourceBodyPhase, SourceBodyPhaseReachability};
use super::trusted_packages::TrustedPackageLoader;
use super::{SourceBindingPoint, SourceCommandBindings, SourceInvocationBinding};
use std::sync::Arc;
use tcl_registry::CommandRegistry;

impl SourceCommandBindings {
    /// A source query can only prove dispatch for a represented invocation.
    #[must_use]
    pub fn has_site(&self, offset: u32) -> bool {
        self.dispatch_points.contains_key(&offset)
    }

    /// Namespace objects proved to exist immediately before this invocation.
    #[must_use]
    pub fn known_namespaces_at(&self, offset: u32) -> Arc<[String]> {
        let mut points = self.dispatch_points_at(offset).peekable();
        let Some(first) = points.next() else {
            return Arc::from([]);
        };
        if points.peek().is_none() {
            return first.state.namespaces.advisory_names();
        }
        let mut namespaces = first.state.namespaces.as_ref().clone();
        for point in points {
            namespaces.retain(|namespace| point.state.namespaces.contains(namespace));
        }
        namespaces.advisory_names()
    }

    /// Exact source-site namespace proof, shared with place resolution.
    #[must_use]
    pub fn namespaces_at_source(&self, offset: u32) -> Vec<String> {
        self.known_namespaces_at(offset).to_vec()
    }

    /// Command absence before dispatch, independently of `unknown` fallback.
    #[must_use]
    pub fn command_is_absent_at(&self, command: &str, offset: u32) -> bool {
        let mut points = self.dispatch_points_at(offset);
        let Some(first) = points.next() else {
            return false;
        };
        first.state.definitely_absent(command, &first.namespace_key)
            && points.all(|point| point.state.definitely_absent(command, &point.namespace_key))
    }

    /// Whether every selected lifecycle traversal proves a phase unentered.
    /// Missing or unfinished traversal evidence remains explicit uncertainty.
    #[must_use]
    pub fn lifecycle_phase_reachability_at(
        &self,
        site: &CommandAllocationSite,
        phase: SourceBodyPhase,
    ) -> SourceBodyPhaseReachability {
        let Some(coverage) = self.lifecycle_coverage.get(site) else {
            return SourceBodyPhaseReachability::Unknown;
        };
        let coverage = coverage
            .iter()
            .filter(|entry| !entry.declaration_preview)
            .collect::<Vec<_>>();
        if coverage.is_empty() || coverage.iter().any(|entry| entry.entered.is_none()) {
            return SourceBodyPhaseReachability::Unknown;
        }
        let index = match phase {
            SourceBodyPhase::Setup => 0,
            SourceBodyPhase::Body => 1,
            SourceBodyPhase::Cleanup => 2,
        };
        if coverage
            .iter()
            .filter_map(|entry| entry.entered)
            .any(|entered| entered[index])
        {
            SourceBodyPhaseReachability::MayEntered
        } else {
            SourceBodyPhaseReachability::NotEntered
        }
    }

    /// Exact phase-entry binding used for hook and phase dependency proofs.
    #[must_use]
    pub fn invocation_at_phase(
        &self,
        command: &str,
        site: u32,
        phase: SourceBodyPhase,
    ) -> SourceInvocationBinding {
        Self::query_points(
            command,
            self.phases
                .iter()
                .filter(|point| {
                    point.site == site
                        && point.phase == phase
                        && point.point.state.current_source_origin.as_ref()
                            == self.root_origin.as_ref()
                        && !point.point.declaration_preview
                })
                .map(|point| &point.point),
        )
    }

    /// Command absence at a represented lifecycle phase entry.
    #[must_use]
    pub fn command_is_absent_at_phase(
        &self,
        command: &str,
        site: u32,
        phase: SourceBodyPhase,
    ) -> bool {
        let points = self
            .phases
            .iter()
            .filter(|point| {
                point.site == site
                    && point.phase == phase
                    && point.point.state.current_source_origin.as_ref() == self.root_origin.as_ref()
                    && !point.point.declaration_preview
            })
            .collect::<Vec<_>>();
        !points.is_empty()
            && points.iter().all(|point| {
                point
                    .point
                    .state
                    .definitely_absent(command, &point.point.namespace_key)
            })
    }

    /// Whether text-only body lowering can reuse a fresh isolated proof.
    /// Every invocation is compared through the same binding interpretation;
    /// namespace and provider facts are part of the proof, not lexical keys.
    #[must_use]
    pub fn can_reuse_isolated_body(
        &self,
        body: &str,
        base: u32,
        namespace: &str,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) -> bool {
        let baseline = &self.final_state.baseline;
        if baseline.source_point_entry.is_some() {
            return false;
        }

        let loaders = baseline
            .trusted_loaders
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let options = SourceAnalysisOptions {
            analyser_source_settings: self.analyser_source_settings.as_ref(),
            source_point_entry: None,
            metadata_context: crate::registry_invocation::InvocationMetadataInput::Retained(
                &self.final_state.baseline.metadata_context,
            ),
            compilation_scope: tcl_runtime_api::SourceCompilationScope::WholeModule,
            unentered: false,
            invocation_realm: self
                .points
                .iter()
                .find(|point| point.dispatch && point.offset >= base)
                .map_or(self.final_state.baseline.invocation_realm, |point| {
                    point.realm
                }),
            incoming_formals: &[],
            native_entry: self.final_state.baseline.native_entry.as_deref(),
            native_compilation: self
                .points
                .iter()
                .find(|point| point.dispatch && point.offset >= base)
                .map_or(self.final_state.baseline.native_compilation, |point| {
                    point.compilation
                }),
            declared_commands: None,
            trusted_package_loaders: &loaders,
            trusted_source_modules: &self.final_state.baseline.trusted_source_modules,
            unknown_entry: false,
            host_extensible_command_table: baseline.host_extensible_command_table,
            invocation_dialect: self.final_state.baseline.dialect,
            hosted_execution_context: self.final_state.baseline.hosted_execution_context,
            execution_name_policy: self.final_state.baseline.execution_name_policy,
            logical_source_input: self.final_state.baseline.logical_source_input.as_ref(),
            vendor_source_input: self.final_state.baseline.vendor_source_input.as_ref(),
            compiled_variable_provider: self.final_state.baseline.compiled_variable_provider,
            expression_parser: self.final_state.baseline.expression_parser,
        };
        let fallback = crate::var_resolve::VariableExecutionFrame::Namespace(namespace.to_owned());
        let frame = self
            .points
            .iter()
            .find(|point| {
                point.dispatch
                    && point.offset >= base
                    && point.offset
                        < base.saturating_add(u32::try_from(body.len()).unwrap_or(u32::MAX))
            })
            .map_or(&fallback, |point| &point.state.variable_frame);
        let isolated = Self::analyse_in_frame_with_options(body, frame, config, registry, options);
        // A cached isolated Script does not replay document declaration,
        // import/export or deferred-unit publication into the enclosing
        // lowerer. Admit cache reuse only when the shared transfer proves no
        // such state changes, before comparing every actual dispatch point.
        if isolated.has_transitions() || !isolated.deferred.is_empty() {
            return false;
        }
        isolated
            .points
            .iter()
            .filter(|point| point.dispatch)
            .all(|point| {
                let Some(original_offset) = base.checked_add(point.offset) else {
                    return false;
                };
                let Some(head) = point.head.as_deref() else {
                    return false;
                };
                if isolated
                    .invocation_at_source(head, point.offset)
                    .targets
                    .iter()
                    .any(|target| {
                        registry
                            .get(&target.command)
                            .is_some_and(|spec| spec.body_execution.is_some())
                    })
                {
                    return false;
                }
                if !isolated
                    .invocation_at_source(head, point.offset)
                    .same_dispatch_projection(&self.invocation_at_source(head, original_offset))
                {
                    return false;
                }
                self.same_isolated_body_dependencies(
                    &isolated,
                    head,
                    point.offset,
                    original_offset,
                    registry,
                )
            })
    }

    fn same_isolated_body_dependencies(
        &self,
        isolated: &Self,
        head: &str,
        offset: u32,
        original_offset: u32,
        registry: &CommandRegistry,
    ) -> bool {
        let contracts = isolated.invocation_at_source(head, offset).targets;
        contracts.iter().all(|target| {
            registry
                .get(&target.command)
                .and_then(|spec| spec.body_execution)
                .is_none_or(|contract| {
                    let tcl_registry::body_execution::BodyExecutionSpec::CapturedLifecycle(
                        contract,
                    ) = contract
                    else {
                        return true;
                    };
                    isolated.loaded_implementation_at(contract.provider.package, offset)
                        == self.loaded_implementation_at(contract.provider.package, original_offset)
                })
        })
    }

    /// Conditional source argument closure at every retained original point.
    /// This does not supply reached dispatch or Normal operation completion.
    pub(crate) fn original_conditional_arguments_for_source_coverage(
        &self,
        native: &[tcl_lexer::NativeWord],
        registry: &CommandRegistry,
    ) -> bool {
        let Some(head) = native.first() else {
            return false;
        };
        let Some(origin) = self.root_origin.as_ref() else {
            return false;
        };
        let mut seen = false;
        for point in self.dispatch_points_at(head.span().start()) {
            seen = true;
            if point.declaration_preview
                || !point.original_arguments_complete_normally
                || point.observer_execution.observed()
                || point
                    .original_written_words
                    .as_ref()
                    .is_none_or(|words| !words.matches_native_words(native))
                || point
                    .state
                    .current_source_origin
                    .as_ref()
                    .is_none_or(|actual| !Arc::ptr_eq(actual, origin))
                || point.state.baseline.registry_snapshot
                    != Some(registry.snapshot().semantic_key())
            {
                return false;
            }
        }
        seen
    }

    /// A package-owned descriptor is usable as an instance factory only when
    /// the actual selected command is its implementation and every reaching
    /// dispatch carries that provider. A provided version alone is insufficient.
    pub(crate) fn package_factory_is_proved_at(
        &self,
        head: &tcl_lexer::NativeWord,
        descriptor: &tcl_registry::CommandSpec,
        context: &tcl_registry::model::ContextRegistry,
    ) -> bool {
        let Some(package) = descriptor.owning_package() else {
            return true;
        };
        let Some(origin) = self.source_origin() else {
            return false;
        };
        if head.image() != origin.source_image() {
            return false;
        }
        let offset = head.span().start();
        let binding = self.invocation_at_source("", offset);
        if binding
            .proved_execution_target()
            .and_then(super::SourceCommandTarget::registry_identity)
            .is_none_or(|command| {
                !tcl_registry::command_descriptor_labels_match(command, descriptor.name)
            })
        {
            return false;
        }
        let slot = tcl_registry::rooted_command_descriptor_label(descriptor.name);
        let mut points = self.dispatch_points_at(offset).peekable();
        points.peek().is_some()
            && points.all(|point| {
                let state = &point.state;
                if context.context().ambient_package(package)
                    && state.baseline.native_entry.is_some()
                    && !state.has_opaque_domain()
                {
                    return true;
                }
                if state.has_opaque_domain() {
                    return false;
                }
                state.packages.get(package).is_some_and(|provisions| {
                    !provisions.is_empty()
                        && provisions.iter().all(|value| {
                            value.present
                                && value
                                    .implementation
                                    .as_ref()
                                    .is_some_and(|provider| provider.commands.contains(&slot))
                        })
                })
            })
    }

    /// Loader provenance live at an exact invocation site, independently of
    /// its current command-table binding.
    #[must_use]
    pub fn loaded_implementation_at(
        &self,
        package: &str,
        offset: u32,
    ) -> Option<&TrustedPackageLoader> {
        let mut points = self.dispatch_points_at(offset);
        let first = points.next()?;
        let loader = first.state.loaded_provider(package)?;
        points
            .all(|point| point.state.loaded_provider(package) == Some(loader))
            .then_some(loader)
    }

    /// Every possible provider command is original or absent on its audited
    /// version path; document bindings and unknown targets withdraw proof.
    #[must_use]
    pub fn provider_surface_untampered(
        &self,
        package: &str,
        implementation_id: &str,
        offset: u32,
    ) -> bool {
        let mut points = self.dispatch_points_at(offset).peekable();
        points.peek().is_some()
            && points.all(|point| {
                point.state.loaded_provider(package).is_some_and(|loader| {
                    loader.implementation_id == implementation_id
                        && point.state.provider_surface_is_live(loader)
                })
            })
    }

    /// Provider closure proof at the exact lifecycle phase entry.
    #[must_use]
    pub fn provider_surface_untampered_at_phase(
        &self,
        package: &str,
        implementation_id: &str,
        site: u32,
        phase: SourceBodyPhase,
    ) -> bool {
        let points = self
            .phases
            .iter()
            .filter(|point| {
                point.site == site
                    && point.phase == phase
                    && point.point.state.current_source_origin.as_ref() == self.root_origin.as_ref()
                    && !point.point.declaration_preview
            })
            .collect::<Vec<_>>();
        !points.is_empty()
            && points.iter().all(|point| {
                point
                    .point
                    .state
                    .loaded_provider(package)
                    .is_some_and(|loader| {
                        loader.implementation_id == implementation_id
                            && point.point.state.provider_surface_is_live(loader)
                    })
            })
    }

    /// Query the binding immediately before dispatch at a source site.
    #[must_use]
    pub fn invocation_at(
        &self,
        _head: &str,
        namespace: &str,
        offset: u32,
    ) -> SourceInvocationBinding {
        self.attach_invocation_reads(
            Self::query_dispatch_points(
                self.dispatch_points_at(offset)
                    .filter(|point| point.namespace == namespace),
            ),
            self.root_origin.as_ref(),
            offset,
        )
    }

    /// Query with the execution namespace recorded for the source region.
    #[must_use]
    pub fn invocation_at_source(&self, _head: &str, offset: u32) -> SourceInvocationBinding {
        self.attach_invocation_reads(
            Self::query_dispatch_points(self.dispatch_points_at(offset)),
            self.root_origin.as_ref(),
            offset,
        )
    }

    /// Whether the interpretation dispatched a command at `offset`. A site it
    /// never reached has no binding fact of either kind.
    #[must_use]
    pub fn reached_site(&self, offset: u32) -> bool {
        self.dispatch_points.contains_key(&offset)
    }

    /// Whether the interpretation began the command at `offset`: dispatched
    /// it, or evaluated its words until one of them raised. A command after
    /// one that was proved to raise was never begun.
    #[must_use]
    pub fn entered_site(&self, offset: u32) -> bool {
        self.reached_site(offset)
            || self
                .pre_handler_failures
                .keys()
                .any(|site| site.offset == offset)
    }

    /// Whether this interpretation entered no command (see
    /// [`SourceAnalysisOptions::unentered`]).
    #[must_use]
    pub fn is_unentered(&self) -> bool {
        self.unentered
    }

    pub(super) fn dispatch_points_at(
        &self,
        offset: u32,
    ) -> impl Iterator<Item = &SourceBindingPoint> {
        let view = self
            .declared_frame_view
            .as_deref()
            .filter(|view| view.covers(offset));
        self.dispatch_points
            .get(&offset)
            .into_iter()
            .flatten()
            .map(|index| &self.points[*index])
            .filter(move |point| view.is_none_or(|view| view.admits(point)))
    }

    pub(super) fn query_points<'a>(
        head: &str,
        mut points: impl Iterator<Item = &'a SourceBindingPoint>,
    ) -> SourceInvocationBinding {
        let Some(first) = points.next() else {
            return SourceInvocationBinding {
                unknown: true,
                ..SourceInvocationBinding::default()
            };
        };
        let mut result = first.binding(head);
        for point in points {
            result.join(&point.binding(head));
        }
        result
    }

    pub(super) fn query_dispatch_points<'a>(
        points: impl Iterator<Item = &'a SourceBindingPoint>,
    ) -> SourceInvocationBinding {
        let points = points.collect::<Vec<_>>();
        let actual = points.iter().any(|point| !point.declaration_preview);
        let mut points = points
            .into_iter()
            .filter(|point| !actual || !point.declaration_preview);
        let Some(first) = points.next() else {
            return SourceInvocationBinding {
                unknown: true,
                ..SourceInvocationBinding::default()
            };
        };
        let binding = |point: &SourceBindingPoint| {
            let mut binding = point.original_dispatch_binding();
            binding
                .frozen_head_object
                .clone_from(&point.frozen_head_object);
            if point.frozen_head_object.is_some() {
                binding.entered_execution_observer = point.observer_execution;
                binding
                    .frozen_written_words
                    .clone_from(&point.frozen_written_words);
                binding
                    .frozen_written_names
                    .clone_from(&point.frozen_written_names);
            }
            binding
                .evaluated_argument_values
                .clone_from(&point.evaluated_argument_values);
            binding
                .evaluated_argument_words
                .clone_from(&point.evaluated_argument_words);
            // naming.source.logical-original-operation-transfer
            // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
            // Keep the same conditional Logical lookup selected above. It has
            // no Native head receipt and supplies no Native execution permits.
            if point.original_head_input.is_none() && point.original_logical_lookup_head().is_none()
            {
                binding.unknown = true;
                binding.targets.clear();
                binding.declared_command = None;
                binding.catalogue_command = None;
            }
            binding
        };
        let mut result = binding(first);
        for point in points {
            result.join(&binding(point));
        }
        result
    }

    /// Compatibility source-position projection for assistance consumers.
    /// Semantic lowering must query an exact dispatch site instead.
    #[must_use]
    pub fn projection_at_source(&self, head: &str, offset: u32) -> SourceInvocationBinding {
        if self
            .final_state
            .baseline
            .source_point_entry
            .as_ref()
            .is_some_and(|entry| entry.conditional_region().is_some())
            && !self.dispatch_points.contains_key(&offset)
        {
            return SourceInvocationBinding {
                unknown: true,
                ..SourceInvocationBinding::default()
            };
        }

        let point = self.point_covering(offset);
        point.map_or_else(
            || {
                let mut proof = source_binding(&self.final_state, head, "::");
                proof.compiled_execution_residual = CompiledExecutionResidual::Unknown;
                proof
            },
            |point| point.lookup_binding(head),
        )
    }

    /// Offset-free consumers must agree across every represented state.
    #[must_use]
    pub fn invocation_unpositioned(&self, head: &str) -> SourceInvocationBinding {
        if self
            .final_state
            .baseline
            .source_point_entry
            .as_ref()
            .is_some_and(|entry| entry.conditional_region().is_some())
        {
            return SourceInvocationBinding {
                unknown: true,
                ..SourceInvocationBinding::default()
            };
        }

        if let Some(projection) = self
            .unpositioned_projections
            .0
            .lock()
            .expect("source projection cache")
            .get(head)
            .cloned()
        {
            return projection;
        }
        let mut result = source_binding(&self.final_state, head, "::");
        result.compiled_execution_residual = CompiledExecutionResidual::Unknown;
        for point in &self.points {
            result.join(&point.lookup_binding(head));
        }
        self.unpositioned_projections
            .0
            .lock()
            .expect("source projection cache")
            .insert(head.to_owned(), result.clone());
        result
    }

    pub(super) fn invalidate_unpositioned_projections(&mut self) {
        self.unpositioned_projections
            .0
            .get_mut()
            .expect("source projection cache")
            .clear();
    }

    /// Whether the document changes the registry entry table.
    #[must_use]
    pub fn has_transitions(&self) -> bool {
        !self.final_state.bindings.is_empty()
            || self.final_state.opaque_domain
            || self.final_state.namespace_resolution.changed
    }
}
