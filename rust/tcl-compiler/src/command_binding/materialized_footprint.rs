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

//! Conditional materialized text footprints anchored at an original source lookup.

use super::original_name_value::OriginalWordSourcePurpose;
use crate::command_binding::SourceOriginLexical;

use super::{ModuleCommandBindings, SourceInvocationBinding};
use crate::ir::CommandTokens;
use crate::registry_invocation::InvocationMetadataContext;
use tcl_lexer::LexerConfig;
use tcl_names::namespace::SourceNamespaceKey;
use tcl_registry::CommandRegistry;

#[cfg(test)]
fn trace_original_child_guard(label: &str, offset: Option<u32>, passed: bool) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
        eprintln!("ORIGINAL_CHILD_GUARD label={label} offset={offset:?} passed={passed}");
    }
}

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

thread_local! {
    static READONLY_METADATA_PROBE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[must_use = "retain the diagnostic scope through the synchronous query"]
pub(crate) struct OriginalReadonlyMetadataProbe {
    previous: bool,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}

impl Drop for OriginalReadonlyMetadataProbe {
    fn drop(&mut self) {
        READONLY_METADATA_PROBE.with(|active| active.set(self.previous));
    }
}

fn readonly_metadata_failure(stage: &'static str) {
    static LOGS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    if READONLY_METADATA_PROBE.with(std::cell::Cell::get)
        && LOGS
            .try_update(
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
                |count| count.checked_add(1).filter(|next| *next <= 128),
            )
            .is_ok()
    {
        eprintln!("NATIVE_READONLY_OWNER stage={stage} present=false");
    }
}

impl SourceInvocationBinding {
    pub(crate) fn begin_original_readonly_metadata_probe() -> OriginalReadonlyMetadataProbe {
        let previous = READONLY_METADATA_PROBE.with(|active| {
            let previous = active.get();
            active.set(
                tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_VAR_ESCAPE_BODY_OWNERS").is_some(),
            );
            previous
        });
        OriginalReadonlyMetadataProbe {
            previous,
            _thread: std::marker::PhantomData,
        }
    }

    pub(crate) fn probe_original_readonly_option<T>(
        stage: &'static str,
        value: Option<T>,
    ) -> Option<T> {
        if value.is_none() {
            readonly_metadata_failure(stage);
        }
        value
    }

    fn probe_original_readonly_result<T>(
        stage: &'static str,
        value: Result<T, crate::registry_invocation::RegistryInvocationDecline>,
    ) -> Result<T, crate::registry_invocation::RegistryInvocationDecline> {
        if value.is_err() {
            readonly_metadata_failure(stage);
        }
        value
    }

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
                target.kind != tcl_names::command::BindingKind::Proc
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
        use crate::command_binding::OriginalInvocationSourcePurpose;
        let metadata = self.original_invocation_metadata_for_input(tokens, input, registry)?;
        let purpose = self.original_invocation_source_purpose(tokens, metadata, registry)?;
        let logical = purpose == OriginalInvocationSourcePurpose::Logical;
        let targets = if purpose == OriginalInvocationSourcePurpose::AuthoredSimulation {
            // Authored source has no Native entry: its closed source binding is
            // the procedure correspondence. Inherited and unknown entries keep
            // their independent refusal.
            let baseline = &self.lookup_state.as_ref()?.state.baseline;
            if baseline.native_entry.is_some()
                || baseline.unknown_entry
                || baseline.source_point_entry.is_some()
            {
                return None;
            }
            self.targets.clone()
        } else if logical {
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
                target.kind != tcl_names::command::BindingKind::Proc
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
    #[doc(hidden)]
    pub fn original_argument_metadata_for_module<'a>(
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
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
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
    #[doc(hidden)]
    pub fn original_invocation_metadata_for_input<'a>(
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
            OwnedInvocationMetadataContext::SuppliedSource(input) => {
                Self::probe_original_readonly_result(
                    "owner.supplied_source",
                    self.original_word_metadata_for_input(tokens, input, registry, purpose)
                        .map(Some)
                        .ok_or(refusal),
                )
            }
            OwnedInvocationMetadataContext::Standalone => {
                Self::probe_original_readonly_result(
                    "owner.standalone_geometry",
                    self.original_word_config(tokens, registry, purpose)
                        .ok_or(refusal),
                )?;
                let point = &Self::probe_original_readonly_result(
                    "owner.standalone_point",
                    self.lookup_state.as_ref().ok_or(refusal),
                )?
                .state
                .baseline
                .metadata_context;
                if !point.is_standalone() {
                    return Self::probe_original_readonly_result(
                        "owner.standalone_kind",
                        Err(refusal),
                    );
                }
                Self::probe_original_readonly_result(
                    "owner.standalone_metadata",
                    point
                        .metadata_context(registry)
                        .map(crate::registry_invocation::InvocationMetadataSelection::context)
                        .ok_or(refusal),
                )
            }
            OwnedInvocationMetadataContext::Supplied(_) => {
                Self::probe_original_readonly_result("owner.supplied", Err(refusal))
            }
            OwnedInvocationMetadataContext::Unavailable => {
                Self::probe_original_readonly_result("owner.unavailable", Err(refusal))
            }
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
        Self::probe_original_readonly_result(
            "owner.declaration_absent",
            self.original_declaration_metadata_for_source(tokens, registry)
                .map(Some)
                .ok_or(crate::registry_invocation::RegistryInvocationDecline::IncompleteResolution),
        )
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
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
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
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
        #[cfg(test)]
        self.trace_materialized_footprint_request(tokens, registry, metadata);
        let metadata = metadata.filter(|metadata| metadata.matches_registry(registry))?;
        let input = metadata.source_analysis_input()?;
        if self.lookup_state.is_none() {
            let result = self.original_logical_declaration_materialized_footprint(
                tokens, source, registry, metadata,
            );
            #[cfg(test)]
            trace_original_child_guard(
                "footprint.declaration",
                tokens.argv.first().map(|span| span.start()),
                result.is_some(),
            );
            return result;
        }
        let metadata = self.original_invocation_metadata_for_input(tokens, input, registry);
        #[cfg(test)]
        trace_original_child_guard(
            "footprint.point_metadata",
            tokens.argv.first().map(|span| span.start()),
            metadata.is_some(),
        );
        let metadata = metadata?;
        if metadata.source_analysis_input() != Some(input) {
            return None;
        }
        let config = self.original_lexer_config_for_tokens(tokens)?;
        let site = self.invocation_site()?;
        let snapshot = self.lookup_state.as_ref()?;
        let state = &snapshot.state;
        #[cfg(test)]
        self.trace_materialized_footprint_state(state, site, source, registry, config, input);
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

    #[cfg(test)]
    fn trace_materialized_footprint_request(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
        metadata: Option<InvocationMetadataContext<'_>>,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            let offset = tokens.argv.first().map(|span| span.start());
            trace_original_child_guard(
                "footprint.request",
                offset,
                metadata.is_some_and(|metadata| metadata.matches_registry(registry)),
            );
            trace_original_child_guard(
                "footprint.request_input",
                offset,
                metadata.is_some_and(|metadata| metadata.source_analysis_input().is_some()),
            );
            trace_original_child_guard(
                "footprint.reached_lookup",
                offset,
                self.lookup_state.is_some(),
            );
        }
    }

    #[cfg(test)]
    fn trace_materialized_footprint_state(
        &self,
        state: &ModuleCommandBindings,
        site: &tcl_names::command::CommandAllocationSite,
        source: &str,
        registry: &CommandRegistry,
        config: LexerConfig,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            let offset = Some(site.offset);
            trace_original_child_guard("footprint.known", offset, !self.unknown);
            trace_original_child_guard("footprint.present", offset, !self.may_be_absent);
            trace_original_child_guard(
                "footprint.closed_domain",
                offset,
                !state.has_opaque_domain(),
            );
            trace_original_child_guard(
                "footprint.quiet",
                offset,
                state.command_observers.is_quiet(),
            );
            trace_original_child_guard("footprint.no_step", offset, !state.source_step_observed());
            trace_original_child_guard(
                "footprint.origin",
                offset,
                state.current_source_origin.as_ref() == Some(&site.source),
            );
            trace_original_child_guard(
                "footprint.image",
                offset,
                site.source.source_image().try_text().ok() == Some(source),
            );
            trace_original_child_guard(
                "footprint.store",
                offset,
                state.baseline.registry_snapshot == Some(registry.snapshot().semantic_key()),
            );
            trace_original_child_guard(
                "footprint.config",
                offset,
                config.normalized() == input.lexer_config().normalized(),
            );
        }
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
        #[cfg(test)]
        trace_original_child_guard(
            "footprint.declaration_layout",
            tokens.argv.first().map(|span| span.start()),
            advice.is_some(),
        );
        let advice = advice?;
        #[cfg(test)]
        trace_original_child_guard(
            "footprint.declaration_closed",
            tokens.argv.first().map(|span| span.start()),
            advice.closed_logical_source_lookup(),
        );
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

fn original_module_origin(
    origin: &tcl_names::origin::SourceOriginId,
    module: &crate::ir::Module,
) -> bool {
    let mut origin = origin;
    while let tcl_names::origin::SourceOriginKind::Derived { parent, .. } = origin.kind() {
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
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            let offset = Some(self.head.span().start());
            trace_original_child_guard(
                "body.head",
                offset,
                words.head_source().and_then(|head| head.word()) == Some(&self.head),
            );
            trace_original_child_guard(
                "body.image_config",
                offset,
                body.matches_source(self.head.image(), input.lexer_config()),
            );
            trace_original_child_guard("body.context", offset, body.matches_context(&context));
            trace_original_child_guard(
                "body.content",
                offset,
                body.original_container().content_span().ok() == Some(body.content_span()),
            );
        }
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
        #[cfg(test)]
        trace_original_child_guard(
            "body.current_interpreter",
            Some(self.head.span().start()),
            current == Some(true),
        );
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
            exact_read_names: projection.read_before_write_names.clone(),
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
mod tests;
