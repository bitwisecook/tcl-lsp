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

//! The metadata context and source purpose a declaration's words are read under.

use super::super::original_name_value::OriginalWordSourcePurpose;
use super::{OriginalDeclarationWordSource, original_declaration_layouts};

impl super::super::OriginalCompilationLookupAdvice {
    /// Metadata of this sealed original declaration, including raw preview
    /// vectors which have no reached invocation. Exact source, grammar and
    /// tagged availability remain prerequisites; this grants no dispatch,
    /// Normal handler, Native preparation, entered frame or variable effects.
    /// Only explicit Standalone may return `Ok(None)`.
    pub(crate) fn original_declaration_metadata_context(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Result<
        Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        crate::registry_invocation::RegistryInvocationDecline,
    > {
        OriginalDeclarationWordSource {
            snapshot: &self.snapshot,
            site: &self.site,
            words: &self.words,
            config: self.config,
            dialect: self.dialect,
        }
        .metadata_context(registry, tokens, OriginalWordSourcePurpose::Invocation)
    }
}

impl super::super::OriginalCompilationLookupAdvice {
    /// Closed source syntax in the positively retained authored simulation.
    /// The namespace belongs to that source world, independently of a physical
    /// variable frame, Native registration, compilation or successful handler.
    pub(crate) fn closed_authored_simulation_source_lookup(
        &self,
        tokens: &crate::ir::CommandTokens,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        let Ok(Some(original)) = self.original_declaration_metadata_context(registry, tokens)
        else {
            return false;
        };
        let (Some(held), Some(input)) = (
            original.source_analysis_input(),
            metadata.source_analysis_input(),
        ) else {
            return false;
        };
        let state = &self.snapshot.state;
        held == input
            && !input.has_logical_source_name_context()
            && !input.has_hosted_source_name_context()
            && self.original_source_purpose(metadata, registry)
                == Some(super::super::OriginalInvocationSourcePurpose::AuthoredSimulation)
            && state.baseline.native_entry.is_none()
            && self
                .source_purpose_state()
                .baseline
                .source_point_entry
                .is_none()
            && state.baseline.logical_source_input.is_none()
            && state.baseline.vendor_source_input.is_none()
            && state.baseline.hosted_execution_context.is_none()
            && !state.baseline.unknown_entry
            && !self.unknown
            && !self.may_be_absent
            && !self.targets.is_empty()
            && !state.has_opaque_domain()
            && !state.source_step_observed()
            && state.command_observers.is_quiet()
            && matches!(
                &self.namespace,
                super::super::SourceNamespaceKey::Authored(_)
            )
            && state.namespaces.contains(&self.namespace)
            && self.targets.iter().all(|target| {
                target.kind == super::super::BindingKind::Builtin
                    && target.registry_backed
                    && target.prepended.is_empty()
                    && target.implementation_generation == 0
                    && target.runtime_implementation_generation.is_none()
                    && state.retained_target_is_current(target)
                    && !state.source_execution_observed(target.identity.as_ref())
                    && !state.runtime_execution_observed(target.identity.as_ref())
            })
    }

    /// Purpose of this already joined original declaration or compiler layout.
    /// The shared baseline classifier issues no invocation, frame or handler.
    pub(crate) fn original_source_purpose(
        &self,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<super::super::OriginalInvocationSourcePurpose> {
        self.source_purpose_state()
            .retained_original_source_purpose(metadata, registry)
    }

    /// The state whose baseline supplies this layout's source purpose: its own,
    /// or, when it continues a source past a conditional region, the state of
    /// that source which issued the region.
    fn source_purpose_state(&self) -> &super::super::ModuleCommandBindings {
        let mut state = &self.snapshot.state;
        while let (Some(point), Some(origin)) = (
            state.baseline.source_point_entry.as_ref(),
            state.current_source_origin.as_deref(),
        ) {
            let Some(issuer) = point.issuing_continuation_state(origin) else {
                break;
            };
            state = issuer;
        }
        state
    }
}

impl super::super::SourceInvocationBinding {
    /// Keep source-layout purpose separate from actual invocation permission.
    /// A reached point retains its terminal invocation checks; only its absence
    /// permits the existing exact declaration/compiler source correspondence.
    #[doc(hidden)]
    pub fn original_source_advice_purpose(
        &self,
        tokens: &crate::ir::CommandTokens,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<super::super::OriginalInvocationSourcePurpose> {
        if self.lookup_state.is_some() {
            return self.original_invocation_source_purpose(tokens, metadata, registry);
        }
        if !metadata.matches_registry(registry) {
            return None;
        }
        let advice = self
            .declaration_operand_layout_advice(tokens)
            .or_else(|| self.original_compilation_lookup_advice(tokens))?;
        let original = advice
            .original_declaration_metadata_context(registry, tokens)
            .ok()?;
        match (original, metadata.source_analysis_input()) {
            (Some(original), Some(request)) => {
                let input = original.source_analysis_input()?;
                if input != request && input != &request.for_nested_source() {
                    return None;
                }
            }
            (None, None) if metadata.is_standalone() => {}
            (Some(original), None)
                if original.is_standalone()
                    && metadata.is_standalone()
                    && original.context() == metadata.context() => {}
            _ => return None,
        }
        advice.original_source_purpose(metadata, registry)
    }
}

impl super::super::SourceCommandBindings {
    /// Purpose of this complete retained source interpretation. Original image,
    /// configuration and metadata remain independent prerequisites; no point,
    /// definition, variable frame or Native permission follows from this tag.
    pub(crate) fn original_source_entry_purpose(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<super::super::OriginalInvocationSourcePurpose> {
        if !self.matches_original_source_image(image, config)
            || metadata
                .source_analysis_input()
                .is_some_and(|input| input.lexer_config() != config)
        {
            return None;
        }
        self.final_state
            .retained_original_source_purpose(metadata, registry)
    }
}

impl<'a> OriginalDeclarationWordSource<'a> {
    pub(super) fn metadata_context(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
        purpose: OriginalWordSourcePurpose,
    ) -> Result<
        Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
        crate::registry_invocation::RegistryInvocationDecline,
    > {
        use crate::registry_invocation::{
            OwnedInvocationMetadataContext, RegistryInvocationDecline,
        };
        let refusal = RegistryInvocationDecline::IncompleteResolution;
        let snapshot: &'a super::super::SourceLookupSnapshot = self.snapshot.as_ref();
        let state = &snapshot.state;
        if !purpose.accepts(tokens)
            || tokens.words() != self.words
            || state.current_source_origin.as_ref() != Some(&self.site.source)
            || state.baseline.registry_snapshot.as_ref()
                != Some(&registry.snapshot().semantic_key())
            || state.source_variables.invocation_dialect != Some(self.dialect)
        {
            return Err(refusal);
        }
        self.site
            .source
            .original_lexical_command(tokens.words(), self.site.offset, self.config)
            .ok_or(refusal)?;
        let owner = &state.baseline.metadata_context;
        if let Some(binding) = tokens.source_binding.as_ref() {
            if binding.invocation_site() != Some(self.site)
                || (if purpose == OriginalWordSourcePurpose::ArgumentEvaluation {
                    binding.original_argument_lexer_config(tokens)
                } else {
                    binding.original_lexer_config_for_tokens(tokens)
                }) != Some(self.config)
            {
                return Err(refusal);
            }
            if let Some(point) = &binding.lookup_state {
                if purpose == OriginalWordSourcePurpose::ArgumentEvaluation {
                    return Err(refusal);
                }
                // An explicitly retained invalid reached owner cannot be
                // repaired from an earlier declaration observation.
                binding
                    .original_invocation_metadata_at_point(tokens, registry)
                    .map(|_| ())?;
                if point.state.baseline.metadata_context != *owner {
                    return Err(refusal);
                }
            } else {
                let rows = original_declaration_layouts(
                    binding
                        .declaration_layout_observations
                        .as_deref()
                        .ok_or(refusal)?,
                )
                .ok_or(refusal)?;
                if !rows.clone().all(|row| {
                    row.entry.owns_source(&self.site.source, self.site.offset)
                        && row.words.as_ref() == self.words
                        && row.config == self.config
                        && row.snapshot.state.baseline.metadata_context == *owner
                }) {
                    return Err(refusal);
                }
            }
        }
        match owner {
            OwnedInvocationMetadataContext::SuppliedSource(input) => owner
                .metadata_context_for_source(registry, self.config, Some(input.unit_profile()))
                .map(Some)
                .ok_or(refusal),
            OwnedInvocationMetadataContext::Standalone => owner
                .metadata_context(registry)
                .map(crate::registry_invocation::InvocationMetadataSelection::context)
                .ok_or(refusal),
            OwnedInvocationMetadataContext::Supplied(_)
            | OwnedInvocationMetadataContext::Unavailable => Err(refusal),
        }
    }
}

impl super::super::SourceInvocationBinding {
    /// Complete original argument metadata without selecting a declaration's
    /// handler. Only the genuine argv-evaluation boundary shares this purpose.
    pub(crate) fn original_declaration_argument_metadata(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'_>> {
        if self.lookup_state.is_some() {
            return None;
        }
        let site = self.invocation_site()?;
        let rows = original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let mut selected = None;
        for row in rows {
            if !row.entry.owns_source(&site.source, site.offset) {
                return None;
            }
            let source = OriginalDeclarationWordSource {
                snapshot: &row.snapshot,
                site,
                words: &row.words,
                config: row.config,
                dialect: row.snapshot.state.source_variables.invocation_dialect?,
            };
            let metadata = source
                .metadata_context(
                    registry,
                    tokens,
                    OriginalWordSourcePurpose::ArgumentEvaluation,
                )
                .ok()??;
            (row.snapshot
                .state
                .retained_original_source_purpose(metadata, registry)?
                == crate::command_binding::OriginalInvocationSourcePurpose::Logical)
                .then_some(())?;
            if selected.is_some_and(
                |prior: crate::registry_invocation::InvocationMetadataContext<'_>| {
                    prior.source_analysis_input() != metadata.source_analysis_input()
                },
            ) {
                return None;
            }
            selected = Some(metadata);
        }
        selected
    }

    /// Positively Logical metadata from unanimous sealed original declaration
    /// observations. No reached point is manufactured. A present reached owner
    /// is handled by the separate checked point route and cannot fall back here.
    pub(crate) fn original_declaration_metadata_for_source(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'_>> {
        if self.lookup_state.is_some() {
            return None;
        }
        let site = self.invocation_site()?;
        let rows = original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let mut selected = None;
        for row in rows {
            if !row.entry.owns_source(&site.source, site.offset)
                || row.words.as_ref() != tokens.words()
            {
                return None;
            }
            let advice = super::super::original_site_operand_layout_advice(
                site,
                tokens,
                &row.snapshot,
                &row.namespace,
                row.config,
            )?;
            let checked = advice
                .original_declaration_metadata_context(registry, tokens)
                .ok()??;
            (row.snapshot
                .state
                .retained_original_source_purpose(checked, registry)?
                == crate::command_binding::OriginalInvocationSourcePurpose::Logical)
                .then_some(())?;
            let input = row
                .snapshot
                .state
                .baseline
                .metadata_context
                .source_analysis_input()?;
            let metadata = row
                .snapshot
                .state
                .baseline
                .metadata_context
                .metadata_context_for_source(registry, row.config, Some(input.unit_profile()))?;
            if selected.is_some_and(
                |prior: crate::registry_invocation::InvocationMetadataContext<'_>| {
                    prior.source_analysis_input() != metadata.source_analysis_input()
                },
            ) {
                return None;
            }
            selected = Some(metadata);
        }
        selected
    }
}
