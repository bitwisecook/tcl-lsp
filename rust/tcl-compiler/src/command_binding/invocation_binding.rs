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

//! The binding observed at one invocation: defaults, joins, proved targets and source owners.

use super::binding_projection::source_binding_projection;
use super::command_identity::{AllocationIncarnation, CommandAllocationSite};
use super::compiled_invocation::{CompiledExecutionResidual, NativeInlineRejection};
use super::compiler_inventory::SourceRuntimeReachability;
use super::executed_script_source::ExecutedScriptSource;
use super::namespace_context::SourceNamespaceKey;
use super::source_origin::{SourceOriginId, SourceOriginKind};
use super::source_transition_advice::{
    OriginalLogicalSourceNameInput, OriginalSourceCommandTransitionAdvice,
};
use super::source_walk::{boxed_source_branch, join_evaluated_words};
use super::{
    BindingKind, DeferredImplementationId, OriginalSourceAnalysisOwner, ReceiverNamespaceContents,
    SourceCommandTarget, SourceConstructorEntry, SourceInvocationBinding, SourceLookupSnapshot,
    compiled_invocation,
};
use std::hash::Hash;
use std::sync::Arc;
use tcl_registry::CommandRegistry;

impl SourceConstructorEntry {
    /// Exact authored constructor declaration and its source instance.
    #[must_use]
    pub fn declaration(&self) -> &CommandAllocationSite {
        &self.declaration
    }
    /// Original evaluated constructor body, before any callback executes.
    #[must_use]
    pub fn body(&self) -> &ExecutedScriptSource {
        &self.body
    }
    /// Parsed native formal names and default values at the declaration.
    #[must_use]
    pub fn formals(&self) -> &[(String, Option<String>)] {
        &self.formals
    }
    /// Actual receiver-method frame selected for this retained body entry.
    #[must_use]
    pub fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        &self.frame
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(super) enum SourceObserverExecution {
    #[default]
    Unobserved,
    MayObserved,
}

impl SourceObserverExecution {
    pub(super) fn from_observed(observed: bool) -> Self {
        if observed {
            Self::MayObserved
        } else {
            Self::Unobserved
        }
    }

    pub(super) fn observed(self) -> bool {
        self == Self::MayObserved
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum OriginalArgumentCompletion {
    Unknown,
    Normal,
}

impl OriginalArgumentCompletion {
    pub(super) fn from_normal(normal: bool) -> Self {
        if normal { Self::Normal } else { Self::Unknown }
    }
    pub(super) fn is_normal(self) -> bool {
        self == Self::Normal
    }
    pub(super) fn join(&mut self, other: Self) {
        if !other.is_normal() {
            *self = Self::Unknown;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum SourceCompilerNameClosure {
    Unknown,
    Preserved,
}

impl From<bool> for SourceCompilerNameClosure {
    fn from(preserved: bool) -> Self {
        if preserved {
            Self::Preserved
        } else {
            Self::Unknown
        }
    }
}

impl SourceCompilerNameClosure {
    pub(super) fn is_preserved(self) -> bool {
        self == Self::Preserved
    }

    pub(super) fn join(&mut self, other: Self) {
        if !other.is_preserved() {
            *self = Self::Unknown;
        }
    }
}

/// The dispatch outcome only: the shared lookup, variable and analysis
/// snapshots a binding retains run to hundreds of megabytes when expanded.
impl std::fmt::Debug for SourceInvocationBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceInvocationBinding")
            .field("lookup_namespace", &self.lookup_namespace)
            .field("lookup_word", &self.lookup_word)
            .field("targets", &self.targets)
            .field("may_be_absent", &self.may_be_absent)
            .field("unknown", &self.unknown)
            .field("may_use_live_dispatch", &self.may_use_live_dispatch)
            .field("compiled_candidates", &self.compiled_candidates.len())
            .field(
                "compiled_named_candidates",
                &self.compiled_named_candidates.len(),
            )
            .field(
                "compiled_execution_residual",
                &self.compiled_execution_residual,
            )
            .field("native_inline_rejection", &self.native_inline_rejection)
            .field("native_handler_envelope", &self.native_handler_envelope)
            .field("provider_state_taint", &self.provider_state_taint)
            .field("declared_command", &self.declared_command)
            .field("catalogue_command", &self.catalogue_command)
            .field("variable_frame", &self.variable_frame)
            .field("known_namespaces", &self.known_namespaces)
            .field("evaluated_argument_values", &self.evaluated_argument_values)
            .finish_non_exhaustive()
    }
}

impl SourceInvocationBinding {
    /// Whether this invocation's lookup is closed and every implementation
    /// it may reach is a user procedure called with its written words: no
    /// unknown or possibly absent lookup (the `unknown` handler runs), no
    /// registry-backed handler and no alias prefix. What such a call does
    /// beyond its own frame is its procedure summary's to state; this
    /// grants no summary, frame or completion.
    pub(crate) fn targets_only_source_procedures(&self) -> bool {
        !self.unknown
            && !self.may_be_absent
            && !self.targets.is_empty()
            && self.targets.iter().all(|target| {
                target.kind == BindingKind::Proc
                    && !target.registry_backed
                    && target.prepended.is_empty()
            })
    }

    /// Passive process-local address only; this supplies no source or lookup authority.
    pub(crate) fn source_cut_trace_dispatch_origin(&self) -> Option<*const SourceOriginId> {
        self.dispatch_site
            .as_ref()
            .map(|site| Arc::as_ptr(&site.source))
    }

    pub(crate) fn has_original_source_layout_owner(&self) -> bool {
        self.lookup_state.is_some()
            || self.compiler_lookup_state.is_some()
            || self.declaration_layout_observations.is_some()
    }

    /// A retained non-point source purpose keeps its own terminal validation.
    /// This presence query supplies no selected facts or alternative source owner.
    pub(crate) fn has_nonpoint_original_source_name_role_owner(&self) -> bool {
        self.compiler_lookup_state.is_some()
            || self.compiler_policy.is_some()
            || self.original_compiler_words.is_some()
            || self.native_compilation_admission.is_some()
            || self.native_compiler_admission.is_some()
            || self.native_structured_preparation.is_some()
            || self.native_switch_preparation.is_some()
            || self.native_operand_layout.is_some()
            || self.conditional_expression_evaluations.is_some()
            || !self.compiled_candidates.is_empty()
            || !self.compiled_named_candidates.is_empty()
            || self.native_handler_envelope.is_some()
            || self.declaration_layout_observations.is_some()
            || self.original_logical_source_advice.is_some()
            || self.original_logical_declaration_targets.is_some()
            || self.original_authored_source_body_scope.is_some()
            || self.original_native_source_command.is_some()
    }

    /// Whether an original Logical source owner requires its own terminal query.
    pub(crate) fn has_original_logical_source_advice(&self) -> bool {
        self.original_logical_source_advice.is_some()
    }

    pub(crate) fn retain_original_logical_source_advice(
        &mut self,
        advice: OriginalSourceCommandTransitionAdvice,
    ) {
        if !self.has_original_source_layout_owner() && advice.logical_source_input().is_some() {
            self.original_logical_source_advice = Some(Arc::new(advice));
        }
    }

    pub(crate) fn original_analysis_attachment(&self) -> Option<&Arc<OriginalSourceAnalysisOwner>> {
        self.original_analysis_owner.as_ref()
    }

    pub(crate) fn retain_original_authored_source_body_scope(
        &mut self,
        scope: Arc<crate::registry_invocation::OriginalAuthoredSourceBodyScope>,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<()> {
        if self.has_original_source_layout_owner()
            || self.original_logical_source_advice.is_some()
            || self.original_native_source_command.is_some()
        {
            return None;
        }
        scope.original_words(tokens, scope.registry())?;
        self.original_analysis_owner = scope.original_analysis_attachment().cloned();
        self.original_authored_source_body_scope = Some(scope);
        Some(())
    }

    pub(crate) fn retain_original_authored_scalar_body_scope(
        &mut self,
        scope: Arc<crate::registry_invocation::OriginalAuthoredSourceBodyScope>,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<()> {
        let advice = self.original_authored_declaration_scalar_layout(tokens, scope.registry())?;
        if advice.has_unclosed_source_coverage() || scope.has_unclosed_declaration_source_coverage()
        {
            return None;
        }
        scope.original_words(tokens, scope.registry())?;
        if advice.namespace() != scope.namespace()
            || self
                .original_analysis_owner
                .as_ref()
                .zip(scope.original_analysis_attachment())
                .is_none_or(|(actual, original)| !Arc::ptr_eq(actual, original))
        {
            return None;
        }
        self.original_authored_source_body_scope = Some(scope);
        Some(())
    }

    /// Held scalar body syntax under its independent Standalone declaration lookup.
    /// Other source layouts retain their terminal role/control readers.
    pub(crate) fn original_authored_scalar_body_scope(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<&crate::registry_invocation::OriginalAuthoredSourceBodyScope> {
        if tokens.source_binding.as_ref() != Some(self) {
            return None;
        }
        let scope = self.original_authored_source_body_scope.as_deref()?;
        let advice = self.original_authored_declaration_scalar_layout(tokens, registry)?;
        if advice.has_unclosed_source_coverage() || scope.has_unclosed_declaration_source_coverage()
        {
            return None;
        }
        scope.original_words(tokens, registry)?;
        (advice.namespace() == scope.namespace()
            && self
                .original_analysis_owner
                .as_ref()
                .zip(scope.original_analysis_attachment())
                .is_some_and(|(actual, original)| Arc::ptr_eq(actual, original)))
        .then_some(scope)
    }

    pub(crate) fn has_original_authored_source_body_name_role_owner(&self) -> bool {
        self.original_authored_source_body_scope.is_some()
    }

    pub(crate) fn original_authored_source_body_scope_for_name_roles(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<&crate::registry_invocation::OriginalAuthoredSourceBodyScope> {
        if self.has_original_source_layout_owner()
            || self.original_logical_source_advice.is_some()
            || self.original_native_source_command.is_some()
            || tokens.source_binding.as_ref() != Some(self)
        {
            return None;
        }
        let scope = self.original_authored_source_body_scope.as_deref()?;
        scope.original_words(tokens, registry)?;
        Some(scope)
    }

    pub(crate) fn has_original_logical_source_name_role_owner(&self) -> bool {
        self.original_logical_source_advice.is_some()
    }

    /// Actual readonly Logical input for name-role metadata, without a point grant.
    /// A stronger retained source layout keeps its terminal owner route.
    pub(crate) fn original_logical_source_input_for_name_roles(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        if self.has_original_source_layout_owner() {
            return None;
        }
        let input = self
            .original_logical_source_advice
            .as_deref()?
            .logical_source_input()?;
        self.original_logical_source_words(tokens, input, registry)?;
        Some(input)
    }

    pub(crate) fn original_logical_source_site_for_name_roles(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<&CommandAllocationSite> {
        self.original_logical_source_input_for_name_roles(tokens, registry)?;
        Some(self.original_logical_source_advice.as_deref()?.site())
    }

    pub(crate) fn original_logical_source_words(
        &self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
        if self.has_original_source_layout_owner()
            || tokens.source_binding.as_ref() != Some(self)
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || !input.has_logical_source_name_context()
        {
            return None;
        }
        let advice = self.original_logical_source_advice.as_deref()?;
        let context = input.borrowed_context_registry();
        let image = advice.site().source.source_image();
        let config = input.lexer_config();
        if registry.snapshot().semantic_key() != context.commands().snapshot().semantic_key()
            || advice.logical_source_input() != Some(input)
            || !advice.matches_source(image, config)
            || !advice.matches_context(context)
            || crate::registry_invocation::original_native_compiler_words(
                image,
                tokens.words(),
                advice.site().offset,
                config,
            )?
            .as_slice()
                != advice.original_words()
        {
            return None;
        }
        crate::registry_invocation::source_structure::source_transition_words_from_advice(
            image.try_text().ok()?,
            config,
            context,
            advice.clone(),
        )
    }
}

impl Default for SourceInvocationBinding {
    fn default() -> Self {
        Self {
            compiled_candidates: Vec::new(),
            compiled_named_candidates: Vec::new(),
            may_use_live_dispatch: true,
            compiled_execution_residual: CompiledExecutionResidual::Closed,
            native_compilation_admission: None,
            native_compiler_admission: None,
            original_compiler_preserves_names: SourceCompilerNameClosure::Unknown,
            native_structured_preparation: None,
            native_switch_preparation: None,
            runtime_reachability: SourceRuntimeReachability::Unknown,
            entered_execution_observer: SourceObserverExecution::Unobserved,
            native_inline_rejection: NativeInlineRejection::None,
            native_handler_envelope: None,
            invocation_variable_reads: None,
            lookup_state: None,
            compiler_lookup_state: None,
            original_logical_source_advice: None,
            original_analysis_owner: None,
            original_source_operand_attachment: None,
            original_argument_syntax: None,
            original_authored_source_body_scope: None,
            original_native_source_command: None,
            declaration_layout_observations: None,
            declaration_flow_inventory: None,
            original_logical_declaration_targets: None,
            compiler_policy: None,
            original_compiler_words: None,
            original_written_words: None,
            conditional_expression_evaluations: None,
            receiver_namespace: ReceiverNamespaceContents::Created,
            lookup_namespace: String::new(),
            lookup_namespace_key: SourceNamespaceKey::default(),
            lookup_word: None,
            dispatch_site: None,
            provider_state_taint: Vec::new(),
            declared_command: None,
            catalogue_command: None,
            variable_context: Arc::default(),
            original_argument_completion: OriginalArgumentCompletion::Unknown,
            normal_variable_continuation: None,
            original_variable_operands: None,
            normal_result_observations: None,
            rhs_read_store_observations: None,
            object_callback_effects: None,
            variable_frame: crate::var_resolve::VariableExecutionFrame::default(),
            existing_namespace_cells: Vec::new(),
            namespace_cell_presence: crate::var_resolve::NamespaceCellPresence::default(),
            known_namespaces: Arc::from([]),
            evaluated_argument_values: Arc::default(),
            evaluated_argument_words: Arc::default(),
            frozen_written_words: None,
            frozen_written_names: None,
            frozen_head_object: None,
            method_prefix_arguments: Vec::new(),
            native_operand_layout: None,
            targets: Vec::new(),
            may_be_absent: false,
            unknown: false,
        }
    }
}

impl Hash for SourceInvocationBinding {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.compiled_candidates.hash(state);
        self.compiled_named_candidates.hash(state);
        self.may_use_live_dispatch.hash(state);
        self.compiled_execution_residual.hash(state);
        self.native_compilation_admission.hash(state);
        self.native_compiler_admission.hash(state);
        self.original_compiler_preserves_names.hash(state);
        self.native_structured_preparation.hash(state);
        self.native_switch_preparation.hash(state);
        self.native_operand_layout.hash(state);
        self.runtime_reachability.hash(state);
        self.entered_execution_observer.hash(state);
        self.native_inline_rejection.hash(state);
        self.native_handler_envelope.hash(state);
        self.invocation_variable_reads.hash(state);
        self.original_argument_completion.hash(state);
        self.normal_variable_continuation.hash(state);
        self.original_variable_operands.hash(state);
        self.rhs_read_store_observations.hash(state);
        self.object_callback_effects.hash(state);
        self.original_analysis_owner.hash(state);
        self.lookup_state.hash(state);
        self.compiler_lookup_state.hash(state);
        self.declaration_layout_observations.hash(state);
        self.declaration_flow_inventory.hash(state);
        self.original_logical_declaration_targets.hash(state);
        self.compiler_policy.hash(state);
        self.original_compiler_words.hash(state);
        self.original_written_words.hash(state);
        self.conditional_expression_evaluations.hash(state);
        self.receiver_namespace.hash(state);
        self.lookup_namespace.hash(state);
        self.lookup_namespace_key.hash(state);
        self.lookup_word.hash(state);
        self.dispatch_site.hash(state);
        self.targets.hash(state);
        self.unknown.hash(state);
        self.may_be_absent.hash(state);
        self.declared_command.hash(state);
        self.catalogue_command.hash(state);
        self.provider_state_taint.hash(state);
        self.evaluated_argument_values.hash(state);
        self.evaluated_argument_words.hash(state);
        self.frozen_written_words.hash(state);
        self.frozen_written_names.hash(state);
        self.frozen_head_object.hash(state);
        self.method_prefix_arguments.hash(state);
        if self.lookup_state.is_none() {
            self.variable_context.hash(state);
            self.variable_frame.hash(state);
            self.existing_namespace_cells.hash(state);
            self.namespace_cell_presence.hash(state);
            self.known_namespaces.hash(state);
        }
    }
}

/// A retained declaration whose lookup slot has not been changed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeclaredCommandCandidate {
    /// Name in the retained document/workspace declaration surface.
    pub name: String,
    /// Execution namespace used to select this declaration slot.
    pub lookup_namespace: String,
    /// Canonical lookup slot carrying the assistance contract.
    pub slot: String,
}

impl SourceInvocationBinding {
    /// Whether the head finds no command of its own and every target is the
    /// unresolved-command handler called with it, for a name the registry
    /// catalogues. Such a command (another dialect's, a vendor's or a
    /// package's) is the host's to provide, so its catalogued semantics stand
    /// in for the missing lookup (#2427).
    pub(crate) fn reaches_only_a_catalogued_absent_head(&self, registry: &CommandRegistry) -> bool {
        let Some(head) = self.lookup_word.as_deref() else {
            return false;
        };
        self.may_be_absent
            && (registry.get(head).is_some() || registry.known_in_any_dialect(head))
            && self.targets.iter().all(|target| {
                matches!(
                    target.prepended.first(),
                    Some(crate::registry_invocation::EffectiveInvocationWord::Literal(word))
                        if word == head
                )
            })
    }

    pub(crate) fn original_logical_prefix_source_input(
        &self,
    ) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        let state = &self.lookup_state.as_ref()?.state;
        let input = state.logical_source_name_advice_input()?;
        (input.has_logical_source_name_context()
            && state.baseline.native_entry.is_none()
            && !state.baseline.unknown_entry
            && state.baseline.execution_name_policy.is_none()
            && state.baseline.hosted_execution_context.is_none())
        .then_some(input)
    }
}

impl SourceCommandTarget {
    /// Captured prefix inputs from this exact selected implementation chain.
    /// A missing or conflicting producer never falls back to literal bytes.
    #[must_use]
    pub fn original_prepended_name_inputs(
        &self,
    ) -> Option<&[crate::signature_scan::scope::SignatureSourceNameInput]> {
        let inputs = self.original_prepended.as_deref()?;
        (inputs.len() == self.prepended.len()).then_some(inputs)
    }
    /// Genuine captured Logical source words, independently of Native prefix inputs.
    pub(crate) fn original_logical_prepended_name_inputs(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> Option<&[crate::signature_scan::scope::SignatureSourceNameInput]> {
        let prefix = self.logical_original_prefix.as_ref()?;
        let inputs = prefix.inputs()?;
        (prefix.matches_source_input(input, image, config)
            && inputs.len() == self.prepended.len()
            && inputs
                .iter()
                .zip(&self.prepended)
                .all(|(input, word)| word.literal_bytes() == Some(input.bytes())))
        .then_some(inputs)
    }

    /// Captured Logical source data from the same actual alias allocations.
    /// This has no Native/authored name-key policy or written call-site anchor.
    pub(crate) fn original_logical_prepended_source_inputs(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> Option<&[OriginalLogicalSourceNameInput]> {
        let prefix = self.logical_original_prefix.as_ref()?;
        let inputs = prefix.source_inputs();
        (prefix.matches_source_words(input, image, config)
            && inputs.len() == self.prepended.len()
            && inputs
                .iter()
                .zip(&self.prepended)
                .all(|(input, word)| word.literal_bytes() == Some(input.bytes())))
        .then_some(inputs)
    }

    /// Original registry descriptor identity, independent of the current slot.
    /// Native issuers preserve their exact identifier bytes; authored registry
    /// rows may use a rooted descriptor name. This supplies no lookup spelling.
    #[must_use]
    pub fn registry_identity(&self) -> Option<&str> {
        self.registry_backed.then_some(self.command.as_str())
    }
}

impl SourceInvocationBinding {
    /// Explicit absence of bounded runtime dispatch and compiler admission.
    #[must_use]
    pub fn unknown() -> Self {
        Self {
            unknown: true,
            ..Self::default()
        }
    }

    /// Explicit availability phase retained at this exact execution point.
    /// Missing entry evidence supplies no phase assumption.
    #[must_use]
    pub fn invocation_realm(&self) -> Option<tcl_dialect::model::InvocationRealm> {
        self.lookup_state.as_ref().map(|snapshot| snapshot.realm)
    }

    /// Whether this point was reached in a closed, transparent command world.
    pub(crate) fn reached_in_closed_world(&self) -> bool {
        self.lookup_state.as_ref().is_some_and(|point| {
            !point.state.has_opaque_domain() && !point.state.unclosed_command_world
        })
    }

    /// Command-word value retained at this lookup, before alias composition.
    /// Different reaching values join to unknown rather than borrowing the
    /// terminal implementation's name.
    #[must_use]
    pub fn evaluated_command_word(&self) -> Option<&str> {
        self.lookup_word.as_deref()
    }

    /// Relocate physical cell worlds without changing command allocations or source ownership.
    pub fn relocate_variable_proofs(
        &mut self,
        relocation: &crate::var_resolve::VariableProofRelocation,
    ) {
        self.relocate_invocation_reads(relocation);
        self.variable_context = Arc::new(self.variable_context.relocated(relocation));
        self.normal_variable_continuation = self
            .normal_variable_continuation
            .as_ref()
            .map(|context| Arc::new(context.relocated(relocation)));
        self.variable_frame = relocation.frame(&self.variable_frame);
        // Captured receiver identities have no physical-frame relocation grant.
        self.method_prefix_arguments.clear();
        self.original_argument_completion = OriginalArgumentCompletion::Unknown;
        self.normal_result_observations = None;
        self.rhs_read_store_observations = None;
        if let Some(snapshot) = &self.lookup_state {
            let mut state = snapshot.state.clone();
            state.source_variables = Arc::new(state.source_variables.relocated(relocation));
            state.variable_frame = relocation.frame(&state.variable_frame);
            self.lookup_state = Some(Arc::new(SourceLookupSnapshot::in_realm(
                state,
                snapshot.realm,
            )));
        }
    }

    /// Contents bytes frozen for one original written argument, before later argv.
    /// This excludes alias prefixes and does not flatten an expanded word into
    /// actual argument positions. It grants no native object representation,
    /// coercion-erasure or command-dispatch licence.
    #[must_use]
    pub fn evaluated_written_argument_value(&self, argument: usize) -> Option<&str> {
        self.evaluated_argument_values.get(argument)?.as_deref()
    }

    /// Whether this point belongs to the fresh F5 rule-loader world
    /// ([`SourceAnalysisEntry::for_fresh_hosted_source`]).
    pub(crate) fn in_hosted_rule_loader_world(&self) -> bool {
        self.lookup_state
            .as_ref()
            .is_some_and(|snapshot| snapshot.state.hosted_rule_loader_world())
    }

    /// Resolve another command word in this exact immutable interpreter point.
    /// Uncertainty and possible document implementations remain represented.
    #[must_use]
    pub fn lookup_command_word(&self, head: &str) -> Self {
        self.lookup_state.as_ref().map_or_else(
            || Self {
                unknown: true,
                variable_context: Arc::clone(&self.variable_context),
                variable_frame: self.variable_frame.clone(),
                ..Self::default()
            },
            |snapshot| {
                let mut binding =
                    source_binding_projection(&snapshot.state, head, &self.lookup_namespace_key);
                binding.lookup_state = Some(Arc::clone(snapshot));
                binding
            },
        )
    }

    /// Actual immutable command store belonging to this complete source vector.
    /// This reborrow supplies metadata only, without lookup or execution authority.
    pub(crate) fn retained_registry_for_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&tcl_registry::CommandRegistry> {
        if !self.has_original_source_layout_owner()
            && let Some(scope) = self.original_authored_source_body_scope.as_deref()
        {
            return Some(
                self.original_authored_source_body_scope_for_name_roles(tokens, scope.registry())?
                    .registry(),
            );
        }
        Some(
            self.retained_registry_snapshot_for_tokens(tokens)?
                .registry(),
        )
    }

    pub(crate) fn retained_registry_snapshot_for_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&tcl_registry::RegistrySnapshot> {
        let owner = self
            .lookup_state
            .as_ref()?
            .state
            .baseline
            .registry_owner
            .as_ref()?;
        self.original_lexer_config_for_tokens(tokens)?;
        self.original_invocation_metadata_at_point(tokens, owner.registry())
            .ok()?;
        Some(owner)
    }

    pub(crate) fn frozen_written_words(
        &self,
    ) -> Option<&[crate::registry_invocation::EffectiveInvocationWord]> {
        self.frozen_written_words.as_deref()
    }

    pub(super) fn same_dispatch_projection(&self, other: &Self) -> bool {
        let mut left = self.clone();
        let mut right = other.clone();
        left.lookup_state = None;
        right.lookup_state = None;
        left.normal_result_observations = None;
        right.normal_result_observations = None;
        left.declaration_layout_observations = None;
        left.declaration_flow_inventory = None;
        right.declaration_layout_observations = None;
        right.declaration_flow_inventory = None;
        left.original_logical_declaration_targets = None;
        right.original_logical_declaration_targets = None;
        left == right
    }

    fn join_original_operand_receipts(&mut self, other: &Self) {
        if self.original_source_operand_attachment != other.original_source_operand_attachment {
            self.original_source_operand_attachment = None;
        }
        if self.original_analysis_owner != other.original_analysis_owner {
            self.original_analysis_owner = None;
        }
        if self.original_argument_syntax != other.original_argument_syntax {
            self.original_argument_syntax = None;
        }
        if self.original_authored_source_body_scope != other.original_authored_source_body_scope {
            self.original_authored_source_body_scope = None;
        }
        if self.original_native_source_command != other.original_native_source_command {
            self.original_native_source_command = None;
        }
        if self.original_logical_source_advice != other.original_logical_source_advice {
            self.original_logical_source_advice = None;
        }
        if self.original_variable_operands != other.original_variable_operands {
            self.original_variable_operands = None;
        }
        if other.entered_execution_observer.observed() {
            self.entered_execution_observer = SourceObserverExecution::MayObserved;
        }
        if self.runtime_reachability != other.runtime_reachability {
            self.runtime_reachability = SourceRuntimeReachability::Unknown;
        }
        if self.native_operand_layout != other.native_operand_layout {
            self.native_operand_layout = None;
        }
        if self.original_written_words != other.original_written_words {
            self.original_written_words = None;
        }
        if self.original_compiler_words != other.original_compiler_words {
            self.original_compiler_words = None;
        }
        if self.conditional_expression_evaluations != other.conditional_expression_evaluations {
            self.conditional_expression_evaluations = None;
        }
        if other.receiver_namespace == ReceiverNamespaceContents::Unproven {
            self.receiver_namespace = ReceiverNamespaceContents::Unproven;
        }
        if self.frozen_written_words != other.frozen_written_words {
            self.frozen_written_words = None;
        }
        if self.frozen_written_names != other.frozen_written_names {
            self.frozen_written_names = None;
        }
        if self.frozen_head_object != other.frozen_head_object {
            self.frozen_head_object = None;
        }
        self.method_prefix_arguments
            .retain(|prefix| other.method_prefix_arguments.contains(prefix));
    }

    pub(super) fn join(&mut self, other: &Self) {
        if self.original_logical_declaration_targets != other.original_logical_declaration_targets {
            self.original_logical_declaration_targets = None;
        }
        self.join_original_operand_receipts(other);
        self.join_compiled_execution(other);
        self.join_invocation_reads(other);
        self.original_argument_completion
            .join(other.original_argument_completion);
        self.join_normal_variable_continuation(other);
        if self.normal_result_observations != other.normal_result_observations {
            self.normal_result_observations = None;
        }
        if self.rhs_read_store_observations != other.rhs_read_store_observations {
            self.rhs_read_store_observations = None;
        }
        self.join_object_callback_effects(other);
        join_evaluated_words(
            &mut self.evaluated_argument_values,
            &other.evaluated_argument_values,
        );
        compiled_invocation::join_argument_words(
            &mut self.evaluated_argument_words,
            &other.evaluated_argument_words,
        );
        if let (Some(state), Some(incoming)) = (&mut self.lookup_state, &other.lookup_state)
            && state.realm == incoming.realm
        {
            let snapshot = Arc::make_mut(state);
            snapshot.state.join(&incoming.state);
            // The joined state owns a new immutable semantic point. A cloned
            // snapshot must not retain its predecessor's derived memos.
            snapshot.forget_derived();
        } else {
            self.lookup_state = None;
        }
        if self.lookup_namespace_key != other.lookup_namespace_key {
            self.lookup_state = None;
        }
        if self.compiler_lookup_state != other.compiler_lookup_state {
            self.compiler_lookup_state = None;
            self.native_structured_preparation = None;
            self.native_switch_preparation = None;
        }
        if self.declaration_flow_inventory != other.declaration_flow_inventory {
            self.declaration_flow_inventory = None;
        }
        if self.declaration_layout_observations != other.declaration_layout_observations {
            self.declaration_layout_observations = None;
        }
        if self.compiler_policy != other.compiler_policy {
            self.compiler_policy = None;
        }
        if self.dispatch_site != other.dispatch_site {
            self.dispatch_site = None;
        }
        if self.lookup_word != other.lookup_word {
            self.lookup_word = None;
        }
        self.provider_state_taint
            .extend(other.provider_state_taint.iter().cloned());
        self.provider_state_taint.sort();
        self.provider_state_taint.dedup();
        if self.declared_command != other.declared_command {
            self.declared_command = None;
        }
        if self.catalogue_command != other.catalogue_command {
            self.catalogue_command = None;
        }
        self.unknown |= other.unknown;
        self.may_be_absent |= other.may_be_absent;
        if !Arc::ptr_eq(&self.known_namespaces, &other.known_namespaces)
            && self.known_namespaces != other.known_namespaces
        {
            self.known_namespaces = self
                .known_namespaces
                .iter()
                .filter(|namespace| other.known_namespaces.contains(namespace))
                .cloned()
                .collect();
        }
        self.existing_namespace_cells
            .retain(|cell| other.existing_namespace_cells.contains(cell));
        self.namespace_cell_presence
            .join(&other.namespace_cell_presence);
        let mut variables = self.variable_context.as_ref().clone();
        variables.join(&other.variable_context);
        if self.variable_frame != other.variable_frame {
            self.variable_frame = crate::var_resolve::VariableExecutionFrame::Unknown;
        }
        if self.variable_frame == crate::var_resolve::VariableExecutionFrame::Unknown {
            variables = variables.in_frame(&self.variable_frame);
        }
        self.variable_context = Arc::new(variables);
        for target in &other.targets {
            if !self.targets.contains(target) {
                self.targets.push(target.clone());
            }
        }
    }

    /// A unique target, provided every possible path reaches that target.
    #[must_use]
    pub fn proved_target(&self) -> Option<&SourceCommandTarget> {
        (!self.unknown && !self.may_be_absent && self.targets.len() == 1).then(|| &self.targets[0])
    }

    /// Original class definition manufacturer, without invoking its constructor.
    /// The returned target retains the exact live class identity and allocation.
    #[must_use]
    pub fn proved_class_definition_factory(&self) -> Option<(&str, &SourceCommandTarget)> {
        if self.runtime_reachability == SourceRuntimeReachability::Conditional {
            return None;
        }
        let target = self
            .proved_target()
            .filter(|target| target.kind == BindingKind::Class)?;
        if target.identity.as_ref()?.runtime.is_none()
            && target
                .implementation_allocation
                .as_ref()
                .is_some_and(|allocation| {
                    allocation.incarnation == AllocationIncarnation::RepeatedFresh
                })
        {
            return None;
        }
        let snapshot = &self.lookup_state.as_ref()?.state;
        if snapshot.tainted_object_dispatch.contains("*")
            || snapshot.tainted_object_dispatch.contains(&target.command)
        {
            return None;
        }
        let definition = snapshot.class_definitions.get(target.identity.as_ref()?)?;
        (definition.dispatcher.is_none()
            && target.implementation_generation == definition.implementation_generation
            && snapshot.class_definition_dependencies_hold(definition))
        .then_some((definition.factory.as_str(), target))
    }

    /// A class result whose original manufacturer and construction closure
    /// are proved at this invocation, with the captured effective arguments.
    #[must_use]
    pub fn proved_construction_result(&self, registry: &CommandRegistry) -> Option<String> {
        let target = self
            .proved_execution_target()
            .filter(|target| target.kind == BindingKind::Class)?;
        let snapshot = self.lookup_state.as_ref()?;
        let words = target
            .prepended
            .iter()
            .cloned()
            .chain(self.evaluated_argument_values.iter().map(|value| {
                value.as_ref().map_or(
                    crate::registry_invocation::EffectiveInvocationWord::Dynamic,
                    |value| {
                        crate::registry_invocation::EffectiveInvocationWord::Literal(value.clone())
                    },
                )
            }))
            .collect::<Vec<_>>();
        let namespace = self.lookup_namespace_key.advisory_key()?;
        let mut state = boxed_source_branch(&snapshot.state);
        state
            .apply_bounded_constructor(
                target,
                &words,
                registry,
                &namespace,
                self.invocation_realm()?,
            )
            .then(|| target.command.clone())
    }

    /// Original constructor entry and its actual declaring class provider.
    /// The payload offset is in the composed effective post-head argv. Callback
    /// completion and the result's object class remain independent unknowns.
    #[must_use]
    pub fn constructor_entry(
        &self,
        registry: &CommandRegistry,
    ) -> Option<(&SourceCommandTarget, &SourceConstructorEntry, usize)> {
        let (factory, target) = self.proved_class_definition_factory()?;
        let snapshot = &self.lookup_state.as_ref()?.state;
        let grammar = registry.native_default_construction_grammar(
            factory,
            snapshot.baseline.dialect?,
            self.invocation_realm()?,
        )?;
        if !snapshot.default_construction_dependencies_hold(grammar) {
            return None;
        }
        let first = match target.prepended.first() {
            Some(word) => word.as_registry_word().literal()?,
            None => self.evaluated_argument_values.first()?.as_deref()?,
        };
        let manufacturer = grammar.manufacturer(first)?;
        let definition = snapshot.class_definitions.get(target.identity.as_ref()?)?;
        let entry = definition.constructor_entry.as_deref()?;
        let provider = definition.constructor_provider.as_ref()?;
        if !snapshot.retained_target_is_current(provider)
            || !snapshot.class_definition_dependencies_hold(definition)
        {
            return None;
        }
        let payload_from = usize::from(manufacturer.constructor_args_from);
        let payload_count = target
            .prepended
            .len()
            .checked_add(self.evaluated_argument_values.len())?
            .checked_sub(payload_from)?;
        let parameters = entry
            .formals
            .iter()
            .map(
                |(name, default)| tcl_syntax::formal_params::FormalParameter {
                    name: name.clone(),
                    default: default.clone(),
                },
            )
            .collect::<Vec<_>>();
        tcl_syntax::formal_params::bind_formal_arguments(
            &parameters,
            payload_count,
            snapshot.baseline.dialect?.parameter_grammar()?,
        )
        .ok()?;
        Some((provider, entry, payload_from))
    }
}

impl SourceCommandTarget {
    /// Whether this retained implementation was allocated by the indicated
    /// authored declaration. Derived sources and repeated fresh summaries do
    /// not become that implementation merely by sharing its numeric offset.
    #[must_use]
    pub fn matches_authored_implementation(&self, source: &str, offset: u32) -> bool {
        self.matches_authored_implementation_image(
            &tcl_lexer::SourceImage::document(source),
            offset,
        )
    }

    /// Match the implementation's original bytes and source input channel.
    #[must_use]
    pub fn matches_authored_implementation_image(
        &self,
        source: &tcl_lexer::SourceImage,
        offset: u32,
    ) -> bool {
        self.implementation_generation == offset
            && self.implementation_allocation.as_ref().is_some_and(|allocation| {
                allocation.site.offset == offset
                    && allocation.incarnation != AllocationIncarnation::RepeatedFresh
                    && matches!(allocation.site.source.kind(), SourceOriginKind::Authored(text) if text == source)
            })
    }

    pub(super) fn implementation_id(&self) -> DeferredImplementationId {
        DeferredImplementationId {
            command: self
                .identity
                .as_ref()
                .map_or_else(|| self.command.clone(), |identity| identity.origin.clone()),
            generation: self.implementation_generation,
            allocation: self.implementation_allocation.clone(),
        }
    }
}
