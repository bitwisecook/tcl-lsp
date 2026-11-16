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

//! Program-point variable bindings. Registry transitions select the grammar;
//! this owner follows cell links and joins state along executable CFG edges.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{
    CallerFrameSelection, CommandRegistry, FrameLevel, NamespaceTransition,
    NamespaceTransitionTarget, StateTransition, StateTransitionDomain, StateTransitionKnowledge,
    TraceTarget, TraceTransition, Traits, VariableAliasTarget, VariableCellAliasTransition,
};

use crate::cfg::{BlockId, Function};
use crate::ir::Statement;
use crate::place::{self, CellGeneration, CellIdentity, CellOwner, Place, PlaceKind};
#[cfg(test)]
use crate::var_resolve::resolve_place;
use crate::var_resolve::{
    ResolveContext, cell_key, project_access, resolve_literal_access, resolve_literal_place,
    trace_key,
};

/// Exact effective post-head operands retained by the original invocation
/// producer. A missing source facet never falls back to its logical display.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OriginalVariableInvocation {
    compiled_operands: Vec<
        Option<
            crate::command_binding::original_variable_compilation::OriginalCompiledVariableOperand,
        >,
    >,
    inputs: Vec<Option<crate::signature_scan::scope::SignatureSourceNameInput>>,
    compiled_locals: Vec<
        Option<
            crate::command_binding::original_variable_compilation::OriginalCompiledNamespaceLocal,
        >,
    >,
}

impl OriginalVariableInvocation {
    pub(crate) fn argument_count(&self) -> usize {
        self.inputs.len()
    }

    pub(crate) fn from_original_inputs(
        inputs: Vec<Option<crate::signature_scan::scope::SignatureSourceNameInput>>,
        compiled_locals: Vec<Option<crate::command_binding::original_variable_compilation::OriginalCompiledNamespaceLocal>>,
    ) -> Self {
        Self {
            inputs,
            compiled_locals,
            compiled_operands: Vec::new(),
        }
    }

    pub(crate) fn with_compiled_operands(
        mut self,
        operands: Vec<Option<crate::command_binding::original_variable_compilation::OriginalCompiledVariableOperand>>,
    ) -> Self {
        self.compiled_operands = operands;
        self
    }

    pub(crate) fn access(
        &self,
        argument_index: usize,
        context: &ResolveContext,
        registry: &CommandRegistry,
        operation: tcl_registry::TraceOperation,
        whole_array: bool,
    ) -> Place {
        if let Some(Some(operand)) = self.compiled_operands.get(argument_index) {
            return operand.resolve(context, registry, whole_array, operation);
        }
        self.input(argument_index, context)
            .map_or_else(place::unknown_top, |input| {
                crate::var_resolve::resolve_original_name_input(
                    input,
                    context,
                    registry,
                    whole_array,
                    operation,
                )
            })
    }

    pub(crate) fn input(
        &self,
        argument_index: usize,
        context: &ResolveContext,
    ) -> Option<&crate::signature_scan::scope::SignatureSourceNameInput> {
        self.inputs
            .get(argument_index)?
            .as_ref()
            .filter(|input| input.is_current(context))
    }

    pub(crate) fn trace_subject_access(
        &self,
        argument_index: usize,
        context: &ResolveContext,
        registry: &CommandRegistry,
    ) -> Place {
        let Some(input) = self.input(argument_index, context) else {
            return place::unknown_top();
        };
        let Ok(subject) = input
            .policy()
            .recipe()
            .trace_registration_input(input.bytes())
        else {
            return place::unknown_top();
        };
        crate::var_resolve::resolve_evaluated_variable_input(
            tcl_syntax::naming::NativeVariableInputForm::Combined(subject.selected()),
            context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        )
    }

    /// Jim unset addresses the actual local table independently of its
    /// retained static fallback. Other engines cannot borrow this purpose.
    pub(crate) fn raw_unset_slot(
        &self,
        argument_index: usize,
        context: &ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<Place> {
        let input = self.input(argument_index, context)?;
        if context
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            != Some(input.policy())
        {
            return None;
        }
        input.policy().recipe().is_jim084().then(|| {
            crate::var_resolve::resolve_original_alias_destination_bytes(
                input.bytes(),
                context,
                registry,
            )
        })
    }

    pub(crate) fn compiled_local(
        &self,
        argument_index: usize,
    ) -> Option<
        &crate::command_binding::original_variable_compilation::OriginalCompiledNamespaceLocal,
    > {
        self.compiled_locals.get(argument_index)?.as_ref()
    }

    fn subject_input(
        &self,
        subject: &tcl_registry::TransitionSubject,
        context: &ResolveContext,
    ) -> Option<&crate::signature_scan::scope::SignatureSourceNameInput> {
        self.input(subject.argument_index()?, context)
    }
}

/// A quiet, positioned scalar write in a positively selected Logical model.
/// Its contents and cell belong to source advice, independently of Native
/// allocation, handler completion, runtime effects and executable erasure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConditionalLogicalScalarStore {
    before: ResolveContext,
    after: ResolveContext,
    input: crate::analyser::ResolvedAnalysisInput,
    words: std::sync::Arc<[crate::ir::WordExpr]>,
    site: crate::command_binding::CommandAllocationSite,
    name: String,
    /// The stored literal; `None` for a substituted value word, whose
    /// contents the store does not know.
    value: Option<String>,
}

impl ConditionalLogicalScalarStore {
    pub(crate) fn retain(
        before: &ResolveContext,
        after: &ResolveContext,
        input: &crate::analyser::ResolvedAnalysisInput,
        words: std::sync::Arc<[crate::ir::WordExpr]>,
        site: crate::command_binding::CommandAllocationSite,
        (name, value): (String, Option<String>),
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let store = Self {
            before: before.clone(),
            after: after.clone(),
            input: input.clone(),
            words,
            site,
            name,
            value,
        };
        store.definition_place(registry)?;
        Some(store)
    }

    pub(crate) fn after(&self) -> &ResolveContext {
        &self.after
    }
    pub(crate) fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    /// Original variable input retained by this conditional source store.
    /// This is readonly name advice, without an editable word or Native grant.
    pub(crate) fn source_name(&self) -> &str {
        &self.name
    }

    pub(crate) fn definition_place(&self, registry: &CommandRegistry) -> Option<Place> {
        let after = &self.after;
        // A hosted F5 rule's advisory walk runs under its authored naming
        // simulation and handler context; the cell must still be the
        // handler's own activation.
        let hosted = self.input.has_hosted_source_name_context();
        if self.before.namespace_identity != after.namespace_identity
            || !matches!(
                after.namespace_identity,
                Some(crate::command_binding::SourceNamespaceKey::Authored(_))
            )
            || !(self.input.has_logical_source_name_context() || hosted)
            || self.before.interpreter.is_some()
            || self.before.execution.is_some()
            || after.interpreter.is_some()
            || after.execution.is_some()
            || (!hosted
                && (after.execution_name_policy.is_some()
                    || after.hosted_execution_context.is_some()))
            || self.before.dynamic_bindings
            || self.before.dynamic_traces
            || after.dynamic_bindings
            || after.dynamic_traces
        {
            return None;
        }
        let place = resolve_literal_access(
            &self.name,
            after,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        let cell = place.cell.as_ref()?;
        if place.dynamic
            || place.observed
            || place.kind != PlaceKind::Scalar
            || place.index.is_some()
            || cell.interpreter.is_some()
            || cell.execution.is_some()
            || cell.generation == CellGeneration::Unknown
            || after
                .generations
                .get(&cell_key(&place))
                .copied()
                .unwrap_or_default()
                != cell.generation
            || !after.contents_have_source(&place, &self.site.source)
            || after.contents_origin(&place)
                != crate::var_resolve::ContentsOrigin::WrittenAt(self.site.offset)
            || after.literal_value(&self.name, registry) != self.value.as_deref()
        {
            return None;
        }
        let local = match &cell.owner {
            CellOwner::Activation(activation) => {
                after.activation.as_ref() == Some(activation) && cell.storage_domain.is_none()
            }
            CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_) if hosted => false,
            CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_) => {
                after.namespace_footprint(&place) == after.namespace_identity
                    && matches!(
                        cell.storage_domain,
                        None | Some(tcl_registry::f5::VariableStorageDomain::InterpreterNamespace)
                    )
            }
            _ => false,
        };
        local.then_some(place)
    }
}

/// Reaching binding environment before every operation and terminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointResolveContexts {
    entry: ResolveContext,
    statements: BTreeMap<(BlockId, usize), ResolveContext>,
    terminators: BTreeMap<BlockId, ResolveContext>,
    after: BTreeMap<(BlockId, usize), ResolveContext>,
    reads: BTreeMap<(BlockId, usize), Vec<crate::command_binding::SourceVariableAccess>>,
    source_tokens: BTreeMap<(BlockId, usize), crate::ir::CommandTokens>,
    metadata_context: crate::registry_invocation::OwnedInvocationMetadataContext,
    unknown_argument_points: BTreeSet<(BlockId, usize)>,
    logical_scalar_stores: BTreeMap<(BlockId, usize), ConditionalLogicalScalarStore>,
}

impl Function {
    /// Borrow the original evaluated words for an exact statement or terminator.
    /// `usize::MAX` selects the terminator. Generated boundaries that do not
    /// evaluate original words and absent source projections yield no carrier.
    /// Transforms replacing an operation must invalidate its retained binding site.
    #[must_use]
    pub fn source_tokens_at(
        &self,
        block: BlockId,
        index: usize,
    ) -> Option<&crate::ir::CommandTokens> {
        self.source_input_tokens_at(block, index)
            .filter(|tokens| tokens.evaluates_words())
    }

    /// Borrow evaluated words or previously captured iteration value inputs.
    /// Iteration input carriers describe pre-loop values; they do not license
    /// invoking a command, reevaluating words or adding new variable reads.
    #[must_use]
    pub fn source_input_tokens_at(
        &self,
        block: BlockId,
        index: usize,
    ) -> Option<&crate::ir::CommandTokens> {
        let block_data = self.blocks.get(&block)?;
        let tokens = if index == usize::MAX {
            match &block_data.terminator {
                Some(crate::cfg::Terminator::Branch { .. })
                    if block_data.statements.iter().any(|statement| {
                        statement.tokens().is_some_and(|tokens| {
                            matches!(
                                tokens.synthetic,
                                Some(crate::ir::SyntheticMarker::IterationBindings(_))
                            )
                        })
                    }) =>
                {
                    None
                }
                Some(crate::cfg::Terminator::Branch { .. }) => self
                    .condition_binding_sites
                    .get(&block)
                    .or_else(|| self.command_boundary_sites.get(&block))
                    .and_then(|site| site.source_tokens.as_deref()),
                Some(crate::cfg::Terminator::Return {
                    tokens: Some(tokens),
                    ..
                }) => Some(tokens.as_ref()),
                // An implicit function exit has no operand evaluation. An
                // observed Complete follows the retained original statement,
                // whose argv reads have already occurred. A stale structured
                // boundary must not create another read at either exit or at
                // a jump. Runtime replay ownership is independent of operand
                // evaluation; explicit argument markers own cached loop inputs.
                Some(
                    crate::cfg::Terminator::Return { tokens: None, .. }
                    | crate::cfg::Terminator::Complete { .. }
                    | crate::cfg::Terminator::Goto { .. },
                ) => None,
                _ => self
                    .command_boundary_sites
                    .get(&block)
                    .and_then(|site| site.source_tokens.as_deref()),
            }
        } else {
            let statement = block_data.statements.get(index)?;
            crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
                &self.command_binding_sites,
                statement,
            )
        };
        tokens.filter(|tokens| {
            !source_free_effect_marker(tokens)
                && (tokens.evaluates_words()
                    || matches!(
                        tokens.synthetic,
                        Some(crate::ir::SyntheticMarker::IterationBindings(_))
                    ))
        })
    }
}

fn source_free_effect_marker(tokens: &crate::ir::CommandTokens) -> bool {
    use crate::ir::SyntheticMarker;
    match tokens.synthetic {
        Some(
            marker @ (SyntheticMarker::Condition
            | SyntheticMarker::EmptyClause
            | SyntheticMarker::UpvarInvalidate
            | SyntheticMarker::RegistryBarrier
            | SyntheticMarker::WordEffects(_)
            | SyntheticMarker::GlobalFrameScript
            | SyntheticMarker::CallerFrameOpaque
            | SyntheticMarker::UnseenCall
            | SyntheticMarker::ArmWrites),
        ) => tokens == &crate::ir::CommandTokens::marker(marker),
        _ => false,
    }
}

impl PointResolveContexts {
    /// Actual CFG input owner. It does not authenticate a changed whole Module.
    pub(crate) fn source_metadata_owner(
        &self,
    ) -> &crate::registry_invocation::OwnedInvocationMetadataContext {
        &self.metadata_context
    }

    /// Reborrow only an exact current source-owned conditional store. Changing
    /// input, registry, written words or either point context withdraws it.
    pub(crate) fn conditional_logical_scalar_store_at(
        &self,
        block: BlockId,
        index: usize,
        registry: &CommandRegistry,
    ) -> Option<&ConditionalLogicalScalarStore> {
        let store = self.logical_scalar_stores.get(&(block, index))?;
        let metadata = self
            .metadata_context
            .metadata_context(registry)
            .and_then(crate::registry_invocation::InvocationMetadataSelection::context)?;
        if metadata.source_analysis_input() != Some(&store.input)
            || self.statements.get(&(block, index)) != Some(&store.before)
            || self.after.get(&(block, index)) != Some(&store.after)
        {
            return None;
        }
        let tokens = self.source_tokens_at(block, index)?;
        (tokens.words() == store.words.as_ref()).then_some(())?;
        let metadata = tokens
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_input(tokens, &store.input, registry)?;
        let purpose = tokens
            .source_binding
            .as_ref()?
            .original_invocation_source_purpose(tokens, metadata, registry)?;
        (purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical
            || (purpose
                == crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
                && store.input.has_hosted_source_name_context()))
        .then_some(())?;
        store.definition_place(registry)?;
        Some(store)
    }

    pub(crate) fn has_unknown_source_arguments(&self) -> bool {
        !self.unknown_argument_points.is_empty()
    }

    /// [`Self::has_unknown_source_arguments`] over only the blocks `admit`
    /// accepts: a block that never runs evaluates no argument.
    pub(crate) fn has_unknown_source_arguments_in(&self, admit: &dyn Fn(BlockId) -> bool) -> bool {
        self.unknown_argument_points
            .iter()
            .any(|(block, _)| admit(*block))
    }

    pub(crate) fn source_arguments_unknown_at(
        &self,
        block: BlockId,
        index: usize,
        registry: &CommandRegistry,
    ) -> bool {
        self.unknown_argument_points.contains(&(block, index))
            || self.source_tokens_at(block, index).is_some_and(|tokens| {
                crate::place_bridge::argument_calls(
                    tokens,
                    registry,
                    crate::place_bridge::ArgumentReadInput::for_owner(
                        &self.metadata_context,
                        registry,
                    ),
                )
                .is_none()
            })
    }

    pub(crate) fn source_token_inventory(
        &self,
    ) -> &BTreeMap<(BlockId, usize), crate::ir::CommandTokens> {
        &self.source_tokens
    }

    /// Move retained lexical carriers with their CFG and SSA statement coordinates.
    /// Immutable source issuers and physical binding identities remain unchanged.
    pub(crate) fn rebase_source_tokens(&mut self, delta: i64) {
        for tokens in self.source_tokens.values_mut() {
            crate::lattice_rebase::rebase_command_tokens(tokens, delta);
        }
        for accesses in self.reads.values_mut() {
            for access in accesses {
                crate::lattice_rebase::rebase_span(&mut access.source.span, delta);
            }
        }
    }

    /// Replace template source ownership with exact carriers from the actual body.
    /// Lexical coordinates must already agree; missing sites decline the artifact.
    pub fn restore_source_tokens(
        &mut self,
        originals: &BTreeMap<u32, crate::ir::CommandTokens>,
    ) -> bool {
        let mut replacements = BTreeMap::new();
        for (&point, tokens) in &self.source_tokens {
            let Some(offset) = tokens.argv.first().map(|span| span.start()) else {
                return false;
            };
            let Some(original) = originals.get(&offset) else {
                return false;
            };
            let mut restored = tokens.clone();
            restored.restore_source_proofs(original);
            replacements.insert(point, restored);
        }
        // Restoration changes the original carrier; it does not validate a
        // new source inventory under an absent current registry/input query.
        self.unknown_argument_points
            .extend(replacements.iter().filter_map(|(&point, tokens)| {
                crate::place_bridge::evaluates_arguments(tokens).then_some(point)
            }));
        self.source_tokens = replacements;
        self.logical_scalar_stores.clear();
        self.reads = self
            .source_tokens
            .iter()
            .filter(|(_, tokens)| tokens.evaluates_words())
            .map(|(&point, tokens)| (point, tokens.variable_accesses.clone()))
            .collect();
        true
    }

    /// Relocate physical binding and contents worlds without changing lexical source coordinates.
    /// Source carriers are relocated by their owner before this projection is consumed.
    pub fn relocate_variable_proofs(
        &mut self,
        relocation: &crate::var_resolve::VariableProofRelocation,
    ) {
        self.entry = self.entry.relocated(relocation);
        for context in self
            .statements
            .values_mut()
            .chain(self.terminators.values_mut())
            .chain(self.after.values_mut())
        {
            *context = context.relocated(relocation);
        }
        for accesses in self.reads.values_mut() {
            for access in accesses {
                *access = access.relocated_variables(relocation);
            }
        }
        for tokens in self.source_tokens.values_mut() {
            tokens.relocate_variable_proofs(relocation);
        }
    }

    /// The entry context is also the conservative answer for unreachable sites.
    #[must_use]
    pub fn before_statement(&self, block: BlockId, index: usize) -> &ResolveContext {
        self.statements.get(&(block, index)).unwrap_or(&self.entry)
    }

    /// Context on the successful continuation of a statement.
    #[must_use]
    pub fn after_statement(&self, block: BlockId, index: usize) -> &ResolveContext {
        self.after.get(&(block, index)).unwrap_or(&self.entry)
    }

    /// Retained contexts at each lexical substitution, before its own observer.
    #[must_use]
    pub fn source_reads_at(
        &self,
        block: BlockId,
        index: usize,
    ) -> &[crate::command_binding::SourceVariableAccess] {
        self.reads.get(&(block, index)).map_or(&[], Vec::as_slice)
    }

    /// Evaluated words or cached iteration value inputs retained for an exact operation.
    /// A cached input does not add a reached read or reevaluate its original words.
    /// Transforms that replace an operation must invalidate its retained binding site.
    #[must_use]
    pub fn source_tokens_at(
        &self,
        block: BlockId,
        index: usize,
    ) -> Option<&crate::ir::CommandTokens> {
        self.source_tokens.get(&(block, index))
    }

    /// Exact retained boundary context; a missing point never selects entry facts.
    /// `usize::MAX` selects the terminator before condition evaluation.
    #[must_use]
    pub fn context_before(&self, block: BlockId, index: usize) -> Option<&ResolveContext> {
        if index == usize::MAX {
            self.terminators.get(&block)
        } else {
            self.statements.get(&(block, index))
        }
    }

    /// Context after the block's statements, before condition evaluation.
    #[must_use]
    pub fn before_terminator(&self, block: BlockId) -> &ResolveContext {
        self.terminators.get(&block).unwrap_or(&self.entry)
    }
}

/// Solve point-specific cell bindings without assuming block creation order is
/// execution order. Exception edges retain partial mutations conservatively.
#[must_use]
pub fn build_point_resolve_contexts(
    cfg: &Function,
    fn_qname: &str,
    registry: &CommandRegistry,
) -> PointResolveContexts {
    build_point_resolve_contexts_with_entry(cfg, ResolveContext::for_function(fn_qname), registry)
}

/// Solve bindings with an explicitly selected activation environment. Event,
/// namespace and method adapters supply their actual frame kind here.
#[must_use]
pub fn build_point_resolve_contexts_with_entry(
    cfg: &Function,
    entry: ResolveContext,
    registry: &CommandRegistry,
) -> PointResolveContexts {
    let mut result = PointResolveContexts {
        entry: entry.clone(),
        statements: BTreeMap::new(),
        terminators: BTreeMap::new(),
        after: BTreeMap::new(),
        reads: BTreeMap::new(),
        source_tokens: BTreeMap::new(),
        metadata_context: cfg.metadata_context.clone(),
        unknown_argument_points: BTreeSet::new(),
        logical_scalar_stores: BTreeMap::new(),
    };
    retain_source_tokens(&mut result, cfg, registry);
    let mut incoming = HashMap::from([(cfg.entry, entry)]);
    let mut queue = VecDeque::from([cfg.entry]);
    while let Some(id) = queue.pop_front() {
        let Some(block) = cfg.blocks.get(&id) else {
            continue;
        };
        let mut state = incoming[&id].clone();
        let mut abrupt = state.clone();
        for (index, statement) in block.statements.iter().enumerate() {
            refresh_source_context(&mut state, statement, cfg);
            result.statements.insert((id, index), state.clone());
            let logical_store = cfg
                .source_input_tokens_at(id, index)
                .and_then(|tokens| tokens.source_binding.as_ref().zip(Some(tokens)))
                .and_then(|(binding, tokens)| {
                    let metadata = result
                        .metadata_context
                        .metadata_context(registry)
                        .and_then(
                            crate::registry_invocation::InvocationMetadataSelection::context,
                        )?;
                    binding.conditional_logical_scalar_store(tokens, &state, metadata, registry)
                });
            if let Some(store) = logical_store {
                state = store.after().clone();
                result.logical_scalar_stores.insert((id, index), store);
            } else {
                result.logical_scalar_stores.remove(&(id, index));
                transfer_statement(&mut state, statement, registry);
            }
            result.after.insert((id, index), state.clone());
            abrupt.join(&state);
        }
        result.terminators.insert(id, state.clone());
        for successor in block.successors() {
            propagate(&mut incoming, &mut queue, successor, &state);
        }
        for &(source, target) in &cfg.analysis_edges {
            if source == id {
                propagate(&mut incoming, &mut queue, target, &state);
            }
        }
        for &(source, handler) in &cfg.exception_edges {
            if source == id {
                propagate(&mut incoming, &mut queue, handler, &abrupt);
            }
        }
    }
    result
}

fn retain_source_tokens(
    result: &mut PointResolveContexts,
    cfg: &Function,
    registry: &CommandRegistry,
) {
    for (&block_id, block) in &cfg.blocks {
        for index in (0..block.statements.len()).chain(std::iter::once(usize::MAX)) {
            if let Some(tokens) = cfg.source_input_tokens_at(block_id, index) {
                if crate::place_bridge::argument_calls(
                    tokens,
                    registry,
                    crate::place_bridge::ArgumentReadInput::for_owner(
                        &result.metadata_context,
                        registry,
                    ),
                )
                .is_none()
                {
                    result.unknown_argument_points.insert((block_id, index));
                }
                if tokens.evaluates_words() {
                    result
                        .reads
                        .insert((block_id, index), tokens.variable_accesses.clone());
                }
                result
                    .source_tokens
                    .insert((block_id, index), tokens.clone());
            } else if !result.metadata_context.is_standalone() {
                let missing_arguments = if index == usize::MAX {
                    matches!(
                        block.terminator,
                        Some(
                            crate::cfg::Terminator::Branch { .. }
                                | crate::cfg::Terminator::Return { value: Some(_), .. }
                                | crate::cfg::Terminator::Return { expr: Some(_), .. }
                        )
                    )
                } else {
                    let statement = &block.statements[index];
                    statement
                        .tokens()
                        .is_none_or(crate::place_bridge::evaluates_arguments)
                };
                if missing_arguments {
                    result.unknown_argument_points.insert((block_id, index));
                }
            }
        }
    }
}

fn refresh_source_context(state: &mut ResolveContext, statement: &Statement, cfg: &Function) {
    if statement.tokens().is_some_and(|tokens| {
        matches!(
            tokens.synthetic,
            Some(
                crate::ir::SyntheticMarker::EvaluatedWrapper
                    | crate::ir::SyntheticMarker::CapturedCatchOutputs
                    | crate::ir::SyntheticMarker::DictionaryScopeWriteback(_)
            )
        )
    }) {
        return;
    }
    let site = cfg
        .command_binding_sites
        .iter()
        .find(|site| site.span == statement.span());
    if let Some(context) = site.and_then(|site| site.variable_context.as_ref()) {
        replace_source_context(state, context);
        return;
    }
    if let Some(binding) = statement
        .tokens()
        .and_then(|tokens| tokens.source_binding.as_ref())
    {
        replace_source_context(state, &binding.variable_context);
        return;
    }
    if let Some(site) = site {
        if let Some(frame) = &site.variable_frame {
            *state = state.in_frame(frame);
        }
        if let Some(cells) = &site.existing_namespace_cells {
            state.namespace_cells.present = cells.iter().cloned().collect();
        }
        if let Some(namespaces) = &site.known_namespaces {
            state.known_namespaces = namespaces.iter().cloned().collect();
            state.known_namespaces.insert("::".to_owned());
        }
    }
}

fn replace_source_context(state: &mut ResolveContext, source: &ResolveContext) {
    let mut context = source.clone();
    if context.execution_name_policy.is_none() {
        context
            .instance_vars
            .extend(state.instance_vars.iter().cloned());
        if context.instance_owner.is_empty() {
            context.instance_owner.clone_from(&state.instance_owner);
        }
        if context.execution.is_none() {
            context.execution = state.execution;
        }
        if context.interpreter.is_none() {
            context.interpreter.clone_from(&state.interpreter);
        }
    }
    *state = context;
}

fn propagate(
    incoming: &mut HashMap<BlockId, ResolveContext>,
    queue: &mut VecDeque<BlockId>,
    successor: BlockId,
    state: &ResolveContext,
) {
    if let Some(previous) = incoming.get_mut(&successor) {
        let before = previous.clone();
        previous.join(state);
        if before != *previous {
            queue.push_back(successor);
        }
    } else {
        incoming.insert(successor, state.clone());
        queue.push_back(successor);
    }
}

/// Apply the successful continuation of one statement. Cell operations with
/// active traces lose binding precision because callbacks can re-enter Tcl.
pub fn transfer_statement(
    state: &mut ResolveContext,
    statement: &Statement,
    registry: &CommandRegistry,
) {
    if statement.has_opaque_native_accesses() {
        state.widen();
        return;
    }
    if crate::dictionary_bindings::transfer_scope_marker(state, statement, registry) {
        return;
    }
    if crate::place_bridge::read_places(statement, state, registry)
        .iter()
        .any(|place| place.observed)
    {
        state.widen();
    }
    match statement {
        Statement::AssignConst {
            name, name_braced, ..
        }
        | Statement::AssignValue {
            name, name_braced, ..
        }
        | Statement::AssignExpr {
            name, name_braced, ..
        }
        | Statement::Incr {
            name, name_braced, ..
        } => {
            if state.execution_name_policy.is_some() {
                transfer_invocation(state, statement, statement.tokens(), registry);
                return;
            }
            let target = crate::var_resolve::resolve_target_access(
                name,
                *name_braced,
                state,
                registry,
                tcl_registry::TraceOperation::Write,
            );
            state.record_contents_write(&target, statement.span().start(), false);
            if target.observed {
                state.widen();
            } else {
                establish_written_lifetime(state, &target, statement.span().start());
                if target.is_global() && !target.dynamic {
                    state.namespace_cells.present.insert(cell_key(&target));
                }
            }
        }
        Statement::Block { body, .. } => {
            for inner in &body.statements {
                transfer_statement(state, inner, registry);
            }
        }
        Statement::Call {
            tokens: Some(tokens),
            ..
        } if matches!(
            tokens.synthetic,
            Some(crate::ir::SyntheticMarker::IterationBindings(_))
        ) =>
        {
            let targets = crate::place_bridge::def_places(statement, state, registry);
            invalidate_literal_writes(state, &targets);
            for target in targets {
                state.record_contents_write(&target, statement.span().start(), false);
                if target.observed {
                    state.widen();
                } else {
                    establish_written_lifetime(state, &target, statement.span().start());
                    if target.is_global() && !target.dynamic {
                        state.namespace_cells.present.insert(cell_key(&target));
                    }
                }
            }
        }
        Statement::Call { tokens, .. } | Statement::Barrier { tokens, .. } => {
            transfer_invocation(state, statement, tokens.as_ref(), registry);
        }
        Statement::UpFrame { .. } => state.widen(),
        _ => {}
    }
}

fn transfer_invocation(
    state: &mut ResolveContext,
    statement: &Statement,
    tokens: Option<&crate::ir::CommandTokens>,
    registry: &CommandRegistry,
) {
    let Some(tokens) = tokens else {
        state.widen();
        return;
    };
    if tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments) {
        return;
    }
    let context = registry.profile().map(SemanticContext::for_profile);
    let Some(invocation) =
        crate::registry_invocation::normal_transfer_invocation(registry, context, tokens)
    else {
        state.widen();
        return;
    };
    // The selected invocation supplies the original source instance. A
    // represented offset or a reporting spelling cannot attest this store.
    state.set_contents_write_source(
        tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.invocation_site())
            .map(|site| std::sync::Arc::clone(&site.source)),
    );
    invocation.transfer_variables(state, statement, tokens, registry);
}

pub(crate) fn transfer_captured_variable_outputs(
    state: &mut ResolveContext,
    outputs: &[Place],
    source: u32,
) {
    invalidate_literal_writes(state, outputs);
    for target in outputs {
        state.record_contents_write(target, source, false);
        if target.kind != PlaceKind::Unknown && !target.observed {
            establish_written_lifetime(state, target, source);
            if target.is_global() && !target.dynamic {
                state.namespace_cells.present.insert(cell_key(target));
            }
        }
    }
    if outputs
        .iter()
        .any(|target| target.observed || target.kind == PlaceKind::Unknown)
    {
        state.widen();
    }
}

/// Address/effect knowledge after a reached iterator assignment phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceIterationBindingOutcome {
    /// Every selected variable received this iteration's unknown value.
    Entered,
    /// A proved variable-kind conflict stops before entering the body.
    StoreError,
    /// An unknown name, grammar or observer retains a residual obligation.
    Unknown,
}

/// Apply exact frozen iterator values to their sequentially captured outputs.
/// This retains text only; numeric and object representation remain unknown.
pub(crate) fn transfer_finite_iteration_bindings(
    state: &mut ResolveContext,
    values: &[(&[u8], &[u8])],
    registry: &CommandRegistry,
    source_offset: u32,
) -> SourceIterationBindingOutcome {
    use SourceIterationBindingOutcome as Outcome;
    for &(name, value) in values {
        let (Ok(name), Ok(value)) = (std::str::from_utf8(name), std::str::from_utf8(value)) else {
            state.widen();
            return Outcome::Unknown;
        };
        let target = resolve_literal_access(
            name,
            state,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        if state.store_would_error(&target) {
            return Outcome::StoreError;
        }
        if target.observed || target.dynamic || target.kind == PlaceKind::Unknown {
            return Outcome::Unknown;
        }
        transfer_captured_variable_outputs(state, &[target], source_offset);
        state.define_literal(name, value, registry);
    }
    Outcome::Entered
}

/// Apply authored variable-list operands when an iteration actually enters.
/// The caller selects indices from the resolved loop plan, not command text.
/// Arguments have already been frozen. Sequential address lookup preserves
/// earlier writes when a later output fails or an observer changes bindings.
pub fn transfer_source_iteration_bindings(
    state: &mut ResolveContext,
    variable_lists: &[usize],
    arguments: tcl_registry::InvocationArguments<'_>,
    registry: &CommandRegistry,
    source_offset: u32,
) -> SourceIterationBindingOutcome {
    use SourceIterationBindingOutcome as Outcome;
    if variable_lists.is_empty() {
        return Outcome::Entered;
    }
    let Some(dialect) = arguments.dialect() else {
        state.widen();
        return Outcome::Unknown;
    };
    let rules = tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
    let mut names = Vec::new();
    for &index in variable_lists {
        let Some(value) = arguments.literal_at(index) else {
            state.widen();
            return Outcome::Unknown;
        };
        let Ok(decoded) = rules.split_list(value) else {
            state.widen();
            return Outcome::Unknown;
        };
        if decoded.is_empty() {
            state.widen();
            return Outcome::Unknown;
        }
        names.extend(decoded.into_iter().map(std::borrow::Cow::into_owned));
    }
    let mut outcome = Outcome::Entered;
    for name in names {
        let target = resolve_literal_access(
            &name,
            state,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        if state.store_would_error(&target) {
            return Outcome::StoreError;
        }
        if target.observed || target.dynamic || target.kind == PlaceKind::Unknown {
            outcome = Outcome::Unknown;
        }
        transfer_captured_variable_outputs(state, &[target], source_offset);
    }
    outcome
}

pub(crate) fn transfer_resolved_invocation(
    state: &mut ResolveContext,
    statement: &Statement,
    tokens: &crate::ir::CommandTokens,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    registry: &CommandRegistry,
    output: (Option<&[usize]>, Option<&OriginalVariableInvocation>),
) {
    let (output_order, original) = output;
    let operands = if state.execution_name_policy.is_some() {
        let Some(operands) =
            original.filter(|operands| operands.argument_count() == arguments.len())
        else {
            state.widen();
            return;
        };
        Some(operands)
    } else {
        None
    };
    let mut writes = source_variable_write_places_with_output_order_and_original_operands_impl(
        facts,
        arguments,
        state,
        registry,
        output_order,
        operands,
    );
    detach_retained_destructions(
        state,
        facts,
        arguments,
        &mut writes,
        statement.span().start(),
        registry,
        operands,
    );
    invalidate_literal_writes(state, &writes);
    record_resolved_contents_writes(
        state,
        facts,
        arguments,
        registry,
        &writes,
        statement.span().start(),
        operands,
    );
    let operation = invocation_write_trace_operation(facts);
    let trace_callback = crate::place_bridge::def_places(statement, state, registry)
        .into_iter()
        .any(|place| project_access(place, state, operation).observed)
        || source_variable_read_places_with_original_operands_impl(
            facts, arguments, state, registry, operands,
        )
        .iter()
        .any(|place| place.observed);
    transfer_resolved_state_transitions(
        state,
        facts,
        registry,
        tokens.evaluated_body().is_some(),
        statement.span().start(),
        operands,
    );
    if facts.traits.contains(Traits::DESTROYS_VARIABLE) {
        if operands.is_some() {
            for place in &writes {
                destroy_captured_cell(state, place, statement.span().start(), registry);
            }
        } else {
            destroy_invocation_roots(state, facts, arguments, statement.span().start(), registry);
        }
    }
    if let Some(operands) = operands {
        establish_resolved_written_lifetimes(
            state,
            facts,
            arguments,
            registry,
            output_order,
            operands,
            statement.span().start(),
        );
    }
    let precise = precise_cell_effects(facts);
    apply_effects(
        state,
        &facts.effects,
        tokens.evaluated_body().is_some(),
        precise,
        trace_callback,
    );
}

fn record_resolved_contents_writes(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    registry: &CommandRegistry,
    writes: &[Place],
    source_offset: u32,
    operands: Option<&OriginalVariableInvocation>,
) {
    for place in writes {
        if facts.traits.contains(Traits::DESTROYS_VARIABLE)
            && place
                .index
                .as_ref()
                .is_some_and(|index| index.kind != place::IndexKind::Literal)
        {
            state.record_unknown_element_destruction(place);
            continue;
        }
        if !facts.traits.contains(Traits::DESTROYS_VARIABLE) || place.kind == PlaceKind::Unknown {
            state.record_contents_write(
                place,
                source_offset,
                conditional_contents_write(facts, place, state)
                    && !normal_output_is_written_impl(
                        facts, arguments, place, state, registry, operands,
                    ),
            );
        }
    }
}

fn transfer_resolved_state_transitions(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    registry: &CommandRegistry,
    script_interpreted: bool,
    source_offset: u32,
    operands: Option<&OriginalVariableInvocation>,
) {
    match &facts.state_transitions {
        StateTransitionKnowledge::UnknownInvocation => {}
        StateTransitionKnowledge::Declared(transitions) => {
            for fact in transitions.facts() {
                if script_interpreted && matches!(fact.transition, StateTransition::Widen(_)) {
                    continue;
                }
                match (&fact.transition, operands) {
                    (StateTransition::VariableCellAlias(alias), Some(operands)) => {
                        let local = alias
                            .local
                            .argument_index()
                            .and_then(|index| operands.compiled_local(index));
                        bind_alias_with_compiled_local(
                            state,
                            alias,
                            registry,
                            local,
                            Some(operands),
                        );
                    }
                    (StateTransition::Trace(trace), Some(operands)) => {
                        apply_trace_with_original(state, trace, registry, Some(operands));
                    }
                    _ => apply_transition(state, &fact.transition, registry, source_offset),
                }
            }
        }
    }
}

fn establish_resolved_written_lifetimes(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    registry: &CommandRegistry,
    output_order: Option<&[usize]>,
    operands: &OriginalVariableInvocation,
    source_offset: u32,
) {
    let definitions = source_variable_definitions_with_original_operands(
        facts,
        arguments,
        state,
        registry,
        output_order,
        operands,
    );
    for (_, target) in definitions {
        if conditional_contents_write(facts, &target, state)
            && !normal_output_is_written_impl(
                facts,
                arguments,
                &target,
                state,
                registry,
                Some(operands),
            )
        {
            continue;
        }
        if target.observed || target.kind == PlaceKind::Unknown {
            state.widen();
        } else {
            establish_written_lifetime(state, &target, source_offset);
            if target.is_global() && !target.dynamic {
                state.namespace_cells.present.insert(cell_key(&target));
            }
        }
    }
}

fn destroy_invocation_roots(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    source: u32,
    registry: &CommandRegistry,
) {
    let mut touched = false;
    for &(index, role) in &facts.arg_roles {
        if role != tcl_registry::ArgRole::VarWrite {
            continue;
        }
        touched = true;
        if let Some(name) = arguments.literal_at(facts.argument_offset + usize::from(index)) {
            destroy_root(state, name, source, registry);
        } else if arguments
            .array_element_root_at(facts.argument_offset + usize::from(index))
            .is_some()
        {
            let target = variable_operand_access(
                arguments,
                facts.argument_offset + usize::from(index),
                state,
                registry,
                tcl_registry::TraceOperation::Unset,
                false,
            );
            if target.observed {
                state.widen();
            } else {
                state.record_unknown_element_destruction(&target);
            }
        } else {
            state.widen();
        }
    }
    if !touched {
        state.widen();
    }
}

fn precise_cell_effects(facts: &tcl_registry::InvocationFacts) -> bool {
    (facts.arg_roles_complete
        && facts
            .arg_roles
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::VarWrite))
        || facts
            .state_transitions
            .declared()
            .is_some_and(|transitions| {
                transitions.facts().iter().any(|fact| {
                    matches!(
                        fact.transition,
                        StateTransition::VariableCellAlias(_)
                            | StateTransition::Trace(_)
                            | StateTransition::Namespace(NamespaceTransition::Delete { .. })
                    )
                })
            })
}

fn apply_effects(
    state: &mut ResolveContext,
    effects: &tcl_registry::EffectFootprint,
    body_expanded: bool,
    precise: bool,
    trace_callback: bool,
) {
    use tcl_registry::world_effect::{CallbackKinds, EffectAccessMode, WorldStateDomain};
    let callback = effects.callback();
    let trace_only = callback.kinds == CallbackKinds::TRACE;
    if effects.requires_world_barrier() && !body_expanded && (!trace_only || trace_callback) {
        state.widen();
        return;
    }
    for access in effects.accesses() {
        if access.mode == EffectAccessMode::Read {
            continue;
        }
        match access.domain {
            WorldStateDomain::VariableTraces if !precise => {
                state.mark_unenumerated_variable_observers();
            }
            WorldStateDomain::VariableStore if !precise => {
                for generation in state.generations.values_mut() {
                    *generation = CellGeneration::Unknown;
                }
            }
            _ => {}
        }
    }
}

fn apply_transition(
    state: &mut ResolveContext,
    transition: &StateTransition,
    registry: &CommandRegistry,
    source_offset: u32,
) {
    match transition {
        StateTransition::VariableCellAlias(alias) => bind_alias(state, alias, registry),
        StateTransition::Trace(trace) => apply_trace(state, trace, registry),
        StateTransition::Namespace(namespace) => apply_namespace(state, namespace, source_offset),
        StateTransition::Widen(widening)
            if widening.domains.iter().any(|domain| {
                matches!(
                    domain,
                    StateTransitionDomain::VariableCells
                        | StateTransitionDomain::Namespaces
                        | StateTransitionDomain::VariableTraces
                )
            }) =>
        {
            state.widen();
        }
        _ => {}
    }
}

/// Result of validating and installing a reference formal in an actual call activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerReferenceBinding {
    /// Caller contents are defined and the selected link was installed.
    Bound,
    /// The unobserved actual target definitely has no contents.
    Missing,
    /// The name, caller, contents or observer is not proved.
    Unknown,
}

/// Bind a caller-reference formal through the shared selected-frame and alias owner.
/// Missing literal knowledge is never treated as missing caller contents.
pub fn bind_caller_reference(
    state: &mut ResolveContext,
    local: &str,
    actual: Option<&str>,
    registry: &CommandRegistry,
) -> CallerReferenceBinding {
    let (Some(actual), Some(parent)) = (actual, state.caller.as_deref()) else {
        return CallerReferenceBinding::Unknown;
    };
    let target = resolve_literal_access(
        actual,
        parent,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    match parent.contents_presence(&target) {
        crate::var_resolve::ContentsPresence::Undefined => return CallerReferenceBinding::Missing,
        crate::var_resolve::ContentsPresence::Unknown
        | crate::var_resolve::ContentsPresence::DefinedOrUndefined => {
            return CallerReferenceBinding::Unknown;
        }
        crate::var_resolve::ContentsPresence::Defined => {}
    }
    let local = tcl_registry::TransitionSubject::Literal(local.to_owned());
    let target = VariableAliasTarget::CallerSelectedFrame {
        frame: CallerFrameSelection::Explicit(tcl_registry::TransitionSubject::Literal(
            "1".to_owned(),
        )),
        variable: tcl_registry::TransitionSubject::Literal(actual.to_owned()),
    };
    bind_authored_alias_values(
        state,
        AliasBindingValues {
            destination: tcl_registry::VariableAliasDestination::ProcedureLocal,
            local: &local,
            target: &target,
        },
        registry,
    );
    CallerReferenceBinding::Bound
}

/// Values used to bind a cell link. A formal-reference link has no written
/// invocation, so this view deliberately carries no source-word ordinals.
#[derive(Clone, Copy)]
struct AliasBindingValues<'a> {
    destination: tcl_registry::VariableAliasDestination,
    local: &'a tcl_registry::TransitionSubject,
    target: &'a VariableAliasTarget,
}

impl<'a> From<&'a VariableCellAliasTransition> for AliasBindingValues<'a> {
    fn from(alias: &'a VariableCellAliasTransition) -> Self {
        Self {
            destination: alias.destination,
            local: &alias.local,
            target: &alias.target,
        }
    }
}

/// Whether a proved alias operation can install its destination on normal completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasBindingValidity {
    /// The destination and target owners permit the link.
    Valid,
    /// The destination already has direct contents and cannot become a link.
    DefinedDestination,
    /// C Tcl forbids a namespace slot retaining a procedure-local variable.
    NamespaceToLocal,
    /// An unresolved destination, target, or interpreter policy prevents proof.
    Unknown,
}

/// Whether an alias destination still denotes its own direct slot.
/// This does not infer contents, trace absence, or successful alias registration.
pub(crate) fn alias_destination_is_direct(
    state: &ResolveContext,
    local: &str,
    slot: &Place,
) -> bool {
    let raw = state
        .raw_bindings
        .bindings
        .get(&cell_key(slot))
        .and_then(|key| state.raw_bindings.slots.get(key));
    raw.map_or_else(
        || {
            !state.alias_bindings.contains_key(local)
                && !state.name_alias_bindings.contains_key(local)
                && !state.namespace_alias_bindings.contains_key(&cell_key(slot))
                && !state
                    .namespace_name_alias_bindings
                    .contains_key(&cell_key(slot))
        },
        |slot| {
            matches!(&slot.contents,
                crate::raw_binding::RawBindingContents::Direct(target)
                    if !matches!(target.cell.as_ref().map(|cell| &cell.owner),
                        Some(CellOwner::AllocatedInstance(_))))
        },
    )
}

/// A native alias-pair registration is normal-only when its actual frame,
/// every destination and target, and the complete literal layout are proved.
/// This does not license an opcode or infer success from a catalogue role.
pub(crate) fn alias_registration_is_closed(
    state: &ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    registry: &CommandRegistry,
) -> bool {
    use tcl_registry::frame_effect::{FrameArgLayout, FrameArgumentResolution};
    let Some(effect) = facts
        .frame_effect
        .filter(|effect| effect.layout == FrameArgLayout::AliasPairs)
    else {
        return false;
    };
    let FrameArgumentResolution::Valid {
        level_word_len,
        level,
    } = effect.resolve_arguments(arguments)
    else {
        return false;
    };
    let Some(count) = arguments.exact_argv_len() else {
        return false;
    };
    if state.binding_identity != crate::var_resolve::BindingIdentity::Bound
        || !arguments.are_all_literals()
        || state.dynamic_bindings
        || state.selected_frame_context(level).is_none()
        || facts
            .successful_handler_effects(arguments, state.alias_frame())
            .is_none()
    {
        return false;
    }
    let Some(transitions) = facts.state_transitions.declared() else {
        return false;
    };
    let aliases = transitions
        .facts()
        .iter()
        .filter_map(|fact| {
            if let StateTransition::VariableCellAlias(alias) = &fact.transition {
                Some(alias)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if aliases.len() != (count - level_word_len) / 2 {
        return false;
    }
    let mut candidate = state.clone();
    for alias in aliases {
        if !alias_registration_address_is_closed(&candidate, alias, registry) {
            return false;
        }
        bind_alias(&mut candidate, alias, registry);
    }
    true
}

/// A selected variable-trace registration never invokes its stored prefix.
/// Valid removal is a no-op for missing or incompatible variable names; adding
/// still requires a live physical destination with a valid namespace/root kind.
#[cfg(test)]
fn variable_trace_registration_is_closed(
    state: &ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    registry: &CommandRegistry,
) -> bool {
    variable_trace_registration_is_closed_impl(state, facts, registry, None)
}

pub(crate) fn variable_trace_registration_is_closed_with_original_operands(
    state: &ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    registry: &CommandRegistry,
    operands: &OriginalVariableInvocation,
) -> bool {
    variable_trace_registration_is_closed_impl(state, facts, registry, Some(operands))
}

fn variable_trace_registration_is_closed_impl(
    state: &ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) -> bool {
    if state.invocation_dialect.is_none_or(|dialect| {
        dialect.tcl_version.is_none()
            || dialect.variable_lookup_policy != Some(tcl_dialect::VariableLookupPolicy::Tcl)
    }) {
        return false;
    }
    let Some(transitions) = facts.state_transitions.declared() else {
        return false;
    };
    let [fact] = transitions.facts() else {
        return false;
    };
    let StateTransition::Trace(trace) = &fact.transition else {
        return false;
    };
    let (target, operations, added) = match trace {
        TraceTransition::Add {
            target, operations, ..
        } => (target, operations, true),
        TraceTransition::Remove {
            target, operations, ..
        } => (target, operations, false),
    };
    let TraceTarget::Variable(variable) = target else {
        return false;
    };
    let tcl_registry::TraceOperationSet::Known(operations) = operations else {
        return false;
    };
    if operations.is_empty() {
        return false;
    }
    if !added {
        return true;
    }
    let mut target = operands.map_or_else(
        || {
            variable.literal().map_or_else(place::unknown_top, |name| {
                resolve_literal_place(name, state, false, registry)
            })
        },
        |operands| {
            variable
                .argument_index()
                .map_or_else(place::unknown_top, |index| {
                    operands.trace_subject_access(index, state, registry)
                })
        },
    );
    target.observed = false;
    // Registering a trace on an array root does not perform a scalar store.
    // An indexed target still requires an actual array receiver.
    if target.dynamic
        || target.kind == PlaceKind::Unknown
        || (target.index.is_some() && state.store_would_error(&target))
    {
        return false;
    }
    if target.index.is_some()
        && state.root_contents_kind(&target).is_none()
        && state.contents_presence(&target.base())
            != crate::var_resolve::ContentsPresence::Undefined
    {
        return false;
    }
    target.cell.as_ref().is_some_and(|cell| {
        cell.generation != CellGeneration::Unknown
            && !matches!(cell.owner, CellOwner::SelectedFrame(_))
            && (!target.is_global()
                || operands.map_or_else(
                    || state.known_namespaces.contains(&target.ns),
                    |_| {
                        state.namespace_footprint(&target).is_some_and(|namespace| {
                            state.namespace_addressable_identities.contains(&namespace)
                        })
                    },
                ))
    })
}

fn alias_registration_address_is_closed(
    state: &ResolveContext,
    alias: &VariableCellAliasTransition,
    registry: &CommandRegistry,
) -> bool {
    let Some(local) = alias.local.literal() else {
        return false;
    };
    if alias
        .destination
        .is_active_in_frame_with_policy(state.alias_frame(), state.invocation_dialect)
        != Some(true)
        || alias_binding_validity(state, alias, registry) != AliasBindingValidity::Valid
    {
        return false;
    }
    let mut slot = crate::var_resolve::resolve_alias_destination_slot(local, state, registry);
    let mut target = alias_target(state, &alias.target, registry);
    slot.observed = false;
    target.observed = false;
    let Some(cell) = target.cell.as_ref() else {
        return false;
    };
    if target.dynamic
        || target.kind == PlaceKind::Unknown
        || cell.generation == CellGeneration::Unknown
        || matches!(cell.owner, CellOwner::SelectedFrame(_))
        || (target.is_global() && !state.known_namespaces.contains(&target.ns))
        || (target.index.is_some() && state.store_would_error(&target))
        || (slot.cell == target.cell)
    {
        return false;
    }
    let established = state.alias_bindings.contains_key(local)
        || state
            .namespace_alias_bindings
            .contains_key(&cell_key(&slot));
    if !established
        && state.contents_presence(&slot) != crate::var_resolve::ContentsPresence::Undefined
    {
        return false;
    }
    if target.index.is_some() && state.root_contents_kind(&target).is_none() {
        let root = target.base();
        if state.contents_presence(&root) != crate::var_resolve::ContentsPresence::Undefined {
            return false;
        }
    }
    !slot.is_global() || state.known_namespaces.contains(&slot.ns)
}

/// Validate owner compatibility without manufacturing or mutating either cell.
#[must_use]
pub fn alias_binding_validity(
    state: &ResolveContext,
    alias: &VariableCellAliasTransition,
    registry: &CommandRegistry,
) -> AliasBindingValidity {
    alias_value_binding_validity(state, alias.into(), registry)
}

fn alias_value_binding_validity(
    state: &ResolveContext,
    alias: AliasBindingValues<'_>,
    registry: &CommandRegistry,
) -> AliasBindingValidity {
    let Some(local) = alias.local.literal() else {
        return AliasBindingValidity::Unknown;
    };
    let policy = state.invocation_dialect.or_else(|| {
        registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile)
    });
    if alias
        .destination
        .is_active_in_frame_with_policy(state.alias_frame(), policy)
        == Some(false)
    {
        return AliasBindingValidity::Valid;
    }
    if policy.and_then(|dialect| dialect.variable_lookup_policy)
        == Some(tcl_dialect::VariableLookupPolicy::Tcl)
        && (state.namespace_scope() || state.global_frame())
        && matches!(alias.target, VariableAliasTarget::CurrentNamespace { .. })
    {
        return AliasBindingValidity::Valid;
    }
    let destination = crate::var_resolve::resolve_literal_place(local, state, false, registry);
    let slot = crate::var_resolve::resolve_alias_destination_slot(local, state, registry);
    let direct = alias_destination_is_direct(state, local, &slot);
    if direct
        && !destination.observed
        && state.contents_presence(&destination) == crate::var_resolve::ContentsPresence::Defined
    {
        return AliasBindingValidity::DefinedDestination;
    }
    if state.frame_kind == crate::var_resolve::VariableFrameKind::Local
        && !tcl_syntax::naming::is_qualified(local.as_bytes())
    {
        return AliasBindingValidity::Valid;
    }
    let slot = crate::var_resolve::resolve_alias_destination_slot(local, state, registry);
    let target = alias_target(state, alias.target, registry);
    if slot.kind == PlaceKind::Unknown || target.kind == PlaceKind::Unknown {
        return AliasBindingValidity::Unknown;
    }
    if !slot.is_global() || target.is_global() {
        return AliasBindingValidity::Valid;
    }
    let policy = state
        .invocation_dialect
        .and_then(|dialect| dialect.variable_lookup_policy)
        .or_else(|| {
            registry
                .profile()
                .and_then(tcl_dialect::DialectProfile::variable_lookup_policy)
        });
    match policy {
        Some(tcl_dialect::VariableLookupPolicy::Tcl) => {
            match target.cell.as_ref().map(|cell| &cell.owner) {
                Some(CellOwner::Activation(_) | CellOwner::CurrentFrame) => {
                    AliasBindingValidity::NamespaceToLocal
                }
                _ => AliasBindingValidity::Unknown,
            }
        }
        Some(tcl_dialect::VariableLookupPolicy::Jim) => AliasBindingValidity::Valid,
        None => AliasBindingValidity::Unknown,
    }
}

fn bind_alias(
    state: &mut ResolveContext,
    alias: &VariableCellAliasTransition,
    registry: &CommandRegistry,
) {
    bind_alias_with_compiled_local(state, alias, registry, None, None);
}

fn bind_authored_alias_values(
    state: &mut ResolveContext,
    alias: AliasBindingValues<'_>,
    registry: &CommandRegistry,
) {
    bind_alias_values_with_compiled_local(state, alias, registry, None, None);
}

fn bind_alias_with_compiled_local(
    state: &mut ResolveContext,
    alias: &VariableCellAliasTransition,
    registry: &CommandRegistry,
    compiled_local: Option<
        &crate::command_binding::original_variable_compilation::OriginalCompiledNamespaceLocal,
    >,
    operands: Option<&OriginalVariableInvocation>,
) {
    bind_alias_values_with_compiled_local(state, alias.into(), registry, compiled_local, operands);
}

fn bind_alias_values_with_compiled_local(
    state: &mut ResolveContext,
    alias: AliasBindingValues<'_>,
    registry: &CommandRegistry,
    compiled_local: Option<
        &crate::command_binding::original_variable_compilation::OriginalCompiledNamespaceLocal,
    >,
    operands: Option<&OriginalVariableInvocation>,
) {
    let dialect = state.invocation_dialect.or_else(|| {
        registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile)
    });
    let frame = state.alias_frame();
    let active = alias
        .destination
        .is_active_in_frame_with_policy(frame, dialect);
    if active == Some(false) {
        return;
    }
    let original_slot = if let Some(local) = compiled_local {
        if let Some(slot) = local.binding_slot(state) {
            Some(slot)
        } else {
            #[cfg(test)]
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
                eprintln!(
                    "ORIGINAL_VARIABLE_ALIAS_BIND invalid_compiled_local frame={frame:?} policy={:?} dynamic={}",
                    state.execution_name_policy, state.dynamic_bindings
                );
            }
            state.widen();
            return;
        }
    } else {
        operands.map(|operands| original_alias_destination(state, alias, operands, registry))
    };
    if original_slot
        .as_ref()
        .is_some_and(|slot| slot.kind == PlaceKind::Unknown)
    {
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
            eprintln!(
                "ORIGINAL_VARIABLE_ALIAS_BIND unknown_destination frame={frame:?} local_ordinal={:?} original_input={} dynamic={}",
                alias.local.argument_index(),
                operands
                    .and_then(|operands| operands.subject_input(alias.local, state))
                    .is_some(),
                state.dynamic_bindings
            );
        }
        state.widen();
        return;
    }
    let local = alias.local.literal();
    let key = match (&original_slot, local) {
        (Some(slot), _) => cell_key(slot),
        (None, Some(local)) => crate::var_resolve::VariableCellKey::Authored(local.to_owned()),
        (None, None) => {
            state.widen();
            return;
        }
    };
    if active.is_none() {
        state.unknown_bindings.insert(key.clone());
        state.alias_bindings.remove(&key);
        return;
    }
    let target = operands.map_or_else(
        || alias_target(state, alias.target, registry),
        |operands| original_alias_target(state, alias.target, operands, registry, false),
    );
    #[cfg(test)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
        eprintln!(
            "ORIGINAL_VARIABLE_ALIAS_BIND frame={frame:?} target_kind={:?} target_cell={} local_kind={:?} dynamic={}",
            target.kind,
            target.cell.is_some(),
            original_slot.as_ref().map(|slot| slot.kind),
            state.dynamic_bindings
        );
    }
    let validity = selected_alias_binding_validity(
        state,
        alias,
        registry,
        original_slot.as_ref(),
        &key,
        &target,
    );
    if matches!(
        validity,
        AliasBindingValidity::NamespaceToLocal | AliasBindingValidity::DefinedDestination
    ) {
        state.unknown_bindings.insert(key.clone());
        state.alias_bindings.remove(&key);
        return;
    }
    install_alias_binding(
        state,
        alias,
        registry,
        original_slot.as_ref(),
        key,
        target,
        operands,
    );
}

fn selected_alias_binding_validity(
    state: &ResolveContext,
    alias: AliasBindingValues<'_>,
    registry: &CommandRegistry,
    original_slot: Option<&Place>,
    key: &crate::var_resolve::VariableCellKey,
    target: &Place,
) -> AliasBindingValidity {
    original_slot.map_or_else(
        || alias_value_binding_validity(state, alias, registry),
        |slot| {
            let direct = !state.alias_bindings.contains_key(key)
                && !state.name_alias_bindings.contains_key(key)
                && !state.raw_bindings.bindings.contains_key(key);
            if direct
                && state.contents_presence(slot) == crate::var_resolve::ContentsPresence::Defined
            {
                AliasBindingValidity::DefinedDestination
            } else if slot.is_global()
                && !target.is_global()
                && state
                    .invocation_dialect
                    .and_then(|dialect| dialect.variable_lookup_policy)
                    == Some(tcl_dialect::VariableLookupPolicy::Tcl)
            {
                AliasBindingValidity::NamespaceToLocal
            } else {
                AliasBindingValidity::Valid
            }
        },
    )
}

fn install_alias_binding(
    state: &mut ResolveContext,
    alias: AliasBindingValues<'_>,
    registry: &CommandRegistry,
    original_slot: Option<&Place>,
    key: crate::var_resolve::VariableCellKey,
    target: Place,
    operands: Option<&OriginalVariableInvocation>,
) {
    let local = alias.local.literal();
    let name_link = state
        .invocation_dialect
        .and_then(|dialect| dialect.variable_link_binding)
        .or_else(|| {
            registry
                .profile()
                .and_then(tcl_dialect::DialectProfile::variable_link_binding)
        })
        == Some(tcl_dialect::VariableLinkBinding::SelectedFrameName);
    let named_target = name_link.then(|| {
        operands.map_or_else(
            || alias_name_target(state, alias.target, registry),
            |operands| original_alias_target(state, alias.target, operands, registry, true),
        )
    });
    if let Some(target) = &named_target {
        state
            .name_alias_bindings
            .insert(key.clone(), target.clone());
    } else {
        state.name_alias_bindings.remove(&key);
    }
    if let Some(named) = &named_target {
        if let Some(slot) = original_slot {
            state.retarget_raw_binding_at_slot(slot, named);
        } else if let Some(local) = local {
            state.retarget_raw_binding(local, named, registry);
        }
    }
    if target.kind == PlaceKind::Unknown {
        // Importing an unknown name can allocate an undefined cell, but it
        // does not delete known cells or alter their contents and observers.
        state.namespace_cells.closed = false;
        state.namespace_cells.closed_namespaces.clear();
    }
    if target.is_global() && !target.dynamic {
        state.namespace_cells.present.insert(cell_key(&target));
    }
    if state.global_frame()
        || state.namespace_scope()
        || original_slot.is_some_and(Place::is_global)
        || (operands.is_none()
            && local.is_some_and(|local| tcl_syntax::naming::is_qualified(local.as_bytes())))
    {
        let slot = original_slot.cloned().unwrap_or_else(|| {
            crate::var_resolve::resolve_alias_destination_slot(
                local.expect("authored alias local"),
                state,
                registry,
            )
        });
        if slot.is_global() && !slot.dynamic {
            state
                .namespace_alias_bindings
                .insert(cell_key(&slot), target.clone());
            if let Some(target) = &named_target {
                state
                    .namespace_name_alias_bindings
                    .insert(cell_key(&slot), target.clone());
            } else {
                state.namespace_name_alias_bindings.remove(&cell_key(&slot));
            }
            state.namespace_cells.present.insert(cell_key(&slot));
        }
    }
    state.unknown_bindings.remove(&key);
    state.alias_bindings.insert(key, target);
}

fn original_alias_destination(
    state: &ResolveContext,
    alias: AliasBindingValues<'_>,
    operands: &OriginalVariableInvocation,
    registry: &CommandRegistry,
) -> Place {
    let Some(input) = operands.subject_input(alias.local, state) else {
        return place::unknown_top();
    };
    let bytes = match alias.target {
        VariableAliasTarget::Global { .. } => {
            tcl_syntax::naming::global_local_name_bytes(input.policy().recipe(), input.bytes())
        }
        VariableAliasTarget::CurrentNamespace { .. } => Some(
            tcl_syntax::naming::variable_local_name_bytes(input.policy().recipe(), input.bytes()),
        ),
        _ => Some(input.bytes().to_vec()),
    };
    let Some(bytes) = bytes else {
        return place::unknown_top();
    };
    crate::var_resolve::resolve_original_alias_destination_bytes(&bytes, state, registry)
}

fn original_alias_target(
    state: &ResolveContext,
    target: &VariableAliasTarget,
    operands: &OriginalVariableInvocation,
    registry: &CommandRegistry,
    destination: bool,
) -> Place {
    let variable = match target {
        VariableAliasTarget::Global { variable }
        | VariableAliasTarget::CurrentNamespace { variable }
        | VariableAliasTarget::CallerSelectedFrame { variable, .. }
        | VariableAliasTarget::Namespace { variable, .. } => variable,
    };
    let Some(input) = operands.subject_input(variable, state) else {
        return place::unknown_top();
    };
    let mut result = if let VariableAliasTarget::CallerSelectedFrame { frame, .. } = target {
        let level = match frame {
            CallerFrameSelection::DefaultCaller => Some(FrameLevel::DEFAULT),
            CallerFrameSelection::Explicit(subject) => {
                operands.subject_input(subject, state).and_then(|input| {
                    FrameLevel::parse_native_bytes(input.bytes(), state.invocation_dialect?)
                        .flatten()
                })
            }
        };
        let Some(level) = level else {
            return place::unknown_top();
        };
        if level.is_global_frame() {
            let Some(root) = state
                .root_namespace_identity()
                .filter(|root| state.namespace_identities.contains(root))
            else {
                return place::unknown_top();
            };
            // Absolute zero selects the retained interpreter root table.
            // This is alias target geometry, not an invented caller frame.
            crate::var_resolve::resolve_original_namespace_variable_bytes(
                input.bytes(),
                &root,
                state,
                registry,
                destination,
            )
        } else {
            let Some(selected) = state.selected_frame_context(level) else {
                return place::unknown_top();
            };
            if destination {
                crate::var_resolve::resolve_original_alias_destination_bytes(
                    input.bytes(),
                    &selected,
                    registry,
                )
            } else {
                crate::var_resolve::resolve_evaluated_variable_input(
                    tcl_syntax::naming::NativeVariableInputForm::Combined(input.bytes()),
                    &selected,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Read,
                )
            }
        }
    } else {
        let namespace = match target {
            VariableAliasTarget::Global { .. } => state.root_namespace_identity(),
            VariableAliasTarget::CurrentNamespace { .. } => state
                .namespace_identity
                .clone()
                .filter(|_| state.namespace_known),
            VariableAliasTarget::Namespace { namespace, .. } => operands
                .subject_input(namespace, state)
                .and_then(|input| state.namespace_identity_for_original_input(input)),
            VariableAliasTarget::CallerSelectedFrame { .. } => unreachable!(),
        };
        let Some(namespace) = namespace else {
            return place::unknown_top();
        };
        crate::var_resolve::resolve_original_namespace_variable_bytes(
            input.bytes(),
            &namespace,
            state,
            registry,
            destination,
        )
    };
    if destination {
        result.observed = false;
        if let Some(cell) = &mut result.cell {
            cell.generation = CellGeneration::Incoming;
        }
    }
    result
}

fn selected_alias_frame(
    state: &ResolveContext,
    selection: &CallerFrameSelection,
    registry: &CommandRegistry,
) -> Option<FrameLevel> {
    match selection {
        CallerFrameSelection::DefaultCaller => Some(FrameLevel::DEFAULT),
        CallerFrameSelection::Explicit(word) => word.literal().and_then(|word| {
            state.invocation_dialect.map_or_else(
                || FrameLevel::parse_in(word, registry),
                |dialect| FrameLevel::parse_for_dialect(word, dialect),
            )
        }),
    }
}

fn alias_name_target(
    state: &ResolveContext,
    target: &VariableAliasTarget,
    registry: &CommandRegistry,
) -> Place {
    let mut target = match target {
        VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
            let Some(variable) = variable.literal() else {
                return place::unknown_top();
            };
            let Some(level) = selected_alias_frame(state, frame, registry) else {
                return place::unknown_top();
            };
            let Some(selected) = state.selected_frame_context(level) else {
                return place::unknown_top();
            };
            crate::var_resolve::resolve_alias_destination_slot(variable, &selected, registry)
        }
        VariableAliasTarget::Global { variable } => {
            namespace_name_slot(state, "::", variable.literal(), registry)
        }
        VariableAliasTarget::CurrentNamespace { variable } => {
            namespace_name_slot(state, &state.namespace, variable.literal(), registry)
        }
        VariableAliasTarget::Namespace {
            namespace,
            variable,
        } => {
            let Some(namespace) = namespace.literal() else {
                return place::unknown_top();
            };
            if state.namespace_identity.is_some() {
                let Some(identity) = state.namespace_identity_for_written(namespace) else {
                    return place::unknown_top();
                };
                namespace_target_in_identity(state, &identity, variable.literal(), registry, true)
            } else {
                let namespace = tcl_syntax::naming::qualify_namespace(&state.namespace, namespace);
                namespace_name_slot(state, &namespace, variable.literal(), registry)
            }
        }
    };
    target.observed = false;
    if let Some(cell) = &mut target.cell {
        cell.generation = CellGeneration::Incoming;
    }
    target
}

fn namespace_name_slot(
    state: &ResolveContext,
    namespace: &str,
    variable: Option<&str>,
    registry: &CommandRegistry,
) -> Place {
    let Some(variable) = variable else {
        return place::unknown_top();
    };
    if let Some(identity) = if namespace == "::" {
        state.root_namespace_identity()
    } else {
        state
            .namespace_identity
            .clone()
            .filter(|_| namespace == state.namespace)
    } {
        return namespace_target_in_identity(state, &identity, Some(variable), registry, true);
    }
    let qualified = tcl_syntax::naming::qualify(namespace, variable);
    crate::var_resolve::resolve_alias_destination_slot(&qualified, state, registry)
}

pub(crate) fn alias_target(
    state: &ResolveContext,
    target: &VariableAliasTarget,
    registry: &CommandRegistry,
) -> Place {
    match target {
        VariableAliasTarget::Global { variable } => {
            namespace_target(state, "::", variable.literal(), registry)
        }
        VariableAliasTarget::CurrentNamespace { variable } => {
            if !state.namespace_known {
                return place::unknown_top();
            }
            namespace_target(state, &state.namespace, variable.literal(), registry)
        }
        VariableAliasTarget::Namespace {
            namespace,
            variable,
        } => {
            let Some(namespace) = namespace.literal() else {
                return place::unknown_top();
            };
            if !state.namespace_known && !tcl_syntax::naming::is_absolute(namespace) {
                return place::unknown_top();
            }
            if state.namespace_identity.is_some() {
                let Some(identity) = state.namespace_identity_for_written(namespace) else {
                    return place::unknown_top();
                };
                namespace_target_in_identity(state, &identity, variable.literal(), registry, false)
            } else {
                let namespace = tcl_syntax::naming::qualify_namespace(&state.namespace, namespace);
                namespace_target(state, &namespace, variable.literal(), registry)
            }
        }
        VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
            selected_frame_target(state, frame, variable.literal(), registry)
        }
    }
}

fn namespace_target(
    state: &ResolveContext,
    namespace: &str,
    variable: Option<&str>,
    registry: &CommandRegistry,
) -> Place {
    let Some(variable) = variable else {
        return place::unknown_top();
    };
    if let Some(identity) = if namespace == "::" {
        state.root_namespace_identity()
    } else {
        state
            .namespace_identity
            .clone()
            .filter(|_| namespace == state.namespace)
    } {
        return namespace_target_in_identity(state, &identity, Some(variable), registry, false);
    }
    let qualified = tcl_syntax::naming::qualify(namespace, variable);
    resolve_literal_place(&qualified, state, false, registry)
}

fn namespace_target_in_identity(
    state: &ResolveContext,
    identity: &crate::command_binding::SourceNamespaceKey,
    variable: Option<&str>,
    registry: &CommandRegistry,
    destination: bool,
) -> Place {
    let Some(variable) = variable else {
        return place::unknown_top();
    };
    let mut selected = state.clone();
    selected.retain_namespace_world(
        identity.clone(),
        state.namespace_identities.iter().cloned(),
        state.namespace_name_protocol,
    );
    selected.frame_kind = crate::var_resolve::VariableFrameKind::Namespace;
    selected.ns_vars.insert(variable.to_owned());
    if destination {
        crate::var_resolve::resolve_alias_destination_slot(variable, &selected, registry)
    } else {
        resolve_literal_place(variable, &selected, false, registry)
    }
}

fn selected_frame_target(
    state: &ResolveContext,
    selection: &CallerFrameSelection,
    variable: Option<&str>,
    registry: &CommandRegistry,
) -> Place {
    let Some(variable) = variable else {
        return place::unknown_top();
    };
    let Some(level) = selected_alias_frame(state, selection, registry) else {
        return place::unknown_top();
    };
    if level.is_current_frame() {
        return resolve_literal_place(variable, state, false, registry);
    }
    if let Some(selected) = state.selected_frame_context(level) {
        return resolve_literal_place(variable, &selected, false, registry);
    }

    if level.is_global_frame() || tcl_syntax::naming::is_absolute(variable) {
        return namespace_target(state, "::", Some(variable), registry);
    }
    if level == FrameLevel::Dynamic {
        return place::unknown_top();
    }
    let (name, index) = crate::naming::split_array_name(variable);
    let mut target = place::upvar_alias(name, format!("{level:?}:{variable}"), false);
    target.cell = Some(CellIdentity {
        owner: CellOwner::SelectedFrame(level),
        name: name.into(),
        generation: CellGeneration::Incoming,
        interpreter: state.interpreter.clone(),
        storage_domain: None,
        execution: state.execution,
    });
    if let Some(index) = index {
        target.index = Some(place::Index::literal(index));
    }
    target
}

fn apply_trace(state: &mut ResolveContext, trace: &TraceTransition, registry: &CommandRegistry) {
    apply_trace_with_original(state, trace, registry, None);
}

fn apply_trace_with_original(
    state: &mut ResolveContext,
    trace: &TraceTransition,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) {
    let (target, operations, prefix, added) = match trace {
        TraceTransition::Add {
            target,
            operations,
            prefix,
        } => (target, operations, prefix, true),
        TraceTransition::Remove {
            target,
            operations,
            prefix,
        } => (target, operations, prefix, false),
    };
    let TraceTarget::Variable(variable) = target else {
        return;
    };
    let target = operands.map_or_else(
        || {
            variable.literal().map_or_else(place::unknown_top, |name| {
                resolve_literal_place(name, state, false, registry)
            })
        },
        |operands| {
            variable
                .argument_index()
                .map_or_else(place::unknown_top, |index| {
                    operands.trace_subject_access(index, state, registry)
                })
        },
    );
    let prefix = operands.map_or_else(
        || {
            prefix
                .literal()
                .map(|value| crate::var_resolve::VariableTracePrefix::Authored(value.to_owned()))
        },
        |operands| {
            crate::var_resolve::VariableTracePrefix::copied_original(
                operands.subject_input(prefix, state)?,
                state,
            )
        },
    );
    let registration = match (operations, prefix) {
        (tcl_registry::TraceOperationSet::Known(operations), Some(prefix)) => {
            Some((operations.clone(), prefix))
        }
        _ => None,
    };
    if target.kind == PlaceKind::Unknown {
        retain_possible_trace_registration(state, target, registration, added);
        return;
    }
    let key = trace_key(&target);
    if added && target.is_global() && !target.dynamic {
        state.namespace_cells.present.insert(cell_key(&target));
    }
    update_trace_registration(state, &key, registration, added);
    let observed = state.untracked_traces.contains(&key)
        || state
            .trace_registrations
            .get(&key)
            .is_some_and(|registrations| !registrations.is_empty());
    if observed {
        state.traced.insert(key.clone());
        state
            .trace_registration_receivers
            .insert(key.clone(), std::sync::Arc::new(target.clone()));
    } else {
        state.traced.remove(&key);
        state.trace_registration_receivers.remove(&key);
    }
    let aliases = state
        .alias_bindings
        .iter()
        .filter(|(_, alias)| place::overlap(alias, &target))
        .map(|(key, alias)| {
            (
                key.clone(),
                observed || state.unenumerated_observers_may_run(alias),
            )
        })
        .collect::<Vec<_>>();
    for (key, observed) in aliases {
        state
            .alias_bindings
            .get_mut(&key)
            .expect("selected alias")
            .observed = observed;
    }
}

fn retain_possible_trace_registration(
    state: &mut ResolveContext,
    target: Place,
    registration: Option<(
        Vec<tcl_registry::TraceOperation>,
        crate::var_resolve::VariableTracePrefix,
    )>,
    added: bool,
) {
    if added {
        if let Some((operations, prefix)) = registration {
            state.possible_trace_registrations.push(
                crate::var_resolve::PossibleVariableTraceRegistration {
                    target,
                    operations,
                    prefix,
                },
            );
        } else {
            state.mark_unenumerated_variable_observers();
        }
    }
}

fn update_trace_registration(
    state: &mut ResolveContext,
    key: &crate::var_resolve::VariableCellKey,
    registration: Option<(
        Vec<tcl_registry::TraceOperation>,
        crate::var_resolve::VariableTracePrefix,
    )>,
    added: bool,
) {
    if let Some(registration) = registration {
        if state.traced.contains(key) && !state.trace_registrations.contains_key(key) {
            state.untracked_traces.insert(key.clone());
        }
        if added {
            state
                .trace_registrations
                .entry(key.clone())
                .or_default()
                .push(registration);
        } else {
            let selected = state
                .trace_registrations
                .get(key)
                .and_then(|registrations| {
                    registrations.iter().rposition(|item| {
                        item.0 == registration.0 && item.1.removal_matches(&registration.1, state)
                    })
                });
            if let Some(index) = selected {
                state
                    .trace_registrations
                    .get_mut(key)
                    .expect("selected trace inventory")
                    .remove(index);
            }
        }
    } else if added {
        state.untracked_traces.insert(key.clone());
    }
}

/// Cell-store destruction declared by a namespace-deletion transition.
#[must_use]
pub fn namespace_destruction_places(
    statement: &Statement,
    state: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    namespace_destruction_places_with_metadata_context(
        statement,
        state,
        registry,
        Some(
            registry
                .profile()
                .map(SemanticContext::for_profile)
                .map(crate::registry_invocation::InvocationMetadataContext::from)
                .into(),
        ),
    )
}

/// Namespace destruction under the caller's complete retained source input.
#[must_use]
pub(crate) fn namespace_destruction_places_with_metadata_context(
    statement: &Statement,
    state: &ResolveContext,
    registry: &CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataSelection<'_>>,
) -> Vec<Place> {
    let Some(metadata) = metadata else {
        return Vec::new();
    };
    let Some(invocation) =
        crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
            registry,
            metadata.context(),
            statement,
        )
    else {
        return Vec::new();
    };
    namespace_destruction_from_facts(&invocation.facts, state)
}

fn namespace_destruction_from_facts(
    facts: &tcl_registry::InvocationFacts,
    state: &ResolveContext,
) -> Vec<Place> {
    let Some(transitions) = facts.state_transitions.declared() else {
        return Vec::new();
    };
    transitions
        .facts()
        .iter()
        .filter_map(|fact| {
            let StateTransition::Namespace(NamespaceTransition::Delete { namespace }) =
                &fact.transition
            else {
                return None;
            };
            Some(match namespace {
                NamespaceTransitionTarget::Current if state.namespace_known => {
                    place::unknown_namespace(state.namespace.clone())
                }
                NamespaceTransitionTarget::Named(subject) => subject
                    .literal()
                    .filter(|namespace| {
                        state.namespace_known || tcl_syntax::naming::is_absolute(namespace)
                    })
                    .map_or_else(place::unknown_top, |namespace| {
                        place::unknown_namespace(tcl_syntax::naming::qualify(
                            &state.namespace,
                            namespace,
                        ))
                    }),
                NamespaceTransitionTarget::Current => place::unknown_top(),
            })
        })
        .collect()
}

/// Whether a variable-name role is an actual contents write rather than a
/// registry-owned binding or trace-registration operand.
#[must_use]
pub(crate) fn contents_write_operand(
    facts: &tcl_registry::InvocationFacts,
    literal: Option<&str>,
) -> bool {
    let Some(transitions) = facts.state_transitions.declared() else {
        return true;
    };
    let mut binding_only = false;
    for fact in transitions.facts() {
        match &fact.transition {
            StateTransition::Trace(
                TraceTransition::Add {
                    target: TraceTarget::Variable(variable),
                    ..
                }
                | TraceTransition::Remove {
                    target: TraceTarget::Variable(variable),
                    ..
                },
            ) if variable.literal() == literal => binding_only = true,
            StateTransition::VariableCellAlias(alias) if alias.local.literal() == literal => {
                if alias.writes_value {
                    return true;
                }
                binding_only = true;
            }
            _ => {}
        }
    }
    !binding_only
}

fn contents_write_operand_impl(
    facts: &tcl_registry::InvocationFacts,
    literal: Option<&str>,
    index: usize,
    operands: Option<&OriginalVariableInvocation>,
) -> bool {
    if facts.variable_receiver_operand_form(index)
        == Some(tcl_registry::resolved_invocation::VariableReceiverOperandForm::TraceSubject)
    {
        return false;
    }
    if operands.is_none() {
        return contents_write_operand(facts, literal);
    }
    let mut binding_only = false;
    for fact in facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
    {
        match &fact.transition {
            StateTransition::Trace(
                TraceTransition::Add {
                    target: TraceTarget::Variable(variable),
                    ..
                }
                | TraceTransition::Remove {
                    target: TraceTarget::Variable(variable),
                    ..
                },
            ) if variable.argument_index() == Some(index) => binding_only = true,
            StateTransition::VariableCellAlias(alias)
                if alias.local.argument_index() == Some(index) =>
            {
                if alias.writes_value {
                    return true;
                }
                binding_only = true;
            }
            _ => {}
        }
    }
    !binding_only
}

/// Named cells read by a proved getter or the read side of a cell update.
/// Literal argument values are already frozen by the caller's invocation owner.
#[must_use]
pub fn source_variable_read_places(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    source_variable_read_places_with_original_operands_impl(facts, arguments, state, registry, None)
}

pub(crate) fn source_variable_read_places_with_original_operands(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: &OriginalVariableInvocation,
) -> Vec<Place> {
    source_variable_read_places_with_original_operands_impl(
        facts,
        arguments,
        state,
        registry,
        Some(operands),
    )
}

fn source_variable_read_places_with_original_operands_impl(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) -> Vec<Place> {
    if facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
        .any(|fact| matches!(fact.transition, StateTransition::VariableCellAlias(_)))
    {
        return Vec::new();
    }
    facts
        .arg_roles
        .iter()
        .filter_map(|&(index, role)| {
            if role != tcl_registry::ArgRole::VarRead
                && !(role == tcl_registry::ArgRole::VarWrite
                    && facts.traits.contains(Traits::READS_BEFORE_WRITE))
            {
                return None;
            }
            let index = facts.argument_offset + usize::from(index);
            if facts.variable_receiver_operand_form(index)
                == Some(
                    tcl_registry::resolved_invocation::VariableReceiverOperandForm::TraceSubject,
                )
            {
                return None;
            }
            Some(variable_operand_access_impl(
                arguments,
                index,
                state,
                registry,
                tcl_registry::TraceOperation::Read,
                facts.traits.contains(Traits::WHOLE_ARRAY_ARG),
                operands,
            ))
        })
        .collect()
}

pub(crate) fn variable_output_operand_access(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
    state: &ResolveContext,
    registry: &CommandRegistry,
) -> Place {
    variable_output_operand_access_with_original_operands_impl(
        facts, arguments, index, state, registry, None,
    )
}

pub(crate) fn variable_output_operand_access_with_original_operands(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: &OriginalVariableInvocation,
) -> Place {
    variable_output_operand_access_with_original_operands_impl(
        facts,
        arguments,
        index,
        state,
        registry,
        Some(operands),
    )
}

/// Actual normal value-definition receivers at their effective operand
/// ordinals. Trace registrations, binding-only declarations and destruction
/// remain independent from value definitions.
pub(crate) fn source_variable_definitions_with_original_operands(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    order: Option<&[usize]>,
    operands: &OriginalVariableInvocation,
) -> Vec<(usize, Place)> {
    if facts.traits.contains(Traits::DESTROYS_VARIABLE) {
        return Vec::new();
    }
    let uncertain = uncertain_variable_output_addresses_with_original_operands_impl(
        facts,
        arguments,
        state,
        registry,
        order,
        Some(operands),
    );
    facts
        .arg_roles
        .iter()
        .filter_map(|&(index, role)| {
            let index = facts.argument_offset + usize::from(index);
            if role != tcl_registry::ArgRole::VarWrite
                || !contents_write_operand_impl(
                    facts,
                    arguments.literal_at(index),
                    index,
                    Some(operands),
                )
            {
                return None;
            }
            Some((
                index,
                if uncertain.contains(&index) {
                    project_access(
                        place::unknown_top(),
                        state,
                        tcl_registry::TraceOperation::Write,
                    )
                } else {
                    variable_output_operand_access_with_original_operands(
                        facts, arguments, index, state, registry, operands,
                    )
                },
            ))
        })
        .collect()
}

fn variable_output_operand_access_with_original_operands_impl(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) -> Place {
    let operation = if facts.traits.contains(Traits::DESTROYS_VARIABLE) {
        tcl_registry::TraceOperation::Unset
    } else {
        tcl_registry::TraceOperation::Write
    };
    let name = arguments.literal_at(index);
    let alias = facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
        .find_map(|fact| {
            if let StateTransition::VariableCellAlias(alias) = &fact.transition
                && alias.writes_value
                && operands.map_or_else(
                    || name.is_some() && alias.local.literal() == name,
                    |_| alias.local.argument_index() == Some(index),
                )
            {
                Some(alias)
            } else {
                None
            }
        });
    alias.map_or_else(
        || {
            variable_operand_access_impl(
                arguments,
                index,
                state,
                registry,
                operation,
                facts.traits.contains(Traits::WHOLE_ARRAY_ARG),
                operands,
            )
        },
        |alias| {
            project_access(
                operands.map_or_else(
                    || alias_target(state, &alias.target, registry),
                    |operands| {
                        original_alias_target(state, &alias.target, operands, registry, false)
                    },
                ),
                state,
                operation,
            )
        },
    )
}

/// Effective output indices whose address may change after an earlier observer.
/// `order` is an independently selected native protocol; absence retains every
/// possible ordering. This does not grant dispatch, compiler or value evidence.
#[must_use]
pub fn uncertain_variable_output_addresses(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    order: Option<&[usize]>,
) -> std::collections::BTreeSet<usize> {
    uncertain_variable_output_addresses_with_original_operands_impl(
        facts, arguments, state, registry, order, None,
    )
}

fn uncertain_variable_output_addresses_with_original_operands_impl(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    order: Option<&[usize]>,
    operands: Option<&OriginalVariableInvocation>,
) -> std::collections::BTreeSet<usize> {
    let outputs: Vec<_> = facts
        .arg_roles
        .iter()
        .filter_map(|&(index, role)| {
            let index = facts.argument_offset + usize::from(index);
            (role == tcl_registry::ArgRole::VarWrite
                && output_commitment(facts, arguments, index)
                    != Some(tcl_registry::variable_output::VariableOutputCommitment::Unchanged)
                && contents_write_operand_impl(facts, arguments.literal_at(index), index, operands))
            .then(|| {
                (
                    index,
                    variable_output_operand_access_with_original_operands_impl(
                        facts, arguments, index, state, registry, operands,
                    ),
                )
            })
        })
        .collect();
    let may_observe = |index: usize| {
        outputs.iter().any(|(argument, place)| {
            *argument == index && (place.observed || place.kind == PlaceKind::Unknown)
        })
    };
    let order = order.filter(|order| outputs.iter().all(|(index, _)| order.contains(index)));
    outputs
        .iter()
        .filter_map(|(index, _)| {
            let exposed = order.map_or_else(
                || {
                    outputs
                        .iter()
                        .any(|(other, _)| other != index && may_observe(*other))
                },
                |order| {
                    order
                        .iter()
                        .take_while(|other| *other != index)
                        .any(|other| may_observe(*other))
                },
            );
            exposed.then_some(*index)
        })
        .collect()
}

fn default_variable_output_order(facts: &tcl_registry::InvocationFacts) -> Option<Vec<usize>> {
    use tcl_registry::native_compilation::VariableOutputLookup;
    let indices: Vec<_> = facts
        .arg_roles
        .iter()
        .filter_map(|&(index, role)| {
            (role == tcl_registry::ArgRole::VarWrite)
                .then_some(facts.argument_offset + usize::from(index))
        })
        .collect();
    let selected = facts
        .successful_handler
        .map(tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_output_lookup);
    (indices.len() <= 1
        || matches!(
            selected,
            Some(VariableOutputLookup::Sequential | VariableOutputLookup::SingleTarget)
        ))
    .then_some(indices)
}

/// Bound changes with an independently selected native output lookup order.
#[must_use]
pub fn source_variable_write_places_with_output_order(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    order: Option<&[usize]>,
) -> Vec<Place> {
    source_variable_write_places_with_output_order_and_original_operands_impl(
        facts, arguments, state, registry, order, None,
    )
}

pub(crate) fn source_variable_write_places_with_output_order_and_original_operands(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    order: Option<&[usize]>,
    operands: &OriginalVariableInvocation,
) -> Vec<Place> {
    source_variable_write_places_with_output_order_and_original_operands_impl(
        facts,
        arguments,
        state,
        registry,
        order,
        Some(operands),
    )
}

fn source_variable_write_places_with_output_order_and_original_operands_impl(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    order: Option<&[usize]>,
    operands: Option<&OriginalVariableInvocation>,
) -> Vec<Place> {
    let uncertain = uncertain_variable_output_addresses_with_original_operands_impl(
        facts, arguments, state, registry, order, operands,
    );
    let unknown_targets: Vec<_> = uncertain
        .into_iter()
        .map(|index| {
            variable_output_operand_access_with_original_operands_impl(
                facts, arguments, index, state, registry, operands,
            )
        })
        .collect();
    unordered_source_variable_write_places(facts, arguments, state, registry, operands)
        .into_iter()
        .map(|place| {
            if unknown_targets.contains(&place) {
                project_access(
                    place::unknown_top(),
                    state,
                    tcl_registry::TraceOperation::Write,
                )
            } else {
                place
            }
        })
        .collect()
}

/// Bound cells written or destroyed by one already-resolved source invocation.
/// Role indices refer to the effective argument vector, including alias prefixes.
#[must_use]
pub fn source_variable_write_places(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    let order = default_variable_output_order(facts);
    source_variable_write_places_with_output_order(
        facts,
        arguments,
        state,
        registry,
        order.as_deref(),
    )
}

pub(crate) fn source_variable_write_places_with_original_operands(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: &OriginalVariableInvocation,
) -> Vec<Place> {
    let order = default_variable_output_order(facts);
    source_variable_write_places_with_output_order_and_original_operands(
        facts,
        arguments,
        state,
        registry,
        order.as_deref(),
        operands,
    )
}

fn raw_static_unset_operand_error(
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) -> bool {
    operands.map_or_else(
        || {
            arguments
                .literal_at(index)
                .is_some_and(|name| state.raw_static_unset_error(name, registry))
        },
        |operands| {
            operands
                .raw_unset_slot(index, state, registry)
                .is_some_and(|slot| state.raw_static_unset_error_at_slot(&slot))
        },
    )
}

fn unordered_source_variable_write_places(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) -> Vec<Place> {
    let mut writes = namespace_destruction_from_facts(facts, state);
    let aliases = invocation_variable_aliases(facts);
    for alias in &aliases {
        if alias.writes_value {
            writes.push(operands.map_or_else(
                || alias_target(state, &alias.target, registry),
                |operands| original_alias_target(state, &alias.target, operands, registry, false),
            ));
        }
    }
    for &(index, role) in &facts.arg_roles {
        if role != tcl_registry::ArgRole::VarWrite {
            continue;
        }
        if output_commitment(facts, arguments, facts.argument_offset + usize::from(index))
            == Some(tcl_registry::variable_output::VariableOutputCommitment::Unchanged)
        {
            continue;
        }
        let index = facts.argument_offset + usize::from(index);
        let literal = arguments.literal_at(index);
        if facts.traits.contains(Traits::DESTROYS_VARIABLE)
            && raw_static_unset_operand_error(arguments, index, state, registry, operands)
        {
            // A direct Jim static fallback is not an own local table entry.
            // Unset reports missing (or succeeds under -nocomplain) without
            // destroying the retained VarVal.
            continue;
        }
        if !contents_write_operand_impl(facts, literal, index, operands) {
            continue;
        }
        if aliases.iter().any(|alias| {
            !alias.writes_value
                && operands.map_or_else(
                    || alias.local.literal() == literal,
                    |_| alias.local.argument_index() == Some(index),
                )
        }) {
            continue;
        }
        let target = source_write_operand_target(
            facts, arguments, index, state, registry, operands, &aliases,
        );
        let target = project_access(
            target,
            state,
            if facts.traits.contains(Traits::DESTROYS_VARIABLE) {
                tcl_registry::TraceOperation::Unset
            } else {
                tcl_registry::TraceOperation::Write
            },
        );
        if !empty_initialisation_writes(facts, &target, state) {
            continue;
        }
        if !writes.contains(&target) {
            writes.push(target);
        }
    }
    if !facts.arg_roles_complete
        && facts.effects.accesses().iter().any(|access| {
            access.domain == tcl_registry::WorldStateDomain::VariableStore
                && access.mode != tcl_registry::EffectAccessMode::Read
        })
    {
        writes.push(place::unknown_top());
    }
    scoped_world_write_places(facts, state, registry, &mut writes);
    let operation = invocation_write_trace_operation(facts);
    writes
        .into_iter()
        .map(|place| project_access(place, state, operation))
        .collect()
}

fn source_write_operand_target(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
    aliases: &[&VariableCellAliasTransition],
) -> Place {
    let literal = arguments.literal_at(index);
    if let Some(operands) = operands {
        variable_output_operand_access_with_original_operands_impl(
            facts,
            arguments,
            index,
            state,
            registry,
            Some(operands),
        )
    } else {
        literal.map_or_else(
            || {
                variable_operand_access(
                    arguments,
                    index,
                    state,
                    registry,
                    tcl_registry::TraceOperation::Write,
                    facts.traits.contains(Traits::WHOLE_ARRAY_ARG),
                )
            },
            |name| {
                if let Some(alias) = aliases
                    .iter()
                    .find(|alias| alias.local.literal() == Some(name))
                {
                    alias_target(state, &alias.target, registry)
                } else {
                    resolve_literal_place(
                        name,
                        state,
                        facts.traits.contains(Traits::WHOLE_ARRAY_ARG),
                        registry,
                    )
                }
            },
        )
    }
}

fn invocation_write_trace_operation(
    facts: &tcl_registry::InvocationFacts,
) -> tcl_registry::TraceOperation {
    if facts.traits.contains(Traits::DESTROYS_VARIABLE)
        || facts
            .state_transitions
            .declared()
            .is_some_and(|transitions| {
                transitions.facts().iter().any(|fact| {
                    matches!(
                        fact.transition,
                        StateTransition::Namespace(NamespaceTransition::Delete { .. })
                    )
                })
            })
    {
        tcl_registry::TraceOperation::Unset
    } else {
        tcl_registry::TraceOperation::Write
    }
}

fn invocation_variable_aliases(
    facts: &tcl_registry::InvocationFacts,
) -> Vec<&VariableCellAliasTransition> {
    facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
        .filter_map(|fact| match &fact.transition {
            StateTransition::VariableCellAlias(alias) => Some(alias),
            _ => None,
        })
        .collect()
}

fn empty_initialisation_writes(
    facts: &tcl_registry::InvocationFacts,
    target: &Place,
    state: &ResolveContext,
) -> bool {
    if facts.successful_handler
        != Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::InitialiseEmptyVariable)
    {
        return true;
    }
    let read = project_access(target.clone(), state, tcl_registry::TraceOperation::Read);
    read.observed || state.contents_presence(&read) != crate::var_resolve::ContentsPresence::Defined
}

/// Resolve an already evaluated variable-name operand, retaining a bounded array shape.
#[must_use]
pub fn variable_operand_access(
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
    context: &ResolveContext,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
    whole_array: bool,
) -> Place {
    if let Some(name) = arguments.literal_at(index) {
        resolve_literal_access(name, context, whole_array, registry, operation)
    } else if let Some(root) = arguments.array_element_root_at(index) {
        crate::var_resolve::resolve_array_element_root(root, context, registry, operation)
    } else {
        project_access(place::unknown_top(), context, operation)
    }
}

fn variable_operand_access_impl(
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
    context: &ResolveContext,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
    whole_array: bool,
    operands: Option<&OriginalVariableInvocation>,
) -> Place {
    operands.map_or_else(
        || variable_operand_access(arguments, index, context, registry, operation, whole_array),
        |operands| operands.access(index, context, registry, operation, whole_array),
    )
}

fn scoped_world_write_places(
    facts: &tcl_registry::InvocationFacts,
    state: &ResolveContext,
    registry: &CommandRegistry,
    writes: &mut Vec<Place>,
) {
    use tcl_registry::world_effect::{
        EffectAccessMode, InterpreterScope, NamespaceScope, SubjectScope, WorldStateDomain,
    };
    for access in facts.effects.accesses() {
        if access.domain != WorldStateDomain::VariableStore || access.mode == EffectAccessMode::Read
        {
            continue;
        }
        let namespace = match &access.namespace {
            NamespaceScope::Named(namespace) => Some(namespace.as_ref()),
            _ => None,
        };
        if let Some(namespace) = namespace {
            let target = if access.interpreter == InterpreterScope::Current {
                if let Some(identity) = state.namespace_identity_for_written(namespace) {
                    match &access.subject {
                        SubjectScope::Named(name) => {
                            state.namespace_place_in_identity(name, &identity, false)
                        }
                        SubjectScope::Wildcard => {
                            let mut target =
                                state.namespace_place_in_identity("", &identity, false);
                            target.kind = PlaceKind::Unknown;
                            target
                        }
                    }
                } else if state.namespace_identity.is_some() {
                    place::unknown_top()
                } else {
                    match &access.subject {
                        SubjectScope::Named(name) => resolve_literal_place(
                            &tcl_syntax::naming::qualify(namespace, name),
                            state,
                            false,
                            registry,
                        ),
                        SubjectScope::Wildcard => place::unknown_namespace(namespace),
                    }
                }
            } else {
                place::unknown_top()
            };
            if !writes.contains(&target) {
                writes.push(target);
            }
        }
    }
}

fn apply_namespace(
    state: &mut ResolveContext,
    transition: &NamespaceTransition,
    source_offset: u32,
) {
    let (target, deleted) = match transition {
        NamespaceTransition::Ensure { namespace } => (namespace, false),
        NamespaceTransition::Delete { namespace } => (namespace, true),
        _ => return,
    };
    let identity = match target {
        NamespaceTransitionTarget::Current => state.namespace_identity.clone().or_else(|| {
            Some(crate::command_binding::SourceNamespaceKey::authored(
                &state.namespace,
            ))
        }),
        NamespaceTransitionTarget::Named(subject) => subject.literal().and_then(|name| {
            state.namespace_identity_for_written(name).or_else(|| {
                state.namespace_identity.is_none().then(|| {
                    crate::command_binding::SourceNamespaceKey::authored(
                        tcl_syntax::naming::qualify_namespace(&state.namespace, name),
                    )
                })
            })
        }),
    };
    let Some(identity) = identity else {
        state.widen();
        return;
    };
    if deleted {
        delete_namespace_cells_in_identity(state, &identity, source_offset);
    } else {
        if let Some(display) = identity.display() {
            state.known_namespaces.insert(display);
        }
        state.namespace_identities.insert(identity);
    }
}

/// Retire cells in the exact original namespace tree before unset callbacks.
pub(crate) fn delete_namespace_cells_in_identity(
    state: &mut ResolveContext,
    namespace: &crate::command_binding::SourceNamespaceKey,
    source_offset: u32,
) {
    use crate::command_binding::SourceNamespaceKey;
    let mut original: crate::var_resolve::VariableNamespaceSet = state
        .namespace_identities
        .iter()
        .filter(|identity| crate::var_resolve::namespace_contains(namespace, identity))
        .cloned()
        .collect();
    original.insert(namespace.clone());
    state
        .namespace_addressable_identities
        .retain(|identity| !original.contains(identity));
    // Native active namespaces detach now, but their original contents remain until pop.
    let mut held = Vec::new();
    let mut frame = Some(&*state);
    while let Some(current) = frame {
        if matches!(
            current.frame_kind,
            crate::var_resolve::VariableFrameKind::Namespace
                | crate::var_resolve::VariableFrameKind::Local
        ) && let Some(key) = &current.namespace_identity
            && !matches!(key, SourceNamespaceKey::Authored(_))
            && original.contains(key)
        {
            held.push(key.clone());
        }
        frame = current.caller.as_deref();
    }
    for owner in held {
        let tree: crate::var_resolve::VariableNamespaceSet = original
            .iter()
            .filter(|key| crate::var_resolve::namespace_contains(&owner, key))
            .cloned()
            .collect();
        original.retain(|key| !tree.contains(key));
        state.pending_namespace_retirements.entry(owner).or_insert(
            crate::var_resolve::PendingNamespaceRetirement {
                namespaces: tree,
                source: source_offset,
                conditional: false,
            },
        );
    }
    retire_namespace_cell_incarnations(state, &original, source_offset, false);
}

/// Pop-time retirement of the captured original tree, never a later equal-path namespace.
pub(crate) fn complete_pending_namespace_retirements(state: &mut ResolveContext) {
    let mut active = std::collections::HashSet::new();
    let mut frame = Some(&*state);
    while let Some(current) = frame {
        if matches!(
            current.frame_kind,
            crate::var_resolve::VariableFrameKind::Namespace
                | crate::var_resolve::VariableFrameKind::Local
        ) && let Some(key) = &current.namespace_identity
        {
            active.insert(key.clone());
        }
        frame = current.caller.as_deref();
    }
    let completed: Vec<_> = state
        .pending_namespace_retirements
        .keys()
        .filter(|key| !active.contains(*key))
        .cloned()
        .collect();
    for key in completed {
        let pending = state
            .pending_namespace_retirements
            .remove(&key)
            .expect("selected pending owner");
        // A child activation can outlive its detached parent; defer that child's tree.
        let mut remaining = pending.namespaces;
        for child in &active {
            if remaining.contains(child) {
                let tree: crate::var_resolve::VariableNamespaceSet = remaining
                    .iter()
                    .filter(|key| crate::var_resolve::namespace_contains(child, key))
                    .cloned()
                    .collect();
                remaining.retain(|key| !tree.contains(key));
                state
                    .pending_namespace_retirements
                    .entry(child.clone())
                    .or_insert(crate::var_resolve::PendingNamespaceRetirement {
                        namespaces: tree,
                        source: pending.source,
                        conditional: pending.conditional,
                    });
            }
        }
        retire_namespace_cell_incarnations(state, &remaining, pending.source, pending.conditional);
    }
}

fn retire_namespace_cell_incarnations(
    state: &mut ResolveContext,
    namespaces: &crate::var_resolve::VariableNamespaceSet,
    source_offset: u32,
    conditional: bool,
) {
    let selected = |key: &crate::var_resolve::VariableCellKey| {
        namespaces
            .iter()
            .any(|namespace| key.is_owned_by_namespace(namespace))
    };
    state
        .captured_cells
        .delete_namespace_incarnations(namespaces);
    for namespace in namespaces {
        if !conditional
            && let crate::command_binding::SourceNamespaceKey::Authored(name) = namespace
        {
            state.captured_cells.delete_namespace(name);
        }
    }
    state
        .outward_namespace_destructions
        .extend(namespaces.iter().cloned());
    state.retain_literal_values(|key| !selected(key));
    state.value_representations.retain(|key, _| !selected(key));
    state.contents_kinds.retain(|key, _| !selected(key));
    for (key, presence) in &mut state.contents_presence {
        if selected(key) {
            *presence = if conditional {
                crate::var_resolve::ContentsPresence::Unknown
            } else {
                crate::var_resolve::ContentsPresence::Undefined
            };
        }
    }
    for (key, origin) in &mut state.contents_origins {
        if selected(key) {
            *origin = if conditional {
                crate::var_resolve::ContentsOrigin::Unknown
            } else {
                crate::var_resolve::ContentsOrigin::WrittenAt(source_offset)
            };
        }
    }
    for key in state
        .namespace_cells
        .present
        .iter()
        .chain(&state.namespace_cells.possible)
        .filter(|key| selected(key))
        .cloned()
        .collect::<Vec<_>>()
    {
        state.generations.insert(
            key,
            if conditional {
                CellGeneration::Unknown
            } else {
                CellGeneration::After(source_offset)
            },
        );
    }
    state.namespace_cells.present.retain(|key| !selected(key));
    state.namespace_cells.possible.retain(|key| !selected(key));
    state
        .namespace_cells
        .closed_namespaces
        .retain(|key| !namespaces.contains(key));
    state.closed_array_roots.retain(|key| !selected(key));
    state
        .namespace_identities
        .retain(|identity| !namespaces.contains(identity));
    state
        .namespace_addressable_identities
        .retain(|identity| !namespaces.contains(identity));
    state.namespace_objects.retain(|identity, _| {
        state.namespace_identity.as_ref() == Some(identity) || !namespaces.contains(identity)
    });
    for namespace in namespaces {
        if let crate::command_binding::SourceNamespaceKey::Authored(namespace) = namespace {
            state.known_namespaces.retain(|known| {
                known != namespace && !known.starts_with(&format!("{namespace}::"))
            });
        }
    }
    retire_namespace_observers_and_links(state, selected, source_offset);
}

fn retire_namespace_observers_and_links(
    state: &mut ResolveContext,
    selected: impl Fn(&crate::var_resolve::VariableCellKey) -> bool,
    source_offset: u32,
) {
    state.traced.retain(|key| !selected(key));
    state.trace_registrations.retain(|key, _| !selected(key));
    state
        .trace_registration_receivers
        .retain(|_, receiver| !selected(&cell_key(receiver)));
    state.untracked_traces.retain(|key| !selected(key));
    state
        .namespace_alias_bindings
        .retain(|slot, _| !selected(slot));
    state
        .namespace_name_alias_bindings
        .retain(|slot, _| !selected(slot));
    let name_following = state
        .invocation_dialect
        .and_then(|dialect| dialect.variable_link_binding)
        == Some(tcl_dialect::VariableLinkBinding::SelectedFrameName);
    for alias in state.namespace_alias_bindings.values_mut() {
        if selected(&cell_key(alias)) {
            if name_following {
                if let Some(cell) = &mut alias.cell {
                    cell.generation = CellGeneration::After(source_offset);
                }
            } else {
                *alias = place::unknown_top();
            }
        }
    }
    for (name, alias) in &mut state.alias_bindings {
        if selected(&cell_key(alias)) {
            if name_following {
                if let Some(cell) = &mut alias.cell {
                    cell.generation = CellGeneration::After(source_offset);
                }
            } else {
                state.unknown_bindings.insert(name.clone());
            }
        }
    }
}

fn establish_written_lifetime(state: &mut ResolveContext, target: &Place, offset: u32) {
    if target.dynamic
        || target.kind == PlaceKind::Unknown
        || !target
            .cell
            .as_ref()
            .is_some_and(|cell| cell.generation == CellGeneration::Unknown)
    {
        return;
    }
    let key = cell_key(target);
    let generation = CellGeneration::After(offset);
    state.generations.insert(key.clone(), generation);
    for alias in state
        .alias_bindings
        .values_mut()
        .chain(state.namespace_alias_bindings.values_mut())
    {
        if cell_key(alias) == key
            && let Some(cell) = &mut alias.cell
        {
            cell.generation = generation;
        }
    }
}

/// Apply a reached literal unset through the shared cell lifetime/alias owner.
/// Dictionary entry and captured output protocols use this without inventing
/// a command invocation or re-running its argument evaluation.
pub fn destroy_literal_binding(
    state: &mut ResolveContext,
    name: &str,
    source: u32,
    registry: &CommandRegistry,
) {
    let target = resolve_literal_access(
        name,
        state,
        false,
        registry,
        tcl_registry::TraceOperation::Unset,
    );
    state.invalidate_contents_literals(&target);
    destroy_root(state, name, source, registry);
}

fn destroy_root(state: &mut ResolveContext, name: &str, offset: u32, registry: &CommandRegistry) {
    if state.raw_static_unset_error(name, registry)
        || state.detach_retained_binding(name, offset, registry)
    {
        return;
    }
    let target = resolve_literal_access(
        name,
        state,
        false,
        registry,
        tcl_registry::TraceOperation::Unset,
    );
    destroy_captured_cell(state, &target, offset, registry);
}

/// Clear the selected cell without resolving its original variable name again.
/// The caller owns the captured unset callback list and completion protocol.
pub(crate) fn destroy_captured_cell(
    state: &mut ResolveContext,
    target: &Place,
    offset: u32,
    registry: &CommandRegistry,
) {
    state.record_presence_slot(target);
    if target.index.is_none()
        && let Some(root) = crate::var_resolve::physical_array_key(target)
    {
        state.closed_array_roots.remove(&root);
    }
    let root_kind = state.contents_kinds.get(&cell_key(target)).copied();
    state.captured_cells.destroy(target, root_kind);
    if target.observed {
        state.widen();
        return;
    }
    if let Some(name) = crate::var_resolve::canonical_binding_value_key(target) {
        state
            .contents_presence
            .insert(name, crate::var_resolve::ContentsPresence::Undefined);
    }
    let key = cell_key(target);
    let trace = trace_key(target);
    let removed: Vec<_> = state
        .traced
        .iter()
        .filter(|registered| {
            *registered == &trace
                || (target.kind != PlaceKind::ArrayElem && registered.is_member_of(&key))
        })
        .cloned()
        .collect();
    for registered in removed {
        state.traced.remove(&registered);
        state.trace_registrations.remove(&registered);
        state.trace_registration_receivers.remove(&registered);
        state.untracked_traces.remove(&registered);
    }
    if target.kind == PlaceKind::ArrayElem {
        return;
    }
    state.contents_kinds.remove(&key);
    state
        .generations
        .insert(key.clone(), CellGeneration::After(offset));
    let jim = state
        .invocation_dialect
        .and_then(|dialect| dialect.variable_link_binding)
        .or_else(|| {
            registry
                .profile()
                .and_then(tcl_dialect::DialectProfile::variable_link_binding)
        })
        == Some(tcl_dialect::VariableLinkBinding::SelectedFrameName);
    for (name, alias) in &mut state.alias_bindings {
        if cell_key(alias) == key {
            if alias.kind == PlaceKind::ArrayElem && !jim {
                state.unknown_bindings.insert(name.clone());
            } else if let Some(cell) = &mut alias.cell {
                cell.generation = CellGeneration::After(offset);
            }
        }
    }
}

fn detach_retained_destructions(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    writes: &mut Vec<Place>,
    source: u32,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) {
    if !facts.traits.contains(Traits::DESTROYS_VARIABLE) {
        return;
    }
    for &(index, role) in &facts.arg_roles {
        if role != tcl_registry::ArgRole::VarWrite {
            continue;
        }
        let index = facts.argument_offset + usize::from(index);
        if let Some(operands) = operands {
            let Some(slot) = operands.raw_unset_slot(index, state, registry) else {
                continue;
            };
            let before = operands.access(
                index,
                state,
                registry,
                tcl_registry::TraceOperation::Unset,
                false,
            );
            if !before.observed && state.detach_retained_binding_at_slot(&slot, source) {
                writes.retain(|target| target != &before);
            }
        } else if let Some(name) = arguments.literal_at(index) {
            let before = resolve_literal_place(name, state, false, registry);
            if !before.observed && state.detach_retained_binding(name, source, registry) {
                writes.retain(|target| target != &before);
            }
        }
    }
}

fn invalidate_literal_writes(state: &mut ResolveContext, writes: &[Place]) {
    for place in writes {
        state.invalidate_contents_literals(place);
    }
}

fn source_literal_store(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) -> Option<(Place, String)> {
    use tcl_registry::SemanticOperationId::StructuredLowering;
    use tcl_registry::hooks::LoweringHookId;
    let offset = facts.argument_offset;
    let count = arguments.exact_argv_len()?.checked_sub(offset)?;
    let target = variable_operand_access_impl(
        arguments,
        offset,
        state,
        registry,
        tcl_registry::TraceOperation::Write,
        false,
        operands,
    );
    if target.kind == PlaceKind::Unknown || target.observed {
        return None;
    }
    if facts.successful_handler
        == Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::InitialiseEmptyVariable)
    {
        let read = project_access(target.clone(), state, tcl_registry::TraceOperation::Read);
        return (!read.observed
            && state.contents_presence(&read) == crate::var_resolve::ContentsPresence::Undefined)
            .then_some((target, String::new()));
    }
    let value = match facts.operation {
        StructuredLowering(LoweringHookId::Set) if count == 2 => {
            arguments.literal_at(offset + 1)?.to_owned()
        }
        StructuredLowering(LoweringHookId::Incr) if count == 1 || count == 2 => {
            let read = project_access(target.clone(), state, tcl_registry::TraceOperation::Read);
            let dialect = state.invocation_dialect.or_else(|| arguments.dialect());
            let initial = if state.contents_presence(&read)
                == crate::var_resolve::ContentsPresence::Undefined
                && dialect.and_then(|dialect| {
                    dialect.native_rmw_read_policy(
                        tcl_registry::native_rmw::NativeRmwOperation::Increment,
                    )
                }) == Some(tcl_registry::native_rmw::NativeRmwReadPolicy::InitialiseZero)
            {
                "0"
            } else {
                state.literal_contents_at(&read, registry)?
            };
            source_increment_contents(initial, arguments, offset, state, registry)?
        }
        _ => return None,
    };
    Some((target, value))
}

/// Compute increment bytes from an already captured value, without resolving its name again.
pub(crate) fn source_increment_contents(
    value: &str,
    arguments: tcl_registry::InvocationArguments<'_>,
    offset: usize,
    state: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<String> {
    let numbers = state.invocation_dialect.map_or_else(
        || {
            registry
                .profile()
                .map_or(tcl_syntax::number::Numbers::Unknown, |profile| {
                    tcl_syntax::number::Numbers::of_profile(Some(profile))
                })
        },
        |dialect| tcl_syntax::number::Numbers::Target(dialect.numbers),
    );
    let value = numbers.parse_wide(value)?;
    let count = arguments.exact_argv_len()?.checked_sub(offset)?;
    let increment = if count == 2 {
        numbers.parse_wide(arguments.literal_at(offset + 1)?)?
    } else {
        1
    };
    value.checked_add(increment).map(|value| value.to_string())
}

fn conditional_contents_write(
    facts: &tcl_registry::InvocationFacts,
    place: &Place,
    state: &ResolveContext,
) -> bool {
    facts.traits.contains(Traits::CONDITIONAL_VARIABLE_WRITE)
        && !(facts.successful_handler
            == Some(
                tcl_registry::native_compilation::SuccessfulHandlerSpec::InitialiseEmptyVariable,
            )
            && !place.observed
            && state.contents_presence(place) == crate::var_resolve::ContentsPresence::Undefined)
}

fn output_commitment(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
) -> Option<tcl_registry::variable_output::VariableOutputCommitment> {
    facts
        .successful_variable_output_commitments(arguments)?
        .into_iter()
        .find_map(|(argument, commitment)| (argument == index).then_some(commitment))
}

fn normal_output_is_written_impl(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    target: &Place,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) -> bool {
    if target.observed || target.dynamic || target.cell.is_none() {
        return false;
    }
    facts
        .successful_variable_output_commitments(arguments)
        .is_some_and(|commitments| {
            commitments.into_iter().any(|(index, commitment)| {
                commitment == tcl_registry::variable_output::VariableOutputCommitment::Written
                    && variable_output_operand_access_with_original_operands_impl(
                        facts, arguments, index, state, registry, operands,
                    ) == *target
            })
        })
}

/// Project a proved source invocation into shared namespace-cell allocation
/// facts. The source execution owner supplies the actual selected frame and
/// whether its literal script effects have already been interpreted.
pub fn transfer_source_namespace_cells(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    registry: &CommandRegistry,
    script_interpreted: bool,
    source_offset: u32,
) {
    let order = default_variable_output_order(facts);
    transfer_source_namespace_cells_with_output_order(
        state,
        facts,
        arguments,
        registry,
        script_interpreted,
        source_offset,
        order.as_deref(),
    );
}

/// Apply source cell effects with an independently selected output lookup order.
/// Unknown order preserves every observer-driven address retargeting possibility.
pub fn transfer_source_namespace_cells_with_output_order(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    registry: &CommandRegistry,
    script_interpreted: bool,
    source_offset: u32,
    output_order: Option<&[usize]>,
) {
    transfer_source_namespace_cells_with_input(
        state,
        facts,
        arguments,
        registry,
        SourceNamespaceTransfer::new(script_interpreted, source_offset, output_order),
    );
}

/// Original source-variable transfer facets retained independently of display names.
#[derive(Clone, Copy)]
pub(crate) struct SourceNamespaceTransfer<'a> {
    script_interpreted: bool,
    source_offset: u32,
    output_order: Option<&'a [usize]>,
    original_locals: &'a [Option<
        crate::command_binding::original_variable_compilation::OriginalCompiledNamespaceLocal,
    >],
    operands: Option<&'a OriginalVariableInvocation>,
    namespace_operations:
        crate::command_binding::original_namespace_ensure::OriginalNamespaceCellOperations<'a>,
}

impl<'a> SourceNamespaceTransfer<'a> {
    pub(crate) fn new(
        script_interpreted: bool,
        source_offset: u32,
        output_order: Option<&'a [usize]>,
    ) -> Self {
        Self {
            script_interpreted,
            source_offset,
            output_order,
            original_locals: &[],
            operands: None,
            namespace_operations: crate::command_binding::original_namespace_ensure::OriginalNamespaceCellOperations::default(),
        }
    }

    pub(crate) fn with_original_operands(
        mut self,
        operands: &'a OriginalVariableInvocation,
    ) -> Self {
        self.original_locals = &operands.compiled_locals;
        self.operands = Some(operands);
        self
    }

    pub(crate) fn with_namespace_operations(
        mut self,
        operations: crate::command_binding::original_namespace_ensure::OriginalNamespaceCellOperations<'a>,
    ) -> Self {
        self.namespace_operations = operations;
        self
    }
}

pub(crate) fn transfer_source_namespace_cells_with_input(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    registry: &CommandRegistry,
    input: SourceNamespaceTransfer<'_>,
) {
    use tcl_registry::world_effect::{CallbackKinds, EffectAccessMode, WorldStateDomain};
    let SourceNamespaceTransfer {
        script_interpreted,
        source_offset,
        output_order,
        operands,
        ..
    } = input;
    let uncertain = uncertain_variable_output_addresses_with_original_operands_impl(
        facts,
        arguments,
        state,
        registry,
        output_order,
        operands,
    );
    let mut writes = source_variable_write_places_with_output_order_and_original_operands_impl(
        facts,
        arguments,
        state,
        registry,
        output_order,
        operands,
    );
    detach_retained_destructions(
        state,
        facts,
        arguments,
        &mut writes,
        source_offset,
        registry,
        operands,
    );
    let trace_callback = writes.iter().any(|target| target.observed)
        || source_variable_read_places_with_original_operands_impl(
            facts, arguments, state, registry, operands,
        )
        .iter()
        .any(|target| target.observed);
    let literal_store = source_literal_store(facts, arguments, state, registry, operands);
    invalidate_literal_writes(state, &writes);
    record_source_contents_writes(state, facts, &writes, source_offset);
    transfer_source_state_transitions(state, facts, registry, input);
    update_written_namespace_cells(
        state,
        facts,
        arguments,
        registry,
        source_offset,
        &uncertain,
        operands,
    );
    if trace_callback {
        state.widen();
    }
    if !trace_callback && let Some((mut receiver, value)) = literal_store {
        let generation = state
            .generations
            .get(&cell_key(&receiver))
            .copied()
            .unwrap_or_default();
        if let Some(cell) = &mut receiver.cell {
            cell.generation = generation;
        }
        if generation != CellGeneration::Unknown && !state.store_would_error(&receiver) {
            state.publish_captured_store(&receiver, Some(&value), source_offset);
        }
    }
    let trace_only = facts.effects.callback().kinds == CallbackKinds::TRACE;
    if facts.effects.requires_world_barrier()
        && !script_interpreted
        && (!trace_only || trace_callback)
    {
        state.namespace_cells.widen();
    }
    if !script_interpreted
        && facts.effects.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::VariableStore
                && access.mode != EffectAccessMode::Read
                && !facts.arg_roles_complete
        })
    {
        state.namespace_cells.widen();
    }
}

fn record_source_contents_writes(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    writes: &[Place],
    source_offset: u32,
) {
    for target in writes {
        if facts.traits.contains(Traits::DESTROYS_VARIABLE)
            && target
                .index
                .as_ref()
                .is_some_and(|index| index.kind != place::IndexKind::Literal)
        {
            state.record_unknown_element_destruction(target);
        } else if !facts.traits.contains(Traits::DESTROYS_VARIABLE)
            || target.kind == PlaceKind::Unknown
        {
            state.record_contents_write(
                target,
                source_offset,
                conditional_contents_write(facts, target, state),
            );
        }
    }
}

fn transfer_source_state_transitions(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    registry: &CommandRegistry,
    input: SourceNamespaceTransfer<'_>,
) {
    let SourceNamespaceTransfer {
        script_interpreted,
        source_offset,
        original_locals,
        operands,
        namespace_operations,
        ..
    } = input;
    if let StateTransitionKnowledge::Declared(transitions) = &facts.state_transitions {
        for fact in transitions.facts() {
            if script_interpreted && matches!(fact.transition, StateTransition::Widen(_)) {
                continue;
            }
            if let StateTransition::VariableCellAlias(alias) = &fact.transition {
                let target = operands.map_or_else(
                    || alias_target(state, &alias.target, registry),
                    |operands| {
                        original_alias_target(state, &alias.target, operands, registry, false)
                    },
                );
                if target.is_global() && !target.dynamic {
                    state.namespace_cells.present.insert(cell_key(&target));
                }
            }
            if let StateTransition::VariableCellAlias(alias) = &fact.transition {
                let local = alias.local.argument_index().and_then(|index| {
                    operands.map_or_else(
                        || original_locals.get(index).and_then(Option::as_ref),
                        |operands| operands.compiled_local(index),
                    )
                });
                bind_alias_with_compiled_local(state, alias, registry, local, operands);
            } else if let StateTransition::Trace(trace) = &fact.transition {
                apply_trace_with_original(state, trace, registry, operands);
            } else if let StateTransition::Namespace(NamespaceTransition::Ensure {
                namespace: NamespaceTransitionTarget::Named(subject),
            }) = &fact.transition
                && let Some(namespace) = namespace_operations.ensures.and_then(|ensured| {
                    ensured
                        .namespace(subject, operands, state, source_offset)
                        .cloned()
                })
            {
                // naming.namespace.original-counted-namespace-allocation
                // docs/design/analysis/name-resolution-proofs/namespace-original-counted-allocation.md
                state.namespace_identities.insert(namespace.clone());
                state.namespace_addressable_identities.insert(namespace);
            } else if let StateTransition::Namespace(NamespaceTransition::Delete {
                namespace: NamespaceTransitionTarget::Named(subject),
            }) = &fact.transition
                && namespace_operations
                    .deletions
                    .is_some_and(|retired| retired.matches(subject, operands, state, source_offset))
            {
                // The canonical command transfer has already retired this exact
                // namespace incarnation. Do not resolve its old name again.
            } else {
                apply_transition(state, &fact.transition, registry, source_offset);
            }
        }
    }
}

fn update_written_namespace_cells(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    registry: &CommandRegistry,
    source_offset: u32,
    uncertain_outputs: &std::collections::BTreeSet<usize>,
    operands: Option<&OriginalVariableInvocation>,
) {
    let destroying = facts.traits.contains(Traits::DESTROYS_VARIABLE);
    let conditional = facts.traits.contains(Traits::CONDITIONAL_VARIABLE_WRITE);
    if facts.arg_roles_complete {
        for &(index, role) in &facts.arg_roles {
            if role != tcl_registry::ArgRole::VarWrite {
                continue;
            }
            if output_commitment(facts, arguments, facts.argument_offset + usize::from(index))
                == Some(tcl_registry::variable_output::VariableOutputCommitment::Unchanged)
            {
                continue;
            }
            if uncertain_outputs.contains(&(facts.argument_offset + usize::from(index))) {
                state.namespace_cells.widen();
                continue;
            }
            let argument = facts.argument_offset + usize::from(index);
            if destroying
                && raw_static_unset_operand_error(arguments, argument, state, registry, operands)
            {
                continue;
            }
            let literal = arguments.literal_at(argument);
            if !contents_write_operand_impl(
                facts,
                literal,
                facts.argument_offset + usize::from(index),
                operands,
            ) {
                continue;
            }
            let target = variable_operand_access_impl(
                arguments,
                facts.argument_offset + usize::from(index),
                state,
                registry,
                if destroying {
                    tcl_registry::TraceOperation::Unset
                } else {
                    tcl_registry::TraceOperation::Write
                },
                facts.traits.contains(Traits::WHOLE_ARRAY_ARG),
                operands,
            );
            if target.kind == PlaceKind::Unknown {
                // The writer can select either current or global candidate.
                state.namespace_cells.widen();
                continue;
            }
            if destroying {
                if operands.is_some() {
                    destroy_captured_cell(state, &target, source_offset, registry);
                } else if let Some(name) = literal {
                    destroy_root(state, name, source_offset, registry);
                } else {
                    state.record_unknown_element_destruction(&target);
                }
            }
            if target.observed {
                state.widen();
            } else if !destroying && !conditional {
                establish_written_lifetime(state, &target, source_offset);
            }
            if !target.is_global() || target.dynamic {
                continue;
            }
            let key = cell_key(&target);
            if destroying {
                if target.kind != PlaceKind::ArrayElem {
                    state.namespace_cells.present.remove(&key);
                    state.namespace_cells.possible.insert(key);
                }
            } else if conditional {
                state.namespace_cells.possible.insert(key);
            } else {
                state.namespace_cells.present.insert(key);
            }
        }
    }
}

/// Allocation baseline for a driver-proved fresh interpreter. Catalogue
/// availability alone never selects this constructor.
#[must_use]
pub fn fresh_namespace_cells(
    registry: &CommandRegistry,
) -> crate::var_resolve::NamespaceCellPresence {
    let query = registry.own_surface_query();
    let present = tcl_registry::special_vars::SPECIAL_VARS
        .iter()
        .filter(|variable| variable.readable_at_startup_in(query))
        .map(|variable| tcl_syntax::naming::qualify("::", variable.name))
        .collect();
    crate::var_resolve::NamespaceCellPresence {
        present,
        possible: crate::var_resolve::VariableCellSet::default(),
        closed: true,
        closed_namespaces: crate::var_resolve::VariableNamespaceSet::default(),
    }
}

#[cfg(test)]
mod tests;
