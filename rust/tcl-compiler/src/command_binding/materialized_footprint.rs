// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional materialized text footprints anchored at an original source lookup.

use super::original_name_value::OriginalWordSourcePurpose;

use super::{ModuleCommandBindings, SourceInvocationBinding, SourceNamespaceKey};
use crate::ir::CommandTokens;
use crate::registry_invocation::InvocationMetadataContext;
use tcl_lexer::LexerConfig;
use tcl_registry::CommandRegistry;

/// Readonly source lookup for possible writes in a materialized script value.
/// It supplies no authored child word/span, Native entry, frame or store.
pub(crate) struct OriginalSourceMaterializedFootprint<'a> {
    binding: &'a SourceInvocationBinding,
    lookup: OriginalMaterializedSourceLookup<'a>,
    registry: &'a CommandRegistry,
    metadata: InvocationMetadataContext<'a>,
    config: LexerConfig,
    head: tcl_lexer::NativeWord,
}

/// Reached installer state and conditional declaration advice are different
/// purposes. Retaining the latter never manufactures an execution snapshot.
enum OriginalMaterializedSourceLookup<'a> {
    Reached {
        state: &'a ModuleCommandBindings,
        namespace: &'a SourceNamespaceKey,
    },
    LogicalDeclaration {
        snapshot: std::sync::Arc<super::SourceLookupSnapshot>,
        namespace: SourceNamespaceKey,
    },
}

impl OriginalMaterializedSourceLookup<'_> {
    fn state(&self) -> &ModuleCommandBindings {
        match self {
            Self::Reached { state, .. } => state,
            Self::LogicalDeclaration { snapshot, .. } => &snapshot.state,
        }
    }

    fn namespace(&self) -> &SourceNamespaceKey {
        match self {
            Self::Reached { namespace, .. } => namespace,
            Self::LogicalDeclaration { namespace, .. } => namespace,
        }
    }
}

/// Original procedure argv evaluates lexical substitutions only. This
/// borrowed source-purpose projection supplies no callable, body or Normal grant.
pub(crate) struct OriginalSourceProcedureArgumentRoles<'a> {
    _binding: &'a SourceInvocationBinding,
    _tokens: &'a CommandTokens,
    target: Option<super::SourceCommandTarget>,
}

impl OriginalSourceProcedureArgumentRoles<'_> {
    pub(crate) fn target(&self) -> Option<&super::SourceCommandTarget> {
        self.target.as_ref()
    }
}

impl SourceInvocationBinding {
    /// Conditional original procedure allocations in the retained Logical
    /// model. Native handler/activation purposes consume their separate owners.
    pub(crate) fn original_logical_procedure_call_targets(
        &self,
        tokens: &CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<super::SourceCommandTarget>> {
        if !input.has_logical_source_name_context()
            || self.original_invocation_source_purpose(
                tokens,
                self.original_invocation_metadata_for_input(
                    tokens,
                    input,
                    input.borrowed_context_registry().commands(),
                )?,
                input.borrowed_context_registry().commands(),
            )? != crate::command_binding::OriginalInvocationSourcePurpose::Logical
        {
            return None;
        }
        let advice = self.original_logical_declaration_layout_advice(tokens)?;
        if !advice.closed_logical_source_lookup()
            || advice.targets().iter().any(|target| {
                target.kind != super::BindingKind::Proc
                    || target.implementation_allocation.is_none()
            })
        {
            return None;
        }
        Some(advice.targets().to_vec())
    }

    /// Original source procedure argv roles under its complete current input.
    /// Registry body/expression roles are absent only for a unanimously selected
    /// original source procedure. Logical declarations retain their own sealed
    /// source purpose; Native argv retains its actual entry and closed lookup.
    pub(crate) fn original_source_procedure_argument_roles<'a>(
        &'a self,
        tokens: &'a CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &CommandRegistry,
    ) -> Option<OriginalSourceProcedureArgumentRoles<'a>> {
        let metadata = self.original_invocation_metadata_for_input(tokens, input, registry)?;
        let logical = self.original_invocation_source_purpose(tokens, metadata, registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
        let targets = if logical {
            // The sealed declaration keeps raw runtime uncertainty. Join its
            // complete source targets before using internal allocations solely
            // for original procedure correspondence, never actual dispatch.
            let declared =
                self.original_logical_declaration_call_targets(tokens, input, registry)?;
            let targets = self.original_logical_procedure_call_targets(tokens, input)?;
            if declared.len() != targets.len()
                || declared.iter().zip(&targets).any(|(declared, target)| {
                    let mut source = target.clone();
                    source.implementation_allocation = None;
                    declared != &source
                })
            {
                return None;
            }
            targets
        } else {
            let point = self.lookup_state.as_ref()?;
            let entry = point.state.baseline.native_entry.as_deref()?;
            if entry.execution_name_policy() != point.state.source_variables.execution_name_policy
                || entry.command_name_policy().is_none()
            {
                return None;
            }
            let rows = super::declaration_layout::original_declaration_layouts(
                self.declaration_layout_observations.as_deref()?,
            )?;
            if !rows.clone().all(|row| {
                row.snapshot
                    .state
                    .baseline
                    .native_entry
                    .as_deref()
                    .is_some_and(|original| entry.same_compilation_world(original))
            }) {
                return None;
            }
            let advice = self.declaration_call_layout_advice(tokens)?;
            advice.source_targets_are_closed().then_some(())?;
            advice.targets().to_vec()
        };
        let first = targets.first()?;
        let allocation = first.implementation_allocation.as_ref()?;
        if (!logical && (self.unknown || self.may_be_absent || self.targets != targets))
            || targets.iter().any(|target| {
                target.kind != super::BindingKind::Proc
                    || target.registry_backed
                    || target.command != first.command
                    || target.implementation_allocation.as_ref() != Some(allocation)
            })
        {
            return None;
        }
        Some(OriginalSourceProcedureArgumentRoles {
            _binding: self,
            _tokens: tokens,
            target: targets
                .iter()
                .all(|target| target == first)
                .then(|| first.clone()),
        })
    }

    /// Actual metadata at this whole original source vector under its Module
    /// producer. Positively Logical, declaration-only source uses the sealed
    /// original-declaration purpose; Native source still requires its point.
    /// Unknown dispatch remains unknown: this grants no closed lookup,
    /// handler, frame, Normal completion or materialized body.
    pub(crate) fn original_invocation_metadata_for_module<'a>(
        &'a self,
        tokens: &CommandTokens,
        module: &crate::ir::Module,
        registry: &CommandRegistry,
    ) -> Option<InvocationMetadataContext<'a>> {
        self.original_word_metadata_for_module(
            tokens,
            module,
            registry,
            OriginalWordSourcePurpose::Invocation,
        )
    }

    /// Original argv geometry under the actual whole Module. The synthetic
    /// argument boundary keeps its marker and grants no invocation effects.
    pub(crate) fn original_argument_metadata_for_module<'a>(
        &'a self,
        tokens: &CommandTokens,
        module: &crate::ir::Module,
        registry: &CommandRegistry,
    ) -> Option<InvocationMetadataContext<'a>> {
        self.original_word_metadata_for_module(
            tokens,
            module,
            registry,
            OriginalWordSourcePurpose::ArgumentEvaluation,
        )
    }

    fn original_word_metadata_for_module<'a>(
        &'a self,
        tokens: &CommandTokens,
        module: &crate::ir::Module,
        registry: &CommandRegistry,
        purpose: OriginalWordSourcePurpose,
    ) -> Option<InvocationMetadataContext<'a>> {
        // naming.diagnostic.original-materialized-write-footprint

        if !module
            .retained_source_bindings
            .as_deref()?
            .matches_module(module, registry)
        {
            return None;
        }
        let metadata = if module.source_metadata_input.is_some() {
            let module_metadata = InvocationMetadataContext::for_module(registry, module)?;
            let module_input = module_metadata.source_analysis_input()?;
            self.original_word_metadata_for_input(tokens, module_input, registry, purpose)?
        } else {
            let entry = &module.source_entry;
            if !entry.metadata_context.is_standalone()
                || entry.unknown_entry
                || entry.native_entry.is_some()
                || entry.logical_source_input.is_some()
                || entry.vendor_source_input.is_some()
                || entry.hosted_execution_context.is_some()
                || self
                    .original_word_config(tokens, registry, purpose)?
                    .nested()
                    .normalized()
                    != module.native_lexer_config().nested().normalized()
            {
                return None;
            }
            self.original_word_metadata_for_owner(
                tokens,
                &crate::registry_invocation::OwnedInvocationMetadataContext::Standalone,
                registry,
                purpose,
            )
            .ok()??
        };
        if !original_module_origin(&self.invocation_site()?.source, module) {
            return None;
        }
        Some(metadata)
    }

    /// Availability retained by this function and its exact original source.
    /// A positively Logical declaration uses the sealed source-only purpose;
    /// Native input still requires its independently retained point.
    /// This validates the FU input/configuration and complete source vector; it
    /// cannot authenticate a whole changed Module without the Module overload.
    /// Neither route grants a handler, physical frame, read or normal result.
    ///
    /// The outer `None` is terminal refusal. A missing FU input is compatible
    /// only when the original point independently retains explicit Standalone;
    /// supplied-missing ownership never reconstructs that mode.
    pub(crate) fn original_invocation_metadata_for_function<'a>(
        &'a self,
        tokens: &CommandTokens,
        function: &crate::compilation_unit::FunctionUnit,
        registry: &CommandRegistry,
    ) -> Option<crate::registry_invocation::InvocationMetadataSelection<'a>> {
        if let Some(input) = function.source_metadata_input() {
            function.invocation_metadata_context(registry)?;
            return self
                .original_invocation_metadata_for_input(tokens, input, registry)
                .map(crate::registry_invocation::InvocationMetadataSelection::Retained);
        }
        let config = self.original_invocation_config(tokens, registry)?;
        if config.nested().normalized() != function.source_lexer_config().nested().normalized() {
            return None;
        }
        let owner = &self.lookup_state.as_ref()?.state.baseline.metadata_context;
        owner.is_standalone().then_some(())?;
        owner.metadata_context(registry)
    }

    /// Join unchanged original source metadata to its full supplied input,
    /// config, origin and command store. A positively Logical declaration-only
    /// vector uses its sealed original-declaration purpose; a present reached
    /// owner remains terminal on refusal. Native input requires the point route.
    /// This grants no execution, handler completion, body entry or physical frame.
    pub(crate) fn original_invocation_metadata_for_input<'a>(
        &'a self,
        tokens: &CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &CommandRegistry,
    ) -> Option<InvocationMetadataContext<'a>> {
        self.original_word_metadata_for_input(
            tokens,
            input,
            registry,
            OriginalWordSourcePurpose::Invocation,
        )
    }

    /// Source-only argv geometry. Synthetic argv evaluation never replays the
    /// original invocation and cannot select its Normal handler or effects.
    pub(crate) fn original_argument_metadata_for_input<'a>(
        &'a self,
        tokens: &CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &CommandRegistry,
    ) -> Option<InvocationMetadataContext<'a>> {
        self.original_word_metadata_for_input(
            tokens,
            input,
            registry,
            OriginalWordSourcePurpose::ArgumentEvaluation,
        )
    }

    fn original_word_metadata_for_input<'a>(
        &'a self,
        tokens: &CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &CommandRegistry,
        purpose: OriginalWordSourcePurpose,
    ) -> Option<InvocationMetadataContext<'a>> {
        // A declaration-only Logical body retains its own full source owner.
        // This metadata projection does not manufacture a reached point, and
        // Native input cannot take the original-declaration purpose branch.
        if self.lookup_state.is_none() {
            input.has_logical_source_name_context().then_some(())?;
            let metadata = if purpose == OriginalWordSourcePurpose::ArgumentEvaluation {
                self.original_declaration_argument_metadata(tokens, registry)?
            } else {
                self.original_declaration_metadata_for_source(tokens, registry)?
            };
            let point = metadata.source_analysis_input()?;
            return (point == input || point == &input.for_nested_source()).then_some(metadata);
        }
        let config = self.original_word_config(tokens, registry, purpose)?;
        let point = self
            .lookup_state
            .as_ref()?
            .state
            .baseline
            .metadata_context
            .source_analysis_input()?;
        if point != input && point != &input.for_nested_source() {
            return None;
        }
        InvocationMetadataContext::for_source_input(
            registry,
            point,
            config,
            Some(point.unit_profile()),
        )
    }

    /// Descriptive realm of this complete original source vector under the
    /// current supplied input. A reached owner keeps its independently checked
    /// realm and cannot fall back on refusal. Only an absent reached owner may
    /// use positively Logical, closed, sealed original-declaration advice.
    /// This supplies no reached lookup, Native handler, body entry or frame.
    pub(crate) fn original_source_realm_for_input(
        &self,
        tokens: &CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &CommandRegistry,
    ) -> Option<tcl_dialect::model::InvocationRealm> {
        let metadata = self.original_invocation_metadata_for_input(tokens, input, registry)?;
        if let Some(point) = &self.lookup_state {
            return Some(point.realm);
        }
        (self.original_source_advice_purpose(tokens, metadata, registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical)
            .then_some(())?;
        let advice = self
            .declaration_operand_layout_advice(tokens)
            .or_else(|| self.original_compilation_lookup_advice(tokens))?;
        if !advice.closed_logical_source_lookup()
            || advice
                .original_declaration_metadata_context(registry, tokens)
                .ok()??
                .source_analysis_input()
                != metadata.source_analysis_input()
        {
            return None;
        }
        Some(advice.realm())
    }

    /// Check the request's existing tagged owner against this original point.
    /// `Ok(None)` is possible only when both owners explicitly retain Standalone.
    /// Missing, availability-only, stale or foreign supplied ownership is an error,
    /// never a scalar catalogue fallback. Geometry remains independently required.
    pub(crate) fn original_invocation_metadata_for_owner<'a>(
        &'a self,
        tokens: &CommandTokens,
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
        registry: &CommandRegistry,
    ) -> Result<
        Option<InvocationMetadataContext<'a>>,
        crate::registry_invocation::RegistryInvocationDecline,
    > {
        self.original_word_metadata_for_owner(
            tokens,
            owner,
            registry,
            OriginalWordSourcePurpose::Invocation,
        )
    }

    /// Retained CFG metadata for original argv evaluation alone. Availability,
    /// complete input and source geometry remain checked by the same owner.
    pub(crate) fn original_argument_metadata_for_owner<'a>(
        &'a self,
        tokens: &CommandTokens,
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
        registry: &CommandRegistry,
    ) -> Result<
        Option<InvocationMetadataContext<'a>>,
        crate::registry_invocation::RegistryInvocationDecline,
    > {
        self.original_word_metadata_for_owner(
            tokens,
            owner,
            registry,
            OriginalWordSourcePurpose::ArgumentEvaluation,
        )
    }

    fn original_word_metadata_for_owner<'a>(
        &'a self,
        tokens: &CommandTokens,
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
        registry: &CommandRegistry,
        purpose: OriginalWordSourcePurpose,
    ) -> Result<
        Option<InvocationMetadataContext<'a>>,
        crate::registry_invocation::RegistryInvocationDecline,
    > {
        use crate::registry_invocation::{
            OwnedInvocationMetadataContext, RegistryInvocationDecline,
        };
        let refusal = RegistryInvocationDecline::IncompleteResolution;
        match owner {
            OwnedInvocationMetadataContext::SuppliedSource(input) => self
                .original_word_metadata_for_input(tokens, input, registry, purpose)
                .map(Some)
                .ok_or(refusal),
            OwnedInvocationMetadataContext::Standalone => {
                self.original_word_config(tokens, registry, purpose)
                    .ok_or(refusal)?;
                let point = &self
                    .lookup_state
                    .as_ref()
                    .ok_or(refusal)?
                    .state
                    .baseline
                    .metadata_context;
                if !point.is_standalone() {
                    return Err(refusal);
                }
                point
                    .metadata_context(registry)
                    .map(crate::registry_invocation::InvocationMetadataSelection::context)
                    .ok_or(refusal)
            }
            OwnedInvocationMetadataContext::Supplied(_)
            | OwnedInvocationMetadataContext::Unavailable => Err(refusal),
        }
    }

    /// Reborrow metadata retained at this sealed original lookup point. This
    /// validates its complete vector and tagged source owner, not the currency
    /// of a later request. Request consumers must independently join their owner.
    /// Only explicit Standalone can return `Ok(None)`; partial or stale supplied
    /// ownership declines without reselecting a catalogue or native context.
    pub(crate) fn original_invocation_metadata_at_point(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> Result<
        Option<InvocationMetadataContext<'_>>,
        crate::registry_invocation::RegistryInvocationDecline,
    > {
        let owner = &self
            .lookup_state
            .as_ref()
            .ok_or(crate::registry_invocation::RegistryInvocationDecline::IncompleteResolution)?
            .state
            .baseline
            .metadata_context;
        self.original_invocation_metadata_for_owner(tokens, owner, registry)
    }

    /// Conditional source-advice metadata of this original vector. An actual
    /// reached point keeps its own checked route, including terminal refusal.
    /// Only its absence permits positively Logical, unanimous declaration
    /// observations to supply source metadata. Neither branch grants handler,
    /// frame, Native preparation, Normal completion or executable effects.
    pub(crate) fn original_source_metadata_for_advice(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> Result<
        Option<InvocationMetadataContext<'_>>,
        crate::registry_invocation::RegistryInvocationDecline,
    > {
        if self.lookup_state.is_some() {
            return self.original_invocation_metadata_at_point(tokens, registry);
        }
        self.original_declaration_metadata_for_source(tokens, registry)
            .map(Some)
            .ok_or(crate::registry_invocation::RegistryInvocationDecline::IncompleteResolution)
    }

    /// Common original point/vector geometry for actual FU and Module inputs.
    /// Availability and source ancestry are checked by their separate callers.
    fn original_invocation_config(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<LexerConfig> {
        self.original_word_config(tokens, registry, OriginalWordSourcePurpose::Invocation)
    }

    fn original_word_config(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
        purpose: OriginalWordSourcePurpose,
    ) -> Option<LexerConfig> {
        let snapshot = self.lookup_state.as_ref()?;
        let config = if purpose == OriginalWordSourcePurpose::ArgumentEvaluation {
            self.original_argument_lexer_config(tokens)?
        } else {
            self.original_lexer_config_for_tokens(tokens)?
        };
        let site = self.invocation_site()?;
        if snapshot.state.current_source_origin.as_ref() != Some(&site.source)
            || snapshot.state.baseline.registry_snapshot != Some(registry.snapshot().semantic_key())
        {
            return None;
        }
        site.source
            .original_lexical_command(tokens.words(), site.offset, config)?;
        Some(config)
    }

    /// Select this installer's retained point input under the genuine Module
    /// producer. Its separate closed footprint checks still own lookup and
    /// observer eligibility; no final Module bindings or profile are donors.
    pub(crate) fn original_materialized_footprint_for_module<'a>(
        &'a self,
        tokens: &CommandTokens,
        module: &crate::ir::Module,
        registry: &'a CommandRegistry,
    ) -> Option<OriginalSourceMaterializedFootprint<'a>> {
        // naming.diagnostic.original-materialized-write-footprint

        let metadata = self.original_invocation_metadata_for_module(tokens, module, registry)?;
        let source = self
            .invocation_site()?
            .source
            .source_image()
            .try_text()
            .ok()?;
        self.original_materialized_footprint(tokens, source, registry, Some(metadata))
    }

    /// Retain this genuine installer's closed source lookup independently of
    /// the child script's text. Missing source, context or namespace refuses.
    pub(crate) fn original_materialized_footprint<'a>(
        &'a self,
        tokens: &CommandTokens,
        source: &str,
        registry: &'a CommandRegistry,
        metadata: Option<InvocationMetadataContext<'a>>,
    ) -> Option<OriginalSourceMaterializedFootprint<'a>> {
        // naming.diagnostic.original-materialized-write-footprint

        let metadata = metadata.filter(|metadata| metadata.matches_registry(registry))?;
        let input = metadata.source_analysis_input()?;
        if self.lookup_state.is_none() {
            let result = self.original_logical_declaration_materialized_footprint(
                tokens, source, registry, metadata,
            );
            return result;
        }
        let metadata = self.original_invocation_metadata_for_input(tokens, input, registry);
        let metadata = metadata?;
        if metadata.source_analysis_input() != Some(input) {
            return None;
        }
        let config = self.original_lexer_config_for_tokens(tokens)?;
        let site = self.invocation_site()?;
        let snapshot = self.lookup_state.as_ref()?;
        let state = &snapshot.state;
        if self.unknown
            || self.may_be_absent
            || state.has_opaque_domain()
            || !state.command_observers.is_quiet()
            || state.source_step_observed()
            || state.current_source_origin.as_ref() != Some(&site.source)
            || site.source.source_image().try_text().ok() != Some(source)
            || state.baseline.registry_snapshot != Some(registry.snapshot().semantic_key())
            || config.normalized() != input.lexer_config().normalized()
            || state
                .baseline
                .logical_source_input
                .as_ref()
                .is_some_and(|original| original != input)
            || state
                .baseline
                .vendor_source_input
                .as_ref()
                .is_some_and(|original| original != input)
        {
            return None;
        }
        if state.baseline.logical_source_input.is_none()
            && state.baseline.vendor_source_input.is_none()
            && state.baseline.dialect
                != Some(super::source_analysis_entry::source_input_dialect(input))
        {
            return None;
        }
        let original = site
            .source
            .original_lexical_command(tokens.words(), site.offset, config)?;
        let head = original.native_words().first()?.clone();
        Some(OriginalSourceMaterializedFootprint {
            binding: self,
            lookup: OriginalMaterializedSourceLookup::Reached {
                state,
                namespace: &self.lookup_namespace_key,
            },
            registry,
            metadata,
            config: config.nested(),
            head,
        })
    }

    /// Possible text names under a positively Logical original declaration.
    /// The complete immutable source world and inert original operands retain
    /// the installer horizon; no reached point, body entry or store is issued.
    fn original_logical_declaration_materialized_footprint<'a>(
        &'a self,
        tokens: &CommandTokens,
        source: &str,
        registry: &'a CommandRegistry,
        request: InvocationMetadataContext<'_>,
    ) -> Option<OriginalSourceMaterializedFootprint<'a>> {
        use super::declaration_layout::original_declaration_layouts;
        let input = request.source_analysis_input()?;
        let metadata = self.original_declaration_metadata_for_source(tokens, registry)?;
        if metadata.source_analysis_input() != Some(input)
            || self.original_source_advice_purpose(tokens, metadata, registry)?
                != crate::command_binding::OriginalInvocationSourcePurpose::Logical
        {
            return None;
        }
        let advice = self.declaration_operand_layout_advice(tokens);
        let advice = advice?;
        if !advice.closed_logical_source_lookup()
            || advice
                .original_declaration_metadata_context(registry, tokens)
                .ok()??
                .source_analysis_input()
                != Some(input)
        {
            return None;
        }
        let config = self.original_lexer_config_for_tokens(tokens)?;
        let site = self.invocation_site()?;
        let state = &advice.snapshot.state;
        if config.normalized() != input.lexer_config().normalized()
            || site.source.source_image().try_text().ok() != Some(source)
            || state.current_source_origin.as_ref() != Some(&site.source)
            || state.baseline.registry_snapshot.as_ref()
                != Some(&registry.snapshot().semantic_key())
            || state.baseline.metadata_context.source_analysis_input() != Some(input)
            || state.has_opaque_domain()
            || !state.command_observers.is_quiet()
            || state.source_step_observed()
        {
            return None;
        }
        let rows = original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        if !rows.clone().all(|row| {
            row.entry.owns_source(&site.source, site.offset)
                && row.words.as_ref() == tokens.words()
                && row.config == config
                && row.namespace == *advice.namespace()
                && (std::sync::Arc::ptr_eq(&row.snapshot, &advice.snapshot)
                    || row.snapshot.as_ref() == advice.snapshot.as_ref())
                && advice.targets().iter().all(|target| {
                    target.registry_backed
                        && !row
                            .snapshot
                            .state
                            .source_execution_observed(target.identity.as_ref())
                        && !row
                            .snapshot
                            .state
                            .runtime_execution_observed(target.identity.as_ref())
                })
                && self.original_operands_preserve_lookup(tokens, registry, row, 0)
        }) {
            return None;
        }
        let original = site
            .source
            .original_lexical_command(tokens.words(), site.offset, config)?;
        let head = original.native_words().first()?.clone();
        Some(OriginalSourceMaterializedFootprint {
            binding: self,
            lookup: OriginalMaterializedSourceLookup::LogicalDeclaration {
                snapshot: advice.snapshot,
                namespace: advice.namespace,
            },
            registry,
            metadata,
            config: config.nested(),
            head,
        })
    }
}

fn original_module_origin(origin: &super::SourceOriginId, module: &crate::ir::Module) -> bool {
    let mut origin = origin;
    while let super::SourceOriginKind::Derived { parent, .. } = origin.kind() {
        origin = &parent.source;
    }
    origin.source_image() == &module.source
}

impl<'a> OriginalSourceMaterializedFootprint<'a> {
    /// The original installer world, independently of final Module state.
    /// A Logical declaration world remains conditional source advice.
    pub(crate) fn bindings(&self) -> &ModuleCommandBindings {
        self.lookup.state()
    }

    fn namespace(&self) -> &SourceNamespaceKey {
        self.lookup.namespace()
    }

    /// Actual complete point input and availability retained by the issuer.
    pub(crate) const fn metadata_context(&self) -> InvocationMetadataContext<'a> {
        self.metadata
    }

    /// Exact nested lexical policy for materialized source-only inventories.
    pub(crate) const fn lexer_config(&self) -> LexerConfig {
        self.config
    }

    /// Only an authored global-frame callback recipe has a source coordinate
    /// here. Future triggering/invoking frames require their separate owners.
    pub(crate) fn deferred_namespace(
        &self,
        scope: Option<tcl_registry::ScriptLookupScope>,
    ) -> Option<crate::ir::ExecutionNamespace> {
        if scope != Some(tcl_registry::ScriptLookupScope::GlobalFrame) {
            return None;
        }
        Some(crate::ir::ExecutionNamespace::SourceContext(
            self.bindings().source_root_namespace_key()?,
        ))
    }

    /// Possible local output names, preserving current aliases and replacements.
    /// The materialized value receives no original child-source geometry.
    pub(crate) fn script_writes(&self, script: &str) -> crate::ir_helpers::VariableWriteEffects {
        crate::ir_helpers::script_value_possible_writes_with_metadata_context(
            script,
            self.registry,
            self.bindings(),
            &crate::ir::ExecutionNamespace::SourceContext(self.namespace().clone()),
            Some(self.metadata),
            self.config,
        )
    }

    /// Possible local bindings and by-name reads in an original whole body.
    /// The selected interpreter must be the installer's current interpreter;
    /// named children require their independently retained command world.
    pub(crate) fn body_name_ownership(
        &self,
        body: &crate::registry_invocation::OriginalSourceScriptBody,
    ) -> crate::ir_helpers::VariableWriteEffects {
        let unavailable = || crate::ir_helpers::VariableWriteEffects {
            opaque: true,
            ..Default::default()
        };
        let Some(input) = self.metadata.source_analysis_input() else {
            return unavailable();
        };
        let context = input.context_registry();
        let words = body.source_words();
        if words.head_source().and_then(|head| head.word()) != Some(&self.head)
            || !body.matches_source(self.head.image(), input.lexer_config())
            || !body.matches_context(&context)
            || body.original_container().content_span().ok() != Some(body.content_span())
        {
            return unavailable();
        }
        let current = words.with_source_schema(&context, |schema| {
            schema.semantics.body_interpreter.resolve_with(|ordinal| {
                std::str::from_utf8(words.arguments().get(ordinal)?.literal_bytes()?).ok()
            }) == tcl_registry::InterpreterScope::Current
        });
        if current != Some(true) {
            return unavailable();
        }
        let Some(value) =
            words
                .operands()
                .iter()
                .zip(words.arguments())
                .find_map(|(operand, value)| {
                    (operand.as_ref().and_then(|operand| operand.word())
                        == Some(body.original_container()))
                    .then(|| {
                        std::str::from_utf8(value.literal_bytes()?)
                            .ok()
                            .map(str::to_owned)
                    })
                    .flatten()
                })
        else {
            return unavailable();
        };
        crate::ir_helpers::script_value_name_ownership_with_metadata_context(
            &value,
            self.registry,
            self.bindings(),
            &crate::ir::ExecutionNamespace::SourceContext(self.namespace().clone()),
            self.metadata,
            self.config,
            crate::script_binds::Ownership::BindingsOrNameReads,
        )
    }

    /// Lexical ownership in one unchanged, literal written body operand.
    /// Selected immediate/current-interpreter roles retain aliases and their
    /// captured argument offset. This supplies no original child words or frame.
    pub(crate) fn literal_body_name_ownership(
        &self,
        tokens: &CommandTokens,
        written_argument: usize,
        purpose: crate::script_binds::Ownership,
    ) -> Option<crate::ir_helpers::VariableWriteEffects> {
        let (command, original) = self.binding.original_recorded_command()?;
        if original.words() != tokens.words() || tokens.synthetic.is_some() {
            return None;
        }
        let config = self.binding.original_lexer_config_for_tokens(tokens)?;
        let value = crate::registry_invocation::effective_invocation_word(
            tokens.words().get(written_argument.checked_add(1)?)?,
            config.escapes,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
        );
        let crate::registry_invocation::EffectiveInvocationWord::Literal(value) = value else {
            return None;
        };
        let words = crate::ir_helpers::footprint_command_words(
            &tcl_lexer::SourceMap::from_image(self.head.image()),
            config,
            &command,
        );
        let logical_source = self
            .bindings()
            .retained_original_source_purpose(self.metadata, self.registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
        let mut selected = false;
        let mut accepted = true;
        self.bindings().for_each_resolved_command_words(&words, self.namespace(), |target, invocation| {
            selected = true;
            if !target.registry_backed { accepted = false; return; }
            let resolution = tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.registry, Some(self.metadata.context()), invocation, self.bindings().invocation_realm(),
            );
            let Some(schema) = resolution.resolved() else { accepted = false; return; };
            let Some(argument) = written_argument.checked_add(target.prepended.len()) else {
                accepted = false; return;
            };
            accepted &= schema.semantics.body_kind == tcl_registry::BodyKind::Plain
                && schema.semantics.body_interpreter.resolve(schema.words.arguments())
                    == tcl_registry::InterpreterScope::Current
                && (if logical_source {
                    schema.authored_logical_source_plain_script_arguments()
                } else { schema.authored_source_plain_script_arguments() })
                    .is_some_and(|arguments| arguments.contains(&argument))
                && schema.authored_source_script_timing_at(argument)
                    == Some(tcl_registry::ScriptTiming::SameInvocation);
        });
        (selected && accepted).then(|| {
            crate::ir_helpers::script_value_name_ownership_with_metadata_context(
                &value,
                self.registry,
                self.bindings(),
                &crate::ir::ExecutionNamespace::SourceContext(self.namespace().clone()),
                self.metadata,
                config.nested(),
                purpose,
            )
        })
    }

    /// Conditional by-name and re-evaluated-body reads of this exact original
    /// command. Its own lookup horizon supplies the alias target; no newly
    /// parsed child command receives an original source or entered frame.
    pub(crate) fn invocation_reads(&self) -> crate::ir_helpers::VariableWriteEffects {
        self.read_footprint(true)
    }

    /// Reads in selected re-evaluated expressions and immediate source values.
    pub(crate) fn reevaluated_reads(&self) -> crate::ir_helpers::VariableWriteEffects {
        self.read_footprint(false)
    }

    /// Possible names of this selected original command, including templates
    /// re-evaluated under its own retained source lookup and grammar.
    pub(crate) fn invocation_footprint(&self) -> crate::ir_helpers::VariableWriteEffects {
        self.command_footprint(true, true)
    }

    fn read_footprint(&self, include_invocation: bool) -> crate::ir_helpers::VariableWriteEffects {
        self.command_footprint(include_invocation, false)
    }

    fn command_footprint(
        &self,
        include_invocation: bool,
        include_writes: bool,
    ) -> crate::ir_helpers::VariableWriteEffects {
        let Some((command, tokens)) = self.binding.original_recorded_command() else {
            return crate::ir_helpers::VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        };
        let image = self.head.image();
        let map = tcl_lexer::SourceMap::from_image(image);
        let Some(config) = self.binding.original_lexer_config_for_tokens(&tokens) else {
            return crate::ir_helpers::VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        };
        let Some(site) = self.binding.invocation_site() else {
            return crate::ir_helpers::VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        };
        let Some(original) =
            site.source
                .original_lexical_command(tokens.words(), site.offset, config)
        else {
            return crate::ir_helpers::VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        };
        let native = original.native_words();
        if native.first() != Some(&self.head) {
            return crate::ir_helpers::VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        }
        let words = crate::ir_helpers::footprint_command_words(&map, config, &command);
        if include_writes {
            return crate::ir_helpers::command_possible_footprint_with_metadata_context(
                &words,
                self.registry,
                self.bindings(),
                &crate::ir::ExecutionNamespace::SourceContext(self.namespace().clone()),
                self.metadata,
                self.config,
            );
        }
        crate::ir_helpers::command_possible_reads_with_metadata_context(
            &words,
            self.registry,
            self.bindings(),
            &crate::ir::ExecutionNamespace::SourceContext(self.namespace().clone()),
            self.metadata,
            self.config,
            include_invocation,
        )
    }

    /// Possible writes of the exact selected original invocation and its
    /// immediate same-frame script values. The source head is revalidated;
    /// captured values do not acquire a child word or entered frame.
    pub(crate) fn invocation_writes(
        &self,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> crate::ir_helpers::VariableWriteEffects {
        let input = self
            .metadata
            .source_analysis_input()
            .expect("sealed complete input");
        if words.head_source().and_then(|head| head.word()) != Some(&self.head)
            || !words.matches_source(self.head.image(), input.lexer_config())
        {
            return crate::ir_helpers::VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        }
        let context = input.context_registry();
        let Some((projection, bodies)) = words.with_source_schema(&context, |schema| {
            (
                tcl_registry::CommandRegistry::variable_write_projection_for_selected_source(
                    schema,
                ),
                crate::ir_helpers::immediate_same_frame_script_values(schema),
            )
        }) else {
            return crate::ir_helpers::VariableWriteEffects {
                opaque: true,
                ..Default::default()
            };
        };
        let mut writes = crate::ir_helpers::VariableWriteEffects {
            names: projection.literal_names,
            read_names: projection.read_before_write_names,
            opaque: projection.opaque_variable_frame,
        };
        for body in bodies {
            writes.include_possible(&self.script_writes(&body));
        }
        writes
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::ResolvedAnalysisInput;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use crate::registry_invocation::InvocationMetadataContext;
    use std::sync::Arc;

    fn unit(source: &str, context: &Arc<tcl_registry::model::ContextRegistry>) -> CompilationUnit {
        let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
        CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        )
    }

    #[test]
    fn original_module_metadata_preserves_explicit_standalone_without_supplied_fallback() {
        use crate::registry_invocation::OwnedInvocationMetadataContext;
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        // A profiled default build is a fresh-interpreter document;
        // explicit Standalone authoring is the
        // dialect-blind build, still profiled through its Registry.
        let fresh =
            CompilationUnit::build_for_profile("set value ORIGINAL", &registry, false, profile);
        assert!(fresh.ir_module.source_metadata_input.is_some());
        let standalone = CompilationUnit::build_for_with_config(
            "set value ORIGINAL",
            &registry,
            false,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        );
        let module = &standalone.ir_module;
        assert!(module.source_metadata_input.is_none());
        assert!(module.source_entry.metadata_context.is_standalone());
        assert!(module.source_entry.native_entry.is_none());
        let script = &module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let metadata = binding
            .original_invocation_metadata_for_module(tokens, module, &registry)
            .expect("actual retained standalone Module and original point");
        assert!(metadata.is_standalone());
        assert!(metadata.source_analysis_input().is_none());
        assert!(
            crate::registry_invocation::admitted_native_compiler_invocation(
                &registry, None, tokens
            )
            .is_none()
        );
        for change in 0..4 {
            let mut changed = module.clone();
            match change {
                0 => changed.source_entry.unknown_entry = true,
                1 => {
                    changed.source_entry.metadata_context =
                        OwnedInvocationMetadataContext::Unavailable
                }
                2 => changed.lexer_config.strict_quoting = !changed.lexer_config.strict_quoting,
                3 => changed.source = tcl_lexer::SourceImage::document("set value CHANGED"),
                _ => unreachable!(),
            }
            assert!(
                binding
                    .original_invocation_metadata_for_module(tokens, &changed, &registry)
                    .is_none()
            );
        }
        let foreign_profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile();
        let foreign =
            tcl_registry::CommandRegistry::build_default().project_for_profile(foreign_profile);
        assert_ne!(
            registry.snapshot().semantic_key(),
            foreign.snapshot().semantic_key()
        );
        assert!(
            binding
                .original_invocation_metadata_for_module(tokens, module, &foreign)
                .is_none()
        );
        let mut changed_tokens = tokens.clone();
        changed_tokens.argv_texts[1] = "changed".into();
        assert!(
            binding
                .original_invocation_metadata_for_module(&changed_tokens, module, &registry)
                .is_none()
        );

        let context =
            tcl_registry::model::ingress::resolve_environment("tcl").default_context_registry();
        let supplied = unit("set value ORIGINAL", &context);
        let script = &supplied.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let mut missing = supplied.ir_module.clone();
        missing.source_metadata_input = None;
        missing.source_entry.metadata_context = OwnedInvocationMetadataContext::Standalone;
        assert!(
            binding
                .original_invocation_metadata_for_module(tokens, &missing, context.commands())
                .is_none()
        );
    }

    #[test]
    fn original_tagged_point_selection_keeps_supplied_refusal_distinct_from_standalone() {
        // naming.compiler.original-analysis-metadata-context

        use crate::registry_invocation::OwnedInvocationMetadataContext;
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = unit("eval set ::original VALUE", &context);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
        let owner = OwnedInvocationMetadataContext::for_source_input(Some(input));
        let metadata = binding
            .original_invocation_metadata_for_owner(tokens, &owner, context.commands())
            .unwrap()
            .unwrap();
        assert_eq!(metadata.source_analysis_input(), Some(input));
        assert!(
            binding
                .original_invocation_metadata_for_owner(
                    tokens,
                    &OwnedInvocationMetadataContext::Standalone,
                    context.commands()
                )
                .is_err()
        );
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(context.commands())),
        );
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        for changed_context in [older, foreign] {
            let changed = ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                changed_context,
                input.lexer_config(),
            );
            assert!(
                binding
                    .original_invocation_metadata_for_owner(
                        tokens,
                        &OwnedInvocationMetadataContext::for_source_input(Some(&changed)),
                        context.commands()
                    )
                    .is_err()
            );
        }
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        let changed = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::clone(&context),
            config,
        );
        for owner in [
            OwnedInvocationMetadataContext::Unavailable,
            OwnedInvocationMetadataContext::Supplied(Arc::clone(&context)),
            OwnedInvocationMetadataContext::for_source_input(Some(&changed)),
        ] {
            assert!(
                binding
                    .original_invocation_metadata_for_owner(tokens, &owner, context.commands())
                    .is_err()
            );
        }
        let mut changed = tokens.clone();
        changed.word_exprs.pop();
        assert!(
            binding
                .original_invocation_metadata_for_owner(&changed, &owner, context.commands())
                .is_err()
        );
        let registry = tcl_registry::CommandRegistry::build_default();
        let standalone =
            CompilationUnit::build_for("missing_original_command VALUE", &registry, false);
        let script = &standalone.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(matches!(
            binding.original_invocation_metadata_for_owner(
                tokens,
                &OwnedInvocationMetadataContext::Standalone,
                &registry
            ),
            Ok(None)
        ));
        assert!(
            binding
                .original_invocation_metadata_for_owner(
                    tokens,
                    &OwnedInvocationMetadataContext::Unavailable,
                    &registry
                )
                .is_err()
        );
    }

    #[test]
    fn original_declaration_flow_keeps_selected_point_availability_and_supplied_refusal() {
        // naming.compiler.original-analysis-metadata-context

        // Conditional setter metadata supplies no successful store or physical frame.
        use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
        use crate::registry_invocation::OwnedInvocationMetadataContext;
        let mut registry = tcl_registry::CommandRegistry::build_default();
        let setter = registry.get("set").unwrap().clone();
        registry.insert(tcl_registry::CommandSpec {
            name: "metadata_write",
            surface: registry.get("dict").unwrap().surface,
            ..setter
        });
        let current = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl9.0")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        assert_eq!(
            current.commands().snapshot().semantic_key(),
            older.commands().snapshot().semantic_key()
        );
        let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let source = "proc p {} {metadata_write output VALUE}; p";
        let offset = u32::try_from(source.find("metadata_write").unwrap()).unwrap();
        let original = |context: &Arc<tcl_registry::model::ContextRegistry>| {
            let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
            let options = SourceAnalysisOptions::for_logical_source(&input).unwrap();
            let inventory =
                SourceCommandBindings::analyse_with_options(source, config, &registry, options);
            let origin = inventory.root_origin.as_ref().unwrap();
            let entry = inventory.conditional_body_entry_at(origin, offset).unwrap();
            let tokens = inventory
                .declaration_original_tokens_at(&entry, offset)
                .unwrap();
            (tokens, input)
        };
        let (tokens, input) = original(&current);
        let binding = tokens.source_binding.as_ref().unwrap();
        let metadata = binding
            .original_invocation_metadata_at_point(&tokens, &registry)
            .unwrap()
            .unwrap();
        assert_eq!(metadata.source_analysis_input(), Some(&input));
        let advice = binding.declaration_operand_layout_advice(&tokens).unwrap();
        let selected =
            crate::registry_invocation::declaration_invocation_flow(&registry, &tokens, &advice)
                .unwrap();
        assert_eq!(
            selected.literal_stores,
            vec![("output".to_owned(), "VALUE".to_owned())]
        );
        let (older_tokens, older_input) = original(&older);
        let older_binding = older_tokens.source_binding.as_ref().unwrap();
        assert_eq!(
            older_binding
                .original_invocation_metadata_at_point(&older_tokens, &registry)
                .unwrap()
                .unwrap()
                .source_analysis_input(),
            Some(&older_input)
        );
        assert!(
            older_binding
                .declaration_operand_layout_advice(&older_tokens)
                .and_then(|advice| {
                    crate::registry_invocation::declaration_invocation_flow(
                        &registry,
                        &older_tokens,
                        &advice,
                    )
                })
                .is_none()
        );
        let mut missing = tokens.clone();
        let missing_binding = missing.source_binding.as_mut().unwrap();
        let point = Arc::make_mut(missing_binding.lookup_state.as_mut().unwrap());
        Arc::make_mut(&mut point.state.baseline).metadata_context =
            OwnedInvocationMetadataContext::Unavailable;
        assert!(
            missing
                .source_binding
                .as_ref()
                .unwrap()
                .original_invocation_metadata_at_point(&missing, &registry)
                .is_err()
        );
        assert!(
            crate::registry_invocation::declaration_invocation_flow(&registry, &missing, &advice)
                .is_none()
        );
        let mut changed_config = config;
        changed_config.strict_quoting = !changed_config.strict_quoting;
        let changed_input =
            ResolvedAnalysisInput::new(profile, profile, Arc::clone(&current), changed_config);
        let mut stale = tokens.clone();
        let point = Arc::make_mut(
            stale
                .source_binding
                .as_mut()
                .unwrap()
                .lookup_state
                .as_mut()
                .unwrap(),
        );
        Arc::make_mut(&mut point.state.baseline).metadata_context =
            OwnedInvocationMetadataContext::for_source_input(Some(&changed_input));
        assert!(
            stale
                .source_binding
                .as_ref()
                .unwrap()
                .original_invocation_metadata_at_point(&stale, &registry)
                .is_err()
        );
        let mut changed = tokens.clone();
        changed.word_exprs.pop();
        assert!(
            binding
                .original_invocation_metadata_at_point(&changed, &registry)
                .is_err()
        );
        assert!(
            crate::registry_invocation::declaration_invocation_flow(&registry, &changed, &advice)
                .is_none()
        );
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        assert!(
            binding
                .original_invocation_metadata_at_point(&tokens, foreign.commands())
                .is_err()
        );
        for owner in [
            OwnedInvocationMetadataContext::Unavailable,
            OwnedInvocationMetadataContext::Supplied(Arc::clone(&current)),
            OwnedInvocationMetadataContext::for_source_input(Some(&older_input)),
        ] {
            assert!(
                binding
                    .original_invocation_metadata_for_owner(&tokens, &owner, &registry)
                    .is_err()
            );
        }
    }

    #[test]
    fn original_caller_alias_operands_keep_captured_geometry_and_selected_metadata() {
        // naming.compiler.original-analysis-metadata-context

        // Conditional formal mentions only: no reached upvar or caller cell.
        use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
        use crate::registry_invocation::{InvocationWordOrigin, OwnedInvocationMetadataContext};
        let mut registry = tcl_registry::CommandRegistry::build_default();
        let link = registry.get("upvar").unwrap().clone();
        registry.insert(tcl_registry::CommandSpec {
            name: "metadata_link",
            surface: registry.get("dict").unwrap().surface,
            ..link
        });
        let store = registry.snapshot().shared_registry();
        let current = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl9.0")
                .with_command_store(Arc::clone(&store)),
        );
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4").with_command_store(store),
        );
        assert_eq!(
            current.commands().snapshot().semantic_key(),
            older.commands().snapshot().semantic_key()
        );
        let mut foreign_registry = tcl_registry::CommandRegistry::build_default();
        foreign_registry.insert(tcl_registry::CommandSpec {
            name: "foreign_caller_alias_axis",
            ..foreign_registry.get("set").unwrap().clone()
        });
        let foreign = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl9.0")
                .with_command_store(foreign_registry.snapshot().shared_registry()),
        );
        assert_ne!(
            current.commands().snapshot().semantic_key(),
            foreign.commands().snapshot().semantic_key()
        );
        let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        for (source, call, written, captured) in [
            (
                "proc p {name} {metadata_link 1 $name local}; p target",
                "metadata_link 1 $name local",
                2,
                false,
            ),
            (
                "interp alias {} link {} metadata_link 1; proc p {name} {link $name local}; p target",
                "link $name local",
                1,
                true,
            ),
        ] {
            let offset = u32::try_from(source.find(call).unwrap()).unwrap();
            let original = |context: &Arc<tcl_registry::model::ContextRegistry>| {
                let input =
                    ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
                let options = SourceAnalysisOptions::for_logical_source(&input).unwrap();
                let inventory =
                    SourceCommandBindings::analyse_with_options(source, config, &registry, options);
                let origin = inventory.root_origin.as_ref().unwrap();
                let entry = inventory.conditional_body_entry_at(origin, offset).unwrap();
                let tokens = inventory
                    .declaration_original_tokens_at(&entry, offset)
                    .unwrap();
                (tokens, input)
            };
            let (tokens, input) = original(&current);
            let binding = tokens.source_binding.as_ref().unwrap();
            let metadata = binding
                .original_invocation_metadata_at_point(&tokens, &registry)
                .unwrap()
                .unwrap();
            assert_eq!(metadata.source_analysis_input(), Some(&input));
            let advice = binding.declaration_operand_layout_advice(&tokens).unwrap();
            let selected = crate::registry_invocation::declaration_invocation_flow(
                &registry, &tokens, &advice,
            )
            .expect("genuine original caller-alias source recipe");
            assert_eq!(
                selected.caller_alias_operands,
                vec![tokens.words()[written].clone()]
            );
            assert_eq!(
                selected.effective.origins[2],
                InvocationWordOrigin::Written(written)
            );
            assert_eq!(
                matches!(
                    selected.effective.origins[1],
                    InvocationWordOrigin::BindingPrefix(_)
                ),
                captured,
            );
            assert_eq!(binding.logical_source_name_advice_input(), Some(&input));
            assert!(advice.dialect().native_name_protocol().is_none());
            assert!(
                SourceAnalysisOptions::for_logical_source(&input)
                    .unwrap()
                    .native_entry
                    .is_none()
            );

            let (older_tokens, _) = original(&older);
            assert!(
                older_tokens
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .declaration_operand_layout_advice(&older_tokens)
                    .and_then(
                        |advice| crate::registry_invocation::declaration_invocation_flow(
                            &registry,
                            &older_tokens,
                            &advice,
                        )
                    )
                    .is_none()
            );
            let changed_grammar = ResolvedAnalysisInput::new(
                profile,
                profile,
                Arc::clone(&current),
                tcl_lexer::LexerConfig {
                    strict_quoting: !config.strict_quoting,
                    ..config
                },
            );
            let foreign_input =
                ResolvedAnalysisInput::new(profile, profile, Arc::clone(&foreign), config);
            for owner in [
                OwnedInvocationMetadataContext::Unavailable,
                OwnedInvocationMetadataContext::Supplied(Arc::clone(&current)),
                OwnedInvocationMetadataContext::for_source_input(Some(&changed_grammar)),
                OwnedInvocationMetadataContext::for_source_input(Some(&foreign_input)),
            ] {
                let mut changed = tokens.clone();
                let point = Arc::make_mut(
                    changed
                        .source_binding
                        .as_mut()
                        .unwrap()
                        .lookup_state
                        .as_mut()
                        .unwrap(),
                );
                Arc::make_mut(&mut point.state.baseline).metadata_context = owner;
                assert!(
                    changed
                        .source_binding
                        .as_ref()
                        .unwrap()
                        .original_invocation_metadata_at_point(&changed, &registry)
                        .is_err()
                );
                assert!(
                    crate::registry_invocation::declaration_invocation_flow(
                        &registry, &changed, &advice,
                    )
                    .is_none()
                );
            }
            let mut changed_vector = tokens.clone();
            changed_vector.word_exprs.pop();
            assert!(
                crate::registry_invocation::declaration_invocation_flow(
                    &registry,
                    &changed_vector,
                    &advice,
                )
                .is_none()
            );
        }
    }

    #[test]
    fn warmed_original_lexical_projection_cannot_donate_changed_module_metadata() {
        // naming.compiler.original-analysis-metadata-context

        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = unit("eval set ::original VALUE", &context);
        let module = &unit.ir_module;
        let script = &module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let site = binding.invocation_site().unwrap();
        let config = binding.original_lexer_config_for_tokens(tokens).unwrap();
        let first = site
            .source
            .original_lexical_command(tokens.words(), site.offset, config)
            .unwrap();
        let count = site.source.original_lexical_projection().capture_count();
        let second = site
            .source
            .original_lexical_command(tokens.words(), site.offset, config)
            .unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(
            site.source.original_lexical_projection().capture_count(),
            count
        );
        assert!(
            binding
                .original_invocation_metadata_for_module(tokens, module, context.commands())
                .is_some()
        );
        let input = module.source_metadata_input.as_ref().unwrap();
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(context.commands())),
        );
        assert!(Arc::ptr_eq(older.commands(), context.commands()));
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        assert!(!Arc::ptr_eq(foreign.commands(), context.commands()));
        for change in 0..5 {
            let mut changed = module.clone();
            match change {
                0 => changed.source_metadata_input = None,
                1 | 2 => {
                    changed.source_metadata_input = Some(ResolvedAnalysisInput::new(
                        input.analyser_profile(),
                        input.unit_profile(),
                        if change == 1 {
                            Arc::clone(&older)
                        } else {
                            Arc::clone(&foreign)
                        },
                        input.lexer_config(),
                    ))
                }
                3 => changed.lexer_config.strict_quoting = !changed.lexer_config.strict_quoting,
                4 => changed.source = tcl_lexer::SourceImage::document("eval set ::changed VALUE"),
                _ => unreachable!(),
            }
            assert!(
                binding
                    .original_invocation_metadata_for_module(tokens, &changed, context.commands())
                    .is_none(),
                "changed owner {change}"
            );
        }
    }

    #[test]
    fn original_installer_module_projection_keeps_snapshot_and_future_scope_refusal() {
        // naming.diagnostic.original-materialized-write-footprint

        // Software source snapshot/lookup controls, not callback or frame entry.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = unit("eval set ::early VALUE; proc set args {}", &context);
        let module = &unit.ir_module;
        let script = &module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert_eq!(
            binding
                .original_invocation_metadata_for_module(tokens, module, context.commands())
                .unwrap()
                .source_analysis_input(),
            module.source_metadata_input.as_ref(),
        );
        let footprint = binding
            .original_materialized_footprint_for_module(tokens, module, context.commands())
            .unwrap();
        assert!(std::ptr::eq(
            footprint.bindings(),
            &binding.lookup_state.as_ref().unwrap().state
        ));
        assert_eq!(
            footprint.metadata_context().source_analysis_input(),
            module.source_metadata_input.as_ref()
        );
        assert_eq!(footprint.lexer_config(), module.lexer_config.nested());
        assert!(
            footprint
                .script_writes("set ::observed VALUE")
                .names
                .iter()
                .any(|name| name == "::observed")
        );
        assert!(matches!(
            footprint.deferred_namespace(Some(tcl_registry::ScriptLookupScope::GlobalFrame)),
            Some(crate::ir::ExecutionNamespace::SourceContext(_)),
        ));
        for scope in [
            None,
            Some(tcl_registry::ScriptLookupScope::InvokingFrame),
            Some(tcl_registry::ScriptLookupScope::TriggerFrame),
        ] {
            assert!(footprint.deferred_namespace(scope).is_none());
        }
        let mut refused = module.clone();
        refused.source_metadata_input = None;
        assert!(
            binding
                .original_materialized_footprint_for_module(tokens, &refused, context.commands())
                .is_none()
        );
        refused = module.clone();
        refused.lexer_config.expand_syntax = !refused.lexer_config.expand_syntax;
        assert!(
            binding
                .original_materialized_footprint_for_module(tokens, &refused, context.commands())
                .is_none()
        );
        refused = module.clone();
        refused.source = tcl_lexer::SourceImage::document("eval set ::different VALUE");
        assert!(
            binding
                .original_materialized_footprint_for_module(tokens, &refused, context.commands())
                .is_none()
        );
        let original = module.source_metadata_input.as_ref().unwrap();
        refused = module.clone();
        refused.source_metadata_input = Some(ResolvedAnalysisInput::new(
            original.analyser_profile(),
            original.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            original.lexer_config(),
        ));
        assert!(
            binding
                .original_materialized_footprint_for_module(tokens, &refused, context.commands())
                .is_none()
        );
    }

    #[test]
    fn original_installer_module_projection_withdraws_changed_entry_and_frame_owner() {
        // naming.diagnostic.original-materialized-write-footprint

        // Full source-owner reuse controls, independently of physical frame entry.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = unit("eval set ::original VALUE", &context);
        let module = &unit.ir_module;
        let owner = module.retained_source_bindings.as_ref().unwrap();
        assert!(owner.matches_module(module, context.commands()));
        let script = &module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(
            binding
                .original_materialized_footprint_for_module(tokens, module, context.commands())
                .is_some()
        );
        for change in 0..7 {
            let mut changed = module.clone();
            match change {
                0 => changed.retained_source_bindings = None,
                1 => changed.source_entry.unknown_entry = !changed.source_entry.unknown_entry,
                2 => {
                    changed.source_entry.compilation_scope =
                        tcl_runtime_api::SourceCompilationScope::EnteredSource
                }
                3 => changed.top_level_namespace = "::other".into(),
                4 => {
                    changed.native_namespace =
                        Some(tcl_core_types::ByteNamespacePath::from_segments(["other"]))
                }
                5 => changed.top_level_kind = crate::ir::TopLevelKind::ProcedureBody,
                6 => changed.source_entry.incoming_formals.push("new".into()),
                _ => unreachable!(),
            }
            assert!(InvocationMetadataContext::for_module(context.commands(), &changed).is_some());
            assert!(
                binding
                    .original_invocation_metadata_for_module(tokens, &changed, context.commands())
                    .is_none()
            );
            assert!(
                binding
                    .original_materialized_footprint_for_module(
                        tokens,
                        &changed,
                        context.commands()
                    )
                    .is_none(),
                "change {change} retained stale source interpretation"
            );
        }
    }

    #[test]
    fn original_installer_module_projection_accepts_only_the_retained_nested_input() {
        // naming.diagnostic.original-materialized-write-footprint

        // Independently produced source points retain their explicit nested policy.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let source = "eval set ::nested VALUE";
        let root = unit(source, &context);
        let original = root.ir_module.source_metadata_input.as_ref().unwrap();
        let input = original.for_nested_source();
        let nested = CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        let script = &nested.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert_eq!(
            binding
                .original_invocation_metadata_for_module(
                    tokens,
                    &root.ir_module,
                    context.commands()
                )
                .unwrap()
                .source_analysis_input(),
            Some(&input),
        );
        let footprint = binding
            .original_materialized_footprint_for_module(tokens, &root.ir_module, context.commands())
            .unwrap();
        assert_eq!(
            footprint.metadata_context().source_analysis_input(),
            Some(&input)
        );
        assert!(
            footprint
                .script_writes("set ::observed VALUE")
                .names
                .iter()
                .any(|name| name == "::observed")
        );
        let mut changed = root.ir_module.clone();
        let mut config = original.lexer_config();
        config.expand_syntax = !config.expand_syntax;
        changed.lexer_config = config;
        changed.source_metadata_input = Some(ResolvedAnalysisInput::new(
            original.analyser_profile(),
            original.unit_profile(),
            Arc::clone(&context),
            config,
        ));
        assert!(InvocationMetadataContext::for_module(context.commands(), &changed).is_some());
        assert!(
            binding
                .original_materialized_footprint_for_module(tokens, &changed, context.commands())
                .is_none()
        );
    }

    #[test]
    fn original_point_metadata_keeps_unknown_lookup_and_changed_vector_separate() {
        // naming.diagnostic.original-materialized-write-footprint

        // Source/API ownership only: metadata does not prove a reached command.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = unit("missing_original_command VALUE", &context);
        let module = &unit.ir_module;
        let script = &module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.unknown || binding.may_be_absent);
        assert_eq!(
            binding
                .original_invocation_metadata_for_module(tokens, module, context.commands())
                .unwrap()
                .source_analysis_input(),
            module.source_metadata_input.as_ref(),
        );
        assert!(
            binding
                .original_materialized_footprint_for_module(tokens, module, context.commands())
                .is_none()
        );
        let mut changed = tokens.clone();
        changed.word_exprs.pop();
        assert!(
            binding
                .original_invocation_metadata_for_module(&changed, module, context.commands())
                .is_none()
        );
        let mut missing = module.clone();
        missing.source_metadata_input = None;
        assert!(
            binding
                .original_invocation_metadata_for_module(tokens, &missing, context.commands())
                .is_none()
        );
    }

    #[test]
    fn original_function_point_metadata_retains_input_without_granting_module_or_normality() {
        // naming.compiler.original-analysis-metadata-context

        // Actual FU/point metadata is distinct from whole-Module currency and
        // from a reached handler, successful read, physical frame or result.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = unit("missing_original_command VALUE", &context);
        let function = &unit.top_level;
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.unknown || binding.may_be_absent);
        let metadata = binding
            .original_invocation_metadata_for_function(tokens, function, context.commands())
            .unwrap()
            .context()
            .unwrap();
        assert_eq!(
            metadata.source_analysis_input(),
            function.source_metadata_input()
        );
        assert!(
            crate::registry_invocation::normal_representation_invocation_with_metadata_context(
                context.commands(),
                Some(metadata),
                tokens,
            )
            .is_none()
        );
        let mut changed = unit.ir_module.clone();
        changed.top_level_namespace = "::changed".into();
        assert!(
            binding
                .original_invocation_metadata_for_module(tokens, &changed, context.commands())
                .is_none()
        );
        assert!(
            binding
                .original_invocation_metadata_for_function(tokens, function, context.commands())
                .is_some()
        );
    }

    #[test]
    fn original_function_point_metadata_declines_missing_foreign_stale_and_changed_geometry() {
        // naming.compiler.original-analysis-metadata-context

        // Even a same-store availability change cannot replace the point's
        // original full input. Missing supplied input is never Standalone.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = unit("missing_original_command VALUE", &context);
        let function = &unit.top_level;
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let input = function.source_metadata_input().unwrap();
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(context.commands())),
        );
        assert!(Arc::ptr_eq(older.commands(), context.commands()));
        for change in 0..4 {
            let mut changed = function.clone();
            match change {
                0 => changed.source_metadata_input = None,
                1 => {
                    changed.source_metadata_input = Some(ResolvedAnalysisInput::new(
                        input.analyser_profile(),
                        input.unit_profile(),
                        Arc::clone(&older),
                        input.lexer_config(),
                    ))
                }
                2 => {
                    changed.source_metadata_input = Some(ResolvedAnalysisInput::new(
                        input.analyser_profile(),
                        input.unit_profile(),
                        tcl_registry::model::ingress::resolve_environment("tcl9.1")
                            .default_context_registry(),
                        input.lexer_config(),
                    ))
                }
                3 => changed.source_config.strict_quoting = !changed.source_config.strict_quoting,
                _ => unreachable!(),
            }
            assert!(binding.original_invocation_metadata_for_function(
                tokens, &changed, context.commands(),
            ).is_none(), "changed facet {change}");
        }
        let mut changed = tokens.clone();
        changed.word_exprs.pop();
        assert!(
            binding
                .original_invocation_metadata_for_function(&changed, function, context.commands(),)
                .is_none()
        );
        let registry = tcl_registry::CommandRegistry::build_default();
        let standalone =
            CompilationUnit::build_for("missing_original_command VALUE", &registry, false);
        let script = &standalone.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        assert!(standalone.top_level.source_metadata_input().is_none());
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_invocation_metadata_for_function(tokens, &standalone.top_level, &registry)
                .is_some()
        );
    }

    #[test]
    fn original_materialized_footprint_keeps_alias_targets_and_literal_roots() {
        // naming.diagnostic.original-materialized-write-footprint

        // Source/API possible names only; no process result, child argv or store.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        for (source, text, expected) in [
            ("eval set {} VALUE", "set {} VALUE", Some("")),
            ("eval set {(key)} VALUE", "set {(key)} VALUE", Some("")),
            (
                "interp alias {} put {} set {}; eval put VALUE",
                "put VALUE",
                Some(""),
            ),
            ("proc set args {}; eval set {} VALUE", "set {} VALUE", None),
            (
                "eval set {café(open} VALUE",
                "set {café(open} VALUE",
                Some("café(open"),
            ),
            (
                "interp alias {} put {} set {$literal}; eval put VALUE",
                "put VALUE",
                Some("$literal"),
            ),
            (
                "rename set moved; interp alias {} put {} moved café; eval put VALUE",
                "put VALUE",
                Some("café"),
            ),
            (
                "proc set args {}; eval set hidden VALUE",
                "set hidden VALUE",
                None,
            ),
            (
                "interp alias {} put {} set fixed; rename set moved; eval put VALUE",
                "put VALUE",
                None,
            ),
        ] {
            let unit = unit(source, &context);
            let script = &unit.ir_module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            let metadata =
                InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
            let footprint = binding
                .original_materialized_footprint(tokens, source, context.commands(), metadata)
                .expect("original closed installer horizon");
            let writes = footprint.script_writes(text);
            if let Some(expected) = expected {
                assert!(
                    writes.names.iter().any(|name| name == expected),
                    "{source}: {:?}",
                    writes.names
                );
            } else {
                assert!(
                    writes.names.is_empty(),
                    "known replacement: {:?}",
                    writes.names
                );
            }
        }
    }

    #[test]
    fn original_materialized_footprint_keeps_empty_reads_and_unset_targets() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        for text in [
            "set {} VALUE; set {}; unset {}",
            "set {(key)} VALUE; set {(key)}; unset {(key)}",
        ] {
            let source = format!("eval {{{text}}}");
            let unit = unit(&source, &context);
            let script = &unit.ir_module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            let metadata =
                InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
            let footprint = binding
                .original_materialized_footprint(tokens, &source, context.commands(), metadata)
                .expect("original materialized source owner");
            let effects = footprint.script_writes(text);
            assert!(
                effects.names.iter().any(String::is_empty),
                "{text}: {effects:?}"
            );
            assert!(
                effects.read_names.iter().any(String::is_empty),
                "{text}: {effects:?}"
            );
        }
    }

    #[test]
    fn original_materialized_footprint_uses_actual_availability_on_the_same_store() {
        // naming.diagnostic.original-materialized-write-footprint

        // The command generation is identical; only actual availability changes.
        let mut registry = tcl_registry::CommandRegistry::build_default();
        let setter = registry.get("set").unwrap().clone();
        registry.insert(tcl_registry::CommandSpec {
            name: "metadata_write",
            surface: registry.get("dict").unwrap().surface,
            ..setter
        });
        let current = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl9.0")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let source = "eval metadata_write output VALUE";
        for (context, expected) in [(&current, true), (&older, false)] {
            let unit = unit(source, context);
            let script = &unit.ir_module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            let metadata = InvocationMetadataContext::for_module(&registry, &unit.ir_module);
            let footprint = binding
                .original_materialized_footprint(tokens, source, &registry, metadata)
                .unwrap();
            let writes = footprint.script_writes("metadata_write output VALUE");
            assert_eq!(
                writes.names.iter().any(|name| name == "output"),
                expected,
                "{expected}: {writes:?}"
            );
            if !expected {
                assert!(
                    writes.opaque,
                    "unavailable source target keeps its residual"
                );
            }
        }
    }

    #[test]
    fn materialized_bodies_keep_the_original_entry_availability_phase() {
        // naming.diagnostic.original-materialized-write-footprint

        // Exact source/API phase selection only. The explicit entry has no
        // native interpreter, body-entry, worker or successful store receipt.
        use tcl_dialect::model::InvocationRealm;
        let context = tcl_registry::model::ingress::resolve_environment("f5-irules")
            .default_context_registry();
        let registry = context.commands();
        let profile = tcl_dialect::DialectProfile::find("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
        assert!(context.context().resolve_spec(registry, "time").is_none());
        assert!(
            context
                .context()
                .resolve_spec_in_realm(registry, "time", InvocationRealm::InterpreterRuntime,)
                .is_some()
        );
        let source = "eval time {set output VALUE}";
        for (realm, expected) in [
            (InvocationRealm::RuleLoader, false),
            (InvocationRealm::InterpreterRuntime, true),
        ] {
            let entry = super::super::SourceAnalysisEntry {
                invocation_realm: realm,
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                ..Default::default()
            };
            let unit = CompilationUnit::build_with_analysis_input(
                source,
                UnitBuildOptions {
                    registry,
                    defer_top_level: false,
                    config,
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                Some(&entry),
                &input,
            );
            assert!(entry.native_entry.is_none());
            let script = &unit.ir_module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            assert_eq!(binding.invocation_realm(), Some(realm));
            let footprint = binding
                .original_materialized_footprint(
                    tokens,
                    source,
                    registry,
                    InvocationMetadataContext::for_module(registry, &unit.ir_module),
                )
                .unwrap();
            let writes = footprint.script_writes("time {set output VALUE}");
            assert_eq!(
                writes.names.iter().any(|name| name == "output"),
                expected,
                "{realm:?}: {writes:?}"
            );
            if !expected {
                assert!(writes.opaque, "loader refusal keeps the residual");
            }
        }
    }

    #[test]
    fn original_materialized_footprint_refuses_missing_foreign_and_stale_owners() {
        // naming.diagnostic.original-materialized-write-footprint

        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let source = "eval set x VALUE";
        let unit = unit(source, &context);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let metadata = InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
        assert!(
            binding
                .original_materialized_footprint(tokens, source, context.commands(), metadata)
                .is_some()
        );
        assert!(
            binding
                .original_materialized_footprint(tokens, source, context.commands(), None)
                .is_none()
        );
        assert!(
            binding
                .original_materialized_footprint(
                    tokens,
                    "eval set x OTHER",
                    context.commands(),
                    metadata
                )
                .is_none()
        );
        let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
        let foreign = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            input.lexer_config(),
        );
        assert!(
            binding
                .original_materialized_footprint(
                    tokens,
                    source,
                    context.commands(),
                    InvocationMetadataContext::for_analysis_input(context.commands(), &foreign)
                )
                .is_none()
        );
        let stale = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::clone(&context),
            tcl_lexer::LexerConfig {
                expand_syntax: tcl_lexer::LexerConfig::for_dialect("tcl8.4").expand_syntax,
                ..input.lexer_config()
            },
        );
        assert!(
            binding
                .original_materialized_footprint(
                    tokens,
                    source,
                    context.commands(),
                    InvocationMetadataContext::for_analysis_input(context.commands(), &stale)
                )
                .is_none()
        );
    }
    #[test]
    fn original_literal_body_ownership_keeps_named_purposes_and_literal_roots() {
        // naming.compiler.original-analysis-metadata-context

        // Conditional lexical names only, never installed aliases or current reads.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let source = "catch {global ::N::x; set {$literal} VALUE; set {café(open} VALUE; puts $missing; set byname}";
        let unit = unit(source, &context);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let metadata = InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
        for (purpose, expected_names, expected_reads) in [
            (
                crate::script_binds::Ownership::Bindings,
                vec!["x", "$literal", "café(open"],
                Vec::<&str>::new(),
            ),
            (
                crate::script_binds::Ownership::BindingsOrNameReads,
                vec!["x", "$literal", "café(open"],
                vec!["byname"],
            ),
            (
                crate::script_binds::Ownership::ScopeAliases,
                vec!["x"],
                Vec::<&str>::new(),
            ),
        ] {
            let names = crate::script_binds::original_literal_body_ownership(
                context.commands(),
                metadata,
                tokens,
                0,
                purpose,
            )
            .unwrap();
            assert!(!names.opaque, "{names:?}");
            assert_eq!(
                names
                    .names
                    .iter()
                    .map(String::as_str)
                    .collect::<std::collections::BTreeSet<_>>(),
                expected_names.into_iter().collect()
            );
            assert_eq!(
                names
                    .read_names
                    .iter()
                    .map(String::as_str)
                    .collect::<std::collections::BTreeSet<_>>(),
                expected_reads.into_iter().collect()
            );
            assert!(
                !names
                    .names
                    .iter()
                    .chain(&names.read_names)
                    .any(|name| name == "missing")
            );
        }
        assert!(
            crate::script_binds::original_literal_body_ownership(
                context.commands(),
                None,
                tokens,
                0,
                crate::script_binds::Ownership::Bindings,
            )
            .is_none()
        );
        assert!(
            crate::script_binds::original_literal_body_ownership(
                context.commands(),
                metadata,
                tokens,
                1,
                crate::script_binds::Ownership::Bindings,
            )
            .is_none()
        );
    }

    #[test]
    fn original_literal_body_aliases_keep_local_names_and_opaque_residuals() {
        // naming.compiler.original-analysis-metadata-context

        // Held alias names/captures and a conditional residual, no frame or execution.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        for (source, expected, opaque) in [
            (
                "interp alias {} expose {} global ::N::x; catch {expose}",
                Some("x"),
                false,
            ),
            (
                "rename global retired; interp alias {} expose {} retired ::N::x; catch {expose}",
                Some("x"),
                false,
            ),
            (
                "interp alias {} expose {} global ::N::x; rename global {}; catch {expose}",
                None,
                true,
            ),
            ("catch {set known VALUE; eval $script}", None, true),
        ] {
            let unit = unit(source, &context);
            let script = &unit.ir_module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let names = crate::script_binds::original_literal_body_ownership(
                context.commands(),
                InvocationMetadataContext::for_module(context.commands(), &unit.ir_module),
                tokens,
                0,
                crate::script_binds::Ownership::ScopeAliases,
            )
            .unwrap();
            assert_eq!(
                names.names.first().map(String::as_str),
                expected,
                "{source}: {names:?}"
            );
            assert_eq!(names.opaque, opaque, "{source}: {names:?}");
        }
    }

    #[test]
    fn original_literal_body_ownership_keeps_availability_and_source_currency() {
        // naming.compiler.original-analysis-metadata-context

        // Conditional source names only; availability is not actual execution.
        let current =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(current.commands())),
        );
        for (context, expected_opaque) in [(&current, false), (&older, true)] {
            let unit = unit("catch {throw {ERR} message}", context);
            let script = &unit.ir_module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let metadata =
                InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
            let ownership = crate::script_binds::original_literal_body_ownership(
                context.commands(),
                metadata,
                tokens,
                0,
                crate::script_binds::Ownership::Bindings,
            )
            .unwrap();
            assert_eq!(ownership.opaque, expected_opaque, "{ownership:?}");
            let foreign = tcl_registry::model::ingress::resolve_environment("tcl9.1")
                .default_context_registry();
            assert!(
                crate::script_binds::original_literal_body_ownership(
                    foreign.commands(),
                    metadata,
                    tokens,
                    0,
                    crate::script_binds::Ownership::Bindings,
                )
                .is_none()
            );
            let mut changed = tokens.clone();
            let crate::ir::WordExpr::BracedLiteral { text, .. } = &mut changed.word_exprs[1] else {
                panic!("original body word");
            };
            *text = "set fabricated VALUE".to_owned();
            assert!(
                crate::script_binds::original_literal_body_ownership(
                    context.commands(),
                    metadata,
                    &changed,
                    0,
                    crate::script_binds::Ownership::Bindings,
                )
                .is_none()
            );
        }
    }

    fn logical_script_context() -> Arc<tcl_registry::model::ContextRegistry> {
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "run-script",
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, tcl_registry::ArgRole::Body)],
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let environment = tcl_registry::model::ingress::resolve_environment("tcl");
        Arc::new(
            tcl_registry::model::context_for_profile(environment.analyser_profile())
                .with_command_store(Arc::new(registry)),
        )
    }

    fn logical_original_tokens(
        source: &str,
        context: &Arc<tcl_registry::model::ContextRegistry>,
    ) -> (crate::analyser::AnalysisResult, crate::ir::CommandTokens) {
        let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
        assert!(input.has_logical_source_name_context());
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "tcl");
        let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            segments.last().expect("actual final original command"),
        );
        analysis
            .retained_command_realm()
            .expect("actual analysis retains its original source world")
            .stamp_original_tokens(&mut tokens);
        (analysis, tokens)
    }

    #[test]
    fn original_source_realm_keeps_sealed_declaration_and_reached_owners_independent() {
        // naming.compiler.original-analysis-metadata-context

        // Readonly source realm; no reached lookup or body/frame permission.
        let context = logical_script_context();
        let source = "interp alias {} shared {} run-script; shared {set local VALUE}";
        let (analysis, tokens) = logical_original_tokens(source, &context);
        let input = analysis.resolved_input.as_ref().unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.lookup_state.is_none());
        let advice = binding.declaration_operand_layout_advice(&tokens).unwrap();
        assert!(advice.closed_logical_source_lookup());
        assert_eq!(
            binding.original_source_realm_for_input(&tokens, input, context.commands()),
            Some(advice.realm())
        );
        assert!(binding.lookup_state.is_none());
        assert!(advice.dialect().native_name_protocol().is_none());

        let mut missing = tokens.clone();
        missing
            .source_binding
            .as_mut()
            .unwrap()
            .declaration_layout_observations = None;
        assert!(
            missing
                .source_binding
                .as_ref()
                .unwrap()
                .original_source_realm_for_input(&missing, input, context.commands())
                .is_none()
        );
        let mut changed = tokens.clone();
        changed.word_exprs.pop();
        assert!(
            binding
                .original_source_realm_for_input(&changed, input, context.commands())
                .is_none()
        );
        let older_context = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(context.commands())),
        );
        assert!(Arc::ptr_eq(context.commands(), older_context.commands()));
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        let mut foreign_store = tcl_registry::CommandRegistry::build_default();
        foreign_store.insert(tcl_registry::CommandSpec {
            name: "foreign_source_realm_axis",
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let foreign_context = Arc::new(context.with_command_store(Arc::new(foreign_store)));
        let native = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        for changed_input in [
            ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                older_context,
                input.lexer_config(),
            ),
            ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                Arc::clone(&context),
                config,
            ),
            ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                foreign_context,
                input.lexer_config(),
            ),
            ResolvedAnalysisInput::new(native, native, Arc::clone(&context), input.lexer_config()),
        ] {
            assert!(
                binding
                    .original_source_realm_for_input(&tokens, &changed_input, context.commands())
                    .is_none()
            );
        }
        // Present reached ownership is terminal even with valid declaration rows.
        let mut invalid = tokens.clone();
        let mut point = advice.snapshot.as_ref().clone();
        Arc::make_mut(&mut point.state.baseline).metadata_context =
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
        invalid.source_binding.as_mut().unwrap().lookup_state = Some(Arc::new(point));
        assert!(
            invalid
                .source_binding
                .as_ref()
                .unwrap()
                .original_source_realm_for_input(&invalid, input, context.commands())
                .is_none()
        );
    }

    #[test]
    fn original_logical_materialized_names_keep_static_and_captured_source_horizons() {
        // naming.diagnostic.original-materialized-write-footprint

        // Conditional MAY names; no reached handler, child entry or physical store.
        // An actual abrupt prefix retains lexical declarations without a source dispatch point.
        let context = logical_script_context();
        for source in [
            "return DONE; run-script {set {$literal} VALUE; set {café(open} VALUE; puts $read; set byname}",
            "interp alias {} shared {} run-script {set {$literal} VALUE; set {café(open} VALUE; puts $read; set byname}; return DONE; shared",
        ] {
            let (analysis, tokens) = logical_original_tokens(source, &context);
            let binding = tokens
                .source_binding
                .as_ref()
                .expect("original source binding");
            assert!(
                binding.lookup_state.is_none(),
                "declaration-only Logical purpose"
            );
            let input = analysis.resolved_input.as_ref().unwrap();
            let advice = binding.declaration_operand_layout_advice(&tokens).unwrap();
            assert!(advice.closed_logical_source_lookup());
            assert!(advice.dialect().native_string_protocol().is_none());
            let words =
                crate::registry_invocation::source_structure::original_registry_words_for_tokens(
                    source, &analysis, &tokens,
                )
                .expect("selected original source schema");
            let bodies = words.source_script_bodies_for(
                &context,
                crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
            );
            assert_eq!(
                bodies.len(),
                1,
                "original selected immediate body: {source}"
            );
            let footprint = binding
                .original_materialized_footprint(
                    &tokens,
                    source,
                    context.commands(),
                    InvocationMetadataContext::for_analysis_input(context.commands(), input),
                )
                .expect("sealed Logical installer horizon");
            let names = footprint.body_name_ownership(&bodies[0]);
            assert!(!names.opaque, "{source}: {names:?}");
            assert!(names.names.iter().any(|name| name == "$literal"));
            assert!(names.names.iter().any(|name| name == "café(open"));
            assert!(names.read_names.iter().any(|name| name == "byname"));
            assert!(
                !names
                    .names
                    .iter()
                    .chain(&names.read_names)
                    .any(|name| name == "read")
            );
            assert!(
                binding.lookup_state.is_none(),
                "query issues no reached point"
            );
        }
    }

    #[test]
    fn original_logical_materialized_names_refuse_changed_and_missing_source_owners() {
        // naming.compiler.original-analysis-metadata-context

        // An actual abrupt prefix retains lexical declarations without a source dispatch point.
        let context = logical_script_context();
        let source = "return DONE; run-script {set x VALUE}";
        let (analysis, tokens) = logical_original_tokens(source, &context);
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.lookup_state.is_none());
        let input = analysis.resolved_input.as_ref().unwrap();
        let metadata = InvocationMetadataContext::for_analysis_input(context.commands(), input);
        assert!(
            binding
                .original_materialized_footprint(&tokens, source, context.commands(), metadata)
                .is_some()
        );
        assert!(
            binding
                .original_materialized_footprint(&tokens, source, context.commands(), None)
                .is_none()
        );
        assert!(
            binding
                .original_materialized_footprint(
                    &tokens,
                    "run-script {set x OTHER}",
                    context.commands(),
                    metadata
                )
                .is_none()
        );

        let older_context = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(context.commands())),
        );
        assert!(Arc::ptr_eq(context.commands(), older_context.commands()));
        let older = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older_context,
            input.lexer_config(),
        );
        assert!(
            binding
                .original_materialized_footprint(
                    &tokens,
                    source,
                    context.commands(),
                    InvocationMetadataContext::for_analysis_input(context.commands(), &older),
                )
                .is_none()
        );
        let changed = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::clone(&context),
            tcl_lexer::LexerConfig {
                expand_syntax: !input.lexer_config().expand_syntax,
                ..input.lexer_config()
            },
        );
        assert!(
            binding
                .original_materialized_footprint(
                    &tokens,
                    source,
                    context.commands(),
                    InvocationMetadataContext::for_analysis_input(context.commands(), &changed),
                )
                .is_none()
        );
        let mut foreign_registry = tcl_registry::CommandRegistry::build_default();
        foreign_registry.insert(tcl_registry::CommandSpec {
            name: "foreign_materialized_source_axis",
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let foreign_context = Arc::new(
            tcl_registry::model::context_for_profile(input.analyser_profile())
                .with_command_store(Arc::new(foreign_registry)),
        );
        let foreign = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::clone(&foreign_context),
            input.lexer_config(),
        );
        assert!(
            binding
                .original_materialized_footprint(
                    &tokens,
                    source,
                    foreign_context.commands(),
                    InvocationMetadataContext::for_analysis_input(
                        foreign_context.commands(),
                        &foreign
                    ),
                )
                .is_none()
        );

        let native_profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let native_input = ResolvedAnalysisInput::new(
            native_profile,
            native_profile,
            Arc::clone(&context),
            input.lexer_config(),
        );
        assert!(!native_input.has_logical_source_name_context());
        assert!(
            binding
                .original_materialized_footprint(
                    &tokens,
                    source,
                    context.commands(),
                    InvocationMetadataContext::for_analysis_input(
                        context.commands(),
                        &native_input
                    ),
                )
                .is_none()
        );

        let mut missing = binding.clone();
        missing.declaration_layout_observations = None;
        assert!(
            missing
                .original_materialized_footprint(&tokens, source, context.commands(), metadata)
                .is_none()
        );
        let mut unavailable = binding.clone();
        let mut rows = unavailable
            .declaration_layout_observations
            .as_deref()
            .unwrap()
            .to_vec();
        for row in &mut rows {
            let mut state = row.snapshot.state.clone();
            Arc::make_mut(&mut state.baseline).metadata_context =
                crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
            row.snapshot = Arc::new(super::super::SourceLookupSnapshot::in_realm(
                state,
                row.snapshot.realm,
            ));
        }
        unavailable.declaration_layout_observations = Some(Arc::from(rows));
        let mut unavailable_tokens = tokens.clone();
        unavailable_tokens.source_binding = Some(unavailable.clone());
        assert!(
            unavailable
                .original_materialized_footprint(
                    &unavailable_tokens,
                    source,
                    context.commands(),
                    metadata
                )
                .is_none()
        );

        // Matching head targets do not identify a whole installer world.
        // A second original row with a different nested setter table cannot
        // donate the first row's materialized name facts.
        let mut alternatives = binding.clone();
        let original_row = super::super::declaration_layout::original_declaration_layouts(
            binding.declaration_layout_observations.as_deref().unwrap(),
        )
        .unwrap()
        .next()
        .unwrap()
        .clone();
        let mut different = original_row.clone();
        let mut state = different.snapshot.state.clone();
        state.remove("::set");
        different.snapshot = Arc::new(super::super::SourceLookupSnapshot::in_realm(
            state,
            different.snapshot.realm,
        ));
        alternatives.declaration_layout_observations =
            Some(Arc::from(vec![original_row, different]));
        let mut alternative_tokens = tokens.clone();
        alternative_tokens.source_binding = Some(alternatives.clone());
        assert!(
            alternatives
                .declaration_operand_layout_advice(&alternative_tokens)
                .unwrap()
                .closed_logical_source_lookup()
        );
        assert!(
            alternatives
                .original_materialized_footprint(
                    &alternative_tokens,
                    source,
                    context.commands(),
                    metadata
                )
                .is_none()
        );

        // A retained reached owner is authoritative even when stale. The
        // earlier valid declaration cannot repair this explicit invalid point.
        let mut invalid_point = binding.clone();
        let point = advice_snapshot(binding, &tokens);
        let mut state = point.state.clone();
        Arc::make_mut(&mut state.baseline).metadata_context =
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
        invalid_point.lookup_state = Some(Arc::new(super::super::SourceLookupSnapshot::in_realm(
            state,
            point.realm,
        )));
        let mut invalid_tokens = tokens.clone();
        invalid_tokens.source_binding = Some(invalid_point.clone());
        assert!(
            invalid_point
                .original_materialized_footprint(
                    &invalid_tokens,
                    source,
                    context.commands(),
                    metadata
                )
                .is_none()
        );
    }

    fn advice_snapshot(
        binding: &super::SourceInvocationBinding,
        tokens: &crate::ir::CommandTokens,
    ) -> Arc<super::super::SourceLookupSnapshot> {
        binding
            .declaration_operand_layout_advice(tokens)
            .unwrap()
            .snapshot
    }

    #[test]
    fn original_logical_materialized_horizon_refuses_dynamic_expanded_and_invalid_vectors() {
        // naming.diagnostic.original-materialized-write-footprint

        // An actual abrupt prefix retains lexical declarations without a source dispatch point.
        let context = logical_script_context();
        for (source, expected) in [
            ("return DONE; run-script {set x VALUE}", true),
            ("return DONE; run-script $script", false),
            ("return DONE; run-script [unknown]", false),
            ("return DONE; run-script {*}{set x VALUE}", false),
        ] {
            let (analysis, tokens) = logical_original_tokens(source, &context);
            let binding = tokens.source_binding.as_ref().unwrap();
            let rows = super::super::declaration_layout::original_declaration_layouts(
                binding.declaration_layout_observations.as_deref().unwrap(),
            )
            .expect("actual original declaration row");
            assert!(rows.clone().all(|row| {
                row.snapshot
                    .state
                    .source_variables
                    .invocation_dialect
                    .unwrap()
                    .native_string_protocol()
                    .is_none()
            }));
            assert_eq!(
                rows.clone()
                    .all(|row| binding.original_operands_preserve_lookup(
                        &tokens,
                        context.commands(),
                        row,
                        0
                    )),
                expected,
                "{source}"
            );
            let input = analysis.resolved_input.as_ref().unwrap();
            assert_eq!(
                binding
                    .original_materialized_footprint(
                        &tokens,
                        source,
                        context.commands(),
                        InvocationMetadataContext::for_analysis_input(context.commands(), input),
                    )
                    .is_some(),
                expected,
                "{source}"
            );
            if expected {
                let mut altered = tokens.clone();
                let crate::ir::WordExpr::BracedLiteral { text, .. } = &mut altered.word_exprs[1]
                else {
                    panic!("original whole braced body");
                };
                *text = "set fabricated VALUE".to_owned();
                assert!(
                    rows.clone()
                        .all(|row| !binding.original_operands_preserve_lookup(
                            &altered,
                            context.commands(),
                            row,
                            0
                        ))
                );
                assert!(
                    binding
                        .original_materialized_footprint(
                            &altered,
                            source,
                            context.commands(),
                            InvocationMetadataContext::for_analysis_input(
                                context.commands(),
                                input
                            ),
                        )
                        .is_none()
                );
            }
        }
    }

    fn original_native_procedure_argument_tokens(
        source: &str,
    ) -> (
        tcl_vm::Vm,
        Arc<tcl_registry::model::ContextRegistry>,
        ResolvedAnalysisInput,
        crate::ir::CommandTokens,
    ) {
        let environment =
            tcl_registry::model::ingress::resolve_known_environment("tcl9.0").unwrap();
        let context = environment.default_context_registry();
        let profile = environment.unit_profile();
        let (owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        let config = super::super::SourceAnalysisOptions {
            native_entry: Some(&entry),
            ..Default::default()
        }
        .native_lexer_config(tcl_lexer::LexerConfig::for_file_grammar(profile.grammar));
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
        let options = super::super::SourceAnalysisOptions {
            metadata_context: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
                Some(&input),
            ),
            native_entry: Some(&entry),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let image = tcl_lexer::SourceImage::document(source);
        let bindings = super::super::SourceCommandBindings::analyse_image_in_frame_with_options(
            &image,
            &crate::var_resolve::VariableExecutionFrame::Global,
            config,
            context.commands(),
            options,
        )
        .unwrap();
        let segments =
            crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
                .unwrap();
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &image.source_map(),
            config,
            segments.last().unwrap(),
        );
        bindings.stamp_original_tokens(&mut tokens);
        (owner, context, input, tokens)
    }

    #[test]
    fn original_procedure_argument_roles_keep_native_entry_and_logical_source_owners_separate() {
        // naming.compiler.original-analysis-metadata-context

        // Software-owned source argv roles, independently of entered calls and Normal.
        for source in [
            "proc selected {value} {}; selected {[inert CHILD]}",
            "proc selected {value} {}; rename selected relocated; relocated VALUE",
            "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded VALUE",
        ] {
            let (_owner, context, input, tokens) =
                original_native_procedure_argument_tokens(source);
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(
                binding
                    .original_source_procedure_argument_roles(&tokens, &input, context.commands())
                    .is_some(),
                "{source}"
            );
            assert!(!input.has_logical_source_name_context());
            assert!(
                binding
                    .lookup_state
                    .as_ref()
                    .unwrap()
                    .state
                    .baseline
                    .native_entry
                    .is_some()
            );
            let advice = binding.declaration_call_layout_advice(&tokens).unwrap();
            assert!(advice.source_targets_are_closed());
            assert!(
                advice
                    .targets()
                    .iter()
                    .all(|target| target.implementation_allocation.is_some())
            );
        }
        let context = logical_script_context();
        // The original declaration suffix remains lexical advice after the actual return.
        let source = "proc selected {value} {}; return DONE; selected {[inert CHILD]}";
        let (analysis, tokens) = logical_original_tokens(source, &context);
        let input = analysis.resolved_input.as_ref().unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(
            binding
                .original_source_procedure_argument_roles(&tokens, input, context.commands())
                .is_some()
        );
        assert!(input.has_logical_source_name_context());
        assert!(binding.lookup_state.is_none());
    }

    #[test]
    fn original_procedure_argument_roles_refuse_changed_missing_and_open_owners() {
        // naming.compiler.original-analysis-metadata-context

        let source = "proc selected {value} {}; selected VALUE";
        let (_owner, context, input, tokens) = original_native_procedure_argument_tokens(source);
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(
            binding
                .original_source_procedure_argument_roles(&tokens, &input, context.commands())
                .is_some()
        );
        let changed_input = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            tcl_lexer::LexerConfig {
                strict_quoting: !input.lexer_config().strict_quoting,
                ..input.lexer_config()
            },
        );
        assert!(
            binding
                .original_source_procedure_argument_roles(
                    &tokens,
                    &changed_input,
                    context.commands()
                )
                .is_none()
        );
        let foreign = tcl_registry::model::ingress::resolve_known_environment("tcl8.4")
            .unwrap()
            .default_context_registry();
        assert!(
            binding
                .original_source_procedure_argument_roles(&tokens, &input, foreign.commands())
                .is_none()
        );
        for axis in 0..7 {
            let mut changed = tokens.clone();
            match axis {
                0 => changed.argv_texts[1] = "DIFFERENT".into(),
                1 => changed.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedArguments),
                2 => changed.source_binding.as_mut().unwrap().unknown = true,
                3 => changed.source_binding.as_mut().unwrap().may_be_absent = true,
                4 => {
                    changed.source_binding.as_mut().unwrap().targets[0].implementation_allocation =
                        None
                }
                5 => {
                    let point = Arc::make_mut(
                        changed
                            .source_binding
                            .as_mut()
                            .unwrap()
                            .lookup_state
                            .as_mut()
                            .unwrap(),
                    );
                    Arc::make_mut(&mut point.state.baseline).native_entry = None;
                }
                6 => {
                    let point = Arc::make_mut(
                        changed
                            .source_binding
                            .as_mut()
                            .unwrap()
                            .lookup_state
                            .as_mut()
                            .unwrap(),
                    );
                    Arc::make_mut(&mut point.state.baseline).metadata_context =
                        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
                }
                _ => unreachable!(),
            }
            assert!(
                changed
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .original_source_procedure_argument_roles(&changed, &input, context.commands())
                    .is_none(),
                "axis {axis}"
            );
        }
        for source in [
            "list VALUE",
            "proc selected {value} {}; unknown; selected VALUE",
        ] {
            let (_owner, context, input, tokens) =
                original_native_procedure_argument_tokens(source);
            assert!(
                tokens
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .original_source_procedure_argument_roles(&tokens, &input, context.commands())
                    .is_none()
            );
        }
    }
    #[test]
    fn original_logical_procedure_roles_keep_declaration_layout_separate_from_allocation_advice() {
        // naming.compiler.original-positioned-callee-body-availability

        // naming.source.original-point-operand-projection

        let context = logical_script_context();
        for source in [
            "proc selected {value} {}; selected VALUE",
            "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded VALUE",
            "proc selected {value} {}; return DONE; selected {[inert CHILD]}",
        ] {
            let (analysis, tokens) = logical_original_tokens(source, &context);
            let input = analysis.resolved_input.as_ref().unwrap();
            assert!(input.has_logical_source_name_context());
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(binding.declaration_call_layout_advice(&tokens).is_none());
            let declared = binding
                .original_logical_declaration_call_targets(&tokens, input, context.commands())
                .expect("complete Logical source declaration");
            assert!(!declared.is_empty());
            assert!(declared.iter().all(|target| !target.registry_backed
                && target.kind == super::super::BindingKind::Proc
                && target.implementation_allocation.is_none()));
            assert!(
                binding
                    .original_source_procedure_argument_roles(&tokens, input, context.commands(),)
                    .is_some()
            );
            assert!(binding.proved_execution_target().is_none());
            assert!(binding.proved_handler_target().is_none());
            assert!(binding.native_compilation_admission.is_none());
            let mut changed = tokens.clone();
            changed.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedArguments);
            assert!(
                binding
                    .original_source_procedure_argument_roles(&changed, input, context.commands(),)
                    .is_none()
            );
            let mut altered_config = input.lexer_config();
            altered_config.strict_quoting = !altered_config.strict_quoting;
            let altered = ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                input.context_registry(),
                altered_config,
            );
            assert!(binding.original_source_procedure_argument_roles(
                &tokens, &altered, context.commands(),
            ).is_none());
        }
    }

    #[test]
    fn original_native_source_procedure_roles_keep_the_genuine_allocation_purpose() {
        // naming.compiler.original-positioned-callee-body-availability

        // The actual held software Native entry remains distinct from the
        // positive Logical source model. No external Native execution is observed.
        let (_owner, context, input, tokens) =
            original_native_procedure_argument_tokens("proc selected {value} {}; selected VALUE");
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(!input.has_logical_source_name_context());
        let advice = binding.declaration_call_layout_advice(&tokens).unwrap();
        assert!(
            advice
                .targets()
                .iter()
                .all(|target| target.implementation_allocation.is_some())
        );
        assert!(
            binding
                .original_logical_declaration_call_targets(&tokens, &input, context.commands(),)
                .is_none()
        );
        assert!(
            binding
                .original_source_procedure_argument_roles(&tokens, &input, context.commands(),)
                .is_some()
        );
    }

    #[test]
    fn original_logical_procedure_argv_keeps_unentered_alias_prefix_and_literal_child() {
        // naming.compiler.original-positioned-callee-body-availability

        // Lexical argv roles retain the real declaration prefix, not a call frame.
        let context = logical_script_context();
        let source = "proc café {prefix value} {}; interp alias {} forwarded {} café PREFIX; return DONE; forwarded {[inert CHILD]}";
        let (analysis, tokens) = logical_original_tokens(source, &context);
        let input = analysis.resolved_input.as_ref().unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.lookup_state.is_none());
        assert!(binding.unknown);
        assert!(binding.targets.is_empty());
        let declared = binding
            .original_logical_declaration_call_targets(&tokens, input, context.commands())
            .unwrap();
        assert_eq!(declared.len(), 1);
        assert_eq!(declared[0].prepended.len(), 1);
        assert_eq!(
            declared[0].prepended[0].literal_bytes(),
            Some(b"PREFIX".as_slice())
        );
        assert!(declared[0].implementation_allocation.is_none());
        assert!(
            binding
                .original_source_procedure_argument_roles(&tokens, input, context.commands())
                .is_some()
        );
        assert!(
            crate::word_subst::checked_argument_lifted_calls(&tokens, input.lexer_config(),)
                .unwrap()
                .is_empty()
        );
        assert!(binding.declaration_call_layout_advice(&tokens).is_none());
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.proved_handler_target().is_none());
        assert_ne!(
            binding.runtime_reachability(),
            super::super::SourceRuntimeReachability::Reached
        );
        assert_ne!(
            binding.runtime_reachability(),
            super::super::SourceRuntimeReachability::Conditional
        );
    }

    #[test]
    fn original_logical_procedure_argv_refuses_replaced_receipt_vector_and_source_world() {
        // naming.compiler.original-positioned-callee-body-availability

        let context = logical_script_context();
        let source = "proc selected {value} {}; return DONE; selected {LITERAL}";
        let (analysis, tokens) = logical_original_tokens(source, &context);
        let input = analysis.resolved_input.as_ref().unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(
            binding
                .original_source_procedure_argument_roles(&tokens, input, context.commands(),)
                .is_some()
        );
        let (_, foreign) = logical_original_tokens(
            "proc different {value} {}; return DONE; different {LITERAL}",
            &context,
        );
        for axis in 0..4 {
            let mut changed = tokens.clone();
            match axis {
                0 => changed.argv_texts[1] = "CHANGED".into(),
                1 => {
                    changed
                        .source_binding
                        .as_mut()
                        .unwrap()
                        .original_logical_declaration_targets = foreign
                        .source_binding
                        .as_ref()
                        .unwrap()
                        .original_logical_declaration_targets
                        .clone()
                }
                2 => {
                    changed
                        .source_binding
                        .as_mut()
                        .unwrap()
                        .declaration_layout_observations = foreign
                        .source_binding
                        .as_ref()
                        .unwrap()
                        .declaration_layout_observations
                        .clone()
                }
                3 => {
                    changed.source_binding.as_mut().unwrap().targets = binding
                        .original_logical_procedure_call_targets(&tokens, input)
                        .unwrap()
                }
                _ => unreachable!(),
            }
            assert!(
                changed
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .original_source_procedure_argument_roles(&changed, input, context.commands())
                    .is_none(),
                "axis {axis}"
            );
        }
        let (alias_analysis, mut alias_tokens) = logical_original_tokens(
            "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded VALUE",
            &context,
        );
        let alias_input = alias_analysis.resolved_input.as_ref().unwrap();
        let alias_binding = alias_tokens.source_binding.as_ref().unwrap();
        assert!(
            alias_binding
                .original_source_procedure_argument_roles(
                    &alias_tokens,
                    alias_input,
                    context.commands(),
                )
                .is_some()
        );
        alias_tokens.source_binding.as_mut().unwrap().targets[0].prepended[0] =
            crate::registry_invocation::EffectiveInvocationWord::Literal("CHANGED".into());
        assert!(
            alias_tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_source_procedure_argument_roles(
                    &alias_tokens,
                    alias_input,
                    context.commands()
                )
                .is_none()
        );
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        let changed_input = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        assert!(
            binding
                .original_source_procedure_argument_roles(
                    &tokens,
                    &changed_input,
                    context.commands(),
                )
                .is_none()
        );
        let foreign_registry = tcl_registry::model::ingress::resolve_known_environment("tcl8.4")
            .unwrap()
            .default_context_registry();
        assert!(
            binding
                .original_source_procedure_argument_roles(
                    &tokens,
                    input,
                    foreign_registry.commands(),
                )
                .is_none()
        );
    }

    #[test]
    fn original_logical_procedure_argv_cannot_repair_open_or_registry_targets() {
        // naming.compiler.original-positioned-callee-body-availability

        let context = logical_script_context();
        for source in [
            "proc selected {value} {}; unknown; selected VALUE",
            "proc selected {prefix value} {}; interp alias {} forwarded {} selected [unknown]; forwarded VALUE",
            "proc selected {value} {}; return DONE; list VALUE",
        ] {
            let (analysis, tokens) = logical_original_tokens(source, &context);
            let input = analysis.resolved_input.as_ref().unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(
                binding
                    .original_source_procedure_argument_roles(&tokens, input, context.commands(),)
                    .is_none(),
                "{source}"
            );
            assert!(binding.proved_execution_target().is_none());
            assert!(binding.proved_handler_target().is_none());
        }
    }

    #[test]
    fn original_logical_materialized_root_keeps_its_actual_source_lookup_point() {
        // naming.diagnostic.original-materialized-write-footprint

        // The root source point supplies conditional schema and MAY names;
        // it is distinct from an unentered suffix and grants no handler/store.
        let context = logical_script_context();
        let source = "run-script {set x VALUE}";
        let (analysis, tokens) = logical_original_tokens(source, &context);
        let input = analysis.resolved_input.as_ref().unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let point = binding.lookup_state.as_ref().unwrap();
        assert_eq!(point.state.logical_source_name_advice_input(), Some(input));
        assert!(point.state.baseline.native_entry.is_none());
        assert_eq!(
            binding.runtime_reachability(),
            super::super::SourceRuntimeReachability::Conditional
        );
        let words =
            crate::registry_invocation::source_structure::original_registry_words_for_tokens(
                source, &analysis, &tokens,
            )
            .expect("same original source point and Registry source role");
        let bodies = words.source_script_bodies_for(
            &context,
            crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
        );
        assert_eq!(bodies.len(), 1);
        let metadata = InvocationMetadataContext::for_analysis_input(context.commands(), input);
        let footprint = binding
            .original_materialized_footprint(&tokens, source, context.commands(), metadata)
            .unwrap();
        let names = footprint.body_name_ownership(&bodies[0]);
        assert!(!names.opaque);
        assert!(names.names.iter().any(|name| name == "x"));
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.proved_handler_target().is_none());
        assert!(
            binding
                .lookup_state
                .as_ref()
                .is_some_and(|retained| Arc::ptr_eq(retained, point))
        );
    }
}
