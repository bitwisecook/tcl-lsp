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

#[cfg(test)]
use tcl_registry::{
    CallerFrameSelection, FrameLevel, NamespaceTransition, NamespaceTransitionTarget,
    StateTransition, TraceTarget, TraceTransition,
};
use tcl_registry::{CommandRegistry, VariableAliasTarget, VariableCellAliasTransition};

use crate::cfg::{BlockId, Function};
use crate::ir::Statement;
use crate::place::{self, CellGeneration, CellOwner, Place, PlaceKind};
#[cfg(test)]
use crate::var_resolve::resolve_place;
use crate::var_resolve::{ResolveContext, cell_key, resolve_literal_access};
#[cfg(test)]
use crate::var_resolve::{project_access, resolve_literal_place, trace_key};

mod alias_binding;
mod alias_targets;
mod cell_lifetimes;
mod invocation_transfer;
mod namespace_cells;
mod namespace_retirement;
mod source_places;
mod variable_traces;

#[cfg(test)]
use alias_binding::bind_alias_with_compiled_local;
pub use alias_binding::{
    AliasBindingValidity, CallerReferenceBinding, alias_binding_validity, bind_caller_reference,
};
pub(crate) use alias_binding::{alias_destination_is_direct, alias_registration_is_closed};
#[cfg(test)]
use alias_binding::{alias_value_binding_validity, bind_authored_alias_values};
#[cfg(test)]
use alias_targets::original_alias_target;
pub(crate) use cell_lifetimes::destroy_captured_cell;
pub use cell_lifetimes::destroy_literal_binding;
#[cfg(test)]
use cell_lifetimes::{destroy_root, establish_written_lifetime};
pub use invocation_transfer::{
    SourceIterationBindingOutcome, transfer_source_iteration_bindings, transfer_statement,
};
pub(crate) use invocation_transfer::{
    transfer_captured_variable_outputs, transfer_finite_iteration_bindings,
    transfer_resolved_invocation,
};
pub(crate) use namespace_cells::{
    SourceNamespaceTransfer, source_increment_contents, transfer_source_namespace_cells_with_input,
};
pub use namespace_cells::{
    fresh_namespace_cells, transfer_source_namespace_cells,
    transfer_source_namespace_cells_with_output_order,
};
#[cfg(test)]
use namespace_retirement::apply_namespace;
pub use namespace_retirement::namespace_destruction_places;
pub(crate) use namespace_retirement::{
    complete_pending_namespace_retirements, delete_namespace_cells_in_identity,
    namespace_destruction_places_with_metadata_context,
};
pub(crate) use source_places::{
    contents_write_operand, source_variable_definitions_with_original_operands,
    source_variable_read_places_with_original_operands,
    source_variable_write_places_with_original_operands,
    source_variable_write_places_with_output_order_and_original_operands,
    variable_output_operand_access, variable_output_operand_access_with_original_operands,
};
pub use source_places::{
    source_variable_read_places, source_variable_write_places,
    source_variable_write_places_with_output_order, uncertain_variable_output_addresses,
    variable_operand_access,
};
#[cfg(test)]
use source_places::{
    source_variable_read_places_with_original_operands_impl,
    source_variable_write_places_with_output_order_and_original_operands_impl,
};
#[cfg(test)]
use variable_traces::variable_trace_registration_is_closed;
pub(crate) use variable_traces::variable_trace_registration_is_closed_with_original_operands;
#[cfg(test)]
use variable_traces::{apply_trace, apply_trace_with_original};

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

#[cfg(test)]
mod tests;
